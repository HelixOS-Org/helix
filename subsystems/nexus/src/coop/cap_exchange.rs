//! # Coop Capability Exchange
//!
//! Capability-based resource sharing between cooperating processes:
//! - Capability token generation and validation
//! - Delegation with attenuation
//! - Time-limited capability grants
//! - Revocation cascades
//! - Cross-process capability transfer

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// CAPABILITY TYPES
// ============================================================================

/// Capability rights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapRight {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
    /// Share with others
    Share,
    /// Create sub-capabilities
    Delegate,
    /// Administrative control
    Admin,
}

/// Capability scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapScope {
    /// Single resource
    Resource,
    /// Resource group
    Group,
    /// Namespace-wide
    Namespace,
    /// System-wide
    System,
}

/// Capability state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapExState {
    /// Active and usable
    Active,
    /// Suspended temporarily
    Suspended,
    /// Revoked permanently
    Revoked,
    /// Expired
    Expired,
}

// ============================================================================
// CAPABILITY TOKEN
// ============================================================================

/// Capability token
#[derive(Debug, Clone)]
pub struct CapToken {
    /// Token ID (FNV-1a)
    pub token_id: u64,
    /// Owner PID
    pub owner_pid: u64,
    /// Resource identifier
    pub resource_id: u64,
    /// Rights mask
    pub rights: Vec<CapRight>,
    /// Scope
    pub scope: CapScope,
    /// State
    pub state: CapExState,
    /// Parent token (if delegated)
    pub parent_token: Option<u64>,
    /// Delegation depth
    pub delegation_depth: u32,
    /// Max delegation depth
    pub max_depth: u32,
    /// Created at (ns)
    pub created_ns: u64,
    /// Expires at (ns, 0 = never)
    pub expires_ns: u64,
    /// Usage count
    pub usage_count: u64,
}

impl CapToken {
    pub fn new(token_id: u64, owner_pid: u64, resource_id: u64, now: u64) -> Self {
        Self {
            token_id,
            owner_pid,
            resource_id,
            rights: Vec::new(),
            scope: CapScope::Resource,
            state: CapExState::Active,
            parent_token: None,
            delegation_depth: 0,
            max_depth: 3,
            created_ns: now,
            expires_ns: 0,
            usage_count: 0,
        }
    }

    /// Add right
    #[inline]
    pub fn add_right(&mut self, right: CapRight) {
        if !self.rights.contains(&right) {
            self.rights.push(right);
        }
    }

    /// Check if has right
    #[inline(always)]
    pub fn has_right(&self, right: CapRight) -> bool {
        self.rights.contains(&right)
    }

    /// Check if expired
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_ns > 0 && now >= self.expires_ns
    }

    /// Check if valid
    #[inline(always)]
    pub fn is_valid(&self, now: u64) -> bool {
        self.state == CapExState::Active && !self.is_expired(now)
    }

    /// Use (record usage)
    #[inline(always)]
    pub fn use_token(&mut self) {
        self.usage_count += 1;
    }

    /// Revoke
    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = CapExState::Revoked;
    }

    /// Suspend
    #[inline]
    pub fn suspend(&mut self) {
        if self.state == CapExState::Active {
            self.state = CapExState::Suspended;
        }
    }

    /// Resume
    #[inline]
    pub fn resume(&mut self) {
        if self.state == CapExState::Suspended {
            self.state = CapExState::Active;
        }
    }

    /// Can delegate
    #[inline(always)]
    pub fn can_delegate(&self) -> bool {
        self.has_right(CapRight::Delegate) && self.delegation_depth < self.max_depth
    }

    /// Create delegated token with attenuated rights
    pub fn delegate(
        &self,
        new_id: u64,
        target_pid: u64,
        rights: &[CapRight],
        now: u64,
    ) -> Option<CapToken> {
        if !self.can_delegate() {
            return None;
        }
        // Can only delegate rights we have
        let mut valid_rights = Vec::new();
        for r in rights {
            if self.has_right(*r) {
                valid_rights.push(*r);
            }
        }

        let mut child = CapToken {
            token_id: new_id,
            owner_pid: target_pid,
            resource_id: self.resource_id,
            rights: valid_rights,
            scope: self.scope,
            state: CapExState::Active,
            parent_token: Some(self.token_id),
            delegation_depth: self.delegation_depth + 1,
            max_depth: self.max_depth,
            created_ns: now,
            expires_ns: self.expires_ns, // inherit expiry
            usage_count: 0,
        };

        // Delegated tokens can't exceed parent expiry
        if self.expires_ns > 0 && (child.expires_ns == 0 || child.expires_ns > self.expires_ns) {
            child.expires_ns = self.expires_ns;
        }

        Some(child)
    }
}

// ============================================================================
// TRANSFER RECORD
// ============================================================================

/// Transfer record
#[derive(Debug, Clone)]
pub struct CapTransfer {
    /// Transfer ID
    pub transfer_id: u64,
    /// Source PID
    pub from_pid: u64,
    /// Target PID
    pub to_pid: u64,
    /// Token ID
    pub token_id: u64,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Transfer succeeded
    pub success: bool,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Capability exchange stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopCapExchangeStats {
    /// Total tokens
    pub total_tokens: usize,
    /// Active tokens
    pub active_tokens: usize,
    /// Total transfers
    pub total_transfers: u64,
    /// Revoked tokens
    pub revoked_count: u64,
    /// Max delegation depth seen
    pub max_depth_seen: u32,
}

/// Coop capability exchange engine
pub struct CoopCapExchange {
    /// All tokens
    tokens: BTreeMap<u64, CapToken>,
    /// Process -> token IDs
    process_tokens: BTreeMap<u64, Vec<u64>>,
    /// Transfer log
    transfers: VecDeque<CapTransfer>,
    /// Stats
    stats: CoopCapExchangeStats,
    /// Next token ID counter
    next_token_id: u64,
}

impl CoopCapExchange {
    pub fn new() -> Self {
        Self {
            tokens: BTreeMap::new(),
            process_tokens: BTreeMap::new(),
            transfers: VecDeque::new(),
            stats: CoopCapExchangeStats::default(),
            next_token_id: 1,
        }
    }

    /// Generate FNV-1a token ID
    fn gen_token_id(&mut self, owner: u64, resource: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in owner.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for b in resource.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for b in self.next_token_id.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.next_token_id += 1;
        hash
    }

    /// Grant capability to process
    #[inline]
    pub fn grant(
        &mut self,
        owner_pid: u64,
        resource_id: u64,
        rights: &[CapRight],
        now: u64,
    ) -> u64 {
        let token_id = self.gen_token_id(owner_pid, resource_id);
        let mut token = CapToken::new(token_id, owner_pid, resource_id, now);
        for r in rights {
            token.add_right(*r);
        }
        self.tokens.insert(token_id, token);
        self.process_tokens
            .entry(owner_pid)
            .or_insert_with(Vec::new)
            .push(token_id);
        self.update_stats();
        token_id
    }

    /// Grant with expiry
    #[inline]
    pub fn grant_timed(
        &mut self,
        owner_pid: u64,
        resource_id: u64,
        rights: &[CapRight],
        expires_ns: u64,
        now: u64,
    ) -> u64 {
        let token_id = self.grant(owner_pid, resource_id, rights, now);
        if let Some(t) = self.tokens.get_mut(&token_id) {
            t.expires_ns = expires_ns;
        }
        token_id
    }

    /// Check capability
    pub fn check(&mut self, token_id: u64, right: CapRight, now: u64) -> bool {
        if let Some(token) = self.tokens.get_mut(&token_id) {
            if token.is_valid(now) && token.has_right(right) {
                token.use_token();
                return true;
            }
            // Auto-expire
            if token.is_expired(now) {
                token.state = CapExState::Expired;
            }
        }
        false
    }

    /// Delegate capability
    pub fn delegate(
        &mut self,
        token_id: u64,
        target_pid: u64,
        rights: &[CapRight],
        now: u64,
    ) -> Option<u64> {
        let new_id = self.gen_token_id(target_pid, token_id);
        let child = if let Some(parent) = self.tokens.get(&token_id) {
            parent.delegate(new_id, target_pid, rights, now)
        } else {
            None
        };

        if let Some(child_token) = child {
            let child_id = child_token.token_id;
            if child_token.delegation_depth > self.stats.max_depth_seen {
                self.stats.max_depth_seen = child_token.delegation_depth;
            }
            self.tokens.insert(child_id, child_token);
            self.process_tokens
                .entry(target_pid)
                .or_insert_with(Vec::new)
                .push(child_id);

            // Record transfer
            if self.transfers.len() >= 4096 {
                self.transfers.pop_front();
            }
            self.transfers.push_back(CapTransfer {
                transfer_id: self.stats.total_transfers,
                from_pid: self.tokens.get(&token_id).map(|t| t.owner_pid).unwrap_or(0),
                to_pid: target_pid,
                token_id: child_id,
                timestamp_ns: now,
                success: true,
            });
            self.stats.total_transfers += 1;
            self.update_stats();
            Some(child_id)
        } else {
            None
        }
    }

    /// Revoke token and all descendants
    pub fn revoke_cascade(&mut self, token_id: u64) {
        let mut to_revoke = alloc::vec![token_id];
        let mut i = 0;
        while i < to_revoke.len() {
            let current = to_revoke[i];
            // Find children
            for (id, token) in &self.tokens {
                if token.parent_token == Some(current) && !to_revoke.contains(id) {
                    to_revoke.push(*id);
                }
            }
            i += 1;
        }

        for id in &to_revoke {
            if let Some(token) = self.tokens.get_mut(id) {
                token.revoke();
                self.stats.revoked_count += 1;
            }
        }
        self.update_stats();
    }

    /// Remove process
    #[inline]
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(token_ids) = self.process_tokens.remove(&pid) {
            for id in token_ids {
                self.revoke_cascade(id);
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_tokens = self.tokens.len();
        self.stats.active_tokens = self
            .tokens
            .values()
            .filter(|t| t.state == CapExState::Active)
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopCapExchangeStats {
        &self.stats
    }
}
