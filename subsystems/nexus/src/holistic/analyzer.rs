//! # System-Wide Analyzer
//!
//! Cross-cutting analysis that spans all subsystems:
//! - CPU, memory, I/O, network correlation analysis
//! - Bottleneck identification
//! - Resource contention detection
//! - System health scoring
//! - Trend analysis and prediction
//! - Anomaly detection at system level

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SYSTEM METRICS
// ============================================================================

/// System-wide metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemMetricType {
    /// Overall CPU utilization (percent * 100)
    CpuUtilization,
    /// Per-core CPU utilization
    CpuPerCore,
    /// CPU run queue length
    CpuRunQueue,
    /// Context switches per second
    ContextSwitches,
    /// Memory utilization (percent * 100)
    MemoryUtilization,
    /// Memory pressure score
    MemoryPressure,
    /// Page fault rate
    PageFaultRate,
    /// Swap usage
    SwapUsage,
    /// Disk I/O operations per second
    DiskIops,
    /// Disk bandwidth (bytes/sec)
    DiskBandwidth,
    /// Disk queue depth
    DiskQueueDepth,
    /// I/O wait percentage
    IoWait,
    /// Network packets per second
    NetworkPps,
    /// Network bandwidth (bytes/sec)
    NetworkBandwidth,
    /// Network error rate
    NetworkErrors,
    /// Interrupt rate
    InterruptRate,
    /// Syscall rate
    SyscallRate,
    /// Process count
    ProcessCount,
    /// Thread count
    ThreadCount,
    /// Open file descriptors
    OpenFds,
}

/// A metric sample
#[derive(Debug, Clone, Copy)]
pub struct MetricSample {
    /// Metric type
    pub metric: SystemMetricType,
    /// Value
    pub value: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Metric time series
struct MetricTimeSeries {
    /// Samples (ring buffer)
    samples: Vec<MetricSample>,
    /// Capacity
    capacity: usize,
    /// Write position
    write_pos: usize,
    /// Count
    count: usize,
    /// Running sum (for fast average)
    running_sum: u64,
    /// Running min
    running_min: u64,
    /// Running max
    running_max: u64,
}

impl MetricTimeSeries {
    fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
            write_pos: 0,
            count: 0,
            running_sum: 0,
            running_min: u64::MAX,
            running_max: 0,
        }
    }

    fn record(&mut self, sample: MetricSample) {
        if self.samples.len() < self.capacity {
            self.samples.push(sample);
        } else {
            // Remove old value from running sum
            let old = self.samples[self.write_pos].value;
            self.running_sum = self.running_sum.saturating_sub(old);
            self.samples[self.write_pos] = sample;
        }
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.count += 1;
        self.running_sum += sample.value;
        if sample.value < self.running_min {
            self.running_min = sample.value;
        }
        if sample.value > self.running_max {
            self.running_max = sample.value;
        }
    }

    fn average(&self) -> f64 {
        let len = self.samples.len();
        if len == 0 {
            return 0.0;
        }
        self.running_sum as f64 / len as f64
    }

    fn latest(&self) -> Option<u64> {
        if self.samples.is_empty() {
            return None;
        }
        let idx = if self.write_pos == 0 {
            self.samples.len() - 1
        } else {
            self.write_pos - 1
        };
        Some(self.samples[idx].value)
    }

    fn variance(&self) -> f64 {
        let len = self.samples.len();
        if len < 2 {
            return 0.0;
        }
        let avg = self.average();
        let sum_sq: f64 = self
            .samples
            .iter()
            .map(|s| {
                let d = s.value as f64 - avg;
                d * d
            })
            .sum();
        sum_sq / (len - 1) as f64
    }

    fn stddev(&self) -> f64 {
        libm::sqrt(self.variance())
    }

    fn trend(&self) -> f64 {
        let len = self.samples.len();
        if len < 3 {
            return 0.0;
        }
        // Simple linear regression slope
        let n = len as f64;
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_xx = 0.0f64;

        for (i, s) in self.samples.iter().enumerate() {
            let x = i as f64;
            let y = s.value as f64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let denom = n * sum_xx - sum_x * sum_x;
        if libm::fabs(denom) < 1e-10 {
            return 0.0;
        }
        (n * sum_xy - sum_x * sum_y) / denom
    }
}

// ============================================================================
// BOTTLENECK DETECTION
// ============================================================================

/// System bottleneck type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckType {
    /// CPU saturated
    CpuSaturation,
    /// Memory exhaustion
    MemoryExhaustion,
    /// I/O bottleneck
    IoBottleneck,
    /// Network bottleneck
    NetworkBottleneck,
    /// Lock contention
    LockContention,
    /// Context switch overhead
    ContextSwitchOverhead,
    /// Interrupt storm
    InterruptStorm,
    /// FD exhaustion
    FdExhaustion,
    /// None detected
    None,
}

/// Severity of bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BottleneckSeverity {
    /// Low impact
    Low,
    /// Moderate impact
    Moderate,
    /// High impact
    High,
    /// Critical impact
    Critical,
}

/// A detected bottleneck
#[derive(Debug, Clone)]
pub struct Bottleneck {
    /// Type
    pub bottleneck_type: BottleneckType,
    /// Severity
    pub severity: BottleneckSeverity,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Primary metric value
    pub metric_value: u64,
    /// Threshold exceeded
    pub threshold: u64,
    /// Processes most affected
    pub affected_pids: Vec<u64>,
    /// Detection timestamp
    pub timestamp: u64,
}

// ============================================================================
// CORRELATION ENGINE
// ============================================================================

/// Correlation between two metrics
#[derive(Debug, Clone, Copy)]
pub struct MetricCorrelation {
    /// First metric
    pub metric_a: SystemMetricType,
    /// Second metric
    pub metric_b: SystemMetricType,
    /// Correlation coefficient (-1.0 to 1.0)
    pub coefficient: f64,
    /// Sample count used
    pub sample_count: usize,
}

/// Correlation analyzer
pub struct CorrelationAnalyzer {
    /// Metric pairs and their correlation
    correlations: Vec<MetricCorrelation>,
    /// Update interval (samples)
    update_interval: usize,
    /// Samples since last update
    samples_since_update: usize,
}

impl CorrelationAnalyzer {
    pub fn new(update_interval: usize) -> Self {
        Self {
            correlations: Vec::new(),
            update_interval,
            samples_since_update: 0,
        }
    }

    /// Compute correlation between two series
    pub fn compute_correlation(series_a: &[u64], series_b: &[u64]) -> f64 {
        let n = series_a.len().min(series_b.len());
        if n < 3 {
            return 0.0;
        }

        let avg_a: f64 = series_a[..n].iter().map(|&v| v as f64).sum::<f64>() / n as f64;
        let avg_b: f64 = series_b[..n].iter().map(|&v| v as f64).sum::<f64>() / n as f64;

        let mut cov = 0.0f64;
        let mut var_a = 0.0f64;
        let mut var_b = 0.0f64;

        for i in 0..n {
            let da = series_a[i] as f64 - avg_a;
            let db = series_b[i] as f64 - avg_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        let denom = libm::sqrt(var_a * var_b);
        if denom < 1e-10 {
            return 0.0;
        }
        cov / denom
    }

    /// Get top positive correlations
    pub fn top_positive(&self, count: usize) -> Vec<&MetricCorrelation> {
        let mut sorted: Vec<&MetricCorrelation> = self.correlations.iter().collect();
        sorted.sort_by(|a, b| b.coefficient.partial_cmp(&a.coefficient).unwrap_or(core::cmp::Ordering::Equal));
        sorted.truncate(count);
        sorted
    }

    /// Get top negative correlations
    pub fn top_negative(&self, count: usize) -> Vec<&MetricCorrelation> {
        let mut sorted: Vec<&MetricCorrelation> = self.correlations.iter().collect();
        sorted.sort_by(|a, b| a.coefficient.partial_cmp(&b.coefficient).unwrap_or(core::cmp::Ordering::Equal));
        sorted.truncate(count);
        sorted
    }

    pub fn needs_update(&self) -> bool {
        self.samples_since_update >= self.update_interval
    }

    pub fn mark_updated(&mut self) {
        self.samples_since_update = 0;
    }

    pub fn record_sample(&mut self) {
        self.samples_since_update += 1;
    }

    pub fn update_correlations(&mut self, correlations: Vec<MetricCorrelation>) {
        self.correlations = correlations;
        self.mark_updated();
    }
}

// ============================================================================
// SYSTEM HEALTH
// ============================================================================

/// System health score breakdown
#[derive(Debug, Clone)]
pub struct SystemHealth {
    /// Overall health (0.0 = critical, 1.0 = perfect)
    pub overall: f64,
    /// CPU health
    pub cpu_health: f64,
    /// Memory health
    pub memory_health: f64,
    /// I/O health
    pub io_health: f64,
    /// Network health
    pub network_health: f64,
    /// Active bottlenecks
    pub bottlenecks: Vec<Bottleneck>,
    /// Health trend (positive = improving)
    pub trend: f64,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// SYSTEM ANALYZER
// ============================================================================

/// Thresholds for bottleneck detection
#[derive(Debug, Clone)]
pub struct AnalyzerThresholds {
    pub cpu_high: u64,       // percent * 100
    pub cpu_critical: u64,
    pub memory_high: u64,
    pub memory_critical: u64,
    pub io_high: u64,        // IOPS
    pub io_critical: u64,
    pub network_high: u64,   // bytes/sec
    pub network_critical: u64,
    pub ctx_switch_high: u64,
    pub irq_high: u64,
}

impl Default for AnalyzerThresholds {
    fn default() -> Self {
        Self {
            cpu_high: 8000,           // 80%
            cpu_critical: 9500,       // 95%
            memory_high: 8000,
            memory_critical: 9500,
            io_high: 10000,
            io_critical: 50000,
            network_high: 1_000_000_000,  // 1 Gbps
            network_critical: 10_000_000_000,
            ctx_switch_high: 50000,
            irq_high: 100000,
        }
    }
}

/// System-wide analyzer
pub struct SystemAnalyzer {
    /// Per-metric time series
    series: BTreeMap<u8, MetricTimeSeries>,
    /// Series capacity
    series_capacity: usize,
    /// Correlation analyzer
    correlations: CorrelationAnalyzer,
    /// Thresholds
    thresholds: AnalyzerThresholds,
    /// Active bottlenecks
    active_bottlenecks: Vec<Bottleneck>,
    /// Health history
    health_history: Vec<f64>,
    /// Max health history
    max_health_history: usize,
    /// Total samples recorded
    pub total_samples: u64,
    /// Total bottlenecks detected
    pub total_bottlenecks: u64,
}

impl SystemAnalyzer {
    pub fn new(series_capacity: usize) -> Self {
        Self {
            series: BTreeMap::new(),
            series_capacity,
            correlations: CorrelationAnalyzer::new(100),
            thresholds: AnalyzerThresholds::default(),
            active_bottlenecks: Vec::new(),
            health_history: Vec::new(),
            max_health_history: 60,
            total_samples: 0,
            total_bottlenecks: 0,
        }
    }

    /// Record a metric sample
    pub fn record(&mut self, metric: SystemMetricType, value: u64, timestamp: u64) {
        let key = metric as u8;
        let series = self
            .series
            .entry(key)
            .or_insert_with(|| MetricTimeSeries::new(self.series_capacity));

        series.record(MetricSample {
            metric,
            value,
            timestamp,
        });

        self.total_samples += 1;
        self.correlations.record_sample();
    }

    /// Get latest value for metric
    pub fn latest(&self, metric: SystemMetricType) -> Option<u64> {
        self.series.get(&(metric as u8))?.latest()
    }

    /// Get average for metric
    pub fn average(&self, metric: SystemMetricType) -> f64 {
        self.series
            .get(&(metric as u8))
            .map_or(0.0, |s| s.average())
    }

    /// Get trend for metric
    pub fn trend(&self, metric: SystemMetricType) -> f64 {
        self.series
            .get(&(metric as u8))
            .map_or(0.0, |s| s.trend())
    }

    /// Detect bottlenecks
    pub fn detect_bottlenecks(&mut self, timestamp: u64) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // CPU check
        if let Some(cpu) = self.latest(SystemMetricType::CpuUtilization) {
            if cpu >= self.thresholds.cpu_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::CpuSaturation,
                    severity: BottleneckSeverity::Critical,
                    confidence: 0.95,
                    metric_value: cpu,
                    threshold: self.thresholds.cpu_critical,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            } else if cpu >= self.thresholds.cpu_high {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::CpuSaturation,
                    severity: BottleneckSeverity::Moderate,
                    confidence: 0.8,
                    metric_value: cpu,
                    threshold: self.thresholds.cpu_high,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            }
        }

        // Memory check
        if let Some(mem) = self.latest(SystemMetricType::MemoryUtilization) {
            if mem >= self.thresholds.memory_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::MemoryExhaustion,
                    severity: BottleneckSeverity::Critical,
                    confidence: 0.95,
                    metric_value: mem,
                    threshold: self.thresholds.memory_critical,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            } else if mem >= self.thresholds.memory_high {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::MemoryExhaustion,
                    severity: BottleneckSeverity::Moderate,
                    confidence: 0.8,
                    metric_value: mem,
                    threshold: self.thresholds.memory_high,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            }
        }

        // I/O check
        if let Some(iops) = self.latest(SystemMetricType::DiskIops) {
            if iops >= self.thresholds.io_critical {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::IoBottleneck,
                    severity: BottleneckSeverity::Critical,
                    confidence: 0.9,
                    metric_value: iops,
                    threshold: self.thresholds.io_critical,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            } else if iops >= self.thresholds.io_high {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::IoBottleneck,
                    severity: BottleneckSeverity::Moderate,
                    confidence: 0.7,
                    metric_value: iops,
                    threshold: self.thresholds.io_high,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            }
        }

        // Context switch overhead
        if let Some(cs) = self.latest(SystemMetricType::ContextSwitches) {
            if cs >= self.thresholds.ctx_switch_high {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::ContextSwitchOverhead,
                    severity: BottleneckSeverity::Moderate,
                    confidence: 0.7,
                    metric_value: cs,
                    threshold: self.thresholds.ctx_switch_high,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            }
        }

        // Interrupt storm
        if let Some(irq) = self.latest(SystemMetricType::InterruptRate) {
            if irq >= self.thresholds.irq_high {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::InterruptStorm,
                    severity: BottleneckSeverity::High,
                    confidence: 0.8,
                    metric_value: irq,
                    threshold: self.thresholds.irq_high,
                    affected_pids: Vec::new(),
                    timestamp,
                });
            }
        }

        self.active_bottlenecks = bottlenecks.clone();
        self.total_bottlenecks += bottlenecks.len() as u64;
        bottlenecks
    }

    /// Compute system health
    pub fn compute_health(&mut self, timestamp: u64) -> SystemHealth {
        let cpu_health = self
            .latest(SystemMetricType::CpuUtilization)
            .map_or(1.0, |v| 1.0 - (v as f64 / 10000.0));

        let memory_health = self
            .latest(SystemMetricType::MemoryUtilization)
            .map_or(1.0, |v| 1.0 - (v as f64 / 10000.0));

        let io_health = self
            .latest(SystemMetricType::IoWait)
            .map_or(1.0, |v| 1.0 - (v as f64 / 10000.0));

        let network_health = self
            .latest(SystemMetricType::NetworkErrors)
            .map_or(1.0, |v| {
                let rate = v as f64;
                1.0 / (1.0 + rate / 100.0)
            });

        // Weight: CPU 30%, Memory 30%, I/O 25%, Network 15%
        let overall = cpu_health * 0.30
            + memory_health * 0.30
            + io_health * 0.25
            + network_health * 0.15;

        let overall = if overall < 0.0 { 0.0 } else if overall > 1.0 { 1.0 } else { overall };

        // Track health trend
        self.health_history.push(overall);
        if self.health_history.len() > self.max_health_history {
            self.health_history.remove(0);
        }

        let trend = if self.health_history.len() >= 3 {
            let recent = self.health_history[self.health_history.len() - 1];
            let older = self.health_history[self.health_history.len() - 3];
            recent - older
        } else {
            0.0
        };

        SystemHealth {
            overall,
            cpu_health,
            memory_health,
            io_health,
            network_health,
            bottlenecks: self.active_bottlenecks.clone(),
            trend,
            timestamp,
        }
    }

    /// Get correlation analyzer
    pub fn correlations(&self) -> &CorrelationAnalyzer {
        &self.correlations
    }

    /// Metric count being tracked
    pub fn metric_count(&self) -> usize {
        self.series.len()
    }
}
