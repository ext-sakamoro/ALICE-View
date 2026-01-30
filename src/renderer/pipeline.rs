//! Procedural rendering pipeline

use crate::app::ViewerState;
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
