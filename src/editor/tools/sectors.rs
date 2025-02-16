use super::{Tool, GridSettings};
use crate::document::Document;
use crate::editor::commands::{Command, CommandType, SectorProperties};
use crate::map::{Sector, LineDef};
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommonSectorType {
    Normal = 0,
    BlinkingLights = 1,
    SecretArea = 9,
    DoorClose30Seconds = 10,
    SyncStrobeFast = 12,
    SyncStrobeSlow = 13,
    Door = 14,
    LowDamage = 7,
    HighDamage = 5,
    DeathPit = 11,
}

impl CommonSectorType {
    fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::BlinkingLights => "Blinking Lights",
            Self::SecretArea => "Secret Area",
            Self::DoorClose30Seconds => "Door Close (30s)",
            Self::SyncStrobeFast => "Sync Strobe Fast",
            Self::SyncStrobeSlow => "Sync Strobe Slow",
            Self::Door => "Door",
            Self::LowDamage => "Low Damage",
            Self::HighDamage => "High Damage",
            Self::DeathPit => "Death Pit",
        }
    }

    fn all() -> Vec<CommonSectorType> {
        vec![
            Self::Normal,
            Self::BlinkingLights,
            Self::SecretArea,
            Self::DoorClose30Seconds,
            Self::SyncStrobeFast,
            Self::SyncStrobeSlow,
            Self::Door,
            Self::LowDamage,
            Self::HighDamage,
            Self::DeathPit,
        ]
    }
}

const COMMON_FLATS: &[(&str, &str)] = &[
    ("FLOOR0_1", "Basic Floor"),
    ("FLOOR0_3", "Carpet"),
    ("FLOOR1_1", "Metal"),
    ("FLOOR4_1", "Blue Tiles"),
    ("FLOOR4_5", "Green Tiles"),
    ("FLOOR4_8", "Red Tiles"),
    ("CEIL1_1", "Basic Ceiling"),
    ("CEIL3_1", "Brown Ceiling"),
    ("CEIL3_2", "Metal Ceiling"),
    ("FLAT1", "Light Panel"),
    ("FLAT14", "Water"),
    ("FLAT5_4", "Blue Panel"),
    ("NUKAGE1", "Nukage"),
    ("BLOOD1", "Blood"),
    ("LAVA1", "Lava"),
];

pub struct SectorsTool {
    grid_settings: GridSettings,
    selected_sector: Option<usize>,
    selected_type: CommonSectorType,
    floor_height: i32,
    ceiling_height: i32,
    light_level: i32,
    floor_tex: String,
    ceiling_tex: String,
    tag: i32,
    show_heights: bool,
    show_light_levels: bool,
}

impl Default for SectorsTool {
    fn default() -> Self {
        Self {
            grid_settings: GridSettings::default(),
            selected_sector: None,
            selected_type: CommonSectorType::Normal,
            floor_height: 0,
            ceiling_height: 128,
            light_level: 192,
            floor_tex: "FLOOR0_1".to_string(),
            ceiling_tex: "CEIL1_1".to_string(),
            tag: 0,
            show_heights: true,
            show_light_levels: true,
        }
    }
}

impl Tool for SectorsTool {
    fn name(&self) -> &'static str {
        "Edit Sectors"
    }

    fn handle_input(
        &mut self,
        doc: &Arc<RwLock<Document>>,
        world_pos: egui::Pos2,
        primary_clicked: bool,
        secondary_clicked: bool,
        is_dragging: bool,
        _drag_delta: egui::Vec2,
        modifiers: egui::Modifiers,
    ) {
        if primary_clicked {
            self.select_sector_at(doc, world_pos);
        }

        if secondary_clicked {
            self.cleanup();
        }

        if modifiers.ctrl && self.selected_sector.is_some() {
            if modifiers.shift {
                self.adjust_ceiling_height(doc, if is_dragging { 8 } else { -8 });
            } else {
                self.adjust_floor_height(doc, if is_dragging { 8 } else { -8 });
            }
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>) {
        let doc_read = doc.read();
        let sectors = doc_read.sectors.read();
        let lines = doc_read.linedefs.read();

        for (idx, sector) in sectors.iter().enumerate() {
            let is_selected = Some(idx) == self.selected_sector;
            let color = if is_selected {
                egui::Color32::YELLOW
            } else {
                self.get_sector_color(sector)
            };

            self.draw_sector_boundaries(ui, &lines, idx, color);

            if self.show_heights {
                self.draw_height_indicators(ui, doc, sector, idx, is_selected);
            }

            if self.show_light_levels {
                self.draw_light_indicator(ui, doc, sector, idx, is_selected);
            }
        }

        self.draw_properties_panel(ui, doc, &sectors);
    }

    fn cleanup(&mut self) {
        self.selected_sector = None;
    }
}

impl SectorsTool {
    fn select_sector_at(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        let doc_read = doc.read();
        let sectors = doc_read.sectors.read();

        if let Some(sector_id) = self.find_sector_at_position(doc, pos) {
            if let Some(sector) = sectors.get(sector_id) {
                self.selected_sector = Some(sector_id);
                self.floor_height = sector.floor_height;
                self.ceiling_height = sector.ceiling_height;
                self.light_level = sector.light;
                self.floor_tex = sector.floor_tex.clone();
                self.ceiling_tex = sector.ceiling_tex.clone();
                self.tag = sector.tag;
                self.selected_type = CommonSectorType::all()
                    .into_iter()
                    .find(|&t| t as i32 == sector.r#type)
                    .unwrap_or(CommonSectorType::Normal);
            }
        }
    }

    fn apply_sector_changes(&self, doc: &Arc<RwLock<Document>>, sector_id: usize) {
        let mut cmd = CommandType::ModifySector {
            sector_id,
            floor_height: Some(self.floor_height),
            ceiling_height: Some(self.ceiling_height),
            floor_tex: Some(self.floor_tex.clone()),
            ceiling_tex: Some(self.ceiling_tex.clone()),
            light: Some(self.light_level),
            r#type: Some(self.selected_type as i32),
            tag: Some(self.tag),
        };

        let mut doc = doc.write();
        if let Ok(_) = cmd.execute(&mut doc) {
            // Command executed successfully
        } else {
            println!("Error modifying sector");
        }
    }

    fn adjust_floor_height(&mut self, doc: &Arc<RwLock<Document>>, delta: i32) {
        if let Some(sector_id) = self.selected_sector {
            let mut cmd = CommandType::ModifySector {
                sector_id,
                floor_height: Some(self.floor_height + delta),
                ceiling_height: None,
                floor_tex: None,
                ceiling_tex: None,
                light: None,
                r#type: None,
                tag: None,
            };

            let mut doc = doc.write();
            if let Ok(_) = cmd.execute(&mut doc) {
                self.floor_height += delta;
            }
        }
    }

    fn adjust_ceiling_height(&mut self, doc: &Arc<RwLock<Document>>, delta: i32) {
        if let Some(sector_id) = self.selected_sector {
            let mut cmd = CommandType::ModifySector {
                sector_id,
                floor_height: None,
                ceiling_height: Some(self.ceiling_height + delta),
                floor_tex: None,
                ceiling_tex: None,
                light: None,
                r#type: None,
                tag: None,
            };

            let mut doc = doc.write();
            if let Ok(_) = cmd.execute(&mut doc) {
                self.ceiling_height += delta;
            }
        }
    }

    fn get_sector_color(&self, sector: &Sector) -> egui::Color32 {
        match sector.r#type {
            0 => egui::Color32::from_rgb(100, 100, 100),  // Normal
            1 | 12 | 13 => egui::Color32::from_rgb(150, 150, 100),  // Light effects
            5 | 7 | 11 => egui::Color32::from_rgb(150, 50, 50),     // Damage
            9 => egui::Color32::from_rgb(100, 150, 100),            // Secret
            14 => egui::Color32::from_rgb(100, 100, 150),           // Door
            _ => egui::Color32::from_rgb(120, 120, 120),            // Other
        }
    }

    fn draw_sector_boundaries(&self, _ui: &mut egui::Ui, lines: &[Arc<LineDef>], sector_id: usize, _color: egui::Color32) {
        // Draw all lines that belong to this sector
        for line in lines {
            if line.right == sector_id as i32 || line.left == sector_id as i32 {
                // Get line vertices and draw
                // TODO: Implement proper line drawing with vertices
            }
        }
    }

    fn draw_height_indicators(&self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>, sector: &Sector, idx: usize, is_selected: bool) {
        if let Some(center) = self.calculate_sector_center(doc, idx) {
            if is_selected {
                ui.painter().text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    format!("F:{} C:{}", sector.floor_height, sector.ceiling_height),
                    egui::FontId::proportional(14.0),
                    egui::Color32::YELLOW,
                );
            }
        }
    }

    fn draw_light_indicator(&self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>, sector: &Sector, idx: usize, is_selected: bool) {
        if let Some(pos) = self.calculate_sector_center(doc, idx) {
            if is_selected {
                ui.painter().text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    format!("Light: {}", sector.light),
                    egui::FontId::proportional(12.0),
                    egui::Color32::YELLOW,
                );
            }
        }
    }

    fn draw_properties_panel(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>, sectors: &[Arc<Sector>]) {
        egui::Window::new("Sector Properties")
            .default_pos([10.0, 30.0])
            .show(ui.ctx(), |ui| {
                if let Some(sector_id) = self.selected_sector {
                    if let Some(sector) = sectors.get(sector_id) {
                        ui.label(format!("Sector ID: {}", sector_id));
                        
                        ui.label("Sector Type:");
                        egui::ComboBox::from_label("")
                            .selected_text(self.selected_type.name())
                            .show_ui(ui, |ui| {
                                for sector_type in CommonSectorType::all() {
                                    ui.selectable_value(
                                        &mut self.selected_type,
                                        sector_type,
                                        sector_type.name()
                                    );
                                }
                            });

                        ui.add(egui::Slider::new(&mut self.floor_height, -512..=512)
                            .text("Floor Height")
                            .clamp_to_range(true));
                        ui.add(egui::Slider::new(&mut self.ceiling_height, -512..=512)
                            .text("Ceiling Height")
                            .clamp_to_range(true));

                        ui.add(egui::Slider::new(&mut self.light_level, 0..=255)
                            .text("Light Level")
                            .clamp_to_range(true));

                        ui.label("Floor Texture:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.floor_tex)
                            .show_ui(ui, |ui| {
                                for &(flat, name) in COMMON_FLATS {
                                    if ui.selectable_label(
                                        self.floor_tex == flat,
                                        name
                                    ).clicked() {
                                        self.floor_tex = flat.to_string();
                                    }
                                }
                            });

                        ui.label("Ceiling Texture:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.ceiling_tex)
                            .show_ui(ui, |ui| {
                                for &(flat, name) in COMMON_FLATS {
                                    if ui.selectable_label(
                                        self.ceiling_tex == flat,
                                        name
                                    ).clicked() {
                                        self.ceiling_tex = flat.to_string();
                                    }
                                }
                            });

                        ui.add(egui::Slider::new(&mut self.tag, 0..=255)
                            .text("Tag")
                            .clamp_to_range(true));

                        ui.checkbox(&mut self.show_heights, "Show Heights");
                        ui.checkbox(&mut self.show_light_levels, "Show Light Levels");

                        if ui.button("Apply Changes").clicked() {
                            self.apply_sector_changes(doc, sector_id);
                        }
                    }
                } else {
                    ui.label("No sector selected");
                }
            });
    }

    // Helper functions for sector manipulation
    fn find_sector_at_position(&self, doc: &Arc<RwLock<Document>>, point: egui::Pos2) -> Option<usize> {
        let doc_read = doc.read();
        let sectors = doc_read.sectors.read();

        for (idx, _) in sectors.iter().enumerate() {
            if self.is_point_in_sector(doc, idx, point) {
                return Some(idx);
            }
        }
        None
    }

    fn is_point_in_sector(&self, doc: &Arc<RwLock<Document>>, sector_id: usize, point: egui::Pos2) -> bool {
        let vertices = self.get_sector_vertices(doc, sector_id);
        if vertices.len() < 3 {
            return false;
        }

        let mut inside = false;
        let mut j = vertices.len() - 1;

        for i in 0..vertices.len() {
            if ((vertices[i].y > point.y) != (vertices[j].y > point.y)) &&
                (point.x < (vertices[j].x - vertices[i].x) * (point.y - vertices[i].y) /
                (vertices[j].y - vertices[i].y) + vertices[i].x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    fn get_sector_vertices(&self, doc: &Arc<RwLock<Document>>, sector_id: usize) -> Vec<egui::Pos2> {
        let doc_read = doc.read();
        let linedefs = doc_read.linedefs.read();
        let vertices = doc_read.vertices.read();
        let mut result = Vec::new();
        let mut processed_lines = std::collections::HashSet::new();

        // Find the first linedef that belongs to this sector
        if let Some((first_idx, first_line)) = linedefs.iter().enumerate().find(|(_, line)| {
            line.right == sector_id as i32 || line.left == sector_id as i32
        }) {
            let mut current_line = first_line;
            let mut current_vertex = current_line.start;
            processed_lines.insert(first_idx);

            loop {
                // Add current vertex to result
                if let Some(vertex) = vertices.get(current_vertex as usize) {
                    result.push(egui::pos2(vertex.x as f32, vertex.y as f32));
                }

                // Find next connected line
                current_vertex = current_line.end;
                let next_line = linedefs.iter().enumerate()
                    .find(|(idx, line)| {
                        !processed_lines.contains(idx) &&
                        (line.right == sector_id as i32 || line.left == sector_id as i32) &&
                        (line.start == current_vertex || line.end == current_vertex)
                    });

                if let Some((idx, line)) = next_line {
                    processed_lines.insert(idx);
                    current_line = line;
                } else {
                    break;
                }
            }
        }

        result
    }

    fn calculate_sector_center(&self, doc: &Arc<RwLock<Document>>, sector_id: usize) -> Option<egui::Pos2> {
        let vertices = self.get_sector_vertices(doc, sector_id);
        if vertices.is_empty() {
            return None;
        }

        let sum = vertices.iter().fold(egui::pos2(0.0, 0.0), |acc, p| {
            egui::pos2(acc.x + p.x, acc.y + p.y)
        });
        let count = vertices.len() as f32;

        Some(egui::pos2(sum.x / count, sum.y / count))
    }

    fn merge_sectors(&mut self, doc: &Arc<RwLock<Document>>, sector_a: usize, sector_b: usize) {
        let mut cmd = CommandType::MergeSectors {
            sector_a,
            sector_b,
            target_properties: SectorProperties {
                floor_height: self.floor_height,
                ceiling_height: self.ceiling_height,
                floor_tex: self.floor_tex.clone(),
                ceiling_tex: self.ceiling_tex.clone(),
                light: self.light_level,
                r#type: self.selected_type as i32,
                tag: self.tag,
            },
        };

        let mut doc = doc.write();
        if let Err(e) = cmd.execute(&mut doc) {
            println!("Error merging sectors: {}", e);
        }
    }

    fn split_sector(&mut self, doc: &Arc<RwLock<Document>>, sector_id: usize, split_line: (egui::Pos2, egui::Pos2)) {
        let mut commands = vec![
            CommandType::AddVertex {
                x: split_line.0.x as i32,
                y: split_line.0.y as i32,
                vertex_id: None,
            },
            CommandType::AddVertex {
                x: split_line.1.x as i32,
                y: split_line.1.y as i32,
                vertex_id: None,
            },
        ];

        // Create new linedef for the split
        commands.push(CommandType::AddLineDef {
            start: 0,
            end: 1,
            flags: 0x0001,
            line_type: 0,
            tag: 0,
            right: sector_id as i32,
            left: -1,
            linedef_id: None,
        });

        // Create new sector with same properties
        commands.push(CommandType::AddSector {
            floor_height: self.floor_height,
            ceiling_height: self.ceiling_height,
            floor_tex: self.floor_tex.clone(),
            ceiling_tex: self.ceiling_tex.clone(),
            light: self.light_level,
            r#type: self.selected_type as i32,
            tag: self.tag,
            sector_id: None,
        });

        let mut batch_cmd = CommandType::BatchCommand { commands };
        let mut doc = doc.write();
        if let Err(e) = batch_cmd.execute(&mut doc) {
            println!("Error splitting sector: {}", e);
        }
    }

    fn get_sector_sidedefs(&self, doc: &Arc<RwLock<Document>>, sector_id: usize) -> Vec<usize> {
        let doc_read = doc.read();
        let linedefs = doc_read.linedefs.read();
        
        linedefs.iter()
            .filter_map(|line| {
                if line.right == sector_id as i32 {
                    Some(line.right as usize)
                } else if line.left == sector_id as i32 {
                    Some(line.left as usize)
                } else {
                    None
                }
            })
            .collect()
    }

    fn sectors_are_adjacent(&self, doc: &Arc<RwLock<Document>>, sector_a: usize, sector_b: usize) -> bool {
        let doc_read = doc.read();
        let linedefs = doc_read.linedefs.read();

        linedefs.iter().any(|line| {
            (line.right == sector_a as i32 && line.left == sector_b as i32) ||
            (line.right == sector_b as i32 && line.left == sector_a as i32)
        })
    }
}