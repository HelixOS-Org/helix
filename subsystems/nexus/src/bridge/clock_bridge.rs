//! # Bridge Clock Bridge
//!
//! Bridges clock/timer hardware between kernel and devices:
//! - Clock source registration and selection
//! - Timer hardware abstraction
//! - Clockevent device management
//! - Timekeeping synchronization
//! - Watchdog clock verification
//! - High-resolution timer support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Clock source rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockRating {
    Unusable,
    Low,
    Normal,
    Good,
    Perfect,
    Ideal,
}

/// Clock source flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockFlag {
    Continuous,
    MustVerify,
    ValidForHres,
    Watchdog,
    Unstable,
    Suspended,
}

/// Clock source
#[derive(Debug, Clone)]
pub struct ClockSource {
    pub id: u64,
    pub name: String,
    pub rating: ClockRating,
    pub freq_hz: u64,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
    pub max_idle_ns: u64,
    pub flags: Vec<ClockFlag>,
    pub read_count: u64,
    pub uncertainty_ns: u64,
    pub last_read: u64,
    pub cycle_last: u64,
}

impl ClockSource {
    pub fn new(id: u64, name: String, freq: u64, rating: ClockRating) -> Self {
        let shift = 20u32;
        let mult = if freq > 0 { ((1u64 << shift) * 1_000_000_000 / freq) as u32 } else { 1 };
        Self {
            id, name, rating, freq_hz: freq, mask: u64::MAX,
            mult, shift, max_idle_ns: 1_000_000_000,
            flags: Vec::new(), read_count: 0, uncertainty_ns: 0,
            last_read: 0, cycle_last: 0,
        }
    }

    #[inline]
    pub fn read(&mut self, cycles: u64) -> u64 {
        self.read_count += 1;
        self.cycle_last = cycles;
        let ns = (cycles as u128 * self.mult as u128) >> self.shift;
        self.last_read = ns as u64;
        ns as u64
    }

    #[inline(always)]
    pub fn is_stable(&self) -> bool { !self.flags.contains(&ClockFlag::Unstable) }
    #[inline(always)]
    pub fn is_hres_valid(&self) -> bool { self.flags.contains(&ClockFlag::ValidForHres) }
    #[inline(always)]
    pub fn resolution_ns(&self) -> u64 { if self.freq_hz == 0 { u64::MAX } else { 1_000_000_000 / self.freq_hz } }
}

/// Clock event mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockEventMode {
    Unused,
    Periodic,
    OneShot,
    OneGlobalShot,
    Shutdown,
}

/// Clock event device
#[derive(Debug, Clone)]
pub struct ClockEventDevice {
    pub id: u64,
    pub name: String,
    pub rating: ClockRating,
    pub mode: ClockEventMode,
    pub cpu_id: u32,
    pub freq_hz: u64,
    pub min_delta_ns: u64,
    pub max_delta_ns: u64,
    pub event_count: u64,
    pub next_event_ns: u64,
}

impl ClockEventDevice {
    pub fn new(id: u64, name: String, cpu: u32, freq: u64) -> Self {
        Self {
            id, name, rating: ClockRating::Normal, mode: ClockEventMode::Unused,
            cpu_id: cpu, freq_hz: freq, min_delta_ns: 1000,
            max_delta_ns: 1_000_000_000_000, event_count: 0, next_event_ns: 0,
        }
    }

    #[inline(always)]
    pub fn set_mode(&mut self, mode: ClockEventMode) { self.mode = mode; }
    #[inline(always)]
    pub fn program_event(&mut self, ns: u64) { self.next_event_ns = ns; }
    #[inline(always)]
    pub fn fire(&mut self) { self.event_count += 1; }
}

/// Timekeeping state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimekeepingState {
    pub clock_id: u64,
    pub xtime_sec: u64,
    pub xtime_nsec: u64,
    pub wall_to_monotonic_sec: i64,
    pub wall_to_monotonic_nsec: i64,
    pub boot_ns: u64,
    pub tai_offset: i32,
    pub ntp_error_ns: i64,
    pub ntp_err_mult: u32,
    pub seq: u32,
}

impl TimekeepingState {
    pub fn new() -> Self {
        Self {
            clock_id: 0, xtime_sec: 0, xtime_nsec: 0,
            wall_to_monotonic_sec: 0, wall_to_monotonic_nsec: 0,
            boot_ns: 0, tai_offset: 0, ntp_error_ns: 0, ntp_err_mult: 0, seq: 0,
        }
    }

    #[inline]
    pub fn monotonic_ns(&self) -> u64 {
        let wall_ns = self.xtime_sec * 1_000_000_000 + self.xtime_nsec;
        let mono_off = self.wall_to_monotonic_sec * 1_000_000_000 + self.wall_to_monotonic_nsec;
        (wall_ns as i64 + mono_off) as u64
    }

    #[inline(always)]
    pub fn update_wall(&mut self, sec: u64, nsec: u64) {
        self.xtime_sec = sec; self.xtime_nsec = nsec; self.seq += 1;
    }
}

/// Clock bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ClockBridgeStats {
    pub total_sources: usize,
    pub total_events: usize,
    pub current_source_id: u64,
    pub total_reads: u64,
    pub total_timer_fires: u64,
    pub unstable_sources: usize,
}

/// Bridge clock manager
#[repr(align(64))]
pub struct BridgeClockBridge {
    sources: BTreeMap<u64, ClockSource>,
    events: BTreeMap<u64, ClockEventDevice>,
    timekeeping: TimekeepingState,
    current_source: u64,
    watchdog_source: Option<u64>,
    stats: ClockBridgeStats,
    next_id: u64,
}

impl BridgeClockBridge {
    pub fn new() -> Self {
        Self {
            sources: BTreeMap::new(), events: BTreeMap::new(),
            timekeeping: TimekeepingState::new(), current_source: 0,
            watchdog_source: None, stats: ClockBridgeStats::default(), next_id: 1,
        }
    }

    #[inline]
    pub fn register_source(&mut self, name: String, freq: u64, rating: ClockRating) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let src = ClockSource::new(id, name, freq, rating);
        self.sources.insert(id, src);
        self.select_best_source();
        id
    }

    #[inline]
    pub fn register_event_device(&mut self, name: String, cpu: u32, freq: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.events.insert(id, ClockEventDevice::new(id, name, cpu, freq));
        id
    }

    fn select_best_source(&mut self) {
        let best = self.sources.values().filter(|s| s.is_stable()).max_by_key(|s| s.rating);
        if let Some(b) = best { self.current_source = b.id; self.timekeeping.clock_id = b.id; }
    }

    #[inline(always)]
    pub fn read_clock(&mut self, cycles: u64) -> u64 {
        if let Some(s) = self.sources.get_mut(&self.current_source) { s.read(cycles) } else { 0 }
    }

    #[inline(always)]
    pub fn mark_unstable(&mut self, id: u64) {
        if let Some(s) = self.sources.get_mut(&id) { if !s.flags.contains(&ClockFlag::Unstable) { s.flags.push(ClockFlag::Unstable); } }
        if self.current_source == id { self.select_best_source(); }
    }

    #[inline(always)]
    pub fn set_watchdog(&mut self, id: u64) { self.watchdog_source = Some(id); }

    #[inline(always)]
    pub fn program_timer(&mut self, dev_id: u64, ns: u64) {
        if let Some(d) = self.events.get_mut(&dev_id) { d.program_event(ns); }
    }

    #[inline(always)]
    pub fn fire_timer(&mut self, dev_id: u64) {
        if let Some(d) = self.events.get_mut(&dev_id) { d.fire(); }
    }

    #[inline(always)]
    pub fn update_wall_time(&mut self, sec: u64, nsec: u64) { self.timekeeping.update_wall(sec, nsec); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_sources = self.sources.len();
        self.stats.total_events = self.events.len();
        self.stats.current_source_id = self.current_source;
        self.stats.total_reads = self.sources.values().map(|s| s.read_count).sum();
        self.stats.total_timer_fires = self.events.values().map(|e| e.event_count).sum();
        self.stats.unstable_sources = self.sources.values().filter(|s| !s.is_stable()).count();
    }

    #[inline(always)]
    pub fn source(&self, id: u64) -> Option<&ClockSource> { self.sources.get(&id) }
    #[inline(always)]
    pub fn event_device(&self, id: u64) -> Option<&ClockEventDevice> { self.events.get(&id) }
    #[inline(always)]
    pub fn timekeeping(&self) -> &TimekeepingState { &self.timekeeping }
    #[inline(always)]
    pub fn stats(&self) -> &ClockBridgeStats { &self.stats }
}
