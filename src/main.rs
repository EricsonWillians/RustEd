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

use rust_ed::*;

use ui::main_window::{MainWindow, WindowConfig};

/// Perform any platform-specific initialization (Windows, Linux, etc.).
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

/// The EGUI/eframe application struct that owns a `MainWindow`.
///
/// `eframe::App` requires an `update(&mut self, &egui::Context, &mut eframe::Frame)` method,
/// which we'll forward to our `MainWindow`.
struct RustEdApp {
    main_window: MainWindow,
}

impl RustEdApp {
    fn new() -> Self {
        // You can load config from a file or use defaults:
        let config = WindowConfig::default();
        let main_window = MainWindow::new(config);

        RustEdApp { main_window }
    }
}

impl eframe::App for RustEdApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Delegate all UI updates to your MainWindow struct
        self.main_window.update(ctx, frame);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging.
    env_logger::init();
    info!("RustEd starting...");

    // Perform platform-specific initialization.
    init_platform();

    // Define windowing options (size, vsync, icon, etc.)
    let native_options = eframe::NativeOptions::default();

    // Launch eframe with our RustEdApp.
    eframe::run_native(
        "RustEd Editor",
        native_options,
        Box::new(|_cc| Box::new(RustEdApp::new())),
    );

    info!("RustEd exiting.");
    Ok(())
}
