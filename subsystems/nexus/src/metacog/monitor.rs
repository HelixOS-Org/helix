//! # Metacognition Monitor for NEXUS
//!
//! Year 2 "COGNITION" - Revolutionary kernel-level metacognitive monitoring
//! system that enables the AI to observe, evaluate, and regulate its own
//! cognitive processes in real-time.
//!
//! ## Features
//!
//! - Cognitive process monitoring
//! - Performance evaluation
//! - Confidence calibration
//! - Resource usage tracking
//! - Cognitive load estimation
//! - Anomaly detection in reasoning
//! - Self-diagnostics

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum monitored processes
const MAX_PROCESSES: usize = 1000;

/// History buffer size
const HISTORY_SIZE: usize = 1000;

/// Default confidence threshold
const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.7;

/// Default cognitive load threshold
const DEFAULT_LOAD_THRESHOLD: f64 = 0.8;

/// EMA smoothing factor
const EMA_ALPHA: f64 = 0.1;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Unique identifier for a cognitive process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CognitiveProcessId(pub u64);

/// Types of cognitive processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CognitiveProcessType {
    /// Perception/sensing
    Perception,
    /// Attention focusing
    Attention,
    /// Memory retrieval
    MemoryRetrieval,
    /// Memory encoding
    MemoryEncoding,
    /// Reasoning/inference
    Reasoning,
    /// Decision making
    Decision,
    /// Planning
    Planning,
    /// Learning
    Learning,
    /// Prediction
    Prediction,
    /// Explanation generation
    Explanation,
    /// Self-monitoring
    Monitoring,
    /// Error correction
    ErrorCorrection,
}

/// State of a cognitive process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Not yet started
    Pending,
    /// Currently running
    Running,
    /// Paused/waiting
    Paused,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Timeout
    Timeout,
}

/// A cognitive process being monitored
#[derive(Debug, Clone)]
pub struct CognitiveProcess {
    /// Process ID
    pub id: CognitiveProcessId,
    /// Process type
    pub process_type: CognitiveProcessType,
    /// Name/description
    pub name: String,
    /// Current state
    pub state: ProcessState,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp (if completed)
    pub end_time: Option<u64>,
    /// CPU cycles used
    pub cpu_cycles: u64,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Input size (elements)
    pub input_size: usize,
    /// Output size (elements)
    pub output_size: usize,
    /// Confidence score of result
    pub confidence: f64,
    /// Quality score (0-1)
    pub quality: f64,
    /// Parent process (if any)
    pub parent: Option<CognitiveProcessId>,
    /// Child processes
    pub children: Vec<CognitiveProcessId>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Custom metrics
    pub metrics: BTreeMap<String, f64>,
}

impl CognitiveProcess {
    /// Create a new cognitive process
    pub fn new(id: CognitiveProcessId, process_type: CognitiveProcessType, name: String) -> Self {
        Self {
            id,
            process_type,
            name,
            state: ProcessState::Pending,
            start_time: 0,
            end_time: None,
            cpu_cycles: 0,
            memory_used: 0,
            input_size: 0,
            output_size: 0,
            confidence: 0.0,
            quality: 0.0,
            parent: None,
            children: Vec::new(),
            error: None,
            metrics: BTreeMap::new(),
        }
    }

    /// Duration in cycles
    pub fn duration(&self) -> u64 {
        match self.end_time {
            Some(end) => end.saturating_sub(self.start_time),
            None => 0,
        }
    }

    /// Is the process complete?
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            ProcessState::Completed | ProcessState::Failed | ProcessState::Cancelled
        )
    }

    /// Is the process successful?
    pub fn is_successful(&self) -> bool {
        self.state == ProcessState::Completed
    }
}

// ============================================================================
// PERFORMANCE METRICS
// ============================================================================

/// Performance metrics for a cognitive domain
#[derive(Debug, Clone)]
pub struct DomainMetrics {
    /// Domain/process type
    pub domain: CognitiveProcessType,
    /// Total processes executed
    pub total_count: u64,
    /// Successful processes
    pub success_count: u64,
    /// Failed processes
    pub failure_count: u64,
    /// Total CPU cycles used
    pub total_cycles: u64,
    /// Total memory used
    pub total_memory: u64,
    /// Average duration
    pub avg_duration: f64,
    /// Average confidence
    pub avg_confidence: f64,
    /// Average quality
    pub avg_quality: f64,
    /// Success rate (EMA)
    pub success_rate: f64,
    /// Throughput (processes per unit time)
    pub throughput: f64,
    /// Current load (0-1)
    pub current_load: f64,
}

impl DomainMetrics {
    /// Create new metrics for a domain
    pub fn new(domain: CognitiveProcessType) -> Self {
        Self {
            domain,
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            total_cycles: 0,
            total_memory: 0,
            avg_duration: 0.0,
            avg_confidence: 0.0,
            avg_quality: 0.0,
            success_rate: 1.0, // Optimistic start
            throughput: 0.0,
            current_load: 0.0,
        }
    }

    /// Update metrics with a completed process
    pub fn update(&mut self, process: &CognitiveProcess) {
        self.total_count += 1;

        if process.is_successful() {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        self.total_cycles += process.cpu_cycles;
        self.total_memory += process.memory_used;

        // Update averages with EMA
        let duration = process.duration() as f64;
        self.avg_duration = self.avg_duration * (1.0 - EMA_ALPHA) + duration * EMA_ALPHA;
        self.avg_confidence =
            self.avg_confidence * (1.0 - EMA_ALPHA) + process.confidence * EMA_ALPHA;
        self.avg_quality = self.avg_quality * (1.0 - EMA_ALPHA) + process.quality * EMA_ALPHA;

        // Update success rate
        let current_success = if process.is_successful() { 1.0 } else { 0.0 };
        self.success_rate = self.success_rate * (1.0 - EMA_ALPHA) + current_success * EMA_ALPHA;
    }
}

/// Overall cognitive system metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Total processes executed
    pub total_processes: u64,
    /// Total CPU cycles used
    pub total_cycles: u64,
    /// Total memory used
    pub total_memory: u64,
    /// Overall success rate
    pub success_rate: f64,
    /// Overall confidence
    pub avg_confidence: f64,
    /// Cognitive load (0-1)
    pub cognitive_load: f64,
    /// Processes per second
    pub throughput: f64,
    /// Number of active processes
    pub active_processes: usize,
    /// Number of pending processes
    pub pending_processes: usize,
    /// Health score (0-1)
    pub health_score: f64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_processes: 0,
            total_cycles: 0,
            total_memory: 0,
            success_rate: 1.0,
            avg_confidence: 1.0,
            cognitive_load: 0.0,
            throughput: 0.0,
            active_processes: 0,
            pending_processes: 0,
            health_score: 1.0,
        }
    }
}

// ============================================================================
// ANOMALY DETECTION
// ============================================================================

/// Types of cognitive anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    /// Process taking too long
    SlowProcess,
    /// Low confidence in results
    LowConfidence,
    /// High failure rate
    HighFailureRate,
    /// Memory overuse
    HighMemoryUsage,
    /// CPU overuse
    HighCpuUsage,
    /// Cognitive overload
    CognitiveOverload,
    /// Inconsistent results
    InconsistentResults,
    /// Stuck process
    StuckProcess,
    /// Unexpected error
    UnexpectedError,
    /// Quality degradation
    QualityDegradation,
}

/// A detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly ID
    pub id: u64,
    /// Type of anomaly
    pub anomaly_type: AnomalyType,
    /// Related process (if any)
    pub process_id: Option<CognitiveProcessId>,
    /// Related domain
    pub domain: Option<CognitiveProcessType>,
    /// Severity (0-1)
    pub severity: f64,
    /// Detection timestamp
    pub timestamp: u64,
    /// Description
    pub description: String,
    /// Suggested action
    pub suggested_action: Option<String>,
    /// Is resolved?
    pub resolved: bool,
}

/// Anomaly detector
pub struct AnomalyDetector {
    /// Duration threshold multiplier (vs average)
    pub duration_threshold: f64,
    /// Confidence threshold
    pub confidence_threshold: f64,
    /// Failure rate threshold
    pub failure_rate_threshold: f64,
    /// Memory threshold (bytes)
    pub memory_threshold: u64,
    /// CPU threshold (cycles)
    pub cpu_threshold: u64,
    /// Load threshold
    pub load_threshold: f64,
    /// Detected anomalies
    anomalies: Vec<Anomaly>,
    /// Next anomaly ID
    next_anomaly_id: u64,
}

impl AnomalyDetector {
    /// Create a new detector with default thresholds
    pub fn new() -> Self {
        Self {
            duration_threshold: 3.0, // 3x average is anomalous
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
            failure_rate_threshold: 0.3,
            memory_threshold: 100 * 1024 * 1024, // 100MB
            cpu_threshold: 1_000_000_000,        // 1B cycles
            load_threshold: DEFAULT_LOAD_THRESHOLD,
            anomalies: Vec::new(),
            next_anomaly_id: 0,
        }
    }

    /// Check a process for anomalies
    pub fn check_process(
        &mut self,
        process: &CognitiveProcess,
        domain_metrics: &DomainMetrics,
        timestamp: u64,
    ) -> Vec<Anomaly> {
        let mut detected = Vec::new();

        // Check duration
        if domain_metrics.avg_duration > 0.0 {
            let duration_ratio = process.duration() as f64 / domain_metrics.avg_duration;
            if duration_ratio > self.duration_threshold {
                detected.push(self.create_anomaly(
                    AnomalyType::SlowProcess,
                    Some(process.id),
                    Some(process.process_type),
                    (duration_ratio / self.duration_threshold).min(1.0),
                    timestamp,
                    alloc::format!(
                        "Process {} taking {:.1}x longer than average",
                        process.name,
                        duration_ratio
                    ),
                ));
            }
        }

        // Check confidence
        if process.is_successful() && process.confidence < self.confidence_threshold {
            detected.push(self.create_anomaly(
                AnomalyType::LowConfidence,
                Some(process.id),
                Some(process.process_type),
                1.0 - process.confidence,
                timestamp,
                alloc::format!(
                    "Process {} has low confidence: {:.2}",
                    process.name,
                    process.confidence
                ),
            ));
        }

        // Check memory
        if process.memory_used > self.memory_threshold {
            detected.push(self.create_anomaly(
                AnomalyType::HighMemoryUsage,
                Some(process.id),
                Some(process.process_type),
                (process.memory_used as f64 / self.memory_threshold as f64).min(1.0),
                timestamp,
                alloc::format!(
                    "Process {} using {} bytes of memory",
                    process.name,
                    process.memory_used
                ),
            ));
        }

        // Check CPU
        if process.cpu_cycles > self.cpu_threshold {
            detected.push(self.create_anomaly(
                AnomalyType::HighCpuUsage,
                Some(process.id),
                Some(process.process_type),
                (process.cpu_cycles as f64 / self.cpu_threshold as f64).min(1.0),
                timestamp,
                alloc::format!(
                    "Process {} used {} CPU cycles",
                    process.name,
                    process.cpu_cycles
                ),
            ));
        }

        // Check for unexpected error
        if process.state == ProcessState::Failed {
            if let Some(ref error) = process.error {
                detected.push(self.create_anomaly(
                    AnomalyType::UnexpectedError,
                    Some(process.id),
                    Some(process.process_type),
                    0.8,
                    timestamp,
                    alloc::format!("Process {} failed: {}", process.name, error),
                ));
            }
        }

        // Store anomalies
        for anomaly in &detected {
            self.anomalies.push(anomaly.clone());
        }

        detected
    }

    /// Check domain metrics for anomalies
    pub fn check_domain(&mut self, metrics: &DomainMetrics, timestamp: u64) -> Vec<Anomaly> {
        let mut detected = Vec::new();

        // Check failure rate
        if metrics.success_rate < (1.0 - self.failure_rate_threshold) {
            detected.push(self.create_anomaly(
                AnomalyType::HighFailureRate,
                None,
                Some(metrics.domain),
                1.0 - metrics.success_rate,
                timestamp,
                alloc::format!(
                    "Domain {:?} has high failure rate: {:.1}%",
                    metrics.domain,
                    (1.0 - metrics.success_rate) * 100.0
                ),
            ));
        }

        // Check cognitive load
        if metrics.current_load > self.load_threshold {
            detected.push(self.create_anomaly(
                AnomalyType::CognitiveOverload,
                None,
                Some(metrics.domain),
                metrics.current_load,
                timestamp,
                alloc::format!(
                    "Domain {:?} is overloaded: {:.1}% load",
                    metrics.domain,
                    metrics.current_load * 100.0
                ),
            ));
        }

        // Check quality degradation
        if metrics.avg_quality < 0.5 && metrics.total_count > 10 {
            detected.push(self.create_anomaly(
                AnomalyType::QualityDegradation,
                None,
                Some(metrics.domain),
                1.0 - metrics.avg_quality,
                timestamp,
                alloc::format!(
                    "Domain {:?} showing quality degradation: {:.2}",
                    metrics.domain,
                    metrics.avg_quality
                ),
            ));
        }

        for anomaly in &detected {
            self.anomalies.push(anomaly.clone());
        }

        detected
    }

    /// Create an anomaly
    fn create_anomaly(
        &mut self,
        anomaly_type: AnomalyType,
        process_id: Option<CognitiveProcessId>,
        domain: Option<CognitiveProcessType>,
        severity: f64,
        timestamp: u64,
        description: String,
    ) -> Anomaly {
        let id = self.next_anomaly_id;
        self.next_anomaly_id += 1;

        Anomaly {
            id,
            anomaly_type,
            process_id,
            domain,
            severity,
            timestamp,
            description,
            suggested_action: self.suggest_action(anomaly_type),
            resolved: false,
        }
    }

    /// Suggest an action for an anomaly
    fn suggest_action(&self, anomaly_type: AnomalyType) -> Option<String> {
        let action = match anomaly_type {
            AnomalyType::SlowProcess => "Consider caching results or simplifying computation",
            AnomalyType::LowConfidence => "Gather more data or use ensemble methods",
            AnomalyType::HighFailureRate => "Review error patterns and add error handling",
            AnomalyType::HighMemoryUsage => "Implement memory pooling or streaming",
            AnomalyType::HighCpuUsage => "Optimize algorithm or add early termination",
            AnomalyType::CognitiveOverload => "Reduce concurrent processes or prioritize",
            AnomalyType::InconsistentResults => "Add validation or retry logic",
            AnomalyType::StuckProcess => "Add timeout and cancellation support",
            AnomalyType::UnexpectedError => "Add error handling for this case",
            AnomalyType::QualityDegradation => "Retrain models or recalibrate",
        };
        Some(String::from(action))
    }

    /// Get unresolved anomalies
    pub fn get_unresolved(&self) -> Vec<&Anomaly> {
        self.anomalies.iter().filter(|a| !a.resolved).collect()
    }

    /// Resolve an anomaly
    pub fn resolve(&mut self, id: u64) {
        for anomaly in &mut self.anomalies {
            if anomaly.id == id {
                anomaly.resolved = true;
                break;
            }
        }
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CONFIDENCE CALIBRATION
// ============================================================================

/// Confidence calibration system
pub struct ConfidenceCalibrator {
    /// Calibration bins (predicted confidence -> actual accuracy)
    bins: Vec<CalibrationBin>,
    /// Number of bins
    num_bins: usize,
    /// Total samples
    total_samples: u64,
    /// Expected calibration error (ECE)
    ece: f64,
    /// Maximum calibration error (MCE)
    mce: f64,
}

/// A calibration bin
#[derive(Debug, Clone)]
struct CalibrationBin {
    /// Lower bound of confidence
    lower: f64,
    /// Upper bound of confidence
    upper: f64,
    /// Number of samples in bin
    count: u64,
    /// Sum of confidences
    confidence_sum: f64,
    /// Number of correct predictions
    correct_count: u64,
}

impl CalibrationBin {
    fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower,
            upper,
            count: 0,
            confidence_sum: 0.0,
            correct_count: 0,
        }
    }

    fn avg_confidence(&self) -> f64 {
        if self.count == 0 {
            (self.lower + self.upper) / 2.0
        } else {
            self.confidence_sum / self.count as f64
        }
    }

    fn accuracy(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.correct_count as f64 / self.count as f64
        }
    }

    fn contains(&self, confidence: f64) -> bool {
        confidence >= self.lower && confidence < self.upper
    }
}

impl ConfidenceCalibrator {
    /// Create a new calibrator
    pub fn new(num_bins: usize) -> Self {
        let mut bins = Vec::with_capacity(num_bins);
        let bin_size = 1.0 / num_bins as f64;

        for i in 0..num_bins {
            let lower = i as f64 * bin_size;
            let upper = (i + 1) as f64 * bin_size;
            bins.push(CalibrationBin::new(lower, upper));
        }

        Self {
            bins,
            num_bins,
            total_samples: 0,
            ece: 0.0,
            mce: 0.0,
        }
    }

    /// Add a prediction result
    pub fn add_sample(&mut self, predicted_confidence: f64, was_correct: bool) {
        let confidence = predicted_confidence.clamp(0.0, 0.9999);

        for bin in &mut self.bins {
            if bin.contains(confidence) {
                bin.count += 1;
                bin.confidence_sum += confidence;
                if was_correct {
                    bin.correct_count += 1;
                }
                break;
            }
        }

        self.total_samples += 1;
        self.update_errors();
    }

    /// Update ECE and MCE
    fn update_errors(&mut self) {
        if self.total_samples == 0 {
            return;
        }

        let mut ece = 0.0;
        let mut mce = 0.0;

        for bin in &self.bins {
            if bin.count == 0 {
                continue;
            }

            let weight = bin.count as f64 / self.total_samples as f64;
            let error = (bin.avg_confidence() - bin.accuracy()).abs();

            ece += weight * error;
            if error > mce {
                mce = error;
            }
        }

        self.ece = ece;
        self.mce = mce;
    }

    /// Get expected calibration error
    pub fn expected_calibration_error(&self) -> f64 {
        self.ece
    }

    /// Get maximum calibration error
    pub fn maximum_calibration_error(&self) -> f64 {
        self.mce
    }

    /// Calibrate a confidence value
    pub fn calibrate(&self, raw_confidence: f64) -> f64 {
        let confidence = raw_confidence.clamp(0.0, 0.9999);

        // Find the bin
        for bin in &self.bins {
            if bin.contains(confidence) {
                if bin.count > 0 {
                    // Adjust based on observed accuracy vs confidence
                    let gap = bin.accuracy() - bin.avg_confidence();
                    return (confidence + gap).clamp(0.0, 1.0);
                }
                break;
            }
        }

        raw_confidence
    }

    /// Get calibration reliability diagram data
    pub fn reliability_diagram(&self) -> Vec<(f64, f64, u64)> {
        self.bins
            .iter()
            .map(|bin| (bin.avg_confidence(), bin.accuracy(), bin.count))
            .collect()
    }
}

// ============================================================================
// METACOGNITION MONITOR
// ============================================================================

/// The main metacognition monitor
pub struct MetacognitionMonitor {
    /// Active cognitive processes
    active_processes: BTreeMap<CognitiveProcessId, CognitiveProcess>,
    /// Completed process history
    history: Vec<CognitiveProcess>,
    /// Domain metrics
    domain_metrics: BTreeMap<CognitiveProcessType, DomainMetrics>,
    /// System metrics
    system_metrics: SystemMetrics,
    /// Anomaly detector
    anomaly_detector: AnomalyDetector,
    /// Confidence calibrator
    confidence_calibrator: ConfidenceCalibrator,
    /// Next process ID
    next_process_id: u64,
    /// Current timestamp
    current_time: u64,
    /// History limit
    history_limit: usize,
}

impl MetacognitionMonitor {
    /// Create a new monitor
    pub fn new() -> Self {
        Self {
            active_processes: BTreeMap::new(),
            history: Vec::new(),
            domain_metrics: BTreeMap::new(),
            system_metrics: SystemMetrics::default(),
            anomaly_detector: AnomalyDetector::new(),
            confidence_calibrator: ConfidenceCalibrator::new(10),
            next_process_id: 0,
            current_time: 0,
            history_limit: HISTORY_SIZE,
        }
    }

    /// Set current time
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Start a new cognitive process
    pub fn start_process(
        &mut self,
        process_type: CognitiveProcessType,
        name: String,
    ) -> CognitiveProcessId {
        let id = CognitiveProcessId(self.next_process_id);
        self.next_process_id += 1;

        let mut process = CognitiveProcess::new(id, process_type, name);
        process.state = ProcessState::Running;
        process.start_time = self.current_time;

        self.active_processes.insert(id, process);

        // Update system metrics
        self.system_metrics.active_processes += 1;

        id
    }

    /// Complete a process
    pub fn complete_process(
        &mut self,
        id: CognitiveProcessId,
        confidence: f64,
        quality: f64,
        cpu_cycles: u64,
        memory_used: u64,
    ) {
        if let Some(mut process) = self.active_processes.remove(&id) {
            process.state = ProcessState::Completed;
            process.end_time = Some(self.current_time);
            process.confidence = confidence;
            process.quality = quality;
            process.cpu_cycles = cpu_cycles;
            process.memory_used = memory_used;

            // Update metrics
            self.update_metrics(&process);

            // Check for anomalies
            let domain_metrics = self
                .domain_metrics
                .get(&process.process_type)
                .cloned()
                .unwrap_or_else(|| DomainMetrics::new(process.process_type));

            let _anomalies =
                self.anomaly_detector
                    .check_process(&process, &domain_metrics, self.current_time);

            // Add to history
            self.add_to_history(process);

            // Update system metrics
            self.system_metrics.active_processes =
                self.system_metrics.active_processes.saturating_sub(1);
        }
    }

    /// Fail a process
    pub fn fail_process(&mut self, id: CognitiveProcessId, error: String) {
        if let Some(mut process) = self.active_processes.remove(&id) {
            process.state = ProcessState::Failed;
            process.end_time = Some(self.current_time);
            process.error = Some(error);

            // Update metrics
            self.update_metrics(&process);

            // Check for anomalies
            let domain_metrics = self
                .domain_metrics
                .get(&process.process_type)
                .cloned()
                .unwrap_or_else(|| DomainMetrics::new(process.process_type));

            let _anomalies =
                self.anomaly_detector
                    .check_process(&process, &domain_metrics, self.current_time);

            self.add_to_history(process);
            self.system_metrics.active_processes =
                self.system_metrics.active_processes.saturating_sub(1);
        }
    }

    /// Update a process's metrics during execution
    pub fn update_process(
        &mut self,
        id: CognitiveProcessId,
        input_size: usize,
        output_size: usize,
        metrics: BTreeMap<String, f64>,
    ) {
        if let Some(process) = self.active_processes.get_mut(&id) {
            process.input_size = input_size;
            process.output_size = output_size;
            process.metrics = metrics;
        }
    }

    /// Update metrics with a completed process
    fn update_metrics(&mut self, process: &CognitiveProcess) {
        // Update domain metrics
        let domain_metrics = self
            .domain_metrics
            .entry(process.process_type)
            .or_insert_with(|| DomainMetrics::new(process.process_type));

        domain_metrics.update(process);

        // Update system metrics
        self.system_metrics.total_processes += 1;
        self.system_metrics.total_cycles += process.cpu_cycles;
        self.system_metrics.total_memory += process.memory_used;

        // Update success rate
        let success = if process.is_successful() { 1.0 } else { 0.0 };
        self.system_metrics.success_rate =
            self.system_metrics.success_rate * (1.0 - EMA_ALPHA) + success * EMA_ALPHA;

        // Update confidence
        self.system_metrics.avg_confidence =
            self.system_metrics.avg_confidence * (1.0 - EMA_ALPHA) + process.confidence * EMA_ALPHA;

        // Update cognitive load
        let load = self.calculate_cognitive_load();
        self.system_metrics.cognitive_load = load;

        // Update health score
        self.system_metrics.health_score = self.calculate_health_score();
    }

    /// Add process to history
    fn add_to_history(&mut self, process: CognitiveProcess) {
        self.history.push(process);

        // Trim history if needed
        while self.history.len() > self.history_limit {
            self.history.remove(0);
        }
    }

    /// Calculate current cognitive load
    fn calculate_cognitive_load(&self) -> f64 {
        // Based on active processes and their resource usage
        let active_count = self.active_processes.len() as f64;
        let max_concurrent = MAX_PROCESSES as f64;

        let process_load = active_count / max_concurrent;

        // Could also factor in memory/CPU usage
        process_load.min(1.0)
    }

    /// Calculate overall health score
    fn calculate_health_score(&self) -> f64 {
        let mut score = 1.0;

        // Factor in success rate
        score *= self.system_metrics.success_rate;

        // Factor in confidence
        score *= self.system_metrics.avg_confidence;

        // Factor in load (inverse - high load reduces health)
        score *= 1.0 - (self.system_metrics.cognitive_load * 0.5);

        // Factor in unresolved anomalies
        let unresolved = self.anomaly_detector.get_unresolved().len();
        let anomaly_penalty = (unresolved as f64 * 0.05).min(0.5);
        score *= 1.0 - anomaly_penalty;

        score.clamp(0.0, 1.0)
    }

    /// Get system metrics
    pub fn get_system_metrics(&self) -> &SystemMetrics {
        &self.system_metrics
    }

    /// Get domain metrics
    pub fn get_domain_metrics(&self, domain: CognitiveProcessType) -> Option<&DomainMetrics> {
        self.domain_metrics.get(&domain)
    }

    /// Get all domain metrics
    pub fn get_all_domain_metrics(&self) -> &BTreeMap<CognitiveProcessType, DomainMetrics> {
        &self.domain_metrics
    }

    /// Get active processes
    pub fn get_active_processes(&self) -> Vec<&CognitiveProcess> {
        self.active_processes.values().collect()
    }

    /// Get recent history
    pub fn get_recent_history(&self, limit: usize) -> Vec<&CognitiveProcess> {
        let start = self.history.len().saturating_sub(limit);
        self.history[start..].iter().collect()
    }

    /// Get unresolved anomalies
    pub fn get_anomalies(&self) -> Vec<&Anomaly> {
        self.anomaly_detector.get_unresolved()
    }

    /// Resolve an anomaly
    pub fn resolve_anomaly(&mut self, id: u64) {
        self.anomaly_detector.resolve(id);
    }

    /// Calibrate confidence
    pub fn calibrate_confidence(&self, raw_confidence: f64) -> f64 {
        self.confidence_calibrator.calibrate(raw_confidence)
    }

    /// Add calibration sample
    pub fn add_calibration_sample(&mut self, confidence: f64, was_correct: bool) {
        self.confidence_calibrator
            .add_sample(confidence, was_correct);
    }

    /// Get expected calibration error
    pub fn get_calibration_error(&self) -> f64 {
        self.confidence_calibrator.expected_calibration_error()
    }

    /// Generate a health report
    pub fn generate_report(&self) -> HealthReport {
        HealthReport {
            timestamp: self.current_time,
            health_score: self.system_metrics.health_score,
            cognitive_load: self.system_metrics.cognitive_load,
            success_rate: self.system_metrics.success_rate,
            avg_confidence: self.system_metrics.avg_confidence,
            active_processes: self.system_metrics.active_processes,
            total_processes: self.system_metrics.total_processes,
            unresolved_anomalies: self.anomaly_detector.get_unresolved().len(),
            calibration_error: self.confidence_calibrator.expected_calibration_error(),
            recommendations: self.generate_recommendations(),
        }
    }

    /// Generate recommendations based on current state
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.system_metrics.cognitive_load > 0.8 {
            recommendations.push(String::from(
                "Consider reducing concurrent cognitive processes",
            ));
        }

        if self.system_metrics.success_rate < 0.9 {
            recommendations.push(String::from(
                "Investigate and address causes of process failures",
            ));
        }

        if self.confidence_calibrator.expected_calibration_error() > 0.1 {
            recommendations.push(String::from("Confidence calibration needed"));
        }

        let unresolved = self.anomaly_detector.get_unresolved();
        if unresolved.len() > 5 {
            recommendations.push(String::from("Multiple unresolved anomalies need attention"));
        }

        recommendations
    }
}

impl Default for MetacognitionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Health report
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Timestamp
    pub timestamp: u64,
    /// Overall health score (0-1)
    pub health_score: f64,
    /// Cognitive load (0-1)
    pub cognitive_load: f64,
    /// Success rate (0-1)
    pub success_rate: f64,
    /// Average confidence (0-1)
    pub avg_confidence: f64,
    /// Number of active processes
    pub active_processes: usize,
    /// Total processes executed
    pub total_processes: u64,
    /// Unresolved anomalies
    pub unresolved_anomalies: usize,
    /// Calibration error
    pub calibration_error: f64,
    /// Recommendations
    pub recommendations: Vec<String>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_lifecycle() {
        let mut monitor = MetacognitionMonitor::new();
        monitor.set_time(0);

        // Start a process
        let id = monitor.start_process(CognitiveProcessType::Reasoning, String::from("test"));

        assert_eq!(monitor.get_active_processes().len(), 1);

        // Complete it
        monitor.set_time(100);
        monitor.complete_process(id, 0.9, 0.95, 1000, 1024);

        assert_eq!(monitor.get_active_processes().len(), 0);
        assert_eq!(monitor.get_recent_history(10).len(), 1);
    }

    #[test]
    fn test_metrics_update() {
        let mut monitor = MetacognitionMonitor::new();
        monitor.set_time(0);

        // Run several processes
        for i in 0..10 {
            let id = monitor.start_process(CognitiveProcessType::Decision, String::from("test"));
            monitor.set_time((i + 1) * 100);
            monitor.complete_process(id, 0.9, 0.85, 500, 512);
        }

        let metrics = monitor
            .get_domain_metrics(CognitiveProcessType::Decision)
            .unwrap();
        assert_eq!(metrics.total_count, 10);
        assert_eq!(metrics.success_count, 10);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut detector = AnomalyDetector::new();
        detector.confidence_threshold = 0.7;

        let mut process = CognitiveProcess::new(
            CognitiveProcessId(0),
            CognitiveProcessType::Prediction,
            String::from("test"),
        );
        process.state = ProcessState::Completed;
        process.confidence = 0.3; // Low confidence

        let domain_metrics = DomainMetrics::new(CognitiveProcessType::Prediction);

        let anomalies = detector.check_process(&process, &domain_metrics, 0);
        assert!(!anomalies.is_empty());
        assert_eq!(anomalies[0].anomaly_type, AnomalyType::LowConfidence);
    }

    #[test]
    fn test_confidence_calibration() {
        let mut calibrator = ConfidenceCalibrator::new(10);

        // Add samples where high confidence = correct
        for _ in 0..100 {
            calibrator.add_sample(0.9, true);
        }

        // Add samples where low confidence = incorrect
        for _ in 0..100 {
            calibrator.add_sample(0.1, false);
        }

        // Well-calibrated
        assert!(calibrator.expected_calibration_error() < 0.2);
    }

    #[test]
    fn test_health_score() {
        let mut monitor = MetacognitionMonitor::new();
        monitor.set_time(0);

        // Good processes
        for _ in 0..10 {
            let id = monitor.start_process(CognitiveProcessType::Learning, String::from("good"));
            monitor.complete_process(id, 0.95, 0.9, 100, 100);
        }

        assert!(monitor.get_system_metrics().health_score > 0.8);

        // Some failures
        for _ in 0..5 {
            let id = monitor.start_process(CognitiveProcessType::Learning, String::from("bad"));
            monitor.fail_process(id, String::from("error"));
        }

        // Health should decrease
        assert!(monitor.get_system_metrics().health_score < 0.9);
    }

    #[test]
    fn test_health_report() {
        let mut monitor = MetacognitionMonitor::new();
        monitor.set_time(1000);

        let id = monitor.start_process(CognitiveProcessType::Attention, String::from("test"));
        monitor.complete_process(id, 0.8, 0.85, 200, 256);

        let report = monitor.generate_report();
        assert_eq!(report.timestamp, 1000);
        assert!(report.health_score > 0.0);
        assert_eq!(report.total_processes, 1);
    }
}
