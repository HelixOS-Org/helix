//! # Holistic Workload Characterization
//!
//! System-wide workload analysis and classification:
//! - Workload phase detection
//! - Application mix analysis
//! - Workload fingerprinting
//! - Load pattern recognition
//! - Capacity planning support
//! - Workload replay profiling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// WORKLOAD TYPES
// ============================================================================

/// Workload class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkloadClass {
    /// CPU-intensive computation
    CpuBound,
    /// Memory-intensive
    MemoryBound,
    /// I/O-intensive
    IoBound,
    /// Network-intensive
    NetworkBound,
    /// Mixed workload
    Mixed,
    /// Interactive (latency-sensitive)
    Interactive,
    /// Batch processing
    Batch,
    /// Idle
    Idle,
}

/// Workload phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadPhase {
    /// Startup / initialization
    Startup,
    /// Ramp up
    RampUp,
    /// Steady state
    SteadyState,
    /// Burst
    Burst,
    /// Ramp down
    RampDown,
    /// Idle
    Idle,
    /// Shutdown
    Shutdown,
}

/// Load pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPattern {
    /// Constant load
    Constant,
    /// Periodic (diurnal/cyclic)
    Periodic,
    /// Bursty (irregular spikes)
    Bursty,
    /// Trending up
    TrendingUp,
    /// Trending down
    TrendingDown,
    /// Random
    Random,
}

// ============================================================================
// WORKLOAD FINGERPRINT
// ============================================================================

/// Resource utilization snapshot
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    /// CPU utilization (0.0-1.0)
    pub cpu_util: f64,
    /// Memory utilization (0.0-1.0)
    pub memory_util: f64,
    /// I/O utilization (0.0-1.0)
    pub io_util: f64,
    /// Network utilization (0.0-1.0)
    pub network_util: f64,
    /// IPC rate (messages/s)
    pub ipc_rate: u64,
    /// Context switch rate (/s)
    pub context_switch_rate: u64,
    /// Page fault rate (/s)
    pub page_fault_rate: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl ResourceSnapshot {
    pub fn new(timestamp: u64) -> Self {
        Self {
            cpu_util: 0.0,
            memory_util: 0.0,
            io_util: 0.0,
            network_util: 0.0,
            ipc_rate: 0,
            context_switch_rate: 0,
            page_fault_rate: 0,
            timestamp,
        }
    }

    /// Classify workload from snapshot
    pub fn classify(&self) -> WorkloadClass {
        if self.cpu_util < 0.05
            && self.memory_util < 0.1
            && self.io_util < 0.05
            && self.network_util < 0.05
        {
            return WorkloadClass::Idle;
        }

        // Find dominant resource
        let max_util = [self.cpu_util, self.memory_util, self.io_util, self.network_util];
        let max_idx = max_util
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Check for mixed (no single dominant)
        let max_val = max_util[max_idx];
        let second_max = max_util
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != max_idx)
            .map(|(_, v)| *v)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        if max_val > 0.0 && second_max / max_val > 0.7 {
            return WorkloadClass::Mixed;
        }

        // Check for interactive
        if self.context_switch_rate > 10000 && self.cpu_util < 0.5 {
            return WorkloadClass::Interactive;
        }

        match max_idx {
            0 => WorkloadClass::CpuBound,
            1 => WorkloadClass::MemoryBound,
            2 => WorkloadClass::IoBound,
            3 => WorkloadClass::NetworkBound,
            _ => WorkloadClass::Mixed,
        }
    }
}

/// Workload fingerprint
#[derive(Debug, Clone)]
pub struct WorkloadFingerprint {
    /// Fingerprint ID
    pub id: u64,
    /// Process or group
    pub entity_id: u64,
    /// Dominant class
    pub class: WorkloadClass,
    /// Phase
    pub phase: WorkloadPhase,
    /// Average CPU
    pub avg_cpu: f64,
    /// Average memory
    pub avg_memory: f64,
    /// Average I/O
    pub avg_io: f64,
    /// Average network
    pub avg_network: f64,
    /// Typical context switch rate
    pub typical_csw_rate: u64,
    /// Typical IPC rate
    pub typical_ipc_rate: u64,
    /// Burstiness score (0.0 = constant, 1.0 = very bursty)
    pub burstiness: f64,
    /// Duration samples
    pub sample_count: u64,
}

impl WorkloadFingerprint {
    pub fn new(id: u64, entity_id: u64) -> Self {
        Self {
            id,
            entity_id,
            class: WorkloadClass::Idle,
            phase: WorkloadPhase::Startup,
            avg_cpu: 0.0,
            avg_memory: 0.0,
            avg_io: 0.0,
            avg_network: 0.0,
            typical_csw_rate: 0,
            typical_ipc_rate: 0,
            burstiness: 0.0,
            sample_count: 0,
        }
    }

    /// Update from snapshot (running average)
    pub fn update(&mut self, snapshot: &ResourceSnapshot) {
        self.sample_count += 1;
        let n = self.sample_count as f64;
        let alpha = 1.0 / n;

        self.avg_cpu += alpha * (snapshot.cpu_util - self.avg_cpu);
        self.avg_memory += alpha * (snapshot.memory_util - self.avg_memory);
        self.avg_io += alpha * (snapshot.io_util - self.avg_io);
        self.avg_network += alpha * (snapshot.network_util - self.avg_network);

        // EWMA for rates
        let rate_alpha = 0.1;
        self.typical_csw_rate = (rate_alpha * snapshot.context_switch_rate as f64
            + (1.0 - rate_alpha) * self.typical_csw_rate as f64)
            as u64;
        self.typical_ipc_rate = (rate_alpha * snapshot.ipc_rate as f64
            + (1.0 - rate_alpha) * self.typical_ipc_rate as f64)
            as u64;

        self.class = snapshot.classify();
    }

    /// Similarity to another fingerprint (0.0-1.0)
    #[inline]
    pub fn similarity(&self, other: &WorkloadFingerprint) -> f64 {
        let cpu_diff = libm::fabs(self.avg_cpu - other.avg_cpu);
        let mem_diff = libm::fabs(self.avg_memory - other.avg_memory);
        let io_diff = libm::fabs(self.avg_io - other.avg_io);
        let net_diff = libm::fabs(self.avg_network - other.avg_network);

        let avg_diff = (cpu_diff + mem_diff + io_diff + net_diff) / 4.0;
        1.0 - avg_diff
    }
}

// ============================================================================
// PHASE DETECTOR
// ============================================================================

/// Phase detection window
#[derive(Debug, Clone)]
pub struct PhaseDetector {
    /// Window of snapshots
    window: VecDeque<ResourceSnapshot>,
    /// Window size
    window_size: usize,
    /// Current phase
    pub current_phase: WorkloadPhase,
    /// Phase duration (in samples)
    pub phase_duration: u64,
    /// Phase transitions
    pub transitions: u64,
}

impl PhaseDetector {
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::new(),
            window_size,
            current_phase: WorkloadPhase::Startup,
            phase_duration: 0,
            transitions: 0,
        }
    }

    /// Add snapshot and detect phase
    pub fn observe(&mut self, snapshot: ResourceSnapshot) -> WorkloadPhase {
        self.window.push_back(snapshot);
        if self.window.len() > self.window_size {
            self.window.pop_front();
        }

        let new_phase = self.detect();
        if new_phase != self.current_phase {
            self.current_phase = new_phase;
            self.phase_duration = 0;
            self.transitions += 1;
        } else {
            self.phase_duration += 1;
        }

        self.current_phase
    }

    fn detect(&self) -> WorkloadPhase {
        if self.window.len() < 3 {
            return WorkloadPhase::Startup;
        }

        // Compute average utilization trend
        let len = self.window.len();
        let first_half: f64 = self.window[..len / 2]
            .iter()
            .map(|s| s.cpu_util + s.io_util + s.network_util)
            .sum::<f64>()
            / (len / 2) as f64;

        let second_half: f64 = self.window[len / 2..]
            .iter()
            .map(|s| s.cpu_util + s.io_util + s.network_util)
            .sum::<f64>()
            / (len - len / 2) as f64;

        let overall_avg: f64 = self
            .window
            .iter()
            .map(|s| s.cpu_util + s.io_util + s.network_util)
            .sum::<f64>()
            / len as f64;

        // Compute variance for burstiness
        let variance: f64 = self
            .window
            .iter()
            .map(|s| {
                let total = s.cpu_util + s.io_util + s.network_util;
                (total - overall_avg) * (total - overall_avg)
            })
            .sum::<f64>()
            / len as f64;

        let stddev = libm::sqrt(variance);

        if overall_avg < 0.15 {
            WorkloadPhase::Idle
        } else if stddev > overall_avg * 0.5 {
            WorkloadPhase::Burst
        } else if second_half > first_half * 1.3 {
            WorkloadPhase::RampUp
        } else if second_half < first_half * 0.7 {
            WorkloadPhase::RampDown
        } else {
            WorkloadPhase::SteadyState
        }
    }
}

// ============================================================================
// WORKLOAD MIX
// ============================================================================

/// Workload mix analysis
#[derive(Debug, Clone)]
pub struct WorkloadMix {
    /// Class distribution (class → count)
    pub class_distribution: BTreeMap<u8, usize>,
    /// Total processes
    pub total_processes: usize,
    /// Diversity score (0.0 = homogeneous, 1.0 = diverse)
    pub diversity: f64,
}

impl WorkloadMix {
    pub fn new() -> Self {
        Self {
            class_distribution: BTreeMap::new(),
            total_processes: 0,
            diversity: 0.0,
        }
    }

    /// Compute from fingerprints
    pub fn compute(fingerprints: &[WorkloadFingerprint]) -> Self {
        let mut mix = Self::new();
        mix.total_processes = fingerprints.len();

        for fp in fingerprints {
            *mix.class_distribution.entry(fp.class as u8).or_insert(0) += 1;
        }

        // Shannon entropy for diversity
        if mix.total_processes > 0 {
            let mut entropy = 0.0;
            for &count in mix.class_distribution.values() {
                if count > 0 {
                    let p = count as f64 / mix.total_processes as f64;
                    entropy -= p * libm::log(p);
                }
            }
            // Normalize by max entropy (log of number of classes)
            let max_entropy = libm::log(8.0); // 8 workload classes
            mix.diversity = if max_entropy > 0.0 {
                entropy / max_entropy
            } else {
                0.0
            };
        }

        mix
    }

    /// Dominant class
    pub fn dominant_class(&self) -> Option<WorkloadClass> {
        self.class_distribution
            .iter()
            .max_by_key(|(_, &c)| c)
            .and_then(|(&k, _)| match k {
                0 => Some(WorkloadClass::CpuBound),
                1 => Some(WorkloadClass::MemoryBound),
                2 => Some(WorkloadClass::IoBound),
                3 => Some(WorkloadClass::NetworkBound),
                4 => Some(WorkloadClass::Mixed),
                5 => Some(WorkloadClass::Interactive),
                6 => Some(WorkloadClass::Batch),
                7 => Some(WorkloadClass::Idle),
                _ => None,
            })
    }
}

// ============================================================================
// WORKLOAD ANALYZER
// ============================================================================

/// Workload analyzer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticWorkloadStats {
    /// Tracked entities
    pub tracked_entities: usize,
    /// Current system class
    pub system_class: u8,
    /// Current phase
    pub system_phase: u8,
    /// Diversity score
    pub diversity: f64,
    /// Phase transitions
    pub total_transitions: u64,
}

/// System-wide workload analyzer
pub struct HolisticWorkloadAnalyzer {
    /// Per-entity fingerprints
    fingerprints: BTreeMap<u64, WorkloadFingerprint>,
    /// Phase detector (system-wide)
    phase_detector: PhaseDetector,
    /// Load pattern history
    load_history: VecDeque<f64>,
    /// Max history
    max_history: usize,
    /// Next fingerprint ID
    next_fp_id: u64,
    /// Current mix
    current_mix: WorkloadMix,
    /// Stats
    stats: HolisticWorkloadStats,
}

impl HolisticWorkloadAnalyzer {
    pub fn new() -> Self {
        Self {
            fingerprints: BTreeMap::new(),
            phase_detector: PhaseDetector::new(30),
            load_history: VecDeque::new(),
            max_history: 1000,
            next_fp_id: 1,
            current_mix: WorkloadMix::new(),
            stats: HolisticWorkloadStats::default(),
        }
    }

    /// Register entity
    #[inline]
    pub fn register_entity(&mut self, entity_id: u64) -> u64 {
        let fp_id = self.next_fp_id;
        self.next_fp_id += 1;
        self.fingerprints
            .insert(entity_id, WorkloadFingerprint::new(fp_id, entity_id));
        self.stats.tracked_entities = self.fingerprints.len();
        fp_id
    }

    /// Unregister entity
    #[inline(always)]
    pub fn unregister_entity(&mut self, entity_id: u64) {
        self.fingerprints.remove(&entity_id);
        self.stats.tracked_entities = self.fingerprints.len();
    }

    /// Report snapshot for entity
    #[inline]
    pub fn report_entity(&mut self, entity_id: u64, snapshot: &ResourceSnapshot) {
        if let Some(fp) = self.fingerprints.get_mut(&entity_id) {
            fp.update(snapshot);
        }
    }

    /// Report system-wide snapshot
    pub fn report_system(&mut self, snapshot: ResourceSnapshot) {
        let total_load = snapshot.cpu_util + snapshot.io_util + snapshot.network_util;
        self.load_history.push_back(total_load);
        if self.load_history.len() > self.max_history {
            self.load_history.pop_front();
        }

        let phase = self.phase_detector.observe(snapshot);
        self.stats.system_phase = phase as u8;
        self.stats.total_transitions = self.phase_detector.transitions;

        // Recompute mix
        let fps: Vec<WorkloadFingerprint> = self.fingerprints.values().cloned().collect();
        self.current_mix = WorkloadMix::compute(&fps);
        self.stats.diversity = self.current_mix.diversity;

        if let Some(dominant) = self.current_mix.dominant_class() {
            self.stats.system_class = dominant as u8;
        }
    }

    /// Detect load pattern
    pub fn detect_pattern(&self) -> LoadPattern {
        if self.load_history.len() < 10 {
            return LoadPattern::Random;
        }

        let len = self.load_history.len();
        let avg: f64 = self.load_history.iter().sum::<f64>() / len as f64;

        // Variance
        let variance: f64 = self
            .load_history
            .iter()
            .map(|v| (v - avg) * (v - avg))
            .sum::<f64>()
            / len as f64;
        let stddev = libm::sqrt(variance);
        let cv = if avg > 0.0 { stddev / avg } else { 0.0 };

        // Trend
        let first_quarter: f64 =
            self.load_history[..len / 4].iter().sum::<f64>() / (len / 4) as f64;
        let last_quarter: f64 = self.load_history[3 * len / 4..]
            .iter()
            .sum::<f64>()
            / (len - 3 * len / 4) as f64;

        if cv < 0.1 {
            LoadPattern::Constant
        } else if last_quarter > first_quarter * 1.5 {
            LoadPattern::TrendingUp
        } else if last_quarter < first_quarter * 0.5 {
            LoadPattern::TrendingDown
        } else if cv > 0.5 {
            LoadPattern::Bursty
        } else {
            LoadPattern::Periodic
        }
    }

    /// Get fingerprint
    #[inline(always)]
    pub fn fingerprint(&self, entity_id: u64) -> Option<&WorkloadFingerprint> {
        self.fingerprints.get(&entity_id)
    }

    /// Current mix
    #[inline(always)]
    pub fn mix(&self) -> &WorkloadMix {
        &self.current_mix
    }

    /// Find similar entities
    pub fn find_similar(&self, entity_id: u64, min_similarity: f64) -> Vec<(u64, f64)> {
        let Some(target) = self.fingerprints.get(&entity_id) else {
            return Vec::new();
        };

        let mut similar: Vec<(u64, f64)> = self
            .fingerprints
            .iter()
            .filter(|(&id, _)| id != entity_id)
            .map(|(&id, fp)| (id, target.similarity(fp)))
            .filter(|(_, sim)| *sim >= min_similarity)
            .collect();

        similar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        similar
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticWorkloadStats {
        &self.stats
    }
}
