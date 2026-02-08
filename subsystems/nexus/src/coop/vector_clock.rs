//! # Coop Vector Clock
//!
//! Vector clock implementation for cooperative causal ordering:
//! - Per-node logical timestamp vectors
//! - Causal ordering comparison (happens-before)
//! - Concurrent event detection
//! - Vector clock compression for large clusters
//! - Event history with causal chains
//! - Version vectors for optimistic replication

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Causal ordering relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalOrder {
    HappensBefore,
    HappensAfter,
    Concurrent,
    Equal,
}

/// A vector clock
#[derive(Debug, Clone)]
pub struct VectorClock {
    pub entries: BTreeMap<u64, u64>,
}

impl VectorClock {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn increment(&mut self, node_id: u64) -> u64 {
        let counter = self.entries.entry(node_id).or_insert(0);
        *counter += 1;
        *counter
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

    pub fn compare(&self, other: &VectorClock) -> CausalOrder {
        let all_keys: Vec<u64> = {
            let mut keys: Vec<u64> = self.entries.keys().chain(other.entries.keys()).copied().collect();
            keys.sort_unstable();
            keys.dedup();
            keys
        };

        let mut self_le = true;
        let mut other_le = true;

        for key in &all_keys {
            let a = self.get(*key);
            let b = other.get(*key);
            if a > b { other_le = false; }
            if b > a { self_le = false; }
        }

        match (self_le, other_le) {
            (true, true) => CausalOrder::Equal,
            (true, false) => CausalOrder::HappensBefore,
            (false, true) => CausalOrder::HappensAfter,
            (false, false) => CausalOrder::Concurrent,
        }
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        self.compare(other) == CausalOrder::HappensBefore
    }

    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        self.compare(other) == CausalOrder::Concurrent
    }

    pub fn dimension(&self) -> usize { self.entries.len() }

    pub fn max_component(&self) -> u64 {
        self.entries.values().copied().max().unwrap_or(0)
    }

    pub fn sum(&self) -> u64 { self.entries.values().sum() }

    /// Prune entries with zero counters
    pub fn compact(&mut self) {
        self.entries.retain(|_, v| *v > 0);
    }

    /// Dominates if all components >= other
    pub fn dominates(&self, other: &VectorClock) -> bool {
        for (&node, &counter) in &other.entries {
            if self.get(node) < counter { return false; }
        }
        true
    }
}

/// Version vector for optimistic replication
#[derive(Debug, Clone)]
pub struct VersionVector {
    pub clock: VectorClock,
    pub node_id: u64,
}

impl VersionVector {
    pub fn new(node_id: u64) -> Self { Self { clock: VectorClock::new(), node_id } }

    pub fn update(&mut self) -> u64 { self.clock.increment(self.node_id) }

    pub fn merge_remote(&mut self, remote: &VectorClock) {
        self.clock.merge(remote);
        self.clock.increment(self.node_id);
    }

    pub fn can_apply(&self, remote: &VectorClock) -> bool {
        // Remote is applicable if it causally follows or is concurrent
        match self.clock.compare(remote) {
            CausalOrder::HappensBefore | CausalOrder::Concurrent | CausalOrder::Equal => true,
            CausalOrder::HappensAfter => false,
        }
    }
}

/// Causal event with vector clock
#[derive(Debug, Clone)]
pub struct CausalEvent {
    pub event_id: u64,
    pub node_id: u64,
    pub clock: VectorClock,
    pub timestamp_ns: u64,
    pub event_type: u32,
}

/// Causal history tracker
#[derive(Debug, Clone)]
pub struct CausalHistory {
    events: Vec<CausalEvent>,
    max_events: usize,
}

impl CausalHistory {
    pub fn new(max: usize) -> Self { Self { events: Vec::new(), max_events: max } }

    pub fn record(&mut self, event: CausalEvent) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    pub fn find_concurrent_pairs(&self) -> Vec<(u64, u64)> {
        let mut pairs = Vec::new();
        for i in 0..self.events.len() {
            for j in (i + 1)..self.events.len() {
                if self.events[i].clock.is_concurrent(&self.events[j].clock) {
                    pairs.push((self.events[i].event_id, self.events[j].event_id));
                }
            }
        }
        pairs
    }

    pub fn causal_chain(&self, event_id: u64) -> Vec<u64> {
        let target = match self.events.iter().find(|e| e.event_id == event_id) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let mut chain: Vec<u64> = self.events.iter()
            .filter(|e| e.event_id != event_id && e.clock.happens_before(&target.clock))
            .map(|e| e.event_id)
            .collect();
        chain.sort_unstable();
        chain
    }

    pub fn latest_per_node(&self) -> BTreeMap<u64, u64> {
        let mut latest: BTreeMap<u64, u64> = BTreeMap::new();
        for event in &self.events {
            let entry = latest.entry(event.node_id).or_insert(0);
            if event.event_id > *entry { *entry = event.event_id; }
        }
        latest
    }

    pub fn event_count(&self) -> usize { self.events.len() }
}

/// Vector clock stats
#[derive(Debug, Clone, Default)]
pub struct VectorClockStats {
    pub total_clocks: usize,
    pub total_events: usize,
    pub concurrent_pairs: usize,
    pub avg_dimension: f64,
    pub max_dimension: usize,
    pub total_increments: u64,
    pub total_merges: u64,
}

/// Cooperative vector clock manager
pub struct CoopVectorClock {
    clocks: BTreeMap<u64, VectorClock>,
    versions: BTreeMap<u64, VersionVector>,
    history: CausalHistory,
    next_event_id: u64,
    total_increments: u64,
    total_merges: u64,
    stats: VectorClockStats,
}

impl CoopVectorClock {
    pub fn new(max_history: usize) -> Self {
        Self {
            clocks: BTreeMap::new(), versions: BTreeMap::new(),
            history: CausalHistory::new(max_history), next_event_id: 1,
            total_increments: 0, total_merges: 0,
            stats: VectorClockStats::default(),
        }
    }

    pub fn create_clock(&mut self, node_id: u64) {
        self.clocks.insert(node_id, VectorClock::new());
        self.versions.insert(node_id, VersionVector::new(node_id));
    }

    pub fn increment(&mut self, node_id: u64, ts: u64, event_type: u32) -> Option<u64> {
        let clock = self.clocks.get_mut(&node_id)?;
        clock.increment(node_id);
        self.total_increments += 1;
        let eid = self.next_event_id; self.next_event_id += 1;
        self.history.record(CausalEvent {
            event_id: eid, node_id, clock: clock.clone(), timestamp_ns: ts, event_type,
        });
        Some(eid)
    }

    pub fn merge_clocks(&mut self, a: u64, b: u64) {
        let b_clock = match self.clocks.get(&b) { Some(c) => c.clone(), None => return };
        if let Some(a_clock) = self.clocks.get_mut(&a) {
            a_clock.merge(&b_clock);
            self.total_merges += 1;
        }
    }

    pub fn compare(&self, a: u64, b: u64) -> CausalOrder {
        match (self.clocks.get(&a), self.clocks.get(&b)) {
            (Some(ca), Some(cb)) => ca.compare(cb),
            _ => CausalOrder::Concurrent,
        }
    }

    pub fn clock(&self, node_id: u64) -> Option<&VectorClock> { self.clocks.get(&node_id) }
    pub fn history(&self) -> &CausalHistory { &self.history }

    pub fn recompute(&mut self) {
        self.stats.total_clocks = self.clocks.len();
        self.stats.total_events = self.history.event_count();
        self.stats.concurrent_pairs = self.history.find_concurrent_pairs().len();
        if !self.clocks.is_empty() {
            let dims: Vec<usize> = self.clocks.values().map(|c| c.dimension()).collect();
            self.stats.avg_dimension = dims.iter().sum::<usize>() as f64 / dims.len() as f64;
            self.stats.max_dimension = dims.iter().copied().max().unwrap_or(0);
        }
        self.stats.total_increments = self.total_increments;
        self.stats.total_merges = self.total_merges;
    }

    pub fn stats(&self) -> &VectorClockStats { &self.stats }
}
