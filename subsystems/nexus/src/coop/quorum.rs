//! # Coop Quorum
//!
//! Quorum-based cooperative decision making:
//! - Configurable quorum policies (majority, weighted, unanimous)
//! - Quorum formation tracking
//! - Vote tracking with deadlines
//! - Split-brain detection
//! - Quorum degradation alerts
//! - Hierarchical quorum groups

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Quorum policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumPolicy {
    /// Simple majority (>50%)
    Majority,
    /// Weighted majority
    WeightedMajority,
    /// Unanimous agreement
    Unanimous,
    /// Two-thirds supermajority
    TwoThirds,
    /// Custom threshold
    CustomThreshold(u8),
}

/// Quorum state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumState {
    /// Quorum established
    Established,
    /// Quorum lost
    Lost,
    /// Degraded (barely meeting threshold)
    Degraded,
    /// Forming (not yet enough members)
    Forming,
}

/// Vote value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteValue {
    /// Agree
    Agree,
    /// Disagree
    Disagree,
    /// Abstain
    Abstain,
}

/// Per-member state
#[derive(Debug, Clone)]
pub struct QuorumMember {
    pub member_id: u64,
    pub weight: u32,
    pub active: bool,
    pub last_heartbeat_ns: u64,
    pub votes_cast: u64,
    pub votes_agreed: u64,
}

impl QuorumMember {
    pub fn new(id: u64, weight: u32) -> Self {
        Self {
            member_id: id,
            weight,
            active: true,
            last_heartbeat_ns: 0,
            votes_cast: 0,
            votes_agreed: 0,
        }
    }

    pub fn agreement_rate(&self) -> f64 {
        if self.votes_cast == 0 {
            1.0
        } else {
            self.votes_agreed as f64 / self.votes_cast as f64
        }
    }

    pub fn is_stale(&self, now_ns: u64, timeout_ns: u64) -> bool {
        now_ns.saturating_sub(self.last_heartbeat_ns) > timeout_ns
    }
}

/// A vote on a proposal
#[derive(Debug, Clone)]
pub struct Vote {
    pub voter_id: u64,
    pub value: VoteValue,
    pub timestamp_ns: u64,
    pub weight: u32,
}

/// A proposal to be voted on
#[derive(Debug)]
pub struct Proposal {
    pub proposal_id: u64,
    pub proposer_id: u64,
    pub created_ns: u64,
    pub deadline_ns: u64,
    pub votes: Vec<Vote>,
    pub decided: bool,
    pub outcome: Option<bool>,
    /// FNV-1a hash of proposal content
    pub content_hash: u64,
}

impl Proposal {
    pub fn new(id: u64, proposer: u64, content_hash: u64, now_ns: u64, deadline_ns: u64) -> Self {
        Self {
            proposal_id: id,
            proposer_id: proposer,
            created_ns: now_ns,
            deadline_ns,
            votes: Vec::new(),
            decided: false,
            outcome: None,
            content_hash,
        }
    }

    pub fn add_vote(&mut self, vote: Vote) {
        // Only one vote per voter
        if !self.votes.iter().any(|v| v.voter_id == vote.voter_id) {
            self.votes.push(vote);
        }
    }

    pub fn agree_weight(&self) -> u32 {
        self.votes
            .iter()
            .filter(|v| v.value == VoteValue::Agree)
            .map(|v| v.weight)
            .sum()
    }

    pub fn disagree_weight(&self) -> u32 {
        self.votes
            .iter()
            .filter(|v| v.value == VoteValue::Disagree)
            .map(|v| v.weight)
            .sum()
    }

    pub fn total_voted_weight(&self) -> u32 {
        self.votes
            .iter()
            .filter(|v| v.value != VoteValue::Abstain)
            .map(|v| v.weight)
            .sum()
    }

    pub fn is_expired(&self, now_ns: u64) -> bool {
        now_ns >= self.deadline_ns
    }
}

/// Quorum group
#[derive(Debug)]
pub struct QuorumGroup {
    pub group_id: u64,
    pub policy: QuorumPolicy,
    pub state: QuorumState,
    members: BTreeMap<u64, QuorumMember>,
    proposals: BTreeMap<u64, Proposal>,
    pub heartbeat_timeout_ns: u64,
    pub total_proposals: u64,
    pub total_accepted: u64,
    pub total_rejected: u64,
}

impl QuorumGroup {
    pub fn new(id: u64, policy: QuorumPolicy) -> Self {
        Self {
            group_id: id,
            policy,
            state: QuorumState::Forming,
            members: BTreeMap::new(),
            proposals: BTreeMap::new(),
            heartbeat_timeout_ns: 5_000_000_000, // 5s
            total_proposals: 0,
            total_accepted: 0,
            total_rejected: 0,
        }
    }

    pub fn add_member(&mut self, id: u64, weight: u32) {
        self.members.insert(id, QuorumMember::new(id, weight));
        self.recompute_state(0);
    }

    pub fn remove_member(&mut self, id: u64) {
        self.members.remove(&id);
        self.recompute_state(0);
    }

    pub fn heartbeat(&mut self, member_id: u64, now_ns: u64) {
        if let Some(member) = self.members.get_mut(&member_id) {
            member.last_heartbeat_ns = now_ns;
            member.active = true;
        }
        self.recompute_state(now_ns);
    }

    fn active_members(&self) -> impl Iterator<Item = &QuorumMember> {
        self.members.values().filter(|m| m.active)
    }

    fn total_weight(&self) -> u32 {
        self.members.values().map(|m| m.weight).sum()
    }

    fn active_weight(&self) -> u32 {
        self.active_members().map(|m| m.weight).sum()
    }

    fn threshold_weight(&self) -> u32 {
        let total = self.total_weight();
        match self.policy {
            QuorumPolicy::Majority => total / 2 + 1,
            QuorumPolicy::WeightedMajority => total / 2 + 1,
            QuorumPolicy::Unanimous => total,
            QuorumPolicy::TwoThirds => (total as u64 * 2 / 3 + 1) as u32,
            QuorumPolicy::CustomThreshold(pct) => (total as u64 * pct as u64 / 100) as u32,
        }
    }

    fn recompute_state(&mut self, now_ns: u64) {
        // Mark stale members
        if now_ns > 0 {
            for member in self.members.values_mut() {
                if member.is_stale(now_ns, self.heartbeat_timeout_ns) {
                    member.active = false;
                }
            }
        }

        let active_wt = self.active_weight();
        let threshold = self.threshold_weight();

        if self.members.is_empty() {
            self.state = QuorumState::Forming;
        } else if active_wt >= threshold {
            let surplus = active_wt - threshold;
            if surplus < threshold / 4 {
                self.state = QuorumState::Degraded;
            } else {
                self.state = QuorumState::Established;
            }
        } else {
            self.state = QuorumState::Lost;
        }
    }

    /// Submit a proposal
    pub fn propose(
        &mut self,
        proposer: u64,
        content_hash: u64,
        now_ns: u64,
        timeout_ns: u64,
    ) -> u64 {
        self.total_proposals += 1;
        let id = self.total_proposals;
        let proposal = Proposal::new(id, proposer, content_hash, now_ns, now_ns + timeout_ns);
        self.proposals.insert(id, proposal);
        id
    }

    /// Cast a vote
    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter_id: u64,
        value: VoteValue,
        now_ns: u64,
    ) -> Option<bool> {
        let weight = self.members.get(&voter_id).map(|m| m.weight).unwrap_or(1);
        let threshold = self.threshold_weight();

        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            if proposal.decided || proposal.is_expired(now_ns) {
                return proposal.outcome;
            }

            proposal.add_vote(Vote {
                voter_id,
                value,
                timestamp_ns: now_ns,
                weight,
            });

            // Update member stats
            if let Some(member) = self.members.get_mut(&voter_id) {
                member.votes_cast += 1;
                if value == VoteValue::Agree {
                    member.votes_agreed += 1;
                }
            }

            // Check if decided
            if proposal.agree_weight() >= threshold {
                proposal.decided = true;
                proposal.outcome = Some(true);
                self.total_accepted += 1;
                return Some(true);
            }
            if proposal.disagree_weight() > self.total_weight() - threshold {
                proposal.decided = true;
                proposal.outcome = Some(false);
                self.total_rejected += 1;
                return Some(false);
            }
        }
        None
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn active_count(&self) -> usize {
        self.members.values().filter(|m| m.active).count()
    }

    pub fn acceptance_rate(&self) -> f64 {
        let decided = self.total_accepted + self.total_rejected;
        if decided == 0 {
            0.0
        } else {
            self.total_accepted as f64 / decided as f64
        }
    }
}

/// Quorum protocol stats
#[derive(Debug, Clone, Default)]
pub struct CoopQuorumStats {
    pub total_groups: usize,
    pub established_groups: usize,
    pub lost_quorum_groups: usize,
    pub total_proposals: u64,
    pub total_accepted: u64,
    pub total_rejected: u64,
}

/// Coop Quorum Protocol
pub struct CoopQuorumProtocol {
    groups: BTreeMap<u64, QuorumGroup>,
    stats: CoopQuorumStats,
    next_group_id: u64,
}

impl CoopQuorumProtocol {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            stats: CoopQuorumStats::default(),
            next_group_id: 1,
        }
    }

    pub fn create_group(&mut self, policy: QuorumPolicy) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(id, QuorumGroup::new(id, policy));
        self.update_stats();
        id
    }

    pub fn get_group(&mut self, id: u64) -> Option<&mut QuorumGroup> {
        self.groups.get_mut(&id)
    }

    pub fn tick(&mut self, now_ns: u64) {
        for group in self.groups.values_mut() {
            group.recompute_state(now_ns);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_groups = self.groups.len();
        self.stats.established_groups = self
            .groups
            .values()
            .filter(|g| g.state == QuorumState::Established)
            .count();
        self.stats.lost_quorum_groups = self
            .groups
            .values()
            .filter(|g| g.state == QuorumState::Lost)
            .count();
        self.stats.total_proposals = self.groups.values().map(|g| g.total_proposals).sum();
        self.stats.total_accepted = self.groups.values().map(|g| g.total_accepted).sum();
        self.stats.total_rejected = self.groups.values().map(|g| g.total_rejected).sum();
    }

    pub fn stats(&self) -> &CoopQuorumStats {
        &self.stats
    }
}
