// src/bsp/mod.rs (CORRECTED)
pub mod bsp_level;
pub mod bsp_node;
mod bsp_procedural; // Not public, used internally
mod bsp_util; // Not public, used internally
pub mod debug_viz; // Make it public
pub use bsp_level::BspLevel; // Export Seg and Block
pub use bsp_node::BspNode;
pub use bsp_util::{Line2D, Point2D, BoundingBox}; // Re-export geometry types
pub use bsp_level::Seg;



// Constants (consider putting these in a separate config module)
pub const BLOCK_SIZE: i32 = 128;
pub const MAX_SEGS: usize = 32768;  // Example limit, adjust as needed
pub const BSP_DEPTH_LIMIT: i32 = 64;  // Prevent excessively deep trees
pub const EPSILON: f64 = 1e-6;      // For floating-point comparisons

// Enum for classifying segment positions relative to a partition line
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegPosition {
    Front,
    Back,
    Spanning, // Changed from Split
    Coincident,
}