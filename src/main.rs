//! ALICE-View: The Infinite Canvas
//!
//! Real-time procedural rendering engine for the ALICE ecosystem.
//! "See the Math. Not the Pixels."

mod app;
mod decoder;
mod renderer;
mod ui;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use winit::event_loop::{ControlFlow, EventLoop};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("ALICE-View v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("\"See the Math. Not the Pixels.\"");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let file_path = args.get(1).cloned();

    // Create event loop
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create app instance
    let mut app = app::App::new(file_path);

    // Run event loop (winit 0.29 style)
    event_loop.run(move |event, target| {
        app.handle_event(event, target);
    })?;

    Ok(())
}
