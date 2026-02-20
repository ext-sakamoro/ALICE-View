//! SDF Control Panel
//!
//! Interactive controls for 3D SDF visualization.
//! Author: Moroya Sakamoto

const RGB_RCP: f32 = 1.0 / 255.0;

use crate::app::{Camera3D, RenderMode, ViewerState};
use super::export::ExportFormat;
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
    /// Current scene
    pub scene: SdfScene,
    /// Whether dynamic SDF is available
    has_dynamic_sdf: bool,
    /// Loaded .asdf file info
    loaded_asdf_info: Option<String>,
    /// Export mesh resolution
    pub export_resolution: u32,
    /// Pending export request
    pub pending_export: Option<ExportFormat>,
}

impl Default for SdfPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SdfPanel {
    pub fn new() -> Self {
        Self {
            scene: SdfScene::default(),
            has_dynamic_sdf: false,
            loaded_asdf_info: None,
            export_resolution: 64,
            pending_export: None,
        }
    }

    /// Set dynamic SDF availability (called when .asdf is loaded)
    pub fn set_dynamic_sdf(&mut self, available: bool, info: Option<String>) {
        self.has_dynamic_sdf = available;
        self.loaded_asdf_info = info;
        if available {
            self.scene = SdfScene::LoadedAsdf;
        }
    }

    /// Render the SDF control panel
    pub fn render(&mut self, ctx: &Context, state: &mut ViewerState) {
        if state.render_mode != RenderMode::Sdf3D {
            return;
        }

        egui::SidePanel::left("sdf_panel")
            .default_width(260.0)
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
            if self.has_dynamic_sdf {
                ui.label(egui::RichText::new("Loaded SDF").strong().color(egui::Color32::from_rgb(100, 255, 100)));
                let label = if let Some(ref info) = self.loaded_asdf_info {
                    format!("  {}", info)
                } else {
                    "  Loaded .asdf".to_string()
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
            let mut pos = state.camera.position;
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut pos.x).speed(0.1).clamp_range(-50.0..=50.0));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut pos.y).speed(0.1).clamp_range(-50.0..=50.0));
                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut pos.z).speed(0.1).clamp_range(-50.0..=50.0));
            });
            state.camera.position = pos;

            let mut fov_deg = state.camera.fov.to_degrees();
            ui.add(egui::Slider::new(&mut fov_deg, 20.0..=120.0).text("FOV").suffix("°"));
            state.camera.fov = fov_deg.to_radians();

            ui.add_space(4.0);
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
                if ui.button("Reset").clicked() {
                    state.camera = Camera3D::default();
                }
            });
        });

        ui.add_space(8.0);

        // Lighting Controls
        ui.collapsing("Lighting", |ui| {
            ui.add(egui::Slider::new(&mut state.light_dir[0], -1.0..=1.0).text("Light X"));
            ui.add(egui::Slider::new(&mut state.light_dir[1], -1.0..=1.0).text("Light Y"));
            ui.add(egui::Slider::new(&mut state.light_dir[2], -1.0..=1.0).text("Light Z"));
            ui.add(egui::Slider::new(&mut state.light_intensity, 0.0..=3.0).text("Intensity"));
            ui.add(egui::Slider::new(&mut state.ambient_intensity, 0.0..=1.0).text("Ambient"));

            ui.add_space(4.0);
            ui.label("Background");
            let mut color = egui::Color32::from_rgb(
                (state.bg_color[0] * 255.0) as u8,
                (state.bg_color[1] * 255.0) as u8,
                (state.bg_color[2] * 255.0) as u8,
            );
            if ui.color_edit_button_srgba(&mut color).changed() {
                state.bg_color = [
                    color.r() as f32 * RGB_RCP,
                    color.g() as f32 * RGB_RCP,
                    color.b() as f32 * RGB_RCP,
                ];
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Sunset").clicked() {
                    state.light_dir = [0.8, 0.2, 0.3];
                    state.light_intensity = 1.5;
                    state.ambient_intensity = 0.1;
                    state.bg_color = [0.05, 0.02, 0.02];
                }
                if ui.button("Studio").clicked() {
                    state.light_dir = [0.5, 1.0, 0.3];
                    state.light_intensity = 1.0;
                    state.ambient_intensity = 0.15;
                    state.bg_color = [0.02, 0.02, 0.05];
                }
                if ui.button("Flat").clicked() {
                    state.light_dir = [0.0, 1.0, 0.0];
                    state.light_intensity = 0.8;
                    state.ambient_intensity = 0.4;
                    state.bg_color = [0.1, 0.1, 0.1];
                }
            });
        });

        ui.add_space(8.0);

        // Raymarching Settings
        ui.collapsing("Raymarching", |ui| {
            ui.add(egui::Slider::new(&mut state.sdf_max_steps, 16..=512).text("Max Steps"));

            let mut epsilon_log = state.sdf_epsilon.log10();
            ui.add(egui::Slider::new(&mut epsilon_log, -5.0..=-1.0).text("Epsilon"));
            state.sdf_epsilon = 10.0_f32.powf(epsilon_log);
            ui.label(egui::RichText::new(format!("  = {:.6}", state.sdf_epsilon)).small().weak());
        });

        ui.add_space(8.0);

        // Visualization Options
        ui.collapsing("Visualization", |ui| {
            ui.checkbox(&mut state.sdf_show_normals, "Show Normals (N)");
            ui.checkbox(&mut state.sdf_ambient_occlusion, "Ambient Occlusion (O)");
        });

        ui.add_space(8.0);

        // Actions
        ui.collapsing("Actions", |ui| {
            if ui.button("Screenshot (F12)").clicked() {
                state.screenshot_requested = true;
            }

            if self.has_dynamic_sdf {
                ui.separator();
                ui.label(egui::RichText::new("Export Mesh").strong());
                ui.add(egui::Slider::new(&mut self.export_resolution, 16..=256).text("Resolution"));

                ui.horizontal(|ui| {
                    if ui.button("Export GLB").clicked() {
                        self.pending_export = Some(ExportFormat::Glb);
                    }
                    if ui.button("Export OBJ").clicked() {
                        self.pending_export = Some(ExportFormat::Obj);
                    }
                });
            }
        });

        ui.add_space(8.0);

        // Shortcuts
        ui.collapsing("Shortcuts", |ui| {
            let shortcuts = [
                ("WASD", "Move camera"),
                ("QE", "Up / Down"),
                ("Drag", "Orbit"),
                ("Scroll", "Dolly"),
                ("R", "Reset camera"),
                ("M", "Toggle 2D/3D"),
                ("N", "Toggle normals"),
                ("O", "Toggle AO"),
                ("F12", "Screenshot"),
                ("F11", "Fullscreen"),
            ];
            for (key, desc) in shortcuts {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(key).strong().monospace());
                    ui.label(desc);
                });
            }
        });

        // Bottom info
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("3D SDF").color(egui::Color32::from_rgb(100, 200, 255)));
            ui.separator();
            ui.label(egui::RichText::new(self.scene.name()).strong());
        });

        if state.sdf_max_steps > 256 {
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
