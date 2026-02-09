//! # Holistic RCU Tracker
//!
//! Read-Copy-Update mechanism tracking with holistic view:
//! - Grace period tracking
//! - Per-CPU quiescent state monitoring
//! - Callback queue management
//! - RCU stall detection
//! - Expedited grace period support
//! - RCU flavor tracking (preempt, sched, bh)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RCU flavor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuFlavor {
    RcuPreempt,
    RcuSched,
    RcuBh,
    Srcu,
    Tasks,
}

/// Grace period state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GracePeriodState {
    Idle,
    Started,
    WaitingForQs,
    ForcingQs,
    Completing,
    Completed,
}

/// Per-CPU RCU state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PerCpuRcuState {
    pub cpu_id: u32,
    pub qs_passed: bool,
    pub last_qs_ts: u64,
    pub callbacks_pending: u32,
    pub callbacks_processed: u64,
    pub in_critical_section: bool,
    pub nesting_depth: u32,
    pub online: bool,
}

impl PerCpuRcuState {
    pub fn new(cpu: u32) -> Self {
        Self { cpu_id: cpu, qs_passed: false, last_qs_ts: 0, callbacks_pending: 0, callbacks_processed: 0, in_critical_section: false, nesting_depth: 0, online: true }
    }

    #[inline(always)]
    pub fn enter_read(&mut self) { self.nesting_depth += 1; self.in_critical_section = true; }
    #[inline(always)]
    pub fn exit_read(&mut self) { self.nesting_depth = self.nesting_depth.saturating_sub(1); if self.nesting_depth == 0 { self.in_critical_section = false; } }
    #[inline(always)]
    pub fn report_qs(&mut self, ts: u64) { self.qs_passed = true; self.last_qs_ts = ts; }
    #[inline(always)]
    pub fn new_gp(&mut self) { self.qs_passed = false; }
}

/// Grace period descriptor
#[derive(Debug, Clone)]
pub struct GracePeriod {
    pub gp_num: u64,
    pub flavor: RcuFlavor,
    pub state: GracePeriodState,
    pub start_ts: u64,
    pub end_ts: u64,
    pub cpus_pending: Vec<u32>,
    pub expedited: bool,
    pub force_qs_count: u32,
}

impl GracePeriod {
    pub fn new(num: u64, flavor: RcuFlavor, ts: u64, cpus: Vec<u32>, expedited: bool) -> Self {
        Self { gp_num: num, flavor, state: GracePeriodState::Started, start_ts: ts, end_ts: 0, cpus_pending: cpus, expedited, force_qs_count: 0 }
    }

    #[inline(always)]
    pub fn report_qs(&mut self, cpu: u32) {
        self.cpus_pending.retain(|&c| c != cpu);
        if self.cpus_pending.is_empty() { self.state = GracePeriodState::Completing; }
    }

    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.state = GracePeriodState::Completed; self.end_ts = ts; }
    #[inline(always)]
    pub fn force_qs(&mut self) { self.force_qs_count += 1; self.state = GracePeriodState::ForcingQs; }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.end_ts.saturating_sub(self.start_ts) }
    #[inline(always)]
    pub fn is_complete(&self) -> bool { self.state == GracePeriodState::Completed }
}

/// RCU callback
#[derive(Debug, Clone)]
pub struct RcuCallback {
    pub id: u64,
    pub gp_num: u64,
    pub cpu: u32,
    pub func_hash: u64,
    pub register_ts: u64,
    pub invoke_ts: u64,
    pub invoked: bool,
}

/// Stall record
#[derive(Debug, Clone)]
pub struct RcuStall {
    pub gp_num: u64,
    pub cpu: u32,
    pub stall_ts: u64,
    pub duration_ns: u64,
    pub flavor: RcuFlavor,
    pub nesting_depth: u32,
}

/// RCU tracker stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RcuStats {
    pub gp_completed: u64,
    pub gp_active: usize,
    pub total_callbacks: u64,
    pub callbacks_invoked: u64,
    pub stalls_detected: u64,
    pub avg_gp_latency_ns: u64,
    pub max_gp_latency_ns: u64,
    pub expedited_count: u64,
}

/// Holistic RCU tracker
pub struct HolisticRcuTracker {
    cpus: BTreeMap<u32, PerCpuRcuState>,
    grace_periods: BTreeMap<u64, GracePeriod>,
    callbacks: Vec<RcuCallback>,
    stalls: Vec<RcuStall>,
    stats: RcuStats,
    current_gp: u64,
    next_cb_id: u64,
    stall_timeout_ns: u64,
}

impl HolisticRcuTracker {
    pub fn new(stall_timeout: u64) -> Self {
        Self {
            cpus: BTreeMap::new(), grace_periods: BTreeMap::new(),
            callbacks: Vec::new(), stalls: Vec::new(),
            stats: RcuStats::default(), current_gp: 0,
            next_cb_id: 1, stall_timeout_ns: stall_timeout,
        }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: u32) { self.cpus.insert(cpu, PerCpuRcuState::new(cpu)); }
    #[inline(always)]
    pub fn offline_cpu(&mut self, cpu: u32) { if let Some(c) = self.cpus.get_mut(&cpu) { c.online = false; c.qs_passed = true; } }
    #[inline(always)]
    pub fn online_cpu(&mut self, cpu: u32) { if let Some(c) = self.cpus.get_mut(&cpu) { c.online = true; } }

    #[inline]
    pub fn start_gp(&mut self, flavor: RcuFlavor, ts: u64, expedited: bool) -> u64 {
        self.current_gp += 1;
        let cpus: Vec<u32> = self.cpus.values().filter(|c| c.online).map(|c| c.cpu_id).collect();
        for c in self.cpus.values_mut() { if c.online { c.new_gp(); } }
        let gp = GracePeriod::new(self.current_gp, flavor, ts, cpus, expedited);
        self.grace_periods.insert(self.current_gp, gp);
        if expedited { self.stats.expedited_count += 1; }
        self.current_gp
    }

    #[inline]
    pub fn report_qs(&mut self, cpu: u32, ts: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.report_qs(ts); }
        let gp_nums: Vec<u64> = self.grace_periods.keys().copied().collect();
        for gp_num in gp_nums {
            if let Some(gp) = self.grace_periods.get_mut(&gp_num) {
                if !gp.is_complete() { gp.report_qs(cpu); }
            }
        }
    }

    pub fn try_complete(&mut self, ts: u64) -> Vec<u64> {
        let mut completed = Vec::new();
        for gp in self.grace_periods.values_mut() {
            if gp.state == GracePeriodState::Completing {
                gp.complete(ts);
                completed.push(gp.gp_num);
                self.stats.gp_completed += 1;
            }
        }
        // Invoke ready callbacks
        for cb in &mut self.callbacks {
            if !cb.invoked && completed.contains(&cb.gp_num) {
                cb.invoked = true;
                cb.invoke_ts = ts;
                self.stats.callbacks_invoked += 1;
                if let Some(c) = self.cpus.get_mut(&cb.cpu) { c.callbacks_pending = c.callbacks_pending.saturating_sub(1); c.callbacks_processed += 1; }
            }
        }
        completed
    }

    #[inline]
    pub fn register_callback(&mut self, gp: u64, cpu: u32, func_hash: u64, ts: u64) -> u64 {
        let id = self.next_cb_id; self.next_cb_id += 1;
        self.callbacks.push(RcuCallback { id, gp_num: gp, cpu, func_hash, register_ts: ts, invoke_ts: 0, invoked: false });
        if let Some(c) = self.cpus.get_mut(&cpu) { c.callbacks_pending += 1; }
        self.stats.total_callbacks += 1;
        id
    }

    pub fn detect_stalls(&mut self, now: u64) -> Vec<RcuStall> {
        let mut new_stalls = Vec::new();
        for gp in self.grace_periods.values() {
            if gp.is_complete() { continue; }
            if now.saturating_sub(gp.start_ts) < self.stall_timeout_ns { continue; }
            for &cpu in &gp.cpus_pending {
                let depth = self.cpus.get(&cpu).map(|c| c.nesting_depth).unwrap_or(0);
                let stall = RcuStall { gp_num: gp.gp_num, cpu, stall_ts: now, duration_ns: now.saturating_sub(gp.start_ts), flavor: gp.flavor, nesting_depth: depth };
                new_stalls.push(stall.clone());
                self.stalls.push(stall);
                self.stats.stalls_detected += 1;
            }
        }
        new_stalls
    }

    #[inline(always)]
    pub fn enter_read(&mut self, cpu: u32) { if let Some(c) = self.cpus.get_mut(&cpu) { c.enter_read(); } }
    #[inline(always)]
    pub fn exit_read(&mut self, cpu: u32) { if let Some(c) = self.cpus.get_mut(&cpu) { c.exit_read(); } }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.gp_active = self.grace_periods.values().filter(|g| !g.is_complete()).count();
        let done: Vec<&GracePeriod> = self.grace_periods.values().filter(|g| g.is_complete()).collect();
        if !done.is_empty() {
            let total: u64 = done.iter().map(|g| g.latency()).sum();
            self.stats.avg_gp_latency_ns = total / done.len() as u64;
            self.stats.max_gp_latency_ns = done.iter().map(|g| g.latency()).max().unwrap_or(0);
        }
    }

    #[inline(always)]
    pub fn cpu(&self, id: u32) -> Option<&PerCpuRcuState> { self.cpus.get(&id) }
    #[inline(always)]
    pub fn gp(&self, num: u64) -> Option<&GracePeriod> { self.grace_periods.get(&num) }
    #[inline(always)]
    pub fn stats(&self) -> &RcuStats { &self.stats }
    #[inline(always)]
    pub fn stalls(&self) -> &[RcuStall] { &self.stalls }
}
