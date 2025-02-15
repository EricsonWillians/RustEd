// src/ui/mod.rs
pub use crate::editor::Command; // Correct import

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
pub mod sidedef;
pub mod theme;
pub mod thing_ui;
pub mod tile;
pub mod vertex_ui;
//pub mod window; //Remove this module

pub use dialog::DialogManager;
pub use theme::Theme;
//pub use tool_window_manager::ToolWindowManager; // Remove or comment out for now

mod tool_window_manager; // Declare the module
pub use tool_window_manager::ToolWindowManager; // and re-export it