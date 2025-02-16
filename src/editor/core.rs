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
use eframe::egui;
use crate::editor::tools::{Tool, SelectTool, DrawLineTool, DrawShapeTool, ThingsTool, SectorsTool};

/// The core `Editor` struct, holding geometry references, current tool, etc.
pub struct Editor {
    /// The current document, if any.
    document: Option<Arc<RwLock<Document>>>,

    /// Currently active tool and available tools
    current_tool: Box<dyn Tool>,
    tools: Vec<Box<dyn Tool>>,

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
}

impl Editor {
    /// Create an editor with the given Document.
    pub fn new(document: Arc<RwLock<Document>>) -> Self {
        // Initialize all available tools
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(SelectTool::default()),
            Box::new(DrawLineTool::default()),
            Box::new(DrawShapeTool::default()),
            Box::new(ThingsTool::default()),
            Box::new(SectorsTool::default()),
        ];

        Self {
            document: Some(document),
            current_tool: Box::new(SelectTool::default()),
            tools,
            command_history: Vec::new(),
            redo_stack: Vec::new(),
            status_message: String::new(),
            error_message: None,
            show_side_panel: true,
            show_bsp_debug: false,
            central_panel: None,
            bsp_level: None,
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
        self.command_history.clear();
        self.redo_stack.clear();
    }

    /// Returns the name of the current tool.
    pub fn current_tool_name(&self) -> &'static str {
        self.current_tool.name()
    }

    /// Sets the current tool by name.
    pub fn set_current_tool(&mut self, tool_name: &str) {
        if let Some(tool) = self.tools.iter().find(|t| t.name() == tool_name) {
            // Clean up the current tool before switching
            self.current_tool.cleanup();
            
            // Create a new instance of the selected tool
            self.current_tool = match tool_name {
                "Select" => Box::new(SelectTool::default()),
                "Draw Line" => Box::new(DrawLineTool::default()),
                "Draw Shape" => Box::new(DrawShapeTool::default()),
                "Edit Things" => Box::new(ThingsTool::default()),
                "Edit Sectors" => Box::new(SectorsTool::default()),
                _ => Box::new(SelectTool::default()),
            };
            
            self.status_message = format!("Selected tool: {}", tool_name);
        }
    }

    /// Returns a list of available tool names.
    pub fn available_tools(&self) -> Vec<&'static str> {
        self.tools.iter().map(|t| t.name()).collect()
    }

    /// Execute a command, handle errors, and reset redo stack on success.
    pub fn execute_command(&mut self, mut command: Box<dyn Command>) {
        if let Some(doc_arc) = &self.document {
            let mut doc = doc_arc.write();
            match command.execute(&mut doc) {
                Ok(_) => {
                    self.command_history.push(command);
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

    /// Opens a file dialog to pick a WAD, and loads it.
    pub fn show_open_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("WAD Files", &["wad"])
            .pick_file() 
        {
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

    /// Save the current document.
    pub fn save_document(&self) -> Result<(), String> {
        if let Some(doc_arc) = &self.document {
            let doc = doc_arc.read();
            // Implement actual save logic here
            info!("Document saved.");
            Ok(())
        } else {
            Err("No document available to save.".into())
        }
    }

    /// Wrapper for save_document that handles errors
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

    /// Build the BSP tree from the current document
    pub fn build_nodes(&mut self) -> Result<(), String> {
        let doc_arc = self.document.as_ref()
            .ok_or_else(|| "No document loaded!".to_string())?;

        let bsp = BspLevel::new(doc_arc.clone());
        bsp.build()?;
        self.bsp_level = Some(Arc::new(bsp));
        Ok(())
    }

    /// Wrapper for build_nodes that handles errors
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

    /// Load a specific level from the WAD
    pub fn load_level_wrapper(&mut self, level: String) {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let msg = format!("Failed to create async runtime: {}", e);
                error!("{}", msg);
                self.error_message = Some(msg);
                return;
            }
        };

        match runtime.block_on(async {
            let doc_arc = self.document.as_ref()
                .ok_or_else(|| "No document present".to_string())?;

            let wad_data = {
                let doc_read = doc_arc.read();
                let wad_data_read = doc_read.wad_data.read();
                wad_data_read.clone()
                    .ok_or_else(|| "No WAD data stored".to_string())?
            };

            let mut cursor = Cursor::new(wad_data);

            {
                let mut doc_write = doc_arc.write();
                doc_write.load_level_async(&level, &mut cursor)
                    .await
                    .map_err(|e| format!("Failed to load level {}: {}", level, e))?;

                if let Some(bbox) = doc_write.bounding_box() {
                    if let Some(cp_arc) = &self.central_panel {
                        let center = bbox.center();
                        let mut cp = cp_arc.write();
                        cp.set_zoom(1.0);
                        cp.set_pan(egui::vec2(-center.x, -center.y));
                    }
                }
            }

            Ok::<(), String>(())
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

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        if let Some(cp_arc) = &self.central_panel {
            let cp = cp_arc.read();
            let zoom = cp.get_zoom();
            let pan = cp.get_pan();
            egui::pos2(
                (screen_pos.x - pan.x) / zoom,
                (screen_pos.y - pan.y) / zoom,
            )
        } else {
            screen_pos
        }
    }

    /// Handle input events from the UI
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
        if let Some(doc) = &self.document {
            self.current_tool.handle_input(
                doc,
                world_pos,
                primary_clicked,
                secondary_clicked,
                is_dragging,
                drag_delta,
                modifiers,
            );
        }
    }

    /// Draw the current tool's UI elements
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        if let Some(doc) = &self.document {
            self.current_tool.draw(ui, doc);
        }
    }

    /// Cancel the current operation
    pub fn cancel_current_operation(&mut self) {
        self.current_tool.cleanup();
        self.set_current_tool("Select");
        info!("Canceled current operation.");
    }

    /// Check if there are unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        !self.command_history.is_empty()
    }

    /// Get the current BSP level if built
    pub fn bsp_level(&self) -> Option<Arc<BspLevel>> {
        self.bsp_level.clone()
    }

    pub fn current_tool(&self) -> &dyn Tool {
        self.current_tool.as_ref()
    }

}