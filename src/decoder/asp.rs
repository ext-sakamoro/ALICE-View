//! ALICE Streaming Protocol (.asp) decoder

// ASP structs and enums define the streaming wire protocol.
// Full decoding is not yet implemented; types are stubs for future work.
#![allow(dead_code)]

/// ASP packet types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspPacketType {
    /// I-Packet: Keyframe (full procedural description)
    IPacket = 0x49,
    /// D-Packet: Delta (incremental updates + motion vectors)
    DPacket = 0x44,
    /// C-Packet: Correction (ROI-based pixel corrections)
    CPacket = 0x43,
    /// S-Packet: Sync (flow control commands)
    SPacket = 0x53,
}

impl TryFrom<u8> for AspPacketType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x49 => Ok(Self::IPacket),
            0x44 => Ok(Self::DPacket),
            0x43 => Ok(Self::CPacket),
            0x53 => Ok(Self::SPacket),
            _ => Err(()),
        }
    }
}

/// ASP packet header (16 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AspHeader {
    /// Magic: "ASP\x01"
    pub magic: [u8; 4],
    /// Packet type
    pub packet_type: u8,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub reserved: u16,
    /// Sequence number
    pub sequence: u32,
    /// Payload size
    pub payload_size: u32,
}

impl AspHeader {
    pub const MAGIC: [u8; 4] = *b"ASP\x01";

    /// Validate header
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// Motion vector (compact, 2 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MotionVectorCompact {
    pub dx: i8,
    pub dy: i8,
}

/// Stream state for ASP decoding
pub struct AspStreamState {
    /// Last keyframe data
    pub keyframe: Option<KeyframeData>,
    /// Current sequence number
    pub sequence: u32,
    /// Accumulated motion vectors
    pub motion_vectors: Vec<MotionVectorCompact>,
}

/// Keyframe data
#[derive(Debug, Clone)]
pub struct KeyframeData {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    /// Procedural parameters
    pub params: Vec<u8>,
}

impl AspStreamState {
    pub fn new() -> Self {
        Self {
            keyframe: None,
            sequence: 0,
            motion_vectors: Vec::new(),
        }
    }

    /// Process incoming packet
    pub fn process_packet(&mut self, _data: &[u8]) -> Result<(), &'static str> {
        log::warn!("process_packet() is a stub — ASP packet processing not yet implemented");
        // TODO: Implement actual packet processing
        Ok(())
    }
}

impl Default for AspStreamState {
    fn default() -> Self {
        Self::new()
    }
}
