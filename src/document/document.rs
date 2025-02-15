// src/document/document.rs

use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::io::{self, Read, Seek};
use byteorder::{LE, ReadBytesExt}; 
use std::str;

#[derive(Debug, Clone, Copy)]
pub enum ObjType {
    Things,
    Linedefs,
    Sidedefs,
    Vertices,
    Sectors,
}

#[derive(Debug, Clone, Copy)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)] // Add PartialEq
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

#[derive(Debug, Clone, PartialEq)] // Add PartialEq
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

#[derive(Debug, Clone, PartialEq)] // Add PartialEq
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

#[derive(Debug, Clone, PartialEq)] // Add PartialEq
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
            flags: reader.read_i16::<LE>()? as i32, // Corrected to i16
            line_type: reader.read_i16::<LE>()? as i32, // Corrected to i16
            tag: reader.read_i16::<LE>()? as i32,    // Corrected to i16
            right: reader.read_i16::<LE>()? as i32,
            left: reader.read_i16::<LE>()? as i32,
        })
    }
}

#[derive(Debug, Clone, PartialEq)] // Add PartialEq
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

#[derive(Default)]
pub struct Document {
    pub things: Arc<RwLock<Vec<Arc<Thing>>>>,
    pub vertices: Arc<RwLock<Vec<Arc<Vertex>>>>,
    pub sectors: Arc<RwLock<Vec<Arc<Sector>>>>,
    pub sidedefs: Arc<RwLock<Vec<Arc<SideDef>>>>,
    pub linedefs: Arc<RwLock<Vec<Arc<LineDef>>>>,

    pub header_data: Arc<RwLock<Vec<u8>>>,
    pub behavior_data: Arc<RwLock<Vec<u8>>>,
    pub scripts_data: Arc<RwLock<Vec<u8>>>,
    pub basis: Arc<RwLock<Vec<u8>>>, // Is this still needed?

    pub checksum: Arc<RwLock<u32>>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    // Thread-safe accessors (these return clones of the Arcs)
    pub fn vertices(&self) -> Arc<RwLock<Vec<Arc<Vertex>>>> { Arc::clone(&self.vertices) }
    pub fn linedefs(&self) -> Arc<RwLock<Vec<Arc<LineDef>>>> { Arc::clone(&self.linedefs) }
    pub fn sectors(&self) -> Arc<RwLock<Vec<Arc<Sector>>>> { Arc::clone(&self.sectors) }
    pub fn sidedefs(&self) -> Arc<RwLock<Vec<Arc<SideDef>>>> { Arc::clone(&self.sidedefs) }
    pub fn things(&self) -> Arc<RwLock<Vec<Arc<Thing>>>> { Arc::clone(&self.things) }


    pub fn num_objects(&self, obj_type: ObjType) -> usize {
        match obj_type {
            ObjType::Things => self.things.read().len(),
            ObjType::Linedefs => self.linedefs.read().len(),
            ObjType::Sidedefs => self.sidedefs.read().len(),
            ObjType::Vertices => self.vertices.read().len(),
            ObjType::Sectors => self.sectors.read().len(),
        }
    }
    // --- Mutating methods (require write locks) ---

    pub fn add_vertex(&mut self, x: i32, y: i32) -> usize {
        let mut vertices = self.vertices.write();
        let new_vertex = Arc::new(Vertex { raw_x: x, raw_y: y });
        vertices.push(new_vertex);
        vertices.len() - 1 // Return the index of the new vertex
    }

    pub fn move_vertex(&mut self, vertex_id: usize, new_x: i32, new_y: i32) -> Result<(), String> {
        let mut vertices = self.vertices.write();
        if let Some(vertex) = vertices.get_mut(vertex_id) {
            let mut vertex_ref = Arc::make_mut(vertex); // Get a mutable reference to the Vertex
            vertex_ref.raw_x = new_x;
            vertex_ref.raw_y = new_y;
            Ok(())
        } else {
            Err(format!("Vertex with ID {} not found", vertex_id))
        }
    }

    pub fn remove_vertex(&mut self, vertex_id: usize) -> Option<Vertex> {
        let mut vertices = self.vertices.write();
        if vertex_id < vertices.len() {
            let removed = vertices.remove(vertex_id);
    
            // Decrement the vertex indices in each linedef
            {
                let mut linedefs = self.linedefs.write();
                for linedef_arc in linedefs.iter_mut() {
                    let linedef = Arc::make_mut(linedef_arc);
                    if linedef.start > vertex_id {
                        linedef.start -= 1;
                    }
                    if linedef.end > vertex_id {
                        linedef.end -= 1;
                    }
                }
            }
    
            Some(Arc::try_unwrap(removed).unwrap())
        } else {
            None
        }
    }    
    
    pub fn add_linedef(&mut self, start_vertex_id: usize, end_vertex_id: usize, right_side_sector_id: i16, left_side_sector_id: i16) -> usize {
        let mut linedefs = self.linedefs.write();
        let new_linedef = Arc::new(LineDef{
            start: start_vertex_id,
            end: end_vertex_id,
            flags: 0,
            line_type: 0,
            tag: 0,
            right: right_side_sector_id as i32,
            left: left_side_sector_id as i32
        });
        linedefs.push(new_linedef);
        linedefs.len() - 1
    }

    pub fn remove_linedef(&mut self, linedef_id: usize) -> Option<LineDef>{
        let mut linedefs = self.linedefs.write();
        if linedef_id < linedefs.len(){
            Some(Arc::try_unwrap(linedefs.remove(linedef_id)).unwrap()) //Return the linedef
        } else{
            None //Invalid Id
        }
    }

    pub fn add_sector(&mut self, floor_z: i32, ceiling_z: i32, floor_texture: String, ceiling_texture: String, light_level: u8, sector_type: u8) -> usize {
        let mut sectors = self.sectors.write();
        let new_sector = Arc::new(Sector{
            floorh: floor_z,
            ceilh: ceiling_z,
            floor_tex: floor_texture,
            ceil_tex: ceiling_texture,
            light: light_level as i32,
            sector_type: sector_type as i32,
            tag: 0,
        });
        sectors.push(new_sector);
        sectors.len() - 1
    }

    pub fn remove_sector(&mut self, sector_id: usize) -> Option<Sector>{
        let mut sectors = self.sectors.write();
        if sector_id < sectors.len(){
            Some(Arc::try_unwrap(sectors.remove(sector_id)).unwrap()) //Return the linedef
        } else{
            None //Invalid Id
        }
    }

    pub fn add_thing(&mut self, x: i32, y: i32, angle: i32, thing_type: u16, options: u16) -> usize {
        let mut things = self.things.write();
        let new_thing = Arc::new(Thing{
            raw_x: x,
            raw_y: y,
            angle: angle as i32,
            thing_type: thing_type as i32,
            options: options as i32
        });
        things.push(new_thing);
        things.len() - 1
    }

    pub fn remove_thing(&mut self, thing_id: usize) -> Option<Thing>{
        let mut things = self.things.write();
        if thing_id < things.len(){
            Some(Arc::try_unwrap(things.remove(thing_id)).unwrap()) //Return the thing
        } else{
            None //Invalid Id
        }
    }

    pub fn get_level_checksum(&self) -> u32 {
        let mut checksum = 0u32;

        // Parallel checksum computation for all object types
        {
            let things = self.things.read();
            checksum = checksum.wrapping_add(
                things.par_iter()
                    .map(|thing| {
                        let mut crc = 0u32;
                        checksum_thing(&mut crc, thing);
                        crc
                    })
                    .sum::<u32>()
            );
        }

        {
            let vertices = self.vertices.read();
            checksum = checksum.wrapping_add(
                vertices.par_iter()
                    .map(|vertex| {
                        let mut crc = 0u32;
                        checksum_vertex(&mut crc, vertex);
                        crc
                    })
                    .sum::<u32>()
            );
        }

        {
            let sectors = self.sectors.read();
            checksum = checksum.wrapping_add(
                sectors.par_iter()
                    .map(|sector| {
                        let mut crc = 0u32;
                        checksum_sector(&mut crc, sector);
                        crc
                    })
                    .sum::<u32>()
            );
        }

        {
            let linedefs = self.linedefs.read();
            checksum = checksum.wrapping_add(
                linedefs.par_iter()
                    .map(|line| {
                        let mut crc = 0u32;
                        checksum_linedef(&mut crc, line, self);
                        crc
                    })
                    .sum::<u32>()
            );
        }

        *self.checksum.write() = checksum;
        checksum
    }

    pub fn calc_length(&self, line: &LineDef) -> f64 {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        let dx = (start.raw_x - end.raw_x) as f64;
        let dy = (start.raw_y - end.raw_y) as f64;
        dx.hypot(dy)
    }

    pub fn is_zero_length(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        start.raw_x == end.raw_x && start.raw_y == end.raw_y
    }

    pub fn touches_coord(&self, line: &LineDef, tx: i32, ty: i32) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        start.matches(tx, ty) || end.matches(tx, ty)
    }

    pub fn touches_sector(&self, line: &LineDef, sec_num: i32) -> bool {
        let sidedefs = self.sidedefs.read();

        if line.right >= 0 {
            if let Some(sd) = sidedefs.get(line.right as usize) {
                if sd.sector as i32 == sec_num {
                    return true;
                }
            }
        }

        if line.left >= 0 {
            if let Some(sd) = sidedefs.get(line.left as usize) {
                if sd.sector as i32 == sec_num {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_horizontal(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        vertices[line.start].raw_y == vertices[line.end].raw_y
    }

    pub fn is_vertical(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        vertices[line.start].raw_x == vertices[line.end].raw_x
    }

    pub fn is_self_ref(&self, line: &LineDef) -> bool {
        if line.left >= 0 && line.right >= 0 {
            let sidedefs = self.sidedefs.read();
            if let (Some(left), Some(right)) = (
                sidedefs.get(line.left as usize),
                sidedefs.get(line.right as usize)
            ) {
                return left.sector == right.sector;
            }
        }
        false
    }
    pub fn clear(&mut self) {
      self.things.write().clear();
      self.vertices.write().clear();
      self.sectors.write().clear();
      self.sidedefs.write().clear();
      self.linedefs.write().clear();
      self.header_data.write().clear();
      self.behavior_data.write().clear();
      self.scripts_data.write().clear();
      *self.checksum.write() = 0;
    }

    // WAD parsing functionality (remains largely the same, but with corrected type in load_linedefs)
    pub fn load_wad<R: Read + Seek>(&mut self, reader: &mut R) -> io::Result<()> {
      self.clear();

      // Read WAD header
      let mut header = vec![0u8; 12];
      reader.read_exact(&mut header)?;
      *self.header_data.write() = header;

      // Read directory
      let num_lumps = reader.read_i32::<LE>()?;
      let dir_offset = reader.read_i32::<LE>()?;
      reader.seek(io::SeekFrom::Start(dir_offset as u64))?;

      // Process all lumps
      for _ in 0..num_lumps {
          let lump_offset = reader.read_i32::<LE>()?;
          let lump_size = reader.read_i32::<LE>()?;
          let mut name = vec![0u8; 8];
          reader.read_exact(&mut name)?;

          let lump_name = str::from_utf8(&name)
              .unwrap_or("")
              .trim_end_matches('\0');

            match lump_name {
                "THINGS" => self.load_things(reader, lump_offset, lump_size)?,
                "LINEDEFS" => self.load_linedefs(reader, lump_offset, lump_size)?,
                "SIDEDEFS" => self.load_sidedefs(reader, lump_offset, lump_size)?,
                "VERTEXES" => self.load_vertices(reader, lump_offset, lump_size)?,
                "SECTORS" => self.load_sectors(reader, lump_offset, lump_size)?,
                "BEHAVIOR" => self.load_behavior(reader, lump_offset, lump_size)?,  // Assuming you want to store this
                "SCRIPTS" => self.load_scripts(reader, lump_offset, lump_size)?,   //  and this.
                _ => {} // Skip unknown lumps
            }
      }

      Ok(())
    }


    fn load_things<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let num_things = size / 10; // Each thing is 10 bytes

        let mut things = self.things.write();
        things.clear();
        things.reserve(num_things as usize);  // Reserve space for efficiency

        for _ in 0..num_things {
            let thing = Thing::from_wad(reader)?;
            things.push(Arc::new(thing));
        }

        Ok(())
    }


    fn load_vertices<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let num_vertices = size / 4;

        let mut vertices = self.vertices.write();
        vertices.clear();
        vertices.reserve(num_vertices as usize);

        for _ in 0..num_vertices {
            let vertex = Vertex::from_wad(reader)?;
            vertices.push(Arc::new(vertex));
        }
        Ok(())
    }

    fn load_sectors<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let num_sectors = size / 26;

        let mut sectors = self.sectors.write();
        sectors.clear();
        sectors.reserve(num_sectors as usize);

        for _ in 0..num_sectors {
            let sector = Sector::from_wad(reader)?;
            sectors.push(Arc::new(sector));
        }
        Ok(())
    }

    fn load_sidedefs<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let num_sidedefs = size / 30;

        let mut sidedefs = self.sidedefs.write();
        sidedefs.clear();
        sidedefs.reserve(num_sidedefs as usize);

        for _ in 0..num_sidedefs {
            let sidedef = SideDef::from_wad(reader)?;
            sidedefs.push(Arc::new(sidedef));
        }
        Ok(())
    }


    fn load_linedefs<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let num_linedefs = size / 14;

        let mut linedefs = self.linedefs.write();
        linedefs.clear();
        linedefs.reserve(num_linedefs as usize);

        for _ in 0..num_linedefs {
            let linedef = LineDef::from_wad(reader)?;
            linedefs.push(Arc::new(linedef));
        }
        Ok(())
    }

    fn load_behavior<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        *self.behavior_data.write() = data;  // Use interior mutability correctly
        Ok(())
    }

    fn load_scripts<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(io::SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        *self.scripts_data.write() = data; // Use interior mutability correctly
        Ok(())
    }


    // Sector relationships
    pub fn get_sector_from_side(&self, side: &SideDef) -> Option<Arc<Sector>> {
        self.sectors.read().get(side.sector).cloned()
    }


    pub fn get_sector_id(&self, line: &LineDef, side: Side) -> i32 {
        let sidedefs = self.sidedefs.read();
        match side {
            Side::Left =>  {
                if line.left >= 0 {
                    if let Some(sd) = sidedefs.get(line.left as usize) {
                        return sd.sector as i32
                    }
                }
                return -1
            },
            Side::Right => {
                if line.right >= 0 {
                    if let Some(sd) = sidedefs.get(line.right as usize) {
                        return sd.sector as i32
                    }
                }
                return -1
            }
        }
    }

    pub fn get_sector_for_line(&self, line: &LineDef, side: Side) -> Option<Arc<Sector>> {
        let sid = self.get_sector_id(line, side);
        if sid >= 0 {
            self.sectors.read().get(sid as usize).cloned()
        } else {
            None
        }
    }
}


// Helper functions for checksums (these remain the same)
fn add_crc(crc: &mut u32, value: i32) {
    *crc = crc.wrapping_add(value as u32);
}

fn checksum_thing(crc: &mut u32, thing: &Thing) {
    add_crc(crc, thing.raw_x);
    add_crc(crc, thing.raw_y);
    add_crc(crc, thing.angle);
    add_crc(crc, thing.thing_type);
    add_crc(crc, thing.options);
}

fn checksum_vertex(crc: &mut u32, vertex: &Vertex) {
    add_crc(crc, vertex.raw_x);
    add_crc(crc, vertex.raw_y);
}

fn checksum_sector(crc: &mut u32, sector: &Sector) {
    add_crc(crc, sector.floorh);
    add_crc(crc, sector.ceilh);
    add_crc(crc, sector.light);
    add_crc(crc, sector.sector_type);
    add_crc(crc, sector.tag);
    // Hash strings consistently
    for byte in sector.floor_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
    for byte in sector.ceil_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
}

fn checksum_sidedef(crc: &mut u32, sidedef: &SideDef) {
    add_crc(crc, sidedef.x_offset);
    add_crc(crc, sidedef.y_offset);
    for byte in sidedef.upper_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
    for byte in sidedef.lower_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
    for byte in sidedef.mid_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
    add_crc(crc, sidedef.sector as i32);
}


fn checksum_linedef(crc: &mut u32, linedef: &LineDef, doc: &Document) {
    add_crc(crc, linedef.flags);
    add_crc(crc, linedef.line_type);
    add_crc(crc, linedef.tag);
    add_crc(crc, linedef.start as i32);
    add_crc(crc, linedef.end as i32);
    add_crc(crc, linedef.right);
    add_crc(crc, linedef.left);
}
// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_empty_document() {
        let doc = Document::new();
        assert_eq!(doc.num_objects(ObjType::Things), 0);
        assert_eq!(doc.num_objects(ObjType::Vertices), 0);
        assert_eq!(doc.num_objects(ObjType::Sectors), 0);
        assert_eq!(doc.num_objects(ObjType::Linedefs), 0);
        assert_eq!(doc.num_objects(ObjType::Sidedefs), 0);
    }

    #[test]
    fn test_vertex_operations() {
        let mut doc = Document::new();
        let v1 = doc.add_vertex(0, 0);
        let v2 = doc.add_vertex(100, 100);

        let linedef = Arc::new(LineDef {
            start: v1,
            end: v2,
            flags: 0,
            line_type: 0,
            tag: 0,
            right: -1,
            left: -1,
        });

        assert!(!doc.is_zero_length(&linedef));
        assert!(!doc.is_horizontal(&linedef));
        assert!(!doc.is_vertical(&linedef));
        assert!(doc.touches_coord(&linedef, 0, 0));
        assert!(doc.touches_coord(&linedef, 100, 100));
        assert!(!doc.touches_coord(&linedef, 50, 50));
    }
    
    #[test]
    fn test_remove_vertex(){
        let mut doc = Document::new();
        let v1 = doc.add_vertex(0, 0);
        let v2 = doc.add_vertex(100, 100);
        let _ = doc.add_linedef(v1, v2, -1, -1);
        doc.remove_vertex(v1);
        assert_eq!(doc.vertices().read().len(), 1);
    }

    #[test]
    fn test_add_remove_linedef(){
        let mut doc = Document::new();
        let v1 = doc.add_vertex(0, 0);
        let v2 = doc.add_vertex(100, 100);
        let l1 = doc.add_linedef(v1, v2, -1, -1);
        assert_eq!(doc.linedefs().read().len(), 1);
        doc.remove_linedef(l1);
        assert_eq!(doc.linedefs().read().len(), 0);
    }

    #[test]
    fn test_add_remove_sector(){
        let mut doc = Document::new();
        let s1 = doc.add_sector(128,0, "FLOOR4_8".to_string(), "CEIL3_5".to_string(), 0, 0);
        assert_eq!(doc.sectors().read().len(), 1);
        doc.remove_sector(s1);
        assert_eq!(doc.sectors().read().len(), 0);

    }

    #[test]
    fn test_add_remove_thing(){
        let mut doc = Document::new();
        let t1 = doc.add_thing(32,32, 90, 1, 0);
        assert_eq!(doc.things().read().len(), 1);
        doc.remove_thing(t1);
        assert_eq!(doc.things().read().len(), 0);
    }

    #[test]
    fn test_concurrent_access() {
        let doc = Document::new();

        // Use std::thread::scope for scoped threads (requires Rust 1.63+)
        std::thread::scope(|s| {
            // Writer thread
            s.spawn(|| {
                let mut vertices = doc.vertices.write();
                vertices.push(Arc::new(Vertex { raw_x: 0, raw_y: 0 }));
            });

            // Reader thread
            s.spawn(|| {
                let vertices = doc.vertices.read();
                let _len = vertices.len(); // Just access the length
            });
        });
    }
    #[test]
    fn test_wad_loading() {
        // Create a minimal test WAD in memory
        let mut wad_data = vec![];

        // WAD header (12 bytes)
        wad_data.extend_from_slice(b"PWAD");  // WAD type
        wad_data.extend_from_slice(&2i32.to_le_bytes()); // Number of lumps
        wad_data.extend_from_slice(&32i32.to_le_bytes()); // Directory offset

        // First lump (THINGS)
        let things_data = vec![
            0, 0,    // x
            0, 0,    // y
            0, 0,    // angle
            1, 0,    // type
            7, 0,    // options
        ];

        // Second lump (VERTEXES)
        let vertices_data = vec![
            0, 0,    // x
            0, 0,    // y
        ];

        // Directory entries
        let dir_entry1 = vec![
            12i32.to_le_bytes().to_vec(), // offset
            10i32.to_le_bytes().to_vec(), // size
            b"THINGS  ".to_vec()         // name
        ].concat();

        let dir_entry2 = vec![
            22i32.to_le_bytes().to_vec(), // offset
            4i32.to_le_bytes().to_vec(),  // size
            b"VERTEXES".to_vec()        // name
        ].concat();


        wad_data.extend(things_data);
        wad_data.extend(vertices_data);
        wad_data.extend(dir_entry1);
        wad_data.extend(dir_entry2);
        

        let mut cursor = Cursor::new(wad_data);
        let mut doc = Document::new();
        doc.load_wad(&mut cursor).unwrap();

        assert_eq!(doc.num_objects(ObjType::Things), 1);
        assert_eq!(doc.num_objects(ObjType::Vertices), 1);
    }
}
        