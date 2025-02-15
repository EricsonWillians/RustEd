use crate::bsp::{Line2D, Seg, SegPosition}; // Import Seg, Line2D, and SegPosition
use std::sync::Arc;

#[derive(Debug)]
pub struct BspNode {
    pub partition: Option<Line2D>, // Use Line2D for partition
    pub front: Box<BspNode>,
    pub back: Box<BspNode>,
    pub segs: Vec<Arc<Seg>>, // Keep the segs field
}
// ... rest of BspNode implementation ...
impl BspNode {
  pub fn new(partition: Line2D, front: BspNode, back: BspNode) -> Self {
      BspNode {
          partition: Some(partition),
          front: Box::new(front),
          back: Box::new(back),
          segs: Vec::new(), // No segs in inner nodes.
      }
  }

  pub fn create_leaf(segs: Vec<Arc<Seg>>) -> Self {
      BspNode {
          partition: None, // No partition in a leaf
          front: Box::new(BspNode::empty_leaf()), // Empty leaves for consistency
          back: Box::new(BspNode::empty_leaf()),
          segs,
      }
  }
  pub fn empty_leaf() -> Self{ //Return empty leaf
      BspNode{
          partition: None,
          front: Box::new(BspNode::empty_leaf()),
          back: Box::new(BspNode::empty_leaf()),
          segs: Vec::new(),
      }
  }

  pub fn is_leaf(&self) -> bool {
      self.partition.is_none()
  }

  pub fn create_node(
      partition: Line2D, // Takes Line2D
      front: BspNode,
      back: BspNode,
      _bbox: crate::bsp::BoundingBox,  //Keep BoundingBox as a parameter
  ) -> Self {
      BspNode {
          partition: Some(partition),
          front: Box::new(front),
          back: Box::new(back),
          segs: Vec::new(),  // Internal nodes don't store segs directly
      }
  }
}