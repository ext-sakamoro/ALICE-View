//! ALICE-Zip (.alz) decoder

// AlzHeader, AlzContentType and their methods are the public ALZ wire format
// API. Not all are instantiated within this crate but are required for
// external ALZ stream consumers.
#![allow(dead_code)]

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
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alz_header_valid_magic() {
        let header = AlzHeader {
            magic: *b"ALICE",
            version: 1,
            content_type: 0,
            flags: 0,
            original_size: 1000,
            compressed_size: 100,
        };
        assert!(header.is_valid());
    }

    #[test]
    fn alz_header_invalid_magic() {
        let header = AlzHeader {
            magic: *b"WRONG",
            version: 1,
            content_type: 0,
            flags: 0,
            original_size: 1000,
            compressed_size: 100,
        };
        assert!(!header.is_valid());
    }

    #[test]
    fn alz_content_type_all_valid() {
        assert_eq!(AlzContentType::try_from(0u8), Ok(AlzContentType::RawLzma));
        assert_eq!(AlzContentType::try_from(1u8), Ok(AlzContentType::Perlin));
        assert_eq!(
            AlzContentType::try_from(2u8),
            Ok(AlzContentType::Polynomial)
        );
        assert_eq!(AlzContentType::try_from(3u8), Ok(AlzContentType::Sine));
        assert_eq!(AlzContentType::try_from(4u8), Ok(AlzContentType::Fourier));
        assert_eq!(AlzContentType::try_from(5u8), Ok(AlzContentType::Fractal));
    }

    #[test]
    fn alz_content_type_invalid() {
        assert!(AlzContentType::try_from(6u8).is_err());
        assert!(AlzContentType::try_from(255u8).is_err());
    }

    #[test]
    fn alz_magic_constant() {
        assert_eq!(&AlzHeader::MAGIC, b"ALICE");
    }
}
