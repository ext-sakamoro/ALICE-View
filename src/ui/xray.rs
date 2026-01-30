//! X-Ray mode visualization overlay
//!
//! Displays underlying mathematical structure and parameters.

use crate::app::{ViewerState, XRayType};
use egui::{Color32, RichText, Stroke};

/// Render X-Ray mode overlay
pub fn render_xray_overlay(ctx: &egui::Context, state: &ViewerState) {
    egui::Area::new(egui::Id::new("xray_overlay"))
        .anchor(egui::Align2::LEFT_TOP, [10.0, 40.0])
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(Color32::from_rgba_premultiplied(0, 20, 40, 240))
                .stroke(Stroke::new(1.0, Color32::from_rgb(0, 255, 255)))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔬");
                        ui.label(RichText::new("X-RAY DEBUGGER").strong().color(Color32::from_rgb(0, 255, 255)));
                    });

                    ui.separator();

                    // Mode indicator
                    let (mode_name, desc) = match state.xray_type {
                        XRayType::MotionVectors => (
                            "MOTION VECTORS",
                            "Visualizing ASP flow field (Green=H, Red=V)",
                        ),
                        XRayType::FftHeatmap => (
                            "FFT HEATMAP",
                            "Frequency domain intensity (High=Bright)",
                        ),
                        XRayType::EquationOverlay => (
                            "MATH OVERLAY",
                            "Underlying parametric equations",
                        ),
                        XRayType::Wireframe => (
                            "WIREFRAME",
                            "Procedural mesh tessellation",
                        ),
                    };

                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        ui.label(RichText::new(mode_name).strong().color(Color32::YELLOW));
                    });
                    ui.label(RichText::new(desc).small().italics());

                    ui.add_space(5.0);
                    ui.separator();

                    // Active Equation
                    ui.label(RichText::new("Active Equation:").strong());

                    match state.xray_type {
                        XRayType::MotionVectors => {
                            ui.monospace("v(x,y) = ∇f(x,y)");
                            ui.label(RichText::new("Gradient of noise field").small());
                        }
                        XRayType::FftHeatmap => {
                            ui.monospace("F(ω) = ∫f(x)e^(-iωx)dx");
                            ui.label(RichText::new("Fourier transform magnitude").small());
                        }
                        XRayType::EquationOverlay => {
                            ui.monospace("f(x,y) = Σ[A·noise(f·x, f·y)]");
                            ui.label(RichText::new("Fractal Brownian Motion").small());
                        }
                        XRayType::Wireframe => {
                            ui.monospace("mesh(u,v) → (x,y,z)");
                            ui.label(RichText::new("Parametric surface").small());
                        }
                    }

                    ui.add_space(5.0);
                    ui.separator();

                    // Parameters Grid
                    ui.label(RichText::new("Parameters:").strong());
                    egui::Grid::new("params_grid")
                        .striped(true)
                        .spacing([15.0, 3.0])
                        .show(ui, |ui| {
                            ui.label("Scale (α):");
                            ui.monospace(format!("{:.6}", 1.0 / state.zoom));
                            ui.end_row();

                            ui.label("Offset (δ):");
                            ui.monospace(format!("[{:.4}, {:.4}]", state.pan[0], state.pan[1]));
                            ui.end_row();

                            ui.label("Octaves:");
                            ui.monospace("6");
                            ui.end_row();

                            ui.label("Persistence:");
                            ui.monospace("0.5");
                            ui.end_row();

                            ui.label("Lacunarity:");
                            ui.monospace("2.0");
                            ui.end_row();
                        });

                    ui.add_space(5.0);

                    // Legend for current mode
                    match state.xray_type {
                        XRayType::MotionVectors => {
                            ui.separator();
                            ui.label(RichText::new("Legend:").small().strong());
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::GREEN, "■");
                                ui.label(RichText::new("Horizontal").small());
                                ui.colored_label(Color32::RED, "■");
                                ui.label(RichText::new("Vertical").small());
                            });
                        }
                        XRayType::FftHeatmap => {
                            ui.separator();
                            ui.label(RichText::new("Legend:").small().strong());
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::DARK_BLUE, "■");
                                ui.label(RichText::new("Low freq").small());
                                ui.colored_label(Color32::YELLOW, "■");
                                ui.label(RichText::new("High freq").small());
                            });
                        }
                        _ => {}
                    }

                    ui.add_space(5.0);
                    ui.label(RichText::new("Press [Tab] to cycle modes").small().weak());
                });
        });
}

/// X-Ray color scheme
pub struct XRayColors {
    pub motion_positive: [f32; 3],
    pub motion_negative: [f32; 3],
    pub frequency_low: [f32; 3],
    pub frequency_high: [f32; 3],
    pub wireframe: [f32; 3],
}

impl Default for XRayColors {
    fn default() -> Self {
        Self {
            motion_positive: [0.0, 1.0, 0.5],  // Green
            motion_negative: [1.0, 0.3, 0.3],  // Red
            frequency_low: [0.0, 0.0, 0.2],    // Dark blue
            frequency_high: [1.0, 1.0, 0.0],   // Yellow
            wireframe: [0.0, 1.0, 1.0],        // Cyan
        }
    }
}
