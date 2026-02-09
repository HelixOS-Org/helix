//! Performance Analyzer and Tracepoint Manager
//!
//! Statistics collection and tracepoint management.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{TracepointDef, TracepointId, TracepointState, TracepointSubsystem};

/// Tracepoint performance sample
#[derive(Debug, Clone, Copy)]
pub struct TraceSample {
    /// Tracepoint ID
    pub tracepoint_id: TracepointId,
    /// Timestamp
    pub timestamp: u64,
    /// Processing time (ns)
    pub processing_time_ns: u64,
    /// Event size (bytes)
    pub event_size: usize,
    /// CPU
    pub cpu: u32,
}

/// Tracepoint statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TraceStats {
    /// Tracepoint ID
    pub tracepoint_id: TracepointId,
    /// Total events
    pub total_events: u64,
    /// Average processing time (ns)
    pub avg_processing_ns: f32,
    /// Max processing time (ns)
    pub max_processing_ns: u64,
    /// Min processing time (ns)
    pub min_processing_ns: u64,
    /// Total data bytes
    pub total_bytes: u64,
    /// Events per second
    pub events_per_second: f32,
    /// Last update timestamp
    pub last_update: u64,
}

impl TraceStats {
    /// Create new stats
    pub fn new(tracepoint_id: TracepointId) -> Self {
        Self {
            tracepoint_id,
            total_events: 0,
            avg_processing_ns: 0.0,
            max_processing_ns: 0,
            min_processing_ns: u64::MAX,
            total_bytes: 0,
            events_per_second: 0.0,
            last_update: 0,
        }
    }

    /// Update with sample
    pub fn update(&mut self, sample: &TraceSample) {
        self.total_events += 1;

        // Update processing time stats
        if sample.processing_time_ns > self.max_processing_ns {
            self.max_processing_ns = sample.processing_time_ns;
        }
        if sample.processing_time_ns < self.min_processing_ns {
            self.min_processing_ns = sample.processing_time_ns;
        }

        // Exponential moving average for processing time
        let alpha = 0.1;
        self.avg_processing_ns =
            (1.0 - alpha) * self.avg_processing_ns + alpha * sample.processing_time_ns as f32;

        self.total_bytes += sample.event_size as u64;
        self.last_update = sample.timestamp;
    }

    /// Update events per second
    #[inline]
    pub fn update_rate(&mut self, events_in_period: u64, period_ns: u64) {
        if period_ns > 0 {
            self.events_per_second =
                (events_in_period as f64 * 1_000_000_000.0 / period_ns as f64) as f32;
        }
    }
}

/// Performance analyzer
pub struct PerformanceAnalyzer {
    /// Per-tracepoint stats
    stats: BTreeMap<TracepointId, TraceStats>,
    /// Recent samples
    samples: VecDeque<TraceSample>,
    /// Max samples
    max_samples: usize,
    /// Global stats
    global_events: AtomicU64,
    global_bytes: AtomicU64,
    /// Start timestamp
    start_time: u64,
}

impl PerformanceAnalyzer {
    /// Create new performance analyzer
    pub fn new(start_time: u64) -> Self {
        Self {
            stats: BTreeMap::new(),
            samples: Vec::with_capacity(10000),
            max_samples: 10000,
            global_events: AtomicU64::new(0),
            global_bytes: AtomicU64::new(0),
            start_time,
        }
    }

    /// Record sample
    pub fn record(&mut self, sample: TraceSample) {
        // Update global stats
        self.global_events.fetch_add(1, Ordering::Relaxed);
        self.global_bytes
            .fetch_add(sample.event_size as u64, Ordering::Relaxed);

        // Update per-tracepoint stats
        let stats = self
            .stats
            .entry(sample.tracepoint_id)
            .or_insert_with(|| TraceStats::new(sample.tracepoint_id));
        stats.update(&sample);

        // Store sample
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Get stats for tracepoint
    #[inline(always)]
    pub fn get_stats(&self, tracepoint_id: TracepointId) -> Option<&TraceStats> {
        self.stats.get(&tracepoint_id)
    }

    /// Get all stats
    #[inline(always)]
    pub fn all_stats(&self) -> impl Iterator<Item = &TraceStats> {
        self.stats.values()
    }

    /// Get global event count
    #[inline(always)]
    pub fn global_events(&self) -> u64 {
        self.global_events.load(Ordering::Relaxed)
    }

    /// Get global bytes
    #[inline(always)]
    pub fn global_bytes(&self) -> u64 {
        self.global_bytes.load(Ordering::Relaxed)
    }

    /// Get events per second (global)
    #[inline]
    pub fn global_events_per_second(&self, current_time: u64) -> f32 {
        let elapsed = current_time.saturating_sub(self.start_time);
        if elapsed == 0 {
            return 0.0;
        }
        (self.global_events() as f64 * 1_000_000_000.0 / elapsed as f64) as f32
    }

    /// Find hottest tracepoints
    #[inline]
    pub fn hottest_tracepoints(&self, limit: usize) -> Vec<&TraceStats> {
        let mut sorted: Vec<_> = self.stats.values().collect();
        sorted.sort_by(|a, b| b.total_events.cmp(&a.total_events));
        sorted.into_iter().take(limit).collect()
    }

    /// Find slowest tracepoints
    #[inline]
    pub fn slowest_tracepoints(&self, limit: usize) -> Vec<&TraceStats> {
        let mut sorted: Vec<_> = self.stats.values().collect();
        sorted.sort_by(|a, b| {
            b.avg_processing_ns
                .partial_cmp(&a.avg_processing_ns)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(limit).collect()
    }

    /// Get recent samples
    #[inline(always)]
    pub fn recent_samples(&self, count: usize) -> &[TraceSample] {
        let start = self.samples.len().saturating_sub(count);
        &self.samples[start..]
    }
}

/// Tracepoint manager
pub struct TracepointManager {
    /// Registered tracepoints
    tracepoints: BTreeMap<TracepointId, TracepointDef>,
    /// Tracepoints by name
    by_name: BTreeMap<String, TracepointId>,
    /// Tracepoints by subsystem
    by_subsystem: BTreeMap<TracepointSubsystem, Vec<TracepointId>>,
    /// Next ID
    next_id: AtomicU64,
    /// Event ID counter
    next_event_id: AtomicU64,
    /// Total registered
    total_registered: AtomicU64,
    /// Active count
    active_count: u32,
}

impl TracepointManager {
    /// Create new tracepoint manager
    pub fn new() -> Self {
        Self {
            tracepoints: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_subsystem: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            next_event_id: AtomicU64::new(1),
            total_registered: AtomicU64::new(0),
            active_count: 0,
        }
    }

    /// Allocate tracepoint ID
    #[inline(always)]
    pub fn allocate_id(&self) -> TracepointId {
        TracepointId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate event ID
    #[inline(always)]
    pub fn allocate_event_id(&self) -> super::EventId {
        super::EventId::new(self.next_event_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Register tracepoint
    pub fn register(
        &mut self,
        name: String,
        subsystem: TracepointSubsystem,
        timestamp: u64,
    ) -> TracepointId {
        // Check if already registered
        if let Some(&id) = self.by_name.get(&name) {
            return id;
        }

        let id = self.allocate_id();
        let def = TracepointDef::new(id, name.clone(), subsystem, timestamp);

        self.by_name.insert(name, id);
        self.by_subsystem.entry(subsystem).or_default().push(id);
        self.tracepoints.insert(id, def);
        self.total_registered.fetch_add(1, Ordering::Relaxed);

        id
    }

    /// Unregister tracepoint
    pub fn unregister(&mut self, id: TracepointId) -> bool {
        if let Some(def) = self.tracepoints.remove(&id) {
            self.by_name.remove(&def.name);
            if let Some(list) = self.by_subsystem.get_mut(&def.subsystem) {
                list.retain(|&x| x != id);
            }
            if def.is_enabled() {
                self.active_count = self.active_count.saturating_sub(1);
            }
            return true;
        }
        false
    }

    /// Get tracepoint
    #[inline(always)]
    pub fn get(&self, id: TracepointId) -> Option<&TracepointDef> {
        self.tracepoints.get(&id)
    }

    /// Get tracepoint mutably
    #[inline(always)]
    pub fn get_mut(&mut self, id: TracepointId) -> Option<&mut TracepointDef> {
        self.tracepoints.get_mut(&id)
    }

    /// Get by name
    #[inline]
    pub fn get_by_name(&self, name: &str) -> Option<&TracepointDef> {
        self.by_name
            .get(name)
            .and_then(|id| self.tracepoints.get(id))
    }

    /// Get by subsystem
    #[inline]
    pub fn get_by_subsystem(&self, subsystem: TracepointSubsystem) -> Vec<&TracepointDef> {
        self.by_subsystem
            .get(&subsystem)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.tracepoints.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Enable tracepoint
    #[inline]
    pub fn enable(&mut self, id: TracepointId) -> bool {
        if let Some(def) = self.tracepoints.get_mut(&id) {
            if !def.is_enabled() {
                def.state = TracepointState::Enabled;
                self.active_count += 1;
            }
            return true;
        }
        false
    }

    /// Disable tracepoint
    #[inline]
    pub fn disable(&mut self, id: TracepointId) -> bool {
        if let Some(def) = self.tracepoints.get_mut(&id) {
            if def.is_enabled() {
                def.state = TracepointState::Disabled;
                self.active_count = self.active_count.saturating_sub(1);
            }
            return true;
        }
        false
    }

    /// Get active count
    #[inline(always)]
    pub fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Get total registered
    #[inline(always)]
    pub fn total_registered(&self) -> u64 {
        self.total_registered.load(Ordering::Relaxed)
    }

    /// Get all tracepoints
    #[inline(always)]
    pub fn all(&self) -> impl Iterator<Item = &TracepointDef> {
        self.tracepoints.values()
    }
}

impl Default for TracepointManager {
    fn default() -> Self {
        Self::new()
    }
}
