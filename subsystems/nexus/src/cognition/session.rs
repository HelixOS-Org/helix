//! # Cognitive Session Management
//!
//! Manages cognitive processing sessions.
//! Tracks session state and provides isolation.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// SESSION TYPES
// ============================================================================

/// A cognitive session
#[derive(Debug, Clone)]
pub struct CognitiveSession {
    /// Session ID
    pub id: u64,
    /// Session name
    pub name: String,
    /// Session type
    pub session_type: SessionType,
    /// State
    pub state: SessionState,
    /// Owner domain
    pub owner: DomainId,
    /// Creation time
    pub created: Timestamp,
    /// Last active time
    pub last_active: Timestamp,
    /// Session data
    pub data: BTreeMap<String, SessionValue>,
    /// Configuration
    pub config: SessionConfig,
    /// Statistics
    pub stats: SessionStats,
}

/// Session type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    /// Interactive session
    Interactive,
    /// Batch processing
    Batch,
    /// Streaming
    Streaming,
    /// Background task
    Background,
    /// One-shot
    OneShot,
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initializing
    Initializing,
    /// Active
    Active,
    /// Paused
    Paused,
    /// Waiting
    Waiting,
    /// Completing
    Completing,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Expired
    Expired,
}

/// Session value
#[derive(Debug, Clone)]
pub enum SessionValue {
    /// Null
    Null,
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// Float
    Float(f64),
    /// String
    String(String),
    /// Bytes
    Bytes(Vec<u8>),
    /// Array
    Array(Vec<SessionValue>),
    /// Map
    Map(BTreeMap<String, SessionValue>),
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Timeout (nanoseconds)
    pub timeout_ns: u64,
    /// Idle timeout (nanoseconds)
    pub idle_timeout_ns: u64,
    /// Maximum data size
    pub max_data_size: usize,
    /// Allow persistence
    pub persistent: bool,
    /// Priority
    pub priority: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_ns: 3600_000_000_000,      // 1 hour
            idle_timeout_ns: 300_000_000_000,   // 5 minutes
            max_data_size: 1024 * 1024,         // 1MB
            persistent: false,
            priority: 100,
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SessionStats {
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Pause count
    pub pause_count: u64,
    /// Total active time (ns)
    pub active_time_ns: u64,
    /// Total wait time (ns)
    pub wait_time_ns: u64,
}

// ============================================================================
// SESSION MANAGER
// ============================================================================

/// Manages cognitive sessions
pub struct SessionManager {
    /// Active sessions
    sessions: BTreeMap<u64, CognitiveSession>,
    /// Sessions by owner
    sessions_by_owner: BTreeMap<DomainId, Vec<u64>>,
    /// Next session ID
    next_id: AtomicU64,
    /// Configuration
    config: SessionManagerConfig,
    /// Statistics
    stats: SessionManagerStats,
}

/// Manager configuration
#[derive(Debug, Clone)]
pub struct SessionManagerConfig {
    /// Maximum sessions
    pub max_sessions: usize,
    /// Maximum sessions per domain
    pub max_sessions_per_domain: usize,
    /// Default session timeout
    pub default_timeout_ns: u64,
    /// Enable session persistence
    pub enable_persistence: bool,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10000,
            max_sessions_per_domain: 100,
            default_timeout_ns: 3600_000_000_000,
            enable_persistence: false,
        }
    }
}

/// Manager statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SessionManagerStats {
    /// Total sessions created
    pub total_created: u64,
    /// Total sessions completed
    pub total_completed: u64,
    /// Total sessions failed
    pub total_failed: u64,
    /// Total sessions expired
    pub total_expired: u64,
    /// Active sessions
    pub active_sessions: u64,
    /// Peak sessions
    pub peak_sessions: u64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionManagerConfig) -> Self {
        Self {
            sessions: BTreeMap::new(),
            sessions_by_owner: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SessionManagerStats::default(),
        }
    }

    /// Create a new session
    pub fn create_session(
        &mut self,
        name: &str,
        session_type: SessionType,
        owner: DomainId,
        config: Option<SessionConfig>,
    ) -> Result<u64, &'static str> {
        // Check limits
        if self.sessions.len() >= self.config.max_sessions {
            return Err("Maximum sessions reached");
        }

        let owner_sessions = self.sessions_by_owner.entry(owner).or_default();
        if owner_sessions.len() >= self.config.max_sessions_per_domain {
            return Err("Maximum sessions per domain reached");
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let session = CognitiveSession {
            id,
            name: name.into(),
            session_type,
            state: SessionState::Initializing,
            owner,
            created: now,
            last_active: now,
            data: BTreeMap::new(),
            config: config.unwrap_or_default(),
            stats: SessionStats::default(),
        };

        self.sessions.insert(id, session);
        owner_sessions.push(id);

        self.stats.total_created += 1;
        self.stats.active_sessions = self.sessions.len() as u64;
        if self.stats.active_sessions > self.stats.peak_sessions {
            self.stats.peak_sessions = self.stats.active_sessions;
        }

        Ok(id)
    }

    /// Get a session
    #[inline(always)]
    pub fn get_session(&self, session_id: u64) -> Option<&CognitiveSession> {
        self.sessions.get(&session_id)
    }

    /// Get mutable session
    #[inline(always)]
    pub fn get_session_mut(&mut self, session_id: u64) -> Option<&mut CognitiveSession> {
        self.sessions.get_mut(&session_id)
    }

    /// Activate a session
    #[inline]
    pub fn activate(&mut self, session_id: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            if matches!(session.state, SessionState::Initializing | SessionState::Paused | SessionState::Waiting) {
                session.state = SessionState::Active;
                session.last_active = Timestamp::now();
                return true;
            }
        }
        false
    }

    /// Pause a session
    #[inline]
    pub fn pause(&mut self, session_id: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            if session.state == SessionState::Active {
                session.state = SessionState::Paused;
                session.stats.pause_count += 1;
                return true;
            }
        }
        false
    }

    /// Complete a session
    pub fn complete(&mut self, session_id: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.state = SessionState::Completing;
            session.last_active = Timestamp::now();

            // Calculate total active time
            let active_time = session.last_active.elapsed_since(session.created);
            session.stats.active_time_ns = active_time;

            session.state = SessionState::Completed;
            self.stats.total_completed += 1;

            // Remove from owner tracking
            if let Some(owner_sessions) = self.sessions_by_owner.get_mut(&session.owner) {
                owner_sessions.retain(|&id| id != session_id);
            }

            return true;
        }
        false
    }

    /// Fail a session
    pub fn fail(&mut self, session_id: u64, _reason: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.state = SessionState::Failed;
            session.last_active = Timestamp::now();
            self.stats.total_failed += 1;

            // Remove from owner tracking
            if let Some(owner_sessions) = self.sessions_by_owner.get_mut(&session.owner) {
                owner_sessions.retain(|&id| id != session_id);
            }

            return true;
        }
        false
    }

    /// Set session data
    pub fn set_data(&mut self, session_id: u64, key: &str, value: SessionValue) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            // Check size limit
            let current_size: usize = session.data.len();
            if current_size >= session.config.max_data_size {
                return false;
            }

            session.data.insert(key.into(), value);
            session.last_active = Timestamp::now();
            return true;
        }
        false
    }

    /// Get session data
    #[inline(always)]
    pub fn get_data(&self, session_id: u64, key: &str) -> Option<&SessionValue> {
        self.sessions.get(&session_id)
            .and_then(|s| s.data.get(key))
    }

    /// Remove session data
    #[inline(always)]
    pub fn remove_data(&mut self, session_id: u64, key: &str) -> Option<SessionValue> {
        self.sessions.get_mut(&session_id)
            .and_then(|s| s.data.remove(key))
    }

    /// Record operation
    #[inline]
    pub fn record_operation(&mut self, session_id: u64, success: bool) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.stats.total_operations += 1;
            if success {
                session.stats.successful_operations += 1;
            } else {
                session.stats.failed_operations += 1;
            }
            session.last_active = Timestamp::now();
        }
    }

    /// Get sessions by owner
    #[inline]
    pub fn get_sessions_by_owner(&self, owner: DomainId) -> Vec<&CognitiveSession> {
        self.sessions_by_owner.get(&owner)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sessions.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get sessions by state
    #[inline]
    pub fn get_sessions_by_state(&self, state: SessionState) -> Vec<&CognitiveSession> {
        self.sessions.values()
            .filter(|s| s.state == state)
            .collect()
    }

    /// Check for expired sessions
    pub fn check_expired(&mut self) -> Vec<u64> {
        let now = Timestamp::now();
        let mut expired = Vec::new();

        for (id, session) in &self.sessions {
            // Check timeout
            let elapsed = now.elapsed_since(session.created);
            if elapsed > session.config.timeout_ns {
                expired.push(*id);
                continue;
            }

            // Check idle timeout
            let idle = now.elapsed_since(session.last_active);
            if idle > session.config.idle_timeout_ns {
                expired.push(*id);
            }
        }

        // Expire sessions
        for id in &expired {
            if let Some(session) = self.sessions.get_mut(id) {
                session.state = SessionState::Expired;
                self.stats.total_expired += 1;

                // Remove from owner tracking
                if let Some(owner_sessions) = self.sessions_by_owner.get_mut(&session.owner) {
                    owner_sessions.retain(|sid| sid != id);
                }
            }
        }

        self.stats.active_sessions = self.sessions.values()
            .filter(|s| matches!(s.state, SessionState::Active | SessionState::Paused | SessionState::Waiting))
            .count() as u64;

        expired
    }

    /// Cleanup completed/failed/expired sessions
    pub fn cleanup(&mut self) -> usize {
        let before = self.sessions.len();

        self.sessions.retain(|_, session| {
            !matches!(
                session.state,
                SessionState::Completed | SessionState::Failed | SessionState::Expired
            ) || session.config.persistent
        });

        let removed = before - self.sessions.len();
        self.stats.active_sessions = self.sessions.len() as u64;

        removed
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SessionManagerStats {
        &self.stats
    }

    /// Get all sessions
    #[inline(always)]
    pub fn all_sessions(&self) -> Vec<&CognitiveSession> {
        self.sessions.values().collect()
    }

    /// Get session count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(SessionManagerConfig::default())
    }
}

// ============================================================================
// SESSION BUILDER
// ============================================================================

/// Builder for creating sessions
pub struct SessionBuilder {
    name: String,
    session_type: SessionType,
    owner: DomainId,
    config: SessionConfig,
    initial_data: BTreeMap<String, SessionValue>,
}

impl SessionBuilder {
    /// Create a new builder
    pub fn new(name: &str, owner: DomainId) -> Self {
        Self {
            name: name.into(),
            session_type: SessionType::Interactive,
            owner,
            config: SessionConfig::default(),
            initial_data: BTreeMap::new(),
        }
    }

    /// Set session type
    #[inline(always)]
    pub fn session_type(mut self, t: SessionType) -> Self {
        self.session_type = t;
        self
    }

    /// Set timeout
    #[inline(always)]
    pub fn timeout_ns(mut self, ns: u64) -> Self {
        self.config.timeout_ns = ns;
        self
    }

    /// Set idle timeout
    #[inline(always)]
    pub fn idle_timeout_ns(mut self, ns: u64) -> Self {
        self.config.idle_timeout_ns = ns;
        self
    }

    /// Set persistent
    #[inline(always)]
    pub fn persistent(mut self, p: bool) -> Self {
        self.config.persistent = p;
        self
    }

    /// Set priority
    #[inline(always)]
    pub fn priority(mut self, p: u32) -> Self {
        self.config.priority = p;
        self
    }

    /// Add initial data
    #[inline(always)]
    pub fn with_data(mut self, key: &str, value: SessionValue) -> Self {
        self.initial_data.insert(key.into(), value);
        self
    }

    /// Build and create the session
    pub fn build(self, manager: &mut SessionManager) -> Result<u64, &'static str> {
        let id = manager.create_session(
            &self.name,
            self.session_type,
            self.owner,
            Some(self.config),
        )?;

        // Set initial data
        for (key, value) in self.initial_data {
            manager.set_data(id, &key, value);
        }

        Ok(id)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let mut manager = SessionManager::default();
        let domain = DomainId::new(1);

        let session_id = manager.create_session(
            "test_session",
            SessionType::Interactive,
            domain,
            None,
        ).unwrap();

        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.name, "test_session");
        assert_eq!(session.state, SessionState::Initializing);
    }

    #[test]
    fn test_session_lifecycle() {
        let mut manager = SessionManager::default();
        let domain = DomainId::new(1);

        let session_id = manager.create_session(
            "lifecycle_test",
            SessionType::Batch,
            domain,
            None,
        ).unwrap();

        // Activate
        assert!(manager.activate(session_id));
        assert_eq!(manager.get_session(session_id).unwrap().state, SessionState::Active);

        // Pause
        assert!(manager.pause(session_id));
        assert_eq!(manager.get_session(session_id).unwrap().state, SessionState::Paused);

        // Activate again
        assert!(manager.activate(session_id));

        // Complete
        assert!(manager.complete(session_id));
        assert_eq!(manager.get_session(session_id).unwrap().state, SessionState::Completed);
    }

    #[test]
    fn test_session_data() {
        let mut manager = SessionManager::default();
        let domain = DomainId::new(1);

        let session_id = manager.create_session(
            "data_test",
            SessionType::Interactive,
            domain,
            None,
        ).unwrap();

        // Set data
        manager.set_data(session_id, "key1", SessionValue::Int(42));
        manager.set_data(session_id, "key2", SessionValue::String("value".into()));

        // Get data
        let value = manager.get_data(session_id, "key1");
        assert!(matches!(value, Some(SessionValue::Int(42))));

        // Remove data
        let removed = manager.remove_data(session_id, "key1");
        assert!(matches!(removed, Some(SessionValue::Int(42))));
        assert!(manager.get_data(session_id, "key1").is_none());
    }

    #[test]
    fn test_session_builder() {
        let mut manager = SessionManager::default();
        let domain = DomainId::new(1);

        let session_id = SessionBuilder::new("built_session", domain)
            .session_type(SessionType::Streaming)
            .priority(50)
            .with_data("init_key", SessionValue::Bool(true))
            .build(&mut manager)
            .unwrap();

        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.session_type, SessionType::Streaming);
        assert_eq!(session.config.priority, 50);
        assert!(matches!(session.data.get("init_key"), Some(SessionValue::Bool(true))));
    }

    #[test]
    fn test_sessions_by_owner() {
        let mut manager = SessionManager::default();
        let domain1 = DomainId::new(1);
        let domain2 = DomainId::new(2);

        manager.create_session("s1", SessionType::Interactive, domain1, None).unwrap();
        manager.create_session("s2", SessionType::Interactive, domain1, None).unwrap();
        manager.create_session("s3", SessionType::Interactive, domain2, None).unwrap();

        let sessions1 = manager.get_sessions_by_owner(domain1);
        assert_eq!(sessions1.len(), 2);

        let sessions2 = manager.get_sessions_by_owner(domain2);
        assert_eq!(sessions2.len(), 1);
    }
}
