//! ALICE Format (.alice) - Complete Parser Implementation
//!
//! "Store equations, not pixels"
//!
//! File Format:
//! ```text
//! ┌──────────────────────────────────────┐
//! │ Header (32 bytes)                    │
//! │   Magic: "ALICE" (5 bytes)           │
//! │   Version: u8                        │
//! │   Content Type: u8                   │
//! │   Flags: u8                          │
//! │   Original Size: u64 (LE)            │
//! │   Compressed Size: u64 (LE)          │
//! │   Metadata Length: u32 (LE)          │
//! │   Reserved: 4 bytes                  │
//! ├──────────────────────────────────────┤
//! │ Payload (variable)                   │
//! ├──────────────────────────────────────┤
//! │ Metadata (JSON, optional)            │
//! └──────────────────────────────────────┘
//! ```

use anyhow::{bail, Context, Result};
use std::io::{Cursor, Read};

/// ALICE file magic bytes
pub const ALICE_MAGIC: &[u8; 5] = b"ALICE";

/// Current format version
pub const ALICE_VERSION: u8 = 1;

/// Content types stored in .alice files
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliceContentType {
    /// Linear model: y = slope * x + intercept
    Linear = 0,
    /// Polynomial: y = Σ(coef[i] * x^i)
    Polynomial = 1,
    /// Perlin noise parameters
    Perlin = 2,
    /// Fractal (Mandelbrot, Julia, etc.)
    Fractal = 3,
    /// Fourier series
    Fourier = 4,
    /// Voronoi pattern
    Voronoi = 5,
    /// Sine wave composition
    SineWave = 6,
}

impl TryFrom<u8> for AliceContentType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Linear),
            1 => Ok(Self::Polynomial),
            2 => Ok(Self::Perlin),
            3 => Ok(Self::Fractal),
            4 => Ok(Self::Fourier),
            5 => Ok(Self::Voronoi),
            6 => Ok(Self::SineWave),
            _ => bail!("Unknown content type: {}", value),
        }
    }
}

impl AliceContentType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Polynomial => "Polynomial",
            Self::Perlin => "Perlin Noise",
            Self::Fractal => "Fractal",
            Self::Fourier => "Fourier Series",
            Self::Voronoi => "Voronoi",
            Self::SineWave => "Sine Wave",
        }
    }
}

/// ALICE file header (32 bytes)
#[derive(Debug, Clone)]
pub struct AliceHeader {
    pub magic: [u8; 5],
    pub version: u8,
    pub content_type: AliceContentType,
    pub flags: u8,
    pub original_size: u64,
    pub compressed_size: u64,
    pub metadata_length: u32,
}

impl AliceHeader {
    pub const SIZE: usize = 32;

    /// Parse header from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            bail!("Header too short: {} bytes (need {})", data.len(), Self::SIZE);
        }

        let mut magic = [0u8; 5];
        magic.copy_from_slice(&data[0..5]);

        if &magic != ALICE_MAGIC {
            bail!("Invalid magic: {:?} (expected {:?})", magic, ALICE_MAGIC);
        }

        let version = data[5];
        let content_type = AliceContentType::try_from(data[6])?;
        let flags = data[7];
        let original_size = u64::from_le_bytes(data[8..16].try_into()?);
        let compressed_size = u64::from_le_bytes(data[16..24].try_into()?);
        let metadata_length = u32::from_le_bytes(data[24..28].try_into()?);

        Ok(Self {
            magic,
            version,
            content_type,
            flags,
            original_size,
            compressed_size,
            metadata_length,
        })
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..5].copy_from_slice(&self.magic);
        buf[5] = self.version;
        buf[6] = self.content_type as u8;
        buf[7] = self.flags;
        buf[8..16].copy_from_slice(&self.original_size.to_le_bytes());
        buf[16..24].copy_from_slice(&self.compressed_size.to_le_bytes());
        buf[24..28].copy_from_slice(&self.metadata_length.to_le_bytes());
        buf
    }

    /// Check if file has metadata
    pub fn has_metadata(&self) -> bool {
        self.metadata_length > 0
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size > 0 {
            self.original_size as f64 / self.compressed_size as f64
        } else {
            1.0
        }
    }
}

/// Linear model payload: y = slope * x + intercept (Q16.16 fixed point)
#[derive(Debug, Clone)]
pub struct LinearPayload {
    /// Slope in Q16.16 fixed point
    pub slope_q16: i32,
    /// Intercept in Q16.16 fixed point
    pub intercept_q16: i32,
    /// Sample count (optional, for display)
    pub sample_count: u32,
}

impl LinearPayload {
    pub const SIZE: usize = 12;

    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            bail!("Linear payload too short");
        }
        let slope_q16 = i32::from_le_bytes(data[0..4].try_into()?);
        let intercept_q16 = i32::from_le_bytes(data[4..8].try_into()?);
        let sample_count = if data.len() >= 12 {
            u32::from_le_bytes(data[8..12].try_into()?)
        } else {
            0
        };

        Ok(Self {
            slope_q16,
            intercept_q16,
            sample_count,
        })
    }

    /// Convert Q16.16 to float
    pub fn slope_f32(&self) -> f32 {
        self.slope_q16 as f32 / 65536.0
    }

    pub fn intercept_f32(&self) -> f32 {
        self.intercept_q16 as f32 / 65536.0
    }

    /// Get human-readable equation string
    pub fn equation_string(&self) -> String {
        let slope = self.slope_f32();
        let intercept = self.intercept_f32();

        if slope.abs() < 0.0001 {
            format!("y = {:.4}", intercept)
        } else if intercept.abs() < 0.0001 {
            format!("y = {:.6}x", slope)
        } else if intercept >= 0.0 {
            format!("y = {:.6}x + {:.4}", slope, intercept)
        } else {
            format!("y = {:.6}x - {:.4}", slope, intercept.abs())
        }
    }

    /// Evaluate at point x
    pub fn evaluate(&self, x: i32) -> f32 {
        let mx = (self.slope_q16 as i64).wrapping_mul(x as i64);
        let q16_val = (mx as i32).wrapping_add(self.intercept_q16);
        q16_val as f32 / 65536.0
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.slope_q16.to_le_bytes());
        buf.extend_from_slice(&self.intercept_q16.to_le_bytes());
        buf.extend_from_slice(&self.sample_count.to_le_bytes());
        buf
    }
}

/// Perlin noise payload
#[derive(Debug, Clone)]
pub struct PerlinPayload {
    pub seed: u64,
    pub scale: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f32,
}

impl PerlinPayload {
    pub const SIZE: usize = 24;

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            bail!("Perlin payload too short");
        }
        Ok(Self {
            seed: u64::from_le_bytes(data[0..8].try_into()?),
            scale: f32::from_le_bytes(data[8..12].try_into()?),
            octaves: u32::from_le_bytes(data[12..16].try_into()?),
            persistence: f32::from_le_bytes(data[16..20].try_into()?),
            lacunarity: f32::from_le_bytes(data[20..24].try_into()?),
        })
    }

    pub fn equation_string(&self) -> String {
        format!(
            "FBM(seed={}, scale={:.2}, octaves={}, persistence={:.2}, lacunarity={:.2})",
            self.seed, self.scale, self.octaves, self.persistence, self.lacunarity
        )
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.seed.to_le_bytes());
        buf.extend_from_slice(&self.scale.to_le_bytes());
        buf.extend_from_slice(&self.octaves.to_le_bytes());
        buf.extend_from_slice(&self.persistence.to_le_bytes());
        buf.extend_from_slice(&self.lacunarity.to_le_bytes());
        buf
    }
}

/// Fractal payload
#[derive(Debug, Clone)]
pub struct FractalPayload {
    /// 0=Mandelbrot, 1=Julia, 2=BurningShip, 3=Tricorn
    pub fractal_type: u8,
    pub max_iterations: u32,
    pub escape_radius: f32,
    pub center_x: f64,
    pub center_y: f64,
    /// Julia set constant (optional)
    pub julia_cx: f64,
    pub julia_cy: f64,
}

impl FractalPayload {
    pub const SIZE: usize = 45;

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            bail!("Fractal payload too short");
        }
        Ok(Self {
            fractal_type: data[0],
            max_iterations: u32::from_le_bytes(data[1..5].try_into()?),
            escape_radius: f32::from_le_bytes(data[5..9].try_into()?),
            center_x: f64::from_le_bytes(data[9..17].try_into()?),
            center_y: f64::from_le_bytes(data[17..25].try_into()?),
            julia_cx: f64::from_le_bytes(data[25..33].try_into()?),
            julia_cy: f64::from_le_bytes(data[33..41].try_into()?),
        })
    }

    pub fn fractal_name(&self) -> &'static str {
        match self.fractal_type {
            0 => "Mandelbrot",
            1 => "Julia",
            2 => "Burning Ship",
            3 => "Tricorn",
            _ => "Unknown",
        }
    }

    pub fn equation_string(&self) -> String {
        match self.fractal_type {
            0 => format!(
                "Mandelbrot: z = z² + c, iter={}, center=({:.6}, {:.6})",
                self.max_iterations, self.center_x, self.center_y
            ),
            1 => format!(
                "Julia: z = z² + ({:.4}, {:.4}), iter={}",
                self.julia_cx, self.julia_cy, self.max_iterations
            ),
            2 => format!(
                "BurningShip: z = (|Re(z)| + i|Im(z)|)² + c, iter={}",
                self.max_iterations
            ),
            3 => format!(
                "Tricorn: z = conj(z)² + c, iter={}",
                self.max_iterations
            ),
            _ => "Unknown fractal".to_string(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.push(self.fractal_type);
        buf.extend_from_slice(&self.max_iterations.to_le_bytes());
        buf.extend_from_slice(&self.escape_radius.to_le_bytes());
        buf.extend_from_slice(&self.center_x.to_le_bytes());
        buf.extend_from_slice(&self.center_y.to_le_bytes());
        buf.extend_from_slice(&self.julia_cx.to_le_bytes());
        buf.extend_from_slice(&self.julia_cy.to_le_bytes());
        buf
    }
}

/// Parsed content from .alice file
#[derive(Debug, Clone)]
pub enum AlicePayload {
    Linear(LinearPayload),
    Perlin(PerlinPayload),
    Fractal(FractalPayload),
    // TODO: Polynomial, Fourier, Voronoi, SineWave
}

impl AlicePayload {
    /// Get human-readable equation string
    pub fn equation_string(&self) -> String {
        match self {
            Self::Linear(p) => p.equation_string(),
            Self::Perlin(p) => p.equation_string(),
            Self::Fractal(p) => p.equation_string(),
        }
    }
}

/// Metadata stored in .alice file (JSON)
#[derive(Debug, Clone, Default)]
pub struct AliceMetadata {
    /// Sensor ID
    pub sensor_id: Option<String>,
    /// Timestamp (ISO 8601)
    pub timestamp: Option<String>,
    /// Location
    pub location: Option<String>,
    /// Unit of measurement
    pub unit: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Custom fields (JSON)
    pub custom: Option<String>,
}

impl AliceMetadata {
    /// Parse from JSON bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Ok(Self::default());
        }

        let json_str = std::str::from_utf8(data).context("Invalid UTF-8 in metadata")?;

        // Simple JSON parsing (no serde dependency)
        let mut meta = Self::default();

        // Extract fields manually (simple approach)
        if let Some(start) = json_str.find("\"sensor_id\":\"") {
            let rest = &json_str[start + 13..];
            if let Some(end) = rest.find('"') {
                meta.sensor_id = Some(rest[..end].to_string());
            }
        }
        if let Some(start) = json_str.find("\"timestamp\":\"") {
            let rest = &json_str[start + 13..];
            if let Some(end) = rest.find('"') {
                meta.timestamp = Some(rest[..end].to_string());
            }
        }
        if let Some(start) = json_str.find("\"location\":\"") {
            let rest = &json_str[start + 12..];
            if let Some(end) = rest.find('"') {
                meta.location = Some(rest[..end].to_string());
            }
        }
        if let Some(start) = json_str.find("\"unit\":\"") {
            let rest = &json_str[start + 8..];
            if let Some(end) = rest.find('"') {
                meta.unit = Some(rest[..end].to_string());
            }
        }
        if let Some(start) = json_str.find("\"description\":\"") {
            let rest = &json_str[start + 15..];
            if let Some(end) = rest.find('"') {
                meta.description = Some(rest[..end].to_string());
            }
        }

        meta.custom = Some(json_str.to_string());
        Ok(meta)
    }

    /// Serialize to JSON bytes
    pub fn to_json(&self) -> Vec<u8> {
        let mut parts = Vec::new();
        if let Some(ref id) = self.sensor_id {
            parts.push(format!("\"sensor_id\":\"{}\"", id));
        }
        if let Some(ref ts) = self.timestamp {
            parts.push(format!("\"timestamp\":\"{}\"", ts));
        }
        if let Some(ref loc) = self.location {
            parts.push(format!("\"location\":\"{}\"", loc));
        }
        if let Some(ref unit) = self.unit {
            parts.push(format!("\"unit\":\"{}\"", unit));
        }
        if let Some(ref desc) = self.description {
            parts.push(format!("\"description\":\"{}\"", desc));
        }
        format!("{{{}}}", parts.join(",")).into_bytes()
    }
}

/// Complete parsed .alice file
#[derive(Debug, Clone)]
pub struct AliceFile {
    pub header: AliceHeader,
    pub payload: AlicePayload,
    pub metadata: AliceMetadata,
}

impl AliceFile {
    /// Parse .alice file from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let header = AliceHeader::parse(data)?;

        let payload_start = AliceHeader::SIZE;
        let payload_end = data.len() - header.metadata_length as usize;

        if payload_end < payload_start {
            bail!("Invalid payload bounds");
        }

        let payload_data = &data[payload_start..payload_end];
        let payload = match header.content_type {
            AliceContentType::Linear => AlicePayload::Linear(LinearPayload::parse(payload_data)?),
            AliceContentType::Perlin => AlicePayload::Perlin(PerlinPayload::parse(payload_data)?),
            AliceContentType::Fractal => AlicePayload::Fractal(FractalPayload::parse(payload_data)?),
            _ => bail!("Unsupported content type: {:?}", header.content_type),
        };

        let metadata = if header.has_metadata() {
            let meta_data = &data[payload_end..];
            AliceMetadata::parse(meta_data)?
        } else {
            AliceMetadata::default()
        };

        Ok(Self {
            header,
            payload,
            metadata,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload_bytes = match &self.payload {
            AlicePayload::Linear(p) => p.to_bytes(),
            AlicePayload::Perlin(p) => p.to_bytes(),
            AlicePayload::Fractal(p) => p.to_bytes(),
        };

        let meta_bytes = self.metadata.to_json();

        let mut header = self.header.clone();
        header.metadata_length = meta_bytes.len() as u32;

        let mut out = Vec::new();
        out.extend_from_slice(&header.to_bytes());
        out.extend_from_slice(&payload_bytes);
        out.extend_from_slice(&meta_bytes);
        out
    }

    /// Get equation string
    pub fn equation_string(&self) -> String {
        self.payload.equation_string()
    }

    /// Get content type name
    pub fn content_type_name(&self) -> &'static str {
        self.header.content_type.name()
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        self.header.compression_ratio()
    }
}

/// Builder for creating .alice files
pub struct AliceFileBuilder {
    content_type: AliceContentType,
    original_size: u64,
    payload: Option<AlicePayload>,
    metadata: AliceMetadata,
}

impl AliceFileBuilder {
    pub fn new(content_type: AliceContentType) -> Self {
        Self {
            content_type,
            original_size: 0,
            payload: None,
            metadata: AliceMetadata::default(),
        }
    }

    /// Create from ALICE-Edge linear model output
    pub fn from_linear(slope_q16: i32, intercept_q16: i32, sample_count: u32) -> Self {
        let mut builder = Self::new(AliceContentType::Linear);
        builder.original_size = sample_count as u64 * 4; // 4 bytes per sample
        builder.payload = Some(AlicePayload::Linear(LinearPayload {
            slope_q16,
            intercept_q16,
            sample_count,
        }));
        builder
    }

    /// Create Mandelbrot fractal
    pub fn mandelbrot(max_iterations: u32, center_x: f64, center_y: f64) -> Self {
        let mut builder = Self::new(AliceContentType::Fractal);
        builder.payload = Some(AlicePayload::Fractal(FractalPayload {
            fractal_type: 0,
            max_iterations,
            escape_radius: 2.0,
            center_x,
            center_y,
            julia_cx: 0.0,
            julia_cy: 0.0,
        }));
        builder
    }

    /// Create Julia set
    pub fn julia(max_iterations: u32, cx: f64, cy: f64) -> Self {
        let mut builder = Self::new(AliceContentType::Fractal);
        builder.payload = Some(AlicePayload::Fractal(FractalPayload {
            fractal_type: 1,
            max_iterations,
            escape_radius: 2.0,
            center_x: 0.0,
            center_y: 0.0,
            julia_cx: cx,
            julia_cy: cy,
        }));
        builder
    }

    /// Create Perlin noise
    pub fn perlin(seed: u64, scale: f32, octaves: u32) -> Self {
        let mut builder = Self::new(AliceContentType::Perlin);
        builder.payload = Some(AlicePayload::Perlin(PerlinPayload {
            seed,
            scale,
            octaves,
            persistence: 0.5,
            lacunarity: 2.0,
        }));
        builder
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: AliceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set sensor ID
    pub fn sensor_id(mut self, id: &str) -> Self {
        self.metadata.sensor_id = Some(id.to_string());
        self
    }

    /// Set timestamp
    pub fn timestamp(mut self, ts: &str) -> Self {
        self.metadata.timestamp = Some(ts.to_string());
        self
    }

    /// Set unit
    pub fn unit(mut self, unit: &str) -> Self {
        self.metadata.unit = Some(unit.to_string());
        self
    }

    /// Build the .alice file
    pub fn build(self) -> Result<AliceFile> {
        let payload = self.payload.context("Payload not set")?;

        let payload_bytes = match &payload {
            AlicePayload::Linear(p) => p.to_bytes(),
            AlicePayload::Perlin(p) => p.to_bytes(),
            AlicePayload::Fractal(p) => p.to_bytes(),
        };

        let meta_bytes = self.metadata.to_json();

        let compressed_size = AliceHeader::SIZE as u64 + payload_bytes.len() as u64 + meta_bytes.len() as u64;

        let header = AliceHeader {
            magic: *ALICE_MAGIC,
            version: ALICE_VERSION,
            content_type: self.content_type,
            flags: 0,
            original_size: if self.original_size > 0 {
                self.original_size
            } else {
                compressed_size * 100 // Estimate for non-data content
            },
            compressed_size,
            metadata_length: meta_bytes.len() as u32,
        };

        Ok(AliceFile {
            header,
            payload,
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_roundtrip() {
        let file = AliceFileBuilder::from_linear(32767, 163824115, 1000)
            .sensor_id("TEMP-001")
            .unit("°C")
            .build()
            .unwrap();

        let bytes = file.to_bytes();
        let parsed = AliceFile::parse(&bytes).unwrap();

        assert_eq!(parsed.header.content_type, AliceContentType::Linear);
        if let AlicePayload::Linear(p) = &parsed.payload {
            assert_eq!(p.slope_q16, 32767);
            assert_eq!(p.intercept_q16, 163824115);
        } else {
            panic!("Wrong payload type");
        }
    }

    #[test]
    fn test_equation_string() {
        let payload = LinearPayload {
            slope_q16: 32767,           // ~0.5
            intercept_q16: 163840000,   // ~2500
            sample_count: 1000,
        };
        let eq = payload.equation_string();
        assert!(eq.contains("y ="));
        assert!(eq.contains("x"));
    }
}
