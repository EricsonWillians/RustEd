// src/editor/tools/things.rs

use super::{Tool, GridSettings};
use crate::document::Document;
use crate::editor::commands::{Command, CommandType};
use crate::map::Thing;
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;
use std::f32::consts::PI;

// Common DOOM thing types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommonThingType {
    Player1Start = 1,
    Player2Start = 2,
    Player3Start = 3,
    Player4Start = 4,
    DeathmatchStart = 11,
    TeleportDest = 14,
    GreenArmor = 2018,
    BlueArmor = 2019,
    HealthBonus = 2014,
    ArmorBonus = 2015,
    Medikit = 2012,
    Stimpack = 2011,
    Shotgun = 2001,
    Chaingun = 2002,
    RocketLauncher = 2003,
    PlasmaRifle = 2004,
    BFG9000 = 2006,
    Imp = 3001,
    Demon = 3002,
    Baron = 3003,
    Zombieman = 3004,
    Cacodemon = 3005,
    LostSoul = 3006,
}

impl CommonThingType {
    fn name(&self) -> &'static str {
        match self {
            Self::Player1Start => "Player 1 Start",
            Self::Player2Start => "Player 2 Start",
            Self::Player3Start => "Player 3 Start",
            Self::Player4Start => "Player 4 Start",
            Self::DeathmatchStart => "Deathmatch Start",
            Self::TeleportDest => "Teleport Destination",
            Self::GreenArmor => "Green Armor",
            Self::BlueArmor => "Blue Armor",
            Self::HealthBonus => "Health Bonus",
            Self::ArmorBonus => "Armor Bonus",
            Self::Medikit => "Medikit",
            Self::Stimpack => "Stimpack",
            Self::Shotgun => "Shotgun",
            Self::Chaingun => "Chaingun",
            Self::RocketLauncher => "Rocket Launcher",
            Self::PlasmaRifle => "Plasma Rifle",
            Self::BFG9000 => "BFG9000",
            Self::Imp => "Imp",
            Self::Demon => "Demon",
            Self::Baron => "Baron of Hell",
            Self::Zombieman => "Zombieman",
            Self::Cacodemon => "Cacodemon",
            Self::LostSoul => "Lost Soul",
        }
    }

    fn all() -> Vec<CommonThingType> {
        vec![
            Self::Player1Start,
            Self::Player2Start,
            Self::Player3Start,
            Self::Player4Start,
            Self::DeathmatchStart,
            Self::TeleportDest,
            Self::GreenArmor,
            Self::BlueArmor,
            Self::HealthBonus,
            Self::ArmorBonus,
            Self::Medikit,
            Self::Stimpack,
            Self::Shotgun,
            Self::Chaingun,
            Self::RocketLauncher,
            Self::PlasmaRifle,
            Self::BFG9000,
            Self::Imp,
            Self::Demon,
            Self::Baron,
            Self::Zombieman,
            Self::Cacodemon,
            Self::LostSoul,
        ]
    }
}

pub struct ThingsTool {
    grid_settings: GridSettings,
    selected_thing: Option<usize>,
    dragging_thing: Option<usize>,
    drag_start: Option<egui::Pos2>,
    current_type: CommonThingType,
    current_angle: i32,
    show_angles: bool,
    custom_type: Option<i32>,
}

impl Default for ThingsTool {
    fn default() -> Self {
        Self {
            grid_settings: GridSettings::default(),
            selected_thing: None,
            dragging_thing: None,
            drag_start: None,
            current_type: CommonThingType::Player1Start,
            current_angle: 0,
            show_angles: true,
            custom_type: None,
        }
    }
}

impl Tool for ThingsTool {
    fn name(&self) -> &'static str {
        "Edit Things"
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
            if modifiers.shift {
                // Place new thing
                self.place_thing(doc, snapped_pos);
            } else {
                // Select thing
                self.select_thing_at(doc, snapped_pos);
                if self.selected_thing.is_some() {
                    self.drag_start = Some(snapped_pos);
                    self.dragging_thing = self.selected_thing;
                }
            }
        }

        if is_dragging && self.dragging_thing.is_some() {
            self.move_thing(doc, drag_delta);
        }

        if secondary_clicked {
            if self.selected_thing.is_some() {
                self.rotate_selected_thing(doc);
            } else {
                self.cleanup();
            }
        }

        // Handle angle adjustment with keyboard
        if modifiers.ctrl {
            if modifiers.shift {
                self.current_angle = (self.current_angle + 45) % 360;
            } else {
                self.current_angle = (self.current_angle - 45 + 360) % 360;
            }
        }
    }

    fn draw(&mut self, ui: &mut egui::Ui, doc: &Arc<RwLock<Document>>) {
        let doc_read = doc.read();
        let things = doc_read.things.read();

        // Draw all things
        for (idx, thing) in things.iter().enumerate() {
            let pos = egui::pos2(thing.x as f32, thing.y as f32);
            let is_selected = Some(idx) == self.selected_thing;
            let color = if is_selected {
                egui::Color32::YELLOW
            } else {
                self.get_thing_color(thing.doom_type)
            };

            // Draw thing circle
            ui.painter().circle_stroke(
                pos,
                if is_selected { 12.0 } else { 8.0 },
                egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, color),
            );

            // Draw angle indicator if enabled
            if self.show_angles {
                let angle_rad = (thing.angle as f32) * PI / 180.0;
                let direction = egui::vec2(angle_rad.cos(), angle_rad.sin());
                let line_end = pos + (direction * 20.0);
                
                ui.painter().line_segment(
                    [pos, line_end],
                    egui::Stroke::new(1.0, color),
                );
            }
        }

        // Draw UI panel for thing properties
        egui::Window::new("Thing Properties")
            .default_pos([10.0, 30.0])
            .show(ui.ctx(), |ui| {
                ui.label("Thing Type:");
                egui::ComboBox::from_label("")
                    .selected_text(self.current_type.name())
                    .show_ui(ui, |ui| {
                        for thing_type in CommonThingType::all() {
                            ui.selectable_value(
                                &mut self.current_type,
                                thing_type,
                                thing_type.name()
                            );
                        }
                    });

                ui.add(egui::Slider::new(&mut self.current_angle, 0..=359)
                    .text("Angle")
                    .clamp_to_range(true));

                ui.checkbox(&mut self.show_angles, "Show Angles");

                if let Some(thing_id) = self.selected_thing {
                    if let Some(thing) = things.get(thing_id) {
                        ui.separator();
                        ui.label(format!("Selected Thing ID: {}", thing_id));
                        ui.label(format!("Position: ({}, {})", thing.x, thing.y));
                        ui.label(format!("Type: {}", thing.doom_type));
                        ui.label(format!("Angle: {}", thing.angle));
                        ui.label(format!("Flags: {:#04x}", thing.flags));
                    }
                }
            });
    }

    fn cleanup(&mut self) {
        self.selected_thing = None;
        self.dragging_thing = None;
        self.drag_start = None;
    }
}

impl ThingsTool {
    fn get_thing_color(&self, doom_type: i32) -> egui::Color32 {
        match doom_type {
            1..=4 => egui::Color32::GREEN,  // Player starts
            11 => egui::Color32::YELLOW,    // Deathmatch starts
            2001..=2099 => egui::Color32::BLUE,  // Weapons and items
            3001..=3999 => egui::Color32::RED,   // Monsters
            _ => egui::Color32::WHITE,      // Other things
        }
    }

    fn place_thing(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        let doom_type = self.custom_type.unwrap_or(self.current_type as i32);
        let mut cmd = CommandType::AddThing {
            x: pos.x as i32,
            y: pos.y as i32,
            angle: self.current_angle,
            doom_type,
            flags: 0x0007, // Default flags - Easy, Medium, Hard difficulties
            thing_id: None,
        };

        let mut doc = doc.write();
        if let Err(e) = cmd.execute(&mut doc) {
            println!("Error placing thing: {}", e);
        }
    }

    fn select_thing_at(&mut self, doc: &Arc<RwLock<Document>>, pos: egui::Pos2) {
        let doc_read = doc.read();
        let things = doc_read.things.read();

        self.selected_thing = things.iter().enumerate()
            .find(|(_, thing)| {
                let thing_pos = egui::pos2(thing.x as f32, thing.y as f32);
                thing_pos.distance(pos) < 12.0
            })
            .map(|(idx, _)| idx);
    }

    fn move_thing(&mut self, doc: &Arc<RwLock<Document>>, drag_delta: egui::Vec2) {
        if let Some(thing_id) = self.dragging_thing {
            let delta = if self.grid_settings.enabled {
                egui::vec2(
                    (drag_delta.x / self.grid_settings.size as f32).round() * self.grid_settings.size as f32,
                    (drag_delta.y / self.grid_settings.size as f32).round() * self.grid_settings.size as f32,
                )
            } else {
                drag_delta
            };

            let mut cmd = CommandType::MoveThing {
                thing_id,
                dx: delta.x as i32,
                dy: delta.y as i32,
            };

            let mut doc = doc.write();
            if let Err(e) = cmd.execute(&mut doc) {
                println!("Error moving thing: {}", e);
            }
        }
    }

    fn rotate_selected_thing(&mut self, doc: &Arc<RwLock<Document>>) {
        if let Some(thing_id) = self.selected_thing {
            let mut cmd = CommandType::RotateThing {
                thing_id,
                new_angle: (self.current_angle + 45) % 360,
            };

            let mut doc = doc.write();
            if let Err(e) = cmd.execute(&mut doc) {
                println!("Error rotating thing: {}", e);
            }
        }
    }
}