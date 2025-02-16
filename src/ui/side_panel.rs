// src/ui/side_panel.rs

use std::sync::Arc;
use eframe::egui::{self, Context, Ui};
use parking_lot::RwLock;

use crate::editor::core::{Editor, Selection};

/// Manages the left-side panel with tool buttons, level list, and properties.
pub struct SidePanel {
    editor: Arc<RwLock<Editor>>,
    pub show_side_panel: bool, // Whether this panel is currently visible
}

impl SidePanel {
    /// Create a new SidePanel, by providing an `Editor`.
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self {
            editor,
            show_side_panel: true,
        }
    }

    /// Called each frame, updates the side panel UI if it's visible.
    pub fn update(&mut self, ctx: &Context) {
        if !self.show_side_panel {
            return; // Early exit if user has hidden it
        }

        egui::SidePanel::left("tools_panel")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                self.show_tools(ui);
                ui.separator();
                self.show_levels(ui);
                ui.separator();
                self.show_properties(ui);
            });
    }

    /// Displays a list of available tools (from the Editor), and lets user click to select one.
    fn show_tools(&self, ui: &mut Ui) {
        ui.heading("Tools");

        // Acquire a read lock to see the available tools and current tool.
        let editor = self.editor.read();

        for tool in editor.available_tools() {
            let selected = editor.current_tool() == tool;

            // A clickable label which becomes selected if current tool == tool
            if ui.selectable_label(selected, tool.name()).clicked() {
                // Before we mutate the editor, release our read lock
                drop(editor);

                // Acquire a write lock to actually set the tool
                self.editor.write().set_current_tool(tool);
                return; // We must exit early, because we can't use `editor` after the drop
            }
        }
    }

    /// Displays the list of levels found in the WAD, enabling user to switch levels.
    fn show_levels(&self, ui: &mut Ui) {
        ui.heading("Levels");

        // Acquire the editor read lock, then get the Document
        if let Some(doc_arc) = self.editor.read().document() {
            // Acquire doc read lock to get the available levels
            let levels = {
                let doc = doc_arc.read();
                doc.available_levels()
            };

            if levels.is_empty() {
                ui.label("No levels found. Check your WAD file.");
            } else {
                for level_name in levels {
                    // A button for each level. Click to load it.
                    let level_name_clone = level_name.clone();
                    let doc_arc_clone = Arc::clone(&doc_arc);
                    if ui.button(&level_name).clicked() {
                        // Release editor read lock before mutating
                        drop(doc_arc_clone);

                        // Then call the load wrapper
                        self.editor.write().load_level_wrapper(level_name_clone);
                    }
                }
            }
        } else {
            ui.label("No document loaded.");
        }
    }

    /// Shows properties for whatever the user has selected (vertex, linedef, etc.).
    fn show_properties(&self, ui: &mut Ui) {
        ui.heading("Properties");

        // We only need a read lock to see what is selected.
        let selection = self.editor.read().selected_object();

        match selection {
            Selection::Vertex(vertex) => {
                ui.label(format!("Vertex: ({}, {})", vertex.x, vertex.y));
            }
            Selection::Line(linedef) => {
                ui.label(format!("Linedef: from vertex {} to {}", linedef.start, linedef.end));
            }
            Selection::Sector(sector) => {
                ui.label(format!(
                    "Sector: floor {} / ceiling {}",
                    sector.floor_height,
                    sector.ceiling_height
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