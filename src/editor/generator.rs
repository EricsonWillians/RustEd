// src/editor/generator.rs

use crate::document::Document;

// For now, keep this simple. We'll add more sophisticated generation later.
#[derive(Debug, Clone)]
pub struct ProceduralGenerator {
    // Add configuration options later (e.g., room size, complexity)
}

impl ProceduralGenerator {
    pub fn new() -> Self {
        ProceduralGenerator {}
    }

    // Placeholder:  A very basic map generation.
    pub fn generate_simple_map(&self, document: &mut Document) {
      //Later implementation.
      todo!();
    }
}

impl Default for ProceduralGenerator {
    fn default() -> Self {
        Self::new()
    }
}