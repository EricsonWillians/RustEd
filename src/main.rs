#![warn(non_snake_case)]
//! # RustEd Main Entry Point
//!
//! RustEd is a high-performance, concurrent Doom map editor with procedural generation,
//! inspired by the classic Eureka editor. This file initializes core subsystems (such as
//! BSP processing, document management, editor state, and platform-specific initialization)
//! and then starts the main event loop using eframe/egui.
//!
//! ## License
//! Licensed under the MIT License.
//!
//! ## Authors
//! Your Name <you@example.com>

use env_logger;
use log::info;
use std::error::Error;

mod bsp;
mod document;
mod editor;
mod platform;
mod ui;
mod utils;

/// Perform any platform-specific initialization.
fn init_platform() {
    #[cfg(target_os = "windows")]
    {
        platform::win::init_platform();
    }
    #[cfg(target_os = "linux")]
    {
        platform::x11::init_platform();
    }
}

/// A minimal application that embeds the editor. Here we use a stub application
/// that launches an egui window. In a full implementation, you would integrate your
/// editor instance.
struct RustEdApp {}

impl Default for RustEdApp {
    fn default() -> Self {
        RustEdApp {}
    }
}

impl eframe::App for RustEdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RustEd Editor");
            ui.label("Editor is running. Press ESC to exit.");
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging.
    env_logger::init();
    info!("RustEd starting...");

    // Perform platform-specific initialization.
    init_platform();

    // Set native options for the egui window.
    let native_options = eframe::NativeOptions::default();

    // Run the egui application.
    eframe::run_native(
        "RustEd Editor",
        native_options,
        Box::new(|_cc| Box::new(RustEdApp::default())),
    );
    // run_native returns () so we simply return Ok.
    info!("RustEd exiting.");
    Ok(())
}
