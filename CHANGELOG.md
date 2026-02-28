# Changelog

All notable changes to ALICE-View are documented here.

## [0.3.0] — 2026-02-28

### Added
- **Adaptive Quality** — Distance-based epsilon scaling and step over-relaxation for faster rendering of heavy SDF scenes
- **Quality Presets** — Fast / Balanced / Quality / Ultra one-click presets for raymarching settings
- **Adaptive AO** — Ambient occlusion sample count reduced for distant surfaces (5 → 2 when `t > 20.0`)
- Quality Preset combo box and Adaptive Quality checkbox in SDF panel
- `QualityPreset` enum with `apply_quality_preset()` for programmatic preset application
- `quality_flags` uniform field (replaces `_pad3` at offset 92) for GPU-side feature toggles
- 8 new unit tests for `QualityPreset` and adaptive quality defaults

## [0.2.1] — 2026-02-28

### Changed
- Clippy pedantic 0 warnings across all targets (lib + binaries)
- Added `#![allow]` blocks to binary crate roots for package-wide consistency
- Changed `&self` to `self` on small Copy types (`AliceContentType::name`, `ExportFormat::extension/filter_name`, `SdfScene::name`)
- Fixed `alice-create` format strings, clone efficiency, long literal separators, doc backticks
- Fixed wildcard import in `alice-create` to explicit imports
- Added `#[allow]` annotations on stub methods (`asp::process_packet`, `ui::handle_event`)
- Fixed `physics_bridge.rs` test imports

## [0.2.0] — 2026-02-23

### Added
- **SDF 3D raymarching** — wgpu pipeline with WGSL shaders for real-time SDF rendering
- **ASDF format loader** — `.asdf`, `.asdf.json`, `.json` SDF scene files
- **Camera3D** — Orbit, dolly, pan with `#[inline(always)]` hot methods
- **SDF panel** — UI controls for 3D scene parameters, mesh export (GLB/OBJ)
- **X-Ray overlays** — MotionVectors, FftHeatmap, EquationOverlay, Wireframe
- **File info panel** — Metadata display with compression ratio
- **Stats collector** — Zero-alloc ring buffer for frame timing (O(1) per frame)
- **Export** — GLB and OBJ mesh export from SDF scenes
- **ViewerConfig constructors** — `for_fractal()`, `minimal()`, `for_sdf_file()`, `for_temperature_data()`
- **Bridge modules** — `analytics_bridge`, `physics_bridge`, `db_bridge` (feature-gated)
- **alice-create binary** — CLI tool for creating `.alice` files (linear, polynomial, fractal, Perlin, demo)

### Changed
- Migrated to wgpu 0.19 / winit 0.29 / egui 0.27
- Async I/O via tokio `spawn_blocking` for non-blocking file loading

## [0.1.0] — 2026-01-15

### Added
- **Procedural 2D rendering** — Perlin noise, fractals (Mandelbrot, Julia, BurningShip, Tricorn), linear/polynomial sensor data
- **ALICE format decoder** — `.alice` binary format with header, payload, metadata, CRC32
- **ALZ format decoder** — Zip archive format
- **egui UI** — Viewport controls, stats overlay, file dialog
- **Infinite zoom** — Procedural zoom with LOD adaptation
- **Screenshot** — F12 to capture PNG
- Release profile: `opt-level=3`, `lto=fat`, `codegen-units=1`, `strip=true`, `panic=abort`
- 68 unit tests
