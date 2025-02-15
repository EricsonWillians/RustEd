use std::sync::Arc;

use eframe::egui::{self, Context, Sense, Vec2};
use log::{info, error};
use parking_lot::RwLock;
use crate::editor::commands::{Command, CommandType};

use crate::editor::objects::EditObject;
use crate::editor::Selection; 
use crate::{
    bsp::debug_viz::BspDebugger,
    document::Document,
    editor::{Editor, Tool},
    ui::{DialogManager, Theme},
};

/// MainWindow configuration settings
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
/// Application window state and UI management
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
}

impl MainWindow {
    /// Creates a new MainWindow with the given configuration
    pub fn new(config: WindowConfig) -> Self {
        info!(
            "Initializing main window {}x{}",
            config.default_width, config.default_height
        );

        // 1) Create an Arc<RwLock<Document>>
        let doc = Arc::new(RwLock::new(Document::new()));

        // 2) Create Editor with that Document, also Arc<RwLock>
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
        }
    }

    /// Main UI update loop
    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.handle_input(ctx);
        self.update_layout(ctx, frame);
    }

    // ------------------------------------------------------------------------
    // Input Handling
    // ------------------------------------------------------------------------
    fn handle_input(&mut self, ctx: &Context) {
        let input = ctx.input();

        // Example: toggle the BSP debug window with F11
        if input.key_pressed(egui::Key::F11) {
            self.show_bsp_debug = !self.show_bsp_debug;
        }

        // Cancel current operation with ESC
        if input.key_pressed(egui::Key::Escape) {
            // NOTE: parking_lot::RwLock::write() never errors, so no match needed
            let mut editor = self.editor.write();
            editor.cancel_current_operation();
        }

        // Example: Ctrl + A = add a vertex (just a demonstration)
        if input.modifiers.ctrl || input.modifiers.mac_cmd {
            if input.key_pressed(egui::Key::A) {
                let mut editor = self.editor.write();
                if let Some(pos) = input.pointer.hover_pos() {
                    let world_pos = self.screen_to_world(pos);

                    let cmd = CommandType::AddVertex {
                        x: world_pos.x as i32,
                        y: world_pos.y as i32,
                        vertex_id: None,
                    };
                    // Editor::execute_command expects Box<dyn Command> or similar
                    editor.execute_command(Box::new(cmd) as Box<dyn Command>);
                }
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

        if let Some(error) = &self.error_message {
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
                // FILE menu
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

                // EDIT menu
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        self.undo();
                    }
                    if ui.button("Redo").clicked() {
                        self.redo();
                    }
                });

                // VIEW menu
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_side_panel, "Tools Panel");
                    ui.checkbox(&mut self.show_bsp_debug, "BSP Debug");
                });

                // TOOLS menu
                ui.menu_button("Tools", |ui| {
                    if ui.button("Build Nodes").clicked() {
                        self.build_nodes();
                    }
                    if ui.button("Generate Test Map").clicked() {
                        self.generate_test_map();
                    }
                });

                // HELP menu
                ui.menu_button("Help", |ui| {
                    if ui.button("About...").clicked() {
                        self.show_about_dialog();
                    }
                });
            });
        });
    }

    // ------------------------------------------------------------------------
    // Left Side Tools Panel
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

                let editor = self.editor.read();
                for tool in editor.available_tools() {
                    let selected = editor.current_tool() == *tool;
                    if ui.selectable_label(selected, tool.name()).clicked() {
                        // We must drop 'editor' before writing again
                        drop(editor);
                        self.select_tool(*tool);
                        return;
                    }
                }

                ui.separator();
                ui.heading("Properties");

                if let Some(selection) = editor.selected_object() {
                    // Now we correctly match against the Selection enum.
                    let edit_obj = match selection {
                        Selection::Vertex(idx) => EditObject::Vertex(*idx),
                        Selection::Line(idx) => EditObject::LineDef(*idx),
                        Selection::Sector(idx) => EditObject::Sector(*idx),
                        Selection::Thing(idx) => EditObject::Thing(*idx),
                    };
                    drop(editor);
                    self.show_property_editor(ui, edit_obj);
                } else {
                    ui.label("No object selected");
                }
            });
    }
    
    // ------------------------------------------------------------------------
    // Central Area
    // ------------------------------------------------------------------------
    fn update_central_area(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size(), Sense::click_and_drag());

            let editor = self.editor.read();

            // Some optional background grid or something
            self.draw_editor_grid(&painter, response.rect);

            // Draw the actual Document’s geometry if present
            if let Some(doc_arc) = editor.document() {
                self.draw_map_geometry(&painter, doc_arc, response.rect);
            }

            // If user clicks, handle it
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
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
                // Show current tool on the right side
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
                if let Some(bsp) = editor.bsp_tree() {
                    self.bsp_debugger.show(ui, &bsp);
                } else {
                    ui.label("No BSP data available.");
                    // Quick button to generate a test map
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
        // No lock errors with parking_lot
        let mut editor = self.editor.write();

        // Example: check if unsaved
        if editor.has_unsaved_changes() {
            self.dialog_manager.show_save_changes_dialog();
            return;
        }
        // Actually create a new document if you have a method for that:
        // editor.new_document();
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
        editor.generate_test_map(); // This calls build_nodes() internally
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
    // Misc Utility
    // ------------------------------------------------------------------------
    fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        // If you had a camera / zoom, you'd transform it here
        screen_pos
    }

    // Stub property editor
    fn show_property_editor(&mut self, ui: &mut egui::Ui, obj: EditObject) {
        match obj {
            EditObject::Vertex(idx) => {
                ui.label(format!("Editing Vertex {}", idx));
                // ... vertex editing UI code ...
            }
            EditObject::LineDef(idx) => {
                ui.label(format!("Editing Linedef {}", idx));
                // ... linedef editing UI code ...
            }
            // Handle other variants similarly...
            _ => { ui.label("Editing other object types..."); }
        }
    }
       

    // Stub: draws a grid
    fn draw_editor_grid(&self, _painter: &egui::Painter, _rect: egui::Rect) {
        // optional
    }

    // Stub: draws map geometry
    fn draw_map_geometry(
        &self,
        _painter: &egui::Painter,
        _doc: Arc<RwLock<Document>>,
        _rect: egui::Rect,
    ) {
        let _doc_read = _doc.read();
        // draw lines, vertices, etc.
    }

    // Stub: handle user clicks
    fn handle_editor_click(&self, _pos: egui::Pos2) {
        // TODO
    }

    // ------------------------------------------------------------------------
    // Dialogs etc.
    // ------------------------------------------------------------------------
    fn update_dialogs(&mut self, _ctx: &Context) {
        // You might have a file dialog or something
    }

    fn show_open_dialog(&mut self) {
        // placeholder
    }

    fn show_save_changes_dialog(&mut self) {
        // placeholder
    }

    fn request_exit(&mut self) {
        // placeholder: e.g. confirm close
    }

    fn show_about_dialog(&mut self) {
        // placeholder
    }
}

// Error handling dialog
impl MainWindow {
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
}

// Optional tests
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

        // Access the editor
        let editor = window.editor.read();
        assert_eq!(editor.current_tool(), Tool::Select);
    }
}
