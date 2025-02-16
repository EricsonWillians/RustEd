// src/map/thing.rs
use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct Thing {
    pub raw_x: i32,
    pub raw_y: i32,
    pub angle: i32,
    pub thing_type: i32,
    pub options: i32,
}

impl Thing {
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Ok(Thing {
            raw_x: reader.read_i16::<LE>()? as i32,
            raw_y: reader.read_i16::<LE>()? as i32,
            angle: reader.read_i16::<LE>()? as i32,
            thing_type: reader.read_i16::<LE>()? as i32,
            options: reader.read_i16::<LE>()? as i32,
        })
    }
}