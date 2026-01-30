// ALICE-View X-Ray Visualization Shader
// See the math behind the pixels

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    zoom: f32,
    pan: vec2<f32>,
    xray_type: u32,  // 0=MotionVectors, 1=FFTHeatmap, 2=EquationOverlay, 3=Wireframe
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var base_texture: texture_2d<f32>;
@group(0) @binding(2) var base_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Full-screen triangle technique
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Magic formula for full-screen triangle
    let x = f32((i32(vertex_index) << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vertex_index) & 2) * 2.0 - 1.0;

    out.position = vec4<f32>(x, -y, 0.0, 1.0); // Y inverted for screen coords
    out.uv = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);

    return out;
}

// ============================================
// Helper Functions
// ============================================

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn grad_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let a = hash2(i + vec2<f32>(0.0, 0.0));
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Compute gradient of noise field
fn compute_gradient(p: vec2<f32>) -> vec2<f32> {
    let eps = 0.01;
    let dx = grad_noise(p + vec2<f32>(eps, 0.0)) - grad_noise(p - vec2<f32>(eps, 0.0));
    let dy = grad_noise(p + vec2<f32>(0.0, eps)) - grad_noise(p - vec2<f32>(0.0, eps));
    return vec2<f32>(dx, dy) / (2.0 * eps);
}

// ============================================
// X-Ray Modes
// ============================================

// Motion Vectors visualization
fn xray_motion_vectors(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;
    let scale = uniforms.param1;

    // Compute motion vectors from noise field
    let motion = compute_gradient(world_pos * scale + uniforms.time * 0.5);

    // Normalize and visualize
    let magnitude = length(motion);
    let direction = motion / max(magnitude, 0.001);

    // Green = horizontal, Red = vertical
    let r = abs(direction.y) * 0.8 + 0.2;
    let g = abs(direction.x) * 0.8 + 0.2;
    let b = magnitude * 2.0;

    // Draw arrows at grid points
    let grid_size = 0.1 / uniforms.zoom;
    let grid_pos = fract(world_pos / grid_size);
    let grid_center = abs(grid_pos - 0.5);

    // Arrow head
    let arrow_dir = normalize(motion);
    let arrow_tip = grid_pos - 0.5;
    let along_arrow = dot(arrow_tip, arrow_dir);
    let perp_arrow = abs(dot(arrow_tip, vec2<f32>(-arrow_dir.y, arrow_dir.x)));

    var arrow = 0.0;
    if (along_arrow > 0.0 && along_arrow < 0.4 && perp_arrow < 0.05) {
        arrow = 1.0;
    }
    // Arrow head
    if (along_arrow > 0.3 && along_arrow < 0.45 && perp_arrow < (0.45 - along_arrow) * 2.0) {
        arrow = 1.0;
    }

    var color = vec3<f32>(r, g, b) * 0.5;
    color += vec3<f32>(1.0, 1.0, 1.0) * arrow * 0.8;

    return color;
}

// FFT Heatmap visualization
fn xray_fft_heatmap(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;

    // Simulate frequency domain by computing local frequency content
    var freq_sum = 0.0;
    let sample_radius = 0.05 / uniforms.zoom;

    // Sample noise at different frequencies
    for (var i = 0; i < 8; i++) {
        let freq = pow(2.0, f32(i));
        let sample = grad_noise(world_pos * freq * uniforms.param1);
        freq_sum += sample * (1.0 / freq);
    }

    freq_sum = freq_sum * 0.5 + 0.5;

    // High frequency = bright, low frequency = dark
    // Heat map coloring
    var color: vec3<f32>;
    if (freq_sum < 0.25) {
        color = mix(vec3<f32>(0.0, 0.0, 0.2), vec3<f32>(0.0, 0.0, 1.0), freq_sum * 4.0);
    } else if (freq_sum < 0.5) {
        color = mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 1.0), (freq_sum - 0.25) * 4.0);
    } else if (freq_sum < 0.75) {
        color = mix(vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 0.0), (freq_sum - 0.5) * 4.0);
    } else {
        color = mix(vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), (freq_sum - 0.75) * 4.0);
    }

    return color;
}

// Equation Overlay
fn xray_equation_overlay(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;

    // Base noise visualization
    let scale = uniforms.param1;
    let octaves = i32(uniforms.param2);

    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = world_pos * scale;

    for (var i = 0; i < 6; i++) {
        if (i >= octaves) { break; }
        value += amplitude * grad_noise(pos * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    // Contour lines
    let contour_spacing = 0.1;
    let contour = fract(value / contour_spacing);
    let contour_line = smoothstep(0.0, 0.02, contour) * (1.0 - smoothstep(0.98, 1.0, contour));

    // Base color
    var color = vec3<f32>(0.1, 0.15, 0.2);

    // Add contour lines in cyan
    color = mix(color, vec3<f32>(0.0, 0.8, 1.0), 1.0 - contour_line);

    // Grid overlay
    let grid_major = 1.0;
    let grid_minor = 0.25;

    let major_x = abs(fract(world_pos.x / grid_major + 0.5) - 0.5) * grid_major / uniforms.zoom;
    let major_y = abs(fract(world_pos.y / grid_major + 0.5) - 0.5) * grid_major / uniforms.zoom;
    let major_line = min(major_x, major_y);

    let minor_x = abs(fract(world_pos.x / grid_minor + 0.5) - 0.5) * grid_minor / uniforms.zoom;
    let minor_y = abs(fract(world_pos.y / grid_minor + 0.5) - 0.5) * grid_minor / uniforms.zoom;
    let minor_line = min(minor_x, minor_y);

    // Draw grid
    if (major_line < 0.002) {
        color = mix(color, vec3<f32>(0.4, 0.4, 0.5), 0.5);
    } else if (minor_line < 0.001) {
        color = mix(color, vec3<f32>(0.3, 0.3, 0.35), 0.3);
    }

    // Axes
    if (abs(world_pos.x) < 0.005 / uniforms.zoom) {
        color = vec3<f32>(0.8, 0.2, 0.2); // Y-axis red
    }
    if (abs(world_pos.y) < 0.005 / uniforms.zoom) {
        color = vec3<f32>(0.2, 0.8, 0.2); // X-axis green
    }

    return color;
}

// Wireframe visualization
fn xray_wireframe(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;

    // Create procedural wireframe mesh
    let mesh_scale = uniforms.param1;
    let p = world_pos * mesh_scale;

    // Triangular mesh pattern
    let cell = floor(p);
    let f = fract(p);

    // Triangle edges
    var min_dist = 10.0;

    // Edge 1: bottom
    min_dist = min(min_dist, f.y);

    // Edge 2: left
    min_dist = min(min_dist, f.x);

    // Edge 3: diagonal
    min_dist = min(min_dist, abs(f.x + f.y - 1.0) / 1.414);

    // Create wireframe color
    let wire_width = 0.02;
    let wire = 1.0 - smoothstep(0.0, wire_width, min_dist);

    // Background: subtle grid glow
    let glow = exp(-min_dist * 10.0) * 0.3;

    // Vertex highlights
    let vertex_dist = min(
        min(length(f), length(f - vec2<f32>(1.0, 0.0))),
        min(length(f - vec2<f32>(0.0, 1.0)), length(f - vec2<f32>(1.0, 1.0)))
    );
    let vertex = 1.0 - smoothstep(0.0, 0.05, vertex_dist);

    // Wireframe color: cyan
    var color = vec3<f32>(0.02, 0.05, 0.08);
    color += vec3<f32>(0.0, 1.0, 1.0) * wire;
    color += vec3<f32>(0.0, 0.3, 0.4) * glow;
    color += vec3<f32>(1.0, 1.0, 1.0) * vertex * 0.8;

    // Animate: pulse effect
    let pulse = 0.5 + 0.5 * sin(uniforms.time * 2.0 - length(world_pos) * 3.0);
    color *= 0.7 + 0.3 * pulse;

    return color;
}

// ============================================
// Main Fragment Shader
// ============================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;

    switch (uniforms.xray_type) {
        case 0u: { color = xray_motion_vectors(in.uv); }
        case 1u: { color = xray_fft_heatmap(in.uv); }
        case 2u: { color = xray_equation_overlay(in.uv); }
        case 3u: { color = xray_wireframe(in.uv); }
        default: { color = vec3<f32>(0.0, 0.5, 0.5); }
    }

    // X-Ray scan line effect
    let scan_line = sin(in.uv.y * uniforms.resolution.y * 0.5 + uniforms.time * 5.0) * 0.5 + 0.5;
    color *= 0.95 + 0.05 * scan_line;

    return vec4<f32>(color, 1.0);
}
