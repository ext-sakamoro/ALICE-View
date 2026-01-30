# ALICE Shaders for Godot 4.x

"Store equations, not pixels" - GPU-computed infinite resolution content.

## Installation

1. Copy shader files to your Godot project:
   ```
   res://shaders/ALICE/
   ├── ALICE_Common.gdshaderinc
   └── ALICE_Procedural.gdshader
   ```

2. Create a ShaderMaterial and assign `ALICE_Procedural.gdshader`.

## Usage

### Method 1: Direct Material Assignment

1. Create a new **ShaderMaterial**
2. Set Shader to `ALICE_Procedural.gdshader`
3. Adjust parameters in the Inspector

### Method 2: Programmatic Control

```gdscript
extends Sprite2D

@onready var material: ShaderMaterial = $ShaderMaterial

func _ready():
    # Set content type (0=Perlin, 1=Mandelbrot, 2=Julia, 3=Voronoi, 4=Plasma, 5=Wireframe)
    material.set_shader_parameter("content_type", 1)
    material.set_shader_parameter("zoom", 1.0)
    material.set_shader_parameter("pan", Vector2(-0.5, 0.0))
    material.set_shader_parameter("max_iterations", 256)

func _process(delta):
    # Animate zoom
    var new_zoom = material.get_shader_parameter("zoom") * 1.01
    material.set_shader_parameter("zoom", new_zoom)
```

### Method 3: Using Common Functions

Include `ALICE_Common.gdshaderinc` in your custom shaders:

```glsl
shader_type canvas_item;

#include "res://shaders/ALICE/ALICE_Common.gdshaderinc"

uniform float zoom = 1.0;
uniform vec2 pan = vec2(0.0);

void fragment() {
    vec2 world_pos = alice_transform_uv(UV, zoom, pan);

    // Use ALICE functions
    float noise = alice_fbm(world_pos * 10.0, 6, 0.5, 2.0);
    float fractal = alice_mandelbrot(world_pos * 3.0, 256, 2.0);

    VoronoiResult vor = alice_voronoi(world_pos * 5.0, 1.0);

    COLOR = vec4(vec3(noise), 1.0);
}
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `zoom` | float | Zoom level (1.0 = default, higher = zoom in) |
| `pan` | vec2 | Pan offset in world space |
| `content_type` | int | 0=Perlin, 1=Mandelbrot, 2=Julia, 3=Voronoi, 4=Plasma, 5=Wireframe |
| `scale` | float | Noise/pattern scale |
| `octaves` | int | FBM detail level (1-12) |
| `max_iterations` | int | Fractal detail (16-512) |
| `julia_c` | vec2 | Julia set constant parameter |
| `edge_width` | float | Voronoi edge width |
| `animation_speed` | float | Time-based animation speed |

## Available Functions (ALICE_Common.gdshaderinc)

### Hash Functions
```glsl
float alice_hash2(vec2 p)           // 2D hash
float alice_hash3(vec3 p)           // 3D hash
vec2 alice_hash2d(vec2 p)           // 2D vector hash
```

### Noise Functions
```glsl
float alice_gradient_noise(vec2 p)  // Gradient (Perlin-like) noise
float alice_value_noise(vec2 p)     // Value noise
float alice_fbm(vec2 p, int octaves, float persistence, float lacunarity)
float alice_fbm_simple(vec2 p, int octaves)
```

### Voronoi
```glsl
VoronoiResult alice_voronoi(vec2 p, float randomness)
// Returns: distance, cell_center, cell_id
```

### Fractals
```glsl
float alice_mandelbrot(vec2 c, int max_iterations, float escape_radius)
float alice_julia(vec2 z, vec2 c, int max_iterations, float escape_radius)
```

### Utilities
```glsl
vec2 alice_transform_uv(vec2 uv, float zoom, vec2 pan)
vec3 alice_hsv_to_rgb(vec3 hsv)
vec2 alice_compute_gradient(vec2 p, float epsilon)
```

## Integration with ALICE Streaming Protocol

### GDScript Receiver Example

```gdscript
extends Node

@export var alice_material: ShaderMaterial

func on_asp_packet_received(packet: Dictionary):
    alice_material.set_shader_parameter("zoom", packet.zoom)
    alice_material.set_shader_parameter("pan", Vector2(packet.pan_x, packet.pan_y))
    alice_material.set_shader_parameter("content_type", packet.content_type)
    alice_material.set_shader_parameter("scale", packet.scale)
    alice_material.set_shader_parameter("octaves", packet.octaves)
```

### ASP Packet Structure

```gdscript
class ASPPacket:
    var zoom: float
    var pan_x: float
    var pan_y: float
    var content_type: int
    var scale: float
    var octaves: int
    var param1: float
    var param2: float
```

## Performance Tips

1. **LOD System**: Reduce `octaves` for distant objects
2. **Baking**: For static content, use `SubViewport` to render once and cache
3. **Compute Shaders**: Use Godot's compute shaders for heavy calculations
4. **Texture Caching**: Cache frequently used zoom levels in textures

## Example: Infinite Zoom Scene

```gdscript
extends Node2D

@onready var shader_rect: ColorRect = $ColorRect
var material: ShaderMaterial

var target_zoom: float = 1.0
var current_zoom: float = 1.0
var zoom_speed: float = 0.1

func _ready():
    material = shader_rect.material as ShaderMaterial

func _input(event):
    if event is InputEventMouseButton:
        if event.button_index == MOUSE_BUTTON_WHEEL_UP:
            target_zoom *= 1.5
        elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
            target_zoom /= 1.5

func _process(delta):
    current_zoom = lerp(current_zoom, target_zoom, zoom_speed)
    material.set_shader_parameter("zoom", current_zoom)
```

## Compatibility

| Feature | Godot 4.0+ | Godot 3.x |
|---------|------------|-----------|
| ALICE_Procedural.gdshader | ✅ | ❌ (use .shader) |
| ALICE_Common.gdshaderinc | ✅ | ❌ (use .shader) |
| Compute Shaders | ✅ | ❌ |

For Godot 3.x, rename files to `.shader` and adjust syntax as needed.

## License

MIT License - Part of Project A.L.I.C.E.
