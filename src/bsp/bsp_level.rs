// src/bsp/bsp_level.rs

use crate::bsp::{BspNode, SegPosition, Line2D, BoundingBox, EPSILON, BLOCK_SIZE, Point2D, BSP_DEPTH_LIMIT};
use crate::document::{Document, LineDef, Vertex, Sector}; // Corrected type paths
use std::sync::Arc;
use parking_lot::RwLock;
//use rayon::prelude::*; // Remove this for now.

// Definition of 'Seg'
#[derive(Debug, Clone)] // Add Clone
pub struct Seg {
    pub start: Point2D,
    pub end: Point2D,
    pub angle: f64,
    pub length: f64,
    pub linedef: Option<Arc<LineDef>>,
    pub side: SegmentSide,  // Keep track if it's a front (right) or back (left) side
    pub partner: Option<Arc<Seg>>, // For two-sided linedefs.  Optional.
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentSide {
    Front, // Right side of linedef
    Back,  // Left side of linedef
}

#[derive(Debug)]
pub struct Subsector {
    pub segs: Vec<Arc<Seg>>,
    pub bbox: BoundingBox,        // Bounding box for the subsector
    pub sector: Option<Arc<Sector>>,   // The sector this subsector belongs to
}

// Block definition
#[derive(Debug, Default)]  // Add Default for easier initialization
pub struct Block {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub cells: Vec<Vec<usize>>, // Linedef indices
}
// Implementing new for Bsp_level::Block
impl Block {
    pub fn new(bounds: BoundingBox) -> Self {
      let x = bounds.min_x as i32 / BLOCK_SIZE;
      let y = bounds.min_y as i32 / BLOCK_SIZE;
      let width = ((bounds.max_x - bounds.min_x) as i32 / BLOCK_SIZE) + 1;
      let height = ((bounds.max_y - bounds.min_y) as i32 / BLOCK_SIZE) + 1;
        Self {
            x,
            y,
            width,
            height,
            cells: vec![vec![]; (width * height) as usize], // Initialize cells
        }
    }
    // Add get_cell_mut method too
    pub fn get_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut Vec<usize>> {
      let adjusted_x = x - self.x;
      let adjusted_y = y - self.y;
        if adjusted_x >= 0 && adjusted_x < self.width && adjusted_y >= 0 && adjusted_y < self.height{
            let index = (adjusted_y * self.width + adjusted_x) as usize;
            return self.cells.get_mut(index)
        }
        None
  }
}

pub struct BspLevel {
    doc: Arc<Document>,  // Keep a reference to the Document.
    root: Arc<RwLock<Option<Arc<BspNode>>>>,
    subsectors: Arc<RwLock<Vec<Arc<Subsector>>>>, // Store subsectors
    blocks: Arc<RwLock<Block>>,    // Store the blockmap
    segs: Arc<RwLock<Vec<Arc<Seg>>>>,    // Store all generated segs
}

impl BspLevel {
    pub fn new(doc: Arc<Document>) -> Self {
        let bounds = Self::compute_map_bounds(&doc);  // Compute the bounds *once*.
        let block = Block::new(bounds);      // Create a new Block
        BspLevel {
            doc,
            root: Arc::new(RwLock::new(None)),
            subsectors: Arc::new(RwLock::new(Vec::new())),
            blocks: Arc::new(RwLock::new(block)),          // Initialize with empty blockmap
            segs: Arc::new(RwLock::new(Vec::new())),         // Initialize with empty segs
        }
    }

    pub fn build(&self) -> Result<(), String> {
        // 1. Create initial Segs from Linedefs.
        let initial_segs = self.create_initial_segs()?;

        // 2. Build the BSP tree recursively.
        let root = self.build_bsp_tree(initial_segs, 0)?;
        *self.root.write() = Some(Arc::new(root));

        // 3. Build the blockmap (optional, for optimization).
        self.build_blockmap()?;

        // 4. Process subsectors (for rendering).
        self.process_subsectors()?;

        Ok(())
    }

    fn create_initial_segs(&self) -> Result<Vec<Arc<Seg>>, String> {
        let linedefs = self.doc.linedefs().read();
        let vertices = self.doc.vertices().read();

        let mut segs = Vec::with_capacity(linedefs.len() * 2);

        for linedef in linedefs.iter() {
            let start = vertices.get(linedef.start).ok_or(format!("Invalid start vertex index: {}", linedef.start))?;
            let end = vertices.get(linedef.end).ok_or(format!("Invalid end vertex index: {}", linedef.end))?;

            // Create front side seg (if the linedef has a right side)
            if linedef.right >= 0 {
                segs.push(Arc::new(Seg {
                    start: Point2D::new(start.raw_x as f64, start.raw_y as f64),
                    end: Point2D::new(end.raw_x as f64, end.raw_y as f64),
                    angle: Self::compute_angle(&start, &end),
                    length: Self::compute_length(&start, &end),
                    linedef: Some(linedef.clone()),
                    side: SegmentSide::Front,
                    partner: None, // Filled in later
                }));
            }

            // Create back side seg (if the linedef has a left side)
            if linedef.left >= 0 {
                segs.push(Arc::new(Seg {
                    start: Point2D::new(end.raw_x as f64, end.raw_y as f64),
                    end: Point2D::new(start.raw_x as f64, start.raw_y as f64),
                    angle: Self::compute_angle(&end, &start), // Corrected call
                    length: Self::compute_length(&end, &start), // Corrected call
                    linedef: Some(linedef.clone()),
                    side: SegmentSide::Back,
                    partner: None, // Filled in later
                }));
            }
        }

        self.link_partner_segs(&segs);
        Ok(segs)
    }


    fn link_partner_segs(&self, segs: &[Arc<Seg>]) {
        for i in 0..segs.len() {
            for j in i + 1..segs.len() {
                if let (Some(linedef_i), Some(linedef_j)) = (&segs[i].linedef, &segs[j].linedef) {
                    if Arc::ptr_eq(linedef_i, linedef_j) && segs[i].side != segs[j].side {
                        // Use let bindings to create a longer lived value
                        let mut seg_i_clone = segs[i].as_ref().clone();
                        let mut seg_j_clone = segs[j].as_ref().clone();
                        seg_i_clone.partner = Some(Arc::new(seg_j_clone.clone()));
                        seg_j_clone.partner = Some(Arc::new(seg_i_clone.clone()));

                        // Now use make_mut with the clones
                        let seg_i_mut = Arc::make_mut(&mut segs[i].clone());
                        *seg_i_mut = seg_i_clone;
                        let seg_j_mut = Arc::make_mut(&mut segs[j].clone());
                        *seg_j_mut = seg_j_clone;
                    }
                }
            }
        }
    }


    fn build_bsp_tree(&self, segs: Vec<Arc<Seg>>, depth: i32) -> Result<BspNode, String> {
        if depth >= BSP_DEPTH_LIMIT {
            return Err("BSP tree depth limit exceeded".into());
        }

        if segs.is_empty() {
            return Ok(BspNode::empty_leaf());
        }

        if segs.len() <= 3 {
            return Ok(BspNode::create_leaf(segs));
        }

        let partition = self.choose_partition(&segs)?;
        let (mut front_segs, mut back_segs, spanning_segs) = self.split_segs(&segs, &partition)?;

        for seg in spanning_segs {
            let (front_seg, back_seg) = self.split_seg(&seg, &partition)?;
            if let Some(s) = front_seg {
                front_segs.push(Arc::new(s));
            }
            if let Some(s) = back_seg {
                back_segs.push(Arc::new(s));
            }
        }

        let front = self.build_bsp_tree(front_segs, depth + 1)?;
        let back = self.build_bsp_tree(back_segs, depth + 1)?;

        Ok(BspNode::create_node(
            partition,
            front,
            back,
            self.compute_node_bbox(&segs)
        ))
    }

    fn choose_partition(&self, segs: &[Arc<Seg>]) -> Result<Line2D, String> {
        if let Some(seg) = segs.first() {
            Ok(Line2D::from_seg(seg))
        } else {
            Err("No segs to partition!".into())
        }
    }

    fn split_segs(&self, segs: &[Arc<Seg>], partition: &Line2D) -> Result<(Vec<Arc<Seg>>, Vec<Arc<Seg>>, Vec<Arc<Seg>>), String> {
        let mut front_segs = Vec::new();
        let mut back_segs = Vec::new();
        let mut spanning_segs = Vec::new();

        for seg in segs {
            match self.classify_seg_against_partition(seg, partition) {
                SegPosition::Front => front_segs.push(seg.clone()),
                SegPosition::Back => back_segs.push(seg.clone()),
                SegPosition::Spanning => spanning_segs.push(seg.clone()),
                SegPosition::Coincident => {
                    front_segs.push(seg.clone());
                    back_segs.push(seg.clone());
                }
            }
        }
        Ok((front_segs, back_segs, spanning_segs))
    }

    fn classify_seg_against_partition(&self, seg: &Seg, partition: &Line2D) -> SegPosition {
        let start_side = partition.classify_point(&seg.start);
        let end_side = partition.classify_point(&seg.end);

        if start_side > EPSILON {
            if end_side > EPSILON {
                SegPosition::Front
            } else if end_side < -EPSILON {
                SegPosition::Spanning
            } else {
                SegPosition::Front
            }
        } else if start_side < -EPSILON {
            if end_side < -EPSILON {
                SegPosition::Back
            } else if end_side > EPSILON {
                SegPosition::Spanning
            } else {
                SegPosition::Back
            }
        } else { // start_side == 0.0 (within epsilon)
            if end_side > EPSILON {
                SegPosition::Front
            } else if end_side < -EPSILON{
                SegPosition::Back
            } else{
                SegPosition::Coincident
            }
        }
    }
    
    // This method splits a seg and returns two new segs
    fn split_seg(&self, seg: &Seg, partition: &Line2D) -> Result<(Option<Seg>, Option<Seg>), String> {
        if let Some(intersection) = partition.intersect(&Line2D::from_seg(seg)){
            let front_seg = Seg{
                start: seg.start,
                end: intersection,
                angle: 0.0, //To compute
                length: 0.0, //To compute
                linedef: seg.linedef.clone(),
                side: seg.side,
                partner: None
            };

            let back_seg = Seg{
                start: intersection,
                end: seg.end,
                angle: 0.0, //To compute
                length: 0.0, //To compute
                linedef: seg.linedef.clone(),
                side: seg.side,
                partner: None
            };
            Ok((Some(front_seg), Some(back_seg)))
        } else{
            Err("The segment intersects the partition line in more than one point".to_string())
        }
    }
    

    // Placeholder:  Compute the bounding box of a set of segs.
    fn compute_node_bbox(&self, segs: &[Arc<Seg>]) -> BoundingBox {
        BoundingBox::from_segs(segs)
    }
    // Placeholder: Build the blockmap
    fn build_blockmap(&self) -> Result<(), String> {
        // TODO: Implement blockmap generation
        Ok(())
    }

    // Placeholder: Process subsectors (for rendering)
    fn process_subsectors(&self) -> Result<(), String> {
      // TODO: Implement after implementing BSP Tree.
      Ok(())
    }

    // Placeholder: Compute subsector bbox
    fn compute_subsector_bbox(&self, _segs: &[Arc<Seg>]) -> BoundingBox{
        // TODO: Implement after implementing BSP Tree and subsectors.
        BoundingBox::default()
    }
    
    fn determine_subsector_sector(&self, _segs: &[Arc<Seg>]) -> Option<Arc<Sector>>{
        None
    }

    // Provided helper methods (corrected to use references)
    fn compute_angle(start: &Vertex, end: &Vertex) -> f64 {
        let dx = (end.raw_x - start.raw_x) as f64;
        let dy = (end.raw_y - start.raw_y) as f64;
        dy.atan2(dx)
    }

    fn compute_length(start: &Vertex, end: &Vertex) -> f64 {
        let dx = (end.raw_x - start.raw_x) as f64;
        let dy = (end.raw_y - start.raw_y) as f64;
        dx.hypot(dy)
    }

    fn compute_map_bounds(doc: &Document) -> BoundingBox {
      // Use a let binding to ensure the temporary value lives long enough
        let binding = doc.vertices();
        let vertices = binding.read();
        let mut bounds = BoundingBox::new_empty();

        for vertex in vertices.iter() {
            bounds.expand_point(vertex.raw_x as f64, vertex.raw_y as f64);
        }

        bounds
    }
}