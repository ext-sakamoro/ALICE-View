<p align="center">
  <img src="assets/logo.png" alt="ALICE-View" width="400">
</p>

<h1 align="center">ALICE-View</h1>

<p align="center">
  <a href="https://github.com/ext-sakamoro/ALICE-View"><img src="https://img.shields.io/badge/version-0.2.0-blue.svg" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust"></a>
</p>

> **The Infinite Canvas**
> *Render the Math. See beyond the Pixels.*

ALICE-View is a high-performance real-time 3D SDF visualizer and procedural rendering engine for the ALICE ecosystem. Powered by Rust, wgpu, and [ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF).

## Features

### 3D SDF Raymarching

Real-time GPU raymarching of SDF (Signed Distance Function) models via WGSL shaders transpiled by ALICE-SDF.

- Load `.json`, `.asdf`, `.asdf.json` SDF files
- Drag & drop files onto the window
- Orbit camera with mouse, WASD movement
- Adjustable lighting (direction, intensity, ambient, background color)
- Lighting presets (Sunset, Studio, Flat)
- Raymarching controls (max steps, epsilon)
- Normal visualization and ambient occlusion toggle

### Mesh Export

Export loaded SDF models to standard 3D formats via ALICE-SDF's Marching Cubes mesher:

| Format | Description |
|--------|-------------|
| `.glb` | glTF 2.0 Binary (recommended) |
| `.obj` | Wavefront OBJ |

Adjustable export resolution (16-256).

### Screenshot

Press `F12` to capture a PNG screenshot (saved to Desktop).

### Infinite Zoom (Procedural 2D)

For procedural content (`.alz`, `.asp`), zoom indefinitely without quality loss. The GPU recalculates equations at each zoom level.

### X-Ray Debugging Mode

Press `F1` to toggle X-Ray Mode:

| Mode | Description |
|------|-------------|
| **Motion Vectors** | Visualize ASP streaming flow |
| **FFT Heatmap** | See the frequency domain |
| **Equation Overlay** | Display active parameters |
| **Wireframe** | Show procedural mesh structure |

## Installation

### From Source

```bash
git clone https://github.com/ext-sakamoro/ALICE-View.git
cd ALICE-View
cargo build --release
```

The binary is at `target/release/alice-view`.

### Install to PATH

```bash
cargo install --path .
```

## Usage

### Open an SDF file

```bash
alice-view model.json
alice-view scene.asdf
```

### Reopen last file

```bash
alice-view --last
```

### Interactive mode

```bash
alice-view
# Drag & drop files or use File > Open
```

### Options

```
alice-view [OPTIONS] [FILE]

Arguments:
  [FILE]         SDF file to open (.json, .asdf, .asdf.json)

Options:
  --last         Reopen last opened file
  --width <N>    Window width (default: 1280)
  --height <N>   Window height (default: 720)
  --stats        Show performance stats on startup
  --help, -h     Show help
  --version, -V  Show version
```

## Controls

### 3D Mode (SDF)

| Key | Action |
|-----|--------|
| `WASD` | Move camera |
| `Q / E` | Camera up / down |
| `Mouse drag` | Orbit camera |
| `Scroll` | Dolly (zoom) |
| `R` | Reset camera |
| `N` | Toggle normal visualization |
| `O` | Toggle ambient occlusion |
| `M` | Toggle 2D/3D mode |

### General

| Key | Action |
|-----|--------|
| `F1` | Toggle X-Ray mode |
| `F2` | Toggle performance stats |
| `F3` | Toggle file info panel |
| `F11` | Toggle fullscreen |
| `F12` | Screenshot (PNG) |
| `Space` | Pause / Play |

## Supported Formats

| Format | Type | Mode |
|--------|------|------|
| `.json` | SDF JSON (ALICE-SDF) | 3D Raymarching |
| `.asdf` | ALICE-SDF Binary | 3D Raymarching |
| `.asdf.json` | ALICE-SDF JSON | 3D Raymarching |
| `.alz` / `.alice` | ALICE-Zip Archive | 2D Procedural |
| `.asp` | ALICE Streaming | 2D Procedural |
| `.png`, `.jpg` | Standard Images | Raster fallback |

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                       ALICE-View v0.2.0                      │
├──────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐  │
│  │   Decoder    │  │   Renderer    │  │       UI         │  │
│  │  asdf/alice  │──│  wgpu + WGSL  │──│  egui panels     │  │
│  └──────────────┘  └───────────────┘  └──────────────────┘  │
│         │                  │                    │            │
│         ▼                  ▼                    ▼            │
│  ┌────────────────────────────────────────────────────┐     │
│  │              ALICE-SDF Integration                  │     │
│  │  SdfTree → WgslShader (transpile) → GPU Raymarch   │     │
│  │  SdfTree → MarchingCubes → GLB/OBJ Export          │     │
│  └────────────────────────────────────────────────────┘     │
│         │                                                    │
│         ▼                                                    │
│  ┌────────────────────────────────────────────────────┐     │
│  │              GPU Shaders (WGSL)                     │     │
│  │  raymarching.wgsl | procedural.wgsl                 │     │
│  │  + dynamic SDF shaders (transpiled at runtime)      │     │
│  └────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| **Language** | Rust |
| **Graphics** | wgpu (WebGPU) |
| **SDF Engine** | [ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF) |
| **UI** | egui |
| **Math** | glam |
| **Allocator** | mimalloc |

## Library Usage

```rust
use alice_view::{ViewerConfig, launch_viewer};

// Launch with default settings
launch_viewer(ViewerConfig::default()).unwrap();

// Launch with SDF file
launch_viewer(ViewerConfig::for_sdf_file("model.json")).unwrap();

// Custom configuration
launch_viewer(ViewerConfig {
    title: "My Viewer".to_string(),
    width: 1920,
    height: 1080,
    show_stats: true,
    ..Default::default()
}).unwrap();
```

## Related Projects

| Project | Description |
|---------|-------------|
| [ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF) | SDF library with 36 primitives, SIMD eval, mesh export, WGSL/HLSL/GLSL transpilers |
| [ALICE-Zip](https://github.com/ext-sakamoro/ALICE-Zip) | Core procedural compression engine |
| [ALICE-DB](https://github.com/ext-sakamoro/ALICE-DB) | Model-based time-series database |
| [ALICE-Edge](https://github.com/ext-sakamoro/ALICE-Edge) | Embedded/IoT model generator (no_std) |
| [ALICE-Streaming-Protocol](https://github.com/ext-sakamoro/ALICE-Streaming-Protocol) | Ultra-low bandwidth video streaming |

## License

MIT License

## Author

Moroya Sakamoto

---

*"See the Math. Not the Pixels."*
