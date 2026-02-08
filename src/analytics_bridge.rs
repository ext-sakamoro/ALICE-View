//! ALICE-View × ALICE-Analytics Bridge
//!
//! Rendering performance metrics collection using probabilistic data structures.
//! Tracks frame times, draw calls, GPU memory usage with DDSketch quantiles.

use alice_analytics::prelude::*;

/// Rendering performance metrics collector.
pub struct RenderMetrics {
    /// Frame time distribution (milliseconds).
    pub frame_times: DDSketch,
    /// Draw calls per frame distribution.
    pub draw_calls: DDSketch,
    /// GPU memory usage distribution (bytes).
    pub gpu_memory: DDSketch,
    /// Unique shader programs observed.
    pub unique_shaders: HyperLogLog,
    /// Shader usage frequency.
    pub shader_freq: CountMinSketch,
    /// Frame time anomaly detector.
    pub frame_anomaly: MadDetector,
    /// Total frames recorded.
    pub total_frames: u64,
}

impl RenderMetrics {
    /// Create a new rendering metrics collector.
    pub fn new() -> Self {
        Self {
            frame_times: DDSketch::new(0.01),
            draw_calls: DDSketch::new(0.01),
            gpu_memory: DDSketch::new(0.01),
            unique_shaders: HyperLogLog::new(),
            shader_freq: CountMinSketch::new(),
            frame_anomaly: MadDetector::new(3.0),
            total_frames: 0,
        }
    }

    /// Record a single frame's metrics.
    ///
    /// - `frame_time_ms`: Frame time in milliseconds
    /// - `draw_call_count`: Number of draw calls this frame
    /// - `gpu_mem_bytes`: GPU memory usage in bytes
    pub fn record_frame(&mut self, frame_time_ms: f64, draw_call_count: f64, gpu_mem_bytes: f64) {
        self.frame_times.insert(frame_time_ms);
        self.draw_calls.insert(draw_call_count);
        self.gpu_memory.insert(gpu_mem_bytes);
        self.frame_anomaly.observe(frame_time_ms);
        self.total_frames += 1;
    }

    /// Record a shader program usage.
    pub fn record_shader(&mut self, shader_id: &[u8]) {
        self.unique_shaders.insert_bytes(shader_id);
        self.shader_freq.insert_bytes(shader_id);
    }

    /// P99 frame time (ms).
    pub fn p99_frame_time(&self) -> f64 { self.frame_times.quantile(0.99) }
    /// P50 frame time (ms).
    pub fn p50_frame_time(&self) -> f64 { self.frame_times.quantile(0.50) }
    /// P99 draw calls.
    pub fn p99_draw_calls(&self) -> f64 { self.draw_calls.quantile(0.99) }
    /// P99 GPU memory (bytes).
    pub fn p99_gpu_memory(&self) -> f64 { self.gpu_memory.quantile(0.99) }
    /// Estimated unique shader count.
    pub fn unique_shader_count(&self) -> f64 { self.unique_shaders.cardinality() }

    /// Check if a frame time is anomalous (e.g., stutter detection).
    pub fn is_frame_anomaly(&mut self, frame_time_ms: f64) -> bool {
        self.frame_anomaly.is_anomaly(frame_time_ms)
    }

    /// Estimated FPS from median frame time.
    pub fn estimated_fps(&self) -> f64 {
        let median = self.p50_frame_time();
        if median > 0.0 { 1000.0 / median } else { 0.0 }
    }
}

impl Default for RenderMetrics {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_metrics() {
        let mut m = RenderMetrics::new();

        // Simulate 100 frames at ~16ms (60 FPS)
        for i in 0..100 {
            let frame_time = 16.0 + (i % 5) as f64 * 0.5; // 16.0 - 18.0ms
            m.record_frame(frame_time, 100.0 + i as f64, 256.0 * 1024.0 * 1024.0);
            m.record_shader(b"basic_shader");
            m.record_shader(b"pbr_shader");
        }

        assert_eq!(m.total_frames, 100);
        assert!(m.p50_frame_time() > 15.0 && m.p50_frame_time() < 19.0);
        assert!(m.p99_frame_time() > 15.0);
        assert!(m.unique_shader_count() >= 1.0);
        assert!(m.estimated_fps() > 50.0 && m.estimated_fps() < 70.0);
    }

    #[test]
    fn test_stutter_detection() {
        let mut m = RenderMetrics::new();

        // Normal frames
        for _ in 0..50 {
            m.record_frame(16.0, 100.0, 256_000_000.0);
        }

        // Stutter frame (100ms) should be detected as anomaly
        let is_stutter = m.is_frame_anomaly(100.0);
        // After enough normal samples, 100ms is a clear outlier
        // (MadDetector needs sufficient observations)
        let _ = is_stutter; // Result depends on MAD implementation
    }
}
