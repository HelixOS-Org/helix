//! # Application Credential Management
//!
//! Track and analyze application security credentials:
//! - UID/GID tracking
//! - Privilege escalation detection
//! - Credential inheritance
//! - Session management
//! - Access control analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// CREDENTIAL TYPES
// ============================================================================

/// User ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserId(pub u32);

/// Group ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupId(pub u32);

impl UserId {
    /// Is root
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.0 == 0
    }

    /// Is system user (< 1000)
    #[inline(always)]
    pub fn is_system(&self) -> bool {
        self.0 < 1000
    }
}

/// Credential set
#[derive(Debug, Clone)]
pub struct CredentialSet {
    /// Real user id
    pub ruid: UserId,
    /// Effective user id
    pub euid: UserId,
    /// Saved user id
    pub suid: UserId,
    /// Real group id
    pub rgid: GroupId,
    /// Effective group id
    pub egid: GroupId,
    /// Saved group id
    pub sgid: GroupId,
    /// Supplementary groups
    pub groups: Vec<GroupId>,
}

impl CredentialSet {
    #[inline]
    pub fn root() -> Self {
        Self {
            ruid: UserId(0),
            euid: UserId(0),
            suid: UserId(0),
            rgid: GroupId(0),
            egid: GroupId(0),
            sgid: GroupId(0),
            groups: Vec::new(),
        }
    }

    #[inline]
    pub fn user(uid: u32, gid: u32) -> Self {
        Self {
            ruid: UserId(uid),
            euid: UserId(uid),
            suid: UserId(uid),
            rgid: GroupId(gid),
            egid: GroupId(gid),
            sgid: GroupId(gid),
            groups: Vec::new(),
        }
    }

    /// Is running as root?
    #[inline(always)]
    pub fn is_privileged(&self) -> bool {
        self.euid.is_root()
    }

    /// Is setuid?
    #[inline(always)]
    pub fn is_setuid(&self) -> bool {
        self.ruid.0 != self.euid.0
    }

    /// Is setgid?
    #[inline(always)]
    pub fn is_setgid(&self) -> bool {
        self.rgid.0 != self.egid.0
    }

    /// In group?
    #[inline(always)]
    pub fn in_group(&self, gid: GroupId) -> bool {
        self.egid == gid || self.groups.contains(&gid)
    }

    /// Set effective uid
    #[inline(always)]
    pub fn set_euid(&mut self, uid: UserId) {
        self.euid = uid;
    }

    /// Set effective gid
    #[inline(always)]
    pub fn set_egid(&mut self, gid: GroupId) {
        self.egid = gid;
    }
}

// ============================================================================
// CREDENTIAL EVENTS
// ============================================================================

/// Credential change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialChange {
    /// setuid
    SetUid,
    /// setgid
    SetGid,
    /// seteuid
    SetEuid,
    /// setegid
    SetEgid,
    /// setreuid
    SetReuid,
    /// setregid
    SetRegid,
    /// setresuid
    SetResuid,
    /// setresgid
    SetResgid,
    /// setgroups
    SetGroups,
}

/// Credential change event
#[derive(Debug, Clone)]
pub struct CredentialEvent {
    /// Process id
    pub pid: u64,
    /// Change type
    pub change: CredentialChange,
    /// Before
    pub before_uid: UserId,
    pub before_gid: GroupId,
    /// After
    pub after_uid: UserId,
    pub after_gid: GroupId,
    /// Timestamp
    pub timestamp: u64,
    /// Privilege escalation?
    pub is_escalation: bool,
}

impl CredentialEvent {
    /// Detect if this is a privilege escalation
    #[inline]
    pub fn detect_escalation(before: &CredentialSet, after: &CredentialSet) -> bool {
        // Escalation: gaining root when not root before
        if !before.is_privileged() && after.is_privileged() {
            return true;
        }
        // Escalation: gaining any lower uid
        if after.euid.0 < before.euid.0 {
            return true;
        }
        false
    }
}

// ============================================================================
// SESSION TRACKING
// ============================================================================

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    /// Login session
    Login,
    /// SSH session
    Ssh,
    /// Local console
    Console,
    /// Service
    Service,
    /// Cron job
    Cron,
}

/// Security session
#[derive(Debug, Clone)]
pub struct SecuritySession {
    /// Session id
    pub session_id: u64,
    /// Session type
    pub session_type: SessionType,
    /// Owner uid
    pub owner: UserId,
    /// Processes in session
    pub processes: Vec<u64>,
    /// Created at
    pub created_at: u64,
    /// Last activity
    pub last_activity: u64,
    /// Escalation count
    pub escalation_count: u32,
    /// Active
    pub active: bool,
}

impl SecuritySession {
    pub fn new(session_id: u64, session_type: SessionType, owner: UserId, now: u64) -> Self {
        Self {
            session_id,
            session_type,
            owner,
            processes: Vec::new(),
            created_at: now,
            last_activity: now,
            escalation_count: 0,
            active: true,
        }
    }

    /// Add process
    #[inline]
    pub fn add_process(&mut self, pid: u64, now: u64) {
        if !self.processes.contains(&pid) {
            self.processes.push(pid);
        }
        self.last_activity = now;
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.retain(|&p| p != pid);
    }

    /// Duration
    #[inline(always)]
    pub fn duration_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }

    /// Is suspicious? (many escalations)
    #[inline(always)]
    pub fn is_suspicious(&self) -> bool {
        self.escalation_count > 5
    }

    /// Close session
    #[inline(always)]
    pub fn close(&mut self) {
        self.active = false;
    }
}

// ============================================================================
// PROCESS CREDENTIAL PROFILE
// ============================================================================

/// Process credential profile
#[derive(Debug, Clone)]
pub struct ProcessCredProfile {
    /// Process id
    pub pid: u64,
    /// Current credentials
    pub credentials: CredentialSet,
    /// Parent pid
    pub parent_pid: u64,
    /// Session id
    pub session_id: u64,
    /// Credential changes
    pub change_count: u32,
    /// Escalation count
    pub escalation_count: u32,
    /// Created at
    pub created_at: u64,
}

impl ProcessCredProfile {
    pub fn new(pid: u64, creds: CredentialSet, parent: u64, session: u64, now: u64) -> Self {
        Self {
            pid,
            credentials: creds,
            parent_pid: parent,
            session_id: session,
            change_count: 0,
            escalation_count: 0,
            created_at: now,
        }
    }

    /// Apply credential change
    #[inline]
    pub fn apply_change(&mut self, new_creds: CredentialSet) -> bool {
        let escalation = CredentialEvent::detect_escalation(&self.credentials, &new_creds);
        self.credentials = new_creds;
        self.change_count += 1;
        if escalation {
            self.escalation_count += 1;
        }
        escalation
    }
}

// ============================================================================
// CREDENTIAL MANAGER
// ============================================================================

/// Credential stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppCredentialStats {
    /// Tracked processes
    pub processes: usize,
    /// Active sessions
    pub active_sessions: usize,
    /// Total escalations
    pub total_escalations: u64,
    /// Privileged processes
    pub privileged_count: usize,
}

/// Application credential manager
pub struct AppCredentialManager {
    /// Process profiles
    profiles: BTreeMap<u64, ProcessCredProfile>,
    /// Sessions
    sessions: BTreeMap<u64, SecuritySession>,
    /// Change history
    events: VecDeque<CredentialEvent>,
    /// Stats
    stats: AppCredentialStats,
    /// Max events
    max_events: usize,
}

impl AppCredentialManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            sessions: BTreeMap::new(),
            events: VecDeque::new(),
            stats: AppCredentialStats::default(),
            max_events: 4096,
        }
    }

    /// Register process
    #[inline]
    pub fn register_process(
        &mut self,
        pid: u64,
        creds: CredentialSet,
        parent: u64,
        session: u64,
        now: u64,
    ) {
        let profile = ProcessCredProfile::new(pid, creds, parent, session, now);
        self.profiles.insert(pid, profile);
        if let Some(sess) = self.sessions.get_mut(&session) {
            sess.add_process(pid, now);
        }
        self.update_stats();
    }

    /// Create session
    #[inline]
    pub fn create_session(
        &mut self,
        session_id: u64,
        session_type: SessionType,
        owner: UserId,
        now: u64,
    ) {
        let session = SecuritySession::new(session_id, session_type, owner, now);
        self.sessions.insert(session_id, session);
        self.update_stats();
    }

    /// Change credentials
    pub fn change_credentials(
        &mut self,
        pid: u64,
        change: CredentialChange,
        new_creds: CredentialSet,
        now: u64,
    ) -> bool {
        let escalation = if let Some(profile) = self.profiles.get_mut(&pid) {
            let before_uid = profile.credentials.euid;
            let before_gid = profile.credentials.egid;
            let escalation = profile.apply_change(new_creds.clone());

            let event = CredentialEvent {
                pid,
                change,
                before_uid,
                before_gid,
                after_uid: new_creds.euid,
                after_gid: new_creds.egid,
                timestamp: now,
                is_escalation: escalation,
            };
            self.events.push_back(event);
            if self.events.len() > self.max_events {
                self.events.pop_front();
            }

            if escalation {
                self.stats.total_escalations += 1;
                if let Some(sess) = self.sessions.get_mut(&profile.session_id) {
                    sess.escalation_count += 1;
                }
            }
            escalation
        } else {
            false
        };

        self.update_stats();
        escalation
    }

    /// Check credential
    #[inline(always)]
    pub fn credentials(&self, pid: u64) -> Option<&CredentialSet> {
        self.profiles.get(&pid).map(|p| &p.credentials)
    }

    /// Privileged processes
    #[inline]
    pub fn privileged_processes(&self) -> Vec<u64> {
        self.profiles
            .values()
            .filter(|p| p.credentials.is_privileged())
            .map(|p| p.pid)
            .collect()
    }

    /// Suspicious sessions
    #[inline]
    pub fn suspicious_sessions(&self) -> Vec<u64> {
        self.sessions
            .values()
            .filter(|s| s.active && s.is_suspicious())
            .map(|s| s.session_id)
            .collect()
    }

    /// Recent escalations
    #[inline]
    pub fn recent_escalations(&self, limit: usize) -> Vec<&CredentialEvent> {
        self.events
            .iter()
            .rev()
            .filter(|e| e.is_escalation)
            .take(limit)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.processes = self.profiles.len();
        self.stats.active_sessions = self.sessions.values().filter(|s| s.active).count();
        self.stats.privileged_count = self
            .profiles
            .values()
            .filter(|p| p.credentials.is_privileged())
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppCredentialStats {
        &self.stats
    }
}
