<p align="center">
  <img src="assets/logo.png" alt="ALICE-View" width="400">
</p>

<h1 align="center">ALICE-View</h1>

<p align="center">
  <a href="https://github.com/ext-sakamoro/ALICE-View"><img src="https://img.shields.io/badge/version-0.1.0-blue.svg" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust"></a>
</p>

> **The Infinite Canvas**
> *Render the Math. See beyond the Pixels.*

ALICE-View is the official high-performance visualizer for the ALICE ecosystem (`.alz`, `.asp`).
It is not just a media player; it is a **real-time procedural rendering engine** powered by Rust and wgpu.

## Why ALICE-View?

Traditional players display static grids of pixels. ALICE-View **solves equations** 60 times per second to generate visuals.

| Traditional Player | ALICE-View |
|-------------------|------------|
| Displays pre-rendered pixels | Renders math in real-time |
| Fixed resolution | **Infinite resolution** |
| Zoom = pixelation | Zoom = recalculation |
| "What you see is what you get" | "What you see is computed" |

## Philosophy

```
"Don't just watch the video. Watch the math."
```

ALICE-View completes the ALICE ecosystem:

```
[Compress]      [Stream]       [Store]        [View]
 ALICE-Zip  →    ASP      →   ALICE-DB   →  ALICE-View
   (Math)      (Protocol)     (Model)       (Render)
```

## Features

### 🔍 Infinite Zoom

Experience the power of Procedural Compression. Since the data is stored as mathematical descriptions (fractals, curves, gradients), you can zoom indefinitely without quality loss.

```
Zoom: 1x      →     100x      →     10,000x
[Sharp]           [Sharp]           [Still Sharp!]
```

The GPU recalculates the equations at each zoom level, providing infinite detail.

### ⚡ X-Ray Debugging Mode

Press `F1` to toggle X-Ray Mode and see the underlying mathematics:

| Mode | Description |
|------|-------------|
| **Motion Vectors** | Visualize ASP streaming flow |
| **FFT Heatmap** | See the frequency domain |
| **Equation Overlay** | Display active polynomial/noise parameters |
| **Wireframe** | Show procedural mesh structure |

### 📊 Real-time Benchmark

Press `F2` to show performance overlay:

```
┌─────────────────────────────────┐
│ FPS: 144 | GPU: 12%             │
│ Decode: 2.4 GB/s                │
│ Compression: 1,450x             │
│ Resolution: ∞ (Procedural)      │
└─────────────────────────────────┘
```

### 🚀 Universal Format Support

| Format | Type | Notes |
|--------|------|-------|
| `.alz` | ALICE-Zip Archive | Full procedural support |
| `.asp` | ALICE Streaming | Real-time network streaming |
| `.alice-db` | ALICE Database | Time-series visualization |
| `.png`, `.jpg` | Standard Images | Fallback raster mode |
| `.mp4`, `.webm` | Standard Video | Fallback video mode |

## Installation

### From Cargo

```bash
cargo install alice-view
```

### From Source

```bash
git clone https://github.com/ext-sakamoro/ALICE-View.git
cd ALICE-View
cargo build --release
./target/release/alice-view
```

## Usage

### Open a file

```bash
alice-view path/to/file.alz
```

### Stream from network

```bash
alice-view asp://stream.example.com:8080
```

### Interactive mode

```bash
alice-view
# Then drag & drop files or use File → Open
```

## Controls

| Key | Action |
|-----|--------|
| `Scroll` | Zoom in/out (Infinite on procedural content) |
| `Click + Drag` | Pan viewport |
| `Space` | Pause/Play |
| `F1` | Toggle X-Ray / Debug Overlay |
| `F2` | Show Performance Stats |
| `F11` | Toggle Fullscreen |
| `Ctrl+O` | Open File |
| `Esc` | Exit |

## Tech Stack

| Component | Technology |
|-----------|------------|
| **Language** | Rust |
| **Graphics** | wgpu (WebGPU ecosystem) |
| **UI** | egui (Immediate Mode GUI) |
| **Math** | glam + noise |
| **Engine** | libalice + libasp |

### Why wgpu?

wgpu provides a unified API across:
- **Metal** (macOS/iOS)
- **Vulkan** (Linux/Android/Windows)
- **DirectX 12** (Windows)
- **WebGPU** (Browser)

One codebase, maximum performance everywhere.

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                      ALICE-View                             │
├────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Decoder    │  │   Renderer   │  │       UI         │  │
│  │  (alz/asp)   │──│    (wgpu)    │──│     (egui)       │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
│         │                 │                   │            │
│         ▼                 ▼                   ▼            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Math Engine                         │   │
│  │  Polynomial | Fourier | Perlin | Sine | Fractal     │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                 │
│                          ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  GPU Shaders                         │   │
│  │  procedural.wgsl | xray.wgsl | composite.wgsl       │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
```

## Related Projects

| Project | Description |
|---------|-------------|
| [ALICE-Zip](https://github.com/ext-sakamoro/ALICE-Zip) | Core procedural compression engine |
| [ALICE-DB](https://github.com/ext-sakamoro/ALICE-DB) | Model-based time-series database |
| [ALICE-Edge](https://github.com/ext-sakamoro/ALICE-Edge) | Embedded/IoT model generator (no_std) |
| [ALICE-Streaming-Protocol](https://github.com/ext-sakamoro/ALICE-Streaming-Protocol) | Ultra-low bandwidth video streaming |
| [ALICE-Eco-System](https://github.com/ext-sakamoro/ALICE-Eco-System) | Complete Edge-to-Cloud pipeline demo |

## License

MIT License

## Author

Moroya Sakamoto

---

*"See the Math. Not the Pixels."*
