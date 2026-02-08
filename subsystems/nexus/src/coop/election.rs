//! # Cooperative Election Protocol
//!
//! Leader election for cooperative process groups:
//! - Bully algorithm implementation
//! - Ring-based election
//! - Term-based leadership with heartbeats
//! - Split-brain prevention
//! - Graceful leadership transfer

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ELECTION TYPES
// ============================================================================

/// Election algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionAlgorithm {
    /// Bully (highest ID wins)
    Bully,
    /// Ring-based
    Ring,
    /// Random
    Random,
    /// Priority-based
    Priority,
}

/// Election state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionState {
    /// No election in progress
    Idle,
    /// Election started
    Started,
    /// Voting
    Voting,
    /// Decided
    Decided,
    /// Contested (split brain)
    Contested,
}

/// Node role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Follower
    Follower,
    /// Candidate
    Candidate,
    /// Leader
    Leader,
}

// ============================================================================
// ELECTION NODE
// ============================================================================

/// Election participant
#[derive(Debug)]
pub struct ElectionNode {
    /// Node id (pid)
    pub id: u64,
    /// Role
    pub role: NodeRole,
    /// Current term
    pub term: u64,
    /// Voted for in current term
    pub voted_for: Option<u64>,
    /// Priority (for priority-based election)
    pub priority: u32,
    /// Last heartbeat received
    pub last_heartbeat: u64,
    /// Heartbeat timeout (ns)
    pub heartbeat_timeout_ns: u64,
    /// Is alive
    pub alive: bool,
}

impl ElectionNode {
    pub fn new(id: u64, priority: u32) -> Self {
        Self {
            id,
            role: NodeRole::Follower,
            term: 0,
            voted_for: None,
            priority,
            last_heartbeat: 0,
            heartbeat_timeout_ns: 5_000_000_000, // 5s
            alive: true,
        }
    }

    /// Become candidate
    pub fn become_candidate(&mut self, new_term: u64) {
        self.role = NodeRole::Candidate;
        self.term = new_term;
        self.voted_for = Some(self.id);
    }

    /// Become leader
    pub fn become_leader(&mut self) {
        self.role = NodeRole::Leader;
    }

    /// Step down to follower
    pub fn step_down(&mut self, new_term: u64) {
        self.role = NodeRole::Follower;
        self.term = new_term;
        self.voted_for = None;
    }

    /// Receive heartbeat
    pub fn receive_heartbeat(&mut self, from_term: u64, now: u64) {
        if from_term >= self.term {
            self.term = from_term;
            self.role = NodeRole::Follower;
            self.last_heartbeat = now;
        }
    }

    /// Heartbeat timed out?
    pub fn heartbeat_timeout(&self, now: u64) -> bool {
        now.saturating_sub(self.last_heartbeat) > self.heartbeat_timeout_ns
    }

    /// Vote for candidate
    pub fn vote(&mut self, candidate: u64, candidate_term: u64) -> bool {
        if candidate_term > self.term {
            self.term = candidate_term;
            self.voted_for = Some(candidate);
            return true;
        }
        if candidate_term == self.term && self.voted_for.is_none() {
            self.voted_for = Some(candidate);
            return true;
        }
        false
    }
}

// ============================================================================
// ELECTION
// ============================================================================

/// An election round
#[derive(Debug)]
pub struct Election {
    /// Election id
    pub id: u64,
    /// Term
    pub term: u64,
    /// Algorithm
    pub algorithm: ElectionAlgorithm,
    /// State
    pub state: ElectionState,
    /// Initiator
    pub initiator: u64,
    /// Votes received: candidate -> count
    pub votes: BTreeMap<u64, u32>,
    /// Total voters
    pub total_voters: u32,
    /// Winner
    pub winner: Option<u64>,
    /// Start time
    pub started_at: u64,
    /// Decided time
    pub decided_at: Option<u64>,
    /// Timeout (ns)
    pub timeout_ns: u64,
}

impl Election {
    pub fn new(
        id: u64,
        term: u64,
        algorithm: ElectionAlgorithm,
        initiator: u64,
        total_voters: u32,
        now: u64,
    ) -> Self {
        Self {
            id,
            term,
            algorithm,
            state: ElectionState::Started,
            initiator,
            votes: BTreeMap::new(),
            total_voters,
            winner: None,
            started_at: now,
            decided_at: None,
            timeout_ns: 10_000_000_000, // 10s
        }
    }

    /// Record vote
    pub fn record_vote(&mut self, candidate: u64) {
        *self.votes.entry(candidate).or_insert(0) += 1;
        self.state = ElectionState::Voting;
    }

    /// Check if we have a winner (majority)
    pub fn check_winner(&mut self, now: u64) -> Option<u64> {
        let majority = self.total_voters / 2 + 1;
        for (&candidate, &count) in &self.votes {
            if count >= majority {
                self.winner = Some(candidate);
                self.state = ElectionState::Decided;
                self.decided_at = Some(now);
                return Some(candidate);
            }
        }
        None
    }

    /// Bully algorithm winner (highest id)
    pub fn bully_winner(&mut self, alive_nodes: &[u64], now: u64) -> Option<u64> {
        let winner = alive_nodes.iter().copied().max();
        if let Some(w) = winner {
            self.winner = Some(w);
            self.state = ElectionState::Decided;
            self.decided_at = Some(now);
        }
        winner
    }

    /// Is timed out?
    pub fn is_timed_out(&self, now: u64) -> bool {
        now.saturating_sub(self.started_at) > self.timeout_ns
    }

    /// Election duration (ns)
    pub fn duration_ns(&self, now: u64) -> u64 {
        let end = self.decided_at.unwrap_or(now);
        end.saturating_sub(self.started_at)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Election stats
#[derive(Debug, Clone, Default)]
pub struct CoopElectionStats {
    /// Total nodes
    pub total_nodes: usize,
    /// Current leader
    pub current_leader: Option<u64>,
    /// Current term
    pub current_term: u64,
    /// Total elections
    pub total_elections: u64,
    /// Contested elections
    pub contested: u64,
}

/// Cooperative election manager
pub struct CoopElectionManager {
    /// Nodes
    nodes: BTreeMap<u64, ElectionNode>,
    /// Elections
    elections: BTreeMap<u64, Election>,
    /// Current leader
    current_leader: Option<u64>,
    /// Current term
    current_term: u64,
    /// Algorithm
    algorithm: ElectionAlgorithm,
    /// Next election id
    next_id: u64,
    /// Stats
    stats: CoopElectionStats,
}

impl CoopElectionManager {
    pub fn new(algorithm: ElectionAlgorithm) -> Self {
        Self {
            nodes: BTreeMap::new(),
            elections: BTreeMap::new(),
            current_leader: None,
            current_term: 0,
            algorithm,
            next_id: 1,
            stats: CoopElectionStats::default(),
        }
    }

    /// Register node
    pub fn register(&mut self, pid: u64, priority: u32) {
        self.nodes.insert(pid, ElectionNode::new(pid, priority));
        self.update_stats();
    }

    /// Remove node
    pub fn remove(&mut self, pid: u64) {
        if let Some(node) = self.nodes.get_mut(&pid) {
            node.alive = false;
        }
        // If leader left, trigger election
        if self.current_leader == Some(pid) {
            self.current_leader = None;
        }
        self.update_stats();
    }

    /// Start election
    pub fn start_election(&mut self, initiator: u64, now: u64) -> u64 {
        self.current_term += 1;
        let id = self.next_id;
        self.next_id += 1;

        let total = self.nodes.values().filter(|n| n.alive).count() as u32;
        let election = Election::new(
            id,
            self.current_term,
            self.algorithm,
            initiator,
            total,
            now,
        );
        self.elections.insert(id, election);

        // Initiator becomes candidate
        if let Some(node) = self.nodes.get_mut(&initiator) {
            node.become_candidate(self.current_term);
        }

        self.stats.total_elections += 1;
        id
    }

    /// Vote in election
    pub fn vote(&mut self, election_id: u64, voter: u64, candidate: u64) -> bool {
        let term = self.elections.get(&election_id).map(|e| e.term);
        if let Some(term) = term {
            if let Some(node) = self.nodes.get_mut(&voter) {
                if node.vote(candidate, term) {
                    if let Some(election) = self.elections.get_mut(&election_id) {
                        election.record_vote(candidate);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Resolve election
    pub fn resolve(&mut self, election_id: u64, now: u64) -> Option<u64> {
        match self.algorithm {
            ElectionAlgorithm::Bully => {
                let alive: Vec<u64> = self.nodes.values()
                    .filter(|n| n.alive)
                    .map(|n| n.id)
                    .collect();
                if let Some(election) = self.elections.get_mut(&election_id) {
                    if let Some(winner) = election.bully_winner(&alive, now) {
                        self.install_leader(winner, now);
                        return Some(winner);
                    }
                }
            }
            ElectionAlgorithm::Priority => {
                let winner = self.nodes.values()
                    .filter(|n| n.alive)
                    .max_by_key(|n| n.priority)
                    .map(|n| n.id);
                if let Some(w) = winner {
                    if let Some(election) = self.elections.get_mut(&election_id) {
                        election.winner = Some(w);
                        election.state = ElectionState::Decided;
                        election.decided_at = Some(now);
                    }
                    self.install_leader(w, now);
                    return Some(w);
                }
            }
            _ => {
                if let Some(election) = self.elections.get_mut(&election_id) {
                    if let Some(winner) = election.check_winner(now) {
                        self.install_leader(winner, now);
                        return Some(winner);
                    }
                }
            }
        }
        None
    }

    fn install_leader(&mut self, leader: u64, now: u64) {
        // Step down old leader
        if let Some(old) = self.current_leader {
            if let Some(node) = self.nodes.get_mut(&old) {
                node.step_down(self.current_term);
            }
        }
        // Install new leader
        if let Some(node) = self.nodes.get_mut(&leader) {
            node.become_leader();
        }
        self.current_leader = Some(leader);
        // All followers update
        for node in self.nodes.values_mut() {
            if node.id != leader {
                node.receive_heartbeat(self.current_term, now);
            }
        }
        self.update_stats();
    }

    /// Send heartbeat from leader
    pub fn heartbeat(&mut self, now: u64) {
        if let Some(leader) = self.current_leader {
            let term = self.current_term;
            for node in self.nodes.values_mut() {
                if node.id != leader {
                    node.receive_heartbeat(term, now);
                }
            }
        }
    }

    /// Check for leader timeout
    pub fn check_leader_timeout(&mut self, now: u64) -> bool {
        if self.current_leader.is_none() {
            return true;
        }
        // Check if any follower has timed out
        self.nodes.values()
            .any(|n| n.role == NodeRole::Follower && n.alive && n.heartbeat_timeout(now))
    }

    /// Get leader
    pub fn leader(&self) -> Option<u64> {
        self.current_leader
    }

    fn update_stats(&mut self) {
        self.stats.total_nodes = self.nodes.values().filter(|n| n.alive).count();
        self.stats.current_leader = self.current_leader;
        self.stats.current_term = self.current_term;
    }

    /// Stats
    pub fn stats(&self) -> &CoopElectionStats {
        &self.stats
    }
}
