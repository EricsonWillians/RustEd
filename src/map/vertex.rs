// src/map/vertex.rs

use std::io::{self, Read, Seek, Write};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

/// A vertex in classic DOOM format: 4 bytes total.
///
/// In a WAD's `VERTEXES` lump, each vertex is:
/// 
/// ```text
/// offset  field   type
/// ------  ------  -----
/// 0-1     x       i16
/// 2-3     y       i16
/// ```
/// 
/// Each coordinate is in map units.
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    /// X coordinate in the map (signed 16-bit in the WAD).
    pub x: i32,

    /// Y coordinate in the map (signed 16-bit in the WAD).
    pub y: i32,
}

impl Vertex {
    /// Creates a new vertex purely in memory.
    ///
    /// # Example
    /// ```
    /// let v = Vertex::new(128, -64);
    /// ```
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Reads a single vertex from a DOOM WAD vertex lump (4 bytes):
    ///
    /// ```text
    /// 0-1  x (i16)
    /// 2-3  y (i16)
    /// ```
    /// 
    /// # Errors
    /// Returns `io::Error` if reading fails.
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let x = reader.read_i16::<LE>()? as i32;
        let y = reader.read_i16::<LE>()? as i32;
        Ok(Vertex { x, y })
    }

    /// Writes this vertex to a DOOM WAD vertex lump (4 bytes).
    ///
    /// ```text
    /// 0-1  x (i16)
    /// 2-3  y (i16)
    /// ```
    /// 
    /// # Errors
    /// Returns `io::Error` if writing fails.
    pub fn to_wad<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_i16::<LE>(self.x as i16)?;
        writer.write_i16::<LE>(self.y as i16)?;
        Ok(())
    }

    /// Computes the squared distance (in map units^2) between
    /// this vertex and another.
    ///
    /// # Example
    /// ```
    /// let v1 = Vertex::new(0, 0);
    /// let v2 = Vertex::new(3, 4);
    /// assert_eq!(v1.dist_squared(&v2), 25);
    /// ```
    pub fn dist_squared(&self, other: &Vertex) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// (Optional) sets defaults, for instance if you want to
    /// place it at some grid or origin in your editor.
    pub fn set_defaults(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}
