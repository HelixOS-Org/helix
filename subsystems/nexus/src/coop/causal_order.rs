//! # Coop Causal Ordering
//!
//! Causal ordering for distributed events:
//! - Vector clocks per node
//! - Causal dependency tracking
//! - Happens-before relation enforcement
//! - Concurrent event detection
//! - Causal barrier synchronization
//! - Total order broadcast support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Vector clock
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorClock {
    pub entries: BTreeMap<u64, u64>,
}

impl VectorClock {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn increment(&mut self, node_id: u64) {
        let counter = self.entries.entry(node_id).or_insert(0);
        *counter += 1;
    }

    pub fn get(&self, node_id: u64) -> u64 {
        self.entries.get(&node_id).copied().unwrap_or(0)
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (&node, &counter) in &other.entries {
            let entry = self.entries.entry(node).or_insert(0);
            if counter > *entry { *entry = counter; }
        }
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for (&node, &counter) in &self.entries {
            let other_counter = other.get(node);
            if counter > other_counter { return false; }
            if counter < other_counter { at_least_one_less = true; }
        }
        // Check for nodes in other not in self
        for (&node, &counter) in &other.entries {
            if !self.entries.contains_key(&node) && counter > 0 {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }

    pub fn dominates(&self, other: &VectorClock) -> bool {
        other.happens_before(self)
    }

    pub fn size(&self) -> usize { self.entries.len() }
}

/// Causal event
#[derive(Debug, Clone)]
pub struct CausalEvent {
    pub id: u64,
    pub node_id: u64,
    pub clock: VectorClock,
    pub timestamp: u64,
    pub payload_hash: u64,
    pub dependencies: Vec<u64>,
    pub delivered: bool,
    pub delivery_ts: Option<u64>,
}

impl CausalEvent {
    pub fn new(id: u64, node: u64, clock: VectorClock, ts: u64) -> Self {
        Self { id, node_id: node, clock, timestamp: ts, payload_hash: 0, dependencies: Vec::new(), delivered: false, delivery_ts: None }
    }

    pub fn add_dependency(&mut self, dep_id: u64) { self.dependencies.push(dep_id); }
}

/// Delivery order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryOrder {
    Fifo,
    Causal,
    Total,
}

/// Causal barrier
#[derive(Debug, Clone)]
pub struct CausalBarrier {
    pub id: u64,
    pub expected_nodes: Vec<u64>,
    pub received_clocks: BTreeMap<u64, VectorClock>,
    pub is_complete: bool,
    pub created_ts: u64,
    pub completed_ts: Option<u64>,
}

impl CausalBarrier {
    pub fn new(id: u64, expected: Vec<u64>, ts: u64) -> Self {
        Self { id, expected_nodes: expected, received_clocks: BTreeMap::new(), is_complete: false, created_ts: ts, completed_ts: None }
    }

    pub fn receive(&mut self, node: u64, clock: VectorClock, now: u64) {
        self.received_clocks.insert(node, clock);
        if self.expected_nodes.iter().all(|n| self.received_clocks.contains_key(n)) {
            self.is_complete = true;
            self.completed_ts = Some(now);
        }
    }

    pub fn merged_clock(&self) -> VectorClock {
        let mut merged = VectorClock::new();
        for clock in self.received_clocks.values() { merged.merge(clock); }
        merged
    }
}

/// Per-node causal state
#[derive(Debug, Clone)]
pub struct NodeCausalState {
    pub node_id: u64,
    pub clock: VectorClock,
    pub pending_events: Vec<u64>,
    pub delivered_events: Vec<u64>,
    pub last_delivered_clock: VectorClock,
    pub events_sent: u64,
    pub events_delivered: u64,
    pub events_buffered: u64,
    pub out_of_order_count: u64,
}

impl NodeCausalState {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id, clock: VectorClock::new(), pending_events: Vec::new(),
            delivered_events: Vec::new(), last_delivered_clock: VectorClock::new(),
            events_sent: 0, events_delivered: 0, events_buffered: 0,
            out_of_order_count: 0,
        }
    }

    pub fn send_event(&mut self) -> VectorClock {
        self.clock.increment(self.node_id);
        self.events_sent += 1;
        self.clock.clone()
    }

    pub fn can_deliver(&self, event_clock: &VectorClock, source_node: u64) -> bool {
        // For causal delivery: event's clock[source] == delivered_clock[source] + 1
        // and for all other j: event's clock[j] <= delivered_clock[j]
        let expected = self.last_delivered_clock.get(source_node) + 1;
        if event_clock.get(source_node) != expected { return false; }
        for (&node, &counter) in &event_clock.entries {
            if node != source_node && counter > self.last_delivered_clock.get(node) { return false; }
        }
        true
    }

    pub fn mark_delivered(&mut self, event_id: u64, event_clock: &VectorClock) {
        self.last_delivered_clock.merge(event_clock);
        self.delivered_events.push(event_id);
        self.events_delivered += 1;
        self.pending_events.retain(|&e| e != event_id);
    }
}

/// Causal ordering stats
#[derive(Debug, Clone, Default)]
pub struct CausalOrderStats {
    pub total_nodes: usize,
    pub total_events: usize,
    pub delivered_events: u64,
    pub pending_events: u64,
    pub out_of_order_total: u64,
    pub active_barriers: usize,
    pub completed_barriers: usize,
    pub max_clock_size: usize,
}

/// Coop causal ordering
pub struct CoopCausalOrder {
    nodes: BTreeMap<u64, NodeCausalState>,
    events: BTreeMap<u64, CausalEvent>,
    barriers: BTreeMap<u64, CausalBarrier>,
    stats: CausalOrderStats,
    next_id: u64,
}

impl CoopCausalOrder {
    pub fn new() -> Self {
        Self { nodes: BTreeMap::new(), events: BTreeMap::new(), barriers: BTreeMap::new(), stats: CausalOrderStats::default(), next_id: 1 }
    }

    pub fn add_node(&mut self, node_id: u64) {
        self.nodes.entry(node_id).or_insert_with(|| NodeCausalState::new(node_id));
    }

    pub fn send(&mut self, source_node: u64, ts: u64) -> Option<u64> {
        let clock = self.nodes.get_mut(&source_node)?.send_event();
        let id = self.next_id; self.next_id += 1;
        self.events.insert(id, CausalEvent::new(id, source_node, clock, ts));
        Some(id)
    }

    pub fn receive(&mut self, dest_node: u64, event_id: u64) -> bool {
        let (source, clock) = if let Some(e) = self.events.get(&event_id) { (e.node_id, e.clock.clone()) } else { return false; };
        if let Some(node) = self.nodes.get_mut(&dest_node) {
            if node.can_deliver(&clock, source) {
                node.mark_delivered(event_id, &clock);
                node.clock.merge(&clock);
                if let Some(e) = self.events.get_mut(&event_id) { e.delivered = true; }
                return true;
            } else {
                node.pending_events.push(event_id);
                node.events_buffered += 1;
            }
        }
        false
    }

    pub fn create_barrier(&mut self, expected: Vec<u64>, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.barriers.insert(id, CausalBarrier::new(id, expected, ts));
        id
    }

    pub fn barrier_receive(&mut self, barrier_id: u64, node: u64, clock: VectorClock, now: u64) {
        if let Some(b) = self.barriers.get_mut(&barrier_id) { b.receive(node, clock, now); }
    }

    pub fn try_deliver_pending(&mut self, dest_node: u64) -> Vec<u64> {
        let mut delivered = Vec::new();
        loop {
            let mut found = None;
            if let Some(node) = self.nodes.get(&dest_node) {
                for &eid in &node.pending_events {
                    if let Some(e) = self.events.get(&eid) {
                        if node.can_deliver(&e.clock, e.node_id) { found = Some((eid, e.clock.clone())); break; }
                    }
                }
            }
            if let Some((eid, clock)) = found {
                if let Some(node) = self.nodes.get_mut(&dest_node) {
                    node.mark_delivered(eid, &clock);
                    node.clock.merge(&clock);
                }
                if let Some(e) = self.events.get_mut(&eid) { e.delivered = true; }
                delivered.push(eid);
            } else { break; }
        }
        delivered
    }

    pub fn recompute(&mut self) {
        self.stats.total_nodes = self.nodes.len();
        self.stats.total_events = self.events.len();
        self.stats.delivered_events = self.events.values().filter(|e| e.delivered).count() as u64;
        self.stats.pending_events = self.nodes.values().map(|n| n.pending_events.len() as u64).sum();
        self.stats.out_of_order_total = self.nodes.values().map(|n| n.out_of_order_count).sum();
        self.stats.active_barriers = self.barriers.values().filter(|b| !b.is_complete).count();
        self.stats.completed_barriers = self.barriers.values().filter(|b| b.is_complete).count();
        self.stats.max_clock_size = self.nodes.values().map(|n| n.clock.size()).max().unwrap_or(0);
    }

    pub fn stats(&self) -> &CausalOrderStats { &self.stats }
}
