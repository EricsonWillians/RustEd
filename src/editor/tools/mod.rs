// src/editor/tools/mod.rs
mod select;
mod draw;
mod shape;
mod things;
mod sectors;

pub use select::SelectTool;
pub use draw::DrawLineTool;
pub use shape::DrawShapeTool;
pub use things::ThingsTool;
pub use sectors::SectorsTool;

use eframe::egui;
use crate::document::Document;
use std::sync::Arc;
use parking_lot::RwLock;

pub trait Tool {
    fn name(&self) -> &'static str;
    fn handle_input(
        &mut self,
        doc: &Arc<RwLock<Document>>,
        world_pos: egui::Pos2,
        primary_clicked: bool,
        secondary_clicked: bool,
        is_dragging: bool,
        drag_delta: egui::Vec2,
        modifiers: egui::Modifiers,
    );
    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>);
    fn cleanup(&mut self);
}

// Grid settings struct
#[derive(Debug, Clone)]
pub struct GridSettings {
    pub enabled: bool,
    pub size: i32,
    pub snap_threshold: f32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 32, // Default DOOM grid size
            snap_threshold: 16.0,
        }
    }
}

impl GridSettings {
    pub fn snap_position(&self, pos: egui::Pos2) -> egui::Pos2 {
        if !self.enabled {
            return pos;
        }
        
        let grid_size = self.size as f32;
        egui::pos2(
            (pos.x / grid_size).round() * grid_size,
            (pos.y / grid_size).round() * grid_size,
        )
    }
}