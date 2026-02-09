//! Behavior Analyzer
//!
//! Process behavior pattern analysis.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::math;
use super::{ProcessAnomalyDetector, ProcessId, ProcessMetrics, ProcessProfile};

/// Process behavior event
#[derive(Debug, Clone)]
pub struct BehaviorEvent {
    /// Event type
    pub event_type: BehaviorEventType,
    /// Process ID
    pub pid: ProcessId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Details
    pub details: String,
}

/// Types of behavior events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorEventType {
    /// CPU spike
    CpuSpike,
    /// Memory spike
    MemorySpike,
    /// I/O spike
    IoSpike,
    /// Process started
    Started,
    /// Process exited
    Exited,
    /// Behavior change
    BehaviorChange,
    /// Resource anomaly
    ResourceAnomaly,
}

/// Analyzes process behavior patterns
pub struct ProcessBehaviorAnalyzer {
    /// Process profiles
    profiles: BTreeMap<ProcessId, ProcessProfile>,
    /// Recent metrics
    recent_metrics: BTreeMap<ProcessId, ProcessMetrics>,
    /// Behavior events
    events: VecDeque<BehaviorEvent>,
    /// Max events to keep
    max_events: usize,
    /// Anomaly detector
    anomaly_detector: ProcessAnomalyDetector,
}

impl ProcessBehaviorAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            recent_metrics: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 10000,
            anomaly_detector: ProcessAnomalyDetector::new(),
        }
    }

    /// Record process metrics
    pub fn record_metrics(&mut self, metrics: ProcessMetrics) -> Option<BehaviorEvent> {
        let pid = metrics.pid;
        let previous = self.recent_metrics.get(&pid).cloned();

        // Update or create profile
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessProfile::new(pid));
        profile.update(&metrics, previous.as_ref());

        // Check for anomalies
        let event = self.check_anomalies(&metrics, &previous, profile);

        // Store metrics
        self.recent_metrics.insert(pid, metrics);

        event
    }

    /// Check for behavioral anomalies
    fn check_anomalies(
        &mut self,
        current: &ProcessMetrics,
        previous: Option<&ProcessMetrics>,
        profile: &ProcessProfile,
    ) -> Option<BehaviorEvent> {
        if let Some(prev) = previous {
            let duration = current.timestamp.duration_since(prev.timestamp);
            let cpu = current.cpu_usage(prev, duration);

            // CPU spike
            if cpu > profile.avg_cpu_usage + 3.0 * math::sqrt(profile.cpu_variance) {
                return Some(self.record_event(
                    BehaviorEventType::CpuSpike,
                    current.pid,
                    format!(
                        "CPU spike: {:.1}% (avg: {:.1}%)",
                        cpu * 100.0,
                        profile.avg_cpu_usage * 100.0
                    ),
                ));
            }

            // Memory spike
            let mem_delta = current.memory_delta(prev);
            if mem_delta > 100_000_000 {
                return Some(self.record_event(
                    BehaviorEventType::MemorySpike,
                    current.pid,
                    format!("Memory spike: +{} MB", mem_delta / 1_000_000),
                ));
            }

            // I/O spike
            let (read_rate, write_rate) = current.io_rate(prev, duration);
            let io_rate = read_rate + write_rate;
            if io_rate > profile.avg_io_rate * 10.0 && io_rate > 100_000_000.0 {
                return Some(self.record_event(
                    BehaviorEventType::IoSpike,
                    current.pid,
                    format!("I/O spike: {:.1} MB/s", io_rate / 1_000_000.0),
                ));
            }
        }

        None
    }

    /// Record behavior event
    fn record_event(
        &mut self,
        event_type: BehaviorEventType,
        pid: ProcessId,
        details: String,
    ) -> BehaviorEvent {
        let event = BehaviorEvent {
            event_type,
            pid,
            timestamp: NexusTimestamp::now(),
            details,
        };

        self.events.push_back(event.clone());
        if self.events.len() > self.max_events {
            self.events.pop_front();
        }

        event
    }

    /// Record process start
    #[inline]
    pub fn record_start(&mut self, pid: ProcessId, name: &str) {
        let mut profile = ProcessProfile::new(pid);
        profile.name = String::from(name);
        self.profiles.insert(pid, profile);

        self.record_event(
            BehaviorEventType::Started,
            pid,
            format!("Process started: {}", name),
        );
    }

    /// Record process exit
    #[inline]
    pub fn record_exit(&mut self, pid: ProcessId, exit_code: i32) {
        self.record_event(
            BehaviorEventType::Exited,
            pid,
            format!("Process exited with code {}", exit_code),
        );

        self.recent_metrics.remove(&pid);
    }

    /// Get process profile
    #[inline(always)]
    pub fn get_profile(&self, pid: ProcessId) -> Option<&ProcessProfile> {
        self.profiles.get(&pid)
    }

    /// Get all profiles
    #[inline(always)]
    pub fn all_profiles(&self) -> impl Iterator<Item = (&ProcessId, &ProcessProfile)> {
        self.profiles.iter()
    }

    /// Get recent events
    #[inline(always)]
    pub fn recent_events(&self, n: usize) -> &[BehaviorEvent] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }

    /// Get events for process
    #[inline(always)]
    pub fn events_for_process(&self, pid: ProcessId) -> Vec<&BehaviorEvent> {
        self.events.iter().filter(|e| e.pid == pid).collect()
    }

    /// Cleanup old data
    #[inline]
    pub fn cleanup(&mut self, max_age_ticks: u64) {
        let now = NexusTimestamp::now();

        self.profiles
            .retain(|_, profile| now.duration_since(profile.last_update) < max_age_ticks);

        let pids: Vec<_> = self.profiles.keys().copied().collect();
        self.recent_metrics.retain(|pid, _| pids.contains(pid));
    }
}

impl Default for ProcessBehaviorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
