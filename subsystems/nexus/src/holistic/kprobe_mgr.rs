// SPDX-License-Identifier: GPL-2.0
//! Holistic kprobe_mgr — kernel probe management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    Kprobe,
    Kretprobe,
    Tracepoint,
    Rawtracepoint,
}

/// Probe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeState {
    Registered,
    Enabled,
    Disabled,
    Unregistered,
    Failed,
}

/// Probe handler hit
#[derive(Debug, Clone)]
pub struct ProbeHit {
    pub cpu: u32,
    pub pid: u64,
    pub timestamp: u64,
    pub regs_hash: u64,
    pub ret_val: i64,
}

/// Kernel probe
#[derive(Debug)]
pub struct KprobeEntry {
    pub id: u64,
    pub probe_type: ProbeType,
    pub state: ProbeState,
    pub address: u64,
    pub symbol_hash: u64,
    pub offset: u32,
    pub hit_count: u64,
    pub miss_count: u64,
    pub recent_hits: Vec<ProbeHit>,
    pub max_recent: usize,
    pub overhead_ns: u64,
}

impl KprobeEntry {
    pub fn new(id: u64, ptype: ProbeType, addr: u64) -> Self {
        Self { id, probe_type: ptype, state: ProbeState::Registered, address: addr, symbol_hash: addr, offset: 0, hit_count: 0, miss_count: 0, recent_hits: Vec::new(), max_recent: 64, overhead_ns: 0 }
    }

    pub fn enable(&mut self) { self.state = ProbeState::Enabled; }
    pub fn disable(&mut self) { self.state = ProbeState::Disabled; }

    pub fn hit(&mut self, cpu: u32, pid: u64, now: u64) {
        self.hit_count += 1;
        if self.recent_hits.len() >= self.max_recent { self.recent_hits.remove(0); }
        self.recent_hits.push(ProbeHit { cpu, pid, timestamp: now, regs_hash: 0, ret_val: 0 });
    }

    pub fn is_active(&self) -> bool { self.state == ProbeState::Enabled }
}

/// Stats
#[derive(Debug, Clone)]
pub struct KprobeMgrStats {
    pub total_probes: u32,
    pub enabled_probes: u32,
    pub total_hits: u64,
    pub kprobes: u32,
    pub kretprobes: u32,
    pub tracepoints: u32,
    pub avg_overhead_ns: u64,
}

/// Main kprobe manager
pub struct HolisticKprobeMgr {
    probes: BTreeMap<u64, KprobeEntry>,
    next_id: u64,
}

impl HolisticKprobeMgr {
    pub fn new() -> Self { Self { probes: BTreeMap::new(), next_id: 1 } }

    pub fn register(&mut self, ptype: ProbeType, addr: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.probes.insert(id, KprobeEntry::new(id, ptype, addr));
        id
    }

    pub fn enable(&mut self, id: u64) { if let Some(p) = self.probes.get_mut(&id) { p.enable(); } }
    pub fn disable(&mut self, id: u64) { if let Some(p) = self.probes.get_mut(&id) { p.disable(); } }

    pub fn hit(&mut self, id: u64, cpu: u32, pid: u64, now: u64) {
        if let Some(p) = self.probes.get_mut(&id) { if p.is_active() { p.hit(cpu, pid, now); } }
    }

    pub fn stats(&self) -> KprobeMgrStats {
        let enabled = self.probes.values().filter(|p| p.is_active()).count() as u32;
        let hits: u64 = self.probes.values().map(|p| p.hit_count).sum();
        let kp = self.probes.values().filter(|p| p.probe_type == ProbeType::Kprobe).count() as u32;
        let kr = self.probes.values().filter(|p| p.probe_type == ProbeType::Kretprobe).count() as u32;
        let tp = self.probes.values().filter(|p| p.probe_type == ProbeType::Tracepoint).count() as u32;
        let overheads: Vec<u64> = self.probes.values().filter(|p| p.overhead_ns > 0).map(|p| p.overhead_ns).collect();
        let avg = if overheads.is_empty() { 0 } else { overheads.iter().sum::<u64>() / overheads.len() as u64 };
        KprobeMgrStats { total_probes: self.probes.len() as u32, enabled_probes: enabled, total_hits: hits, kprobes: kp, kretprobes: kr, tracepoints: tp, avg_overhead_ns: avg }
    }
}
