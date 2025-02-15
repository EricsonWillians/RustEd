// src/bsp/bsp_procedural.rs

use crate::document::Document;
use crate::bsp::{BoundingBox, Point2D};
use std::collections::HashMap;
use union_find::UnionFind;
use rayon::prelude::*; // Import for into_par_iter
use rand::Rng; // Import for Rng

pub struct ProceduralGenerator {
    config: GeneratorConfig,
    rng: rand::rngs::ThreadRng,
    rooms: Vec<BoundingBox>,       
    corridors: Vec<(Point2D, Point2D)>, 
    stats: Option<GenerationStats>, 
}

#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub min_room_size: i32,
    pub max_room_size: i32,
    pub min_corridor_width: i32,
    pub max_corridor_width: i32,
    pub room_density: f64,
    pub branching_factor: f64,
}
// Add this
#[derive(Default, Debug)]
pub struct GenerationStats {
    pub generation_time: f64,
    pub room_count: usize,
    pub corridor_count: usize,
    pub vertex_count: usize,
}

impl ProceduralGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        ProceduralGenerator {
            config,
            rng: rand::thread_rng(), // Use rand::thread_rng() directly
            rooms: Vec::new(), // Initialize
            corridors: Vec::new(), // Initialize
            stats: Some(GenerationStats::default()), // Initialize
        }
    }

    pub fn generate(&mut self, width: i32, height: i32) -> Result<Document, String> {
        let mut doc = Document::new();

        // Generate rooms in parallel
        self.rooms = self.generate_rooms(width, height);

        // Connect rooms with corridors
        self.corridors = self.generate_corridors(&self.rooms);

        // Convert geometry to Doom format
        self.build_document(&mut doc, &self.rooms, &self.corridors)?;

        Ok(doc)
    }

    fn generate_rooms(&mut self, width: i32, height: i32) -> Vec<BoundingBox> {
        let room_count = ((width * height) as f64 * self.config.room_density) as i32;

        (0..room_count)
            .into_par_iter() // Use into_par_iter() for parallel iteration
            .map(|_| {
                let mut rng = rand::thread_rng(); // Local RNG for each thread
                let w = rng.gen_range(self.config.min_room_size..=self.config.max_room_size);
                let h = rng.gen_range(self.config.min_room_size..=self.config.max_room_size);
                let x = rng.gen_range(0..width - w);
                let y = rng.gen_range(0..height - h);

                BoundingBox::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64)
            })
            .collect()
    }


    fn generate_corridors(&mut self, rooms: &[BoundingBox]) -> Vec<(Point2D, Point2D)> {
        let mut corridors = Vec::new();
        let mut connections = UnionFind::new(rooms.len());

        // Create minimum spanning tree of rooms
        let mut edges = Vec::new();
        for i in 0..rooms.len() {
            for j in i+1..rooms.len() {
                let dist = self.room_distance(&rooms[i], &rooms[j]);
                edges.push((dist, i, j));
            }
        }
        edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Add required corridors
        for (_, i, j) in edges {
            if !connections.connected(i, j) {
                connections.union(i, j);
                corridors.push(self.create_corridor(&rooms[i], &rooms[j]));
            }
        }

        // Add some extra corridors based on branching factor
        for (dist, i, j) in edges {  // Iterate over edges again
           if self.rng.gen::<f64>() < self.config.branching_factor {
                corridors.push(self.create_corridor(&rooms[i], &rooms[j]));
            }
        }

        corridors
    }


    fn build_document(
        &self,
        doc: &mut Document,
        rooms: &[BoundingBox],
        corridors: &[(Point2D, Point2D)],
    ) -> Result<(), String> {
        // Convert rooms to sectors, linedefs, and vertices
        let mut vertex_map = HashMap::new();

        // Create vertices for rooms
        for room in rooms {
            self.add_rectangle_to_doc(doc, room, &mut vertex_map)?;
        }

        // Create vertices and linedefs for corridors
        for (start, end) in corridors {
            self.add_corridor_to_doc(doc, start, end, &mut vertex_map)?;
        }

        Ok(())
    }
    // Placeholder implementations.  FILL THESE IN.
    fn room_distance(&self, _room1: &BoundingBox, _room2: &BoundingBox) -> f64 {
        0.0 // TODO: Implement actual distance calculation (e.g., distance between centers)
    }

    fn create_corridor(&self, _room1: &BoundingBox, _room2: &BoundingBox) -> (Point2D, Point2D) {
        // TODO:  Implement corridor creation logic (find good connection points)
        (Point2D { x: 0.0, y: 0.0 }, Point2D { x: 0.0, y: 0.0 })
    }

    fn add_rectangle_to_doc(&self, _doc: &mut Document, _room: &BoundingBox, _vertex_map: &mut HashMap<(i32, i32), usize>) -> Result<(), String> {
        // TODO:  Implement room geometry creation.
        Ok(())
    }

    fn add_corridor_to_doc(&self, _doc: &mut Document, _start: &Point2D, _end: &Point2D, _vertex_map: &mut HashMap<(i32, i32), usize>) -> Result<(), String> {
        // TODO: Implement corridor geometry creation.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_generation() {
        let config = GeneratorConfig {
            min_room_size: 64,
            max_room_size: 256,
            min_corridor_width: 32,
            max_corridor_width: 64,
            room_density: 0.1,
            branching_factor: 0.2,
        };

        let mut generator = ProceduralGenerator::new(config);
        let doc = generator.generate(1024, 1024).unwrap();

        assert!(doc.vertices().read().len() > 0);
        assert!(doc.linedefs().read().len() > 0);
        assert!(doc.sectors().read().len() > 0);
    }
}
    #[test]
    fn test_bsp_with_procedural_map() {
        // Generate a test map
        let config = GeneratorConfig {
            min_room_size: 64,
            max_room_size: 128,
            min_corridor_width: 32,
            max_corridor_width: 32,
            room_density: 0.05,
            branching_factor: 0.1,
        };

        let mut generator = ProceduralGenerator::new(config);
        let doc = Arc::new(generator.generate(512, 512).unwrap());
        
        // Build BSP tree
        let bsp = BspLevel::new(doc.clone());
        let result = bsp.build();
        
        assert!(result.is_ok());
        
        // Verify BSP properties
        let root = bsp.root.read();
        assert!(root.is_some());
        
        // Check node properties recursively
        fn check_node(node: &BspNode) {
            if !node.is_leaf() {
                assert!(node.partition.is_some());
                assert!(node.front.is_some());
                assert!(node.back.is_some());
                
                check_node(node.front.as_ref().unwrap());
                check_node(node.back.as_ref().unwrap());
            } else {
                assert!(!node.segs.is_empty());
            }
        }
        
        check_node(root.as_ref().unwrap());
    }