// src/ui/mod.rs
pub mod dialog;
pub mod file;
pub mod main_window;
pub mod menu;
pub mod theme;
pub mod central_panel;
pub mod side_panel;
pub mod status_bar;
pub use dialog::DialogManager;
pub use theme::Theme;
mod tool_window_manager; 
pub use tool_window_manager::ToolWindowManager; 