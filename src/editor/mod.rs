// src/editor/mod.rs

pub mod instance;

// Optionally, re-export the Instance type for easier access.
pub use instance::Instance;

// Other modules such as commands, cutpaste, etc., would also be declared here.
pub mod commands;
pub mod cutpaste;
pub mod generator;
pub mod hover;
pub mod linedef;
pub mod objects;
pub mod sector;
pub mod things;
pub mod vertex;
