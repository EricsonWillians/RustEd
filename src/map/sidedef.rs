// src/map/sidedef.rs

use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct SideDef {
    pub x_offset: i32,
    pub y_offset: i32,
    pub upper_tex: String,
    pub lower_tex: String,
    pub mid_tex: String,
    pub sector: usize,
}

impl SideDef {
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let x_offset = reader.read_i16::<LE>()? as i32;
        let y_offset = reader.read_i16::<LE>()? as i32;

        let mut upper_tex = [0u8; 8];
        let mut lower_tex = [0u8; 8];
        let mut mid_tex = [0u8; 8];
        reader.read_exact(&mut upper_tex)?;
        reader.read_exact(&mut lower_tex)?;
        reader.read_exact(&mut mid_tex)?;

        Ok(SideDef {
            x_offset,
            y_offset,
            upper_tex: String::from_utf8_lossy(&upper_tex).trim_end_matches('\0').to_string(),
            lower_tex: String::from_utf8_lossy(&lower_tex).trim_end_matches('\0').to_string(),
            mid_tex: String::from_utf8_lossy(&mid_tex).trim_end_matches('\0').to_string(),
            sector: reader.read_u16::<LE>()? as usize,
        })
    }
}