// ALICE Procedural Content Shader for Godot 4.x
// "Store equations, not pixels" - GPU-computed infinite resolution
//
// Usage: Create a ShaderMaterial with this shader

shader_type canvas_item;

// Transform parameters
uniform float zoom : hint_range(0.001, 1000000.0) = 1.0;
uniform vec2 pan = vec2(0.0, 0.0);

// Content type: 0=Perlin, 1=Mandelbrot, 2=Julia, 3=Voronoi, 4=Plasma, 5=Wireframe
uniform int content_type : hint_range(0, 5) = 0;

// Perlin settings
uniform float scale : hint_range(0.1, 100.0) = 10.0;
uniform int octaves : hint_range(1, 12) = 6;

// Fractal settings
uniform int max_iterations : hint_range(16, 512) = 256;
uniform vec2 julia_c = vec2(-0.7, 0.27);

// Voronoi settings
uniform float edge_width : hint_range(0.001, 0.2) = 0.05;

// Animation
uniform float animation_speed : hint_range(0.0, 5.0) = 1.0;

// ============================================
// Hash Functions
// ============================================

float hash2(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

vec2 hash2d(vec2 p) {
	return vec2(
		hash2(p),
		hash2(p + vec2(57.0, 113.0))
	);
}

// ============================================
// Noise Functions
// ============================================

float gradient_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

	float a = hash2(i + vec2(0.0, 0.0));
	float b = hash2(i + vec2(1.0, 0.0));
	float c = hash2(i + vec2(0.0, 1.0));
	float d = hash2(i + vec2(1.0, 1.0));

	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p, int oct, float persistence, float lacunarity) {
	float value = 0.0;
	float amplitude = 0.5;
	float frequency = 1.0;
	float max_value = 0.0;

	for (int i = 0; i < oct; i++) {
		value += amplitude * gradient_noise(p * frequency);
		max_value += amplitude;
		amplitude *= persistence;
		frequency *= lacunarity;
	}

	return value / max_value;
}

// ============================================
// Transform
// ============================================

vec2 transform_uv(vec2 uv) {
	return (uv - 0.5) / zoom + pan;
}

// ============================================
// Content Generators
// ============================================

vec3 generate_perlin(vec2 uv) {
	vec2 world_pos = transform_uv(uv);
	float n = fbm(world_pos * scale, octaves, 0.5, 2.0);

	vec3 low = vec3(0.1, 0.2, 0.4);
	vec3 mid = vec3(0.3, 0.6, 0.3);
	vec3 high = vec3(0.9, 0.8, 0.6);

	if (n < 0.5) {
		return mix(low, mid, n * 2.0);
	} else {
		return mix(mid, high, (n - 0.5) * 2.0);
	}
}

vec3 fractal_color(float t, float time) {
	float hue = t + time * 0.1;
	return vec3(
		0.5 + 0.5 * cos(6.28318 * hue),
		0.5 + 0.5 * cos(6.28318 * (hue + 0.333)),
		0.5 + 0.5 * cos(6.28318 * (hue + 0.666))
	);
}

vec3 generate_mandelbrot(vec2 uv, float time) {
	vec2 world_pos = transform_uv(uv);
	vec2 c = world_pos * 3.0 + vec2(-0.5, 0.0);

	vec2 z = vec2(0.0);
	int i = 0;

	for (int iter = 0; iter < 256; iter++) {
		if (iter >= max_iterations) break;
		if (dot(z, z) > 4.0) break;

		z = vec2(
			z.x * z.x - z.y * z.y + c.x,
			2.0 * z.x * z.y + c.y
		);
		i++;
	}

	if (i >= max_iterations) {
		return vec3(0.0);
	}

	float t = float(i) / float(max_iterations);
	return fractal_color(t, time);
}

vec3 generate_julia(vec2 uv, float time) {
	vec2 world_pos = transform_uv(uv);
	vec2 z = world_pos * 2.0;
	vec2 c = julia_c;

	int i = 0;

	for (int iter = 0; iter < 256; iter++) {
		if (iter >= max_iterations) break;
		if (dot(z, z) > 4.0) break;

		z = vec2(
			z.x * z.x - z.y * z.y + c.x,
			2.0 * z.x * z.y + c.y
		);
		i++;
	}

	if (i >= max_iterations) {
		return vec3(0.0);
	}

	float t = float(i) / float(max_iterations);
	return fractal_color(t, time);
}

vec3 generate_voronoi(vec2 uv) {
	vec2 world_pos = transform_uv(uv);
	vec2 p = world_pos * scale;

	vec2 n = floor(p);
	vec2 f = fract(p);

	float min_dist = 10.0;
	vec2 cell_center = vec2(0.0);

	for (int j = -1; j <= 1; j++) {
		for (int i = -1; i <= 1; i++) {
			vec2 neighbor = vec2(float(i), float(j));
			vec2 point = hash2d(n + neighbor);
			vec2 diff = neighbor + point - f;
			float dist = length(diff);

			if (dist < min_dist) {
				min_dist = dist;
				cell_center = n + neighbor + point;
			}
		}
	}

	vec3 cell_color = vec3(
		hash2(cell_center),
		hash2(cell_center + vec2(17.0, 31.0)),
		hash2(cell_center + vec2(73.0, 89.0))
	);

	float edge = smoothstep(0.0, edge_width, min_dist);
	return cell_color * edge;
}

vec3 hsv_to_rgb(vec3 hsv) {
	vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
	vec3 p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
	return hsv.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), hsv.y);
}

vec3 generate_plasma(vec2 uv, float time) {
	vec2 world_pos = transform_uv(uv);

	float v1 = sin(world_pos.x * 10.0 + time);
	float v2 = sin(world_pos.y * 10.0 + time * 0.5);
	float v3 = sin((world_pos.x + world_pos.y) * 10.0 + time * 0.7);
	float v4 = sin(length(world_pos * 10.0) + time);

	float v = (v1 + v2 + v3 + v4) * 0.25 + 0.5;

	return hsv_to_rgb(vec3(v, 0.8, 0.9));
}

vec3 generate_wireframe(vec2 uv, float time) {
	vec2 world_pos = transform_uv(uv);
	float grid_size = 0.1;
	vec2 p = world_pos / grid_size;
	vec2 f = fract(p);

	float min_dist = min(min(f.y, f.x), abs(f.x + f.y - 1.0) / 1.414);
	float wire = 1.0 - smoothstep(0.0, 0.02, min_dist);
	float glow = exp(-min_dist * 10.0) * 0.3;

	float vertex_dist = min(
		min(length(f), length(f - vec2(1.0, 0.0))),
		min(length(f - vec2(0.0, 1.0)), length(f - vec2(1.0, 1.0)))
	);
	float vertex = 1.0 - smoothstep(0.0, 0.05, vertex_dist);

	float pulse = 0.5 + 0.5 * sin(time * 2.0 - length(world_pos) * 3.0);

	vec3 color = vec3(0.02, 0.05, 0.08);
	color += vec3(0.0, 1.0, 1.0) * wire;
	color += vec3(0.0, 0.3, 0.4) * glow;
	color += vec3(1.0, 1.0, 1.0) * vertex * 0.8;
	color *= 0.7 + 0.3 * pulse;

	return color;
}

// ============================================
// Main Fragment
// ============================================

void fragment() {
	float time = TIME * animation_speed;
	vec3 color;

	if (content_type == 0) {
		color = generate_perlin(UV);
	} else if (content_type == 1) {
		color = generate_mandelbrot(UV, time);
	} else if (content_type == 2) {
		color = generate_julia(UV, time);
	} else if (content_type == 3) {
		color = generate_voronoi(UV);
	} else if (content_type == 4) {
		color = generate_plasma(UV, time);
	} else if (content_type == 5) {
		color = generate_wireframe(UV, time);
	} else {
		color = vec3(0.5);
	}

	// Subtle vignette
	float vignette = 1.0 - length(UV - 0.5) * 0.3;
	color *= vignette;

	COLOR = vec4(color, 1.0);
}
