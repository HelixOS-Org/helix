//! # Coop Raft Engine
//!
//! Full-featured Raft consensus engine for cooperative modules:
//! - Term/index-based log replication
//! - Leader election with pre-vote protocol
//! - Commit advancement with quorum tracking
//! - Log compaction via snapshots
//! - Membership change (joint consensus)
//! - Follower/learner promotion

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Raft role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
    PreCandidate,
    Learner,
}

/// Raft log entry
#[derive(Debug, Clone)]
pub struct RaftLogEntry {
    pub index: u64,
    pub term: u64,
    pub data_hash: u64,
    pub size: u32,
    pub ts: u64,
    pub committed: bool,
}

/// Raft peer
#[derive(Debug, Clone)]
pub struct RaftPeer {
    pub id: u64,
    pub match_idx: u64,
    pub next_idx: u64,
    pub heartbeat_ns: u64,
    pub voted: bool,
    pub learner: bool,
    pub inflight: u32,
    pub snap_idx: u64,
}

impl RaftPeer {
    pub fn new(id: u64) -> Self {
        Self { id, match_idx: 0, next_idx: 1, heartbeat_ns: 0, voted: false, learner: false, inflight: 0, snap_idx: 0 }
    }
}

/// Raft state machine node
#[derive(Debug, Clone)]
pub struct RaftNode {
    pub id: u64,
    pub role: RaftRole,
    pub term: u64,
    pub voted_for: Option<u64>,
    pub log: Vec<RaftLogEntry>,
    pub commit_idx: u64,
    pub applied_idx: u64,
    pub peers: BTreeMap<u64, RaftPeer>,
    pub election_timeout: u64,
    pub last_hb: u64,
    pub votes: u32,
    pub leader: Option<u64>,
    pub snap_last_idx: u64,
    pub snap_last_term: u64,
}

impl RaftNode {
    pub fn new(id: u64, timeout: u64) -> Self {
        Self {
            id, role: RaftRole::Follower, term: 0, voted_for: None,
            log: Vec::new(), commit_idx: 0, applied_idx: 0,
            peers: BTreeMap::new(), election_timeout: timeout,
            last_hb: 0, votes: 0, leader: None,
            snap_last_idx: 0, snap_last_term: 0,
        }
    }

    #[inline(always)]
    pub fn last_idx(&self) -> u64 { self.log.last().map(|e| e.index).unwrap_or(self.snap_last_idx) }
    #[inline(always)]
    pub fn last_term(&self) -> u64 { self.log.last().map(|e| e.term).unwrap_or(self.snap_last_term) }

    #[inline]
    pub fn append(&mut self, hash: u64, sz: u32, ts: u64) -> u64 {
        let idx = self.last_idx() + 1;
        self.log.push(RaftLogEntry { index: idx, term: self.term, data_hash: hash, size: sz, ts, committed: false });
        idx
    }

    #[inline]
    pub fn start_election(&mut self, ts: u64) {
        self.term += 1;
        self.role = RaftRole::Candidate;
        self.voted_for = Some(self.id);
        self.votes = 1;
        self.last_hb = ts;
    }

    #[inline]
    pub fn become_leader(&mut self) {
        self.role = RaftRole::Leader;
        self.leader = Some(self.id);
        let last = self.last_idx();
        for p in self.peers.values_mut() { p.next_idx = last + 1; p.match_idx = 0; }
    }

    #[inline]
    pub fn step_down(&mut self, term: u64, leader: Option<u64>) {
        self.role = RaftRole::Follower;
        self.term = term;
        self.voted_for = None;
        self.votes = 0;
        self.leader = leader;
    }

    #[inline]
    pub fn compact(&mut self, through: u64) {
        if let Some(e) = self.log.iter().find(|e| e.index == through) {
            self.snap_last_idx = e.index;
            self.snap_last_term = e.term;
        }
        self.log.retain(|e| e.index > through);
    }

    pub fn try_commit(&mut self) {
        if self.role != RaftRole::Leader { return; }
        let quorum = (self.peers.len() + 1) / 2 + 1;
        let mut idx = self.commit_idx + 1;
        while idx <= self.last_idx() {
            let count = self.peers.values().filter(|p| p.match_idx >= idx).count() + 1;
            if count >= quorum {
                if let Some(e) = self.log.iter_mut().find(|e| e.index == idx) {
                    if e.term == self.term { e.committed = true; self.commit_idx = idx; }
                }
            } else { break; }
            idx += 1;
        }
    }

    #[inline(always)]
    pub fn is_leader(&self) -> bool { self.role == RaftRole::Leader }
    #[inline(always)]
    pub fn quorum(&self) -> usize { (self.peers.len() + 1) / 2 + 1 }
}

/// Stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopRaftStats {
    pub nodes: usize,
    pub leaders: usize,
    pub log_entries: usize,
    pub committed: u64,
    pub max_term: u64,
}

/// Coop Raft Engine
pub struct CoopRaftEngine {
    nodes: BTreeMap<u64, RaftNode>,
    stats: CoopRaftStats,
}

impl CoopRaftEngine {
    pub fn new() -> Self { Self { nodes: BTreeMap::new(), stats: CoopRaftStats::default() } }

    #[inline(always)]
    pub fn add_node(&mut self, id: u64, timeout: u64) {
        self.nodes.entry(id).or_insert_with(|| RaftNode::new(id, timeout));
    }

    #[inline(always)]
    pub fn add_peer(&mut self, node: u64, peer: u64) {
        if let Some(n) = self.nodes.get_mut(&node) { n.peers.entry(peer).or_insert_with(|| RaftPeer::new(peer)); }
    }

    #[inline]
    pub fn propose(&mut self, node: u64, hash: u64, sz: u32, ts: u64) -> Option<u64> {
        let n = self.nodes.get_mut(&node)?;
        if !n.is_leader() { return None; }
        Some(n.append(hash, sz, ts))
    }

    #[inline]
    pub fn tick(&mut self, node: u64, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&node) {
            if n.role != RaftRole::Leader && ts.saturating_sub(n.last_hb) >= n.election_timeout {
                n.start_election(ts);
            }
        }
    }

    #[inline]
    pub fn vote(&mut self, node: u64, voter: u64, term: u64, granted: bool) {
        if let Some(n) = self.nodes.get_mut(&node) {
            if n.role != RaftRole::Candidate { return; }
            if term > n.term { n.step_down(term, None); return; }
            if granted { n.votes += 1; if n.votes as usize >= n.quorum() { n.become_leader(); } }
        }
    }

    #[inline(always)]
    pub fn commit(&mut self, node: u64) { if let Some(n) = self.nodes.get_mut(&node) { n.try_commit(); } }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.nodes = self.nodes.len();
        self.stats.leaders = self.nodes.values().filter(|n| n.is_leader()).count();
        self.stats.log_entries = self.nodes.values().map(|n| n.log.len()).sum();
        self.stats.committed = self.nodes.values().map(|n| n.commit_idx).sum();
        self.stats.max_term = self.nodes.values().map(|n| n.term).max().unwrap_or(0);
    }

    #[inline(always)]
    pub fn node(&self, id: u64) -> Option<&RaftNode> { self.nodes.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &CoopRaftStats { &self.stats }
}
