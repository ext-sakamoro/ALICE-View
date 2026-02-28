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
    #[must_use]
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
            [self.initial_pan[0] + delta_x, self.initial_pan[1] + delta_y]
        })
    }

    /// End drag
    pub fn end_drag(&mut self) {
        self.drag_start = None;
    }

    /// Check if dragging
    #[must_use]
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_initial_state() {
        let vs = ViewportState::new();
        assert!(!vs.is_dragging());
    }

    #[test]
    fn viewport_start_drag() {
        let mut vs = ViewportState::new();
        vs.start_drag([100.0, 200.0], [0.0, 0.0]);
        assert!(vs.is_dragging());
    }

    #[test]
    fn viewport_end_drag() {
        let mut vs = ViewportState::new();
        vs.start_drag([100.0, 200.0], [0.0, 0.0]);
        assert!(vs.is_dragging());
        vs.end_drag();
        assert!(!vs.is_dragging());
    }

    #[test]
    fn viewport_update_drag_returns_pan() {
        let mut vs = ViewportState::new();
        vs.start_drag([0.0, 0.0], [0.0, 0.0]);
        let pan = vs.update_drag([10.0, 5.0], 1.0);
        assert!(pan.is_some());
        let p = pan.unwrap();
        assert!((p[0] - 10.0).abs() < 1e-5);
        assert!((p[1] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn viewport_update_drag_zoom_scaling() {
        let mut vs = ViewportState::new();
        vs.start_drag([0.0, 0.0], [0.0, 0.0]);
        // At zoom=2, movement should be halved
        let pan = vs.update_drag([10.0, 0.0], 2.0);
        let p = pan.unwrap();
        assert!((p[0] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn viewport_no_drag_returns_none() {
        let mut vs = ViewportState::new();
        let pan = vs.update_drag([10.0, 5.0], 1.0);
        assert!(pan.is_none());
    }
}
