//! wgpu-based renderer for procedural content

mod pipeline;
mod infinite_zoom;

pub use pipeline::*;
pub use infinite_zoom::*;

use crate::app::{RenderMode, ViewerState};
use crate::decoder::Decoder;
use crate::ui::Ui;
use anyhow::Result;
use std::sync::Arc;
use wgpu::*;
use winit::{dpi::PhysicalSize, window::Window};
use image::RgbaImage;

/// Main renderer
pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    // 2D procedural pipeline
    procedural_pipeline: ProceduralPipeline,
    // 3D SDF raymarching pipeline
    sdf_pipeline: SdfPipeline,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    start_time: std::time::Instant,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        tracing::info!("GPU: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("ALICE-View Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps.present_modes.contains(&PresentMode::Mailbox) {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        // Create both pipelines
        let procedural_pipeline = ProceduralPipeline::new(&device, surface_format);
        let sdf_pipeline = SdfPipeline::new(&device, surface_format);

        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            &window,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            procedural_pipeline,
            sdf_pipeline,
            egui_renderer,
            egui_state,
            egui_ctx,
            start_time: std::time::Instant::now(),
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Rebuild SDF pipeline with dynamic WGSL shader from .asdf file
    ///
    /// This allows loading arbitrary SDF trees and rendering them in real-time.
    pub fn rebuild_sdf_pipeline_with_wgsl(&mut self, sdf_wgsl: &str) {
        tracing::info!("Rebuilding SDF pipeline with dynamic shader...");
        self.sdf_pipeline = self.sdf_pipeline.rebuild_with_dynamic_sdf(&self.device, sdf_wgsl);
        tracing::info!("SDF pipeline rebuilt successfully");
    }

    /// Check if dynamic SDF is currently loaded
    pub fn has_dynamic_sdf(&self) -> bool {
        self.sdf_pipeline.has_dynamic_sdf()
    }

    /// Capture screenshot of the current frame
    pub fn capture_screenshot(&self) -> Result<()> {
        let width = self.size.width;
        let height = self.size.height;

        // Create a texture to copy into
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("Screenshot Texture"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.config.format,
            usage: TextureUsages::COPY_DST | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;

        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("Screenshot Buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Get current surface texture and copy
        let output = self.surface.get_current_texture()?;
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Screenshot Encoder"),
        });

        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                texture: &output.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            ImageCopyBuffer {
                buffer: &buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d { width, height, depth_or_array_layers: 1 },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Read buffer
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(Maintain::Wait);
        rx.recv()??;

        let data = buffer_slice.get_mapped_range();

        // Remove padding
        let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
        for row in 0..height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + (width * bytes_per_pixel) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();
        output.present();

        // Save to file
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("alice-view_{}.png", timestamp);

        // Try Desktop, then current dir
        let save_path = dirs::desktop_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(&filename);

        if let Some(img) = RgbaImage::from_raw(width, height, pixels) {
            img.save(&save_path)?;
            tracing::info!("Screenshot saved: {}", save_path.display());
        }

        Ok(())
    }

    pub fn render(&mut self, state: &mut ViewerState, decoder: &Decoder, ui: &mut Ui) -> Result<()> {
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture()?
            }
            Err(e) => return Err(e.into()),
        };

        let view = output.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let time = self.start_time.elapsed().as_secs_f32();
        let resolution = [self.size.width as f32, self.size.height as f32];

        // Update appropriate pipeline uniforms based on render mode
        match state.render_mode {
            RenderMode::Procedural2D => {
                self.procedural_pipeline.update_uniforms(&self.queue, state, time, resolution);
            }
            RenderMode::Sdf3D => {
                let scene_id = ui.sdf_scene_id();
                self.sdf_pipeline.update_uniforms(&self.queue, state, time, resolution, scene_id);
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render with appropriate pipeline
            match state.render_mode {
                RenderMode::Procedural2D => {
                    self.procedural_pipeline.render(&mut render_pass, state, decoder);
                }
                RenderMode::Sdf3D => {
                    self.sdf_pipeline.render(&mut render_pass);
                }
            }
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: 1.0,
        };

        let full_output = ui.render(&self.egui_ctx, state);

        let clipped_primitives = self.egui_ctx.tessellate(
            full_output.shapes,
            full_output.pixels_per_point,
        );

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
