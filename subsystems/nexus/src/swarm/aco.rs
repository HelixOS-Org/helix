//! # Ant Colony Optimization (ACO)
//!
//! ACO algorithm for combinatorial optimization.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::types::{AcoConfig, Ant, PheromoneMatrix};

// ============================================================================
// GRAPH REPRESENTATION
// ============================================================================

/// Distance/cost matrix for graph problems
#[derive(Debug, Clone)]
pub struct DistanceMatrix {
    /// Number of nodes
    pub n_nodes: usize,
    /// Distances (flattened n×n)
    distances: Vec<f64>,
}

impl DistanceMatrix {
    /// Create new distance matrix
    pub fn new(n_nodes: usize) -> Self {
        Self {
            n_nodes,
            distances: alloc::vec![f64::MAX; n_nodes * n_nodes],
        }
    }

    /// Set distance between nodes
    #[inline]
    pub fn set(&mut self, from: usize, to: usize, dist: f64) {
        if from < self.n_nodes && to < self.n_nodes {
            self.distances[from * self.n_nodes + to] = dist;
        }
    }

    /// Set symmetric distance
    #[inline(always)]
    pub fn set_symmetric(&mut self, a: usize, b: usize, dist: f64) {
        self.set(a, b, dist);
        self.set(b, a, dist);
    }

    /// Get distance between nodes
    #[inline]
    pub fn get(&self, from: usize, to: usize) -> f64 {
        if from < self.n_nodes && to < self.n_nodes {
            self.distances[from * self.n_nodes + to]
        } else {
            f64::MAX
        }
    }

    /// Create from edge list
    #[inline]
    pub fn from_edges(n_nodes: usize, edges: &[(usize, usize, f64)]) -> Self {
        let mut dm = Self::new(n_nodes);
        for &(a, b, d) in edges {
            dm.set_symmetric(a, b, d);
        }
        // Self-loops have zero distance
        for i in 0..n_nodes {
            dm.set(i, i, 0.0);
        }
        dm
    }
}

// ============================================================================
// ACO OPTIMIZER
// ============================================================================

/// Ant Colony Optimizer
pub struct AcoOptimizer {
    /// Configuration
    pub config: AcoConfig,
    /// Distance matrix
    pub distances: DistanceMatrix,
    /// Pheromone matrix
    pub pheromones: PheromoneMatrix,
    /// Ants
    ants: Vec<Ant>,
    /// Best path found
    pub best_path: Vec<usize>,
    /// Best path cost
    pub best_cost: f64,
    /// RNG state
    rng_state: u64,
}

impl AcoOptimizer {
    /// Create new ACO optimizer
    pub fn new(distances: DistanceMatrix, config: AcoConfig) -> Self {
        let n = distances.n_nodes;
        let pheromones = PheromoneMatrix::new(n, config.initial_pheromone);
        let ants = (0..config.n_ants).map(|i| Ant::new(i % n)).collect();

        Self {
            config,
            distances,
            pheromones,
            ants,
            best_path: Vec::new(),
            best_cost: f64::MAX,
            rng_state: 42,
        }
    }

    /// Random float [0, 1)
    fn rand(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }

    /// Heuristic value (inverse distance)
    fn heuristic(&self, from: usize, to: usize) -> f64 {
        let d = self.distances.get(from, to);
        if d < 1e-10 || d == f64::MAX {
            0.0
        } else {
            1.0 / d
        }
    }

    /// Choose next node for ant
    fn choose_next(&mut self, ant: &Ant) -> Option<usize> {
        let current = *ant.path.last()?;
        let n = self.distances.n_nodes;

        // Calculate probabilities for unvisited nodes
        let mut probs = Vec::with_capacity(n);
        let mut total = 0.0;

        for next in 0..n {
            if ant.is_visited(next) {
                probs.push(0.0);
                continue;
            }

            let tau = self.pheromones.get(current, next);
            let eta = self.heuristic(current, next);

            let prob = libm::pow(tau, self.config.alpha) * libm::pow(eta, self.config.beta);
            probs.push(prob);
            total += prob;
        }

        if total < 1e-10 {
            return None;
        }

        // Roulette wheel selection
        let r = self.rand() * total;
        let mut cumsum = 0.0;

        for (node, &prob) in probs.iter().enumerate() {
            cumsum += prob;
            if r <= cumsum {
                return Some(node);
            }
        }

        // Fallback: find first unvisited
        probs
            .iter()
            .enumerate()
            .find(|&(_, p)| *p > 0.0)
            .map(|(i, _)| i)
    }

    /// Build path for ant
    fn build_path(&mut self, ant_idx: usize) {
        let n = self.distances.n_nodes;

        loop {
            if self.ants[ant_idx].path_len() >= n {
                break;
            }
            // Clone ant data to avoid borrowing self twice
            let ant_clone = self.ants[ant_idx].clone();
            if let Some(next) = self.choose_next(&ant_clone) {
                let current = *self.ants[ant_idx].path.last().unwrap();
                let cost = self.distances.get(current, next);
                self.ants[ant_idx].visit(next, cost);
            } else {
                break;
            }
        }

        // Return to start for TSP
        if self.ants[ant_idx].path_len() == n {
            let start = self.ants[ant_idx].path[0];
            let last = *self.ants[ant_idx].path.last().unwrap();
            let return_cost = self.distances.get(last, start);
            if return_cost < f64::MAX {
                self.ants[ant_idx].path_cost += return_cost;
            }
        }
    }

    /// Update pheromones
    fn update_pheromones(&mut self) {
        // Evaporation
        self.pheromones.evaporate(self.config.evaporation_rate);

        // Deposit
        for ant in &self.ants {
            if ant.path_cost < f64::MAX && ant.path_cost > 0.0 {
                let deposit = 1.0 / ant.path_cost;

                for i in 0..ant.path.len() {
                    let from = ant.path[i];
                    let to = ant.path[(i + 1) % ant.path.len()];
                    self.pheromones.deposit(from, to, deposit);
                }
            }
        }
    }

    /// Run one iteration
    pub fn iterate(&mut self) {
        let n = self.distances.n_nodes;

        // Reset and build paths for all ants
        for i in 0..self.ants.len() {
            let start = i % n;
            self.ants[i].reset(start);
        }

        for i in 0..self.ants.len() {
            self.build_path(i);
        }

        // Update best
        for ant in &self.ants {
            if ant.path_cost < self.best_cost {
                self.best_cost = ant.path_cost;
                self.best_path = ant.path.clone();
            }
        }

        // Update pheromones
        self.update_pheromones();
    }

    /// Run full optimization
    pub fn optimize(&mut self) -> AcoResult {
        let mut history = Vec::with_capacity(self.config.max_iterations);

        for _ in 0..self.config.max_iterations {
            self.iterate();
            history.push(self.best_cost);
        }

        AcoResult {
            best_path: self.best_path.clone(),
            best_cost: self.best_cost,
            history,
        }
    }
}

/// ACO result
#[derive(Debug, Clone)]
pub struct AcoResult {
    /// Best path found
    pub best_path: Vec<usize>,
    /// Best path cost
    pub best_cost: f64,
    /// Cost history
    pub history: Vec<f64>,
}

// ============================================================================
// SPECIALIZED ACO VARIANTS
// ============================================================================

/// Max-Min Ant System (MMAS)
pub struct MaxMinAntSystem {
    /// Base ACO
    inner: AcoOptimizer,
    /// Minimum pheromone
    tau_min: f64,
    /// Maximum pheromone
    tau_max: f64,
    /// Stagnation counter
    stagnation: usize,
    /// Stagnation limit
    stagnation_limit: usize,
}

impl MaxMinAntSystem {
    /// Create MMAS
    pub fn new(distances: DistanceMatrix, config: AcoConfig) -> Self {
        let inner = AcoOptimizer::new(distances, config);
        Self {
            inner,
            tau_min: 0.001,
            tau_max: 10.0,
            stagnation: 0,
            stagnation_limit: 20,
        }
    }

    /// Update pheromone bounds
    fn update_bounds(&mut self) {
        if self.inner.best_cost < f64::MAX && self.inner.best_cost > 0.0 {
            self.tau_max = 1.0 / (self.inner.config.evaporation_rate * self.inner.best_cost);
            let n = self.inner.distances.n_nodes as f64;
            self.tau_min = self.tau_max / (2.0 * n);
        }
    }

    /// Clamp pheromones to bounds
    fn clamp_pheromones(&mut self) {
        for v in &mut self.inner.pheromones.values {
            if *v < self.tau_min {
                *v = self.tau_min;
            }
            if *v > self.tau_max {
                *v = self.tau_max;
            }
        }
    }

    /// Check and handle stagnation
    fn check_stagnation(&mut self) {
        // Simple stagnation detection based on lack of improvement
        self.stagnation += 1;

        if self.stagnation >= self.stagnation_limit {
            // Reset pheromones to tau_max
            for v in &mut self.inner.pheromones.values {
                *v = self.tau_max;
            }
            self.stagnation = 0;
        }
    }

    /// Run one iteration
    pub fn iterate(&mut self) {
        let prev_best = self.inner.best_cost;
        self.inner.iterate();

        if self.inner.best_cost < prev_best {
            self.stagnation = 0;
        } else {
            self.check_stagnation();
        }

        self.update_bounds();
        self.clamp_pheromones();
    }

    /// Run full optimization
    pub fn optimize(&mut self) -> AcoResult {
        let mut history = Vec::with_capacity(self.inner.config.max_iterations);

        for _ in 0..self.inner.config.max_iterations {
            self.iterate();
            history.push(self.inner.best_cost);
        }

        AcoResult {
            best_path: self.inner.best_path.clone(),
            best_cost: self.inner.best_cost,
            history,
        }
    }
}

/// Ant Colony System (ACS) with local search
pub struct AntColonySystem {
    /// Base ACO
    inner: AcoOptimizer,
    /// Exploitation parameter q0
    q0: f64,
    /// Local pheromone decay
    local_decay: f64,
}

impl AntColonySystem {
    /// Create ACS
    pub fn new(distances: DistanceMatrix, config: AcoConfig) -> Self {
        let inner = AcoOptimizer::new(distances, config);
        Self {
            inner,
            q0: 0.9, // 90% exploitation, 10% exploration
            local_decay: 0.1,
        }
    }

    /// Choose next node with pseudo-random proportional rule
    fn acs_choose_next(&mut self, ant: &Ant) -> Option<usize> {
        let current = *ant.path.last()?;
        let n = self.inner.distances.n_nodes;

        // Find best unvisited
        let mut best_node = None;
        let mut best_score = 0.0;

        let mut probs = Vec::with_capacity(n);
        let mut total = 0.0;

        for next in 0..n {
            if ant.is_visited(next) {
                probs.push(0.0);
                continue;
            }

            let tau = self.inner.pheromones.get(current, next);
            let eta = self.inner.heuristic(current, next);

            let score = tau * libm::pow(eta, self.inner.config.beta);
            probs.push(score);
            total += score;

            if score > best_score {
                best_score = score;
                best_node = Some(next);
            }
        }

        if total < 1e-10 {
            return None;
        }

        let q = self.inner.rand();

        if q < self.q0 {
            // Exploitation: choose best
            best_node
        } else {
            // Exploration: probabilistic
            let r = self.inner.rand() * total;
            let mut cumsum = 0.0;

            for (node, &prob) in probs.iter().enumerate() {
                cumsum += prob;
                if r <= cumsum {
                    return Some(node);
                }
            }

            best_node
        }
    }

    /// Local pheromone update
    fn local_pheromone_update(&mut self, from: usize, to: usize) {
        let current = self.inner.pheromones.get(from, to);
        let new_val = (1.0 - self.local_decay) * current
            + self.local_decay * self.inner.config.initial_pheromone;
        self.inner.pheromones.set(from, to, new_val);
    }

    /// Global pheromone update (best-so-far ant only)
    fn global_pheromone_update(&mut self) {
        if self.inner.best_cost >= f64::MAX || self.inner.best_cost <= 0.0 {
            return;
        }

        // Evaporate all
        self.inner
            .pheromones
            .evaporate(self.inner.config.evaporation_rate);

        // Deposit only on best path
        let deposit = 1.0 / self.inner.best_cost;
        for i in 0..self.inner.best_path.len() {
            let from = self.inner.best_path[i];
            let to = self.inner.best_path[(i + 1) % self.inner.best_path.len()];
            self.inner.pheromones.deposit(from, to, deposit);
        }
    }

    /// Run full optimization
    pub fn optimize(&mut self) -> AcoResult {
        let mut history = Vec::with_capacity(self.inner.config.max_iterations);

        for _ in 0..self.inner.config.max_iterations {
            // Build solutions with local updates
            let n = self.inner.distances.n_nodes;

            for i in 0..self.inner.ants.len() {
                let start = i % n;
                self.inner.ants[i].reset(start);
            }

            // Build paths with local update
            for ant_idx in 0..self.inner.ants.len() {
                loop {
                    if self.inner.ants[ant_idx].path_len() >= n {
                        break;
                    }
                    // Clone ant data to avoid borrowing self.inner twice
                    let ant_clone = self.inner.ants[ant_idx].clone();
                    if let Some(next) = self.acs_choose_next(&ant_clone) {
                        let current = *self.inner.ants[ant_idx].path.last().unwrap();
                        let cost = self.inner.distances.get(current, next);
                        self.inner.ants[ant_idx].visit(next, cost);
                        self.local_pheromone_update(current, next);
                    } else {
                        break;
                    }
                }

                // Return to start
                if self.inner.ants[ant_idx].path_len() == n {
                    let start = self.inner.ants[ant_idx].path[0];
                    let last = *self.inner.ants[ant_idx].path.last().unwrap();
                    let return_cost = self.inner.distances.get(last, start);
                    if return_cost < f64::MAX {
                        self.inner.ants[ant_idx].path_cost += return_cost;
                    }
                }
            }

            // Update best
            for ant in &self.inner.ants {
                if ant.path_cost < self.inner.best_cost {
                    self.inner.best_cost = ant.path_cost;
                    self.inner.best_path = ant.path.clone();
                }
            }

            self.global_pheromone_update();
            history.push(self.inner.best_cost);
        }

        AcoResult {
            best_path: self.inner.best_path.clone(),
            best_cost: self.inner.best_cost,
            history,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_graph() -> DistanceMatrix {
        // Simple 4-node graph
        let edges = [
            (0, 1, 1.0),
            (0, 2, 2.0),
            (0, 3, 3.0),
            (1, 2, 1.5),
            (1, 3, 2.5),
            (2, 3, 1.0),
        ];
        DistanceMatrix::from_edges(4, &edges)
    }

    #[test]
    fn test_distance_matrix() {
        let dm = make_test_graph();

        assert!((dm.get(0, 1) - 1.0).abs() < 1e-10);
        assert!((dm.get(1, 0) - 1.0).abs() < 1e-10);
        assert!((dm.get(2, 3) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_aco_basic() {
        let dm = make_test_graph();
        let config = AcoConfig {
            n_ants: 10,
            max_iterations: 20,
            ..Default::default()
        };

        let mut aco = AcoOptimizer::new(dm, config);
        let result = aco.optimize();

        // Should find a valid path
        assert!(!result.best_path.is_empty());
        assert!(result.best_cost < f64::MAX);
    }

    #[test]
    fn test_mmas() {
        let dm = make_test_graph();
        let config = AcoConfig {
            n_ants: 10,
            max_iterations: 30,
            ..Default::default()
        };

        let mut mmas = MaxMinAntSystem::new(dm, config);
        let result = mmas.optimize();

        assert!(!result.best_path.is_empty());
        assert!(result.best_cost < f64::MAX);
    }

    #[test]
    fn test_acs() {
        let dm = make_test_graph();
        let config = AcoConfig {
            n_ants: 10,
            max_iterations: 30,
            ..Default::default()
        };

        let mut acs = AntColonySystem::new(dm, config);
        let result = acs.optimize();

        assert!(!result.best_path.is_empty());
        assert!(result.best_cost < f64::MAX);
    }
}
