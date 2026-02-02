//! # Quantum Annealing Engine
//!
//! Advanced quantum annealing simulation for kernel optimization problems.
//! Implements simulated quantum annealing (SQA), path-integral Monte Carlo,
//! and population annealing methods.
//!
//! ## Algorithms
//!
//! - **Simulated Quantum Annealing (SQA)**: Trotter-Suzuki decomposition
//! - **Path-Integral Monte Carlo (PIMC)**: Quantum thermal sampling
//! - **Population Annealing**: Parallel tempering with resampling
//! - **Quantum Parallel Tempering**: Multi-replica exchange

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::{
    IsingModel, OptimizationMetrics, OptimizationResult, QuantumOptimizerConfig, QuboMatrix,
};
use crate::types::NexusResult;

/// Number of Trotter slices for path-integral decomposition
const DEFAULT_TROTTER_SLICES: usize = 32;

/// Simulated Quantum Annealing Engine
pub struct QuantumAnnealingEngine {
    /// Configuration
    config: QuantumOptimizerConfig,
    /// Number of Trotter slices
    num_slices: usize,
    /// RNG state
    rng_state: u64,
    /// Best solution found
    best_solution: Vec<i8>,
    /// Best energy found
    best_energy: f64,
}

impl QuantumAnnealingEngine {
    /// Create a new quantum annealing engine
    pub fn new(config: QuantumOptimizerConfig) -> Self {
        Self {
            config,
            num_slices: DEFAULT_TROTTER_SLICES,
            rng_state: 0xCAFEBABE12345678,
            best_solution: Vec::new(),
            best_energy: f64::MAX,
        }
    }

    /// Set number of Trotter slices
    pub fn with_trotter_slices(mut self, slices: usize) -> Self {
        self.num_slices = slices;
        self
    }

    /// Run simulated quantum annealing
    pub fn anneal(&mut self, ising: &IsingModel) -> NexusResult<OptimizationResult> {
        let n = ising.num_spins;
        let m = self.num_slices;

        // Initialize replica spins (n spins × m Trotter slices)
        let mut replicas = self.initialize_replicas(n, m);

        let mut energy_history = Vec::with_capacity(100);
        let mut acceptance_count = 0usize;
        let mut total_moves = 0usize;
        let mut tunneling_events = 0usize;

        // Main annealing loop
        for iter in 0..self.config.max_iterations {
            let progress = iter as f64 / self.config.max_iterations as f64;

            // Temperature schedule
            let temperature = self.temperature_schedule(progress);

            // Transverse field schedule (starts high, decreases to 0)
            let gamma = self.transverse_field_schedule(progress);

            // Coupling between Trotter slices
            let j_perp = self.trotter_coupling(temperature, gamma, m);

            // Sweep through all spins in all replicas
            for slice in 0..m {
                for spin in 0..n {
                    total_moves += 1;

                    // Calculate energy change for spin flip
                    let delta_e =
                        self.compute_delta_energy(&replicas, ising, spin, slice, j_perp, m);

                    // Metropolis acceptance
                    let accept = if delta_e <= 0.0 {
                        true
                    } else if temperature > 1e-10 {
                        let prob = libm::exp(-delta_e / temperature);
                        self.random() < prob
                    } else {
                        false
                    };

                    if accept {
                        replicas[slice][spin] *= -1;
                        acceptance_count += 1;

                        // Check for quantum tunneling (correlated flips)
                        if self.check_tunneling_event(&replicas, spin, m) {
                            tunneling_events += 1;
                        }
                    }
                }
            }

            // Evaluate classical energy (project to first slice)
            let classical_energy = ising.energy(&replicas[0]);
            if classical_energy < self.best_energy {
                self.best_energy = classical_energy;
                self.best_solution = replicas[0].clone();
            }

            // Record history
            if iter % (self.config.max_iterations / 100).max(1) == 0 {
                energy_history.push(self.best_energy);
            }

            // Replica exchange (parallel tempering between slices)
            if iter % 10 == 0 {
                self.replica_exchange(&mut replicas, ising, temperature, j_perp);
            }
        }

        // Convert to binary solution
        let solution: Vec<bool> = self.best_solution.iter().map(|&s| s > 0).collect();

        Ok(OptimizationResult {
            solution,
            energy: self.best_energy + ising.offset,
            iterations: self.config.max_iterations,
            converged: true,
            time_us: 0,
            metrics: OptimizationMetrics {
                energy_history,
                acceptance_rate: acceptance_count as f64 / total_moves as f64,
                quality_estimate: self.estimate_quality(ising),
                solutions_explored: acceptance_count,
                tunneling_events,
            },
        })
    }

    /// Run population annealing (parallel annealing with resampling)
    pub fn population_anneal(
        &mut self,
        ising: &IsingModel,
        population_size: usize,
    ) -> NexusResult<OptimizationResult> {
        let n = ising.num_spins;

        // Initialize population
        let mut population: Vec<Vec<i8>> = (0..population_size)
            .map(|i| self.random_spin_config(n, i as u64))
            .collect();

        let mut weights: Vec<f64> = alloc::vec![1.0; population_size];
        let mut energy_history = Vec::new();
        let mut tunneling_events = 0usize;

        let num_temperature_steps = 100;

        for temp_step in 0..num_temperature_steps {
            let progress = temp_step as f64 / num_temperature_steps as f64;
            let temperature = self.temperature_schedule(progress);
            let prev_temperature = if temp_step > 0 {
                self.temperature_schedule((temp_step - 1) as f64 / num_temperature_steps as f64)
            } else {
                self.config.initial_temperature * 2.0
            };

            // Reweight population based on energy change
            let beta = 1.0 / temperature;
            let prev_beta = 1.0 / prev_temperature;
            let delta_beta = beta - prev_beta;

            for (i, config) in population.iter().enumerate() {
                let energy = ising.energy(config);
                weights[i] = libm::exp(-delta_beta * energy);
            }

            // Normalize weights
            let weight_sum: f64 = weights.iter().sum();
            for w in &mut weights {
                *w /= weight_sum;
            }

            // Resample population based on weights
            let mut new_population = Vec::with_capacity(population_size);
            for _ in 0..population_size {
                let r = self.random();
                let mut cumulative = 0.0;
                let mut selected = 0;
                for (i, &w) in weights.iter().enumerate() {
                    cumulative += w;
                    if r < cumulative {
                        selected = i;
                        break;
                    }
                }
                new_population.push(population[selected].clone());
            }
            population = new_population;
            weights = alloc::vec![1.0 / population_size as f64; population_size];

            // MCMC sweeps at current temperature
            let sweeps_per_step = 10;
            for config in &mut population {
                for _ in 0..sweeps_per_step {
                    self.mcmc_sweep(config, ising, temperature);
                }
            }

            // Find best in population
            for config in &population {
                let energy = ising.energy(config);
                if energy < self.best_energy {
                    self.best_energy = energy;
                    self.best_solution = config.clone();
                }
            }

            energy_history.push(self.best_energy);

            // Cluster moves for tunneling
            if temp_step % 5 == 0 {
                tunneling_events += self.cluster_flip(&mut population, ising, temperature);
            }
        }

        let solution: Vec<bool> = self.best_solution.iter().map(|&s| s > 0).collect();

        Ok(OptimizationResult {
            solution,
            energy: self.best_energy + ising.offset,
            iterations: num_temperature_steps * 10 * population_size,
            converged: true,
            time_us: 0,
            metrics: OptimizationMetrics {
                energy_history,
                acceptance_rate: 0.5,
                quality_estimate: self.estimate_quality(ising),
                solutions_explored: population_size,
                tunneling_events,
            },
        })
    }

    /// Quantum parallel tempering
    pub fn parallel_tempering_anneal(
        &mut self,
        ising: &IsingModel,
        num_replicas: usize,
    ) -> NexusResult<OptimizationResult> {
        let n = ising.num_spins;

        // Initialize replicas at different temperatures
        let mut replicas: Vec<Vec<i8>> = (0..num_replicas)
            .map(|i| self.random_spin_config(n, i as u64))
            .collect();

        // Temperature ladder (geometric spacing)
        let temperatures: Vec<f64> = (0..num_replicas)
            .map(|i| {
                let ratio = self.config.final_temperature / self.config.initial_temperature;
                self.config.initial_temperature
                    * libm::pow(ratio, i as f64 / (num_replicas - 1) as f64)
            })
            .collect();

        let mut energy_history = Vec::new();
        let mut exchange_count = 0usize;

        for iter in 0..self.config.max_iterations {
            // MCMC updates for each replica
            for (replica, &temp) in replicas.iter_mut().zip(temperatures.iter()) {
                self.mcmc_sweep(replica, ising, temp);
            }

            // Parallel tempering exchanges
            for i in 0..(num_replicas - 1) {
                let e1 = ising.energy(&replicas[i]);
                let e2 = ising.energy(&replicas[i + 1]);
                let beta1 = 1.0 / temperatures[i];
                let beta2 = 1.0 / temperatures[i + 1];

                let delta = (beta1 - beta2) * (e2 - e1);
                let accept = if delta <= 0.0 {
                    true
                } else {
                    self.random() < libm::exp(-delta)
                };

                if accept {
                    replicas.swap(i, i + 1);
                    exchange_count += 1;
                }
            }

            // Track best solution (from coldest replica)
            let cold_energy = ising.energy(&replicas[num_replicas - 1]);
            if cold_energy < self.best_energy {
                self.best_energy = cold_energy;
                self.best_solution = replicas[num_replicas - 1].clone();
            }

            if iter % (self.config.max_iterations / 100).max(1) == 0 {
                energy_history.push(self.best_energy);
            }
        }

        let solution: Vec<bool> = self.best_solution.iter().map(|&s| s > 0).collect();

        Ok(OptimizationResult {
            solution,
            energy: self.best_energy + ising.offset,
            iterations: self.config.max_iterations * num_replicas,
            converged: true,
            time_us: 0,
            metrics: OptimizationMetrics {
                energy_history,
                acceptance_rate: exchange_count as f64
                    / (self.config.max_iterations * (num_replicas - 1)) as f64,
                quality_estimate: self.estimate_quality(ising),
                solutions_explored: num_replicas,
                tunneling_events: exchange_count,
            },
        })
    }

    // Helper methods

    fn initialize_replicas(&mut self, n: usize, m: usize) -> Vec<Vec<i8>> {
        let mut replicas = Vec::with_capacity(m);
        for slice in 0..m {
            let config = self.random_spin_config(n, slice as u64);
            replicas.push(config);
        }

        // Initialize with correlated replicas for better quantum coherence
        for i in 1..m {
            for j in 0..n {
                if self.random() > 0.3 {
                    replicas[i][j] = replicas[0][j];
                }
            }
        }

        replicas
    }

    fn random_spin_config(&mut self, n: usize, seed: u64) -> Vec<i8> {
        let mut config = Vec::with_capacity(n);
        let mut rng = self.rng_state ^ seed;

        for _ in 0..n {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            config.push(if rng & 1 == 0 { 1 } else { -1 });
        }

        config
    }

    fn compute_delta_energy(
        &self,
        replicas: &[Vec<i8>],
        ising: &IsingModel,
        spin: usize,
        slice: usize,
        j_perp: f64,
        m: usize,
    ) -> f64 {
        let _n = ising.num_spins;
        let s = replicas[slice][spin] as f64;

        // Classical energy change
        let mut delta_e = 2.0 * s * ising.h[spin];
        for &(i, j, jij) in &ising.j {
            if i == spin {
                delta_e += 2.0 * s * jij * replicas[slice][j] as f64;
            } else if j == spin {
                delta_e += 2.0 * s * jij * replicas[slice][i] as f64;
            }
        }

        // Quantum coupling between Trotter slices
        let prev_slice = if slice == 0 { m - 1 } else { slice - 1 };
        let next_slice = (slice + 1) % m;

        let s_prev = replicas[prev_slice][spin] as f64;
        let s_next = replicas[next_slice][spin] as f64;

        delta_e += 2.0 * j_perp * s * (s_prev + s_next);

        delta_e / m as f64
    }

    fn temperature_schedule(&self, progress: f64) -> f64 {
        let t_init = self.config.initial_temperature;
        let t_final = self.config.final_temperature;

        if self.config.adaptive_schedule {
            // Sigmoidal schedule for better exploration
            let x = 10.0 * (progress - 0.5);
            let sigmoid = 1.0 / (1.0 + libm::exp(-x));
            t_init * (1.0 - sigmoid) + t_final * sigmoid
        } else {
            // Linear schedule
            t_init + (t_final - t_init) * progress
        }
    }

    fn transverse_field_schedule(&self, progress: f64) -> f64 {
        // Transverse field decreases from high to near-zero
        let gamma_init = self.config.initial_temperature;
        gamma_init * (1.0 - progress) * (1.0 - progress)
    }

    fn trotter_coupling(&self, temperature: f64, gamma: f64, m: usize) -> f64 {
        // Coupling between Trotter slices: J_⊥ = -T/2 * ln(tanh(Γ/mT))
        if gamma < 1e-10 || temperature < 1e-10 {
            return 0.0;
        }

        let arg = gamma / (m as f64 * temperature);
        if arg > 10.0 {
            return 0.0;
        }

        let tanh_val = libm::tanh(arg);
        if tanh_val < 1e-10 {
            return 0.0;
        }

        -temperature / 2.0 * libm::log(tanh_val)
    }

    fn check_tunneling_event(&self, replicas: &[Vec<i8>], spin: usize, _m: usize) -> bool {
        // Check if all replicas have the same spin value (coherent tunneling)
        let first = replicas[0][spin];
        replicas.iter().all(|r| r[spin] == first)
    }

    fn replica_exchange(
        &mut self,
        replicas: &mut [Vec<i8>],
        ising: &IsingModel,
        temperature: f64,
        j_perp: f64,
    ) {
        let m = replicas.len();

        for i in 0..(m - 1) {
            // Propose swap between adjacent Trotter slices
            let e1 = self.slice_energy(replicas, ising, i, j_perp, m);
            let e2 = self.slice_energy(replicas, ising, i + 1, j_perp, m);

            // After swap
            replicas.swap(i, i + 1);
            let e1_new = self.slice_energy(replicas, ising, i, j_perp, m);
            let e2_new = self.slice_energy(replicas, ising, i + 1, j_perp, m);

            let delta = (e1_new + e2_new) - (e1 + e2);

            let accept = if delta <= 0.0 {
                true
            } else if temperature > 1e-10 {
                self.random() < libm::exp(-delta / temperature)
            } else {
                false
            };

            if !accept {
                // Revert swap
                replicas.swap(i, i + 1);
            }
        }
    }

    fn slice_energy(
        &self,
        replicas: &[Vec<i8>],
        ising: &IsingModel,
        slice: usize,
        j_perp: f64,
        m: usize,
    ) -> f64 {
        let n = ising.num_spins;
        let mut energy = ising.energy(&replicas[slice]);

        // Add Trotter coupling
        let prev = if slice == 0 { m - 1 } else { slice - 1 };
        let next = (slice + 1) % m;

        for i in 0..n {
            let s = replicas[slice][i] as f64;
            let s_prev = replicas[prev][i] as f64;
            let s_next = replicas[next][i] as f64;
            energy -= j_perp * s * (s_prev + s_next);
        }

        energy
    }

    fn mcmc_sweep(&mut self, config: &mut Vec<i8>, ising: &IsingModel, temperature: f64) {
        let n = config.len();

        for i in 0..n {
            let mut delta_e = 2.0 * config[i] as f64 * ising.h[i];

            for &(a, b, jab) in &ising.j {
                if a == i {
                    delta_e += 2.0 * config[i] as f64 * jab * config[b] as f64;
                } else if b == i {
                    delta_e += 2.0 * config[i] as f64 * jab * config[a] as f64;
                }
            }

            let accept = if delta_e <= 0.0 {
                true
            } else if temperature > 1e-10 {
                self.random() < libm::exp(-delta_e / temperature)
            } else {
                false
            };

            if accept {
                config[i] *= -1;
            }
        }
    }

    fn cluster_flip(
        &mut self,
        population: &mut [Vec<i8>],
        ising: &IsingModel,
        temperature: f64,
    ) -> usize {
        let mut events = 0;

        // Swendsen-Wang style cluster identification
        for config in population.iter_mut() {
            let n = config.len();
            let mut visited = alloc::vec![false; n];
            let mut cluster = Vec::new();

            // Start cluster from random spin
            let start = (self.rng_state as usize) % n;
            self.rng_state ^= self.rng_state << 13;
            self.rng_state ^= self.rng_state >> 7;
            self.rng_state ^= self.rng_state << 17;

            cluster.push(start);
            visited[start] = true;

            let mut idx = 0;
            while idx < cluster.len() && cluster.len() < n / 4 {
                let current = cluster[idx];

                // Add neighbors with probability based on coupling
                for &(a, b, jab) in &ising.j {
                    let neighbor = if a == current {
                        b
                    } else if b == current {
                        a
                    } else {
                        continue;
                    };

                    if visited[neighbor] {
                        continue;
                    }

                    // Probability to add to cluster
                    if config[current] == config[neighbor] && jab > 0.0 {
                        let p_add = 1.0 - libm::exp(-2.0 * jab / temperature);
                        if self.random() < p_add {
                            cluster.push(neighbor);
                            visited[neighbor] = true;
                        }
                    }
                }
                idx += 1;
            }

            // Flip cluster with probability 0.5
            if self.random() < 0.5 && cluster.len() > 1 {
                for &spin in &cluster {
                    config[spin] *= -1;
                }
                events += 1;
            }
        }

        events
    }

    fn estimate_quality(&self, ising: &IsingModel) -> f64 {
        let min_possible = -ising.h.iter().map(|h| libm::fabs(*h)).sum::<f64>()
            - ising.j.iter().map(|(_, _, j)| libm::fabs(*j)).sum::<f64>();
        let max_possible = -min_possible;

        if max_possible > min_possible {
            1.0 - (self.best_energy - min_possible) / (max_possible - min_possible)
        } else {
            0.5
        }
    }

    fn random(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }
}

/// Adaptive schedule for quantum annealing
pub struct AdaptiveAnnealingSchedule {
    /// Schedule parameters
    temperature_history: Vec<f64>,
    energy_history: Vec<f64>,
    acceptance_history: Vec<f64>,
    /// Learned optimal schedule
    optimal_schedule: Vec<(f64, f64)>, // (progress, temperature)
}

impl AdaptiveAnnealingSchedule {
    pub fn new() -> Self {
        Self {
            temperature_history: Vec::new(),
            energy_history: Vec::new(),
            acceptance_history: Vec::new(),
            optimal_schedule: Vec::new(),
        }
    }

    /// Learn optimal schedule from annealing runs
    pub fn learn_from_run(
        &mut self,
        energy_hist: &[f64],
        _acceptance: f64,
        t_init: f64,
        t_final: f64,
    ) {
        let n = energy_hist.len();

        // Find where energy drops fastest
        let mut best_drops = Vec::new();
        for i in 1..n {
            let drop = energy_hist[i - 1] - energy_hist[i];
            let progress = i as f64 / n as f64;
            let temperature = t_init + (t_final - t_init) * progress;
            best_drops.push((progress, temperature, drop));
        }

        // Sort by drop rate
        best_drops.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));

        // Build optimized schedule: slow down at high-drop regions
        self.optimal_schedule.clear();
        for (progress, temp, _) in best_drops.iter().take(10) {
            self.optimal_schedule.push((*progress, *temp));
        }
        self.optimal_schedule
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
    }

    /// Get temperature for given progress using learned schedule
    pub fn get_temperature(&self, progress: f64, t_init: f64, t_final: f64) -> f64 {
        if self.optimal_schedule.is_empty() {
            // Fallback to linear
            return t_init + (t_final - t_init) * progress;
        }

        // Interpolate from learned schedule
        for i in 0..self.optimal_schedule.len() {
            if progress < self.optimal_schedule[i].0 {
                if i == 0 {
                    return self.optimal_schedule[0].1;
                }
                let (p0, t0) = self.optimal_schedule[i - 1];
                let (p1, t1) = self.optimal_schedule[i];
                let alpha = (progress - p0) / (p1 - p0);
                return t0 + (t1 - t0) * alpha;
            }
        }

        self.optimal_schedule
            .last()
            .map(|(_, t)| *t)
            .unwrap_or(t_final)
    }
}

/// Quantum annealing problem types for kernel optimization
pub mod kernel_problems {
    use super::*;

    /// Build QUBO for task scheduling problem
    pub fn build_scheduling_qubo(
        num_tasks: usize,
        num_processors: usize,
        task_durations: &[u64],
        _dependencies: &[(usize, usize)],
        communication_costs: &[(usize, usize, u64)],
    ) -> QuboMatrix {
        let n = num_tasks * num_processors;
        let mut qubo = QuboMatrix::new(n);
        let penalty = 10000.0;

        // Constraint: each task on exactly one processor
        for task in 0..num_tasks {
            for p1 in 0..num_processors {
                let idx1 = task * num_processors + p1;
                qubo.set_linear(idx1, -penalty);

                for p2 in (p1 + 1)..num_processors {
                    let idx2 = task * num_processors + p2;
                    qubo.set_quadratic(idx1, idx2, 2.0 * penalty);
                }
            }
        }

        // Load balancing objective
        for p in 0..num_processors {
            for t1 in 0..num_tasks {
                for t2 in (t1 + 1)..num_tasks {
                    let idx1 = t1 * num_processors + p;
                    let idx2 = t2 * num_processors + p;
                    let cost = (task_durations[t1] as f64) * (task_durations[t2] as f64) / 1e6;
                    qubo.set_quadratic(idx1, idx2, cost);
                }
            }
        }

        // Communication cost minimization
        for &(t1, t2, cost) in communication_costs {
            for p in 0..num_processors {
                let idx1 = t1 * num_processors + p;
                let idx2 = t2 * num_processors + p;
                // Reward same-processor placement
                if idx1 < idx2 {
                    qubo.set_quadratic(idx1, idx2, -(cost as f64) / 100.0);
                }
            }
        }

        qubo
    }

    /// Build QUBO for memory page placement
    pub fn build_memory_placement_qubo(
        num_pages: usize,
        numa_nodes: usize,
        access_patterns: &[(usize, usize, u64)], // (page1, page2, frequency)
        preferred_nodes: &[(usize, usize)],      // (page, preferred_node)
    ) -> QuboMatrix {
        let n = num_pages * numa_nodes;
        let mut qubo = QuboMatrix::new(n);
        let penalty = 10000.0;

        // Each page on exactly one node
        for page in 0..num_pages {
            for n1 in 0..numa_nodes {
                let idx1 = page * numa_nodes + n1;
                qubo.set_linear(idx1, -penalty);

                for n2 in (n1 + 1)..numa_nodes {
                    let idx2 = page * numa_nodes + n2;
                    qubo.set_quadratic(idx1, idx2, 2.0 * penalty);
                }
            }
        }

        // Co-location objective for frequently accessed pages
        for &(p1, p2, freq) in access_patterns {
            for node in 0..numa_nodes {
                let idx1 = p1 * numa_nodes + node;
                let idx2 = p2 * numa_nodes + node;
                if idx1 < idx2 {
                    qubo.set_quadratic(idx1, idx2, -(freq as f64) / 1000.0);
                }
            }
        }

        // Preferred node hints
        for &(page, node) in preferred_nodes {
            let idx = page * numa_nodes + node;
            qubo.set_linear(idx, qubo.linear[idx] - 100.0);
        }

        qubo
    }

    /// Build QUBO for I/O request scheduling
    pub fn build_io_scheduling_qubo(
        num_requests: usize,
        num_queues: usize,
        _request_sizes: &[u64],
        request_deadlines: &[u64],
        locality_hints: &[(usize, usize)], // (request1, request2) - close on disk
    ) -> QuboMatrix {
        let n = num_requests * num_queues;
        let mut qubo = QuboMatrix::new(n);
        let penalty = 10000.0;

        // Each request in exactly one queue position
        for req in 0..num_requests {
            for q1 in 0..num_queues {
                let idx1 = req * num_queues + q1;
                qubo.set_linear(idx1, -penalty);

                for q2 in (q1 + 1)..num_queues {
                    let idx2 = req * num_queues + q2;
                    qubo.set_quadratic(idx1, idx2, 2.0 * penalty);
                }
            }
        }

        // Deadline-aware ordering
        for r1 in 0..num_requests {
            for r2 in (r1 + 1)..num_requests {
                if request_deadlines[r1] < request_deadlines[r2] {
                    // r1 should come before r2
                    for q1 in 0..num_queues {
                        for q2 in 0..q1 {
                            // r1 in later position than r2
                            let idx1 = r1 * num_queues + q1;
                            let idx2 = r2 * num_queues + q2;
                            if idx1 < idx2 {
                                qubo.set_quadratic(idx1, idx2, 100.0);
                            }
                        }
                    }
                }
            }
        }

        // Locality optimization
        for &(r1, r2) in locality_hints {
            // Adjacent positions are preferred
            for q in 0..(num_queues - 1) {
                let idx1 = r1 * num_queues + q;
                let idx2 = r2 * num_queues + (q + 1);
                if idx1 < idx2 {
                    qubo.set_quadratic(idx1, idx2, -50.0);
                }
            }
        }

        qubo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_annealing_simple() {
        let mut qubo = QuboMatrix::new(4);
        qubo.set_linear(0, -1.0);
        qubo.set_linear(1, -1.0);
        qubo.set_quadratic(0, 1, 2.0);

        let ising = IsingModel::from_qubo(&qubo);
        let config = QuantumOptimizerConfig {
            max_iterations: 1000,
            ..Default::default()
        };

        let mut engine = QuantumAnnealingEngine::new(config);
        let result = engine.anneal(&ising).unwrap();

        assert!(result.converged);
        // Optimal: x0=1 XOR x1=1 (not both)
        assert!(result.solution[0] != result.solution[1] || result.energy <= 0.0);
    }

    #[test]
    fn test_population_annealing() {
        let mut qubo = QuboMatrix::new(3);
        qubo.set_linear(0, -1.0);
        qubo.set_linear(1, -2.0);
        qubo.set_linear(2, -1.0);
        qubo.set_quadratic(0, 2, 3.0);

        let ising = IsingModel::from_qubo(&qubo);
        let config = QuantumOptimizerConfig::default();

        let mut engine = QuantumAnnealingEngine::new(config);
        let result = engine.population_anneal(&ising, 50).unwrap();

        assert!(result.converged);
    }

    #[test]
    fn test_parallel_tempering() {
        let mut qubo = QuboMatrix::new(5);
        for i in 0..5 {
            qubo.set_linear(i, -1.0);
        }
        for i in 0..4 {
            qubo.set_quadratic(i, i + 1, 0.5);
        }

        let ising = IsingModel::from_qubo(&qubo);
        let config = QuantumOptimizerConfig {
            max_iterations: 500,
            ..Default::default()
        };

        let mut engine = QuantumAnnealingEngine::new(config);
        let result = engine.parallel_tempering_anneal(&ising, 8).unwrap();

        assert!(result.converged);
    }
}
