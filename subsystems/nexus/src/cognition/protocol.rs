//! # Cognitive Protocol
//!
//! Protocol definitions for cognitive communication.
//! Standardizes message formats and communication patterns.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// PROTOCOL TYPES
// ============================================================================

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProtocolVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const CURRENT: Self = Self::new(1, 0, 0);

    /// Check compatibility
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

/// Message frame
#[derive(Debug, Clone)]
pub struct MessageFrame {
    /// Frame header
    pub header: FrameHeader,
    /// Payload
    pub payload: Vec<u8>,
    /// Trailer (checksum, etc.)
    pub trailer: FrameTrailer,
}

/// Frame header
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Protocol version
    pub version: ProtocolVersion,
    /// Frame type
    pub frame_type: FrameType,
    /// Message ID
    pub message_id: u64,
    /// Source domain
    pub source: DomainId,
    /// Target domain
    pub target: DomainId,
    /// Sequence number
    pub sequence: u64,
    /// Flags
    pub flags: FrameFlags,
    /// Payload length
    pub payload_length: u32,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// Data frame
    Data      = 0,
    /// Control frame
    Control   = 1,
    /// Acknowledgment
    Ack       = 2,
    /// Negative acknowledgment
    Nack      = 3,
    /// Heartbeat
    Heartbeat = 4,
    /// Error
    Error     = 5,
    /// Ping
    Ping      = 6,
    /// Pong
    Pong      = 7,
}

/// Frame flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags {
    bits: u16,
}

impl FrameFlags {
    /// Request acknowledgment
    pub const ACK_REQUIRED: u16 = 0x0001;
    /// Fragmented message
    pub const FRAGMENTED: u16 = 0x0002;
    /// Last fragment
    pub const LAST_FRAGMENT: u16 = 0x0004;
    /// Priority message
    pub const PRIORITY: u16 = 0x0008;
    /// Compressed payload
    pub const COMPRESSED: u16 = 0x0010;
    /// Encrypted payload
    pub const ENCRYPTED: u16 = 0x0020;

    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn set(&mut self, flag: u16) {
        self.bits |= flag;
    }

    pub fn clear(&mut self, flag: u16) {
        self.bits &= !flag;
    }

    pub fn is_set(&self, flag: u16) -> bool {
        self.bits & flag != 0
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }
}

/// Frame trailer
#[derive(Debug, Clone)]
pub struct FrameTrailer {
    /// CRC32 checksum
    pub checksum: u32,
}

impl FrameTrailer {
    pub fn new(payload: &[u8]) -> Self {
        Self {
            checksum: Self::calculate_crc(payload),
        }
    }

    pub fn calculate_crc(data: &[u8]) -> u32 {
        // Simple CRC32 placeholder
        let mut crc: u32 = 0xFFFFFFFF;
        for byte in data {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB88320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    pub fn verify(&self, payload: &[u8]) -> bool {
        self.checksum == Self::calculate_crc(payload)
    }
}

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// Control message
#[derive(Debug, Clone)]
pub enum ControlMessage {
    /// Connect request
    Connect(ConnectRequest),
    /// Connect response
    ConnectAck(ConnectAck),
    /// Disconnect
    Disconnect(DisconnectReason),
    /// Configuration update
    Configure(BTreeMap<String, String>),
    /// Flow control
    FlowControl(FlowControlMessage),
    /// Keep-alive
    KeepAlive,
}

/// Connect request
#[derive(Debug, Clone)]
pub struct ConnectRequest {
    /// Client version
    pub version: ProtocolVersion,
    /// Client ID
    pub client_id: String,
    /// Requested capabilities
    pub capabilities: Vec<String>,
    /// Authentication token
    pub auth_token: Option<Vec<u8>>,
}

/// Connect acknowledgment
#[derive(Debug, Clone)]
pub struct ConnectAck {
    /// Accepted
    pub accepted: bool,
    /// Server version
    pub version: ProtocolVersion,
    /// Session ID
    pub session_id: u64,
    /// Granted capabilities
    pub capabilities: Vec<String>,
    /// Rejection reason
    pub reason: Option<String>,
}

/// Disconnect reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Normal close
    Normal,
    /// Going away
    GoingAway,
    /// Protocol error
    ProtocolError,
    /// Timeout
    Timeout,
    /// Overload
    Overload,
    /// Administrative
    Administrative,
}

/// Flow control message
#[derive(Debug, Clone)]
pub enum FlowControlMessage {
    /// Pause sending
    Pause,
    /// Resume sending
    Resume,
    /// Window update
    WindowUpdate(u32),
    /// Backpressure
    Backpressure(f32),
}

// ============================================================================
// PROTOCOL HANDLER
// ============================================================================

/// Protocol handler
pub struct ProtocolHandler {
    /// Local domain
    local_domain: DomainId,
    /// Sessions
    sessions: BTreeMap<u64, ProtocolSession>,
    /// Pending fragments
    pending_fragments: BTreeMap<u64, FragmentBuffer>,
    /// Next message ID
    next_message_id: AtomicU64,
    /// Next session ID
    next_session_id: AtomicU64,
    /// Sequence number
    sequence: AtomicU64,
    /// Configuration
    config: ProtocolConfig,
    /// Statistics
    stats: ProtocolStats,
}

/// Protocol session
#[derive(Debug, Clone)]
pub struct ProtocolSession {
    /// Session ID
    pub id: u64,
    /// Remote domain
    pub remote: DomainId,
    /// State
    pub state: SessionState,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Created time
    pub created: Timestamp,
    /// Last activity
    pub last_activity: Timestamp,
    /// Sent messages
    pub sent: u64,
    /// Received messages
    pub received: u64,
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Closing
    Closing,
    /// Closed
    Closed,
}

/// Fragment buffer
#[derive(Debug)]
struct FragmentBuffer {
    /// Original message ID
    message_id: u64,
    /// Expected fragments
    expected: usize,
    /// Received fragments
    fragments: Vec<Option<Vec<u8>>>,
    /// Started time
    started: Timestamp,
}

/// Protocol configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Maximum frame size
    pub max_frame_size: usize,
    /// Maximum payload size
    pub max_payload_size: usize,
    /// Fragment threshold
    pub fragment_threshold: usize,
    /// Session timeout (ns)
    pub session_timeout_ns: u64,
    /// Fragment timeout (ns)
    pub fragment_timeout_ns: u64,
    /// Keep-alive interval (ns)
    pub keepalive_interval_ns: u64,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            max_frame_size: 65536,
            max_payload_size: 1048576, // 1MB
            fragment_threshold: 32768,
            session_timeout_ns: 300_000_000_000,   // 5 minutes
            fragment_timeout_ns: 30_000_000_000,   // 30 seconds
            keepalive_interval_ns: 30_000_000_000, // 30 seconds
        }
    }
}

/// Protocol statistics
#[derive(Debug, Clone, Default)]
pub struct ProtocolStats {
    /// Frames sent
    pub frames_sent: u64,
    /// Frames received
    pub frames_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Checksum errors
    pub checksum_errors: u64,
    /// Protocol errors
    pub protocol_errors: u64,
    /// Active sessions
    pub active_sessions: u64,
}

impl ProtocolHandler {
    /// Create a new protocol handler
    pub fn new(local_domain: DomainId, config: ProtocolConfig) -> Self {
        Self {
            local_domain,
            sessions: BTreeMap::new(),
            pending_fragments: BTreeMap::new(),
            next_message_id: AtomicU64::new(1),
            next_session_id: AtomicU64::new(1),
            sequence: AtomicU64::new(0),
            config,
            stats: ProtocolStats::default(),
        }
    }

    /// Create a data frame
    pub fn create_data_frame(
        &mut self,
        target: DomainId,
        payload: Vec<u8>,
        ack_required: bool,
    ) -> Result<Vec<MessageFrame>, &'static str> {
        if payload.len() > self.config.max_payload_size {
            return Err("Payload too large");
        }

        let message_id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        // Check if fragmentation needed
        if payload.len() > self.config.fragment_threshold {
            return self.create_fragmented_frames(target, message_id, payload, ack_required);
        }

        let mut flags = FrameFlags::new();
        if ack_required {
            flags.set(FrameFlags::ACK_REQUIRED);
        }

        let header = FrameHeader {
            version: ProtocolVersion::CURRENT,
            frame_type: FrameType::Data,
            message_id,
            source: self.local_domain,
            target,
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            flags,
            payload_length: payload.len() as u32,
            timestamp: Timestamp::now(),
        };

        let trailer = FrameTrailer::new(&payload);

        Ok(vec![MessageFrame {
            header,
            payload,
            trailer,
        }])
    }

    /// Create fragmented frames
    fn create_fragmented_frames(
        &mut self,
        target: DomainId,
        message_id: u64,
        payload: Vec<u8>,
        ack_required: bool,
    ) -> Result<Vec<MessageFrame>, &'static str> {
        let chunk_size = self.config.fragment_threshold;
        let chunks: Vec<_> = payload.chunks(chunk_size).collect();
        let mut frames = Vec::with_capacity(chunks.len());

        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;

            let mut flags = FrameFlags::new();
            flags.set(FrameFlags::FRAGMENTED);
            if is_last {
                flags.set(FrameFlags::LAST_FRAGMENT);
            }
            if ack_required {
                flags.set(FrameFlags::ACK_REQUIRED);
            }

            let header = FrameHeader {
                version: ProtocolVersion::CURRENT,
                frame_type: FrameType::Data,
                message_id,
                source: self.local_domain,
                target,
                sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
                flags,
                payload_length: chunk.len() as u32,
                timestamp: Timestamp::now(),
            };

            let payload_vec = chunk.to_vec();
            let trailer = FrameTrailer::new(&payload_vec);

            frames.push(MessageFrame {
                header,
                payload: payload_vec,
                trailer,
            });
        }

        Ok(frames)
    }

    /// Create an acknowledgment frame
    pub fn create_ack(&mut self, target: DomainId, ack_message_id: u64) -> MessageFrame {
        let header = FrameHeader {
            version: ProtocolVersion::CURRENT,
            frame_type: FrameType::Ack,
            message_id: self.next_message_id.fetch_add(1, Ordering::Relaxed),
            source: self.local_domain,
            target,
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            flags: FrameFlags::new(),
            payload_length: 8,
            timestamp: Timestamp::now(),
        };

        let payload = ack_message_id.to_le_bytes().to_vec();
        let trailer = FrameTrailer::new(&payload);

        MessageFrame {
            header,
            payload,
            trailer,
        }
    }

    /// Create a heartbeat frame
    pub fn create_heartbeat(&mut self, target: DomainId) -> MessageFrame {
        let header = FrameHeader {
            version: ProtocolVersion::CURRENT,
            frame_type: FrameType::Heartbeat,
            message_id: self.next_message_id.fetch_add(1, Ordering::Relaxed),
            source: self.local_domain,
            target,
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            flags: FrameFlags::new(),
            payload_length: 0,
            timestamp: Timestamp::now(),
        };

        MessageFrame {
            header,
            payload: Vec::new(),
            trailer: FrameTrailer::new(&[]),
        }
    }

    /// Process received frame
    pub fn process_frame(&mut self, frame: MessageFrame) -> Result<Option<Vec<u8>>, &'static str> {
        // Verify checksum
        if !frame.trailer.verify(&frame.payload) {
            self.stats.checksum_errors += 1;
            return Err("Checksum verification failed");
        }

        // Check version compatibility
        if !frame
            .header
            .version
            .is_compatible(&ProtocolVersion::CURRENT)
        {
            self.stats.protocol_errors += 1;
            return Err("Incompatible protocol version");
        }

        self.stats.frames_received += 1;
        self.stats.bytes_received += frame.payload.len() as u64;

        // Update session activity
        if let Some(session) = self
            .sessions
            .values_mut()
            .find(|s| s.remote == frame.header.source)
        {
            session.last_activity = Timestamp::now();
            session.received += 1;
        }

        // Handle frame types
        match frame.header.frame_type {
            FrameType::Data => {
                if frame.header.flags.is_set(FrameFlags::FRAGMENTED) {
                    self.handle_fragment(frame)
                } else {
                    Ok(Some(frame.payload))
                }
            },
            FrameType::Ack => {
                // ACK handling - just acknowledge receipt
                Ok(None)
            },
            FrameType::Heartbeat | FrameType::Ping => {
                // Respond with pong (handled elsewhere)
                Ok(None)
            },
            FrameType::Pong => Ok(None),
            FrameType::Control => {
                // Control message handling
                Ok(Some(frame.payload))
            },
            FrameType::Nack | FrameType::Error => {
                self.stats.protocol_errors += 1;
                Err("Received error frame")
            },
        }
    }

    /// Handle fragmented message
    fn handle_fragment(&mut self, frame: MessageFrame) -> Result<Option<Vec<u8>>, &'static str> {
        let message_id = frame.header.message_id;
        let is_last = frame.header.flags.is_set(FrameFlags::LAST_FRAGMENT);

        // Get or create fragment buffer
        if !self.pending_fragments.contains_key(&message_id) {
            self.pending_fragments.insert(message_id, FragmentBuffer {
                message_id,
                expected: 0,
                fragments: Vec::new(),
                started: Timestamp::now(),
            });
        }

        let buffer = self.pending_fragments.get_mut(&message_id).unwrap();
        let seq = frame.header.sequence as usize;

        // Expand fragments vector if needed
        while buffer.fragments.len() <= seq {
            buffer.fragments.push(None);
        }

        buffer.fragments[seq] = Some(frame.payload);

        if is_last {
            buffer.expected = seq + 1;
        }

        // Check if complete
        if buffer.expected > 0
            && buffer
                .fragments
                .iter()
                .take(buffer.expected)
                .all(|f| f.is_some())
        {
            let fragments: Vec<_> = buffer
                .fragments
                .iter()
                .take(buffer.expected)
                .filter_map(|f| f.clone())
                .collect();

            self.pending_fragments.remove(&message_id);

            // Reassemble
            let complete: Vec<u8> = fragments.into_iter().flatten().collect();
            Ok(Some(complete))
        } else {
            Ok(None)
        }
    }

    /// Start a session
    pub fn start_session(&mut self, remote: DomainId, capabilities: Vec<String>) -> u64 {
        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let session = ProtocolSession {
            id,
            remote,
            state: SessionState::Connecting,
            capabilities,
            created: now,
            last_activity: now,
            sent: 0,
            received: 0,
        };

        self.sessions.insert(id, session);
        self.stats.active_sessions = self.sessions.len() as u64;

        id
    }

    /// Complete session connection
    pub fn connect_session(&mut self, session_id: u64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.state = SessionState::Connected;
        }
    }

    /// Close a session
    pub fn close_session(&mut self, session_id: u64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.state = SessionState::Closed;
        }
        self.sessions.remove(&session_id);
        self.stats.active_sessions = self.sessions.len() as u64;
    }

    /// Get session
    pub fn get_session(&self, session_id: u64) -> Option<&ProtocolSession> {
        self.sessions.get(&session_id)
    }

    /// Get sessions for domain
    pub fn get_sessions_for_domain(&self, domain: DomainId) -> Vec<&ProtocolSession> {
        self.sessions
            .values()
            .filter(|s| s.remote == domain)
            .collect()
    }

    /// Cleanup expired fragments and sessions
    pub fn cleanup(&mut self) {
        let now = Timestamp::now();

        // Remove expired fragments
        self.pending_fragments.retain(|_, buffer| {
            now.elapsed_since(buffer.started) < self.config.fragment_timeout_ns
        });

        // Remove expired sessions
        self.sessions.retain(|_, session| {
            session.state != SessionState::Closed
                && now.elapsed_since(session.last_activity) < self.config.session_timeout_ns
        });

        self.stats.active_sessions = self.sessions.len() as u64;
    }

    /// Get statistics
    pub fn stats(&self) -> &ProtocolStats {
        &self.stats
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_creation() {
        let config = ProtocolConfig::default();
        let mut handler = ProtocolHandler::new(DomainId::new(1), config);

        let frames = handler
            .create_data_frame(DomainId::new(2), b"Hello, World!".to_vec(), true)
            .unwrap();

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].header.frame_type, FrameType::Data);
        assert!(frames[0].header.flags.is_set(FrameFlags::ACK_REQUIRED));
    }

    #[test]
    fn test_fragmentation() {
        let config = ProtocolConfig {
            fragment_threshold: 10,
            ..Default::default()
        };
        let mut handler = ProtocolHandler::new(DomainId::new(1), config);

        let payload = vec![0u8; 25]; // Larger than threshold
        let frames = handler
            .create_data_frame(DomainId::new(2), payload, false)
            .unwrap();

        assert!(frames.len() > 1);
        assert!(
            frames
                .iter()
                .all(|f| f.header.flags.is_set(FrameFlags::FRAGMENTED))
        );
        assert!(
            frames
                .last()
                .unwrap()
                .header
                .flags
                .is_set(FrameFlags::LAST_FRAGMENT)
        );
    }

    #[test]
    fn test_checksum() {
        let payload = b"Test payload".to_vec();
        let trailer = FrameTrailer::new(&payload);

        assert!(trailer.verify(&payload));

        let mut corrupted = payload.clone();
        corrupted[0] = 0xFF;
        assert!(!trailer.verify(&corrupted));
    }

    #[test]
    fn test_session_lifecycle() {
        let config = ProtocolConfig::default();
        let mut handler = ProtocolHandler::new(DomainId::new(1), config);

        let session_id = handler.start_session(DomainId::new(2), vec!["compression".into()]);
        assert_eq!(
            handler.get_session(session_id).unwrap().state,
            SessionState::Connecting
        );

        handler.connect_session(session_id);
        assert_eq!(
            handler.get_session(session_id).unwrap().state,
            SessionState::Connected
        );

        handler.close_session(session_id);
        assert!(handler.get_session(session_id).is_none());
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = ProtocolVersion::new(1, 0, 0);
        let v1_1 = ProtocolVersion::new(1, 1, 0);
        let v2 = ProtocolVersion::new(2, 0, 0);

        assert!(v1.is_compatible(&v1_1));
        assert!(!v1.is_compatible(&v2));
    }
}
