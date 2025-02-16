// src/editor/tools/select.rs

use super::{Tool, GridSettings};
use crate::bsp::BoundingBox;
use crate::document::Document;
use crate::map::{Vertex, LineDef, Sector, Thing};
use crate::editor::commands::{Command, CommandType};
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Default)]
pub struct SelectionState {
    vertices: Vec<usize>,
    linedefs: Vec<usize>,
    sectors: Vec<usize>,
    things: Vec<usize>,
}

pub struct SelectTool {
    grid_settings: GridSettings,
    selection: SelectionState,
    drag_start: Option<egui::Pos2>,
    drag_end: Option<egui::Pos2>,
    is_dragging: bool,
    drag_offset: Option<egui::Vec2>,
    initial_positions: Option<SelectionState>,
}

impl Default for SelectTool {
    fn default() -> Self {
        Self {
            grid_settings: GridSettings::default(),
            selection: SelectionState::default(),
            drag_start: None,
            drag_end: None,
            is_dragging: false,
            drag_offset: None,
            initial_positions: None,
        }
    }
}

impl Tool for SelectTool {
    fn name(&self) -> &'static str {
        "Select"
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

        if primary_clicked {
            if !modifiers.shift {
                self.clear_selection();
            }
            
            if !is_dragging {
                self.handle_click_selection(doc, snapped_pos);
            } else {
                self.drag_start = Some(snapped_pos);
                self.is_dragging = true;
            }
        }

        if is_dragging {
            if self.is_dragging {
                self.drag_end = Some(snapped_pos);
                self.update_rubber_band_selection(doc);
            } else if !self.selection.vertices.is_empty() {
                self.handle_drag_movement(doc, drag_delta);
            }
        }

        if secondary_clicked {
            self.clear_selection();
            self.cleanup();
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>) {
        // Draw selection rectangle if dragging
        if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
            let rect = [start, end];
            ui.painter().rect_stroke(
                egui::Rect::from_two_pos(rect[0], rect[1]),
                0.0,
                egui::Stroke::new(1.0, egui::Color32::YELLOW),
            );
        }

        // Draw selected vertices
        let doc_read = doc.read();
        let vertices = doc_read.vertices.read();
        let selection_color = egui::Color32::YELLOW;

        for &vertex_id in &self.selection.vertices {
            if let Some(vertex) = vertices.get(vertex_id) {
                let pos = egui::pos2(vertex.x as f32, vertex.y as f32);
                ui.painter().circle_stroke(
                    pos,
                    5.0,
                    egui::Stroke::new(2.0, selection_color),
                );
            }
        }

        // Draw selected linedefs
        let linedefs = doc_read.linedefs.read();
        for &linedef_id in &self.selection.linedefs {
            if let Some(linedef) = linedefs.get(linedef_id) {
                if let (Some(v1), Some(v2)) = (
                    vertices.get(linedef.start as usize),
                    vertices.get(linedef.end as usize)
                ) {
                    let start = egui::pos2(v1.x as f32, v1.y as f32);
                    let end = egui::pos2(v2.x as f32, v2.y as f32);
                    ui.painter().line_segment(
                        [start, end],
                        egui::Stroke::new(2.0, selection_color),
                    );
                }
            }
        }

        // Draw selected things
        let things = doc_read.things.read();
        for &thing_id in &self.selection.things {
            if let Some(thing) = things.get(thing_id) {
                let pos = egui::pos2(thing.x as f32, thing.y as f32);
                ui.painter().circle_stroke(
                    pos,
                    10.0,
                    egui::Stroke::new(2.0, selection_color),
                );
                
                // Draw thing angle indicator
                let angle_rad = (thing.angle as f32) * std::f32::consts::PI / 180.0;
                let direction = egui::vec2(angle_rad.cos(), angle_rad.sin()) * 20.0;
                ui.painter().line_segment(
                    [pos, pos + direction],
                    egui::Stroke::new(2.0, selection_color),
                );
            }
        }
    }

    fn cleanup(&mut self) {
        self.drag_start = None;
        self.drag_end = None;
        self.is_dragging = false;
        self.drag_offset = None;
        self.initial_positions = None;
    }
}

impl SelectTool {
    fn clear_selection(&mut self) {
        self.selection = SelectionState::default();
    }

    fn handle_click_selection(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        let doc_read = doc.read();
        let mut found = false;

        // Check vertices first (they're smallest)
        let vertices = doc_read.vertices.read();
        for (idx, vertex) in vertices.iter().enumerate() {
            let vertex_pos = egui::pos2(vertex.x as f32, vertex.y as f32);
            if vertex_pos.distance(pos) < 5.0 {
                self.selection.vertices.push(idx);
                found = true;
                break;
            }
        }

        if !found {
            // Check linedefs
            let linedefs = doc_read.linedefs.read();
            for (idx, linedef) in linedefs.iter().enumerate() {
                if let (Some(v1), Some(v2)) = (
                    vertices.get(linedef.start as usize),
                    vertices.get(linedef.end as usize)
                ) {
                    let start = egui::pos2(v1.x as f32, v1.y as f32);
                    let end = egui::pos2(v2.x as f32, v2.y as f32);
                    if line_distance_to_point(start, end, pos) < 5.0 {
                        self.selection.linedefs.push(idx);
                        found = true;
                        break;
                    }
                }
            }
        }

        if !found {
            // Check things
            let things = doc_read.things.read();
            for (idx, thing) in things.iter().enumerate() {
                let thing_pos = egui::pos2(thing.x as f32, thing.y as f32);
                if thing_pos.distance(pos) < 10.0 {
                    self.selection.things.push(idx);
                    break;
                }
            }
        }
    }

    fn update_rubber_band_selection(&mut self, doc: &Arc<RwLock<Document>>) {
        if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
            let selection_rect = BoundingBox::from_points(&[
                crate::bsp::Point2D::new(start.x as f64, start.y as f64),
                crate::bsp::Point2D::new(end.x as f64, end.y as f64)
            ]);

            let doc_read = doc.read();
            
            // Select vertices in rectangle
            let vertices = doc_read.vertices.read();
            self.selection.vertices = vertices.iter().enumerate()
                .filter(|(_, v)| selection_rect.contains_point(v.x as f64, v.y as f64))
                .map(|(idx, _)| idx)
                .collect();

            // Select linedefs with both vertices in rectangle
            let linedefs = doc_read.linedefs.read();
            self.selection.linedefs = linedefs.iter().enumerate()
                .filter(|(_, linedef)| {
                    if let (Some(v1), Some(v2)) = (
                        vertices.get(linedef.start as usize),
                        vertices.get(linedef.end as usize)
                    ) {
                        selection_rect.contains_point(v1.x as f64, v1.y as f64) && 
                        selection_rect.contains_point(v2.x as f64, v2.y as f64)
                    } else {
                        false
                    }
                })
                .map(|(idx, _)| idx)
                .collect();

            // Select things in rectangle
            let things = doc_read.things.read();
            self.selection.things = things.iter().enumerate()
                .filter(|(_, t)| selection_rect.contains_point(t.x as f64, t.y as f64))
                .map(|(idx, _)| idx)
                .collect();
        }
    }

    fn handle_drag_movement(&mut self, doc: &Arc<RwLock<Document>>, drag_delta: egui::Vec2) {
        if self.initial_positions.is_none() {
            // Store initial positions when starting drag
            let doc_read = doc.read();
            let vertices = doc_read.vertices.read();
            let things = doc_read.things.read();

            let mut initial = SelectionState::default();
            for &idx in &self.selection.vertices {
                if let Some(vertex) = vertices.get(idx) {
                    initial.vertices.push(idx);
                }
            }
            for &idx in &self.selection.things {
                if let Some(thing) = things.get(idx) {
                    initial.things.push(idx);
                }
            }
            self.initial_positions = Some(initial);
        }

        // Apply movement to selected objects
        let snapped_delta = if self.grid_settings.enabled {
            egui::vec2(
                (drag_delta.x / self.grid_settings.size as f32).round() * self.grid_settings.size as f32,
                (drag_delta.y / self.grid_settings.size as f32).round() * self.grid_settings.size as f32,
            )
        } else {
            drag_delta
        };

        // Create and execute move commands
        let mut commands = Vec::new();

        for &vertex_id in &self.selection.vertices {
            commands.push(CommandType::MoveVertex {
                vertex_id,
                dx: snapped_delta.x as i32,
                dy: snapped_delta.y as i32,
            });
        }

        for &thing_id in &self.selection.things {
            commands.push(CommandType::MoveThing {
                thing_id,
                dx: snapped_delta.x as i32,
                dy: snapped_delta.y as i32,
            });
        }

        if !commands.is_empty() {
            let mut batch_cmd = CommandType::BatchCommand { commands };
            let mut doc = doc.write();
            if let Err(e) = batch_cmd.execute(&mut doc) {
                println!("Error moving selection: {}", e);
            }
        }
    }
}

fn line_distance_to_point(line_start: egui::Pos2, line_end: egui::Pos2, point: egui::Pos2) -> f32 {
    let line_vec = line_end - line_start;
    let point_vec = point - line_start;
    let line_len = line_vec.length();
    
    if line_len == 0.0 {
        return point_vec.length();
    }
    
    let t = ((point_vec.x * line_vec.x + point_vec.y * line_vec.y) / line_len).clamp(0.0, line_len);
    let projection = line_start + (line_vec * (t / line_len));
    (point - projection).length()
}