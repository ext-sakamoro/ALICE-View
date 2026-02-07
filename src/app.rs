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
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Get right vector (normalized)
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    /// Orbit around target (spherical coordinates)
    pub fn orbit(&mut self, delta_theta: f32, delta_phi: f32) {
        let radius = (self.position - self.target).length();
        let offset = self.position - self.target;

        // Current spherical coordinates
        let mut theta = offset.z.atan2(offset.x);
        let mut phi = (offset.y / radius).acos();

        // Apply rotation
        theta += delta_theta;
        phi = (phi + delta_phi).clamp(0.01, std::f32::consts::PI - 0.01);

        // Convert back to Cartesian
        self.position = self.target + Vec3::new(
            radius * phi.sin() * theta.cos(),
            radius * phi.cos(),
            radius * phi.sin() * theta.sin(),
        );
    }

    /// Dolly (move along view direction)
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
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let right = self.right();
        let up = self.up;
        let offset = right * delta_x + up * delta_y;
        self.position += offset;
        self.target += offset;
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

    // Lighting
    pub light_dir: [f32; 3],
    pub light_intensity: f32,
    pub ambient_intensity: f32,
    pub bg_color: [f32; 3],

    // Screenshot request
    pub screenshot_requested: bool,
}

impl ViewerState {
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
            light_dir: [0.5, 1.0, 0.3],
            light_intensity: 1.0,
            ambient_intensity: 0.15,
            bg_color: [0.02, 0.02, 0.05],
            screenshot_requested: false,
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
    pub gpu_usage: f32,
    pub resolution: String,
}

/// Viewer configuration for library usage
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

impl App {
    pub fn new(initial_file: Option<String>) -> Self {
        // Auto-detect render mode from file extension
        let render_mode = initial_file.as_ref()
            .map(|f| {
                if f.ends_with(".asdf") || f.ends_with(".asdf.json") || f.ends_with(".json") {
                    RenderMode::Sdf3D
                } else {
                    RenderMode::Procedural2D
                }
            })
            .unwrap_or(RenderMode::Procedural2D);

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
    pub fn with_config(config: ViewerConfig) -> Self {
        // Auto-detect render mode from file extension
        let render_mode = config.initial_file.as_ref()
            .map(|f| {
                if f.ends_with(".asdf") || f.ends_with(".asdf.json") || f.ends_with(".json") {
                    RenderMode::Sdf3D
                } else {
                    RenderMode::Procedural2D
                }
            })
            .unwrap_or(RenderMode::Procedural2D);

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
    pub fn init(&mut self, target: &EventLoopWindowTarget<()>) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .with_title(&self.config.title)
                .with_inner_size(PhysicalSize::new(self.config.width, self.config.height))
                .build(target)
                .unwrap()
        );

        // Initialize renderer
        self.renderer = Some(pollster::block_on(Renderer::new(window.clone())).unwrap());

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

    /// Main event handling logic (winit 0.29 style)
    pub fn handle_event(&mut self, event: Event<()>, target: &EventLoopWindowTarget<()>) {
        // Handle UI events first
        if let (Some(renderer), Event::WindowEvent { event: ref w_event, .. }) = (&mut self.renderer, &event) {
            let response = self.ui.handle_event(w_event, renderer.egui_ctx());
            if response.consumed {
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
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                    ..
                } => {
                    self.handle_key(key, state == ElementState::Pressed);
                    // Request redraw to reflect state changes
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                    };
                    self.handle_scroll(scroll);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                // Mouse button press/release
                WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => {
                    self.mouse_pressed = state == ElementState::Pressed;
                }
                // Mouse movement (drag to pan/orbit)
                WindowEvent::CursorMoved { position, .. } => {
                    if self.mouse_pressed {
                        if let Some(last_pos) = self.last_mouse_pos {
                            let dx = (position.x - last_pos.x) as f32;
                            let dy = (position.y - last_pos.y) as f32;

                            match self.state.render_mode {
                                RenderMode::Procedural2D => {
                                    // 2D: Scale movement by zoom level
                                    let sensitivity = 0.002 / self.state.zoom;
                                    self.state.pan[0] -= dx * sensitivity;
                                    self.state.pan[1] += dy * sensitivity;
                                }
                                RenderMode::Sdf3D => {
                                    // 3D: Orbit camera around target
                                    let orbit_sensitivity = 0.01;
                                    self.state.camera.orbit(
                                        -dx * orbit_sensitivity,
                                        dy * orbit_sensitivity,
                                    );
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

                        let renderer = self.renderer.as_mut().unwrap();

                        // Check for pending WGSL shader from loaded .asdf file
                        if let Some(wgsl) = self.ui.take_pending_wgsl() {
                            renderer.rebuild_sdf_pipeline_with_wgsl(&wgsl);
                        }

                        if let Err(e) = renderer.render(&mut self.state, &self.decoder, &mut self.ui) {
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
