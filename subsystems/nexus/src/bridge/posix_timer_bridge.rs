// SPDX-License-Identifier: GPL-2.0
//! Bridge posix_timer_bridge — POSIX timer management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Clock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixClockId {
    Realtime,
    Monotonic,
    ProcessCpuTime,
    ThreadCpuTime,
    MonotonicRaw,
    RealtimeCoarse,
    MonotonicCoarse,
    Boottime,
    RealtimeAlarm,
    BoottimeAlarm,
}

/// Timer notification method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerNotify {
    Signal(i32),
    ThreadId(u64),
    None,
}

/// Interval timer
#[derive(Debug)]
#[repr(align(64))]
pub struct PosixTimer {
    pub id: u64,
    pub clock_id: PosixClockId,
    pub notify: TimerNotify,
    pub interval_ns: u64,
    pub value_ns: u64,
    pub armed: bool,
    pub absolute: bool,
    pub overrun_count: u64,
    pub fire_count: u64,
    pub created_at: u64,
    pub last_fire: u64,
}

impl PosixTimer {
    pub fn new(id: u64, clock: PosixClockId, notify: TimerNotify, now: u64) -> Self {
        Self {
            id, clock_id: clock, notify, interval_ns: 0, value_ns: 0,
            armed: false, absolute: false, overrun_count: 0,
            fire_count: 0, created_at: now, last_fire: 0,
        }
    }

    #[inline]
    pub fn arm(&mut self, value: u64, interval: u64, absolute: bool) {
        self.value_ns = value;
        self.interval_ns = interval;
        self.absolute = absolute;
        self.armed = true;
    }

    #[inline(always)]
    pub fn disarm(&mut self) { self.armed = false; }

    #[inline]
    pub fn fire(&mut self, now: u64) {
        self.fire_count += 1;
        self.last_fire = now;
        if self.interval_ns > 0 {
            self.value_ns = now + self.interval_ns;
        } else {
            self.armed = false;
        }
    }

    #[inline(always)]
    pub fn check_expired(&self, now: u64) -> bool {
        self.armed && now >= self.value_ns
    }

    #[inline]
    pub fn compute_overrun(&mut self, now: u64) {
        if self.interval_ns > 0 && self.last_fire > 0 {
            let elapsed = now.saturating_sub(self.last_fire);
            let missed = elapsed / self.interval_ns;
            if missed > 1 { self.overrun_count += missed - 1; }
        }
    }

    #[inline(always)]
    pub fn is_periodic(&self) -> bool { self.interval_ns > 0 }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PosixTimerBridgeStats {
    pub total_timers: u32,
    pub armed_timers: u32,
    pub periodic_timers: u32,
    pub total_fires: u64,
    pub total_overruns: u64,
    pub signal_timers: u32,
}

/// Main posix timer bridge
#[repr(align(64))]
pub struct BridgePosixTimer {
    timers: BTreeMap<u64, PosixTimer>,
    next_id: u64,
}

impl BridgePosixTimer {
    pub fn new() -> Self { Self { timers: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, clock: PosixClockId, notify: TimerNotify, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.timers.insert(id, PosixTimer::new(id, clock, notify, now));
        id
    }

    #[inline(always)]
    pub fn arm(&mut self, id: u64, value: u64, interval: u64, absolute: bool) {
        if let Some(t) = self.timers.get_mut(&id) { t.arm(value, interval, absolute); }
    }

    #[inline(always)]
    pub fn disarm(&mut self, id: u64) {
        if let Some(t) = self.timers.get_mut(&id) { t.disarm(); }
    }

    #[inline]
    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let mut fired = Vec::new();
        for timer in self.timers.values_mut() {
            if timer.check_expired(now) {
                timer.fire(now);
                fired.push(timer.id);
            }
        }
        fired
    }

    #[inline(always)]
    pub fn delete(&mut self, id: u64) { self.timers.remove(&id); }

    pub fn stats(&self) -> PosixTimerBridgeStats {
        let armed = self.timers.values().filter(|t| t.armed).count() as u32;
        let periodic = self.timers.values().filter(|t| t.is_periodic()).count() as u32;
        let fires: u64 = self.timers.values().map(|t| t.fire_count).sum();
        let overruns: u64 = self.timers.values().map(|t| t.overrun_count).sum();
        let sig = self.timers.values().filter(|t| matches!(t.notify, TimerNotify::Signal(_))).count() as u32;
        PosixTimerBridgeStats {
            total_timers: self.timers.len() as u32, armed_timers: armed,
            periodic_timers: periodic, total_fires: fires,
            total_overruns: overruns, signal_timers: sig,
        }
    }
}
