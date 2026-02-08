// SPDX-License-Identifier: GPL-2.0
//! Coop consensus_mgr — distributed consensus protocol for cooperative decisions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Consensus algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusAlgorithm {
    Majority,
    Unanimous,
    Weighted,
    Raft,
    TwoPhaseCommit,
}

/// Vote value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Accept,
    Reject,
    Abstain,
}

/// Proposal state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalState {
    Pending,
    Voting,
    Accepted,
    Rejected,
    TimedOut,
    Committed,
    Aborted,
}

/// Voter record
#[derive(Debug, Clone)]
pub struct VoterRecord {
    pub voter_id: u32,
    pub vote: Vote,
    pub weight: u32,
    pub voted_at: u64,
    pub reason_code: u32,
}

/// Consensus proposal
#[derive(Debug)]
pub struct Proposal {
    pub id: u64,
    pub proposer_id: u32,
    pub algorithm: ConsensusAlgorithm,
    pub state: ProposalState,
    pub voters: Vec<VoterRecord>,
    pub required_voters: u32,
    pub total_weight: u32,
    pub accept_threshold: u32,
    pub timeout_ns: u64,
    pub created_at: u64,
    pub resolved_at: u64,
    pub round: u32,
    pub max_rounds: u32,
}

impl Proposal {
    pub fn new(id: u64, proposer: u32, algo: ConsensusAlgorithm, required: u32, timeout: u64, now: u64) -> Self {
        let threshold = match algo {
            ConsensusAlgorithm::Majority => (required / 2) + 1,
            ConsensusAlgorithm::Unanimous => required,
            ConsensusAlgorithm::Weighted => 0, // set externally
            ConsensusAlgorithm::Raft => (required / 2) + 1,
            ConsensusAlgorithm::TwoPhaseCommit => required,
        };
        Self {
            id, proposer_id: proposer, algorithm: algo,
            state: ProposalState::Pending, voters: Vec::new(),
            required_voters: required, total_weight: 0,
            accept_threshold: threshold, timeout_ns: timeout,
            created_at: now, resolved_at: 0, round: 1, max_rounds: 3,
        }
    }

    pub fn start_voting(&mut self) {
        self.state = ProposalState::Voting;
    }

    pub fn cast_vote(&mut self, voter_id: u32, vote: Vote, weight: u32, now: u64) -> bool {
        if self.state != ProposalState::Voting { return false; }
        if self.voters.iter().any(|v| v.voter_id == voter_id) { return false; }
        self.voters.push(VoterRecord {
            voter_id, vote, weight, voted_at: now, reason_code: 0,
        });
        self.total_weight += weight;
        true
    }

    pub fn accept_count(&self) -> u32 {
        self.voters.iter().filter(|v| v.vote == Vote::Accept).count() as u32
    }

    pub fn reject_count(&self) -> u32 {
        self.voters.iter().filter(|v| v.vote == Vote::Reject).count() as u32
    }

    pub fn accept_weight(&self) -> u32 {
        self.voters.iter().filter(|v| v.vote == Vote::Accept).map(|v| v.weight).sum()
    }

    pub fn try_resolve(&mut self, now: u64) -> bool {
        if self.state != ProposalState::Voting { return false; }

        match self.algorithm {
            ConsensusAlgorithm::Majority | ConsensusAlgorithm::Raft => {
                if self.accept_count() >= self.accept_threshold {
                    self.state = ProposalState::Accepted;
                    self.resolved_at = now;
                    return true;
                }
                if self.reject_count() > self.required_voters - self.accept_threshold {
                    self.state = ProposalState::Rejected;
                    self.resolved_at = now;
                    return true;
                }
            }
            ConsensusAlgorithm::Unanimous | ConsensusAlgorithm::TwoPhaseCommit => {
                if self.reject_count() > 0 {
                    self.state = ProposalState::Rejected;
                    self.resolved_at = now;
                    return true;
                }
                if self.accept_count() >= self.required_voters {
                    self.state = ProposalState::Accepted;
                    self.resolved_at = now;
                    return true;
                }
            }
            ConsensusAlgorithm::Weighted => {
                if self.accept_weight() >= self.accept_threshold {
                    self.state = ProposalState::Accepted;
                    self.resolved_at = now;
                    return true;
                }
            }
        }
        false
    }

    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.state != ProposalState::Voting { return false; }
        if self.timeout_ns > 0 && now.saturating_sub(self.created_at) >= self.timeout_ns {
            self.state = ProposalState::TimedOut;
            self.resolved_at = now;
            return true;
        }
        false
    }

    pub fn completion_ratio(&self) -> f64 {
        if self.required_voters == 0 { return 0.0; }
        self.voters.len() as f64 / self.required_voters as f64
    }

    pub fn duration(&self) -> u64 {
        if self.resolved_at > 0 { self.resolved_at - self.created_at } else { 0 }
    }
}

/// Consensus stats
#[derive(Debug, Clone)]
pub struct ConsensusMgrStats {
    pub active_proposals: u32,
    pub total_proposed: u64,
    pub total_accepted: u64,
    pub total_rejected: u64,
    pub total_timed_out: u64,
    pub total_votes_cast: u64,
}

/// Main consensus manager
pub struct CoopConsensusMgr {
    proposals: BTreeMap<u64, Proposal>,
    history: Vec<Proposal>,
    max_history: usize,
    next_id: u64,
    total_proposed: u64,
    total_accepted: u64,
    total_rejected: u64,
    total_timed_out: u64,
    total_votes: u64,
}

impl CoopConsensusMgr {
    pub fn new(max_history: usize) -> Self {
        Self {
            proposals: BTreeMap::new(), history: Vec::new(),
            max_history, next_id: 1, total_proposed: 0,
            total_accepted: 0, total_rejected: 0,
            total_timed_out: 0, total_votes: 0,
        }
    }

    pub fn propose(&mut self, proposer: u32, algo: ConsensusAlgorithm, required: u32, timeout: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.total_proposed += 1;
        let mut p = Proposal::new(id, proposer, algo, required, timeout, now);
        p.start_voting();
        self.proposals.insert(id, p);
        id
    }

    pub fn vote(&mut self, proposal_id: u64, voter: u32, vote: Vote, weight: u32, now: u64) -> bool {
        if let Some(p) = self.proposals.get_mut(&proposal_id) {
            if p.cast_vote(voter, vote, weight, now) {
                self.total_votes += 1;
                p.try_resolve(now);
                if p.state == ProposalState::Accepted { self.total_accepted += 1; }
                if p.state == ProposalState::Rejected { self.total_rejected += 1; }
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self, now: u64) {
        for p in self.proposals.values_mut() {
            if p.check_timeout(now) {
                self.total_timed_out += 1;
            }
        }
    }

    pub fn collect_resolved(&mut self) -> Vec<u64> {
        let resolved: Vec<u64> = self.proposals.iter()
            .filter(|(_, p)| matches!(p.state, ProposalState::Accepted | ProposalState::Rejected | ProposalState::TimedOut))
            .map(|(&id, _)| id)
            .collect();
        for id in &resolved {
            if let Some(p) = self.proposals.remove(id) {
                if self.history.len() >= self.max_history { self.history.remove(0); }
                self.history.push(p);
            }
        }
        resolved
    }

    pub fn stats(&self) -> ConsensusMgrStats {
        ConsensusMgrStats {
            active_proposals: self.proposals.len() as u32,
            total_proposed: self.total_proposed,
            total_accepted: self.total_accepted,
            total_rejected: self.total_rejected,
            total_timed_out: self.total_timed_out,
            total_votes_cast: self.total_votes,
        }
    }
}

// ============================================================================
// Merged from consensus_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusV2State {
    Idle,
    Proposing,
    Voting,
    Committed,
    Aborted,
}

/// Vote type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteV2 {
    Accept,
    Reject,
    Abstain,
}

/// Proposal
#[derive(Debug)]
pub struct ProposalV2 {
    pub id: u64,
    pub proposer: u64,
    pub round: u64,
    pub value_hash: u64,
    pub state: ConsensusV2State,
    pub votes: BTreeMap<u64, VoteV2>,
    pub quorum_size: u32,
    pub timestamp: u64,
}

impl ProposalV2 {
    pub fn new(id: u64, proposer: u64, round: u64, value_hash: u64, quorum: u32, now: u64) -> Self {
        Self { id, proposer, round, value_hash, state: ConsensusV2State::Proposing, votes: BTreeMap::new(), quorum_size: quorum, timestamp: now }
    }

    pub fn vote(&mut self, voter: u64, v: VoteV2) {
        self.votes.insert(voter, v);
        let accepts = self.votes.values().filter(|&&vt| vt == VoteV2::Accept).count() as u32;
        let rejects = self.votes.values().filter(|&&vt| vt == VoteV2::Reject).count() as u32;
        if accepts >= self.quorum_size { self.state = ConsensusV2State::Committed; }
        else if rejects >= self.quorum_size { self.state = ConsensusV2State::Aborted; }
    }

    pub fn is_decided(&self) -> bool {
        self.state == ConsensusV2State::Committed || self.state == ConsensusV2State::Aborted
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ConsensusV2Stats {
    pub total_proposals: u32,
    pub committed: u32,
    pub aborted: u32,
    pub pending: u32,
    pub current_round: u64,
}

/// Main coop consensus manager v2
pub struct CoopConsensusMgrV2 {
    proposals: BTreeMap<u64, ProposalV2>,
    current_round: u64,
    next_id: u64,
}

impl CoopConsensusMgrV2 {
    pub fn new() -> Self { Self { proposals: BTreeMap::new(), current_round: 0, next_id: 1 } }

    pub fn propose(&mut self, proposer: u64, value_hash: u64, quorum: u32, now: u64) -> u64 {
        self.current_round += 1;
        let id = self.next_id; self.next_id += 1;
        self.proposals.insert(id, ProposalV2::new(id, proposer, self.current_round, value_hash, quorum, now));
        id
    }

    pub fn vote(&mut self, proposal_id: u64, voter: u64, v: VoteV2) {
        if let Some(p) = self.proposals.get_mut(&proposal_id) { p.vote(voter, v); }
    }

    pub fn stats(&self) -> ConsensusV2Stats {
        let committed = self.proposals.values().filter(|p| p.state == ConsensusV2State::Committed).count() as u32;
        let aborted = self.proposals.values().filter(|p| p.state == ConsensusV2State::Aborted).count() as u32;
        let pending = self.proposals.len() as u32 - committed - aborted;
        ConsensusV2Stats { total_proposals: self.proposals.len() as u32, committed, aborted, pending, current_round: self.current_round }
    }
}
