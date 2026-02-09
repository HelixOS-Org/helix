//! # Coop Quorum Tracker
//!
//! Quorum tracking and management for cooperative protocols:
//! - Configurable quorum sizes (majority, supermajority, unanimous)
//! - Weighted quorum support
//! - Joint quorum for membership changes
//! - Quorum intersection proofs
//! - Vote tracking with timeout
//! - Flexible quorum policies

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Quorum policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumPolicy {
    Majority,
    SuperMajority,
    Unanimous,
    Fixed(u32),
    Weighted(u32),
    Joint,
}

/// Vote value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteValue {
    Yes,
    No,
    Abstain,
    Timeout,
}

/// A single vote
#[derive(Debug, Clone)]
pub struct QuorumVote {
    pub voter_id: u64,
    pub value: VoteValue,
    pub weight: u32,
    pub ts: u64,
    pub term: u64,
    pub metadata: u64,
}

impl QuorumVote {
    pub fn new(voter: u64, value: VoteValue, weight: u32, ts: u64, term: u64) -> Self {
        Self { voter_id: voter, value, weight, ts, term, metadata: 0 }
    }
}

/// Ballot — a quorum decision instance
#[derive(Debug, Clone)]
pub struct Ballot {
    pub id: u64,
    pub term: u64,
    pub policy: QuorumPolicy,
    pub voters: LinearMap<u32, 64>,
    pub votes: Vec<QuorumVote>,
    pub required: u32,
    pub yes_weight: u32,
    pub no_weight: u32,
    pub abstain_weight: u32,
    pub decided: bool,
    pub accepted: bool,
    pub create_ts: u64,
    pub decide_ts: u64,
    pub timeout_ns: u64,
}

impl Ballot {
    pub fn new(id: u64, term: u64, policy: QuorumPolicy, voters: LinearMap<u32, 64>, ts: u64, timeout: u64) -> Self {
        let total_weight: u32 = voters.values().sum();
        let required = match policy {
            QuorumPolicy::Majority => total_weight / 2 + 1,
            QuorumPolicy::SuperMajority => (total_weight * 2 + 2) / 3,
            QuorumPolicy::Unanimous => total_weight,
            QuorumPolicy::Fixed(n) => n,
            QuorumPolicy::Weighted(w) => w,
            QuorumPolicy::Joint => total_weight / 2 + 1,
        };
        Self {
            id, term, policy, voters, votes: Vec::new(), required,
            yes_weight: 0, no_weight: 0, abstain_weight: 0,
            decided: false, accepted: false, create_ts: ts,
            decide_ts: 0, timeout_ns: timeout,
        }
    }

    pub fn cast(&mut self, vote: QuorumVote) -> bool {
        if self.decided { return false; }
        if !self.voters.contains_key(&vote.voter_id) { return false; }
        if self.votes.iter().any(|v| v.voter_id == vote.voter_id) { return false; }
        let w = *self.voters.get(&vote.voter_id).unwrap_or(&0);
        match vote.value {
            VoteValue::Yes => self.yes_weight += w,
            VoteValue::No => self.no_weight += w,
            VoteValue::Abstain => self.abstain_weight += w,
            VoteValue::Timeout => self.no_weight += w,
        }
        self.votes.push(vote);
        self.check_decided();
        true
    }

    fn check_decided(&mut self) {
        if self.decided { return; }
        if self.yes_weight >= self.required {
            self.decided = true;
            self.accepted = true;
        } else if self.no_weight + self.abstain_weight >= self.required {
            self.decided = true;
            self.accepted = false;
        }
        let total: u32 = self.voters.values().sum();
        let remaining = total.saturating_sub(self.yes_weight + self.no_weight + self.abstain_weight);
        if self.yes_weight + remaining < self.required {
            self.decided = true;
            self.accepted = false;
        }
    }

    #[inline]
    pub fn finalize(&mut self, ts: u64) {
        if !self.decided {
            self.decided = true;
            self.accepted = self.yes_weight >= self.required;
        }
        self.decide_ts = ts;
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool { now.saturating_sub(self.create_ts) > self.timeout_ns }
    #[inline(always)]
    pub fn participation(&self) -> f64 { let t: u32 = self.voters.values().sum(); if t == 0 { 0.0 } else { (self.yes_weight + self.no_weight + self.abstain_weight) as f64 / t as f64 } }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.decide_ts.saturating_sub(self.create_ts) }
}

/// Joint quorum (for membership changes)
#[derive(Debug, Clone)]
pub struct JointQuorum {
    pub old_members: LinearMap<u32, 64>,
    pub new_members: LinearMap<u32, 64>,
    pub old_ballot: u64,
    pub new_ballot: u64,
    pub committed: bool,
}

impl JointQuorum {
    pub fn new(old: LinearMap<u32, 64>, new: LinearMap<u32, 64>) -> Self {
        Self { old_members: old, new_members: new, old_ballot: 0, new_ballot: 0, committed: false }
    }

    #[inline]
    pub fn both_decided(&self, ballots: &BTreeMap<u64, Ballot>) -> bool {
        let old_ok = ballots.get(&self.old_ballot).map(|b| b.decided && b.accepted).unwrap_or(false);
        let new_ok = ballots.get(&self.new_ballot).map(|b| b.decided && b.accepted).unwrap_or(false);
        old_ok && new_ok
    }
}

/// Quorum tracker stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct QuorumStats {
    pub total_ballots: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub expired: u64,
    pub pending: u64,
    pub avg_latency_ns: u64,
    pub avg_participation: f64,
}

/// Cooperative quorum tracker
pub struct CoopQuorumTracker {
    ballots: BTreeMap<u64, Ballot>,
    joints: Vec<JointQuorum>,
    stats: QuorumStats,
    next_id: u64,
    default_timeout: u64,
}

impl CoopQuorumTracker {
    pub fn new(default_timeout: u64) -> Self {
        Self { ballots: BTreeMap::new(), joints: Vec::new(), stats: QuorumStats::default(), next_id: 1, default_timeout }
    }

    #[inline]
    pub fn create_ballot(&mut self, term: u64, policy: QuorumPolicy, voters: LinearMap<u32, 64>, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let ballot = Ballot::new(id, term, policy, voters, ts, self.default_timeout);
        self.ballots.insert(id, ballot);
        self.stats.total_ballots += 1;
        id
    }

    #[inline(always)]
    pub fn vote(&mut self, ballot_id: u64, vote: QuorumVote) -> bool {
        if let Some(b) = self.ballots.get_mut(&ballot_id) { b.cast(vote) } else { false }
    }

    #[inline]
    pub fn check_expired(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for b in self.ballots.values_mut() {
            if !b.decided && b.is_expired(now) {
                b.finalize(now);
                expired.push(b.id);
            }
        }
        expired
    }

    #[inline]
    pub fn create_joint(&mut self, old: LinearMap<u32, 64>, new: LinearMap<u32, 64>, term: u64, ts: u64) -> (u64, u64) {
        let bid1 = self.create_ballot(term, QuorumPolicy::Majority, old.clone(), ts);
        let bid2 = self.create_ballot(term, QuorumPolicy::Majority, new.clone(), ts);
        let mut jq = JointQuorum::new(old, new);
        jq.old_ballot = bid1;
        jq.new_ballot = bid2;
        self.joints.push(jq);
        (bid1, bid2)
    }

    #[inline]
    pub fn check_joints(&mut self) -> Vec<usize> {
        let mut committed = Vec::new();
        for (i, jq) in self.joints.iter_mut().enumerate() {
            if !jq.committed && jq.both_decided(&self.ballots) {
                jq.committed = true;
                committed.push(i);
            }
        }
        committed
    }

    pub fn recompute(&mut self) {
        self.stats.accepted = self.ballots.values().filter(|b| b.decided && b.accepted).count() as u64;
        self.stats.rejected = self.ballots.values().filter(|b| b.decided && !b.accepted).count() as u64;
        self.stats.pending = self.ballots.values().filter(|b| !b.decided).count() as u64;
        let decided: Vec<&Ballot> = self.ballots.values().filter(|b| b.decided && b.decide_ts > 0).collect();
        if !decided.is_empty() {
            let total_lat: u64 = decided.iter().map(|b| b.latency()).sum();
            self.stats.avg_latency_ns = total_lat / decided.len() as u64;
            let total_part: f64 = decided.iter().map(|b| b.participation()).sum();
            self.stats.avg_participation = total_part / decided.len() as f64;
        }
    }

    #[inline(always)]
    pub fn ballot(&self, id: u64) -> Option<&Ballot> { self.ballots.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &QuorumStats { &self.stats }
}
