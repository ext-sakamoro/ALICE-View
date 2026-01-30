//! Main application state and event handling (winit 0.29 compat)

use crate::decoder::Decoder;
use crate::renderer::Renderer;
use crate::ui::Ui;
use std::sync::Arc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};


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
    pub zoom: f32,
    pub pan: [f32; 2],
    pub xray_mode: bool,
    pub xray_type: XRayType,
    pub show_stats: bool,
    pub paused: bool,
    pub stats: FrameStats,
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
        Self {
            window: None,
            renderer: None,
            ui: Ui::new(),
            decoder: Decoder::new(),
            state: ViewerState {
                zoom: 1.0,
                pan: [0.0, 0.0],
                xray_mode: false,
                xray_type: XRayType::default(),
                show_stats: false,
                paused: false,
                stats: FrameStats {
                    fps: 0.0,
                    decode_speed: 0.0,
                    compression_ratio: 1.0,
                    gpu_usage: 0.0,
                    resolution: "∞ (Procedural)".to_string(),
                },
            },
            initial_file,
            mouse_pressed: false,
            last_mouse_pos: None,
            config: ViewerConfig::default(),
        }
    }

    /// Create App with custom configuration (for library usage)
    pub fn with_config(config: ViewerConfig) -> Self {
        Self {
            window: None,
            renderer: None,
            ui: Ui::new(),
            decoder: Decoder::new(),
            state: ViewerState {
                zoom: config.initial_zoom,
                pan: config.initial_pan,
                xray_mode: config.xray_mode,
                xray_type: config.xray_type,
                show_stats: config.show_stats,
                paused: config.paused,
                stats: FrameStats {
                    fps: 0.0,
                    decode_speed: 0.0,
                    compression_ratio: 1.0,
                    gpu_usage: 0.0,
                    resolution: "∞ (Procedural)".to_string(),
                },
            },
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

        match key {
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
        let zoom_factor = 1.1f32;
        if delta > 0.0 {
            self.state.zoom *= zoom_factor;
        } else {
            self.state.zoom /= zoom_factor;
        }
        self.state.zoom = self.state.zoom.clamp(0.001, 1_000_000.0);
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
                // Mouse movement (drag to pan)
                WindowEvent::CursorMoved { position, .. } => {
                    if self.mouse_pressed {
                        if let Some(last_pos) = self.last_mouse_pos {
                            let dx = (position.x - last_pos.x) as f32;
                            let dy = (position.y - last_pos.y) as f32;

                            // Scale movement by zoom level (higher zoom = finer movement)
                            let sensitivity = 0.002 / self.state.zoom;
                            self.state.pan[0] -= dx * sensitivity;
                            self.state.pan[1] += dy * sensitivity; // Y-axis inverted

                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                    self.last_mouse_pos = Some(position);
                }
                WindowEvent::RedrawRequested => {
                    if self.window.is_some() && self.renderer.is_some() {
                        self.ui.update(&mut self.state, &mut self.decoder);

                        let renderer = self.renderer.as_mut().unwrap();
                        if let Err(e) = renderer.render(&mut self.state, &self.decoder, &mut self.ui) {
                            tracing::error!("Render error: {}", e);
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
