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
    pub const fn new() -> Self {
        Self {
            drag_start: None,
            initial_pan: [0.0, 0.0],
        }
    }

    /// Handle drag start
    pub const fn start_drag(&mut self, pos: [f32; 2], current_pan: [f32; 2]) {
        self.drag_start = Some(pos);
        self.initial_pan = current_pan;
    }

    /// Handle drag update
    #[must_use]
    pub fn update_drag(&self, pos: [f32; 2], zoom: f32) -> Option<[f32; 2]> {
        self.drag_start.map(|start| {
            let zoom_rcp = 1.0 / zoom;
            let delta_x = (pos[0] - start[0]) * zoom_rcp;
            let delta_y = (pos[1] - start[1]) * zoom_rcp;
            [self.initial_pan[0] + delta_x, self.initial_pan[1] + delta_y]
        })
    }

    /// End drag
    pub const fn end_drag(&mut self) {
        self.drag_start = None;
    }

    /// Check if dragging
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
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
        let vs = ViewportState::new();
        let pan = vs.update_drag([10.0, 5.0], 1.0);
        assert!(pan.is_none());
    }

    #[test]
    fn viewport_default_not_dragging() {
        let vs = ViewportState::default();
        assert!(!vs.is_dragging());
    }

    #[test]
    fn viewport_drag_with_initial_pan_offset() {
        let mut vs = ViewportState::new();
        // Start at (50, 50) with existing pan (2.0, 3.0)
        vs.start_drag([50.0, 50.0], [2.0, 3.0]);
        // Move to (60, 55) at zoom=1 → delta=(10,5), result=initial+delta
        let pan = vs.update_drag([60.0, 55.0], 1.0).unwrap();
        assert!((pan[0] - 12.0).abs() < 1e-5, "pan[0]={}", pan[0]);
        assert!((pan[1] - 8.0).abs() < 1e-5, "pan[1]={}", pan[1]);
    }

    #[test]
    fn viewport_zoom_half_doubles_movement() {
        let mut vs = ViewportState::new();
        vs.start_drag([0.0, 0.0], [0.0, 0.0]);
        // zoom=0.5 → 1/zoom=2 → pixel movement is doubled
        let pan = vs.update_drag([10.0, 0.0], 0.5).unwrap();
        assert!((pan[0] - 20.0).abs() < 1e-5, "pan[0]={}", pan[0]);
    }

    #[test]
    fn viewport_update_drag_y_only() {
        let mut vs = ViewportState::new();
        vs.start_drag([0.0, 0.0], [0.0, 0.0]);
        let pan = vs.update_drag([0.0, 8.0], 1.0).unwrap();
        assert!(pan[0].abs() < 1e-5);
        assert!((pan[1] - 8.0).abs() < 1e-5, "pan[1]={}", pan[1]);
    }

    #[test]
    fn viewport_restart_drag_uses_new_initial_pan() {
        let mut vs = ViewportState::new();
        vs.start_drag([0.0, 0.0], [5.0, 5.0]);
        vs.end_drag();
        vs.start_drag([0.0, 0.0], [0.0, 0.0]);
        let pan = vs.update_drag([3.0, 0.0], 1.0).unwrap();
        assert!((pan[0] - 3.0).abs() < 1e-5, "pan[0]={}", pan[0]);
    }
}
