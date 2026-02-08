//! # Apps Timer Manager
//!
//! Application timer management:
//! - POSIX timer tracking (timer_create/timer_settime)
//! - Interval timer (setitimer) management
//! - Timerfd integration
//! - Timer coalescing for power efficiency
//! - Overrun counting for periodic timers
//! - Clock source selection per timer

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Clock type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppClockType {
    Realtime,
    Monotonic,
    ProcessCputime,
    ThreadCputime,
    RealtimeCoarse,
    MonotonicCoarse,
    Boottime,
    RealtimeAlarm,
    BoottimeAlarm,
}

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTimerType {
    OneShot,
    Periodic,
    Interval,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTimerState {
    Disarmed,
    Armed,
    Expired,
    Overrun,
}

/// Timer entry
#[derive(Debug, Clone)]
pub struct AppTimer {
    pub id: u64,
    pub process_id: u64,
    pub clock: AppClockType,
    pub timer_type: AppTimerType,
    pub state: AppTimerState,
    pub value_ns: u64,
    pub interval_ns: u64,
    pub expiry_ns: u64,
    pub overrun_count: u64,
    pub delivery_signal: u32,
    pub created_ns: u64,
    pub total_expirations: u64,
    pub coalesce_window_ns: u64,
}

impl AppTimer {
    pub fn new(id: u64, pid: u64, clock: AppClockType, ts: u64) -> Self {
        Self {
            id, process_id: pid, clock,
            timer_type: AppTimerType::OneShot,
            state: AppTimerState::Disarmed,
            value_ns: 0, interval_ns: 0, expiry_ns: 0,
            overrun_count: 0, delivery_signal: 14, // SIGALRM
            created_ns: ts, total_expirations: 0,
            coalesce_window_ns: 0,
        }
    }

    pub fn arm(&mut self, value_ns: u64, interval_ns: u64, now: u64) {
        self.value_ns = value_ns;
        self.interval_ns = interval_ns;
        self.expiry_ns = now + value_ns;
        self.timer_type = if interval_ns > 0 { AppTimerType::Periodic } else { AppTimerType::OneShot };
        self.state = AppTimerState::Armed;
    }

    pub fn disarm(&mut self) {
        self.state = AppTimerState::Disarmed;
    }

    pub fn check_expiry(&mut self, now: u64) -> bool {
        if self.state != AppTimerState::Armed { return false; }
        if now >= self.expiry_ns {
            self.total_expirations += 1;
            if self.interval_ns > 0 {
                // Periodic: calculate overruns
                let overruns = (now - self.expiry_ns) / self.interval_ns;
                self.overrun_count += overruns;
                self.total_expirations += overruns;
                self.expiry_ns += (overruns + 1) * self.interval_ns;
                self.state = AppTimerState::Armed; // re-arm
            } else {
                self.state = AppTimerState::Expired;
            }
            return true;
        }
        false
    }

    pub fn remaining(&self, now: u64) -> u64 {
        if self.state != AppTimerState::Armed { return 0; }
        self.expiry_ns.saturating_sub(now)
    }

    pub fn is_armed(&self) -> bool { self.state == AppTimerState::Armed }
}

/// Interval timer (setitimer)
#[derive(Debug, Clone)]
pub struct IntervalTimer {
    pub which: IntervalTimerWhich,
    pub value_ns: u64,
    pub interval_ns: u64,
    pub expiry_ns: u64,
    pub armed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalTimerWhich {
    Real,
    Virtual,
    Prof,
}

impl IntervalTimer {
    pub fn new(which: IntervalTimerWhich) -> Self {
        Self { which, value_ns: 0, interval_ns: 0, expiry_ns: 0, armed: false }
    }
}

/// Per-process timer set
#[derive(Debug, Clone)]
pub struct ProcessTimerSet {
    pub process_id: u64,
    pub timers: BTreeMap<u64, AppTimer>,
    pub itimers: [IntervalTimer; 3],
    pub total_created: u64,
    pub total_deleted: u64,
    pub total_expirations: u64,
}

impl ProcessTimerSet {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            timers: BTreeMap::new(),
            itimers: [
                IntervalTimer::new(IntervalTimerWhich::Real),
                IntervalTimer::new(IntervalTimerWhich::Virtual),
                IntervalTimer::new(IntervalTimerWhich::Prof),
            ],
            total_created: 0,
            total_deleted: 0,
            total_expirations: 0,
        }
    }
}

/// Coalesce group for power savings
#[derive(Debug, Clone)]
pub struct TimerCoalesceGroup {
    pub window_ns: u64,
    pub timers: Vec<u64>,
    pub earliest_expiry: u64,
    pub latest_expiry: u64,
}

/// Apps timer manager stats
#[derive(Debug, Clone, Default)]
pub struct AppsTimerMgrStats {
    pub total_processes: usize,
    pub total_timers: usize,
    pub armed_timers: usize,
    pub total_expirations: u64,
    pub total_overruns: u64,
    pub coalesce_groups: usize,
}

/// Apps Timer Manager
pub struct AppsTimerMgr {
    process_timers: BTreeMap<u64, ProcessTimerSet>,
    coalesce_groups: Vec<TimerCoalesceGroup>,
    stats: AppsTimerMgrStats,
    next_timer_id: u64,
    coalesce_window: u64,
}

impl AppsTimerMgr {
    pub fn new(coalesce_window_ns: u64) -> Self {
        Self {
            process_timers: BTreeMap::new(),
            coalesce_groups: Vec::new(),
            stats: AppsTimerMgrStats::default(),
            next_timer_id: 1,
            coalesce_window: coalesce_window_ns,
        }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.process_timers.entry(pid).or_insert_with(|| ProcessTimerSet::new(pid));
    }

    pub fn timer_create(&mut self, pid: u64, clock: AppClockType, ts: u64) -> Option<u64> {
        let timer_id = self.next_timer_id;
        self.next_timer_id += 1;
        let timer = AppTimer::new(timer_id, pid, clock, ts);
        if let Some(set) = self.process_timers.get_mut(&pid) {
            set.timers.insert(timer_id, timer);
            set.total_created += 1;
            Some(timer_id)
        } else { None }
    }

    pub fn timer_settime(&mut self, pid: u64, timer_id: u64, value_ns: u64, interval_ns: u64, now: u64) -> bool {
        if let Some(set) = self.process_timers.get_mut(&pid) {
            if let Some(timer) = set.timers.get_mut(&timer_id) {
                timer.arm(value_ns, interval_ns, now);
                return true;
            }
        }
        false
    }

    pub fn timer_delete(&mut self, pid: u64, timer_id: u64) -> bool {
        if let Some(set) = self.process_timers.get_mut(&pid) {
            if set.timers.remove(&timer_id).is_some() {
                set.total_deleted += 1;
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self, now: u64) -> Vec<(u64, u64, u32)> {
        let mut expired = Vec::new();
        for set in self.process_timers.values_mut() {
            for timer in set.timers.values_mut() {
                if timer.check_expiry(now) {
                    expired.push((set.process_id, timer.id, timer.delivery_signal));
                    set.total_expirations += 1;
                }
            }
        }
        expired
    }

    pub fn remove_process(&mut self, pid: u64) { self.process_timers.remove(&pid); }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.process_timers.len();
        self.stats.total_timers = self.process_timers.values().map(|s| s.timers.len()).sum();
        self.stats.armed_timers = self.process_timers.values()
            .flat_map(|s| s.timers.values())
            .filter(|t| t.is_armed()).count();
        self.stats.total_expirations = self.process_timers.values().map(|s| s.total_expirations).sum();
        self.stats.total_overruns = self.process_timers.values()
            .flat_map(|s| s.timers.values())
            .map(|t| t.overrun_count).sum();
        self.stats.coalesce_groups = self.coalesce_groups.len();
    }

    pub fn process_timers(&self, pid: u64) -> Option<&ProcessTimerSet> { self.process_timers.get(&pid) }
    pub fn stats(&self) -> &AppsTimerMgrStats { &self.stats }
}
