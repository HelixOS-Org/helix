//! # Execution Monitor
//!
//! Monitors action execution and detects anomalies.
//! Provides feedback for action adjustment.
//!
//! Part of Year 2 COGNITION - Action Execution Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// MONITOR TYPES
// ============================================================================

/// Execution
#[derive(Debug, Clone)]
pub struct Execution {
    /// Execution ID
    pub id: u64,
    /// Action being executed
    pub action_id: u64,
    /// Status
    pub status: ExecutionStatus,
    /// Started
    pub started: Timestamp,
    /// Completed
    pub completed: Option<Timestamp>,
    /// Progress
    pub progress: f64,
    /// Metrics
    pub metrics: ExecutionMetrics,
    /// Events
    pub events: Vec<ExecutionEvent>,
    /// Outcome
    pub outcome: Option<ExecutionOutcome>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

/// Execution metrics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ExecutionMetrics {
    /// Duration (ns)
    pub duration_ns: u64,
    /// CPU usage
    pub cpu_percent: f64,
    /// Memory usage
    pub memory_bytes: u64,
    /// Operations performed
    pub operations: u64,
    /// Errors
    pub errors: u64,
    /// Retries
    pub retries: u64,
}

/// Execution event
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: EventType,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Severity
    pub severity: Severity,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Started,
    Progress,
    Checkpoint,
    Warning,
    Error,
    Retry,
    Completed,
    Failed,
}

/// Severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Execution outcome
#[derive(Debug, Clone)]
pub struct ExecutionOutcome {
    /// Success
    pub success: bool,
    /// Result value
    pub result: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Side effects
    pub side_effects: Vec<SideEffect>,
}

/// Side effect
#[derive(Debug, Clone)]
pub struct SideEffect {
    /// Type
    pub effect_type: String,
    /// Target
    pub target: String,
    /// Description
    pub description: String,
}

// ============================================================================
// ANOMALY DETECTION
// ============================================================================

/// Anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly ID
    pub id: u64,
    /// Execution ID
    pub execution_id: u64,
    /// Type
    pub anomaly_type: AnomalyType,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
    /// Detected at
    pub detected_at: Timestamp,
    /// Resolved
    pub resolved: bool,
}

/// Anomaly type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    /// Execution taking too long
    SlowExecution,
    /// Too many errors
    HighErrorRate,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Unexpected behavior
    UnexpectedBehavior,
    /// Deadlock detected
    Deadlock,
    /// Infinite loop
    InfiniteLoop,
    /// Memory leak
    MemoryLeak,
}

// ============================================================================
// EXECUTION MONITOR
// ============================================================================

/// Execution monitor
pub struct ExecutionMonitor {
    /// Active executions
    executions: BTreeMap<u64, Execution>,
    /// Completed executions
    history: VecDeque<Execution>,
    /// Detected anomalies
    anomalies: BTreeMap<u64, Anomaly>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MonitorConfig,
    /// Statistics
    stats: MonitorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Error rate threshold
    pub error_threshold: f64,
    /// Memory threshold (bytes)
    pub memory_threshold: u64,
    /// History size
    pub history_size: usize,
    /// Enable anomaly detection
    pub anomaly_detection: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            timeout_ns: 30_000_000_000,          // 30 seconds
            error_threshold: 0.1,                // 10% error rate
            memory_threshold: 1024 * 1024 * 100, // 100 MB
            history_size: 1000,
            anomaly_detection: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MonitorStats {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful: u64,
    /// Failed executions
    pub failed: u64,
    /// Anomalies detected
    pub anomalies_detected: u64,
    /// Average duration (ns)
    pub avg_duration_ns: f64,
}

impl ExecutionMonitor {
    /// Create new monitor
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            executions: BTreeMap::new(),
            history: VecDeque::new(),
            anomalies: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: MonitorStats::default(),
        }
    }

    /// Start monitoring execution
    pub fn start(&mut self, action_id: u64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let execution = Execution {
            id,
            action_id,
            status: ExecutionStatus::Running,
            started: now,
            completed: None,
            progress: 0.0,
            metrics: ExecutionMetrics::default(),
            events: vec![ExecutionEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                event_type: EventType::Started,
                message: "Execution started".into(),
                timestamp: now,
                severity: Severity::Info,
            }],
            outcome: None,
        };

        self.executions.insert(id, execution);
        self.stats.total_executions += 1;

        id
    }

    /// Update progress
    pub fn update_progress(&mut self, execution_id: u64, progress: f64) {
        if let Some(execution) = self.executions.get_mut(&execution_id) {
            execution.progress = progress.clamp(0.0, 1.0);

            execution.events.push(ExecutionEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                event_type: EventType::Progress,
                message: format!("Progress: {:.1}%", progress * 100.0),
                timestamp: Timestamp::now(),
                severity: Severity::Debug,
            });
        }
    }

    /// Update metrics
    #[inline]
    pub fn update_metrics(&mut self, execution_id: u64, metrics: ExecutionMetrics) {
        if let Some(execution) = self.executions.get_mut(&execution_id) {
            execution.metrics = metrics;

            // Check for anomalies
            if self.config.anomaly_detection {
                self.check_anomalies(execution_id);
            }
        }
    }

    /// Record event
    pub fn record_event(
        &mut self,
        execution_id: u64,
        event_type: EventType,
        message: &str,
        severity: Severity,
    ) {
        if let Some(execution) = self.executions.get_mut(&execution_id) {
            execution.events.push(ExecutionEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                event_type,
                message: message.into(),
                timestamp: Timestamp::now(),
                severity,
            });

            if event_type == EventType::Error {
                execution.metrics.errors += 1;
            } else if event_type == EventType::Retry {
                execution.metrics.retries += 1;
            }
        }
    }

    /// Complete execution
    pub fn complete(&mut self, execution_id: u64, success: bool, result: Option<&str>) {
        if let Some(execution) = self.executions.get_mut(&execution_id) {
            let now = Timestamp::now();

            execution.status = if success {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed
            };
            execution.completed = Some(now);
            execution.progress = 1.0;
            execution.metrics.duration_ns = now.0 - execution.started.0;

            execution.outcome = Some(ExecutionOutcome {
                success,
                result: result.map(String::from),
                error: if !success {
                    result.map(String::from)
                } else {
                    None
                },
                side_effects: Vec::new(),
            });

            execution.events.push(ExecutionEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                event_type: if success {
                    EventType::Completed
                } else {
                    EventType::Failed
                },
                message: if success {
                    "Execution completed"
                } else {
                    "Execution failed"
                }
                .into(),
                timestamp: now,
                severity: if success {
                    Severity::Info
                } else {
                    Severity::Error
                },
            });

            // Update stats
            let n = self.stats.total_executions as f64;
            self.stats.avg_duration_ns =
                (self.stats.avg_duration_ns * (n - 1.0) + execution.metrics.duration_ns as f64) / n;

            if success {
                self.stats.successful += 1;
            } else {
                self.stats.failed += 1;
            }
        }

        // Move to history
        if let Some(execution) = self.executions.remove(&execution_id) {
            self.history.push_back(execution);

            // Trim history
            while self.history.len() > self.config.history_size {
                self.history.pop_front();
            }
        }
    }

    /// Cancel execution
    pub fn cancel(&mut self, execution_id: u64, reason: &str) {
        if let Some(execution) = self.executions.get_mut(&execution_id) {
            execution.status = ExecutionStatus::Cancelled;
            execution.completed = Some(Timestamp::now());

            execution.events.push(ExecutionEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                event_type: EventType::Failed,
                message: format!("Cancelled: {}", reason),
                timestamp: Timestamp::now(),
                severity: Severity::Warning,
            });
        }
    }

    fn check_anomalies(&mut self, execution_id: u64) {
        let execution = match self.executions.get(&execution_id) {
            Some(e) => e.clone(),
            None => return,
        };

        let now = Timestamp::now();
        let elapsed = now.0 - execution.started.0;

        // Check timeout
        if elapsed > self.config.timeout_ns {
            self.create_anomaly(
                execution_id,
                AnomalyType::SlowExecution,
                "Execution exceeds timeout",
                Severity::Warning,
            );
        }

        // Check error rate
        if execution.metrics.operations > 0 {
            let error_rate = execution.metrics.errors as f64 / execution.metrics.operations as f64;
            if error_rate > self.config.error_threshold {
                self.create_anomaly(
                    execution_id,
                    AnomalyType::HighErrorRate,
                    &format!("High error rate: {:.1}%", error_rate * 100.0),
                    Severity::Warning,
                );
            }
        }

        // Check memory
        if execution.metrics.memory_bytes > self.config.memory_threshold {
            self.create_anomaly(
                execution_id,
                AnomalyType::ResourceExhaustion,
                "Memory threshold exceeded",
                Severity::Error,
            );
        }
    }

    fn create_anomaly(
        &mut self,
        execution_id: u64,
        anomaly_type: AnomalyType,
        description: &str,
        severity: Severity,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Check if similar anomaly already exists
        let exists = self.anomalies.values().any(|a| {
            a.execution_id == execution_id && a.anomaly_type == anomaly_type && !a.resolved
        });

        if exists {
            return 0;
        }

        let anomaly = Anomaly {
            id,
            execution_id,
            anomaly_type,
            description: description.into(),
            severity,
            detected_at: Timestamp::now(),
            resolved: false,
        };

        self.anomalies.insert(id, anomaly);
        self.stats.anomalies_detected += 1;

        id
    }

    /// Resolve anomaly
    #[inline]
    pub fn resolve_anomaly(&mut self, anomaly_id: u64) {
        if let Some(anomaly) = self.anomalies.get_mut(&anomaly_id) {
            anomaly.resolved = true;
        }
    }

    /// Get execution
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Execution> {
        self.executions.get(&id)
    }

    /// Get active executions
    #[inline]
    pub fn active(&self) -> Vec<&Execution> {
        self.executions
            .values()
            .filter(|e| e.status == ExecutionStatus::Running)
            .collect()
    }

    /// Get anomalies
    #[inline(always)]
    pub fn unresolved_anomalies(&self) -> Vec<&Anomaly> {
        self.anomalies.values().filter(|a| !a.resolved).collect()
    }

    /// Get history
    #[inline(always)]
    pub fn history(&self) -> &[Execution] {
        &self.history
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &MonitorStats {
        &self.stats
    }
}

impl Default for ExecutionMonitor {
    fn default() -> Self {
        Self::new(MonitorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_and_complete() {
        let mut monitor = ExecutionMonitor::default();

        let id = monitor.start(1);
        assert!(monitor.get(id).is_some());
        assert_eq!(monitor.get(id).unwrap().status, ExecutionStatus::Running);

        monitor.complete(id, true, Some("Success"));

        // Should be in history now
        assert!(monitor.get(id).is_none());
        assert!(!monitor.history.is_empty());
    }

    #[test]
    fn test_update_progress() {
        let mut monitor = ExecutionMonitor::default();

        let id = monitor.start(1);
        monitor.update_progress(id, 0.5);

        let exec = monitor.get(id).unwrap();
        assert!((exec.progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_record_events() {
        let mut monitor = ExecutionMonitor::default();

        let id = monitor.start(1);
        monitor.record_event(id, EventType::Checkpoint, "Checkpoint 1", Severity::Info);
        monitor.record_event(id, EventType::Error, "Error occurred", Severity::Error);

        let exec = monitor.get(id).unwrap();
        assert!(exec.events.len() > 1);
        assert_eq!(exec.metrics.errors, 1);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut monitor = ExecutionMonitor::new(MonitorConfig {
            memory_threshold: 100,
            ..Default::default()
        });

        let id = monitor.start(1);

        let metrics = ExecutionMetrics {
            memory_bytes: 200, // Exceeds threshold
            operations: 10,
            ..Default::default()
        };

        monitor.update_metrics(id, metrics);

        let anomalies = monitor.unresolved_anomalies();
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_cancel() {
        let mut monitor = ExecutionMonitor::default();

        let id = monitor.start(1);
        monitor.cancel(id, "User requested");

        let exec = monitor.get(id).unwrap();
        assert_eq!(exec.status, ExecutionStatus::Cancelled);
    }
}
