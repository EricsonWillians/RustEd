// src/editor/tools/shape.rs

use super::{Tool, GridSettings};
use crate::document::Document;
use crate::editor::commands::{Command, CommandType};
use crate::map::{Vertex, LineDef, Sector};
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct DrawShapeTool {
    grid_settings: GridSettings,
    vertices: Vec<(i32, i32)>,  // Accumulated vertices for the shape
    preview_pos: Option<egui::Pos2>, // Current mouse position for preview
    is_drawing: bool,
    default_ceiling_height: i32,
    default_floor_height: i32,
    default_light: i32,
}

impl Default for DrawShapeTool {
    fn default() -> Self {
        Self {
            grid_settings: GridSettings::default(),
            vertices: Vec::new(),
            preview_pos: None,
            is_drawing: false,
            default_ceiling_height: 128,
            default_floor_height: 0,
            default_light: 192,
        }
    }
}

impl Tool for DrawShapeTool {
    fn name(&self) -> &'static str {
        "Draw Shape"
    }

    fn handle_input(
        &mut self,
        doc: &Arc<RwLock<Document>>,
        world_pos: egui::Pos2,
        primary_clicked: bool,
        secondary_clicked: bool,
        is_dragging: bool,
        drag_delta: egui::Vec2,
        modifiers: egui::Modifiers,
    ) {
        let snapped_pos = self.grid_settings.snap_position(world_pos);
        self.preview_pos = Some(snapped_pos);

        if primary_clicked {
            let pos = (snapped_pos.x as i32, snapped_pos.y as i32);
            
            // Check if we're closing the shape
            if !self.vertices.is_empty() && self.can_close_shape(pos) {
                self.complete_shape(doc);
            } else {
                // Add new vertex to shape
                self.vertices.push(pos);
                self.is_drawing = true;
            }
        }

        if secondary_clicked {
            // Cancel shape if drawing, or cleanup if not
            if self.is_drawing {
                self.vertices.clear();
                self.is_drawing = false;
            } else {
                self.cleanup();
            }
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>) {
        if self.vertices.is_empty() {
            return;
        }

        // Draw existing shape vertices and lines
        let points: Vec<egui::Pos2> = self.vertices.iter()
            .map(|(x, y)| egui::pos2(*x as f32, *y as f32))
            .collect();

        // Draw lines between vertices
        for i in 0..points.len() - 1 {
            ui.painter().line_segment(
                [points[i], points[i + 1]],
                egui::Stroke::new(1.0, egui::Color32::YELLOW),
            );
        }

        // Draw preview line from last vertex to current mouse position
        if let Some(preview) = self.preview_pos {
            if let Some(&last) = points.last() {
                // Draw dashed preview line
                draw_dashed_line(ui, last, preview, egui::Color32::YELLOW);

                // If we can close the shape, draw preview to first vertex
                if self.vertices.len() >= 3 && self.can_close_shape((preview.x as i32, preview.y as i32)) {
                    draw_dashed_line(ui, preview, points[0], egui::Color32::GREEN);
                }
            }
        }

        // Draw vertices
        for point in points {
            ui.painter().circle_filled(
                point,
                3.0,
                egui::Color32::YELLOW,
            );
        }
    }

    fn cleanup(&mut self) {
        self.vertices.clear();
        self.preview_pos = None;
        self.is_drawing = false;
    }
}

impl DrawShapeTool {
    fn can_close_shape(&self, current_pos: (i32, i32)) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }

        let first_vertex = self.vertices[0];
        let dx = (current_pos.0 - first_vertex.0).abs();
        let dy = (current_pos.1 - first_vertex.1).abs();
        
        // Can close if within snap distance of first vertex
        dx <= self.grid_settings.snap_threshold as i32 
            && dy <= self.grid_settings.snap_threshold as i32
    }

    fn complete_shape(&mut self, doc: &Arc<RwLock<Document>>) {
        if self.vertices.len() < 3 {
            return;
        }

        // Create batch command for shape creation
        let mut commands = Vec::new();

        // First, create all vertices
        for &(x, y) in &self.vertices {
            commands.push(CommandType::AddVertex {
                x,
                y,
                vertex_id: None,
            });
        }

        // Create a new sector
        commands.push(CommandType::AddSector {
            floor_height: self.default_floor_height,
            ceiling_height: self.default_ceiling_height,
            floor_tex: "FLOOR0_1".to_string(),
            ceiling_tex: "CEIL1_1".to_string(),
            light: self.default_light,
            r#type: 0,
            tag: 0,
            sector_id: Some(0), // Will be updated after creation
        });

        // Create sidedefs and linedefs connecting vertices
        for i in 0..self.vertices.len() {
            let next_i = (i + 1) % self.vertices.len();

            // Add right sidedef
            commands.push(CommandType::AddSideDef {
                x_offset: 0,
                y_offset: 0,
                upper_tex: "-".to_string(),
                lower_tex: "-".to_string(),
                mid_tex: "STARTAN2".to_string(),
                sector: 0, // Will be the ID of our new sector
                sidedef_id: None,
            });

            // Add linedef connecting current vertex to next
            commands.push(CommandType::AddLineDef {
                start: i,
                end: next_i,
                flags: 0x0001, // blocking
                line_type: 0,  // normal
                tag: 0,
                right: 0,     // Will be updated after creation
                left: -1,     // No left side initially
                linedef_id: Some(0),
            });
        }

        // Execute the batch command
        let mut batch_cmd = CommandType::BatchCommand { commands };
        let mut doc = doc.write();
        if let Err(e) = batch_cmd.execute(&mut doc) {
            println!("Error creating shape: {}", e);
            return;
        }

        // Clear state after successful creation
        self.cleanup();
    }
}

fn draw_dashed_line(ui: &mut egui::Ui, start: egui::Pos2, end: egui::Pos2, color: egui::Color32) {
    const DASH_LENGTH: f32 = 8.0;
    let vector = end - start;
    let length = vector.length();
    let unit = vector / length;
    let num_dashes = (length / DASH_LENGTH).floor() as i32;

    for i in 0..num_dashes {
        let t1 = i as f32 * DASH_LENGTH;
        let t2 = ((i as f32 + 0.5) * DASH_LENGTH).min(length);
        
        let dash_start = start + (unit * t1);
        let dash_end = start + (unit * t2);

        ui.painter().line_segment(
            [dash_start, dash_end],
            egui::Stroke::new(1.0, color),
        );
    }
}