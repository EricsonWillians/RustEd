// src/editor/instance.rs
pub struct Instance;

impl Instance {
    pub fn new() -> Self {
        Instance
    }
    pub fn editor_init(&mut self) {
        // Stub initialization
    }
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
