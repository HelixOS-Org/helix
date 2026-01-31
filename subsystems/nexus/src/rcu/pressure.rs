//! Memory Pressure Analyzer
//!
//! This module provides memory pressure analysis for RCU callback management.

use alloc::vec::Vec;
use super::RcuDomainId;

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressureLevel {
    /// No pressure
    None     = 0,
    /// Low pressure
    Low      = 1,
    /// Medium pressure
    Medium   = 2,
    /// High pressure
    High     = 3,
    /// Critical pressure
    Critical = 4,
}

impl MemoryPressureLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Memory pressure sample
#[derive(Debug, Clone, Copy)]
pub struct MemoryPressureSample {
    /// Timestamp
    pub timestamp_ns: u64,
    /// Pending callback count
    pub pending_callbacks: u64,
    /// Pending memory bytes
    pub pending_memory_bytes: u64,
    /// Grace periods per second
    pub gp_rate: f32,
    /// Callback processing rate
    pub callback_rate: f32,
}

/// Memory pressure analyzer for RCU
pub struct MemoryPressureAnalyzer {
    /// Domain ID
    domain_id: RcuDomainId,
    /// Historical samples
    samples: Vec<MemoryPressureSample>,
    /// Maximum samples
    max_samples: usize,
    /// Current pressure level
    current_level: MemoryPressureLevel,
    /// Pending callback threshold for medium pressure
    medium_threshold: u64,
    /// Pending callback threshold for high pressure
    high_threshold: u64,
    /// Pending callback threshold for critical pressure
    critical_threshold: u64,
    /// Memory byte threshold for pressure
    memory_byte_threshold: u64,
    /// Pressure level changes
    level_changes: u64,
    /// Time in each pressure level
    time_in_level: [u64; 5],
    /// Last level change timestamp
    last_level_change_ns: u64,
}

impl MemoryPressureAnalyzer {
    /// Create new memory pressure analyzer
    pub fn new(domain_id: RcuDomainId) -> Self {
        Self {
            domain_id,
            samples: Vec::with_capacity(256),
            max_samples: 256,
            current_level: MemoryPressureLevel::None,
            medium_threshold: 1000,
            high_threshold: 10000,
            critical_threshold: 100000,
            memory_byte_threshold: 100 * 1024 * 1024, // 100MB
            level_changes: 0,
            time_in_level: [0; 5],
            last_level_change_ns: 0,
        }
    }

    /// Record sample and update pressure level
    pub fn record_sample(&mut self, sample: MemoryPressureSample) {
        // Calculate new pressure level
        let new_level = self.calculate_level(sample.pending_callbacks, sample.pending_memory_bytes);

        // Track time in previous level
        if self.last_level_change_ns > 0 && sample.timestamp_ns > self.last_level_change_ns {
            let duration = sample.timestamp_ns - self.last_level_change_ns;
            self.time_in_level[self.current_level as usize] += duration;
        }

        // Update level if changed
        if new_level != self.current_level {
            self.level_changes += 1;
            self.current_level = new_level;
            self.last_level_change_ns = sample.timestamp_ns;
        }

        // Store sample
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    /// Calculate pressure level from metrics
    fn calculate_level(&self, pending_callbacks: u64, pending_memory: u64) -> MemoryPressureLevel {
        // Check callback count thresholds
        if pending_callbacks >= self.critical_threshold {
            return MemoryPressureLevel::Critical;
        }
        if pending_callbacks >= self.high_threshold {
            return MemoryPressureLevel::High;
        }
        if pending_callbacks >= self.medium_threshold {
            return MemoryPressureLevel::Medium;
        }

        // Check memory threshold
        if pending_memory >= self.memory_byte_threshold {
            return MemoryPressureLevel::High;
        }
        if pending_memory >= self.memory_byte_threshold / 2 {
            return MemoryPressureLevel::Medium;
        }
        if pending_memory >= self.memory_byte_threshold / 4 {
            return MemoryPressureLevel::Low;
        }

        if pending_callbacks > 0 {
            MemoryPressureLevel::Low
        } else {
            MemoryPressureLevel::None
        }
    }

    /// Get current pressure level
    pub fn current_level(&self) -> MemoryPressureLevel {
        self.current_level
    }

    /// Check if expedited grace period is recommended
    pub fn recommend_expedited(&self) -> bool {
        self.current_level >= MemoryPressureLevel::High
    }

    /// Predict time until critical pressure
    pub fn predict_time_to_critical(&self) -> Option<u64> {
        if self.samples.len() < 2 {
            return None;
        }

        if self.current_level >= MemoryPressureLevel::Critical {
            return Some(0);
        }

        // Calculate callback growth rate
        let recent = &self.samples[self.samples.len() - 1];
        let older = &self.samples[self.samples.len().saturating_sub(10).max(0)];

        let time_diff = recent.timestamp_ns.saturating_sub(older.timestamp_ns);
        if time_diff == 0 {
            return None;
        }

        let callback_diff = recent.pending_callbacks as i64 - older.pending_callbacks as i64;
        if callback_diff <= 0 {
            return None; // Not growing
        }

        let growth_rate = callback_diff as f64 / time_diff as f64; // callbacks/ns
        let remaining = self.critical_threshold as i64 - recent.pending_callbacks as i64;

        if remaining <= 0 {
            return Some(0);
        }

        Some((remaining as f64 / growth_rate) as u64)
    }

    /// Get domain ID
    pub fn domain_id(&self) -> RcuDomainId {
        self.domain_id
    }

    /// Get level changes count
    pub fn level_changes(&self) -> u64 {
        self.level_changes
    }

    /// Set thresholds
    pub fn set_thresholds(&mut self, medium: u64, high: u64, critical: u64) {
        self.medium_threshold = medium;
        self.high_threshold = high;
        self.critical_threshold = critical;
    }

    /// Get time distribution across levels
    pub fn get_time_distribution(&self) -> [(MemoryPressureLevel, u64); 5] {
        [
            (MemoryPressureLevel::None, self.time_in_level[0]),
            (MemoryPressureLevel::Low, self.time_in_level[1]),
            (MemoryPressureLevel::Medium, self.time_in_level[2]),
            (MemoryPressureLevel::High, self.time_in_level[3]),
            (MemoryPressureLevel::Critical, self.time_in_level[4]),
        ]
    }
}
