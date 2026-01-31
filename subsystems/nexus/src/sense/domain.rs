//! Sense Domain Orchestrator
//!
//! Main domain implementation coordinating probes, collection, and normalization.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::types::{DomainId, ProbeId, Timestamp};
use super::collector::{CollectorStats, EventCollector, EventCollectorConfig};
use super::probe::{Probe, ProbeError};
use super::probes::{CpuProbe, MemoryProbe};
use super::registry::ProbeRegistry;
use super::signal::{NormalizerStats, Signal, SignalNormalizer};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for Sense domain
#[derive(Debug, Clone)]
pub struct SenseConfig {
    /// Event collector config
    pub collector: EventCollectorConfig,
    /// Enable built-in probes
    pub enable_builtin_probes: bool,
    /// Maximum probes
    pub max_probes: usize,
}

impl Default for SenseConfig {
    fn default() -> Self {
        Self {
            collector: EventCollectorConfig::default(),
            enable_builtin_probes: true,
            max_probes: 256,
        }
    }
}

impl SenseConfig {
    /// Minimal config for testing
    pub fn minimal() -> Self {
        Self {
            collector: EventCollectorConfig::small(),
            enable_builtin_probes: false,
            max_probes: 16,
        }
    }

    /// Production config
    pub fn production() -> Self {
        Self {
            collector: EventCollectorConfig::large(),
            enable_builtin_probes: true,
            max_probes: 256,
        }
    }
}

// ============================================================================
// SENSE DOMAIN
// ============================================================================

/// The Sense domain - perception layer
pub struct SenseDomain {
    /// Domain ID
    id: DomainId,
    /// Configuration
    config: SenseConfig,
    /// Is running
    running: AtomicBool,
    /// Probe registry
    probes: ProbeRegistry,
    /// Event collector
    collector: EventCollector,
    /// Signal normalizer
    normalizer: SignalNormalizer,
    /// Output signal buffer
    output_buffer: Vec<Signal>,
    /// Total ticks
    total_ticks: AtomicU64,
}

impl SenseDomain {
    /// Create new Sense domain
    pub fn new(config: SenseConfig) -> Self {
        let mut domain = Self {
            id: DomainId::generate(),
            config: config.clone(),
            running: AtomicBool::new(false),
            probes: ProbeRegistry::new(),
            collector: EventCollector::new(config.collector),
            normalizer: SignalNormalizer::new(),
            output_buffer: Vec::new(),
            total_ticks: AtomicU64::new(0),
        };

        if config.enable_builtin_probes {
            domain.register_builtin_probes();
        }

        domain
    }

    /// Register built-in probes
    fn register_builtin_probes(&mut self) {
        self.probes.register(Box::new(CpuProbe::new()));
        self.probes.register(Box::new(MemoryProbe::new()));
    }

    /// Get domain ID
    pub fn id(&self) -> DomainId {
        self.id
    }

    /// Register a probe
    pub fn register_probe(&mut self, probe: Box<dyn Probe>) -> ProbeId {
        self.probes.register(probe)
    }

    /// Unregister a probe
    pub fn unregister_probe(&mut self, id: ProbeId) -> Option<Box<dyn Probe>> {
        self.probes.unregister(id)
    }

    /// Get probe
    pub fn get_probe(&self, id: ProbeId) -> Option<&dyn Probe> {
        self.probes.get(id)
    }

    /// Start the domain
    pub fn start(&mut self) -> Result<(), SenseError> {
        if self.running.load(Ordering::Acquire) {
            return Err(SenseError::AlreadyRunning);
        }

        // Start all probes
        self.probes.start_all().map_err(SenseError::ProbeErrors)?;

        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Stop the domain
    pub fn stop(&mut self) -> Result<(), SenseError> {
        if !self.running.load(Ordering::Acquire) {
            return Err(SenseError::NotRunning);
        }

        self.running.store(false, Ordering::Release);

        // Stop all probes
        self.probes.stop_all().map_err(SenseError::ProbeErrors)?;

        Ok(())
    }

    /// Pause the domain
    pub fn pause(&mut self) -> Result<(), SenseError> {
        self.probes.pause_all().map_err(SenseError::ProbeErrors)
    }

    /// Resume the domain
    pub fn resume(&mut self) -> Result<(), SenseError> {
        self.probes.resume_all().map_err(SenseError::ProbeErrors)
    }

    /// Is running?
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Process one tick
    pub fn tick(&mut self, now: Timestamp) -> Vec<Signal> {
        if !self.running.load(Ordering::Acquire) {
            return Vec::new();
        }

        self.total_ticks.fetch_add(1, Ordering::Relaxed);

        // Poll all active probes
        let active_ids = self.probes.active_ids();

        for id in active_ids {
            if let Some(probe) = self.probes.get_mut(id) {
                while let Some(event) = probe.poll() {
                    self.collector.collect(event);
                }
            }
        }

        // Update rate calculation
        self.collector.update_rate(now);

        // Get batch of events and normalize to signals
        let events = self.collector.get_batch();
        self.normalizer.normalize_batch(events)
    }

    /// Get domain statistics
    pub fn stats(&self) -> SenseStats {
        SenseStats {
            domain_id: self.id,
            is_running: self.running.load(Ordering::Relaxed),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            probes_registered: self.probes.count(),
            probes_active: self.probes.active_count(),
            collector: self.collector.stats(),
            normalizer: self.normalizer.stats(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.collector.reset_stats();
        self.normalizer.reset_stats();
    }
}

impl Default for SenseDomain {
    fn default() -> Self {
        Self::new(SenseConfig::default())
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Sense domain statistics
#[derive(Debug, Clone)]
pub struct SenseStats {
    /// Domain ID
    pub domain_id: DomainId,
    /// Is running
    pub is_running: bool,
    /// Total ticks processed
    pub total_ticks: u64,
    /// Probes registered
    pub probes_registered: usize,
    /// Probes active
    pub probes_active: usize,
    /// Collector stats
    pub collector: CollectorStats,
    /// Normalizer stats
    pub normalizer: NormalizerStats,
}

// ============================================================================
// ERRORS
// ============================================================================

/// Sense domain errors
#[derive(Debug)]
pub enum SenseError {
    /// Domain already running
    AlreadyRunning,
    /// Domain not running
    NotRunning,
    /// Probe errors
    ProbeErrors(Vec<(ProbeId, ProbeError)>),
    /// Other error
    Other(String),
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sense_domain_new() {
        let config = SenseConfig::minimal();
        let domain = SenseDomain::new(config);

        assert!(!domain.is_running());
        assert_eq!(domain.stats().probes_registered, 0);
    }

    #[test]
    fn test_sense_domain_with_builtin() {
        let config = SenseConfig::default();
        let domain = SenseDomain::new(config);

        assert_eq!(domain.stats().probes_registered, 2); // CPU + Memory
    }

    #[test]
    fn test_sense_domain_start_stop() {
        let config = SenseConfig::minimal();
        let mut domain = SenseDomain::new(config);

        domain.start().unwrap();
        assert!(domain.is_running());

        domain.stop().unwrap();
        assert!(!domain.is_running());
    }

    #[test]
    fn test_sense_domain_tick() {
        let config = SenseConfig::default();
        let mut domain = SenseDomain::new(config);

        domain.start().unwrap();

        // Process multiple ticks
        for _ in 0..100 {
            let _ = domain.tick(Timestamp::now());
        }

        assert!(domain.stats().total_ticks >= 100);
    }
}
