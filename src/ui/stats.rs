//! Performance statistics overlay (Zero-Allocation Version)
//!
//! Displays real-time metrics with micro-graphs using ring buffers
//! to avoid per-frame memory allocation.

use crate::app::ViewerState;
use egui::epaint::PathShape;
use egui::{Color32, Pos2, Stroke};

/// Statistics collector with zero-allocation ring buffer
pub struct StatsCollector {
    /// Ring buffer for frame times
    frame_times: Vec<f32>,
    /// Current write index (head of ring buffer)
    head: usize,
    /// Cached sum for O(1) FPS calculation
    total_time: f32,
    /// Last frame timestamp
    last_frame: std::time::Instant,
    /// Pre-allocated buffer for graph points (reuse to avoid alloc)
    graph_points_buffer: Vec<Pos2>,
}

impl StatsCollector {
    pub fn new() -> Self {
        const CAPACITY: usize = 120; // 2 seconds at 60fps
        Self {
            frame_times: vec![0.0; CAPACITY],
            head: 0,
            total_time: 0.0,
            last_frame: std::time::Instant::now(),
            graph_points_buffer: Vec::with_capacity(CAPACITY),
        }
    }

    /// Record frame time (O(1) - no memory allocation)
    pub fn record_frame(&mut self) {
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32() * 1000.0; // ms
        self.last_frame = now;

        // Ring buffer update: subtract old value, add new value
        let old_val = self.frame_times[self.head];
        self.frame_times[self.head] = delta;
        self.total_time = self.total_time - old_val + delta;

        // Advance index (wrap around)
        self.head = (self.head + 1) % self.frame_times.len();
    }

    /// Calculate average FPS (O(1) - uses cached sum)
    pub fn fps(&self) -> f32 {
        let avg_ms = self.total_time / self.frame_times.len() as f32;
        if avg_ms > 0.001 {
            1000.0 / avg_ms
        } else {
            0.0
        }
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.frame_times.len()
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Render performance stats overlay with graph
pub fn render_stats_overlay(ctx: &egui::Context, state: &ViewerState, collector: &mut StatsCollector) {
    egui::Area::new(egui::Id::new("stats_overlay"))
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgba_premultiplied(10, 10, 10, 230))
                .stroke(Stroke::new(1.0, Color32::from_gray(60)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("⚡");
                        ui.label(egui::RichText::new("ENGINE STATS").strong().color(Color32::WHITE));
                    });

                    ui.add_space(4.0);

                    // 1. Mini Frame-time Graph
                    let graph_size = egui::vec2(200.0, 40.0);
                    let (rect, _) = ui.allocate_exact_size(graph_size, egui::Sense::hover());

                    ui.painter().rect_filled(rect, 2.0, Color32::from_black_alpha(100));

                    // Clear and reuse buffer (no allocation)
                    collector.graph_points_buffer.clear();
                    let history_len = collector.frame_times.len();

                    // Read ring buffer in correct order (oldest to newest)
                    for i in 0..history_len {
                        let idx = (collector.head + i) % history_len;
                        let ms = collector.frame_times[idx];

                        let x = rect.min.x + (i as f32 / history_len as f32) * rect.width();
                        // Scale: 0ms = bottom, 33ms (30fps) = top
                        let h = (ms / 33.3).min(1.0);
                        let y = rect.max.y - h * rect.height();

                        collector.graph_points_buffer.push(Pos2::new(x, y));
                    }

                    if collector.graph_points_buffer.len() >= 2 {
                        ui.painter().add(PathShape::line(
                            collector.graph_points_buffer.clone(),
                            Stroke::new(1.5, Color32::GREEN),
                        ));
                    }

                    // Target line (16.6ms / 60fps)
                    let target_y = rect.max.y - (16.6 / 33.3) * rect.height();
                    ui.painter().line_segment(
                        [Pos2::new(rect.min.x, target_y), Pos2::new(rect.max.x, target_y)],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 50)),
                    );

                    // 30fps warning line
                    let warn_y = rect.max.y - (33.3 / 33.3) * rect.height();
                    ui.painter().line_segment(
                        [Pos2::new(rect.min.x, warn_y), Pos2::new(rect.max.x, warn_y)],
                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 100, 100, 50)),
                    );

                    ui.add_space(4.0);

                    // 2. Metrics Grid
                    egui::Grid::new("stats_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            // FPS
                            ui.label("FPS:");
                            let fps_color = if state.stats.fps > 55.0 {
                                Color32::GREEN
                            } else if state.stats.fps > 30.0 {
                                Color32::YELLOW
                            } else {
                                Color32::RED
                            };
                            ui.colored_label(fps_color, format!("{:.0}", state.stats.fps));
                            ui.end_row();

                            // Frame Time
                            ui.label("Frame:");
                            let frame_ms = 1000.0 / state.stats.fps.max(1.0);
                            ui.label(format!("{:.2} ms", frame_ms));
                            ui.end_row();

                            // Decode Speed
                            ui.label("Decode:");
                            ui.colored_label(
                                Color32::LIGHT_BLUE,
                                format!("{:.2} GB/s", state.stats.decode_speed),
                            );
                            ui.end_row();

                            // Compression Ratio
                            ui.label("Ratio:");
                            let ratio_color = if state.stats.compression_ratio > 100.0 {
                                Color32::GREEN
                            } else if state.stats.compression_ratio > 10.0 {
                                Color32::YELLOW
                            } else {
                                Color32::WHITE
                            };
                            ui.colored_label(ratio_color, format!("{:.0}x", state.stats.compression_ratio));
                            ui.end_row();

                            // Resolution
                            ui.label("Resolution:");
                            ui.label(&state.stats.resolution);
                            ui.end_row();

                            // Zoom
                            ui.label("Zoom:");
                            ui.label(format!("{:.4}x", state.zoom));
                            ui.end_row();
                        });
                });
        });
}
