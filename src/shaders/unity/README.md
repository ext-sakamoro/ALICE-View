# ALICE Shaders for Unity

"Store equations, not pixels" - GPU-computed infinite resolution content.

## Installation

1. Copy all files to your Unity project:
   ```
   Assets/Shaders/ALICE/
   ├── ALICE_Common.cginc
   ├── ALICE_Procedural.shader
   └── ALICE_ShaderGraph.hlsl
   ```

2. Unity will automatically import the shaders.

## Method 1: Built-in Render Pipeline (Standard Shader)

### Create Material

1. Right-click in Project → Create → Material
2. Set Shader to `ALICE/Procedural`
3. Adjust parameters in Inspector

### Parameters

| Parameter | Description |
|-----------|-------------|
| Zoom | Zoom level (1.0 = default, higher = zoom in) |
| Pan X/Y | Pan offset in world space |
| Content Type | Perlin, Mandelbrot, Julia, Voronoi, Plasma, Wireframe |
| Scale | Noise/pattern scale |
| Octaves | FBM detail level (1-12) |
| Max Iterations | Fractal detail (16-512) |
| Julia C | Julia set constant parameter |
| Animation Speed | Time-based animation speed |

## Method 2: Shader Graph (URP/HDRP)

### Setup Custom Function Nodes

1. In Shader Graph, create a **Custom Function** node
2. Set Type to **File**
3. Set Source to `ALICE_ShaderGraph.hlsl`
4. Set Name to the function you want (e.g., `ALICE_GenerateMandelbrot`)

### Available Functions

#### Transform
```
ALICE_TransformUV_float(UV, Zoom, Pan) → TransformedUV
```

#### Noise
```
ALICE_GradientNoise_float(Position) → NoiseValue
ALICE_FBM_float(Position, Octaves, Persistence, Lacunarity) → FBMValue
```

#### Patterns
```
ALICE_Voronoi_float(Position, Randomness) → Distance, CellCenter, CellID
```

#### Fractals
```
ALICE_Mandelbrot_float(C, MaxIterations, EscapeRadius) → IterationRatio
ALICE_Julia_float(Z, C, MaxIterations, EscapeRadius) → IterationRatio
```

#### Complete Generators
```
ALICE_GeneratePerlin_float(UV, Zoom, Pan, Scale, Octaves) → Color
ALICE_GenerateMandelbrot_float(UV, Zoom, Pan, MaxIterations, Time) → Color
ALICE_GenerateVoronoi_float(UV, Zoom, Pan, Scale, EdgeWidth) → Color
```

### Example: Shader Graph Mandelbrot

```
[UV] → [Custom Function: ALICE_TransformUV] → [Custom Function: ALICE_Mandelbrot] → [Custom Function: ALICE_FractalColor] → [Base Color]
         ↑                                        ↑                                      ↑
      [Zoom]                              [MaxIterations: 256]                      [Time]
      [Pan]                               [EscapeRadius: 2.0]
```

## Method 3: VFX Graph

Use ALICE functions in VFX Graph for procedural particle effects:

1. Create Custom HLSL block
2. Include ALICE functions
3. Use for particle position, color, or lifetime

```hlsl
// In VFX Graph HLSL block
#include "Assets/Shaders/ALICE/ALICE_ShaderGraph.hlsl"

float noise;
ALICE_FBM_float(position.xz * 0.1, 4, 0.5, 2.0, noise);
position.y = noise * 10.0;
```

## Integration with ALICE Streaming Protocol

### C# Receiver Example

```csharp
using UnityEngine;

public class ALICEReceiver : MonoBehaviour
{
    public Material aliceMaterial;

    void OnASPPacketReceived(ASPPacket packet)
    {
        aliceMaterial.SetFloat("_Zoom", packet.zoom);
        aliceMaterial.SetFloat("_PanX", packet.panX);
        aliceMaterial.SetFloat("_PanY", packet.panY);
        aliceMaterial.SetInt("_ContentType", packet.contentType);
        aliceMaterial.SetFloat("_Scale", packet.scale);
        aliceMaterial.SetInt("_Octaves", packet.octaves);
    }
}
```

### ASP Packet Structure

```csharp
[System.Serializable]
public struct ASPPacket
{
    public float zoom;
    public float panX;
    public float panY;
    public int contentType;
    public float scale;
    public int octaves;
    public float param1;
    public float param2;
}
```

## Performance Tips

1. **LOD System**: Reduce `_Octaves` for distant objects
2. **Baking**: For static content, render to RenderTexture once
3. **Compute Shaders**: Use for heavy calculations on GPU
4. **Texture Arrays**: Cache frequently used zoom levels

## Render Pipeline Compatibility

| Feature | Built-in | URP | HDRP |
|---------|----------|-----|------|
| ALICE_Procedural.shader | ✅ | ❌ | ❌ |
| ALICE_ShaderGraph.hlsl | ❌ | ✅ | ✅ |
| Custom Function Nodes | ❌ | ✅ | ✅ |

For URP/HDRP, use Shader Graph with the Custom Function approach.

## License

MIT License - Part of Project A.L.I.C.E.
