//! # ALICE-View
//!
//! Real-time SDF visualizer and procedural rendering engine for the ALICE ecosystem.
//!
//! > "See the Math. Not the Pixels."
//!
//! ALICE-View renders SDF scenes from [`alice_sdf`] in real time using wgpu
//! (WebGPU). It supports both 2D procedural content (fractals, Perlin noise,
//! sensor data) and 3D SDF raymarching with orbit camera, X-Ray overlays,
//! and NPR shading.
//!
//! ## Modules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`app`] | Application state, [`Camera3D`](app::Camera3D), [`ViewerConfig`], event handling |
//! | [`decoder`] | Format loaders: `.alice`, `.alz`, `.asdf`, `.asp`, images |
//! | [`renderer`] | wgpu GPU renderer, WGSL shader pipelines, egui integration |
//! | [`ui`] | egui panels: stats, viewport, X-Ray, SDF controls, file info, export |
//!
//! ## Feature-Gated Modules
//!
//! | Feature | Module | Dependency | License |
//! |---------|--------|-----------|---------|
//! | `analytics` | `analytics_bridge` | ALICE-Analytics | MIT |
//! | `physics` | `physics_bridge` | ALICE-Physics | AGPL-3.0 |
//! | `db` | `db_bridge` | ALICE-DB | Proprietary |
//!
//! ## Supported Formats
//!
//! | Extension | Type | Render Mode |
//! |-----------|------|-------------|
//! | `.asdf`, `.asdf.json`, `.json` | SDF scene | 3D raymarching |
//! | `.alice` | ALICE binary | 2D procedural |
//! | `.alz` | ALICE zip archive | 2D procedural |
//! | `.png`, `.jpg` | Raster image | 2D raster |
//!
//! ## Library Usage
//!
//! ```rust,no_run
//! use alice_view::{ViewerConfig, launch_viewer};
//!
//! // Launch with default config (1280x720)
//! launch_viewer(ViewerConfig::default()).unwrap();
//!
//! // Launch with custom parameters
//! launch_viewer(ViewerConfig {
//!     title: "My ALICE Viewer".to_string(),
//!     initial_zoom: 2.0,
//!     initial_pan: [0.5, 0.0],
//!     show_stats: true,
//!     ..Default::default()
//! }).unwrap();
//! ```
//!
//! ## Keyboard Controls
//!
//! | Key | Action |
//! |-----|--------|
//! | WASD / QE | Camera movement (3D mode) |
//! | Mouse drag | Orbit (3D) / Pan (2D) |
//! | Scroll | Dolly zoom |
//! | R | Reset camera |
//! | M | Toggle 2D/3D mode |
//! | F1 | X-Ray overlay |
//! | F2 | Stats overlay |
//! | F11 | Fullscreen |
//! | F12 | Screenshot (PNG) |

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::module_name_repetitions,
    clippy::inline_always,
    clippy::too_many_lines
)]

#[cfg(feature = "analytics")]
pub mod analytics_bridge;
pub mod app;
#[cfg(feature = "db")]
pub mod db_bridge;
pub mod decoder;
pub mod perf;
#[cfg(feature = "physics")]
pub mod physics_bridge;
pub mod renderer;
pub mod ui;

use anyhow::Result;
use winit::event_loop::EventLoop;

// Re-export key types
pub use app::{App, FrameStats, ViewerConfig, ViewerState, XRayType};
pub use decoder::Decoder;
pub use perf::{FrameTimer, PerfCounter, PerfStats};

/// Launch the ALICE-View window with the given configuration.
///
/// This function blocks until the window is closed.
///
/// # Errors
///
/// Returns an error if the event loop cannot be created or if the window
/// event loop exits with a non-zero status.
///
/// # Example
///
/// ```rust,no_run
/// use alice_view::{ViewerConfig, launch_viewer};
///
/// launch_viewer(ViewerConfig::default()).unwrap();
/// ```
pub fn launch_viewer(config: ViewerConfig) -> Result<()> {
    // Create event loop
    let event_loop = EventLoop::new()?;

    // Create app with config
    let mut app = App::with_config(config);

    // Run event loop (winit 0.30 style)
    event_loop.run_app(&mut app)?;

    Ok(())
}

/// Launch viewer in a separate thread (non-blocking).
///
/// Returns a handle that can be used to wait for the viewer to close.
///
/// # Example
///
/// ```rust,no_run
/// use alice_view::{ViewerConfig, launch_viewer_async};
///
/// let handle = launch_viewer_async(ViewerConfig::default());
/// // Do other work...
/// handle.join().unwrap();
/// ```
#[must_use]
pub fn launch_viewer_async(config: ViewerConfig) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || launch_viewer(config))
}

/// Version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Quick launch with default settings.
///
/// # Errors
///
/// Returns an error if the viewer window cannot be created or the event loop
/// exits with an error.
pub fn quick_launch() -> Result<()> {
    launch_viewer(ViewerConfig::default())
}
