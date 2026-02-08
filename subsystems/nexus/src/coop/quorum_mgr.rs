// SPDX-License-Identifier: GPL-2.0
//! Coop quorum_mgr — quorum-based consensus management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Quorum type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumType {
    Simple,
    SuperMajority,
    Unanimous,
    Weighted,
}

/// Vote result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteResult {
    Accept,
    Reject,
    Abstain,
}

/// Quorum member
#[derive(Debug)]
pub struct QuorumMember {
    pub id: u64,
    pub weight: u32,
    pub vote: Option<VoteResult>,
    pub voted_at: u64,
    pub total_votes: u64,
}

impl QuorumMember {
    pub fn new(id: u64, weight: u32) -> Self { Self { id, weight, vote: None, voted_at: 0, total_votes: 0 } }
    pub fn cast_vote(&mut self, vote: VoteResult, now: u64) { self.vote = Some(vote); self.voted_at = now; self.total_votes += 1; }
    pub fn reset(&mut self) { self.vote = None; }
}

/// Quorum proposal
#[derive(Debug)]
pub struct QuorumProposal {
    pub id: u64,
    pub proposer: u64,
    pub quorum_type: QuorumType,
    pub members: Vec<QuorumMember>,
    pub threshold: f64,
    pub created_at: u64,
    pub decided: bool,
    pub accepted: bool,
}

impl QuorumProposal {
    pub fn new(id: u64, proposer: u64, qtype: QuorumType, threshold: f64, now: u64) -> Self {
        Self { id, proposer, quorum_type: qtype, members: Vec::new(), threshold, created_at: now, decided: false, accepted: false }
    }

    pub fn add_member(&mut self, id: u64, weight: u32) { self.members.push(QuorumMember::new(id, weight)); }

    pub fn vote(&mut self, member_id: u64, result: VoteResult, now: u64) {
        if let Some(m) = self.members.iter_mut().find(|m| m.id == member_id) { m.cast_vote(result, now); }
    }

    pub fn check_quorum(&mut self) -> Option<bool> {
        let total_weight: u32 = self.members.iter().map(|m| m.weight).sum();
        let voted: Vec<&QuorumMember> = self.members.iter().filter(|m| m.vote.is_some()).collect();
        let voted_weight: u32 = voted.iter().map(|m| m.weight).sum();

        if (voted_weight as f64 / total_weight as f64) < self.threshold { return None; }

        let accept_weight: u32 = voted.iter().filter(|m| m.vote == Some(VoteResult::Accept)).map(|m| m.weight).sum();
        let result = match self.quorum_type {
            QuorumType::Simple => accept_weight * 2 > voted_weight,
            QuorumType::SuperMajority => accept_weight * 3 > voted_weight * 2,
            QuorumType::Unanimous => accept_weight == total_weight,
            QuorumType::Weighted => (accept_weight as f64 / total_weight as f64) >= self.threshold,
        };
        self.decided = true;
        self.accepted = result;
        Some(result)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct QuorumMgrStats {
    pub total_proposals: u32,
    pub decided: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub pending: u32,
    pub avg_participation: f64,
}

/// Main quorum manager
pub struct CoopQuorumMgr {
    proposals: BTreeMap<u64, QuorumProposal>,
    next_id: u64,
}

impl CoopQuorumMgr {
    pub fn new() -> Self { Self { proposals: BTreeMap::new(), next_id: 1 } }

    pub fn propose(&mut self, proposer: u64, qtype: QuorumType, threshold: f64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.proposals.insert(id, QuorumProposal::new(id, proposer, qtype, threshold, now));
        id
    }

    pub fn vote(&mut self, proposal: u64, member: u64, result: VoteResult, now: u64) {
        if let Some(p) = self.proposals.get_mut(&proposal) { p.vote(member, result, now); }
    }

    pub fn stats(&self) -> QuorumMgrStats {
        let decided = self.proposals.values().filter(|p| p.decided).count() as u32;
        let accepted = self.proposals.values().filter(|p| p.accepted).count() as u32;
        let rejected = decided - accepted;
        let pending = self.proposals.values().filter(|p| !p.decided).count() as u32;
        let parts: Vec<f64> = self.proposals.values().map(|p| {
            let voted = p.members.iter().filter(|m| m.vote.is_some()).count();
            if p.members.is_empty() { 0.0 } else { voted as f64 / p.members.len() as f64 }
        }).collect();
        let avg = if parts.is_empty() { 0.0 } else { parts.iter().sum::<f64>() / parts.len() as f64 };
        QuorumMgrStats { total_proposals: self.proposals.len() as u32, decided, accepted, rejected, pending, avg_participation: avg }
    }
}

// ============================================================================
// Merged from quorum_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumTypeV2 {
    Simple,
    Majority,
    SuperMajority,
    Unanimous,
    Weighted,
}

/// Member status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatusV2 {
    Active,
    Suspected,
    Failed,
    Leaving,
}

/// Quorum member
#[derive(Debug)]
pub struct QuorumMemberV2 {
    pub id: u64,
    pub status: MemberStatusV2,
    pub weight: u32,
    pub last_heartbeat: u64,
    pub votes_cast: u64,
    pub responses: u64,
}

impl QuorumMemberV2 {
    pub fn new(id: u64, weight: u32) -> Self {
        Self { id, status: MemberStatusV2::Active, weight, last_heartbeat: 0, votes_cast: 0, responses: 0 }
    }

    pub fn heartbeat(&mut self, now: u64) { self.last_heartbeat = now; self.status = MemberStatusV2::Active; }
}

/// Quorum configuration
#[derive(Debug)]
pub struct QuorumConfigV2 {
    pub quorum_type: QuorumTypeV2,
    pub members: BTreeMap<u64, QuorumMemberV2>,
    pub failure_threshold_ms: u64,
}

impl QuorumConfigV2 {
    pub fn new(qt: QuorumTypeV2, threshold_ms: u64) -> Self {
        Self { quorum_type: qt, members: BTreeMap::new(), failure_threshold_ms: threshold_ms }
    }

    pub fn add_member(&mut self, id: u64, weight: u32) { self.members.insert(id, QuorumMemberV2::new(id, weight)); }

    pub fn active_count(&self) -> u32 { self.members.values().filter(|m| m.status == MemberStatusV2::Active).count() as u32 }

    pub fn total_weight(&self) -> u32 { self.members.values().filter(|m| m.status == MemberStatusV2::Active).map(|m| m.weight).sum() }

    pub fn has_quorum(&self) -> bool {
        let active = self.active_count();
        let total = self.members.len() as u32;
        match self.quorum_type {
            QuorumTypeV2::Simple => active >= 1,
            QuorumTypeV2::Majority => active > total / 2,
            QuorumTypeV2::SuperMajority => active * 3 > total * 2,
            QuorumTypeV2::Unanimous => active == total,
            QuorumTypeV2::Weighted => { let tw = self.total_weight(); let full: u32 = self.members.values().map(|m| m.weight).sum(); tw > full / 2 }
        }
    }

    pub fn detect_failures(&mut self, now: u64) {
        for m in self.members.values_mut() {
            if m.status == MemberStatusV2::Active && now.saturating_sub(m.last_heartbeat) > self.failure_threshold_ms * 1_000_000 {
                m.status = MemberStatusV2::Suspected;
            }
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct QuorumMgrV2Stats {
    pub total_members: u32,
    pub active_members: u32,
    pub suspected: u32,
    pub has_quorum: bool,
    pub total_weight: u32,
}

/// Main coop quorum manager v2
pub struct CoopQuorumMgrV2 {
    configs: Vec<QuorumConfigV2>,
}

impl CoopQuorumMgrV2 {
    pub fn new() -> Self { Self { configs: Vec::new() } }

    pub fn create(&mut self, qt: QuorumTypeV2, threshold_ms: u64) -> usize {
        let idx = self.configs.len(); self.configs.push(QuorumConfigV2::new(qt, threshold_ms)); idx
    }

    pub fn add_member(&mut self, idx: usize, id: u64, weight: u32) {
        if let Some(c) = self.configs.get_mut(idx) { c.add_member(id, weight); }
    }

    pub fn stats(&self) -> Vec<QuorumMgrV2Stats> {
        self.configs.iter().map(|c| {
            let suspected = c.members.values().filter(|m| m.status == MemberStatusV2::Suspected).count() as u32;
            QuorumMgrV2Stats { total_members: c.members.len() as u32, active_members: c.active_count(), suspected, has_quorum: c.has_quorum(), total_weight: c.total_weight() }
        }).collect()
    }
}
