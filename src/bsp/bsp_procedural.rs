//! src/bsp/bsp_procedural.rs
//! Procedurally generates rooms and corridors, then builds a `Document`.

use std::collections::HashMap;
use union_find::Size;
use rand::Rng; // Provides .random(), .random_range(), etc.
use rayon::prelude::*; // For parallel iterator
use union_find::UnionFind;

use crate::{
    bsp::{BoundingBox, Point2D},
    document::Document,
};

/// Configuration for procedural generation
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub min_room_size: i32,
    pub max_room_size: i32,
    pub min_corridor_width: i32,
    pub max_corridor_width: i32,
    pub room_density: f64,
    pub branching_factor: f64,
}

/// Tracks some optional stats about a generation run
#[derive(Default, Debug)]
pub struct GenerationStats {
    pub generation_time: f64,
    pub room_count: usize,
    pub corridor_count: usize,
    pub vertex_count: usize,
}

/// Encapsulates the generator state
pub struct ProceduralGenerator {
    /// The user-facing config
    pub config: GeneratorConfig,

    /// Internal RNG used for generation
    rng: rand::rngs::ThreadRng,

    /// List of generated room bounding boxes
    pub rooms: Vec<BoundingBox>,

    /// List of corridors as pairs of points
    pub corridors: Vec<(Point2D, Point2D)>,

    /// Optional stats for debugging / analysis
    pub stats: Option<GenerationStats>,
}

impl ProceduralGenerator {
    /// Creates a new generator with the provided config
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            config,
            rng: rand::rng(), // modern replacement for thread_rng()
            rooms: Vec::new(),
            corridors: Vec::new(),
            stats: Some(GenerationStats::default()),
        }
    }

    /// Generate a complete map of size (width x height), returning a Document.
    pub fn generate(&mut self, width: i32, height: i32) -> Result<Document, String> {
        let mut doc = Document::new();

        // 1) Generate rooms (in parallel)
        self.rooms = self.generate_rooms(width, height);

        // 2) Connect them with corridors
        //    Avoid "borrow self.immutable and self.mutable" by cloning
        let rooms_snapshot = self.rooms.clone();
        let new_corridors = self.generate_corridors(&rooms_snapshot);
        self.corridors = new_corridors;

        // 3) Convert geometry into the Document
        self.build_document(&mut doc, &self.rooms, &self.corridors)?;

        // (Optional) fill in self.stats fields here if you like
        // e.g.: self.stats.as_mut().map(|st| { st.room_count = self.rooms.len(); ... });

        Ok(doc)
    }

    /// Generate the bounding boxes for rooms in parallel
    fn generate_rooms(&mut self, width: i32, height: i32) -> Vec<BoundingBox> {
        // Number of rooms is proportionate to area * density
        let room_count = ((width * height) as f64 * self.config.room_density) as i32;

        (0..room_count)
            .into_par_iter() // parallel
            .map(|_| {
                // Thread-local RNG each iteration for consistent parallel usage
                let mut local_rng = rand::rng();

                let w = local_rng.random_range(self.config.min_room_size..=self.config.max_room_size);
                let h = local_rng.random_range(self.config.min_room_size..=self.config.max_room_size);

                // Keep the room entirely within (width, height)
                let x = local_rng.random_range(0..(width - w));
                let y = local_rng.random_range(0..(height - h));

                BoundingBox::new(
                    x as f64,
                    y as f64,
                    (x + w) as f64,
                    (y + h) as f64,
                )
            })
            .collect()
    }

    /// Create corridors between rooms using a Union-Find MST approach
    fn generate_corridors(&mut self, rooms: &[BoundingBox]) -> Vec<(Point2D, Point2D)> {
        let mut corridors = Vec::new();
        if rooms.len() < 2 {
            return corridors;
        }

        let mut connections: UnionFind<Size> = UnionFind::new(rooms.len());

        // 1) Build edge list: (dist, i, j)
        let mut edges = Vec::new();
        for i in 0..rooms.len() {
            for j in (i + 1)..rooms.len() {
                let dist = self.room_distance(&rooms[i], &rooms[j]);
                edges.push((dist, i, j));
            }
        }
        edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // 2) Minimum spanning tree
        for &(_, i, j) in &edges {
            if !connections.find(i, j) {
                connections.union(i, j);
                corridors.push(self.create_corridor(&rooms[i], &rooms[j]));
            }
        }

        // 3) Extra corridors based on branching_factor
        for &(_, i, j) in &edges {
            // Probability check
            if self.rng.random::<f64>() < self.config.branching_factor {
                corridors.push(self.create_corridor(&rooms[i], &rooms[j]));
            }
        }

        corridors
    }

    /// Convert the final geometry (rooms + corridors) into a Document
    fn build_document(
        &self,
        doc: &mut Document,
        rooms: &[BoundingBox],
        corridors: &[(Point2D, Point2D)],
    ) -> Result<(), String> {
        // We'll keep a map from (x,y) to a vertex ID to avoid duplicating vertices
        let mut vertex_map = HashMap::new();

        // 1) Convert each room bounding-box into geometry
        for room in rooms {
            self.add_rectangle_to_doc(doc, room, &mut vertex_map)?;
        }

        // 2) Convert corridor lines into geometry
        for (start, end) in corridors {
            self.add_corridor_to_doc(doc, start, end, &mut vertex_map)?;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Below are placeholder geometry routines. Fill them in as needed:
    // -------------------------------------------------------------------------

    /// Roughly measure distance between two rooms, e.g. center distance
    fn room_distance(&self, r1: &BoundingBox, r2: &BoundingBox) -> f64 {
        let cx1 = (r1.min_x + r1.max_x) * 0.5;
        let cy1 = (r1.min_y + r1.max_y) * 0.5;
        let cx2 = (r2.min_x + r2.max_x) * 0.5;
        let cy2 = (r2.min_y + r2.max_y) * 0.5;

        let dx = cx2 - cx1;
        let dy = cy2 - cy1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Decide how to link two bounding boxes with a corridor
    fn create_corridor(&self, r1: &BoundingBox, r2: &BoundingBox) -> (Point2D, Point2D) {
        // For simplicity, connect the centers
        let cx1 = (r1.min_x + r1.max_x) * 0.5;
        let cy1 = (r1.min_y + r1.max_y) * 0.5;
        let cx2 = (r2.min_x + r2.max_x) * 0.5;
        let cy2 = (r2.min_y + r2.max_y) * 0.5;

        (
            Point2D { x: cx1, y: cy1 },
            Point2D { x: cx2, y: cy2 },
        )
    }

    /// Add a rectangular room to the Document
    fn add_rectangle_to_doc(
        &self,
        _doc: &mut Document,
        _room: &BoundingBox,
        _vertex_map: &mut HashMap<(i32, i32), usize>,
    ) -> Result<(), String> {
        // TODO: add vertices for corners, linedefs for edges, a sector, etc.
        //  e.g.:
        //   1. corner coords => doc.add_vertex
        //   2. doc.add_linedef(...) for each edge
        //   3. maybe doc.add_sector(...) for the interior
        Ok(())
    }

    /// Add a corridor line to the Document
    fn add_corridor_to_doc(
        &self,
        _doc: &mut Document,
        _start: &Point2D,
        _end: &Point2D,
        _vertex_map: &mut HashMap<(i32, i32), usize>,
    ) -> Result<(), String> {
        // TODO: 1) find or create vertices for start & end
        //       2) add linedef (with some default sector or two)
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// Example tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::bsp::{BspLevel, BspNode};

    #[test]
    fn test_simple_generation() {
        let config = GeneratorConfig {
            min_room_size: 64,
            max_room_size: 128,
            min_corridor_width: 32,
            max_corridor_width: 64,
            room_density: 0.02,
            branching_factor: 0.15,
        };

        let mut gen = ProceduralGenerator::new(config);
        let _doc = gen.generate(512, 512).unwrap();
        // At least some geometry
        assert!(!gen.rooms.is_empty());
        // Because add_rectangle_to_doc is placeholder, doc might still be empty
        // but in a real scenario you can assert doc has data
    }

    #[test]
    fn test_bsp_integration() {
        let config = GeneratorConfig {
            min_room_size: 64,
            max_room_size: 128,
            min_corridor_width: 32,
            max_corridor_width: 64,
            room_density: 0.03,
            branching_factor: 0.1,
        };

        let mut gen = ProceduralGenerator::new(config);
        let doc = gen.generate(512, 512).unwrap();
        let bsp = BspLevel::new(Arc::new(parking_lot::RwLock::new(doc)));

        let res = bsp.build();

        assert!(res.is_ok(), "BSP build must succeed");

        let root_guard = bsp.root.read();
        let root_opt = root_guard.as_ref();
        assert!(root_opt.is_some(), "Expected a valid root node");

        // Check node structure
        fn walk_node(node: &BspNode) {
            if !node.is_leaf() {
                // Has partition and children
                assert!(node.partition.is_some());
                assert!(node.front.is_some());
                assert!(node.back.is_some());
                walk_node(node.front.as_ref().unwrap());
                walk_node(node.back.as_ref().unwrap());
            } else {
                // Leaf should have segs
                assert!(!node.segs.is_empty());
            }
        }
        if let Some(rnode) = root_opt {
            walk_node(rnode);
        }
    }
}
