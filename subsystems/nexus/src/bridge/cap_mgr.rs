//! # Bridge Capability Manager
//!
//! Capability-based access control for syscall bridging:
//! - Fine-grained capability tokens
//! - Capability inheritance and delegation
//! - Hierarchical capability namespaces
//! - Capability revocation with reference tracking
//! - Audit trail for capability usage
//! - Ambient capability management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Capability type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BridgeCapType {
    Read,
    Write,
    Execute,
    Create,
    Delete,
    Admin,
    Grant,
    Revoke,
    Signal,
    Network,
    Mount,
    Ptrace,
}

/// Capability scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapScope {
    Process,
    Thread,
    Namespace,
    Global,
}

/// Capability state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapState {
    Active,
    Suspended,
    Revoked,
    Expired,
}

/// Capability token
#[derive(Debug, Clone)]
pub struct CapToken {
    pub token_id: u64,
    pub cap_type: BridgeCapType,
    pub scope: CapScope,
    pub state: CapState,
    pub owner_id: u64,
    pub resource_id: u64,
    pub granted_by: u64,
    pub granted_ns: u64,
    pub expires_ns: Option<u64>,
    pub delegatable: bool,
    pub use_count: u64,
    pub max_uses: Option<u64>,
}

impl CapToken {
    pub fn new(id: u64, cap_type: BridgeCapType, owner: u64, resource: u64, now: u64) -> Self {
        Self {
            token_id: id,
            cap_type,
            scope: CapScope::Process,
            state: CapState::Active,
            owner_id: owner,
            resource_id: resource,
            granted_by: 0,
            granted_ns: now,
            expires_ns: None,
            delegatable: false,
            use_count: 0,
            max_uses: None,
        }
    }

    #[inline]
    pub fn is_valid(&self, now: u64) -> bool {
        if self.state != CapState::Active { return false; }
        if let Some(exp) = self.expires_ns {
            if now >= exp { return false; }
        }
        if let Some(max) = self.max_uses {
            if self.use_count >= max { return false; }
        }
        true
    }

    #[inline(always)]
    pub fn consume(&mut self) {
        self.use_count += 1;
    }

    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = CapState::Revoked;
    }

    #[inline(always)]
    pub fn suspend(&mut self) {
        self.state = CapState::Suspended;
    }
}

/// Capability set for a process
#[derive(Debug, Clone)]
pub struct CapSet {
    pub process_id: u64,
    pub effective: Vec<u64>,     // active token IDs
    pub permitted: Vec<u64>,     // allowed token IDs
    pub inheritable: Vec<u64>,   // inherited on exec
    pub ambient: Vec<BridgeCapType>,
}

impl CapSet {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            effective: Vec::new(),
            permitted: Vec::new(),
            inheritable: Vec::new(),
            ambient: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn has_effective(&self, token_id: u64) -> bool {
        self.effective.contains(&token_id)
    }

    #[inline(always)]
    pub fn has_ambient(&self, cap: BridgeCapType) -> bool {
        self.ambient.contains(&cap)
    }
}

/// Capability audit entry
#[derive(Debug, Clone)]
pub struct CapAuditEntry {
    pub timestamp_ns: u64,
    pub token_id: u64,
    pub process_id: u64,
    pub action: CapAuditAction,
    pub resource_id: u64,
    pub result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapAuditAction {
    Check,
    Grant,
    Revoke,
    Delegate,
    Use,
    Deny,
}

/// Holistic Cap Manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeCapMgrStats {
    pub total_tokens: usize,
    pub active_tokens: usize,
    pub revoked_tokens: usize,
    pub total_checks: u64,
    pub total_denials: u64,
}

/// Bridge Capability Manager
#[repr(align(64))]
pub struct BridgeCapMgr {
    tokens: BTreeMap<u64, CapToken>,
    cap_sets: BTreeMap<u64, CapSet>,
    audit_log: VecDeque<CapAuditEntry>,
    next_token_id: u64,
    max_audit: usize,
    stats: BridgeCapMgrStats,
}

impl BridgeCapMgr {
    pub fn new(max_audit: usize) -> Self {
        Self {
            tokens: BTreeMap::new(),
            cap_sets: BTreeMap::new(),
            audit_log: VecDeque::new(),
            next_token_id: 1,
            max_audit,
            stats: BridgeCapMgrStats::default(),
        }
    }

    /// Grant a capability
    pub fn grant(&mut self, cap_type: BridgeCapType, owner: u64, resource: u64, grantor: u64, now: u64) -> u64 {
        let id = self.next_token_id;
        self.next_token_id += 1;
        let mut token = CapToken::new(id, cap_type, owner, resource, now);
        token.granted_by = grantor;
        self.tokens.insert(id, token);

        let set = self.cap_sets.entry(owner).or_insert_with(|| CapSet::new(owner));
        set.effective.push(id);
        set.permitted.push(id);

        self.audit(now, id, owner, CapAuditAction::Grant, resource, true);
        id
    }

    /// Check capability
    pub fn check(&mut self, process_id: u64, cap_type: BridgeCapType, resource: u64, now: u64) -> bool {
        self.stats.total_checks += 1;

        // Check ambient first
        if let Some(set) = self.cap_sets.get(&process_id) {
            if set.has_ambient(cap_type) {
                return true;
            }
            for &tid in &set.effective {
                if let Some(token) = self.tokens.get(&tid) {
                    if token.cap_type == cap_type && token.resource_id == resource && token.is_valid(now) {
                        self.audit(now, tid, process_id, CapAuditAction::Check, resource, true);
                        return true;
                    }
                }
            }
        }

        self.stats.total_denials += 1;
        self.audit(now, 0, process_id, CapAuditAction::Deny, resource, false);
        false
    }

    /// Use (consume) a capability
    #[inline]
    pub fn use_cap(&mut self, token_id: u64, now: u64) -> bool {
        if let Some(token) = self.tokens.get_mut(&token_id) {
            if token.is_valid(now) {
                token.consume();
                self.audit(now, token_id, token.owner_id, CapAuditAction::Use, token.resource_id, true);
                return true;
            }
        }
        false
    }

    /// Revoke a capability
    #[inline]
    pub fn revoke(&mut self, token_id: u64, now: u64) {
        if let Some(token) = self.tokens.get_mut(&token_id) {
            let owner = token.owner_id;
            let resource = token.resource_id;
            token.revoke();
            if let Some(set) = self.cap_sets.get_mut(&owner) {
                set.effective.retain(|&t| t != token_id);
            }
            self.audit(now, token_id, owner, CapAuditAction::Revoke, resource, true);
        }
    }

    /// Delegate a capability to another process
    #[inline]
    pub fn delegate(&mut self, token_id: u64, target_pid: u64, now: u64) -> Option<u64> {
        let (cap_type, resource, delegatable) = if let Some(token) = self.tokens.get(&token_id) {
            (token.cap_type, token.resource_id, token.delegatable)
        } else { return None; };

        if !delegatable { return None; }
        let new_id = self.grant(cap_type, target_pid, resource, token_id, now);
        Some(new_id)
    }

    fn audit(&mut self, ts: u64, token: u64, pid: u64, action: CapAuditAction, resource: u64, result: bool) {
        self.audit_log.push_back(CapAuditEntry {
            timestamp_ns: ts,
            token_id: token,
            process_id: pid,
            action,
            resource_id: resource,
            result,
        });
        while self.audit_log.len() > self.max_audit {
            self.audit_log.pop_front();
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_tokens = self.tokens.len();
        self.stats.active_tokens = self.tokens.values()
            .filter(|t| t.state == CapState::Active).count();
        self.stats.revoked_tokens = self.tokens.values()
            .filter(|t| t.state == CapState::Revoked).count();
    }

    #[inline(always)]
    pub fn token(&self, id: u64) -> Option<&CapToken> { self.tokens.get(&id) }
    #[inline(always)]
    pub fn cap_set(&self, pid: u64) -> Option<&CapSet> { self.cap_sets.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeCapMgrStats { &self.stats }
}
