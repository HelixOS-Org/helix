//! # Distributed Coordination
//!
//! Year 3 EVOLUTION - Coordination protocols for distributed evolution

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// COORDINATION TYPES
// ============================================================================

/// Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Epoch ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochId(pub u64);

/// Transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxId(pub u64);

static TX_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TxId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(TX_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Coordination state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordState {
    Idle,
    Proposing,
    Voting,
    Committing,
    Aborting,
    Recovering,
}

// ============================================================================
// LEADER ELECTION
// ============================================================================

/// Leader election protocol
pub struct LeaderElection {
    /// Node ID
    node_id: NodeId,
    /// Current leader
    leader: Option<NodeId>,
    /// Current epoch
    epoch: EpochId,
    /// Votes received
    votes: BTreeMap<NodeId, EpochId>,
    /// Known nodes
    nodes: Vec<NodeId>,
    /// Is candidate
    is_candidate: AtomicBool,
    /// Election timeout (ticks)
    election_timeout: u64,
    /// Heartbeat timeout (ticks)
    heartbeat_timeout: u64,
    /// Last heartbeat
    last_heartbeat: AtomicU64,
    /// Random state
    random_state: AtomicU64,
}

impl LeaderElection {
    pub fn new(node_id: NodeId, nodes: Vec<NodeId>) -> Self {
        Self {
            node_id,
            leader: None,
            epoch: EpochId(0),
            votes: BTreeMap::new(),
            nodes,
            is_candidate: AtomicBool::new(false),
            election_timeout: 150,
            heartbeat_timeout: 50,
            last_heartbeat: AtomicU64::new(0),
            random_state: AtomicU64::new(node_id.0),
        }
    }

    /// Start election
    pub fn start_election(&mut self) -> Vec<ElectionMessage> {
        self.epoch = EpochId(self.epoch.0 + 1);
        self.is_candidate.store(true, Ordering::Relaxed);
        self.votes.clear();
        self.votes.insert(self.node_id, self.epoch); // Vote for self

        // Request votes from all nodes
        self.nodes
            .iter()
            .filter(|&&id| id != self.node_id)
            .map(|&to| ElectionMessage::RequestVote {
                from: self.node_id,
                to,
                epoch: self.epoch,
            })
            .collect()
    }

    /// Handle vote request
    pub fn handle_vote_request(&mut self, from: NodeId, epoch: EpochId) -> Option<ElectionMessage> {
        if epoch > self.epoch {
            self.epoch = epoch;
            self.leader = None;
            self.is_candidate.store(false, Ordering::Relaxed);

            Some(ElectionMessage::Vote {
                from: self.node_id,
                to: from,
                epoch,
                granted: true,
            })
        } else {
            Some(ElectionMessage::Vote {
                from: self.node_id,
                to: from,
                epoch,
                granted: false,
            })
        }
    }

    /// Handle vote response
    pub fn handle_vote(
        &mut self,
        from: NodeId,
        epoch: EpochId,
        granted: bool,
    ) -> Option<ElectionMessage> {
        if epoch != self.epoch || !self.is_candidate.load(Ordering::Relaxed) {
            return None;
        }

        if granted {
            self.votes.insert(from, epoch);

            // Check if we have majority
            let majority = (self.nodes.len() / 2) + 1;
            if self.votes.len() >= majority {
                self.leader = Some(self.node_id);
                self.is_candidate.store(false, Ordering::Relaxed);

                // Announce leadership
                return Some(ElectionMessage::Leader {
                    leader: self.node_id,
                    epoch,
                });
            }
        }

        None
    }

    /// Handle leader announcement
    #[inline]
    pub fn handle_leader(&mut self, leader: NodeId, epoch: EpochId) {
        if epoch >= self.epoch {
            self.epoch = epoch;
            self.leader = Some(leader);
            self.is_candidate.store(false, Ordering::Relaxed);
            self.last_heartbeat.store(0, Ordering::Relaxed);
        }
    }

    /// Handle heartbeat
    #[inline]
    pub fn handle_heartbeat(&mut self, from: NodeId, epoch: EpochId) {
        if epoch >= self.epoch && Some(from) == self.leader {
            self.last_heartbeat.store(0, Ordering::Relaxed);
        }
    }

    /// Tick (check timeouts)
    pub fn tick(&mut self, current_tick: u64) -> Option<Vec<ElectionMessage>> {
        let last_hb = self.last_heartbeat.fetch_add(1, Ordering::Relaxed);

        if self.leader == Some(self.node_id) {
            // We are leader, send heartbeats
            if last_hb >= self.heartbeat_timeout {
                self.last_heartbeat.store(0, Ordering::Relaxed);
                return Some(
                    self.nodes
                        .iter()
                        .filter(|&&id| id != self.node_id)
                        .map(|&to| ElectionMessage::Heartbeat {
                            from: self.node_id,
                            to,
                            epoch: self.epoch,
                        })
                        .collect(),
                );
            }
        } else if !self.is_candidate.load(Ordering::Relaxed) {
            // Check election timeout with randomization
            let timeout = self.election_timeout + (self.random() % 50);
            if last_hb >= timeout {
                return Some(self.start_election());
            }
        }

        None
    }

    fn random(&self) -> u64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        x
    }

    /// Get current leader
    #[inline(always)]
    pub fn leader(&self) -> Option<NodeId> {
        self.leader
    }

    /// Is this node the leader?
    #[inline(always)]
    pub fn is_leader(&self) -> bool {
        self.leader == Some(self.node_id)
    }
}

/// Election message
#[derive(Debug, Clone)]
pub enum ElectionMessage {
    RequestVote {
        from: NodeId,
        to: NodeId,
        epoch: EpochId,
    },
    Vote {
        from: NodeId,
        to: NodeId,
        epoch: EpochId,
        granted: bool,
    },
    Leader {
        leader: NodeId,
        epoch: EpochId,
    },
    Heartbeat {
        from: NodeId,
        to: NodeId,
        epoch: EpochId,
    },
}

// ============================================================================
// TWO-PHASE COMMIT
// ============================================================================

/// Two-phase commit coordinator
pub struct TwoPhaseCommit {
    /// Node ID
    node_id: NodeId,
    /// Is coordinator
    is_coordinator: bool,
    /// Participants
    participants: Vec<NodeId>,
    /// Active transactions
    transactions: BTreeMap<TxId, TransactionState>,
    /// Log
    log: Vec<LogEntry>,
}

/// Transaction state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TransactionState {
    /// Transaction ID
    pub id: TxId,
    /// Phase
    pub phase: TwoPhasePhase,
    /// Votes
    pub votes: BTreeMap<NodeId, bool>,
    /// Decision
    pub decision: Option<bool>,
    /// Timeout
    pub timeout: u64,
    /// Data
    pub data: Vec<u8>,
}

/// Two-phase commit phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoPhasePhase {
    Init,
    Prepare,
    Voting,
    Commit,
    Abort,
    Done,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Transaction ID
    pub tx_id: TxId,
    /// Entry type
    pub entry_type: LogEntryType,
    /// Timestamp
    pub timestamp: u64,
}

/// Log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryType {
    Prepare,
    VoteYes,
    VoteNo,
    Commit,
    Abort,
    Ack,
}

impl TwoPhaseCommit {
    pub fn new(node_id: NodeId, participants: Vec<NodeId>, is_coordinator: bool) -> Self {
        Self {
            node_id,
            is_coordinator,
            participants,
            transactions: BTreeMap::new(),
            log: Vec::new(),
        }
    }

    /// Begin transaction (coordinator only)
    pub fn begin(&mut self, data: Vec<u8>) -> Option<(TxId, Vec<TwoPhaseMessage>)> {
        if !self.is_coordinator {
            return None;
        }

        let tx_id = TxId::generate();
        let state = TransactionState {
            id: tx_id,
            phase: TwoPhasePhase::Prepare,
            votes: BTreeMap::new(),
            decision: None,
            timeout: 100,
            data: data.clone(),
        };

        self.transactions.insert(tx_id, state);
        self.log.push(LogEntry {
            tx_id,
            entry_type: LogEntryType::Prepare,
            timestamp: 0,
        });

        // Send prepare to all participants
        let messages = self
            .participants
            .iter()
            .map(|&to| TwoPhaseMessage::Prepare {
                tx_id,
                from: self.node_id,
                to,
                data: data.clone(),
            })
            .collect();

        Some((tx_id, messages))
    }

    /// Handle prepare (participant)
    pub fn handle_prepare(&mut self, tx_id: TxId, from: NodeId, data: Vec<u8>) -> TwoPhaseMessage {
        // Decide whether to vote yes or no
        let vote = self.can_commit(&data);

        self.log.push(LogEntry {
            tx_id,
            entry_type: if vote {
                LogEntryType::VoteYes
            } else {
                LogEntryType::VoteNo
            },
            timestamp: 0,
        });

        let state = TransactionState {
            id: tx_id,
            phase: TwoPhasePhase::Voting,
            votes: BTreeMap::new(),
            decision: None,
            timeout: 100,
            data,
        };
        self.transactions.insert(tx_id, state);

        TwoPhaseMessage::Vote {
            tx_id,
            from: self.node_id,
            to: from,
            vote,
        }
    }

    fn can_commit(&self, _data: &[u8]) -> bool {
        // Simplified: always vote yes
        true
    }

    /// Handle vote (coordinator)
    pub fn handle_vote(
        &mut self,
        tx_id: TxId,
        from: NodeId,
        vote: bool,
    ) -> Option<Vec<TwoPhaseMessage>> {
        if !self.is_coordinator {
            return None;
        }

        let state = self.transactions.get_mut(&tx_id)?;
        state.votes.insert(from, vote);

        // Check if all votes received
        if state.votes.len() == self.participants.len() {
            let all_yes = state.votes.values().all(|&v| v);
            state.decision = Some(all_yes);
            state.phase = if all_yes {
                TwoPhasePhase::Commit
            } else {
                TwoPhasePhase::Abort
            };

            self.log.push(LogEntry {
                tx_id,
                entry_type: if all_yes {
                    LogEntryType::Commit
                } else {
                    LogEntryType::Abort
                },
                timestamp: 0,
            });

            // Send decision to all participants
            let messages = self
                .participants
                .iter()
                .map(|&to| {
                    if all_yes {
                        TwoPhaseMessage::Commit {
                            tx_id,
                            from: self.node_id,
                            to,
                        }
                    } else {
                        TwoPhaseMessage::Abort {
                            tx_id,
                            from: self.node_id,
                            to,
                        }
                    }
                })
                .collect();

            return Some(messages);
        }

        None
    }

    /// Handle commit (participant)
    pub fn handle_commit(&mut self, tx_id: TxId) -> Option<TwoPhaseMessage> {
        let state = self.transactions.get_mut(&tx_id)?;
        state.phase = TwoPhasePhase::Done;
        state.decision = Some(true);

        self.log.push(LogEntry {
            tx_id,
            entry_type: LogEntryType::Ack,
            timestamp: 0,
        });

        // Would actually apply changes here

        Some(TwoPhaseMessage::Ack {
            tx_id,
            from: self.node_id,
        })
    }

    /// Handle abort (participant)
    pub fn handle_abort(&mut self, tx_id: TxId) -> Option<TwoPhaseMessage> {
        let state = self.transactions.get_mut(&tx_id)?;
        state.phase = TwoPhasePhase::Done;
        state.decision = Some(false);

        self.log.push(LogEntry {
            tx_id,
            entry_type: LogEntryType::Ack,
            timestamp: 0,
        });

        // Would rollback changes here

        Some(TwoPhaseMessage::Ack {
            tx_id,
            from: self.node_id,
        })
    }

    /// Get transaction state
    #[inline(always)]
    pub fn get_transaction(&self, tx_id: TxId) -> Option<&TransactionState> {
        self.transactions.get(&tx_id)
    }
}

/// Two-phase commit message
#[derive(Debug, Clone)]
pub enum TwoPhaseMessage {
    Prepare {
        tx_id: TxId,
        from: NodeId,
        to: NodeId,
        data: Vec<u8>,
    },
    Vote {
        tx_id: TxId,
        from: NodeId,
        to: NodeId,
        vote: bool,
    },
    Commit {
        tx_id: TxId,
        from: NodeId,
        to: NodeId,
    },
    Abort {
        tx_id: TxId,
        from: NodeId,
        to: NodeId,
    },
    Ack {
        tx_id: TxId,
        from: NodeId,
    },
}

// ============================================================================
// PAXOS
// ============================================================================

/// Simplified Paxos consensus
pub struct Paxos {
    /// Node ID
    node_id: NodeId,
    /// Current proposal number
    proposal_number: AtomicU64,
    /// Highest promised
    promised: AtomicU64,
    /// Accepted proposal
    accepted: Option<(u64, Vec<u8>)>,
    /// Chosen value
    chosen: Option<Vec<u8>>,
    /// Nodes
    nodes: Vec<NodeId>,
    /// Promises received
    promises: BTreeMap<u64, Vec<(NodeId, Option<(u64, Vec<u8>)>)>>,
    /// Accepts received
    accepts: BTreeMap<u64, Vec<NodeId>>,
}

impl Paxos {
    pub fn new(node_id: NodeId, nodes: Vec<NodeId>) -> Self {
        Self {
            node_id,
            proposal_number: AtomicU64::new(node_id.0),
            promised: AtomicU64::new(0),
            accepted: None,
            chosen: None,
            nodes,
            promises: BTreeMap::new(),
            accepts: BTreeMap::new(),
        }
    }

    /// Propose a value (Phase 1a: Prepare)
    pub fn propose(&mut self, value: Vec<u8>) -> (u64, Vec<PaxosMessage>) {
        let n = self
            .proposal_number
            .fetch_add(self.nodes.len() as u64, Ordering::SeqCst);

        let messages = self
            .nodes
            .iter()
            .map(|&to| PaxosMessage::Prepare {
                from: self.node_id,
                to,
                proposal: n,
            })
            .collect();

        // Store our value for Phase 2
        self.promises.insert(n, Vec::new());

        (n, messages)
    }

    /// Handle Prepare (Phase 1b: Promise)
    pub fn handle_prepare(&mut self, from: NodeId, proposal: u64) -> PaxosMessage {
        let promised = self.promised.load(Ordering::Relaxed);

        if proposal > promised {
            self.promised.store(proposal, Ordering::Relaxed);

            PaxosMessage::Promise {
                from: self.node_id,
                to: from,
                proposal,
                accepted: self.accepted.clone(),
            }
        } else {
            PaxosMessage::Nack {
                from: self.node_id,
                to: from,
                proposal,
                highest: promised,
            }
        }
    }

    /// Handle Promise (collect Phase 1b responses)
    pub fn handle_promise(
        &mut self,
        from: NodeId,
        proposal: u64,
        accepted: Option<(u64, Vec<u8>)>,
        value: &[u8],
    ) -> Option<Vec<PaxosMessage>> {
        let promises = self.promises.entry(proposal).or_default();
        promises.push((from, accepted));

        // Check for majority
        let majority = (self.nodes.len() / 2) + 1;
        if promises.len() >= majority {
            // Find highest accepted value, or use our proposed value
            let value_to_propose = promises
                .iter()
                .filter_map(|(_, acc)| acc.as_ref())
                .max_by_key(|(n, _)| *n)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| value.to_vec());

            // Phase 2a: Accept
            let messages = self
                .nodes
                .iter()
                .map(|&to| PaxosMessage::Accept {
                    from: self.node_id,
                    to,
                    proposal,
                    value: value_to_propose.clone(),
                })
                .collect();

            return Some(messages);
        }

        None
    }

    /// Handle Accept (Phase 2b: Accepted)
    pub fn handle_accept(&mut self, from: NodeId, proposal: u64, value: Vec<u8>) -> PaxosMessage {
        let promised = self.promised.load(Ordering::Relaxed);

        if proposal >= promised {
            self.promised.store(proposal, Ordering::Relaxed);
            self.accepted = Some((proposal, value.clone()));

            PaxosMessage::Accepted {
                from: self.node_id,
                to: from,
                proposal,
            }
        } else {
            PaxosMessage::Nack {
                from: self.node_id,
                to: from,
                proposal,
                highest: promised,
            }
        }
    }

    /// Handle Accepted (collect Phase 2b responses)
    pub fn handle_accepted(&mut self, _from: NodeId, proposal: u64, value: &[u8]) -> bool {
        let accepts = self.accepts.entry(proposal).or_default();
        accepts.push(self.node_id); // Track our acceptance

        // Check for majority
        let majority = (self.nodes.len() / 2) + 1;
        if accepts.len() >= majority {
            self.chosen = Some(value.to_vec());
            return true;
        }

        false
    }

    /// Get chosen value
    #[inline(always)]
    pub fn chosen(&self) -> Option<&Vec<u8>> {
        self.chosen.as_ref()
    }
}

/// Paxos message
#[derive(Debug, Clone)]
pub enum PaxosMessage {
    Prepare {
        from: NodeId,
        to: NodeId,
        proposal: u64,
    },
    Promise {
        from: NodeId,
        to: NodeId,
        proposal: u64,
        accepted: Option<(u64, Vec<u8>)>,
    },
    Accept {
        from: NodeId,
        to: NodeId,
        proposal: u64,
        value: Vec<u8>,
    },
    Accepted {
        from: NodeId,
        to: NodeId,
        proposal: u64,
    },
    Nack {
        from: NodeId,
        to: NodeId,
        proposal: u64,
        highest: u64,
    },
}

// ============================================================================
// BARRIER SYNCHRONIZATION
// ============================================================================

/// Distributed barrier
pub struct Barrier {
    /// Node ID
    node_id: NodeId,
    /// Total nodes
    total: usize,
    /// Arrived nodes
    arrived: Vec<NodeId>,
    /// Barrier ID
    barrier_id: u64,
    /// Is released
    released: bool,
}

impl Barrier {
    pub fn new(node_id: NodeId, total: usize) -> Self {
        Self {
            node_id,
            total,
            arrived: Vec::new(),
            barrier_id: 0,
            released: false,
        }
    }

    /// Arrive at barrier
    pub fn arrive(&mut self, barrier_id: u64) -> BarrierMessage {
        if barrier_id > self.barrier_id {
            self.barrier_id = barrier_id;
            self.arrived.clear();
            self.released = false;
        }

        if !self.arrived.contains(&self.node_id) {
            self.arrived.push(self.node_id);
        }

        BarrierMessage::Arrive {
            from: self.node_id,
            barrier_id,
        }
    }

    /// Handle arrival
    pub fn handle_arrive(&mut self, from: NodeId, barrier_id: u64) -> Option<BarrierMessage> {
        if barrier_id > self.barrier_id {
            self.barrier_id = barrier_id;
            self.arrived.clear();
            self.released = false;
        }

        if !self.arrived.contains(&from) {
            self.arrived.push(from);
        }

        // Check if all arrived
        if self.arrived.len() == self.total {
            self.released = true;
            return Some(BarrierMessage::Release { barrier_id });
        }

        None
    }

    /// Handle release
    #[inline]
    pub fn handle_release(&mut self, barrier_id: u64) {
        if barrier_id == self.barrier_id {
            self.released = true;
        }
    }

    /// Is barrier released?
    #[inline(always)]
    pub fn is_released(&self) -> bool {
        self.released
    }

    /// Reset for next barrier
    #[inline]
    pub fn reset(&mut self) {
        self.barrier_id += 1;
        self.arrived.clear();
        self.released = false;
    }
}

/// Barrier message
#[derive(Debug, Clone)]
pub enum BarrierMessage {
    Arrive { from: NodeId, barrier_id: u64 },
    Release { barrier_id: u64 },
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leader_election() {
        let nodes = vec![NodeId(1), NodeId(2), NodeId(3)];
        let mut election = LeaderElection::new(NodeId(1), nodes);

        let messages = election.start_election();
        assert_eq!(messages.len(), 2); // To other 2 nodes
    }

    #[test]
    fn test_two_phase_commit() {
        let participants = vec![NodeId(2), NodeId(3)];
        let mut coordinator = TwoPhaseCommit::new(NodeId(1), participants, true);

        let result = coordinator.begin(vec![1, 2, 3]);
        assert!(result.is_some());

        let (tx_id, messages) = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_barrier() {
        let mut barrier = Barrier::new(NodeId(1), 3);

        let msg = barrier.arrive(1);
        assert!(!barrier.is_released());

        barrier.handle_arrive(NodeId(2), 1);
        assert!(!barrier.is_released());

        let release = barrier.handle_arrive(NodeId(3), 1);
        assert!(release.is_some());
        assert!(barrier.is_released());
    }
}
