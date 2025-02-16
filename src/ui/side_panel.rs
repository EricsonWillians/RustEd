// src/ui/side_panel.rs

use std::sync::Arc;
use eframe::egui::{self, Context, Ui, ScrollArea};
use parking_lot::RwLock;
use log::{error, info};

use crate::editor::core::{Editor, Selection};

/// Manages the left-side panel with tool buttons, a level list, and properties of the selected object.
pub struct SidePanel {
    /// Reference to the main Editor
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
                // Put everything in a vertical scroll area
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

    /// Shows the tool selection (e.g., Select, DrawLine, etc.).
    fn show_tools(&self, ui: &mut Ui) {
        ui.heading("Tools");

        // Acquire a read lock on the Editor
        let ed_read = self.editor.read();

        let available_tools = ed_read.available_tools();
        let current_tool = ed_read.current_tool();

        // Release the read lock before we mutate the editor if user clicks
        drop(ed_read);

        // Now display the tools
        for tool in available_tools {
            // Check if it's currently selected
            let selected = self.editor.read().current_tool() == tool;
            if ui.selectable_label(selected, tool.name()).clicked() {
                // Acquire write lock to change tool
                let mut ed_write = self.editor.write();
                ed_write.set_current_tool(tool);
                return; // Early return because we can't keep using ed_write while we hold it
            }
        }
    }

    /// Shows a list of levels from the WAD, letting the user load a different level.
    fn show_levels(&mut self, ui: &mut Ui) {
        ui.heading("Levels");

        // We only need to read the Editor to get the Document.
        let ed_read = self.editor.read();
        let doc_opt = ed_read.document();

        if doc_opt.is_none() {
            ui.label("No document loaded.");
            return;
        }
        // Get the Document Arc
        let doc_arc = doc_opt.unwrap();
        // Acquire the Document read lock to fetch the level list
        let levels = {
            let doc = doc_arc.read();
            doc.available_levels()
        };

        if levels.is_empty() {
            ui.label("No levels found. Check your WAD file.");
            return;
        }

        // Release the Editor read lock before we might mutate it.
        drop(ed_read);

        // Display each level name as a button
        for level_name in &levels {
            if ui.button(level_name).clicked() {
                // Acquire a write lock, in case we want to reset selection / load level
                let mut ed_write = self.editor.write();

                // Reset any ongoing selection or partial operation to avoid referencing old geometry
                ed_write.cancel_current_operation();

                info!("Attempting to load level: {}", level_name);

                // Actually load the new level
                ed_write.load_level_wrapper(level_name.clone());

                // If there's an error message set by load_level_wrapper, log it
                if let Some(ref err_msg) = ed_write.error_message {
                    error!("Failed to load level {}: {}", level_name, err_msg);
                }
                // Once we're done, drop the write lock
                return; // Possibly break out so we don't keep iterating
            }
        }
    }

    /// Shows properties of the currently selected object (vertex, linedef, etc.).
    fn show_properties(&self, ui: &mut Ui) {
        ui.heading("Properties");

        // We just read the selection from the editor.
        let selection = self.editor.read().selected_object();

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
        }
    }
}
