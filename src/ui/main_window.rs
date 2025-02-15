// src/ui/main_window.rs

use crate::bsp::debug_viz::BspDebugger;
use crate::editor::objects::Selection;

use eframe::egui::{self, Context, Sense}; //Remove unused imports
use log::{info, error};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::document::Document;

//use crate::document::Document; // No longer directly using Document here
use crate::editor::{Editor, Tool, Command};
use crate::ui::{DialogManager, Theme};

/// MainWindow configuration settings (remains unchanged)
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
    editor: Arc<RwLock<Editor>>, // Now holds the Editor, not EditorState
    dialog_manager: DialogManager, // You'll implement this later

    // Debug tools
    bsp_debugger: BspDebugger,
    show_bsp_debug: bool,

    // UI state
    show_side_panel: bool,
    side_panel_width: f32,
    //tool_windows: ToolWindowManager, // You'll implement this later.
    status_message: String,
    error_message: Option<String>,
}

impl MainWindow {
    /// Creates a new MainWindow with the given configuration
    pub fn new(config: WindowConfig) -> Self {
        info!("Initializing main window {}x{}", config.default_width, config.default_height);
        let doc = Arc::new(Document::new());
        Self {
            config: config.clone(),
            editor: Arc::new(RwLock::new(Editor::new(Arc::clone(&doc)))),
            dialog_manager: DialogManager::new(),
            bsp_debugger: BspDebugger::new(),
            show_bsp_debug: false,
            show_side_panel: true,
            side_panel_width: 250.0,
            //tool_windows: ToolWindowManager::new(),  // You'll implement this later
            status_message: String::new(),
            error_message: None,
        }
    }

    /// Main UI update loop
    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_input(ctx);
        self.update_layout(ctx, _frame); //Keep the _frame, although unused
        //self.process_commands(); // No longer needed, commands are handled directly
    }

    /// Handles keyboard and mouse input
    fn handle_input(&mut self, ctx: &Context) {
        let input = ctx.input();

        // Global keyboard shortcuts
        if input.key_pressed(egui::Key::F11) {
            self.show_bsp_debug = !self.show_bsp_debug;
        }

        // Handle editor commands (using the new Command system)
        if input.key_pressed(egui::Key::Escape) {
            if let Ok(mut editor) = self.editor.write() {
                editor.cancel_current_operation();
            }
        }

        // Example:  Add a vertex with Ctrl+A (or Cmd+A on macOS)
        if input.modifiers.ctrl || input.modifiers.mac_cmd {
            if input.key_pressed(egui::Key::A) {
                if let Ok(mut editor) = self.editor.write() {
                    // Get the mouse position in screen coordinates
                    if let Some(pos) = input.pointer.hover_pos() {
                      // Convert the screen coordinates to the world coordinates
                      let world_pos = self.screen_to_world(pos);
                      // Create the AddVertex Command
                      let add_vertex_command = Command::AddVertex{
                        x: world_pos.x as i32,
                        y: world_pos.y as i32,
                        vertex_id: None
                      };
                      // Execute
                      editor.execute_command(add_vertex_command);
                    }
                }
            }
        }
    }

    /// Updates the complete UI layout
    fn update_layout(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.update_menu_bar(ctx);
        self.update_side_panel(ctx);
        self.update_central_area(ctx);
        self.update_status_bar(ctx);
        self.update_dialogs(ctx);

        // Show BSP debug window if enabled
        if self.show_bsp_debug {
            self.show_bsp_debug_window(ctx);
        }

        // Show error message if any
        if let Some(error) = &self.error_message {
            self.show_error_dialog(ctx, error);
        }
    }

      /// Updates the top menu bar
    fn update_menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // File menu
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

                // Edit menu
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        self.undo();
                    }
                    if ui.button("Redo").clicked() {
                        self.redo();
                    }
                });

                // View menu
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_side_panel, "Tools Panel");
                    ui.checkbox(&mut self.show_bsp_debug, "BSP Debug");
                });

                // Tools menu
                ui.menu_button("Tools", |ui| {
                    if ui.button("Build Nodes").clicked() {
                        self.build_nodes();
                    }
                    if ui.button("Generate Test Map").clicked() {
                        self.generate_test_map();
                    }
                });

                // Help menu
                ui.menu_button("Help", |ui| {
                    if ui.button("About...").clicked() {
                        self.show_about_dialog();
                    }
                });
            });
        });
    }

    /// Updates the side tool panel
    fn update_side_panel(&mut self, ctx: &Context) {
        if !self.show_side_panel {
            return;
        }

        egui::SidePanel::left("tools_panel")
            .default_width(self.side_panel_width)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Tools");
                
                // !! Access the Editor directly, not EditorState
                let editor = self.editor.read();  
                for tool in editor.available_tools() {
                    let selected = editor.current_tool() == *tool; // Compare directly
                    if ui.selectable_label(selected, tool.name()).clicked() {
                        drop(editor); // Explicitly drop the read lock
                        self.select_tool(*tool);  // Pass the Tool enum, not an Option
                    }
                }

                ui.separator();
                ui.heading("Properties");
                if let Some(obj) = editor.selected_object() {
                    self.show_property_editor(ui, obj.clone()); //Pass the Selection
                } else {
                    ui.label("No object selected");
                }
            });
    }
    /// Updates the central editor area
    fn update_central_area(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Create a full-size canvas
            let (response, painter) = ui.allocate_painter(
                ui.available_size(),
                Sense::click_and_drag()
            );

            let editor = self.editor.read();

            // Draw grid
            self.draw_editor_grid(&painter, response.rect);

            // Draw map geometry. No need to wrap the document in an Arc and RwLock again.
            if let Some(doc) = editor.document() {
                self.draw_map_geometry(&painter, doc.clone(), response.rect);
            }

            // Handle input
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.handle_editor_click(pos);
                }
            }
        });
    }

    /// Updates the status bar
    fn update_status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);

                // Show coordinates if hovering over editor
                if let Some(pos) = ui.input().pointer.hover_pos() {
                    let world_pos = self.screen_to_world(pos);
                    ui.label(format!("({}, {})", world_pos.x as i32, world_pos.y as i32));
                }

                // Show current tool. No longer needs try_read, as we use the editor.
                let editor = self.editor.read(); // Keep this lock as short as possible
                if let Some(tool) = Some(editor.current_tool()) { // Directly get the tool
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Tool: {}", tool.name()));
                    });
                }
            });
        });
    }

    /// Shows the BSP debug window
    fn show_bsp_debug_window(&mut self, ctx: &Context) {
        egui::Window::new("BSP Debug")
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                // Access the BSP tree through the editor
                let editor = self.editor.read(); // Get a read lock on the editor
                if let Some(bsp) = editor.bsp_tree() { // Use the bsp_tree() method
                    self.bsp_debugger.show(ui, &bsp); // Pass the Arc<BspLevel>
                } else {
                    ui.label("No BSP data available.");
                    if ui.button("Generate Test Map").clicked() {
                        drop(editor); // Drop the read lock *before* calling generate_test_map
                        self.generate_test_map(); // Call generate_test_map on self (MainWindow)
                    }
                }
            });
    }

    // Command handlers (now use the Editor's methods)
    fn new_document(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            if editor.has_unsaved_changes() {
                self.dialog_manager.show_save_changes_dialog(); // You'll implement this later
                return;
            }
            // In a real implementation, you'd create a new document here.
            // editor.new_document(); // You don't have a new_document() method on Editor
            self.status_message = "Created new document".to_string(); // Update status
        }
    }


    fn save_document(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            match editor.save_document() {
                Ok(_) => self.status_message = "Document saved".to_string(),
                Err(e) => {
                    error!("Failed to save document: {}", e);
                    self.error_message = Some(format!("Failed to save: {}", e));
                }
            }
        }
    }

    fn build_nodes(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            match editor.build_nodes() {
                Ok(_) => {
                    self.status_message = "BSP nodes built successfully".to_string();
                    self.show_bsp_debug = true; // Show debug window
                }
                Err(e) => {
                    error!("Failed to build nodes: {}", e);
                    self.error_message = Some(format!("Node building failed: {}", e));
                }
            }
        }
    }

    // Helper methods
    fn generate_test_map(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            editor.generate_test_map();
            self.status_message = "Generated test map".to_string();
            // No longer need to call build_nodes here, it's done inside generate_test_map
        }
    }

    fn select_tool(&mut self, tool: Tool) {
        if let Ok(mut editor) = self.editor.write() {
            editor.set_current_tool(tool);
            self.status_message = format!("Selected tool: {}", tool.name());
        }
    }


    fn screen_to_world(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        // TODO: Implement proper coordinate transformation (using zoom and pan)
        screen_pos
    }

    fn undo(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            editor.undo();
        }
    }

    fn redo(&mut self) {
        if let Ok(mut editor) = self.editor.write() {
            editor.redo();
        }
    }

    // --- Placeholder methods (to be implemented later) ---
    fn update_dialogs(&mut self, _ctx: &Context) {
        // TODO: Implement dialog management (open file, save file, settings, etc.)
    }

    fn show_open_dialog(&mut self) {
        // TODO: Implement file open dialog
    }
    fn show_save_changes_dialog(&mut self) {
        // TODO: Implement
    }

    fn request_exit(&mut self) {
        // TODO: Handle window close request (prompt to save, etc.)
        //       You'll likely need to use `frame.close()` here.
    }

    fn show_about_dialog(&mut self) {
        // TODO: Implement about dialog
    }

    fn show_property_editor(&mut self, _ui: &mut egui::Ui, _obj: Selection) {
        // TODO: Implement property editor for selected objects
    }


    fn draw_editor_grid(&self, _painter: &egui::Painter, _rect: egui::Rect) {
        // TODO: Implement grid drawing
    }

    fn draw_map_geometry(&self, _painter: &egui::Painter, _doc: Arc<Document>, _rect: egui::Rect) {
        // TODO: Implement map geometry drawing (vertices, lines, etc.)
    }

    fn handle_editor_click(&self, _pos: egui::Pos2) {
        // TODO: Implement mouse click handling (selection, tool actions, etc.)
    }

}

// Error handling (remains the same)
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

// --- Test Cases (adapted for the new structure) ---
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
        window.select_tool(Tool::Select); // Directly call select_tool

        // Access the editor through a read lock
        let editor = window.editor.read();
        assert_eq!(editor.current_tool(), Tool::Select); // Compare directly with the enum
    }
}