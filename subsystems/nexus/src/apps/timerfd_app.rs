// SPDX-License-Identifier: GPL-2.0
//! Apps timerfd_app — timerfd timer file descriptor application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Timer clock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerClockId {
    Realtime,
    Monotonic,
    BootTime,
    RealtimeAlarm,
    BootTimeAlarm,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdState {
    Disarmed,
    Armed,
    Expired,
    Cancelled,
}

/// Timerfd spec
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdSpec {
    pub interval_ns: u64,
    pub value_ns: u64,
    pub absolute: bool,
    pub cancel_on_set: bool,
}

/// Timerfd instance
#[derive(Debug)]
#[repr(align(64))]
pub struct TimerfdInstance {
    pub fd: u64,
    pub clock: TimerClockId,
    pub state: TimerfdState,
    pub spec: TimerfdSpec,
    pub created_at: u64,
    pub armed_at: u64,
    pub expiration_count: u64,
    pub overrun_count: u64,
    pub total_reads: u64,
}

impl TimerfdInstance {
    pub fn new(fd: u64, clock: TimerClockId, now: u64) -> Self {
        Self {
            fd, clock, state: TimerfdState::Disarmed,
            spec: TimerfdSpec { interval_ns: 0, value_ns: 0, absolute: false, cancel_on_set: false },
            created_at: now, armed_at: 0, expiration_count: 0, overrun_count: 0, total_reads: 0,
        }
    }

    #[inline]
    pub fn arm(&mut self, spec: TimerfdSpec, now: u64) {
        self.spec = spec;
        self.state = TimerfdState::Armed;
        self.armed_at = now;
    }

    #[inline(always)]
    pub fn disarm(&mut self) { self.state = TimerfdState::Disarmed; }

    #[inline(always)]
    pub fn expire(&mut self) {
        self.expiration_count += 1;
        if self.spec.interval_ns == 0 { self.state = TimerfdState::Expired; }
    }

    #[inline]
    pub fn read(&mut self) -> u64 {
        self.total_reads += 1;
        let count = self.expiration_count;
        self.expiration_count = 0;
        count
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdAppStats {
    pub total_timers: u32,
    pub armed_timers: u32,
    pub total_expirations: u64,
    pub total_overruns: u64,
    pub total_reads: u64,
}

/// Main app timerfd
#[repr(align(64))]
pub struct AppTimerfd {
    timers: BTreeMap<u64, TimerfdInstance>,
    next_fd: u64,
}

impl AppTimerfd {
    pub fn new() -> Self { Self { timers: BTreeMap::new(), next_fd: 1 } }

    #[inline]
    pub fn create(&mut self, clock: TimerClockId, now: u64) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.timers.insert(fd, TimerfdInstance::new(fd, clock, now));
        fd
    }

    #[inline(always)]
    pub fn settime(&mut self, fd: u64, spec: TimerfdSpec, now: u64) {
        if let Some(t) = self.timers.get_mut(&fd) { t.arm(spec, now); }
    }

    #[inline(always)]
    pub fn expire(&mut self, fd: u64) {
        if let Some(t) = self.timers.get_mut(&fd) { t.expire(); }
    }

    #[inline(always)]
    pub fn read(&mut self, fd: u64) -> u64 {
        if let Some(t) = self.timers.get_mut(&fd) { t.read() }
        else { 0 }
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.timers.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> TimerfdAppStats {
        let armed = self.timers.values().filter(|t| t.state == TimerfdState::Armed).count() as u32;
        let exps: u64 = self.timers.values().map(|t| t.expiration_count).sum();
        let overruns: u64 = self.timers.values().map(|t| t.overrun_count).sum();
        let reads: u64 = self.timers.values().map(|t| t.total_reads).sum();
        TimerfdAppStats { total_timers: self.timers.len() as u32, armed_timers: armed, total_expirations: exps, total_overruns: overruns, total_reads: reads }
    }
}
