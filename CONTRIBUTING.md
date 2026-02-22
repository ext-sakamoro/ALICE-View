# Contributing to ALICE-View

## Prerequisites

- Rust stable (latest)
- GPU with Vulkan, Metal, or DX12 support (wgpu requirement)
- `alice-sdf` crate (path dependency `../ALICE-SDF`)

## Build & Test

```bash
# Build library only
cargo build --lib

# Build both binaries (alice-view + alice-create)
cargo build

# Run all tests (lib only — binaries require GPU)
cargo test --lib

# Clippy
cargo clippy -- -W clippy::all

# Format check
cargo fmt -- --check

# Generate docs
cargo doc --no-deps --open
```

## Architecture

ALICE-View is a wgpu-based real-time renderer with two modes:

- **Procedural 2D**: Fractals, Perlin noise, sensor data rendered via WGSL compute
- **SDF 3D**: Raymarched SDF scenes from `alice-sdf` with orbit camera

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `app` | Application state machine, Camera3D, input handling |
| `decoder` | File format parsing (ALICE, ALZ, ASDF, ASP, image) |
| `renderer` | wgpu surface, pipelines, egui integration |
| `ui` | egui panels (stats, viewport, SDF controls, export) |

### Adding a New Bridge

1. Create `src/{name}_bridge.rs`
2. Add feature gate in `Cargo.toml` and `lib.rs`
3. Document license implications if the dependency is AGPL

## Code Style

- `cargo fmt` before every commit
- No `clippy::all` warnings
- `#[inline(always)]` only for per-frame camera/math functions
- All public types must have doc comments
- GPU resources (textures, buffers) must be explicitly dropped or documented

## Commit Messages

Use English, imperative mood. No auto-signatures.

## License

MIT. Copyright (c) 2026 Moroya Sakamoto.
