// SPDX-License-Identifier: GPL-2.0
//! Holistic kprobes — kernel probe instrumentation management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    Kprobe,
    Kretprobe,
    Tracepoint,
    Uprobe,
    Uretprobe,
    Fentry,
    Fexit,
}

/// Probe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeState {
    Registered,
    Active,
    Disabled,
    Missed,
    Error,
}

/// Kernel probe
#[derive(Debug)]
pub struct KernelProbe {
    pub id: u64,
    pub probe_type: ProbeType,
    pub state: ProbeState,
    pub symbol_hash: u64,
    pub offset: u64,
    pub addr: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub total_ns: u64,
    pub max_handler_ns: u64,
    pub attached_prog: u64,
}

impl KernelProbe {
    pub fn new(id: u64, ptype: ProbeType, sym_hash: u64, addr: u64) -> Self {
        Self { id, probe_type: ptype, state: ProbeState::Registered, symbol_hash: sym_hash, offset: 0, addr, hit_count: 0, miss_count: 0, total_ns: 0, max_handler_ns: 0, attached_prog: 0 }
    }

    pub fn hit(&mut self, handler_ns: u64) {
        self.hit_count += 1;
        self.total_ns += handler_ns;
        if handler_ns > self.max_handler_ns { self.max_handler_ns = handler_ns; }
        self.state = ProbeState::Active;
    }

    pub fn miss(&mut self) { self.miss_count += 1; self.state = ProbeState::Missed; }

    pub fn disable(&mut self) { self.state = ProbeState::Disabled; }
    pub fn enable(&mut self) { self.state = ProbeState::Registered; }

    pub fn avg_handler_ns(&self) -> u64 {
        if self.hit_count == 0 { 0 } else { self.total_ns / self.hit_count }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct KprobesStats {
    pub total_probes: u32,
    pub active: u32,
    pub total_hits: u64,
    pub total_misses: u64,
    pub avg_handler_ns: u64,
}

/// Main holistic kprobes manager
pub struct HolisticKprobes {
    probes: BTreeMap<u64, KernelProbe>,
    next_id: u64,
}

impl HolisticKprobes {
    pub fn new() -> Self { Self { probes: BTreeMap::new(), next_id: 1 } }

    pub fn register(&mut self, ptype: ProbeType, sym_hash: u64, addr: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.probes.insert(id, KernelProbe::new(id, ptype, sym_hash, addr));
        id
    }

    pub fn hit(&mut self, id: u64, ns: u64) {
        if let Some(p) = self.probes.get_mut(&id) { p.hit(ns); }
    }

    pub fn disable(&mut self, id: u64) {
        if let Some(p) = self.probes.get_mut(&id) { p.disable(); }
    }

    pub fn unregister(&mut self, id: u64) { self.probes.remove(&id); }

    pub fn stats(&self) -> KprobesStats {
        let active = self.probes.values().filter(|p| p.state == ProbeState::Active).count() as u32;
        let hits: u64 = self.probes.values().map(|p| p.hit_count).sum();
        let misses: u64 = self.probes.values().map(|p| p.miss_count).sum();
        let ns: u64 = self.probes.values().map(|p| p.total_ns).sum();
        let avg = if hits == 0 { 0 } else { ns / hits };
        KprobesStats { total_probes: self.probes.len() as u32, active, total_hits: hits, total_misses: misses, avg_handler_ns: avg }
    }
}
