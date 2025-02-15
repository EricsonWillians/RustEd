//! src/bsp/bsp_node.rs

use std::sync::Arc;

use crate::bsp::{Line2D, Seg};

/// A node in the BSP tree. Each node has:
/// - An optional `partition` line (None for leaves).
/// - Optionally a `front` child and a `back` child.
/// - A list of `segs` if it’s a leaf (or empty if it’s an internal node).
#[derive(Debug)]
pub struct BspNode {
    pub partition: Option<Line2D>,            // The splitting line, None for a leaf
    pub front: Option<Box<BspNode>>,          // Child in the "front" half, or None if leaf
    pub back: Option<Box<BspNode>>,           // Child in the "back" half, or None if leaf
    pub segs: Vec<Arc<Seg>>,                  // Segs for a leaf node, empty for internal
}

impl BspNode {
    /// Create a leaf node with the given segs.
    pub fn create_leaf(segs: Vec<Arc<Seg>>) -> Self {
        BspNode {
            partition: None,
            front: None,
            back: None,
            segs,
        }
    }

    /// Create an internal node with a partition line, front/back children,
    /// and no segs stored in the node itself.
    /// `_bbox` is often stored for debugging or culling, but you can ignore it if not needed.
    pub fn create_node(
        partition: Line2D,
        front: BspNode,
        back: BspNode,
        _bbox: crate::bsp::BoundingBox,
    ) -> Self {
        BspNode {
            partition: Some(partition),
            front: Some(Box::new(front)),
            back: Some(Box::new(back)),
            segs: Vec::new(),
        }
    }

    /// Construct a “leaf” with no segs. 
    /// This is safe if you know the node truly has no children or segs.
    /// (No recursion needed.)
    pub fn empty_leaf() -> Self {
        BspNode {
            partition: None,
            front: None,
            back: None,
            segs: Vec::new(),
        }
    }

    /// Returns `true` if `self` is a leaf (i.e. `partition.is_none()`).
    pub fn is_leaf(&self) -> bool {
        self.partition.is_none()
    }
}
