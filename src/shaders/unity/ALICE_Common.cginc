// ALICE Common Functions for Unity
// "Store equations, not pixels" - GPU-computed infinite resolution
//
// Usage: #include "ALICE_Common.cginc"

#ifndef ALICE_COMMON_INCLUDED
#define ALICE_COMMON_INCLUDED

// ============================================
// Hash Functions
// ============================================

float ALICE_Hash2(float2 p)
{
    return frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453);
}

float ALICE_Hash3(float3 p)
{
    return frac(sin(dot(p, float3(127.1, 311.7, 74.7))) * 43758.5453);
}

float2 ALICE_Hash2D(float2 p)
{
    return float2(
        ALICE_Hash2(p),
        ALICE_Hash2(p + float2(57.0, 113.0))
    );
}

// ============================================
// Noise Functions
// ============================================

float ALICE_GradientNoise(float2 p)
{
    float2 i = floor(p);
    float2 f = frac(p);

    // Quintic interpolation
    float2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    float a = ALICE_Hash2(i + float2(0.0, 0.0));
    float b = ALICE_Hash2(i + float2(1.0, 0.0));
    float c = ALICE_Hash2(i + float2(0.0, 1.0));
    float d = ALICE_Hash2(i + float2(1.0, 1.0));

    return lerp(lerp(a, b, u.x), lerp(c, d, u.x), u.y);
}

float ALICE_ValueNoise(float2 p)
{
    float2 i = floor(p);
    float2 f = frac(p);

    float2 u = f * f * (3.0 - 2.0 * f);

    return lerp(
        lerp(ALICE_Hash2(i + float2(0.0, 0.0)), ALICE_Hash2(i + float2(1.0, 0.0)), u.x),
        lerp(ALICE_Hash2(i + float2(0.0, 1.0)), ALICE_Hash2(i + float2(1.0, 1.0)), u.x),
        u.y
    );
}

// ============================================
// Fractal Brownian Motion
// ============================================

float ALICE_FBM(float2 p, int octaves, float persistence, float lacunarity)
{
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 1.0;
    float maxValue = 0.0;

    for (int i = 0; i < octaves; i++)
    {
        value += amplitude * ALICE_GradientNoise(p * frequency);
        maxValue += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    return value / maxValue;
}

float ALICE_FBM_Simple(float2 p, int octaves)
{
    return ALICE_FBM(p, octaves, 0.5, 2.0);
}

// ============================================
// Voronoi
// ============================================

struct ALICE_VoronoiResult
{
    float distance;
    float2 cellCenter;
    float cellID;
};

ALICE_VoronoiResult ALICE_Voronoi(float2 p, float randomness)
{
    float2 n = floor(p);
    float2 f = frac(p);

    ALICE_VoronoiResult result;
    result.distance = 10.0;
    result.cellCenter = float2(0.0, 0.0);
    result.cellID = 0.0;

    for (int j = -1; j <= 1; j++)
    {
        for (int i = -1; i <= 1; i++)
        {
            float2 neighbor = float2(float(i), float(j));
            float2 point = ALICE_Hash2D(n + neighbor) * randomness;
            float2 diff = neighbor + point - f;
            float dist = length(diff);

            if (dist < result.distance)
            {
                result.distance = dist;
                result.cellCenter = n + neighbor + point;
                result.cellID = ALICE_Hash2(n + neighbor);
            }
        }
    }

    return result;
}

// ============================================
// Fractals
// ============================================

float ALICE_Mandelbrot(float2 c, int maxIterations, float escapeRadius)
{
    float2 z = float2(0.0, 0.0);
    int i = 0;
    float escape2 = escapeRadius * escapeRadius;

    for (int iter = 0; iter < 256; iter++)
    {
        if (iter >= maxIterations) break;
        if (dot(z, z) > escape2) break;

        z = float2(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        i++;
    }

    if (i >= maxIterations) return 0.0;
    return float(i) / float(maxIterations);
}

float ALICE_Julia(float2 z, float2 c, int maxIterations, float escapeRadius)
{
    int i = 0;
    float escape2 = escapeRadius * escapeRadius;

    for (int iter = 0; iter < 256; iter++)
    {
        if (iter >= maxIterations) break;
        if (dot(z, z) > escape2) break;

        z = float2(
            z.x * z.x - z.y * z.y + c.x,
            2.0 * z.x * z.y + c.y
        );
        i++;
    }

    if (i >= maxIterations) return 0.0;
    return float(i) / float(maxIterations);
}

// ============================================
// Utilities
// ============================================

float2 ALICE_TransformUV(float2 uv, float zoom, float2 pan)
{
    return (uv - 0.5) / zoom + pan;
}

float3 ALICE_HSVtoRGB(float3 hsv)
{
    float4 K = float4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    float3 p = abs(frac(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * lerp(K.xxx, saturate(p - K.xxx), hsv.y);
}

float2 ALICE_ComputeGradient(float2 p, float epsilon)
{
    float dx = ALICE_GradientNoise(p + float2(epsilon, 0.0)) - ALICE_GradientNoise(p - float2(epsilon, 0.0));
    float dy = ALICE_GradientNoise(p + float2(0.0, epsilon)) - ALICE_GradientNoise(p - float2(0.0, epsilon));
    return float2(dx, dy) / (2.0 * epsilon);
}

#endif // ALICE_COMMON_INCLUDED
