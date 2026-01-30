// ALICE Common Functions for Godot 4.x
// "Store equations, not pixels" - GPU-computed infinite resolution
//
// Usage: #include "ALICE_Common.gdshaderinc"

// ============================================
// Hash Functions
// ============================================

float alice_hash2(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float alice_hash3(vec3 p) {
	return fract(sin(dot(p, vec3(127.1, 311.7, 74.7))) * 43758.5453);
}

vec2 alice_hash2d(vec2 p) {
	return vec2(
		alice_hash2(p),
		alice_hash2(p + vec2(57.0, 113.0))
	);
}

// ============================================
// Noise Functions
// ============================================

float alice_gradient_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);

	// Quintic interpolation
	vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

	float a = alice_hash2(i + vec2(0.0, 0.0));
	float b = alice_hash2(i + vec2(1.0, 0.0));
	float c = alice_hash2(i + vec2(0.0, 1.0));
	float d = alice_hash2(i + vec2(1.0, 1.0));

	return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float alice_value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);

	vec2 u = f * f * (3.0 - 2.0 * f);

	return mix(
		mix(alice_hash2(i + vec2(0.0, 0.0)), alice_hash2(i + vec2(1.0, 0.0)), u.x),
		mix(alice_hash2(i + vec2(0.0, 1.0)), alice_hash2(i + vec2(1.0, 1.0)), u.x),
		u.y
	);
}

// ============================================
// Fractal Brownian Motion
// ============================================

float alice_fbm(vec2 p, int octaves, float persistence, float lacunarity) {
	float value = 0.0;
	float amplitude = 0.5;
	float frequency = 1.0;
	float max_value = 0.0;

	for (int i = 0; i < octaves; i++) {
		value += amplitude * alice_gradient_noise(p * frequency);
		max_value += amplitude;
		amplitude *= persistence;
		frequency *= lacunarity;
	}

	return value / max_value;
}

float alice_fbm_simple(vec2 p, int octaves) {
	return alice_fbm(p, octaves, 0.5, 2.0);
}

// ============================================
// Voronoi
// ============================================

struct VoronoiResult {
	float distance;
	vec2 cell_center;
	float cell_id;
};

VoronoiResult alice_voronoi(vec2 p, float randomness) {
	vec2 n = floor(p);
	vec2 f = fract(p);

	VoronoiResult result;
	result.distance = 10.0;
	result.cell_center = vec2(0.0);
	result.cell_id = 0.0;

	for (int j = -1; j <= 1; j++) {
		for (int i = -1; i <= 1; i++) {
			vec2 neighbor = vec2(float(i), float(j));
			vec2 point = alice_hash2d(n + neighbor) * randomness;
			vec2 diff = neighbor + point - f;
			float dist = length(diff);

			if (dist < result.distance) {
				result.distance = dist;
				result.cell_center = n + neighbor + point;
				result.cell_id = alice_hash2(n + neighbor);
			}
		}
	}

	return result;
}

// ============================================
// Fractals
// ============================================

float alice_mandelbrot(vec2 c, int max_iterations, float escape_radius) {
	vec2 z = vec2(0.0);
	int i = 0;
	float escape2 = escape_radius * escape_radius;

	for (int iter = 0; iter < 256; iter++) {
		if (iter >= max_iterations) break;
		if (dot(z, z) > escape2) break;

		z = vec2(
			z.x * z.x - z.y * z.y + c.x,
			2.0 * z.x * z.y + c.y
		);
		i++;
	}

	if (i >= max_iterations) return 0.0;
	return float(i) / float(max_iterations);
}

float alice_julia(vec2 z, vec2 c, int max_iterations, float escape_radius) {
	int i = 0;
	float escape2 = escape_radius * escape_radius;

	for (int iter = 0; iter < 256; iter++) {
		if (iter >= max_iterations) break;
		if (dot(z, z) > escape2) break;

		z = vec2(
			z.x * z.x - z.y * z.y + c.x,
			2.0 * z.x * z.y + c.y
		);
		i++;
	}

	if (i >= max_iterations) return 0.0;
	return float(i) / float(max_iterations);
}

// ============================================
// Utilities
// ============================================

vec2 alice_transform_uv(vec2 uv, float zoom, vec2 pan) {
	return (uv - 0.5) / zoom + pan;
}

vec3 alice_hsv_to_rgb(vec3 hsv) {
	vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
	vec3 p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
	return hsv.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), hsv.y);
}

vec2 alice_compute_gradient(vec2 p, float epsilon) {
	float dx = alice_gradient_noise(p + vec2(epsilon, 0.0)) - alice_gradient_noise(p - vec2(epsilon, 0.0));
	float dy = alice_gradient_noise(p + vec2(0.0, epsilon)) - alice_gradient_noise(p - vec2(0.0, epsilon));
	return vec2(dx, dy) / (2.0 * epsilon);
}
