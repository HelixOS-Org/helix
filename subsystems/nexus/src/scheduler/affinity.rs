//! CPU/core affinity prediction.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;

use super::types::TaskFeatures;

// ============================================================================
// AFFINITY PREDICTOR
// ============================================================================

/// CPU/core affinity predictor
#[repr(align(64))]
pub struct AffinityPredictor {
    /// Core utilization history
    core_history: Vec<CoreHistory>,
    /// Task-core affinity scores
    affinity_scores: BTreeMap<u64, Vec<f64>>,
    /// Number of cores
    num_cores: usize,
    /// NUMA topology
    numa_topology: Option<NumaTopology>,
}

/// Core utilization history
#[derive(Debug, Clone)]
struct CoreHistory {
    utilization: VecDeque<f64>,
    temperature: VecDeque<f64>,
    #[allow(dead_code)]
    power_state: Vec<u8>,
}

impl CoreHistory {
    fn new() -> Self {
        Self {
            utilization: VecDeque::new(),
            temperature: VecDeque::new(),
            power_state: Vec::new(),
        }
    }

    fn avg_utilization(&self) -> f64 {
        if self.utilization.is_empty() {
            return 0.0;
        }
        self.utilization.iter().sum::<f64>() / self.utilization.len() as f64
    }
}

/// NUMA topology information
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NumaTopology {
    /// Nodes
    pub nodes: Vec<NumaNode>,
    /// Inter-node latency matrix (ns)
    pub latency_matrix: Vec<Vec<u64>>,
}

/// NUMA node
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: u32,
    /// Cores in this node
    pub cores: Vec<u32>,
    /// Memory size (bytes)
    pub memory_size: u64,
    /// Available memory
    pub memory_available: u64,
}

impl AffinityPredictor {
    /// Create new affinity predictor
    pub fn new(num_cores: usize) -> Self {
        Self {
            core_history: (0..num_cores).map(|_| CoreHistory::new()).collect(),
            affinity_scores: BTreeMap::new(),
            num_cores,
            numa_topology: None,
        }
    }

    /// Set NUMA topology
    #[inline(always)]
    pub fn set_numa_topology(&mut self, topology: NumaTopology) {
        self.numa_topology = Some(topology);
    }

    /// Update core utilization
    #[inline]
    pub fn update_core_utilization(&mut self, core_id: usize, utilization: f64) {
        if core_id < self.core_history.len() {
            let history = &mut self.core_history[core_id];
            history.utilization.push_back(utilization);
            if history.utilization.len() > 100 {
                history.utilization.pop_front();
            }
        }
    }

    /// Update core temperature
    #[inline]
    pub fn update_core_temperature(&mut self, core_id: usize, temp: f64) {
        if core_id < self.core_history.len() {
            let history = &mut self.core_history[core_id];
            history.temperature.push_back(temp);
            if history.temperature.len() > 100 {
                history.temperature.pop_front();
            }
        }
    }

    /// Predict best core for a task
    pub fn predict_best_core(&self, task_hash: u64, features: &TaskFeatures) -> usize {
        let task_scores = self.affinity_scores.get(&task_hash);

        let mut best_core = 0;
        let mut best_score = f64::NEG_INFINITY;

        for core_id in 0..self.num_cores {
            let mut score = 0.0;

            let utilization = self.core_history[core_id].avg_utilization();
            score += (1.0 - utilization) * 30.0;

            if let Some(scores) = task_scores {
                if core_id < scores.len() {
                    score += scores[core_id] * 40.0;
                }
            }

            if let Some(temp) = self.core_history[core_id].temperature.last() {
                score -= (temp / 100.0) * 10.0;
            }

            if let Some(ref topo) = self.numa_topology {
                let node_for_core = topo
                    .nodes
                    .iter()
                    .position(|n| n.cores.contains(&(core_id as u32)));

                if let Some(node_id) = node_for_core {
                    if features.memory_footprint > 1024 * 1024 * 10 {
                        let node = &topo.nodes[node_id];
                        let mem_ratio = node.memory_available as f64 / node.memory_size as f64;
                        score += mem_ratio * 20.0;
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_core = core_id;
            }
        }

        best_core
    }

    /// Record task execution on a core (for learning)
    #[inline]
    pub fn record_execution(&mut self, task_hash: u64, core_id: usize, performance: f64) {
        let scores = self
            .affinity_scores
            .entry(task_hash)
            .or_insert_with(|| vec![0.5; self.num_cores]);

        if core_id < scores.len() {
            scores[core_id] = 0.8 * scores[core_id] + 0.2 * performance;
        }
    }

    /// Get recommended cores (top N)
    pub fn recommend_cores(
        &self,
        task_hash: u64,
        _features: &TaskFeatures,
        n: usize,
    ) -> Vec<usize> {
        let mut scores: Vec<(usize, f64)> = (0..self.num_cores)
            .map(|core_id| {
                let mut score = 0.0;

                let utilization = self.core_history[core_id].avg_utilization();
                score += (1.0 - utilization) * 30.0;

                if let Some(task_scores) = self.affinity_scores.get(&task_hash) {
                    if core_id < task_scores.len() {
                        score += task_scores[core_id] * 40.0;
                    }
                }

                (core_id, score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        scores.into_iter().take(n).map(|(id, _)| id).collect()
    }
}
