// src/ui/menu.rs

use std::sync::Arc;
use eframe::egui::{self, Context};
use parking_lot::RwLock;
use crate::editor::Editor;

pub struct MenuBar {
    editor: Arc<RwLock<Editor>>,
}

impl MenuBar {
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self { editor }
    }

    pub fn update(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.editor.write().new_document();
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        // Calls the method you re-added in Editor
                        self.editor.write().show_open_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        // Calls the wrapper that handles errors, sets status message
                        self.editor.write().save_document_wrapper();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        // Handle exit at a higher level (e.g., eframe integration)
                        ui.close_menu();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        self.editor.write().undo();
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        self.editor.write().redo();
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    let mut editor = self.editor.write();

                    // Both fields exist in Editor as booleans
                    if ui.checkbox(&mut editor.show_side_panel, "Tools Panel").clicked() {
                        ui.close_menu();
                    }
                    if ui.checkbox(&mut editor.show_bsp_debug, "BSP Debug").clicked() {
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Build Nodes").clicked() {
                        // Wraps build_nodes() in Editor
                        self.editor.write().build_nodes_wrapper();
                        ui.close_menu();
                    }
                    if ui.button("Generate Test Map").clicked() {
                        self.editor.write().generate_test_map();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About...").clicked() {
                        // Possibly show an 'About' dialog
                        ui.close_menu();
                    }
                });
            });
        });
    }
}
