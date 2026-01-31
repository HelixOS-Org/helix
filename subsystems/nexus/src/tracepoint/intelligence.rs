//! Tracepoint Intelligence
//!
//! Comprehensive tracing analysis and optimization engine.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    EventData, EventFilter, EventRingBuffer, FilterExpr, PerformanceAnalyzer, ProbeId,
    ProbeManager, ProbeType, TraceSample, TraceStats, TracepointId, TracepointManager,
    TracepointSubsystem,
};

/// Tracepoint analysis result
#[derive(Debug, Clone)]
pub struct TracepointAnalysis {
    /// Tracepoint ID
    pub tracepoint_id: TracepointId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Performance stats
    pub stats: Option<TraceStats>,
    /// Issues detected
    pub issues: Vec<TraceIssue>,
    /// Recommendations
    pub recommendations: Vec<TraceRecommendation>,
}

/// Trace issue
#[derive(Debug, Clone)]
pub struct TraceIssue {
    /// Issue type
    pub issue_type: TraceIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

impl TraceIssue {
    /// Create new issue
    pub fn new(issue_type: TraceIssueType, severity: u8, description: String) -> Self {
        Self {
            issue_type,
            severity,
            description,
        }
    }
}

/// Trace issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceIssueType {
    /// High event rate
    HighEventRate,
    /// Slow processing
    SlowProcessing,
    /// Data loss
    DataLoss,
    /// Filter inefficiency
    FilterInefficient,
    /// Buffer overflow
    BufferOverflow,
    /// Probe overhead
    ProbeOverhead,
}

/// Trace recommendation
#[derive(Debug, Clone)]
pub struct TraceRecommendation {
    /// Action
    pub action: TraceAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

impl TraceRecommendation {
    /// Create new recommendation
    pub fn new(action: TraceAction, expected_improvement: f32, reason: String) -> Self {
        Self {
            action,
            expected_improvement,
            reason,
        }
    }
}

/// Trace actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceAction {
    /// Add filter
    AddFilter,
    /// Increase buffer
    IncreaseBuffer,
    /// Reduce rate
    ReduceRate,
    /// Disable tracepoint
    DisableTracepoint,
    /// Optimize filter
    OptimizeFilter,
}

/// Tracepoint Intelligence - comprehensive tracing analysis and optimization
pub struct TracepointIntelligence {
    /// Tracepoint manager
    tracepoint_mgr: TracepointManager,
    /// Probe manager
    probe_mgr: ProbeManager,
    /// Event ring buffer
    event_buffer: EventRingBuffer,
    /// Filters
    filters: BTreeMap<u64, EventFilter>,
    /// Next filter ID
    next_filter_id: AtomicU64,
    /// Performance analyzer
    perf_analyzer: PerformanceAnalyzer,
}

impl TracepointIntelligence {
    /// Create new tracepoint intelligence
    pub fn new(start_time: u64, buffer_capacity: usize) -> Self {
        Self {
            tracepoint_mgr: TracepointManager::new(),
            probe_mgr: ProbeManager::new(),
            event_buffer: EventRingBuffer::new(buffer_capacity),
            filters: BTreeMap::new(),
            next_filter_id: AtomicU64::new(1),
            perf_analyzer: PerformanceAnalyzer::new(start_time),
        }
    }

    /// Register tracepoint
    pub fn register_tracepoint(
        &mut self,
        name: String,
        subsystem: TracepointSubsystem,
        timestamp: u64,
    ) -> TracepointId {
        self.tracepoint_mgr.register(name, subsystem, timestamp)
    }

    /// Enable tracepoint
    pub fn enable_tracepoint(&mut self, id: TracepointId) -> bool {
        self.tracepoint_mgr.enable(id)
    }

    /// Disable tracepoint
    pub fn disable_tracepoint(&mut self, id: TracepointId) -> bool {
        self.tracepoint_mgr.disable(id)
    }

    /// Register probe
    pub fn register_probe(
        &mut self,
        probe_type: ProbeType,
        target: String,
        timestamp: u64,
    ) -> ProbeId {
        self.probe_mgr.register(probe_type, target, timestamp)
    }

    /// Attach probe to tracepoint
    pub fn attach_probe(&mut self, probe_id: ProbeId, tracepoint_id: TracepointId) -> bool {
        if self.probe_mgr.attach(probe_id, tracepoint_id) {
            if let Some(def) = self.tracepoint_mgr.get_mut(tracepoint_id) {
                def.probe_count += 1;
            }
            return true;
        }
        false
    }

    /// Enable probe
    pub fn enable_probe(&mut self, probe_id: ProbeId) -> bool {
        self.probe_mgr.enable(probe_id)
    }

    /// Disable probe
    pub fn disable_probe(&mut self, probe_id: ProbeId) -> bool {
        self.probe_mgr.disable(probe_id)
    }

    /// Add filter
    pub fn add_filter(&mut self, tracepoint_id: TracepointId, expression: FilterExpr) -> u64 {
        let id = self.next_filter_id.fetch_add(1, Ordering::Relaxed);
        let filter = EventFilter::new(id, tracepoint_id, expression);
        self.filters.insert(id, filter);
        id
    }

    /// Remove filter
    pub fn remove_filter(&mut self, filter_id: u64) -> bool {
        self.filters.remove(&filter_id).is_some()
    }

    /// Record event
    pub fn record_event(&mut self, event: EventData, processing_time_ns: u64) {
        // Apply filters
        let tracepoint_id = event.tracepoint_id;
        for filter in self.filters.values() {
            if filter.tracepoint_id == tracepoint_id && !filter.apply(&event) {
                return; // Filtered out
            }
        }

        // Record performance sample
        let sample = TraceSample {
            tracepoint_id,
            timestamp: event.timestamp,
            processing_time_ns,
            event_size: event.data.len(),
            cpu: event.cpu,
        };
        self.perf_analyzer.record(sample);

        // Store event
        self.event_buffer.write(event);
    }

    /// Analyze tracepoint
    pub fn analyze(&self, tracepoint_id: TracepointId) -> Option<TracepointAnalysis> {
        let _def = self.tracepoint_mgr.get(tracepoint_id)?;
        let stats = self.perf_analyzer.get_stats(tracepoint_id).cloned();

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        if let Some(ref s) = stats {
            // Check for high event rate
            if s.events_per_second > 10000.0 {
                health_score -= 20.0;
                issues.push(TraceIssue {
                    issue_type: TraceIssueType::HighEventRate,
                    severity: 7,
                    description: alloc::format!("High event rate: {:.0}/s", s.events_per_second),
                });
                recommendations.push(TraceRecommendation {
                    action: TraceAction::AddFilter,
                    expected_improvement: 30.0,
                    reason: String::from("Add filter to reduce event volume"),
                });
            }

            // Check for slow processing
            if s.avg_processing_ns > 10000.0 {
                health_score -= 15.0;
                issues.push(TraceIssue {
                    issue_type: TraceIssueType::SlowProcessing,
                    severity: 6,
                    description: alloc::format!(
                        "Slow processing: {:.0}ns avg",
                        s.avg_processing_ns
                    ),
                });
            }
        }

        // Check for data loss
        let events_lost = self.event_buffer.events_lost();
        if events_lost > 0 {
            health_score -= 25.0;
            issues.push(TraceIssue {
                issue_type: TraceIssueType::DataLoss,
                severity: 8,
                description: alloc::format!("{} events lost", events_lost),
            });
            recommendations.push(TraceRecommendation {
                action: TraceAction::IncreaseBuffer,
                expected_improvement: 20.0,
                reason: String::from("Increase buffer size to prevent data loss"),
            });
        }

        health_score = health_score.max(0.0);

        Some(TracepointAnalysis {
            tracepoint_id,
            health_score,
            stats,
            issues,
            recommendations,
        })
    }

    /// Get tracepoint manager
    pub fn tracepoint_manager(&self) -> &TracepointManager {
        &self.tracepoint_mgr
    }

    /// Get tracepoint manager mutably
    pub fn tracepoint_manager_mut(&mut self) -> &mut TracepointManager {
        &mut self.tracepoint_mgr
    }

    /// Get probe manager
    pub fn probe_manager(&self) -> &ProbeManager {
        &self.probe_mgr
    }

    /// Get probe manager mutably
    pub fn probe_manager_mut(&mut self) -> &mut ProbeManager {
        &mut self.probe_mgr
    }

    /// Get performance analyzer
    pub fn performance_analyzer(&self) -> &PerformanceAnalyzer {
        &self.perf_analyzer
    }

    /// Get buffer statistics
    pub fn buffer_stats(&self) -> (u64, u64, usize) {
        (
            self.event_buffer.events_written(),
            self.event_buffer.events_lost(),
            self.event_buffer.available(),
        )
    }

    /// Get filter by ID
    pub fn get_filter(&self, id: u64) -> Option<&EventFilter> {
        self.filters.get(&id)
    }
}
