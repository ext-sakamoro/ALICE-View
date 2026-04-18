//! WASM entry point for browser-based ALICE-View
//!
//! Provides a WebGPU-based SDF viewer that runs in a browser canvas.
//! Uses the same WGSL shaders and SdfPipeline as the native version.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::app::{Camera3D, ViewerState};
#[cfg(target_arch = "wasm32")]
use crate::renderer::SdfPipeline;

/// Initialize ALICE-View in a browser canvas element.
///
/// Call from JavaScript:
/// ```js
/// import init, { alice_view_init } from './alice_view_wasm.js';
/// await init();
/// const handle = await alice_view_init("canvas-id");
/// // Render loop
/// function frame() {
///   handle.render_frame(1.0 / 60.0);
///   requestAnimationFrame(frame);
/// }
/// frame();
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn alice_view_init(canvas_id: &str) -> Result<AliceViewHandle, JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("canvas not found")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "element is not a canvas")?;

    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("surface: {e}")))?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or("no adapter")?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("ALICE-View WASM"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("device: {e}")))?;

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let sdf_pipeline = SdfPipeline::new(&device, format);

    let mut viewer_state = ViewerState::default();
    viewer_state.camera = Camera3D {
        position: glam::Vec3::new(0.0, 2.0, 5.0),
        target: glam::Vec3::ZERO,
        ..Camera3D::default()
    };

    Ok(AliceViewHandle {
        _canvas: canvas,
        surface,
        device,
        queue,
        config,
        sdf_pipeline,
        viewer_state,
        time: 0.0,
    })
}

/// Handle to a running ALICE-View instance in the browser.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct AliceViewHandle {
    _canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    sdf_pipeline: SdfPipeline,
    viewer_state: ViewerState,
    time: f32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl AliceViewHandle {
    /// Update the SDF scene with a WGSL shader string generated from LOL DSL.
    pub fn set_sdf_shader(&mut self, wgsl_source: &str) {
        self.sdf_pipeline = self
            .sdf_pipeline
            .rebuild_with_dynamic_sdf(&self.device, wgsl_source);
    }

    /// Render a single frame.
    pub fn render_frame(&mut self, dt: f32) {
        self.time += dt;

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let resolution = [self.config.width as f32, self.config.height as f32];

        self.sdf_pipeline.update_uniforms(
            &self.queue,
            &self.viewer_state,
            self.time,
            resolution,
            0,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ALICE-View WASM Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SDF Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            self.sdf_pipeline.render(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    /// Resize the viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Orbit camera by delta angles (radians).
    pub fn orbit(&mut self, d_theta: f32, d_phi: f32) {
        self.viewer_state.camera.orbit(d_theta, d_phi);
    }

    /// Zoom camera (dolly).
    pub fn zoom(&mut self, delta: f32) {
        self.viewer_state.camera.dolly(delta);
    }
}
