// src/editor/mod.rs

pub mod commands;
pub mod cutpaste;
pub mod generator;
pub mod hover;
pub mod instance;
// objects.rs is no longer needed

use crate::bsp::bsp_level::BspLevel; // Correct import for BspLevel
use crate::document::Document;
use crate::map::{LineDef, Sector, Thing, Vertex}; // Import map types
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;
use thiserror::Error;
use crate::editor::commands::Command;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("No document is currently open.")]
    NoDocumentOpen,
    #[error("Invalid document state: {0}")]
    InvalidDocumentState(String),
    #[error("BSP Error: {0}")]
    BspError(String), // Wrap BSP errors
    #[error("Command execution error {0}")]
    CommandExecutionError(String),
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    None,
    Vertex(Arc<Vertex>),
    Line(Arc<LineDef>),
    Sector(Arc<Sector>),
    Thing(Arc<Thing>),
    // Add other selection types as needed (e.g., SideDef)
}

pub struct Editor {
    document: Option<Arc<RwLock<Document>>>,
    current_tool: Tool,
    bsp_level: Option<Arc<BspLevel>>, // Store Arc<BspLevel>
    command_history: Vec<Box<dyn Command>>,
    undo_stack: Vec<Box<dyn Command>>,
    selection: Selection, // Store the selection
}

impl Editor {
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Editor {
            document: Some(document),
            current_tool: Tool::Select,
            bsp_level: None, // Initialize as None
            command_history: Vec::new(),
            undo_stack: Vec::new(),
            selection: Selection::None,
        }
    }

    pub fn document(&self) -> Option<Arc<RwLock<Document>>> {
        self.document.clone()
    }

    pub fn current_tool(&self) -> Tool {
        self.current_tool
    }

    pub fn set_document(&mut self, doc: Arc<RwLock<Document>>) {
        self.document = Some(doc);
    }

    pub fn new_document(&mut self) {
        self.document = Some(Arc::new(RwLock::new(Document::new())));
        self.bsp_level = None; // Reset the BSP level
    }

    pub fn set_current_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        info!("Current tool set to: {:?}", self.current_tool);
        // Clear selection when tool changes
        self.selection = Selection::None;
    }
    pub fn available_tools(&self) -> Vec<Tool> {
        vec![Tool::Select, Tool::Draw] // Return all available tools
    }
    pub fn bsp_level(&self) -> Option<Arc<BspLevel>> { // Return the Arc<BspLevel>
        self.bsp_level.clone()
    }

    pub fn has_unsaved_changes(&self) -> bool {
        false // Implement
    }

    pub fn save_document(&mut self) -> Result<(), EditorError> {
        // Placeholder for save logic
        Err(EditorError::NoDocumentOpen)
    }

      pub fn build_nodes(&mut self) -> Result<(), EditorError> {
        if let Some(doc) = &self.document {
            let bsp_level = Arc::new(BspLevel::new(Arc::clone(doc))); // Create BspLevel
            {  // limit the lifetime of the mutable borrow
                let mut bsp_level_mut = bsp_level.clone();
                bsp_level_mut.build().map_err(EditorError::BspError)?; // Build the BSP (using the ? operator)
            }
            self.bsp_level = Some(bsp_level); // Store the Arc<BspLevel>
            Ok(())
        } else {
            Err(EditorError::NoDocumentOpen)
        }
    }
    pub fn generate_test_map(&mut self) {
        if let Some(doc) = &self.document {
            let mut doc_write = doc.write(); // Get write access to the document
            doc_write.clear();
            // Add some basic geometry for testing:
            let v1 = doc_write.add_vertex(-100, -100);
            let v2 = doc_write.add_vertex(100, -100);
            let v3 = doc_write.add_vertex(100, 100);
            let v4 = doc_write.add_vertex(-100, 100);
            doc_write.add_linedef(v1, v2, 0, -1);
            doc_write.add_linedef(v2, v3, 0, -1);
            doc_write.add_linedef(v3, v4, 0, -1);
            doc_write.add_linedef(v4, v1, 0, -1);

            doc_write.add_sector(
                128,
                0,
                "FLOOR4_8".to_string(),
                "CEIL3_5".to_string(),
                192,
                9,
            );
            drop(doc_write);
            self.build_nodes().expect("Failed to build nodes");
        }
    }
    pub fn execute_command(&mut self, command: Box<dyn Command>) {
        if let Some(doc) = &self.document {
            let mut doc = doc.write();  // Get write access
            if let Err(e) = command.execute(&mut doc) {
                log::error!("Failed to execute command: {}", e); // Or use the `error` macro
                // Optionally, set an error state in the editor
                return;
            }
            self.command_history.push(command);
            self.undo_stack.clear();

        }
    }

    pub fn undo(&mut self) {
        if let Some(mut command) = self.command_history.pop() {
            if let Some(doc) = &self.document {
                let mut doc = doc.write(); // Get a write lock
                if let Err(e) = command.undo(&mut doc) {
                    log::error!("Undo failed: {}", e);
                    return;
                }
                self.undo_stack.push(command);
            }
        }
    }
    
    pub fn redo(&mut self) {
        if let Some(mut command) = self.undo_stack.pop() {
            if let Some(doc) = &self.document {
                let mut doc = doc.write(); // Get a write lock
                if let Err(e) = command.execute(&mut doc) {
                    log::error!("Redo failed: {}", e);
                    return;
                }
                self.command_history.push(command);
            }
        }
    }

    pub fn cancel_current_operation(&mut self) {
        // Reset any tool-specific state, clear selections, etc.
        self.selection = Selection::None; // Example: Clear selection
        info!("Current operation cancelled");
    }

    pub fn selected_object(&self) -> Selection {
        match &self.selection {
            Selection::None => Selection::None,
            Selection::Vertex(v) => {
                if let Some(doc) = &self.document {
                    let doc_read = doc.read();
                    let vertices_read = doc_read.vertices.read();
                    // Find the vertex that matches the selection and return it
                    if let Some(vertex) = vertices_read.iter().find(|x| Arc::ptr_eq(x, v)) {
                        return Selection::Vertex(vertex.clone());
                    }
                }
                Selection::None
            },
            Selection::Line(l) => {
                if let Some(doc) = &self.document {
                    let doc_read = doc.read();
                    let linedefs_read = doc_read.linedefs.read();
                    // Find the matching linedef in the document
                    if let Some(linedef) = linedefs_read.iter().find(|x| Arc::ptr_eq(x, l)){
                        return Selection::Line(linedef.clone());
                    }
                }
                Selection::None
            },
            Selection::Sector(s) => {
                if let Some(doc) = &self.document {
                    let doc_read = doc.read();
                    let sectors_read = doc_read.sectors.read();
                    // Find the sector by matching the pointer equality and return it
                    if let Some(sector) = sectors_read.iter().find(|x| Arc::ptr_eq(x, s)) {
                        return Selection::Sector(sector.clone());
                    }
                }
                Selection::None
            },
            Selection::Thing(t) => {
                if let Some(doc) = &self.document {
                    let doc_read = doc.read();
                    let things_read = doc_read.things.read();
                    // Find the thing in the document and return its Arc reference
                    if let Some(thing) = things_read.iter().find(|x| Arc::ptr_eq(x, t)) {
                        return Selection::Thing(thing.clone());
                    }
                }
                Selection::None
            }
        }
    }
}