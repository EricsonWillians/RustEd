// src/document/mod.rs
mod document;

// Re-export everything (or selectively export only what you need).
pub use self::document::{Document, ObjType, Side}; // Removed map-specific types