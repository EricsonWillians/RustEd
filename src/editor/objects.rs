// src/editor/objects.rs

/// `EditObject` represents any object in the document that can be edited.
/// The inner `usize` typically represents an index or unique ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditObject {
    Vertex(usize),
    LineDef(usize),
    SideDef(usize),
    Sector(usize),
    Thing(usize),
}
