//! Decoder module for ALICE formats (Async & Optimized)
//!
//! - Async I/O to avoid blocking UI thread
//! - spawn_blocking for heavy image decode
//! - Arc<Vec<u8>> for zero-copy raster data

pub mod alice;
pub mod asdf;
mod alz;
mod asp;

pub use alice::*;
pub use alz::*;
pub use asdf::*;
pub use asp::*;

use anyhow::{Context, Result};
use glam::DVec2;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

/// Content type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// No content loaded
    None,
    /// ALICE-Zip procedural content
    AliceZip,
    /// ALICE Streaming Protocol
    AspStream,
    /// ALICE-SDF 3D content
    AliceSdf,
    /// Standard image (fallback)
    Image,
    /// Standard video (fallback)
    Video,
}

/// Decoded procedural content
#[derive(Debug, Clone)]
pub enum ProceduralContent {
    /// Perlin noise parameters
    Perlin {
        seed: u64,
        scale: f32,
        octaves: u32,
        persistence: f32,
        lacunarity: f32,
    },
    /// Polynomial coefficients
    Polynomial {
        coefficients: Vec<f64>,
    },
    /// Sine wave parameters
    SineWave {
        frequency: f32,
        amplitude: f32,
        phase: f32,
    },
    /// Fourier series
    Fourier {
        coefficients: Vec<(usize, f32, f32)>, // (frequency, amplitude, phase)
    },
    /// Fractal parameters (Mandelbrot, Julia, etc.)
    Fractal {
        fractal_type: FractalType,
        max_iterations: u32,
        escape_radius: f32,
        center: DVec2,
        julia_c: Option<DVec2>,
    },
    /// Raster image data (RGBA8) - Arc for zero-copy sharing
    Raster {
        width: u32,
        height: u32,
        data: Arc<Vec<u8>>,
    },
}

/// Fractal types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalType {
    Mandelbrot,
    Julia,
    BurningShip,
    Tricorn,
}

/// Main decoder (Async-capable)
pub struct Decoder {
    content_type: ContentType,
    content: Option<ProceduralContent>,
    file_path: Option<String>,
    /// Statistics
    original_size: u64,
    compressed_size: u64,
    /// Loaded ALICE file (for file info display)
    alice_file: Option<alice::AliceFile>,
    /// Loaded SDF content (for 3D visualization)
    sdf_content: Option<asdf::SdfContent>,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            content_type: ContentType::None,
            content: None,
            file_path: None,
            original_size: 0,
            compressed_size: 0,
            alice_file: None,
            sdf_content: None,
        }
    }

    /// Get loaded SDF content (if available)
    pub fn sdf_content(&self) -> Option<&asdf::SdfContent> {
        self.sdf_content.as_ref()
    }

    /// Get loaded ALICE file (if available)
    pub fn alice_file(&self) -> Option<&alice::AliceFile> {
        self.alice_file.as_ref()
    }

    /// Load content from file path (synchronous wrapper for compatibility)
    /// For async loading, use load_async() instead
    pub fn load(&mut self, path: &str) -> Result<()> {
        let p = Path::new(path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let path_str_cow = p.to_string_lossy();

        // ASDF/SDF files: load synchronously (avoids Tokio runtime requirement)
        if path_str_cow.ends_with(".asdf.json") || path_str_cow.ends_with(".asdf") || ext == "json" {
            return self.load_asdf_sync(path);
        }

        // Other formats: use pollster to block on async
        pollster::block_on(self.load_async(path))
    }

    /// Load ASDF/SDF file synchronously (no Tokio runtime required)
    fn load_asdf_sync(&mut self, path: &str) -> Result<()> {
        let p = Path::new(path);
        tracing::info!("Loading ASDF/SDF file (sync): {:?}", p);

        self.file_path = Some(path.to_string());
        self.alice_file = None;
        self.sdf_content = None;

        let sdf_content = asdf::SdfContent::load(p)?;

        let metadata = std::fs::metadata(p)?;
        let file_size = metadata.len();
        let estimated_original = file_size * 100;

        tracing::info!(
            "ASDF loaded: {} nodes, bounds: {:?} - {:?}",
            sdf_content.node_count,
            sdf_content.bounds.0,
            sdf_content.bounds.1
        );

        self.sdf_content = Some(sdf_content);
        self.content_type = ContentType::AliceSdf;
        self.content = None;
        self.original_size = estimated_original;
        self.compressed_size = file_size;

        Ok(())
    }

    /// Load content asynchronously (non-blocking)
    pub async fn load_async(&mut self, path: &str) -> Result<()> {
        let path = Path::new(path);
        let path_buf = path.to_path_buf();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        self.file_path = Some(path.to_string_lossy().to_string());
        self.alice_file = None; // Reset
        self.sdf_content = None; // Reset

        // Check for SDF files first (compound extension .asdf.json, binary .asdf, or plain .json)
        let path_str = path.to_string_lossy();
        if path_str.ends_with(".asdf.json") || path_str.ends_with(".asdf") || extension == "json" {
            return self.load_asdf_async(path_buf).await;
        }

        let (content, c_type, o_size, c_size, alice_file) = match extension.as_str() {
            "alz" | "alice" => Self::load_alice_async(path_buf).await?,
            "asp" => {
                let (c, t, o, s) = Self::load_asp_async(path_buf).await?;
                (c, t, o, s, None)
            }
            "png" | "jpg" | "jpeg" | "bmp" | "gif" => {
                let (c, t, o, s) = Self::load_image_async(path_buf).await?;
                (c, t, o, s, None)
            }
            "mp4" | "webm" | "avi" | "mov" => {
                anyhow::bail!("Video playback not yet implemented");
            }
            _ => anyhow::bail!("Unknown file format: {}", extension),
        };

        self.content = Some(content);
        self.content_type = c_type;
        self.original_size = o_size;
        self.compressed_size = c_size;
        self.alice_file = alice_file;

        Ok(())
    }

    /// Load ASDF file (SDF 3D content)
    async fn load_asdf_async(&mut self, path: PathBuf) -> Result<()> {
        tracing::info!("Loading ASDF file: {:?}", path);

        // Load SDF in blocking thread (file I/O)
        let sdf_content = tokio::task::spawn_blocking(move || {
            asdf::SdfContent::load(&path)
        })
        .await
        .context("Spawn blocking task failed")??;

        // Get file size for stats
        let metadata = fs::metadata(&self.file_path.as_ref().unwrap()).await?;
        let file_size = metadata.len();

        // Estimate original size (mesh equivalent would be much larger)
        // SDF is extremely compact compared to mesh representation
        let estimated_original = file_size * 100; // Conservative estimate

        tracing::info!(
            "ASDF loaded: {} nodes, bounds: {:?} - {:?}",
            sdf_content.node_count,
            sdf_content.bounds.0,
            sdf_content.bounds.1
        );

        self.sdf_content = Some(sdf_content);
        self.content_type = ContentType::AliceSdf;
        self.content = None; // SDF uses separate content
        self.original_size = estimated_original;
        self.compressed_size = file_size;

        Ok(())
    }

    /// Load ALICE file (Async)
    async fn load_alice_async(path: PathBuf) -> Result<(ProceduralContent, ContentType, u64, u64, Option<alice::AliceFile>)> {
        tracing::info!("Loading ALICE file (Async): {:?}", path);

        // Read file contents
        let data = fs::read(&path).await.context("Failed to read file")?;

        // Try to parse as .alice format first
        if data.len() >= 5 && &data[0..5] == b"ALICE" {
            let alice_file = alice::AliceFile::parse(&data)?;
            tracing::info!("Parsed ALICE file: {}", alice_file.equation_string());

            let content = match &alice_file.payload {
                alice::AlicePayload::Linear(p) => {
                    // Convert to Perlin visualization with slope as scale
                    ProceduralContent::Perlin {
                        seed: p.slope_q16 as u64,
                        scale: p.slope_f32().abs() * 10.0 + 1.0,
                        octaves: 6,
                        persistence: 0.5,
                        lacunarity: 2.0,
                    }
                }
                alice::AlicePayload::Perlin(p) => ProceduralContent::Perlin {
                    seed: p.seed,
                    scale: p.scale,
                    octaves: p.octaves,
                    persistence: p.persistence,
                    lacunarity: p.lacunarity,
                },
                alice::AlicePayload::Fractal(p) => ProceduralContent::Fractal {
                    fractal_type: match p.fractal_type {
                        0 => FractalType::Mandelbrot,
                        1 => FractalType::Julia,
                        2 => FractalType::BurningShip,
                        3 => FractalType::Tricorn,
                        _ => FractalType::Mandelbrot,
                    },
                    max_iterations: p.max_iterations,
                    escape_radius: p.escape_radius,
                    center: DVec2::new(p.center_x, p.center_y),
                    julia_c: if p.fractal_type == 1 {
                        Some(DVec2::new(p.julia_cx, p.julia_cy))
                    } else {
                        None
                    },
                },
            };

            let o_size = alice_file.header.original_size;
            let c_size = alice_file.header.compressed_size;

            return Ok((
                content,
                ContentType::AliceZip,
                o_size,
                c_size,
                Some(alice_file),
            ));
        }

        // Fallback: legacy ALZ format or demo content
        let metadata = fs::metadata(&path).await.context("Failed to read metadata")?;
        let compressed_size = metadata.len();

        let content = ProceduralContent::Fractal {
            fractal_type: FractalType::Mandelbrot,
            max_iterations: 256,
            escape_radius: 2.0,
            center: DVec2::new(-0.75, 0.0),
            julia_c: None,
        };

        Ok((content, ContentType::AliceZip, compressed_size * 500, compressed_size, None))
    }

    /// Load ASP stream file (Async)
    async fn load_asp_async(path: PathBuf) -> Result<(ProceduralContent, ContentType, u64, u64)> {
        tracing::info!("Loading ASP stream (Async): {:?}", path);

        let metadata = fs::metadata(&path).await.context("Failed to read metadata")?;
        let compressed_size = metadata.len();

        // TODO: Implement actual ASP parsing
        log::warn!("ASP stream parsing not yet implemented — returning placeholder Perlin content");
        let content = ProceduralContent::Perlin {
            seed: 12345,
            scale: 5.0,
            octaves: 8,
            persistence: 0.5,
            lacunarity: 2.0,
        };

        Ok((content, ContentType::AspStream, compressed_size * 1000, compressed_size))
    }

    /// Load standard image (Async + spawn_blocking for heavy decode)
    async fn load_image_async(path: PathBuf) -> Result<(ProceduralContent, ContentType, u64, u64)> {
        tracing::info!("Loading image (Async): {:?}", path);

        // Offload heavy image decoding to blocking thread pool
        let result = tokio::task::spawn_blocking(move || -> Result<(ProceduralContent, u64, u64)> {
            let img = image::open(&path).context("Failed to open image")?;
            let rgba = img.to_rgba8(); // Convert to RGBA for GPU upload
            let (width, height) = rgba.dimensions();
            let raw_data = rgba.into_raw();
            let original_size = (width * height * 4) as u64;
            let compressed_size = std::fs::metadata(&path)?.len();

            tracing::info!("Image decoded: {}x{}, {} bytes", width, height, original_size);

            Ok((
                ProceduralContent::Raster {
                    width,
                    height,
                    data: Arc::new(raw_data), // Zero-copy sharing
                },
                original_size,
                compressed_size,
            ))
        })
        .await
        .context("Spawn blocking task failed")??;

        Ok((result.0, ContentType::Image, result.1, result.2))
    }

    /// Get content type
    pub fn content_type(&self) -> ContentType {
        self.content_type
    }

    /// Get procedural content
    pub fn content(&self) -> Option<&ProceduralContent> {
        self.content.as_ref()
    }

    /// Get file path
    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.compressed_size > 0 {
            self.original_size as f32 / self.compressed_size as f32
        } else {
            1.0
        }
    }

    /// Check if content is procedural (infinite zoom capable)
    pub fn is_procedural(&self) -> bool {
        matches!(
            self.content,
            Some(ProceduralContent::Perlin { .. })
                | Some(ProceduralContent::Polynomial { .. })
                | Some(ProceduralContent::SineWave { .. })
                | Some(ProceduralContent::Fourier { .. })
                | Some(ProceduralContent::Fractal { .. })
        )
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
