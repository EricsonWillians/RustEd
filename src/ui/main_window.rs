//! # Main Window Module
//!
//! This module provides the main user interface for RustEd using eframe/egui.
//! It defines a `MainWindow` struct that encapsulates basic UI components:
//! - A top menu bar for common actions.
//! - A left side panel for tools.
//! - A central canvas area (placeholder for the editor view).
//! - A bottom status bar for messages.
//!
//! The module also provides a helper function `run_main_window()` to launch
//! the UI as a standalone egui application.

use eframe::egui;
use eframe::egui::{CentralPanel, SidePanel, TopBottomPanel};
use std::error::Error;

/// MainWindow holds the state of the UI.
pub struct MainWindow {
    /// The width of the canvas area.
    canvas_width: u32,
    /// The height of the canvas area.
    canvas_height: u32,
    /// A status message to display in the status bar.
    status_message: String,
}

impl MainWindow {
    /// Creates a new MainWindow with default settings.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(MainWindow {
            canvas_width: 800,
            canvas_height: 600,
            status_message: "Welcome to RustEd!".to_owned(),
        })
    }

    /// Returns the canvas width.
    pub fn canvas_width(&self) -> u32 {
        self.canvas_width
    }

    /// Returns the canvas height.
    pub fn canvas_height(&self) -> u32 {
        self.canvas_height
    }

    /// Updates the status message.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }

    /// Draws the complete UI layout.
    pub fn update(&mut self, ctx: &egui::Context) {
        // Top menu bar.
        TopBottomPanel::top("top_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("File").clicked() {
                    self.set_status("File menu clicked");
                }
                if ui.button("Edit").clicked() {
                    self.set_status("Edit menu clicked");
                }
                if ui.button("View").clicked() {
                    self.set_status("View menu clicked");
                }
                if ui.button("Help").clicked() {
                    self.set_status("Help menu clicked");
                }
            });
        });

        // Left side panel for tools.
        SidePanel::left("side_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Tools");
                if ui.button("Select").clicked() {
                    self.set_status("Select tool activated");
                }
                if ui.button("Draw").clicked() {
                    self.set_status("Draw tool activated");
                }
                if ui.button("Erase").clicked() {
                    self.set_status("Erase tool activated");
                }
            });
        });

        // Central canvas area.
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor Canvas");
            ui.label("This area will display the map and editing tools.");
        });

        // Bottom status bar.
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status_message));
            });
        });
    }
}

/// A wrapper to integrate MainWindow into an eframe App.
struct MainWindowApp {
    window: MainWindow,
}

impl Default for MainWindowApp {
    fn default() -> Self {
        let window = MainWindow::new().unwrap_or_else(|_| MainWindow {
            canvas_width: 800,
            canvas_height: 600,
            status_message: "Initialization error".to_owned(),
        });
        MainWindowApp { window }
    }
}

impl eframe::App for MainWindowApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.window.update(ctx);
    }
}

/// Runs the MainWindow UI as a standalone egui application.
pub fn run_main_window() -> Result<(), Box<dyn Error>> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RustEd Main Window",
        native_options,
        Box::new(|_cc| Box::new(MainWindowApp::default())),
    );
    // Since run_native returns (), we simply return Ok.
    Ok(())
}
