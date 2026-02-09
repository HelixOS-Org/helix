//! # Coop CRDT Engine
//!
//! Conflict-free replicated data types:
//! - G-Counter (grow-only counter)
//! - PN-Counter (positive-negative counter)
//! - G-Set (grow-only set)
//! - OR-Set (observed-remove set)
//! - LWW-Register (last-writer-wins register)
//! - Merge semantics and causality tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Replica identifier
pub type ReplicaId = u64;

/// G-Counter (grow-only)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GCounter {
    counts: BTreeMap<ReplicaId, u64>,
}

impl GCounter {
    pub fn new() -> Self { Self { counts: BTreeMap::new() } }

    #[inline(always)]
    pub fn increment(&mut self, replica: ReplicaId, amount: u64) {
        let entry = self.counts.entry(replica).or_insert(0);
        *entry += amount;
    }

    #[inline(always)]
    pub fn value(&self) -> u64 { self.counts.values().sum() }

    #[inline]
    pub fn merge(&mut self, other: &GCounter) {
        for (&replica, &count) in &other.counts {
            let entry = self.counts.entry(replica).or_insert(0);
            if count > *entry { *entry = count; }
        }
    }

    #[inline(always)]
    pub fn replica_count(&self) -> usize { self.counts.len() }
}

/// PN-Counter (supports decrement)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PNCounter {
    positive: GCounter,
    negative: GCounter,
}

impl PNCounter {
    pub fn new() -> Self { Self { positive: GCounter::new(), negative: GCounter::new() } }

    #[inline(always)]
    pub fn increment(&mut self, replica: ReplicaId, amount: u64) { self.positive.increment(replica, amount); }
    #[inline(always)]
    pub fn decrement(&mut self, replica: ReplicaId, amount: u64) { self.negative.increment(replica, amount); }

    #[inline(always)]
    pub fn value(&self) -> i64 { self.positive.value() as i64 - self.negative.value() as i64 }

    #[inline(always)]
    pub fn merge(&mut self, other: &PNCounter) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}

/// G-Set (grow-only set)
#[derive(Debug, Clone)]
pub struct GSet {
    elements: Vec<u64>,
}

impl GSet {
    pub fn new() -> Self { Self { elements: Vec::new() } }

    #[inline(always)]
    pub fn insert(&mut self, elem: u64) {
        if !self.elements.contains(&elem) { self.elements.push(elem); }
    }

    #[inline(always)]
    pub fn contains(&self, elem: u64) -> bool { self.elements.contains(&elem) }

    #[inline]
    pub fn merge(&mut self, other: &GSet) {
        for &e in &other.elements {
            if !self.elements.contains(&e) { self.elements.push(e); }
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize { self.elements.len() }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.elements.is_empty() }
}

/// Tagged element for OR-Set
#[derive(Debug, Clone)]
pub struct TaggedElement {
    pub value: u64,
    pub tag: u64,
    pub replica: ReplicaId,
}

/// OR-Set (observed-remove set)
#[derive(Debug, Clone)]
pub struct ORSet {
    elements: Vec<TaggedElement>,
    tombstones: Vec<u64>,
    next_tag: u64,
}

impl ORSet {
    pub fn new() -> Self { Self { elements: Vec::new(), tombstones: Vec::new(), next_tag: 1 } }

    #[inline]
    pub fn insert(&mut self, value: u64, replica: ReplicaId) -> u64 {
        let tag = self.next_tag;
        self.next_tag += 1;
        self.elements.push(TaggedElement { value, tag, replica });
        tag
    }

    #[inline]
    pub fn remove(&mut self, value: u64) {
        let tags: Vec<u64> = self.elements.iter().filter(|e| e.value == value).map(|e| e.tag).collect();
        for tag in &tags { self.tombstones.push(*tag); }
        self.elements.retain(|e| e.value != value);
    }

    #[inline(always)]
    pub fn contains(&self, value: u64) -> bool {
        self.elements.iter().any(|e| e.value == value && !self.tombstones.contains(&e.tag))
    }

    pub fn merge(&mut self, other: &ORSet) {
        for elem in &other.elements {
            if !self.tombstones.contains(&elem.tag) {
                if !self.elements.iter().any(|e| e.tag == elem.tag) {
                    self.elements.push(elem.clone());
                }
            }
        }
        for &tomb in &other.tombstones {
            if !self.tombstones.contains(&tomb) { self.tombstones.push(tomb); }
            self.elements.retain(|e| e.tag != tomb);
        }
        if other.next_tag > self.next_tag { self.next_tag = other.next_tag; }
    }

    #[inline]
    pub fn values(&self) -> Vec<u64> {
        let mut vals: Vec<u64> = self.elements.iter()
            .filter(|e| !self.tombstones.contains(&e.tag))
            .map(|e| e.value)
            .collect();
        vals.sort();
        vals.dedup();
        vals
    }

    #[inline(always)]
    pub fn len(&self) -> usize { self.values().len() }
}

/// LWW-Register (last-writer-wins)
#[derive(Debug, Clone)]
pub struct LWWRegister {
    pub value: u64,
    pub timestamp: u64,
    pub replica: ReplicaId,
}

impl LWWRegister {
    pub fn new(value: u64, ts: u64, replica: ReplicaId) -> Self {
        Self { value, timestamp: ts, replica }
    }

    #[inline]
    pub fn set(&mut self, value: u64, ts: u64, replica: ReplicaId) {
        if ts > self.timestamp || (ts == self.timestamp && replica > self.replica) {
            self.value = value;
            self.timestamp = ts;
            self.replica = replica;
        }
    }

    #[inline(always)]
    pub fn merge(&mut self, other: &LWWRegister) {
        self.set(other.value, other.timestamp, other.replica);
    }
}

/// CRDT type tag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrdtType {
    GCounter,
    PNCounter,
    GSet,
    ORSet,
    LWWRegister,
}

/// CRDT engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CrdtEngineStats {
    pub total_instances: usize,
    pub total_merges: u64,
    pub total_operations: u64,
    pub g_counters: usize,
    pub pn_counters: usize,
    pub g_sets: usize,
    pub or_sets: usize,
    pub lww_registers: usize,
}

/// Cooperative CRDT engine
pub struct CoopCrdtEngine {
    g_counters: BTreeMap<u64, GCounter>,
    pn_counters: BTreeMap<u64, PNCounter>,
    g_sets: BTreeMap<u64, GSet>,
    or_sets: BTreeMap<u64, ORSet>,
    lww_registers: BTreeMap<u64, LWWRegister>,
    stats: CrdtEngineStats,
}

impl CoopCrdtEngine {
    pub fn new() -> Self {
        Self {
            g_counters: BTreeMap::new(), pn_counters: BTreeMap::new(),
            g_sets: BTreeMap::new(), or_sets: BTreeMap::new(),
            lww_registers: BTreeMap::new(),
            stats: CrdtEngineStats::default(),
        }
    }

    #[inline(always)]
    pub fn create_g_counter(&mut self, id: u64) { self.g_counters.insert(id, GCounter::new()); }
    #[inline(always)]
    pub fn create_pn_counter(&mut self, id: u64) { self.pn_counters.insert(id, PNCounter::new()); }
    #[inline(always)]
    pub fn create_g_set(&mut self, id: u64) { self.g_sets.insert(id, GSet::new()); }
    #[inline(always)]
    pub fn create_or_set(&mut self, id: u64) { self.or_sets.insert(id, ORSet::new()); }
    #[inline(always)]
    pub fn create_lww_register(&mut self, id: u64, initial: u64, ts: u64, replica: ReplicaId) {
        self.lww_registers.insert(id, LWWRegister::new(initial, ts, replica));
    }

    #[inline(always)]
    pub fn g_counter_inc(&mut self, id: u64, replica: ReplicaId, amount: u64) {
        if let Some(c) = self.g_counters.get_mut(&id) { c.increment(replica, amount); self.stats.total_operations += 1; }
    }

    #[inline(always)]
    pub fn pn_counter_inc(&mut self, id: u64, replica: ReplicaId, amount: u64) {
        if let Some(c) = self.pn_counters.get_mut(&id) { c.increment(replica, amount); self.stats.total_operations += 1; }
    }

    #[inline(always)]
    pub fn pn_counter_dec(&mut self, id: u64, replica: ReplicaId, amount: u64) {
        if let Some(c) = self.pn_counters.get_mut(&id) { c.decrement(replica, amount); self.stats.total_operations += 1; }
    }

    #[inline(always)]
    pub fn or_set_insert(&mut self, id: u64, value: u64, replica: ReplicaId) {
        if let Some(s) = self.or_sets.get_mut(&id) { s.insert(value, replica); self.stats.total_operations += 1; }
    }

    #[inline(always)]
    pub fn or_set_remove(&mut self, id: u64, value: u64) {
        if let Some(s) = self.or_sets.get_mut(&id) { s.remove(value); self.stats.total_operations += 1; }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.g_counters = self.g_counters.len();
        self.stats.pn_counters = self.pn_counters.len();
        self.stats.g_sets = self.g_sets.len();
        self.stats.or_sets = self.or_sets.len();
        self.stats.lww_registers = self.lww_registers.len();
        self.stats.total_instances = self.stats.g_counters + self.stats.pn_counters + self.stats.g_sets + self.stats.or_sets + self.stats.lww_registers;
    }

    #[inline(always)]
    pub fn stats(&self) -> &CrdtEngineStats { &self.stats }
}
