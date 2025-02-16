// src/map/vertex.rs
use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub raw_x: i32,
    pub raw_y: i32,
}

impl Vertex {
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Ok(Vertex {
            raw_x: reader.read_i16::<LE>()? as i32,
            raw_y: reader.read_i16::<LE>()? as i32,
        })
    }

    pub fn matches(&self, tx: i32, ty: i32) -> bool {
        self.raw_x == tx && self.raw_y == ty
    }
}