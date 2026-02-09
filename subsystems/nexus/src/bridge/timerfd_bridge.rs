// SPDX-License-Identifier: GPL-2.0
//! Bridge timerfd_bridge — timerfd interface bridge for timer notification via fd.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Clock type for timerfd
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerClockType {
    Realtime,
    Monotonic,
    BoottimeAlarm,
    RealtimeAlarm,
}

/// Timerfd flags
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct TimerfdFlags(pub u32);

impl TimerfdFlags {
    pub const CLOEXEC: Self = Self(0x01);
    pub const NONBLOCK: Self = Self(0x02);
    pub const ABSTIME: Self = Self(0x04);
    pub const CANCEL_ON_SET: Self = Self(0x08);

    #[inline(always)]
    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// Timer spec with value + interval
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct TimerSpec {
    pub value_ns: u64,
    pub interval_ns: u64,
}

impl TimerSpec {
    #[inline(always)]
    pub fn oneshot(ns: u64) -> Self {
        Self { value_ns: ns, interval_ns: 0 }
    }

    #[inline(always)]
    pub fn periodic(interval_ns: u64) -> Self {
        Self { value_ns: interval_ns, interval_ns }
    }

    #[inline(always)]
    pub fn is_disarmed(&self) -> bool {
        self.value_ns == 0 && self.interval_ns == 0
    }

    #[inline(always)]
    pub fn is_periodic(&self) -> bool {
        self.interval_ns > 0
    }
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Disarmed,
    Armed,
    Expired,
    Cancelled,
}

/// A timerfd instance
#[derive(Debug)]
#[repr(align(64))]
pub struct TimerfdInstance {
    pub fd: i32,
    pub owner_pid: u32,
    pub clock: TimerClockType,
    pub flags: TimerfdFlags,
    pub spec: TimerSpec,
    pub state: TimerState,
    pub expirations: u64,
    pub unread_expirations: u64,
    pub arm_time_ns: u64,
    pub next_expiry_ns: u64,
    pub total_fires: u64,
    pub total_reads: u64,
    pub overruns: u64,
    pub created: u64,
    pub last_read: u64,
}

impl TimerfdInstance {
    pub fn new(fd: i32, owner: u32, clock: TimerClockType, flags: TimerfdFlags, now: u64) -> Self {
        Self {
            fd, owner_pid: owner, clock, flags,
            spec: TimerSpec { value_ns: 0, interval_ns: 0 },
            state: TimerState::Disarmed,
            expirations: 0, unread_expirations: 0,
            arm_time_ns: 0, next_expiry_ns: 0,
            total_fires: 0, total_reads: 0,
            overruns: 0, created: now, last_read: 0,
        }
    }

    #[inline]
    pub fn arm(&mut self, spec: TimerSpec, now: u64) {
        self.spec = spec;
        if spec.is_disarmed() {
            self.state = TimerState::Disarmed;
            self.next_expiry_ns = 0;
        } else {
            self.state = TimerState::Armed;
            self.arm_time_ns = now;
            self.next_expiry_ns = now + spec.value_ns;
        }
    }

    #[inline]
    pub fn disarm(&mut self) {
        self.spec = TimerSpec { value_ns: 0, interval_ns: 0 };
        self.state = TimerState::Disarmed;
        self.next_expiry_ns = 0;
    }

    pub fn fire(&mut self, now: u64) -> bool {
        if self.state != TimerState::Armed { return false; }
        if now < self.next_expiry_ns { return false; }

        let elapsed = now.saturating_sub(self.next_expiry_ns);
        self.expirations += 1;
        self.unread_expirations += 1;
        self.total_fires += 1;

        if self.spec.is_periodic() {
            let extra = if self.spec.interval_ns > 0 {
                elapsed / self.spec.interval_ns
            } else { 0 };
            self.overruns += extra;
            self.unread_expirations += extra;
            self.expirations += extra;
            self.next_expiry_ns = now + self.spec.interval_ns;
        } else {
            self.state = TimerState::Expired;
            self.next_expiry_ns = 0;
        }
        true
    }

    #[inline]
    pub fn read(&mut self, now: u64) -> u64 {
        let count = self.unread_expirations;
        self.unread_expirations = 0;
        self.total_reads += 1;
        self.last_read = now;
        count
    }

    #[inline(always)]
    pub fn is_readable(&self) -> bool {
        self.unread_expirations > 0
    }

    #[inline]
    pub fn time_until_expiry(&self, now: u64) -> Option<u64> {
        if self.state != TimerState::Armed { return None; }
        if now >= self.next_expiry_ns { return Some(0); }
        Some(self.next_expiry_ns - now)
    }

    #[inline(always)]
    pub fn overrun_rate(&self) -> f64 {
        if self.total_fires == 0 { return 0.0; }
        self.overruns as f64 / self.total_fires as f64
    }
}

/// Timerfd operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdOp {
    Create,
    Arm,
    Disarm,
    GetTime,
    Read,
    Close,
}

/// Timerfd event
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdEvent {
    pub fd: i32,
    pub op: TimerfdOp,
    pub pid: u32,
    pub timestamp: u64,
}

/// Timerfd bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdBridgeStats {
    pub active_timerfds: u32,
    pub total_created: u64,
    pub total_fires: u64,
    pub total_reads: u64,
    pub total_overruns: u64,
    pub armed_count: u32,
}

/// Main timerfd bridge
#[repr(align(64))]
pub struct BridgeTimerfd {
    instances: BTreeMap<i32, TimerfdInstance>,
    events: VecDeque<TimerfdEvent>,
    max_events: usize,
    stats: TimerfdBridgeStats,
}

impl BridgeTimerfd {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 2048,
            stats: TimerfdBridgeStats {
                active_timerfds: 0, total_created: 0,
                total_fires: 0, total_reads: 0,
                total_overruns: 0, armed_count: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self, fd: i32, owner: u32, clock: TimerClockType, flags: TimerfdFlags, now: u64) {
        let inst = TimerfdInstance::new(fd, owner, clock, flags, now);
        self.stats.total_created += 1;
        self.stats.active_timerfds += 1;
        self.instances.insert(fd, inst);
    }

    #[inline]
    pub fn arm(&mut self, fd: i32, spec: TimerSpec, now: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let was_armed = inst.state == TimerState::Armed;
            inst.arm(spec, now);
            if !was_armed && inst.state == TimerState::Armed {
                self.stats.armed_count += 1;
            }
            true
        } else { false }
    }

    #[inline]
    pub fn disarm(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.state == TimerState::Armed {
                if self.stats.armed_count > 0 { self.stats.armed_count -= 1; }
            }
            inst.disarm();
            true
        } else { false }
    }

    #[inline]
    pub fn tick(&mut self, now: u64) -> Vec<i32> {
        let mut fired = Vec::new();
        for (&fd, inst) in self.instances.iter_mut() {
            if inst.fire(now) {
                self.stats.total_fires += 1;
                fired.push(fd);
            }
        }
        fired
    }

    #[inline]
    pub fn read(&mut self, fd: i32, now: u64) -> Option<u64> {
        let inst = self.instances.get_mut(&fd)?;
        let count = inst.read(now);
        self.stats.total_reads += 1;
        Some(count)
    }

    #[inline]
    pub fn close(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.remove(&fd) {
            if inst.state == TimerState::Armed {
                if self.stats.armed_count > 0 { self.stats.armed_count -= 1; }
            }
            if self.stats.active_timerfds > 0 { self.stats.active_timerfds -= 1; }
            true
        } else { false }
    }

    #[inline(always)]
    pub fn record_event(&mut self, event: TimerfdEvent) {
        if self.events.len() >= self.max_events { self.events.pop_front(); }
        self.events.push_back(event);
    }

    #[inline]
    pub fn next_expiring(&self, now: u64) -> Option<(i32, u64)> {
        self.instances.iter()
            .filter(|(_, inst)| inst.state == TimerState::Armed)
            .filter_map(|(&fd, inst)| inst.time_until_expiry(now).map(|t| (fd, t)))
            .min_by_key(|&(_, t)| t)
    }

    #[inline]
    pub fn overrun_summary(&self) -> Vec<(i32, u64, f64)> {
        let mut v: Vec<_> = self.instances.iter()
            .filter(|(_, inst)| inst.overruns > 0)
            .map(|(&fd, inst)| (fd, inst.overruns, inst.overrun_rate()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    #[inline(always)]
    pub fn stats(&self) -> &TimerfdBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from timerfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdV2Clock {
    Realtime,
    Monotonic,
    BootTime,
    RealtimeAlarm,
    BootTimeAlarm,
}

/// Timer v2 flags
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct TimerfdV2Flags(pub u32);

impl TimerfdV2Flags {
    pub const NONBLOCK: u32 = 1;
    pub const CLOEXEC: u32 = 2;
    pub const ABSTIME: u32 = 4;
    pub const CANCEL_ON_SET: u32 = 8;
    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Timer v2 spec
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct TimerfdV2Spec {
    pub interval_ns: u64,
    pub value_ns: u64,
}

/// Timer fd v2 instance
#[derive(Debug)]
#[repr(align(64))]
pub struct TimerfdV2Instance {
    pub id: u64,
    pub clock: TimerfdV2Clock,
    pub flags: TimerfdV2Flags,
    pub spec: TimerfdV2Spec,
    pub armed: bool,
    pub expirations: u64,
    pub overruns: u64,
    pub created_at: u64,
    pub last_fire: u64,
    pub owner_pid: u64,
}

impl TimerfdV2Instance {
    pub fn new(id: u64, clock: TimerfdV2Clock, pid: u64, now: u64) -> Self {
        Self { id, clock, flags: TimerfdV2Flags::new(), spec: TimerfdV2Spec { interval_ns: 0, value_ns: 0 }, armed: false, expirations: 0, overruns: 0, created_at: now, last_fire: 0, owner_pid: pid }
    }

    #[inline(always)]
    pub fn arm(&mut self, spec: TimerfdV2Spec) { self.spec = spec; self.armed = true; }
    #[inline(always)]
    pub fn disarm(&mut self) { self.armed = false; }

    #[inline]
    pub fn fire(&mut self, now: u64) {
        self.expirations += 1;
        self.last_fire = now;
        if self.spec.interval_ns == 0 { self.armed = false; }
    }

    #[inline(always)]
    pub fn is_periodic(&self) -> bool { self.spec.interval_ns > 0 }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdV2BridgeStats {
    pub total_timers: u32,
    pub armed_timers: u32,
    pub periodic_timers: u32,
    pub total_expirations: u64,
    pub total_overruns: u64,
}

/// Main timerfd v2 bridge
#[repr(align(64))]
pub struct BridgeTimerfdV2 {
    timers: BTreeMap<u64, TimerfdV2Instance>,
    next_id: u64,
}

impl BridgeTimerfdV2 {
    pub fn new() -> Self { Self { timers: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, clock: TimerfdV2Clock, pid: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.timers.insert(id, TimerfdV2Instance::new(id, clock, pid, now));
        id
    }

    #[inline(always)]
    pub fn close(&mut self, id: u64) { self.timers.remove(&id); }

    #[inline(always)]
    pub fn arm(&mut self, id: u64, spec: TimerfdV2Spec) {
        if let Some(t) = self.timers.get_mut(&id) { t.arm(spec); }
    }

    #[inline]
    pub fn tick(&mut self, now: u64) {
        for timer in self.timers.values_mut() {
            if timer.armed && timer.spec.value_ns > 0 && now >= timer.created_at + timer.spec.value_ns {
                timer.fire(now);
            }
        }
    }

    #[inline]
    pub fn stats(&self) -> TimerfdV2BridgeStats {
        let armed = self.timers.values().filter(|t| t.armed).count() as u32;
        let periodic = self.timers.values().filter(|t| t.is_periodic()).count() as u32;
        let exps: u64 = self.timers.values().map(|t| t.expirations).sum();
        let overruns: u64 = self.timers.values().map(|t| t.overruns).sum();
        TimerfdV2BridgeStats { total_timers: self.timers.len() as u32, armed_timers: armed, periodic_timers: periodic, total_expirations: exps, total_overruns: overruns }
    }
}

// ============================================================================
// Merged from timerfd_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerV3Clock {
    Realtime,
    Monotonic,
    BootTime,
    RealtimeAlarm,
    BootTimeAlarm,
}

/// Timer v3 state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerV3State {
    Disarmed,
    Armed,
    Expired,
}

/// Timer v3 entry
#[derive(Debug)]
#[repr(align(64))]
pub struct TimerV3Entry {
    pub fd: u64,
    pub clock: TimerV3Clock,
    pub state: TimerV3State,
    pub interval_ns: u64,
    pub value_ns: u64,
    pub absolute: bool,
    pub expiration_count: u64,
    pub overrun_count: u64,
    pub created_at: u64,
    pub armed_at: u64,
}

impl TimerV3Entry {
    pub fn new(fd: u64, clock: TimerV3Clock, now: u64) -> Self {
        Self { fd, clock, state: TimerV3State::Disarmed, interval_ns: 0, value_ns: 0, absolute: false, expiration_count: 0, overrun_count: 0, created_at: now, armed_at: 0 }
    }

    #[inline]
    pub fn arm(&mut self, value: u64, interval: u64, absolute: bool, now: u64) {
        self.value_ns = value;
        self.interval_ns = interval;
        self.absolute = absolute;
        self.state = TimerV3State::Armed;
        self.armed_at = now;
    }

    #[inline(always)]
    pub fn disarm(&mut self) { self.state = TimerV3State::Disarmed; }

    #[inline(always)]
    pub fn expire(&mut self) {
        self.expiration_count += 1;
        if self.interval_ns == 0 { self.state = TimerV3State::Expired; }
    }

    #[inline]
    pub fn read(&mut self) -> u64 {
        let c = self.expiration_count;
        self.expiration_count = 0;
        c
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdV3BridgeStats {
    pub total_timers: u32,
    pub armed: u32,
    pub total_expirations: u64,
    pub total_overruns: u64,
}

/// Main bridge timerfd v3
#[repr(align(64))]
pub struct BridgeTimerfdV3 {
    timers: BTreeMap<u64, TimerV3Entry>,
    next_fd: u64,
}

impl BridgeTimerfdV3 {
    pub fn new() -> Self { Self { timers: BTreeMap::new(), next_fd: 1 } }

    #[inline]
    pub fn create(&mut self, clock: TimerV3Clock, now: u64) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.timers.insert(fd, TimerV3Entry::new(fd, clock, now));
        fd
    }

    #[inline(always)]
    pub fn settime(&mut self, fd: u64, value: u64, interval: u64, abs: bool, now: u64) {
        if let Some(t) = self.timers.get_mut(&fd) { t.arm(value, interval, abs, now); }
    }

    #[inline(always)]
    pub fn expire(&mut self, fd: u64) {
        if let Some(t) = self.timers.get_mut(&fd) { t.expire(); }
    }

    #[inline(always)]
    pub fn read(&mut self, fd: u64) -> u64 {
        if let Some(t) = self.timers.get_mut(&fd) { t.read() } else { 0 }
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.timers.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> TimerfdV3BridgeStats {
        let armed = self.timers.values().filter(|t| t.state == TimerV3State::Armed).count() as u32;
        let exps: u64 = self.timers.values().map(|t| t.expiration_count).sum();
        let overruns: u64 = self.timers.values().map(|t| t.overrun_count).sum();
        TimerfdV3BridgeStats { total_timers: self.timers.len() as u32, armed, total_expirations: exps, total_overruns: overruns }
    }
}

// ============================================================================
// Merged from timerfd_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdV4Clock {
    Realtime,
    Monotonic,
    BoottimeAlarm,
    RealtimeAlarm,
    Boottime,
    Tai,
}

/// Timer flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdV4Flag {
    Abstime,
    CancelOnSet,
    NonBlock,
    CloseExec,
}

/// Timer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerfdV4State {
    Disarmed,
    Armed,
    Expired,
    Cancelled,
    Suspended,
}

/// A timerfd instance.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdV4Instance {
    pub timer_id: u64,
    pub fd: i32,
    pub clock: TimerfdV4Clock,
    pub state: TimerfdV4State,
    pub flags: Vec<TimerfdV4Flag>,
    pub initial_expiry_ns: u64,
    pub interval_ns: u64,
    pub expirations: u64,
    pub cpu_affinity: Option<u32>,
    pub cancel_on_set: bool,
    pub settime_count: u64,
    pub read_count: u64,
}

impl TimerfdV4Instance {
    pub fn new(timer_id: u64, fd: i32, clock: TimerfdV4Clock) -> Self {
        Self {
            timer_id,
            fd,
            clock,
            state: TimerfdV4State::Disarmed,
            flags: Vec::new(),
            initial_expiry_ns: 0,
            interval_ns: 0,
            expirations: 0,
            cpu_affinity: None,
            cancel_on_set: false,
            settime_count: 0,
            read_count: 0,
        }
    }

    #[inline]
    pub fn arm(&mut self, expiry_ns: u64, interval_ns: u64) {
        self.initial_expiry_ns = expiry_ns;
        self.interval_ns = interval_ns;
        self.state = TimerfdV4State::Armed;
        self.settime_count += 1;
    }

    #[inline]
    pub fn disarm(&mut self) {
        self.state = TimerfdV4State::Disarmed;
        self.initial_expiry_ns = 0;
        self.interval_ns = 0;
    }

    #[inline]
    pub fn expire(&mut self) {
        self.expirations += 1;
        if self.interval_ns == 0 {
            self.state = TimerfdV4State::Expired;
        }
        // periodic timers stay armed
    }

    #[inline]
    pub fn read_expirations(&mut self) -> u64 {
        let count = self.expirations;
        self.expirations = 0;
        self.read_count += 1;
        count
    }

    #[inline(always)]
    pub fn is_periodic(&self) -> bool {
        self.interval_ns > 0
    }

    #[inline]
    pub fn is_alarm(&self) -> bool {
        matches!(
            self.clock,
            TimerfdV4Clock::BoottimeAlarm | TimerfdV4Clock::RealtimeAlarm
        )
    }
}

/// Statistics for timerfd V4 bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerfdV4BridgeStats {
    pub total_timers: u64,
    pub armed_timers: u64,
    pub total_expirations: u64,
    pub periodic_timers: u64,
    pub alarm_timers: u64,
    pub cancel_on_set_count: u64,
    pub settime_calls: u64,
    pub read_calls: u64,
}

/// Main bridge timerfd V4 manager.
#[repr(align(64))]
pub struct BridgeTimerfdV4 {
    pub timers: BTreeMap<u64, TimerfdV4Instance>,
    pub fd_to_timer: BTreeMap<i32, u64>,
    pub next_timer_id: u64,
    pub stats: TimerfdV4BridgeStats,
}

impl BridgeTimerfdV4 {
    pub fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
            fd_to_timer: BTreeMap::new(),
            next_timer_id: 1,
            stats: TimerfdV4BridgeStats {
                total_timers: 0,
                armed_timers: 0,
                total_expirations: 0,
                periodic_timers: 0,
                alarm_timers: 0,
                cancel_on_set_count: 0,
                settime_calls: 0,
                read_calls: 0,
            },
        }
    }

    pub fn create_timer(&mut self, fd: i32, clock: TimerfdV4Clock) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        let timer = TimerfdV4Instance::new(id, fd, clock);
        if timer.is_alarm() {
            self.stats.alarm_timers += 1;
        }
        self.fd_to_timer.insert(fd, id);
        self.timers.insert(id, timer);
        self.stats.total_timers += 1;
        id
    }

    pub fn arm_timer(&mut self, timer_id: u64, expiry_ns: u64, interval_ns: u64) -> bool {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.arm(expiry_ns, interval_ns);
            self.stats.armed_timers += 1;
            self.stats.settime_calls += 1;
            if interval_ns > 0 {
                self.stats.periodic_timers += 1;
            }
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn expire_timer(&mut self, timer_id: u64) -> bool {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.expire();
            self.stats.total_expirations += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn timer_count(&self) -> usize {
        self.timers.len()
    }
}
