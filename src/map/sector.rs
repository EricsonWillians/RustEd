// src/map/sector.rs

use std::io::{self, Read, Seek, Write};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A sector in classic DOOM format (26 bytes).
///
/// Layout (all little-endian):
///
/// ```text
/// offset  field          type / size
/// ------  -------------  ------------
///  0-1    floor_height   i16
///  2-3    ceiling_height i16
///  4-11   floor_tex      [u8; 8]
/// 12-19   ceiling_tex    [u8; 8]
/// 20-21   light_level    i16
/// 22-23   special_type   i16
/// 24-25   tag            i16
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Sector {
    /// The floor height (in map units).
    pub floor_height: i32,

    /// The ceiling height (in map units).
    pub ceiling_height: i32,

    /// The name of the floor flat. Classic DOOM uses up to 8 chars (padded).
    pub floor_tex: String,

    /// The name of the ceiling flat, up to 8 chars (padded).
    pub ceiling_tex: String,

    /// Light level (0-255 in classic DOOM, often 0-255).
    pub light: i32,

    /// Special type (a.k.a. "effect" or "sector type").
    /// In DOOM, this is an i16; values > 255 are used in BOOM, etc.
    pub r#type: i32,

    /// Sector tag, used to link linedefs, etc.
    pub tag: i32,
}

impl Sector {
    /// Creates a new sector in memory with the specified field values.
    ///
    /// **Example**:
    /// ```
    /// let s = Sector::new(
    ///     0,         // floor height
    ///     64,        // ceiling height
    ///     "FLOOR4_8".to_string(),
    ///     "CEIL3_5".to_string(),
    ///     160,       // light
    ///     0,         // type
    ///     0,         // tag
    /// );
    /// ```
    pub fn new(
        floor_height: i32,
        ceiling_height: i32,
        floor_tex: String,
        ceiling_tex: String,
        light: i32,
        r#type: i32,
        tag: i32,
    ) -> Self {
        Sector {
            floor_height,
            ceiling_height,
            floor_tex,
            ceiling_tex,
            light,
            r#type,
            tag,
        }
    }

    /// Reads a `Sector` from a classic DOOM WAD in its 26-byte format.
    ///
    /// # Layout
    /// ```text
    ///  0-1   floor_height   (i16)
    ///  2-3   ceiling_height (i16)
    ///  4-11  floor_tex      (8 bytes)
    /// 12-19  ceiling_tex    (8 bytes)
    /// 20-21  light          (i16)
    /// 22-23  special/type   (i16)
    /// 24-25  tag            (i16)
    /// ```
    /// We store these as `i32` or `String` in the struct.
    /// Textures are trimmed for trailing spaces/zeros.
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let floor_height = reader.read_i16::<LE>()? as i32;
        let ceiling_height = reader.read_i16::<LE>()? as i32;
        let floor_tex = read_flat8(reader)?;
        let ceiling_tex = read_flat8(reader)?;
        let light = reader.read_i16::<LE>()? as i32;
        let r#type = reader.read_i16::<LE>()? as i32;
        let tag = reader.read_i16::<LE>()? as i32;

        Ok(Sector {
            floor_height,
            ceiling_height,
            floor_tex,
            ceiling_tex,
            light,
            r#type,
            tag,
        })
    }

    /// Writes this `Sector` to a DOOM WAD in the 26-byte format.
    ///
    /// # Layout
    /// ```text
    ///  0-1   floor_height   (i16)
    ///  2-3   ceiling_height (i16)
    ///  4-11  floor_tex      (8 bytes, zero-padded)
    /// 12-19  ceiling_tex    (8 bytes, zero-padded)
    /// 20-21  light          (i16)
    /// 22-23  special/type   (i16)
    /// 24-25  tag            (i16)
    /// ```
    /// Textures are uppercased, then clipped/padded to 8 bytes.
    pub fn to_wad<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_i16::<LE>(self.floor_height as i16)?;
        writer.write_i16::<LE>(self.ceiling_height as i16)?;
        write_flat8(writer, &self.floor_tex)?;
        write_flat8(writer, &self.ceiling_tex)?;
        writer.write_i16::<LE>(self.light as i16)?;
        writer.write_i16::<LE>(self.r#type as i16)?;
        writer.write_i16::<LE>(self.tag as i16)?;
        Ok(())
    }

    /// Returns the difference between ceiling and floor height.
    pub fn headroom(&self) -> i32 {
        self.ceiling_height - self.floor_height
    }

    /// Sets common defaults for a newly created sector.
    /// This is a convenience method similar to what Eureka does.
    ///
    /// # Example
    /// ```
    /// let mut sector = Sector::new(0, 64, "", "", 160, 0, 0);
    /// sector.set_defaults("FLOOR4_8", "CEIL3_5", 160, 0, 0);
    /// ```
    pub fn set_defaults(
        &mut self,
        floor_tex: &str,
        ceiling_tex: &str,
        light: i32,
        r#type: i32,
        tag: i32,
    ) {
        self.floor_height = 0;
        self.ceiling_height = 64;
        self.floor_tex = floor_tex.to_uppercase();
        self.ceiling_tex = ceiling_tex.to_uppercase();
        self.light = light;
        self.r#type = r#type;
        self.tag = tag;
    }
}

/// Helper: reads an 8-byte "flat name" from the WAD.  
/// Trims trailing `\0` and spaces, uppercases it.
fn read_flat8<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;

    let raw = buf
        .iter()
        .map(|&c| c as char)
        .collect::<String>()
        .to_uppercase();

    let trimmed = raw.trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
    Ok(trimmed.to_string())
}

/// Helper: writes an 8-byte flat, uppercased + zero-padded.
fn write_flat8<W: Write>(writer: &mut W, flat: &str) -> io::Result<()> {
    let upper = flat.to_uppercase();
    let bytes = upper.as_bytes();

    let mut buf = [0u8; 8];
    for (i, &b) in bytes.iter().take(8).enumerate() {
        buf[i] = b;
    }
    writer.write_all(&buf)?;
    Ok(())
}
