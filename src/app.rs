//! Main application state and event handling (winit 0.29 compat)

use crate::decoder::Decoder;
use crate::renderer::Renderer;
use crate::ui::Ui;
use glam::Vec3;
use std::sync::Arc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

/// 3D Camera for raymarching
#[derive(Debug, Clone)]
pub struct Camera3D {
    /// Camera position in world space
    pub position: Vec3,
    /// Look-at target point
    pub target: Vec3,
    /// Up vector (usually Y-up)
    pub up: Vec3,
    /// Field of view in radians
    pub fov: f32,
    /// Near clipping plane
    // Stored for potential depth buffer configuration; not currently wired to
    // the raymarcher which uses epsilon-based termination instead.
    #[allow(dead_code)]
    pub near: f32,
    /// Far clipping plane (max raymarch distance)
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            near: 0.01,
            far: 100.0,
        }
    }
}

impl Camera3D {
    /// Get view direction (normalized)
    #[inline(always)]
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Get right vector (normalized)
    #[inline(always)]
    #[must_use]
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    /// Orbit around target (spherical coordinates)
    #[inline(always)]
    pub fn orbit(&mut self, delta_theta: f32, delta_phi: f32) {
        let radius = (self.position - self.target).length();
        let offset = self.position - self.target;

        // Current spherical coordinates
        let mut theta = offset.z.atan2(offset.x);
        let radius_rcp = 1.0 / radius;
        let mut phi = (offset.y * radius_rcp).acos();

        // Apply rotation
        theta += delta_theta;
        phi = (phi + delta_phi).clamp(0.01, std::f32::consts::PI - 0.01);

        // Convert back to Cartesian
        self.position = self.target
            + Vec3::new(
                radius * phi.sin() * theta.cos(),
                radius * phi.cos(),
                radius * phi.sin() * theta.sin(),
            );
    }

    /// Dolly (move along view direction)
    #[inline(always)]
    pub fn dolly(&mut self, distance: f32) {
        let direction = self.forward();
        self.position += direction * distance;
        // Keep minimum distance from target
        let to_target = self.target - self.position;
        if to_target.length() < 0.5 {
            self.position = self.target - direction * 0.5;
        }
    }

    /// Pan (move camera and target together)
    #[inline(always)]
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up;
        let offset = right * delta_x + up * delta_y;
        self.position += offset;
        self.target += offset;
    }
}

/// Quality preset for SDF raymarching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityPreset {
    /// Fast: `max_steps` = 64, epsilon = 0.01, AO off
    Fast,
    /// Balanced: `max_steps` = 128, epsilon = 0.001, AO on (default)
    #[default]
    Balanced,
    /// Quality: `max_steps` = 256, epsilon = 0.0001, AO on
    Quality,
    /// Ultra: `max_steps` = 512, epsilon = 0.00001, AO on
    Ultra,
}

impl QualityPreset {
    /// Display name of this preset
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fast => "Fast",
            Self::Balanced => "Balanced",
            Self::Quality => "Quality",
            Self::Ultra => "Ultra",
        }
    }

    /// All available presets in order
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Fast, Self::Balanced, Self::Quality, Self::Ultra]
    }
}

/// Render mode selection
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// 2D procedural content (legacy)
    #[default]
    Procedural2D,
    /// 3D SDF raymarching
    Sdf3D,
}

/// Application state
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    ui: Ui,
    decoder: Decoder,
    state: ViewerState,
    initial_file: Option<String>,
    // Mouse drag state
    mouse_pressed: bool,
    last_mouse_pos: Option<PhysicalPosition<f64>>,
    // Configuration (for library usage)
    config: ViewerConfig,
}

/// Viewer state
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
pub struct ViewerState {
    // 2D controls (legacy)
    pub zoom: f32,
    pub pan: [f32; 2],

    // 3D camera
    pub camera: Camera3D,
    pub render_mode: RenderMode,

    // Visualization options
    pub xray_mode: bool,
    pub xray_type: XRayType,
    pub show_stats: bool,
    pub paused: bool,
    pub stats: FrameStats,

    // SDF-specific options
    pub sdf_max_steps: u32,
    pub sdf_epsilon: f32,
    pub sdf_show_normals: bool,
    pub sdf_ambient_occlusion: bool,
    /// Adaptive quality: scales epsilon and step size with ray distance
    pub sdf_adaptive_quality: bool,
    /// Current quality preset
    pub sdf_quality_preset: QualityPreset,

    // Lighting
    pub light_dir: [f32; 3],
    pub light_intensity: f32,
    pub ambient_intensity: f32,
    pub bg_color: [f32; 3],

    // Screenshot request
    pub screenshot_requested: bool,
}

impl ViewerState {
    #[must_use]
    pub fn new(render_mode: RenderMode, show_stats: bool) -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            camera: Camera3D::default(),
            render_mode,
            xray_mode: false,
            xray_type: XRayType::default(),
            show_stats,
            paused: false,
            stats: FrameStats {
                fps: 0.0,
                decode_speed: 0.0,
                compression_ratio: 1.0,
                gpu_usage: 0.0,
                resolution: "∞ (Procedural)".to_string(),
            },
            sdf_max_steps: 128,
            sdf_epsilon: 0.001,
            sdf_show_normals: false,
            sdf_ambient_occlusion: true,
            sdf_adaptive_quality: true,
            sdf_quality_preset: QualityPreset::default(),
            light_dir: [0.5, 1.0, 0.3],
            light_intensity: 1.0,
            ambient_intensity: 0.15,
            bg_color: [0.02, 0.02, 0.05],
            screenshot_requested: false,
        }
    }

    /// Apply a quality preset, updating `max_steps`, epsilon, and AO accordingly
    pub const fn apply_quality_preset(&mut self, preset: QualityPreset) {
        self.sdf_quality_preset = preset;
        match preset {
            QualityPreset::Fast => {
                self.sdf_max_steps = 64;
                self.sdf_epsilon = 0.01;
                self.sdf_ambient_occlusion = false;
            }
            QualityPreset::Balanced => {
                self.sdf_max_steps = 128;
                self.sdf_epsilon = 0.001;
                self.sdf_ambient_occlusion = true;
            }
            QualityPreset::Quality => {
                self.sdf_max_steps = 256;
                self.sdf_epsilon = 0.0001;
                self.sdf_ambient_occlusion = true;
            }
            QualityPreset::Ultra => {
                self.sdf_max_steps = 512;
                self.sdf_epsilon = 0.000_01;
                self.sdf_ambient_occlusion = true;
            }
        }
    }
}

/// X-Ray visualization types
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRayType {
    #[default]
    MotionVectors,
    FftHeatmap,
    EquationOverlay,
    Wireframe,
}

/// Frame statistics
#[derive(Default, Clone)]
pub struct FrameStats {
    pub fps: f32,
    pub decode_speed: f64,
    pub compression_ratio: f32,
    // Reserved for future GPU query integration (wgpu timestamp queries).
    #[allow(dead_code)]
    pub gpu_usage: f32,
    pub resolution: String,
}

/// Viewer configuration for library usage
// Fields are part of the public library API; not all are consumed by the
// binary target but are used by external embedders via with_config().
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ViewerConfig {
    /// Window title
    pub title: String,
    /// Initial zoom level (1.0 = default)
    pub initial_zoom: f32,
    /// Initial pan offset [x, y]
    pub initial_pan: [f32; 2],
    /// Start with X-Ray mode enabled
    pub xray_mode: bool,
    /// X-Ray visualization type
    pub xray_type: XRayType,
    /// Show statistics overlay
    pub show_stats: bool,
    /// Start paused
    pub paused: bool,
    /// Initial file to load (ASP/ALZ)
    pub initial_file: Option<String>,
    /// Window width
    pub width: u32,
    /// Window height
    pub height: u32,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            title: "ALICE-View - The Infinite Canvas".to_string(),
            initial_zoom: 1.0,
            initial_pan: [0.0, 0.0],
            xray_mode: false,
            xray_type: XRayType::default(),
            show_stats: false,
            paused: false,
            initial_file: None,
            width: 1280,
            height: 720,
        }
    }
}

#[allow(dead_code)] // Public library API — unused by the binary
impl ViewerConfig {
    /// Create config for displaying temperature data visualization
    #[must_use]
    pub fn for_temperature_data() -> Self {
        Self {
            title: "ALICE-View - Temperature Visualization".to_string(),
            show_stats: true,
            ..Default::default()
        }
    }

    /// Create config for fractal exploration
    #[must_use]
    pub fn for_fractal() -> Self {
        Self {
            title: "ALICE-View - Fractal Explorer".to_string(),
            initial_zoom: 1.0,
            initial_pan: [-0.5, 0.0],
            ..Default::default()
        }
    }

    /// Create minimal viewer for embedding
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            title: "ALICE-View".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    /// Create config for viewing an SDF file
    #[must_use]
    pub fn for_sdf_file(path: &str) -> Self {
        Self {
            title: format!(
                "ALICE-View - {}",
                std::path::Path::new(path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
            initial_file: Some(path.to_string()),
            ..Default::default()
        }
    }
}

impl App {
    // App::new is the public library entry point; the binary uses with_config()
    // but external embedders call new() directly.
    #[allow(dead_code)]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    #[must_use]
    pub fn new(initial_file: Option<String>) -> Self {
        // Auto-detect render mode from file extension
        let render_mode = initial_file.as_ref().map_or(RenderMode::Procedural2D, |f| {
            if f.ends_with(".asdf")
                || f.ends_with(".asdf.json")
                || f.ends_with(".json")
                || f.ends_with(".lol")
            {
                RenderMode::Sdf3D
            } else {
                RenderMode::Procedural2D
            }
        });

        Self {
            window: None,
            renderer: None,
            ui: Ui::new(),
            decoder: Decoder::new(),
            state: ViewerState::new(render_mode, false),
            initial_file,
            mouse_pressed: false,
            last_mouse_pos: None,
            config: ViewerConfig::default(),
        }
    }

    /// Create App with custom configuration (for library usage)
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    #[must_use]
    pub fn with_config(config: ViewerConfig) -> Self {
        // Auto-detect render mode from file extension
        let render_mode = config
            .initial_file
            .as_ref()
            .map_or(RenderMode::Procedural2D, |f| {
                if f.ends_with(".asdf")
                    || f.ends_with(".asdf.json")
                    || f.ends_with(".json")
                    || f.ends_with(".lol")
                {
                    RenderMode::Sdf3D
                } else {
                    RenderMode::Procedural2D
                }
            });

        Self {
            window: None,
            renderer: None,
            ui: Ui::new(),
            decoder: Decoder::new(),
            state: ViewerState::new(render_mode, config.show_stats),
            initial_file: config.initial_file.clone(),
            mouse_pressed: false,
            last_mouse_pos: None,
            config,
        }
    }

    /// Init window and renderer
    ///
    /// # Panics
    ///
    /// Panics if the OS window cannot be created or if no suitable GPU adapter is found.
    pub fn init(&mut self, target: &EventLoopWindowTarget<()>) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .with_title(&self.config.title)
                .with_inner_size(PhysicalSize::new(self.config.width, self.config.height))
                .build(target)
                .unwrap(),
        );

        // Initialize renderer
        self.renderer = Some(
            pollster::block_on(Renderer::new(window.clone()))
                .expect("Failed to initialize GPU renderer — no suitable adapter found"),
        );

        // Load initial file
        if let Some(path) = self.initial_file.take() {
            tracing::info!("Loading: {}", path);
            if let Err(e) = self.decoder.load(&path) {
                tracing::error!("Failed to load file: {}", e);
            }
        }

        self.window = Some(window);
    }

    fn handle_key(&mut self, key: KeyCode, pressed: bool) {
        if !pressed {
            return;
        }

        tracing::debug!("Key pressed: {:?}", key);

        // Camera movement speed
        let move_speed = 0.3;
        let pan_speed = 0.2;

        match key {
            // 3D Camera controls (WASD + QE)
            KeyCode::KeyW => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.dolly(move_speed);
                }
            }
            KeyCode::KeyS => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.dolly(-move_speed);
                }
            }
            KeyCode::KeyA => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.pan(-pan_speed, 0.0);
                }
            }
            KeyCode::KeyD => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.pan(pan_speed, 0.0);
                }
            }
            KeyCode::KeyQ => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.pan(0.0, pan_speed);
                }
            }
            KeyCode::KeyE => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera.pan(0.0, -pan_speed);
                }
            }
            KeyCode::KeyR => {
                // Reset camera to default
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.camera = Camera3D::default();
                    tracing::info!("Camera reset to default");
                }
            }

            // Toggle between 2D/3D modes
            KeyCode::KeyM => {
                self.state.render_mode = match self.state.render_mode {
                    RenderMode::Procedural2D => RenderMode::Sdf3D,
                    RenderMode::Sdf3D => RenderMode::Procedural2D,
                };
                tracing::info!("Render mode: {:?}", self.state.render_mode);
            }

            // SDF visualization options
            KeyCode::KeyN => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.sdf_show_normals = !self.state.sdf_show_normals;
                    tracing::info!("Show normals: {}", self.state.sdf_show_normals);
                }
            }
            KeyCode::KeyO => {
                if self.state.render_mode == RenderMode::Sdf3D {
                    self.state.sdf_ambient_occlusion = !self.state.sdf_ambient_occlusion;
                    tracing::info!("Ambient occlusion: {}", self.state.sdf_ambient_occlusion);
                }
            }

            // General controls
            KeyCode::F1 => {
                self.state.xray_mode = !self.state.xray_mode;
                tracing::info!("X-Ray mode: {}", self.state.xray_mode);
            }
            KeyCode::F2 => {
                self.state.show_stats = !self.state.show_stats;
                tracing::info!("Show stats: {}", self.state.show_stats);
            }
            KeyCode::F3 => {
                self.ui.toggle_file_info();
                tracing::info!("File info panel toggled");
            }
            KeyCode::F11 => {
                if let Some(window) = &self.window {
                    let fullscreen = window.fullscreen();
                    window.set_fullscreen(if fullscreen.is_some() {
                        None
                    } else {
                        Some(winit::window::Fullscreen::Borderless(None))
                    });
                    tracing::info!("Fullscreen toggled");
                }
            }
            KeyCode::F12 => {
                self.state.screenshot_requested = true;
                tracing::info!("Screenshot requested");
            }
            KeyCode::Space => {
                self.state.paused = !self.state.paused;
                tracing::info!("Paused: {}", self.state.paused);
            }
            KeyCode::Tab => {
                self.state.xray_type = match self.state.xray_type {
                    XRayType::MotionVectors => XRayType::FftHeatmap,
                    XRayType::FftHeatmap => XRayType::EquationOverlay,
                    XRayType::EquationOverlay => XRayType::Wireframe,
                    XRayType::Wireframe => XRayType::MotionVectors,
                };
                tracing::info!("X-Ray type: {:?}", self.state.xray_type);
            }
            _ => {}
        }
    }

    fn handle_scroll(&mut self, delta: f32) {
        match self.state.render_mode {
            RenderMode::Procedural2D => {
                // 2D: Zoom in/out
                let zoom_factor = 1.1f32;
                if delta > 0.0 {
                    self.state.zoom *= zoom_factor;
                } else {
                    self.state.zoom /= zoom_factor;
                }
                self.state.zoom = self.state.zoom.clamp(0.001, 1_000_000.0);
            }
            RenderMode::Sdf3D => {
                // 3D: Dolly camera forward/backward
                let dolly_speed = 0.5;
                self.state.camera.dolly(delta * dolly_speed);
            }
        }
    }

    /// Check if egui wants pointer input (mouse is over a UI element)
    fn egui_wants_pointer(&self) -> bool {
        self.renderer
            .as_ref()
            .is_some_and(|r| r.egui_ctx().wants_pointer_input())
    }

    /// Check if egui wants keyboard input (e.g. text field focused)
    fn egui_wants_keyboard(&self) -> bool {
        self.renderer
            .as_ref()
            .is_some_and(|r| r.egui_ctx().wants_keyboard_input())
    }

    /// Main event handling logic (winit 0.29 style)
    ///
    /// # Panics
    ///
    /// Panics if the renderer has not been initialised before a render request arrives.
    pub fn handle_event(&mut self, event: Event<()>, target: &EventLoopWindowTarget<()>) {
        // Forward window events to egui for input processing
        if let (
            Some(renderer),
            Event::WindowEvent {
                event: ref w_event, ..
            },
        ) = (&mut self.renderer, &event)
        {
            let response = renderer.on_window_event(w_event);
            if response.consumed {
                if response.repaint {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                return;
            }
        }

        match event {
            Event::Resumed => {
                self.init(target);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size);
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state: key_state,
                            ..
                        },
                    ..
                } => {
                    // Only process camera/app keys if egui doesn't want keyboard
                    if !self.egui_wants_keyboard() {
                        self.handle_key(key, key_state == ElementState::Pressed);
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // Only process scroll for camera if egui doesn't want pointer
                    if !self.egui_wants_pointer() {
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                        };
                        self.handle_scroll(scroll);
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                // Mouse button press/release
                WindowEvent::MouseInput {
                    state: btn_state,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    if self.egui_wants_pointer() {
                        // egui is handling this click; release camera drag
                        self.mouse_pressed = false;
                    } else {
                        self.mouse_pressed = btn_state == ElementState::Pressed;
                    }
                }
                // Mouse movement (drag to pan/orbit)
                WindowEvent::CursorMoved { position, .. } => {
                    if self.mouse_pressed && !self.egui_wants_pointer() {
                        if let Some(last_pos) = self.last_mouse_pos {
                            let dx = (position.x - last_pos.x) as f32;
                            let dy = (position.y - last_pos.y) as f32;

                            match self.state.render_mode {
                                RenderMode::Procedural2D => {
                                    let sensitivity = 0.002 / self.state.zoom;
                                    self.state.pan[0] -= dx * sensitivity;
                                    self.state.pan[1] += dy * sensitivity;
                                }
                                RenderMode::Sdf3D => {
                                    let orbit_sensitivity = 0.01;
                                    self.state
                                        .camera
                                        .orbit(-dx * orbit_sensitivity, dy * orbit_sensitivity);
                                }
                            }

                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                    self.last_mouse_pos = Some(position);
                }
                WindowEvent::DroppedFile(path) => {
                    let path_str = path.to_string_lossy().to_string();
                    tracing::info!("File dropped: {}", path_str);
                    self.ui.queue_file(path_str);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    if self.window.is_some() && self.renderer.is_some() {
                        self.ui.update(&mut self.state, &mut self.decoder);

                        let renderer = self
                            .renderer
                            .as_mut()
                            .expect("renderer must be initialized before draw");

                        // Check for pending WGSL shader from loaded .asdf file
                        if let Some(wgsl) = self.ui.take_pending_wgsl() {
                            renderer.rebuild_sdf_pipeline_with_wgsl(&wgsl);
                        }

                        if let Err(e) =
                            renderer.render(&mut self.state, &self.decoder, &mut self.ui)
                        {
                            tracing::error!("Render error: {}", e);
                        }

                        // Handle screenshot after render
                        if self.state.screenshot_requested {
                            self.state.screenshot_requested = false;
                            if let Err(e) = renderer.capture_screenshot() {
                                tracing::error!("Screenshot failed: {}", e);
                            }
                        }

                        if !self.state.paused {
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    // ── Camera3D ────────────────────────────────────────────────────

    #[test]
    fn camera_default_position() {
        let cam = Camera3D::default();
        assert_eq!(cam.position, Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(cam.target, Vec3::ZERO);
        assert_eq!(cam.up, Vec3::Y);
    }

    #[test]
    fn camera_forward_direction() {
        let cam = Camera3D::default();
        let fwd = cam.forward();
        // Default camera looks from (0,0,5) toward (0,0,0) → forward = -Z
        assert!((fwd.x).abs() < 1e-5);
        assert!((fwd.y).abs() < 1e-5);
        assert!(fwd.z < 0.0);
        assert!((fwd.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn camera_right_vector() {
        let cam = Camera3D::default();
        let right = cam.right();
        // forward = -Z, up = Y → right = X
        assert!(right.x > 0.0);
        assert!((right.y).abs() < 1e-5);
        assert!((right.z).abs() < 1e-5);
        assert!((right.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn camera_dolly_moves_position() {
        let mut cam = Camera3D::default();
        let pos_before = cam.position;
        cam.dolly(1.0);
        assert!(cam.position.z < pos_before.z);
    }

    #[test]
    fn camera_dolly_min_distance() {
        let mut cam = Camera3D::default();
        cam.dolly(100.0);
        let dist = (cam.target - cam.position).length();
        assert!(dist >= 0.4, "Distance {dist} too small");
    }

    #[test]
    fn camera_pan_moves_both() {
        let mut cam = Camera3D::default();
        let pos_before = cam.position;
        let target_before = cam.target;
        cam.pan(1.0, 0.0);
        let pos_delta = cam.position - pos_before;
        let target_delta = cam.target - target_before;
        assert!((pos_delta - target_delta).length() < 1e-5);
    }

    #[test]
    fn camera_orbit_preserves_distance() {
        let mut cam = Camera3D::default();
        let dist_before = (cam.position - cam.target).length();
        cam.orbit(0.3, 0.2);
        let dist_after = (cam.position - cam.target).length();
        assert!(
            (dist_before - dist_after).abs() < 1e-4,
            "Distance changed: {dist_before} -> {dist_after}"
        );
    }

    #[test]
    fn camera_orbit_changes_position() {
        let mut cam = Camera3D::default();
        let pos_before = cam.position;
        cam.orbit(0.5, 0.0);
        let moved = (cam.position - pos_before).length();
        assert!(moved > 0.01, "Camera didn't move: delta={moved}");
    }

    // ── ViewerConfig ────────────────────────────────────────────────

    #[test]
    fn config_default_values() {
        let cfg = ViewerConfig::default();
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
        assert!((cfg.initial_zoom - 1.0).abs() < 1e-10);
        assert!(!cfg.show_stats);
        assert!(!cfg.xray_mode);
        assert!(cfg.initial_file.is_none());
    }

    #[test]
    fn config_for_fractal() {
        let cfg = ViewerConfig::for_fractal();
        assert!((cfg.initial_zoom - 1.0).abs() < 1e-10);
        assert!((cfg.initial_pan[0] - (-0.5)).abs() < 1e-10);
    }

    #[test]
    fn config_minimal() {
        let cfg = ViewerConfig::minimal();
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
    }

    #[test]
    fn config_for_sdf_file() {
        let cfg = ViewerConfig::for_sdf_file("/tmp/test.asdf");
        assert!(cfg.title.contains("test.asdf"));
        assert_eq!(cfg.initial_file.as_deref(), Some("/tmp/test.asdf"));
    }

    // ── ViewerState ─────────────────────────────────────────────────

    #[test]
    fn viewer_state_defaults() {
        let state = ViewerState::new(RenderMode::Procedural2D, false);
        assert!((state.zoom - 1.0).abs() < 1e-10);
        assert!(!state.paused);
        assert!(!state.xray_mode);
        assert!(!state.show_stats);
        assert_eq!(state.render_mode, RenderMode::Procedural2D);
    }

    #[test]
    fn viewer_state_3d_mode() {
        let state = ViewerState::new(RenderMode::Sdf3D, true);
        assert_eq!(state.render_mode, RenderMode::Sdf3D);
        assert!(state.show_stats);
        assert_eq!(state.sdf_max_steps, 128);
        assert!((state.sdf_epsilon - 0.001).abs() < 1e-10);
    }

    // ── RenderMode / XRayType ───────────────────────────────────────

    #[test]
    fn render_mode_default_is_2d() {
        assert_eq!(RenderMode::default(), RenderMode::Procedural2D);
    }

    #[test]
    fn xray_type_default_is_motion_vectors() {
        assert_eq!(XRayType::default(), XRayType::MotionVectors);
    }

    // ── QualityPreset ───────────────────────────────────────────

    #[test]
    fn quality_preset_default_is_balanced() {
        assert_eq!(QualityPreset::default(), QualityPreset::Balanced);
    }

    #[test]
    fn quality_preset_names() {
        assert_eq!(QualityPreset::Fast.name(), "Fast");
        assert_eq!(QualityPreset::Balanced.name(), "Balanced");
        assert_eq!(QualityPreset::Quality.name(), "Quality");
        assert_eq!(QualityPreset::Ultra.name(), "Ultra");
    }

    #[test]
    fn quality_preset_all_count() {
        assert_eq!(QualityPreset::all().len(), 4);
    }

    #[test]
    fn apply_quality_preset_fast() {
        let mut state = ViewerState::new(RenderMode::Sdf3D, false);
        state.apply_quality_preset(QualityPreset::Fast);
        assert_eq!(state.sdf_max_steps, 64);
        assert!((state.sdf_epsilon - 0.01).abs() < 1e-10);
        assert!(!state.sdf_ambient_occlusion);
        assert_eq!(state.sdf_quality_preset, QualityPreset::Fast);
    }

    #[test]
    fn apply_quality_preset_balanced() {
        let mut state = ViewerState::new(RenderMode::Sdf3D, false);
        state.apply_quality_preset(QualityPreset::Fast); // change first
        state.apply_quality_preset(QualityPreset::Balanced);
        assert_eq!(state.sdf_max_steps, 128);
        assert!((state.sdf_epsilon - 0.001).abs() < 1e-10);
        assert!(state.sdf_ambient_occlusion);
    }

    #[test]
    fn apply_quality_preset_quality() {
        let mut state = ViewerState::new(RenderMode::Sdf3D, false);
        state.apply_quality_preset(QualityPreset::Quality);
        assert_eq!(state.sdf_max_steps, 256);
        assert!((state.sdf_epsilon - 0.0001).abs() < 1e-10);
        assert!(state.sdf_ambient_occlusion);
    }

    #[test]
    fn apply_quality_preset_ultra() {
        let mut state = ViewerState::new(RenderMode::Sdf3D, false);
        state.apply_quality_preset(QualityPreset::Ultra);
        assert_eq!(state.sdf_max_steps, 512);
        assert!((state.sdf_epsilon - 0.00001).abs() < 1e-10);
        assert!(state.sdf_ambient_occlusion);
    }

    #[test]
    fn viewer_state_adaptive_quality_default() {
        let state = ViewerState::new(RenderMode::Sdf3D, false);
        assert!(state.sdf_adaptive_quality);
        assert_eq!(state.sdf_quality_preset, QualityPreset::Balanced);
    }
}
