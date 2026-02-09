// SPDX-License-Identifier: GPL-2.0
//! Bridge rseq_bridge — restartable sequences (rseq) bridge for per-CPU ops.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Rseq flags
#[derive(Debug, Clone, Copy)]
pub struct RseqFlags(pub u32);

impl RseqFlags {
    pub const UNREGISTER: Self = Self(0x01);
    pub const FLAG_ALIGN: Self = Self(0x02);

    #[inline(always)]
    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// Critical section abort reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RseqAbortReason {
    Signal,
    Preemption,
    Migration,
    Timeout,
    Unknown,
}

/// An rseq registration
#[derive(Debug)]
pub struct RseqRegistration {
    pub pid: u32,
    pub tid: u32,
    pub rseq_addr: u64,
    pub rseq_len: u32,
    pub signature: u32,
    pub cpu_id: u32,
    pub node_id: u32,
    pub mm_cid: u32,
    pub flags: RseqFlags,
    pub registered: bool,
    pub created: u64,
}

impl RseqRegistration {
    pub fn new(pid: u32, tid: u32, addr: u64, sig: u32, now: u64) -> Self {
        Self {
            pid, tid, rseq_addr: addr,
            rseq_len: 32, signature: sig,
            cpu_id: 0, node_id: 0, mm_cid: 0,
            flags: RseqFlags(0),
            registered: true, created: now,
        }
    }

    #[inline]
    pub fn update_cpu(&mut self, cpu_id: u32, node_id: u32, mm_cid: u32) {
        self.cpu_id = cpu_id;
        self.node_id = node_id;
        self.mm_cid = mm_cid;
    }
}

/// A critical section descriptor
#[derive(Debug, Clone)]
pub struct CriticalSection {
    pub start_ip: u64,
    pub post_commit_offset: u32,
    pub abort_ip: u64,
    pub execution_count: u64,
    pub abort_count: u64,
    pub avg_cycles: u64,
}

impl CriticalSection {
    pub fn new(start: u64, post_commit: u32, abort: u64) -> Self {
        Self {
            start_ip: start, post_commit_offset: post_commit,
            abort_ip: abort,
            execution_count: 0, abort_count: 0,
            avg_cycles: 0,
        }
    }

    #[inline(always)]
    pub fn abort_rate(&self) -> f64 {
        if self.execution_count == 0 { return 0.0; }
        self.abort_count as f64 / self.execution_count as f64
    }

    #[inline(always)]
    pub fn is_problematic(&self) -> bool {
        self.abort_rate() > 0.1 && self.execution_count > 100
    }

    #[inline(always)]
    pub fn end_ip(&self) -> u64 {
        self.start_ip + self.post_commit_offset as u64
    }
}

/// Rseq abort event
#[derive(Debug, Clone)]
pub struct RseqAbort {
    pub tid: u32,
    pub cs_start_ip: u64,
    pub reason: RseqAbortReason,
    pub cpu_from: u32,
    pub cpu_to: Option<u32>,
    pub timestamp: u64,
}

/// Per-CPU rseq stats
#[derive(Debug)]
#[repr(align(64))]
pub struct CpuRseqState {
    pub cpu_id: u32,
    pub active_registrations: u32,
    pub total_executions: u64,
    pub total_aborts: u64,
    pub migration_aborts: u64,
    pub preemption_aborts: u64,
}

impl CpuRseqState {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id, active_registrations: 0,
            total_executions: 0, total_aborts: 0,
            migration_aborts: 0, preemption_aborts: 0,
        }
    }

    #[inline(always)]
    pub fn abort_rate(&self) -> f64 {
        if self.total_executions == 0 { return 0.0; }
        self.total_aborts as f64 / self.total_executions as f64
    }
}

/// Rseq bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RseqBridgeStats {
    pub registered_threads: u32,
    pub total_registrations: u64,
    pub total_unregistrations: u64,
    pub total_aborts: u64,
    pub total_migrations: u64,
    pub critical_sections_tracked: u32,
    pub avg_abort_rate: f64,
}

/// Main rseq bridge
#[repr(align(64))]
pub struct BridgeRseq {
    registrations: BTreeMap<u32, RseqRegistration>,
    critical_sections: BTreeMap<u64, CriticalSection>,
    cpu_states: BTreeMap<u32, CpuRseqState>,
    aborts: VecDeque<RseqAbort>,
    max_aborts: usize,
    stats: RseqBridgeStats,
}

impl BridgeRseq {
    pub fn new() -> Self {
        Self {
            registrations: BTreeMap::new(),
            critical_sections: BTreeMap::new(),
            cpu_states: BTreeMap::new(),
            aborts: VecDeque::new(),
            max_aborts: 4096,
            stats: RseqBridgeStats {
                registered_threads: 0, total_registrations: 0,
                total_unregistrations: 0, total_aborts: 0,
                total_migrations: 0, critical_sections_tracked: 0,
                avg_abort_rate: 0.0,
            },
        }
    }

    #[inline]
    pub fn register(&mut self, reg: RseqRegistration) {
        self.stats.total_registrations += 1;
        self.stats.registered_threads += 1;
        if let Some(cs) = self.cpu_states.get_mut(&reg.cpu_id) {
            cs.active_registrations += 1;
        }
        self.registrations.insert(reg.tid, reg);
    }

    #[inline]
    pub fn unregister(&mut self, tid: u32) -> bool {
        if let Some(reg) = self.registrations.remove(&tid) {
            self.stats.total_unregistrations += 1;
            if self.stats.registered_threads > 0 { self.stats.registered_threads -= 1; }
            if let Some(cs) = self.cpu_states.get_mut(&reg.cpu_id) {
                if cs.active_registrations > 0 { cs.active_registrations -= 1; }
            }
            true
        } else { false }
    }

    #[inline(always)]
    pub fn track_cs(&mut self, cs: CriticalSection) {
        self.stats.critical_sections_tracked += 1;
        self.critical_sections.insert(cs.start_ip, cs);
    }

    pub fn record_abort(&mut self, abort: RseqAbort) {
        self.stats.total_aborts += 1;
        if abort.reason == RseqAbortReason::Migration {
            self.stats.total_migrations += 1;
        }

        if let Some(cs) = self.critical_sections.get_mut(&abort.cs_start_ip) {
            cs.abort_count += 1;
        }

        if let Some(cpu) = self.cpu_states.get_mut(&abort.cpu_from) {
            cpu.total_aborts += 1;
            match abort.reason {
                RseqAbortReason::Migration => cpu.migration_aborts += 1,
                RseqAbortReason::Preemption => cpu.preemption_aborts += 1,
                _ => {}
            }
        }

        if self.aborts.len() >= self.max_aborts { self.aborts.pop_front(); }
        self.aborts.push_back(abort);
    }

    #[inline]
    pub fn record_execution(&mut self, cs_start: u64, cpu_id: u32, cycles: u64) {
        if let Some(cs) = self.critical_sections.get_mut(&cs_start) {
            cs.execution_count += 1;
            let n = cs.execution_count;
            cs.avg_cycles = ((cs.avg_cycles * (n - 1)) + cycles) / n;
        }
        if let Some(cpu) = self.cpu_states.get_mut(&cpu_id) {
            cpu.total_executions += 1;
        }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu_id: u32) {
        self.cpu_states.insert(cpu_id, CpuRseqState::new(cpu_id));
    }

    #[inline]
    pub fn problematic_cs(&self) -> Vec<(u64, f64)> {
        self.critical_sections.iter()
            .filter(|(_, cs)| cs.is_problematic())
            .map(|(&ip, cs)| (ip, cs.abort_rate()))
            .collect()
    }

    #[inline]
    pub fn worst_cpus(&self, n: usize) -> Vec<(u32, f64)> {
        let mut v: Vec<_> = self.cpu_states.iter()
            .map(|(&cpu, s)| (cpu, s.abort_rate()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    #[inline(always)]
    pub fn stats(&self) -> &RseqBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from rseq_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct RseqV2Flags(pub u32);

impl RseqV2Flags {
    pub const CS_FLAG_NO_RESTART_ON_PREEMPT: u32 = 1 << 0;
    pub const CS_FLAG_NO_RESTART_ON_SIGNAL: u32 = 1 << 1;
    pub const CS_FLAG_NO_RESTART_ON_MIGRATE: u32 = 1 << 2;
    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Critical section descriptor
#[derive(Debug, Clone)]
pub struct RseqV2CriticalSection {
    pub start_ip: u64,
    pub post_commit_offset: u32,
    pub abort_ip: u64,
    pub flags: RseqV2Flags,
}

/// Per-thread RSEQ v2 state
#[derive(Debug)]
#[repr(align(64))]
pub struct RseqV2ThreadState {
    pub tid: u64,
    pub cpu_id: u32,
    pub cpu_id_start: u32,
    pub registered: bool,
    pub rseq_cs: Option<RseqV2CriticalSection>,
    pub abort_count: u64,
    pub migration_count: u64,
    pub preempt_count: u64,
    pub signal_count: u64,
    pub success_count: u64,
}

impl RseqV2ThreadState {
    pub fn new(tid: u64) -> Self {
        Self { tid, cpu_id: 0, cpu_id_start: 0, registered: false, rseq_cs: None, abort_count: 0, migration_count: 0, preempt_count: 0, signal_count: 0, success_count: 0 }
    }

    #[inline(always)]
    pub fn register(&mut self, cpu: u32) { self.registered = true; self.cpu_id = cpu; self.cpu_id_start = cpu; }
    #[inline(always)]
    pub fn record_abort(&mut self) { self.abort_count += 1; }
    #[inline(always)]
    pub fn record_success(&mut self) { self.success_count += 1; }
    #[inline(always)]
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.abort_count;
        if total == 0 { 1.0 } else { self.success_count as f64 / total as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RseqV2BridgeStats {
    pub registered_threads: u32,
    pub total_aborts: u64,
    pub total_successes: u64,
    pub total_migrations: u64,
    pub avg_success_rate: f64,
}

/// Main rseq v2 bridge
#[repr(align(64))]
pub struct BridgeRseqV2 {
    threads: BTreeMap<u64, RseqV2ThreadState>,
}

impl BridgeRseqV2 {
    pub fn new() -> Self { Self { threads: BTreeMap::new() } }

    #[inline]
    pub fn register(&mut self, tid: u64, cpu: u32) {
        let mut state = RseqV2ThreadState::new(tid);
        state.register(cpu);
        self.threads.insert(tid, state);
    }

    #[inline(always)]
    pub fn unregister(&mut self, tid: u64) { self.threads.remove(&tid); }

    #[inline(always)]
    pub fn notify_preempt(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { if t.rseq_cs.is_some() { t.preempt_count += 1; t.record_abort(); } }
    }

    #[inline(always)]
    pub fn notify_migrate(&mut self, tid: u64, new_cpu: u32) {
        if let Some(t) = self.threads.get_mut(&tid) { t.migration_count += 1; t.cpu_id = new_cpu; if t.rseq_cs.is_some() { t.record_abort(); } }
    }

    #[inline]
    pub fn stats(&self) -> RseqV2BridgeStats {
        let reg = self.threads.values().filter(|t| t.registered).count() as u32;
        let aborts: u64 = self.threads.values().map(|t| t.abort_count).sum();
        let successes: u64 = self.threads.values().map(|t| t.success_count).sum();
        let migrations: u64 = self.threads.values().map(|t| t.migration_count).sum();
        let rates: Vec<f64> = self.threads.values().map(|t| t.success_rate()).collect();
        let avg = if rates.is_empty() { 1.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        RseqV2BridgeStats { registered_threads: reg, total_aborts: aborts, total_successes: successes, total_migrations: migrations, avg_success_rate: avg }
    }
}
