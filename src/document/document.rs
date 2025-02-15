//! # Document Module
//!
//! This module stores level data and provides functions to query and
//! process the level. It includes logic for computing level checksums,
//! accessing objects by type, and various queries (e.g. line length,
//! zero-length detection, etc.).
//!
//! Inspired by the original Eureka DOOM Editor's Document.cc.
//!
//! Licensed under the GNU General Public License v2 (or later).

use std::f64;
use std::rc::Rc;

/// Represents the object types in the level.
#[derive(Debug)]
pub enum ObjType {
    Things,
    Linedefs,
    Sidedefs,
    Vertices,
    Sectors,
}

/// A simple 2D vertex.
#[derive(Debug)]
pub struct Vertex {
    pub raw_x: i32,
    pub raw_y: i32,
}

impl Vertex {
    /// Returns the x-coordinate.
    pub fn x(&self) -> i32 {
        self.raw_x
    }

    /// Returns the y-coordinate.
    pub fn y(&self) -> i32 {
        self.raw_y
    }

    /// Checks if the vertex matches the given coordinates.
    pub fn matches(&self, tx: i32, ty: i32) -> bool {
        self.raw_x == tx && self.raw_y == ty
    }
}

/// Represents a sector.
#[derive(Debug)]
pub struct Sector {
    pub floorh: i32,
    pub ceilh: i32,
    pub light: i32,
    pub sector_type: i32,
    pub tag: i32,
    // For textures, we assume integer representations.
    pub floor_tex: i32,
    pub ceil_tex: i32,
}

impl Sector {
    pub fn floor_tex(&self) -> i32 {
        self.floor_tex
    }

    pub fn ceil_tex(&self) -> i32 {
        self.ceil_tex
    }
}

/// Represents a sidedef.
#[derive(Debug)]
pub struct SideDef {
    pub x_offset: i32,
    pub y_offset: i32,
    pub lower_tex: i32,
    pub mid_tex: i32,
    pub upper_tex: i32,
    /// Index into the Document’s sectors vector.
    pub sector: usize,
}

impl SideDef {
    pub fn lower_tex(&self) -> i32 {
        self.lower_tex
    }

    pub fn mid_tex(&self) -> i32 {
        self.mid_tex
    }

    pub fn upper_tex(&self) -> i32 {
        self.upper_tex
    }
}

/// Represents a linedef.
#[derive(Debug)]
pub struct LineDef {
    pub flags: i32,
    pub line_type: i32,
    pub tag: i32,
    /// Index into the vertices vector.
    pub start: usize,
    /// Index into the vertices vector.
    pub end: usize,
    /// Index into the sidedefs vector; -1 indicates none.
    pub right: i32,
    /// Index into the sidedefs vector; -1 indicates none.
    pub left: i32,
}

/// Represents a thing (e.g. enemy, item, etc.).
#[derive(Debug)]
pub struct Thing {
    pub raw_x: i32,
    pub raw_y: i32,
    pub angle: i32,
    pub thing_type: i32,
    pub options: i32,
}

/// The Document struct stores all level objects.
pub struct Document {
    pub things: Vec<Rc<Thing>>,
    pub vertices: Vec<Rc<Vertex>>,
    pub sectors: Vec<Rc<Sector>>,
    pub sidedefs: Vec<Rc<SideDef>>,
    pub linedefs: Vec<Rc<LineDef>>,

    // Additional raw data.
    pub header_data: Vec<u8>,
    pub behavior_data: Vec<u8>,
    pub scripts_data: Vec<u8>,
    pub basis: Vec<u8>,
}

impl Document {
    /// Creates a new, empty document.
    pub fn new() -> Self {
        Document {
            things: Vec::new(),
            vertices: Vec::new(),
            sectors: Vec::new(),
            sidedefs: Vec::new(),
            linedefs: Vec::new(),
            header_data: Vec::new(),
            behavior_data: Vec::new(),
            scripts_data: Vec::new(),
            basis: Vec::new(),
        }
    }

    /// Returns the number of objects for the given type.
    pub fn num_objects(&self, obj_type: ObjType) -> usize {
        match obj_type {
            ObjType::Things => self.things.len(),
            ObjType::Linedefs => self.linedefs.len(),
            ObjType::Sidedefs => self.sidedefs.len(),
            ObjType::Vertices => self.vertices.len(),
            ObjType::Sectors => self.sectors.len(),
        }
    }

    /// Computes a checksum for the level. The checksum is accumulated in `crc`.
    pub fn get_level_checksum(&self, crc: &mut u32) {
        for thing in &self.things {
            checksum_thing(crc, thing);
        }
        for linedef in &self.linedefs {
            checksum_linedef(crc, linedef, self);
        }
    }

    /// Returns a reference to the sector referenced by a sidedef.
    pub fn get_sector_from_side(&self, side: &SideDef) -> &Sector {
        &self.sectors[side.sector]
    }

    /// Returns the sector ID for a given linedef side.
    pub fn get_sector_id(&self, line: &LineDef, side: Side) -> i32 {
        match side {
            Side::Left => self.get_left(line).map_or(-1, |sd| sd.sector as i32),
            Side::Right => self.get_right(line).map_or(-1, |sd| sd.sector as i32),
        }
    }

    /// Returns an optional reference to the sector for the given linedef side.
    pub fn get_sector_for_line(&self, line: &LineDef, side: Side) -> Option<&Sector> {
        let sid = self.get_sector_id(line, side);
        if sid >= 0 {
            self.sectors.get(sid as usize).map(|rc| rc.as_ref())
        } else {
            None
        }
    }

    /// Returns a reference to the starting vertex of a linedef.
    pub fn get_start(&self, line: &LineDef) -> &Vertex {
        &self.vertices[line.start]
    }

    /// Returns a reference to the ending vertex of a linedef.
    pub fn get_end(&self, line: &LineDef) -> &Vertex {
        &self.vertices[line.end]
    }

    /// Returns the right sidedef of a linedef (if any).
    pub fn get_right(&self, line: &LineDef) -> Option<&SideDef> {
        if line.right >= 0 {
            self.sidedefs.get(line.right as usize).map(|rc| rc.as_ref())
        } else {
            None
        }
    }

    /// Returns the left sidedef of a linedef (if any).
    pub fn get_left(&self, line: &LineDef) -> Option<&SideDef> {
        if line.left >= 0 {
            self.sidedefs.get(line.left as usize).map(|rc| rc.as_ref())
        } else {
            None
        }
    }

    /// Calculates the length of a linedef.
    pub fn calc_length(&self, line: &LineDef) -> f64 {
        let start = self.get_start(line);
        let end = self.get_end(line);
        let dx = (start.x() - end.x()) as f64;
        let dy = (start.y() - end.y()) as f64;
        dx.hypot(dy)
    }

    /// Returns true if the linedef touches the coordinate (tx, ty).
    pub fn touches_coord(&self, line: &LineDef, tx: i32, ty: i32) -> bool {
        let start = self.get_start(line);
        let end = self.get_end(line);
        start.matches(tx, ty) || end.matches(tx, ty)
    }

    /// Returns true if the linedef touches a sector with the given ID.
    pub fn touches_sector(&self, line: &LineDef, sec_num: i32) -> bool {
        if let Some(sd) = self.get_right(line) {
            if sd.sector as i32 == sec_num {
                return true;
            }
        }
        if let Some(sd) = self.get_left(line) {
            if sd.sector as i32 == sec_num {
                return true;
            }
        }
        false
    }

    /// Returns true if the linedef is zero length.
    pub fn is_zero_length(&self, line: &LineDef) -> bool {
        let start = self.get_start(line);
        let end = self.get_end(line);
        start.raw_x == end.raw_x && start.raw_y == end.raw_y
    }

    /// Returns true if the linedef is self-referential (both sidedefs reference the same sector).
    pub fn is_self_ref(&self, line: &LineDef) -> bool {
        if line.left >= 0 && line.right >= 0 {
            if let (Some(left_sd), Some(right_sd)) = (self.get_left(line), self.get_right(line)) {
                return left_sd.sector == right_sd.sector;
            }
        }
        false
    }

    /// Returns true if the linedef is horizontal.
    pub fn is_horizontal(&self, line: &LineDef) -> bool {
        self.get_start(line).raw_y == self.get_end(line).raw_y
    }

    /// Returns true if the linedef is vertical.
    pub fn is_vertical(&self, line: &LineDef) -> bool {
        self.get_start(line).raw_x == self.get_end(line).raw_x
    }

    /// Clears all document data.
    pub fn clear(&mut self) {
        self.things.clear();
        self.vertices.clear();
        self.sectors.clear();
        self.sidedefs.clear();
        self.linedefs.clear();
        self.header_data.clear();
        self.behavior_data.clear();
        self.scripts_data.clear();
        self.basis.clear();
        // TODO: Clear other modules or clipboard state as necessary.
    }
}

/// A simple enum to distinguish between left and right sidedefs.
#[derive(Debug)]
pub enum Side {
    Left,
    Right,
}

/// Helper: adds a value to the checksum accumulator using wrapping arithmetic.
fn add_crc(crc: &mut u32, value: i32) {
    *crc = crc.wrapping_add(value as u32);
}

/// Computes the checksum contribution of a Thing.
fn checksum_thing(crc: &mut u32, thing: &Thing) {
    add_crc(crc, thing.raw_x);
    add_crc(crc, thing.raw_y);
    add_crc(crc, thing.angle);
    add_crc(crc, thing.thing_type);
    add_crc(crc, thing.options);
}

/// Computes the checksum contribution of a Vertex.
fn checksum_vertex(crc: &mut u32, vertex: &Vertex) {
    add_crc(crc, vertex.raw_x);
    add_crc(crc, vertex.raw_y);
}

/// Computes the checksum contribution of a Sector.
fn checksum_sector(crc: &mut u32, sector: &Sector) {
    add_crc(crc, sector.floorh);
    add_crc(crc, sector.ceilh);
    add_crc(crc, sector.light);
    add_crc(crc, sector.sector_type);
    add_crc(crc, sector.tag);
    add_crc(crc, sector.floor_tex());
    add_crc(crc, sector.ceil_tex());
}

/// Computes the checksum contribution of a SideDef.
fn checksum_sidedef(crc: &mut u32, sidedef: &SideDef, doc: &Document) {
    add_crc(crc, sidedef.x_offset);
    add_crc(crc, sidedef.y_offset);
    add_crc(crc, sidedef.lower_tex());
    add_crc(crc, sidedef.mid_tex());
    add_crc(crc, sidedef.upper_tex());
    // Also incorporate the sector data.
    checksum_sector(crc, doc.get_sector_from_side(sidedef));
}

/// Computes the checksum contribution of a LineDef.
fn checksum_linedef(crc: &mut u32, linedef: &LineDef, doc: &Document) {
    add_crc(crc, linedef.flags);
    add_crc(crc, linedef.line_type);
    add_crc(crc, linedef.tag);
    checksum_vertex(crc, doc.get_start(linedef));
    checksum_vertex(crc, doc.get_end(linedef));
    if let Some(sd) = doc.get_right(linedef) {
        checksum_sidedef(crc, sd, doc);
    }
    if let Some(sd) = doc.get_left(linedef) {
        checksum_sidedef(crc, sd, doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    /// Creates a sample document with one linedef and one thing.
    fn create_sample_document() -> Document {
        let mut doc = Document::new();

        // Create sample vertices.
        let v1 = Rc::new(Vertex { raw_x: 0, raw_y: 0 });
        let v2 = Rc::new(Vertex { raw_x: 10, raw_y: 0 });
        doc.vertices.push(v1.clone());
        doc.vertices.push(v2.clone());

        // Create a sample linedef.
        let line = Rc::new(LineDef {
            flags: 1,
            line_type: 2,
            tag: 0,
            start: 0,
            end: 1,
            right: -1,
            left: -1,
        });
        doc.linedefs.push(line);

        // Create a sample thing.
        let thing = Rc::new(Thing {
            raw_x: 5,
            raw_y: 5,
            angle: 90,
            thing_type: 1,
            options: 0,
        });
        doc.things.push(thing);

        doc
    }

    #[test]
    fn test_num_objects() {
        let mut doc = Document::new();
        assert_eq!(doc.num_objects(ObjType::Things), 0);
        let thing = Rc::new(Thing {
            raw_x: 0,
            raw_y: 0,
            angle: 0,
            thing_type: 0,
            options: 0,
        });
        doc.things.push(thing);
        assert_eq!(doc.num_objects(ObjType::Things), 1);
    }

    #[test]
    fn test_checksum() {
        let doc = create_sample_document();
        let mut crc = 0u32;
        doc.get_level_checksum(&mut crc);
        // For this sample document, just check that the checksum is nonzero.
        assert!(crc != 0);
    }

    #[test]
    fn test_calc_length() {
        let doc = create_sample_document();
        let line = &doc.linedefs[0];
        let length = doc.calc_length(line);
        assert!((length - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_is_zero_length() {
        let mut doc = create_sample_document();
        // Modify the line so that start == end.
        if let Some(line) = doc.linedefs.get_mut(0) {
            line.end = line.start;
        }
        assert!(doc.is_zero_length(&doc.linedefs[0]));
    }

    #[test]
    fn test_touches_coord() {
        let doc = create_sample_document();
        let line = &doc.linedefs[0];
        assert!(doc.touches_coord(line, 0, 0));
        assert!(doc.touches_coord(line, 10, 0));
        assert!(!doc.touches_coord(line, 5, 5));
    }

    #[test]
    fn test_is_horizontal() {
        let doc = create_sample_document();
        let line = &doc.linedefs[0];
        assert!(doc.is_horizontal(line));
    }
}
