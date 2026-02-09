//! # Holistic Timer Wheel
//!
//! Hierarchical timer wheel for efficient timer management:
//! - Multi-level timer wheel (nanosecond to minute granularity)
//! - Cascading timer promotion/demotion
//! - Timer coalescing for power savings
//! - High-resolution timer support
//! - Timer statistics and jitter tracking
//! - Batch timer expiry processing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Pending,
    Armed,
    Expired,
    Cancelled,
    Migrated,
}

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    OneShot,
    Periodic,
    HighResolution,
    Deferrable,
    Pinned,
}

/// Timer entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerEntry {
    pub timer_id: u64,
    pub timer_type: TimerType,
    pub state: TimerState,
    pub expires_ns: u64,
    pub interval_ns: u64,
    pub callback_id: u64,
    pub cpu_affinity: Option<u32>,
    pub created_ts: u64,
    pub fired_count: u64,
    pub total_jitter_ns: u64,
    pub max_jitter_ns: u64,
    pub coalesced: bool,
}

impl TimerEntry {
    pub fn new(id: u64, ttype: TimerType, expires: u64, interval: u64, cb: u64, ts: u64) -> Self {
        Self {
            timer_id: id, timer_type: ttype, state: TimerState::Pending,
            expires_ns: expires, interval_ns: interval, callback_id: cb,
            cpu_affinity: None, created_ts: ts, fired_count: 0,
            total_jitter_ns: 0, max_jitter_ns: 0, coalesced: false,
        }
    }

    pub fn fire(&mut self, actual_ts: u64) {
        let jitter = if actual_ts > self.expires_ns {
            actual_ts - self.expires_ns
        } else {
            self.expires_ns - actual_ts
        };
        self.fired_count += 1;
        self.total_jitter_ns += jitter;
        if jitter > self.max_jitter_ns { self.max_jitter_ns = jitter; }
        self.state = TimerState::Expired;

        // Reschedule periodic timers
        if self.timer_type == TimerType::Periodic && self.interval_ns > 0 {
            self.expires_ns = actual_ts + self.interval_ns;
            self.state = TimerState::Armed;
        }
    }

    #[inline(always)]
    pub fn avg_jitter_ns(&self) -> f64 {
        if self.fired_count == 0 { return 0.0; }
        self.total_jitter_ns as f64 / self.fired_count as f64
    }

    #[inline(always)]
    pub fn is_deferrable(&self) -> bool { self.timer_type == TimerType::Deferrable }
}

/// Timer wheel level
#[derive(Debug, Clone)]
pub struct WheelLevel {
    pub level: u32,
    pub granularity_ns: u64,
    pub slots: u32,
    pub current_slot: u32,
    pub timer_counts: Vec<u32>,
}

impl WheelLevel {
    pub fn new(level: u32, granularity: u64, slots: u32) -> Self {
        Self {
            level, granularity_ns: granularity, slots,
            current_slot: 0,
            timer_counts: alloc::vec![0u32; slots as usize],
        }
    }

    #[inline(always)]
    pub fn slot_for(&self, expires_ns: u64, base_ns: u64) -> u32 {
        let ticks = expires_ns.saturating_sub(base_ns) / self.granularity_ns;
        (ticks as u32) % self.slots
    }

    #[inline(always)]
    pub fn advance(&mut self) {
        self.current_slot = (self.current_slot + 1) % self.slots;
    }

    #[inline(always)]
    pub fn total_timers(&self) -> u32 { self.timer_counts.iter().sum() }
}

/// Coalescing group
#[derive(Debug, Clone)]
pub struct CoalesceGroup {
    pub group_id: u64,
    pub window_ns: u64,
    pub timer_ids: Vec<u64>,
    pub target_expires_ns: u64,
    pub savings_ns: u64,
}

/// Per-CPU timer state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuTimerState {
    pub cpu_id: u32,
    pub active_timers: u32,
    pub expired_this_tick: u32,
    pub total_expired: u64,
    pub total_cancelled: u64,
    pub next_expiry_ns: u64,
}

impl CpuTimerState {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu_id: cpu, active_timers: 0, expired_this_tick: 0,
            total_expired: 0, total_cancelled: 0, next_expiry_ns: u64::MAX,
        }
    }
}

/// Timer wheel stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TimerWheelStats {
    pub total_timers: usize,
    pub armed_timers: usize,
    pub periodic_timers: usize,
    pub hrtimers: usize,
    pub total_fired: u64,
    pub total_cancelled: u64,
    pub avg_jitter_ns: f64,
    pub max_jitter_ns: u64,
    pub coalesced_timers: usize,
    pub coalesce_savings_ns: u64,
    pub wheel_levels: usize,
    pub total_cpus: usize,
}

/// Holistic timer wheel manager
#[repr(align(64))]
pub struct HolisticTimerWheel {
    timers: BTreeMap<u64, TimerEntry>,
    levels: Vec<WheelLevel>,
    cpus: BTreeMap<u32, CpuTimerState>,
    coalesce_groups: Vec<CoalesceGroup>,
    base_ns: u64,
    next_timer_id: u64,
    next_group_id: u64,
    coalesce_window_ns: u64,
    stats: TimerWheelStats,
}

impl HolisticTimerWheel {
    pub fn new(base_ns: u64) -> Self {
        // Create hierarchical levels
        let levels = alloc::vec![
            WheelLevel::new(0, 1_000,         256),  // 1μs granularity
            WheelLevel::new(1, 256_000,        64),   // 256μs granularity
            WheelLevel::new(2, 16_384_000,     64),   // ~16ms granularity
            WheelLevel::new(3, 1_048_576_000,  64),   // ~1s granularity
            WheelLevel::new(4, 67_108_864_000, 64),   // ~67s granularity
        ];
        Self {
            timers: BTreeMap::new(), levels, cpus: BTreeMap::new(),
            coalesce_groups: Vec::new(), base_ns,
            next_timer_id: 1, next_group_id: 1,
            coalesce_window_ns: 1_000_000, // 1ms coalesce window
            stats: TimerWheelStats::default(),
        }
    }

    #[inline(always)]
    pub fn init_cpu(&mut self, cpu: u32) { self.cpus.insert(cpu, CpuTimerState::new(cpu)); }

    pub fn add_timer(&mut self, ttype: TimerType, expires: u64, interval: u64, cb: u64, cpu: Option<u32>, ts: u64) -> u64 {
        let id = self.next_timer_id; self.next_timer_id += 1;
        let mut entry = TimerEntry::new(id, ttype, expires, interval, cb, ts);
        entry.cpu_affinity = cpu;
        entry.state = TimerState::Armed;

        // Place in correct wheel level
        let delta = expires.saturating_sub(self.base_ns);
        for level in &mut self.levels {
            if delta < level.granularity_ns * level.slots as u64 {
                let slot = level.slot_for(expires, self.base_ns);
                if (slot as usize) < level.timer_counts.len() {
                    level.timer_counts[slot as usize] += 1;
                }
                break;
            }
        }

        if let Some(cpu_id) = cpu {
            if let Some(c) = self.cpus.get_mut(&cpu_id) {
                c.active_timers += 1;
                if expires < c.next_expiry_ns { c.next_expiry_ns = expires; }
            }
        }

        self.timers.insert(id, entry);
        id
    }

    pub fn cancel_timer(&mut self, id: u64) -> bool {
        if let Some(timer) = self.timers.get_mut(&id) {
            timer.state = TimerState::Cancelled;
            if let Some(cpu) = timer.cpu_affinity {
                if let Some(c) = self.cpus.get_mut(&cpu) {
                    c.active_timers = c.active_timers.saturating_sub(1);
                    c.total_cancelled += 1;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for timer in self.timers.values_mut() {
            if timer.state == TimerState::Armed && timer.expires_ns <= now {
                timer.fire(now);
                expired.push(timer.timer_id);
                if let Some(cpu) = timer.cpu_affinity {
                    if let Some(c) = self.cpus.get_mut(&cpu) {
                        c.expired_this_tick += 1;
                        c.total_expired += 1;
                    }
                }
            }
        }
        // Remove one-shot expired timers
        self.timers.retain(|_, t| t.state != TimerState::Expired || t.timer_type == TimerType::Periodic);
        self.base_ns = now;
        for level in &mut self.levels { level.advance(); }
        expired
    }

    pub fn coalesce_deferrable(&mut self) {
        let deferrable: Vec<u64> = self.timers.iter()
            .filter(|(_, t)| t.is_deferrable() && t.state == TimerState::Armed)
            .map(|(&id, _)| id)
            .collect();

        if deferrable.len() < 2 { return; }

        // Group timers within coalesce window
        let mut groups: Vec<Vec<u64>> = Vec::new();
        let mut used: Vec<bool> = alloc::vec![false; deferrable.len()];

        for i in 0..deferrable.len() {
            if used[i] { continue; }
            let base_expires = self.timers.get(&deferrable[i]).map(|t| t.expires_ns).unwrap_or(0);
            let mut group = alloc::vec![deferrable[i]];
            used[i] = true;
            for j in (i + 1)..deferrable.len() {
                if used[j] { continue; }
                let other_expires = self.timers.get(&deferrable[j]).map(|t| t.expires_ns).unwrap_or(0);
                if other_expires.abs_diff(base_expires) <= self.coalesce_window_ns {
                    group.push(deferrable[j]);
                    used[j] = true;
                }
            }
            if group.len() > 1 { groups.push(group); }
        }

        for group in groups {
            let target = group.iter()
                .filter_map(|id| self.timers.get(id).map(|t| t.expires_ns))
                .max()
                .unwrap_or(0);
            let savings: u64 = group.iter()
                .filter_map(|id| self.timers.get(id).map(|t| target.saturating_sub(t.expires_ns)))
                .sum();

            for &id in &group {
                if let Some(t) = self.timers.get_mut(&id) {
                    t.expires_ns = target;
                    t.coalesced = true;
                }
            }

            let gid = self.next_group_id; self.next_group_id += 1;
            self.coalesce_groups.push(CoalesceGroup {
                group_id: gid, window_ns: self.coalesce_window_ns,
                timer_ids: group, target_expires_ns: target, savings_ns: savings,
            });
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_timers = self.timers.len();
        self.stats.armed_timers = self.timers.values().filter(|t| t.state == TimerState::Armed).count();
        self.stats.periodic_timers = self.timers.values().filter(|t| t.timer_type == TimerType::Periodic).count();
        self.stats.hrtimers = self.timers.values().filter(|t| t.timer_type == TimerType::HighResolution).count();
        self.stats.total_fired = self.timers.values().map(|t| t.fired_count).sum();
        self.stats.total_cancelled = self.timers.values().filter(|t| t.state == TimerState::Cancelled).count() as u64;
        if !self.timers.is_empty() {
            self.stats.avg_jitter_ns = self.timers.values().map(|t| t.avg_jitter_ns()).sum::<f64>() / self.timers.len() as f64;
            self.stats.max_jitter_ns = self.timers.values().map(|t| t.max_jitter_ns).max().unwrap_or(0);
        }
        self.stats.coalesced_timers = self.timers.values().filter(|t| t.coalesced).count();
        self.stats.coalesce_savings_ns = self.coalesce_groups.iter().map(|g| g.savings_ns).sum();
        self.stats.wheel_levels = self.levels.len();
        self.stats.total_cpus = self.cpus.len();
    }

    #[inline(always)]
    pub fn stats(&self) -> &TimerWheelStats { &self.stats }
}

// ============================================================================
// Merged from timer_wheel_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Pending,
    Expired,
    Cancelled,
    Running,
}

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    OneShot,
    Periodic,
    HiRes,
    Deferrable,
    Pinned,
}

/// Timer entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerEntry {
    pub id: u64,
    pub timer_type: TimerType,
    pub state: TimerState,
    pub expires_at: u64,
    pub period_ns: u64,
    pub callback_hash: u64,
    pub fire_count: u64,
    pub slack_ns: u64,
    pub cpu_affinity: Option<u32>,
}

impl TimerEntry {
    pub fn new(id: u64, ttype: TimerType, expires: u64) -> Self {
        Self {
            id, timer_type: ttype, state: TimerState::Pending,
            expires_at: expires, period_ns: 0, callback_hash: 0,
            fire_count: 0, slack_ns: 0, cpu_affinity: None,
        }
    }

    #[inline(always)]
    pub fn set_periodic(&mut self, period: u64) { self.period_ns = period; }

    #[inline]
    pub fn fire(&mut self, now: u64) {
        self.state = TimerState::Running;
        self.fire_count += 1;
        if self.timer_type == TimerType::Periodic && self.period_ns > 0 {
            self.expires_at = now + self.period_ns;
            self.state = TimerState::Pending;
        } else {
            self.state = TimerState::Expired;
        }
    }

    #[inline(always)]
    pub fn cancel(&mut self) { self.state = TimerState::Cancelled; }

    #[inline(always)]
    pub fn time_until(&self, now: u64) -> i64 {
        self.expires_at as i64 - now as i64
    }
}

/// Wheel level (bucket ring)
#[derive(Debug)]
pub struct WheelLevel {
    pub granularity_ns: u64,
    pub num_buckets: u32,
    pub buckets: Vec<Vec<u64>>,
    pub current_index: u32,
}

impl WheelLevel {
    pub fn new(granularity: u64, num_buckets: u32) -> Self {
        let mut buckets = Vec::with_capacity(num_buckets as usize);
        for _ in 0..num_buckets { buckets.push(Vec::new()); }
        Self { granularity_ns: granularity, num_buckets, buckets, current_index: 0 }
    }

    #[inline(always)]
    pub fn bucket_for(&self, delta_ns: u64) -> u32 {
        let ticks = delta_ns / self.granularity_ns;
        ((self.current_index as u64 + ticks) % self.num_buckets as u64) as u32
    }

    #[inline]
    pub fn insert(&mut self, bucket: u32, timer_id: u64) {
        if (bucket as usize) < self.buckets.len() {
            self.buckets[bucket as usize].push(timer_id);
        }
    }

    #[inline]
    pub fn advance(&mut self) -> Vec<u64> {
        let expired = core::mem::take(&mut self.buckets[self.current_index as usize]);
        self.current_index = (self.current_index + 1) % self.num_buckets;
        expired
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerWheelV2Stats {
    pub total_timers: u32,
    pub pending_timers: u32,
    pub expired_timers: u64,
    pub cancelled_timers: u64,
    pub total_fires: u64,
    pub levels: u32,
    pub periodic_timers: u32,
}

/// Main timer wheel v2 manager
#[repr(align(64))]
pub struct HolisticTimerWheelV2 {
    timers: BTreeMap<u64, TimerEntry>,
    levels: Vec<WheelLevel>,
    next_id: u64,
    current_ns: u64,
    total_expired: u64,
}

impl HolisticTimerWheelV2 {
    pub fn new() -> Self {
        // 4-level wheel: 1ms, 256ms, ~65s, ~16384s
        let levels = alloc::vec![
            WheelLevel::new(1_000_000, 256),       // Level 0: 1ms granularity, 256 buckets
            WheelLevel::new(256_000_000, 256),      // Level 1: 256ms granularity
            WheelLevel::new(65_536_000_000, 256),   // Level 2: ~65s granularity
            WheelLevel::new(16_777_216_000_000, 64),// Level 3: ~16384s granularity
        ];
        Self { timers: BTreeMap::new(), levels, next_id: 1, current_ns: 0, total_expired: 0 }
    }

    pub fn add_timer(&mut self, ttype: TimerType, expires_ns: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let timer = TimerEntry::new(id, ttype, expires_ns);
        let delta = expires_ns.saturating_sub(self.current_ns);
        // Place in appropriate level
        let level_idx = if delta < 256_000_000 { 0 }
            else if delta < 65_536_000_000 { 1 }
            else if delta < 16_777_216_000_000 { 2 }
            else { 3 };
        if level_idx < self.levels.len() {
            let bucket = self.levels[level_idx].bucket_for(delta);
            self.levels[level_idx].insert(bucket, id);
        }
        self.timers.insert(id, timer);
        id
    }

    #[inline(always)]
    pub fn cancel(&mut self, id: u64) {
        if let Some(t) = self.timers.get_mut(&id) { t.cancel(); }
    }

    pub fn advance(&mut self, now: u64) -> Vec<u64> {
        self.current_ns = now;
        let mut fired = Vec::new();
        for timer in self.timers.values_mut() {
            if timer.state == TimerState::Pending && timer.expires_at <= now {
                timer.fire(now);
                fired.push(timer.id);
                self.total_expired += 1;
            }
        }
        fired
    }

    pub fn stats(&self) -> TimerWheelV2Stats {
        let pending = self.timers.values().filter(|t| t.state == TimerState::Pending).count() as u32;
        let fires: u64 = self.timers.values().map(|t| t.fire_count).sum();
        let cancelled = self.timers.values().filter(|t| t.state == TimerState::Cancelled).count() as u64;
        let periodic = self.timers.values().filter(|t| t.timer_type == TimerType::Periodic).count() as u32;
        TimerWheelV2Stats {
            total_timers: self.timers.len() as u32, pending_timers: pending,
            expired_timers: self.total_expired, cancelled_timers: cancelled,
            total_fires: fires, levels: self.levels.len() as u32,
            periodic_timers: periodic,
        }
    }
}
