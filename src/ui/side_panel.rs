use std::sync::Arc;
use eframe::egui::{self, Context, Ui, ScrollArea};
use parking_lot::RwLock;
use log::{error, info};

use crate::editor::core::Editor;
// Use the Selection enum from the central panel module.
use crate::ui::central_panel::Selection;

/// Manages the left-side panel with tool buttons, a level list, and properties of the selected object.
pub struct SidePanel {
    /// Reference to the main Editor.
    editor: Arc<RwLock<Editor>>,
    /// Whether this side panel is currently visible.
    pub show_side_panel: bool,
}

impl SidePanel {
    /// Creates a new side panel that references the given Editor.
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self {
            editor,
            show_side_panel: true,
        }
    }

    /// Called every frame. Draws the panel if `show_side_panel` is true.
    pub fn update(&mut self, ctx: &Context) {
        if !self.show_side_panel {
            return;
        }

        egui::SidePanel::left("tools_panel")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Put everything in a vertical scroll area.
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        self.show_tools(ui);
                        ui.separator();
                        self.show_levels(ui);
                        ui.separator();
                        self.show_properties(ui);
                    });
            });
    }

    /// Displays the list of available tools.
    fn show_tools(&self, ui: &mut Ui) {
        ui.heading("Tools");

        // Acquire a read lock on the Editor.
        let ed_read = self.editor.read();
        let available_tools = ed_read.available_tools();
        let current_tool_name = ed_read.current_tool_name();
        drop(ed_read);

        // Display each tool as a selectable label.
        for tool in available_tools {
            let selected = current_tool_name == tool;
            if ui.selectable_label(selected, tool).clicked() {
                // Acquire a write lock to change the current tool.
                let mut ed_write = self.editor.write();
                ed_write.set_current_tool(tool);
                return; // Exit after switching tool.
            }
        }
    }

    /// Displays the list of levels from the WAD.
    fn show_levels(&mut self, ui: &mut Ui) {
        ui.heading("Levels");

        let ed_read = self.editor.read();
        let doc_opt = ed_read.document();

        if doc_opt.is_none() {
            ui.label("No document loaded.");
            return;
        }
        // Acquire the document and fetch the list of levels.
        let doc_arc = doc_opt.unwrap();
        let levels = {
            let doc = doc_arc.read();
            doc.available_levels()
        };

        if levels.is_empty() {
            ui.label("No levels found. Check your WAD file.");
            return;
        }
        drop(ed_read);

        // Display each level as a button.
        for level_name in &levels {
            if ui.button(level_name).clicked() {
                let mut ed_write = self.editor.write();
                // Cancel any ongoing selection/operation.
                ed_write.cancel_current_operation();
                info!("Attempting to load level: {}", level_name);
                ed_write.load_level_wrapper(level_name.clone());
                if let Some(ref err_msg) = ed_write.error_message {
                    error!("Failed to load level {}: {}", level_name, err_msg);
                }
                return;
            }
        }
    }

    /// Displays properties of the currently selected object.
    fn show_properties(&self, ui: &mut Ui) {
        ui.heading("Properties");

        // Get the current selection from the Editor.
        // (Ensure that Editor::selected_object() is implemented,
        // for example by delegating to the central panel’s selection.)
        /* let selection = self.editor.read().selected_object();

        match selection {
            Selection::Vertex(vertex) => {
                ui.label(format!("Vertex: ({}, {})", vertex.x, vertex.y));
            }
            Selection::Line(line_def) => {
                ui.label(format!(
                    "Linedef: from {} to {}",
                    line_def.start, line_def.end
                ));
            }
            Selection::Sector(sector) => {
                ui.label(format!(
                    "Sector: floor {} / ceiling {}",
                    sector.floor_height, sector.ceiling_height
                ));
            }
            Selection::Thing(thing) => {
                ui.label(format!("Thing type: {}", thing.doom_type));
            }
            Selection::None => {
                ui.label("Nothing selected.");
            }
        } */
    }
}
