//! # Bridge Credential Proxy
//!
//! Credential and capability management for syscall authorization:
//! - Credential validation before syscall dispatch
//! - Capability inheritance tracking
//! - Privilege escalation detection
//! - Credential caching for fast path
//! - Ambient capability management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Credential type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialType {
    /// Unix UID/GID
    UnixId,
    /// Linux capability
    Capability,
    /// Security label (SELinux-like)
    SecurityLabel,
    /// Namespace token
    NamespaceToken,
    /// Custom kernel credential
    Custom,
}

/// Privilege level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    /// Root / kernel
    Root,
    /// Elevated (some caps)
    Elevated,
    /// Normal user
    Normal,
    /// Restricted (sandbox)
    Restricted,
    /// Minimal (seccomp-like)
    Minimal,
}

/// Escalation event
#[derive(Debug, Clone)]
pub struct EscalationEvent {
    /// PID
    pub pid: u64,
    /// From level
    pub from: PrivilegeLevel,
    /// To level
    pub to: PrivilegeLevel,
    /// Syscall that triggered it
    pub syscall_nr: u32,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Was it allowed?
    pub allowed: bool,
}

/// Credential set
#[derive(Debug, Clone)]
pub struct CredentialSet {
    /// UID
    pub uid: u32,
    /// GID
    pub gid: u32,
    /// Effective capabilities (bitmask)
    pub effective_caps: u64,
    /// Permitted capabilities (bitmask)
    pub permitted_caps: u64,
    /// Inheritable capabilities (bitmask)
    pub inheritable_caps: u64,
    /// Ambient capabilities (bitmask)
    pub ambient_caps: u64,
    /// Security label hash (FNV-1a)
    pub security_label: u64,
    /// Privilege level
    pub privilege: PrivilegeLevel,
}

impl CredentialSet {
    pub fn new(uid: u32, gid: u32) -> Self {
        let privilege = if uid == 0 {
            PrivilegeLevel::Root
        } else {
            PrivilegeLevel::Normal
        };
        Self {
            uid,
            gid,
            effective_caps: 0,
            permitted_caps: 0,
            inheritable_caps: 0,
            ambient_caps: 0,
            security_label: 0,
            privilege,
        }
    }

    /// Check if capability is effective
    pub fn has_cap(&self, cap_bit: u8) -> bool {
        if cap_bit >= 64 {
            return false;
        }
        self.effective_caps & (1u64 << cap_bit) != 0
    }

    /// Grant capability
    pub fn grant_cap(&mut self, cap_bit: u8) {
        if cap_bit < 64 {
            self.effective_caps |= 1u64 << cap_bit;
            self.permitted_caps |= 1u64 << cap_bit;
            self.update_privilege();
        }
    }

    /// Revoke capability
    pub fn revoke_cap(&mut self, cap_bit: u8) {
        if cap_bit < 64 {
            self.effective_caps &= !(1u64 << cap_bit);
            self.update_privilege();
        }
    }

    /// Count effective capabilities
    pub fn cap_count(&self) -> u32 {
        self.effective_caps.count_ones()
    }

    fn update_privilege(&mut self) {
        if self.uid == 0 || self.effective_caps == u64::MAX {
            self.privilege = PrivilegeLevel::Root;
        } else if self.cap_count() > 5 {
            self.privilege = PrivilegeLevel::Elevated;
        } else if self.effective_caps == 0 && self.uid != 0 {
            self.privilege = PrivilegeLevel::Restricted;
        } else {
            self.privilege = PrivilegeLevel::Normal;
        }
    }

    /// Set security label
    pub fn set_label(&mut self, label: &str) {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in label.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.security_label = hash;
    }
}

/// Credential cache entry
#[derive(Debug, Clone)]
pub struct CredCacheEntry {
    /// Credentials
    pub creds: CredentialSet,
    /// Cache timestamp
    pub cached_at_ns: u64,
    /// TTL (ns)
    pub ttl_ns: u64,
    /// Cache hits
    pub hits: u64,
}

impl CredCacheEntry {
    pub fn new(creds: CredentialSet, now_ns: u64, ttl_ns: u64) -> Self {
        Self {
            creds,
            cached_at_ns: now_ns,
            ttl_ns,
            hits: 0,
        }
    }

    pub fn is_valid(&self, now_ns: u64) -> bool {
        now_ns.saturating_sub(self.cached_at_ns) < self.ttl_ns
    }
}

/// Authorization decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthzDecision {
    Allow,
    Deny,
    Audit,
    DenyAndLog,
}

/// Credential proxy stats
#[derive(Debug, Clone, Default)]
pub struct BridgeCredentialStats {
    pub cached_credentials: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub escalations_blocked: u64,
    pub escalations_allowed: u64,
    pub authz_checks: u64,
}

/// Bridge credential proxy
pub struct BridgeCredentialProxy {
    /// Credential cache (pid -> entry)
    cache: BTreeMap<u64, CredCacheEntry>,
    /// Escalation log
    escalations: Vec<EscalationEvent>,
    /// Max escalation log size
    max_log: usize,
    /// Default TTL (ns)
    default_ttl_ns: u64,
    /// Stats
    stats: BridgeCredentialStats,
}

impl BridgeCredentialProxy {
    pub fn new() -> Self {
        Self {
            cache: BTreeMap::new(),
            escalations: Vec::new(),
            max_log: 256,
            default_ttl_ns: 5_000_000_000, // 5 seconds
            stats: BridgeCredentialStats::default(),
        }
    }

    /// Cache credentials for PID
    pub fn cache_creds(&mut self, pid: u64, creds: CredentialSet, now_ns: u64) {
        let entry = CredCacheEntry::new(creds, now_ns, self.default_ttl_ns);
        self.cache.insert(pid, entry);
        self.update_stats();
    }

    /// Get cached credentials
    pub fn get_creds(&mut self, pid: u64, now_ns: u64) -> Option<&CredentialSet> {
        if let Some(entry) = self.cache.get_mut(&pid) {
            if entry.is_valid(now_ns) {
                entry.hits += 1;
                self.stats.cache_hits += 1;
                return Some(&entry.creds);
            }
        }
        self.stats.cache_misses += 1;
        None
    }

    /// Check syscall authorization
    pub fn check_authz(&mut self, pid: u64, syscall_nr: u32, now_ns: u64) -> AuthzDecision {
        self.stats.authz_checks += 1;
        if let Some(entry) = self.cache.get(&pid) {
            if entry.is_valid(now_ns) {
                // Privileged syscalls require capabilities
                if syscall_nr < 10 && !entry.creds.has_cap(0) && entry.creds.uid != 0 {
                    return AuthzDecision::Deny;
                }
                return AuthzDecision::Allow;
            }
        }
        AuthzDecision::Allow // Default allow if no cached creds
    }

    /// Record escalation attempt
    pub fn record_escalation(&mut self, pid: u64, from: PrivilegeLevel, to: PrivilegeLevel,
                              syscall_nr: u32, now_ns: u64, allowed: bool) {
        if allowed {
            self.stats.escalations_allowed += 1;
        } else {
            self.stats.escalations_blocked += 1;
        }
        if self.escalations.len() >= self.max_log {
            self.escalations.remove(0);
        }
        self.escalations.push(EscalationEvent {
            pid, from, to, syscall_nr, timestamp_ns: now_ns, allowed,
        });
    }

    /// Evict expired cache entries
    pub fn evict_expired(&mut self, now_ns: u64) {
        self.cache.retain(|_, entry| entry.is_valid(now_ns));
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.cached_credentials = self.cache.len();
    }

    pub fn stats(&self) -> &BridgeCredentialStats {
        &self.stats
    }
}
