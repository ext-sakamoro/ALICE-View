//! Main viewport rendering

// ViewportState and render_viewport_info are complete drag/overlay utilities
// to be integrated once the egui viewport panel is wired up.
#![allow(dead_code)]

use crate::app::ViewerState;

/// Viewport state
pub struct ViewportState {
    /// Drag start position
    drag_start: Option<[f32; 2]>,
    /// Initial pan when drag started
    initial_pan: [f32; 2],
}

impl ViewportState {
    pub fn new() -> Self {
        Self {
            drag_start: None,
            initial_pan: [0.0, 0.0],
        }
    }

    /// Handle drag start
    pub fn start_drag(&mut self, pos: [f32; 2], current_pan: [f32; 2]) {
        self.drag_start = Some(pos);
        self.initial_pan = current_pan;
    }

    /// Handle drag update
    pub fn update_drag(&mut self, pos: [f32; 2], zoom: f32) -> Option<[f32; 2]> {
        self.drag_start.map(|start| {
            let zoom_rcp = 1.0 / zoom;
            let delta_x = (pos[0] - start[0]) * zoom_rcp;
            let delta_y = (pos[1] - start[1]) * zoom_rcp;
            [
                self.initial_pan[0] + delta_x,
                self.initial_pan[1] + delta_y,
            ]
        })
    }

    /// End drag
    pub fn end_drag(&mut self) {
        self.drag_start = None;
    }

    /// Check if dragging
    pub fn is_dragging(&self) -> bool {
        self.drag_start.is_some()
    }
}

impl Default for ViewportState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render viewport info overlay
pub fn render_viewport_info(ctx: &egui::Context, state: &ViewerState) {
    egui::Area::new(egui::Id::new("viewport_info"))
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 25, 200))
                .show(ui, |ui| {
                    ui.label(format!("Pan: ({:.1}, {:.1})", state.pan[0], state.pan[1]));
                    ui.label(format!("Zoom: {:.2}x", state.zoom));
                });
        });
}
