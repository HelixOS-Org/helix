//! Hierarchical Timer Wheel
//!
//! Efficient timer scheduling using a multi-level wheel.

use alloc::vec;
use alloc::vec::Vec;

use super::TimerId;

/// Hierarchical timer wheel
#[repr(align(64))]
pub struct TimerWheel {
    /// Wheel levels
    levels: Vec<WheelLevel>,
    /// Current time
    now_ns: u64,
    /// Resolution (ns per tick)
    resolution_ns: u64,
    /// Total timers
    total_timers: u64,
}

/// Wheel level
#[derive(Debug, Clone)]
struct WheelLevel {
    /// Slots
    slots: Vec<Vec<TimerId>>,
    /// Current slot index
    current: usize,
    /// Bits for this level
    bits: u32,
}

impl WheelLevel {
    fn new(bits: u32) -> Self {
        let size = 1 << bits;
        Self {
            slots: vec![Vec::new(); size],
            current: 0,
            bits,
        }
    }

    fn size(&self) -> usize {
        self.slots.len()
    }

    fn advance(&mut self) -> bool {
        self.current = (self.current + 1) % self.size();
        self.current == 0
    }

    fn slot_for(&self, ticks: u64, level: usize) -> usize {
        let shift = level as u32 * self.bits;
        ((ticks >> shift) as usize) & (self.size() - 1)
    }

    fn add(&mut self, slot: usize, timer_id: TimerId) {
        if slot < self.slots.len() {
            self.slots[slot].push(timer_id);
        }
    }

    fn take_current(&mut self) -> Vec<TimerId> {
        core::mem::take(&mut self.slots[self.current])
    }
}

impl TimerWheel {
    /// Create new timer wheel
    pub fn new(resolution_ns: u64, levels: usize, bits_per_level: u32) -> Self {
        let wheel_levels = (0..levels)
            .map(|_| WheelLevel::new(bits_per_level))
            .collect();

        Self {
            levels: wheel_levels,
            now_ns: 0,
            resolution_ns,
            total_timers: 0,
        }
    }

    /// Set current time
    #[inline(always)]
    pub fn set_time(&mut self, now_ns: u64) {
        self.now_ns = now_ns;
    }

    /// Add timer
    pub fn add(&mut self, timer_id: TimerId, deadline_ns: u64) {
        let ticks = deadline_ns.saturating_sub(self.now_ns) / self.resolution_ns;

        // Find appropriate level
        for (level, wheel) in self.levels.iter_mut().enumerate() {
            let max_ticks = 1u64 << ((level + 1) as u32 * wheel.bits);
            if ticks < max_ticks {
                let slot = wheel.slot_for(ticks, level);
                wheel.add(slot, timer_id);
                self.total_timers += 1;
                return;
            }
        }

        // Too far in future, add to last level
        if let Some(last) = self.levels.last_mut() {
            let slot = last.slot_for(ticks, self.levels.len() - 1);
            last.add(slot, timer_id);
            self.total_timers += 1;
        }
    }

    /// Advance time and get expired timers
    pub fn advance(&mut self, now_ns: u64) -> Vec<TimerId> {
        let mut expired = Vec::new();

        while self.now_ns < now_ns {
            self.now_ns += self.resolution_ns;

            // Process level 0
            if let Some(level0) = self.levels.first_mut() {
                expired.extend(level0.take_current());

                // Cascade through levels
                if level0.advance() {
                    self.cascade(1);
                }
            }
        }

        self.total_timers = self.total_timers.saturating_sub(expired.len() as u64);
        expired
    }

    /// Cascade from higher level
    fn cascade(&mut self, level: usize) {
        if level >= self.levels.len() {
            return;
        }

        // Take timers from current slot and reinsert
        let timers = self.levels[level].take_current();

        for timer_id in timers {
            // These timers need to be re-inserted at lower levels
            if let Some(level0) = self.levels.first_mut() {
                level0.add(0, timer_id);
            }
        }

        // Check if we need to cascade further
        if self.levels[level].advance() {
            self.cascade(level + 1);
        }
    }

    /// Get total timers
    #[inline(always)]
    pub fn total_timers(&self) -> u64 {
        self.total_timers
    }

    /// Get resolution
    #[inline(always)]
    pub fn resolution(&self) -> u64 {
        self.resolution_ns
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new(1_000_000, 4, 6) // 1ms resolution, 4 levels, 64 slots each
    }
}
