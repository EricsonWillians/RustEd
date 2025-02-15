//! src/bsp/debug_viz.rs

use std::sync::Arc;

use eframe::egui;
use egui::{Color32, Rect, Stroke, Vec2};

use crate::bsp::{
    BspLevel,
    BspNode,
    Seg,
    BLOCK_SIZE,
};
use crate::utils::geometry::{Line2D, Point2D as GeoPoint2D}; // unify geometry usage

// If you want to debug procedural gen:
use crate::bsp::bsp_procedural::ProceduralGenerator;

/// Manages drawing + interacting with the BSP data structure for debug purposes.
pub struct BspDebugger {
    /// Current zoom factor (1.0 = 1:1)
    zoom: f32,
    /// Panning offset in screen coordinates
    pan: Vec2,

    // Toggles for various debug overlays
    show_grid: bool,
    show_blockmap: bool,
    show_bsp_tree: bool,
    show_subsectors: bool,
    display_stats: bool,

    /// Which node (by depth) is currently highlighted (if any)
    highlight_node: Option<usize>,
    /// Which seg is currently selected (if any)
    selected_seg: Option<usize>,

    /// Predefined colors for recursively drawing the BSP tree
    node_colors: Vec<Color32>,
}

impl Default for BspDebugger {
    fn default() -> Self {
        Self {
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
                Color32::from_rgb(46, 204, 113),   // Green
                Color32::from_rgb(52, 152, 219),   // Blue
                Color32::from_rgb(155, 89, 182),   // Purple
                Color32::from_rgb(231, 76, 60),    // Red
                Color32::from_rgb(241, 196, 15),   // Yellow
            ],
        }
    }
}

impl BspDebugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw the main BSP debug UI
    pub fn show(&mut self, ui: &mut egui::Ui, bsp_level: &Arc<BspLevel>) {
        // 1) Controls (checkboxes, buttons, sliders)
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

            ui.add(egui::Slider::new(&mut self.zoom, 0.1..=10.0).text("Zoom"));
        });

        // 2) Main canvas region
        let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());

        // 3) Handle mouse dragging => pan
        if response.dragged() {
            self.pan += response.drag_delta();
        }

        // 4) Handle scroll-wheel zoom
        if let Some(hover_pos) = response.hover_pos() {
            let scroll = ui.input().scroll_delta.y / 1000.0;
            if scroll != 0.0 {
                let mouse_offset = hover_pos - response.rect.min; // relative to top-left of painter
                let old_world = self.screen_to_world(mouse_offset, response.rect);
                self.zoom *= 1.0 + scroll;
                let new_world = self.screen_to_world(mouse_offset, response.rect);
                // Pan so that world position under mouse stays the same
                self.pan += (new_world - old_world) * self.zoom;
            }
        }

        // 5) Draw overlays
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

        // 6) Display stats
        if self.display_stats {
            self.draw_stats(ui, bsp_level);
        }

        // 7) Interaction: e.g. select closest seg on click
        if let Some(hover_pos) = response.hover_pos() {
            let screen_offset = hover_pos - response.rect.min;
            let world_pos = self.screen_to_world(screen_offset, response.rect);

            if response.clicked() {
                self.handle_selection(world_pos, bsp_level);
            }

            // Show coordinates as a tooltip or debug text
            ui.ctx().debug_painter().text(
                hover_pos,
                egui::Align2::LEFT_BOTTOM,
                format!("X: {:.1}, Y: {:.1}", world_pos.x, world_pos.y),
                egui::FontId::default(),
                Color32::WHITE
            );
        }
    }

    // -----------------------
    // Conversion helpers
    // -----------------------
    fn screen_to_world(&self, screen_pos: Vec2, rect: Rect) -> Vec2 {
        // translate by pan and zoom, relative to canvas center
        let center = rect.center();
        (screen_pos - center.to_vec2() - self.pan) / self.zoom
    }

    fn world_to_screen(&self, world_pos: Vec2, rect: Rect) -> Vec2 {
        let center = rect.center();
        world_pos * self.zoom + self.pan + center.to_vec2()
    }

    // -----------------------
    // Debug Overlays
    // -----------------------
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        // e.g. 64x64 grid
        const GRID: f32 = 64.0;

        let top_left_world = self.screen_to_world(Vec2::ZERO, rect);
        let bottom_right_world = self.screen_to_world(rect.size(), rect);

        let min_x = (top_left_world.x / GRID).floor() as i32;
        let max_x = (bottom_right_world.x / GRID).ceil() as i32;
        let min_y = (top_left_world.y / GRID).floor() as i32;
        let max_y = (bottom_right_world.y / GRID).ceil() as i32;

        let stroke = Stroke::new(1.0, Color32::from_rgba_premultiplied(200, 200, 200, 60));

        // Vertical lines
        for x in min_x..=max_x {
            let sx = x as f32 * GRID;
            let start = self.world_to_screen(Vec2::new(sx, min_y as f32 * GRID), rect);
            let end = self.world_to_screen(Vec2::new(sx, max_y as f32 * GRID), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
        // Horizontal lines
        for y in min_y..=max_y {
            let sy = y as f32 * GRID;
            let start = self.world_to_screen(Vec2::new(min_x as f32 * GRID, sy), rect);
            let end = self.world_to_screen(Vec2::new(max_x as f32 * GRID, sy), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
    }

    fn draw_blockmap(&self, painter: &egui::Painter, rect: Rect, bsp: &Arc<BspLevel>) {
        // Assume BspLevel has a public getter: `fn blocks(&self) -> &Arc<RwLock<Block>>`
        if let Ok(block) = bsp.blocks().try_read() {
            let stroke = Stroke::new(1.0, Color32::YELLOW);

            for cy in 0..block.height {
                for cx in 0..block.width {
                    // Convert block coords to world coords
                    let wx = (block.x + cx * BLOCK_SIZE) as f32;
                    let wy = (block.y + cy * BLOCK_SIZE) as f32;

                    let min_screen = self.world_to_screen(Vec2::new(wx, wy), rect).to_pos2();
                    let size = Vec2::splat(BLOCK_SIZE as f32 * self.zoom);
                    let block_rect = Rect::from_min_size(min_screen, size);

                    painter.rect_stroke(block_rect, 0.0, stroke);

                    // Display how many linedefs are in this cell, if any
                    let index = (cy * block.width + cx) as usize;
                    if let Some(lst) = block.cells.get(index) {
                        if !lst.is_empty() {
                            painter.text(
                                block_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{}", lst.len()),
                                egui::FontId::default(),
                                Color32::YELLOW
                            );
                        }
                    }
                }
            }
        }
    }

    fn draw_bsp_tree(&self, painter: &egui::Painter, rect: Rect, bsp: &Arc<BspLevel>) {
        // Assume BspLevel has `fn root(&self) -> &Arc<RwLock<Option<Arc<BspNode>>>>`
        if let Some(root_node) = &*bsp.root().read() {
            self.draw_bsp_node(painter, rect, root_node, 0);
        }
    }

    fn draw_bsp_node(&self, painter: &egui::Painter, rect: Rect, node: &BspNode, depth: usize) {
        // partition line
        let color = self.node_colors[depth % self.node_colors.len()];
        let stroke = Stroke::new(1.0, color);

        if let Some(ref partition) = node.partition {
            // draw the partition line
            let start = self.world_to_screen(Vec2::new(partition.start.x as f32, partition.start.y as f32), rect);
            let end = self.world_to_screen(Vec2::new(partition.end.x as f32, partition.end.y as f32), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }

        // If you want a bounding box, store one in BspNode, e.g. `node.bbox: BoundingBox`.
        // Then you can highlight it if `Some(depth) == self.highlight_node`.

        // recursively draw children
        if let Some(ref front_node) = node.front {
            self.draw_bsp_node(painter, rect, front_node, depth + 1);
        }
        if let Some(ref back_node) = node.back {
            self.draw_bsp_node(painter, rect, back_node, depth + 1);
        }
    }

    fn draw_subsectors(&self, painter: &egui::Painter, rect: Rect, bsp: &Arc<BspLevel>) {
        // Assume BspLevel has `fn subsectors(&self) -> &Arc<RwLock<Vec<Arc<Subsector>>>>`
        // and also has `fn segs(&self) -> &Arc<RwLock<Vec<Arc<Seg>>>>`
        if let Ok(subsectors) = bsp.subsectors().try_read() {
            if let Ok(all_segs) = bsp.segs().try_read() {
                // e.g. each subsector has a bounding box, a list of seg indices, etc.
                for (i, subsector) in subsectors.iter().enumerate() {
                    // highlight bounding box
                    let color = if Some(i) == self.highlight_node {
                        Color32::RED
                    } else {
                        Color32::from_rgba_premultiplied(0, 255, 0, 40)
                    };
                    let rect_min = self.world_to_screen(
                        Vec2::new(subsector.bbox.min_x as f32, subsector.bbox.min_y as f32),
                        rect
                    ).to_pos2();
                    let rect_max = self.world_to_screen(
                        Vec2::new(subsector.bbox.max_x as f32, subsector.bbox.max_y as f32),
                        rect
                    ).to_pos2();
                    let fill_rect = Rect::from_min_max(rect_min, rect_max);
                    painter.rect_filled(fill_rect, 0.0, color);

                    // draw the segs
                    for seg_arc in &subsector.segs {
                        let seg_obj = seg_arc.as_ref();
                        let seg_color = Color32::WHITE;
                        let seg_stroke = Stroke::new(1.0, seg_color);

                        let start = self.world_to_screen(
                            Vec2::new(seg_obj.start.x as f32, seg_obj.start.y as f32),
                            rect
                        );
                        let end = self.world_to_screen(
                            Vec2::new(seg_obj.end.x as f32, seg_obj.end.y as f32),
                            rect
                        );
                        painter.line_segment([start.to_pos2(), end.to_pos2()], seg_stroke);
                    }
                }
            }
        }
    }

    fn draw_stats(&self, ui: &mut egui::Ui, bsp: &Arc<BspLevel>) {
        egui::Window::new("BSP Stats")
            .resizable(false)
            .show(ui.ctx(), |ui| {
                // Show your stats: e.g. number of subsectors, segs, tree height, etc.
                if let Ok(segs) = bsp.segs().try_read() {
                    ui.label(format!("Segs: {}", segs.len()));
                }
                if let Ok(subsectors) = bsp.subsectors().try_read() {
                    ui.label(format!("Subsectors: {}", subsectors.len()));
                }
                if let Some(root_node) = &*bsp.root().read() {
                    let height = self.calculate_tree_height(root_node);
                    ui.label(format!("BSP Tree Height: {}", height));
                }
                if let Some(seg_idx) = self.selected_seg {
                    // Show detail about the selected seg
                    if let Ok(all_segs) = bsp.segs().try_read() {
                        if let Some(seg) = all_segs.get(seg_idx) {
                            ui.separator();
                            ui.heading("Selected Seg");
                            ui.label(format!(
                                "({}, {}) -> ({}, {})",
                                seg.start.x, seg.start.y,
                                seg.end.x, seg.end.y
                            ));
                            ui.label(format!("Length: {:.1}, Angle: {:.1}°", seg.length, seg.angle.to_degrees()));
                        }
                    }
                }
            });
    }

    // Recursively compute tree height
    fn calculate_tree_height(&self, node: &BspNode) -> usize {
        match (&node.front, &node.back) {
            (Some(front), Some(back)) => {
                1 + self.calculate_tree_height(front).max(self.calculate_tree_height(back))
            }
            (Some(front), None) => 1 + self.calculate_tree_height(front),
            (None, Some(back)) => 1 + self.calculate_tree_height(back),
            (None, None) => 0,
        }
    }

    // -------------
    // Selection
    // -------------
    fn handle_selection(&mut self, world_pos: Vec2, bsp: &Arc<BspLevel>) {
        // find the closest seg
        if let Ok(all_segs) = bsp.segs().try_read() {
            let mut best = None;
            let mut best_dist = f64::MAX;

            for (idx, seg_arc) in all_segs.iter().enumerate() {
                let seg_obj = seg_arc.as_ref();
                let d = self.distance_world_to_seg(world_pos, seg_obj);
                if d < best_dist {
                    best_dist = d;
                    best = Some(idx);
                }
            }
            // If close enough, select it
            let threshold = 10.0 / self.zoom as f64; 
            if best_dist < threshold {
                self.selected_seg = best;
                self.highlight_node = None; 
                // Optionally find which node contains it
            } else {
                self.selected_seg = None;
                self.highlight_node = None;
            }
        }
    }

    fn distance_world_to_seg(&self, wpos: Vec2, seg: &Seg) -> f64 {
        // Convert world Vec2 -> geometry's type
        let test_pt = GeoPoint2D { x: wpos.x as f64, y: wpos.y as f64 };
        let seg_line = Line2D::new(
            GeoPoint2D { x: seg.start.x, y: seg.start.y },
            GeoPoint2D { x: seg.end.x,   y: seg.end.y   },
        );
        seg_line.distance_to_point(&test_pt)
    }
}

// If you want to debug procedural generation in the same file:
impl BspDebugger {
    pub fn debug_procedural_generation(
        &mut self,
        ui: &mut egui::Ui,
        generator: &ProceduralGenerator,
    ) {
        egui::Window::new("Procedural Generation Debug")
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.heading("Generator Settings");

                // If you have public config fields or getters, you can do:
                // let config = generator.config(); ...
                // For brevity, we assume they're public:
                ui.add(egui::Slider::new(&mut generator.config.min_room_size, 32..=128)
                    .text("Min Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.max_room_size, 64..=256)
                    .text("Max Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.room_density, 0.0..=1.0)
                    .text("Room Density"));
                ui.add(egui::Slider::new(&mut generator.config.branching_factor, 0.0..=1.0)
                    .text("Branching Factor"));

                ui.separator();
                if ui.button("Generate New Map").clicked() {
                    // You could trigger a re-generation here
                }

                ui.separator();
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.show_blockmap, "Show Blockmap");
                ui.checkbox(&mut self.show_bsp_tree, "Show BSP Tree");
                ui.checkbox(&mut self.show_subsectors, "Show Subsectors");

                if let Some(stats) = &generator.stats {
                    ui.separator();
                    ui.heading("Performance Stats");
                    ui.label(format!("Generation Time: {:.1} ms", stats.generation_time));
                    ui.label(format!("Room Count: {}", stats.room_count));
                    ui.label(format!("Corridor Count: {}", stats.corridor_count));
                    ui.label(format!("Total Vertices: {}", stats.vertex_count));
                }
            });
    }

    /// Example: Draw a preview of the generated map (rooms, corridors, etc.).
    pub fn draw_generation_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        generator: &ProceduralGenerator,
    ) {
        // Possibly draw a grid for rooms
        if self.show_grid {
            self.draw_gen_grid(painter, rect, generator.config.min_room_size);
        }

        // Draw rooms
        for room in &generator.rooms {
            let min = Vec2::new(room.min_x as f32, room.min_y as f32);
            let max = Vec2::new(room.max_x as f32, room.max_y as f32);

            let screen_min = self.world_to_screen(min, rect).to_pos2();
            let screen_max = self.world_to_screen(max, rect).to_pos2();

            let room_rect = Rect::from_min_max(screen_min, screen_max);
            painter.rect_filled(room_rect, 0.0, Color32::from_rgba_premultiplied(0, 100, 255, 40));
            painter.rect_stroke(room_rect, 0.0, Stroke::new(1.0, Color32::WHITE));
        }

        // Draw corridors
        for &(start_pt, end_pt) in &generator.corridors {
            let start = self.world_to_screen(Vec2::new(start_pt.x as f32, start_pt.y as f32), rect);
            let end = self.world_to_screen(Vec2::new(end_pt.x as f32, end_pt.y as f32), rect);

            painter.line_segment(
                [start.to_pos2(), end.to_pos2()],
                Stroke::new(
                    generator.config.min_corridor_width as f32 * self.zoom,
                    Color32::from_rgba_premultiplied(0, 200, 0, 160),
                ),
            );
        }
    }

    fn draw_gen_grid(&self, painter: &egui::Painter, rect: Rect, size: i32) {
        // Example: minimal grid for the generator's coordinate space
        // Adjust or remove if you want a different approach.
        let top_left = self.screen_to_world(Vec2::ZERO, rect);
        let bottom_right = self.screen_to_world(rect.size(), rect);

        let min_x = (top_left.x / size as f32).floor() as i32;
        let max_x = (bottom_right.x / size as f32).ceil() as i32;
        let min_y = (top_left.y / size as f32).floor() as i32;
        let max_y = (bottom_right.y / size as f32).ceil() as i32;

        let stroke = Stroke::new(1.0, Color32::WHITE);

        for x in min_x..=max_x {
            let wx = x as f32 * size as f32;
            let start = self.world_to_screen(Vec2::new(wx, min_y as f32 * size as f32), rect);
            let end = self.world_to_screen(Vec2::new(wx, max_y as f32 * size as f32), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
        for y in min_y..=max_y {
            let wy = y as f32 * size as f32;
            let start = self.world_to_screen(Vec2::new(min_x as f32 * size as f32, wy), rect);
            let end = self.world_to_screen(Vec2::new(max_x as f32 * size as f32, wy), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }
    }
}

// ---------------------------
// Optional test
// ---------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn test_screen_world_roundtrip() {
        let dbg = BspDebugger::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let screen = Vec2::new(400.0, 300.0);
        let world = dbg.screen_to_world(screen, rect);
        let round = dbg.world_to_screen(world, rect);

        // Because of potential float rounding, allow some small epsilon
        assert!((screen - round).length() < 0.0001);
    }

    #[test]
    fn test_distance_to_seg() {
        let dbg = BspDebugger::default();
        let seg = Seg {
            start: Point2D::new(0.0, 0.0),
            end: Point2D::new(10.0, 0.0),
            angle: 0.0,
            length: 10.0,
            linedef: None,
            side: SegmentSide::Front,
            partner: None,
        };
        let dist = dbg.distance_world_to_seg(Vec2::new(5.0, 5.0), &seg);
        // Dist to a horizontal line from (0,0) to (10,0) from point (5,5) is 5
        assert!((dist - 5.0).abs() < 0.0001);
    }
}
