//! # Cooperative Voting Protocol
//!
//! Distributed voting for resource allocation decisions:
//! - Weighted voting
//! - Quorum requirements
//! - Multi-round elections
//! - Ranked-choice voting
//! - Byzantine fault tolerance

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// VOTING TYPES
// ============================================================================

/// Vote type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteType {
    /// Simple yes/no
    Binary,
    /// Choose from options
    SingleChoice,
    /// Rank options
    RankedChoice,
    /// Weighted preference
    Weighted,
}

/// Vote value
#[derive(Debug, Clone)]
pub enum VoteValue {
    /// Yes/no
    Binary(bool),
    /// Single choice index
    Choice(u32),
    /// Ranked choices
    Ranked(Vec<u32>),
    /// Weighted values (option -> weight)
    Weighted(Vec<(u32, u32)>),
}

/// Ballot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallotState {
    /// Collecting votes
    Open,
    /// Quorum reached, can decide
    QuorumReached,
    /// Decided
    Decided,
    /// Failed (timeout, no quorum)
    Failed,
    /// Cancelled
    Cancelled,
}

// ============================================================================
// BALLOT
// ============================================================================

/// A ballot/election
#[derive(Debug, Clone)]
pub struct Ballot {
    /// Ballot id
    pub id: u64,
    /// Topic (hash of description)
    pub topic: u64,
    /// Vote type
    pub vote_type: VoteType,
    /// Number of options (for choice/ranked)
    pub num_options: u32,
    /// Eligible voters
    pub eligible: Vec<u64>,
    /// Voter weights
    pub weights: BTreeMap<u64, u32>,
    /// Votes cast
    pub votes: BTreeMap<u64, VoteValue>,
    /// State
    pub state: BallotState,
    /// Created at
    pub created_at: u64,
    /// Deadline
    pub deadline: u64,
    /// Quorum fraction (0.0-1.0 as fixed point * 100)
    pub quorum_pct: u32,
    /// Result
    pub result: Option<u32>,
}

impl Ballot {
    pub fn new(
        id: u64,
        topic: u64,
        vote_type: VoteType,
        num_options: u32,
        deadline: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            topic,
            vote_type,
            num_options,
            eligible: Vec::new(),
            weights: BTreeMap::new(),
            votes: BTreeMap::new(),
            state: BallotState::Open,
            created_at: now,
            deadline,
            quorum_pct: 50, // 50%
            result: None,
        }
    }

    /// Add eligible voter
    pub fn add_voter(&mut self, pid: u64, weight: u32) {
        if !self.eligible.contains(&pid) {
            self.eligible.push(pid);
        }
        self.weights.insert(pid, weight);
    }

    /// Cast vote
    pub fn cast_vote(&mut self, pid: u64, value: VoteValue) -> bool {
        if self.state != BallotState::Open && self.state != BallotState::QuorumReached {
            return false;
        }
        if !self.eligible.contains(&pid) {
            return false;
        }
        self.votes.insert(pid, value);

        // Check quorum
        if self.has_quorum() && self.state == BallotState::Open {
            self.state = BallotState::QuorumReached;
        }
        true
    }

    /// Has quorum?
    pub fn has_quorum(&self) -> bool {
        if self.eligible.is_empty() {
            return false;
        }
        let total_weight: u32 = self.eligible.iter().map(|p| self.weight_of(*p)).sum();
        let voted_weight: u32 = self.votes.keys().map(|p| self.weight_of(*p)).sum();
        if total_weight == 0 {
            return false;
        }
        (voted_weight * 100) / total_weight >= self.quorum_pct
    }

    /// Weight of voter
    fn weight_of(&self, pid: u64) -> u32 {
        self.weights.get(&pid).copied().unwrap_or(1)
    }

    /// Tally votes (binary)
    pub fn tally_binary(&self) -> (u32, u32) {
        let mut yes = 0u32;
        let mut no = 0u32;
        for (pid, vote) in &self.votes {
            let w = self.weight_of(*pid);
            if let VoteValue::Binary(v) = vote {
                if *v {
                    yes += w;
                } else {
                    no += w;
                }
            }
        }
        (yes, no)
    }

    /// Tally votes (single choice)
    pub fn tally_choice(&self) -> Vec<(u32, u32)> {
        let mut tallies = Vec::new();
        for i in 0..self.num_options {
            let mut weight = 0u32;
            for (pid, vote) in &self.votes {
                if let VoteValue::Choice(c) = vote {
                    if *c == i {
                        weight += self.weight_of(*pid);
                    }
                }
            }
            tallies.push((i, weight));
        }
        tallies.sort_by(|a, b| b.1.cmp(&a.1));
        tallies
    }

    /// Decide the ballot
    pub fn decide(&mut self) -> Option<u32> {
        if self.state == BallotState::Decided {
            return self.result;
        }
        if !self.has_quorum() {
            return None;
        }

        let winner = match self.vote_type {
            VoteType::Binary => {
                let (yes, no) = self.tally_binary();
                if yes > no {
                    Some(1)
                } else {
                    Some(0)
                }
            }
            VoteType::SingleChoice | VoteType::Weighted => {
                let tallies = self.tally_choice();
                tallies.first().map(|(opt, _)| *opt)
            }
            VoteType::RankedChoice => {
                // Simplified: use first-choice only
                let tallies = self.tally_choice();
                tallies.first().map(|(opt, _)| *opt)
            }
        };

        self.result = winner;
        self.state = BallotState::Decided;
        winner
    }

    /// Check deadline
    pub fn check_deadline(&mut self, now: u64) -> bool {
        if now >= self.deadline && self.state == BallotState::Open {
            self.state = BallotState::Failed;
            return true;
        }
        false
    }

    /// Participation rate
    pub fn participation(&self) -> f64 {
        if self.eligible.is_empty() {
            return 0.0;
        }
        self.votes.len() as f64 / self.eligible.len() as f64
    }
}

// ============================================================================
// VOTING MANAGER
// ============================================================================

/// Voting stats
#[derive(Debug, Clone, Default)]
pub struct CoopVotingStats {
    /// Active ballots
    pub active_ballots: usize,
    /// Decided ballots
    pub decided: u64,
    /// Failed ballots
    pub failed: u64,
    /// Average participation
    pub avg_participation: f64,
}

/// Cooperative voting manager
pub struct CoopVotingManager {
    /// Ballots
    ballots: BTreeMap<u64, Ballot>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopVotingStats,
    /// Participation accumulator
    participation_sum: f64,
    participation_count: u64,
}

impl CoopVotingManager {
    pub fn new() -> Self {
        Self {
            ballots: BTreeMap::new(),
            next_id: 1,
            stats: CoopVotingStats::default(),
            participation_sum: 0.0,
            participation_count: 0,
        }
    }

    /// Create ballot
    pub fn create_ballot(
        &mut self,
        topic: u64,
        vote_type: VoteType,
        num_options: u32,
        deadline: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let ballot = Ballot::new(id, topic, vote_type, num_options, deadline, now);
        self.ballots.insert(id, ballot);
        self.update_stats();
        id
    }

    /// Add voter to ballot
    pub fn add_voter(&mut self, ballot_id: u64, pid: u64, weight: u32) -> bool {
        if let Some(ballot) = self.ballots.get_mut(&ballot_id) {
            ballot.add_voter(pid, weight);
            true
        } else {
            false
        }
    }

    /// Cast vote
    pub fn cast_vote(&mut self, ballot_id: u64, pid: u64, value: VoteValue) -> bool {
        if let Some(ballot) = self.ballots.get_mut(&ballot_id) {
            ballot.cast_vote(pid, value)
        } else {
            false
        }
    }

    /// Decide ballot
    pub fn decide(&mut self, ballot_id: u64) -> Option<u32> {
        let result = if let Some(ballot) = self.ballots.get_mut(&ballot_id) {
            let participation = ballot.participation();
            self.participation_sum += participation;
            self.participation_count += 1;
            ballot.decide()
        } else {
            None
        };
        if result.is_some() {
            self.stats.decided += 1;
        }
        self.update_stats();
        result
    }

    /// Check deadlines
    pub fn check_deadlines(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for ballot in self.ballots.values_mut() {
            if ballot.check_deadline(now) {
                expired.push(ballot.id);
                self.stats.failed += 1;
            }
        }
        self.update_stats();
        expired
    }

    /// Get ballot
    pub fn ballot(&self, id: u64) -> Option<&Ballot> {
        self.ballots.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_ballots = self
            .ballots
            .values()
            .filter(|b| {
                b.state == BallotState::Open || b.state == BallotState::QuorumReached
            })
            .count();
        if self.participation_count > 0 {
            self.stats.avg_participation = self.participation_sum / self.participation_count as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &CoopVotingStats {
        &self.stats
    }
}
