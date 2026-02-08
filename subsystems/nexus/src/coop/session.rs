//! # Cooperation Session Management
//!
//! Manages cooperation sessions between kernel and applications:
//! - Session lifecycle (handshake, active, suspend, terminate)
//! - Per-process session state
//! - Session multiplexing for multi-threaded apps
//! - Session persistence across exec/fork
//! - Session group management

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SESSION STATE
// ============================================================================

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial handshake in progress
    Handshake,
    /// Session active
    Active,
    /// Session suspended (app sleeping or low activity)
    Suspended,
    /// Session renegotiating terms
    Renegotiating,
    /// Session terminating
    Terminating,
    /// Session terminated
    Terminated,
    /// Session error
    Error,
}

/// Session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionId(pub u64);

/// Cooperation session
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID
    pub id: SessionId,
    /// Process ID
    pub pid: u64,
    /// Session state
    pub state: SessionState,
    /// Protocol version agreed upon
    pub protocol_version: u32,
    /// Capabilities negotiated
    pub capabilities: SessionCapabilities,
    /// Creation time
    pub created_at: u64,
    /// Last activity time
    pub last_activity: u64,
    /// Messages sent by app
    pub app_messages: u64,
    /// Messages sent by kernel
    pub kernel_messages: u64,
    /// Hints sent
    pub hints_sent: u64,
    /// Advisories sent
    pub advisories_sent: u64,
    /// Session quality score (0.0 - 1.0)
    pub quality: f64,
    /// Idle timeout (ms)
    pub idle_timeout_ms: u64,
    /// Whether app supports cooperative scheduling
    pub cooperative_scheduling: bool,
    /// Whether app supports memory advisories
    pub memory_advisories: bool,
    /// Group ID (if part of a session group)
    pub group_id: Option<u64>,
}

/// Capabilities negotiated for a session
#[derive(Debug, Clone, Copy)]
pub struct SessionCapabilities {
    /// Hint protocol supported
    pub hints: bool,
    /// Advisory protocol supported
    pub advisories: bool,
    /// Resource negotiation supported
    pub negotiation: bool,
    /// Cooperative scheduling supported
    pub coop_scheduling: bool,
    /// Memory cooperation supported
    pub memory_coop: bool,
    /// I/O cooperation supported
    pub io_coop: bool,
    /// Shared memory IPC
    pub shared_memory: bool,
    /// Event notification
    pub events: bool,
}

impl SessionCapabilities {
    pub fn minimal() -> Self {
        Self {
            hints: true,
            advisories: false,
            negotiation: false,
            coop_scheduling: false,
            memory_coop: false,
            io_coop: false,
            shared_memory: false,
            events: false,
        }
    }

    pub fn full() -> Self {
        Self {
            hints: true,
            advisories: true,
            negotiation: true,
            coop_scheduling: true,
            memory_coop: true,
            io_coop: true,
            shared_memory: true,
            events: true,
        }
    }

    /// Count active capabilities
    pub fn count(&self) -> u32 {
        let mut c = 0u32;
        if self.hints { c += 1; }
        if self.advisories { c += 1; }
        if self.negotiation { c += 1; }
        if self.coop_scheduling { c += 1; }
        if self.memory_coop { c += 1; }
        if self.io_coop { c += 1; }
        if self.shared_memory { c += 1; }
        if self.events { c += 1; }
        c
    }
}

impl Session {
    pub fn new(id: SessionId, pid: u64, timestamp: u64) -> Self {
        Self {
            id,
            pid,
            state: SessionState::Handshake,
            protocol_version: 1,
            capabilities: SessionCapabilities::minimal(),
            created_at: timestamp,
            last_activity: timestamp,
            app_messages: 0,
            kernel_messages: 0,
            hints_sent: 0,
            advisories_sent: 0,
            quality: 0.5,
            idle_timeout_ms: 60_000,
            cooperative_scheduling: false,
            memory_advisories: false,
            group_id: None,
        }
    }

    /// Activate session
    pub fn activate(&mut self, caps: SessionCapabilities) {
        self.state = SessionState::Active;
        self.capabilities = caps;
        self.cooperative_scheduling = caps.coop_scheduling;
        self.memory_advisories = caps.memory_coop;
    }

    /// Record app message
    pub fn record_app_message(&mut self, timestamp: u64) {
        self.app_messages += 1;
        self.last_activity = timestamp;
    }

    /// Record kernel message
    pub fn record_kernel_message(&mut self, timestamp: u64) {
        self.kernel_messages += 1;
        self.last_activity = timestamp;
    }

    /// Check if session is idle
    pub fn is_idle(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.last_activity) > self.idle_timeout_ms
    }

    /// Session age (ms)
    pub fn age(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.created_at)
    }

    /// Is session active?
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Total messages exchanged
    pub fn total_messages(&self) -> u64 {
        self.app_messages + self.kernel_messages
    }
}

// ============================================================================
// SESSION GROUP
// ============================================================================

/// A group of related sessions (e.g., multi-process app)
#[derive(Debug, Clone)]
pub struct SessionGroup {
    /// Group ID
    pub group_id: u64,
    /// Member session IDs
    pub members: Vec<SessionId>,
    /// Group leader PID
    pub leader_pid: u64,
    /// Shared capabilities
    pub shared_caps: SessionCapabilities,
    /// Creation time
    pub created_at: u64,
}

impl SessionGroup {
    pub fn new(group_id: u64, leader_pid: u64, timestamp: u64) -> Self {
        Self {
            group_id,
            members: Vec::new(),
            leader_pid,
            shared_caps: SessionCapabilities::minimal(),
            created_at: timestamp,
        }
    }

    /// Add a member
    pub fn add_member(&mut self, session_id: SessionId) {
        if !self.members.contains(&session_id) {
            self.members.push(session_id);
        }
    }

    /// Remove a member
    pub fn remove_member(&mut self, session_id: SessionId) {
        self.members.retain(|&id| id != session_id);
    }

    /// Member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

// ============================================================================
// SESSION MANAGER
// ============================================================================

/// Manages all cooperation sessions
pub struct SessionManager {
    /// Sessions by ID
    sessions: BTreeMap<u64, Session>,
    /// PID → Session ID mapping
    pid_sessions: BTreeMap<u64, SessionId>,
    /// Session groups
    groups: BTreeMap<u64, SessionGroup>,
    /// Next session ID
    next_id: u64,
    /// Next group ID
    next_group_id: u64,
    /// Max sessions
    max_sessions: usize,
    /// Total sessions created
    pub total_created: u64,
    /// Total sessions terminated
    pub total_terminated: u64,
}

impl SessionManager {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            sessions: BTreeMap::new(),
            pid_sessions: BTreeMap::new(),
            groups: BTreeMap::new(),
            next_id: 1,
            next_group_id: 1,
            max_sessions,
            total_created: 0,
            total_terminated: 0,
        }
    }

    /// Create a new session
    pub fn create_session(&mut self, pid: u64, timestamp: u64) -> Option<SessionId> {
        if self.sessions.len() >= self.max_sessions {
            // Try to cleanup idle sessions
            self.cleanup_idle(timestamp);
            if self.sessions.len() >= self.max_sessions {
                return None;
            }
        }

        let id = SessionId(self.next_id);
        self.next_id += 1;

        let session = Session::new(id, pid, timestamp);
        self.sessions.insert(id.0, session);
        self.pid_sessions.insert(pid, id);
        self.total_created += 1;

        Some(id)
    }

    /// Get session by ID
    pub fn get(&self, id: SessionId) -> Option<&Session> {
        self.sessions.get(&id.0)
    }

    /// Get mutable session by ID
    pub fn get_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        self.sessions.get_mut(&id.0)
    }

    /// Get session for a PID
    pub fn get_by_pid(&self, pid: u64) -> Option<&Session> {
        self.pid_sessions.get(&pid).and_then(|id| self.sessions.get(&id.0))
    }

    /// Get mutable session for a PID
    pub fn get_by_pid_mut(&mut self, pid: u64) -> Option<&mut Session> {
        let id = self.pid_sessions.get(&pid).copied()?;
        self.sessions.get_mut(&id.0)
    }

    /// Terminate session
    pub fn terminate(&mut self, id: SessionId) {
        if let Some(session) = self.sessions.get_mut(&id.0) {
            session.state = SessionState::Terminated;
            self.pid_sessions.remove(&session.pid);
            self.total_terminated += 1;

            // Remove from group
            if let Some(gid) = session.group_id {
                if let Some(group) = self.groups.get_mut(&gid) {
                    group.remove_member(id);
                }
            }
        }
        self.sessions.remove(&id.0);
    }

    /// Terminate session by PID
    pub fn terminate_by_pid(&mut self, pid: u64) {
        if let Some(&id) = self.pid_sessions.get(&pid) {
            self.terminate(id);
        }
    }

    /// Create a session group
    pub fn create_group(&mut self, leader_pid: u64, timestamp: u64) -> u64 {
        let gid = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(gid, SessionGroup::new(gid, leader_pid, timestamp));
        gid
    }

    /// Add session to group
    pub fn add_to_group(&mut self, session_id: SessionId, group_id: u64) {
        if let Some(session) = self.sessions.get_mut(&session_id.0) {
            session.group_id = Some(group_id);
        }
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_member(session_id);
        }
    }

    /// Cleanup idle sessions
    fn cleanup_idle(&mut self, current_time: u64) {
        let idle_ids: Vec<u64> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.is_idle(current_time))
            .map(|(&id, _)| id)
            .collect();

        for id in idle_ids {
            self.terminate(SessionId(id));
        }
    }

    /// Active session count
    pub fn active_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_active()).count()
    }

    /// Total session count
    pub fn total_count(&self) -> usize {
        self.sessions.len()
    }

    /// Group count
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}
