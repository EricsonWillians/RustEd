// src/editor/objects.rs
// Add this line
pub type Selection = EditObject; // Add pub

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditObject {
    Vertex(usize),
    LineDef(usize),
    SideDef(usize),
    Sector(usize),
    Thing(usize)
}