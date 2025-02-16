// src/map/linedef.rs

use std::io::{self, Read, Seek, Write};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A linedef in classic DOOM format.
/// 
/// Fields:
///  - `start` and `end` are indexes into the vertex array (u16 in WAD).
///  - `flags` is a combination of DOOM engine line flags (i16 in WAD).
///  - `line_type` is the "special type" or action (i16 in WAD).
///  - `tag` is the sector tag used by many actions (i16 in WAD).
///  - `right` and `left` are indexes into the sidedef array (i16 in WAD, -1 for "none").
#[derive(Debug, Clone, PartialEq)]
pub struct LineDef {
    pub start: usize,   // DOOM stores this as unsigned 16-bit
    pub end: usize,     // DOOM stores this as unsigned 16-bit
    pub flags: i32,     // DOOM stores this as signed 16-bit
    pub line_type: i32, // DOOM stores this as signed 16-bit
    pub tag: i32,       // DOOM stores this as signed 16-bit
    pub right: i32,     // DOOM stores this as signed 16-bit
    pub left: i32,      // DOOM stores this as signed 16-bit
}

impl LineDef {
    /// Creates a new linedef in memory, with the specified field values.
    /// 
    /// Example:
    /// ```
    /// let ld = LineDef::new(0, 1, 0x0001, 0, 0, 0, -1);
    /// ```
    pub fn new(
        start: usize,
        end: usize,
        flags: i32,
        line_type: i32,
        tag: i32,
        right: i32,
        left: i32,
    ) -> Self {
        Self {
            start,
            end,
            flags,
            line_type,
            tag,
            right,
            left,
        }
    }

    /// Reads a linedef from a WAD (little-endian) in the classic 14-byte DOOM format.
    ///
    /// **Layout**:
    /// ```
    /// offset  field       type
    /// ------  ----------  -----------
    /// 0-1     start       u16
    /// 2-3     end         u16
    /// 4-5     flags       i16
    /// 6-7     line_type   i16
    /// 8-9     tag         i16
    /// 10-11   right_side  i16
    /// 12-13   left_side   i16
    /// ```
    /// 
    /// # Errors
    /// Returns any `io::Error` that happens while reading.
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            start: reader.read_u16::<LE>()? as usize,
            end: reader.read_u16::<LE>()? as usize,
            flags: reader.read_i16::<LE>()? as i32,
            line_type: reader.read_i16::<LE>()? as i32,
            tag: reader.read_i16::<LE>()? as i32,
            right: reader.read_i16::<LE>()? as i32,
            left: reader.read_i16::<LE>()? as i32,
        })
    }

    /// Writes this linedef back out in the classic DOOM 14-byte format.
    ///
    /// **Layout**:
    /// ```
    /// offset  field       type
    /// ------  ----------  -----------
    /// 0-1     start       u16
    /// 2-3     end         u16
    /// 4-5     flags       i16
    /// 6-7     line_type   i16
    /// 8-9     tag         i16
    /// 10-11   right_side  i16
    /// 12-13   left_side   i16
    /// ```
    /// 
    /// # Errors
    /// Returns any `io::Error` that happens while writing.
    pub fn to_wad<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LE>(self.start as u16)?;
        writer.write_u16::<LE>(self.end as u16)?;
        writer.write_i16::<LE>(self.flags as i16)?;
        writer.write_i16::<LE>(self.line_type as i16)?;
        writer.write_i16::<LE>(self.tag as i16)?;
        writer.write_i16::<LE>(self.right as i16)?;
        writer.write_i16::<LE>(self.left as i16)?;
        Ok(())
    }

    /// Returns `true` if the "Two-Sided" bit is set in `flags`.
    ///
    /// In classic DOOM, that bit is typically `0x0004`.
    pub fn is_two_sided(&self) -> bool {
        (self.flags & 0x0004) != 0
    }

    /// Sets or clears the "Two-Sided" bit in `flags`.
    ///
    /// If you want to make the line two-sided, you might also want to remove
    /// the "Blocking" bit (`0x0001`) or other flags. This helper does not do that.
    pub fn set_two_sided(&mut self, two_sided: bool) {
        if two_sided {
            self.flags |= 0x0004;
        } else {
            self.flags &= !0x0004;
        }
    }
}
