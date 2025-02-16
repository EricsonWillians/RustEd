// src/map/sector.rs
use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt};

#[derive(Debug, Clone, PartialEq)]
pub struct Sector {
    pub floorh: i32,
    pub ceilh: i32,
    pub floor_tex: String,
    pub ceil_tex: String,
    pub light: i32,
    pub sector_type: i32,
    pub tag: i32,
}

impl Sector {
    pub fn from_wad<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let floorh = reader.read_i16::<LE>()? as i32;
        let ceilh = reader.read_i16::<LE>()? as i32;

        let mut floor_tex = [0u8; 8];
        reader.read_exact(&mut floor_tex)?;
        let mut ceil_tex = [0u8; 8];
        reader.read_exact(&mut ceil_tex)?;

        Ok(Sector {
            floorh,
            ceilh,
            floor_tex: String::from_utf8_lossy(&floor_tex).trim_end_matches('\0').to_string(),
            ceil_tex: String::from_utf8_lossy(&ceil_tex).trim_end_matches('\0').to_string(),
            light: reader.read_i16::<LE>()? as i32,
            sector_type: reader.read_i16::<LE>()? as i32,
            tag: reader.read_i16::<LE>()? as i32,
        })
    }
}