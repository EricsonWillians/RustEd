// src/document/document.rs

use crate::map::{LineDef, Sector, SideDef, Thing, Vertex};
use crate::bsp::BspLevel;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::io::{self, Read, Seek, SeekFrom, Cursor};
use std::str;
use std::sync::Arc;
use byteorder::{LE, ReadBytesExt};
use log::{error, info};

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

    pub map_name: String,

    pub dirty: bool,
}


#[derive(Debug, Clone)]
pub enum LoadProgress {
    Started,
    ChunkLoaded(u8), // Percentage complete
    Completed,
}

// Helper trait for async loading
#[async_trait::async_trait]
pub trait AsyncLoadable: Sized {
    async fn load_async<R: Read + Seek + Send>(reader: &mut R) -> io::Result<Self>;
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
            map_name: String::new(),
            dirty: false,
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
        let new_vertex = Arc::new(Vertex { x, y });
        vertices.push(new_vertex);
        vertices.len() - 1
    }

    /// Moves a vertex to new coordinates.
    pub fn move_vertex(&mut self, vertex_id: usize, new_x: i32, new_y: i32) -> Result<(), String> {
        let mut vertices = self.vertices.write();
        if let Some(vertex) = vertices.get_mut(vertex_id) {
            let vertex_ref = Arc::make_mut(vertex);
            vertex_ref.x = new_x;
            vertex_ref.y = new_y;
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
                // Adjust all linedefs that reference any vertex > vertex_id.
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
            // Arc::try_unwrap(...) returns the underlying Vertex if we
            // were the only Arc holder. We assume we are.
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
            floor_height: floor_z,
            ceiling_height: ceiling_z,
            floor_tex: floor_texture,
            ceiling_tex: ceiling_texture,
            light: light_level as i32,
            r#type: sector_type as i32,
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
    pub fn add_thing(&mut self, x: i32, y: i32, angle: i32, doom_type: u16, flags: u16) -> usize {
        let mut things = self.things.write();
        let new_thing = Arc::new(Thing {
            x,
            y,
            angle,
            doom_type: doom_type as i32,
            flags: flags as i32,
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
        let dx = (start.x - end.x) as f64;
        let dy = (start.y - end.y) as f64;
        dx.hypot(dy)
    }

    /// Returns true if the linedef has zero length.
    pub fn is_zero_length(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        start.x == end.x && start.y == end.y
    }

    /// Returns true if the linedef touches the given coordinate.
    pub fn touches_coord(&self, line: &LineDef, tx: i32, ty: i32) -> bool {
        let vertices = self.vertices.read();
        let start = &vertices[line.start];
        let end = &vertices[line.end];
        (start.x == tx && start.y == ty) || (end.x == tx && end.y == ty)
    }

    /// Returns true if the linedef touches the given sector.
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
        vertices[line.start].y == vertices[line.end].y
    }

    /// Returns true if the linedef is vertical.
    pub fn is_vertical(&self, line: &LineDef) -> bool {
        let vertices = self.vertices.read();
        vertices[line.start].x == vertices[line.end].x
    }

    /// Returns true if the linedef’s sidedefs reference the same sector.
    pub fn is_self_ref(&self, line: &LineDef) -> bool {
        if line.left >= 0 && line.right >= 0 {
            let sidedefs = self.sidedefs.read();
            if let (Some(left_sd), Some(right_sd)) = (
                sidedefs.get(line.left as usize),
                sidedefs.get(line.right as usize)
            ) {
                return left_sd.sector == right_sd.sector;
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
    /// Reads the entire file into memory (wad_data) so that levels can be reloaded on demand.
    pub fn load_wad<R: Read + Seek>(&mut self, reader: &mut R) -> io::Result<()> {
        // Create a new runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        runtime.block_on(async {
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
                self.load_level_async(&level_name, &mut Cursor::new(wad_data_clone)).await?;
                *self.selected_level.write() = Some(level_name);
            }
            Ok(())
        })
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
            upper.chars().skip(3).take(2).all(|c| c.is_ascii_digit())
        } else if upper.len() == 4 && upper.starts_with('E') && upper.chars().nth(2) == Some('M') {
            upper.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) &&
            upper.chars().nth(3).map_or(false, |c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Computes the bounding box of the level from its vertices.
    pub fn bounding_box(&self) -> Option<egui::Rect> {
        let vertices = self.vertices.read();
        if vertices.is_empty() {
            return None;
        }
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for vertex in vertices.iter() {
            let x = vertex.x as f32;
            let y = vertex.y as f32;
            if x < min_x { min_x = x; }
            if y < min_y { min_y = y; }
            if x > max_x { max_x = x; }
            if y > max_y { max_y = y; }
        }
        Some(egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y)))
    }

    // --- WAD Lump Loading Methods ---

    async fn load_vertices_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        if size % 4 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                "VERTEXES lump size is not a multiple of 4"));
        }

        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let vertices_data = tokio::task::spawn_blocking(move || {
            let mut vertices = Vec::new();
            let mut cursor = Cursor::new(buffer);
            let num_vertices = size / 4;
            
            for _ in 0..num_vertices {
                if let Ok(vertex) = Vertex::from_wad(&mut cursor) {
                    vertices.push(Arc::new(vertex));
                }
            }
            vertices
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut vertices = self.vertices.write();
        *vertices = vertices_data;
        
        Ok(())
    }

    async fn load_sectors_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        if size % 26 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                "SECTORS lump size is not a multiple of 26"));
        }

        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let sectors_data = tokio::task::spawn_blocking(move || {
            let mut sectors = Vec::new();
            let mut cursor = Cursor::new(buffer);
            let num_sectors = size / 26;
            
            for _ in 0..num_sectors {
                if let Ok(sector) = Sector::from_wad(&mut cursor) {
                    sectors.push(Arc::new(sector));
                }
            }
            sectors
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut sectors = self.sectors.write();
        *sectors = sectors_data;
        
        Ok(())
    }

    async fn load_sidedefs_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        if size % 30 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                "SIDEDEFS lump size is not a multiple of 30"));
        }

        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let sidedefs_data = tokio::task::spawn_blocking(move || {
            let mut sidedefs = Vec::new();
            let mut cursor = Cursor::new(buffer);
            let num_sidedefs = size / 30;
            
            for _ in 0..num_sidedefs {
                if let Ok(sidedef) = SideDef::from_wad(&mut cursor) {
                    sidedefs.push(Arc::new(sidedef));
                }
            }
            sidedefs
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut sidedefs = self.sidedefs.write();
        *sidedefs = sidedefs_data;
        
        Ok(())
    }

    async fn load_linedefs_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        if size % 14 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                "LINEDEFS lump size is not a multiple of 14"));
        }

        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let linedefs_data = tokio::task::spawn_blocking(move || {
            let mut linedefs = Vec::new();
            let mut cursor = Cursor::new(buffer);
            let num_linedefs = size / 14;
            
            for _ in 0..num_linedefs {
                if let Ok(linedef) = LineDef::from_wad(&mut cursor) {
                    linedefs.push(Arc::new(linedef));
                }
            }
            linedefs
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut linedefs = self.linedefs.write();
        *linedefs = linedefs_data;
        
        Ok(())
    }

    async fn load_things_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        if size % 10 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, 
                "THINGS lump size is not a multiple of 10"));
        }

        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let things_data = tokio::task::spawn_blocking(move || {
            let mut things = Vec::new();
            let mut cursor = Cursor::new(buffer);
            let num_things = size / 10;
            
            for _ in 0..num_things {
                if let Ok(thing) = Thing::from_wad(&mut cursor) {
                    things.push(Arc::new(thing));
                }
            }
            things
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut things = self.things.write();
        *things = things_data;
        
        Ok(())
    }

    async fn load_behavior_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let mut behavior_data = self.behavior_data.write();
        *behavior_data = buffer;
        
        Ok(())
    }

    async fn load_scripts_async<R: Read + Seek>(
        &self,
        reader: &mut R,
        offset: i32,
        size: i32
    ) -> io::Result<()> {
        let mut buffer = vec![0u8; size as usize];
        reader.seek(SeekFrom::Start(offset as u64))?;
        reader.read_exact(&mut buffer)?;

        let mut scripts_data = self.scripts_data.write();
        *scripts_data = buffer;
        
        Ok(())
    }

    /// Loads the geometry for a level given by its marker (e.g. "MAP01").
    /// 
    pub async fn load_level_async<R: Read + Seek>(&mut self, level_name: &str, reader: &mut R) -> io::Result<()> {
        self.clear_geometry();
        let level_info_opt = {
            let levels = self.levels.read();
            levels.iter().find(|lvl| lvl.name.eq_ignore_ascii_case(level_name)).cloned()
        };
        if let Some(level_info) = level_info_opt {
            let directory = self.directory.read();
            for &index in &level_info.lump_indices {
                let entry = &directory[index];
                // Normalize lump name by trimming and converting to uppercase.
                let lump_name = entry.name.trim().to_uppercase();
                match lump_name.as_str() {
                    "THINGS" => { self.load_things_async(reader, entry.offset, entry.size).await?; },
                    "VERTEXES" => { self.load_vertices_async(reader, entry.offset, entry.size).await?; },
                    "SECTORS" => { self.load_sectors_async(reader, entry.offset, entry.size).await?; },
                    "SIDEDEFS" => { self.load_sidedefs_async(reader, entry.offset, entry.size).await?; },
                    "LINEDEFS" => { self.load_linedefs_async(reader, entry.offset, entry.size).await?; },
                    "BEHAVIOR" => { self.load_behavior_async(reader, entry.offset, entry.size).await?; },
                    "SCRIPTS" => { self.load_scripts_async(reader, entry.offset, entry.size).await?; },
                    _ => { /* Ignore unknown lumps */ }
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

    // --- Sector relationships and geometry helper methods ---

    pub fn get_sector_from_side(&self, side: &SideDef) -> Option<Arc<Sector>> {
        self.sectors.read().get(side.sector as usize).cloned()
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

    // (Optional) For demonstration or testing
    #[allow(unused)] // remove this if you use it
    pub fn build_bsp(&self) -> Option<BspLevel> {
        // This is just a stub for if you actually build a BSP.
        // Return None or Some(BspLevel) as needed.
        None
    }

    /// A small helper field for the "generate_test_map" example.
    /// This might not exist in your code. Remove or adapt as needed.

    /// Generate a simple test map (square room) for demonstration.
    /// Adjust or remove as needed.
    pub fn generate_test_map(&mut self) {
        // Clear existing data
        self.clear_geometry();

        // Add vertices for a simple square
        let mut v_lock = self.vertices.write();
        let v0 = v_lock.len() as i16;
        v_lock.push(Arc::new(Vertex::new(-100, -100)));
        v_lock.push(Arc::new(Vertex::new(100, -100)));
        v_lock.push(Arc::new(Vertex::new(100, 100)));
        v_lock.push(Arc::new(Vertex::new(-100, 100)));
        drop(v_lock);

        // Add a single sector
        let mut s_lock = self.sectors.write();
        let s0 = s_lock.len() as i16;
        s_lock.push(Arc::new(Sector::new(
            0,  // floor height
            128, // ceiling height
            "FLOOR4_8".into(),
            "CEIL3_5".into(),
            160, // light
            0,   // type
            0,   // tag
        )));
        drop(s_lock);

        // Add sidedef
        let mut sd_lock = self.sidedefs.write();
        let sd0 = sd_lock.len() as i16;
        sd_lock.push(Arc::new(SideDef::new(
            0, 0,
            "STARTAN2".into(),
            "STARTAN2".into(),
            "STARG3".into(),
            s0 as i32,
        )));
        drop(sd_lock);

        // Add linedefs for the square
        let mut l_lock = self.linedefs.write();
        let l0 = l_lock.len() as i16;
        // Each side references the same sidedef and sector on the "right"
        l_lock.push(Arc::new(LineDef::new(
            v0 as usize, (v0 + 1) as usize,
            0, 0, 0, sd0 as i32, -1,
        )));
        l_lock.push(Arc::new(LineDef::new(
            (v0 + 1) as usize, (v0 + 2) as usize,
            0, 0, 0, sd0 as i32, -1,
        )));
        l_lock.push(Arc::new(LineDef::new(
            (v0 + 2) as usize, (v0 + 3) as usize,
            0, 0, 0, sd0 as i32, -1,
        )));
        l_lock.push(Arc::new(LineDef::new(
            (v0 + 3) as usize, v0 as usize,
            0, 0, 0, sd0 as i32, -1,
        )));
        drop(l_lock);

        // Add a Thing (e.g., Player 1 start).
        let mut t_lock = self.things.write();
        t_lock.push(Arc::new(Thing::new(
            0, // x
            0, // y
            0, // angle
            1, // doom_type (player 1 start)
            0, // flags
        )));
        drop(t_lock);

        info!("Generated a simple test map.");
        self.map_name = "TESTMAP".into();

        // If you want to build a BSP right away, do it:
        self.build_bsp();
    }
}

// --- Checksum helper functions ---

fn add_crc(crc: &mut u32, value: i32) {
    *crc = crc.wrapping_add(value as u32);
}

fn checksum_thing(crc: &mut u32, thing: &Thing) {
    add_crc(crc, thing.x);
    add_crc(crc, thing.y);
    add_crc(crc, thing.angle);
    add_crc(crc, thing.doom_type);
    add_crc(crc, thing.flags);
}

fn checksum_vertex(crc: &mut u32, vertex: &Vertex) {
    add_crc(crc, vertex.x);
    add_crc(crc, vertex.y);
}

fn checksum_sector(crc: &mut u32, sector: &Sector) {
    add_crc(crc, sector.floor_height);
    add_crc(crc, sector.ceiling_height);
    add_crc(crc, sector.light);
    add_crc(crc, sector.r#type);
    add_crc(crc, sector.tag);
    for byte in sector.floor_tex.as_bytes() {
        add_crc(crc, *byte as i32);
    }
    for byte in sector.ceiling_tex.as_bytes() {
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
    add_crc(crc, sidedef.sector);
}

fn checksum_linedef(crc: &mut u32, linedef: &LineDef, _doc: &Document) {
    add_crc(crc, linedef.flags);
    add_crc(crc, linedef.line_type);
    add_crc(crc, linedef.tag);
    add_crc(crc, linedef.start as i32);
    add_crc(crc, linedef.end as i32);
    add_crc(crc, linedef.right);
    add_crc(crc, linedef.left);
}
