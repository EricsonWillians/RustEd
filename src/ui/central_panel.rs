// src/ui/central_panel.rs

use std::sync::Arc;
use eframe::egui::{self, Context, Painter, Pos2, Rect, Sense, Stroke, Color32, Ui};
use parking_lot::RwLock;
use crate::editor::core::Editor;
use crate::bsp::debug_viz::BspDebugger;

/// CentralPanel is the main view/editor area in your UI.
/// We have made `zoom` and `pan` publicly accessible. Alternatively, you can
/// keep them private and add setter/getter methods for each.
pub struct CentralPanel {
    editor: Arc<RwLock<Editor>>,

    /// Zoom factor for the viewport (scales world-to-screen).
    pub zoom: f32,

    /// Pan/offset in screen coordinates.
    pub pan: egui::Vec2,

    bsp_debugger: BspDebugger,

    /// Whether to show the BSP debug overlay/window.
    pub show_bsp_debug: bool,
}

impl CentralPanel {
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self {
            editor,
            zoom: 1.0,
            pan: egui::vec2(0.0, 0.0),
            bsp_debugger: BspDebugger::new(),
            show_bsp_debug: false,
        }
    }

    /// Called every frame to update/draw the central region of the editor.
    pub fn update(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let response = ui.interact(rect, ui.id(), Sense::drag());

                // Zooming with mouse wheel
                self.handle_zoom(ui, &response);

                // Panning by dragging
                self.handle_pan(&response);

                // Fill background
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

                // Draw the grid
                self.draw_grid(&painter, rect);

                // Draw the map geometry (linedefs, vertices, etc.)
                if let Some(doc_arc) = self.editor.read().document() {
                    self.draw_map(&painter, doc_arc.clone(), rect);
                }

                // Handle mouse clicks
                if response.clicked() {
                    if let Some(pos) = ui.input().pointer.interact_pos() {
                        self.handle_click(pos);
                    }
                }

                // Show debug overlay for BSP if needed
                if self.show_bsp_debug {
                    self.show_bsp_debug_window(ctx);
                }
            });
    }

    /// Handles zoom changes (via scroll wheel) while hovering over the central area.
    fn handle_zoom(&mut self, ui: &Ui, response: &egui::Response) {
        if response.hovered() && ui.input().scroll_delta.y.abs() > 0.0 {
            let old_zoom = self.zoom;
            let zoom_sensitivity = 0.001;
            let factor = 1.0 + ui.input().scroll_delta.y * zoom_sensitivity;
            let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);

            if let Some(pointer) = ui.input().pointer.hover_pos() {
                // Re-center on the cursor: Convert cursor to world coords with old zoom,
                // then re-apply new zoom to keep the same world point under the cursor.
                let world_before = self.screen_to_world(pointer);
                self.zoom = new_zoom;
                self.pan = pointer.to_vec2() - world_before.to_vec2() * self.zoom;
            } else {
                self.zoom = new_zoom;
            }
            ui.ctx().request_repaint();
        }
    }

    /// Handles panning by dragging the mouse.
    fn handle_pan(&mut self, response: &egui::Response) {
        if response.dragged() {
            self.pan += response.drag_delta();
            response.ctx.request_repaint();
        }
    }

    /// Draw the map geometry (linedefs, vertices, etc.) on the painter.
    fn draw_map(&self, painter: &Painter, doc: Arc<RwLock<crate::document::Document>>, _rect: Rect) {
        let doc_read = doc.read();
        let vertices = doc_read.vertices.read();
        let linedefs = doc_read.linedefs.read();

        for linedef in linedefs.iter() {
            if linedef.start < vertices.len() && linedef.end < vertices.len() {
                let start_vertex = &vertices[linedef.start];
                let end_vertex = &vertices[linedef.end];

                // Convert from world -> screen
                let p1 = self.world_to_screen(Pos2::new(start_vertex.x as f32, start_vertex.y as f32));
                let p2 = self.world_to_screen(Pos2::new(end_vertex.x as f32, end_vertex.y as f32));

                // Draw the linedef
                painter.line_segment([p1, p2], Stroke::new(1.5, Color32::GREEN));

                // Draw each vertex as a small red circle
                painter.circle_filled(p1, 3.0, egui::Color32::RED);
                painter.circle_filled(p2, 3.0, egui::Color32::RED);
            }
        }
    }

    /// Draw a simple background grid using the current zoom/pan.
    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let grid_spacing_world = 50.0;
        let world_min = self.screen_to_world(rect.min);
        let world_max = self.screen_to_world(rect.max);
        let start_x = (world_min.x / grid_spacing_world).floor() * grid_spacing_world;
        let start_y = (world_min.y / grid_spacing_world).floor() * grid_spacing_world;
        let stroke = Stroke::new(0.5, Color32::from_gray(40));

        let mut x = start_x;
        while x <= world_max.x {
            let p1 = self.world_to_screen(Pos2::new(x, world_min.y));
            let p2 = self.world_to_screen(Pos2::new(x, world_max.y));
            painter.line_segment([p1, p2], stroke);
            x += grid_spacing_world;
        }

        let mut y = start_y;
        while y <= world_max.y {
            let p1 = self.world_to_screen(Pos2::new(world_min.x, y));
            let p2 = self.world_to_screen(Pos2::new(world_max.x, y));
            painter.line_segment([p1, p2], stroke);
            y += grid_spacing_world;
        }
    }

    /// Convert a world coordinate to a screen coordinate given current zoom/pan.
    fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        Pos2::new(
            world_pos.x * self.zoom + self.pan.x,
            world_pos.y * self.zoom + self.pan.y,
        )
    }

    /// Convert a screen coordinate to a world coordinate given current zoom/pan.
    fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.pan.x) / self.zoom,
            (screen_pos.y - self.pan.y) / self.zoom,
        )
    }

    /// Handle a mouse click in screen coordinates. We transform to world coords
    /// and pass to the Editor's `handle_click` method.
    fn handle_click(&self, pos: Pos2) {
        let world_pos = self.screen_to_world(pos);
        let mut editor = self.editor.write();
        editor.handle_click(world_pos);
    }

    /// Show a debug window (or overlay) with the BSP data if available.
    fn show_bsp_debug_window(&mut self, ctx: &Context) {
        egui::Window::new("BSP Debug")
            .resizable(true)
            .default_size(egui::Vec2::new(800.0, 600.0))
            .show(ctx, |ui| {
                let editor = self.editor.read();
                if let Some(bsp_level) = editor.bsp_level() {
                    let root_guard = bsp_level.root.read();
                    if let Some(_root_node) = &*root_guard {
                        // Render some debugging info about the BSP nodes.
                        self.bsp_debugger.show(ui, &bsp_level);
                    } else {
                        ui.label("BSP root node not built yet.");
                    }
                    let subsectors_guard = bsp_level.subsectors.read();
                    ui.label(format!("Number of subsectors: {}", subsectors_guard.len()));
                    let blocks_guard = bsp_level.blocks.read();
                    ui.label(format!("Blockmap size: {}x{}", blocks_guard.width, blocks_guard.height));
                } else {
                    ui.label("No BSP data available.");
                    if ui.button("Generate Test Map").clicked() {
                        drop(editor);
                        self.editor.write().generate_test_map();
                    }
                }
            });
    }

    /// Provide read-only access to the current zoom factor.
    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    /// Provide read-only access to the current pan offset.
    pub fn get_pan(&self) -> egui::Vec2 {
        self.pan
    }

    /// If you want to allow external code to set the zoom, you can provide a setter.
    pub fn set_zoom(&mut self, new_zoom: f32) {
        self.zoom = new_zoom;
    }

    /// Similarly, a setter for pan if you want external code to modify it.
    pub fn set_pan(&mut self, new_pan: egui::Vec2) {
        self.pan = new_pan;
    }
}
