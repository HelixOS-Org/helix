//! # Cooperative Gossip Protocol
//!
//! Distributed state dissemination between cores/nodes:
//! - Epidemic gossip protocol
//! - Anti-entropy synchronization
//! - Rumor mongering
//! - Membership management
//! - Vector clocks for causality
//! - Failure detection via heartbeats

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// GOSSIP MESSAGE TYPES
// ============================================================================

/// Gossip message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipMessageType {
    /// Regular state update
    StateUpdate,
    /// Heartbeat / alive signal
    Heartbeat,
    /// Join announcement
    Join,
    /// Leave announcement
    Leave,
    /// Anti-entropy sync request
    SyncRequest,
    /// Anti-entropy sync response
    SyncResponse,
    /// Rumor
    Rumor,
    /// Ack
    Ack,
}

/// Gossip message
#[derive(Debug, Clone)]
pub struct GossipMessage {
    /// Message ID
    pub id: u64,
    /// Type
    pub msg_type: GossipMessageType,
    /// Source node
    pub source: u32,
    /// Destination node (0 = broadcast)
    pub destination: u32,
    /// Key
    pub key: u64,
    /// Value
    pub value: u64,
    /// Version (vector clock component)
    pub version: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Hops remaining
    pub ttl: u8,
}

// ============================================================================
// VECTOR CLOCK
// ============================================================================

/// Vector clock for causal ordering
#[derive(Debug, Clone)]
pub struct VectorClock {
    /// Per-node logical timestamps
    pub clocks: BTreeMap<u32, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: BTreeMap::new(),
        }
    }

    /// Increment for node
    pub fn increment(&mut self, node: u32) {
        let entry = self.clocks.entry(node).or_insert(0);
        *entry += 1;
    }

    /// Get clock for node
    pub fn get(&self, node: u32) -> u64 {
        self.clocks.get(&node).copied().unwrap_or(0)
    }

    /// Merge with other clock (take max)
    pub fn merge(&mut self, other: &VectorClock) {
        for (&node, &time) in &other.clocks {
            let entry = self.clocks.entry(node).or_insert(0);
            if time > *entry {
                *entry = time;
            }
        }
    }

    /// Check if this clock happened-before other
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for (&node, &time) in &self.clocks {
            let other_time = other.get(node);
            if time > other_time {
                return false;
            }
            if time < other_time {
                at_least_one_less = true;
            }
        }
        // Check nodes in other but not in self
        for (&node, &time) in &other.clocks {
            if !self.clocks.contains_key(&node) && time > 0 {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    /// Are these concurrent?
    pub fn concurrent(&self, other: &VectorClock) -> bool {
        !self.happened_before(other) && !other.happened_before(self) && self.clocks != other.clocks
    }
}

// ============================================================================
// GOSSIP NODE STATE
// ============================================================================

/// Node health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHealth {
    /// Alive
    Alive,
    /// Suspected dead
    Suspected,
    /// Confirmed dead
    Dead,
    /// Joining
    Joining,
    /// Leaving
    Leaving,
}

/// Gossip node state
#[derive(Debug, Clone)]
pub struct GossipNode {
    /// Node ID (core/CPU)
    pub id: u32,
    /// Health
    pub health: NodeHealth,
    /// Last heartbeat received
    pub last_heartbeat: u64,
    /// Heartbeat count
    pub heartbeat_count: u64,
    /// Vector clock
    pub vclock: VectorClock,
    /// Key-value state
    pub state: BTreeMap<u64, (u64, u64)>, // key → (value, version)
    /// Rumor buffer (unconfirmed messages)
    pub rumors: Vec<GossipMessage>,
    /// Suspected by nodes
    pub suspected_by: Vec<u32>,
}

impl GossipNode {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            health: NodeHealth::Alive,
            last_heartbeat: 0,
            heartbeat_count: 0,
            vclock: VectorClock::new(),
            state: BTreeMap::new(),
            rumors: Vec::new(),
            suspected_by: Vec::new(),
        }
    }

    /// Update state entry
    pub fn update(&mut self, key: u64, value: u64) {
        self.vclock.increment(self.id);
        let version = self.vclock.get(self.id);
        self.state.insert(key, (value, version));
    }

    /// Get state entry
    pub fn get(&self, key: u64) -> Option<(u64, u64)> {
        self.state.get(&key).copied()
    }

    /// Record heartbeat
    pub fn heartbeat(&mut self, now: u64) {
        self.last_heartbeat = now;
        self.heartbeat_count += 1;
        self.health = NodeHealth::Alive;
        self.suspected_by.clear();
    }

    /// Check if suspect
    pub fn check_suspect(&mut self, now: u64, timeout_ms: u64) -> bool {
        if self.health == NodeHealth::Alive && now.saturating_sub(self.last_heartbeat) > timeout_ms {
            self.health = NodeHealth::Suspected;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// GOSSIP CONFIG
// ============================================================================

/// Gossip protocol configuration
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Fanout (number of peers to gossip to)
    pub fanout: u32,
    /// Gossip interval (ms)
    pub interval_ms: u64,
    /// Heartbeat interval (ms)
    pub heartbeat_interval_ms: u64,
    /// Suspect timeout (ms)
    pub suspect_timeout_ms: u64,
    /// Dead timeout (ms)
    pub dead_timeout_ms: u64,
    /// Message TTL (hops)
    pub ttl: u8,
    /// Max rumors per node
    pub max_rumors: usize,
    /// Anti-entropy interval (ms)
    pub anti_entropy_interval_ms: u64,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            interval_ms: 100,
            heartbeat_interval_ms: 500,
            suspect_timeout_ms: 2000,
            dead_timeout_ms: 10000,
            ttl: 5,
            max_rumors: 64,
            anti_entropy_interval_ms: 5000,
        }
    }
}

// ============================================================================
// GOSSIP MANAGER
// ============================================================================

/// Gossip stats
#[derive(Debug, Clone, Default)]
pub struct GossipStats {
    /// Total nodes
    pub total_nodes: usize,
    /// Alive nodes
    pub alive_nodes: usize,
    /// Suspected nodes
    pub suspected_nodes: usize,
    /// Dead nodes
    pub dead_nodes: usize,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// State entries across cluster
    pub total_state_entries: usize,
    /// Convergence time (ms)
    pub last_convergence_ms: u64,
}

/// Cooperative gossip manager
pub struct CoopGossipManager {
    /// Local node ID
    local_id: u32,
    /// Nodes
    nodes: BTreeMap<u32, GossipNode>,
    /// Configuration
    config: GossipConfig,
    /// Outgoing message queue
    outbox: Vec<GossipMessage>,
    /// Next message ID
    next_msg_id: u64,
    /// Last gossip round
    last_gossip: u64,
    /// Last anti-entropy
    last_anti_entropy: u64,
    /// Stats
    stats: GossipStats,
}

impl CoopGossipManager {
    pub fn new(local_id: u32, config: GossipConfig) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(local_id, GossipNode::new(local_id));

        Self {
            local_id,
            nodes,
            config,
            outbox: Vec::new(),
            next_msg_id: 1,
            last_gossip: 0,
            last_anti_entropy: 0,
            stats: GossipStats::default(),
        }
    }

    /// Add node
    pub fn add_node(&mut self, node_id: u32, now: u64) {
        let mut node = GossipNode::new(node_id);
        node.health = NodeHealth::Joining;
        node.last_heartbeat = now;
        self.nodes.insert(node_id, node);

        // Broadcast join
        self.broadcast(GossipMessageType::Join, 0, node_id as u64, now);
        self.update_stats();
    }

    /// Remove node
    pub fn remove_node(&mut self, node_id: u32, now: u64) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.health = NodeHealth::Leaving;
        }
        self.broadcast(GossipMessageType::Leave, 0, node_id as u64, now);
        self.update_stats();
    }

    /// Update local state
    pub fn update_state(&mut self, key: u64, value: u64, now: u64) {
        if let Some(local) = self.nodes.get_mut(&self.local_id) {
            local.update(key, value);
        }
        self.broadcast(GossipMessageType::StateUpdate, key, value, now);
    }

    /// Receive message
    pub fn receive(&mut self, msg: GossipMessage, now: u64) {
        self.stats.messages_received += 1;

        match msg.msg_type {
            GossipMessageType::Heartbeat => {
                if let Some(node) = self.nodes.get_mut(&msg.source) {
                    node.heartbeat(now);
                }
            }
            GossipMessageType::StateUpdate => {
                // Merge state
                if let Some(node) = self.nodes.get_mut(&msg.source) {
                    let existing_version = node
                        .state
                        .get(&msg.key)
                        .map(|(_, v)| *v)
                        .unwrap_or(0);

                    if msg.version > existing_version {
                        node.state.insert(msg.key, (msg.value, msg.version));
                    }
                }

                // Propagate if TTL remaining
                if msg.ttl > 1 {
                    let mut forwarded = msg.clone();
                    forwarded.ttl -= 1;
                    self.outbox.push(forwarded);
                }
            }
            GossipMessageType::Join => {
                if !self.nodes.contains_key(&msg.source) {
                    self.add_node(msg.source, now);
                }
            }
            GossipMessageType::Leave => {
                if let Some(node) = self.nodes.get_mut(&msg.source) {
                    node.health = NodeHealth::Dead;
                }
            }
            _ => {}
        }
    }

    /// Gossip tick — run periodically
    pub fn tick(&mut self, now: u64) {
        // Heartbeat
        if let Some(local) = self.nodes.get_mut(&self.local_id) {
            local.heartbeat(now);
        }

        // Check suspects
        let timeout = self.config.suspect_timeout_ms;
        let dead_timeout = self.config.dead_timeout_ms;
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();

        for node_id in node_ids {
            if node_id == self.local_id {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(&node_id) {
                let elapsed = now.saturating_sub(node.last_heartbeat);
                if elapsed > dead_timeout && node.health != NodeHealth::Dead {
                    node.health = NodeHealth::Dead;
                } else if elapsed > timeout && node.health == NodeHealth::Alive {
                    node.health = NodeHealth::Suspected;
                }
            }
        }

        // Gossip round
        if now.saturating_sub(self.last_gossip) >= self.config.interval_ms {
            self.gossip_round(now);
            self.last_gossip = now;
        }

        self.update_stats();
    }

    /// Execute a gossip round
    fn gossip_round(&mut self, now: u64) {
        let alive: Vec<u32> = self
            .nodes
            .iter()
            .filter(|(&id, n)| id != self.local_id && n.health == NodeHealth::Alive)
            .map(|(id, _)| *id)
            .collect();

        if alive.is_empty() {
            return;
        }

        // Select fanout peers (simple: take first N)
        let fanout = (self.config.fanout as usize).min(alive.len());
        let peers = &alive[..fanout];

        // Send heartbeat to each
        for &peer in peers {
            let msg = GossipMessage {
                id: self.next_msg_id,
                msg_type: GossipMessageType::Heartbeat,
                source: self.local_id,
                destination: peer,
                key: 0,
                value: 0,
                version: 0,
                timestamp: now,
                ttl: 1,
            };
            self.next_msg_id += 1;
            self.outbox.push(msg);
            self.stats.messages_sent += 1;
        }
    }

    /// Broadcast message to all alive nodes
    fn broadcast(&mut self, msg_type: GossipMessageType, key: u64, value: u64, now: u64) {
        let version = self
            .nodes
            .get(&self.local_id)
            .map(|n| n.vclock.get(self.local_id))
            .unwrap_or(0);

        for (&node_id, node) in &self.nodes {
            if node_id == self.local_id || node.health == NodeHealth::Dead {
                continue;
            }

            let msg = GossipMessage {
                id: self.next_msg_id,
                msg_type,
                source: self.local_id,
                destination: node_id,
                key,
                value,
                version,
                timestamp: now,
                ttl: self.config.ttl,
            };
            self.next_msg_id += 1;
            self.outbox.push(msg);
            self.stats.messages_sent += 1;
        }
    }

    /// Drain outbox
    pub fn drain_outbox(&mut self) -> Vec<GossipMessage> {
        let msgs = self.outbox.clone();
        self.outbox.clear();
        msgs
    }

    /// Get node state
    pub fn node(&self, id: u32) -> Option<&GossipNode> {
        self.nodes.get(&id)
    }

    /// Get local state value
    pub fn get_state(&self, key: u64) -> Option<u64> {
        self.nodes
            .get(&self.local_id)
            .and_then(|n| n.get(key))
            .map(|(v, _)| v)
    }

    fn update_stats(&mut self) {
        self.stats.total_nodes = self.nodes.len();
        self.stats.alive_nodes = self
            .nodes
            .values()
            .filter(|n| n.health == NodeHealth::Alive)
            .count();
        self.stats.suspected_nodes = self
            .nodes
            .values()
            .filter(|n| n.health == NodeHealth::Suspected)
            .count();
        self.stats.dead_nodes = self
            .nodes
            .values()
            .filter(|n| n.health == NodeHealth::Dead)
            .count();
        self.stats.total_state_entries = self.nodes.values().map(|n| n.state.len()).sum();
    }

    /// Get stats
    pub fn stats(&self) -> &GossipStats {
        &self.stats
    }
}

// ============================================================================
// Merged from gossip_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipMsgType {
    Ping,
    PingReq,
    Ack,
    Alive,
    Suspect,
    Dead,
    PushPull,
    Digest,
    Delta,
}

/// Node liveness state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    Alive,
    Suspected,
    Dead,
    Left,
}

/// Rumor
#[derive(Debug, Clone)]
pub struct Rumor {
    pub id: u64,
    pub key: String,
    pub version: u64,
    pub origin_node: u64,
    pub payload_hash: u64,
    pub created_ts: u64,
    pub infections: u32,
    pub max_infections: u32,
    pub is_expired: bool,
}

impl Rumor {
    pub fn new(id: u64, key: String, version: u64, origin: u64, ts: u64) -> Self {
        Self {
            id, key, version, origin_node: origin, payload_hash: 0,
            created_ts: ts, infections: 0, max_infections: 10, is_expired: false,
        }
    }

    pub fn infect(&mut self) {
        self.infections += 1;
        if self.infections >= self.max_infections { self.is_expired = true; }
    }

    pub fn should_spread(&self) -> bool { !self.is_expired }
}

/// Gossip peer state
#[derive(Debug, Clone)]
pub struct GossipPeer {
    pub node_id: u64,
    pub liveness: LivenessState,
    pub incarnation: u64,
    pub last_ping_ts: u64,
    pub last_ack_ts: u64,
    pub ping_req_targets: Vec<u64>,
    pub suspicion_ts: Option<u64>,
    pub suspicion_confirmers: Vec<u64>,
    pub suspicion_timeout_ns: u64,
    pub msgs_sent: u64,
    pub msgs_recv: u64,
    pub round_trip_ns: u64,
    pub min_rtt_ns: u64,
}

impl GossipPeer {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id, liveness: LivenessState::Alive, incarnation: 0,
            last_ping_ts: 0, last_ack_ts: 0, ping_req_targets: Vec::new(),
            suspicion_ts: None, suspicion_confirmers: Vec::new(),
            suspicion_timeout_ns: 10_000_000_000, msgs_sent: 0, msgs_recv: 0,
            round_trip_ns: 0, min_rtt_ns: u64::MAX,
        }
    }

    pub fn ping(&mut self, ts: u64) { self.last_ping_ts = ts; self.msgs_sent += 1; }

    pub fn ack(&mut self, ts: u64) {
        self.last_ack_ts = ts;
        self.msgs_recv += 1;
        self.round_trip_ns = ts.saturating_sub(self.last_ping_ts);
        if self.round_trip_ns < self.min_rtt_ns { self.min_rtt_ns = self.round_trip_ns; }
        if self.liveness == LivenessState::Suspected { self.liveness = LivenessState::Alive; self.suspicion_ts = None; }
    }

    pub fn suspect(&mut self, ts: u64) {
        if self.liveness == LivenessState::Alive {
            self.liveness = LivenessState::Suspected;
            self.suspicion_ts = Some(ts);
            self.suspicion_confirmers.clear();
        }
    }

    pub fn confirm_suspicion(&mut self, confirmer: u64) {
        if !self.suspicion_confirmers.contains(&confirmer) {
            self.suspicion_confirmers.push(confirmer);
        }
    }

    pub fn is_suspicion_expired(&self, now: u64) -> bool {
        if let Some(ts) = self.suspicion_ts {
            now.saturating_sub(ts) >= self.suspicion_timeout_ns
        } else { false }
    }

    pub fn mark_dead(&mut self) { self.liveness = LivenessState::Dead; }
}

/// Digest entry for push-pull
#[derive(Debug, Clone)]
pub struct DigestEntry {
    pub key: String,
    pub version: u64,
    pub node_id: u64,
}

/// Gossip protocol v2 stats
#[derive(Debug, Clone, Default)]
pub struct GossipV2Stats {
    pub total_peers: usize,
    pub alive_peers: usize,
    pub suspected_peers: usize,
    pub dead_peers: usize,
    pub active_rumors: usize,
    pub expired_rumors: usize,
    pub total_msgs_sent: u64,
    pub total_msgs_recv: u64,
    pub gossip_rounds: u64,
    pub avg_rtt_ns: f64,
}

/// Coop gossip protocol v2
pub struct CoopGossipV2 {
    self_id: u64,
    peers: BTreeMap<u64, GossipPeer>,
    rumors: BTreeMap<u64, Rumor>,
    state_versions: BTreeMap<u64, u64>,
    stats: GossipV2Stats,
    next_id: u64,
    fanout: u32,
    gossip_interval_ns: u64,
    last_gossip_ts: u64,
    incarnation: u64,
    rng_state: u64,
}

impl CoopGossipV2 {
    pub fn new(self_id: u64, fanout: u32, interval_ns: u64) -> Self {
        Self {
            self_id, peers: BTreeMap::new(), rumors: BTreeMap::new(),
            state_versions: BTreeMap::new(), stats: GossipV2Stats::default(),
            next_id: 1, fanout, gossip_interval_ns: interval_ns,
            last_gossip_ts: 0, incarnation: 1, rng_state: self_id ^ 0xdeadbeef,
        }
    }

    fn next_rand(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    pub fn add_peer(&mut self, node_id: u64) {
        self.peers.entry(node_id).or_insert_with(|| GossipPeer::new(node_id));
    }

    pub fn remove_peer(&mut self, node_id: u64) { self.peers.remove(&node_id); }

    pub fn spread_rumor(&mut self, key: String, version: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.rumors.insert(id, Rumor::new(id, key, version, self.self_id, ts));
        id
    }

    pub fn select_targets(&mut self) -> Vec<u64> {
        let alive: Vec<u64> = self.peers.values().filter(|p| p.liveness == LivenessState::Alive).map(|p| p.node_id).collect();
        if alive.is_empty() { return Vec::new(); }
        let mut targets = Vec::new();
        let n = (self.fanout as usize).min(alive.len());
        let mut indices: Vec<usize> = (0..alive.len()).collect();
        for i in 0..n {
            let j = (self.next_rand() as usize) % (alive.len() - i) + i;
            indices.swap(i, j);
            targets.push(alive[indices[i]]);
        }
        targets
    }

    pub fn ping(&mut self, target: u64, ts: u64) {
        if let Some(p) = self.peers.get_mut(&target) { p.ping(ts); }
    }

    pub fn receive_ack(&mut self, from: u64, ts: u64) {
        if let Some(p) = self.peers.get_mut(&from) { p.ack(ts); }
    }

    pub fn suspect_node(&mut self, node_id: u64, ts: u64) {
        if let Some(p) = self.peers.get_mut(&node_id) { p.suspect(ts); }
    }

    pub fn check_suspicions(&mut self, now: u64) {
        let expired: Vec<u64> = self.peers.values()
            .filter(|p| p.liveness == LivenessState::Suspected && p.is_suspicion_expired(now))
            .map(|p| p.node_id).collect();
        for id in expired {
            if let Some(p) = self.peers.get_mut(&id) { p.mark_dead(); }
        }
    }

    pub fn refute_suspicion(&mut self) {
        self.incarnation += 1;
    }

    pub fn gossip_round(&mut self, now: u64) {
        self.stats.gossip_rounds += 1;
        self.last_gossip_ts = now;
        // Age rumors
        let expired: Vec<u64> = self.rumors.iter().filter(|(_, r)| r.is_expired).map(|(&id, _)| id).collect();
        for id in expired { self.rumors.remove(&id); }
    }

    pub fn infect_rumor(&mut self, rumor_id: u64) {
        if let Some(r) = self.rumors.get_mut(&rumor_id) { r.infect(); }
    }

    pub fn get_active_rumors(&self) -> Vec<&Rumor> {
        self.rumors.values().filter(|r| r.should_spread()).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_peers = self.peers.len();
        self.stats.alive_peers = self.peers.values().filter(|p| p.liveness == LivenessState::Alive).count();
        self.stats.suspected_peers = self.peers.values().filter(|p| p.liveness == LivenessState::Suspected).count();
        self.stats.dead_peers = self.peers.values().filter(|p| p.liveness == LivenessState::Dead).count();
        self.stats.active_rumors = self.rumors.values().filter(|r| !r.is_expired).count();
        self.stats.expired_rumors = self.rumors.values().filter(|r| r.is_expired).count();
        self.stats.total_msgs_sent = self.peers.values().map(|p| p.msgs_sent).sum();
        self.stats.total_msgs_recv = self.peers.values().map(|p| p.msgs_recv).sum();
        let alive_rtts: Vec<u64> = self.peers.values().filter(|p| p.round_trip_ns > 0).map(|p| p.round_trip_ns).collect();
        self.stats.avg_rtt_ns = if alive_rtts.is_empty() { 0.0 } else { alive_rtts.iter().sum::<u64>() as f64 / alive_rtts.len() as f64 };
    }

    pub fn peer(&self, id: u64) -> Option<&GossipPeer> { self.peers.get(&id) }
    pub fn stats(&self) -> &GossipV2Stats { &self.stats }
}
