// src/editor/mod.rs

pub mod commands;
pub mod cutpaste;
pub mod generator;
pub mod hover;
pub mod instance;
pub mod core;

use thiserror::Error;

/// Re-export the Editor struct from the `core` module,
/// so you can `use crate::editor::Editor;` in other code.
pub use core::Editor;

use crate::map::{Vertex, LineDef, Sector, Thing};
use std::sync::Arc;

/// Editor-related errors.
#[derive(Error, Debug)]
pub enum EditorError {
    #[error("No document is currently open.")]
    NoDocumentOpen,

    #[error("Invalid document state: {0}")]
    InvalidDocumentState(String),

    #[error("BSP Error: {0}")]
    BspError(String), // For wrapping any BSP-building errors

    #[error("Command execution error: {0}")]
    CommandExecutionError(String),
}

/// Various editing tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Draw,
    // ... other tools ...
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Draw => "Draw",
            // ... other tools ...
        }
    }
}