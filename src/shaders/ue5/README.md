# ALICE Shaders for Unreal Engine 5

"Store equations, not pixels" - GPU-computed infinite resolution content.

## Installation

1. Copy `ALICE_Common.usf` and `ALICE_Procedural.usf` to your project:
   ```
   YourProject/Shaders/ALICE/ALICE_Common.usf
   YourProject/Shaders/ALICE/ALICE_Procedural.usf
   ```

2. In your project's `.Build.cs` file, ensure shader directory is included:
   ```csharp
   PublicIncludePaths.Add(Path.Combine(ModuleDirectory, "Shaders"));
   ```

3. Restart the Unreal Editor.

## Usage in Materials

### Method 1: Custom Expression

Create a Material with a Custom node:

```hlsl
// In Custom Expression
#include "/Project/ALICE/ALICE_Common.usf"
#include "/Project/ALICE/ALICE_Procedural.usf"

float2 UV = GetUV(0);
float Time = View.GameTime;

return ALICE_GenerateMandelbrot(UV, Zoom, Pan, 256, Time);
```

Inputs:
- `Zoom` (Scalar): Zoom level (1.0 = default)
- `Pan` (Vector2): Pan offset

### Method 2: Material Function

Create reusable Material Functions for each generator:

```
MF_ALICE_Perlin
MF_ALICE_Mandelbrot
MF_ALICE_Voronoi
MF_ALICE_Wireframe
```

## Available Functions

### Noise Functions
| Function | Description |
|----------|-------------|
| `ALICE_Hash2(p)` | 2D hash function |
| `ALICE_GradientNoise(p)` | Gradient (Perlin-like) noise |
| `ALICE_ValueNoise(p)` | Value noise |
| `ALICE_FBM(p, octaves, persistence, lacunarity)` | Fractal Brownian Motion |

### Generators
| Function | Description |
|----------|-------------|
| `ALICE_GenerateTerrain()` | Procedural terrain with height coloring |
| `ALICE_GenerateMandelbrot()` | Mandelbrot fractal |
| `ALICE_GenerateJulia()` | Julia set fractal |
| `ALICE_GenerateVoronoi()` | Voronoi cell pattern |
| `ALICE_GeneratePlasma()` | Animated plasma effect |
| `ALICE_GenerateWireframe()` | Debug wireframe visualization |

### Utilities
| Function | Description |
|----------|-------------|
| `ALICE_TransformUV(UV, Zoom, Pan)` | Apply zoom and pan to UV |
| `ALICE_HSVtoRGB(hsv)` | HSV to RGB conversion |
| `ALICE_ComputeGradient(p, eps)` | Compute noise gradient |

## Example: Infinite Zoom Material

```hlsl
// Custom Expression for infinite zoom Mandelbrot
#include "/Project/ALICE/ALICE_Procedural.usf"

float2 UV = TexCoords;
float Zoom = pow(2.0, ZoomLevel); // ZoomLevel: 0-50
float2 Pan = float2(-0.75, 0.0) + PanOffset;

return ALICE_GenerateMandelbrot(UV, Zoom, Pan, 256 + ZoomLevel * 10, Time);
```

## Integration with ALICE Streaming Protocol (ASP)

When receiving ASP packets, decode parameters and pass to shader:

```cpp
// C++ Actor
void AASPReceiver::OnReceivePacket(const FASPPacket& Packet)
{
    DynamicMaterial->SetScalarParameterValue("Zoom", Packet.Zoom);
    DynamicMaterial->SetVectorParameterValue("Pan", FLinearColor(Packet.PanX, Packet.PanY, 0, 0));
    DynamicMaterial->SetScalarParameterValue("ContentType", Packet.ContentType);
}
```

## Performance Tips

1. **LOD System**: Reduce `Octaves` and `MaxIterations` for distant objects
2. **Texture Caching**: For static views, render to RenderTarget once
3. **Async Compute**: Use compute shaders for heavy calculations
4. **Niagara Integration**: Use ALICE functions in Niagara modules for particles

## License

MIT License - Part of Project A.L.I.C.E.
