// src/editor/core.rs

use std::fs::File;
use std::io::Cursor;
use std::sync::Arc;

use futures::FutureExt;
use log::{error, info};
use parking_lot::RwLock;
use rfd::FileDialog;

use crate::bsp::BspLevel;
use crate::document::Document;
use crate::editor::commands::{Command, CommandType};
use crate::ui::central_panel::CentralPanel;
use eframe::egui; // Import egui

/// Tools for the editor, e.g. Select, DrawLine, etc.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Tool {
    Select,
    DrawLine,
    DrawShape,
    EditThings,
    EditSectors,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::DrawLine => "Draw Line",
            Tool::DrawShape => "Draw Shape",
            Tool::EditThings => "Edit Things",
            Tool::EditSectors => "Edit Sectors",
        }
    }
}

/// A simplified selection enum with cloned data (not Arc<>).
#[derive(Clone, Debug)]
pub enum Selection {
    Vertex(crate::map::Vertex),
    Line(crate::map::LineDef),
    Sector(crate::map::Sector),
    Thing(crate::map::Thing),
    None,
}

/// The core `Editor` struct, holding geometry references, current tool, etc.
pub struct Editor {
    /// The current document, if any.
    document: Option<Arc<RwLock<Document>>>,

    /// Which tool is active.
    current_tool: Tool,

    /// Undo/redo stacks of commands.
    command_history: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,

    /// Messages or status for UI.
    pub status_message: String,
    pub error_message: Option<String>,

    /// Whether the side panel or BSP debug is shown.
    pub show_side_panel: bool,
    pub show_bsp_debug: bool,

    /// A handle to the central panel (camera, pan/zoom) if needed.
    central_panel: Option<Arc<RwLock<CentralPanel>>>,

    /// We store a built BSP here (instead of the Document).
    bsp_level: Option<Arc<BspLevel>>,

    last_vertex_id: Option<usize>,
}

impl Editor {
    /// Create an editor with the given Document.
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        Self {
            document: Some(document),
            current_tool: Tool::Select,
            command_history: Vec::new(),
            redo_stack: Vec::new(),
            status_message: String::new(),
            error_message: None,
            show_side_panel: true,
            show_bsp_debug: false,
            central_panel: None,
            bsp_level: None,
            last_vertex_id: None,
        }
    }

    /// Attach a central panel for pan/zoom usage.
    pub fn attach_central_panel(&mut self, central_panel: Arc<RwLock<CentralPanel>>) {
        self.central_panel = Some(central_panel);
    }

    /// Returns the Document arc if any.
    pub fn document(&self) -> Option<Arc<RwLock<Document>>> {
        self.document.clone()
    }

    /// Sets a new Document, discarding the old one.
    pub fn set_document(&mut self, document: Arc<RwLock<Document>>) {
        self.document = Some(document);
        self.error_message = None;
    }

    /// Returns the current tool.
    pub fn current_tool(&self) -> Tool {
        self.current_tool
    }

    /// Sets the current tool and logs a status message.
    pub fn set_current_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        self.status_message = format!("Selected tool: {}", tool.name());
    }

    /// Returns a list of all available tools.
    pub fn available_tools(&self) -> Vec<Tool> {
        vec![
            Tool::Select,
            Tool::DrawLine,
            Tool::DrawShape,
            Tool::EditThings,
            Tool::EditSectors,
        ]
    }

    /// A minimal way to get a selected object (always returns the first vertex, if any).
    /// Real logic is needed for real selection code.
    pub fn selected_object(&self) -> Selection {
        if let Some(doc_arc) = &self.document {
            let doc = doc_arc.read();
            let verts = doc.vertices.read();
            if let Some(first_v) = verts.first() {
                return Selection::Vertex((**first_v).clone());
            }
        }
        Selection::None
    }

    /// Execute a command, handle errors, and reset redo stack on success.
    pub fn execute_command(&mut self, mut command: Box<dyn Command>) { // Take ownership
        if let Some(doc_arc) = &self.document {
            let mut doc = doc_arc.write();
            match command.execute(&mut doc) { // Execute on the mutable reference
                Ok(_) => {
                    self.command_history.push(command);  // Now push the potentially modified command
                    self.redo_stack.clear();
                }
                Err(err) => {
                    self.error_message = Some(format!("Error executing command: {}", err));
                }
            }
        } else {
            self.error_message = Some("No document to execute command on.".to_string());
        }
    }

    /// Undo the last command, if any.
    pub fn undo(&mut self) {
        if let Some(mut cmd) = self.command_history.pop() {
            if let Some(doc_arc) = &self.document {
                let mut doc = doc_arc.write();
                if let Err(err) = cmd.unexecute(&mut doc) {
                    self.error_message = Some(format!("Error undoing command: {}", err));
                } else {
                    self.redo_stack.push(cmd);
                }
            }
        }
    }

    /// Redo the last undone command, if any.
    pub fn redo(&mut self) {
        if let Some(mut cmd) = self.redo_stack.pop() {
            if let Some(doc_arc) = &self.document {
                let mut doc = doc_arc.write();
                if let Err(err) = cmd.execute(&mut doc) {
                    self.error_message = Some(format!("Error redoing command: {}", err));
                } else {
                    self.command_history.push(cmd);
                }
            }
        }
    }

    // ----------------- Document Management  -----------------

    /// Create a brand new, empty document.
    pub fn new_document(&mut self) {
        self.document = Some(Arc::new(RwLock::new(Document::new())));
        self.command_history.clear();
        self.redo_stack.clear();
        self.status_message = "Created new document.".to_string();
        self.error_message = None;
    }

    /// A convenience wrapper for saving, logs on error.
    pub fn save_document_wrapper(&mut self) {
        match self.save_document() {
            Ok(_) => {
                self.status_message = "Document saved.".to_string();
            }
            Err(e) => {
                error!("Failed to save document: {}", e);
                self.error_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    /// Actually saves the document (placeholder).
    pub fn save_document(&self) -> Result<(), String> {
        if let Some(doc_arc) = &self.document {
            let _doc = doc_arc.read();
            // Real logic needed here
            info!("(Placeholder) Document saved.");
            Ok::<(), String>(())
        } else {
            Err("No document available to save.".into())
        }
    }

    /// Opens a file dialog to pick a WAD, and loads it, replacing the doc if successful.
    pub fn show_open_dialog(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAD Files", &["wad"]).pick_file() {
            let path_str = path.to_string_lossy().to_string();
            match File::open(&path) {
                Ok(mut file) => {
                    let mut new_doc = Document::new();
                    if let Err(e) = new_doc.load_wad(&mut file) {
                        error!("WAD load error: {}", e);
                        self.error_message = Some(format!("Failed to load WAD: {}", e));
                    } else {
                        self.document = Some(Arc::new(RwLock::new(new_doc)));
                        self.command_history.clear();
                        self.redo_stack.clear();
                        self.status_message = format!("Loaded WAD file: {}", path_str);
                        self.error_message = None;
                    }
                }
                Err(e) => {
                    error!("File open error: {}", e);
                    self.error_message = Some(format!("Failed to open file {}: {}", path_str, e));
                }
            }
        }
    }

    /// Build the BSP, sets show_bsp_debug to true if successful.
    pub fn build_nodes_wrapper(&mut self) {
        match self.build_nodes() {
            Ok(_) => {
                self.status_message = "BSP built successfully.".to_string();
                self.show_bsp_debug = true;
            }
            Err(e) => {
                error!("BSP build error: {}", e);
                self.error_message = Some(format!("BSP build error: {}", e));
            }
        }
    }

    /// Actually create the BspLevel from the doc, store in self.bsp_level.
    pub fn build_nodes(&mut self) -> Result<(), String> {
        let doc_arc = match &self.document {
            Some(arc) => arc,
            None => return Err("No document loaded!".into()),
        };

        let bsp = BspLevel::new(doc_arc.clone());
        bsp.build()?; // short-circuits on error
        self.bsp_level = Some(Arc::new(bsp));
        Ok(())
    }

    /// Example placeholder for test map creation.
    pub fn generate_test_map(&mut self) {
        if self.document.is_none() {
            self.error_message = Some("No document loaded.".to_string());
            return;
        }
        // Real logic needed
        self.status_message = "Generated test map (placeholder).".into();
    }

    /// Cancels the current operation, e.g. if user was drawing lines.
    pub fn cancel_current_operation(&mut self) {
        self.current_tool = Tool::Select;
        // You could also clear selection or partial geometry if needed
        // self.clear_selection();
        info!("Canceled current operation. Tool reset to Select.");
    }

    /// Loads a map level from the current WAD data.
    pub fn load_level_wrapper(&mut self, level: String) {
        // Create a new runtime for async operations
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let msg = format!("Failed to create async runtime: {}", e);
                error!("{}", msg);
                self.error_message = Some(msg);
                return;
            }
        };

        // Run the async loading process in the runtime
        match runtime.block_on(async {
            // Early return if no document is present
            let doc_arc = self.document.as_ref()
                .ok_or_else(|| "No document present".to_string())?;

            // Extract WAD data under a smaller read lock scope
            let wad_data = {
                let doc_read = doc_arc.read(); // `mut` removed
                let wad_data_read = doc_read.wad_data.read();
                wad_data_read.clone()
                    .ok_or_else(|| "No WAD data stored for the current document".to_string())?
            }; // Locks are dropped here

            // Create cursor outside of any locks
            let mut cursor = Cursor::new(wad_data);

            // Load the level data under a write lock
            {
                let mut doc_write = doc_arc.write();
                doc_write.load_level_async(&level, &mut cursor)
                    .await
                    .map_err(|e| format!("Failed to load level {}: {}", level, e))?;
                
                // Calculate bounding box while we still have the write lock
                if let Some(bbox) = doc_write.bounding_box() {
                    if let Some(cp_arc) = &self.central_panel {
                        let center = bbox.center();
                        let mut cp = cp_arc.write();
                        cp.set_zoom(1.0);
                        cp.set_pan(egui::vec2(-center.x, -center.y));
                    }
                }
            }

            Ok::<(), String>(()) // Explicit Ok with type
        }) {
            Ok(_) => {
                self.status_message = format!("Loaded level: {}", level);
                self.error_message = None;
                info!("Loaded level {} successfully.", level);
            }
            Err(e) => {
                let msg = e.to_string();
                error!("{}", msg);
                self.error_message = Some(msg);
            }
        }
    }

    /// Helper method to update the view center, separated for clarity
    /// and potential reuse
    fn _update_view_center(&self, center: egui::Pos2) -> Result<(), String> {
        if let Some(cp_arc) = &self.central_panel {
            let mut cp = cp_arc.write();
            cp.set_zoom(1.0);
            cp.set_pan(egui::vec2(-center.x, -center.y));
            Ok::<(), String>(())
        } else {
            Err("No central panel available".to_string())
        }
    }

    /// Return the BSP if built
    pub fn bsp_level(&self) -> Option<Arc<BspLevel>> {
        self.bsp_level.clone()
    }

    /// Convert from screen to world coords using the central panel's pan/zoom.
    pub fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        if let Some(cp_arc) = &self.central_panel {
            let cp = cp_arc.read();
            let zoom = cp.get_zoom();
            let pan = cp.get_pan();
            return egui::pos2(
                (screen_pos.x - pan.x) / zoom,
                (screen_pos.y - pan.y) / zoom,
            );
        }
        screen_pos // fallback: no transform
    }

    /// Handles raw input events from the CentralPanel.
    /// Handles raw input events from the CentralPanel.
    pub fn handle_input(
        &mut self,
        world_pos: egui::Pos2,
        primary_clicked: bool,
        secondary_clicked: bool,
        middle_clicked: bool,
        is_dragging: bool,
        drag_delta: egui::Vec2,
        modifiers: egui::Modifiers,
    ) {
        match self.current_tool {
            Tool::Select => {
                if primary_clicked {
                    // Selection logic (using world_pos) goes here.
                    println!("Select tool click at: {:?}", world_pos);
                }
            }
            Tool::DrawLine => {
                if primary_clicked {
                    // First click: add a vertex.
                    let cmd = CommandType::AddVertex {
                        x: world_pos.x as i32,
                        y: world_pos.y as i32,
                        vertex_id: None,
                    };
                    self.execute_command(Box::new(cmd));
                    self.last_vertex_id = self.document.as_ref().and_then(|doc| {
                        let doc = doc.read();
                        let vertices = doc.vertices.read();
                        vertices.last().map(|_| vertices.len() - 1)
                    });
                } else if is_dragging && self.last_vertex_id.is_some(){
                    // While dragging: add a vertex AND a linedef.
                    let vertex_cmd = CommandType::AddVertex {
                        x: world_pos.x as i32,
                        y: world_pos.y as i32,
                        vertex_id: None,
                    };
                    
                    let start_vertex_id = self.last_vertex_id.unwrap();
                    let mut end_vertex_id: Option<usize> = None; // store new vertex id here
                    
                    // Use batch command
                    let batch_cmd = CommandType::BatchCommand{
                        commands: vec![
                            vertex_cmd,
                            CommandType::AddLineDef {
                                start_vertex_id,
                                end_vertex_id: 0, // Temporary value, will be updated
                                right_side_sector_id: -1,  //  placeholders
                                left_side_sector_id: -1,   //  placeholders
                                linedef_id: None,
                            }
                        ]
                    };
                    self.execute_command(Box::new(batch_cmd));
                    
                    self.last_vertex_id = self.document.as_ref().and_then(|doc| {
                        let doc = doc.read();
                        let vertices = doc.vertices.read();
                        vertices.last().map(|_| vertices.len() - 1)
                    });
                }

                if secondary_clicked {
                    self.cancel_current_operation();
                }
            }
            Tool::DrawShape => {
                // Implement shape drawing logic.
                if primary_clicked {
                    println!("Draw Shape tool click at: {:?}", world_pos);
                }
            }
            Tool::EditThings => {
                // Implement thing editing logic (add, move, delete).
                 if primary_clicked {
                    println!("Edit things click at: {:?}", world_pos);
                }
            }
            Tool::EditSectors => {
                // Implement sector editing logic.
                 if primary_clicked {
                    println!("Edit sectors click at: {:?}", world_pos);
                }
            }
        }
        if primary_clicked {
            println!("Primary click at: {:?}", world_pos);
        }
        if secondary_clicked {
            println!("Secondary click at: {:?}", world_pos);
        }
        if middle_clicked {
            println!("Middle click at: {:?}", world_pos);
        }
        if is_dragging {
            // Example: If dragging with the Select tool, you might
            // implement rubber-band selection.
            println!("Dragging: delta {:?}", drag_delta);
        }
         if modifiers.shift {
            println!("Shift key pressed");
        }
        if modifiers.ctrl {
            println!("Ctrl key pressed");
        }
        if modifiers.alt {
            println!("Alt key pressed");
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        // Implement real logic for tracking changes
        false
    }
}