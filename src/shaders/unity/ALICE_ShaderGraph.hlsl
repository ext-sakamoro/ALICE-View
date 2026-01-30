// ALICE Functions for Unity Shader Graph Custom Function Nodes
// "Store equations, not pixels" - GPU-computed infinite resolution
//
// Usage: Create Custom Function node in Shader Graph
//        Type: File, Source: ALICE_ShaderGraph.hlsl

#ifndef ALICE_SHADERGRAPH_INCLUDED
#define ALICE_SHADERGRAPH_INCLUDED

// ============================================
// Hash Functions
// ============================================

void ALICE_Hash_float(float2 p, out float result)
{
    result = frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453);
}

void ALICE_Hash2D_float(float2 p, out float2 result)
{
    result = float2(
        frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453),
        frac(sin(dot(p + float2(57.0, 113.0), float2(127.1, 311.7))) * 43758.5453)
    );
}

// ============================================
// Noise Functions
// ============================================

void ALICE_GradientNoise_float(float2 p, out float result)
{
    float2 i = floor(p);
    float2 f = frac(p);
    float2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    float a, b, c, d;
    ALICE_Hash_float(i + float2(0.0, 0.0), a);
    ALICE_Hash_float(i + float2(1.0, 0.0), b);
    ALICE_Hash_float(i + float2(0.0, 1.0), c);
    ALICE_Hash_float(i + float2(1.0, 1.0), d);

    result = lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y);
}

void ALICE_FBM_float(float2 p, int Octaves, float Persistence, float Lacunarity, out float result)
{
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 1.0;
    float maxValue = 0.0;

    for (int i = 0; i < Octaves; i++)
    {
        float n;
        ALICE_GradientNoise_float(p * frequency, n);
        value += amplitude * n;
        maxValue += amplitude;
        amplitude *= Persistence;
        frequency *= Lacunarity;
    }

    result = value / maxValue;
}

// ============================================
// Transform Functions
// ============================================

void ALICE_TransformUV_float(float2 UV, float Zoom, float2 Pan, out float2 result)
{
    result = (UV - 0.5) / Zoom + Pan;
}

// ============================================
// Voronoi
// ============================================

void ALICE_Voronoi_float(float2 p, float Randomness, out float Distance, out float2 CellCenter, out float CellID)
{
    float2 n = floor(p);
    float2 f = frac(p);

    Distance = 10.0;
    CellCenter = float2(0.0, 0.0);
    CellID = 0.0;

    for (int j = -1; j <= 1; j++)
    {
        for (int i = -1; i <= 1; i++)
        {
            float2 neighbor = float2(float(i), float(j));
            float2 point;
            ALICE_Hash2D_float(n + neighbor, point);
            point *= Randomness;

            float2 diff = neighbor + point - f;
            float dist = length(diff);

            if (dist < Distance)
            {
                Distance = dist;
                CellCenter = n + neighbor + point;
                ALICE_Hash_float(n + neighbor, CellID);
            }
        }
    }
}

// ============================================
// Fractals
// ============================================

void ALICE_Mandelbrot_float(float2 c, int MaxIterations, float EscapeRadius, out float result)
{
    float2 z = float2(0.0, 0.0);
    int i = 0;
    float escape2 = EscapeRadius * EscapeRadius;

    for (int iter = 0; iter < 256; iter++)
    {
        if (iter >= MaxIterations) break;
        if (dot(z, z) > escape2) break;

        z = float2(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        i++;
    }

    result = (i >= MaxIterations) ? 0.0 : float(i) / float(MaxIterations);
}

void ALICE_Julia_float(float2 z, float2 c, int MaxIterations, float EscapeRadius, out float result)
{
    int i = 0;
    float escape2 = EscapeRadius * EscapeRadius;

    for (int iter = 0; iter < 256; iter++)
    {
        if (iter >= MaxIterations) break;
        if (dot(z, z) > escape2) break;

        z = float2(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        i++;
    }

    result = (i >= MaxIterations) ? 0.0 : float(i) / float(MaxIterations);
}

// ============================================
// Color Utilities
// ============================================

void ALICE_HSVtoRGB_float(float3 HSV, out float3 RGB)
{
    float4 K = float4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    float3 p = abs(frac(HSV.xxx + K.xyz) * 6.0 - K.www);
    RGB = HSV.z * lerp(K.xxx, saturate(p - K.xxx), HSV.y);
}

void ALICE_FractalColor_float(float t, float Time, out float3 color)
{
    float hue = t + Time * 0.1;
    color = float3(
        0.5 + 0.5 * cos(6.28318 * hue),
        0.5 + 0.5 * cos(6.28318 * (hue + 0.333)),
        0.5 + 0.5 * cos(6.28318 * (hue + 0.666))
    );
}

// ============================================
// Complete Generators (for simple use)
// ============================================

void ALICE_GeneratePerlin_float(float2 UV, float Zoom, float2 Pan, float Scale, int Octaves, out float3 Color)
{
    float2 worldPos = (UV - 0.5) / Zoom + Pan;
    float n;
    ALICE_FBM_float(worldPos * Scale, Octaves, 0.5, 2.0, n);

    float3 low = float3(0.1, 0.2, 0.4);
    float3 mid = float3(0.3, 0.6, 0.3);
    float3 high = float3(0.9, 0.8, 0.6);

    Color = (n < 0.5) ? lerp(low, mid, n * 2.0) : lerp(mid, high, (n - 0.5) * 2.0);
}

void ALICE_GenerateMandelbrot_float(float2 UV, float Zoom, float2 Pan, int MaxIterations, float Time, out float3 Color)
{
    float2 worldPos = (UV - 0.5) / Zoom + Pan;
    float2 c = worldPos * 3.0 + float2(-0.5, 0.0);

    float t;
    ALICE_Mandelbrot_float(c, MaxIterations, 2.0, t);

    if (t == 0.0)
    {
        Color = float3(0.0, 0.0, 0.0);
    }
    else
    {
        ALICE_FractalColor_float(t, Time, Color);
    }
}

void ALICE_GenerateVoronoi_float(float2 UV, float Zoom, float2 Pan, float Scale, float EdgeWidth, out float3 Color)
{
    float2 worldPos = (UV - 0.5) / Zoom + Pan;
    float2 p = worldPos * Scale;

    float dist;
    float2 cellCenter;
    float cellID;
    ALICE_Voronoi_float(p, 1.0, dist, cellCenter, cellID);

    float r, g, b;
    ALICE_Hash_float(cellCenter, r);
    ALICE_Hash_float(cellCenter + float2(17.0, 31.0), g);
    ALICE_Hash_float(cellCenter + float2(73.0, 89.0), b);

    float edge = smoothstep(0.0, EdgeWidth, dist);
    Color = float3(r, g, b) * edge;
}

#endif // ALICE_SHADERGRAPH_INCLUDED
