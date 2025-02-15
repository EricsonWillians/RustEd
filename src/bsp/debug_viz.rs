//! src/bsp/debug_viz.rs
//! Brutally simplified to avoid private-field errors and child-type mismatches.

use std::sync::Arc;

use eframe::egui;
use egui::{Color32, Rect, Stroke, Vec2};

use crate::bsp::{
    BspLevel,
    BspNode,
    Seg,
    BLOCK_SIZE,
};
use crate::utils::geometry::{Line2D, Point2D as GeoPoint2D};

/// If you're debugging procedural generation:
use crate::bsp::bsp_procedural::ProceduralGenerator;

/// Debug UI to visualize a BSP.
pub struct BspDebugger {
    zoom: f32,
    pan: Vec2,

    show_grid: bool,
    show_blockmap: bool,
    show_bsp_tree: bool,
    show_subsectors: bool,
    display_stats: bool,

    highlight_node: Option<usize>,
    selected_seg: Option<usize>,

    node_colors: Vec<Color32>,
}

impl Default for BspDebugger {
    fn default() -> Self {
        BspDebugger {
            zoom: 1.0,
            pan: Vec2::ZERO,
            show_grid: true,
            show_blockmap: false,
            show_bsp_tree: true,
            show_subsectors: true,
            display_stats: true,
            highlight_node: None,
            selected_seg: None,
            node_colors: vec![
                Color32::GREEN,
                Color32::BLUE,
                Color32::from_rgb(155, 89, 182), // purple
                Color32::RED,
                Color32::YELLOW,
            ],
        }
    }
}

impl BspDebugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// The main function that draws the BSP debug overlays and handles user input.
    pub fn show(&mut self, ui: &mut egui::Ui, bsp_level: &Arc<BspLevel>) {
        // 1) The control panel
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.show_blockmap, "Blockmap");
            ui.checkbox(&mut self.show_bsp_tree, "BSP Tree");
            ui.checkbox(&mut self.show_subsectors, "Subsectors");
            ui.checkbox(&mut self.display_stats, "Stats");

            if ui.button("Reset View").clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }

            ui.add(egui::Slider::new(&mut self.zoom, 0.1..=20.0).text("Zoom"));
        });

        // 2) Main canvas for painting
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());

        // 3) Pan if dragging
        if response.dragged() {
            self.pan += response.drag_delta();
        }

        // 4) Scroll-wheel zoom
        if let Some(hover_pos) = response.hover_pos() {
            let scroll_delta = ui.input().scroll_delta.y / 1000.0;
            if scroll_delta != 0.0 {
                let local_pos = hover_pos - response.rect.min;
                let old_world = self.screen_to_world(local_pos, response.rect);
                self.zoom *= 1.0 + scroll_delta;
                let new_world = self.screen_to_world(local_pos, response.rect);
                self.pan += (new_world - old_world) * self.zoom;
            }
        }

        // 5) Draw layers
        if self.show_grid {
            self.draw_grid(&painter, response.rect);
        }
        if self.show_blockmap {
            self.draw_blockmap(&painter, response.rect, bsp_level);
        }
        if self.show_bsp_tree {
            self.draw_bsp_tree(&painter, response.rect, bsp_level);
        }
        if self.show_subsectors {
            self.draw_subsectors(&painter, response.rect, bsp_level);
        }

        // 6) Stats overlay
        if self.display_stats {
            self.draw_stats(ui, bsp_level);
        }

        // 7) Handle selection on click
        if let Some(hover) = response.hover_pos() {
            let local_pos = hover - response.rect.min;
            let world_pos = self.screen_to_world(local_pos, response.rect);

            if response.clicked() {
                self.select_closest_seg(world_pos, bsp_level);
            }

            // Show coordinates
            ui.ctx().debug_painter().text(
                hover,
                egui::Align2::LEFT_BOTTOM,
                format!("X: {:.1}, Y: {:.1}", world_pos.x, world_pos.y),
                egui::FontId::default(),
                Color32::WHITE,
            );
        }
    }

    /// Convert screen coords to world coords
    fn screen_to_world(&self, screen: Vec2, rect: Rect) -> Vec2 {
        let center = rect.center();
        (screen - center.to_vec2() - self.pan) / self.zoom
    }

    /// Convert world coords to screen coords
    fn world_to_screen(&self, world: Vec2, rect: Rect) -> Vec2 {
        let center = rect.center();
        world * self.zoom + self.pan + center.to_vec2()
    }

    // ----------------------------------------------------------------
    // Drawing
    // ----------------------------------------------------------------
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let cell_size = 64.0;
        let color = Color32::from_rgba_premultiplied(200, 200, 200, 60);
        let stroke = Stroke::new(1.0, color);

        let top_left = self.screen_to_world(Vec2::ZERO, rect);
        let bot_right = self.screen_to_world(rect.size(), rect);

        let min_x = (top_left.x / cell_size).floor() as i32;
        let max_x = (bot_right.x / cell_size).ceil() as i32;
        let min_y = (top_left.y / cell_size).floor() as i32;
        let max_y = (bot_right.y / cell_size).ceil() as i32;

        // vertical lines
        for x in min_x..=max_x {
            let fx = x as f32 * cell_size;
            let start = self.world_to_screen(Vec2::new(fx, min_y as f32 * cell_size), rect);
            let end = self.world_to_screen(Vec2::new(fx, max_y as f32 * cell_size), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
        // horizontal lines
        for y in min_y..=max_y {
            let fy = y as f32 * cell_size;
            let start = self.world_to_screen(Vec2::new(min_x as f32 * cell_size, fy), rect);
            let end = self.world_to_screen(Vec2::new(max_x as f32 * cell_size, fy), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
    }

    fn draw_blockmap(&self, painter: &egui::Painter, rect: Rect, bsp: &BspLevel) {
        // We assume `bsp.blocks` is public, or a getter method exists: e.g. `bsp.blocks()`
        let block_guard = bsp.blocks.read();
        let block = &*block_guard; // the block data

        let stroke = Stroke::new(1.0, Color32::YELLOW);

        for cy in 0..block.height {
            for cx in 0..block.width {
                let wx = (block.x + cx * BLOCK_SIZE) as f32;
                let wy = (block.y + cy * BLOCK_SIZE) as f32;

                let min_screen = self.world_to_screen(Vec2::new(wx, wy), rect).to_pos2();
                let size = Vec2::splat(BLOCK_SIZE as f32 * self.zoom);
                let cell_rect = Rect::from_min_size(min_screen, size);

                painter.rect_stroke(cell_rect, 0.0, stroke);

                let idx = (cy * block.width + cx) as usize;
                if idx < block.cells.len() {
                    let linedefs_in_cell = block.cells[idx].len();
                    if linedefs_in_cell > 0 {
                        painter.text(
                            cell_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            linedefs_in_cell.to_string(),
                            egui::FontId::default(),
                            Color32::YELLOW,
                        );
                    }
                }
            }
        }
    }

    fn draw_bsp_tree(&self, painter: &egui::Painter, rect: Rect, bsp: &BspLevel) {
        let root_guard = bsp.root.read();
        if let Some(root_node) = &*root_guard {
            self.draw_bsp_node(painter, rect, root_node, 0);
        }
    }

    fn draw_bsp_node(&self, painter: &egui::Painter, rect: Rect, node: &BspNode, depth: usize) {
        let color = self.node_colors[depth % self.node_colors.len()];
        let stroke = Stroke::new(1.0, color);

        if let Some(part) = &node.partition {
            let start_scr = self.world_to_screen(Vec2::new(part.start.x as f32, part.start.y as f32), rect);
            let end_scr = self.world_to_screen(Vec2::new(part.end.x as f32, part.end.y as f32), rect);
            painter.line_segment([start_scr.to_pos2(), end_scr.to_pos2()], stroke);
        }

        // Recurse
        if let Some(ref front_node) = node.front {
            self.draw_bsp_node(painter, rect, front_node, depth + 1);
        }
        if let Some(ref back_node) = node.back {
            self.draw_bsp_node(painter, rect, back_node, depth + 1);
        }
    }

    fn draw_subsectors(&self, painter: &egui::Painter, rect: Rect, bsp: &BspLevel) {
        let subs_guard = bsp.subsectors.read();
        let subs = &*subs_guard;

        let segs_guard = bsp.segs.read();
        let all_segs = &*segs_guard;

        for (idx, subsector) in subs.iter().enumerate() {
            let fill_color = if Some(idx) == self.highlight_node {
                Color32::RED
            } else {
                Color32::from_rgba_premultiplied(0, 200, 0, 40)
            };
            // e.g. subsector.bbox
            let bbox_min = self.world_to_screen(
                Vec2::new(subsector.bbox.min_x as f32, subsector.bbox.min_y as f32),
                rect,
            ).to_pos2();
            let bbox_max = self.world_to_screen(
                Vec2::new(subsector.bbox.max_x as f32, subsector.bbox.max_y as f32),
                rect,
            ).to_pos2();
            let bbox_rect = Rect::from_min_max(bbox_min, bbox_max);
            painter.rect_filled(bbox_rect, 0.0, fill_color);

            // Draw segs
            for seg_arc in &subsector.segs {
                let seg_obj = seg_arc.as_ref();
                let stroke = Stroke::new(1.0, Color32::WHITE);
                let start_scr = self.world_to_screen(
                    Vec2::new(seg_obj.start.x as f32, seg_obj.start.y as f32),
                    rect
                );
                let end_scr = self.world_to_screen(
                    Vec2::new(seg_obj.end.x as f32, seg_obj.end.y as f32),
                    rect
                );
                painter.line_segment([start_scr.to_pos2(), end_scr.to_pos2()], stroke);
            }
        }
    }

    fn draw_stats(&self, ui: &mut egui::Ui, bsp: &BspLevel) {
        egui::Window::new("BSP Stats")
            .resizable(false)
            .show(ui.ctx(), |ui| {
                // Show some numbers
                let seg_count = bsp.segs.read().len();
                ui.label(format!("Total Segs: {}", seg_count));

                let subsector_count = bsp.subsectors.read().len();
                ui.label(format!("Subsectors: {}", subsector_count));

                if let Some(node) = &*bsp.root.read() {
                    let height = self.calc_tree_height(node);
                    ui.label(format!("BSP Tree Height: {}", height));
                }

                if let Some(selected) = self.selected_seg {
                    let all_segs = bsp.segs.read();
                    if let Some(seg) = all_segs.get(selected) {
                        ui.separator();
                        ui.label("Selected Seg:");
                        ui.label(format!("start=({}, {}) end=({}, {})",
                                         seg.start.x, seg.start.y, seg.end.x, seg.end.y));
                        ui.label(format!("length={:.1}, angle={:.1}°", seg.length, seg.angle.to_degrees()));
                    }
                }
            });
    }

    fn calc_tree_height(&self, node: &BspNode) -> usize {
        match (&node.front, &node.back) {
            (Some(f), Some(b)) => 1 + self.calc_tree_height(f).max(self.calc_tree_height(b)),
            (Some(f), None) => 1 + self.calc_tree_height(f),
            (None, Some(b)) => 1 + self.calc_tree_height(b),
            (None, None) => 0,
        }
    }

    // -----------------------------------------------------------
    // Selection
    // -----------------------------------------------------------
    fn select_closest_seg(&mut self, world: Vec2, bsp: &BspLevel) {
        let segs_guard = bsp.segs.read();
        let seg_vec = &*segs_guard;

        let mut best_dist = f64::MAX;
        let mut best_idx = None;

        for (i, seg_arc) in seg_vec.iter().enumerate() {
            let seg_obj = seg_arc.as_ref();
            let d = self.dist_to_seg(world, seg_obj);
            if d < best_dist {
                best_dist = d;
                best_idx = Some(i);
            }
        }

        let threshold = 10.0 / self.zoom as f64;
        if best_dist < threshold {
            self.selected_seg = best_idx;
            self.highlight_node = None;
        } else {
            self.selected_seg = None;
            self.highlight_node = None;
        }
    }

    fn dist_to_seg(&self, screen_pos: Vec2, seg: &Seg) -> f64 {
        let line = Line2D::new(
            GeoPoint2D { x: seg.start.x, y: seg.start.y },
            GeoPoint2D { x: seg.end.x, y: seg.end.y }
        );
        let test_pt = GeoPoint2D { x: screen_pos.x as f64, y: screen_pos.y as f64 };
        line.distance_to_point(&test_pt)
    }
}

// -------------------------------------------------------------------
// If you want to debug procedural generation in the same file
// (violent approach: you must make `generator: &mut ProceduralGenerator` if you want to mutate config):
// -------------------------------------------------------------------
impl BspDebugger {
    pub fn debug_procedural_generation(
        &mut self,
        ui: &mut egui::Ui,
        generator: &mut ProceduralGenerator,
    ) {
        egui::Window::new("Procedural Generation Debug")
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.heading("Generator Settings");

                // If `generator.config` is pub:
                ui.add(egui::Slider::new(&mut generator.config.min_room_size, 32..=128)
                    .text("Min Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.max_room_size, 64..=256)
                    .text("Max Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.room_density, 0.0..=1.0)
                    .text("Room Density"));
                ui.add(egui::Slider::new(&mut generator.config.branching_factor, 0.0..=1.0)
                    .text("Branching Factor"));

                if ui.button("Generate Map").clicked() {
                    // Possibly call generator.generate(...).
                }

                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.show_blockmap, "Show Blockmap");
                ui.checkbox(&mut self.show_bsp_tree, "Show BSP Tree");
                ui.checkbox(&mut self.show_subsectors, "Show Subsectors");

                if let Some(st) = &generator.stats {
                    ui.separator();
                    ui.heading("Perf Stats");
                    ui.label(format!("Gen time: {:.2} ms", st.generation_time));
                    ui.label(format!("Rooms: {}", st.room_count));
                    ui.label(format!("Corridors: {}", st.corridor_count));
                    ui.label(format!("Vertices: {}", st.vertex_count));
                }
            });
    }

    pub fn draw_gen_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        generator: &ProceduralGenerator,
    ) {
        // Draw a grid for the generator’s coordinate space
        if self.show_grid {
            self.draw_gen_grid(painter, rect, generator.config.min_room_size);
        }

        // Rooms
        for room in &generator.rooms {
            let min = Vec2::new(room.min_x as f32, room.min_y as f32);
            let max = Vec2::new(room.max_x as f32, room.max_y as f32);

            let screen_min = self.world_to_screen(min, rect).to_pos2();
            let screen_max = self.world_to_screen(max, rect).to_pos2();

            let rr = Rect::from_min_max(screen_min, screen_max);
            painter.rect_filled(rr, 0.0, Color32::from_rgba_premultiplied(0, 0, 200, 40));
            painter.rect_stroke(rr, 0.0, Stroke::new(1.0, Color32::WHITE));
        }

        // Corridors
        for &(start, end) in &generator.corridors {
            let s = self.world_to_screen(Vec2::new(start.x as f32, start.y as f32), rect).to_pos2();
            let e = self.world_to_screen(Vec2::new(end.x as f32, end.y as f32), rect).to_pos2();
            painter.line_segment(
                [s, e],
                Stroke::new(
                    generator.config.min_corridor_width as f32 * self.zoom,
                    Color32::from_rgba_premultiplied(0, 200, 0, 180),
                ),
            );
        }
    }

    fn draw_gen_grid(&self, painter: &egui::Painter, rect: Rect, spacing: i32) {
        let top_left = self.screen_to_world(Vec2::ZERO, rect);
        let bot_right = self.screen_to_world(rect.size(), rect);

        let min_x = (top_left.x / spacing as f32).floor() as i32;
        let max_x = (bot_right.x / spacing as f32).ceil() as i32;
        let min_y = (top_left.y / spacing as f32).floor() as i32;
        let max_y = (bot_right.y / spacing as f32).ceil() as i32;

        let stroke = Stroke::new(1.0, Color32::WHITE);

        for x in min_x..=max_x {
            let wx = x as f32 * spacing as f32;
            let s = self.world_to_screen(Vec2::new(wx, min_y as f32 * spacing as f32), rect).to_pos2();
            let e = self.world_to_screen(Vec2::new(wx, max_y as f32 * spacing as f32), rect).to_pos2();
            painter.line_segment([s, e], stroke);
        }
        for y in min_y..=max_y {
            let wy = y as f32 * spacing as f32;
            let s = self.world_to_screen(Vec2::new(min_x as f32 * spacing as f32, wy), rect).to_pos2();
            let e = self.world_to_screen(Vec2::new(max_x as f32 * spacing as f32, wy), rect).to_pos2();
            painter.line_segment([s, e], stroke);
        }
    }
}

// For quick tests:
#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn test_screen_world() {
        let dbg = BspDebugger::default();
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let screen = Vec2::new(300.0, 200.0);
        let w = dbg.screen_to_world(screen, r);
        let round = dbg.world_to_screen(w, r);
        assert!((screen - round).length() < 0.001);
    }
}
