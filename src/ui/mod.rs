//! User interface module using egui
//!
//! "Crisp & Snappy" UI logic for ALICE-View.
//! - Zero-allocation stats collection
//! - Async file dialog (non-blocking)

mod viewport;
mod xray;
mod stats;
pub mod file_info;
pub mod sdf_panel;
pub mod export;

pub use viewport::*;
pub use xray::*;
pub use stats::*;
pub use file_info::*;
pub use sdf_panel::*;
pub use export::*;

use crate::app::{RenderMode, ViewerState, XRayType};
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
    /// SDF control panel
    sdf_panel: SdfPanel,
    /// Pending WGSL shader for pipeline rebuild (set when .asdf is loaded)
    pending_wgsl: Option<String>,
    /// Export status channel
    export_status_rx: Receiver<ExportStatus>,
    export_status_tx: Sender<ExportStatus>,
    /// Last export status message
    export_message: Option<(ExportStatus, std::time::Instant)>,
}

impl Ui {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let (etx, erx) = channel();
        Self {
            about_open: false,
            file_info_open: false,
            stats_collector: StatsCollector::new(),
            file_loader_rx: rx,
            file_loader_tx: tx,
            current_file_info: None,
            sdf_panel: SdfPanel::new(),
            pending_wgsl: None,
            export_status_rx: erx,
            export_status_tx: etx,
            export_message: None,
        }
    }

    /// Get current SDF scene ID for shader
    pub fn sdf_scene_id(&self) -> u32 {
        self.sdf_panel.scene_id()
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

    /// Start mesh export
    pub fn start_export(&self, decoder: &Decoder, format: ExportFormat, resolution: u32) {
        if let Some(sdf_content) = decoder.sdf_content() {
            export::export_mesh(sdf_content, format, resolution, self.export_status_tx.clone());
        }
    }

    /// Update UI state & Logic (non-blocking)
    pub fn update(&mut self, state: &mut ViewerState, decoder: &mut Decoder) {
        // Record frame time (O(1) ring buffer update)
        self.stats_collector.record_frame();
        state.stats.fps = self.stats_collector.fps();

        // Check export status
        while let Ok(status) = self.export_status_rx.try_recv() {
            match &status {
                ExportStatus::Done(msg) | ExportStatus::Error(msg) => {
                    tracing::info!("Export: {}", msg);
                }
                ExportStatus::Started(msg) | ExportStatus::Progress(msg) => {
                    tracing::info!("Export: {}", msg);
                }
            }
            self.export_message = Some((status, std::time::Instant::now()));
        }

        // Clear old export messages after 5 seconds
        if let Some((_, timestamp)) = &self.export_message {
            if timestamp.elapsed().as_secs() > 5 {
                self.export_message = None;
            }
        }

        // Check for pending export request from SDF panel
        if let Some(format) = self.sdf_panel.pending_export.take() {
            let resolution = self.sdf_panel.export_resolution;
            self.start_export(decoder, format, resolution);
        }

        // Check for loaded files from background thread (non-blocking)
        while let Ok(path) = self.file_loader_rx.try_recv() {
            tracing::info!("Async load complete: {}", path);
            if let Err(e) = decoder.load(&path) {
                tracing::error!("Failed to load file: {}", e);
                self.current_file_info = None;
                self.sdf_panel.set_dynamic_sdf(false, None);
            } else {
                // Check if SDF content was loaded (for .asdf files)
                if let Some(sdf_content) = decoder.sdf_content() {
                    // Generate WGSL shader for the loaded SDF
                    let wgsl = sdf_content.to_wgsl();
                    tracing::info!(
                        "Generated WGSL for SDF: {} nodes, {} bytes",
                        sdf_content.node_count,
                        wgsl.len()
                    );

                    // Store for renderer to pick up
                    self.pending_wgsl = Some(wgsl);

                    // Notify SDF panel
                    let info = format!("{} nodes", sdf_content.node_count);
                    self.sdf_panel.set_dynamic_sdf(true, Some(info));

                    // Switch to 3D mode
                    state.render_mode = RenderMode::Sdf3D;
                }

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

    /// Take pending WGSL shader (for pipeline rebuild)
    ///
    /// Returns the WGSL shader source if a new .asdf was loaded,
    /// clearing the pending state.
    pub fn take_pending_wgsl(&mut self) -> Option<String> {
        self.pending_wgsl.take()
    }

    /// Toggle file info panel
    pub fn toggle_file_info(&mut self) {
        self.file_info_open = !self.file_info_open;
    }

    /// Queue a file path for loading (used by drag-and-drop)
    pub fn queue_file(&self, path: String) {
        let _ = self.file_loader_tx.send(path);
    }

    /// Open file dialog asynchronously (non-blocking)
    fn open_file_dialog(&self) {
        let tx = self.file_loader_tx.clone();
        thread::spawn(move || {
            // Runs in background - UI continues rendering at full speed
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("ALICE SDF", &["asdf", "json"])
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
                    if ui.button("Open... (Ctrl+O)").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Screenshot (F12)").clicked() {
                        state.screenshot_requested = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
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

                // Show mode-specific info
                match state.render_mode {
                    RenderMode::Procedural2D => {
                        ui.label(format!("Zoom: {:.2}x", state.zoom));
                    }
                    RenderMode::Sdf3D => {
                        ui.label(egui::RichText::new("3D").color(egui::Color32::from_rgb(100, 200, 255)));
                        ui.separator();
                        ui.label(format!("Steps: {}", state.sdf_max_steps));
                    }
                }

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

        // 4. SDF Control Panel (only in 3D mode)
        self.sdf_panel.render(ctx, state);

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
                        ui.label("Powered by Rust + wgpu + ALICE-SDF + mimalloc");
                        ui.add_space(5.0);
                        ui.label("Author: Moroya Sakamoto");
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

        // 6. Export status toast
        if let Some((ref status, _)) = self.export_message {
            let (msg, color) = match status {
                ExportStatus::Done(m) => (m.as_str(), egui::Color32::GREEN),
                ExportStatus::Error(m) => (m.as_str(), egui::Color32::RED),
                ExportStatus::Started(m) | ExportStatus::Progress(m) => (m.as_str(), egui::Color32::YELLOW),
            };
            egui::TopBottomPanel::bottom("export_status").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(msg).color(color));
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
