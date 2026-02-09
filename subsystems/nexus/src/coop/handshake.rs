//! # Cooperative Handshake Protocol
//!
//! Initial handshake between processes and the kernel:
//! - Capability negotiation
//! - Feature discovery
//! - Version compatibility
//! - Security credential exchange
//! - Performance hint registration
//! - Connection lifecycle management

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

// ============================================================================
// HANDSHAKE TYPES
// ============================================================================

/// Handshake state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    /// Not started
    Pending,
    /// Hello sent
    HelloSent,
    /// Hello received, negotiating
    Negotiating,
    /// Capabilities exchanged
    CapabilitiesExchanged,
    /// Completed
    Complete,
    /// Failed
    Failed,
    /// Timed out
    TimedOut,
}

/// Protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProtocolVersion {
    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Current version
    pub const CURRENT: ProtocolVersion = ProtocolVersion::new(4, 0, 1);

    /// Compatible with another version
    #[inline(always)]
    pub fn compatible(&self, other: &ProtocolVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

// ============================================================================
// CAPABILITIES
// ============================================================================

/// Capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Basic cooperative scheduling hints
    CoopHints,
    /// Advanced resource contracts
    ResourceContracts,
    /// Deadline support
    Deadlines,
    /// Energy-aware scheduling
    EnergyAware,
    /// NUMA-aware placement
    NumaAware,
    /// Memory pressure notifications
    MemoryPressure,
    /// I/O priority hints
    IoPriority,
    /// Network QoS
    NetworkQos,
    /// Thermal notifications
    ThermalNotify,
    /// Container awareness
    ContainerAware,
    /// GPU scheduling hints
    GpuHints,
    /// Async I/O integration
    AsyncIo,
}

/// Capability set
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    /// Enabled capabilities
    pub capabilities: Vec<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    #[inline]
    pub fn add(&mut self, cap: Capability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    #[inline(always)]
    pub fn has(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Intersect with another set
    #[inline]
    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut result = CapabilitySet::new();
        for cap in &self.capabilities {
            if other.has(*cap) {
                result.add(*cap);
            }
        }
        result
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        self.capabilities.len()
    }
}

// ============================================================================
// HELLO MESSAGE
// ============================================================================

/// Hello message (first handshake step)
#[derive(Debug, Clone)]
pub struct HelloMessage {
    /// Process ID
    pub pid: u64,
    /// Protocol version
    pub version: ProtocolVersion,
    /// Requested capabilities
    pub requested_caps: CapabilitySet,
    /// Process name
    pub process_name: String,
    /// Nonce for security
    pub nonce: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Hello response
#[derive(Debug, Clone)]
pub struct HelloResponse {
    /// Accepted
    pub accepted: bool,
    /// Kernel protocol version
    pub kernel_version: ProtocolVersion,
    /// Granted capabilities
    pub granted_caps: CapabilitySet,
    /// Session ID
    pub session_id: u64,
    /// Response nonce
    pub response_nonce: u64,
    /// Rejection reason
    pub rejection_reason: Option<String>,
}

// ============================================================================
// PERFORMANCE HINTS
// ============================================================================

/// Performance hint during handshake
#[derive(Debug, Clone)]
pub struct PerformanceHint {
    /// Hint type
    pub hint_type: PerfHintType,
    /// Value
    pub value: u64,
}

/// Performance hint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfHintType {
    /// Expected CPU usage (percent × 100)
    ExpectedCpuUsage,
    /// Expected memory (bytes)
    ExpectedMemory,
    /// Expected I/O rate (ops/sec)
    ExpectedIoRate,
    /// Preferred CPU
    PreferredCpu,
    /// Preferred NUMA node
    PreferredNuma,
    /// Latency sensitivity (0-100)
    LatencySensitivity,
    /// Throughput priority (0-100)
    ThroughputPriority,
    /// Expected lifetime (ms)
    ExpectedLifetime,
    /// Thread count hint
    ThreadCountHint,
}

// ============================================================================
// HANDSHAKE SESSION
// ============================================================================

/// Handshake session
#[derive(Debug, Clone)]
pub struct HandshakeSession {
    /// Session ID
    pub session_id: u64,
    /// Process ID
    pub pid: u64,
    /// State
    pub state: HandshakeState,
    /// Protocol version negotiated
    pub negotiated_version: Option<ProtocolVersion>,
    /// Granted capabilities
    pub granted_caps: CapabilitySet,
    /// Performance hints
    pub perf_hints: Vec<PerformanceHint>,
    /// Created timestamp
    pub created_at: u64,
    /// Completed timestamp
    pub completed_at: u64,
    /// Timeout (ms)
    pub timeout_ms: u64,
    /// Retry count
    pub retries: u32,
}

impl HandshakeSession {
    pub fn new(session_id: u64, pid: u64, now: u64) -> Self {
        Self {
            session_id,
            pid,
            state: HandshakeState::Pending,
            negotiated_version: None,
            granted_caps: CapabilitySet::new(),
            perf_hints: Vec::new(),
            created_at: now,
            completed_at: 0,
            timeout_ms: 5000,
            retries: 0,
        }
    }

    /// Duration so far (ms)
    #[inline]
    pub fn duration_ms(&self, now: u64) -> u64 {
        if self.completed_at > 0 {
            self.completed_at.saturating_sub(self.created_at)
        } else {
            now.saturating_sub(self.created_at)
        }
    }

    /// Is timed out
    #[inline]
    pub fn is_timed_out(&self, now: u64) -> bool {
        self.state != HandshakeState::Complete
            && self.state != HandshakeState::Failed
            && now.saturating_sub(self.created_at) > self.timeout_ms
    }

    /// Complete successfully
    #[inline(always)]
    pub fn complete(&mut self, now: u64) {
        self.state = HandshakeState::Complete;
        self.completed_at = now;
    }

    /// Fail
    #[inline(always)]
    pub fn fail(&mut self) {
        self.state = HandshakeState::Failed;
    }
}

// ============================================================================
// HANDSHAKE MANAGER
// ============================================================================

/// Handshake manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HandshakeManagerStats {
    /// Total sessions
    pub total_sessions: u64,
    /// Active sessions
    pub active: usize,
    /// Completed sessions
    pub completed: u64,
    /// Failed sessions
    pub failed: u64,
    /// Timed out
    pub timed_out: u64,
    /// Average handshake time (ms)
    pub avg_handshake_ms: u64,
}

/// Cooperative handshake manager
pub struct CoopHandshakeManager {
    /// Active sessions
    sessions: BTreeMap<u64, HandshakeSession>,
    /// Process to session
    pid_sessions: LinearMap<u64, 64>,
    /// Kernel capabilities
    kernel_caps: CapabilitySet,
    /// Next session ID
    next_session_id: u64,
    /// Stats
    stats: HandshakeManagerStats,
    /// Completed handshake times (for average)
    completion_times: VecDeque<u64>,
    /// Max completion history
    max_history: usize,
}

impl CoopHandshakeManager {
    pub fn new(kernel_caps: CapabilitySet) -> Self {
        Self {
            sessions: BTreeMap::new(),
            pid_sessions: LinearMap::new(),
            kernel_caps,
            next_session_id: 1,
            stats: HandshakeManagerStats::default(),
            completion_times: VecDeque::new(),
            max_history: 256,
        }
    }

    /// Handle hello from process
    pub fn handle_hello(&mut self, hello: HelloMessage, now: u64) -> HelloResponse {
        // Version check
        if !ProtocolVersion::CURRENT.compatible(&hello.version) {
            return HelloResponse {
                accepted: false,
                kernel_version: ProtocolVersion::CURRENT,
                granted_caps: CapabilitySet::new(),
                session_id: 0,
                response_nonce: hello.nonce ^ 0xDEAD,
                rejection_reason: Some(String::from("Incompatible protocol version")),
            };
        }

        // Create session
        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let granted = self.kernel_caps.intersect(&hello.requested_caps);

        let mut session = HandshakeSession::new(session_id, hello.pid, now);
        session.state = HandshakeState::Negotiating;
        session.negotiated_version = Some(hello.version);
        session.granted_caps = granted.clone();

        self.sessions.insert(session_id, session);
        self.pid_sessions.insert(hello.pid, session_id);
        self.stats.total_sessions += 1;
        self.stats.active = self.sessions.len();

        HelloResponse {
            accepted: true,
            kernel_version: ProtocolVersion::CURRENT,
            granted_caps: granted,
            session_id,
            response_nonce: hello.nonce ^ 0xBEEF,
            rejection_reason: None,
        }
    }

    /// Register performance hints
    #[inline]
    pub fn register_hints(&mut self, session_id: u64, hints: Vec<PerformanceHint>) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.perf_hints = hints;
            session.state = HandshakeState::CapabilitiesExchanged;
            true
        } else {
            false
        }
    }

    /// Complete handshake
    pub fn complete_handshake(&mut self, session_id: u64, now: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.complete(now);
            let duration = session.duration_ms(now);

            self.completion_times.push_back(duration);
            if self.completion_times.len() > self.max_history {
                self.completion_times.pop_front();
            }

            self.stats.completed += 1;
            self.stats.active = self
                .sessions
                .values()
                .filter(|s| {
                    !matches!(
                        s.state,
                        HandshakeState::Complete
                            | HandshakeState::Failed
                            | HandshakeState::TimedOut
                    )
                })
                .count();

            if !self.completion_times.is_empty() {
                self.stats.avg_handshake_ms =
                    self.completion_times.iter().sum::<u64>() / self.completion_times.len() as u64;
            }

            true
        } else {
            false
        }
    }

    /// Check timeouts
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();

        for (id, session) in &mut self.sessions {
            if session.is_timed_out(now) {
                session.state = HandshakeState::TimedOut;
                timed_out.push(*id);
            }
        }

        self.stats.timed_out += timed_out.len() as u64;
        timed_out
    }

    /// Get session
    #[inline(always)]
    pub fn session(&self, id: u64) -> Option<&HandshakeSession> {
        self.sessions.get(&id)
    }

    /// Get session for process
    #[inline]
    pub fn session_for_pid(&self, pid: u64) -> Option<&HandshakeSession> {
        self.pid_sessions
            .get(&pid)
            .and_then(|id| self.sessions.get(id))
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &HandshakeManagerStats {
        &self.stats
    }

    /// Cleanup completed sessions
    pub fn cleanup(&mut self) {
        let to_remove: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                matches!(
                    s.state,
                    HandshakeState::Complete | HandshakeState::Failed | HandshakeState::TimedOut
                )
            })
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            if let Some(session) = self.sessions.remove(&id) {
                self.pid_sessions.remove(&session.pid);
            }
        }

        self.stats.active = self.sessions.len();
    }
}
