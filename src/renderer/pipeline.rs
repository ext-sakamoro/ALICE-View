//! Procedural rendering pipeline

use crate::app::ViewerState;
use crate::decoder::Decoder;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferBindingType, BufferDescriptor,
    BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, FrontFace,
    MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, TextureFormat, VertexState,
};

/// Procedural rendering pipeline
pub struct ProceduralPipeline {
    render_pipeline: RenderPipeline,
    // Stored to allow future dynamic bind group rebuilds (e.g. texture updates).
    #[allow(dead_code)]
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
    resolution: [f32; 2], // offset 0  (align 8)
    time: f32,            // offset 8  (align 4)
    zoom: f32,            // offset 12 (align 4)
    pan: [f32; 2],        // offset 16 (align 8)
    content_type: u32,    // offset 24 (align 4)
    param1: f32,          // offset 28
    param2: f32,          // offset 32
    param3: f32,          // offset 36
    param4: f32,          // offset 40
    _pad1: u32,           // offset 44 (padding to 48 bytes, 16-byte boundary)
}

impl ProceduralPipeline {
    #[must_use]
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
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
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
            cache: None,
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
            content_type: 0, // Default to Perlin
            param1: 10.0,    // scale
            param2: 6.0,     // octaves
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
    resolution: [f32; 2], // offset 0
    time: f32,            // offset 8
    _pad0: f32,           // offset 12

    // Camera position as vec4 (16 bytes)
    camera_pos: [f32; 4], // offset 16 (xyz used, w unused)

    // Camera target + fov as vec4 (16 bytes)
    camera_target: [f32; 4], // offset 32 (xyz = target, w = fov)

    // Camera up as vec4 (16 bytes)
    camera_up: [f32; 4], // offset 48 (xyz used, w unused)

    // Raymarching settings (16 bytes)
    max_steps: u32,    // offset 64
    max_distance: f32, // offset 68
    epsilon: f32,      // offset 72
    flags: u32,        // offset 76

    // Scene selection (16 bytes for alignment)
    scene_id: u32,          // offset 80
    light_intensity: f32,   // offset 84
    ambient_intensity: f32, // offset 88
    quality_flags: u32,     // offset 92 — bit 0: adaptive quality

    // Lighting direction + bg color (32 bytes)
    light_dir: [f32; 4], // offset 96  (xyz = dir, w = unused)
    bg_color: [f32; 4],  // offset 112 (xyz = color, w = unused)
}

/// Base shader template for raymarching
const RAYMARCHING_TEMPLATE: &str = include_str!("../shaders/raymarching.wgsl");

/// Dynamic SDF placeholder in shader template
// Used conceptually to document the replacement pattern; actual replacement
// uses the full multi-line string literal in rebuild_with_dynamic_sdf().
#[allow(dead_code)]
const DYNAMIC_SDF_PLACEHOLDER: &str = "// {{DYNAMIC_SDF_FUNCTION}}";

/// SDF Raymarching pipeline with dynamic shader support
pub struct SdfPipeline {
    render_pipeline: RenderPipeline,
    // Stored to allow future dynamic bind group rebuilds.
    #[allow(dead_code)]
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    format: TextureFormat,
    /// Whether dynamic SDF is currently loaded
    // Exposed via has_dynamic_sdf() for renderer-level queries.
    #[allow(dead_code)]
    has_dynamic_sdf: bool,
}

impl SdfPipeline {
    #[must_use]
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        Self::new_with_shader(device, format, RAYMARCHING_TEMPLATE, false)
    }

    /// Create pipeline with custom shader source
    fn new_with_shader(
        device: &Device,
        format: TextureFormat,
        shader_source: &str,
        has_dynamic_sdf: bool,
    ) -> Self {
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
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
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
            cache: None,
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
    /// * `sdf_wgsl` - WGSL code for `sdf_eval` function (from `alice_sdf::WgslShader`)
    ///
    /// # Returns
    /// New `SdfPipeline` with dynamic SDF embedded
    #[must_use]
    pub fn rebuild_with_dynamic_sdf(&self, device: &Device, sdf_wgsl: &str) -> Self {
        // ヘルパー、sdf_eval本体、マテリアル関数を分離
        let (helpers, body, material_fn) = Self::split_helpers_body_material(sdf_wgsl);

        // マテリアル関数がある場合は sdf_material_dynamic も生成
        let material_section = if material_fn.is_empty() {
            "fn sdf_material_dynamic(p: vec3<f32>) -> f32 { return 0.0; }".to_string()
        } else {
            // sdf_eval_material → sdf_material_dynamic にリネーム
            let renamed = material_fn.replace("sdf_eval_material", "sdf_material_dynamic");
            format!("{}\n", renamed)
        };

        // ヘルパー + sdf_eval_dynamic + sdf_material_dynamic を結合
        let dynamic_function = format!(
            "{helpers}\n\
             // Dynamic SDF loaded from file\n\
             fn sdf_eval_dynamic(p: vec3<f32>) -> f32 {{\n\
             {body}\n\
             }}\n\n\
             {material_section}",
            helpers = helpers,
            body = body,
            material_section = material_section,
        );

        let shader_source = RAYMARCHING_TEMPLATE.replace(
            "// {{DYNAMIC_SDF_FUNCTION}}\n// Default fallback when no .asdf is loaded\nfn sdf_eval_dynamic(p: vec3<f32>) -> f32 {\n    return length(p) - 1.0;  // Simple sphere fallback\n}\nfn sdf_material_dynamic(p: vec3<f32>) -> f32 {\n    return 0.0;\n}",
            &dynamic_function,
        );

        tracing::info!(
            "Rebuilt SDF pipeline with dynamic shader ({} bytes)",
            shader_source.len()
        );

        Self::new_with_shader(device, self.format, &shader_source, true)
    }

    /// WGSL ソースからヘルパー関数群と sdf_eval 本体を分離する。
    ///
    /// トランスパイラ出力は以下の形式:
    /// ```wgsl
    /// fn sdf_diamond(...) { ... }   // ← ヘルパー (0個以上)
    /// fn sdf_eval(p: vec3<f32>) -> f32 {
    ///     ...                        // ← 本体
    /// }
    /// ```
    ///
    /// ヘルパーはテンプレートのプレースホルダー上方に挿入し、
    /// 本体のみ `sdf_eval_dynamic` に詰め替える。
    /// (ヘルパー関数群, sdf_eval本体, マテリアル関数群) を返す
    fn split_helpers_body_material(sdf_wgsl: &str) -> (String, String, String) {
        // "fn sdf_eval(" の位置でヘルパーと本体を分離
        if let Some(eval_pos) = sdf_wgsl.find("fn sdf_eval(") {
            let helpers = sdf_wgsl[..eval_pos].trim().to_string();
            let after_helpers = &sdf_wgsl[eval_pos..];

            // sdf_eval 関数の終わり（最初の "}\n" の直後）を見つける
            // sdf_eval の本体を抽出
            let mut brace_depth = 0i32;
            let mut eval_end = after_helpers.len();
            let mut found_start = false;
            for (i, c) in after_helpers.char_indices() {
                if c == '{' {
                    brace_depth += 1;
                    found_start = true;
                } else if c == '}' {
                    brace_depth -= 1;
                    if found_start && brace_depth == 0 {
                        eval_end = i + 1;
                        break;
                    }
                }
            }

            let eval_fn = &after_helpers[..eval_end];
            let material_section = after_helpers[eval_end..].trim().to_string();

            // sdf_eval の本体を抽出
            if let Some(start) = eval_fn.find('{') {
                let body = eval_fn[start + 1..eval_end - 1].trim().to_string();
                return (helpers, body, material_section);
            }

            (helpers, eval_fn.to_string(), material_section)
        } else {
            (String::new(), sdf_wgsl.to_string(), String::new())
        }
    }

    /// Check if dynamic SDF is loaded
    // Called by Renderer::has_dynamic_sdf() for external queries.
    #[allow(dead_code)]
    pub const fn has_dynamic_sdf(&self) -> bool {
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

        // Build quality_flags bitfield
        let mut quality_flags = 0u32;
        if state.sdf_adaptive_quality {
            quality_flags |= 1;
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
            light_intensity: state.light_intensity,
            ambient_intensity: state.ambient_intensity,
            quality_flags,

            light_dir: [
                state.light_dir[0],
                state.light_dir[1],
                state.light_dir[2],
                0.0,
            ],
            bg_color: [state.bg_color[0], state.bg_color[1], state.bg_color[2], 1.0],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
