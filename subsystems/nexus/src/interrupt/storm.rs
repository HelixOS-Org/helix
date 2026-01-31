//! Interrupt storm detection and handling
//!
//! This module provides detection and tracking of interrupt storms,
//! which occur when an excessive number of interrupts are generated
//! in a short period of time.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::{CpuId, Irq};
use crate::core::NexusTimestamp;

/// Detects interrupt storms
pub struct StormDetector {
    /// Interrupt counts per window
    window_counts: BTreeMap<Irq, u64>,
    /// Threshold for storm detection
    threshold: u64,
    /// Window size in ms
    window_ms: u64,
    /// Last window reset
    last_reset: u64,
    /// Active storms
    active_storms: BTreeMap<Irq, StormInfo>,
    /// Storm history
    storm_history: Vec<StormEvent>,
    /// Max history
    max_history: usize,
}

/// Information about an active storm
#[derive(Debug, Clone)]
pub struct StormInfo {
    /// Start time
    pub start_time: u64,
    /// Peak rate
    pub peak_rate: u64,
    /// Current rate
    pub current_rate: u64,
    /// Affected CPUs
    pub affected_cpus: Vec<CpuId>,
}

/// Storm event for history
#[derive(Debug, Clone)]
pub struct StormEvent {
    /// IRQ
    pub irq: Irq,
    /// Event type
    pub event_type: StormEventType,
    /// Timestamp
    pub timestamp: u64,
    /// Rate at time of event
    pub rate: u64,
}

/// Type of storm event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StormEventType {
    /// Storm started
    Started,
    /// Storm ended
    Ended,
    /// Storm escalated
    Escalated,
    /// Storm mitigated
    Mitigated,
}

impl StormDetector {
    /// Create new detector
    pub fn new(threshold: u64, window_ms: u64) -> Self {
        Self {
            window_counts: BTreeMap::new(),
            threshold,
            window_ms,
            last_reset: NexusTimestamp::now().raw(),
            active_storms: BTreeMap::new(),
            storm_history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Record interrupt
    pub fn record(&mut self, irq: Irq, cpu: CpuId) -> Option<StormEvent> {
        self.roll_window();

        *self.window_counts.entry(irq).or_insert(0) += 1;

        let count = *self.window_counts.get(&irq).unwrap_or(&0);

        // Check for storm
        if count >= self.threshold {
            self.handle_storm(irq, cpu, count)
        } else if self.active_storms.contains_key(&irq) && count < self.threshold / 2 {
            // Storm ending
            self.end_storm(irq, count)
        } else {
            None
        }
    }

    /// Roll window if needed
    fn roll_window(&mut self) {
        let now = NexusTimestamp::now().raw();
        let window_ns = self.window_ms * 1_000_000;

        if now - self.last_reset >= window_ns {
            self.window_counts.clear();
            self.last_reset = now;
        }
    }

    /// Handle storm detection
    fn handle_storm(&mut self, irq: Irq, cpu: CpuId, rate: u64) -> Option<StormEvent> {
        let now = NexusTimestamp::now().raw();

        if let Some(info) = self.active_storms.get_mut(&irq) {
            // Existing storm
            info.current_rate = rate;
            if rate > info.peak_rate {
                info.peak_rate = rate;
                let event = StormEvent {
                    irq,
                    event_type: StormEventType::Escalated,
                    timestamp: now,
                    rate,
                };
                self.record_event(event.clone());
                return Some(event);
            }
            if !info.affected_cpus.contains(&cpu) {
                info.affected_cpus.push(cpu);
            }
            None
        } else {
            // New storm
            let info = StormInfo {
                start_time: now,
                peak_rate: rate,
                current_rate: rate,
                affected_cpus: alloc::vec![cpu],
            };
            self.active_storms.insert(irq, info);

            let event = StormEvent {
                irq,
                event_type: StormEventType::Started,
                timestamp: now,
                rate,
            };
            self.record_event(event.clone());
            Some(event)
        }
    }

    /// End storm
    fn end_storm(&mut self, irq: Irq, rate: u64) -> Option<StormEvent> {
        if self.active_storms.remove(&irq).is_some() {
            let event = StormEvent {
                irq,
                event_type: StormEventType::Ended,
                timestamp: NexusTimestamp::now().raw(),
                rate,
            };
            self.record_event(event.clone());
            Some(event)
        } else {
            None
        }
    }

    /// Record event in history
    fn record_event(&mut self, event: StormEvent) {
        self.storm_history.push(event);
        if self.storm_history.len() > self.max_history {
            self.storm_history.remove(0);
        }
    }

    /// Is storm active for IRQ?
    pub fn is_storm_active(&self, irq: Irq) -> bool {
        self.active_storms.contains_key(&irq)
    }

    /// Get storm info
    pub fn get_storm(&self, irq: Irq) -> Option<&StormInfo> {
        self.active_storms.get(&irq)
    }

    /// Get all active storms
    pub fn active_storms(&self) -> impl Iterator<Item = (&Irq, &StormInfo)> {
        self.active_storms.iter()
    }

    /// Get storm history
    pub fn history(&self) -> &[StormEvent] {
        &self.storm_history
    }
}

impl Default for StormDetector {
    fn default() -> Self {
        Self::new(10000, 100) // 10000 interrupts per 100ms
    }
}
