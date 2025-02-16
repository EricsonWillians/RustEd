use super::{Tool, GridSettings};
use crate::editor::commands::{Command, CommandType};
use crate::document::Document;
use crate::map::SideDef;
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct DrawLineTool {
    grid_settings: GridSettings,
    last_vertex_id: Option<usize>,
    preview_line: Option<(egui::Pos2, egui::Pos2)>,
    is_drawing: bool,
    default_line_flags: i32,
    default_line_type: i32,
    default_wall_tex: String,
}

impl Default for DrawLineTool {
    fn default() -> Self {
        Self {
            grid_settings: GridSettings::default(),
            last_vertex_id: None,
            preview_line: None,
            is_drawing: false,
            default_line_flags: 0x0001, // Default to blocking
            default_line_type: 0,       // Normal line
            default_wall_tex: "STARTAN2".to_string(),
        }
    }
}

impl Tool for DrawLineTool {
    fn name(&self) -> &'static str {
        "Draw Line"
    }

    fn handle_input(
        &mut self,
        doc: &Arc<RwLock<Document>>,
        world_pos: egui::Pos2,
        primary_clicked: bool,
        secondary_clicked: bool,
        is_dragging: bool,
        _drag_delta: egui::Vec2,
        _modifiers: egui::Modifiers,
    ) {
        let snapped_pos = self.grid_settings.snap_position(world_pos);

        if primary_clicked {
            if !self.is_drawing {
                // Start new line
                self.start_new_line(doc, snapped_pos);
            } else {
                // Complete current line
                self.complete_line(doc, snapped_pos);
            }
        }

        if is_dragging && self.is_drawing {
            // Update preview line
            if let Some(start_pos) = self.get_last_vertex_position(doc) {
                self.preview_line = Some((start_pos, snapped_pos));
            }
        }

        if secondary_clicked {
            self.cleanup();
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>) {
        // Draw preview line if we're in the middle of drawing
        if let Some((start, end)) = self.preview_line {
            let stroke = egui::Stroke::new(2.0, egui::Color32::YELLOW);
            ui.painter().line_segment([start, end], stroke);
        }

        // Draw grid if enabled
        if self.grid_settings.enabled {
            self.draw_grid(ui);
        }
    }

    fn cleanup(&mut self) {
        self.last_vertex_id = None;
        self.preview_line = None;
        self.is_drawing = false;
    }
}

impl DrawLineTool {
    fn start_new_line(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        let mut cmd = CommandType::AddVertex {
            x: pos.x as i32,
            y: pos.y as i32,
            vertex_id: None,
        };

        // Execute command and store vertex ID
        let mut doc = doc.write();
        if let Ok(_) = cmd.execute(&mut doc) {
            let vertices = doc.vertices.read();
            self.last_vertex_id = Some(vertices.len() - 1);
            self.is_drawing = true;
        }
    }

    fn complete_line(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        if let Some(start_vertex_id) = self.last_vertex_id {
            let mut doc_write = doc.write();

            // Create a new sidedef for the right side
            let right_sidedef = SideDef {
                x_offset: 0,
                y_offset: 0,
                upper_tex: String::new(),
                lower_tex: String::new(),
                mid_tex: self.default_wall_tex.clone(),
                sector: 0,
            };

            // Add end vertex
            let mut add_end_vertex = CommandType::AddVertex {
                x: pos.x as i32,
                y: pos.y as i32,
                vertex_id: None,
            };
            if let Err(e) = add_end_vertex.execute(&mut doc_write) {
                eprintln!("Error adding end vertex: {}", e);
                return;
            }
            let end_vertex_id = if let CommandType::AddVertex { vertex_id: Some(id), .. } = add_end_vertex {
                id
            } else {
                panic!("Vertex ID should be set after execution");
            };

            // Add right sidedef
            let mut add_sidedef = CommandType::AddSideDef {
                x_offset: right_sidedef.x_offset,
                y_offset: right_sidedef.y_offset,
                upper_tex: right_sidedef.upper_tex,
                lower_tex: right_sidedef.lower_tex,
                mid_tex: right_sidedef.mid_tex,
                sector: right_sidedef.sector,
                sidedef_id: None,
            };
            if let Err(e) = add_sidedef.execute(&mut doc_write) {
                eprintln!("Error adding sidedef: {}", e);
                return;
            }
            let sidedef_id = if let CommandType::AddSideDef { sidedef_id: Some(id), .. } = add_sidedef {
                id
            } else {
                panic!("Sidedef ID should be set after execution");
            };

            // Add linedef
            let mut add_linedef = CommandType::AddLineDef {
                start: start_vertex_id,
                end: end_vertex_id,
                flags: self.default_line_flags,
                line_type: self.default_line_type,
                tag: 0,
                right: sidedef_id as i32,
                left: -1,
                linedef_id: None,
            };
            if let Err(e) = add_linedef.execute(&mut doc_write) {
                eprintln!("Error adding linedef: {}", e);
                return;
            }

            self.last_vertex_id = Some(end_vertex_id);
        }
    }

    fn get_last_vertex_position(&self, doc: &Arc<RwLock<Document>>) -> Option<egui::Pos2> {
        if let Some(vertex_id) = self.last_vertex_id {
            let doc = doc.read();
            let vertices = doc.vertices.read();
            if let Some(vertex) = vertices.get(vertex_id) {
                return Some(egui::pos2(vertex.x as f32, vertex.y as f32));
            }
        }
        None
    }

    fn draw_grid(&self, ui: &mut egui::Ui) {
        let grid_size = self.grid_settings.size as f32;
        let rect = ui.max_rect();
        
        // Calculate grid lines based on view bounds
        let start_x = (rect.min.x / grid_size).floor() * grid_size;
        let end_x = (rect.max.x / grid_size).ceil() * grid_size;
        let start_y = (rect.min.y / grid_size).floor() * grid_size;
        let end_y = (rect.max.y / grid_size).ceil() * grid_size;

        let grid_color = egui::Color32::from_gray(100);
        let stroke = egui::Stroke::new(1.0, grid_color);

        // Draw vertical lines
        for x in (start_x as i32..=end_x as i32).step_by(self.grid_settings.size as usize) {
            let x = x as f32;
            ui.painter().line_segment(
                [egui::pos2(x, start_y), egui::pos2(x, end_y)],
                stroke,
            );
        }

        // Draw horizontal lines
        for y in (start_y as i32..=end_y as i32).step_by(self.grid_settings.size as usize) {
            let y = y as f32;
            ui.painter().line_segment(
                [egui::pos2(start_x, y), egui::pos2(end_x, y)],
                stroke,
            );
        }
    }
}