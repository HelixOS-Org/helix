//! # Application Anomaly Detection
//!
//! Real-time detection of abnormal application behavior:
//! - Statistical anomaly detection (z-score, IQR)
//! - Behavioral anomalies (unusual syscall patterns)
//! - Resource anomalies (sudden spikes/drops)
//! - Security anomalies (privilege escalation attempts)
//! - Performance anomalies (degradation detection)

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ANOMALY TYPES
// ============================================================================

/// Type of anomaly detected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    /// CPU usage spike
    CpuSpike,
    /// CPU usage drop
    CpuDrop,
    /// Memory leak pattern
    MemoryLeak,
    /// Memory spike
    MemorySpike,
    /// I/O stall
    IoStall,
    /// I/O flood
    IoFlood,
    /// Network flood
    NetworkFlood,
    /// Unusual syscall pattern
    UnusualSyscallPattern,
    /// Rapid process forking
    ForkBomb,
    /// File descriptor leak
    FdLeak,
    /// Excessive context switches
    ExcessiveContextSwitches,
    /// Privilege escalation attempt
    PrivilegeEscalation,
    /// Unusual file access
    UnusualFileAccess,
    /// Deadlock suspected
    DeadlockSuspected,
    /// Livelock suspected
    LivelockSuspected,
    /// Runaway process (CPU hog)
    RunawayProcess,
    /// Stalled process (no progress)
    StalledProcess,
}

/// Anomaly severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalySeverity {
    /// Informational (may be normal)
    Info,
    /// Warning (worth investigating)
    Warning,
    /// Alert (likely problem)
    Alert,
    /// Critical (immediate action needed)
    Critical,
}

/// A detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Severity
    pub severity: AnomalySeverity,
    /// Affected process
    pub pid: u64,
    /// When detected
    pub timestamp: u64,
    /// Measured value
    pub value: f64,
    /// Expected value
    pub expected: f64,
    /// Deviation (in standard deviations or %)
    pub deviation: f64,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Whether auto-remediation was applied
    pub remediated: bool,
}

// ============================================================================
// STATISTICAL DETECTOR
// ============================================================================

/// Running statistics for a metric
#[derive(Debug, Clone)]
pub struct RunningStats {
    /// Number of samples
    count: u64,
    /// Running mean
    mean: f64,
    /// Running M2 (for variance)
    m2: f64,
    /// Minimum observed
    min: f64,
    /// Maximum observed
    max: f64,
    /// Recent values (sliding window)
    recent: Vec<f64>,
    /// Max window size
    window_size: usize,
}

impl RunningStats {
    pub fn new(window_size: usize) -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            recent: Vec::new(),
            window_size,
        }
    }

    /// Add a sample (Welford's online algorithm)
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        if self.recent.len() >= self.window_size {
            self.recent.remove(0);
        }
        self.recent.push(value);
    }

    /// Variance
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    /// Standard deviation
    pub fn std_dev(&self) -> f64 {
        let var = self.variance();
        if var > 0.0 {
            libm::sqrt(var)
        } else {
            0.0
        }
    }

    /// Z-score for a value
    pub fn z_score(&self, value: f64) -> f64 {
        let sd = self.std_dev();
        if sd < 0.001 {
            0.0
        } else {
            (value - self.mean) / sd
        }
    }

    /// IQR-based outlier detection
    pub fn is_iqr_outlier(&self, value: f64) -> bool {
        if self.recent.len() < 10 {
            return false;
        }
        let mut sorted = self.recent.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let q1 = sorted[sorted.len() / 4];
        let q3 = sorted[3 * sorted.len() / 4];
        let iqr = q3 - q1;
        let lower = q1 - 1.5 * iqr;
        let upper = q3 + 1.5 * iqr;
        value < lower || value > upper
    }

    /// Mean value
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Recent average (over window)
    pub fn recent_avg(&self) -> f64 {
        if self.recent.is_empty() {
            0.0
        } else {
            self.recent.iter().sum::<f64>() / self.recent.len() as f64
        }
    }
}

// ============================================================================
// PER-PROCESS ANOMALY DETECTOR
// ============================================================================

/// Per-process anomaly detector
#[derive(Debug)]
pub struct ProcessAnomalyDetector {
    /// CPU usage statistics
    cpu_stats: RunningStats,
    /// Memory usage statistics
    memory_stats: RunningStats,
    /// Syscall rate statistics
    syscall_rate_stats: RunningStats,
    /// I/O rate statistics
    io_rate_stats: RunningStats,
    /// FD count statistics
    fd_count_stats: RunningStats,
    /// Context switch statistics
    ctx_switch_stats: RunningStats,
    /// Z-score threshold for anomaly
    z_threshold: f64,
    /// Detected anomalies
    anomalies: Vec<Anomaly>,
    /// Max anomalies to keep
    max_anomalies: usize,
    /// Total anomalies detected
    pub total_detected: u64,
}

impl ProcessAnomalyDetector {
    pub fn new(z_threshold: f64) -> Self {
        Self {
            cpu_stats: RunningStats::new(60),
            memory_stats: RunningStats::new(60),
            syscall_rate_stats: RunningStats::new(60),
            io_rate_stats: RunningStats::new(60),
            fd_count_stats: RunningStats::new(60),
            ctx_switch_stats: RunningStats::new(60),
            z_threshold,
            anomalies: Vec::new(),
            max_anomalies: 100,
            total_detected: 0,
        }
    }

    /// Update CPU metric and check for anomalies
    pub fn update_cpu(&mut self, pid: u64, value: f64, timestamp: u64) -> Option<Anomaly> {
        self.cpu_stats.update(value);
        let z = self.cpu_stats.z_score(value);

        if libm::fabs(z) > self.z_threshold && self.cpu_stats.count > 10 {
            let anomaly_type = if z > 0.0 {
                AnomalyType::CpuSpike
            } else {
                AnomalyType::CpuDrop
            };
            let severity = if libm::fabs(z) > self.z_threshold * 2.0 {
                AnomalySeverity::Alert
            } else {
                AnomalySeverity::Warning
            };
            let anomaly = Anomaly {
                anomaly_type,
                severity,
                pid,
                timestamp,
                value,
                expected: self.cpu_stats.mean(),
                deviation: z,
                confidence: 1.0 - libm::exp(-libm::fabs(z)),
                remediated: false,
            };
            self.record_anomaly(anomaly.clone());
            return Some(anomaly);
        }
        None
    }

    /// Update memory metric and check for anomalies
    pub fn update_memory(&mut self, pid: u64, value: f64, timestamp: u64) -> Option<Anomaly> {
        self.memory_stats.update(value);

        // Memory leak detection: monotonically increasing over window
        if self.memory_stats.recent.len() >= 20 {
            let increasing = self
                .memory_stats
                .recent
                .windows(2)
                .filter(|w| w[1] > w[0])
                .count();
            let ratio = increasing as f64 / (self.memory_stats.recent.len() - 1) as f64;

            if ratio > 0.9 {
                let anomaly = Anomaly {
                    anomaly_type: AnomalyType::MemoryLeak,
                    severity: AnomalySeverity::Alert,
                    pid,
                    timestamp,
                    value,
                    expected: self.memory_stats.mean(),
                    deviation: ratio,
                    confidence: ratio,
                    remediated: false,
                };
                self.record_anomaly(anomaly.clone());
                return Some(anomaly);
            }
        }

        // Spike detection
        let z = self.memory_stats.z_score(value);
        if z > self.z_threshold && self.memory_stats.count > 10 {
            let anomaly = Anomaly {
                anomaly_type: AnomalyType::MemorySpike,
                severity: AnomalySeverity::Warning,
                pid,
                timestamp,
                value,
                expected: self.memory_stats.mean(),
                deviation: z,
                confidence: 1.0 - libm::exp(-z),
                remediated: false,
            };
            self.record_anomaly(anomaly.clone());
            return Some(anomaly);
        }
        None
    }

    /// Update syscall rate and check for anomalies
    pub fn update_syscall_rate(&mut self, pid: u64, value: f64, timestamp: u64) -> Option<Anomaly> {
        self.syscall_rate_stats.update(value);
        let z = self.syscall_rate_stats.z_score(value);

        if z > self.z_threshold * 3.0 && value > 100000.0 {
            // Possible fork bomb or DoS
            let anomaly = Anomaly {
                anomaly_type: AnomalyType::ForkBomb,
                severity: AnomalySeverity::Critical,
                pid,
                timestamp,
                value,
                expected: self.syscall_rate_stats.mean(),
                deviation: z,
                confidence: 0.9,
                remediated: false,
            };
            self.record_anomaly(anomaly.clone());
            return Some(anomaly);
        }
        None
    }

    /// Update FD count and check for leaks
    pub fn update_fd_count(&mut self, pid: u64, value: f64, timestamp: u64) -> Option<Anomaly> {
        self.fd_count_stats.update(value);

        // FD leak detection: monotonically increasing
        if self.fd_count_stats.recent.len() >= 15 {
            let increasing = self
                .fd_count_stats
                .recent
                .windows(2)
                .filter(|w| w[1] >= w[0])
                .count();
            let ratio = increasing as f64 / (self.fd_count_stats.recent.len() - 1) as f64;

            if ratio > 0.85 && value > 100.0 {
                let anomaly = Anomaly {
                    anomaly_type: AnomalyType::FdLeak,
                    severity: AnomalySeverity::Warning,
                    pid,
                    timestamp,
                    value,
                    expected: self.fd_count_stats.mean(),
                    deviation: ratio,
                    confidence: ratio * 0.9,
                    remediated: false,
                };
                self.record_anomaly(anomaly.clone());
                return Some(anomaly);
            }
        }
        None
    }

    fn record_anomaly(&mut self, anomaly: Anomaly) {
        self.total_detected += 1;
        if self.anomalies.len() >= self.max_anomalies {
            self.anomalies.remove(0);
        }
        self.anomalies.push(anomaly);
    }

    /// Get recent anomalies
    pub fn recent_anomalies(&self, n: usize) -> &[Anomaly] {
        let start = self.anomalies.len().saturating_sub(n);
        &self.anomalies[start..]
    }

    /// Has critical anomaly
    pub fn has_critical(&self) -> bool {
        self.anomalies
            .iter()
            .any(|a| a.severity == AnomalySeverity::Critical && !a.remediated)
    }
}

// ============================================================================
// GLOBAL ANOMALY MANAGER
// ============================================================================

/// System-wide anomaly detection manager
pub struct AnomalyManager {
    /// Per-process detectors
    detectors: BTreeMap<u64, ProcessAnomalyDetector>,
    /// Z-score threshold
    z_threshold: f64,
    /// Max processes
    max_processes: usize,
    /// Global anomaly log
    global_log: Vec<Anomaly>,
    /// Max global log size
    max_log: usize,
    /// Total system anomalies
    pub total_anomalies: u64,
    /// Critical anomaly count
    pub critical_count: u64,
}

impl AnomalyManager {
    pub fn new(z_threshold: f64, max_processes: usize) -> Self {
        Self {
            detectors: BTreeMap::new(),
            z_threshold,
            max_processes,
            global_log: Vec::new(),
            max_log: 10000,
            total_anomalies: 0,
            critical_count: 0,
        }
    }

    /// Get or create detector for a process
    pub fn get_detector(&mut self, pid: u64) -> &mut ProcessAnomalyDetector {
        let threshold = self.z_threshold;
        if !self.detectors.contains_key(&pid) && self.detectors.len() < self.max_processes {
            self.detectors.insert(pid, ProcessAnomalyDetector::new(threshold));
        }
        self.detectors
            .entry(pid)
            .or_insert_with(|| ProcessAnomalyDetector::new(threshold))
    }

    /// Report an anomaly
    pub fn report(&mut self, anomaly: Anomaly) {
        self.total_anomalies += 1;
        if anomaly.severity == AnomalySeverity::Critical {
            self.critical_count += 1;
        }
        if self.global_log.len() >= self.max_log {
            self.global_log.remove(0);
        }
        self.global_log.push(anomaly);
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.detectors.remove(&pid);
    }

    /// Processes with active critical anomalies
    pub fn critical_processes(&self) -> Vec<u64> {
        self.detectors
            .iter()
            .filter(|(_, d)| d.has_critical())
            .map(|(&pid, _)| pid)
            .collect()
    }

    /// Recent anomalies (global)
    pub fn recent_anomalies(&self, n: usize) -> &[Anomaly] {
        let start = self.global_log.len().saturating_sub(n);
        &self.global_log[start..]
    }
}
