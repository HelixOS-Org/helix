// SPDX-License-Identifier: GPL-2.0
//! Coop rcu_v2 — advanced read-copy-update mechanism for cooperative data sharing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RCU flavor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuFlavor {
    /// Classic preemptible RCU
    Preempt,
    /// Bottom-half (BH) RCU
    Bh,
    /// Scheduler (SCHED) RCU
    Sched,
    /// Sleepable RCU (SRCU)
    Srcu,
    /// Tasks RCU
    Tasks,
    /// Polled/lazy RCU
    Polled,
}

/// Grace period state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GracePeriodState {
    Idle,
    Started,
    WaitingForReaders,
    Completed,
    ForcedExpedited,
}

/// An RCU grace period
#[derive(Debug, Clone)]
pub struct GracePeriod {
    pub id: u64,
    pub flavor: RcuFlavor,
    pub state: GracePeriodState,
    pub start_ns: u64,
    pub end_ns: u64,
    pub expedited: bool,
    pub cpu_stall_count: u32,
    pub pending_callbacks: u32,
}

impl GracePeriod {
    pub fn new(id: u64, flavor: RcuFlavor, now_ns: u64) -> Self {
        Self {
            id,
            flavor,
            state: GracePeriodState::Started,
            start_ns: now_ns,
            end_ns: 0,
            expedited: false,
            cpu_stall_count: 0,
            pending_callbacks: 0,
        }
    }

    #[inline(always)]
    pub fn complete(&mut self, now_ns: u64) {
        self.state = GracePeriodState::Completed;
        self.end_ns = now_ns;
    }

    #[inline]
    pub fn duration_ns(&self) -> u64 {
        if self.end_ns > 0 {
            self.end_ns.saturating_sub(self.start_ns)
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        matches!(self.state, GracePeriodState::Started | GracePeriodState::WaitingForReaders)
    }
}

/// Per-CPU RCU state
#[derive(Debug)]
#[repr(align(64))]
pub struct CpuRcuState {
    pub cpu_id: u32,
    pub in_read_side: bool,
    pub nesting_depth: u32,
    pub quiescent_count: u64,
    pub last_quiescent_ns: u64,
    pub callbacks_pending: u32,
    pub callbacks_executed: u64,
    pub stall_detected: bool,
}

impl CpuRcuState {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            in_read_side: false,
            nesting_depth: 0,
            quiescent_count: 0,
            last_quiescent_ns: 0,
            callbacks_pending: 0,
            callbacks_executed: 0,
            stall_detected: false,
        }
    }

    #[inline(always)]
    pub fn read_lock(&mut self) {
        self.nesting_depth += 1;
        self.in_read_side = true;
    }

    #[inline]
    pub fn read_unlock(&mut self) {
        self.nesting_depth = self.nesting_depth.saturating_sub(1);
        if self.nesting_depth == 0 {
            self.in_read_side = false;
        }
    }

    #[inline(always)]
    pub fn report_quiescent(&mut self, now_ns: u64) {
        self.quiescent_count += 1;
        self.last_quiescent_ns = now_ns;
    }

    #[inline(always)]
    pub fn enqueue_callback(&mut self) {
        self.callbacks_pending += 1;
    }

    #[inline]
    pub fn execute_callbacks(&mut self, count: u32) {
        let executed = count.min(self.callbacks_pending);
        self.callbacks_pending = self.callbacks_pending.saturating_sub(executed);
        self.callbacks_executed += executed as u64;
    }

    #[inline(always)]
    pub fn time_since_quiescent(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.last_quiescent_ns)
    }
}

/// RCU callback
#[derive(Debug)]
pub struct RcuCallback {
    pub id: u64,
    pub grace_period: u64,
    pub size_bytes: usize,
    pub enqueue_ns: u64,
    pub cpu_id: u32,
}

/// SRCU domain
#[derive(Debug)]
pub struct SrcuDomain {
    pub id: u64,
    pub active_readers: u32,
    pub completed_gp: u64,
    pub pending_gp: u64,
}

impl SrcuDomain {
    pub fn new(id: u64) -> Self {
        Self { id, active_readers: 0, completed_gp: 0, pending_gp: 0 }
    }

    #[inline(always)]
    pub fn read_lock(&mut self) -> u32 {
        self.active_readers += 1;
        self.active_readers
    }

    #[inline(always)]
    pub fn read_unlock(&mut self) {
        self.active_readers = self.active_readers.saturating_sub(1);
    }
}

/// RCU v2 stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RcuV2Stats {
    pub total_grace_periods: u64,
    pub completed_grace_periods: u64,
    pub expedited_grace_periods: u64,
    pub total_callbacks: u64,
    pub executed_callbacks: u64,
    pub cpu_stalls: u64,
    pub avg_gp_duration_ns: f64,
    pub max_gp_duration_ns: u64,
    pub total_srcu_domains: u64,
}

/// Main RCU v2 manager
pub struct CoopRcuV2 {
    cpus: BTreeMap<u32, CpuRcuState>,
    grace_periods: Vec<GracePeriod>,
    srcu_domains: BTreeMap<u64, SrcuDomain>,
    next_gp_id: u64,
    next_cb_id: u64,
    next_srcu_id: u64,
    active_gp: Option<u64>,
    stats: RcuV2Stats,
}

impl CoopRcuV2 {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            grace_periods: Vec::new(),
            srcu_domains: BTreeMap::new(),
            next_gp_id: 1,
            next_cb_id: 1,
            next_srcu_id: 1,
            active_gp: None,
            stats: RcuV2Stats {
                total_grace_periods: 0,
                completed_grace_periods: 0,
                expedited_grace_periods: 0,
                total_callbacks: 0,
                executed_callbacks: 0,
                cpu_stalls: 0,
                avg_gp_duration_ns: 0.0,
                max_gp_duration_ns: 0,
                total_srcu_domains: 0,
            },
        }
    }

    #[inline(always)]
    pub fn init_cpu(&mut self, cpu_id: u32) {
        self.cpus.insert(cpu_id, CpuRcuState::new(cpu_id));
    }

    #[inline]
    pub fn read_lock(&mut self, cpu_id: u32) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.read_lock();
        }
    }

    #[inline]
    pub fn read_unlock(&mut self, cpu_id: u32) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.read_unlock();
        }
    }

    #[inline]
    pub fn start_grace_period(&mut self, flavor: RcuFlavor, now_ns: u64) -> u64 {
        let id = self.next_gp_id;
        self.next_gp_id += 1;
        self.grace_periods.push(GracePeriod::new(id, flavor, now_ns));
        self.active_gp = Some(id);
        self.stats.total_grace_periods += 1;
        id
    }

    #[inline]
    pub fn report_quiescent(&mut self, cpu_id: u32, now_ns: u64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.report_quiescent(now_ns);
        }
        // Check if all CPUs have reported
        self.check_gp_completion(now_ns);
    }

    fn check_gp_completion(&mut self, now_ns: u64) {
        if let Some(gp_id) = self.active_gp {
            let all_quiescent = self.cpus.values().all(|cpu| !cpu.in_read_side);
            if all_quiescent {
                if let Some(gp) = self.grace_periods.iter_mut().find(|g| g.id == gp_id) {
                    gp.complete(now_ns);
                    let dur = gp.duration_ns();
                    if dur > self.stats.max_gp_duration_ns {
                        self.stats.max_gp_duration_ns = dur;
                    }
                    self.stats.completed_grace_periods += 1;
                    // Running average
                    let n = self.stats.completed_grace_periods as f64;
                    self.stats.avg_gp_duration_ns =
                        self.stats.avg_gp_duration_ns * ((n - 1.0) / n) + dur as f64 / n;
                }
                self.active_gp = None;
            }
        }
    }

    #[inline]
    pub fn enqueue_callback(&mut self, cpu_id: u32, size: usize) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.enqueue_callback();
            self.stats.total_callbacks += 1;
        }
    }

    #[inline]
    pub fn execute_callbacks(&mut self, cpu_id: u32, max_count: u32) -> u32 {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            let count = max_count.min(cpu.callbacks_pending);
            cpu.execute_callbacks(count);
            self.stats.executed_callbacks += count as u64;
            count
        } else {
            0
        }
    }

    #[inline]
    pub fn detect_stalls(&mut self, now_ns: u64, threshold_ns: u64) -> Vec<u32> {
        let mut stalled = Vec::new();
        for cpu in self.cpus.values_mut() {
            if cpu.in_read_side && cpu.time_since_quiescent(now_ns) > threshold_ns {
                cpu.stall_detected = true;
                stalled.push(cpu.cpu_id);
                self.stats.cpu_stalls += 1;
            }
        }
        stalled
    }

    #[inline]
    pub fn create_srcu_domain(&mut self) -> u64 {
        let id = self.next_srcu_id;
        self.next_srcu_id += 1;
        self.srcu_domains.insert(id, SrcuDomain::new(id));
        self.stats.total_srcu_domains += 1;
        id
    }

    #[inline]
    pub fn srcu_read_lock(&mut self, domain_id: u64) {
        if let Some(d) = self.srcu_domains.get_mut(&domain_id) {
            d.read_lock();
        }
    }

    #[inline]
    pub fn srcu_read_unlock(&mut self, domain_id: u64) {
        if let Some(d) = self.srcu_domains.get_mut(&domain_id) {
            d.read_unlock();
        }
    }

    #[inline(always)]
    pub fn pending_callbacks_total(&self) -> u32 {
        self.cpus.values().map(|c| c.callbacks_pending).sum()
    }

    #[inline]
    pub fn heaviest_callback_cpus(&self, top: usize) -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = self.cpus.iter()
            .map(|(&cpu_id, c)| (cpu_id, c.callbacks_pending))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    #[inline(always)]
    pub fn stats(&self) -> &RcuV2Stats {
        &self.stats
    }
}
