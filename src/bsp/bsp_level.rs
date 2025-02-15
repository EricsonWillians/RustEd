use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    bsp::{
        BspNode, SegPosition, Line2D, BoundingBox,
        EPSILON, BLOCK_SIZE, Point2D, BSP_DEPTH_LIMIT
    },
    document::{Document, LineDef, Vertex, Sector},
};

#[derive(Debug, Clone)]
pub struct Seg {
    pub start: Point2D,
    pub end: Point2D,
    pub angle: f64,
    pub length: f64,
    /// If you want to sort segs by something in the linedef, add that field to `LineDef`.
    pub linedef: Option<Arc<LineDef>>,
    pub side: SegmentSide,
    /// For two-sided linedefs, the front/back segs can reference each other.
    pub partner: Option<Arc<Seg>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentSide {
    Front,
    Back,
}

#[derive(Debug)]
pub struct Subsector {
    pub segs: Vec<Arc<Seg>>,
    pub bbox: BoundingBox,
    pub sector: Option<Arc<Sector>>,
}

// --------------------------------------------------------------------
// Block definition for the blockmap
// --------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct Block {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub cells: Vec<Vec<usize>>,  // each cell references linedef indices or seg indices
}

impl Block {
    pub fn new(bounds: BoundingBox) -> Self {
        let x = (bounds.min_x as i32) / BLOCK_SIZE;
        let y = (bounds.min_y as i32) / BLOCK_SIZE;
        let w = ((bounds.max_x - bounds.min_x) as i32 / BLOCK_SIZE) + 1;
        let h = ((bounds.max_y - bounds.min_y) as i32 / BLOCK_SIZE) + 1;
        let size = (w * h) as usize;

        Self {
            x,
            y,
            width: w,
            height: h,
            cells: vec![vec![]; size],
        }
    }

    pub fn get_cell_mut(&mut self, cx: i32, cy: i32) -> Option<&mut Vec<usize>> {
        let rel_x = cx - self.x;
        let rel_y = cy - self.y;
        if rel_x >= 0 && rel_x < self.width && rel_y >= 0 && rel_y < self.height {
            let idx = (rel_y * self.width + rel_x) as usize;
            self.cells.get_mut(idx)
        } else {
            None
        }
    }
}

// --------------------------------------------------------------------
// BspLevel definition
// --------------------------------------------------------------------
pub struct BspLevel {
    /// The entire map Document, wrapped in Arc<RwLock>.
    pub doc: Arc<RwLock<Document>>,

    /// The root BSP node (if built).
    pub root: Arc<RwLock<Option<Arc<BspNode>>>>,

    /// The list of final subsectors in this BSP.
    pub subsectors: Arc<RwLock<Vec<Arc<Subsector>>>>,

    /// The blockmap, if you’re using it for collision or other logic.
    pub blocks: Arc<RwLock<Block>>,

    /// The list of all Segs built from the Document’s linedefs.
    pub segs: Arc<RwLock<Vec<Arc<Seg>>>>,
}

impl BspLevel {
    /// Create a new BspLevel from an Arc<RwLock<Document>>.
    pub fn new(doc: Arc<RwLock<Document>>) -> Self {
        let bounds = Self::compute_map_bounds(&doc);
        let blockmap = Block::new(bounds);

        BspLevel {
            doc,
            root: Arc::new(RwLock::new(None)),
            subsectors: Arc::new(RwLock::new(Vec::new())),
            blocks: Arc::new(RwLock::new(blockmap)),
            segs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// High-level “build” method that:
    /// 1) Creates initial segs from linedefs,
    /// 2) Builds the BSP tree,
    /// 3) Builds the blockmap,
    /// 4) Processes subsectors.
    pub fn build(&self) -> Result<(), String> {
        let initial = self.create_initial_segs()?;
        let root_node = self.build_bsp_tree(initial, 0)?;
        *self.root.write() = Some(Arc::new(root_node));

        self.build_blockmap()?;
        self.process_subsectors()?;

        Ok(())
    }

    // ----------------------------------------------------------------
    // Step 1: create initial segs from linedefs
    // ----------------------------------------------------------------
    fn create_initial_segs(&self) -> Result<Vec<Arc<Seg>>, String> {
        // NOTE: You said your Document has some .vertices() and .linedefs() accessors;
        // adjust to your actual code. This is just a placeholder pattern:

        let doc_guard = self.doc.read();

        // Assuming doc_guard.linedefs is an Arc<RwLock<Vec<Arc<LineDef>>>>,
        // acquire a read lock to access the Vec.
        let linedefs_ref = doc_guard.linedefs.read();
        let vertices_ref = doc_guard.vertices.read();

        let mut seglist = Vec::with_capacity(linedefs_ref.len() * 2);

        for linedef in linedefs_ref.iter() {
            let start_v = vertices_ref
                .get(linedef.start)
                .ok_or_else(|| format!("Invalid start vertex {}", linedef.start))?;
            let end_v = vertices_ref
                .get(linedef.end)
                .ok_or_else(|| format!("Invalid end vertex {}", linedef.end))?;

            // front seg
            if linedef.right >= 0 {
                let seg_arc = Arc::new(Seg {
                    start: Point2D::new(start_v.raw_x as f64, start_v.raw_y as f64),
                    end: Point2D::new(end_v.raw_x as f64, end_v.raw_y as f64),
                    angle: Self::compute_angle(start_v, end_v),
                    length: Self::compute_length(start_v, end_v),
                    linedef: Some(linedef.clone()), // if linedef is Arc<LineDef>, else wrap
                    side: SegmentSide::Front,
                    partner: None,
                });
                seglist.push(seg_arc);
            }

            // back seg
            if linedef.left >= 0 {
                let seg_arc = Arc::new(Seg {
                    start: Point2D::new(end_v.raw_x as f64, end_v.raw_y as f64),
                    end: Point2D::new(start_v.raw_x as f64, start_v.raw_y as f64),
                    angle: Self::compute_angle(end_v, start_v),
                    length: Self::compute_length(end_v, start_v),
                    linedef: Some(linedef.clone()),
                    side: SegmentSide::Back,
                    partner: None,
                });
                seglist.push(seg_arc);
            }
        }

        self.link_partner_segs(&seglist);
        Ok(seglist)
    }

    /// For any linedef that’s two-sided, link the front/back seg so each has `partner`.
    fn link_partner_segs(&self, segs: &[Arc<Seg>]) {
        for i in 0..segs.len() {
            for j in (i + 1)..segs.len() {
                // If they share the same linedef pointer & different sides, they are front/back pairs
                if let (Some(ld_i), Some(ld_j)) = (&segs[i].linedef, &segs[j].linedef) {
                    // pointer-equality check
                    if Arc::ptr_eq(ld_i, ld_j) && segs[i].side != segs[j].side {
                        // Link them
                        // 1) Make a local clone of each seg Arc
                        let seg_i = segs[i].clone();
                        let seg_j = segs[j].clone();

                        // 2) Mutate each seg’s `partner` field in-place. Because these are plain Arc<Seg>,
                        //    we either re-construct a new seg or mutate with a "get_mut if unique".
                        //    If you expect multiple references, consider a different approach (like interior mutability).
                        if let Some(i_mut) = Arc::get_mut(&mut (segs[i].clone())) {
                            i_mut.partner = Some(seg_j.clone());
                        }
                        if let Some(j_mut) = Arc::get_mut(&mut (segs[j].clone())) {
                            j_mut.partner = Some(seg_i.clone());
                        }
                    }
                }
            }
        }
    }

    // ----------------------------------------------------------------
    // Step 2: recursively build the BSP
    // ----------------------------------------------------------------
    fn build_bsp_tree(&self, segs: Vec<Arc<Seg>>, depth: i32) -> Result<BspNode, String> {
        if depth >= BSP_DEPTH_LIMIT {
            return Err("Reached BSP depth limit".into());
        }
        if segs.is_empty() {
            return Ok(BspNode::empty_leaf());
        }
        if segs.len() <= 3 {
            return Ok(BspNode::create_leaf(segs));
        }

        let partition = self.choose_partition(&segs)?;
        let (mut front_list, mut back_list, spanning) = self.split_list(&segs, &partition)?;

        for seg in spanning {
            let (f_seg, b_seg) = self.split_seg(&seg, &partition)?;
            if let Some(s) = f_seg {
                front_list.push(Arc::new(s));
            }
            if let Some(s) = b_seg {
                back_list.push(Arc::new(s));
            }
        }

        let front_node = self.build_bsp_tree(front_list, depth + 1)?;
        let back_node = self.build_bsp_tree(back_list, depth + 1)?;

        let node_bbox = self.compute_node_bbox(&segs);
        let node = BspNode::create_node(partition, front_node, back_node, node_bbox);
        Ok(node)
    }

    fn choose_partition(&self, segs: &[Arc<Seg>]) -> Result<Line2D, String> {
        if segs.is_empty() {
            return Err("No segs to partition".into());
        }
        Ok(Line2D::from_seg(&segs[0]))
    }

    fn split_list(
        &self,
        segs: &[Arc<Seg>],
        part: &Line2D,
    ) -> Result<(Vec<Arc<Seg>>, Vec<Arc<Seg>>, Vec<Arc<Seg>>), String> {
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut span = Vec::new();

        for seg in segs {
            match self.classify_seg(seg, part) {
                SegPosition::Front => front.push(seg.clone()),
                SegPosition::Back => back.push(seg.clone()),
                SegPosition::Spanning => span.push(seg.clone()),
                SegPosition::Coincident => {
                    front.push(seg.clone());
                    back.push(seg.clone());
                }
            }
        }
        Ok((front, back, span))
    }

    fn classify_seg(&self, seg: &Seg, line: &Line2D) -> SegPosition {
        let side_a = line.classify_point(&seg.start);
        let side_b = line.classify_point(&seg.end);

        if side_a > EPSILON && side_b > EPSILON {
            SegPosition::Front
        } else if side_a < -EPSILON && side_b < -EPSILON {
            SegPosition::Back
        } else if (side_a > EPSILON && side_b < -EPSILON)
            || (side_a < -EPSILON && side_b > EPSILON)
        {
            SegPosition::Spanning
        } else {
            SegPosition::Coincident
        }
    }

    fn split_seg(&self, seg: &Seg, line: &Line2D) -> Result<(Option<Seg>, Option<Seg>), String> {
        let seg_line = Line2D::from_seg(seg);
        if let Some(intersect) = line.intersect(&seg_line) {
            let front_seg = Seg {
                start: seg.start,
                end: intersect,
                angle: 0.0,  // you could recalc if needed
                length: 0.0, // recalc if needed
                linedef: seg.linedef.clone(),
                side: seg.side,
                partner: None,
            };
            let back_seg = Seg {
                start: intersect,
                end: seg.end,
                angle: 0.0,
                length: 0.0,
                linedef: seg.linedef.clone(),
                side: seg.side,
                partner: None,
            };
            Ok((Some(front_seg), Some(back_seg)))
        } else {
            // might be parallel or have no single intersection
            Err("Cannot split seg: no single intersection found".into())
        }
    }

    fn compute_node_bbox(&self, segs: &[Arc<Seg>]) -> BoundingBox {
        BoundingBox::from_segs(segs)
    }

    // ----------------------------------------------------------------
    // (Optional) reorder the segs by some criterion
    // ----------------------------------------------------------------
    pub fn reorder_segs(&mut self) {
        let mut segs_guard = self.segs.write();

        // EXAMPLE: sort by the start.x coordinate, then start.y
        // or if you want to sort by linedef "id", you must have `linedef.id`.
        segs_guard.sort_by(|a, b| {
            let a_read = a.as_ref();
            let b_read = b.as_ref();
            // For example, sort by start.x then start.y
            let ord_x = a_read.start.x.partial_cmp(&b_read.start.x).unwrap_or(std::cmp::Ordering::Equal);
            if ord_x != std::cmp::Ordering::Equal {
                ord_x
            } else {
                // fallback to comparing y
                a_read.start.y.partial_cmp(&b_read.start.y).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
    }

    // ----------------------------------------------------------------
    // Step 3: Build blockmap
    // ----------------------------------------------------------------
    fn build_blockmap(&self) -> Result<(), String> {
        // TODO if desired
        Ok(())
    }

    // ----------------------------------------------------------------
    // Step 4: Process subsectors
    // ----------------------------------------------------------------
    fn process_subsectors(&self) -> Result<(), String> {
        // TODO if desired
        Ok(())
    }

    // ----------------------------------------------------------------
    // Some utility fns
    // ----------------------------------------------------------------
    fn compute_angle(a: &Vertex, b: &Vertex) -> f64 {
        let dx = (b.raw_x - a.raw_x) as f64;
        let dy = (b.raw_y - a.raw_y) as f64;
        dy.atan2(dx)
    }

    fn compute_length(a: &Vertex, b: &Vertex) -> f64 {
        let dx = (b.raw_x - a.raw_x) as f64;
        let dy = (b.raw_y - a.raw_y) as f64;
        dx.hypot(dy)
    }

    fn compute_map_bounds(doc_ref: &Arc<RwLock<Document>>) -> BoundingBox {
        let doc = doc_ref.read();
        // If doc has `vertices` as an Arc<RwLock<Vec<Arc<Vertex>>>>, first acquire a read lock.
        let mut bb = BoundingBox::new_empty();
        let vertices_ref = doc.vertices.read();
        for v in vertices_ref.iter() {
            bb.expand_point(v.raw_x as f64, v.raw_y as f64);
        }
        bb
    }
} 
