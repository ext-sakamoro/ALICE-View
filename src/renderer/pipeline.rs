//! Procedural rendering pipeline

use crate::app::{RenderMode, ViewerState};
use crate::decoder::Decoder;
use wgpu::*;

/// Procedural rendering pipeline
pub struct ProceduralPipeline {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
}

/// Uniforms for procedural shaders
/// WGSL std140 layout requirements:
/// - vec2 requires 8-byte alignment
/// - vec3/vec4 requires 16-byte alignment
/// - struct must be padded to 16-byte boundary
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    resolution: [f32; 2],   // offset 0  (align 8)
    time: f32,              // offset 8  (align 4)
    zoom: f32,              // offset 12 (align 4)
    pan: [f32; 2],          // offset 16 (align 8)
    content_type: u32,      // offset 24 (align 4)
    param1: f32,            // offset 28
    param2: f32,            // offset 32
    param3: f32,            // offset 36
    param4: f32,            // offset 40
    _pad1: u32,             // offset 44 (padding to 48 bytes, 16-byte boundary)
}

impl ProceduralPipeline {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        // Shader module
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Procedural Shader"),
            source: ShaderSource::Wgsl(include_str!("../shaders/procedural.wgsl").into()),
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Procedural Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Uniform buffer
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Procedural Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Procedural Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Procedural Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            uniform_buffer,
            bind_group,
        }
    }

    /// Update uniform buffer with current state
    pub fn update_uniforms(
        &self,
        queue: &Queue,
        state: &ViewerState,
        time: f32,
        resolution: [f32; 2],
    ) {
        let uniforms = Uniforms {
            resolution,
            time,
            zoom: state.zoom,
            pan: state.pan,
            content_type: 0,  // Default to Perlin
            param1: 10.0,     // scale
            param2: 6.0,      // octaves
            param3: 0.0,
            param4: 0.0,
            _pad1: 0,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        _state: &ViewerState,
        _decoder: &Decoder,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

// ============================================
// SDF Raymarching Pipeline (3D)
// ============================================

/// Uniforms for SDF raymarching
/// WGSL std140 layout - using vec4 for proper alignment
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SdfUniforms {
    // Basic uniforms (16 bytes)
    resolution: [f32; 2],   // offset 0
    time: f32,              // offset 8
    _pad0: f32,             // offset 12

    // Camera position as vec4 (16 bytes)
    camera_pos: [f32; 4],   // offset 16 (xyz used, w unused)

    // Camera target + fov as vec4 (16 bytes)
    camera_target: [f32; 4], // offset 32 (xyz = target, w = fov)

    // Camera up as vec4 (16 bytes)
    camera_up: [f32; 4],    // offset 48 (xyz used, w unused)

    // Raymarching settings (16 bytes)
    max_steps: u32,         // offset 64
    max_distance: f32,      // offset 68
    epsilon: f32,           // offset 72
    flags: u32,             // offset 76

    // Scene selection (16 bytes for alignment)
    scene_id: u32,          // offset 80
    _pad1: u32,             // offset 84
    _pad2: u32,             // offset 88
    _pad3: u32,             // offset 92
}

/// Base shader template for raymarching
const RAYMARCHING_TEMPLATE: &str = include_str!("../shaders/raymarching.wgsl");

/// Dynamic SDF placeholder in shader template
const DYNAMIC_SDF_PLACEHOLDER: &str = "// {{DYNAMIC_SDF_FUNCTION}}";

/// SDF Raymarching pipeline with dynamic shader support
pub struct SdfPipeline {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    format: TextureFormat,
    /// Whether dynamic SDF is currently loaded
    has_dynamic_sdf: bool,
}

impl SdfPipeline {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        Self::new_with_shader(device, format, RAYMARCHING_TEMPLATE, false)
    }

    /// Create pipeline with custom shader source
    fn new_with_shader(device: &Device, format: TextureFormat, shader_source: &str, has_dynamic_sdf: bool) -> Self {
        // Shader module
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("SDF Raymarching Shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SDF Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Uniform buffer
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SDF Uniform Buffer"),
            size: std::mem::size_of::<SdfUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("SDF Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("SDF Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("SDF Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
            uniform_buffer,
            bind_group,
            format,
            has_dynamic_sdf,
        }
    }

    /// Rebuild pipeline with dynamic SDF from ALICE-SDF transpiled WGSL
    ///
    /// # Arguments
    /// * `device` - wgpu device
    /// * `sdf_wgsl` - WGSL code for sdf_eval function (from alice_sdf::WgslShader)
    ///
    /// # Returns
    /// New SdfPipeline with dynamic SDF embedded
    pub fn rebuild_with_dynamic_sdf(&self, device: &Device, sdf_wgsl: &str) -> Self {
        // Generate dynamic shader by replacing placeholder
        let dynamic_function = format!(
            "// Dynamic SDF loaded from .asdf file\n\
             fn sdf_eval_dynamic(p: vec3<f32>) -> f32 {{\n\
             {}\n\
             }}",
            Self::convert_sdf_eval_to_dynamic(sdf_wgsl)
        );

        let shader_source = RAYMARCHING_TEMPLATE.replace(
            "// {{DYNAMIC_SDF_FUNCTION}}\n// Default fallback when no .asdf is loaded\nfn sdf_eval_dynamic(p: vec3<f32>) -> f32 {\n    return length(p) - 1.0;  // Simple sphere fallback\n}",
            &dynamic_function,
        );

        tracing::info!("Rebuilt SDF pipeline with dynamic shader ({} bytes)", shader_source.len());

        Self::new_with_shader(device, self.format, &shader_source, true)
    }

    /// Convert sdf_eval function body to sdf_eval_dynamic
    /// The ALICE-SDF transpiler generates `fn sdf_eval(p: vec3<f32>) -> f32 { ... }`
    /// We need to extract the body and rename variables if needed
    fn convert_sdf_eval_to_dynamic(sdf_wgsl: &str) -> String {
        // Find the function body between { and the last }
        // The transpiler output looks like:
        // fn sdf_eval(p: vec3<f32>) -> f32 {
        //     let d0 = ...;
        //     return d0;
        // }

        // Extract content between first { and last }
        if let Some(start) = sdf_wgsl.find('{') {
            if let Some(end) = sdf_wgsl.rfind('}') {
                let body = &sdf_wgsl[start + 1..end];
                return body.trim().to_string();
            }
        }

        // Fallback: return the whole thing and hope it works
        sdf_wgsl.to_string()
    }

    /// Check if dynamic SDF is loaded
    pub fn has_dynamic_sdf(&self) -> bool {
        self.has_dynamic_sdf
    }

    /// Update uniform buffer with current state
    pub fn update_uniforms(
        &self,
        queue: &Queue,
        state: &ViewerState,
        time: f32,
        resolution: [f32; 2],
        scene_id: u32,
    ) {
        let camera = &state.camera;

        // Build flags bitfield
        let mut flags = 0u32;
        if state.sdf_show_normals {
            flags |= 1;
        }
        if state.sdf_ambient_occlusion {
            flags |= 2;
        }

        // Pack camera data into vec4s for proper WGSL alignment
        let pos = camera.position;
        let target = camera.target;
        let up = camera.up;

        let uniforms = SdfUniforms {
            resolution,
            time,
            _pad0: 0.0,

            camera_pos: [pos.x, pos.y, pos.z, 0.0],
            camera_target: [target.x, target.y, target.z, camera.fov], // w = fov
            camera_up: [up.x, up.y, up.z, 0.0],

            max_steps: state.sdf_max_steps,
            max_distance: camera.far,
            epsilon: state.sdf_epsilon,
            flags,

            scene_id,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
