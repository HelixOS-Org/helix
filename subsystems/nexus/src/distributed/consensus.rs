//! # Consensus Protocol
//!
//! Year 3 EVOLUTION - Q4 - Distributed consensus for improvement adoption

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{ImprovementId, NodeId, Term};

// ============================================================================
// CONSENSUS TYPES
// ============================================================================

/// Proposal number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ProposalNumber(pub u64);

/// Log index
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogIndex(pub u64);

static PROPOSAL_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ProposalNumber {
    pub fn generate() -> Self {
        Self(PROPOSAL_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Consensus role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusRole {
    /// Follower
    Follower,
    /// Candidate
    Candidate,
    /// Leader
    Leader,
}

/// Consensus state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusState {
    /// Initial
    Initial,
    /// Waiting for leader
    WaitingForLeader,
    /// Following
    Following,
    /// Electing
    Electing,
    /// Leading
    Leading,
    /// Recovering
    Recovering,
}

// ============================================================================
// LOG ENTRY
// ============================================================================

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Index
    pub index: LogIndex,
    /// Term
    pub term: Term,
    /// Command
    pub command: Command,
    /// Committed
    pub committed: bool,
}

/// Command
#[derive(Debug, Clone)]
pub enum Command {
    /// No-op (for leader election)
    NoOp,
    /// Add improvement
    AddImprovement(ImprovementId),
    /// Approve improvement
    ApproveImprovement(ImprovementId),
    /// Reject improvement
    RejectImprovement(ImprovementId),
    /// Deploy improvement
    DeployImprovement(ImprovementId),
    /// Revert improvement
    RevertImprovement(ImprovementId),
    /// Configuration change
    ConfigChange(ConfigCommand),
    /// Custom
    Custom(Vec<u8>),
}

/// Configuration command
#[derive(Debug, Clone)]
pub enum ConfigCommand {
    /// Add member
    AddMember(NodeId),
    /// Remove member
    RemoveMember(NodeId),
    /// Change voting threshold
    ChangeThreshold(f64),
}

// ============================================================================
// RAFT STATE
// ============================================================================

/// Raft state
pub struct RaftState {
    /// Current term
    current_term: Term,
    /// Voted for in current term
    voted_for: Option<NodeId>,
    /// Log entries
    log: Vec<LogEntry>,
    /// Commit index
    commit_index: LogIndex,
    /// Last applied
    last_applied: LogIndex,
    /// Role
    role: ConsensusRole,
    /// State
    state: ConsensusState,
    /// Leader ID
    leader_id: Option<NodeId>,
    /// Local node ID
    node_id: NodeId,
    /// Cluster members
    members: Vec<NodeId>,
    /// Next index for each follower
    next_index: BTreeMap<NodeId, LogIndex>,
    /// Match index for each follower
    match_index: BTreeMap<NodeId, LogIndex>,
    /// Votes received
    votes_received: Vec<NodeId>,
    /// Election timeout (ms)
    election_timeout: u64,
    /// Last heartbeat
    last_heartbeat: u64,
}

impl RaftState {
    /// Create new Raft state
    pub fn new(node_id: NodeId) -> Self {
        Self {
            current_term: Term(0),
            voted_for: None,
            log: Vec::new(),
            commit_index: LogIndex(0),
            last_applied: LogIndex(0),
            role: ConsensusRole::Follower,
            state: ConsensusState::Initial,
            leader_id: None,
            node_id,
            members: vec![node_id],
            next_index: BTreeMap::new(),
            match_index: BTreeMap::new(),
            votes_received: Vec::new(),
            election_timeout: 1000,
            last_heartbeat: 0,
        }
    }

    /// Start election
    pub fn start_election(&mut self) -> Vec<RequestVote> {
        self.current_term = Term(self.current_term.0 + 1);
        self.role = ConsensusRole::Candidate;
        self.state = ConsensusState::Electing;
        self.voted_for = Some(self.node_id);
        self.votes_received = vec![self.node_id];

        let last_log_index = self.last_log_index();
        let last_log_term = self.last_log_term();

        self.members
            .iter()
            .filter(|&m| *m != self.node_id)
            .map(|&member| RequestVote {
                term: self.current_term,
                candidate_id: self.node_id,
                last_log_index,
                last_log_term,
                destination: member,
            })
            .collect()
    }

    /// Handle vote response
    pub fn handle_vote_response(&mut self, from: NodeId, granted: bool) -> bool {
        if granted && !self.votes_received.contains(&from) {
            self.votes_received.push(from);
        }

        // Check if we have majority
        let majority = (self.members.len() / 2) + 1;
        if self.votes_received.len() >= majority && self.role == ConsensusRole::Candidate {
            self.become_leader();
            return true;
        }

        false
    }

    /// Become leader
    fn become_leader(&mut self) {
        self.role = ConsensusRole::Leader;
        self.state = ConsensusState::Leading;
        self.leader_id = Some(self.node_id);

        // Initialize next/match index
        let last_index = self.last_log_index();
        for member in &self.members {
            if *member != self.node_id {
                self.next_index.insert(*member, LogIndex(last_index.0 + 1));
                self.match_index.insert(*member, LogIndex(0));
            }
        }

        // Append no-op
        self.append_entry(Command::NoOp);
    }

    /// Append entry to log
    pub fn append_entry(&mut self, command: Command) -> LogIndex {
        let index = LogIndex(self.log.len() as u64 + 1);
        self.log.push(LogEntry {
            index,
            term: self.current_term,
            command,
            committed: false,
        });
        index
    }

    /// Handle append entries
    pub fn handle_append_entries(
        &mut self,
        term: Term,
        leader_id: NodeId,
        prev_log_index: LogIndex,
        prev_log_term: Term,
        entries: Vec<LogEntry>,
        leader_commit: LogIndex,
    ) -> AppendEntriesResponse {
        // Reply false if term < currentTerm
        if term < self.current_term {
            return AppendEntriesResponse {
                term: self.current_term,
                success: false,
                match_index: LogIndex(0),
            };
        }

        // Update term
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
            self.role = ConsensusRole::Follower;
            self.state = ConsensusState::Following;
        }

        self.leader_id = Some(leader_id);
        self.last_heartbeat = 0; // Would be current time

        // Check log consistency
        if prev_log_index.0 > 0 {
            if let Some(entry) = self.log.get((prev_log_index.0 - 1) as usize) {
                if entry.term != prev_log_term {
                    return AppendEntriesResponse {
                        term: self.current_term,
                        success: false,
                        match_index: LogIndex(0),
                    };
                }
            } else {
                return AppendEntriesResponse {
                    term: self.current_term,
                    success: false,
                    match_index: LogIndex(0),
                };
            }
        }

        // Append entries
        for entry in entries {
            let idx = (entry.index.0 - 1) as usize;
            if idx < self.log.len() {
                if self.log[idx].term != entry.term {
                    self.log.truncate(idx);
                    self.log.push(entry);
                }
            } else {
                self.log.push(entry);
            }
        }

        // Update commit index
        if leader_commit > self.commit_index {
            self.commit_index = LogIndex(core::cmp::min(leader_commit.0, self.last_log_index().0));
        }

        AppendEntriesResponse {
            term: self.current_term,
            success: true,
            match_index: self.last_log_index(),
        }
    }

    /// Get last log index
    pub fn last_log_index(&self) -> LogIndex {
        self.log.last().map(|e| e.index).unwrap_or(LogIndex(0))
    }

    /// Get last log term
    pub fn last_log_term(&self) -> Term {
        self.log.last().map(|e| e.term).unwrap_or(Term(0))
    }

    /// Get current term
    pub fn current_term(&self) -> Term {
        self.current_term
    }

    /// Is leader
    pub fn is_leader(&self) -> bool {
        self.role == ConsensusRole::Leader
    }

    /// Get leader
    pub fn leader(&self) -> Option<NodeId> {
        self.leader_id
    }

    /// Add member
    pub fn add_member(&mut self, node_id: NodeId) {
        if !self.members.contains(&node_id) {
            self.members.push(node_id);
        }
    }

    /// Remove member
    pub fn remove_member(&mut self, node_id: NodeId) {
        self.members.retain(|&m| m != node_id);
    }

    /// Get committed entries
    pub fn committed_entries(&self) -> Vec<&LogEntry> {
        self.log
            .iter()
            .filter(|e| e.index.0 <= self.commit_index.0)
            .collect()
    }
}

// ============================================================================
// RPC MESSAGES
// ============================================================================

/// RequestVote RPC
#[derive(Debug, Clone)]
pub struct RequestVote {
    /// Term
    pub term: Term,
    /// Candidate ID
    pub candidate_id: NodeId,
    /// Last log index
    pub last_log_index: LogIndex,
    /// Last log term
    pub last_log_term: Term,
    /// Destination
    pub destination: NodeId,
}

/// RequestVote response
#[derive(Debug, Clone)]
pub struct RequestVoteResponse {
    /// Term
    pub term: Term,
    /// Vote granted
    pub vote_granted: bool,
}

/// AppendEntries RPC
#[derive(Debug, Clone)]
pub struct AppendEntries {
    /// Term
    pub term: Term,
    /// Leader ID
    pub leader_id: NodeId,
    /// Previous log index
    pub prev_log_index: LogIndex,
    /// Previous log term
    pub prev_log_term: Term,
    /// Entries
    pub entries: Vec<LogEntry>,
    /// Leader commit
    pub leader_commit: LogIndex,
}

/// AppendEntries response
#[derive(Debug, Clone)]
pub struct AppendEntriesResponse {
    /// Term
    pub term: Term,
    /// Success
    pub success: bool,
    /// Match index
    pub match_index: LogIndex,
}

// ============================================================================
// PAXOS STATE (ALTERNATIVE)
// ============================================================================

/// Paxos state
pub struct PaxosState {
    /// Proposer state
    proposer: ProposerState,
    /// Acceptor state
    acceptor: AcceptorState,
    /// Learner state
    learner: LearnerState,
    /// Node ID
    node_id: NodeId,
    /// Members
    members: Vec<NodeId>,
}

/// Proposer state
#[derive(Debug, Clone, Default)]
pub struct ProposerState {
    /// Current proposal number
    proposal_number: ProposalNumber,
    /// Highest proposal seen
    highest_seen: ProposalNumber,
    /// Proposed value
    proposed_value: Option<Vec<u8>>,
    /// Promises received
    promises: Vec<(NodeId, ProposalNumber, Option<Vec<u8>>)>,
    /// Accepts received
    accepts: Vec<NodeId>,
}

/// Acceptor state
#[derive(Debug, Clone, Default)]
pub struct AcceptorState {
    /// Highest promised
    promised: ProposalNumber,
    /// Accepted proposal
    accepted_proposal: Option<ProposalNumber>,
    /// Accepted value
    accepted_value: Option<Vec<u8>>,
}

/// Learner state
#[derive(Debug, Clone, Default)]
pub struct LearnerState {
    /// Learned values
    learned: BTreeMap<ProposalNumber, Vec<u8>>,
}

impl PaxosState {
    /// Create new Paxos state
    pub fn new(node_id: NodeId) -> Self {
        Self {
            proposer: ProposerState::default(),
            acceptor: AcceptorState::default(),
            learner: LearnerState::default(),
            node_id,
            members: vec![node_id],
        }
    }

    /// Prepare (Phase 1a)
    pub fn prepare(&mut self, value: Vec<u8>) -> Prepare {
        self.proposer.proposal_number = ProposalNumber::generate();
        self.proposer.proposed_value = Some(value);
        self.proposer.promises.clear();
        self.proposer.accepts.clear();

        Prepare {
            proposal_number: self.proposer.proposal_number,
            proposer: self.node_id,
        }
    }

    /// Handle prepare (Promise - Phase 1b)
    pub fn handle_prepare(&mut self, prepare: &Prepare) -> Promise {
        if prepare.proposal_number > self.acceptor.promised {
            self.acceptor.promised = prepare.proposal_number;
            Promise {
                proposal_number: prepare.proposal_number,
                ok: true,
                accepted_proposal: self.acceptor.accepted_proposal,
                accepted_value: self.acceptor.accepted_value.clone(),
            }
        } else {
            Promise {
                proposal_number: prepare.proposal_number,
                ok: false,
                accepted_proposal: None,
                accepted_value: None,
            }
        }
    }

    /// Handle promise
    pub fn handle_promise(&mut self, from: NodeId, promise: Promise) -> Option<Accept> {
        if promise.ok {
            self.proposer.promises.push((
                from,
                promise.accepted_proposal.unwrap_or(ProposalNumber(0)),
                promise.accepted_value,
            ));
        }

        let majority = (self.members.len() / 2) + 1;
        if self.proposer.promises.len() >= majority {
            // Use highest accepted value if any
            let value = self
                .proposer
                .promises
                .iter()
                .filter(|(_, n, v)| v.is_some() && n.0 > 0)
                .max_by_key(|(_, n, _)| n.0)
                .and_then(|(_, _, v)| v.clone())
                .or_else(|| self.proposer.proposed_value.clone())
                .unwrap_or_default();

            Some(Accept {
                proposal_number: self.proposer.proposal_number,
                value,
            })
        } else {
            None
        }
    }

    /// Handle accept (Accepted - Phase 2b)
    pub fn handle_accept(&mut self, accept: &Accept) -> Accepted {
        if accept.proposal_number >= self.acceptor.promised {
            self.acceptor.accepted_proposal = Some(accept.proposal_number);
            self.acceptor.accepted_value = Some(accept.value.clone());
            Accepted {
                proposal_number: accept.proposal_number,
                ok: true,
            }
        } else {
            Accepted {
                proposal_number: accept.proposal_number,
                ok: false,
            }
        }
    }

    /// Handle accepted
    pub fn handle_accepted(&mut self, from: NodeId, accepted: Accepted) -> Option<Vec<u8>> {
        if accepted.ok {
            self.proposer.accepts.push(from);
        }

        let majority = (self.members.len() / 2) + 1;
        if self.proposer.accepts.len() >= majority {
            if let Some(value) = &self.proposer.proposed_value {
                self.learner
                    .learned
                    .insert(self.proposer.proposal_number, value.clone());
                return Some(value.clone());
            }
        }

        None
    }
}

/// Prepare message
#[derive(Debug, Clone)]
pub struct Prepare {
    /// Proposal number
    pub proposal_number: ProposalNumber,
    /// Proposer
    pub proposer: NodeId,
}

/// Promise message
#[derive(Debug, Clone)]
pub struct Promise {
    /// Proposal number
    pub proposal_number: ProposalNumber,
    /// OK
    pub ok: bool,
    /// Previously accepted proposal
    pub accepted_proposal: Option<ProposalNumber>,
    /// Previously accepted value
    pub accepted_value: Option<Vec<u8>>,
}

/// Accept message
#[derive(Debug, Clone)]
pub struct Accept {
    /// Proposal number
    pub proposal_number: ProposalNumber,
    /// Value
    pub value: Vec<u8>,
}

/// Accepted message
#[derive(Debug, Clone)]
pub struct Accepted {
    /// Proposal number
    pub proposal_number: ProposalNumber,
    /// OK
    pub ok: bool,
}

// ============================================================================
// CONSENSUS ENGINE
// ============================================================================

/// Consensus engine
pub struct ConsensusEngine {
    /// Raft state
    raft: RaftState,
    /// Paxos state (alternative)
    paxos: PaxosState,
    /// Algorithm in use
    algorithm: ConsensusAlgorithm,
    /// Configuration
    config: ConsensusConfig,
    /// Running
    running: AtomicBool,
    /// Statistics
    stats: ConsensusStats,
}

/// Consensus algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusAlgorithm {
    /// Raft
    Raft,
    /// Paxos
    Paxos,
    /// Multi-Paxos
    MultiPaxos,
    /// PBFT
    PBFT,
}

/// Consensus configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Algorithm
    pub algorithm: ConsensusAlgorithm,
    /// Election timeout (ms)
    pub election_timeout: u64,
    /// Heartbeat interval (ms)
    pub heartbeat_interval: u64,
    /// Log compaction threshold
    pub compaction_threshold: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            algorithm: ConsensusAlgorithm::Raft,
            election_timeout: 1000,
            heartbeat_interval: 100,
            compaction_threshold: 1000,
        }
    }
}

/// Consensus statistics
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    /// Elections held
    pub elections: u64,
    /// Proposals made
    pub proposals: u64,
    /// Proposals committed
    pub committed: u64,
    /// Leader changes
    pub leader_changes: u64,
}

impl ConsensusEngine {
    /// Create new consensus engine
    pub fn new(node_id: NodeId, config: ConsensusConfig) -> Self {
        Self {
            raft: RaftState::new(node_id),
            paxos: PaxosState::new(node_id),
            algorithm: config.algorithm,
            config,
            running: AtomicBool::new(false),
            stats: ConsensusStats::default(),
        }
    }

    /// Start the engine
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the engine
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Propose a value
    pub fn propose(&mut self, command: Command) -> Result<LogIndex, ConsensusError> {
        match self.algorithm {
            ConsensusAlgorithm::Raft => {
                if !self.raft.is_leader() {
                    return Err(ConsensusError::NotLeader(self.raft.leader()));
                }
                let index = self.raft.append_entry(command);
                self.stats.proposals += 1;
                Ok(index)
            },
            ConsensusAlgorithm::Paxos => {
                // Use Paxos
                self.stats.proposals += 1;
                Ok(LogIndex(0))
            },
            _ => Err(ConsensusError::UnsupportedAlgorithm),
        }
    }

    /// Is leader
    pub fn is_leader(&self) -> bool {
        match self.algorithm {
            ConsensusAlgorithm::Raft => self.raft.is_leader(),
            _ => false,
        }
    }

    /// Get leader
    pub fn leader(&self) -> Option<NodeId> {
        match self.algorithm {
            ConsensusAlgorithm::Raft => self.raft.leader(),
            _ => None,
        }
    }

    /// Get Raft state
    pub fn raft(&self) -> &RaftState {
        &self.raft
    }

    /// Get Raft state mutable
    pub fn raft_mut(&mut self) -> &mut RaftState {
        &mut self.raft
    }

    /// Get statistics
    pub fn stats(&self) -> &ConsensusStats {
        &self.stats
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new(NodeId(0), ConsensusConfig::default())
    }
}

/// Consensus error
#[derive(Debug)]
pub enum ConsensusError {
    /// Not the leader
    NotLeader(Option<NodeId>),
    /// Unsupported algorithm
    UnsupportedAlgorithm,
    /// Log mismatch
    LogMismatch,
    /// Term expired
    TermExpired,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_election() {
        let mut raft = RaftState::new(NodeId(1));
        raft.add_member(NodeId(2));
        raft.add_member(NodeId(3));

        let requests = raft.start_election();
        assert_eq!(requests.len(), 2);
        assert_eq!(raft.current_term(), Term(1));
    }

    #[test]
    fn test_raft_become_leader() {
        let mut raft = RaftState::new(NodeId(1));
        raft.add_member(NodeId(2));
        raft.add_member(NodeId(3));

        raft.start_election();

        // Get majority
        let became_leader = raft.handle_vote_response(NodeId(2), true);
        assert!(became_leader);
        assert!(raft.is_leader());
    }

    #[test]
    fn test_paxos_prepare() {
        let mut paxos = PaxosState::new(NodeId(1));

        let prepare = paxos.prepare(vec![1, 2, 3]);
        assert!(prepare.proposal_number.0 > 0);
    }

    #[test]
    fn test_consensus_engine() {
        let node_id = NodeId(1);
        let engine = ConsensusEngine::new(node_id, ConsensusConfig::default());

        assert!(!engine.is_leader());
    }
}
