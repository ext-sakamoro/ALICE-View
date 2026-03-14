<p align="center">
  <img src="assets/logo.png" alt="ALICE-View" width="400">
</p>

<h1 align="center">ALICE-View</h1>

<p align="center">
  <a href="https://github.com/ext-sakamoro/ALICE-View"><img src="https://img.shields.io/badge/version-0.3.0-blue.svg" alt="Version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust"></a>
  <a href="#quality"><img src="https://img.shields.io/badge/clippy-pedantic%200-brightgreen.svg" alt="Clippy"></a>
  <a href="#quality"><img src="https://img.shields.io/badge/tests-76%20pass-brightgreen.svg" alt="Tests"></a>
</p>

> **The Infinite Canvas**
> *Render the Math. See beyond the Pixels.*

ALICE-View is a high-performance real-time 3D SDF visualizer and procedural rendering engine for the ALICE ecosystem. Powered by Rust, wgpu, and [ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF).

## Features

### 3D SDF Raymarching

Real-time GPU raymarching of SDF (Signed Distance Function) models via WGSL shaders transpiled by ALICE-SDF.

- Load `.json`, `.asdf`, `.asdf.json`, `.lol` (LOL DSL) files
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

### Pre-built Binaries

Download from [Releases](https://github.com/ext-sakamoro/ALICE-View/releases):

| Platform | File |
|----------|------|
| **Windows x86_64** | `alice-view-windows-x86_64-vX.Y.Z.zip` |
| **Linux x86_64** | `alice-view-linux-x86_64-vX.Y.Z.tar.gz` |
| **macOS aarch64** | `alice-view-macos-aarch64-vX.Y.Z.tar.gz` |

**macOS**: Binaries are not code-signed. Remove the quarantine attribute after extracting:

```bash
tar xzf alice-view-macos-aarch64-vX.Y.Z.tar.gz
xattr -cr alice-view alice-create
./alice-view
```

## Usage

### Open an SDF file

```bash
alice-view model.json
alice-view scene.asdf
alice-view sword.lol       # LOL DSL テキストファイル
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
  [FILE]         SDF file to open (.json, .asdf, .asdf.json, .lol)

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
| `.lol` | [ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL) DSL | 3D Raymarching |
| `.alz` / `.alice` | ALICE-Zip Archive | 2D Procedural |
| `.asp` | ALICE Streaming | 2D Procedural |
| `.png`, `.jpg` | Standard Images | Raster fallback |

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                       ALICE-View v0.3.0                      │
├──────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐  │
│  │   Decoder    │  │   Renderer    │  │       UI         │  │
│  │  asdf/alice  │──│  wgpu + WGSL  │──│  egui panels     │  │
│  └──────────────┘  └───────────────┘  └──────────────────┘  │
│         │                  │                    │            │
│         ▼                  ▼                    ▼            │
│  ┌────────────────────────────────────────────────────┐     │
│  │              ALICE-SDF + ALICE-LOL Integration         │     │
│  │  .lol → parse_lol() → SdfNode                        │     │
│  │  SdfTree → WgslShader (transpile) → GPU Raymarch      │     │
│  │  SdfTree → MarchingCubes → GLB/OBJ Export             │     │
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
| **SDF DSL** | [ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL) (120 constructs) |
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

## Cross-Crate Bridges

ALICE-View connects to other ALICE ecosystem crates via feature-gated bridge modules:

| Bridge | Feature | Target Crate | Description |
|--------|---------|--------------|-------------|
| LOL loader | `lol` (default) | [ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL) | `.lol` DSL file loading via `runtime_parser::parse_lol()` |
| `analytics_bridge` | `analytics` | [ALICE-Analytics](../ALICE-Analytics) | Real-time rendering performance analytics |
| `physics_bridge` | `physics` | [ALICE-Physics](../ALICE-Physics) | Interactive physics debug overlay visualization |
| `db_bridge` | `db` | [ALICE-DB](../ALICE-DB) | Time-series DB query to plot data for GPU overlay |

### Cargo Profile

Standardized `[profile.bench]` added for consistent benchmarking across ALICE crates.

## Related Projects

| Project | Description |
|---------|-------------|
| [ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF) | SDF library with 126 primitives/ops, SIMD eval, mesh export, WGSL/HLSL/GLSL transpilers |
| [ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL) | Law-Oriented Language — 120-construct DSL for declarative SDF scene authoring |
| [Open Source SDF Assets](https://github.com/ext-sakamoro/Open-Source-SDF-Assets) | 991 free CC0 3D assets in .asdf.json format |
| [ALICE-Zip](https://github.com/ext-sakamoro/ALICE-Zip) | Core procedural compression engine |
| [ALICE-DB](https://github.com/ext-sakamoro/ALICE-DB) | Model-based time-series database |
| [ALICE-Edge](https://github.com/ext-sakamoro/ALICE-Edge) | Embedded/IoT model generator (no_std) |
| [ALICE-Streaming-Protocol](https://github.com/ext-sakamoro/ALICE-Streaming-Protocol) | Ultra-low bandwidth video streaming |

## Performance

### Adaptive Quality (v0.3.0)

Heavy SDF scenes (deep CSG trees, complex smooth operations) can be GPU-intensive. Adaptive Quality reduces work for distant pixels without visible quality loss on near surfaces.

| Technique | Effect |
|-----------|--------|
| **Adaptive Epsilon** | Epsilon grows with ray distance (`eps * (1 + t * 0.1)`), allowing early termination for far pixels |
| **Over-Relaxation** | Step size increases with distance (`d * (1 + t * 0.05)`), reducing total steps for background rays |
| **Adaptive AO** | AO samples reduced from 5 to 2 for surfaces beyond `t > 20.0` |

### How to Use

Open the **Raymarching** section in the left SDF panel:

1. **Quality Preset** — Select Fast / Balanced / Quality / Ultra from the dropdown. Each preset automatically sets Max Steps, Epsilon, and AO.
2. **Adaptive Quality** — Toggle the checkbox (on by default). When enabled, distant pixels use fewer raymarching steps for better FPS.
3. **Manual Tuning** — Use the Max Steps slider and Epsilon slider below to fine-tune after selecting a preset.

**Tip**: If a complex SDF scene (e.g. `Teleporter01_Art.asdf.json`) is slow, switch to **Fast** preset first, then increase quality as needed.

### Quality Presets

| Preset | Max Steps | Epsilon | AO | Use Case |
|--------|-----------|---------|-----|----------|
| **Fast** | 64 | 0.01 | Off | Preview, low-end GPU |
| **Balanced** | 128 | 0.001 | On | Default, general use |
| **Quality** | 256 | 0.0001 | On | Final renders |
| **Ultra** | 512 | 0.00001 | On | Maximum detail |

## Quality

| Gate | Status |
|------|--------|
| `cargo clippy -- -W clippy::pedantic` | **0 warnings** |
| `cargo doc --no-deps` | **0 warnings** |
| `cargo fmt -- --check` | **0 diff** |
| `cargo test --lib` | **76 passed, 0 failed** |

## License

MIT License

## Author

Moroya Sakamoto

---

*"See the Math. Not the Pixels."*
