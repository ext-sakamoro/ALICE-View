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

/// Config directory for ALICE-View
fn config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("alice-view")
}

/// Save last opened file path
fn save_recent_file(path: &str) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let recent = dir.join("recent.json");
    let data = serde_json::json!({ "last_file": path });
    let _ = std::fs::write(recent, serde_json::to_string_pretty(&data).unwrap_or_default());
}

/// Load last opened file path
fn load_recent_file() -> Option<String> {
    let recent = config_dir().join("recent.json");
    let data = std::fs::read_to_string(recent).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    json.get("last_file")?.as_str().map(|s| s.to_string())
}

fn print_usage() {
    eprintln!("ALICE-View v{} - The Infinite Canvas", env!("CARGO_PKG_VERSION"));
    eprintln!("\"See the Math. Not the Pixels.\"");
    eprintln!();
    eprintln!("Usage: alice-view [OPTIONS] [FILE]");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  [FILE]    SDF file to open (.json, .asdf, .asdf.json, .alice, .alz)");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --last         Reopen last opened file");
    eprintln!("  --width <N>    Window width (default: 1280)");
    eprintln!("  --height <N>   Window height (default: 720)");
    eprintln!("  --stats        Show performance stats on startup");
    eprintln!("  --help, -h     Show this help message");
    eprintln!("  --version, -V  Show version");
    eprintln!();
    eprintln!("Keyboard:");
    eprintln!("  WASD / QE    Camera move / up-down");
    eprintln!("  Mouse drag   Orbit camera");
    eprintln!("  Scroll       Dolly (zoom)");
    eprintln!("  R            Reset camera");
    eprintln!("  F2           Toggle stats");
    eprintln!("  F11          Fullscreen");
    eprintln!("  F12          Screenshot");
    eprintln!("  Ctrl+O       Open file");
    eprintln!();
    eprintln!("Drag & drop .json / .asdf files onto the window to view.");
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();

    let mut file_path: Option<String> = None;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut show_stats = false;
    let mut use_last = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("alice-view {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--last" => use_last = true,
            "--stats" => show_stats = true,
            "--width" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    width = val.parse().unwrap_or(1280);
                }
            }
            "--height" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    height = val.parse().unwrap_or(720);
                }
            }
            arg if !arg.starts_with('-') => {
                file_path = Some(arg.to_string());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // --last flag: reopen last file
    if file_path.is_none() && use_last {
        file_path = load_recent_file();
        if let Some(ref p) = file_path {
            tracing::info!("Reopening last file: {}", p);
        }
    }

    tracing::info!("ALICE-View v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("\"See the Math. Not the Pixels.\"");

    // Save recent file
    if let Some(ref path) = file_path {
        save_recent_file(path);
    }

    // Create event loop
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create app instance with config
    let config = app::ViewerConfig {
        width,
        height,
        show_stats,
        initial_file: file_path,
        ..Default::default()
    };
    let mut app = app::App::with_config(config);

    // Run event loop
    event_loop.run(move |event, target| {
        app.handle_event(event, target);
    })?;

    Ok(())
}
