// src/editor/editor.rs

use std::fs::File;
use std::sync::Arc;
use parking_lot::RwLock;
use log::error;
use rfd::FileDialog;

use crate::bsp::BspLevel;
use crate::document::Document;
use crate::editor::commands::{Command, CommandType};
use crate::ui::central_panel::CentralPanel;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Tool {
    Select,
    DrawLine,
    DrawShape, // For drawing rectangular sectors
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

#[derive(Clone, Debug)]
pub enum Selection {
    Vertex(crate::map::Vertex),
    Line(crate::map::LineDef),
    Sector(crate::map::Sector),
    Thing(crate::map::Thing),
    None,
}

pub struct Editor {
    document: Option<Arc<RwLock<Document>>>,
    current_tool: Tool,
    command_history: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub show_side_panel: bool,
    pub show_bsp_debug: bool,

    /// A handle to the central panel for view/pan/zoom, if needed
    central_panel: Option<Arc<RwLock<CentralPanel>>>,

    /// Where we store the BSP once built (instead of storing it in Document).
    bsp_level: Option<Arc<BspLevel>>,
}

impl Editor {
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
        }
    }

    pub fn attach_central_panel(&mut self, central_panel: Arc<RwLock<CentralPanel>>) {
        self.central_panel = Some(central_panel);
    }

    pub fn document(&self) -> Option<Arc<RwLock<Document>>> {
        self.document.clone()
    }

    pub fn set_document(&mut self, document: Arc<RwLock<Document>>) {
        self.document = Some(document);
    }

    pub fn current_tool(&self) -> Tool {
        self.current_tool
    }

    pub fn set_current_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        self.status_message = format!("Selected tool: {}", tool.name());
    }

    pub fn available_tools(&self) -> Vec<Tool> {
        vec![
            Tool::Select,
            Tool::DrawLine,
            Tool::DrawShape,
            Tool::EditThings,
            Tool::EditSectors,
        ]
    }

    pub fn selected_object(&self) -> Selection {
        if let Some(doc) = &self.document {
            let doc = doc.read();
            let vertices = doc.vertices.read();
            if let Some(vertex) = vertices.first() {
                return Selection::Vertex((**vertex).clone());
            }
        }
        Selection::None
    }

    pub fn execute_command(&mut self, command: Box<dyn Command>) {
        if let Some(doc) = &self.document {
            let mut doc = doc.write();
            if let Err(err) = command.execute(&mut doc) {
                self.error_message = Some(format!("Error executing command: {}", err));
            } else {
                self.command_history.push(command);
                self.redo_stack.clear(); // Clear redo stack after a new command.
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(mut command) = self.command_history.pop() {
            if let Some(doc) = &self.document {
                let mut doc = doc.write();
                if let Err(err) = command.unexecute(&mut doc) {
                    self.error_message = Some(format!("Error undoing command: {}", err));
                } else {
                    self.redo_stack.push(command);
                }
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(mut command) = self.redo_stack.pop() {
            if let Some(doc) = &self.document {
                let mut doc = doc.write();
                if let Err(err) = command.execute(&mut doc) {
                    self.error_message = Some(format!("Error redoing command: {}", err));
                } else {
                    self.command_history.push(command);
                }
            }
        }
    }

    // --- Document Management Methods ---

    pub fn new_document(&mut self) {
        self.document = Some(Arc::new(RwLock::new(Document::new())));
        self.status_message = "Created new document".to_string();
        self.command_history.clear();
        self.redo_stack.clear();
    }

    pub fn save_document_wrapper(&mut self) {
        match self.save_document() {
            Ok(_) => self.status_message = "Document saved".to_string(),
            Err(e) => {
                error!("Failed to save document: {}", e);
                self.error_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    pub fn save_document(&self) -> Result<(), String> {
        if let Some(doc_arc) = &self.document {
            let _doc = doc_arc.read();
            // Implement actual WAD saving logic
            println!("Document saved (placeholder).");
            Ok(())
        } else {
            Err("No document to save".to_string())
        }
    }

    /// Opens a native file dialog to choose a WAD file, loads it, and updates the document.
    pub fn show_open_dialog(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAD Files", &["wad"]).pick_file() {
            let path_str = path.to_string_lossy().to_string();
            match File::open(&path) {
                Ok(mut file) => {
                    let mut new_doc = Document::new();
                    if let Err(e) = new_doc.load_wad(&mut file) {
                        self.error_message = Some(format!("Failed to load WAD: {}", e));
                        error!("WAD load error: {}", e);
                    } else {
                        self.document = Some(Arc::new(RwLock::new(new_doc)));
                        self.status_message = format!("Loaded WAD file: {}", path_str);
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to open file {}: {}", path_str, e));
                    error!("File open error: {}", e);
                }
            }
        }
    }

    /// Wraps building the BSP so we can handle errors gracefully.
    pub fn build_nodes_wrapper(&mut self) {
        match self.build_nodes() {
            Ok(_) => {
                self.status_message = "BSP nodes built successfully".to_string();
                self.show_bsp_debug = true;
            }
            Err(e) => {
                error!("Failed to build nodes: {}", e);
                self.error_message = Some(format!("Node building failed: {}", e));
            }
        }
    }

    /// Actually builds the BSP, storing it in this Editor.
    pub fn build_nodes(&mut self) -> Result<(), String> {
        if let Some(doc_arc) = &self.document {
            let bsp_level = BspLevel::new(doc_arc.clone());
            bsp_level.build()?; // If error, short-circuits

            // Store in the editor
            self.bsp_level = Some(Arc::new(bsp_level));

            Ok(())
        } else {
            Err("No document loaded".to_string())
        }
    }

    pub fn generate_test_map(&mut self) {
        if let Some(_doc_arc) = &self.document {
            // let mut _doc = doc_arc.write();
            // Possibly call some doc-level generator. For now, no-op:
            self.status_message = "Generated test map (placeholder)".to_string();
        } else {
            self.error_message = Some("No document loaded".to_string());
        }
    }

    pub fn cancel_current_operation(&mut self) {
        self.current_tool = Tool::Select;
    }

    pub fn load_level_wrapper(&mut self, level: String) {
        if let Some(doc_arc) = &self.document {
            let wad_data_opt = {
                let doc = doc_arc.read();
                {
                    let wad_data = doc.wad_data.read();
                    wad_data.clone()
                }
            };

            if let Some(wad_data) = wad_data_opt {
                let mut cursor = std::io::Cursor::new(wad_data);
                match doc_arc.write().load_level(&level, &mut cursor) {
                    Ok(_) => {
                        self.status_message = format!("Loaded level: {}", level);
                        // Re-center view
                        if let Some(bbox) = doc_arc.read().bounding_box() {
                            let center = bbox.center();
                            if let Some(central_panel) = &self.central_panel {
                                let mut cp = central_panel.write();
                                let zoom = cp.get_zoom();
                                // Hypothetical methods to safely modify these private fields:
                                cp.set_zoom(1.0);
                                cp.set_pan(egui::vec2(
                                    -center.x * zoom,
                                    -center.y * zoom,
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to load level {}: {}", level, e));
                        error!("Failed to load level {}: {}", level, e);
                    }
                }
            } else {
                self.error_message = Some("No WAD data stored".to_string());
                error!("No WAD data stored");
            }
        }
    }

    /// Returns the BSP, if built.
    pub fn bsp_level(&self) -> Option<Arc<BspLevel>> {
        self.bsp_level.clone()
    }

    /// Uses the editor's central panel to convert screen coordinates to world coordinates.
    pub fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        if let Some(central_panel) = &self.central_panel {
            let cp = central_panel.read();
            let zoom = cp.get_zoom(); // We assume there's a public accessor
            let pan = cp.get_pan();   // Similarly, a public accessor
            return egui::pos2(
                (screen_pos.x - pan.x) / zoom,
                (screen_pos.y - pan.y) / zoom,
            );
        }
        egui::Pos2::default()
    }

    pub fn handle_click(&mut self, world_pos: egui::Pos2) {
        match self.current_tool {
            Tool::DrawLine => {
                // Example: Add a vertex at the clicked position.
                let cmd = CommandType::AddVertex {
                    x: world_pos.x as i32,
                    y: world_pos.y as i32,
                    vertex_id: None,
                };
                self.execute_command(Box::new(cmd));
            }
            Tool::Select => {
                // TODO: implement selection logic
            }
            _ => {}
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        // Real logic goes here if you track changes
        false
    }
}
