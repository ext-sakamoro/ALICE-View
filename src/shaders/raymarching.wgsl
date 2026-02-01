// ALICE-View 3D SDF Raymarching Shader
// "Store equations, not polygons" - GPU-accelerated raymarching
// Author: Moroya Sakamoto

// WGSL std140 layout: vec3 takes 16 bytes (aligned to 16), so use vec4 for safety
struct Uniforms {
    // Basic (16 bytes)
    resolution: vec2<f32>,  // offset 0
    time: f32,              // offset 8
    _pad0: f32,             // offset 12

    // Camera position (16 bytes) - use vec4, w unused
    camera_pos: vec4<f32>,  // offset 16

    // Camera target + fov (16 bytes)
    camera_target: vec4<f32>, // offset 32 (w = fov)

    // Camera up (16 bytes)
    camera_up: vec4<f32>,   // offset 48

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

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Full-screen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((i32(vertex_index) << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vertex_index) & 2) * 2.0 - 1.0;
    out.position = vec4<f32>(x, -y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

// ============================================
// SDF Primitives
// ============================================

fn sdf_sphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdf_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdf_cylinder(p: vec3<f32>, r: f32, h: f32) -> f32 {
    let d = abs(vec2<f32>(length(p.xz), p.y)) - vec2<f32>(r, h);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0)));
}

fn sdf_torus(p: vec3<f32>, major_r: f32, minor_r: f32) -> f32 {
    let q = vec2<f32>(length(p.xz) - major_r, p.y);
    return length(q) - minor_r;
}

fn sdf_capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn sdf_plane(p: vec3<f32>, n: vec3<f32>, d: f32) -> f32 {
    return dot(p, n) + d;
}

// ============================================
// SDF Operations
// ============================================

fn op_union(d1: f32, d2: f32) -> f32 {
    return min(d1, d2);
}

fn op_subtract(d1: f32, d2: f32) -> f32 {
    return max(d1, -d2);
}

fn op_intersect(d1: f32, d2: f32) -> f32 {
    return max(d1, d2);
}

fn op_smooth_union(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

fn op_smooth_subtract(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d1, -d2, h) + k * h * (1.0 - h);
}

fn op_smooth_intersect(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 - 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) + k * h * (1.0 - h);
}

// ============================================
// Domain Transforms
// ============================================

fn op_twist(p: vec3<f32>, k: f32) -> vec3<f32> {
    let c = cos(k * p.y);
    let s = sin(k * p.y);
    let q = vec2<f32>(c * p.x - s * p.z, s * p.x + c * p.z);
    return vec3<f32>(q.x, p.y, q.y);
}

fn op_round(d: f32, r: f32) -> f32 {
    return d - r;
}

fn op_repeat(p: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    return ((p + 0.5 * c) % c) - 0.5 * c;
}

// ============================================
// Dynamic SDF from .asdf file (ALICE-SDF transpiled)
// This placeholder is replaced at runtime when loading .asdf files
// ============================================

// {{DYNAMIC_SDF_FUNCTION}}
// Default fallback when no .asdf is loaded
fn sdf_eval_dynamic(p: vec3<f32>) -> f32 {
    return length(p) - 1.0;  // Simple sphere fallback
}

// ============================================
// Demo Scenes SDF (6 scenes)
// ============================================

// Scene 0: Carved Sphere (default demo)
fn map_scene_0(p: vec3<f32>) -> f32 {
    // Base sphere
    var d = sdf_sphere(p, 1.0);

    // Subtract box
    d = op_smooth_subtract(d, sdf_box(p, vec3<f32>(0.6, 0.6, 0.6)), 0.1);

    // Add cylinders through center
    let cy = sdf_cylinder(p, 0.2, 1.5);
    let cx = sdf_cylinder(p.yxz, 0.2, 1.5);
    let cz = sdf_cylinder(p.xzy, 0.2, 1.5);

    d = op_smooth_union(d, cy, 0.1);
    d = op_smooth_union(d, cx, 0.1);
    d = op_smooth_union(d, cz, 0.1);

    // Animated twist
    let twisted_p = op_twist(p, sin(uniforms.time * 0.5) * 0.3);
    let torus = sdf_torus(twisted_p - vec3<f32>(0.0, 0.0, 0.0), 0.8, 0.15);

    d = op_smooth_union(d, torus, 0.2);

    return d;
}

// Scene 1: Simple Sphere
fn map_scene_1(p: vec3<f32>) -> f32 {
    return sdf_sphere(p, 1.0);
}

// Scene 2: Rounded Box
fn map_scene_2(p: vec3<f32>) -> f32 {
    let box_d = sdf_box(p, vec3<f32>(0.8, 0.8, 0.8));
    return op_round(box_d, 0.1);
}

// Scene 3: Torus Knot
fn map_scene_3(p: vec3<f32>) -> f32 {
    // Animated rotating torus knot
    let t = uniforms.time * 0.3;
    let c = cos(t);
    let s = sin(t);
    let rp = vec3<f32>(c * p.x - s * p.z, p.y, s * p.x + c * p.z);

    // Main torus
    var d = sdf_torus(rp, 0.8, 0.25);

    // Interlocking smaller tori
    let rp2 = vec3<f32>(rp.y, rp.z, rp.x);
    d = op_smooth_union(d, sdf_torus(rp2, 0.6, 0.15), 0.15);

    let rp3 = vec3<f32>(rp.z, rp.x, rp.y);
    d = op_smooth_union(d, sdf_torus(rp3, 0.5, 0.12), 0.15);

    return d;
}

// Scene 4: Infinite Pillars
fn map_scene_4(p: vec3<f32>) -> f32 {
    // Repeat space for infinite pillars
    let rep = vec3<f32>(3.0, 0.0, 3.0);
    var rp = p;
    rp.x = ((p.x + rep.x * 0.5) % rep.x) - rep.x * 0.5;
    rp.z = ((p.z + rep.z * 0.5) % rep.z) - rep.z * 0.5;

    // Cylinder pillars
    var d = sdf_cylinder(rp, 0.3, 10.0);

    // Ground plane
    d = op_union(d, sdf_plane(p, vec3<f32>(0.0, 1.0, 0.0), 2.0));

    // Animated floating sphere
    let sphere_pos = vec3<f32>(
        sin(uniforms.time * 0.7) * 1.5,
        sin(uniforms.time * 0.5) * 0.5 + 0.5,
        cos(uniforms.time * 0.7) * 1.5
    );
    d = op_smooth_union(d, sdf_sphere(p - sphere_pos, 0.4), 0.3);

    return d;
}

// Scene 5: Twisted Box
fn map_scene_5(p: vec3<f32>) -> f32 {
    // Twist amount animated
    let twist_amount = sin(uniforms.time * 0.4) * 2.0 + 0.5;
    let twisted_p = op_twist(p, twist_amount);

    // Rounded twisted box
    var d = sdf_box(twisted_p, vec3<f32>(0.6, 1.2, 0.6));
    d = op_round(d, 0.05);

    // Subtract spheres at corners
    d = op_smooth_subtract(d, sdf_sphere(twisted_p - vec3<f32>(0.0, 1.0, 0.0), 0.4), 0.1);
    d = op_smooth_subtract(d, sdf_sphere(twisted_p - vec3<f32>(0.0, -1.0, 0.0), 0.4), 0.1);

    return d;
}

// Scene dispatcher based on scene_id
// scene_id 0-99: Built-in demo scenes
// scene_id 100: Dynamic SDF from loaded .asdf file
fn map_scene(p: vec3<f32>) -> f32 {
    switch uniforms.scene_id {
        case 0u: { return map_scene_0(p); }
        case 1u: { return map_scene_1(p); }
        case 2u: { return map_scene_2(p); }
        case 3u: { return map_scene_3(p); }
        case 4u: { return map_scene_4(p); }
        case 5u: { return map_scene_5(p); }
        case 100u: { return sdf_eval_dynamic(p); }  // Dynamic SDF from .asdf
        default: { return map_scene_0(p); }
    }
}

// ============================================
// Raymarching
// ============================================

fn calc_normal(p: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(0.0001, 0.0);
    return normalize(vec3<f32>(
        map_scene(p + e.xyy) - map_scene(p - e.xyy),
        map_scene(p + e.yxy) - map_scene(p - e.yxy),
        map_scene(p + e.yyx) - map_scene(p - e.yyx)
    ));
}

fn calc_ao(p: vec3<f32>, n: vec3<f32>) -> f32 {
    var occ = 0.0;
    var sca = 1.0;
    for (var i = 0; i < 5; i++) {
        let h = 0.01 + 0.12 * f32(i) / 4.0;
        let d = map_scene(p + n * h);
        occ += (h - d) * sca;
        sca *= 0.95;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

fn raymarch(ro: vec3<f32>, rd: vec3<f32>) -> vec2<f32> {
    var t = 0.0;
    var steps = 0u;

    for (var i = 0u; i < uniforms.max_steps; i++) {
        let p = ro + rd * t;
        let d = map_scene(p);

        if (d < uniforms.epsilon) {
            return vec2<f32>(t, f32(i));
        }
        if (t > uniforms.max_distance) {
            break;
        }

        t += d;
        steps = i;
    }

    return vec2<f32>(-1.0, f32(steps));
}

// ============================================
// Lighting
// ============================================

fn get_ray(uv: vec2<f32>) -> vec3<f32> {
    let cam_pos = uniforms.camera_pos.xyz;
    let cam_target = uniforms.camera_target.xyz;
    let cam_up = uniforms.camera_up.xyz;
    let fov = uniforms.camera_target.w;  // fov stored in w component

    let forward = normalize(cam_target - cam_pos);
    let right = normalize(cross(forward, cam_up));
    let up = cross(right, forward);

    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    let fov_scale = tan(fov * 0.5);

    let centered_uv = (uv - 0.5) * 2.0;
    return normalize(
        forward +
        right * centered_uv.x * fov_scale * aspect +
        up * centered_uv.y * fov_scale
    );
}

fn shade(p: vec3<f32>, n: vec3<f32>, rd: vec3<f32>) -> vec3<f32> {
    // Show normals mode
    let show_normals = (uniforms.flags & 1u) != 0u;
    if (show_normals) {
        return n * 0.5 + 0.5;
    }

    // Three-point lighting
    let light1_dir = normalize(vec3<f32>(1.0, 1.0, 1.0));
    let light2_dir = normalize(vec3<f32>(-0.5, 0.3, -0.5));
    let light3_dir = normalize(vec3<f32>(0.0, -1.0, 0.0));

    let light1_color = vec3<f32>(1.0, 0.95, 0.9);
    let light2_color = vec3<f32>(0.3, 0.4, 0.6);
    let light3_color = vec3<f32>(0.15, 0.1, 0.1);

    // Diffuse
    let diff1 = max(dot(n, light1_dir), 0.0);
    let diff2 = max(dot(n, light2_dir), 0.0);
    let diff3 = max(dot(n, light3_dir), 0.0);

    // Specular (Blinn-Phong)
    let h1 = normalize(light1_dir - rd);
    let spec1 = pow(max(dot(n, h1), 0.0), 32.0);

    // Fresnel rim lighting
    let fresnel = pow(1.0 - max(dot(n, -rd), 0.0), 3.0);
    let rim = fresnel * 0.3;

    // Ambient occlusion
    var ao = 1.0;
    let use_ao = (uniforms.flags & 2u) != 0u;
    if (use_ao) {
        ao = calc_ao(p, n);
    }

    // Material color (gradient based on position)
    let base_color = vec3<f32>(0.7, 0.5, 0.4) + 0.3 * cos(p * 0.5 + vec3<f32>(0.0, 1.0, 2.0));

    // Combine
    var color = base_color * (
        diff1 * light1_color * 0.8 +
        diff2 * light2_color * 0.4 +
        diff3 * light3_color * 0.2 +
        vec3<f32>(0.05) // Ambient
    );

    color += spec1 * light1_color * 0.5;
    color += rim * vec3<f32>(0.5, 0.6, 0.7);
    color *= ao;

    return color;
}

// ============================================
// Main Fragment Shader
// ============================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let rd = get_ray(in.uv);
    let ro = uniforms.camera_pos.xyz;

    let result = raymarch(ro, rd);
    let t = result.x;
    let steps = result.y;

    var color: vec3<f32>;

    if (t > 0.0) {
        let p = ro + rd * t;
        let n = calc_normal(p);
        color = shade(p, n, rd);

        // Fog
        let fog = 1.0 - exp(-0.02 * t * t);
        let fog_color = vec3<f32>(0.1, 0.12, 0.15);
        color = mix(color, fog_color, fog);
    } else {
        // Background gradient
        let bg_top = vec3<f32>(0.05, 0.08, 0.12);
        let bg_bottom = vec3<f32>(0.15, 0.12, 0.1);
        color = mix(bg_bottom, bg_top, in.uv.y);

        // Subtle grid pattern
        let grid_scale = 50.0;
        let grid = smoothstep(0.98, 1.0, max(
            abs(sin(in.uv.x * grid_scale)),
            abs(sin(in.uv.y * grid_scale))
        ));
        color += grid * 0.03;
    }

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    // Vignette
    let vignette = 1.0 - length(in.uv - 0.5) * 0.5;
    color *= vignette;

    return vec4<f32>(color, 1.0);
}
