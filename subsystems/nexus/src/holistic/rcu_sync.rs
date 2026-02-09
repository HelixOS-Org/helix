// SPDX-License-Identifier: GPL-2.0
//! Holistic rcu_sync — RCU synchronization primitives.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// RCU flavor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuFlavor {
    Preemptible,
    Sched,
    Bh,
    Tasks,
    TasksTrace,
    TasksRude,
}

/// Grace period state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpState {
    Idle,
    Started,
    WaitingQs,
    ForcingQs,
    Completing,
    Completed,
}

/// Per-CPU RCU data
#[derive(Debug)]
pub struct RcuCpuData {
    pub cpu: u32,
    pub gp_seq: u64,
    pub qs_reported: bool,
    pub callbacks_pending: u32,
    pub callbacks_invoked: u64,
    pub online: bool,
}

impl RcuCpuData {
    pub fn new(cpu: u32) -> Self {
        Self { cpu, gp_seq: 0, qs_reported: false, callbacks_pending: 0, callbacks_invoked: 0, online: true }
    }

    #[inline(always)]
    pub fn report_qs(&mut self, seq: u64) { self.qs_reported = true; self.gp_seq = seq; }
}

/// RCU callback
#[derive(Debug)]
pub struct RcuCallback {
    pub id: u64,
    pub gp_seq: u64,
    pub queued_at: u64,
    pub invoked: bool,
}

/// Grace period info
#[derive(Debug)]
pub struct GracePeriod {
    pub seq: u64,
    pub state: GpState,
    pub started_at: u64,
    pub completed_at: u64,
    pub fqs_count: u32,
}

impl GracePeriod {
    pub fn new(seq: u64, now: u64) -> Self {
        Self { seq, state: GpState::Started, started_at: now, completed_at: 0, fqs_count: 0 }
    }

    #[inline(always)]
    pub fn complete(&mut self, now: u64) { self.state = GpState::Completed; self.completed_at = now; }
    #[inline(always)]
    pub fn duration_ns(&self) -> u64 { if self.completed_at > 0 { self.completed_at - self.started_at } else { 0 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RcuSyncStats {
    pub total_cpus: u32,
    pub gp_completed: u64,
    pub gp_current_seq: u64,
    pub callbacks_pending: u32,
    pub callbacks_invoked: u64,
    pub avg_gp_duration_ns: u64,
    pub total_fqs: u64,
}

/// Main RCU sync manager
pub struct HolisticRcuSync {
    flavor: RcuFlavor,
    cpu_data: BTreeMap<u32, RcuCpuData>,
    grace_periods: Vec<GracePeriod>,
    callbacks: Vec<RcuCallback>,
    current_gp_seq: u64,
    next_cb_id: u64,
}

impl HolisticRcuSync {
    pub fn new(flavor: RcuFlavor) -> Self {
        Self { flavor, cpu_data: BTreeMap::new(), grace_periods: Vec::new(), callbacks: Vec::new(), current_gp_seq: 0, next_cb_id: 1 }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: u32) { self.cpu_data.insert(cpu, RcuCpuData::new(cpu)); }

    #[inline]
    pub fn start_gp(&mut self, now: u64) -> u64 {
        self.current_gp_seq += 1;
        for data in self.cpu_data.values_mut() { data.qs_reported = false; }
        self.grace_periods.push(GracePeriod::new(self.current_gp_seq, now));
        self.current_gp_seq
    }

    #[inline(always)]
    pub fn report_qs(&mut self, cpu: u32) {
        let seq = self.current_gp_seq;
        if let Some(data) = self.cpu_data.get_mut(&cpu) { data.report_qs(seq); }
    }

    #[inline]
    pub fn check_gp_complete(&mut self, now: u64) -> bool {
        let all_reported = self.cpu_data.values().filter(|d| d.online).all(|d| d.qs_reported);
        if all_reported {
            if let Some(gp) = self.grace_periods.last_mut() { if gp.state != GpState::Completed { gp.complete(now); return true; } }
        }
        false
    }

    #[inline]
    pub fn queue_callback(&mut self, now: u64) -> u64 {
        let id = self.next_cb_id; self.next_cb_id += 1;
        self.callbacks.push(RcuCallback { id, gp_seq: self.current_gp_seq, queued_at: now, invoked: false });
        id
    }

    #[inline]
    pub fn stats(&self) -> RcuSyncStats {
        let completed = self.grace_periods.iter().filter(|gp| gp.state == GpState::Completed).count() as u64;
        let pending: u32 = self.cpu_data.values().map(|d| d.callbacks_pending).sum();
        let invoked: u64 = self.cpu_data.values().map(|d| d.callbacks_invoked).sum();
        let durs: Vec<u64> = self.grace_periods.iter().filter(|gp| gp.completed_at > 0).map(|gp| gp.duration_ns()).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        let fqs: u64 = self.grace_periods.iter().map(|gp| gp.fqs_count as u64).sum();
        RcuSyncStats { total_cpus: self.cpu_data.len() as u32, gp_completed: completed, gp_current_seq: self.current_gp_seq, callbacks_pending: pending, callbacks_invoked: invoked, avg_gp_duration_ns: avg, total_fqs: fqs }
    }
}
