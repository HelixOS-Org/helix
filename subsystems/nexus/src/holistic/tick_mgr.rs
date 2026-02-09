//! # Holistic Tick Manager
//!
//! System tick and timer management:
//! - Tickless (NO_HZ) mode tracking
//! - Timer wheel management per CPU
//! - High-resolution timer scheduling
//! - Tick broadcasting for idle CPUs
//! - Jiffies accounting and drift compensation
//! - Timer migration between CPUs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Tick mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickMode {
    Periodic,
    NoHzIdle,
    NoHzFull,
    Broadcast,
}

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    SoftTimer,
    HrTimer,
    Deadline,
    Interval,
    OneShot,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Pending,
    Active,
    Expired,
    Cancelled,
    Migrated,
}

/// Individual timer entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerEntry {
    pub timer_id: u64,
    pub timer_type: TimerType,
    pub state: TimerState,
    pub cpu_id: u32,
    pub expires_ns: u64,
    pub period_ns: u64,
    pub slack_ns: u64,
    pub callback_time_ns: u64,
    pub fire_count: u64,
    pub miss_count: u64,
}

impl TimerEntry {
    pub fn new(id: u64, ttype: TimerType, cpu: u32, expires: u64) -> Self {
        Self {
            timer_id: id, timer_type: ttype, state: TimerState::Pending,
            cpu_id: cpu, expires_ns: expires, period_ns: 0, slack_ns: 0,
            callback_time_ns: 0, fire_count: 0, miss_count: 0,
        }
    }

    #[inline]
    pub fn fire(&mut self, actual_ts: u64) {
        self.state = TimerState::Expired;
        self.fire_count += 1;
        if actual_ts > self.expires_ns + self.slack_ns {
            self.miss_count += 1;
        }
    }

    #[inline(always)]
    pub fn rearm(&mut self, new_expires: u64) {
        self.expires_ns = new_expires;
        self.state = TimerState::Pending;
    }

    #[inline(always)]
    pub fn accuracy(&self) -> f64 {
        if self.fire_count == 0 { 1.0 }
        else { 1.0 - (self.miss_count as f64 / self.fire_count as f64) }
    }

    #[inline(always)]
    pub fn is_hrtimer(&self) -> bool { self.timer_type == TimerType::HrTimer }
}

/// Timer wheel bucket
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerWheelBucket {
    pub level: u8,
    pub slot: u32,
    pub timer_count: u32,
    pub next_expiry_ns: u64,
}

impl TimerWheelBucket {
    pub fn new(level: u8, slot: u32) -> Self {
        Self { level, slot, timer_count: 0, next_expiry_ns: u64::MAX }
    }
}

/// Per-CPU tick state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuTickState {
    pub cpu_id: u32,
    pub mode: TickMode,
    pub jiffies: u64,
    pub tick_period_ns: u64,
    pub last_tick_ts: u64,
    pub ticks_missed: u64,
    pub nohz_idle_since: u64,
    pub nohz_exits: u64,
    pub pending_timers: u32,
    pub hr_timers_active: u32,
    pub broadcast_pending: bool,
    pub next_timer_ns: u64,
    pub drift_ns: i64,
}

impl CpuTickState {
    pub fn new(cpu_id: u32, period_ns: u64) -> Self {
        Self {
            cpu_id, mode: TickMode::Periodic, jiffies: 0,
            tick_period_ns: period_ns, last_tick_ts: 0, ticks_missed: 0,
            nohz_idle_since: 0, nohz_exits: 0, pending_timers: 0,
            hr_timers_active: 0, broadcast_pending: false,
            next_timer_ns: u64::MAX, drift_ns: 0,
        }
    }

    #[inline]
    pub fn tick(&mut self, ts: u64) {
        let expected = self.last_tick_ts + self.tick_period_ns;
        if ts > expected + self.tick_period_ns {
            let missed = (ts - expected) / self.tick_period_ns;
            self.ticks_missed += missed;
        }
        self.jiffies += 1;
        self.drift_ns = ts as i64 - (self.last_tick_ts as i64 + self.tick_period_ns as i64);
        self.last_tick_ts = ts;
    }

    #[inline(always)]
    pub fn enter_nohz(&mut self, ts: u64) {
        self.mode = TickMode::NoHzIdle;
        self.nohz_idle_since = ts;
    }

    #[inline]
    pub fn exit_nohz(&mut self, ts: u64) {
        self.mode = TickMode::Periodic;
        self.nohz_exits += 1;
        // Catch up jiffies
        if self.nohz_idle_since > 0 {
            let idle_ns = ts.saturating_sub(self.nohz_idle_since);
            let missed_ticks = idle_ns / self.tick_period_ns;
            self.jiffies += missed_ticks;
        }
    }

    #[inline]
    pub fn idle_time_ns(&self, current_ts: u64) -> u64 {
        if self.mode == TickMode::NoHzIdle || self.mode == TickMode::NoHzFull {
            current_ts.saturating_sub(self.nohz_idle_since)
        } else { 0 }
    }
}

/// Broadcast state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BroadcastState {
    pub broadcaster_cpu: u32,
    pub idle_cpus: Vec<u32>,
    pub next_event_ns: u64,
    pub events_sent: u64,
    pub events_missed: u64,
}

impl BroadcastState {
    pub fn new(broadcaster: u32) -> Self {
        Self { broadcaster_cpu: broadcaster, idle_cpus: Vec::new(), next_event_ns: u64::MAX, events_sent: 0, events_missed: 0 }
    }

    #[inline(always)]
    pub fn add_idle_cpu(&mut self, cpu: u32) {
        if !self.idle_cpus.contains(&cpu) { self.idle_cpus.push(cpu); }
    }

    #[inline(always)]
    pub fn remove_idle_cpu(&mut self, cpu: u32) {
        self.idle_cpus.retain(|&c| c != cpu);
    }
}

/// Tick manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TickMgrStats {
    pub total_cpus: usize,
    pub nohz_idle_cpus: usize,
    pub nohz_full_cpus: usize,
    pub total_timers: usize,
    pub hr_timers: usize,
    pub total_ticks_missed: u64,
    pub avg_drift_ns: f64,
    pub total_nohz_exits: u64,
    pub timer_accuracy: f64,
}

/// Holistic tick manager
pub struct HolisticTickMgr {
    cpus: BTreeMap<u32, CpuTickState>,
    timers: BTreeMap<u64, TimerEntry>,
    broadcast: Option<BroadcastState>,
    stats: TickMgrStats,
    next_timer_id: u64,
}

impl HolisticTickMgr {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(), timers: BTreeMap::new(),
            broadcast: None, stats: TickMgrStats::default(),
            next_timer_id: 1,
        }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, cpu_id: u32, tick_period_ns: u64) {
        self.cpus.insert(cpu_id, CpuTickState::new(cpu_id, tick_period_ns));
    }

    #[inline(always)]
    pub fn setup_broadcast(&mut self, broadcaster: u32) {
        self.broadcast = Some(BroadcastState::new(broadcaster));
    }

    #[inline(always)]
    pub fn tick(&mut self, cpu: u32, ts: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.tick(ts); }
    }

    #[inline(always)]
    pub fn enter_nohz(&mut self, cpu: u32, ts: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.enter_nohz(ts); }
        if let Some(ref mut bc) = self.broadcast { bc.add_idle_cpu(cpu); }
    }

    #[inline(always)]
    pub fn exit_nohz(&mut self, cpu: u32, ts: u64) {
        if let Some(c) = self.cpus.get_mut(&cpu) { c.exit_nohz(ts); }
        if let Some(ref mut bc) = self.broadcast { bc.remove_idle_cpu(cpu); }
    }

    #[inline]
    pub fn add_timer(&mut self, ttype: TimerType, cpu: u32, expires: u64) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        self.timers.insert(id, TimerEntry::new(id, ttype, cpu, expires));
        id
    }

    #[inline(always)]
    pub fn fire_timer(&mut self, timer_id: u64, ts: u64) {
        if let Some(t) = self.timers.get_mut(&timer_id) { t.fire(ts); }
    }

    #[inline(always)]
    pub fn cancel_timer(&mut self, timer_id: u64) {
        if let Some(t) = self.timers.get_mut(&timer_id) { t.state = TimerState::Cancelled; }
    }

    #[inline]
    pub fn migrate_timer(&mut self, timer_id: u64, new_cpu: u32) {
        if let Some(t) = self.timers.get_mut(&timer_id) {
            t.cpu_id = new_cpu;
            t.state = TimerState::Migrated;
        }
    }

    pub fn process_expired(&mut self, ts: u64) -> Vec<u64> {
        let mut fired = Vec::new();
        let ids: Vec<u64> = self.timers.keys().copied().collect();
        for id in ids {
            if let Some(t) = self.timers.get(&id) {
                if t.state == TimerState::Pending && t.expires_ns <= ts {
                    fired.push(id);
                }
            }
        }
        for &id in &fired {
            if let Some(t) = self.timers.get_mut(&id) { t.fire(ts); }
        }
        fired
    }

    pub fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.nohz_idle_cpus = self.cpus.values().filter(|c| c.mode == TickMode::NoHzIdle).count();
        self.stats.nohz_full_cpus = self.cpus.values().filter(|c| c.mode == TickMode::NoHzFull).count();
        self.stats.total_timers = self.timers.len();
        self.stats.hr_timers = self.timers.values().filter(|t| t.is_hrtimer()).count();
        self.stats.total_ticks_missed = self.cpus.values().map(|c| c.ticks_missed).sum();
        let drifts: Vec<f64> = self.cpus.values().map(|c| c.drift_ns as f64).collect();
        self.stats.avg_drift_ns = if drifts.is_empty() { 0.0 } else { drifts.iter().sum::<f64>() / drifts.len() as f64 };
        self.stats.total_nohz_exits = self.cpus.values().map(|c| c.nohz_exits).sum();
        let accs: Vec<f64> = self.timers.values().map(|t| t.accuracy()).collect();
        self.stats.timer_accuracy = if accs.is_empty() { 1.0 } else { accs.iter().sum::<f64>() / accs.len() as f64 };
    }

    #[inline(always)]
    pub fn cpu(&self, id: u32) -> Option<&CpuTickState> { self.cpus.get(&id) }
    #[inline(always)]
    pub fn timer(&self, id: u64) -> Option<&TimerEntry> { self.timers.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &TickMgrStats { &self.stats }
}
