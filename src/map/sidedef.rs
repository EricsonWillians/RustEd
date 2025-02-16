// src/map/sidedef.rs

use std::io::{self, Read, Seek, Write};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A sidedef in classic DOOM format (30 bytes total).
///
/// Layout (all little-endian):
///
/// ```text
/// offset  field       type / size
/// ------  ----------  ------------
///  0-1    x_offset    i16
///  2-3    y_offset    i16
///  4-11   upper_tex   [u8; 8]
/// 12-19   lower_tex   [u8; 8]
/// 20-27   mid_tex     [u8; 8]
/// 28-29   sector      i16  (index into sector list)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SideDef {
    /// Horizontal texture offset (signed 16-bit in a WAD).
    pub x_offset: i32,

    /// Vertical texture offset (signed 16-bit in a WAD).
    pub y_offset: i32,

    /// Upper texture name, up to 8 chars (trimmed/padded in WAD).
    pub upper_tex: String,

    /// Lower texture name, up to 8 chars (trimmed/padded in WAD).
    pub lower_tex: String,

    /// Middle (a.k.a. "mid" or "normal") texture name, up to 8 chars.
    pub mid_tex: String,

    /// Sector index for this sidedef (in DOOM, stored as i16).
    pub sector: i32,
}

impl SideDef {
    /// Creates a new sidedef in memory, with the specified field values.
    ///
    /// You can pass empty strings for the texture names if you want them
    /// blank, or something like `"-"` if you use special placeholders.
    ///
    /// # Example
    /// ```
    /// let sd = SideDef::new(
    ///     0,  // x_offset
    ///     0,  // y_offset
    ///     "UPPER".to_string(),
    ///     "LOWER".to_string(),
    ///     "MID".to_string(),
    ///     0,  // sector index
    /// );
    /// ```
    pub fn new(
        x_offset: i32,
        y_offset: i32,
        upper_tex: String,
        lower_tex: String,
        mid_tex: String,
        sector: i32,
    ) -> Self {
        SideDef {
            x_offset,
            y_offset,
            upper_tex,
            lower_tex,
            mid_tex,
            sector,
        }
    }

    /// Reads a `SideDef` from a DOOM WAD in the 30-byte classic format.
    ///
    /// # Format (little-endian):
    /// ```text
    /// 0-1     x_offset  (i16)
    /// 2-3     y_offset  (i16)
    /// 4-11    upper_tex (8 bytes, often ASCII)
    /// 12-19   lower_tex (8 bytes)
    /// 20-27   mid_tex   (8 bytes)
    /// 28-29   sector    (i16)
    /// ```
    /// Textures are typically uppercase, zero-padded. We trim trailing zeros
    /// (and spaces) for convenience.
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let x_offset = reader.read_i16::<LE>()? as i32;
        let y_offset = reader.read_i16::<LE>()? as i32;

        let upper_tex_raw = read_tex8(reader)?;
        let lower_tex_raw = read_tex8(reader)?;
        let mid_tex_raw   = read_tex8(reader)?;

        let sector = reader.read_i16::<LE>()? as i32;

        Ok(SideDef {
            x_offset,
            y_offset,
            upper_tex: upper_tex_raw,
            lower_tex: lower_tex_raw,
            mid_tex:   mid_tex_raw,
            sector,
        })
    }

    /// Writes this sidedef to a DOOM WAD in the 30-byte classic format.
    ///
    /// # Format (little-endian):
    /// ```text
    /// 0-1     x_offset  (i16)
    /// 2-3     y_offset  (i16)
    /// 4-11    upper_tex (8 bytes, zero-padded)
    /// 12-19   lower_tex (8 bytes)
    /// 20-27   mid_tex   (8 bytes)
    /// 28-29   sector    (i16)
    /// ```
    /// For texture strings, we uppercase them, clip or pad to 8 bytes,
    /// then write them out.
    pub fn to_wad<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_i16::<LE>(self.x_offset as i16)?;
        writer.write_i16::<LE>(self.y_offset as i16)?;

        write_tex8(writer, &self.upper_tex)?;
        write_tex8(writer, &self.lower_tex)?;
        write_tex8(writer, &self.mid_tex)?;

        writer.write_i16::<LE>(self.sector as i16)?;
        Ok(())
    }

    /// Sets common default fields for a newly created sidedef.
    /// This is purely a convenience method, akin to Eureka’s
    /// `SetDefaults`, so you can call it after creation.
    ///
    /// `default_tex` is the default wall texture for your project,
    /// e.g. `"STARTAN2"`. `is_two_sided` can help decide whether
    /// to fill `upper_tex` / `lower_tex` or just `mid_tex`.
    ///
    /// # Example
    /// ```
    /// let mut sd = SideDef::new(0,0,"","","",0);
    /// sd.set_defaults("STARTAN2", true);
    /// ```
    pub fn set_defaults(&mut self, default_tex: &str, is_two_sided: bool) {
        self.x_offset = 0;
        self.y_offset = 0;
        if is_two_sided {
            // For two-sided lines, often we have blank or "-" mid, and
            // fill upper/lower with some default. Adjust to your logic.
            self.upper_tex = default_tex.to_uppercase();
            self.lower_tex = default_tex.to_uppercase();
            self.mid_tex   = "-".to_string();
        } else {
            // For one-sided lines, the mid texture typically is visible.
            self.upper_tex = "-".to_string();
            self.lower_tex = "-".to_string();
            self.mid_tex   = default_tex.to_uppercase();
        }
        // In DOOM, sector must be valid. 0 is often a safe default if
        // you know you have at least 1 sector in your map.
        self.sector = 0;
    }
}

/// Reads exactly 8 bytes of texture name, trimming trailing `\0` and spaces.
fn read_tex8<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;

    // Convert to ASCII/UTF-8 lossily, uppercase, then trim trailing `\0`/space.
    let raw = buf
        .iter()
        .map(|&c| c as char)
        .collect::<String>()
        .to_uppercase();

    let trimmed = raw.trim_end_matches(|ch: char| ch == '\0' || ch.is_ascii_whitespace());
    Ok(trimmed.to_string())
}

/// Writes an 8-byte texture name, uppercase, zero-padded if shorter,
/// truncated if longer than 8.
fn write_tex8<W: Write>(writer: &mut W, tex: &str) -> io::Result<()> {
    let upper = tex.to_uppercase();
    let bytes = upper.as_bytes();

    let mut buf = [0u8; 8];
    for (i, &b) in bytes.iter().take(8).enumerate() {
        buf[i] = b;
    }

    writer.write_all(&buf)?;
    Ok(())
}
