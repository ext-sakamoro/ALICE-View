//! ALICE-Zip (.alz) decoder

/// ALICE-Zip file header
#[repr(C)]
pub struct AlzHeader {
    /// Magic bytes: "ALICE"
    pub magic: [u8; 5],
    /// Version
    pub version: u8,
    /// Content type
    pub content_type: u8,
    /// Flags
    pub flags: u8,
    /// Original data size
    pub original_size: u64,
    /// Compressed size
    pub compressed_size: u64,
}

impl AlzHeader {
    pub const MAGIC: [u8; 5] = *b"ALICE";

    /// Validate header
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// ALZ content types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlzContentType {
    /// Raw LZMA compressed data
    RawLzma = 0,
    /// Perlin noise parameters
    Perlin = 1,
    /// Polynomial coefficients
    Polynomial = 2,
    /// Sine wave parameters
    Sine = 3,
    /// Fourier series
    Fourier = 4,
    /// Fractal parameters
    Fractal = 5,
}

impl TryFrom<u8> for AlzContentType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RawLzma),
            1 => Ok(Self::Perlin),
            2 => Ok(Self::Polynomial),
            3 => Ok(Self::Sine),
            4 => Ok(Self::Fourier),
            5 => Ok(Self::Fractal),
            _ => Err(()),
        }
    }
}
