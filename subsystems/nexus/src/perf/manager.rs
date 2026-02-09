//! Performance Manager
//!
//! PMU and event management.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{EventConfig, EventId, PerfEvent, Pmu, PmuId, PmuType, Sample};

// ============================================================================
// PERF MANAGER
// ============================================================================

/// Perf manager
pub struct PerfManager {
    /// PMUs
    pub(crate) pmus: BTreeMap<PmuId, Pmu>,
    /// Events
    pub(crate) events: BTreeMap<EventId, PerfEvent>,
    /// Samples
    samples: VecDeque<Sample>,
    /// Max samples
    max_samples: usize,
    /// Event counter
    event_counter: AtomicU64,
    /// Sample count
    sample_count: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl PerfManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            pmus: BTreeMap::new(),
            events: BTreeMap::new(),
            samples: VecDeque::new(),
            max_samples: 10000,
            event_counter: AtomicU64::new(0),
            sample_count: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register PMU
    #[inline(always)]
    pub fn register_pmu(&mut self, pmu: Pmu) {
        self.pmus.insert(pmu.id, pmu);
    }

    /// Get PMU
    #[inline(always)]
    pub fn get_pmu(&self, id: PmuId) -> Option<&Pmu> {
        self.pmus.get(&id)
    }

    /// Create event
    #[inline]
    pub fn create_event(&mut self, config: EventConfig, pmu: PmuId) -> EventId {
        let id = EventId(self.event_counter.fetch_add(1, Ordering::Relaxed));
        let event = PerfEvent::new(id, config, pmu);
        self.events.insert(id, event);
        id
    }

    /// Get event
    #[inline(always)]
    pub fn get_event(&self, id: EventId) -> Option<&PerfEvent> {
        self.events.get(&id)
    }

    /// Get event mutably
    #[inline(always)]
    pub fn get_event_mut(&mut self, id: EventId) -> Option<&mut PerfEvent> {
        self.events.get_mut(&id)
    }

    /// Start event
    #[inline]
    pub fn start_event(&mut self, id: EventId) -> bool {
        if let Some(event) = self.events.get_mut(&id) {
            if let Some(pmu) = self.pmus.get(&event.pmu) {
                pmu.add_counter();
                event.start();
                return true;
            }
        }
        false
    }

    /// Stop event
    #[inline]
    pub fn stop_event(&mut self, id: EventId) -> bool {
        if let Some(event) = self.events.get_mut(&id) {
            if let Some(pmu) = self.pmus.get(&event.pmu) {
                pmu.remove_counter();
                event.stop();
                return true;
            }
        }
        false
    }

    /// Record sample
    #[inline]
    pub fn record_sample(&mut self, sample: Sample) {
        self.sample_count.fetch_add(1, Ordering::Relaxed);
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Active events
    #[inline(always)]
    pub fn active_events(&self) -> Vec<&PerfEvent> {
        self.events.values().filter(|e| e.is_running()).collect()
    }

    /// Total sample count
    #[inline(always)]
    pub fn total_samples(&self) -> u64 {
        self.sample_count.load(Ordering::Relaxed)
    }

    /// Get PMU for hardware event
    #[inline]
    pub fn core_pmu(&self) -> Option<&Pmu> {
        self.pmus
            .values()
            .find(|p| matches!(p.pmu_type, PmuType::Core))
    }
}

impl Default for PerfManager {
    fn default() -> Self {
        Self::new()
    }
}
