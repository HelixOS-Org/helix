//! # Cooperative Consensus
//!
//! Consensus mechanisms for cooperative resource decisions:
//! - Voting-based consensus
//! - Weighted voting by trust score
//! - Quorum requirements
//! - Round-based proposals
//! - Byzantine fault tolerance (basic)

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONSENSUS TYPES
// ============================================================================

/// Consensus algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusAlgorithm {
    /// Simple majority
    SimpleMajority,
    /// Super majority (2/3)
    SuperMajority,
    /// Weighted voting
    WeightedVoting,
    /// Unanimity
    Unanimity,
    /// Leader-based
    LeaderBased,
}

/// Proposal state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalState {
    /// Draft
    Draft,
    /// Open for voting
    Open,
    /// Quorum reached
    QuorumReached,
    /// Accepted
    Accepted,
    /// Rejected
    Rejected,
    /// Expired
    Expired,
    /// Withdrawn
    Withdrawn,
}

/// Vote type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteType {
    /// Accept
    Accept,
    /// Reject
    Reject,
    /// Abstain
    Abstain,
}

// ============================================================================
// PROPOSAL
// ============================================================================

/// Proposal for cooperative decision
#[derive(Debug, Clone)]
pub struct Proposal {
    /// Proposal ID
    pub id: u64,
    /// Proposer PID
    pub proposer: u64,
    /// Proposal type
    pub proposal_type: ProposalType,
    /// State
    pub state: ProposalState,
    /// Eligible voters
    pub eligible_voters: Vec<u64>,
    /// Votes
    pub votes: BTreeMap<u64, VoteRecord>,
    /// Algorithm
    pub algorithm: ConsensusAlgorithm,
    /// Quorum (minimum participation)
    pub quorum: f64,
    /// Threshold (fraction to accept)
    pub threshold: f64,
    /// Created at
    pub created_at: u64,
    /// Deadline
    pub deadline: u64,
    /// Round number
    pub round: u32,
}

/// Proposal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalType {
    /// Resource reallocation
    ResourceRealloc,
    /// Priority change
    PriorityChange,
    /// Policy update
    PolicyUpdate,
    /// Membership change
    MembershipChange,
    /// Emergency action
    EmergencyAction,
}

/// Vote record
#[derive(Debug, Clone)]
pub struct VoteRecord {
    /// Voter PID
    pub voter: u64,
    /// Vote
    pub vote: VoteType,
    /// Weight
    pub weight: f64,
    /// Timestamp
    pub timestamp: u64,
}

impl Proposal {
    pub fn new(
        id: u64,
        proposer: u64,
        proposal_type: ProposalType,
        algorithm: ConsensusAlgorithm,
        voters: Vec<u64>,
        duration_ns: u64,
        now: u64,
    ) -> Self {
        let (quorum, threshold) = match algorithm {
            ConsensusAlgorithm::SimpleMajority => (0.5, 0.5),
            ConsensusAlgorithm::SuperMajority => (0.67, 0.67),
            ConsensusAlgorithm::WeightedVoting => (0.5, 0.5),
            ConsensusAlgorithm::Unanimity => (1.0, 1.0),
            ConsensusAlgorithm::LeaderBased => (0.0, 0.0),
        };

        Self {
            id,
            proposer,
            proposal_type,
            state: ProposalState::Draft,
            eligible_voters: voters,
            votes: LinearMap::new(),
            algorithm,
            quorum,
            threshold,
            created_at: now,
            deadline: now + duration_ns,
            round: 1,
        }
    }

    /// Open for voting
    #[inline]
    pub fn open(&mut self) {
        if self.state == ProposalState::Draft {
            self.state = ProposalState::Open;
        }
    }

    /// Cast vote
    pub fn cast_vote(&mut self, voter: u64, vote: VoteType, weight: f64, now: u64) -> bool {
        if self.state != ProposalState::Open {
            return false;
        }
        if !self.eligible_voters.contains(&voter) {
            return false;
        }
        self.votes.insert(
            voter,
            VoteRecord {
                voter,
                vote,
                weight,
                timestamp: now,
            },
        );
        true
    }

    /// Participation rate
    #[inline]
    pub fn participation(&self) -> f64 {
        if self.eligible_voters.is_empty() {
            return 0.0;
        }
        self.votes.len() as f64 / self.eligible_voters.len() as f64
    }

    /// Check if quorum is reached
    #[inline(always)]
    pub fn has_quorum(&self) -> bool {
        self.participation() >= self.quorum
    }

    /// Tally votes and determine outcome
    pub fn tally(&mut self) -> ProposalState {
        if !self.has_quorum() {
            return self.state;
        }

        let (accept_weight, reject_weight, total_weight) = match self.algorithm {
            ConsensusAlgorithm::WeightedVoting => {
                let mut accept = 0.0;
                let mut reject = 0.0;
                let mut total = 0.0;
                for record in self.votes.values() {
                    total += record.weight;
                    match record.vote {
                        VoteType::Accept => accept += record.weight,
                        VoteType::Reject => reject += record.weight,
                        VoteType::Abstain => {}
                    }
                }
                (accept, reject, total)
            }
            _ => {
                let mut accept = 0.0;
                let mut reject = 0.0;
                let total = self.votes.len() as f64;
                for record in self.votes.values() {
                    match record.vote {
                        VoteType::Accept => accept += 1.0,
                        VoteType::Reject => reject += 1.0,
                        VoteType::Abstain => {}
                    }
                }
                (accept, reject, total)
            }
        };

        if total_weight == 0.0 {
            return self.state;
        }

        let accept_ratio = accept_weight / total_weight;
        let _ = reject_weight; // used for logging

        self.state = if accept_ratio >= self.threshold {
            ProposalState::Accepted
        } else {
            ProposalState::Rejected
        };

        self.state
    }

    /// Check expiry
    #[inline]
    pub fn check_expiry(&mut self, now: u64) {
        if now >= self.deadline && self.state == ProposalState::Open {
            self.tally();
            if self.state == ProposalState::Open {
                self.state = ProposalState::Expired;
            }
        }
    }
}

// ============================================================================
// CONSENSUS MANAGER
// ============================================================================

/// Consensus stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopConsensusStats {
    /// Active proposals
    pub active_proposals: usize,
    /// Total proposals
    pub total_proposals: u64,
    /// Accepted
    pub accepted: u64,
    /// Rejected
    pub rejected: u64,
    /// Expired
    pub expired: u64,
    /// Average participation
    pub avg_participation: f64,
}

/// Cooperative consensus manager
pub struct CoopConsensusManager {
    /// Proposals
    proposals: BTreeMap<u64, Proposal>,
    /// Voter weights
    voter_weights: LinearMap<f64, 64>,
    /// Next proposal ID
    next_id: u64,
    /// Stats
    stats: CoopConsensusStats,
}

impl CoopConsensusManager {
    pub fn new() -> Self {
        Self {
            proposals: BTreeMap::new(),
            voter_weights: LinearMap::new(),
            next_id: 1,
            stats: CoopConsensusStats::default(),
        }
    }

    /// Set voter weight
    #[inline(always)]
    pub fn set_voter_weight(&mut self, pid: u64, weight: f64) {
        self.voter_weights.insert(pid, weight);
    }

    /// Create proposal
    #[inline]
    pub fn create_proposal(
        &mut self,
        proposer: u64,
        proposal_type: ProposalType,
        algorithm: ConsensusAlgorithm,
        voters: Vec<u64>,
        duration_ns: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let proposal = Proposal::new(id, proposer, proposal_type, algorithm, voters, duration_ns, now);
        self.proposals.insert(id, proposal);
        self.stats.total_proposals += 1;
        id
    }

    /// Open proposal
    #[inline]
    pub fn open_proposal(&mut self, id: u64) {
        if let Some(p) = self.proposals.get_mut(&id) {
            p.open();
        }
        self.update_active();
    }

    /// Vote on proposal
    #[inline]
    pub fn vote(&mut self, proposal_id: u64, voter: u64, vote: VoteType, now: u64) -> bool {
        let weight = self.voter_weights.get(voter).copied().unwrap_or(1.0);
        if let Some(proposal) = self.proposals.get_mut(&proposal_id) {
            return proposal.cast_vote(voter, vote, weight, now);
        }
        false
    }

    /// Finalize proposal
    #[inline]
    pub fn finalize(&mut self, proposal_id: u64) -> Option<ProposalState> {
        let state = self.proposals.get_mut(&proposal_id)?.tally();
        match state {
            ProposalState::Accepted => self.stats.accepted += 1,
            ProposalState::Rejected => self.stats.rejected += 1,
            _ => {}
        }
        self.update_active();
        Some(state)
    }

    /// Process expirations
    #[inline]
    pub fn process_expirations(&mut self, now: u64) {
        for proposal in self.proposals.values_mut() {
            let old = proposal.state;
            proposal.check_expiry(now);
            if proposal.state == ProposalState::Expired && old != ProposalState::Expired {
                self.stats.expired += 1;
            }
        }
        self.update_active();
    }

    fn update_active(&mut self) {
        self.stats.active_proposals = self
            .proposals
            .values()
            .filter(|p| matches!(p.state, ProposalState::Open | ProposalState::Draft))
            .count();

        // Average participation
        let completed: Vec<_> = self
            .proposals
            .values()
            .filter(|p| {
                matches!(
                    p.state,
                    ProposalState::Accepted | ProposalState::Rejected | ProposalState::Expired
                )
            })
            .collect();
        if !completed.is_empty() {
            self.stats.avg_participation =
                completed.iter().map(|p| p.participation()).sum::<f64>() / completed.len() as f64;
        }
    }

    /// Get proposal
    #[inline(always)]
    pub fn proposal(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopConsensusStats {
        &self.stats
    }
}

// ============================================================================
// Merged from consensus_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusProto {
    /// Simple majority
    Majority,
    /// Two-phase commit
    TwoPhaseCommit,
    /// Three-phase commit
    ThreePhaseCommit,
    /// Paxos-like
    Paxos,
    /// Raft-like
    Raft,
}

/// Node state in Raft
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftState {
    /// Following a leader
    Follower,
    /// Candidate for election
    Candidate,
    /// Current leader
    Leader,
}

/// Two-phase commit phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoPhaseState {
    /// Prepare phase
    Prepare,
    /// Commit phase
    Commit,
    /// Abort
    Abort,
    /// Completed
    Done,
}

// ============================================================================
// LOG ENTRY
// ============================================================================

/// Consensus log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Term number
    pub term: u64,
    /// Log index
    pub index: u64,
    /// Command data (FNV-1a hash of content)
    pub command_hash: u64,
    /// Proposer PID
    pub proposer: u64,
    /// Committed?
    pub committed: bool,
    /// Timestamp
    pub timestamp_ns: u64,
}

// ============================================================================
// RAFT NODE
// ============================================================================

/// Raft consensus node
#[derive(Debug)]
pub struct RaftNode {
    /// Node/PID
    pub pid: u64,
    /// Current state
    pub state: RaftState,
    /// Current term
    pub current_term: u64,
    /// Voted for in current term
    pub voted_for: Option<u64>,
    /// Log entries
    pub log: Vec<LogEntry>,
    /// Commit index
    pub commit_index: u64,
    /// Last applied
    pub last_applied: u64,
    /// Known peers
    pub peers: Vec<u64>,
    /// Votes received (in candidate state)
    pub votes_received: u64,
    /// Leader ID (if known)
    pub leader_id: Option<u64>,
    /// Heartbeat timeout (ns)
    pub heartbeat_timeout_ns: u64,
    /// Last heartbeat
    pub last_heartbeat_ns: u64,
}

impl RaftNode {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            state: RaftState::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            peers: Vec::new(),
            votes_received: 0,
            leader_id: None,
            heartbeat_timeout_ns: 500_000_000, // 500ms
            last_heartbeat_ns: 0,
        }
    }

    /// Add peer
    #[inline]
    pub fn add_peer(&mut self, pid: u64) {
        if !self.peers.contains(&pid) && pid != self.pid {
            self.peers.push(pid);
        }
    }

    /// Start election
    #[inline]
    pub fn start_election(&mut self, now: u64) {
        self.current_term += 1;
        self.state = RaftState::Candidate;
        self.voted_for = Some(self.pid);
        self.votes_received = 1; // Self-vote
        self.last_heartbeat_ns = now;
    }

    /// Receive vote
    #[inline]
    pub fn receive_vote(&mut self, _from: u64, term: u64, granted: bool) {
        if term != self.current_term || self.state != RaftState::Candidate {
            return;
        }
        if granted {
            self.votes_received += 1;
        }
    }

    /// Check if won election
    pub fn check_election(&mut self) -> bool {
        if self.state != RaftState::Candidate {
            return false;
        }
        let quorum = (self.peers.len() + 1) / 2 + 1;
        if self.votes_received as usize >= quorum {
            self.state = RaftState::Leader;
            self.leader_id = Some(self.pid);
            true
        } else {
            false
        }
    }

    /// Step down to follower
    #[inline]
    pub fn step_down(&mut self, new_term: u64) {
        if new_term > self.current_term {
            self.current_term = new_term;
            self.state = RaftState::Follower;
            self.voted_for = None;
            self.votes_received = 0;
        }
    }

    /// Append log entry (leader only)
    pub fn append_entry(&mut self, command_hash: u64, proposer: u64, now: u64) -> Option<u64> {
        if self.state != RaftState::Leader {
            return None;
        }
        let index = self.log.len() as u64 + 1;
        self.log.push(LogEntry {
            term: self.current_term,
            index,
            command_hash,
            proposer,
            committed: false,
            timestamp_ns: now,
        });
        Some(index)
    }

    /// Commit up to index
    #[inline]
    pub fn commit_up_to(&mut self, index: u64) {
        self.commit_index = index;
        for entry in &mut self.log {
            if entry.index <= index {
                entry.committed = true;
            }
        }
    }

    /// Apply committed entries
    #[inline]
    pub fn apply(&mut self) -> Vec<u64> {
        let mut applied = Vec::new();
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            applied.push(self.last_applied);
        }
        applied
    }

    /// Election timeout?
    #[inline(always)]
    pub fn election_timeout(&self, now: u64) -> bool {
        self.state != RaftState::Leader &&
        now.saturating_sub(self.last_heartbeat_ns) > self.heartbeat_timeout_ns
    }

    /// Heartbeat (from leader)
    #[inline]
    pub fn receive_heartbeat(&mut self, leader: u64, term: u64, now: u64) {
        if term >= self.current_term {
            self.current_term = term;
            self.state = RaftState::Follower;
            self.leader_id = Some(leader);
            self.last_heartbeat_ns = now;
        }
    }
}

// ============================================================================
// TWO-PHASE COMMIT
// ============================================================================

/// Two-phase commit transaction
#[derive(Debug)]
pub struct TwoPhaseTransaction {
    /// Transaction ID
    pub txn_id: u64,
    /// Coordinator PID
    pub coordinator: u64,
    /// Participants
    pub participants: Vec<u64>,
    /// Phase
    pub phase: TwoPhaseState,
    /// Votes (pid -> committed)
    pub votes: LinearMap<bool, 64>,
    /// Start time
    pub start_ns: u64,
    /// Timeout (ns)
    pub timeout_ns: u64,
}

impl TwoPhaseTransaction {
    pub fn new(txn_id: u64, coordinator: u64, participants: Vec<u64>, now: u64) -> Self {
        Self {
            txn_id,
            coordinator,
            participants,
            phase: TwoPhaseState::Prepare,
            votes: LinearMap::new(),
            start_ns: now,
            timeout_ns: 5_000_000_000, // 5s
        }
    }

    /// Record vote from participant
    #[inline(always)]
    pub fn record_vote(&mut self, pid: u64, commit: bool) {
        self.votes.insert(pid, commit);
    }

    /// All voted?
    #[inline(always)]
    pub fn all_voted(&self) -> bool {
        self.participants.iter().all(|p| self.votes.contains_key(p))
    }

    /// Can commit? (all voted yes)
    #[inline(always)]
    pub fn can_commit(&self) -> bool {
        self.all_voted() && self.votes.values().all(|&v| v)
    }

    /// Decide
    #[inline]
    pub fn decide(&mut self) -> TwoPhaseState {
        if self.can_commit() {
            self.phase = TwoPhaseState::Commit;
        } else {
            self.phase = TwoPhaseState::Abort;
        }
        self.phase
    }

    /// Is timed out
    #[inline(always)]
    pub fn is_timed_out(&self, now: u64) -> bool {
        now.saturating_sub(self.start_ns) > self.timeout_ns
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Consensus V2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopConsensusV2Stats {
    /// Raft nodes
    pub raft_nodes: usize,
    /// Active leaders
    pub leaders: usize,
    /// Total elections
    pub total_elections: u64,
    /// Active 2PC transactions
    pub active_2pc: usize,
    /// Committed 2PC
    pub committed_2pc: u64,
    /// Aborted 2PC
    pub aborted_2pc: u64,
}

/// Coop consensus V2 engine
pub struct CoopConsensusV2 {
    /// Raft nodes
    raft_nodes: BTreeMap<u64, RaftNode>,
    /// 2PC transactions
    transactions: BTreeMap<u64, TwoPhaseTransaction>,
    /// Stats
    stats: CoopConsensusV2Stats,
    /// Next txn ID
    next_txn_id: u64,
}

impl CoopConsensusV2 {
    pub fn new() -> Self {
        Self {
            raft_nodes: BTreeMap::new(),
            transactions: BTreeMap::new(),
            stats: CoopConsensusV2Stats::default(),
            next_txn_id: 1,
        }
    }

    /// Register raft node
    #[inline(always)]
    pub fn register_node(&mut self, pid: u64) -> &mut RaftNode {
        self.raft_nodes.entry(pid).or_insert_with(|| RaftNode::new(pid))
    }

    /// Trigger election for node
    #[inline]
    pub fn trigger_election(&mut self, pid: u64, now: u64) {
        if let Some(node) = self.raft_nodes.get_mut(&pid) {
            node.start_election(now);
            self.stats.total_elections += 1;
        }
    }

    /// Begin 2PC transaction
    #[inline]
    pub fn begin_2pc(&mut self, coordinator: u64, participants: Vec<u64>, now: u64) -> u64 {
        let txn_id = self.next_txn_id;
        self.next_txn_id += 1;
        self.transactions.insert(txn_id, TwoPhaseTransaction::new(txn_id, coordinator, participants, now));
        self.update_stats();
        txn_id
    }

    /// Vote on 2PC
    #[inline]
    pub fn vote_2pc(&mut self, txn_id: u64, pid: u64, commit: bool) {
        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.record_vote(pid, commit);
        }
    }

    /// Try decide 2PC
    pub fn try_decide_2pc(&mut self, txn_id: u64) -> Option<TwoPhaseState> {
        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            if txn.all_voted() {
                let decision = txn.decide();
                match decision {
                    TwoPhaseState::Commit => self.stats.committed_2pc += 1,
                    TwoPhaseState::Abort => self.stats.aborted_2pc += 1,
                    _ => {}
                }
                return Some(decision);
            }
        }
        None
    }

    /// Remove node
    #[inline(always)]
    pub fn remove_node(&mut self, pid: u64) {
        self.raft_nodes.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.raft_nodes = self.raft_nodes.len();
        self.stats.leaders = self.raft_nodes.values()
            .filter(|n| n.state == RaftState::Leader)
            .count();
        self.stats.active_2pc = self.transactions.values()
            .filter(|t| matches!(t.phase, TwoPhaseState::Prepare))
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopConsensusV2Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from consensus_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusRole {
    /// Follower — replicates log
    Follower,
    /// Candidate — requesting votes
    Candidate,
    /// Leader — drives consensus
    Leader,
    /// Pre-candidate (pre-vote phase)
    PreCandidate,
    /// Learner (non-voting)
    Learner,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    /// FNV-1a hash of command data
    pub command_hash: u64,
    pub data_len: u32,
    pub timestamp_ns: u64,
    pub committed: bool,
}

/// Vote request
#[derive(Debug, Clone)]
pub struct VoteRequest {
    pub candidate_id: u64,
    pub term: u64,
    pub last_log_index: u64,
    pub last_log_term: u64,
    pub is_pre_vote: bool,
}

/// Vote response
#[derive(Debug, Clone)]
pub struct VoteResponse {
    pub voter_id: u64,
    pub term: u64,
    pub granted: bool,
}

/// Append entries request
#[derive(Debug, Clone)]
pub struct AppendRequest {
    pub leader_id: u64,
    pub term: u64,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

/// Per-node state
#[derive(Debug)]
pub struct ConsensusNode {
    pub node_id: u64,
    pub role: ConsensusRole,
    pub current_term: u64,
    pub voted_for: Option<u64>,
    /// Log entries
    log: Vec<LogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    /// Leader-specific: next index for each follower
    next_index: LinearMap<u64, 64>,
    /// Leader-specific: match index for each follower
    match_index: LinearMap<u64, 64>,
    /// Leader ID (as known by follower)
    pub leader_id: Option<u64>,
    /// Election timeout
    pub election_timeout_ns: u64,
    pub last_heartbeat_ns: u64,
    /// Vote count in current election
    pub votes_received: u32,
    pub cluster_size: u32,
}

impl ConsensusNode {
    pub fn new(node_id: u64, cluster_size: u32) -> Self {
        Self {
            node_id,
            role: ConsensusRole::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            next_index: LinearMap::new(),
            match_index: LinearMap::new(),
            leader_id: None,
            election_timeout_ns: 150_000_000, // 150ms
            last_heartbeat_ns: 0,
            votes_received: 0,
            cluster_size,
        }
    }

    #[inline(always)]
    pub fn last_log_index(&self) -> u64 {
        self.log.last().map(|e| e.index).unwrap_or(0)
    }

    #[inline(always)]
    pub fn last_log_term(&self) -> u64 {
        self.log.last().map(|e| e.term).unwrap_or(0)
    }

    /// Majority threshold
    #[inline(always)]
    pub fn majority(&self) -> u32 {
        self.cluster_size / 2 + 1
    }

    /// Start election
    pub fn start_election(&mut self, now_ns: u64) -> VoteRequest {
        self.current_term += 1;
        self.role = ConsensusRole::Candidate;
        self.voted_for = Some(self.node_id);
        self.votes_received = 1; // Vote for self
        self.last_heartbeat_ns = now_ns;

        VoteRequest {
            candidate_id: self.node_id,
            term: self.current_term,
            last_log_index: self.last_log_index(),
            last_log_term: self.last_log_term(),
            is_pre_vote: false,
        }
    }

    /// Handle vote request
    pub fn handle_vote_request(&mut self, req: &VoteRequest) -> VoteResponse {
        if req.term < self.current_term {
            return VoteResponse {
                voter_id: self.node_id,
                term: self.current_term,
                granted: false,
            };
        }

        if req.term > self.current_term {
            self.current_term = req.term;
            self.role = ConsensusRole::Follower;
            self.voted_for = None;
        }

        let log_ok = req.last_log_term > self.last_log_term()
            || (req.last_log_term == self.last_log_term()
                && req.last_log_index >= self.last_log_index());

        let can_vote = self.voted_for.is_none() || self.voted_for == Some(req.candidate_id);

        let granted = log_ok && can_vote;
        if granted {
            self.voted_for = Some(req.candidate_id);
        }

        VoteResponse {
            voter_id: self.node_id,
            term: self.current_term,
            granted,
        }
    }

    /// Handle vote response (as candidate)
    pub fn handle_vote_response(&mut self, resp: &VoteResponse) -> bool {
        if resp.term > self.current_term {
            self.current_term = resp.term;
            self.role = ConsensusRole::Follower;
            return false;
        }

        if self.role != ConsensusRole::Candidate {
            return false;
        }

        if resp.granted {
            self.votes_received += 1;
        }

        // Check if we won
        if self.votes_received >= self.majority() {
            self.become_leader();
            return true;
        }
        false
    }

    fn become_leader(&mut self) {
        self.role = ConsensusRole::Leader;
        self.leader_id = Some(self.node_id);
        // Initialize next_index for all followers
        let last = self.last_log_index() + 1;
        self.next_index.clear();
        self.match_index.clear();
        // Will be populated as followers are discovered
    }

    /// Append entry to leader's log
    pub fn leader_append(&mut self, command_hash: u64, data_len: u32, now_ns: u64) -> Option<u64> {
        if self.role != ConsensusRole::Leader {
            return None;
        }
        let index = self.last_log_index() + 1;
        self.log.push(LogEntry {
            index,
            term: self.current_term,
            command_hash,
            data_len,
            timestamp_ns: now_ns,
            committed: false,
        });
        Some(index)
    }

    /// Handle append request (as follower)
    pub fn handle_append(&mut self, req: &AppendRequest, now_ns: u64) -> bool {
        if req.term < self.current_term {
            return false;
        }

        self.current_term = req.term;
        self.role = ConsensusRole::Follower;
        self.leader_id = Some(req.leader_id);
        self.last_heartbeat_ns = now_ns;

        // Check log consistency
        if req.prev_log_index > 0 {
            let matches = self
                .log
                .iter()
                .any(|e| e.index == req.prev_log_index && e.term == req.prev_log_term);
            if !matches && !self.log.is_empty() {
                return false;
            }
        }

        // Append new entries
        for entry in &req.entries {
            // Remove conflicting entries
            self.log
                .retain(|e| e.index < entry.index || e.term == entry.term);
            self.log.push(entry.clone());
        }

        // Update commit index
        if req.leader_commit > self.commit_index {
            self.commit_index = req.leader_commit.min(self.last_log_index());
            self.apply_committed();
        }

        true
    }

    fn apply_committed(&mut self) {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            if let Some(entry) = self.log.iter_mut().find(|e| e.index == self.last_applied) {
                entry.committed = true;
            }
        }
    }

    /// Check if election timeout elapsed
    #[inline(always)]
    pub fn election_timeout_elapsed(&self, now_ns: u64) -> bool {
        self.role != ConsensusRole::Leader
            && now_ns.saturating_sub(self.last_heartbeat_ns) > self.election_timeout_ns
    }

    /// Log length
    #[inline(always)]
    pub fn log_len(&self) -> usize {
        self.log.len()
    }

    /// Committed entries
    #[inline(always)]
    pub fn committed_count(&self) -> u64 {
        self.log.iter().filter(|e| e.committed).count() as u64
    }
}

/// Consensus cluster stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopConsensusV3Stats {
    pub node_count: usize,
    pub leader_id: Option<u64>,
    pub current_term: u64,
    pub total_log_entries: u64,
    pub total_committed: u64,
    pub elections_held: u64,
}

/// Coop Consensus V3
pub struct CoopConsensusV3 {
    nodes: BTreeMap<u64, ConsensusNode>,
    stats: CoopConsensusV3Stats,
    elections_held: u64,
}

impl CoopConsensusV3 {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            stats: CoopConsensusV3Stats::default(),
            elections_held: 0,
        }
    }

    #[inline]
    pub fn add_node(&mut self, node_id: u64) {
        let cluster_size = (self.nodes.len() + 1) as u32;
        self.nodes
            .insert(node_id, ConsensusNode::new(node_id, cluster_size));
        // Update cluster size for all nodes
        for node in self.nodes.values_mut() {
            node.cluster_size = self.nodes.len() as u32;
        }
    }

    pub fn tick(&mut self, now_ns: u64) {
        let timed_out: Vec<u64> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.election_timeout_elapsed(now_ns))
            .map(|(&id, _)| id)
            .collect();

        for node_id in timed_out {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                let _req = node.start_election(now_ns);
                self.elections_held += 1;
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.node_count = self.nodes.len();
        self.stats.leader_id = self
            .nodes
            .iter()
            .find(|(_, n)| n.role == ConsensusRole::Leader)
            .map(|(&id, _)| id);
        self.stats.current_term = self
            .nodes
            .values()
            .map(|n| n.current_term)
            .max()
            .unwrap_or(0);
        self.stats.total_log_entries = self.nodes.values().map(|n| n.log_len() as u64).sum();
        self.stats.total_committed = self.nodes.values().map(|n| n.committed_count()).sum();
        self.stats.elections_held = self.elections_held;
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopConsensusV3Stats {
        &self.stats
    }
}
