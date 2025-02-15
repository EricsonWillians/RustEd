// src/editor/instance.rs
use std::sync::Arc;
use crate::bsp::BspLevel; // Assuming this is where BspLevel is defined
use crate::bsp::debug_viz::BspDebugger; // And BspDebugger

pub struct Instance {
    pub bsp_level: Option<Arc<BspLevel>>,
    pub bsp_debugger: BspDebugger, // Initialize this
}

impl Instance {
    pub fn new() -> Self {
        Instance {
            bsp_level: None,
            bsp_debugger: BspDebugger::new(), // Initialize your BspDebugger here
        }
    }
    pub fn editor_init(&mut self) {
        // Stub initialization
    }
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}