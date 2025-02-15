//! src/document/mod.rs
//! The main entry point for the `document` module, re-exporting all core definitions.

mod document;

// Re-export everything (or selectively export only what you need).
pub use self::document::{
    Document,
    ObjType,
    Side,
    Vertex,
    Sector,
    LineDef,
    SideDef,
    Thing,
};