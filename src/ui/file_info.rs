//! File Information Panel
//!
//! Displays .alice file details: equation, compression, metadata

use crate::decoder::alice::{AliceFile, AlicePayload};
use egui::{Color32, RichText, Ui};

/// File information for display
#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    /// File path
    pub path: Option<String>,
    /// Content type name
    pub content_type: String,
    /// Human-readable equation
    pub equation: String,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Original size (bytes)
    pub original_size: u64,
    /// Compressed size (bytes)
    pub compressed_size: u64,
    /// Sensor ID
    pub sensor_id: Option<String>,
    /// Timestamp
    pub timestamp: Option<String>,
    /// Location
    pub location: Option<String>,
    /// Unit
    pub unit: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Sample count (for linear)
    pub sample_count: Option<u32>,
    /// Extra details (content-specific)
    pub details: Vec<(String, String)>,
}

impl FileInfo {
    /// Create from AliceFile
    pub fn from_alice_file(file: &AliceFile, path: Option<&str>) -> Self {
        let mut info = Self {
            path: path.map(|s| s.to_string()),
            content_type: file.content_type_name().to_string(),
            equation: file.equation_string(),
            compression_ratio: file.compression_ratio(),
            original_size: file.header.original_size,
            compressed_size: file.header.compressed_size,
            sensor_id: file.metadata.sensor_id.clone(),
            timestamp: file.metadata.timestamp.clone(),
            location: file.metadata.location.clone(),
            unit: file.metadata.unit.clone(),
            description: file.metadata.description.clone(),
            sample_count: None,
            details: Vec::new(),
        };

        // Extract content-specific details
        match &file.payload {
            AlicePayload::Linear(p) => {
                info.sample_count = Some(p.sample_count);
                info.details.push(("Slope (Q16)".to_string(), format!("{}", p.slope_q16)));
                info.details.push(("Intercept (Q16)".to_string(), format!("{}", p.intercept_q16)));
                info.details.push(("Slope (float)".to_string(), format!("{:.6}", p.slope_f32())));
                info.details.push(("Intercept (float)".to_string(), format!("{:.4}", p.intercept_f32())));
            }
            AlicePayload::Perlin(p) => {
                info.details.push(("Seed".to_string(), format!("{}", p.seed)));
                info.details.push(("Scale".to_string(), format!("{:.2}", p.scale)));
                info.details.push(("Octaves".to_string(), format!("{}", p.octaves)));
                info.details.push(("Persistence".to_string(), format!("{:.2}", p.persistence)));
                info.details.push(("Lacunarity".to_string(), format!("{:.2}", p.lacunarity)));
            }
            AlicePayload::Fractal(p) => {
                info.details.push(("Type".to_string(), p.fractal_name().to_string()));
                info.details.push(("Max Iterations".to_string(), format!("{}", p.max_iterations)));
                info.details.push(("Escape Radius".to_string(), format!("{:.1}", p.escape_radius)));
                info.details.push(("Center".to_string(), format!("({:.6}, {:.6})", p.center_x, p.center_y)));
                if p.fractal_type == 1 {
                    info.details.push(("Julia C".to_string(), format!("({:.4}, {:.4})", p.julia_cx, p.julia_cy)));
                }
            }
        }

        info
    }

    /// Render the file info panel
    pub fn render(&self, ui: &mut Ui) {
        ui.heading("📄 File Information");
        ui.separator();

        // File path
        if let Some(path) = &self.path {
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.monospace(path);
            });
        }

        ui.add_space(8.0);

        // Content type badge
        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.label(
                RichText::new(&self.content_type)
                    .color(Color32::from_rgb(100, 200, 100))
                    .strong()
            );
        });

        ui.add_space(8.0);

        // Equation (highlighted)
        ui.group(|ui| {
            ui.label(RichText::new("📐 Equation").strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(&self.equation)
                    .color(Color32::from_rgb(255, 200, 100))
                    .monospace()
                    .size(16.0)
            );
        });

        ui.add_space(8.0);

        // Compression stats
        ui.group(|ui| {
            ui.label(RichText::new("📦 Compression").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Ratio:");
                ui.label(
                    RichText::new(format!("{:.1}x", self.compression_ratio))
                        .color(Color32::from_rgb(100, 200, 255))
                        .strong()
                );
            });

            ui.horizontal(|ui| {
                ui.label("Original:");
                ui.label(format_bytes(self.original_size));
            });

            ui.horizontal(|ui| {
                ui.label("Compressed:");
                ui.label(
                    RichText::new(format_bytes(self.compressed_size))
                        .color(Color32::from_rgb(100, 255, 100))
                );
            });

            if let Some(count) = self.sample_count {
                ui.horizontal(|ui| {
                    ui.label("Samples:");
                    ui.label(format!("{}", count));
                });
            }
        });

        ui.add_space(8.0);

        // Metadata
        let has_metadata = self.sensor_id.is_some()
            || self.timestamp.is_some()
            || self.location.is_some()
            || self.unit.is_some()
            || self.description.is_some();

        if has_metadata {
            ui.group(|ui| {
                ui.label(RichText::new("📋 Metadata").strong());
                ui.add_space(4.0);

                if let Some(ref id) = self.sensor_id {
                    ui.horizontal(|ui| {
                        ui.label("Sensor ID:");
                        ui.monospace(id);
                    });
                }
                if let Some(ref ts) = self.timestamp {
                    ui.horizontal(|ui| {
                        ui.label("Timestamp:");
                        ui.label(ts);
                    });
                }
                if let Some(ref loc) = self.location {
                    ui.horizontal(|ui| {
                        ui.label("Location:");
                        ui.label(loc);
                    });
                }
                if let Some(ref unit) = self.unit {
                    ui.horizontal(|ui| {
                        ui.label("Unit:");
                        ui.label(unit);
                    });
                }
                if let Some(ref desc) = self.description {
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.label(desc);
                    });
                }
            });

            ui.add_space(8.0);
        }

        // Technical details (collapsible)
        if !self.details.is_empty() {
            ui.collapsing("🔧 Technical Details", |ui| {
                for (key, value) in &self.details {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}:", key));
                        ui.monospace(value);
                    });
                }
            });
        }
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Compact file info for status bar
pub fn render_compact_info(ui: &mut Ui, info: &FileInfo) {
    ui.horizontal(|ui| {
        // Type badge
        ui.label(
            RichText::new(&info.content_type)
                .color(Color32::from_rgb(100, 200, 100))
                .small()
        );

        ui.separator();

        // Compression
        ui.label(
            RichText::new(format!("{:.0}x", info.compression_ratio))
                .color(Color32::from_rgb(100, 200, 255))
                .small()
        );

        ui.separator();

        // Size
        ui.label(
            RichText::new(format_bytes(info.compressed_size))
                .small()
        );

        // Unit if available
        if let Some(ref unit) = info.unit {
            ui.separator();
            ui.label(RichText::new(unit).small());
        }
    });
}
