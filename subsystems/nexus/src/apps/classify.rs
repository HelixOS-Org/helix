//! # Automatic Application Classification Engine
//!
//! Classifies applications into workload categories based on behavioral
//! fingerprints, without requiring any metadata or hints from the app.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// WORKLOAD CATEGORIES
// ============================================================================

/// Primary workload category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadCategory {
    /// CPU-bound: high CPU, low I/O
    CpuBound,
    /// I/O-bound: high I/O, moderate CPU
    IoBound,
    /// Network-bound: high network, varied CPU
    NetworkBound,
    /// Memory-bound: high memory pressure, cache-miss heavy
    MemoryBound,
    /// Interactive: irregular, latency-sensitive
    Interactive,
    /// Batch: steady, throughput-oriented
    Batch,
    /// Real-time: strict timing, periodic
    RealTime,
    /// Microservice: short bursts, high connection rate
    Microservice,
    /// Database: mixed I/O + CPU, lock-heavy
    Database,
    /// Streaming: sequential I/O, steady throughput
    Streaming,
    /// Unknown
    Unknown,
}

impl WorkloadCategory {
    /// Recommended scheduler class for this workload
    pub fn recommended_scheduler_class(&self) -> &'static str {
        match self {
            Self::CpuBound => "SCHED_NORMAL",
            Self::IoBound => "SCHED_NORMAL (io_boost)",
            Self::NetworkBound => "SCHED_NORMAL (net_boost)",
            Self::MemoryBound => "SCHED_NORMAL (numa_aware)",
            Self::Interactive => "SCHED_INTERACTIVE",
            Self::Batch => "SCHED_BATCH",
            Self::RealTime => "SCHED_FIFO",
            Self::Microservice => "SCHED_NORMAL (low_latency)",
            Self::Database => "SCHED_NORMAL (io_priority)",
            Self::Streaming => "SCHED_NORMAL (throughput)",
            Self::Unknown => "SCHED_NORMAL",
        }
    }
}

// ============================================================================
// FINGERPRINT
// ============================================================================

/// Behavioral fingerprint — numerical features extracted from observations
#[derive(Debug, Clone)]
pub struct AppFingerprint {
    // CPU features
    pub cpu_usage_avg: f64,
    pub cpu_usage_peak: f64,
    pub cpu_burst_frequency: f64,
    pub ipc: f64,
    pub cache_miss_rate: f64,

    // I/O features
    pub io_ratio: f64,
    pub io_read_write_ratio: f64,
    pub io_avg_size: f64,
    pub io_sequential_ratio: f64,
    pub io_throughput_mbps: f64,

    // Network features
    pub network_ratio: f64,
    pub connection_rate: f64,
    pub avg_message_size: f64,
    pub is_server_pattern: bool,

    // Memory features
    pub memory_ratio: f64,
    pub memory_growth_rate: f64,
    pub page_fault_rate: f64,
    pub working_set_mb: f64,

    // Timing features
    pub syscall_rate: f64,
    pub avg_inter_syscall_ns: f64,
    pub inter_syscall_cv: f64, // coefficient of variation
    pub periodicity_score: f64,
}

impl AppFingerprint {
    pub fn new() -> Self {
        Self {
            cpu_usage_avg: 0.0,
            cpu_usage_peak: 0.0,
            cpu_burst_frequency: 0.0,
            ipc: 0.0,
            cache_miss_rate: 0.0,
            io_ratio: 0.0,
            io_read_write_ratio: 1.0,
            io_avg_size: 0.0,
            io_sequential_ratio: 0.0,
            io_throughput_mbps: 0.0,
            network_ratio: 0.0,
            connection_rate: 0.0,
            avg_message_size: 0.0,
            is_server_pattern: false,
            memory_ratio: 0.0,
            memory_growth_rate: 0.0,
            page_fault_rate: 0.0,
            working_set_mb: 0.0,
            syscall_rate: 0.0,
            avg_inter_syscall_ns: 0.0,
            inter_syscall_cv: 0.0,
            periodicity_score: 0.0,
        }
    }

    /// Compute feature vector for distance calculations
    pub fn feature_vector(&self) -> [f64; 12] {
        [
            self.cpu_usage_avg,
            self.io_ratio,
            self.network_ratio,
            self.memory_ratio,
            self.io_sequential_ratio,
            self.cache_miss_rate,
            self.syscall_rate / 10000.0, // normalize
            self.inter_syscall_cv.min(1.0),
            self.periodicity_score,
            if self.is_server_pattern { 1.0 } else { 0.0 },
            self.connection_rate / 1000.0, // normalize
            self.memory_growth_rate.min(1.0),
        ]
    }
}

/// Behavioral signature — template for each known workload type
#[derive(Debug, Clone)]
pub struct BehaviorSignature {
    /// The category this signature represents
    pub category: WorkloadCategory,
    /// Feature centroid
    pub centroid: [f64; 12],
    /// Feature weight (importance of each feature for this category)
    pub weights: [f64; 12],
    /// Tolerance (how far from centroid is still a match)
    pub tolerance: f64,
}

// ============================================================================
// CLASSIFICATION RESULT
// ============================================================================

/// Result of classification
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Primary classification
    pub primary: WorkloadCategory,
    /// Confidence in primary classification (0.0 - 1.0)
    pub confidence: f64,
    /// Secondary classification (if mixed)
    pub secondary: Option<WorkloadCategory>,
    /// Confidence in secondary
    pub secondary_confidence: f64,
    /// Distance scores for all categories
    pub scores: Vec<(WorkloadCategory, f64)>,
    /// Explanation
    pub explanation: String,
}

// ============================================================================
// CLASSIFIER
// ============================================================================

/// The classification engine — matches fingerprints against known signatures
pub struct Classifier {
    /// Known behavioral signatures
    signatures: Vec<BehaviorSignature>,
    /// Number of classifications performed
    total_classifications: u64,
    /// Classification accuracy tracking (when ground truth is known)
    correct_classifications: u64,
}

impl Classifier {
    /// Create a new classifier with default signatures
    pub fn new() -> Self {
        let signatures = Self::default_signatures();
        Self {
            signatures,
            total_classifications: 0,
            correct_classifications: 0,
        }
    }

    /// Classify an application from its fingerprint
    pub fn classify(&mut self, fingerprint: &AppFingerprint) -> ClassificationResult {
        let features = fingerprint.feature_vector();
        let mut scores: Vec<(WorkloadCategory, f64)> = Vec::new();

        for sig in &self.signatures {
            let distance = self.weighted_distance(&features, &sig.centroid, &sig.weights);
            let similarity = 1.0 / (1.0 + distance);
            scores.push((sig.category, similarity));
        }

        // Sort by similarity (descending)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        let primary = scores
            .first()
            .map(|(c, _)| *c)
            .unwrap_or(WorkloadCategory::Unknown);
        let primary_conf = scores.first().map(|(_, s)| *s).unwrap_or(0.0);

        let secondary = if scores.len() > 1 && scores[1].1 > 0.3 {
            Some(scores[1].0)
        } else {
            None
        };
        let secondary_conf = if scores.len() > 1 { scores[1].1 } else { 0.0 };

        let explanation = self.generate_explanation(fingerprint, primary);

        self.total_classifications += 1;

        ClassificationResult {
            primary,
            confidence: primary_conf,
            secondary,
            secondary_confidence: secondary_conf,
            scores,
            explanation,
        }
    }

    /// Report ground truth for accuracy tracking
    #[inline]
    pub fn report_ground_truth(&mut self, predicted: WorkloadCategory, actual: WorkloadCategory) {
        if predicted == actual {
            self.correct_classifications += 1;
        }
    }

    /// Current accuracy
    #[inline]
    pub fn accuracy(&self) -> f64 {
        if self.total_classifications == 0 {
            return 0.0;
        }
        self.correct_classifications as f64 / self.total_classifications as f64
    }

    /// Weighted Euclidean distance
    fn weighted_distance(&self, a: &[f64; 12], b: &[f64; 12], weights: &[f64; 12]) -> f64 {
        let mut sum = 0.0;
        for i in 0..12 {
            let diff = a[i] - b[i];
            sum += weights[i] * diff * diff;
        }
        libm::sqrt(sum)
    }

    /// Generate human-readable explanation
    fn generate_explanation(&self, fp: &AppFingerprint, category: WorkloadCategory) -> String {
        match category {
            WorkloadCategory::CpuBound => {
                alloc::format!(
                    "CPU-bound: avg CPU {:.0}%, low I/O ratio {:.1}%, IPC {:.2}",
                    fp.cpu_usage_avg * 100.0,
                    fp.io_ratio * 100.0,
                    fp.ipc
                )
            },
            WorkloadCategory::IoBound => {
                alloc::format!(
                    "I/O-bound: I/O ratio {:.0}%, avg size {:.0}B, {:.0} MB/s",
                    fp.io_ratio * 100.0,
                    fp.io_avg_size,
                    fp.io_throughput_mbps
                )
            },
            WorkloadCategory::NetworkBound => {
                alloc::format!(
                    "Network-bound: net ratio {:.0}%, {:.0} conn/s, server={}",
                    fp.network_ratio * 100.0,
                    fp.connection_rate,
                    fp.is_server_pattern
                )
            },
            WorkloadCategory::Interactive => {
                alloc::format!(
                    "Interactive: irregular timing (CV={:.2}), low periodicity {:.2}",
                    fp.inter_syscall_cv,
                    fp.periodicity_score
                )
            },
            WorkloadCategory::RealTime => {
                alloc::format!(
                    "Real-time: high periodicity {:.2}, low CV {:.2}",
                    fp.periodicity_score,
                    fp.inter_syscall_cv
                )
            },
            _ => alloc::format!("Classified as {:?}", category),
        }
    }

    /// Default behavioral signatures for known workload types
    fn default_signatures() -> Vec<BehaviorSignature> {
        alloc::vec![
            BehaviorSignature {
                category: WorkloadCategory::CpuBound,
                //       cpu   io   net  mem  seq  cache sys   cv   period server conn  growth
                centroid: [
                    0.8, 0.05, 0.02, 0.05, 0.5, 0.1, 0.01, 0.3, 0.2, 0.0, 0.0, 0.0
                ],
                weights: [3.0, 2.0, 1.0, 1.0, 0.5, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
                tolerance: 0.3,
            },
            BehaviorSignature {
                category: WorkloadCategory::IoBound,
                centroid: [0.2, 0.6, 0.05, 0.1, 0.7, 0.05, 0.5, 0.4, 0.3, 0.0, 0.0, 0.0],
                weights: [1.0, 3.0, 1.0, 1.0, 1.5, 0.5, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5],
                tolerance: 0.3,
            },
            BehaviorSignature {
                category: WorkloadCategory::NetworkBound,
                centroid: [0.3, 0.1, 0.5, 0.05, 0.3, 0.05, 0.3, 0.5, 0.3, 0.5, 0.3, 0.0],
                weights: [1.0, 1.0, 3.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 2.0, 1.5, 0.5],
                tolerance: 0.3,
            },
            BehaviorSignature {
                category: WorkloadCategory::MemoryBound,
                centroid: [0.5, 0.1, 0.05, 0.4, 0.3, 0.3, 0.2, 0.3, 0.2, 0.0, 0.0, 0.3],
                weights: [1.0, 1.0, 0.5, 3.0, 0.5, 2.0, 0.5, 0.5, 0.5, 0.5, 0.5, 1.5],
                tolerance: 0.3,
            },
            BehaviorSignature {
                category: WorkloadCategory::Interactive,
                centroid: [0.15, 0.2, 0.1, 0.1, 0.3, 0.1, 0.05, 0.8, 0.1, 0.0, 0.0, 0.0],
                weights: [1.0, 1.0, 0.5, 0.5, 0.5, 0.5, 1.0, 3.0, 2.0, 0.5, 0.5, 0.5],
                tolerance: 0.35,
            },
            BehaviorSignature {
                category: WorkloadCategory::Batch,
                centroid: [0.6, 0.3, 0.05, 0.1, 0.6, 0.1, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0],
                weights: [1.5, 1.5, 0.5, 0.5, 1.0, 0.5, 1.0, 2.0, 1.0, 0.5, 0.5, 0.5],
                tolerance: 0.35,
            },
            BehaviorSignature {
                category: WorkloadCategory::RealTime,
                centroid: [
                    0.3, 0.1, 0.05, 0.05, 0.3, 0.05, 0.2, 0.1, 0.9, 0.0, 0.0, 0.0
                ],
                weights: [1.0, 0.5, 0.5, 0.5, 0.5, 0.5, 1.0, 2.0, 3.0, 0.5, 0.5, 0.5],
                tolerance: 0.3,
            },
            BehaviorSignature {
                category: WorkloadCategory::Microservice,
                centroid: [
                    0.2, 0.15, 0.4, 0.05, 0.3, 0.05, 0.4, 0.6, 0.3, 1.0, 0.5, 0.0
                ],
                weights: [0.5, 0.5, 2.0, 0.5, 0.5, 0.5, 1.0, 1.0, 0.5, 2.5, 2.0, 0.5],
                tolerance: 0.35,
            },
            BehaviorSignature {
                category: WorkloadCategory::Database,
                centroid: [0.4, 0.4, 0.1, 0.2, 0.4, 0.15, 0.3, 0.4, 0.2, 0.0, 0.1, 0.1],
                weights: [1.0, 2.0, 0.5, 1.5, 1.0, 1.5, 1.0, 0.5, 0.5, 0.5, 0.5, 0.5],
                tolerance: 0.35,
            },
            BehaviorSignature {
                category: WorkloadCategory::Streaming,
                centroid: [0.2, 0.5, 0.2, 0.1, 0.9, 0.05, 0.2, 0.2, 0.7, 0.0, 0.0, 0.0],
                weights: [0.5, 2.0, 1.0, 0.5, 3.0, 0.5, 0.5, 1.0, 2.0, 0.5, 0.5, 0.5],
                tolerance: 0.3,
            },
        ]
    }
}
