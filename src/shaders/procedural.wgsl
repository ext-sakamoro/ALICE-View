// ALICE-View Procedural Generation Shader
// "Store equations, not pixels" - GPU-computed infinite resolution

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    zoom: f32,
    pan: vec2<f32>,
    content_type: u32,  // 0=Perlin, 1=Polynomial, 2=Fractal, 3=Gradient, 4=Voronoi
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Full-screen triangle technique:
// Generates 3 vertices that form a triangle covering the entire screen.
// Vertices: (-1, -1), (3, -1), (-1, 3) - this oversized triangle covers the [0,1] UV space
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
// Noise Functions
// ============================================

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(hash2(i + vec2<f32>(0.0, 0.0)), hash2(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// Perlin-like gradient noise
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

// Fractal Brownian Motion
fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i++) {
        value += amplitude * grad_noise(pos * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    return value;
}

// ============================================
// Content Type Generators
// ============================================

// Perlin noise with octaves
fn generate_perlin(uv: vec2<f32>) -> vec3<f32> {
    let scale = uniforms.param1;
    let octaves = i32(uniforms.param2);

    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;
    let n = fbm(world_pos * scale, octaves);

    // Color gradient based on noise value
    let low = vec3<f32>(0.1, 0.2, 0.4);
    let mid = vec3<f32>(0.3, 0.6, 0.3);
    let high = vec3<f32>(0.9, 0.8, 0.6);

    if (n < 0.5) {
        return mix(low, mid, n * 2.0);
    } else {
        return mix(mid, high, (n - 0.5) * 2.0);
    }
}

// Polynomial surface visualization
fn generate_polynomial(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;
    let x = world_pos.x;
    let y = world_pos.y;

    // Coefficients from params
    let a = uniforms.param1;
    let b = uniforms.param2;
    let c = uniforms.param3;
    let d = uniforms.param4;

    // z = ax³ + by² + cxy + d
    let z = a * x * x * x + b * y * y + c * x * y + d;

    // Height-based coloring
    let normalized = clamp(z * 0.5 + 0.5, 0.0, 1.0);

    let color1 = vec3<f32>(0.2, 0.1, 0.5);
    let color2 = vec3<f32>(0.1, 0.8, 0.6);
    let color3 = vec3<f32>(1.0, 0.9, 0.3);

    if (normalized < 0.5) {
        return mix(color1, color2, normalized * 2.0);
    } else {
        return mix(color2, color3, (normalized - 0.5) * 2.0);
    }
}

// Mandelbrot fractal
fn generate_fractal(uv: vec2<f32>) -> vec3<f32> {
    let max_iter = i32(uniforms.param1);
    let escape_radius = uniforms.param2;

    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;
    let c = vec2<f32>(world_pos.x * 3.0 - 0.5, world_pos.y * 3.0);

    var z = vec2<f32>(0.0, 0.0);
    var i = 0;

    for (var iter = 0; iter < 256; iter++) {
        if (iter >= max_iter) { break; }
        if (dot(z, z) > escape_radius * escape_radius) { break; }

        z = vec2<f32>(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        i++;
    }

    if (i >= max_iter) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }

    // Smooth coloring
    let t = f32(i) / f32(max_iter);
    let hue = t * 6.28318;

    return vec3<f32>(
        0.5 + 0.5 * cos(hue),
        0.5 + 0.5 * cos(hue + 2.094),
        0.5 + 0.5 * cos(hue + 4.188)
    );
}

// Gradient field
fn generate_gradient(uv: vec2<f32>) -> vec3<f32> {
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;

    // Radial + directional gradient
    let radial = length(world_pos);
    let angle = atan2(world_pos.y, world_pos.x);

    let r = 0.5 + 0.5 * sin(radial * uniforms.param1 + uniforms.time);
    let g = 0.5 + 0.5 * cos(angle * uniforms.param2);
    let b = 0.5 + 0.5 * sin(radial * uniforms.param3 + angle * uniforms.param4);

    return vec3<f32>(r, g, b);
}

// Voronoi cells
fn generate_voronoi(uv: vec2<f32>) -> vec3<f32> {
    let scale = uniforms.param1;
    let world_pos = (uv - 0.5) / uniforms.zoom + uniforms.pan;
    let p = world_pos * scale;

    let n = floor(p);
    let f = fract(p);

    var min_dist = 10.0;
    var min_point = vec2<f32>(0.0);

    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let neighbor = vec2<f32>(f32(i), f32(j));
            let point = vec2<f32>(
                hash2(n + neighbor),
                hash2(n + neighbor + vec2<f32>(57.0, 113.0))
            );

            let diff = neighbor + point - f;
            let dist = length(diff);

            if (dist < min_dist) {
                min_dist = dist;
                min_point = point;
            }
        }
    }

    // Color based on cell
    let cell_color = vec3<f32>(
        hash2(n + min_point * 100.0),
        hash2(n + min_point * 100.0 + vec2<f32>(17.0, 31.0)),
        hash2(n + min_point * 100.0 + vec2<f32>(73.0, 89.0))
    );

    // Add edge highlight
    let edge = smoothstep(0.0, 0.1, min_dist);

    return cell_color * edge;
}

// ============================================
// Main Fragment Shader
// ============================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;

    switch (uniforms.content_type) {
        case 0u: { color = generate_perlin(in.uv); }
        case 1u: { color = generate_polynomial(in.uv); }
        case 2u: { color = generate_fractal(in.uv); }
        case 3u: { color = generate_gradient(in.uv); }
        case 4u: { color = generate_voronoi(in.uv); }
        default: { color = vec3<f32>(0.5, 0.5, 0.5); }
    }

    // Subtle vignette
    let vignette = 1.0 - length(in.uv - 0.5) * 0.5;
    color *= vignette;

    return vec4<f32>(color, 1.0);
}
