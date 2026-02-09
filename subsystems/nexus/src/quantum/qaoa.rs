//! # QAOA - Quantum Approximate Optimization Algorithm
//!
//! Implementation of QAOA for combinatorial optimization.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::gates::{apply_cnot, apply_h, apply_rx, apply_rz};
use super::types::{Hamiltonian, Pauli, PauliString, StateVector};

// ============================================================================
// QAOA PARAMETERS
// ============================================================================

/// QAOA parameters (γ, β angles)
#[derive(Debug, Clone)]
pub struct QaoaParameters {
    /// Mixer angles (β)
    pub betas: Vec<f64>,
    /// Cost angles (γ)
    pub gammas: Vec<f64>,
}

impl QaoaParameters {
    /// Create new parameters for p layers
    pub fn new(p: usize) -> Self {
        Self {
            betas: vec![0.5; p],
            gammas: vec![0.5; p],
        }
    }

    /// Create with initial values
    #[inline]
    pub fn with_values(betas: Vec<f64>, gammas: Vec<f64>) -> Option<Self> {
        if betas.len() != gammas.len() {
            return None;
        }
        Some(Self { betas, gammas })
    }

    /// Number of QAOA layers
    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.betas.len()
    }

    /// Total number of parameters
    #[inline(always)]
    pub fn num_params(&self) -> usize {
        self.betas.len() + self.gammas.len()
    }

    /// Get all parameters as vector
    #[inline]
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = self.gammas.clone();
        v.extend(self.betas.iter());
        v
    }

    /// Set from vector
    #[inline]
    pub fn from_vec(&mut self, params: &[f64]) {
        let p = self.depth();
        if params.len() >= 2 * p {
            self.gammas[..p].copy_from_slice(&params[..p]);
            self.betas[..p].copy_from_slice(&params[p..2 * p]);
        }
    }
}

// ============================================================================
// QAOA PROBLEM
// ============================================================================

/// QAOA problem definition
#[derive(Debug, Clone)]
pub struct QaoaProblem {
    /// Number of qubits
    pub n_qubits: usize,
    /// Cost Hamiltonian
    pub cost_hamiltonian: Hamiltonian,
}

impl QaoaProblem {
    /// Create QAOA problem from cost Hamiltonian
    pub fn new(n_qubits: usize, cost_hamiltonian: Hamiltonian) -> Self {
        Self {
            n_qubits,
            cost_hamiltonian,
        }
    }

    /// Create MaxCut problem
    /// H_C = Σ_{(u,v)∈E} 0.5 * (1 - Z_u * Z_v)
    pub fn maxcut(edges: &[(usize, usize)]) -> Self {
        let mut n_qubits = 0;
        for &(u, v) in edges {
            n_qubits = n_qubits.max(u + 1).max(v + 1);
        }

        let mut hamiltonian = Hamiltonian::new(n_qubits);

        for &(u, v) in edges {
            // Edge contribution: -0.5 * Z_u * Z_v (constant absorbed)
            hamiltonian.add_term(PauliString::zz(n_qubits, u, v, -0.5));
        }

        Self::new(n_qubits, hamiltonian)
    }

    /// Create weighted MaxCut problem
    pub fn weighted_maxcut(edges: &[(usize, usize, f64)]) -> Self {
        let mut n_qubits = 0;
        for &(u, v, _) in edges {
            n_qubits = n_qubits.max(u + 1).max(v + 1);
        }

        let mut hamiltonian = Hamiltonian::new(n_qubits);

        for &(u, v, w) in edges {
            hamiltonian.add_term(PauliString::zz(n_qubits, u, v, -0.5 * w));
        }

        Self::new(n_qubits, hamiltonian)
    }

    /// Create Ising problem from couplings and fields
    #[inline]
    pub fn ising(couplings: &[(usize, usize, f64)], fields: &[(usize, f64)]) -> Self {
        let hamiltonian = Hamiltonian::ising(couplings, fields);
        let n_qubits = hamiltonian.n_qubits;
        Self::new(n_qubits, hamiltonian)
    }

    /// Evaluate cost function for bitstring
    #[inline]
    pub fn evaluate(&self, bitstring: usize) -> f64 {
        let mut cost = 0.0;

        for term in &self.cost_hamiltonian.terms {
            cost += term.expectation_basis(bitstring);
        }

        cost
    }
}

// ============================================================================
// QAOA ENGINE
// ============================================================================

/// QAOA execution engine
pub struct QaoaEngine {
    /// Problem definition
    pub problem: QaoaProblem,
    /// Current parameters
    pub parameters: QaoaParameters,
    /// State vector
    state: StateVector,
}

impl QaoaEngine {
    /// Create new QAOA engine
    pub fn new(problem: QaoaProblem, depth: usize) -> Self {
        let n = problem.n_qubits;
        Self {
            problem,
            parameters: QaoaParameters::new(depth),
            state: StateVector::new(n),
        }
    }

    /// Initialize superposition state |+⟩^n
    fn initialize_plus(&mut self) {
        self.state = StateVector::new(self.problem.n_qubits);
        for i in 0..self.problem.n_qubits {
            apply_h(&mut self.state, i);
        }
    }

    /// Apply cost layer exp(-iγH_C)
    fn apply_cost_layer(&mut self, gamma: f64) {
        // For diagonal cost Hamiltonian (ZZ terms only)
        for term in &self.problem.cost_hamiltonian.terms {
            // Find qubits with Z operators
            let z_qubits: Vec<usize> = term
                .paulis
                .iter()
                .enumerate()
                .filter(|&(_, p)| *p == Pauli::Z)
                .map(|(i, _)| i)
                .collect();

            let angle = 2.0 * gamma * term.coeff;

            match z_qubits.len() {
                0 => {},
                1 => {
                    // Single Z: apply Rz
                    apply_rz(&mut self.state, z_qubits[0], angle);
                },
                2 => {
                    // ZZ term: apply CNOT-Rz-CNOT pattern
                    let q0 = z_qubits[0];
                    let q1 = z_qubits[1];
                    apply_cnot(&mut self.state, q0, q1);
                    apply_rz(&mut self.state, q1, angle);
                    apply_cnot(&mut self.state, q0, q1);
                },
                _ => {
                    // Multi-Z: generalized ladder pattern
                    for i in 0..z_qubits.len() - 1 {
                        apply_cnot(&mut self.state, z_qubits[i], z_qubits[i + 1]);
                    }
                    let last = z_qubits[z_qubits.len() - 1];
                    apply_rz(&mut self.state, last, angle);
                    for i in (0..z_qubits.len() - 1).rev() {
                        apply_cnot(&mut self.state, z_qubits[i], z_qubits[i + 1]);
                    }
                },
            }
        }
    }

    /// Apply mixer layer exp(-iβH_M) where H_M = Σ X_i
    fn apply_mixer_layer(&mut self, beta: f64) {
        let angle = 2.0 * beta;
        for i in 0..self.problem.n_qubits {
            apply_rx(&mut self.state, i, angle);
        }
    }

    /// Execute QAOA circuit
    #[inline]
    pub fn execute(&mut self) {
        self.initialize_plus();

        let p = self.parameters.depth();
        for layer in 0..p {
            self.apply_cost_layer(self.parameters.gammas[layer]);
            self.apply_mixer_layer(self.parameters.betas[layer]);
        }
    }

    /// Compute expectation value of cost Hamiltonian
    pub fn expectation_value(&self) -> f64 {
        let dim = self.state.dimension();
        let mut expectation = 0.0;

        for i in 0..dim {
            let prob = self.state.probability(i);
            let cost = self.problem.evaluate(i);
            expectation += prob * cost;
        }

        expectation
    }

    /// Run QAOA and return expectation value
    #[inline(always)]
    pub fn run(&mut self) -> f64 {
        self.execute();
        self.expectation_value()
    }

    /// Sample from output distribution
    pub fn sample(&self, n_samples: usize, seed: u64) -> Vec<(usize, usize)> {
        let dim = self.state.dimension();
        let mut counts = vec![0usize; dim];
        let mut rng = seed;

        for _ in 0..n_samples {
            // Simple PRNG (xorshift64)
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let rand = (rng as f64) / (u64::MAX as f64);

            let mut cumsum = 0.0;
            for (i, count) in counts.iter_mut().enumerate().take(dim) {
                cumsum += self.state.probability(i);
                if rand < cumsum {
                    *count += 1;
                    break;
                }
            }
        }

        // Return non-zero counts
        counts
            .into_iter()
            .enumerate()
            .filter(|(_, c)| *c > 0)
            .collect()
    }

    /// Find best bitstring from samples
    pub fn best_solution(&self, n_samples: usize, seed: u64) -> (usize, f64) {
        let samples = self.sample(n_samples, seed);

        let mut best_bitstring = 0;
        let mut best_cost = f64::NEG_INFINITY;

        for (bitstring, _) in samples {
            let cost = -self.problem.evaluate(bitstring); // Negate for maximization
            if cost > best_cost {
                best_cost = cost;
                best_bitstring = bitstring;
            }
        }

        (best_bitstring, best_cost)
    }

    /// Get current state
    #[inline(always)]
    pub fn state(&self) -> &StateVector {
        &self.state
    }
}

// ============================================================================
// QAOA OPTIMIZER
// ============================================================================

/// Simple gradient-free optimizer for QAOA
pub struct QaoaOptimizer {
    /// QAOA engine
    engine: QaoaEngine,
    /// Learning rate
    learning_rate: f64,
    /// Number of iterations
    max_iterations: usize,
}

impl QaoaOptimizer {
    /// Create new optimizer
    pub fn new(engine: QaoaEngine) -> Self {
        Self {
            engine,
            learning_rate: 0.1,
            max_iterations: 100,
        }
    }

    /// Set learning rate
    #[inline(always)]
    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.learning_rate = lr;
        self
    }

    /// Set max iterations
    #[inline(always)]
    pub fn with_max_iterations(mut self, iters: usize) -> Self {
        self.max_iterations = iters;
        self
    }

    /// Compute gradient numerically
    fn compute_gradient(&mut self) -> Vec<f64> {
        let eps = 0.01;
        let params = self.engine.parameters.to_vec();
        let mut gradient = vec![0.0; params.len()];

        for i in 0..params.len() {
            // Forward
            let mut params_plus = params.clone();
            params_plus[i] += eps;
            self.engine.parameters.from_vec(&params_plus);
            let f_plus = self.engine.run();

            // Backward
            let mut params_minus = params.clone();
            params_minus[i] -= eps;
            self.engine.parameters.from_vec(&params_minus);
            let f_minus = self.engine.run();

            gradient[i] = (f_plus - f_minus) / (2.0 * eps);
        }

        // Restore original parameters
        self.engine.parameters.from_vec(&params);

        gradient
    }

    /// Run optimization
    pub fn optimize(&mut self) -> QaoaResult {
        let mut best_params = self.engine.parameters.to_vec();
        let mut best_energy = f64::MAX;
        let mut history = Vec::new();

        for _ in 0..self.max_iterations {
            let gradient = self.compute_gradient();

            // Update parameters (gradient descent, minimize energy)
            let mut params = self.engine.parameters.to_vec();
            for i in 0..params.len() {
                params[i] -= self.learning_rate * gradient[i];
            }
            self.engine.parameters.from_vec(&params);

            // Evaluate
            let energy = self.engine.run();
            history.push(energy);

            if energy < best_energy {
                best_energy = energy;
                best_params = params;
            }
        }

        // Set best parameters
        self.engine.parameters.from_vec(&best_params);

        QaoaResult {
            optimal_parameters: self.engine.parameters.clone(),
            optimal_energy: best_energy,
            history,
        }
    }

    /// Get inner engine
    #[inline(always)]
    pub fn into_engine(self) -> QaoaEngine {
        self.engine
    }
}

/// QAOA optimization result
#[derive(Debug, Clone)]
pub struct QaoaResult {
    /// Optimal parameters found
    pub optimal_parameters: QaoaParameters,
    /// Optimal energy
    pub optimal_energy: f64,
    /// Energy history
    pub history: Vec<f64>,
}

// ============================================================================
// WARM START
// ============================================================================

/// Warm-start strategy for QAOA
pub enum WarmStartStrategy {
    /// Start from uniform superposition
    Uniform,
    /// Start from classical solution
    Classical(usize),
    /// Start from previous QAOA solution
    Transfer(QaoaParameters),
}

/// Apply warm start to QAOA engine
pub fn apply_warm_start(engine: &mut QaoaEngine, strategy: WarmStartStrategy) {
    match strategy {
        WarmStartStrategy::Uniform => {
            // Default initialization
        },
        WarmStartStrategy::Classical(bitstring) => {
            // Could initialize state closer to classical solution
            // For now, just use as hint for parameter initialization
            let _ = bitstring;
        },
        WarmStartStrategy::Transfer(params) => {
            // Transfer parameters from previous run
            if params.depth() <= engine.parameters.depth() {
                for i in 0..params.depth() {
                    engine.parameters.gammas[i] = params.gammas[i];
                    engine.parameters.betas[i] = params.betas[i];
                }
            }
        },
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qaoa_parameters() {
        let params = QaoaParameters::new(3);
        assert_eq!(params.depth(), 3);
        assert_eq!(params.num_params(), 6);
    }

    #[test]
    fn test_maxcut_problem() {
        // Triangle graph
        let edges = [(0, 1), (1, 2), (0, 2)];
        let problem = QaoaProblem::maxcut(&edges);

        assert_eq!(problem.n_qubits, 3);

        // Optimal MaxCut for triangle is 2
        // Bitstrings 001, 010, 100, 011, 101, 110 all give cut = 2
        let cost_001 = -problem.evaluate(0b001); // Negate because we minimize ZZ
        let cost_000 = -problem.evaluate(0b000);

        // 001 should have better cut than 000
        assert!(cost_001 > cost_000);
    }

    #[test]
    fn test_qaoa_engine() {
        let edges = [(0, 1)];
        let problem = QaoaProblem::maxcut(&edges);

        let mut engine = QaoaEngine::new(problem, 1);
        engine.parameters.gammas[0] = core::f64::consts::PI / 4.0;
        engine.parameters.betas[0] = core::f64::consts::PI / 4.0;

        let energy = engine.run();

        // Energy should be finite
        assert!(energy.is_finite());
    }

    #[test]
    fn test_qaoa_sampling() {
        let edges = [(0, 1)];
        let problem = QaoaProblem::maxcut(&edges);

        let mut engine = QaoaEngine::new(problem, 1);
        engine.execute();

        let samples = engine.sample(100, 12345);

        // Should have some samples
        assert!(!samples.is_empty());

        // Total count should be 100
        let total: usize = samples.iter().map(|(_, c)| c).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn test_ising_problem() {
        // Simple 2-qubit Ising: J_01 = 1.0, h_0 = 0.5
        let couplings = [(0, 1, 1.0)];
        let fields = [(0, 0.5)];

        let problem = QaoaProblem::ising(&couplings, &fields);
        assert_eq!(problem.n_qubits, 2);
    }

    #[test]
    fn test_weighted_maxcut() {
        let edges = [(0, 1, 2.0), (1, 2, 1.0)];
        let problem = QaoaProblem::weighted_maxcut(&edges);

        assert_eq!(problem.n_qubits, 3);

        // The weighted edge should contribute more
        let cost = problem.evaluate(0b001);
        assert!(cost.is_finite());
    }
}
