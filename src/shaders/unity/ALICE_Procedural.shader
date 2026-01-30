// ALICE Procedural Content Shader for Unity
// "Store equations, not pixels" - GPU-computed infinite resolution
//
// Usage: Create a Material with this shader and adjust parameters

Shader "ALICE/Procedural"
{
    Properties
    {
        [Header(Transform)]
        _Zoom ("Zoom", Range(0.001, 1000000)) = 1.0
        _PanX ("Pan X", Float) = 0.0
        _PanY ("Pan Y", Float) = 0.0

        [Header(Content)]
        [Enum(Perlin,0,Mandelbrot,1,Julia,2,Voronoi,3,Plasma,4,Wireframe,5)]
        _ContentType ("Content Type", Int) = 0

        [Header(Perlin Settings)]
        _Scale ("Scale", Range(0.1, 100)) = 10.0
        _Octaves ("Octaves", Range(1, 12)) = 6

        [Header(Fractal Settings)]
        _MaxIterations ("Max Iterations", Range(16, 512)) = 256
        _JuliaX ("Julia C.x", Range(-2, 2)) = -0.7
        _JuliaY ("Julia C.y", Range(-2, 2)) = 0.27

        [Header(Voronoi Settings)]
        _EdgeWidth ("Edge Width", Range(0.001, 0.2)) = 0.05

        [Header(Animation)]
        _AnimationSpeed ("Animation Speed", Range(0, 5)) = 1.0
    }

    SubShader
    {
        Tags { "RenderType"="Opaque" "Queue"="Geometry" }
        LOD 100

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 3.5

            #include "UnityCG.cginc"
            #include "ALICE_Common.cginc"

            struct appdata
            {
                float4 vertex : POSITION;
                float2 uv : TEXCOORD0;
            };

            struct v2f
            {
                float2 uv : TEXCOORD0;
                float4 vertex : SV_POSITION;
            };

            // Properties
            float _Zoom;
            float _PanX;
            float _PanY;
            int _ContentType;
            float _Scale;
            int _Octaves;
            int _MaxIterations;
            float _JuliaX;
            float _JuliaY;
            float _EdgeWidth;
            float _AnimationSpeed;

            v2f vert(appdata v)
            {
                v2f o;
                o.vertex = UnityObjectToClipPos(v.vertex);
                o.uv = v.uv;
                return o;
            }

            // ============================================
            // Content Generators
            // ============================================

            float3 GeneratePerlin(float2 uv)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));
                float n = ALICE_FBM(worldPos * _Scale, _Octaves, 0.5, 2.0);

                // Terrain coloring
                float3 low = float3(0.1, 0.2, 0.4);
                float3 mid = float3(0.3, 0.6, 0.3);
                float3 high = float3(0.9, 0.8, 0.6);

                if (n < 0.5)
                    return lerp(low, mid, n * 2.0);
                else
                    return lerp(mid, high, (n - 0.5) * 2.0);
            }

            float3 FractalColor(float t, float time)
            {
                float hue = t + time * 0.1;
                return float3(
                    0.5 + 0.5 * cos(6.28318 * hue),
                    0.5 + 0.5 * cos(6.28318 * (hue + 0.333)),
                    0.5 + 0.5 * cos(6.28318 * (hue + 0.666))
                );
            }

            float3 GenerateMandelbrot(float2 uv, float time)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));
                float2 c = worldPos * 3.0 + float2(-0.5, 0.0);

                float t = ALICE_Mandelbrot(c, _MaxIterations, 2.0);

                if (t == 0.0)
                    return float3(0.0, 0.0, 0.0);

                return FractalColor(t, time);
            }

            float3 GenerateJulia(float2 uv, float time)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));
                float2 z = worldPos * 2.0;
                float2 c = float2(_JuliaX, _JuliaY);

                float t = ALICE_Julia(z, c, _MaxIterations, 2.0);

                if (t == 0.0)
                    return float3(0.0, 0.0, 0.0);

                return FractalColor(t, time);
            }

            float3 GenerateVoronoi(float2 uv)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));
                float2 p = worldPos * _Scale;

                ALICE_VoronoiResult vor = ALICE_Voronoi(p, 1.0);

                float3 cellColor = float3(
                    ALICE_Hash2(vor.cellCenter),
                    ALICE_Hash2(vor.cellCenter + float2(17.0, 31.0)),
                    ALICE_Hash2(vor.cellCenter + float2(73.0, 89.0))
                );

                float edge = smoothstep(0.0, _EdgeWidth, vor.distance);
                return cellColor * edge;
            }

            float3 GeneratePlasma(float2 uv, float time)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));

                float v1 = sin(worldPos.x * 10.0 + time);
                float v2 = sin(worldPos.y * 10.0 + time * 0.5);
                float v3 = sin((worldPos.x + worldPos.y) * 10.0 + time * 0.7);
                float v4 = sin(length(worldPos * 10.0) + time);

                float v = (v1 + v2 + v3 + v4) * 0.25 + 0.5;

                return ALICE_HSVtoRGB(float3(v, 0.8, 0.9));
            }

            float3 GenerateWireframe(float2 uv, float time)
            {
                float2 worldPos = ALICE_TransformUV(uv, _Zoom, float2(_PanX, _PanY));
                float gridSize = 0.1;
                float2 p = worldPos / gridSize;
                float2 f = frac(p);

                float minDist = min(min(f.y, f.x), abs(f.x + f.y - 1.0) / 1.414);
                float wire = 1.0 - smoothstep(0.0, 0.02, minDist);
                float glow = exp(-minDist * 10.0) * 0.3;

                float vertexDist = min(
                    min(length(f), length(f - float2(1.0, 0.0))),
                    min(length(f - float2(0.0, 1.0)), length(f - float2(1.0, 1.0)))
                );
                float vertex = 1.0 - smoothstep(0.0, 0.05, vertexDist);

                float pulse = 0.5 + 0.5 * sin(time * 2.0 - length(worldPos) * 3.0);

                float3 color = float3(0.02, 0.05, 0.08);
                color += float3(0.0, 1.0, 1.0) * wire;
                color += float3(0.0, 0.3, 0.4) * glow;
                color += float3(1.0, 1.0, 1.0) * vertex * 0.8;
                color *= 0.7 + 0.3 * pulse;

                return color;
            }

            fixed4 frag(v2f i) : SV_Target
            {
                float time = _Time.y * _AnimationSpeed;
                float3 color;

                switch (_ContentType)
                {
                    case 0: color = GeneratePerlin(i.uv); break;
                    case 1: color = GenerateMandelbrot(i.uv, time); break;
                    case 2: color = GenerateJulia(i.uv, time); break;
                    case 3: color = GenerateVoronoi(i.uv); break;
                    case 4: color = GeneratePlasma(i.uv, time); break;
                    case 5: color = GenerateWireframe(i.uv, time); break;
                    default: color = float3(0.5, 0.5, 0.5); break;
                }

                // Subtle vignette
                float vignette = 1.0 - length(i.uv - 0.5) * 0.3;
                color *= vignette;

                return fixed4(color, 1.0);
            }
            ENDCG
        }
    }

    FallBack "Diffuse"
}
