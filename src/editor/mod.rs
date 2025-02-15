// src/editor/mod.rs

mod commands;
mod cutpaste;  // Example - you had this file, so keep the module.
mod generator;
mod hover;
pub mod instance;
mod linedef;
pub mod objects;
mod sector;
mod things;
mod vertex;

pub use commands::Command;
pub use generator::ProceduralGenerator;  // Make sure you can access the generator

use crate::bsp::{BspLevel};
use crate::document::Document;
use parking_lot::RwLock;
use std::sync::Arc;

// --- Enums and Supporting Types ---

/// Represents the different editing tools available in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Vertex,
    Line,    // Renamed for consistency
    Sector,
    Thing,
}

impl Tool {
    /// Returns a user-friendly name for the tool.
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Vertex => "Vertex",
            Tool::Line => "Line",
            Tool::Sector => "Sector",
            Tool::Thing => "Thing",
        }
    }

    /// Returns all available tools. Useful for UI elements like toolbars.
    pub fn all() -> &'static [Tool] {
        &[
            Tool::Select,
            Tool::Vertex,
            Tool::Line,
            Tool::Sector,
            Tool::Thing,
        ]
    }
}

/// Represents a selectable object within the Doom map.  This is a good
/// place to use an enum, as it provides a single, unified way to refer
/// to different kinds of selectable things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Vertex(usize),  // Index into the vertices vector
    Line(usize),    // Index into the linedefs vector
    Sector(usize),  // Index into the sectors vector
    Thing(usize),   // Index into the things vector
    // Could add SideDef(usize, Side) here, if you want selectable sidedefs.
}


// --- Main Editor Struct ---

/// The main editor state.  This struct holds all the data and logic
/// necessary for editing Doom maps.
pub struct Editor {
    /// The Doom map document being edited.  The `RwLock` allows for
    /// concurrent read access, but exclusive write access.  The `Arc`
    /// allows the document to be shared with other parts of the editor
    /// (like the BSP builder) without cloning the entire document.
    document: Arc<RwLock<Document>>,

    /// The BSP tree for the current map.  It's optional because it might
    /// not be built yet, or it might be invalid after map edits.
    bsp_level: Option<Arc<BspLevel>>,

    /// The currently active editing tool.
    current_tool: Tool,

    /// The currently selected objects.  Using a `Vec` allows for
    /// multiple selections (e.g., holding Shift to select multiple
    /// vertices).  Consider if you really need a Vec, though - a single
    /// selection might be sufficient.
    selection: Vec<Selection>,

    // --- Additions for a more robust editor ---
    /// History of undo/redo actions.
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,

    /// Flag indicating if the document has unsaved changes.
    is_dirty: bool,
    
    // Consider adding:
    // - zoom_level: f32,
    // - pan_offset: (f32, f32),
    // - grid_size: i32,
    // - settings: EditorSettings, // A struct for various editor preferences
}


impl Editor {
    /// Creates a new editor instance.
    pub fn new(doc: Arc<RwLock<Document>>) -> Self {
        Self {
            document: doc,
            bsp_level: None,          // BSP is built later
            current_tool: Tool::Select,    // Start with the selection tool
            selection: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_dirty: false,
        }
    }
    pub fn document(&self) -> Option<Arc<RwLock<Document>>>{
        Some(self.document.clone())
    }
    /// Builds the BSP tree for the current document.
    pub fn build_nodes(&mut self) -> Result<(), String> {
        let doc_clone = self.document.clone(); // Clone the Arc to pass to BspLevel
        let bsp = Arc::new(BspLevel::new(doc_clone));
        bsp.build()?;  // The build method can return an error
        self.bsp_level = Some(bsp);
        Ok(())
    }

    /// Gets the currently active tool.
    pub fn current_tool(&self) -> Tool {
        self.current_tool
    }

    /// Sets the currently active tool.
    pub fn set_current_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        // You might want to clear the selection here, or perform other
        // tool-specific setup.
        self.selection.clear();
    }

    /// Returns a slice of all available tools.
    pub fn available_tools(&self) -> &'static [Tool] {
        Tool::all()
    }

    /// Gets the currently selected object (if any).  Returns the *last*
    /// selected object if multiple objects are selected.  This simplifies
    /// the logic in many cases.  If you need to handle multiple selections
    /// explicitly, you can iterate over `self.selection`.
    pub fn selected_object(&self) -> Option<&Selection> {
        self.selection.last()
    }

    pub fn bsp_tree(&self) -> Option<Arc<BspLevel>>{
        self.bsp_level.clone()
    }

    /// Cancels the current editing operation (e.g., drawing a line).
    pub fn cancel_current_operation(&mut self) {
        // This is a good place to clear any temporary state related to
        // the current tool.
        self.selection.clear();  // For example, clear the selection
    }


    /// Generates a simple test map for quick testing.
    pub fn generate_test_map(&mut self) {
        // Create a simple test map
        let mut doc_guard = self.document.write();
        doc_guard.clear(); // Use the guard to directly modify
        
        let v1 = doc_guard.add_vertex(100, 100);
        let v2 = doc_guard.add_vertex(200, 100);
        let v3 = doc_guard.add_vertex(200, 200);
        let v4 = doc_guard.add_vertex(100, 200);
        let sector = doc_guard.add_sector(128,0, "FLOOR4_8".to_string(), "CEIL3_5".to_string(), 0, 0); // Add a sector.
        doc_guard.add_linedef(v1, v2, -1, -1, sector as i16, sector as i16, false, false, false, false, false, false, false, 0, 0, "WALL1".to_string()); // Example linedef
        doc_guard.add_linedef(v2, v3, -1, -1, sector as i16, sector as i16, false, false, false, false, false, false, false, 0, 0, "WALL1".to_string());
        doc_guard.add_linedef(v3, v4, -1, -1, sector as i16, sector as i16, false, false, false, false, false, false, false, 0, 0, "WALL1".to_string());
        doc_guard.add_linedef(v4, v1, -1, -1, sector as i16, sector as i16, false, false, false, false, false, false, false, 0, 0, "WALL1".to_string());

        drop(doc_guard);
        let _ = self.build_nodes();
    }


    pub fn execute_command(&mut self, command: Command) {
        // 1. Execute the command, potentially modifying the document.
        let mut doc_guard = self.document.write(); // Get write access to the document
        if let Err(err) = command.execute(&mut doc_guard) { // Pass mutable reference
            println!("Error executing command: {}", err);
            return;  // Don't add the command to the undo stack if it failed.
        }
        drop(doc_guard);
        // 2. Push the command onto the undo stack.
        self.undo_stack.push(command);

        // 3. Clear the redo stack.
        self.redo_stack.clear();

        // 4. Set `self.is_dirty = true;`
        self.is_dirty = true;
    }


    pub fn undo(&mut self) {
        if let Some(command) = self.undo_stack.pop() {
            let mut doc_guard = self.document.write();
            if let Err(err) = command.unexecute(&mut doc_guard) { // Pass mutable reference.
                println!("Error undoing command: {}", err);
                // Handle the error (maybe put the command back on the undo stack?)
            } else {
                self.redo_stack.push(command);
                self.is_dirty = !self.undo_stack.is_empty(); // Only clean if undo stack is empty
            }
            drop(doc_guard);
        }
    }


    pub fn redo(&mut self) {
        if let Some(command) = self.redo_stack.pop() {
            let mut doc_guard = self.document.write();
            if let Err(err) = command.execute(&mut doc_guard) { // Pass mutable reference
                println!("Error redoing command: {}", err);
                // Handle the error
            } else{
                self.undo_stack.push(command);
                self.is_dirty = true;
            }
            drop(doc_guard);
        }
    }


    pub fn has_unsaved_changes(&self) -> bool {
        self.is_dirty
    }

    pub fn save_document(&mut self) -> Result<(), String> {
       //self.document.write().save_to_wad("output.wad")?;
       self.is_dirty = false;
       Ok(())
    }
}

impl Default for Editor {
    fn default() -> Self {
        //Default values
        Self::new(Arc::new(RwLock::new(Document::new())))
    }
}