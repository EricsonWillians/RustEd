// src/ui/main_window.rs

use std::sync::Arc;
use eframe::egui::{self, Context};
use parking_lot::RwLock;

use crate::editor::core::Editor;
use crate::ui::{
    menu::MenuBar,
    side_panel::SidePanel,
    central_panel::CentralPanel,
    status_bar::StatusBar,
    dialog::DialogManager, // Keep DialogManager
    Theme,
};

#[derive(Clone, Debug)]
pub struct WindowConfig {
    pub default_width: u32,
    pub default_height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub theme: Theme,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            default_width: 1280,
            default_height: 800,
            min_width: 800,
            min_height: 600,
            theme: Theme::default(),
        }
    }
}
pub struct MainWindow {
    config: WindowConfig,
    editor: Arc<RwLock<Editor>>,
    dialog_manager: DialogManager, // Keep DialogManager
    menu_bar: MenuBar,
    side_panel: SidePanel,
    central_panel: CentralPanel,
    status_bar: StatusBar,
}

impl MainWindow {
    pub fn new(config: WindowConfig) -> Self {
        let doc = Arc::new(RwLock::new(crate::document::Document::new()));
        let editor = Arc::new(RwLock::new(Editor::new(doc)));

        Self {
            config: config.clone(),
            editor: editor.clone(),
            dialog_manager: DialogManager::new(),
            menu_bar: MenuBar::new(editor.clone()), // Pass the editor to components that need it.
            side_panel: SidePanel::new(editor.clone()),
            central_panel: CentralPanel::new(editor.clone()),
            status_bar: StatusBar::new(editor.clone()),
        }
    }

    pub fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Handle top-level input (like global shortcuts)
        self.handle_input(ctx);

        // Update UI elements.  These now call into the separate components.
        self.menu_bar.update(ctx);
        self.side_panel.update(ctx);
        self.central_panel.update(ctx);
        self.status_bar.update(ctx);
        self.dialog_manager.update(ctx); // Update dialogs

    }
     fn handle_input(&mut self, ctx: &Context) {
        let input = ctx.input();
        // Toggle BSP debug window with F11.
        if input.key_pressed(egui::Key::F11) {
           self.central_panel.show_bsp_debug = !self.central_panel.show_bsp_debug;
        }
    }
}