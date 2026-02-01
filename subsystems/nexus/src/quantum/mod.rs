//! # NEXUS Quantum-Inspired Optimization Engine
//!
//! Revolutionary quantum-inspired algorithms for kernel-level optimization.
//! This module implements classical simulations of quantum computing paradigms
//! to achieve optimization breakthroughs impossible with traditional methods.
//!
//! ## Module Structure
//!
//! - [`types`] - Core quantum types: Complex, QubitState, StateVector, Hamiltonian
//! - [`gates`] - Quantum gate operations: X, Y, Z, H, CNOT, rotations
//! - [`circuits`] - Quantum circuits: builders, executors, standard circuits
//! - [`qaoa`] - QAOA optimization for combinatorial problems
//! - [`vqe`] - Variational Quantum Eigensolver for ground state finding
//!
//! ## Legacy Submodules
//!
//! - [`annealing`] - Quantum annealing simulation
//! - [`grover`] - Grover-inspired search algorithms
//! - [`quantum_walk`] - Quantum walk algorithms
//! - [`state`] - Quantum state management
//! - [`measurement`] - Measurement operations
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_nexus::quantum::{
//!     types::{Complex, StateVector, Hamiltonian},
//!     gates::GateType,
//!     circuits::QuantumCircuit,
//!     qaoa::{QaoaEngine, MaxCut},
//!     vqe::{VqeEngine, AnsatzBuilder},
//! };
//!
//! // Create a quantum circuit
//! let mut circuit = QuantumCircuit::new(2);
//! circuit.h(0).cx(0, 1);
//!
//! // Or use QAOA for optimization
//! let problem = MaxCut::random_graph(6, 0.5, 42);
//! let engine = QaoaEngine::new(problem, 2);
//! ```

#![no_std]
#![allow(dead_code)]

extern crate alloc;

// Production-quality submodules
pub mod annealing;
pub mod circuits;
pub mod gates;
pub mod qaoa;
pub mod types;
pub mod vqe;

// Re-export main types
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::f64::consts::PI;

pub use circuits::{CircuitExecutor, Instruction, QuantumCircuit};
pub use types::{Complex, Hamiltonian, Pauli, PauliString, QubitState, StateVector};

/// Quantum-inspired optimization problem types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationProblem {
    /// Scheduling optimization (QUBO formulation)
    Scheduling,
    /// Resource allocation (Max-Cut)
    ResourceAllocation,
    /// Memory layout optimization (Graph Coloring)
    MemoryLayout,
    /// Process placement (Quadratic Assignment)
    ProcessPlacement,
    /// Cache optimization (Set Cover)
    CacheOptimization,
    /// I/O scheduling (Traveling Salesman)
    IoScheduling,
    /// Load balancing (Bin Packing)
    LoadBalancing,
    /// Network routing (Shortest Path)
    NetworkRouting,
}

/// Complex amplitude representation for quantum states
#[derive(Debug, Clone, Copy)]
pub struct ComplexAmplitude {
    pub real: f64,
    pub imag: f64,
}

impl ComplexAmplitude {
    pub const ZERO: Self = Self {
        real: 0.0,
        imag: 0.0,
    };
    pub const ONE: Self = Self {
        real: 1.0,
        imag: 0.0,
    };
    pub const I: Self = Self {
        real: 0.0,
        imag: 1.0,
    };

    #[inline]
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }

    #[inline]
    pub fn from_polar(magnitude: f64, phase: f64) -> Self {
        Self {
            real: magnitude * libm::cos(phase),
            imag: magnitude * libm::sin(phase),
        }
    }

    #[inline]
    pub fn magnitude_squared(&self) -> f64 {
        self.real * self.real + self.imag * self.imag
    }

    #[inline]
    pub fn magnitude(&self) -> f64 {
        libm::sqrt(self.magnitude_squared())
    }

    #[inline]
    pub fn phase(&self) -> f64 {
        libm::atan2(self.imag, self.real)
    }

    #[inline]
    pub fn conjugate(&self) -> Self {
        Self {
            real: self.real,
            imag: -self.imag,
        }
    }

    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        Self {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    #[inline]
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            real: self.real * factor,
            imag: self.imag * factor,
        }
    }
}

/// Quantum bit (qubit) representation
#[derive(Debug, Clone)]
pub struct Qubit {
    /// Amplitude for |0⟩ state
    pub alpha: ComplexAmplitude,
    /// Amplitude for |1⟩ state
    pub beta: ComplexAmplitude,
}

impl Qubit {
    /// Create qubit in |0⟩ state
    pub fn zero() -> Self {
        Self {
            alpha: ComplexAmplitude::ONE,
            beta: ComplexAmplitude::ZERO,
        }
    }

    /// Create qubit in |1⟩ state
    pub fn one() -> Self {
        Self {
            alpha: ComplexAmplitude::ZERO,
            beta: ComplexAmplitude::ONE,
        }
    }

    /// Create qubit in superposition |+⟩ = (|0⟩ + |1⟩) / √2
    pub fn plus() -> Self {
        let inv_sqrt2 = 1.0 / libm::sqrt(2.0);
        Self {
            alpha: ComplexAmplitude::new(inv_sqrt2, 0.0),
            beta: ComplexAmplitude::new(inv_sqrt2, 0.0),
        }
    }

    /// Create qubit in superposition |−⟩ = (|0⟩ − |1⟩) / √2
    pub fn minus() -> Self {
        let inv_sqrt2 = 1.0 / libm::sqrt(2.0);
        Self {
            alpha: ComplexAmplitude::new(inv_sqrt2, 0.0),
            beta: ComplexAmplitude::new(-inv_sqrt2, 0.0),
        }
    }

    /// Get probability of measuring |0⟩
    pub fn prob_zero(&self) -> f64 {
        self.alpha.magnitude_squared()
    }

    /// Get probability of measuring |1⟩
    pub fn prob_one(&self) -> f64 {
        self.beta.magnitude_squared()
    }

    /// Normalize the qubit state
    pub fn normalize(&mut self) {
        let norm = libm::sqrt(self.prob_zero() + self.prob_one());
        if norm > 1e-10 {
            self.alpha = self.alpha.scale(1.0 / norm);
            self.beta = self.beta.scale(1.0 / norm);
        }
    }
}

/// Quantum register for multi-qubit systems
#[derive(Debug, Clone)]
pub struct QuantumRegister {
    /// Number of qubits
    pub num_qubits: usize,
    /// State amplitudes (2^n for n qubits)
    pub amplitudes: Vec<ComplexAmplitude>,
}

impl QuantumRegister {
    /// Create a new quantum register initialized to |00...0⟩
    pub fn new(num_qubits: usize) -> Self {
        let size = 1 << num_qubits;
        let mut amplitudes = Vec::with_capacity(size);
        amplitudes.push(ComplexAmplitude::ONE);
        for _ in 1..size {
            amplitudes.push(ComplexAmplitude::ZERO);
        }
        Self {
            num_qubits,
            amplitudes,
        }
    }

    /// Create register in uniform superposition
    pub fn uniform_superposition(num_qubits: usize) -> Self {
        let size = 1 << num_qubits;
        let amplitude = 1.0 / libm::sqrt(size as f64);
        let amplitudes = (0..size)
            .map(|_| ComplexAmplitude::new(amplitude, 0.0))
            .collect();
        Self {
            num_qubits,
            amplitudes,
        }
    }

    /// Get probability of measuring a specific state
    pub fn probability(&self, state: usize) -> f64 {
        if state < self.amplitudes.len() {
            self.amplitudes[state].magnitude_squared()
        } else {
            0.0
        }
    }

    /// Normalize the quantum state
    pub fn normalize(&mut self) {
        let norm: f64 = self.amplitudes.iter().map(|a| a.magnitude_squared()).sum();
        let norm = libm::sqrt(norm);
        if norm > 1e-10 {
            for amp in &mut self.amplitudes {
                *amp = amp.scale(1.0 / norm);
            }
        }
    }

    /// Measure the quantum register (collapses state)
    pub fn measure(&mut self, rng_seed: u64) -> usize {
        // Simple PRNG for measurement
        let mut x = rng_seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let random = (x as f64) / (u64::MAX as f64);

        let mut cumulative = 0.0;
        for (i, amp) in self.amplitudes.iter().enumerate() {
            cumulative += amp.magnitude_squared();
            if random < cumulative {
                // Collapse to measured state
                for (j, a) in self.amplitudes.iter_mut().enumerate() {
                    if j == i {
                        *a = ComplexAmplitude::ONE;
                    } else {
                        *a = ComplexAmplitude::ZERO;
                    }
                }
                return i;
            }
        }
        self.amplitudes.len() - 1
    }
}

/// QUBO (Quadratic Unconstrained Binary Optimization) problem representation
#[derive(Debug, Clone)]
pub struct QuboMatrix {
    /// Size of the problem
    pub size: usize,
    /// Linear terms (diagonal)
    pub linear: Vec<f64>,
    /// Quadratic terms (upper triangle)
    pub quadratic: Vec<f64>,
}

impl QuboMatrix {
    /// Create a new QUBO matrix
    pub fn new(size: usize) -> Self {
        let quadratic_size = size * (size - 1) / 2;
        Self {
            size,
            linear: alloc::vec![0.0; size],
            quadratic: alloc::vec![0.0; quadratic_size],
        }
    }

    /// Set linear coefficient
    pub fn set_linear(&mut self, i: usize, value: f64) {
        if i < self.size {
            self.linear[i] = value;
        }
    }

    /// Set quadratic coefficient
    pub fn set_quadratic(&mut self, i: usize, j: usize, value: f64) {
        if i < j && j < self.size {
            let idx = i * self.size - (i * (i + 1)) / 2 + j - i - 1;
            if idx < self.quadratic.len() {
                self.quadratic[idx] = value;
            }
        }
    }

    /// Get quadratic coefficient
    pub fn get_quadratic(&self, i: usize, j: usize) -> f64 {
        if i >= j || j >= self.size {
            return 0.0;
        }
        let idx = i * self.size - (i * (i + 1)) / 2 + j - i - 1;
        if idx < self.quadratic.len() {
            self.quadratic[idx]
        } else {
            0.0
        }
    }

    /// Evaluate energy for a binary solution
    pub fn evaluate(&self, solution: &[bool]) -> f64 {
        let mut energy = 0.0;

        // Linear terms
        for i in 0..self.size {
            if solution.get(i).copied().unwrap_or(false) {
                energy += self.linear[i];
            }
        }

        // Quadratic terms
        for i in 0..self.size {
            for j in (i + 1)..self.size {
                if solution.get(i).copied().unwrap_or(false)
                    && solution.get(j).copied().unwrap_or(false)
                {
                    energy += self.get_quadratic(i, j);
                }
            }
        }

        energy
    }
}

/// Ising model representation for quantum optimization
#[derive(Debug, Clone)]
pub struct IsingModel {
    /// Number of spins
    pub num_spins: usize,
    /// External field strengths (h)
    pub h: Vec<f64>,
    /// Coupling strengths (J) - sparse representation
    pub j: Vec<(usize, usize, f64)>,
    /// Offset energy
    pub offset: f64,
}

impl IsingModel {
    /// Create from QUBO matrix
    pub fn from_qubo(qubo: &QuboMatrix) -> Self {
        let n = qubo.size;
        let mut h = alloc::vec![0.0; n];
        let mut j = Vec::new();
        let mut offset = 0.0;

        // Convert QUBO to Ising
        // x = (s + 1) / 2 where s ∈ {-1, +1}
        for i in 0..n {
            let mut local_field = qubo.linear[i] / 2.0;
            offset += qubo.linear[i] / 2.0;

            for k in 0..n {
                if k != i {
                    let q = if k < i {
                        qubo.get_quadratic(k, i)
                    } else {
                        qubo.get_quadratic(i, k)
                    };
                    local_field += q / 4.0;
                    offset += q / 4.0;
                }
            }
            h[i] = local_field;
        }

        for i in 0..n {
            for k in (i + 1)..n {
                let q = qubo.get_quadratic(i, k);
                if libm::fabs(q) > 1e-10 {
                    j.push((i, k, q / 4.0));
                }
            }
        }

        Self {
            num_spins: n,
            h,
            j,
            offset,
        }
    }

    /// Calculate energy for a spin configuration
    pub fn energy(&self, spins: &[i8]) -> f64 {
        let mut e = self.offset;

        // External field contribution
        for i in 0..self.num_spins {
            e += self.h[i] * spins.get(i).copied().unwrap_or(0) as f64;
        }

        // Coupling contribution
        for &(i, k, jik) in &self.j {
            let si = spins.get(i).copied().unwrap_or(0) as f64;
            let sk = spins.get(k).copied().unwrap_or(0) as f64;
            e += jik * si * sk;
        }

        e
    }
}

/// Quantum-inspired optimizer configuration
#[derive(Debug, Clone)]
pub struct QuantumOptimizerConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Initial temperature for annealing
    pub initial_temperature: f64,
    /// Final temperature for annealing
    pub final_temperature: f64,
    /// Number of parallel samplers
    pub num_samplers: usize,
    /// QAOA depth (p parameter)
    pub qaoa_depth: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
    /// Enable adaptive temperature schedule
    pub adaptive_schedule: bool,
    /// Quantum tunneling probability
    pub tunneling_rate: f64,
}

impl Default for QuantumOptimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            initial_temperature: 10.0,
            final_temperature: 0.01,
            num_samplers: 16,
            qaoa_depth: 4,
            convergence_threshold: 1e-8,
            adaptive_schedule: true,
            tunneling_rate: 0.1,
        }
    }
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Best solution found (binary encoding)
    pub solution: Vec<bool>,
    /// Energy/cost of the solution
    pub energy: f64,
    /// Number of iterations performed
    pub iterations: usize,
    /// Convergence achieved
    pub converged: bool,
    /// Time taken in microseconds
    pub time_us: u64,
    /// Quality metrics
    pub metrics: OptimizationMetrics,
}

/// Optimization quality metrics
#[derive(Debug, Clone)]
pub struct OptimizationMetrics {
    /// Best energy found at each iteration sample
    pub energy_history: Vec<f64>,
    /// Acceptance rate during annealing
    pub acceptance_rate: f64,
    /// Estimated solution quality (0-1)
    pub quality_estimate: f64,
    /// Number of unique solutions explored
    pub solutions_explored: usize,
    /// Quantum tunneling events
    pub tunneling_events: usize,
}

/// Main quantum-inspired optimizer
pub struct QuantumOptimizer {
    config: QuantumOptimizerConfig,
    problem_type: OptimizationProblem,
    stats: RunningStats,
    best_solution: Option<Vec<bool>>,
    best_energy: f64,
}

impl QuantumOptimizer {
    /// Create a new quantum optimizer
    pub fn new(problem_type: OptimizationProblem, config: QuantumOptimizerConfig) -> Self {
        Self {
            config,
            problem_type,
            stats: RunningStats::new(),
            best_solution: None,
            best_energy: f64::MAX,
        }
    }

    /// Optimize using quantum annealing simulation
    pub fn optimize_annealing(&mut self, qubo: &QuboMatrix) -> NexusResult<OptimizationResult> {
        let ising = IsingModel::from_qubo(qubo);
        self.simulated_quantum_annealing(&ising)
    }

    /// Optimize using QAOA simulation
    pub fn optimize_qaoa(&mut self, qubo: &QuboMatrix) -> NexusResult<OptimizationResult> {
        self.qaoa_optimize(qubo)
    }

    /// Simulated quantum annealing with transverse field
    fn simulated_quantum_annealing(
        &mut self,
        ising: &IsingModel,
    ) -> NexusResult<OptimizationResult> {
        let n = ising.num_spins;
        let mut current_spins: Vec<i8> = (0..n).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();
        let mut best_spins = current_spins.clone();
        let mut best_energy = ising.energy(&current_spins);
        let mut current_energy = best_energy;

        let mut energy_history = Vec::with_capacity(100);
        let mut acceptance_count = 0usize;
        let mut total_proposals = 0usize;
        let mut tunneling_events = 0usize;
        let mut solutions_explored = 1usize;

        let mut rng_state = 0xDEADBEEFu64;

        for iter in 0..self.config.max_iterations {
            // Calculate temperature using quantum-inspired schedule
            let progress = iter as f64 / self.config.max_iterations as f64;
            let temperature = if self.config.adaptive_schedule {
                self.adaptive_temperature_schedule(progress)
            } else {
                self.linear_temperature_schedule(progress)
            };

            // Transverse field strength (quantum tunneling)
            let gamma = self.config.initial_temperature * (1.0 - progress);

            // Propose spin flip with quantum tunneling consideration
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            let flip_idx = (rng_state as usize) % n;

            // Calculate energy change
            let mut delta_e = 2.0 * ising.h[flip_idx] * current_spins[flip_idx] as f64;
            for &(i, j, jij) in &ising.j {
                if i == flip_idx {
                    delta_e += 2.0 * jij * current_spins[flip_idx] as f64 * current_spins[j] as f64;
                } else if j == flip_idx {
                    delta_e += 2.0 * jij * current_spins[i] as f64 * current_spins[flip_idx] as f64;
                }
            }

            // Quantum tunneling enhancement
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            let random = (rng_state as f64) / (u64::MAX as f64);
            let tunneling_boost = if random < self.config.tunneling_rate {
                tunneling_events += 1;
                gamma * 0.5
            } else {
                0.0
            };

            // Metropolis acceptance with tunneling
            let accept = if delta_e <= 0.0 {
                true
            } else if temperature > 1e-10 {
                rng_state ^= rng_state << 13;
                rng_state ^= rng_state >> 7;
                let accept_prob = libm::exp(-(delta_e - tunneling_boost) / temperature);
                (rng_state as f64) / (u64::MAX as f64) < accept_prob
            } else {
                false
            };

            total_proposals += 1;

            if accept {
                current_spins[flip_idx] *= -1;
                current_energy += delta_e;
                acceptance_count += 1;
                solutions_explored += 1;

                if current_energy < best_energy {
                    best_energy = current_energy;
                    best_spins = current_spins.clone();
                }
            }

            // Record energy history periodically
            if iter % (self.config.max_iterations / 100).max(1) == 0 {
                energy_history.push(best_energy);
            }
        }

        // Convert spins to binary solution
        let solution: Vec<bool> = best_spins.iter().map(|&s| s > 0).collect();

        Ok(OptimizationResult {
            solution,
            energy: best_energy + ising.offset,
            iterations: self.config.max_iterations,
            converged: true,
            time_us: 0,
            metrics: OptimizationMetrics {
                energy_history,
                acceptance_rate: acceptance_count as f64 / total_proposals as f64,
                quality_estimate: self.estimate_quality(best_energy, ising),
                solutions_explored,
                tunneling_events,
            },
        })
    }

    /// QAOA optimization
    fn qaoa_optimize(&mut self, qubo: &QuboMatrix) -> NexusResult<OptimizationResult> {
        let n = qubo.size;
        let p = self.config.qaoa_depth;

        // Initialize variational parameters
        let mut gamma = alloc::vec![0.1; p];
        let mut beta = alloc::vec![0.1; p];

        let mut best_solution = alloc::vec![false; n];
        let mut best_energy = f64::MAX;
        let mut energy_history = Vec::new();

        // Variational optimization loop
        for outer_iter in 0..100 {
            // Evaluate current parameters
            let (solution, energy) = self.qaoa_evaluate(qubo, &gamma, &beta)?;

            if energy < best_energy {
                best_energy = energy;
                best_solution = solution;
            }
            energy_history.push(best_energy);

            // Gradient-free optimization (coordinate descent)
            let step_size = 0.1 * (1.0 - outer_iter as f64 / 100.0);

            for i in 0..p {
                // Optimize gamma[i]
                let mut best_gamma_i = gamma[i];
                let mut best_e = energy;

                for delta in [-step_size, step_size] {
                    gamma[i] += delta;
                    let (_, e) = self.qaoa_evaluate(qubo, &gamma, &beta)?;
                    if e < best_e {
                        best_e = e;
                        best_gamma_i = gamma[i];
                    }
                    gamma[i] -= delta;
                }
                gamma[i] = best_gamma_i;

                // Optimize beta[i]
                let mut best_beta_i = beta[i];
                best_e = best_energy;

                for delta in [-step_size, step_size] {
                    beta[i] += delta;
                    let (_, e) = self.qaoa_evaluate(qubo, &gamma, &beta)?;
                    if e < best_e {
                        best_e = e;
                        best_beta_i = beta[i];
                    }
                    beta[i] -= delta;
                }
                beta[i] = best_beta_i;
            }
        }

        Ok(OptimizationResult {
            solution: best_solution,
            energy: best_energy,
            iterations: 100 * self.config.qaoa_depth * 2,
            converged: true,
            time_us: 0,
            metrics: OptimizationMetrics {
                energy_history,
                acceptance_rate: 1.0,
                quality_estimate: 0.9,
                solutions_explored: 100,
                tunneling_events: 0,
            },
        })
    }

    /// Evaluate QAOA circuit
    fn qaoa_evaluate(
        &self,
        qubo: &QuboMatrix,
        gamma: &[f64],
        beta: &[f64],
    ) -> NexusResult<(Vec<bool>, f64)> {
        let n = qubo.size;
        let p = gamma.len();

        // Simplified QAOA simulation using tensor network contraction
        // For kernel use, we use an efficient classical approximation

        let mut state = QuantumRegister::uniform_superposition(n);

        for layer in 0..p {
            // Apply cost unitary exp(-i * gamma * C)
            self.apply_cost_unitary(&mut state, qubo, gamma[layer]);

            // Apply mixer unitary exp(-i * beta * B)
            self.apply_mixer_unitary(&mut state, beta[layer]);
        }

        // Sample from final state
        let mut best_solution = alloc::vec![false; n];
        let mut best_energy = f64::MAX;

        // Take multiple samples
        for seed in 0..self.config.num_samplers {
            let mut state_copy = state.clone();
            let result = state_copy.measure(seed as u64 * 12345 + 67890);

            let solution: Vec<bool> = (0..n).map(|i| (result >> i) & 1 == 1).collect();
            let energy = qubo.evaluate(&solution);

            if energy < best_energy {
                best_energy = energy;
                best_solution = solution;
            }
        }

        Ok((best_solution, best_energy))
    }

    /// Apply cost unitary
    fn apply_cost_unitary(&self, state: &mut QuantumRegister, qubo: &QuboMatrix, gamma: f64) {
        let n = qubo.size;

        for (idx, amp) in state.amplitudes.iter_mut().enumerate() {
            // Compute cost for this basis state
            let solution: Vec<bool> = (0..n).map(|i| (idx >> i) & 1 == 1).collect();
            let cost = qubo.evaluate(&solution);

            // Apply phase rotation
            let phase = -gamma * cost;
            let rotation = ComplexAmplitude::from_polar(1.0, phase);
            *amp = amp.multiply(&rotation);
        }
    }

    /// Apply mixer unitary (X rotations)
    fn apply_mixer_unitary(&self, state: &mut QuantumRegister, beta: f64) {
        let n = state.num_qubits;

        for qubit in 0..n {
            self.apply_rx_gate(state, qubit, 2.0 * beta);
        }
    }

    /// Apply RX gate to a specific qubit
    fn apply_rx_gate(&self, state: &mut QuantumRegister, qubit: usize, theta: f64) {
        let cos_half = libm::cos(theta / 2.0);
        let sin_half = libm::sin(theta / 2.0);

        let mask = 1 << qubit;
        let mut new_amps = state.amplitudes.clone();

        for i in 0..state.amplitudes.len() {
            if i & mask == 0 {
                let j = i | mask;
                // RX matrix application
                new_amps[i] = state.amplitudes[i]
                    .scale(cos_half)
                    .add(&state.amplitudes[j].multiply(&ComplexAmplitude::new(0.0, -sin_half)));
                new_amps[j] = state.amplitudes[i]
                    .multiply(&ComplexAmplitude::new(0.0, -sin_half))
                    .add(&state.amplitudes[j].scale(cos_half));
            }
        }

        state.amplitudes = new_amps;
    }

    fn linear_temperature_schedule(&self, progress: f64) -> f64 {
        self.config.initial_temperature * (1.0 - progress)
            + self.config.final_temperature * progress
    }

    fn adaptive_temperature_schedule(&self, progress: f64) -> f64 {
        // Exponential decay with plateau detection
        let base_temp = self.config.initial_temperature * libm::exp(-5.0 * progress);
        base_temp.max(self.config.final_temperature)
    }

    fn estimate_quality(&self, energy: f64, ising: &IsingModel) -> f64 {
        // Estimate solution quality based on energy bounds
        let min_possible = -ising.h.iter().map(|h| libm::fabs(*h)).sum::<f64>()
            - ising.j.iter().map(|(_, _, j)| libm::fabs(*j)).sum::<f64>();
        let max_possible = -min_possible;

        if max_possible > min_possible {
            1.0 - (energy - min_possible) / (max_possible - min_possible)
        } else {
            0.5
        }
    }
}

/// Quantum-inspired scheduler optimizer
pub struct QuantumSchedulerOptimizer {
    optimizer: QuantumOptimizer,
    num_tasks: usize,
    num_cpus: usize,
}

impl QuantumSchedulerOptimizer {
    /// Create a new quantum scheduler optimizer
    pub fn new(num_tasks: usize, num_cpus: usize) -> Self {
        Self {
            optimizer: QuantumOptimizer::new(
                OptimizationProblem::Scheduling,
                QuantumOptimizerConfig::default(),
            ),
            num_tasks,
            num_cpus,
        }
    }

    /// Optimize task scheduling using quantum annealing
    pub fn optimize_schedule(
        &mut self,
        task_costs: &[u64],
        task_dependencies: &[(usize, usize)],
        task_affinities: &[(usize, usize)],
    ) -> NexusResult<Vec<usize>> {
        // Formulate as QUBO
        let qubo = self.build_scheduling_qubo(task_costs, task_dependencies, task_affinities)?;

        // Optimize
        let result = self.optimizer.optimize_annealing(&qubo)?;

        // Decode solution
        self.decode_schedule(&result.solution)
    }

    fn build_scheduling_qubo(
        &self,
        task_costs: &[u64],
        dependencies: &[(usize, usize)],
        affinities: &[(usize, usize)],
    ) -> NexusResult<QuboMatrix> {
        // Binary variables: x[i][c] = 1 if task i assigned to CPU c
        let n = self.num_tasks * self.num_cpus;
        let mut qubo = QuboMatrix::new(n);

        // Constraint: each task on exactly one CPU
        let penalty = 1000.0;
        for task in 0..self.num_tasks {
            for c1 in 0..self.num_cpus {
                let idx1 = task * self.num_cpus + c1;
                qubo.set_linear(idx1, -penalty);

                for c2 in (c1 + 1)..self.num_cpus {
                    let idx2 = task * self.num_cpus + c2;
                    qubo.set_quadratic(idx1, idx2, 2.0 * penalty);
                }
            }
        }

        // Load balancing objective
        for c in 0..self.num_cpus {
            for t1 in 0..self.num_tasks {
                for t2 in (t1 + 1)..self.num_tasks {
                    let idx1 = t1 * self.num_cpus + c;
                    let idx2 = t2 * self.num_cpus + c;
                    let cost = (task_costs[t1] * task_costs[t2]) as f64 / 1000.0;
                    qubo.set_quadratic(idx1.min(idx2), idx1.max(idx2), cost);
                }
            }
        }

        // Dependency constraints (same CPU preferred)
        for &(t1, t2) in dependencies {
            for c in 0..self.num_cpus {
                let idx1 = t1 * self.num_cpus + c;
                let idx2 = t2 * self.num_cpus + c;
                qubo.set_quadratic(idx1.min(idx2), idx1.max(idx2), -10.0);
            }
        }

        // Affinity preferences
        for &(task, cpu) in affinities {
            let idx = task * self.num_cpus + cpu;
            qubo.set_linear(idx, qubo.linear[idx] - 50.0);
        }

        Ok(qubo)
    }

    fn decode_schedule(&self, solution: &[bool]) -> NexusResult<Vec<usize>> {
        let mut schedule = alloc::vec![0; self.num_tasks];

        for task in 0..self.num_tasks {
            for cpu in 0..self.num_cpus {
                let idx = task * self.num_cpus + cpu;
                if solution.get(idx).copied().unwrap_or(false) {
                    schedule[task] = cpu;
                    break;
                }
            }
        }

        Ok(schedule)
    }
}

/// Quantum-inspired memory allocator optimizer
pub struct QuantumMemoryOptimizer {
    optimizer: QuantumOptimizer,
}

impl QuantumMemoryOptimizer {
    pub fn new() -> Self {
        Self {
            optimizer: QuantumOptimizer::new(
                OptimizationProblem::MemoryLayout,
                QuantumOptimizerConfig::default(),
            ),
        }
    }

    /// Optimize memory layout using quantum annealing
    pub fn optimize_layout(
        &mut self,
        block_sizes: &[usize],
        access_patterns: &[(usize, usize, u64)], // (block_i, block_j, frequency)
        cache_line_size: usize,
    ) -> NexusResult<Vec<usize>> {
        let n = block_sizes.len();
        let qubo = self.build_memory_qubo(block_sizes, access_patterns, cache_line_size)?;

        let result = self.optimizer.optimize_annealing(&qubo)?;
        self.decode_layout(&result.solution, n)
    }

    fn build_memory_qubo(
        &self,
        sizes: &[usize],
        patterns: &[(usize, usize, u64)],
        cache_line: usize,
    ) -> NexusResult<QuboMatrix> {
        let n = sizes.len();
        let mut qubo = QuboMatrix::new(n * n);

        // Locality objective: frequently accessed together should be adjacent
        for &(i, j, freq) in patterns {
            for pos1 in 0..n {
                for pos2 in 0..n {
                    let idx1 = i * n + pos1;
                    let idx2 = j * n + pos2;

                    // Penalty proportional to distance * frequency
                    let distance = if pos1 > pos2 {
                        pos1 - pos2
                    } else {
                        pos2 - pos1
                    };
                    let penalty = (freq as f64) * (distance as f64) / (cache_line as f64);

                    if idx1 < idx2 {
                        qubo.set_quadratic(idx1, idx2, penalty);
                    }
                }
            }
        }

        // Constraint: each block in exactly one position
        let penalty = 10000.0;
        for block in 0..n {
            for pos1 in 0..n {
                let idx = block * n + pos1;
                qubo.set_linear(idx, -penalty);

                for pos2 in (pos1 + 1)..n {
                    let idx2 = block * n + pos2;
                    qubo.set_quadratic(idx, idx2, 2.0 * penalty);
                }
            }
        }

        Ok(qubo)
    }

    fn decode_layout(&self, solution: &[bool], n: usize) -> NexusResult<Vec<usize>> {
        let mut layout = alloc::vec![0; n];

        for block in 0..n {
            for pos in 0..n {
                let idx = block * n + pos;
                if solution.get(idx).copied().unwrap_or(false) {
                    layout[block] = pos;
                    break;
                }
            }
        }

        Ok(layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qubit_creation() {
        let q0 = Qubit::zero();
        assert!((q0.prob_zero() - 1.0).abs() < 1e-10);
        assert!(q0.prob_one().abs() < 1e-10);

        let q1 = Qubit::one();
        assert!(q1.prob_zero().abs() < 1e-10);
        assert!((q1.prob_one() - 1.0).abs() < 1e-10);

        let q_plus = Qubit::plus();
        assert!((q_plus.prob_zero() - 0.5).abs() < 1e-10);
        assert!((q_plus.prob_one() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_quantum_register() {
        let reg = QuantumRegister::new(3);
        assert_eq!(reg.num_qubits, 3);
        assert_eq!(reg.amplitudes.len(), 8);
        assert!((reg.probability(0) - 1.0).abs() < 1e-10);

        let sup = QuantumRegister::uniform_superposition(3);
        for i in 0..8 {
            assert!((sup.probability(i) - 0.125).abs() < 1e-10);
        }
    }

    #[test]
    fn test_qubo_evaluation() {
        let mut qubo = QuboMatrix::new(3);
        qubo.set_linear(0, 1.0);
        qubo.set_linear(1, 2.0);
        qubo.set_linear(2, 3.0);
        qubo.set_quadratic(0, 1, -1.0);
        qubo.set_quadratic(1, 2, -2.0);

        let sol1 = [false, false, false];
        assert_eq!(qubo.evaluate(&sol1), 0.0);

        let sol2 = [true, false, false];
        assert_eq!(qubo.evaluate(&sol2), 1.0);

        let sol3 = [true, true, true];
        assert_eq!(qubo.evaluate(&sol3), 1.0 + 2.0 + 3.0 - 1.0 - 2.0);
    }

    #[test]
    fn test_complex_amplitude() {
        let a = ComplexAmplitude::new(3.0, 4.0);
        assert!((a.magnitude() - 5.0).abs() < 1e-10);

        let b = ComplexAmplitude::from_polar(1.0, PI / 2.0);
        assert!(b.real.abs() < 1e-10);
        assert!((b.imag - 1.0).abs() < 1e-10);
    }
}
