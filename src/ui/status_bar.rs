// src/ui/status_bar.rs

use std::sync::Arc;
use eframe::egui::{self, Context};
use parking_lot::RwLock;
use crate::editor::Editor;

pub struct StatusBar {
    editor: Arc<RwLock<Editor>>,
}

impl StatusBar {
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self { editor }
    }

    pub fn update(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            let editor = self.editor.read(); // Keep the read lock short.

            let coord_label = if let Some(pos) = ui.input().pointer.hover_pos() {
                let world_pos = editor.screen_to_world(pos); // Assuming you add screen_to_world to Editor
                format!("({}, {})", world_pos.x as i32, world_pos.y as i32)
            } else {
                String::new()
            };

            ui.horizontal(|ui| {
                ui.label(&editor.status_message);
                ui.label(coord_label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Tool: {}", editor.current_tool().name()));
                });
            });
        });
    }
}