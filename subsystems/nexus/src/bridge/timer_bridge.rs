//! # Bridge Timer Bridge
//!
//! Timer management bridge between kernel and userspace:
//! - POSIX timer emulation (timer_create/settime/gettime)
//! - timerfd abstraction
//! - High-resolution timer tracking
//! - Timer coalescing for power savings
//! - Periodic vs one-shot management
//! - Timer wheel implementation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    OneShot,
    Periodic,
    Deadline,
    Watchdog,
}

/// Timer clock source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    Monotonic,
    Realtime,
    BootTime,
    ProcessCpuTime,
    ThreadCpuTime,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Disarmed,
    Armed,
    Expired,
    Cancelled,
}

/// Timer entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerEntry {
    pub timer_id: u64,
    pub owner_pid: u64,
    pub timer_type: TimerType,
    pub clock: ClockSource,
    pub state: TimerState,
    pub interval_ns: u64,
    pub expiry_ns: u64,
    pub overrun_count: u32,
    pub fire_count: u64,
    pub coalesce_window_ns: u64,
    pub created_ts: u64,
}

impl TimerEntry {
    pub fn new(id: u64, pid: u64, timer_type: TimerType, clock: ClockSource) -> Self {
        Self {
            timer_id: id,
            owner_pid: pid,
            timer_type,
            clock,
            state: TimerState::Disarmed,
            interval_ns: 0,
            expiry_ns: 0,
            overrun_count: 0,
            fire_count: 0,
            coalesce_window_ns: 0,
            created_ts: 0,
        }
    }

    #[inline]
    pub fn arm(&mut self, expiry_ns: u64, interval_ns: u64) {
        self.expiry_ns = expiry_ns;
        self.interval_ns = interval_ns;
        self.state = TimerState::Armed;
    }

    #[inline(always)]
    pub fn disarm(&mut self) {
        self.state = TimerState::Disarmed;
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        self.state == TimerState::Armed && now >= self.expiry_ns
    }

    /// Fire the timer; returns true if rearmed
    pub fn fire(&mut self, now: u64) -> bool {
        self.fire_count += 1;

        if now > self.expiry_ns && self.interval_ns > 0 {
            let elapsed = now - self.expiry_ns;
            self.overrun_count = (elapsed / self.interval_ns.max(1)) as u32;
        }

        match self.timer_type {
            TimerType::Periodic => {
                self.expiry_ns += self.interval_ns;
                // Skip past if behind
                while self.expiry_ns <= now && self.interval_ns > 0 {
                    self.expiry_ns += self.interval_ns;
                    self.overrun_count += 1;
                }
                true
            }
            _ => {
                self.state = TimerState::Expired;
                false
            }
        }
    }
}

/// Timer wheel level
#[derive(Debug, Clone)]
pub struct WheelLevel {
    pub slots: Vec<Vec<u64>>, // timer IDs per slot
    pub current_slot: u32,
    pub slot_count: u32,
    pub resolution_ns: u64,
}

impl WheelLevel {
    pub fn new(slot_count: u32, resolution_ns: u64) -> Self {
        let mut slots = Vec::with_capacity(slot_count as usize);
        for _ in 0..slot_count {
            slots.push(Vec::new());
        }
        Self {
            slots,
            current_slot: 0,
            slot_count,
            resolution_ns,
        }
    }

    #[inline]
    pub fn insert(&mut self, timer_id: u64, ticks_from_now: u64) {
        let slot = ((self.current_slot as u64 + ticks_from_now) % self.slot_count as u64) as usize;
        if slot < self.slots.len() {
            self.slots[slot].push(timer_id);
        }
    }

    #[inline]
    pub fn advance(&mut self) -> Vec<u64> {
        self.current_slot = (self.current_slot + 1) % self.slot_count;
        let slot = self.current_slot as usize;
        if slot < self.slots.len() {
            let expired = core::mem::take(&mut self.slots[slot]);
            expired
        } else {
            Vec::new()
        }
    }
}

/// Coalesce group
#[derive(Debug, Clone)]
pub struct CoalesceGroup {
    pub window_ns: u64,
    pub timer_ids: Vec<u64>,
    pub earliest_expiry: u64,
    pub coalesced_expiry: u64,
}

/// Timer bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeTimerBridgeStats {
    pub total_timers: usize,
    pub armed_timers: usize,
    pub periodic_timers: usize,
    pub total_fires: u64,
    pub total_overruns: u64,
    pub coalesced_groups: usize,
    pub coalesced_timers: usize,
}

/// Bridge Timer Bridge
#[repr(align(64))]
pub struct BridgeTimerBridge {
    timers: BTreeMap<u64, TimerEntry>,
    wheel: WheelLevel,
    coalesce_groups: Vec<CoalesceGroup>,
    next_timer_id: u64,
    coalesce_window_ns: u64,
    stats: BridgeTimerBridgeStats,
}

impl BridgeTimerBridge {
    pub fn new(wheel_slots: u32, wheel_resolution_ns: u64) -> Self {
        Self {
            timers: BTreeMap::new(),
            wheel: WheelLevel::new(wheel_slots, wheel_resolution_ns),
            coalesce_groups: Vec::new(),
            next_timer_id: 1,
            coalesce_window_ns: 1_000_000, // 1ms default
            stats: BridgeTimerBridgeStats::default(),
        }
    }

    #[inline]
    pub fn create_timer(&mut self, pid: u64, timer_type: TimerType, clock: ClockSource, now: u64) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        let mut timer = TimerEntry::new(id, pid, timer_type, clock);
        timer.created_ts = now;
        self.timers.insert(id, timer);
        self.recompute();
        id
    }

    pub fn arm_timer(&mut self, timer_id: u64, expiry_ns: u64, interval_ns: u64, now: u64) -> bool {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.arm(expiry_ns, interval_ns);

            // Insert into wheel
            let ticks = (expiry_ns.saturating_sub(now)) / self.wheel.resolution_ns.max(1);
            self.wheel.insert(timer_id, ticks);

            self.recompute();
            true
        } else { false }
    }

    #[inline]
    pub fn disarm_timer(&mut self, timer_id: u64) -> bool {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.disarm();
            self.recompute();
            true
        } else { false }
    }

    #[inline]
    pub fn delete_timer(&mut self, timer_id: u64) -> bool {
        let removed = self.timers.remove(&timer_id).is_some();
        if removed { self.recompute(); }
        removed
    }

    /// Tick the timer wheel; returns fired timer IDs
    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let candidates = self.wheel.advance();
        let mut fired = Vec::new();

        for timer_id in candidates {
            if let Some(timer) = self.timers.get_mut(&timer_id) {
                if timer.is_expired(now) || timer.state == TimerState::Armed {
                    let rearmed = timer.fire(now);
                    fired.push(timer_id);

                    if rearmed {
                        let ticks = timer.interval_ns / self.wheel.resolution_ns.max(1);
                        self.wheel.insert(timer_id, ticks);
                    }
                }
            }
        }

        if !fired.is_empty() { self.recompute(); }
        fired
    }

    /// Coalesce nearby timers
    pub fn coalesce(&mut self) {
        self.coalesce_groups.clear();

        let mut armed: Vec<(u64, u64)> = self.timers.iter()
            .filter(|(_, t)| t.state == TimerState::Armed && t.coalesce_window_ns > 0)
            .map(|(&id, t)| (id, t.expiry_ns))
            .collect();

        armed.sort_by_key(|&(_, exp)| exp);

        let mut i = 0;
        while i < armed.len() {
            let mut group = CoalesceGroup {
                window_ns: self.coalesce_window_ns,
                timer_ids: alloc::vec![armed[i].0],
                earliest_expiry: armed[i].1,
                coalesced_expiry: armed[i].1,
            };

            let mut j = i + 1;
            while j < armed.len() && armed[j].1 - armed[i].1 <= self.coalesce_window_ns {
                group.timer_ids.push(armed[j].0);
                group.coalesced_expiry = armed[j].1;
                j += 1;
            }

            if group.timer_ids.len() > 1 {
                self.coalesce_groups.push(group);
            }
            i = j;
        }

        self.stats.coalesced_groups = self.coalesce_groups.len();
        self.stats.coalesced_timers = self.coalesce_groups.iter()
            .map(|g| g.timer_ids.len())
            .sum();
    }

    fn recompute(&mut self) {
        self.stats.total_timers = self.timers.len();
        self.stats.armed_timers = self.timers.values().filter(|t| t.state == TimerState::Armed).count();
        self.stats.periodic_timers = self.timers.values()
            .filter(|t| t.timer_type == TimerType::Periodic && t.state == TimerState::Armed)
            .count();
        self.stats.total_fires = self.timers.values().map(|t| t.fire_count).sum();
        self.stats.total_overruns = self.timers.values().map(|t| t.overrun_count as u64).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeTimerBridgeStats {
        &self.stats
    }

    #[inline(always)]
    pub fn timer(&self, id: u64) -> Option<&TimerEntry> {
        self.timers.get(&id)
    }
}
