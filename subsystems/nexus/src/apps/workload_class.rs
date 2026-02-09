//! # Apps Workload Class
//!
//! Workload classification engine:
//! - Multi-dimensional feature vector for classification
//! - Workload archetype detection (database, web server, batch, etc.)
//! - Phase detection within workloads
//! - Resource demand prediction per class
//! - Classification confidence scoring
//! - Class transition tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// High-level workload class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    /// Interactive (latency-sensitive)
    Interactive,
    /// Batch processing (throughput-oriented)
    Batch,
    /// Database workload
    Database,
    /// Web server / request-response
    WebServer,
    /// Scientific / HPC
    Scientific,
    /// Media processing (audio/video)
    Media,
    /// Build system / compiler
    Build,
    /// Idle / daemon
    Idle,
    /// Unknown
    Unknown,
}

/// Workload phase within a class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadPhase {
    /// Initialization / startup
    Startup,
    /// Steady state
    Steady,
    /// Ramp up
    RampUp,
    /// Ramp down
    RampDown,
    /// Burst activity
    Burst,
    /// Quiescent
    Quiescent,
}

/// Feature vector for classification
#[derive(Debug, Clone)]
pub struct WorkloadFeatures {
    /// CPU utilization (0.0 - 1.0)
    pub cpu_util: f64,
    /// Memory working set (pages)
    pub memory_wss: u64,
    /// IO operations per second
    pub io_ops_per_sec: f64,
    /// Network bytes per second
    pub net_bytes_per_sec: f64,
    /// Context switch rate
    pub ctx_switch_rate: f64,
    /// Syscall rate
    pub syscall_rate: f64,
    /// Voluntary wait ratio
    pub voluntary_wait_ratio: f64,
    /// Thread count
    pub thread_count: u32,
    /// Cache miss rate
    pub cache_miss_rate: f64,
    /// IPC (instructions per cycle)
    pub ipc: f64,
}

impl WorkloadFeatures {
    pub fn new() -> Self {
        Self {
            cpu_util: 0.0,
            memory_wss: 0,
            io_ops_per_sec: 0.0,
            net_bytes_per_sec: 0.0,
            ctx_switch_rate: 0.0,
            syscall_rate: 0.0,
            voluntary_wait_ratio: 0.0,
            thread_count: 1,
            cache_miss_rate: 0.0,
            ipc: 0.0,
        }
    }

    /// Euclidean distance between two feature vectors (normalized)
    pub fn distance(&self, other: &WorkloadFeatures) -> f64 {
        let d_cpu = self.cpu_util - other.cpu_util;
        let d_io = (self.io_ops_per_sec - other.io_ops_per_sec) / 10000.0;
        let d_net = (self.net_bytes_per_sec - other.net_bytes_per_sec) / 1_000_000.0;
        let d_ctx = (self.ctx_switch_rate - other.ctx_switch_rate) / 10000.0;
        let d_sys = (self.syscall_rate - other.syscall_rate) / 50000.0;
        let d_wait = self.voluntary_wait_ratio - other.voluntary_wait_ratio;
        let d_cache = self.cache_miss_rate - other.cache_miss_rate;
        let d_ipc = (self.ipc - other.ipc) / 4.0;

        let sum = d_cpu * d_cpu + d_io * d_io + d_net * d_net + d_ctx * d_ctx
            + d_sys * d_sys + d_wait * d_wait + d_cache * d_cache + d_ipc * d_ipc;
        libm::sqrt(sum)
    }
}

/// Archetype (centroid for each class)
#[derive(Debug, Clone)]
pub struct WorkloadArchetype {
    pub class: WorkloadClass,
    pub name: String,
    pub centroid: WorkloadFeatures,
    pub match_count: u64,
}

impl WorkloadArchetype {
    pub fn new(class: WorkloadClass, name: String, centroid: WorkloadFeatures) -> Self {
        Self { class, name, centroid, match_count: 0 }
    }
}

/// Classification result
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub class: WorkloadClass,
    pub confidence: f64,
    pub second_best: Option<WorkloadClass>,
    pub second_confidence: f64,
}

/// Per-process classification
#[derive(Debug)]
pub struct ProcessClassification {
    pub pid: u64,
    pub current_class: WorkloadClass,
    pub current_phase: WorkloadPhase,
    pub confidence: f64,
    pub features: WorkloadFeatures,
    /// Class history (ring buffer, last 32)
    class_history: Vec<WorkloadClass>,
    history_head: usize,
    pub class_transitions: u64,
    /// Feature EMA
    feature_ema: WorkloadFeatures,
    pub sample_count: u64,
}

impl ProcessClassification {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            current_class: WorkloadClass::Unknown,
            current_phase: WorkloadPhase::Startup,
            confidence: 0.0,
            features: WorkloadFeatures::new(),
            class_history: Vec::new(),
            history_head: 0,
            class_transitions: 0,
            feature_ema: WorkloadFeatures::new(),
            sample_count: 0,
        }
    }

    /// Update features with EMA
    pub fn update_features(&mut self, new_features: &WorkloadFeatures) {
        let alpha = 0.1;
        self.feature_ema.cpu_util = (1.0 - alpha) * self.feature_ema.cpu_util
            + alpha * new_features.cpu_util;
        self.feature_ema.io_ops_per_sec = (1.0 - alpha) * self.feature_ema.io_ops_per_sec
            + alpha * new_features.io_ops_per_sec;
        self.feature_ema.net_bytes_per_sec = (1.0 - alpha) * self.feature_ema.net_bytes_per_sec
            + alpha * new_features.net_bytes_per_sec;
        self.feature_ema.ctx_switch_rate = (1.0 - alpha) * self.feature_ema.ctx_switch_rate
            + alpha * new_features.ctx_switch_rate;
        self.feature_ema.syscall_rate = (1.0 - alpha) * self.feature_ema.syscall_rate
            + alpha * new_features.syscall_rate;
        self.feature_ema.voluntary_wait_ratio = (1.0 - alpha) * self.feature_ema.voluntary_wait_ratio
            + alpha * new_features.voluntary_wait_ratio;
        self.feature_ema.cache_miss_rate = (1.0 - alpha) * self.feature_ema.cache_miss_rate
            + alpha * new_features.cache_miss_rate;
        self.feature_ema.ipc = (1.0 - alpha) * self.feature_ema.ipc + alpha * new_features.ipc;
        self.feature_ema.memory_wss = new_features.memory_wss;
        self.feature_ema.thread_count = new_features.thread_count;
        self.features = new_features.clone();
        self.sample_count += 1;
    }

    /// Set classification
    pub fn set_class(&mut self, class: WorkloadClass, confidence: f64) {
        if class != self.current_class {
            self.class_transitions += 1;
        }
        self.current_class = class;
        self.confidence = confidence;

        if self.class_history.len() < 32 {
            self.class_history.push(class);
        } else {
            self.class_history[self.history_head] = class;
            self.history_head = (self.history_head + 1) % 32;
        }
    }

    /// Detect phase
    pub fn detect_phase(&mut self) {
        if self.sample_count < 10 {
            self.current_phase = WorkloadPhase::Startup;
            return;
        }

        let cpu = self.feature_ema.cpu_util;
        let io = self.feature_ema.io_ops_per_sec;

        // Simple phase detection from feature trends
        if cpu < 0.05 && io < 10.0 {
            self.current_phase = WorkloadPhase::Quiescent;
        } else if cpu > 0.8 || io > 5000.0 {
            self.current_phase = WorkloadPhase::Burst;
        } else {
            self.current_phase = WorkloadPhase::Steady;
        }
    }

    /// Stability: how often the class stays the same
    #[inline]
    pub fn class_stability(&self) -> f64 {
        if self.class_history.len() < 2 {
            return 1.0;
        }
        let same_count = self.class_history.windows(2)
            .filter(|w| w[0] == w[1])
            .count();
        same_count as f64 / (self.class_history.len() - 1) as f64
    }
}

/// Global workload classifier stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppWorkloadClassStats {
    pub tracked_processes: usize,
    pub class_distribution: BTreeMap<u8, usize>,
    pub avg_confidence: f64,
    pub unstable_count: usize,
}

/// App Workload Classifier
pub struct AppWorkloadClassifier {
    processes: BTreeMap<u64, ProcessClassification>,
    archetypes: Vec<WorkloadArchetype>,
    stats: AppWorkloadClassStats,
}

impl AppWorkloadClassifier {
    pub fn new() -> Self {
        let mut classifier = Self {
            processes: BTreeMap::new(),
            archetypes: Vec::new(),
            stats: AppWorkloadClassStats::default(),
        };
        classifier.init_archetypes();
        classifier
    }

    fn init_archetypes(&mut self) {
        // Interactive: high ctx switch, low CPU, high voluntary wait
        let mut interactive = WorkloadFeatures::new();
        interactive.cpu_util = 0.15;
        interactive.ctx_switch_rate = 5000.0;
        interactive.voluntary_wait_ratio = 0.8;
        interactive.ipc = 1.5;
        self.archetypes.push(WorkloadArchetype::new(
            WorkloadClass::Interactive,
            String::from("interactive"),
            interactive,
        ));

        // Batch: high CPU, low ctx switch, low wait
        let mut batch = WorkloadFeatures::new();
        batch.cpu_util = 0.95;
        batch.ctx_switch_rate = 100.0;
        batch.voluntary_wait_ratio = 0.1;
        batch.ipc = 2.5;
        self.archetypes.push(WorkloadArchetype::new(
            WorkloadClass::Batch,
            String::from("batch"),
            batch,
        ));

        // Database: high IO, moderate CPU, high syscall rate
        let mut db = WorkloadFeatures::new();
        db.cpu_util = 0.5;
        db.io_ops_per_sec = 10000.0;
        db.syscall_rate = 20000.0;
        db.voluntary_wait_ratio = 0.5;
        db.cache_miss_rate = 0.08;
        self.archetypes.push(WorkloadArchetype::new(
            WorkloadClass::Database,
            String::from("database"),
            db,
        ));

        // Web server: high network, moderate CPU, many threads
        let mut web = WorkloadFeatures::new();
        web.cpu_util = 0.4;
        web.net_bytes_per_sec = 500_000.0;
        web.ctx_switch_rate = 3000.0;
        web.syscall_rate = 15000.0;
        web.thread_count = 32;
        self.archetypes.push(WorkloadArchetype::new(
            WorkloadClass::WebServer,
            String::from("web_server"),
            web,
        ));

        // Scientific: very high CPU, high IPC, low IO
        let mut sci = WorkloadFeatures::new();
        sci.cpu_util = 0.99;
        sci.ipc = 3.0;
        sci.cache_miss_rate = 0.02;
        sci.io_ops_per_sec = 10.0;
        self.archetypes.push(WorkloadArchetype::new(
            WorkloadClass::Scientific,
            String::from("scientific"),
            sci,
        ));
    }

    /// Classify a process
    pub fn classify(&mut self, pid: u64, features: &WorkloadFeatures) -> ClassificationResult {
        let proc = self.processes.entry(pid)
            .or_insert_with(|| ProcessClassification::new(pid));
        proc.update_features(features);

        // Find nearest archetype
        let mut distances: Vec<(WorkloadClass, f64)> = self.archetypes.iter()
            .map(|a| (a.class, proc.feature_ema.distance(&a.centroid)))
            .collect();
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        let (best_class, best_dist) = distances.first()
            .copied()
            .unwrap_or((WorkloadClass::Unknown, f64::MAX));
        let (second_class, second_dist) = distances.get(1)
            .copied()
            .unwrap_or((WorkloadClass::Unknown, f64::MAX));

        // Confidence: inverse of distance, normalized
        let confidence = if best_dist < 0.01 {
            1.0
        } else {
            (1.0 / (1.0 + best_dist)).min(1.0)
        };
        let second_confidence = if second_dist < 0.01 {
            1.0
        } else {
            (1.0 / (1.0 + second_dist)).min(1.0)
        };

        proc.set_class(best_class, confidence);
        proc.detect_phase();
        self.update_stats();

        ClassificationResult {
            class: best_class,
            confidence,
            second_best: Some(second_class),
            second_confidence,
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.class_distribution.clear();
        for proc in self.processes.values() {
            *self.stats.class_distribution.entry(proc.current_class as u8).or_insert(0) += 1;
        }
        if !self.processes.is_empty() {
            self.stats.avg_confidence = self.processes.values()
                .map(|p| p.confidence)
                .sum::<f64>() / self.processes.len() as f64;
        }
        self.stats.unstable_count = self.processes.values()
            .filter(|p| p.class_stability() < 0.5)
            .count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppWorkloadClassStats {
        &self.stats
    }

    /// Get process classification
    #[inline(always)]
    pub fn get_classification(&self, pid: u64) -> Option<&ProcessClassification> {
        self.processes.get(&pid)
    }
}
