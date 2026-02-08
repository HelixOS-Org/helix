// SPDX-License-Identifier: GPL-2.0
//! Holistic clock_source — system clock source management and timekeeping.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Clock source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSourceType {
    Tsc,
    Hpet,
    Acpi,
    Pit,
    Lapic,
    ArmGenericTimer,
    Virtual,
    External,
}

/// Clock source quality rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockQuality {
    Perfect = 0,
    Good = 1,
    Acceptable = 2,
    Degraded = 3,
    Unstable = 4,
    Unreliable = 5,
}

/// Clock source state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockState {
    Active,
    Standby,
    Calibrating,
    Suspended,
    Failed,
}

/// Clock source flags
#[derive(Debug, Clone, Copy)]
pub struct ClockFlags {
    pub bits: u32,
}

impl ClockFlags {
    pub const CONTINUOUS: u32 = 1 << 0;
    pub const MONOTONIC: u32 = 1 << 1;
    pub const STABLE_FREQ: u32 = 1 << 2;
    pub const WATCHDOG_OK: u32 = 1 << 3;
    pub const NONSTOP: u32 = 1 << 4;
    pub const PER_CPU: u32 = 1 << 5;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    pub fn is_continuous(&self) -> bool { self.has(Self::CONTINUOUS) }
    pub fn is_monotonic(&self) -> bool { self.has(Self::MONOTONIC) }
}

/// Clock source descriptor
#[derive(Debug, Clone)]
pub struct ClockSource {
    pub id: u32,
    pub name: String,
    pub source_type: ClockSourceType,
    pub state: ClockState,
    pub quality: ClockQuality,
    pub flags: ClockFlags,
    pub frequency_hz: u64,
    pub mask: u64,
    pub mult: u32,
    pub shift: u32,
    pub max_idle_ns: u64,
    pub uncertainty_ns: u64,
    pub last_read: u64,
    pub read_count: u64,
}

impl ClockSource {
    pub fn new(id: u32, name: String, stype: ClockSourceType, freq: u64) -> Self {
        Self {
            id, name, source_type: stype, state: ClockState::Standby,
            quality: ClockQuality::Acceptable, flags: ClockFlags::new(0),
            frequency_hz: freq, mask: u64::MAX, mult: 1, shift: 0,
            max_idle_ns: 0, uncertainty_ns: 0, last_read: 0, read_count: 0,
        }
    }

    pub fn read(&mut self, raw_counter: u64) -> u64 {
        let masked = raw_counter & self.mask;
        self.last_read = masked;
        self.read_count += 1;
        (masked.wrapping_mul(self.mult as u64)) >> self.shift
    }

    pub fn cycles_to_ns(&self, cycles: u64) -> u64 {
        if self.frequency_hz == 0 { return 0; }
        (cycles as u128 * 1_000_000_000 / self.frequency_hz as u128) as u64
    }

    pub fn ns_to_cycles(&self, ns: u64) -> u64 {
        (ns as u128 * self.frequency_hz as u128 / 1_000_000_000) as u64
    }

    pub fn is_suitable(&self) -> bool {
        self.state == ClockState::Active && self.quality <= ClockQuality::Acceptable
    }
}

/// Watchdog state for clock verification
#[derive(Debug, Clone)]
pub struct ClockWatchdog {
    pub reference_id: u32,
    pub last_check: u64,
    pub check_interval_ns: u64,
    pub max_skew_ns: u64,
    pub total_checks: u64,
    pub skew_violations: u64,
}

impl ClockWatchdog {
    pub fn new(ref_id: u32, interval: u64, max_skew: u64) -> Self {
        Self {
            reference_id: ref_id, last_check: 0, check_interval_ns: interval,
            max_skew_ns: max_skew, total_checks: 0, skew_violations: 0,
        }
    }

    pub fn check(&mut self, reference_ns: u64, measured_ns: u64, now: u64) -> bool {
        self.total_checks += 1;
        self.last_check = now;
        let diff = if reference_ns > measured_ns { reference_ns - measured_ns } else { measured_ns - reference_ns };
        if diff > self.max_skew_ns {
            self.skew_violations += 1;
            false
        } else { true }
    }

    pub fn violation_rate(&self) -> f64 {
        if self.total_checks == 0 { return 0.0; }
        self.skew_violations as f64 / self.total_checks as f64
    }
}

/// Clock source stats
#[derive(Debug, Clone)]
pub struct ClockSourceStats {
    pub total_sources: u32,
    pub active_source: Option<u32>,
    pub total_reads: u64,
    pub watchdog_violations: u64,
    pub source_switches: u64,
}

/// Main clock source manager
pub struct HolisticClockSource {
    sources: BTreeMap<u32, ClockSource>,
    active_id: Option<u32>,
    watchdog: Option<ClockWatchdog>,
    source_switches: u64,
    next_id: u32,
}

impl HolisticClockSource {
    pub fn new() -> Self {
        Self { sources: BTreeMap::new(), active_id: None, watchdog: None, source_switches: 0, next_id: 1 }
    }

    pub fn register(&mut self, name: String, stype: ClockSourceType, freq: u64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.sources.insert(id, ClockSource::new(id, name, stype, freq));
        id
    }

    pub fn activate(&mut self, id: u32) -> bool {
        if let Some(src) = self.sources.get_mut(&id) {
            src.state = ClockState::Active;
            if self.active_id != Some(id) { self.source_switches += 1; }
            self.active_id = Some(id);
            true
        } else { false }
    }

    pub fn select_best(&mut self) -> Option<u32> {
        let best = self.sources.values()
            .filter(|s| s.state != ClockState::Failed)
            .min_by_key(|s| (s.quality as u32, s.uncertainty_ns))
            .map(|s| s.id);
        if let Some(id) = best { self.activate(id); }
        best
    }

    pub fn read_active(&mut self, raw: u64) -> Option<u64> {
        let id = self.active_id?;
        self.sources.get_mut(&id).map(|s| s.read(raw))
    }

    pub fn stats(&self) -> ClockSourceStats {
        let total_reads: u64 = self.sources.values().map(|s| s.read_count).sum();
        let violations = self.watchdog.as_ref().map(|w| w.skew_violations).unwrap_or(0);
        ClockSourceStats {
            total_sources: self.sources.len() as u32,
            active_source: self.active_id,
            total_reads, watchdog_violations: violations,
            source_switches: self.source_switches,
        }
    }
}
