//! User interface module using egui
//!
//! "Crisp & Snappy" UI logic for ALICE-View.
//! - Zero-allocation stats collection
//! - Async file dialog (non-blocking)

mod viewport;
mod xray;
mod stats;
pub mod file_info;

pub use viewport::*;
pub use xray::*;
pub use stats::*;
pub use file_info::*;

use crate::app::{ViewerState, XRayType};
use crate::decoder::Decoder;
use egui::FullOutput;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use winit::event::WindowEvent;

/// UI state and rendering
pub struct Ui {
    /// About dialog open state
    about_open: bool,
    /// File info panel open state
    file_info_open: bool,
    /// Stats collector for performance graph (zero-allocation ring buffer)
    stats_collector: StatsCollector,
    /// Async file loading channel (receiver)
    file_loader_rx: Receiver<String>,
    /// Async file loading channel (sender for spawned threads)
    file_loader_tx: Sender<String>,
    /// Current file info
    current_file_info: Option<FileInfo>,
}

impl Ui {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            about_open: false,
            file_info_open: false,
            stats_collector: StatsCollector::new(),
            file_loader_rx: rx,
            file_loader_tx: tx,
            current_file_info: None,
        }
    }

    /// Get current file info
    pub fn file_info(&self) -> Option<&FileInfo> {
        self.current_file_info.as_ref()
    }

    /// Handle window events
    pub fn handle_event(&mut self, _event: &WindowEvent, _ctx: &egui::Context) -> egui_winit::EventResponse {
        egui_winit::EventResponse {
            consumed: false,
            repaint: false,
        }
    }

    /// Update UI state & Logic (non-blocking)
    pub fn update(&mut self, state: &mut ViewerState, decoder: &mut Decoder) {
        // Record frame time (O(1) ring buffer update)
        self.stats_collector.record_frame();
        state.stats.fps = self.stats_collector.fps();

        // Check for loaded files from background thread (non-blocking)
        while let Ok(path) = self.file_loader_rx.try_recv() {
            tracing::info!("Async load complete: {}", path);
            if let Err(e) = decoder.load(&path) {
                tracing::error!("Failed to load file: {}", e);
                self.current_file_info = None;
            } else {
                // Update file info if alice file was loaded
                if let Some(alice_file) = decoder.alice_file() {
                    self.current_file_info = Some(FileInfo::from_alice_file(alice_file, Some(&path)));
                    self.file_info_open = true; // Auto-open file info panel
                    tracing::info!("File info updated: {}", alice_file.equation_string());
                } else {
                    self.current_file_info = None;
                }
            }
        }
    }

    /// Toggle file info panel
    pub fn toggle_file_info(&mut self) {
        self.file_info_open = !self.file_info_open;
    }

    /// Open file dialog asynchronously (non-blocking)
    fn open_file_dialog(&self) {
        let tx = self.file_loader_tx.clone();
        thread::spawn(move || {
            // Runs in background - UI continues rendering at full speed
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("ALICE Files", &["alz", "alice", "asp"])
                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                .add_filter("All Files", &["*"])
                .pick_file()
            {
                let _ = tx.send(path.to_string_lossy().to_string());
            }
        });
    }

    /// Render UI
    pub fn render(&mut self, ctx: &egui::Context, state: &mut ViewerState) -> FullOutput {
        // Begin egui frame
        ctx.begin_frame(egui::RawInput::default());

        // 1. Top Menu Bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📂 Open... (Ctrl+O)").clicked() {
                        self.open_file_dialog(); // Non-blocking
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ Exit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.label(egui::RichText::new("Panels").strong());

                    // File Info Panel (F3)
                    let has_file = self.current_file_info.is_some();
                    let file_info_label = if has_file {
                        "📄 File Info (F3)"
                    } else {
                        "📄 File Info (F3) - No file loaded"
                    };
                    ui.add_enabled_ui(has_file, |ui| {
                        if ui.checkbox(&mut self.file_info_open, file_info_label).clicked() {
                            ui.close_menu();
                        }
                    });

                    // Stats Overlay (F2)
                    if ui.checkbox(&mut state.show_stats, "📊 Performance Stats (F2)").clicked() {
                        ui.close_menu();
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("Display").strong());

                    // X-Ray Mode (F1)
                    if ui.checkbox(&mut state.xray_mode, "🔬 X-Ray Mode (F1)").clicked() {
                        ui.close_menu();
                    }

                    // Pause/Play (Space)
                    if ui.checkbox(&mut state.paused, "⏸ Paused (Space)").clicked() {
                        ui.close_menu();
                    }

                    ui.separator();
                    ui.label(egui::RichText::new("X-Ray Type (Tab)").strong());

                    // Radio buttons for X-Ray mode selection
                    ui.radio_value(&mut state.xray_type, XRayType::MotionVectors, "🌊 Motion Vectors");
                    ui.radio_value(&mut state.xray_type, XRayType::FftHeatmap, "🔥 FFT Heatmap");
                    ui.radio_value(&mut state.xray_type, XRayType::EquationOverlay, "📐 Equation Overlay");
                    ui.radio_value(&mut state.xray_type, XRayType::Wireframe, "🕸️ Wireframe");

                    ui.separator();
                    ui.label(egui::RichText::new("Shortcuts").small().weak());
                    ui.label(egui::RichText::new("  F3: File Info").small().weak());
                    ui.label(egui::RichText::new("  F11: Fullscreen").small().weak());
                    ui.label(egui::RichText::new("  Scroll: Zoom").small().weak());
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("ℹ️ About").clicked() {
                        self.about_open = true;
                        ui.close_menu();
                    }
                });

                // Status indicators (inline after menus)
                ui.separator();
                ui.label(format!("Zoom: {:.2}x", state.zoom));
                ui.separator();
                if state.paused {
                    ui.label(egui::RichText::new("⏸ PAUSED").color(egui::Color32::YELLOW));
                } else {
                    ui.label(egui::RichText::new("▶ PLAYING").color(egui::Color32::GREEN));
                }
                if state.xray_mode {
                    ui.separator();
                    ui.label(egui::RichText::new("🔬 X-RAY").color(egui::Color32::from_rgb(0, 255, 255)));
                }
                if state.show_stats {
                    ui.separator();
                    ui.label(egui::RichText::new("📊 STATS").color(egui::Color32::LIGHT_BLUE));
                }
                // Show equation in status bar if file loaded
                if let Some(ref info) = self.current_file_info {
                    ui.separator();
                    ui.label(egui::RichText::new(format!("📐 {}", info.equation)).color(egui::Color32::from_rgb(255, 200, 100)));
                }
            });
        });

        // 2. Stats Overlay (mutable ref for buffer reuse)
        if state.show_stats {
            render_stats_overlay(ctx, state, &mut self.stats_collector);
        }

        // 3. X-Ray Overlay
        if state.xray_mode {
            render_xray_overlay(ctx, state);
        }

        // 4. File Info Panel (right side)
        if self.file_info_open {
            if let Some(ref info) = self.current_file_info {
                egui::SidePanel::right("file_info_panel")
                    .default_width(280.0)
                    .resizable(true)
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            info.render(ui);
                        });
                    });
            }
        }

        // 5. About Dialog
        if self.about_open {
            egui::Window::new("About ALICE-View")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("ALICE-View");
                        ui.label(egui::RichText::new("The Infinite Canvas").italics());
                        ui.add_space(10.0);
                        ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                        ui.label("Powered by Rust + wgpu + mimalloc");
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("\"See the Math. Not the Pixels.\"").italics());
                        ui.add_space(10.0);
                        ui.hyperlink("https://github.com/ext-sakamoro/ALICE-View");
                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            self.about_open = false;
                        }
                    });
                });
        }

        // End frame and return output
        ctx.end_frame()
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}
