use eframe::egui;

use crate::editor::instance::Instance;

impl Instance {
    pub fn show_debug_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("BSP Debug View")
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                if let Some(bsp_level) = &self.bsp_level {
                    self.bsp_debugger.show(ui, bsp_level);
                } else {
                    ui.label("No BSP data available. Try loading or generating a map first.");
                    
                    if ui.button("Generate Test Map").clicked() {
                        // For testing - generate a simple map
                        self.generate_test_map();
                    }
                }
            });
    }

    fn generate_test_map(&mut self) {
        // Create a simple test map for debugging
        use crate::document::{Document, Vertex, LineDef};
        use std::sync::Arc;

        let mut doc = Document::new();
        
        // Add some test vertices
        let vertices = vec![
            Vertex { raw_x: 0, raw_y: 0 },
            Vertex { raw_x: 256, raw_y: 0 },
            Vertex { raw_x: 256, raw_y: 256 },
            Vertex { raw_x: 0, raw_y: 256 },
        ];

        for vertex in vertices {
            doc.vertices().write().push(Arc::new(vertex));
        }

        // Add test linedefs to form a square
        let linedefs = vec![
            LineDef {
                start: 0,
                end: 1,
                flags: 0,
                line_type: 0,
                tag: 0,
                right: 0,
                left: -1,
            },
            LineDef {
                start: 1,
                end: 2,
                flags: 0,
                line_type: 0,
                tag: 0,
                right: 0,
                left: -1,
            },
            LineDef {
                start: 2,
                end: 3,
                flags: 0,
                line_type: 0,
                tag: 0,
                right: 0,
                left: -1,
            },
            LineDef {
                start: 3,
                end: 0,
                flags: 0,
                line_type: 0,
                tag: 0,
                right: 0,
                left: -1,
            },
        ];

        for linedef in linedefs {
            doc.linedefs().write().push(Arc::new(linedef));
        }

        // Create BSP tree from test map
        let doc = Arc::new(doc);
        let bsp = Arc::new(crate::bsp::BspLevel::new(doc.clone()));
        bsp.build().expect("Failed to build BSP tree");
        self.bsp_level = Some(bsp);
    }
}