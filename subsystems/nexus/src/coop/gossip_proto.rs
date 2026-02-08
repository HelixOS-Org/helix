// SPDX-License-Identifier: GPL-2.0
//! Coop gossip_proto — gossip protocol for distributed state.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Gossip message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipMsgType {
    Heartbeat,
    StateSync,
    Join,
    Leave,
    Suspect,
    Alive,
    Dead,
    UserData,
}

/// Node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipNodeState {
    Alive,
    Suspected,
    Dead,
    Left,
}

/// Gossip node
#[derive(Debug, Clone)]
pub struct GossipNode {
    pub id: u64,
    pub generation: u32,
    pub state: GossipNodeState,
    pub heartbeat_seq: u64,
    pub last_heartbeat: u64,
    pub suspicion_start: u64,
    pub metadata: BTreeMap<u64, u64>,
}

impl GossipNode {
    pub fn new(id: u64, now: u64) -> Self {
        Self {
            id, generation: 1, state: GossipNodeState::Alive,
            heartbeat_seq: 0, last_heartbeat: now, suspicion_start: 0,
            metadata: BTreeMap::new(),
        }
    }

    pub fn heartbeat(&mut self, now: u64) {
        self.heartbeat_seq += 1;
        self.last_heartbeat = now;
        self.state = GossipNodeState::Alive;
    }

    pub fn suspect(&mut self, now: u64) {
        if self.state == GossipNodeState::Alive {
            self.state = GossipNodeState::Suspected;
            self.suspicion_start = now;
        }
    }

    pub fn declare_dead(&mut self) { self.state = GossipNodeState::Dead; }
    pub fn leave(&mut self) { self.state = GossipNodeState::Left; }

    pub fn is_suspect_timeout(&self, now: u64, timeout: u64) -> bool {
        self.state == GossipNodeState::Suspected && now.saturating_sub(self.suspicion_start) > timeout
    }
}

/// Gossip message
#[derive(Debug, Clone)]
pub struct GossipMessage {
    pub msg_type: GossipMsgType,
    pub sender_id: u64,
    pub target_id: u64,
    pub generation: u32,
    pub seq: u64,
    pub payload_hash: u64,
    pub timestamp: u64,
}

/// Infection style (push/pull/push-pull)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipStyle {
    Push,
    Pull,
    PushPull,
}

/// Stats
#[derive(Debug, Clone)]
pub struct GossipProtoStats {
    pub total_nodes: u32,
    pub alive_nodes: u32,
    pub suspected_nodes: u32,
    pub dead_nodes: u32,
    pub total_messages: u64,
    pub total_rounds: u64,
}

/// Main gossip protocol
pub struct CoopGossipProto {
    local_id: u64,
    nodes: BTreeMap<u64, GossipNode>,
    messages: Vec<GossipMessage>,
    style: GossipStyle,
    fanout: u32,
    suspicion_timeout: u64,
    total_rounds: u64,
    prng_state: u64,
}

impl CoopGossipProto {
    pub fn new(local_id: u64, style: GossipStyle) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(local_id, GossipNode::new(local_id, 0));
        Self {
            local_id, nodes, messages: Vec::new(), style,
            fanout: 3, suspicion_timeout: 5_000_000_000,
            total_rounds: 0, prng_state: local_id.wrapping_mul(6364136223846793005) | 1,
        }
    }

    fn xorshift64(&mut self) -> u64 {
        let mut x = self.prng_state;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.prng_state = x;
        x
    }

    pub fn join(&mut self, node_id: u64, now: u64) {
        self.nodes.insert(node_id, GossipNode::new(node_id, now));
    }

    pub fn heartbeat(&mut self, now: u64) {
        if let Some(local) = self.nodes.get_mut(&self.local_id) { local.heartbeat(now); }
    }

    pub fn gossip_round(&mut self, now: u64) {
        self.total_rounds += 1;
        let node_ids: Vec<u64> = self.nodes.keys().filter(|&&id| id != self.local_id).copied().collect();
        if node_ids.is_empty() { return; }

        // Pick fanout random targets
        let targets: Vec<u64> = (0..self.fanout).map(|_| {
            let idx = (self.xorshift64() % node_ids.len() as u64) as usize;
            node_ids[idx]
        }).collect();

        let local_seq = self.nodes.get(&self.local_id).map(|n| n.heartbeat_seq).unwrap_or(0);
        for target in targets {
            self.messages.push(GossipMessage {
                msg_type: GossipMsgType::Heartbeat, sender_id: self.local_id,
                target_id: target, generation: 1, seq: local_seq,
                payload_hash: 0, timestamp: now,
            });
        }

        // Check suspicions
        let timeout = self.suspicion_timeout;
        let suspect_list: Vec<u64> = self.nodes.iter()
            .filter(|(&id, n)| id != self.local_id && n.is_suspect_timeout(now, timeout))
            .map(|(&id, _)| id).collect();
        for id in suspect_list {
            if let Some(node) = self.nodes.get_mut(&id) { node.declare_dead(); }
        }
    }

    pub fn stats(&self) -> GossipProtoStats {
        let alive = self.nodes.values().filter(|n| n.state == GossipNodeState::Alive).count() as u32;
        let suspected = self.nodes.values().filter(|n| n.state == GossipNodeState::Suspected).count() as u32;
        let dead = self.nodes.values().filter(|n| n.state == GossipNodeState::Dead).count() as u32;
        GossipProtoStats {
            total_nodes: self.nodes.len() as u32, alive_nodes: alive,
            suspected_nodes: suspected, dead_nodes: dead,
            total_messages: self.messages.len() as u64, total_rounds: self.total_rounds,
        }
    }
}
