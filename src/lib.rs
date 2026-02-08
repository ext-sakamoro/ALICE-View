//! ALICE-View: The Infinite Canvas
//!
//! Real-time procedural rendering engine for the ALICE ecosystem.
//! "See the Math. Not the Pixels."
//!
//! # Library Usage
//!
//! ```rust,no_run
//! use alice_view::{ViewerConfig, launch_viewer};
//!
//! // Launch with default config
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

pub mod app;
pub mod decoder;
pub mod renderer;
pub mod ui;
#[cfg(feature = "analytics")]
pub mod analytics_bridge;
#[cfg(feature = "physics")]
pub mod physics_bridge;

use anyhow::Result;
use winit::event_loop::{ControlFlow, EventLoop};

// Re-export key types
pub use app::{App, FrameStats, ViewerConfig, ViewerState, XRayType};
pub use decoder::Decoder;

/// Launch the ALICE-View window with the given configuration
///
/// This function blocks until the window is closed.
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
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create app with config
    let mut app = App::with_config(config);

    // Run event loop (winit 0.29 style)
    event_loop.run(move |event, target| {
        app.handle_event(event, target);
    })?;

    Ok(())
}

/// Launch viewer in a separate thread (non-blocking)
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
pub fn launch_viewer_async(config: ViewerConfig) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || launch_viewer(config))
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Quick launch with default settings
pub fn quick_launch() -> Result<()> {
    launch_viewer(ViewerConfig::default())
}

// Convenience constructors for ViewerConfig
impl ViewerConfig {
    /// Create config for displaying temperature data visualization
    pub fn for_temperature_data() -> Self {
        Self {
            title: "ALICE-View - Temperature Visualization".to_string(),
            show_stats: true,
            ..Default::default()
        }
    }

    /// Create config for fractal exploration
    pub fn for_fractal() -> Self {
        Self {
            title: "ALICE-View - Fractal Explorer".to_string(),
            initial_zoom: 1.0,
            initial_pan: [-0.5, 0.0],
            ..Default::default()
        }
    }

    /// Create minimal viewer for embedding
    pub fn minimal() -> Self {
        Self {
            title: "ALICE-View".to_string(),
            width: 800,
            height: 600,
            ..Default::default()
        }
    }

    /// Create config for viewing an SDF file
    pub fn for_sdf_file(path: &str) -> Self {
        Self {
            title: format!("ALICE-View - {}", std::path::Path::new(path).file_name().unwrap_or_default().to_string_lossy()),
            initial_file: Some(path.to_string()),
            ..Default::default()
        }
    }
}
