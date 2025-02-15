// src/bsp/debug_viz.rs

use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2}; // Complete the import
use std::sync::Arc;
use crate::bsp::bsp_procedural::ProceduralGenerator; //Add this line

use crate::bsp::{BspLevel, BspNode, BLOCK_SIZE, Seg}; // Import necessary items
//use crate::utils::geometry::Point2D; //REMOVED
use crate::utils::geometry::Line2D;  // Import Line2D


/// Debug visualization state for BSP construction
pub struct BspDebugger {
    zoom: f32,
    pan: Vec2,
    show_grid: bool,
    show_blockmap: bool,
    show_bsp_tree: bool,
    show_subsectors: bool,
    highlight_node: Option<usize>,
    display_stats: bool,
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
            highlight_node: None,
            display_stats: true,
            selected_seg: None,
            node_colors: vec![
                Color32::from_rgb(46, 204, 113),  // Green
                Color32::from_rgb(52, 152, 219),  // Blue
                Color32::from_rgb(155, 89, 182),  // Purple
                Color32::from_rgb(231, 76, 60),   // Red
                Color32::from_rgb(241, 196, 15),  // Yellow
            ],
        }
    }
}

impl BspDebugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draw debug visualization UI and handle input
    pub fn show(&mut self, ui: &mut egui::Ui, bsp_level: &Arc<BspLevel>) {
        // Debug controls panel
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

        // Main canvas for map visualization
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::drag()
        );

        // Handle pan/zoom input
        if response.dragged() {
            self.pan += response.drag_delta();
        }
        if let Some(hover_pos) = response.hover_pos() {
            let zoom_delta = ui.input(|i| i.scroll_delta.y / 1000.0); // Use ui.input correctly
            if zoom_delta != 0.0 {
                // Zoom centered on mouse position
                let mouse_pos = hover_pos - response.rect.min;
                let old_pos = self.screen_to_world(mouse_pos.to_vec2(), response.rect); //to_vec2()
                self.zoom *= 1.0 + zoom_delta;
                let new_pos = self.screen_to_world(mouse_pos.to_vec2(), response.rect); //to_vec2()
                self.pan += (new_pos - old_pos) * self.zoom;
            }
        }

        // Draw map elements
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

        // Draw stats overlay
        if self.display_stats {
            self.draw_stats(ui, bsp_level);
        }

        // Handle selection
        if let Some(hover_pos) = response.hover_pos() {
            let world_pos = self.screen_to_world(
                (hover_pos - response.rect.min).to_vec2(),
                response.rect
            );

            if response.clicked() {
                // Find closest seg/node to click position
                self.handle_selection(world_pos, bsp_level);
            }

            // Show tooltip with coordinates
            ui.ctx().debug_painter().text( // Use ui.ctx().debug_painter() for debug text
                hover_pos,
                egui::Align2::LEFT_BOTTOM,
                format!("X: {:.1}, Y: {:.1}", world_pos.x, world_pos.y),
                egui::FontId::default(),
                Color32::WHITE
            );
        }
    }

    /// Convert screen coordinates to world coordinates
    fn screen_to_world(&self, screen_pos: Vec2, rect: Rect) -> Vec2 {
        let center = rect.center();
        (screen_pos - center.to_vec2() - self.pan) / self.zoom
    }

    /// Convert world coordinates to screen coordinates
    fn world_to_screen(&self, world_pos: Vec2, rect: Rect) -> Vec2 {
        let center = rect.center();
        (world_pos * self.zoom + self.pan + center.to_vec2())
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        const GRID_SIZE: f32 = 64.0;
        let min_pos = self.screen_to_world(Vec2::ZERO, rect);
        let max_pos = self.screen_to_world(rect.size(), rect);

        let min_x = (min_pos.x / GRID_SIZE).floor() * GRID_SIZE;
        let min_y = (min_pos.y / GRID_SIZE).floor() * GRID_SIZE;
        let max_x = (max_pos.x / GRID_SIZE).ceil() * GRID_SIZE;
        let max_y = (max_pos.y / GRID_SIZE).ceil() * GRID_SIZE;

        let grid_color = Color32::from_rgba_premultiplied(100, 100, 100, 40);
        let grid_stroke = Stroke::new(1.0, grid_color);

        // Draw vertical lines
        for x in (min_x as i32..=max_x as i32).step_by(GRID_SIZE as usize) {
            let start = self.world_to_screen(Vec2::new(x as f32, min_y), rect);
            let end = self.world_to_screen(Vec2::new(x as f32, max_y), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], grid_stroke);
        }

        // Draw horizontal lines
        for y in (min_y as i32..=max_y as i32).step_by(GRID_SIZE as usize) {
            let start = self.world_to_screen(Vec2::new(min_x, y as f32), rect);
            let end = self.world_to_screen(Vec2::new(max_x, y as f32), rect);
            painter.line_segment([start.to_pos2(), end.to_pos2()], grid_stroke);
        }
    }

    fn draw_blockmap(&self, painter: &egui::Painter, rect: Rect, bsp_level: &Arc<BspLevel>) {
      if let Ok(blocks) = bsp_level.blocks.try_read(){
        let block_stroke = Stroke::new(1.0, Color32::YELLOW);

        for y in 0..blocks.height {
            for x in 0..blocks.width {
                let block_rect = Rect::from_min_size(
                    self.world_to_screen(
                        Vec2::new(
                            (blocks.x + x * BLOCK_SIZE) as f32,
                            (blocks.y + y * BLOCK_SIZE) as f32
                        ),
                        rect
                    ).to_pos2(),
                    Vec2::splat(BLOCK_SIZE as f32 * self.zoom)
                );

                painter.rect_stroke(block_rect, 0.0, block_stroke);

                // Show linedef count in block
                let cell_idx = (y * blocks.width + x) as usize;
                if cell_idx < blocks.cells.len() {
                    let count = blocks.cells[cell_idx].len();
                    if count > 0 {
                        painter.text(
                            block_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            count.to_string(),
                            egui::FontId::default(),
                            Color32::YELLOW
                        );
                    }
                }
            }
        }
      }
    }

    fn draw_bsp_tree(&self, painter: &egui::Painter, rect: Rect, bsp_level: &Arc<BspLevel>) {
        if let Some(root) = &*bsp_level.root.read() {
            self.draw_bsp_node(painter, rect, root, 0);
        }
    }

    fn draw_bsp_node(&self, painter: &egui::Painter, rect: Rect, node: &BspNode, depth: usize) {
        let color = self.node_colors[depth % self.node_colors.len()];
        let stroke = Stroke::new(1.0, color);

        // Draw partition line
        if let Some(ref partition) = node.partition {
            let start = self.world_to_screen(
                Vec2::new(partition.start.x as f32, partition.start.y as f32),
                rect
            );
            let end = self.world_to_screen(
                Vec2::new(partition.end.x as f32, partition.end.y as f32),
                rect
            );
            painter.line_segment([start.to_pos2(), end.to_pos2()], stroke);
        }

        // Draw node bbox if highlighted
        if Some(depth) == self.highlight_node {
            let bbox_stroke = Stroke::new(2.0, Color32::RED);
            let bbox_rect = Rect::from_min_max(
                self.world_to_screen(
                    Vec2::new(node.bbox.min_x as f32, node.bbox.min_y as f32),
                    rect
                ).to_pos2(),
                self.world_to_screen(
                    Vec2::new(node.bbox.max_x as f32, node.bbox.max_y as f32),
                    rect
                ).to_pos2()
            );
            painter.rect_stroke(bbox_rect, 0.0, bbox_stroke);
        }

        // Recursively draw children
      if let Some(ref front) = *node.front { // Access the Option within the Box
          self.draw_bsp_node(painter, rect, front, depth + 1);
      }
      if let Some(ref back) = *node.back {  // Access the Option within the Box
          self.draw_bsp_node(painter, rect, back, depth + 1);
        }
    }

    fn draw_subsectors(&self, painter: &egui::Painter, rect: Rect, bsp_level: &Arc<BspLevel>) {
      if let Ok(subsectors) = bsp_level.subsectors.try_read(){
        if let Ok(segs) = bsp_level.segs.try_read(){
          for (i, subsector) in subsectors.iter().enumerate() {
              // Draw subsector bbox
              let bbox_color = if Some(i) == self.highlight_node {
                  Color32::RED
              } else {
                  Color32::from_rgba_premultiplied(0, 255, 0, 40)
              };

              let bbox_rect = Rect::from_min_max(
                  self.world_to_screen(
                      Vec2::new(subsector.bbox.min_x as f32, subsector.bbox.min_y as f32),
                      rect
                  ).to_pos2(),
                  self.world_to_screen(
                      Vec2::new(subsector.bbox.max_x as f32, subsector.bbox.max_y as f32),
                      rect
                  ).to_pos2()
              );
              painter.rect_filled(bbox_rect, 0.0, bbox_color);

              // Draw segs
              for seg in &subsector.segs {
                let line = Line2D::new(crate::utils::geometry::Point2D{x:seg.start.x, y: seg.start.y}, crate::utils::geometry::Point2D{x: seg.end.x, y: seg.end.y});
                  let seg_color = if Some(seg.index()) == self.selected_seg {
                      Color32::RED
                  } else {
                      Color32::WHITE
                  };
                  let seg_stroke = Stroke::new(1.0, seg_color);

                  let start = self.world_to_screen(
                      Vec2::new(seg.start.x as f32, seg.start.y as f32),
                      rect
                  );
                  let end = self.world_to_screen(
                      Vec2::new(seg.end.x as f32, seg.end.y as f32),
                      rect
                  );
                  painter.line_segment([start.to_pos2(), end.to_pos2()], seg_stroke);
              }
            }
          }
        }
    }

    fn draw_stats(&self, ui: &mut egui::Ui, bsp_level: &Arc<BspLevel>) {
        // Create a semi-transparent overlay window
        egui::Window::new("Statistics")
            .fixed_pos(ui.max_rect().left_top())
            .resizable(false)
            .show(ui.ctx(), |ui| {
                //let nodes = bsp_level.nodes.read();
              if let Ok(subsectors) = bsp_level.subsectors.try_read(){
                if let Ok(segs) = bsp_level.segs.try_read(){
                  //ui.label(format!("Nodes: {}", nodes.len()));
                  ui.label(format!("Subsectors: {}", subsectors.len()));
                  ui.label(format!("Segs: {}", segs.len()));
                  
                  if let Some(root) = &*bsp_level.root.read() {
                      let height = self.calculate_tree_height(root);
                      ui.label(format!("Tree Height: {}", height));
                  }

                  if let Some(seg_idx) = self.selected_seg {
                      if let Some(seg) = segs.get(seg_idx) {
                          ui.separator();
                          ui.label("Selected Seg:");
                          ui.label(format!("Start: ({:.1}, {:.1})",
                              seg.start.x, seg.start.y));
                          ui.label(format!("End: ({:.1}, {:.1})",
                              seg.end.x, seg.end.y));
                          ui.label(format!("Length: {:.1}", seg.length));
                          ui.label(format!("Angle: {:.1}°", seg.angle.to_degrees()));
                      }
                  }
                }
              }
            });
    }

    fn calculate_tree_height(&self, node: &BspNode) -> usize {
      match (&node.front, &node.back) {
        (Some(front), Some(back)) => {
            1 + self.calculate_tree_height(front).max(self.calculate_tree_height(back))
        },
        (Some(front), None) => 1 + self.calculate_tree_height(front),
        (None, Some(back)) => 1 + self.calculate_tree_height(back),
        (None, None) => 0, // Leaf node has height 0
      }
    }

    fn handle_selection(&mut self, world_pos: Vec2, bsp_level: &Arc<BspLevel>) {
      let pos = crate::bsp::Point2D::new(world_pos.x as f64, world_pos.y as f64);
        
        // Find closest seg
      if let Ok(segs) = bsp_level.segs.try_read(){
        let mut closest_seg = None;
        let mut min_dist = f64::MAX;
        
        for (idx, seg) in segs.iter().enumerate() {
            let dist = self.point_to_seg_distance(&pos, seg);
            if dist < min_dist {
                min_dist = dist;
                closest_seg = Some(idx);
            }
        }

        // Only select if within reasonable distance
        if min_dist < 10.0 / self.zoom as f64 {
            self.selected_seg = closest_seg;
            
            // If seg is selected, highlight its containing node/subsector
            if let Some(seg_idx) = closest_seg {
                self.highlight_containing_node(seg_idx, bsp_level);
            }
        } else {
            self.selected_seg = None;
            self.highlight_node = None;
        }
      }
    }

    fn point_to_seg_distance(&self, point: &crate::bsp::Point2D, seg: &Seg) -> f64 {
        let line = Line2D::new(crate::utils::geometry::Point2D{x: seg.start.x, y: seg.start.y}, crate::utils::geometry::Point2D{x:seg.end.x, y: seg.end.y});
        line.distance_to_point(point)
    }

    fn highlight_containing_node(&mut self, seg_idx: usize, bsp_level: &Arc<BspLevel>) {
        if let Some(root) = &*bsp_level.root.read() {
            self.highlight_node = self.find_node_containing_seg(root, seg_idx, 0);
        }
    }

    fn find_node_containing_seg(&self, node: &BspNode, seg_idx: usize, depth: usize) -> Option<usize> {
      //The method index() doesn't exist, remove this
      /*
        // Check if this node contains the seg
        if node.segs.iter().any(|s| s.index() == seg_idx) {
            return Some(depth);
        }
        */

        // Check children
      if let Some(ref front) = *node.front {
          if let Some(d) = self.find_node_containing_seg(front, seg_idx, depth + 1) {
              return Some(d);
          }
      }
      if let Some(ref back) = *node.back {
          if let Some(d) = self.find_node_containing_seg(back, seg_idx, depth + 1) {
              return Some(d);
          }
      }

        None
    }
}

// Add procedural debug visualization
impl BspDebugger {
    pub fn debug_procedural_generation(&mut self, ui: &mut egui::Ui, generator: &ProceduralGenerator) {
        egui::Window::new("Procedural Generation Debug")
            .show(ui.ctx(), |ui| {
                // Show generator configuration
                ui.heading("Generator Settings");
                ui.add(egui::Slider::new(&mut generator.config.min_room_size, 32..=128)
                    .text("Min Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.max_room_size, 64..=256)
                    .text("Max Room Size"));
                ui.add(egui::Slider::new(&mut generator.config.room_density, 0.0..=0.5)
                    .text("Room Density"));
                ui.add(egui::Slider::new(&mut generator.config.branching_factor, 0.0..=1.0)
                    .text("Branching Factor"));

                // Generation controls
                ui.separator();
                if ui.button("Generate New Map").clicked() {
                    // Trigger new map generation
                }

                // Debug visualization options
                ui.separator();
                ui.heading("Debug Display");
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.show_blockmap, "Show Blockmap");
                ui.checkbox(&mut self.show_bsp_tree, "Show BSP Tree");
                ui.checkbox(&mut self.show_subsectors, "Show Subsectors");

                // Performance metrics
                if let Some(stats) = &generator.stats {
                    ui.separator();
                    ui.heading("Performance");
                    ui.label(format!("Generation Time: {:.2}ms", stats.generation_time));
                    ui.label(format!("Room Count: {}", stats.room_count));
                    ui.label(format!("Corridor Count: {}", stats.corridor_count));
                    ui.label(format!("Total Vertices: {}", stats.vertex_count));
                }
            });
    }
  pub fn draw_generation_preview(&self, painter: &egui::Painter, rect: Rect, generator: &ProceduralGenerator) {
        // Draw room placement grid
        if self.show_grid {
            self.draw_generation_grid(painter, rect, generator.config.min_room_size);
        }

        // Draw rooms
        for room in &generator.rooms {
            let room_rect = Rect::from_min_max(
                self.world_to_screen(
                    Vec2::new(room.min_x as f32, room.min_y as f32),
                    rect
                ).to_pos2(),
                self.world_to_screen(
                    Vec2::new(room.max_x as f32, room.max_y as f32),
                    rect
                ).to_pos2()
            );

            // Draw room fill
            painter.rect_filled(
                room_rect,
                0.0,
                Color32::from_rgba_premultiplied(100, 100, 255, 40)
            );

            // Draw room outline
            painter.rect_stroke(
                room_rect,
                0.0,
                Stroke::new(1.0, Color32::WHITE)
            );
        }

        // Draw corridors
        for corridor in &generator.corridors {
            let start = self.world_to_screen(
                Vec2::new(corridor.0.x as f32, corridor.0.y as f32), // Access x and y of Point2D
                rect
            );
            let end = self.world_to_screen(
                Vec2::new(corridor.1.x as f32, corridor.1.y as f32), // Access x and y of Point2D
                rect
            );

            painter.line_segment(
                [start.to_pos2(), end.to_pos2()],
                Stroke::new(
                    generator.config.min_corridor_width as f32 * self.zoom,
                    Color32::from_rgba_premultiplied(100, 255, 100, 40)
                )
            );
        }
    }
}

// UI element helpers
impl BspDebugger {
    fn draw_tooltip(&self, ui: &mut egui::Ui, text: &str) {
        egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("hover_tooltip"), |ui|{
            ui.label(text)
        });
    }

    fn draw_status_text(&self, painter: &egui::Painter, pos: Pos2, text: &str) {
        painter.text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            text,
            egui::FontId::proportional(14.0),
            Color32::WHITE
        );
    }
    // Placeholder method
    fn draw_generation_grid(&self, _painter: &egui::Painter, _rect: Rect, _grid_size: i32) {
        // TODO: Implement drawing the grid for room placement
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_conversion() {
        let debugger = BspDebugger::default();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        
        let screen_pos = Vec2::new(400.0, 300.0);
        let world_pos = debugger.screen_to_world(screen_pos, rect);
        let back_to_screen = debugger.world_to_screen(world_pos, rect);
        
        assert!((screen_pos - back_to_screen).length() < 0.001);
    }

    #[test]
    fn test_point_to_seg_distance() {
        let debugger = BspDebugger::default();
        
        let seg = Seg {
            start: crate::bsp::Point2D::new(0.0, 0.0),
            end: crate::bsp::Point2D::new(10.0, 0.0),
            angle: 0.0,
            length: 10.0,
            linedef: None,
            side: crate::bsp::SegmentSide::Front,
            partner: None,
        };
        
        let point = crate::bsp::Point2D::new(5.0, 5.0);
        let dist = debugger.point_to_seg_distance(&point, &seg);
        
        assert_eq!(dist, 5.0);
    }
}