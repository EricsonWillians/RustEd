// src/ui/mod.rs
pub mod about;
pub mod browser;
pub mod canvas;
pub mod dialog;
pub mod editor_ui;
pub mod file;
pub mod hyper;
pub mod infobar;
pub mod linedef_ui;
pub mod main_window;
pub mod menu;
pub mod misc;
pub mod nombre;
pub mod panelinput;
pub mod pic;
pub mod prefs;
pub mod replace;
pub mod scroll;
pub mod sector_ui;
pub mod theme;
pub mod thing_ui;
pub mod tile;
pub mod vertex_ui;
// pub mod commands; // Don't need this here, commands are under editor.


//pub use commands::Command; // No longer needed here.
//pub mod window; //Remove this module

pub use dialog::DialogManager;
pub use theme::Theme;
//pub use tool_window_manager::ToolWindowManager; // Remove or comment out for now

mod tool_window_manager; // Declare the module
pub use tool_window_manager::ToolWindowManager; // and re-export it

// use crate::document::{Document, Vertex, LineDef, Sector, Thing}; // Not needed in this file
// use std::sync::Arc; // Not needed

// pub trait Command {  // This is now in src/editor/commands.rs
//     fn execute(&self, document: &mut Document) -> Result<(), String>;
//     fn unexecute(&self, document: &mut Document) -> Result<(), String>;
// }

// The enum and impl Command for CommandType are part of the editor now
// so they've moved to src/editor/commands.rs

// #[derive(Clone, Debug)]
// pub enum CommandType { ... }

// impl Command for CommandType { ... }

// pub struct BatchCommand { ... }

// impl Command for BatchCommand { ... }