// SPDX-License-Identifier: GPL-2.0
//! Coop claim_mgr — resource claim management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Claim type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimType {
    Exclusive,
    Shared,
    ReadOnly,
    Tentative,
    Preemptible,
}

/// Claim state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    Pending,
    Granted,
    Contested,
    Revoked,
    Expired,
}

/// Resource claim
#[derive(Debug)]
pub struct Claim {
    pub id: u64,
    pub resource_id: u64,
    pub owner: u64,
    pub claim_type: ClaimType,
    pub state: ClaimState,
    pub priority: i32,
    pub created_at: u64,
    pub expires_at: u64,
    pub granted_at: u64,
}

impl Claim {
    pub fn new(id: u64, resource: u64, owner: u64, ctype: ClaimType, prio: i32, now: u64) -> Self {
        Self { id, resource_id: resource, owner, claim_type: ctype, state: ClaimState::Pending, priority: prio, created_at: now, expires_at: 0, granted_at: 0 }
    }

    pub fn grant(&mut self, now: u64) { self.state = ClaimState::Granted; self.granted_at = now; }
    pub fn revoke(&mut self) { self.state = ClaimState::Revoked; }
    pub fn expire(&mut self) { self.state = ClaimState::Expired; }

    pub fn is_active(&self) -> bool { self.state == ClaimState::Granted }
    pub fn check_expiry(&mut self, now: u64) -> bool {
        if self.expires_at > 0 && now >= self.expires_at && self.is_active() { self.expire(); true } else { false }
    }

    pub fn conflicts_with(&self, other: &Claim) -> bool {
        if self.resource_id != other.resource_id { return false; }
        matches!((self.claim_type, other.claim_type),
            (ClaimType::Exclusive, _) | (_, ClaimType::Exclusive))
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ClaimMgrStats {
    pub total_claims: u32,
    pub granted: u32,
    pub pending: u32,
    pub contested: u32,
    pub total_revocations: u64,
    pub avg_grant_latency_ns: u64,
}

/// Main claim manager
pub struct CoopClaimMgr {
    claims: BTreeMap<u64, Claim>,
    next_id: u64,
}

impl CoopClaimMgr {
    pub fn new() -> Self { Self { claims: BTreeMap::new(), next_id: 1 } }

    pub fn claim(&mut self, resource: u64, owner: u64, ctype: ClaimType, prio: i32, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut c = Claim::new(id, resource, owner, ctype, prio, now);
        let conflicts: Vec<u64> = self.claims.values()
            .filter(|e| e.is_active() && c.conflicts_with(e)).map(|e| e.id).collect();
        if conflicts.is_empty() { c.grant(now); }
        self.claims.insert(id, c);
        id
    }

    pub fn release(&mut self, id: u64) {
        if let Some(c) = self.claims.get_mut(&id) { c.revoke(); }
    }

    pub fn stats(&self) -> ClaimMgrStats {
        let granted = self.claims.values().filter(|c| c.state == ClaimState::Granted).count() as u32;
        let pending = self.claims.values().filter(|c| c.state == ClaimState::Pending).count() as u32;
        let contested = self.claims.values().filter(|c| c.state == ClaimState::Contested).count() as u32;
        let revoked = self.claims.values().filter(|c| c.state == ClaimState::Revoked).count() as u64;
        let lats: Vec<u64> = self.claims.values().filter(|c| c.granted_at > 0).map(|c| c.granted_at - c.created_at).collect();
        let avg = if lats.is_empty() { 0 } else { lats.iter().sum::<u64>() / lats.len() as u64 };
        ClaimMgrStats { total_claims: self.claims.len() as u32, granted, pending, contested, total_revocations: revoked, avg_grant_latency_ns: avg }
    }
}
