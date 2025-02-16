// src/document/document.rs

use crate::map::{LineDef, Sector, SideDef, Thing, Vertex};
use parking_lot::RwLock;
use rayon::prelude::*;
use std::io::{self, Read, Seek, SeekFrom, Cursor};
use std::str;
use std::sync::Arc;
use byteorder::{LE, ReadBytesExt};

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

/// A single lump entry from the WAD directory.
#[derive(Debug, Clone)]
pub struct LumpEntry {
    pub offset: i32,
    pub size: i32,
    pub name: String,
}

/// A grouping of lumps that form a level.
#[derive(Debug, Clone)]
pub struct LevelInfo {
    pub name: String,
    pub lump_indices: Vec<usize>,
}

/// The main document representing a WAD’s contents.
#[derive(Default)]
pub struct Document {
    // Geometry data.
    pub things: Arc<RwLock<Vec<Arc<Thing>>>>,
    pub vertices: Arc<RwLock<Vec<Arc<Vertex>>>>,
    pub sectors: Arc<RwLock<Vec<Arc<Sector>>>>,
    pub sidedefs: Arc<RwLock<Vec<Arc<SideDef>>>>,
    pub linedefs: Arc<RwLock<Vec<Arc<LineDef>>>>,

    // Raw WAD data.
    pub header_data: Arc<RwLock<Vec<u8>>>,
    pub behavior_data: Arc<RwLock<Vec<u8>>>,
    pub scripts_data: Arc<RwLock<Vec<u8>>>,
    pub basis: Arc<RwLock<Vec<u8>>>, // (Retained for compatibility)

    pub checksum: Arc<RwLock<u32>>,

    pub directory: Arc<RwLock<Vec<LumpEntry>>>,
    pub levels: Arc<RwLock<Vec<LevelInfo>>>,
    pub selected_level: Arc<RwLock<Option<String>>>,
    pub wad_data: Arc<RwLock<Option<Vec<u8>>>>,
}

const FILELUMP_SIZE: usize = 16; // 4 bytes (filepos) + 4 bytes (size) + 8 bytes (name)

impl Document {
    /// Create a new empty Document.
    pub fn new() -> Self {
        Self {
            things: Arc::new(RwLock::new(Vec::new())),
            vertices: Arc::new(RwLock::new(Vec::new())),
            sectors: Arc::new(RwLock::new(Vec::new())),
            sidedefs: Arc::new(RwLock::new(Vec::new())),
            linedefs: Arc::new(RwLock::new(Vec::new())),
            header_data: Arc::new(RwLock::new(Vec::new())),
            behavior_data: Arc::new(RwLock::new(Vec::new())),
            scripts_data: Arc::new(RwLock::new(Vec::new())),
            basis: Arc::new(RwLock::new(Vec::new())),
            checksum: Arc::new(RwLock::new(0)),
            directory: Arc::new(RwLock::new(Vec::new())),
            levels: Arc::new(RwLock::new(Vec::new())),
            selected_level: Arc::new(RwLock::new(None)),
            wad_data: Arc::new(RwLock::new(None)),
        }
    }

    // Thread-safe getters.
    pub fn vertices(&self) -> Arc<RwLock<Vec<Arc<Vertex>>>> {
        Arc::clone(&self.vertices)
    }
    pub fn linedefs(&self) -> Arc<RwLock<Vec<Arc<LineDef>>>> {
        Arc::clone(&self.linedefs)
    }
    pub fn sectors(&self) -> Arc<RwLock<Vec<Arc<Sector>>>> {
        Arc::clone(&self.sectors)
    }
    pub fn sidedefs(&self) -> Arc<RwLock<Vec<Arc<SideDef>>>> {
        Arc::clone(&self.sidedefs)
    }
    pub fn things(&self) -> Arc<RwLock<Vec<Arc<Thing>>>> {
        Arc::clone(&self.things)
    }

    pub fn num_objects(&self, obj_type: ObjType) -> usize {
        match obj_type {
            ObjType::Things => self.things.read().len(),
            ObjType::Linedefs => self.linedefs.read().len(),
            ObjType::Sidedefs => self.sidedefs.read().len(),
            ObjType::Vertices => self.vertices.read().len(),
            ObjType::Sectors => self.sectors.read().len(),
        }
    }

    // --- Geometry mutation methods ---

    /// Adds a vertex and returns its index.
    pub fn add_vertex(&mut self, x: i32, y: i32) -> usize {
        let mut vertices = self.vertices.write();
        let new_vertex = Arc::new(Vertex { raw_x: x, raw_y: y });
        vertices.push(new_vertex);
        vertices.len() - 1
    }

    /// Moves a vertex to new coordinates.
    pub fn move_vertex(&mut self, vertex_id: usize, new_x: i32, new_y: i32) -> Result<(), String> {
        let mut vertices = self.vertices.write();
        if let Some(vertex) = vertices.get_mut(vertex_id) {
            let vertex_ref = Arc::make_mut(vertex);
            vertex_ref.raw_x = new_x;
            vertex_ref.raw_y = new_y;
            Ok(())
        } else {
            Err(format!("Vertex with ID {} not found", vertex_id))
        }
    }

    /// Removes a vertex by ID.
    pub fn remove_vertex(&mut self, vertex_id: usize) -> Option<Vertex> {
        let mut vertices = self.vertices.write();
        if vertex_id < vertices.len() {
            let removed = vertices.remove(vertex_id);
            {
                let mut linedefs = self.linedefs.write();
                for linedef_arc in linedefs.iter_mut() {
                    let linedef = Arc::make_mut(linedef_arc);
                    if linedef.start > vertex_id { linedef.start -= 1; }
                    if linedef.end > vertex_id { linedef.end -= 1; }
                }
            }
            Some(Arc::try_unwrap(removed).unwrap())
        } else {
            None
        }
    }

    /// Adds a linedef between two vertices.
    pub fn add_linedef(&mut self, start_vertex_id: usize, end_vertex_id: usize, right_side_sector_id: i16, left_side_sector_id: i16) -> usize {
        let mut linedefs = self.linedefs.write();
        let new_linedef = Arc::new(LineDef {
            start: start_vertex_id,
            end: end_vertex_id,
            flags: 0,
            line_type: 0,
            tag: 0,
            right: right_side_sector_id as i32,
            left: left_side_sector_id as i32,
        });
        linedefs.push(new_linedef);
        linedefs.len() - 1
    }

    /// Removes a linedef.
    pub fn remove_linedef(&mut self, linedef_id: usize) -> Option<LineDef> {
        let mut linedefs = self.linedefs.write();
        if linedef_id < linedefs.len() {
            Some(Arc::try_unwrap(linedefs.remove(linedef_id)).unwrap())
        } else {
            None
        }
    }

    /// Adds a sector.
    pub fn add_sector(&mut self, floor_z: i32, ceiling_z: i32, floor_texture: String, ceiling_texture: String, light_level: u8, sector_type: u8) -> usize {
        let mut sectors = self.sectors.write();
        let new_sector = Arc::new(Sector {
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

    /// Removes a sector.
    pub fn remove_sector(&mut self, sector_id: usize) -> Option<Sector> {
        let mut sectors = self.sectors.write();
        if sector_id < sectors.len() {
            Some(Arc::try_unwrap(sectors.remove(sector_id)).unwrap())
        } else {
            None
        }
    }

    /// Adds a thing.
    pub fn add_thing(&mut self, x: i32, y: i32, angle: i32, thing_type: u16, options: u16) -> usize {
        let mut things = self.things.write();
        let new_thing = Arc::new(Thing {
            raw_x: x,
            raw_y: y,
            angle: angle as i32,
            thing_type: thing_type as i32,
            options: options as i32,
        });
        things.push(new_thing);
        things.len() - 1
    }

    /// Removes a thing.
    pub fn remove_thing(&mut self, thing_id: usize) -> Option<Thing> {
        let mut things = self.things.write();
        if thing_id < things.len() {
            Some(Arc::try_unwrap(things.remove(thing_id)).unwrap())
        } else {
            None
        }
    }

    /// Computes a checksum over all geometry.
    pub fn get_level_checksum(&self) -> u32 {
        let mut checksum = 0u32;
        {
            let things = self.things.read();
            checksum = checksum.wrapping_add(
                things.par_iter().map(|thing| {
                    let mut crc = 0u32;
                    checksum_thing(&mut crc, thing);
                    crc
                }).sum::<u32>()
            );
        }
        {
            let vertices = self.vertices.read();
            checksum = checksum.wrapping_add(
                vertices.par_iter().map(|vertex| {
                    let mut crc = 0u32;
                    checksum_vertex(&mut crc, vertex);
                    crc
                }).sum::<u32>()
            );
        }
        {
            let sectors = self.sectors.read();
            checksum = checksum.wrapping_add(
                sectors.par_iter().map(|sector| {
                    let mut crc = 0u32;
                    checksum_sector(&mut crc, sector);
                    crc
                }).sum::<u32>()
            );
        }
        {
            let linedefs = self.linedefs.read();
            checksum = checksum.wrapping_add(
                linedefs.par_iter().map(|line| {
                    let mut crc = 0u32;
                    checksum_linedef(&mut crc, line, self);
                    crc
                }).sum::<u32>()
            );
        }
        *self.checksum.write() = checksum;
        checksum
    }

    /// Computes the length of a linedef.
    pub fn calc_length(&self, line: &LineDef) -> f64 {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        let dx = (start.raw_x - end.raw_x) as f64;
        let dy = (start.raw_y - end.raw_y) as f64;
        dx.hypot(dy)
    }

    /// Returns true if the linedef has zero length.
    pub fn is_zero_length(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        start.raw_x == end.raw_x && start.raw_y == end.raw_y
    }

    /// Returns true if the linedef touches the given coordinate.
    pub fn touches_coord(&self, line: &LineDef, tx: i32, ty: i32) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        start.matches(tx, ty) || end.matches(tx, ty)
    }

    /// Returns true if the linedef touches the given sector.
    pub fn touches_sector(&self, line: &LineDef, sec_num: i32) -> bool {
        let sidedefs = self.sidedefs.read();
        if line.right >= 0 {
            if let Some(sd) = sidedefs.get(line.right as usize) {
                if sd.sector as i32 == sec_num { return true; }
            }
        }
        if line.left >= 0 {
            if let Some(sd) = sidedefs.get(line.left as usize) {
                if sd.sector as i32 == sec_num { return true; }
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

    /// Returns true if the linedef is horizontal.
    pub fn is_horizontal(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        vertices[line.start].raw_y == vertices[line.end].raw_y
    }

    /// Returns true if the linedef is vertical.
    pub fn is_vertical(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        vertices[line.start].raw_x == vertices[line.end].raw_x
    }

    /// Returns true if the linedef’s sidedefs reference the same sector.
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

    /// Clears all geometry (but not directory/level data).
    pub fn clear_geometry(&mut self) {
        self.things.write().clear();
        self.vertices.write().clear();
        self.sectors.write().clear();
        self.sidedefs.write().clear();
        self.linedefs.write().clear();
        self.behavior_data.write().clear();
        self.scripts_data.write().clear();
        *self.checksum.write() = 0;
    }

    // --- WAD Loading and Level Selection ---

    /// Loads a WAD file from the given reader.
    /// In addition to reading header and directory and grouping levels,
    /// this function reads the entire file into memory (wad_data) so that
    /// levels can be reloaded on demand.
    pub fn load_wad<R: Read + Seek>(&mut self, reader: &mut R) -> io::Result<()> {
        self.clear();
        self.clear_geometry();

        // Determine total file size.
        let total_size = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;

        // Read entire file into memory.
        let mut full_data = Vec::with_capacity(total_size as usize);
        reader.read_to_end(&mut full_data)?;
        *self.wad_data.write() = Some(full_data.clone());
        // Create a new cursor over the full data.
        let mut cursor = Cursor::new(full_data);

        // --- Read Header ---
        let mut header_buf = [0u8; 12];
        cursor.read_exact(&mut header_buf)?;
        let ident = &header_buf[0..4];
        if ident != b"IWAD" && ident != b"PWAD" {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                format!("Invalid WAD identifier: {}", String::from_utf8_lossy(ident))));
        }
        *self.header_data.write() = header_buf.to_vec();
        let num_lumps = (&header_buf[4..8]).read_i32::<LE>()?;
        let infotableofs = (&header_buf[8..12]).read_i32::<LE>()?;
        if (infotableofs as u64) > total_size {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Directory offset exceeds total file size"));
        }

        // --- Read Directory ---
        let dir_size = (num_lumps as usize) * FILELUMP_SIZE;
        cursor.seek(SeekFrom::Start(infotableofs as u64))?;
        let mut dir_buf = vec![0u8; dir_size];
        cursor.read_exact(&mut dir_buf)?;
        let mut directory = Vec::with_capacity(num_lumps as usize);
        for i in 0..(num_lumps as usize) {
            let offset = i * FILELUMP_SIZE;
            let lump_offset = (&dir_buf[offset..offset+4]).read_i32::<LE>()?;
            let lump_size = (&dir_buf[offset+4..offset+8]).read_i32::<LE>()?;
            let name_bytes = &dir_buf[offset+8..offset+16];
            let lump_name = str::from_utf8(name_bytes).unwrap_or("").trim_end_matches('\0').to_string();
            if lump_offset < 0 || lump_size < 0 ||
               (lump_offset as u64) > total_size ||
               (lump_offset as u64) + (lump_size as u64) > total_size {
                eprintln!("WARNING: Lump '{}' has invalid offset/size ({}+{} > {})",
                          lump_name, lump_offset, lump_size, total_size);
                continue;
            }
            directory.push(LumpEntry { offset: lump_offset, size: lump_size, name: lump_name });
        }
        *self.directory.write() = directory;

        // --- Group Lumps into Levels ---
        let levels = Self::group_levels(&self.directory.read());
        *self.levels.write() = levels;

        // --- Automatically load the first level (if available) ---
        let first_level_name = {
            let levels = self.levels.read();
            levels.get(0).map(|lvl| lvl.name.clone())
        };
        if let Some(level_name) = first_level_name {
            let wad_data_clone = {
                let wad = self.wad_data.read();
                wad.as_ref().unwrap().clone()
            };
            self.load_level(&level_name, &mut Cursor::new(wad_data_clone))?;
            *self.selected_level.write() = Some(level_name);
        }
        Ok(())
    }

    /// Groups lumps from the directory into levels based on markers (e.g. "MAP01" or "E1M1").
    fn group_levels(directory: &Vec<LumpEntry>) -> Vec<LevelInfo> {
        let mut levels = Vec::new();
        let mut current_level: Option<LevelInfo> = None;
        for (i, entry) in directory.iter().enumerate() {
            if Self::is_level_marker(&entry.name) {
                if let Some(lvl) = current_level.take() {
                    levels.push(lvl);
                }
                current_level = Some(LevelInfo {
                    name: entry.name.clone(),
                    lump_indices: vec![i],
                });
            } else if let Some(ref mut lvl) = current_level {
                lvl.lump_indices.push(i);
            }
        }
        if let Some(lvl) = current_level {
            levels.push(lvl);
        }
        levels
    }

    /// Returns true if the lump name indicates a level marker.
    fn is_level_marker(name: &str) -> bool {
        let name = name.trim();
        let upper = name.to_uppercase();
        if upper.starts_with("MAP") && upper.len() >= 5 {
            upper.chars().skip(3).take(2).all(|c| c.is_digit(10))
        } else if upper.len() == 4 && upper.starts_with("E") && upper.chars().nth(2) == Some('M') {
            upper.chars().nth(1).map_or(false, |c| c.is_digit(10)) &&
            upper.chars().nth(3).map_or(false, |c| c.is_digit(10))
        } else {
            false
        }
    }

    /// Loads the geometry for a specific level (by its marker, e.g. "MAP01").
    pub fn load_level<R: Read + Seek>(&mut self, level_name: &str, reader: &mut R) -> io::Result<()> {
        self.clear_geometry();
        let level_info_opt = {
            let levels = self.levels.read();
            levels.iter().find(|lvl| lvl.name.eq_ignore_ascii_case(level_name)).cloned()
        };
        if let Some(level_info) = level_info_opt {
            let directory = self.directory.read();
            for &index in &level_info.lump_indices {
                let entry = &directory[index];
                match entry.name.as_str() {
                    "THINGS" => { self.load_things(reader, entry.offset, entry.size)?; },
                    "VERTEXES" => { self.load_vertices(reader, entry.offset, entry.size)?; },
                    "SECTORS" => { self.load_sectors(reader, entry.offset, entry.size)?; },
                    "SIDEDEFS" => { self.load_sidedefs(reader, entry.offset, entry.size)?; },
                    "LINEDEFS" => { self.load_linedefs(reader, entry.offset, entry.size)?; },
                    "BEHAVIOR" => { self.load_behavior(reader, entry.offset, entry.size)?; },
                    "SCRIPTS" => { self.load_scripts(reader, entry.offset, entry.size)?; },
                    _ => { /* Ignore other lumps, including the level marker itself */ }
                }
            }
            *self.selected_level.write() = Some(level_info.name.clone());
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Level not found"))
        }
    }

    /// Returns a list of available level markers.
    pub fn available_levels(&self) -> Vec<String> {
        self.levels.read().iter().map(|lvl| lvl.name.clone()).collect()
    }

    // --- Lump-loading helper functions ---

    fn load_things<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let num_things = size / 10;
        let mut things = self.things.write();
        things.clear();
        things.reserve(num_things as usize);
        for _ in 0..num_things {
            let thing = Thing::from_wad(reader)?;
            things.push(Arc::new(thing));
        }
        Ok(())
    }

    fn load_vertices<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(SeekFrom::Start(offset as u64))?;
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
        reader.seek(SeekFrom::Start(offset as u64))?;
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
        reader.seek(SeekFrom::Start(offset as u64))?;
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
        reader.seek(SeekFrom::Start(offset as u64))?;
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
        reader.seek(SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        *self.behavior_data.write() = data;
        Ok(())
    }

    fn load_scripts<R: Read + Seek>(&self, reader: &mut R, offset: i32, size: i32) -> io::Result<()> {
        reader.seek(SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        *self.scripts_data.write() = data;
        Ok(())
    }

    // --- Sector relationships and geometry helper methods ---

    pub fn get_sector_from_side(&self, side: &SideDef) -> Option<Arc<Sector>> {
        self.sectors.read().get(side.sector).cloned()
    }

    pub fn get_sector_id(&self, line: &LineDef, side: Side) -> i32 {
        let sidedefs = self.sidedefs.read();
        match side {
            Side::Left => {
                if line.left >= 0 {
                    if let Some(sd) = sidedefs.get(line.left as usize) {
                        return sd.sector as i32;
                    }
                }
                -1
            }
            Side::Right => {
                if line.right >= 0 {
                    if let Some(sd) = sidedefs.get(line.right as usize) {
                        return sd.sector as i32;
                    }
                }
                -1
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

// --- Checksum helper functions ---

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{LineDef, Sector, Thing, Vertex};
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
    fn test_remove_vertex() {
        let mut doc = Document::new();
        let v1 = doc.add_vertex(0, 0);
        let _ = doc.add_vertex(100, 100);
        let _ = doc.add_linedef(v1, 1, -1, -1);
        doc.remove_vertex(v1);
        assert_eq!(doc.vertices().read().len(), 1);
    }

    #[test]
    fn test_add_remove_linedef() {
        let mut doc = Document::new();
        let v1 = doc.add_vertex(0, 0);
        let v2 = doc.add_vertex(100, 100);
        let l1 = doc.add_linedef(v1, v2, -1, -1);
        assert_eq!(doc.linedefs().read().len(), 1);
        doc.remove_linedef(l1);
        assert_eq!(doc.linedefs().read().len(), 0);
    }

    #[test]
    fn test_add_remove_sector() {
        let mut doc = Document::new();
        let s1 = doc.add_sector(128, 0, "FLOOR4_8".to_string(), "CEIL3_5".to_string(), 0, 0);
        assert_eq!(doc.sectors().read().len(), 1);
        doc.remove_sector(s1);
        assert_eq!(doc.sectors().read().len(), 0);
    }

    #[test]
    fn test_add_remove_thing() {
        let mut doc = Document::new();
        let t1 = doc.add_thing(32, 32, 90, 1, 0);
        assert_eq!(doc.things().read().len(), 1);
        doc.remove_thing(t1);
        assert_eq!(doc.things().read().len(), 0);
    }

    #[test]
    fn test_concurrent_access() {
        let doc = Document::new();
        std::thread::scope(|s| {
            s.spawn(|| {
                let mut vertices = doc.vertices.write();
                vertices.push(Arc::new(Vertex { raw_x: 0, raw_y: 0 }));
            });
            s.spawn(|| {
                let vertices = doc.vertices.read();
                let _ = vertices.len();
            });
        });
    }

    #[test]
    fn test_wad_loading() {
        // Create a minimal test WAD in memory with a single level.
        let mut wad_data = vec![];
        // WAD header: "PWAD", 2 lumps, directory offset at byte 32.
        wad_data.extend_from_slice(b"PWAD");
        wad_data.extend_from_slice(&2i32.to_le_bytes());
        wad_data.extend_from_slice(&32i32.to_le_bytes());
        // First lump: level marker "MAP01" (8 bytes)
        let level_marker = b"MAP01\0\0\0";
        // Second lump: VERTEXES lump (4 bytes of data)
        let vertices_data = vec![0, 0, 0, 0];
        // Directory entries:
        // Lump 0: level marker at offset 12, size = 8, name "MAP01"
        let dir_entry0 = [
            12i32.to_le_bytes().to_vec(),
            8i32.to_le_bytes().to_vec(),
            b"MAP01   ".to_vec(),
        ].concat();
        // Lump 1: VERTEXES at offset 20, size = 4, name "VERTEXES"
        let dir_entry1 = [
            20i32.to_le_bytes().to_vec(),
            4i32.to_le_bytes().to_vec(),
            b"VERTEXES".to_vec(),
        ].concat();
        // Build the WAD: header, level marker lump, vertices lump, then directory.
        wad_data.extend_from_slice(level_marker);
        wad_data.extend_from_slice(&vertices_data);
        wad_data.extend(dir_entry0);
        wad_data.extend(dir_entry1);
        let mut cursor = Cursor::new(wad_data);
        let mut doc = Document::new();
        doc.load_wad(&mut cursor).unwrap();
        // We expect that the automatic level loading picked "MAP01" and loaded the VERTEXES lump.
        assert_eq!(doc.num_objects(ObjType::Vertices), 1);
        // Also, available_levels() should list "MAP01".
        let levels = doc.available_levels();
        assert!(levels.contains(&"MAP01".to_string()));
    }
}
