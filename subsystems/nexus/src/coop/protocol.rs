//! # Core Cooperation Protocol
//!
//! Defines the wire protocol for kernel-application cooperation,
//! including session management, capability negotiation, and message types.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// PROTOCOL VERSION
// ============================================================================

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

// ============================================================================
// CAPABILITIES
// ============================================================================

/// Capabilities that an application can advertise
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopCapability {
    /// App supports receiving memory pressure advisories
    MemoryPressureAware,
    /// App supports receiving CPU throttle advisories
    CpuThrottleAware,
    /// App supports receiving I/O congestion advisories
    IoCongestionAware,
    /// App can provide compute intensity hints
    ComputeHints,
    /// App can provide memory usage predictions
    MemoryPredictions,
    /// App can provide I/O pattern hints
    IoPatternHints,
    /// App can provide latency sensitivity hints
    LatencyHints,
    /// App can provide network usage predictions
    NetworkPredictions,
    /// App supports cooperative scheduling
    CooperativeScheduling,
    /// App supports graceful degradation
    GracefulDegradation,
}

// ============================================================================
// MESSAGES
// ============================================================================

/// Type of cooperation message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMessageType {
    /// Session initialization
    SessionInit,
    /// Session acknowledgment
    SessionAck,
    /// Application hint (app → kernel)
    AppHint,
    /// Kernel advisory (kernel → app)
    KernelAdvisory,
    /// Resource demand (app → kernel)
    ResourceDemand,
    /// Resource offer (kernel → app)
    ResourceOffer,
    /// Contract acceptance (app → kernel)
    ContractAccept,
    /// Contract rejection (app → kernel)
    ContractReject,
    /// Feedback report
    FeedbackReport,
    /// Heartbeat
    Heartbeat,
    /// Session close
    SessionClose,
}

/// A cooperation message
#[derive(Debug, Clone)]
pub struct CoopMessage {
    /// Message type
    pub msg_type: CoopMessageType,
    /// Session ID
    pub session_id: u64,
    /// Source process ID
    pub source_pid: u64,
    /// Sequence number
    pub sequence: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Payload (encoded)
    pub payload: Vec<u8>,
}

impl CoopMessage {
    pub fn new(msg_type: CoopMessageType, session_id: u64, pid: u64) -> Self {
        Self {
            msg_type,
            session_id,
            source_pid: pid,
            sequence: 0,
            timestamp: 0,
            payload: Vec::new(),
        }
    }

    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }

    pub fn with_payload(mut self, data: Vec<u8>) -> Self {
        self.payload = data;
        self
    }
}

// ============================================================================
// SESSION MANAGEMENT
// ============================================================================

/// State of a cooperation session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopSessionState {
    /// Session is being initialized
    Initializing,
    /// Session is active
    Active,
    /// Session is suspended (e.g., app is sleeping)
    Suspended,
    /// Session is closing
    Closing,
    /// Session is closed
    Closed,
}

/// A cooperation session between kernel and an application
#[derive(Debug, Clone)]
pub struct CoopSession {
    /// Unique session ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Application name
    pub app_name: Option<String>,
    /// Protocol version
    pub version: ProtocolVersion,
    /// Session state
    pub state: CoopSessionState,
    /// Capabilities advertised by the app
    pub capabilities: Vec<CoopCapability>,
    /// Messages sent (kernel → app)
    pub messages_sent: u64,
    /// Messages received (app → kernel)
    pub messages_received: u64,
    /// Hints received from app
    pub hints_received: u64,
    /// Advisories sent to app
    pub advisories_sent: u64,
    /// Session start timestamp
    pub started_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Next expected sequence number from app
    pub next_sequence: u64,
}

impl CoopSession {
    /// Create a new session
    pub fn new(id: u64, pid: u64) -> Self {
        Self {
            id,
            pid,
            app_name: None,
            version: ProtocolVersion::CURRENT,
            state: CoopSessionState::Initializing,
            capabilities: Vec::new(),
            messages_sent: 0,
            messages_received: 0,
            hints_received: 0,
            advisories_sent: 0,
            started_at: 0,
            last_activity: 0,
            next_sequence: 1,
        }
    }

    /// Activate the session
    pub fn activate(&mut self, capabilities: Vec<CoopCapability>, timestamp: u64) {
        self.state = CoopSessionState::Active;
        self.capabilities = capabilities;
        self.started_at = timestamp;
        self.last_activity = timestamp;
    }

    /// Check if the app has a capability
    pub fn has_capability(&self, cap: CoopCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Record that a message was sent to the app
    pub fn record_sent(&mut self, timestamp: u64) {
        self.messages_sent += 1;
        self.last_activity = timestamp;
    }

    /// Record that a message was received from the app
    pub fn record_received(&mut self, timestamp: u64) {
        self.messages_received += 1;
        self.last_activity = timestamp;
        self.next_sequence += 1;
    }

    /// Whether the session is active
    pub fn is_active(&self) -> bool {
        self.state == CoopSessionState::Active
    }

    /// Check if session has timed out (no activity for `timeout_ms`)
    pub fn is_timed_out(&self, current_time: u64, timeout_ms: u64) -> bool {
        current_time.saturating_sub(self.last_activity) > timeout_ms
    }

    /// Close the session
    pub fn close(&mut self) {
        self.state = CoopSessionState::Closed;
    }

    /// Cooperation score (0.0 - 1.0) based on how cooperative the app is
    pub fn cooperation_score(&self) -> f64 {
        if self.messages_received == 0 {
            return 0.0;
        }
        let hint_ratio = self.hints_received as f64 / self.messages_received as f64;
        let responsiveness = if self.advisories_sent > 0 {
            // If we sent advisories, did the app respond?
            (self.messages_received as f64 / self.advisories_sent as f64).min(1.0)
        } else {
            0.5
        };
        (hint_ratio * 0.6 + responsiveness * 0.4).min(1.0)
    }
}
