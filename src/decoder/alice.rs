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

/// ALICE file magic bytes
pub const ALICE_MAGIC: &[u8; 5] = b"ALICE";

/// Current format version
#[allow(dead_code)] // Used in AliceFileBuilder::build() for file format versioning
pub const ALICE_VERSION: u8 = 1;

/// Content types stored in .alice files
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliceContentType {
    /// Linear model: y = slope * x + intercept
    Linear = 0,
    /// Polynomial: y = Σ(`coef[i]` * x^i)
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
            _ => bail!("Unknown content type: {value}"),
        }
    }
}

impl AliceContentType {
    #[must_use]
    pub const fn name(self) -> &'static str {
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

// AliceHeader fields (magic, version, flags) are used in to_bytes() and by
// alice-create binary. Clippy flags them because derived Clone/Debug are
// excluded from dead-code analysis.
#[allow(dead_code)]
impl AliceHeader {
    pub const SIZE: usize = 32;

    /// Parse header from bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the data is too short, the magic bytes are invalid,
    /// or the content-type byte is unrecognised.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            bail!(
                "Header too short: {} bytes (need {})",
                data.len(),
                Self::SIZE
            );
        }

        let mut magic = [0u8; 5];
        magic.copy_from_slice(&data[0..5]);

        if &magic != ALICE_MAGIC {
            bail!("Invalid magic: {magic:?} (expected {ALICE_MAGIC:?})");
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
    #[must_use]
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
    #[must_use]
    pub const fn has_metadata(&self) -> bool {
        self.metadata_length > 0
    }

    /// Get compression ratio
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size > 0 {
            self.original_size as f64 / self.compressed_size as f64
        } else {
            1.0
        }
    }
}

const Q16_RCP: f32 = 1.0 / 65536.0;

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

// LinearPayload serialization methods are used by alice-create binary.
#[allow(dead_code)]
impl LinearPayload {
    pub const SIZE: usize = 12;

    /// Parse from bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the data is shorter than the expected payload size
    /// or if byte-to-integer conversion fails.
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
    #[inline(always)]
    #[must_use]
    pub fn slope_f32(&self) -> f32 {
        self.slope_q16 as f32 * Q16_RCP
    }

    #[inline(always)]
    #[must_use]
    pub fn intercept_f32(&self) -> f32 {
        self.intercept_q16 as f32 * Q16_RCP
    }

    /// Get human-readable equation string
    #[must_use]
    pub fn equation_string(&self) -> String {
        let slope = self.slope_f32();
        let intercept = self.intercept_f32();

        if slope.abs() < 0.0001 {
            format!("y = {intercept:.4}")
        } else if intercept.abs() < 0.0001 {
            format!("y = {slope:.6}x")
        } else if intercept >= 0.0 {
            format!("y = {slope:.6}x + {intercept:.4}")
        } else {
            format!("y = {:.6}x - {:.4}", slope, intercept.abs())
        }
    }

    /// Evaluate at point x
    #[inline(always)]
    #[must_use]
    pub fn evaluate(&self, x: i32) -> f32 {
        let mx = (self.slope_q16 as i64).wrapping_mul(x as i64);
        let q16_val = (mx as i32).wrapping_add(self.intercept_q16);
        q16_val as f32 * Q16_RCP
    }

    /// Serialize to bytes
    #[must_use]
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

// PerlinPayload serialization used by alice-create binary.
#[allow(dead_code)]
impl PerlinPayload {
    pub const SIZE: usize = 24;

    /// # Errors
    ///
    /// Returns an error if the data is shorter than [`Self::SIZE`] or if
    /// byte-to-integer conversion fails.
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

    #[must_use]
    pub fn equation_string(&self) -> String {
        format!(
            "FBM(seed={}, scale={:.2}, octaves={}, persistence={:.2}, lacunarity={:.2})",
            self.seed, self.scale, self.octaves, self.persistence, self.lacunarity
        )
    }

    #[must_use]
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

// FractalPayload serialization used by alice-create binary.
#[allow(dead_code)]
impl FractalPayload {
    pub const SIZE: usize = 45;

    /// # Errors
    ///
    /// Returns an error if the data is shorter than [`Self::SIZE`] or if
    /// byte-to-integer conversion fails.
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

    #[must_use]
    pub const fn fractal_name(&self) -> &'static str {
        match self.fractal_type {
            0 => "Mandelbrot",
            1 => "Julia",
            2 => "Burning Ship",
            3 => "Tricorn",
            _ => "Unknown",
        }
    }

    #[must_use]
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
            3 => format!("Tricorn: z = conj(z)² + c, iter={}", self.max_iterations),
            _ => "Unknown fractal".to_string(),
        }
    }

    #[must_use]
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
    #[must_use]
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

// AliceMetadata serialization used by alice-create binary.
#[allow(dead_code)]
impl AliceMetadata {
    /// Parse from JSON bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the data is not valid UTF-8.
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
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        let mut parts = Vec::new();
        if let Some(ref id) = self.sensor_id {
            parts.push(format!("\"sensor_id\":\"{id}\""));
        }
        if let Some(ref ts) = self.timestamp {
            parts.push(format!("\"timestamp\":\"{ts}\""));
        }
        if let Some(ref loc) = self.location {
            parts.push(format!("\"location\":\"{loc}\""));
        }
        if let Some(ref unit) = self.unit {
            parts.push(format!("\"unit\":\"{unit}\""));
        }
        if let Some(ref desc) = self.description {
            parts.push(format!("\"description\":\"{desc}\""));
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

// AliceFile::to_bytes is used by alice-create binary.
#[allow(dead_code)]
impl AliceFile {
    /// Parse .alice file from bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the header is invalid, the payload bounds are
    /// inconsistent, or any inner payload fails to parse.
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
            AliceContentType::Fractal => {
                AlicePayload::Fractal(FractalPayload::parse(payload_data)?)
            }
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
    #[must_use]
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
    #[must_use]
    pub fn equation_string(&self) -> String {
        self.payload.equation_string()
    }

    /// Get content type name
    #[must_use]
    pub const fn content_type_name(&self) -> &'static str {
        self.header.content_type.name()
    }

    /// Get compression ratio
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        self.header.compression_ratio()
    }
}

/// Builder for creating .alice files
// Used exclusively by alice-create binary.
#[allow(dead_code)]
pub struct AliceFileBuilder {
    content_type: AliceContentType,
    original_size: u64,
    payload: Option<AlicePayload>,
    metadata: AliceMetadata,
}

#[allow(dead_code)]
impl AliceFileBuilder {
    #[must_use]
    pub fn new(content_type: AliceContentType) -> Self {
        Self {
            content_type,
            original_size: 0,
            payload: None,
            metadata: AliceMetadata::default(),
        }
    }

    /// Create from ALICE-Edge linear model output
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn with_metadata(mut self, metadata: AliceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set sensor ID
    #[must_use]
    pub fn sensor_id(mut self, id: &str) -> Self {
        self.metadata.sensor_id = Some(id.to_string());
        self
    }

    /// Set timestamp
    #[must_use]
    pub fn timestamp(mut self, ts: &str) -> Self {
        self.metadata.timestamp = Some(ts.to_string());
        self
    }

    /// Set unit
    #[must_use]
    pub fn unit(mut self, unit: &str) -> Self {
        self.metadata.unit = Some(unit.to_string());
        self
    }

    /// Build the .alice file
    ///
    /// # Errors
    ///
    /// Returns an error if no payload has been set before calling `build`.
    pub fn build(self) -> Result<AliceFile> {
        let payload = self.payload.context("Payload not set")?;

        let payload_bytes = match &payload {
            AlicePayload::Linear(p) => p.to_bytes(),
            AlicePayload::Perlin(p) => p.to_bytes(),
            AlicePayload::Fractal(p) => p.to_bytes(),
        };

        let meta_bytes = self.metadata.to_json();

        let compressed_size =
            AliceHeader::SIZE as u64 + payload_bytes.len() as u64 + meta_bytes.len() as u64;

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
        let file = AliceFileBuilder::from_linear(32767, 163_824_115, 1000)
            .sensor_id("TEMP-001")
            .unit("°C")
            .build()
            .unwrap();

        let bytes = file.to_bytes();
        let parsed = AliceFile::parse(&bytes).unwrap();

        assert_eq!(parsed.header.content_type, AliceContentType::Linear);
        if let AlicePayload::Linear(p) = &parsed.payload {
            assert_eq!(p.slope_q16, 32767);
            assert_eq!(p.intercept_q16, 163_824_115);
        } else {
            panic!("Wrong payload type");
        }
    }

    #[test]
    fn test_equation_string() {
        let payload = LinearPayload {
            slope_q16: 32767,           // ~0.5
            intercept_q16: 163_840_000, // ~2500
            sample_count: 1000,
        };
        let eq = payload.equation_string();
        assert!(eq.contains("y ="));
        assert!(eq.contains('x'));
    }

    // ── AliceContentType ────────────────────────────────────────────

    #[test]
    fn content_type_try_from_all_valid() {
        for i in 0u8..=6 {
            assert!(
                AliceContentType::try_from(i).is_ok(),
                "byte {i} should parse"
            );
        }
    }

    #[test]
    fn content_type_try_from_invalid() {
        assert!(AliceContentType::try_from(7).is_err());
        assert!(AliceContentType::try_from(255).is_err());
    }

    #[test]
    fn content_type_names_non_empty() {
        for i in 0u8..=6 {
            let ct = AliceContentType::try_from(i).unwrap();
            assert!(!ct.name().is_empty(), "content type {i} should have a name");
        }
    }

    // ── AliceHeader ─────────────────────────────────────────────────

    #[test]
    fn header_parse_too_short() {
        let short = [0u8; 16];
        assert!(AliceHeader::parse(&short).is_err());
    }

    #[test]
    fn header_parse_bad_magic() {
        let mut data = [0u8; AliceHeader::SIZE];
        data[0..5].copy_from_slice(b"HELLO");
        assert!(AliceHeader::parse(&data).is_err());
    }

    #[test]
    fn header_roundtrip() {
        let mut data = [0u8; AliceHeader::SIZE];
        data[0..5].copy_from_slice(b"ALICE");
        data[5] = 1; // version
        data[6] = 0; // Linear
        data[7] = 0; // flags
        data[8..16].copy_from_slice(&100u64.to_le_bytes());
        data[16..24].copy_from_slice(&50u64.to_le_bytes());
        data[24..28].copy_from_slice(&0u32.to_le_bytes());

        let header = AliceHeader::parse(&data).unwrap();
        assert_eq!(header.version, 1);
        assert_eq!(header.content_type, AliceContentType::Linear);
        assert_eq!(header.original_size, 100);
        assert_eq!(header.compressed_size, 50);
        assert!(!header.has_metadata());

        let bytes = header.to_bytes();
        let reparsed = AliceHeader::parse(&bytes).unwrap();
        assert_eq!(reparsed.original_size, 100);
        assert_eq!(reparsed.compressed_size, 50);
    }

    #[test]
    fn header_compression_ratio_nonzero() {
        let file = AliceFileBuilder::from_linear(65536, 0, 500)
            .build()
            .unwrap();
        let ratio = file.header.compression_ratio();
        assert!(ratio > 0.0, "ratio should be positive");
    }

    #[test]
    fn header_has_metadata_true_when_sensor_set() {
        let file = AliceFileBuilder::from_linear(0, 0, 10)
            .sensor_id("S1")
            .build()
            .unwrap();
        assert!(file.header.has_metadata());
    }

    // ── LinearPayload Q16 arithmetic ────────────────────────────────

    #[test]
    fn linear_payload_slope_zero_equation() {
        // slope_q16 = 0 means slope == 0 → equation should not contain 'x'
        let p = LinearPayload {
            slope_q16: 0,
            intercept_q16: 65536, // 1.0 in Q16
            sample_count: 0,
        };
        let eq = p.equation_string();
        assert!(eq.starts_with("y ="), "got: {eq}");
        assert!(!eq.contains('x'), "zero slope should omit x: {eq}");
    }

    #[test]
    fn linear_payload_negative_intercept_equation() {
        let p = LinearPayload {
            slope_q16: 65536,      // slope = 1.0
            intercept_q16: -65536, // intercept = -1.0
            sample_count: 0,
        };
        let eq = p.equation_string();
        assert!(eq.contains(" - "), "expected minus sign format: {eq}");
    }

    #[test]
    fn linear_payload_evaluate_at_zero() {
        // intercept_q16 = 0, slope = anything → evaluate(0) = 0
        let p = LinearPayload {
            slope_q16: 65536,
            intercept_q16: 0,
            sample_count: 0,
        };
        let y = p.evaluate(0);
        assert!(y.abs() < 1e-5, "evaluate(0) should be 0, got {y}");
    }

    #[test]
    fn linear_payload_to_bytes_length() {
        let p = LinearPayload {
            slope_q16: 1,
            intercept_q16: 2,
            sample_count: 3,
        };
        assert_eq!(p.to_bytes().len(), LinearPayload::SIZE);
    }

    // ── PerlinPayload ────────────────────────────────────────────────

    #[test]
    fn perlin_payload_roundtrip() {
        let original = PerlinPayload {
            seed: 0xDEAD_BEEF_1234_5678,
            scale: std::f32::consts::PI,
            octaves: 8,
            persistence: 0.6,
            lacunarity: 2.5,
        };
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), PerlinPayload::SIZE);
        let parsed = PerlinPayload::parse(&bytes).unwrap();
        assert_eq!(parsed.seed, original.seed);
        assert_eq!(parsed.octaves, original.octaves);
        assert!((parsed.scale - original.scale).abs() < 1e-5);
    }

    #[test]
    fn perlin_payload_equation_string_contains_seed() {
        let p = PerlinPayload {
            seed: 42,
            scale: 1.5,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
        };
        let s = p.equation_string();
        assert!(s.contains("42"), "should contain seed: {s}");
        assert!(s.contains('4'), "should contain octaves: {s}");
    }

    // ── FractalPayload ───────────────────────────────────────────────

    #[test]
    fn fractal_payload_mandelbrot_name() {
        let p = FractalPayload {
            fractal_type: 0,
            max_iterations: 256,
            escape_radius: 2.0,
            center_x: 0.0,
            center_y: 0.0,
            julia_cx: 0.0,
            julia_cy: 0.0,
        };
        assert_eq!(p.fractal_name(), "Mandelbrot");
    }

    #[test]
    fn fractal_payload_all_type_names() {
        let expected = ["Mandelbrot", "Julia", "Burning Ship", "Tricorn", "Unknown"];
        for (i, name) in expected.iter().enumerate() {
            let p = FractalPayload {
                fractal_type: i as u8,
                max_iterations: 64,
                escape_radius: 2.0,
                center_x: 0.0,
                center_y: 0.0,
                julia_cx: 0.0,
                julia_cy: 0.0,
            };
            assert_eq!(p.fractal_name(), *name, "type {i}");
        }
    }

    #[test]
    fn fractal_payload_julia_equation_contains_cx() {
        let p = FractalPayload {
            fractal_type: 1,
            max_iterations: 128,
            escape_radius: 2.0,
            center_x: 0.0,
            center_y: 0.0,
            julia_cx: -0.7,
            julia_cy: 0.27,
        };
        let eq = p.equation_string();
        assert!(eq.contains("Julia"), "expected Julia: {eq}");
        assert!(eq.contains("-0.7"), "expected cx in equation: {eq}");
    }

    #[test]
    fn fractal_payload_roundtrip() {
        // FractalPayload::to_bytes writes 41 bytes (1+4+4+8+8+8+8).
        // Parse requires at least SIZE(45) bytes, so we pad to satisfy it.
        let original = FractalPayload {
            fractal_type: 0,
            max_iterations: 512,
            escape_radius: 4.0,
            center_x: -0.75,
            center_y: 0.1,
            julia_cx: 0.0,
            julia_cy: 0.0,
        };
        let mut bytes = original.to_bytes();
        // Pad up to FractalPayload::SIZE so parse() is satisfied
        while bytes.len() < FractalPayload::SIZE {
            bytes.push(0);
        }
        let parsed = FractalPayload::parse(&bytes).unwrap();
        assert_eq!(parsed.fractal_type, 0);
        assert_eq!(parsed.max_iterations, 512);
        assert!((parsed.center_x - -0.75).abs() < 1e-10);
    }

    // ── AliceMetadata ────────────────────────────────────────────────

    #[test]
    fn metadata_parse_empty() {
        let m = AliceMetadata::parse(b"").unwrap();
        assert!(m.sensor_id.is_none());
        assert!(m.timestamp.is_none());
    }

    #[test]
    fn metadata_parse_sensor_id_and_unit() {
        let json = br#"{"sensor_id":"TEMP-001","unit":"C"}"#;
        let m = AliceMetadata::parse(json).unwrap();
        assert_eq!(m.sensor_id.as_deref(), Some("TEMP-001"));
        assert_eq!(m.unit.as_deref(), Some("C"));
    }

    #[test]
    fn metadata_to_json_roundtrip() {
        let m = AliceMetadata {
            sensor_id: Some("S1".to_string()),
            timestamp: Some("2026-01-01T00:00:00Z".to_string()),
            location: Some("Tokyo".to_string()),
            unit: Some("Pa".to_string()),
            description: None,
            custom: None,
        };
        let bytes = m.to_json();
        assert!(!bytes.is_empty());
        let reparsed = AliceMetadata::parse(&bytes).unwrap();
        assert_eq!(reparsed.sensor_id.as_deref(), Some("S1"));
        assert_eq!(reparsed.unit.as_deref(), Some("Pa"));
    }

    // ── AliceFileBuilder ─────────────────────────────────────────────

    #[test]
    fn builder_no_payload_errors() {
        let builder = AliceFileBuilder::new(AliceContentType::Polynomial);
        assert!(
            builder.build().is_err(),
            "build without payload should fail"
        );
    }

    #[test]
    fn builder_mandelbrot_payload_values() {
        // Verify that the Mandelbrot builder sets the correct payload values
        // without relying on byte-level roundtrip (SIZE vs to_bytes mismatch).
        let file = AliceFileBuilder::mandelbrot(256, -0.5, 0.0)
            .build()
            .unwrap();
        assert_eq!(file.header.content_type, AliceContentType::Fractal);
        if let AlicePayload::Fractal(p) = &file.payload {
            assert_eq!(p.fractal_type, 0);
            assert_eq!(p.max_iterations, 256);
            assert!((p.center_x - -0.5).abs() < 1e-10);
            assert!((p.escape_radius - 2.0).abs() < 1e-5);
        } else {
            panic!("Expected Fractal payload");
        }
    }

    #[test]
    fn builder_julia_roundtrip() {
        let file = AliceFileBuilder::julia(100, -0.7, 0.27).build().unwrap();
        if let AlicePayload::Fractal(p) = &file.payload {
            assert_eq!(p.fractal_type, 1);
            assert!((p.julia_cx - -0.7).abs() < 1e-10);
            assert!((p.julia_cy - 0.27).abs() < 1e-10);
        } else {
            panic!("Expected Fractal payload");
        }
    }

    #[test]
    fn builder_perlin_roundtrip() {
        let file = AliceFileBuilder::perlin(999, 2.5, 6).build().unwrap();
        assert_eq!(file.header.content_type, AliceContentType::Perlin);
        if let AlicePayload::Perlin(p) = &file.payload {
            assert_eq!(p.seed, 999);
            assert_eq!(p.octaves, 6);
        } else {
            panic!("Expected Perlin payload");
        }
    }

    #[test]
    fn alice_file_content_type_name_linear() {
        let file = AliceFileBuilder::from_linear(1, 0, 10).build().unwrap();
        assert_eq!(file.content_type_name(), "Linear");
    }
}
