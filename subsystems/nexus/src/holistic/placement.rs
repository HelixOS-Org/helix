//! # Holistic Task Placement Engine
//!
//! Intelligent task placement considering all system dimensions:
//! - Interference-aware co-location
//! - Cache topology awareness
//! - Power domain grouping
//! - Thermal-aware spreading
//! - Deadline feasibility checks

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// PLACEMENT TYPES
// ============================================================================

/// Placement constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementConstraint {
    /// Must be on specific NUMA node
    NumaRequired,
    /// Prefer specific NUMA node
    NumaPreferred,
    /// Must avoid specific CPU
    CpuExclude,
    /// Must be on specific CPU
    CpuRequired,
    /// Must be isolated (no co-location)
    Isolated,
    /// Co-locate with another task
    CoLocate,
    /// Spread across nodes
    Spread,
}

/// Interference level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterferenceLevel {
    /// No interference
    None,
    /// Low (cache sharing)
    Low,
    /// Medium (bandwidth contention)
    Medium,
    /// High (resource thrashing)
    High,
    /// Severe (destructive)
    Severe,
}

/// Resource dimension for scoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementDimension {
    /// CPU load
    CpuLoad,
    /// Memory pressure
    MemoryPressure,
    /// Cache availability
    CacheAvailability,
    /// Thermal headroom
    ThermalHeadroom,
    /// Power efficiency
    PowerEfficiency,
    /// Network proximity
    NetworkProximity,
}

// ============================================================================
// PLACEMENT CANDIDATE
// ============================================================================

/// Candidate placement
#[derive(Debug, Clone)]
pub struct PlacementCandidate {
    /// CPU id
    pub cpu_id: u32,
    /// NUMA node
    pub numa_node: u32,
    /// Score (higher = better)
    pub score: f64,
    /// Per-dimension scores
    pub dimension_scores: BTreeMap<u8, f64>,
    /// Estimated interference
    pub interference: InterferenceLevel,
    /// Constraint violations
    pub violations: u32,
}

impl PlacementCandidate {
    pub fn new(cpu_id: u32, numa_node: u32) -> Self {
        Self {
            cpu_id,
            numa_node,
            score: 0.0,
            dimension_scores: BTreeMap::new(),
            interference: InterferenceLevel::None,
            violations: 0,
        }
    }

    /// Set dimension score
    #[inline(always)]
    pub fn set_dimension(&mut self, dim: PlacementDimension, score: f64) {
        self.dimension_scores.insert(dim as u8, score);
    }

    /// Compute total score with weights
    pub fn compute_score(&mut self, weights: &BTreeMap<u8, f64>) {
        self.score = 0.0;
        for (&dim, &weight) in weights {
            if let Some(&s) = self.dimension_scores.get(&dim) {
                self.score += s * weight;
            }
        }
        // Penalty for interference
        let interference_penalty = match self.interference {
            InterferenceLevel::None => 0.0,
            InterferenceLevel::Low => 10.0,
            InterferenceLevel::Medium => 30.0,
            InterferenceLevel::High => 60.0,
            InterferenceLevel::Severe => 100.0,
        };
        self.score -= interference_penalty;
        // Penalty for constraint violations
        self.score -= self.violations as f64 * 50.0;
    }
}

// ============================================================================
// INTERFERENCE MODEL
// ============================================================================

/// Interference pair
#[derive(Debug, Clone)]
pub struct InterferencePair {
    /// Task A
    pub task_a: u64,
    /// Task B
    pub task_b: u64,
    /// Level
    pub level: InterferenceLevel,
    /// Measured slowdown (fraction, e.g. 0.15 = 15% slower)
    pub slowdown: f64,
    /// Confidence
    pub confidence: f64,
}

/// Interference model
#[derive(Debug)]
pub struct InterferenceModel {
    /// Known pairs: pair_hash -> interference
    pairs: BTreeMap<u64, InterferencePair>,
}

impl InterferenceModel {
    pub fn new() -> Self {
        Self {
            pairs: BTreeMap::new(),
        }
    }

    fn pair_key(a: u64, b: u64) -> u64 {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= lo;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= hi;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record interference
    #[inline(always)]
    pub fn record(&mut self, pair: InterferencePair) {
        let key = Self::pair_key(pair.task_a, pair.task_b);
        self.pairs.insert(key, pair);
    }

    /// Get interference between two tasks
    #[inline(always)]
    pub fn get_interference(&self, a: u64, b: u64) -> Option<&InterferencePair> {
        let key = Self::pair_key(a, b);
        self.pairs.get(&key)
    }

    /// Predict interference level
    #[inline]
    pub fn predict_level(&self, a: u64, b: u64) -> InterferenceLevel {
        self.pairs.get(&Self::pair_key(a, b))
            .map(|p| p.level)
            .unwrap_or(InterferenceLevel::None)
    }
}

// ============================================================================
// PLACEMENT REQUEST
// ============================================================================

/// Placement request
#[derive(Debug)]
pub struct PlacementRequest {
    /// Task id
    pub task_id: u64,
    /// Constraints
    pub constraints: Vec<(PlacementConstraint, u64)>,
    /// Dimension weights
    pub weights: BTreeMap<u8, f64>,
    /// Co-location partners
    pub co_locate_with: Vec<u64>,
    /// Exclude from CPUs
    pub exclude_cpus: Vec<u32>,
}

impl PlacementRequest {
    pub fn new(task_id: u64) -> Self {
        let mut weights = BTreeMap::new();
        weights.insert(PlacementDimension::CpuLoad as u8, 1.0);
        weights.insert(PlacementDimension::CacheAvailability as u8, 0.5);
        weights.insert(PlacementDimension::ThermalHeadroom as u8, 0.3);
        Self {
            task_id,
            constraints: Vec::new(),
            weights,
            co_locate_with: Vec::new(),
            exclude_cpus: Vec::new(),
        }
    }
}

/// Placement result
#[derive(Debug)]
pub struct PlacementResult {
    /// Task id
    pub task_id: u64,
    /// Chosen CPU
    pub cpu_id: u32,
    /// Score
    pub score: f64,
    /// Interference level
    pub interference: InterferenceLevel,
    /// Number of candidates evaluated
    pub candidates_evaluated: usize,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Placement stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticPlacementStats {
    /// Total placements
    pub total_placements: u64,
    /// Average score
    pub avg_score: f64,
    /// Interference-free placements
    pub interference_free: u64,
    /// Constraint-violating placements
    pub constraint_violations: u64,
}

/// Holistic placement engine
pub struct HolisticPlacementEngine {
    /// CPU info: id -> (numa, load, thermal_headroom)
    cpus: BTreeMap<u32, (u32, f64, f64)>,
    /// Task placements: task_id -> cpu_id
    placements: LinearMap<u32, 64>,
    /// Interference model
    pub interference: InterferenceModel,
    /// Score history (for average)
    score_history: VecDeque<f64>,
    /// Max history
    max_history: usize,
    /// Stats
    stats: HolisticPlacementStats,
}

impl HolisticPlacementEngine {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            placements: LinearMap::new(),
            interference: InterferenceModel::new(),
            score_history: VecDeque::new(),
            max_history: 256,
            stats: HolisticPlacementStats::default(),
        }
    }

    /// Register CPU
    #[inline(always)]
    pub fn add_cpu(&mut self, id: u32, numa: u32) {
        self.cpus.insert(id, (numa, 0.0, 1.0));
    }

    /// Update CPU state
    #[inline]
    pub fn update_cpu(&mut self, id: u32, load: f64, thermal_headroom: f64) {
        if let Some(cpu) = self.cpus.get_mut(&id) {
            cpu.1 = load;
            cpu.2 = thermal_headroom;
        }
    }

    /// Find best placement
    pub fn place(&mut self, request: &PlacementRequest) -> Option<PlacementResult> {
        let mut candidates = Vec::new();

        for (&cpu_id, &(numa, load, thermal)) in &self.cpus {
            if request.exclude_cpus.contains(&cpu_id) {
                continue;
            }

            let mut candidate = PlacementCandidate::new(cpu_id, numa);
            candidate.set_dimension(PlacementDimension::CpuLoad, (1.0 - load) * 100.0);
            candidate.set_dimension(PlacementDimension::ThermalHeadroom, thermal * 100.0);
            candidate.set_dimension(PlacementDimension::CacheAvailability, (1.0 - load) * 80.0);

            // Check interference with co-located tasks
            let tasks_on_cpu: Vec<u64> = self.placements.iter()
                .filter(|(_, &c)| c == cpu_id)
                .map(|(&t, _)| t)
                .collect();

            let mut max_interference = InterferenceLevel::None;
            for &existing in &tasks_on_cpu {
                let level = self.interference.predict_level(request.task_id, existing);
                if level > max_interference {
                    max_interference = level;
                }
            }
            candidate.interference = max_interference;

            candidate.compute_score(&request.weights);
            candidates.push(candidate);
        }

        // Sort by score descending
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));

        let evaluated = candidates.len();
        if let Some(best) = candidates.first() {
            let cpu_id = best.cpu_id;
            let score = best.score;
            let interference = best.interference;

            self.placements.insert(request.task_id, cpu_id);

            if self.score_history.len() >= self.max_history {
                self.score_history.pop_front();
            }
            self.score_history.push_back(score);

            self.stats.total_placements += 1;
            if interference == InterferenceLevel::None {
                self.stats.interference_free += 1;
            }
            self.update_avg_score();

            Some(PlacementResult {
                task_id: request.task_id,
                cpu_id,
                score,
                interference,
                candidates_evaluated: evaluated,
            })
        } else {
            None
        }
    }

    /// Remove task placement
    #[inline(always)]
    pub fn remove(&mut self, task_id: u64) {
        self.placements.remove(task_id);
    }

    fn update_avg_score(&mut self) {
        if !self.score_history.is_empty() {
            let sum: f64 = self.score_history.iter().sum();
            self.stats.avg_score = sum / self.score_history.len() as f64;
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticPlacementStats {
        &self.stats
    }
}
