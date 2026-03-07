//! パフォーマンスプロファイラ
//!
//! フレーム時間計測、FPS 算出、統計集約を提供する。
//! リングバッファで直近 N フレームの移動平均を維持し、
//! min/max/percentile を高速に算出する。
//!
//! Author: Moroya Sakamoto

use std::time::{Duration, Instant};

/// フレーム時間リングバッファのデフォルトサイズ。
const DEFAULT_HISTORY_SIZE: usize = 256;

/// フレーム時間計測器。
///
/// `begin_frame()` / `end_frame()` で各フレームの実行時間を記録し、
/// 移動平均 FPS やフレーム時間統計を算出する。
pub struct FrameTimer {
    history: Vec<Duration>,
    head: usize,
    count: usize,
    capacity: usize,
    frame_start: Option<Instant>,
    total_frames: u64,
}

impl FrameTimer {
    /// 指定サイズの履歴バッファで作成。
    #[must_use]
    pub fn new(history_size: usize) -> Self {
        let capacity = history_size.max(1);
        Self {
            history: vec![Duration::ZERO; capacity],
            head: 0,
            count: 0,
            capacity,
            frame_start: None,
            total_frames: 0,
        }
    }

    /// フレーム計測開始。
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    /// フレーム計測終了。`begin_frame()` が呼ばれていない場合は何もしない。
    pub fn end_frame(&mut self) {
        if let Some(start) = self.frame_start.take() {
            self.record(start.elapsed());
        }
    }

    /// 手動でフレーム時間を記録。
    pub fn record(&mut self, duration: Duration) {
        self.history[self.head] = duration;
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
        self.total_frames += 1;
    }

    /// 直近フレームの FPS (0 除算防止)。
    #[must_use]
    pub fn fps(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let avg_secs = self.avg_frame_secs();
        if avg_secs == 0.0 {
            return 0.0;
        }
        1.0 / avg_secs
    }

    /// 直近フレームの平均フレーム時間 (ミリ秒)。
    #[must_use]
    pub fn avg_frame_ms(&self) -> f64 {
        self.avg_frame_secs() * 1000.0
    }

    /// 直近フレームの最小フレーム時間 (ミリ秒)。
    #[must_use]
    pub fn min_frame_ms(&self) -> f64 {
        self.active_slice()
            .iter()
            .copied()
            .min()
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
            * 1000.0
    }

    /// 直近フレームの最大フレーム時間 (ミリ秒)。
    #[must_use]
    pub fn max_frame_ms(&self) -> f64 {
        self.active_slice()
            .iter()
            .copied()
            .max()
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
            * 1000.0
    }

    /// p-パーセンタイルのフレーム時間 (ミリ秒)。
    ///
    /// `percentile` は 0.0 〜 1.0 (例: 0.99 = 99th percentile)。
    #[must_use]
    pub fn percentile_frame_ms(&self, percentile: f64) -> f64 {
        let slice = self.active_slice();
        if slice.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<Duration> = slice.to_vec();
        sorted.sort_unstable();
        let idx =
            ((sorted.len() as f64 * percentile.clamp(0.0, 1.0)) as usize).min(sorted.len() - 1);
        sorted[idx].as_secs_f64() * 1000.0
    }

    /// 合計フレーム数。
    #[must_use]
    pub const fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// 記録済みフレーム数 (履歴バッファ内)。
    #[must_use]
    pub const fn recorded_count(&self) -> usize {
        self.count
    }

    /// 履歴をリセット。
    pub fn reset(&mut self) {
        self.head = 0;
        self.count = 0;
        self.total_frames = 0;
        self.frame_start = None;
        for d in &mut self.history {
            *d = Duration::ZERO;
        }
    }

    /// スナップショット統計を取得。
    #[must_use]
    pub fn stats(&self) -> PerfStats {
        PerfStats {
            fps: self.fps(),
            avg_ms: self.avg_frame_ms(),
            min_ms: self.min_frame_ms(),
            max_ms: self.max_frame_ms(),
            p99_ms: self.percentile_frame_ms(0.99),
            total_frames: self.total_frames,
        }
    }

    // --- Internal ---

    fn avg_frame_secs(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let sum: Duration = self.active_slice().iter().copied().sum();
        sum.as_secs_f64() / self.count as f64
    }

    fn active_slice(&self) -> &[Duration] {
        if self.count < self.capacity {
            &self.history[..self.count]
        } else {
            &self.history
        }
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(DEFAULT_HISTORY_SIZE)
    }
}

impl std::fmt::Display for FrameTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameTimer[{:.1} FPS, avg={:.2}ms, min={:.2}ms, max={:.2}ms, frames={}]",
            self.fps(),
            self.avg_frame_ms(),
            self.min_frame_ms(),
            self.max_frame_ms(),
            self.total_frames,
        )
    }
}

/// パフォーマンス統計スナップショット。
#[derive(Debug, Clone, Copy)]
pub struct PerfStats {
    /// 平均 FPS。
    pub fps: f64,
    /// 平均フレーム時間 (ms)。
    pub avg_ms: f64,
    /// 最小フレーム時間 (ms)。
    pub min_ms: f64,
    /// 最大フレーム時間 (ms)。
    pub max_ms: f64,
    /// 99th パーセンタイル (ms)。
    pub p99_ms: f64,
    /// 総フレーム数。
    pub total_frames: u64,
}

impl std::fmt::Display for PerfStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1} FPS | avg {:.2}ms | p99 {:.2}ms | {} frames",
            self.fps, self.avg_ms, self.p99_ms, self.total_frames,
        )
    }
}

/// 汎用パフォーマンスカウンタ。
///
/// draw calls、三角形数など任意の指標を追跡する。
pub struct PerfCounter {
    name: String,
    value: u64,
    frame_value: u64,
    total: u64,
}

impl PerfCounter {
    /// 新しいカウンタを作成。
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            value: 0,
            frame_value: 0,
            total: 0,
        }
    }

    /// カウントを加算。
    pub const fn add(&mut self, count: u64) {
        self.frame_value += count;
    }

    /// フレーム終了: 現在値を確定し、フレーム値をリセット。
    pub const fn end_frame(&mut self) {
        self.value = self.frame_value;
        self.total += self.frame_value;
        self.frame_value = 0;
    }

    /// 直前フレームの確定値。
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
    }

    /// 累計値。
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// カウンタ名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// リセット。
    pub const fn reset(&mut self) {
        self.value = 0;
        self.frame_value = 0;
        self.total = 0;
    }
}

impl std::fmt::Display for PerfCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} (total: {})", self.name, self.value, self.total)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- FrameTimer ---

    #[test]
    fn empty_timer() {
        let timer = FrameTimer::default();
        assert_eq!(timer.total_frames(), 0);
        assert_eq!(timer.recorded_count(), 0);
        assert!((timer.fps()).abs() < 0.01);
        assert!((timer.avg_frame_ms()).abs() < 0.01);
        assert!((timer.min_frame_ms()).abs() < 0.01);
        assert!((timer.max_frame_ms()).abs() < 0.01);
    }

    #[test]
    fn record_single_frame() {
        let mut timer = FrameTimer::new(8);
        timer.record(Duration::from_millis(16));
        assert_eq!(timer.total_frames(), 1);
        assert_eq!(timer.recorded_count(), 1);
        assert!((timer.avg_frame_ms() - 16.0).abs() < 0.1);
        assert!((timer.fps() - 62.5).abs() < 1.0);
    }

    #[test]
    fn record_multiple_frames() {
        let mut timer = FrameTimer::new(4);
        timer.record(Duration::from_millis(10));
        timer.record(Duration::from_millis(20));
        timer.record(Duration::from_millis(30));
        assert_eq!(timer.recorded_count(), 3);
        // avg = 20ms
        assert!((timer.avg_frame_ms() - 20.0).abs() < 0.1);
        assert!((timer.min_frame_ms() - 10.0).abs() < 0.1);
        assert!((timer.max_frame_ms() - 30.0).abs() < 0.1);
    }

    #[test]
    fn ring_buffer_wraps() {
        let mut timer = FrameTimer::new(2);
        timer.record(Duration::from_millis(100));
        timer.record(Duration::from_millis(200));
        timer.record(Duration::from_millis(10)); // overwrites slot 0
        assert_eq!(timer.recorded_count(), 2);
        assert_eq!(timer.total_frames(), 3);
        // Only [200, 10] in buffer → avg = 105ms
        assert!((timer.avg_frame_ms() - 105.0).abs() < 0.1);
    }

    #[test]
    fn begin_end_frame() {
        let mut timer = FrameTimer::new(8);
        timer.begin_frame();
        // Simulate some work
        std::thread::sleep(Duration::from_millis(1));
        timer.end_frame();
        assert_eq!(timer.total_frames(), 1);
        assert!(timer.avg_frame_ms() > 0.0);
    }

    #[test]
    fn end_frame_without_begin() {
        let mut timer = FrameTimer::new(8);
        timer.end_frame(); // Should be no-op
        assert_eq!(timer.total_frames(), 0);
    }

    #[test]
    fn percentile() {
        let mut timer = FrameTimer::new(100);
        for i in 1..=100 {
            timer.record(Duration::from_millis(i));
        }
        // p50 ≈ 50ms, p99 ≈ 99ms
        let p50 = timer.percentile_frame_ms(0.5);
        assert!(p50 > 45.0 && p50 < 55.0, "p50 = {p50}");
        let p99 = timer.percentile_frame_ms(0.99);
        assert!(p99 > 95.0 && p99 <= 100.0, "p99 = {p99}");
    }

    #[test]
    fn percentile_empty() {
        let timer = FrameTimer::new(8);
        assert!((timer.percentile_frame_ms(0.99)).abs() < 0.01);
    }

    #[test]
    fn reset() {
        let mut timer = FrameTimer::new(8);
        timer.record(Duration::from_millis(16));
        timer.record(Duration::from_millis(16));
        timer.reset();
        assert_eq!(timer.total_frames(), 0);
        assert_eq!(timer.recorded_count(), 0);
        assert!((timer.fps()).abs() < 0.01);
    }

    #[test]
    fn stats_snapshot() {
        let mut timer = FrameTimer::new(8);
        timer.record(Duration::from_millis(10));
        timer.record(Duration::from_millis(20));
        let stats = timer.stats();
        assert!(stats.fps > 0.0);
        assert!((stats.avg_ms - 15.0).abs() < 0.1);
        assert!((stats.min_ms - 10.0).abs() < 0.1);
        assert!((stats.max_ms - 20.0).abs() < 0.1);
        assert_eq!(stats.total_frames, 2);
    }

    #[test]
    fn display_timer() {
        let mut timer = FrameTimer::new(8);
        timer.record(Duration::from_millis(16));
        let s = format!("{timer}");
        assert!(s.contains("FPS"));
        assert!(s.contains("avg="));
    }

    #[test]
    fn display_stats() {
        let stats = PerfStats {
            fps: 60.0,
            avg_ms: 16.6,
            min_ms: 15.0,
            max_ms: 18.0,
            p99_ms: 17.5,
            total_frames: 1000,
        };
        let s = format!("{stats}");
        assert!(s.contains("60.0 FPS"));
        assert!(s.contains("1000 frames"));
    }

    // --- PerfCounter ---

    #[test]
    fn counter_basic() {
        let mut counter = PerfCounter::new("draw_calls");
        assert_eq!(counter.name(), "draw_calls");
        assert_eq!(counter.value(), 0);
        assert_eq!(counter.total(), 0);

        counter.add(5);
        counter.add(3);
        counter.end_frame();
        assert_eq!(counter.value(), 8);
        assert_eq!(counter.total(), 8);
    }

    #[test]
    fn counter_multi_frame() {
        let mut counter = PerfCounter::new("triangles");
        counter.add(100);
        counter.end_frame();
        counter.add(200);
        counter.end_frame();
        assert_eq!(counter.value(), 200);
        assert_eq!(counter.total(), 300);
    }

    #[test]
    fn counter_reset() {
        let mut counter = PerfCounter::new("test");
        counter.add(42);
        counter.end_frame();
        counter.reset();
        assert_eq!(counter.value(), 0);
        assert_eq!(counter.total(), 0);
    }

    #[test]
    fn counter_display() {
        let mut counter = PerfCounter::new("draws");
        counter.add(10);
        counter.end_frame();
        let s = format!("{counter}");
        assert!(s.contains("draws"));
        assert!(s.contains("10"));
    }

    #[test]
    fn stats_clone_copy() {
        let stats = PerfStats {
            fps: 60.0,
            avg_ms: 16.6,
            min_ms: 15.0,
            max_ms: 18.0,
            p99_ms: 17.5,
            total_frames: 100,
        };
        let copied = stats;
        assert!((copied.fps - 60.0).abs() < 0.01);
    }
}
