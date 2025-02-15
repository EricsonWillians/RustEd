// src/document/mod.rs
mod document;
pub use document::Document;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Vertex {
    pub raw_x: i32,
    pub raw_y: i32,
}

#[derive(Debug, Clone)]
pub struct Sector {
    pub floorh: i32,
    pub ceilh: i32,
    pub floor_tex: String,
    pub ceil_tex: String,
    pub light: i32,
    pub sector_type: i32,
    pub tag: i32,
}

#[derive(Debug, Clone)]
pub struct LineDef {
    pub start: usize,
    pub end: usize,
    pub flags: i32,
    pub line_type: i32,
    pub tag: i32,
    pub right: i32,
    pub left: i32,
}

#[derive(Debug, Clone)]
pub struct SideDef {
    pub x_offset: i32,
    pub y_offset: i32,
    pub upper_tex: String,
    pub lower_tex: String,
    pub mid_tex: String,
    pub sector: usize,
}

#[derive(Debug, Clone)]
pub struct Thing {
    pub raw_x: i32,
    pub raw_y: i32,
    pub angle: i32,
    pub thing_type: i32,
    pub options: i32,
}