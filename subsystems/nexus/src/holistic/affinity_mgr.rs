// SPDX-License-Identifier: GPL-2.0
//! Holistic affinity_mgr — CPU/memory affinity management for processes and IRQs.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Affinity scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityScope {
    Process,
    Thread,
    Irq,
    IoChannel,
    Timer,
    Workqueue,
}

/// Affinity policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityPolicy {
    Strict,
    Preferred,
    SpreadAcross,
    PackTogether,
    NumaLocal,
    AutoBalance,
}

/// CPU mask (bitmask for up to 256 CPUs)
#[derive(Debug, Clone)]
pub struct CpuMask {
    pub bits: [u64; 4],
}

impl CpuMask {
    pub fn empty() -> Self { Self { bits: [0; 4] } }

    pub fn all(nr_cpus: u32) -> Self {
        let mut m = Self::empty();
        for i in 0..nr_cpus.min(256) { m.set(i); }
        m
    }

    pub fn set(&mut self, cpu: u32) {
        if cpu < 256 { self.bits[cpu as usize / 64] |= 1 << (cpu % 64); }
    }

    pub fn clear(&mut self, cpu: u32) {
        if cpu < 256 { self.bits[cpu as usize / 64] &= !(1 << (cpu % 64)); }
    }

    pub fn test(&self, cpu: u32) -> bool {
        if cpu >= 256 { return false; }
        (self.bits[cpu as usize / 64] >> (cpu % 64)) & 1 != 0
    }

    pub fn count(&self) -> u32 {
        self.bits.iter().map(|b| b.count_ones()).sum()
    }

    pub fn and(&self, other: &CpuMask) -> CpuMask {
        let mut r = CpuMask::empty();
        for i in 0..4 { r.bits[i] = self.bits[i] & other.bits[i]; }
        r
    }

    pub fn or(&self, other: &CpuMask) -> CpuMask {
        let mut r = CpuMask::empty();
        for i in 0..4 { r.bits[i] = self.bits[i] | other.bits[i]; }
        r
    }

    pub fn is_empty(&self) -> bool { self.bits.iter().all(|&b| b == 0) }

    pub fn first_set(&self) -> Option<u32> {
        for (i, &word) in self.bits.iter().enumerate() {
            if word != 0 { return Some(i as u32 * 64 + word.trailing_zeros()); }
        }
        None
    }

    pub fn iter_set(&self) -> Vec<u32> {
        let mut v = Vec::new();
        for i in 0..256u32 { if self.test(i) { v.push(i); } }
        v
    }
}

/// NUMA node mask
#[derive(Debug, Clone)]
pub struct NodeMask {
    pub bits: u64,
}

impl NodeMask {
    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn set(&mut self, node: u32) { if node < 64 { self.bits |= 1 << node; } }
    pub fn test(&self, node: u32) -> bool { if node >= 64 { false } else { (self.bits >> node) & 1 != 0 } }
    pub fn count(&self) -> u32 { self.bits.count_ones() }
}

/// Affinity binding for a specific entity
#[derive(Debug, Clone)]
pub struct AffinityBinding {
    pub entity_id: u64,
    pub scope: AffinityScope,
    pub policy: AffinityPolicy,
    pub cpu_mask: CpuMask,
    pub node_mask: NodeMask,
    pub effective_cpu: Option<u32>,
    pub migrations: u64,
    pub last_migration: u64,
    pub created_at: u64,
}

impl AffinityBinding {
    pub fn new(id: u64, scope: AffinityScope, policy: AffinityPolicy, mask: CpuMask, now: u64) -> Self {
        Self {
            entity_id: id, scope, policy, cpu_mask: mask,
            node_mask: NodeMask::empty(), effective_cpu: None,
            migrations: 0, last_migration: 0, created_at: now,
        }
    }

    pub fn migrate_to(&mut self, cpu: u32, now: u64) {
        self.effective_cpu = Some(cpu);
        self.migrations += 1;
        self.last_migration = now;
    }

    pub fn is_allowed(&self, cpu: u32) -> bool { self.cpu_mask.test(cpu) }
}

/// Migration event
#[derive(Debug, Clone)]
pub struct MigrationEvent {
    pub entity_id: u64,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub reason: MigrationReason,
    pub timestamp: u64,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    LoadBalance,
    AffinityChange,
    NumaRebalance,
    CpuHotplug,
    UserRequest,
    ThermalThrottle,
    PowerSave,
}

/// Affinity manager stats
#[derive(Debug, Clone)]
pub struct AffinityMgrStats {
    pub total_bindings: u32,
    pub total_migrations: u64,
    pub strict_bindings: u32,
    pub preferred_bindings: u32,
    pub avg_migrations: f64,
    pub bindings_by_scope: BTreeMap<u8, u32>,
}

/// Main affinity manager
pub struct HolisticAffinityMgr {
    bindings: BTreeMap<u64, AffinityBinding>,
    migrations: Vec<MigrationEvent>,
    max_migration_log: usize,
}

impl HolisticAffinityMgr {
    pub fn new() -> Self {
        Self { bindings: BTreeMap::new(), migrations: Vec::new(), max_migration_log: 8192 }
    }

    pub fn bind(&mut self, id: u64, scope: AffinityScope, policy: AffinityPolicy, mask: CpuMask, now: u64) {
        self.bindings.insert(id, AffinityBinding::new(id, scope, policy, mask, now));
    }

    pub fn unbind(&mut self, id: u64) -> bool { self.bindings.remove(&id).is_some() }

    pub fn migrate(&mut self, id: u64, to_cpu: u32, reason: MigrationReason, now: u64) -> bool {
        let binding = match self.bindings.get_mut(&id) {
            Some(b) => b, None => return false,
        };
        if binding.policy == AffinityPolicy::Strict && !binding.is_allowed(to_cpu) {
            return false;
        }
        let from = binding.effective_cpu.unwrap_or(0);
        binding.migrate_to(to_cpu, now);
        if self.migrations.len() >= self.max_migration_log {
            self.migrations.drain(..self.max_migration_log / 4);
        }
        self.migrations.push(MigrationEvent { entity_id: id, from_cpu: from, to_cpu, reason, timestamp: now });
        true
    }

    pub fn get_binding(&self, id: u64) -> Option<&AffinityBinding> { self.bindings.get(&id) }

    pub fn bindings_on_cpu(&self, cpu: u32) -> Vec<u64> {
        self.bindings.iter()
            .filter(|(_, b)| b.effective_cpu == Some(cpu))
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn stats(&self) -> AffinityMgrStats {
        let strict = self.bindings.values().filter(|b| b.policy == AffinityPolicy::Strict).count() as u32;
        let preferred = self.bindings.values().filter(|b| b.policy == AffinityPolicy::Preferred).count() as u32;
        let total_mig: u64 = self.bindings.values().map(|b| b.migrations).sum();
        let avg = if self.bindings.is_empty() { 0.0 } else { total_mig as f64 / self.bindings.len() as f64 };
        let mut by_scope = BTreeMap::new();
        for b in self.bindings.values() {
            *by_scope.entry(b.scope as u8).or_insert(0u32) += 1;
        }
        AffinityMgrStats {
            total_bindings: self.bindings.len() as u32,
            total_migrations: total_mig,
            strict_bindings: strict, preferred_bindings: preferred,
            avg_migrations: avg, bindings_by_scope: by_scope,
        }
    }
}
