// src/map/thing.rs

use std::io::{self, Read, Seek, Write};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A classic DOOM "thing" (map object) in the vanilla 10-byte format.
///
/// Layout (all little-endian):
///
/// ```text
/// offset  field       type / size
/// ------  ----------  -----------
/// 0-1     x           i16
/// 2-3     y           i16
/// 4-5     angle       i16  (0..359)
/// 6-7     doom_type   i16  (thing type number)
/// 8-9     flags       i16  (bitmask)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Thing {
    /// X coordinate in map units (signed 16-bit in WAD).
    pub x: i32,

    /// Y coordinate in map units (signed 16-bit in WAD).
    pub y: i32,

    /// Angle in degrees (0..359).
    pub angle: i32,

    /// DOOM "thing type" (e.g., 3004 for an Imp, 1 for Player 1 start).
    pub doom_type: i32,

    /// Flags bitmask (e.g., 0x0007 for easy/medium/hard).
    pub flags: i32,
}

impl Thing {
    /// Creates a new `Thing` in memory with given field values.
    ///
    /// # Example
    /// ```
    /// let th = Thing::new(100, 200, 90, 3004, 0);
    /// ```
    pub fn new(x: i32, y: i32, angle: i32, doom_type: i32, flags: i32) -> Self {
        Thing {
            x,
            y,
            angle,
            doom_type,
            flags,
        }
    }

    /// Reads a `Thing` from a DOOM WAD (little-endian) in the classic 10-byte format.
    ///
    /// # Format
    /// ```
    /// 0-1 : x (i16)
    /// 2-3 : y (i16)
    /// 4-5 : angle (i16)
    /// 6-7 : doom_type (i16)
    /// 8-9 : flags (i16)
    /// ```
    /// 
    /// # Errors
    /// Returns any I/O error encountered.
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let x = reader.read_i16::<LE>()? as i32;
        let y = reader.read_i16::<LE>()? as i32;
        let angle = reader.read_i16::<LE>()? as i32;
        let doom_type = reader.read_i16::<LE>()? as i32;
        let flags = reader.read_i16::<LE>()? as i32;

        Ok(Thing {
            x,
            y,
            angle,
            doom_type,
            flags,
        })
    }

    /// Writes this thing to a DOOM WAD in the classic 10-byte format.
    ///
    /// # Format
    /// ```
    /// 0-1 : x (i16)
    /// 2-3 : y (i16)
    /// 4-5 : angle (i16)
    /// 6-7 : doom_type (i16)
    /// 8-9 : flags (i16)
    /// ```
    ///
    /// # Errors
    /// Returns any I/O error encountered.
    pub fn to_wad<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        // Cast to i16, assuming fields are in valid range.
        writer.write_i16::<LE>(self.x as i16)?;
        writer.write_i16::<LE>(self.y as i16)?;
        writer.write_i16::<LE>(self.angle as i16)?;
        writer.write_i16::<LE>(self.doom_type as i16)?;
        writer.write_i16::<LE>(self.flags as i16)?;
        Ok(())
    }

    /// A quick helper to set default fields for a newly created Thing.
    /// 
    /// For instance, you could make Player 1 starts or a certain monster.
    ///
    /// # Example
    /// ```
    /// let mut th = Thing::new(0,0,0,0,0);
    /// // Suppose we want a Pinky (ID #3002) at angle 180, normal flags
    /// th.set_defaults(3002, 180, 0);
    /// ```
    pub fn set_defaults(&mut self, doom_type: i32, angle: i32, flags: i32) {
        self.doom_type = doom_type;
        self.angle = angle % 360; // keep it 0..359
        self.flags = flags;
    }
}

/// Utility to rotate an angle (in degrees) by some delta, wrapping at 360.
///
/// Returns the new angle in 0..359 range.
pub fn calc_new_angle(angle: i32, delta_degrees: i32) -> i32 {
    // Negative angles or angles >= 360 are normalized via modulo
    let mut new_angle = angle + delta_degrees;
    // ensure it is positive before mod
    while new_angle < 0 {
        new_angle += 360;
    }
    new_angle % 360
}
