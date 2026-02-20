//! ALICE-View x ALICE-DB Bridge
//!
//! Query time-series data from ALICE-DB and convert to renderable
//! plot data for GPU visualization.

use alice_db::{AliceDB, Aggregation, StorageStats};

/// Time-series plot data ready for GPU rendering.
#[derive(Clone, Debug)]
pub struct PlotSeries {
    /// Series name/label.
    pub label: String,
    /// Data points as (x, y) in f32 for GPU.
    pub points: Vec<[f32; 2]>,
    /// X-axis min/max bounds for auto-scaling.
    pub x_range: [f32; 2],
    /// Y-axis min/max bounds for auto-scaling.
    pub y_range: [f32; 2],
}

/// Storage statistics for HUD overlay display.
#[derive(Clone, Debug)]
pub struct DbOverlayStats {
    /// Number of compressed segments on disk.
    pub total_segments: usize,
    /// Current memtable entry count.
    pub memtable_size: usize,
    /// Average compression ratio across segments.
    pub compression_ratio: f64,
    /// Model type distribution (name, count).
    pub model_distribution: Vec<(String, usize)>,
    /// Total disk usage in bytes.
    pub disk_size_bytes: u64,
}

/// Query a time range from DB and convert to plot-ready f32 data.
pub fn query_plot_series(
    db: &AliceDB,
    start: i64,
    end: i64,
    label: &str,
) -> std::io::Result<PlotSeries> {
    let raw = db.scan(start, end)?;

    if raw.is_empty() {
        return Ok(PlotSeries {
            label: label.to_string(),
            points: Vec::new(),
            x_range: [0.0, 0.0],
            y_range: [0.0, 0.0],
        });
    }

    let mut x_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;

    let points: Vec<[f32; 2]> = raw
        .iter()
        .map(|&(t, v)| {
            let x = t as f32;
            let y = v;
            if x < x_min { x_min = x; }
            if x > x_max { x_max = x; }
            if y < y_min { y_min = y; }
            if y > y_max { y_max = y; }
            [x, y]
        })
        .collect();

    Ok(PlotSeries {
        label: label.to_string(),
        points,
        x_range: [x_min, x_max],
        y_range: [y_min, y_max],
    })
}

/// Query downsampled data for large time ranges.
///
/// Uses ALICE-DB's aggregation-based downsampling to reduce point count
/// before sending to the GPU.
pub fn query_downsampled_series(
    db: &AliceDB,
    start: i64,
    end: i64,
    interval: i64,
    agg: Aggregation,
    label: &str,
) -> std::io::Result<PlotSeries> {
    let raw = db.downsample(start, end, interval, agg)?;

    if raw.is_empty() {
        return Ok(PlotSeries {
            label: label.to_string(),
            points: Vec::new(),
            x_range: [0.0, 0.0],
            y_range: [0.0, 0.0],
        });
    }

    let mut x_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_min = f32::MAX;
    let mut y_max = f32::MIN;

    let points: Vec<[f32; 2]> = raw
        .iter()
        .map(|&(t, v)| {
            let x = t as f32;
            let y = v as f32;
            if x < x_min { x_min = x; }
            if x > x_max { x_max = x; }
            if y < y_min { y_min = y; }
            if y > y_max { y_max = y; }
            [x, y]
        })
        .collect();

    Ok(PlotSeries {
        label: label.to_string(),
        points,
        x_range: [x_min, x_max],
        y_range: [y_min, y_max],
    })
}

/// Extract DB storage stats for HUD overlay.
pub fn extract_overlay_stats(db: &AliceDB) -> DbOverlayStats {
    let stats: StorageStats = db.stats();
    DbOverlayStats {
        total_segments: stats.total_segments,
        memtable_size: stats.memtable_size,
        compression_ratio: stats.average_compression_ratio,
        model_distribution: stats.model_distribution.into_iter().collect(),
        disk_size_bytes: stats.total_disk_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_db() -> (tempfile::TempDir, AliceDB) {
        let dir = tempdir().unwrap();
        let db = AliceDB::open(dir.path()).unwrap();
        // Insert linear ramp: y = 2x
        for i in 0..200 {
            db.put(i, i as f32 * 2.0).unwrap();
        }
        db.flush().unwrap();
        (dir, db)
    }

    #[test]
    fn test_query_plot_series() {
        let (_dir, db) = make_test_db();

        let series = query_plot_series(&db, 0, 199, "ramp").unwrap();
        assert_eq!(series.label, "ramp");
        assert!(!series.points.is_empty());
        // x range should span 0..199
        assert!(series.x_range[0] <= 1.0);
        assert!(series.x_range[1] >= 198.0);
        // y range should span roughly 0..398
        assert!(series.y_range[0] <= 2.0);
        assert!(series.y_range[1] >= 396.0);
    }

    #[test]
    fn test_query_plot_series_empty() {
        let (_dir, db) = make_test_db();

        // Query range with no data
        let series = query_plot_series(&db, 9000, 9999, "empty").unwrap();
        assert_eq!(series.label, "empty");
        assert!(series.points.is_empty());
        assert_eq!(series.x_range, [0.0, 0.0]);
    }

    #[test]
    fn test_query_downsampled_series() {
        let (_dir, db) = make_test_db();

        let series = query_downsampled_series(
            &db, 0, 199, 50, Aggregation::Avg, "ramp_avg",
        )
        .unwrap();
        assert_eq!(series.label, "ramp_avg");
        // 200 points / 50 interval = ~4 buckets
        assert!(series.points.len() >= 2 && series.points.len() <= 6);
    }

    #[test]
    fn test_extract_overlay_stats() {
        let (_dir, db) = make_test_db();

        let overlay = extract_overlay_stats(&db);
        assert!(overlay.total_segments >= 1);
        assert!(overlay.compression_ratio > 1.0);
        // Linear data should be compressed with some model
        assert!(!overlay.model_distribution.is_empty());
    }
}
