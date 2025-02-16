// src/map/linedef.rs
use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct LineDef {
    pub start: usize,
    pub end: usize,
    pub flags: i32,
    pub line_type: i32,
    pub tag: i32,
    pub right: i32,
    pub left: i32,
}

impl LineDef {
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Ok(LineDef {
            start: reader.read_u16::<LE>()? as usize,
            end: reader.read_u16::<LE>()? as usize,
            flags: reader.read_i16::<LE>()? as i32,
            line_type: reader.read_i16::<LE>()? as i32,
            tag: reader.read_i16::<LE>()? as i32,
            right: reader.read_i16::<LE>()? as i32,
            left: reader.read_i16::<LE>()? as i32,
        })
    }
}