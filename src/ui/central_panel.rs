// src/ui/central_panel.rs

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2, Window,
};
use parking_lot::RwLock;

use crate::bsp::debug_viz::BspDebugger;
use crate::document::Document;
use crate::editor::core::{Editor, Selection, Tool};
use crate::map::{LineDef, Vertex, Thing};

/// CentralPanel is the main view in your UI, handling:
///   - Zoom & pan
///   - Drawing geometry (linedefs, vertices, things)
///   - Hover highlighting
///   - Forwarding clicks to Editor for selection, etc.
pub struct CentralPanel {
    editor: Arc<RwLock<Editor>>,

    /// Current zoom factor (scales world->screen).
    pub zoom: f32,

    /// Current pan offset (screen coords).
    pub pan: Vec2,

    /// If true, shows a BSP debugging overlay/window.
    pub show_bsp_debug: bool,
    bsp_debugger: BspDebugger,

    /// The geometry we are currently hovering (if any). We'll store a copy
    /// matching your Selection variants, because your Selection expects e.g.
    /// `Line(LineDef)` not `Arc<LineDef>`.
    hovered_selection: Selection,
}

impl CentralPanel {
    pub fn new(editor: Arc<RwLock<Editor>>) -> Self {
        Self {
            editor,
            zoom: 1.0,
            pan: Vec2::new(0.0, 0.0),
            show_bsp_debug: false,
            bsp_debugger: BspDebugger::new(),
            hovered_selection: Selection::None,
        }
    }

    /// Called each frame to update the central region.
    pub fn update(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();

                // Let user drag in this area (for pan)
                let response = ui.interact(rect, ui.id(), Sense::click_and_drag());

                // Mouse wheel zoom
                self.handle_zoom(ui, &response);

                // Mouse drag pan
                self.handle_pan(&response);

                // Fill background
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, Color32::BLACK);

                // Draw grid
                self.draw_grid(&painter, rect);

                // Draw geometry if there's a document
                if let Some(doc_arc) = self.editor.read().document() {
                    self.draw_map(&painter, &doc_arc, rect, ui);
                }

                // Hover detection
                if rect.contains(ui.input().pointer.hover_pos().unwrap_or_default()) {
                    if let Some(mouse_pos) = ui.input().pointer.hover_pos() {
                        self.update_hover(mouse_pos);
                    } else {
                        self.hovered_selection = Selection::None;
                    }
                } else {
                    self.hovered_selection = Selection::None;
                }

                // Handle clicks
                if response.clicked() {
                    if let Some(pos) = ui.input().pointer.interact_pos() {
                        self.handle_click(pos);
                    }
                }

                // Possibly show a BSP debug overlay
                if self.show_bsp_debug {
                    self.show_bsp_debug_window(ctx);
                }
            });
    }

    //--- Zoom and Pan Handling ---

    fn handle_zoom(&mut self, ui: &Ui, response: &egui::Response) {
        if response.hovered() && ui.input().scroll_delta.y.abs() > 0.0 {
            let old_zoom = self.zoom;
            let zoom_sensitivity = 0.001;
            let factor = 1.0 + ui.input().scroll_delta.y * zoom_sensitivity;
            let new_zoom = (old_zoom * factor).clamp(0.05, 20.0);

            if let Some(pointer) = ui.input().pointer.hover_pos() {
                // keep mouse pointer stable in world coords
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

    //--- Conversions between world and screen coords ---
    fn world_to_screen(&self, world: Pos2) -> Pos2 {
        Pos2::new(world.x * self.zoom + self.pan.x, world.y * self.zoom + self.pan.y)
    }

    fn screen_to_world(&self, screen: Pos2) -> Pos2 {
        Pos2::new((screen.x - self.pan.x) / self.zoom, (screen.y - self.pan.y) / self.zoom)
    }

    //--- Main draw logic: lines, vertices, things, highlight, etc. ---

    fn draw_map(&self, painter: &Painter, doc_arc: &Arc<RwLock<Document>>, _rect: Rect, ui: &Ui) {
        let doc = doc_arc.read();

        let verts = doc.vertices.read();
        let lines = doc.linedefs.read();
        let things = doc.things.read();

        // 1) Draw linedefs + vertices
        for ld_arc in lines.iter() {
            // each linedef in doc is `Arc<LineDef>`
            let ld: &LineDef = ld_arc.as_ref(); // get the underlying struct

            // check indices in vertices
            if ld.start < verts.len() && ld.end < verts.len() {
                let sv = &*verts[ld.start]; // &Arc<Vertex> -> &Vertex
                let ev = &*verts[ld.end];

                let p1 = self.world_to_screen(Pos2::new(sv.x as f32, sv.y as f32));
                let p2 = self.world_to_screen(Pos2::new(ev.x as f32, ev.y as f32));

                // Are we hovered over this line? Compare to hovered_selection
                let is_line_hovered = match &self.hovered_selection {
                    Selection::Line(hover_ld) => (hover_ld.start == ld.start && hover_ld.end == ld.end),
                    _ => false,
                };

                let line_color = if is_line_hovered { Color32::YELLOW } else { Color32::GREEN };
                painter.line_segment([p1, p2], Stroke::new(2.0, line_color));

                // Are these vertices hovered?
                let start_hover = match &self.hovered_selection {
                    Selection::Vertex(v) => (v.x == sv.x && v.y == sv.y),
                    _ => false,
                };
                let end_hover = match &self.hovered_selection {
                    Selection::Vertex(v) => (v.x == ev.x && v.y == ev.y),
                    _ => false,
                };
                let start_rad = if start_hover { 5.0 } else { 3.0 };
                let end_rad = if end_hover { 5.0 } else { 3.0 };

                painter.circle_filled(p1, start_rad, Color32::RED);
                painter.circle_filled(p2, end_rad, Color32::RED);
            }
        }

        // 2) Draw things
        for thing_arc in things.iter() {
            let th: &Thing = thing_arc.as_ref();
            let screen_pos = self.world_to_screen(Pos2::new(th.x as f32, th.y as f32));

            // check if hovered
            let hovered_thing = match &self.hovered_selection {
                Selection::Thing(t) => (t.x == th.x && t.y == th.y),
                _ => false,
            };

            let radius = if hovered_thing { 6.0 } else { 4.0 };
            let color = if hovered_thing { Color32::LIGHT_BLUE } else { Color32::WHITE };

            painter.circle_filled(screen_pos, radius, color);

            // small label: doom_type
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

    //--- Grid drawing ---
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

    //--- Hover detection & selection ---

    /// Perform a naive hover check. We store the final in `hovered_selection`.
    fn update_hover(&mut self, screen_pos: Pos2) {
        // convert mouse to world coords
        let world_pos = self.screen_to_world(screen_pos);

        self.hovered_selection = Selection::None;

        if let Some(doc_arc) = self.editor.read().document() {
            let doc = doc_arc.read();
            let verts = doc.vertices.read();
            let lines = doc.linedefs.read();
            let things = doc.things.read();

            // threshold for distance checks
            let vertex_thresh_sq = 10.0_f32.powi(2); // 10 units
            let thing_thresh_sq = 12.0_f32.powi(2);  // 12 units
            let line_thresh = 5.0_f32;

            // 1) Check vertices
            for v_arc in verts.iter() {
                let v: &Vertex = v_arc.as_ref();
                let dx = v.x as f32 - world_pos.x;
                let dy = v.y as f32 - world_pos.y;
                if dx*dx + dy*dy < vertex_thresh_sq {
                    // store a clone of the raw Vertex
                    self.hovered_selection = Selection::Vertex(v.clone());
                    return;
                }
            }

            // 2) Check linedefs
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

            // 3) Check things
            for thing_arc in things.iter() {
                let th: &Thing = thing_arc.as_ref();
                let dx = th.x as f32 - world_pos.x;
                let dy = th.y as f32 - world_pos.y;
                if dx*dx + dy*dy < thing_thresh_sq {
                    self.hovered_selection = Selection::Thing(th.clone());
                    return;
                }
            }
        }
    }

    /// If user clicked, pass it to Editor, or do selection logic.
    fn handle_click(&self, screen_pos: Pos2) {
        let world_pos = self.screen_to_world(screen_pos);
        let mut ed = self.editor.write();

        // If something is hovered, you might do:
        if !matches!(self.hovered_selection, Selection::None) {
            // e.g. a direct "select" call:
            // ed.select(self.hovered_selection.clone());
            // For now, we just call `ed.handle_click(world_pos)` to let the Editor decide.
        }

        ed.handle_click(world_pos);
    }

    //--- Debugging the BSP, if you have that system in place ---

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
                    if ui.button("Generate Test Map").clicked() {
                        drop(ed);
                        self.editor.write().generate_test_map();
                    }
                }
            });
    }

    //--- Utility: Expose zoom/pan for external usage ---

    pub fn get_zoom(&self) -> f32 {
        self.zoom
    }

    pub fn get_pan(&self) -> Vec2 {
        self.pan
    }

    pub fn set_zoom(&mut self, z: f32) {
        self.zoom = z;
    }

    pub fn set_pan(&mut self, p: Vec2) {
        self.pan = p;
    }
}

/// Return distance^2 from point P to line segment [A,B].
fn distance_sq_to_segment(
    (ax, ay): (f32, f32),
    (bx, by): (f32, f32),
    (px, py): (f32, f32),
) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let wx = px - ax;
    let wy = py - ay;

    let c1 = wx*vx + wy*vy;
    if c1 <= 0.0 {
        // closest to A
        return wx*wx + wy*wy;
    }
    let c2 = vx*vx + vy*vy;
    if c2 <= c1 {
        // closest to B
        let dx = px - bx;
        let dy = py - by;
        return dx*dx + dy*dy;
    }
    let b = c1 / c2;
    let projx = ax + b*vx;
    let projy = ay + b*vy;
    let dx = px - projx;
    let dy = py - projy;
    dx*dx + dy*dy
}
