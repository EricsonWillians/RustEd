// src/ui/main_window.rs

use std::sync::Arc;
use std::fs::File;

use eframe::egui::{self, Context, Sense, Vec2, Ui};
use log::{info, error};
use parking_lot::RwLock;
use rfd::FileDialog; // For file dialogs

use crate::editor::commands::{Command, CommandType};
use crate::editor::Selection;
use crate::{
    bsp::debug_viz::BspDebugger,
    document::Document,
    editor::{Editor, Tool},
    ui::{DialogManager, Theme},
};

/// MainWindow configuration settings.
#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub default_width: u32,
    pub default_height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub theme: Theme,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            default_width: 1280,
            default_height: 800,
            min_width: 800,
            min_height: 600,
            theme: Theme::default(),
        }
    }
}

/// Application window state and UI management.
pub struct MainWindow {
    // Core state
    config: WindowConfig,
    editor: Arc<RwLock<Editor>>,
    dialog_manager: DialogManager,

    // Debug tools
    bsp_debugger: BspDebugger,
    show_bsp_debug: bool,

    // UI state
    show_side_panel: bool,
    side_panel_width: f32,
    status_message: String,
    error_message: Option<String>,

    // Camera / view parameters (for world-to-screen mapping)
    zoom: f32,
    pan: egui::Vec2,
}

impl MainWindow {
    /// Creates a new MainWindow with the given configuration.
    pub fn new(config: WindowConfig) -> Self {
        info!(
            "Initializing main window {}x{}",
            config.default_width, config.default_height
        );

        // 1) Create an Arc<RwLock<Document>>
        let doc = Arc::new(RwLock::new(Document::new()));

        // 2) Create Editor with that Document
        let editor = Arc::new(RwLock::new(Editor::new(doc)));

        // 3) Build the MainWindow
        Self {
            config: config.clone(),
            editor,
            dialog_manager: DialogManager::new(),
            bsp_debugger: BspDebugger::new(),
            show_bsp_debug: false,
            show_side_panel: true,
            side_panel_width: 250.0,
            status_message: String::new(),
            error_message: None,
            zoom: 1.0,
            pan: egui::vec2(0.0, 0.0),
        }
    }

    /// Main UI update loop.
    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.handle_input(ctx);
        self.update_layout(ctx, frame);
    }

    // ------------------------------------------------------------------------
    // Input Handling
    // ------------------------------------------------------------------------
    fn handle_input(&mut self, ctx: &Context) {
        let input = ctx.input();

        // Toggle BSP debug window with F11.
        if input.key_pressed(egui::Key::F11) {
            self.show_bsp_debug = !self.show_bsp_debug;
        }

        // Cancel current operation with ESC.
        if input.key_pressed(egui::Key::Escape) {
            let mut editor = self.editor.write();
            editor.cancel_current_operation();
        }

        // Example: Ctrl + A = add a vertex at pointer location.
        if (input.modifiers.ctrl || input.modifiers.mac_cmd) && input.key_pressed(egui::Key::A) {
            if let Some(pos) = input.pointer.hover_pos() {
                let world_pos = self.screen_to_world(pos);
                let cmd = CommandType::AddVertex {
                    x: world_pos.x as i32,
                    y: world_pos.y as i32,
                    vertex_id: None,
                };
                let mut editor = self.editor.write();
                editor.execute_command(Box::new(cmd) as Box<dyn Command>);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Overall Layout
    // ------------------------------------------------------------------------
    fn update_layout(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.update_menu_bar(ctx);
        self.update_side_panel(ctx);
        self.update_central_area(ctx);
        self.update_status_bar(ctx);
        self.update_dialogs(ctx);

        if self.show_bsp_debug {
            self.show_bsp_debug_window(ctx);
        }

        if let Some(ref error) = self.error_message {
            let error_clone = error.clone();
            self.show_error_dialog(ctx, &error_clone);
        }
    }

    // ------------------------------------------------------------------------
    // Menu Bar
    // ------------------------------------------------------------------------
    fn update_menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // FILE menu.
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_document();
                    }
                    if ui.button("Open...").clicked() {
                        self.show_open_dialog();
                    }
                    if ui.button("Save").clicked() {
                        self.save_document();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        self.request_exit();
                    }
                });
                // EDIT menu.
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        self.undo();
                    }
                    if ui.button("Redo").clicked() {
                        self.redo();
                    }
                });
                // VIEW menu.
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_side_panel, "Tools Panel");
                    ui.checkbox(&mut self.show_bsp_debug, "BSP Debug");
                });
                // TOOLS menu.
                ui.menu_button("Tools", |ui| {
                    if ui.button("Build Nodes").clicked() {
                        self.build_nodes();
                    }
                    if ui.button("Generate Test Map").clicked() {
                        self.generate_test_map();
                    }
                });
                // HELP menu.
                ui.menu_button("Help", |ui| {
                    if ui.button("About...").clicked() {
                        self.show_about_dialog();
                    }
                });
            });
        });
    }

    // ------------------------------------------------------------------------
    // Left Side Tools Panel (including Level Selection)
    // ------------------------------------------------------------------------
    fn update_side_panel(&mut self, ctx: &Context) {
        if !self.show_side_panel {
            return;
        }
        egui::SidePanel::left("tools_panel")
            .default_width(self.side_panel_width)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Tools");
                {
                    let editor = self.editor.read();
                    for tool in editor.available_tools() {
                        let selected = editor.current_tool() == tool;
                        if ui.selectable_label(selected, tool.name()).clicked() {
                            drop(editor);
                            self.select_tool(tool);
                            return;
                        }
                    }
                }
                ui.separator();
                ui.heading("Levels");
                // Level selection UI.
                if let Some(doc_arc) = self.editor.read().document() {
                    let levels = {
                        let doc = doc_arc.read();
                        let lvl_list = doc.available_levels();
                        info!("Detected levels: {:?}", lvl_list);
                        lvl_list
                    };
                    if levels.is_empty() {
                        ui.label("No levels found. Check your WAD file.");
                    } else {
                        for level in &levels {
                            ui.horizontal(|ui| {
                                if ui.button(level).clicked() {
                                    info!("Reloading level: {}", level);
                                    // Drop any locks before reloading.
                                    if let Some(doc_arc) = self.editor.read().document() {
                                        // Use the stored wad_data to create a new cursor.
                                        let wad_data_opt = {
                                            let doc = doc_arc.read();
                                            let wad_data = doc.wad_data.read().clone();
                                            drop(doc);
                                            wad_data
                                        };
                                        if let Some(wad_data) = wad_data_opt {
                                            let mut cursor = std::io::Cursor::new(wad_data);
                                            match doc_arc.write().load_level(level, &mut cursor) {
                                                Ok(_) => {
                                                    self.status_message = format!("Loaded level: {}", level);
                                                    info!("Successfully loaded level: {}", level);
                                                    // Recenter view:
                                                    if let Some(bbox) = doc_arc.read().bounding_box() {
                                                        let center = bbox.center();
                                                        // Reset zoom (to 1.0 for now) and set pan so that the level center is at (0,0)
                                                        self.zoom = 1.0;
                                                        self.pan = egui::vec2(-center.x * self.zoom, -center.y * self.zoom);
                                                    }
                                                },
                                                Err(e) => {
                                                    self.error_message = Some(format!("Failed to load level {}: {}", level, e));
                                                    error!("Failed to load level {}: {}", level, e);
                                                }
                                            }
                                        } else {
                                            self.error_message = Some("No WAD data stored".to_string());
                                            error!("No WAD data stored");
                                        }
                                    } else {
                                        self.error_message = Some("No document loaded".to_string());
                                        error!("No document loaded");
                                    }
                                }
                            });
                        }
                    }
                } else {
                    ui.label("No document loaded");
                }
                ui.separator();
                ui.heading("Properties");
                let selection = self.editor.read().selected_object();
                self.show_property_editor(ui, &selection);
            });
    }

    // ------------------------------------------------------------------------
    // Central Drawing Area with Pan & Zoom
    // ------------------------------------------------------------------------
    fn update_central_area(&mut self, ctx: &Context) {
        // Use a CentralPanel so that drawing is confined to the proper area.
        egui::CentralPanel::default().frame(egui::Frame::none().fill(egui::Color32::BLACK)).show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();

            // Create an interactive response covering the entire canvas.
            let response = ui.interact(rect, ui.id(), Sense::drag());

            // --- Zooming ---
            if response.hovered() {
                if ui.input().scroll_delta.y.abs() > 0.0 {
                    // Compute new zoom factor.
                    let old_zoom = self.zoom;
                    let zoom_sensitivity = 0.001;
                    let factor = 1.0 + ui.input().scroll_delta.y * zoom_sensitivity;
                    let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);

                    // Adjust pan so that the zoom is centered on the pointer.
                    if let Some(pointer) = ui.input().pointer.hover_pos() {
                        // World coordinate under the pointer before zoom.
                        let world_before = self.screen_to_world(pointer);
                        // Update zoom.
                        self.zoom = new_zoom;
                        // New pan so that world_before maps to the same screen position:
                        self.pan = pointer.to_vec2() - world_before.to_vec2() * self.zoom;
                    } else {
                        self.zoom = new_zoom;
                    }
                    ui.ctx().request_repaint();
                }
            }

            // --- Panning via drag ---
            if response.dragged() {
                self.pan += response.drag_delta();
                ui.ctx().request_repaint();
            }

            // Get a painter for the drawing area.
            let painter = ui.painter_at(rect);

            // Draw background.
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

            // Draw grid.
            self.draw_editor_grid(&painter, rect);

            // Draw map geometry.
            if let Some(doc_arc) = self.editor.read().document() {
                self.draw_map_geometry(&painter, doc_arc.clone(), rect);
                self.draw_vertices(&painter, doc_arc.clone(), rect);
                // (Optional) Extend here to draw sectors, things, etc.
            }

            // Handle mouse clicks on the canvas.
            if response.clicked() {
                if let Some(pos) = ui.input().pointer.interact_pos() {
                    self.handle_editor_click(pos);
                }
            }
        });
    }

    // ------------------------------------------------------------------------
    // Status Bar
    // ------------------------------------------------------------------------
    fn update_status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let coord_label = if let Some(pos) = ui.input().pointer.hover_pos() {
                let world_pos = self.screen_to_world(pos);
                format!("({}, {})", world_pos.x as i32, world_pos.y as i32)
            } else {
                String::new()
            };

            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.label(coord_label);
                let editor = self.editor.read();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Tool: {}", editor.current_tool().name()));
                });
            });
        });
    }

    // ------------------------------------------------------------------------
    // BSP Debug Window
    // ------------------------------------------------------------------------
    fn show_bsp_debug_window(&mut self, ctx: &Context) {
        egui::Window::new("BSP Debug")
            .resizable(true)
            .default_size(Vec2::new(800.0, 600.0))
            .show(ctx, |ui| {
                let editor = self.editor.read();
                if let Some(bsp_level) = editor.bsp_level() {
                    let root_guard = bsp_level.root.read();
                    if let Some(_root_node) = &*root_guard {
                        self.bsp_debugger.show(ui, &bsp_level);
                    } else {
                        ui.label("BSP root node not built yet.");
                    }
                    let subsectors_guard = bsp_level.subsectors.read();
                    ui.label(format!("Number of subsectors: {}", subsectors_guard.len()));
                    let blocks_guard = bsp_level.blocks.read();
                    ui.label(format!("Blockmap size: {}x{}", blocks_guard.width, blocks_guard.height));
                } else {
                    ui.label("No BSP data available.");
                    if ui.button("Generate Test Map").clicked() {
                        drop(editor);
                        self.generate_test_map();
                    }
                }
            });
    }

    // ------------------------------------------------------------------------
    // Commands & Actions
    // ------------------------------------------------------------------------
    fn new_document(&mut self) {
        let mut editor = self.editor.write();
        if editor.has_unsaved_changes() {
            self.dialog_manager.show_save_changes_dialog();
            return;
        }
        editor.new_document();
        self.status_message = "Created new document".to_string();
    }

    fn save_document(&mut self) {
        let mut editor = self.editor.write();
        match editor.save_document() {
            Ok(_) => self.status_message = "Document saved".to_string(),
            Err(e) => {
                error!("Failed to save document: {}", e);
                self.error_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    fn build_nodes(&mut self) {
        let mut editor = self.editor.write();
        match editor.build_nodes() {
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

    fn generate_test_map(&mut self) {
        let mut editor = self.editor.write();
        editor.generate_test_map();
        self.status_message = "Generated test map".to_string();
    }

    fn select_tool(&mut self, tool: Tool) {
        let mut editor = self.editor.write();
        editor.set_current_tool(tool);
        self.status_message = format!("Selected tool: {}", tool.name());
    }

    fn undo(&mut self) {
        let mut editor = self.editor.write();
        editor.undo();
    }

    fn redo(&mut self) {
        let mut editor = self.editor.write();
        editor.redo();
    }

    // ------------------------------------------------------------------------
    // File Dialog & WAD Loading
    // ------------------------------------------------------------------------
    /// Opens a native file dialog to choose a WAD file, loads it, and updates the document.
    fn show_open_dialog(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAD Files", &["wad"]).pick_file() {
            let path_str = path.to_string_lossy().to_string();
            match File::open(&path) {
                Ok(mut file) => {
                    let mut new_doc = Document::new();
                    if let Err(e) = new_doc.load_wad(&mut file) {
                        self.error_message = Some(format!("Failed to load WAD: {}", e));
                        error!("WAD load error: {}", e);
                    } else {
                        let mut editor = self.editor.write();
                        editor.set_document(Arc::new(RwLock::new(new_doc)));
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

    // ------------------------------------------------------------------------
    // Coordinate Transforms & Drawing
    // ------------------------------------------------------------------------
    /// Converts a point from world space to screen space.
    fn world_to_screen(&self, world_pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            world_pos.x * self.zoom + self.pan.x,
            world_pos.y * self.zoom + self.pan.y,
        )
    }

    /// Converts a point from screen space to world space.
    fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            (screen_pos.x - self.pan.x) / self.zoom,
            (screen_pos.y - self.pan.y) / self.zoom,
        )
    }

    /// Draws the map geometry by iterating over linedefs and drawing lines between vertices.
    fn draw_map_geometry(&self, painter: &egui::Painter, doc: Arc<RwLock<Document>>, _rect: egui::Rect) {
        let doc_read = doc.read();
        let vertices = doc_read.vertices.read();
        let linedefs = doc_read.linedefs.read();
        for linedef in linedefs.iter() {
            if linedef.start < vertices.len() && linedef.end < vertices.len() {
                let start_vertex = &vertices[linedef.start];
                let end_vertex = &vertices[linedef.end];
                let p1 = self.world_to_screen(egui::Pos2 {
                    x: start_vertex.raw_x as f32,
                    y: start_vertex.raw_y as f32,
                });
                let p2 = self.world_to_screen(egui::Pos2 {
                    x: end_vertex.raw_x as f32,
                    y: end_vertex.raw_y as f32,
                });
                painter.line_segment([p1, p2], egui::Stroke::new(1.5, egui::Color32::GREEN));
            }
        }
    }

    /// Draws vertices as small circles for better visual feedback.
    fn draw_vertices(&self, painter: &egui::Painter, doc: Arc<RwLock<Document>>, _rect: egui::Rect) {
        let doc_read = doc.read();
        let vertices = doc_read.vertices.read();
        for vertex in vertices.iter() {
            let pos = self.world_to_screen(egui::pos2(vertex.raw_x as f32, vertex.raw_y as f32));
            painter.circle_filled(pos, 3.0, egui::Color32::RED);
        }
    }

    /// Draws a background grid on the editor canvas in world coordinates.
    fn draw_editor_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_spacing_world = 50.0; // Constant spacing in world units.
        // Convert screen rect to world coordinates.
        let world_min = self.screen_to_world(rect.min);
        let world_max = self.screen_to_world(rect.max);
        // Compute starting positions.
        let start_x = (world_min.x / grid_spacing_world).floor() * grid_spacing_world;
        let start_y = (world_min.y / grid_spacing_world).floor() * grid_spacing_world;
        let stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(40));

        let mut x = start_x;
        while x <= world_max.x {
            let p1 = self.world_to_screen(egui::pos2(x, world_min.y));
            let p2 = self.world_to_screen(egui::pos2(x, world_max.y));
            painter.line_segment([p1, p2], stroke);
            x += grid_spacing_world;
        }

        let mut y = start_y;
        while y <= world_max.y {
            let p1 = self.world_to_screen(egui::pos2(world_min.x, y));
            let p2 = self.world_to_screen(egui::pos2(world_max.x, y));
            painter.line_segment([p1, p2], stroke);
            y += grid_spacing_world;
        }
    }

    /// Stub: handle user clicks on the editor canvas.
    fn handle_editor_click(&self, pos: egui::Pos2) {
        // TODO: implement object picking/selecting based on pos.
    }

    // ------------------------------------------------------------------------
    // Dialogs and Misc.
    // ------------------------------------------------------------------------
    fn update_dialogs(&mut self, _ctx: &Context) {
        // Implement additional dialogs as needed.
    }

    fn show_property_editor(&mut self, ui: &mut Ui, selection: &Selection) {
        match selection {
            Selection::Vertex(vertex) => {
                ui.label(format!("Vertex: ({}, {})", vertex.raw_x, vertex.raw_y));
            }
            Selection::Line(linedef) => {
                ui.label(format!("Linedef: {} to {}", linedef.start, linedef.end));
            }
            Selection::Sector(sector) => {
                ui.label(format!("Sector: floor {}, ceiling {}", sector.floorh, sector.ceilh));
            }
            Selection::Thing(thing) => {
                ui.label(format!("Thing: type {}", thing.thing_type));
            }
            Selection::None => {
                ui.label("Nothing selected");
            }
        }
    }

    fn show_error_dialog(&mut self, ctx: &Context, message: &str) {
        egui::Window::new("Error")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(message);
                if ui.button("OK").clicked() {
                    self.error_message = None;
                }
            });
    }

    fn show_open_dialog_placeholder(&mut self) { /* Not used */ }
    fn show_save_changes_dialog(&mut self) { /* Implement if needed */ }
    fn request_exit(&mut self) { /* Implement if needed */ }
    fn show_about_dialog(&mut self) { /* Implement if needed */ }
}

// ------------------------------------------------------------------------
// Optional tests for MainWindow (if needed)
// ------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_window_creation() {
        let config = WindowConfig::default();
        let window = MainWindow::new(config);
        assert!(window.show_side_panel);
        assert!(!window.show_bsp_debug);
    }

    #[test]
    fn test_tool_selection() {
        let config = WindowConfig::default();
        let mut window = MainWindow::new(config);
        window.select_tool(Tool::Select);
        let editor = window.editor.read();
        assert_eq!(editor.current_tool(), Tool::Select);
    }
}
