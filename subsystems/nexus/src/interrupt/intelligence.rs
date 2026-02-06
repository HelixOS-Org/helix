//! Interrupt intelligence coordinator
//!
//! This module provides the central InterruptIntelligence coordinator that
//! integrates all interrupt analysis components including statistics tracking,
//! pattern detection, storm detection, affinity optimization, and coalescing.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::affinity::AffinityOptimizer;
use super::coalescing::CoalescingOptimizer;
use super::pattern::{InterruptPattern, InterruptPatternDetector};
use super::record::InterruptRecord;
use super::stats::IrqStats;
use super::storm::{StormDetector, StormEvent};
use super::types::{CpuId, Irq};

/// Central interrupt intelligence coordinator
pub struct InterruptIntelligence {
    /// IRQ statistics
    stats: BTreeMap<Irq, IrqStats>,
    /// Pattern detector
    pattern: InterruptPatternDetector,
    /// Storm detector
    storm: StormDetector,
    /// Affinity optimizer
    affinity: AffinityOptimizer,
    /// Coalescing optimizer
    coalescing: CoalescingOptimizer,
    /// Total interrupts
    total_interrupts: AtomicU64,
    /// Recent records
    recent: Vec<InterruptRecord>,
    /// Max recent
    max_recent: usize,
}

impl InterruptIntelligence {
    /// Create new interrupt intelligence
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            pattern: InterruptPatternDetector::default(),
            storm: StormDetector::default(),
            affinity: AffinityOptimizer::default(),
            coalescing: CoalescingOptimizer::default(),
            total_interrupts: AtomicU64::new(0),
            recent: Vec::new(),
            max_recent: 1000,
        }
    }

    /// Record interrupt
    pub fn record(&mut self, record: InterruptRecord) -> Option<StormEvent> {
        self.total_interrupts.fetch_add(1, Ordering::Relaxed);

        // Update stats
        let stats = self.stats.entry(record.irq).or_default();
        stats.record(&record);

        // Record pattern
        self.pattern.record(record.irq, record.timestamp);

        // Check for storm
        let storm_event = self.storm.record(record.irq, record.cpu);

        // Store recent
        self.recent.push(record);
        if self.recent.len() > self.max_recent {
            self.recent.remove(0);
        }

        storm_event
    }

    /// Get IRQ stats
    pub fn get_stats(&self, irq: Irq) -> Option<&IrqStats> {
        self.stats.get(&irq)
    }

    /// Get all stats
    pub fn all_stats(&self) -> impl Iterator<Item = (&Irq, &IrqStats)> {
        self.stats.iter()
    }

    /// Get pattern for IRQ
    pub fn get_pattern(&self, irq: Irq) -> Option<InterruptPattern> {
        self.pattern.get_pattern(irq)
    }

    /// Predict next interrupt
    pub fn predict_next(&self, irq: Irq) -> Option<u64> {
        self.pattern.predict_next(irq)
    }

    /// Is storm active?
    pub fn is_storm(&self, irq: Irq) -> bool {
        self.storm.is_storm_active(irq)
    }

    /// Optimize affinity
    pub fn optimize_affinity(&mut self, irq: Irq) -> Option<Vec<CpuId>> {
        if let Some(stats) = self.stats.get(&irq).cloned() {
            self.affinity.update_irq_stats(irq, stats);
        }
        self.affinity.optimize(irq)
    }

    /// Get affinity optimizer
    pub fn affinity(&self) -> &AffinityOptimizer {
        &self.affinity
    }

    /// Get mutable affinity optimizer
    pub fn affinity_mut(&mut self) -> &mut AffinityOptimizer {
        &mut self.affinity
    }

    /// Get coalescing optimizer
    pub fn coalescing(&self) -> &CoalescingOptimizer {
        &self.coalescing
    }

    /// Get mutable coalescing optimizer
    pub fn coalescing_mut(&mut self) -> &mut CoalescingOptimizer {
        &mut self.coalescing
    }

    /// Get storm detector
    pub fn storm(&self) -> &StormDetector {
        &self.storm
    }

    /// Get total interrupts
    pub fn total_interrupts(&self) -> u64 {
        self.total_interrupts.load(Ordering::Relaxed)
    }

    /// Get recent records
    pub fn recent(&self) -> &[InterruptRecord] {
        &self.recent
    }

    /// Get pattern detector
    pub fn pattern_detector(&self) -> &InterruptPatternDetector {
        &self.pattern
    }
}

impl Default for InterruptIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
