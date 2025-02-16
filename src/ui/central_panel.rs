//! Central panel UI module: handles zoom, pan, drawing geometry, hover detection, and
//! input forwarding to the current tool.

use std::sync::Arc;
use parking_lot::RwLock;
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Sense, Stroke, Vec2, Window,
};

use crate::bsp::debug_viz::BspDebugger;
use crate::document::Document;
use crate::editor::core::Editor;
use crate::map::{LineDef, Vertex, Thing};
use crate::editor::tools::Tool;

/// A simple enum for hover detection. This type stores the currently hovered geometry.
#[derive(Clone)]
pub enum Selection {
    None,
    Vertex(Vertex),
    Line(LineDef),
    Thing(Thing),
}

impl Default for Selection {
    fn default() -> Self {
        Selection::None
    }
}

/// The `CentralPanel` struct provides the main viewport for the editor.
/// It is responsible for handling pan/zoom, drawing the map and grid,
/// hover detection, and forwarding pointer input to the active tool.
pub struct CentralPanel {
    editor: Arc<RwLock<Editor>>,

    /// Current zoom factor (scales world→screen).
    zoom: f32,

    /// Current pan offset (in screen coordinates).
    pan: Vec2,

    /// Whether to show the BSP debugging overlay.
    pub show_bsp_debug: bool,
    bsp_debugger: BspDebugger,

    /// The geometry that is currently hovered (if any).
    hovered_selection: Selection,
}

impl CentralPanel {
    /// Create a new central panel instance.
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self {
            editor,
            zoom: 1.0,
            pan: Vec2::new(0.0, 0.0),
            show_bsp_debug: false,
            bsp_debugger: BspDebugger::new(),
            hovered_selection: Selection::default(),
        }
    }

    /// Returns the current zoom factor.
    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    /// Returns the current pan offset.
    pub fn get_pan(&self) -> Vec2 {
        self.pan
    }

    /// Sets the zoom factor.
    pub fn set_zoom(&mut self, z: f32) {
        self.zoom = z;
    }

    /// Sets the pan offset.
    pub fn set_pan(&mut self, p: Vec2) {
        self.pan = p;
    }

    /// Called each frame to update the central panel.
    pub fn update(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // --- Input & Pan/Zoom Handling ---
                if self.editor.read().current_tool_name() == "Select" {
                    // Allow dragging for panning in Select mode.
                    let response = ui.interact(rect, ui.id(), Sense::drag());
                    self.handle_zoom(ui, &response);
                    self.handle_pan(&response);
                } else {
                    // Otherwise, only track hover.
                    let response = ui.interact(rect, ui.id(), Sense::hover());
                    self.handle_zoom(ui, &response);
                }

                // --- Drawing: Background, Grid, Map ---
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);
                self.draw_grid(&painter, rect);

                if let Some(doc_arc) = self.editor.read().document() {
                    self.draw_map(&painter, &doc_arc, rect, ui);
                }

                // --- Hover Detection & Input Forwarding ---
                if rect.contains(ui.input().pointer.hover_pos().unwrap_or_default()) {
                    if let Some(mouse_pos) = ui.input().pointer.hover_pos() {
                        self.update_hover(mouse_pos);
                        let world_pos = self.screen_to_world(mouse_pos);
                        self.editor.write().handle_input(
                            world_pos,
                            ui.input().pointer.primary_clicked(),
                            ui.input().pointer.secondary_clicked(),
                            ui.input().pointer.button_clicked(egui::PointerButton::Middle),
                            ui.input().pointer.button_down(egui::PointerButton::Primary),
                            ui.input().pointer.delta(),
                            ui.input().modifiers,
                        );
                    } else {
                        self.hovered_selection = Selection::None;
                    }
                } else {
                    self.hovered_selection = Selection::None;
                }

                // --- Document Dirty Flag Handling ---
                if let Some(doc_arc) = self.editor.read().document() {
                    let doc = doc_arc.read();
                    if doc.dirty {
                        ctx.request_repaint();
                        drop(doc);
                        doc_arc.write().dirty = false;
                    }
                }

                // --- BSP Debug Window ---
                if self.show_bsp_debug {
                    self.show_bsp_debug_window(ctx);
                }
            });
    }

    // ============================================================
    // Zoom and Pan Handling
    // ============================================================

    fn handle_zoom(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.hovered() && ui.input().scroll_delta.y.abs() > 0.0 {
            let old_zoom = self.zoom;
            let zoom_sensitivity = 0.001;
            let factor = 1.0 + ui.input().scroll_delta.y * zoom_sensitivity;
            let new_zoom = (old_zoom * factor).clamp(0.05, 20.0);

            if let Some(pointer) = ui.input().pointer.hover_pos() {
                // Keep the mouse pointer stable in world coordinates.
                let world_before = self.screen_to_world(pointer);
                self.zoom = new_zoom;
                self.pan = pointer.to_vec2() - world_before.to_vec2() * self.zoom;
            } else {
                self.zoom = new_zoom;
            }
            ui.ctx().request_repaint();
        }
    }

    fn handle_pan(&mut self, response: &egui::Response) {
        if response.dragged() {
            self.pan += response.drag_delta();
            response.ctx.request_repaint();
        }
    }

    // ============================================================
    // Coordinate Conversion
    // ============================================================

    /// Converts world coordinates to screen coordinates.
    fn world_to_screen(&self, world: Pos2) -> Pos2 {
        Pos2::new(world.x * self.zoom + self.pan.x, world.y * self.zoom + self.pan.y)
    }

    /// Converts screen coordinates to world coordinates.
    fn screen_to_world(&self, screen: Pos2) -> Pos2 {
        Pos2::new((screen.x - self.pan.x) / self.zoom, (screen.y - self.pan.y) / self.zoom)
    }

    // ============================================================
    // Drawing Functions
    // ============================================================

    /// Draws the map (linedefs, vertices, things) from the document.
    fn draw_map(&self, painter: &Painter, doc_arc: &Arc<RwLock<Document>>, _rect: Rect, _ui: &egui::Ui) {
        let doc = doc_arc.read();
        let verts = doc.vertices.read();
        let lines = doc.linedefs.read();
        let things = doc.things.read();

        // Draw linedefs and vertices.
        for ld_arc in lines.iter() {
            let ld: &LineDef = ld_arc.as_ref();
            if ld.start < verts.len() && ld.end < verts.len() {
                let sv = &*verts[ld.start];
                let ev = &*verts[ld.end];

                let p1 = self.world_to_screen(Pos2::new(sv.x as f32, sv.y as f32));
                let p2 = self.world_to_screen(Pos2::new(ev.x as f32, ev.y as f32));

                let is_line_hovered = matches!(self.hovered_selection, Selection::Line(ref hover_ld)
                    if hover_ld.start == ld.start && hover_ld.end == ld.end);

                let line_color = if is_line_hovered { Color32::YELLOW } else { Color32::GREEN };
                painter.line_segment([p1, p2], Stroke::new(2.0, line_color));

                let start_hover = matches!(self.hovered_selection, Selection::Vertex(ref v)
                    if v.x == sv.x && v.y == sv.y);
                let end_hover = matches!(self.hovered_selection, Selection::Vertex(ref v)
                    if v.x == ev.x && v.y == ev.y);
                let start_rad = if start_hover { 5.0 } else { 3.0 };
                let end_rad = if end_hover { 5.0 } else { 3.0 };

                painter.circle_filled(p1, start_rad, Color32::RED);
                painter.circle_filled(p2, end_rad, Color32::RED);
            }
        }

        // Draw things.
        for thing_arc in things.iter() {
            let th: &Thing = thing_arc.as_ref();
            let screen_pos = self.world_to_screen(Pos2::new(th.x as f32, th.y as f32));

            let hovered_thing = matches!(self.hovered_selection, Selection::Thing(ref t)
                if t.x == th.x && t.y == th.y);

            let radius = if hovered_thing { 6.0 } else { 4.0 };
            let color = if hovered_thing { Color32::LIGHT_BLUE } else { Color32::WHITE };

            painter.circle_filled(screen_pos, radius, color);

            // Draw a label for the thing's type.
            let label_offset = Vec2::new(8.0, -4.0);
            painter.text(
                screen_pos + label_offset,
                Align2::LEFT_TOP,
                format!("{}", th.doom_type),
                FontId::monospace(12.0),
                color,
            );
        }
    }

    /// Draws a grid in world space.
    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let grid_spacing_world = 64.0;
        let stroke = Stroke::new(1.0, Color32::from_gray(60));

        let world_min = self.screen_to_world(rect.min);
        let world_max = self.screen_to_world(rect.max);

        let start_x = (world_min.x / grid_spacing_world).floor() * grid_spacing_world;
        let start_y = (world_min.y / grid_spacing_world).floor() * grid_spacing_world;

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

    // ============================================================
    // Hover Detection
    // ============================================================

    /// Updates `hovered_selection` by performing a simple distance check on vertices,
    /// lines, and things.
    fn update_hover(&mut self, screen_pos: Pos2) {
        let world_pos = self.screen_to_world(screen_pos);
        self.hovered_selection = Selection::None;

        if let Some(doc_arc) = self.editor.read().document() {
            let doc = doc_arc.read();
            let verts = doc.vertices.read();
            let lines = doc.linedefs.read();
            let things = doc.things.read();

            let vertex_thresh_sq = 10.0_f32.powi(2);
            let thing_thresh_sq = 12.0_f32.powi(2);
            let line_thresh = 5.0_f32;

            // Check vertices.
            for v_arc in verts.iter() {
                let v: &Vertex = v_arc.as_ref();
                let dx = v.x as f32 - world_pos.x;
                let dy = v.y as f32 - world_pos.y;
                if dx * dx + dy * dy < vertex_thresh_sq {
                    self.hovered_selection = Selection::Vertex(v.clone());
                    return;
                }
            }

            // Check linedefs.
            for ld_arc in lines.iter() {
                let ld: &LineDef = ld_arc.as_ref();
                if ld.start >= verts.len() || ld.end >= verts.len() {
                    continue;
                }
                let sv = &*verts[ld.start];
                let ev = &*verts[ld.end];
                let dist_sq = distance_sq_to_segment(
                    (sv.x as f32, sv.y as f32),
                    (ev.x as f32, ev.y as f32),
                    (world_pos.x, world_pos.y),
                );
                if dist_sq < line_thresh.powi(2) {
                    self.hovered_selection = Selection::Line(ld.clone());
                    return;
                }
            }

            // Check things.
            for thing_arc in things.iter() {
                let th: &Thing = thing_arc.as_ref();
                let dx = th.x as f32 - world_pos.x;
                let dy = th.y as f32 - world_pos.y;
                if dx * dx + dy * dy < thing_thresh_sq {
                    self.hovered_selection = Selection::Thing(th.clone());
                    return;
                }
            }
        }
    }

    // ============================================================
    // BSP Debug Window
    // ============================================================

    /// Displays the BSP debug window if BSP data is available.
    fn show_bsp_debug_window(&mut self, ctx: &Context) {
        Window::new("BSP Debug")
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                let ed = self.editor.read();
                if let Some(bsp) = ed.bsp_level() {
                    let root_guard = bsp.root.read();
                    if root_guard.is_some() {
                        self.bsp_debugger.show(ui, &bsp);
                    } else {
                        ui.label("No root node (BSP build incomplete).");
                    }

                    let subsectors_guard = bsp.subsectors.read();
                    ui.label(format!("Subsectors: {}", subsectors_guard.len()));
                    let blocks_guard = bsp.blocks.read();
                    ui.label(format!("Blockmap: {}x{}", blocks_guard.width, blocks_guard.height));
                } else {
                    ui.label("No BSP data available.");
                }
            });
    }
}

/// Returns the squared distance from point P to the line segment [A, B].
fn distance_sq_to_segment(
    (ax, ay): (f32, f32),
    (bx, by): (f32, f32),
    (px, py): (f32, f32),
) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let wx = px - ax;
    let wy = py - ay;

    let c1 = wx * vx + wy * vy;
    if c1 <= 0.0 {
        return wx * wx + wy * wy;
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        let dx = px - bx;
        let dy = py - by;
        return dx * dx + dy * dy;
    }
    let b = c1 / c2;
    let projx = ax + b * vx;
    let projy = ay + b * vy;
    let dx = px - projx;
    let dy = py - projy;
    dx * dx + dy * dy
}
