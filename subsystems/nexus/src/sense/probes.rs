//! Built-in Probe Implementations
//!
//! CPU, Memory, and other standard probes.

#![allow(dead_code)]

use super::events::{CpuSample, EventData, MemorySample, RawEvent};
use super::probe::{Probe, ProbeConfig, ProbeError, ProbeState, ProbeStats, ProbeType};
use crate::types::{ProbeId, Timestamp};

// ============================================================================
// CPU PROBE
// ============================================================================

/// CPU probe implementation
pub struct CpuProbe {
    id: ProbeId,
    config: ProbeConfig,
    state: ProbeState,
    stats: ProbeStats,
    /// Simulated samples for testing
    sample_counter: u64,
}

impl CpuProbe {
    /// Create new CPU probe
    pub fn new() -> Self {
        Self {
            id: ProbeId::generate(),
            config: ProbeConfig::for_type(ProbeType::Cpu),
            state: ProbeState::Registered,
            stats: ProbeStats::default(),
            sample_counter: 0,
        }
    }

    /// Create with custom config
    pub fn with_config(config: ProbeConfig) -> Self {
        Self {
            id: ProbeId::generate(),
            config,
            state: ProbeState::Registered,
            stats: ProbeStats::default(),
            sample_counter: 0,
        }
    }
}

impl Probe for CpuProbe {
    fn id(&self) -> ProbeId {
        self.id
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Cpu
    }

    fn name(&self) -> &str {
        "cpu_probe"
    }

    fn state(&self) -> ProbeState {
        self.state
    }

    fn config(&self) -> &ProbeConfig {
        &self.config
    }

    fn init(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Initializing;
        self.state = ProbeState::Active;
        Ok(())
    }

    fn start(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Active;
        self.stats.start_time = Some(Timestamp::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Active;
        Ok(())
    }

    fn poll(&mut self) -> Option<RawEvent> {
        if self.state != ProbeState::Active {
            return None;
        }

        // Generate simulated sample
        self.sample_counter += 1;

        // Only generate event every N polls
        if self.sample_counter % 10 != 0 {
            return None;
        }

        let sample = CpuSample {
            cpu_id: 0,
            user: 25,
            system: 15,
            idle: 55,
            iowait: 5,
            irq: 0,
            softirq: 0,
            steal: 0,
            frequency_mhz: 3000,
            temperature: Some(55),
        };

        self.stats.events_collected += 1;
        self.stats.last_event = Some(Timestamp::now());

        Some(RawEvent::new(
            self.id,
            ProbeType::Cpu,
            EventData::CpuSample(sample),
        ))
    }

    fn stats(&self) -> ProbeStats {
        self.stats.clone()
    }
}

impl Default for CpuProbe {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MEMORY PROBE
// ============================================================================

/// Memory probe implementation
pub struct MemoryProbe {
    id: ProbeId,
    config: ProbeConfig,
    state: ProbeState,
    stats: ProbeStats,
    sample_counter: u64,
}

impl MemoryProbe {
    /// Create new memory probe
    pub fn new() -> Self {
        Self {
            id: ProbeId::generate(),
            config: ProbeConfig::for_type(ProbeType::Memory),
            state: ProbeState::Registered,
            stats: ProbeStats::default(),
            sample_counter: 0,
        }
    }

    /// Create with custom config
    pub fn with_config(config: ProbeConfig) -> Self {
        Self {
            id: ProbeId::generate(),
            config,
            state: ProbeState::Registered,
            stats: ProbeStats::default(),
            sample_counter: 0,
        }
    }
}

impl Probe for MemoryProbe {
    fn id(&self) -> ProbeId {
        self.id
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Memory
    }

    fn name(&self) -> &str {
        "memory_probe"
    }

    fn state(&self) -> ProbeState {
        self.state
    }

    fn config(&self) -> &ProbeConfig {
        &self.config
    }

    fn init(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Initializing;
        self.state = ProbeState::Active;
        Ok(())
    }

    fn start(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Active;
        self.stats.start_time = Some(Timestamp::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), ProbeError> {
        self.state = ProbeState::Active;
        Ok(())
    }

    fn poll(&mut self) -> Option<RawEvent> {
        if self.state != ProbeState::Active {
            return None;
        }

        self.sample_counter += 1;

        if self.sample_counter % 100 != 0 {
            return None;
        }

        let sample = MemorySample {
            total: 16 * 1024 * 1024 * 1024,
            used: 8 * 1024 * 1024 * 1024,
            free: 4 * 1024 * 1024 * 1024,
            available: 6 * 1024 * 1024 * 1024,
            buffers: 512 * 1024 * 1024,
            cached: 2 * 1024 * 1024 * 1024,
            swap_total: 8 * 1024 * 1024 * 1024,
            swap_used: 0,
            page_faults: 100,
            major_faults: 0,
        };

        self.stats.events_collected += 1;
        self.stats.last_event = Some(Timestamp::now());

        Some(RawEvent::new(
            self.id,
            ProbeType::Memory,
            EventData::MemorySample(sample),
        ))
    }

    fn stats(&self) -> ProbeStats {
        self.stats.clone()
    }
}

impl Default for MemoryProbe {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_probe() {
        let mut probe = CpuProbe::new();
        assert_eq!(probe.probe_type(), ProbeType::Cpu);
        assert_eq!(probe.state(), ProbeState::Registered);

        probe.start().unwrap();
        assert_eq!(probe.state(), ProbeState::Active);

        // Poll many times to get an event
        let mut got_event = false;
        for _ in 0..20 {
            if probe.poll().is_some() {
                got_event = true;
                break;
            }
        }
        assert!(got_event);
    }

    #[test]
    fn test_memory_probe() {
        let mut probe = MemoryProbe::new();
        assert_eq!(probe.probe_type(), ProbeType::Memory);

        probe.start().unwrap();
        assert_eq!(probe.state(), ProbeState::Active);
    }

    #[test]
    fn test_probe_pause_resume() {
        let mut probe = CpuProbe::new();
        probe.start().unwrap();

        probe.pause().unwrap();
        assert_eq!(probe.state(), ProbeState::Paused);

        probe.resume().unwrap();
        assert_eq!(probe.state(), ProbeState::Active);
    }
}
