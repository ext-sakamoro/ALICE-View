//! SDF Control Panel
//!
//! Interactive controls for 3D SDF visualization.
//! Author: Moroya Sakamoto

use crate::app::{Camera3D, RenderMode, ViewerState};
use egui::{Context, Ui};
use glam::Vec3;

/// Available demo scenes for SDF visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SdfScene {
    /// Carved sphere with cylinders (default demo)
    #[default]
    CarvedSphere = 0,
    /// Simple sphere
    Sphere = 1,
    /// Rounded box
    RoundedBox = 2,
    /// Torus knot
    TorusKnot = 3,
    /// Infinite pillars
    InfinitePillars = 4,
    /// Twisted box
    TwistedBox = 5,
    /// Dynamic SDF from loaded .asdf file
    LoadedAsdf = 100,
}

impl SdfScene {
    pub fn name(&self) -> &'static str {
        match self {
            SdfScene::CarvedSphere => "Carved Sphere",
            SdfScene::Sphere => "Simple Sphere",
            SdfScene::RoundedBox => "Rounded Box",
            SdfScene::TorusKnot => "Torus Knot",
            SdfScene::InfinitePillars => "Infinite Pillars",
            SdfScene::TwistedBox => "Twisted Box",
            SdfScene::LoadedAsdf => "Loaded .asdf",
        }
    }

    pub fn all_demo() -> &'static [SdfScene] {
        &[
            SdfScene::CarvedSphere,
            SdfScene::Sphere,
            SdfScene::RoundedBox,
            SdfScene::TorusKnot,
            SdfScene::InfinitePillars,
            SdfScene::TwistedBox,
        ]
    }
}

/// SDF Panel state
pub struct SdfPanel {
    /// Panel open state
    pub open: bool,
    /// Current scene
    pub scene: SdfScene,
    /// Camera preset
    camera_preset: usize,
    /// Whether dynamic SDF is available
    has_dynamic_sdf: bool,
    /// Loaded .asdf file info
    loaded_asdf_info: Option<String>,
}

impl Default for SdfPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SdfPanel {
    pub fn new() -> Self {
        Self {
            open: true, // Open by default in 3D mode
            scene: SdfScene::default(),
            camera_preset: 0,
            has_dynamic_sdf: false,
            loaded_asdf_info: None,
        }
    }

    /// Set dynamic SDF availability (called when .asdf is loaded)
    pub fn set_dynamic_sdf(&mut self, available: bool, info: Option<String>) {
        self.has_dynamic_sdf = available;
        self.loaded_asdf_info = info;
        if available {
            // Auto-switch to loaded SDF
            self.scene = SdfScene::LoadedAsdf;
        }
    }

    /// Check if dynamic SDF is available
    pub fn has_dynamic_sdf(&self) -> bool {
        self.has_dynamic_sdf
    }

    /// Render the SDF control panel
    pub fn render(&mut self, ctx: &Context, state: &mut ViewerState) {
        if state.render_mode != RenderMode::Sdf3D {
            return;
        }

        egui::SidePanel::left("sdf_panel")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_content(ui, state);
                });
            });
    }

    fn render_content(&mut self, ui: &mut Ui, state: &mut ViewerState) {
        ui.heading("SDF Controls");
        ui.separator();

        // Scene Selection
        ui.collapsing("Scene", |ui| {
            // Show loaded .asdf first if available
            if self.has_dynamic_sdf {
                ui.label(egui::RichText::new("Loaded SDF").strong().color(egui::Color32::from_rgb(100, 255, 100)));
                let label = if let Some(ref info) = self.loaded_asdf_info {
                    format!("📦 {}", info)
                } else {
                    "📦 Loaded .asdf".to_string()
                };
                if ui.selectable_label(self.scene == SdfScene::LoadedAsdf, label).clicked() {
                    self.scene = SdfScene::LoadedAsdf;
                }
                ui.separator();
            }

            ui.label(egui::RichText::new("Demo Scenes").small().weak());
            for scene in SdfScene::all_demo() {
                if ui.selectable_label(self.scene == *scene, scene.name()).clicked() {
                    self.scene = *scene;
                }
            }
        });

        ui.add_space(8.0);

        // Camera Controls
        ui.collapsing("Camera", |ui| {
            ui.horizontal(|ui| {
                ui.label("Position:");
            });

            let mut pos = state.camera.position;
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut pos.x).speed(0.1).clamp_range(-50.0..=50.0));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut pos.y).speed(0.1).clamp_range(-50.0..=50.0));
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut pos.z).speed(0.1).clamp_range(-50.0..=50.0));
            });
            state.camera.position = pos;

            ui.add_space(4.0);

            // FOV slider
            let mut fov_deg = state.camera.fov.to_degrees();
            ui.horizontal(|ui| {
                ui.label("FOV:");
                ui.add(egui::Slider::new(&mut fov_deg, 20.0..=120.0).suffix("°"));
            });
            state.camera.fov = fov_deg.to_radians();

            ui.add_space(4.0);

            // Camera presets
            ui.horizontal(|ui| {
                if ui.button("Front").clicked() {
                    state.camera = Camera3D {
                        position: Vec3::new(0.0, 0.0, 5.0),
                        target: Vec3::ZERO,
                        ..state.camera.clone()
                    };
                }
                if ui.button("Top").clicked() {
                    state.camera = Camera3D {
                        position: Vec3::new(0.0, 5.0, 0.01),
                        target: Vec3::ZERO,
                        ..state.camera.clone()
                    };
                }
                if ui.button("Side").clicked() {
                    state.camera = Camera3D {
                        position: Vec3::new(5.0, 0.0, 0.0),
                        target: Vec3::ZERO,
                        ..state.camera.clone()
                    };
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    state.camera = Camera3D::default();
                }
                if ui.button("Close").clicked() {
                    state.camera = Camera3D {
                        position: Vec3::new(0.0, 0.0, 2.0),
                        target: Vec3::ZERO,
                        ..state.camera.clone()
                    };
                }
                if ui.button("Far").clicked() {
                    state.camera = Camera3D {
                        position: Vec3::new(0.0, 0.0, 10.0),
                        target: Vec3::ZERO,
                        ..state.camera.clone()
                    };
                }
            });
        });

        ui.add_space(8.0);

        // Raymarching Settings
        ui.collapsing("Raymarching", |ui| {
            ui.horizontal(|ui| {
                ui.label("Max Steps:");
                ui.add(egui::Slider::new(&mut state.sdf_max_steps, 16..=512));
            });

            // Epsilon with logarithmic scale
            let mut epsilon_log = state.sdf_epsilon.log10();
            ui.horizontal(|ui| {
                ui.label("Epsilon:");
                ui.add(egui::Slider::new(&mut epsilon_log, -5.0..=-1.0));
            });
            state.sdf_epsilon = 10.0_f32.powf(epsilon_log);
            ui.label(format!("  = {:.6}", state.sdf_epsilon));
        });

        ui.add_space(8.0);

        // Visualization Options
        ui.collapsing("Visualization", |ui| {
            ui.checkbox(&mut state.sdf_show_normals, "Show Normals (N)");
            ui.checkbox(&mut state.sdf_ambient_occlusion, "Ambient Occlusion (O)");
        });

        ui.add_space(8.0);

        // Keyboard shortcuts help
        ui.collapsing("Shortcuts", |ui| {
            ui.label("WASD - Move camera");
            ui.label("QE - Up/Down");
            ui.label("Mouse drag - Orbit");
            ui.label("Scroll - Dolly");
            ui.label("R - Reset camera");
            ui.label("M - Toggle 2D/3D");
            ui.label("N - Toggle normals");
            ui.label("O - Toggle AO");
        });

        ui.add_space(8.0);

        // Mode indicator
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Mode:");
            ui.label(egui::RichText::new("3D SDF").color(egui::Color32::from_rgb(100, 200, 255)));
        });

        // Scene info
        ui.horizontal(|ui| {
            ui.label("Scene:");
            ui.label(egui::RichText::new(self.scene.name()).strong());
        });

        // Performance hint
        if state.sdf_max_steps > 256 {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("High step count may reduce FPS")
                    .color(egui::Color32::YELLOW)
                    .small()
            );
        }
    }

    /// Get current scene ID for shader
    pub fn scene_id(&self) -> u32 {
        self.scene as u32
    }
}
