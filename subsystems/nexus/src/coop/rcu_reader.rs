// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop RCU reader — Read-copy-update reader-side tracking
//!
//! Tracks RCU read-side critical sections, quiescent state reports,
//! callback deferral, and grace period participation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// RCU flavor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuFlavor {
    Preempt,
    Bh,
    Sched,
    Tasks,
    TasksRude,
    TasksTrace,
}

/// Reader state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuReaderState {
    Idle,
    InCriticalSection,
    QuiescentReported,
    Blocked,
}

/// A deferred RCU callback.
#[derive(Debug, Clone)]
pub struct RcuCallback {
    pub callback_id: u64,
    pub gp_number: u64,
    pub enqueue_time: u64,
    pub completed: bool,
}

impl RcuCallback {
    pub fn new(callback_id: u64, gp_number: u64) -> Self {
        Self {
            callback_id,
            gp_number,
            enqueue_time: 0,
            completed: false,
        }
    }
}

/// Per-CPU RCU reader state.
#[derive(Debug)]
pub struct RcuCpuState {
    pub cpu: u32,
    pub nesting_depth: u32,
    pub state: RcuReaderState,
    pub qs_reported_gp: u64,
    pub current_gp: AtomicU64,
    pub pending_callbacks: Vec<RcuCallback>,
    pub completed_callbacks: u64,
    pub total_cs_entered: u64,
    pub total_qs_reported: u64,
    pub max_cs_depth: u32,
}

impl RcuCpuState {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu,
            nesting_depth: 0,
            state: RcuReaderState::Idle,
            qs_reported_gp: 0,
            current_gp: AtomicU64::new(0),
            pending_callbacks: Vec::new(),
            completed_callbacks: 0,
            total_cs_entered: 0,
            total_qs_reported: 0,
            max_cs_depth: 0,
        }
    }

    pub fn rcu_read_lock(&mut self) {
        self.nesting_depth += 1;
        self.state = RcuReaderState::InCriticalSection;
        self.total_cs_entered += 1;
        if self.nesting_depth > self.max_cs_depth {
            self.max_cs_depth = self.nesting_depth;
        }
    }

    pub fn rcu_read_unlock(&mut self) {
        if self.nesting_depth > 0 {
            self.nesting_depth -= 1;
        }
        if self.nesting_depth == 0 {
            self.state = RcuReaderState::Idle;
        }
    }

    pub fn report_qs(&mut self, gp: u64) {
        if self.nesting_depth == 0 {
            self.qs_reported_gp = gp;
            self.state = RcuReaderState::QuiescentReported;
            self.total_qs_reported += 1;
        }
    }

    pub fn enqueue_callback(&mut self, cb: RcuCallback) {
        self.pending_callbacks.push(cb);
    }

    pub fn drain_completed(&mut self, completed_gp: u64) -> Vec<RcuCallback> {
        let mut done = Vec::new();
        self.pending_callbacks.retain(|cb| {
            if cb.gp_number <= completed_gp {
                done.push(cb.clone());
                false
            } else {
                true
            }
        });
        self.completed_callbacks += done.len() as u64;
        done
    }

    pub fn pending_count(&self) -> usize {
        self.pending_callbacks.len()
    }
}

/// Statistics for RCU reader.
#[derive(Debug, Clone)]
pub struct RcuReaderStats {
    pub total_cpus: u64,
    pub total_cs_entered: u64,
    pub total_qs_reported: u64,
    pub total_callbacks_completed: u64,
    pub total_callbacks_pending: u64,
    pub max_nesting_depth: u32,
}

/// Main coop RCU reader manager.
pub struct CoopRcuReader {
    pub cpus: BTreeMap<u32, RcuCpuState>,
    pub current_gp: AtomicU64,
    pub completed_gp: AtomicU64,
    pub next_callback_id: u64,
    pub flavor: RcuFlavor,
    pub stats: RcuReaderStats,
}

impl CoopRcuReader {
    pub fn new(flavor: RcuFlavor) -> Self {
        Self {
            cpus: BTreeMap::new(),
            current_gp: AtomicU64::new(1),
            completed_gp: AtomicU64::new(0),
            next_callback_id: 1,
            flavor,
            stats: RcuReaderStats {
                total_cpus: 0,
                total_cs_entered: 0,
                total_qs_reported: 0,
                total_callbacks_completed: 0,
                total_callbacks_pending: 0,
                max_nesting_depth: 0,
            },
        }
    }

    pub fn register_cpu(&mut self, cpu: u32) {
        if !self.cpus.contains_key(&cpu) {
            self.cpus.insert(cpu, RcuCpuState::new(cpu));
            self.stats.total_cpus += 1;
        }
    }

    pub fn rcu_read_lock(&mut self, cpu: u32) {
        if let Some(state) = self.cpus.get_mut(&cpu) {
            state.rcu_read_lock();
            self.stats.total_cs_entered += 1;
        }
    }

    pub fn rcu_read_unlock(&mut self, cpu: u32) {
        if let Some(state) = self.cpus.get_mut(&cpu) {
            state.rcu_read_unlock();
        }
    }

    pub fn check_gp_complete(&self) -> bool {
        let current = self.current_gp.load(Ordering::Acquire);
        for state in self.cpus.values() {
            if state.nesting_depth > 0 {
                return false;
            }
            if state.qs_reported_gp < current {
                return false;
            }
        }
        true
    }

    pub fn advance_gp(&self) {
        if self.check_gp_complete() {
            let current = self.current_gp.load(Ordering::Acquire);
            self.completed_gp.store(current, Ordering::Release);
            self.current_gp.fetch_add(1, Ordering::AcqRel);
        }
    }

    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }
}
