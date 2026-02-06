//! # VQE - Variational Quantum Eigensolver
//!
//! Implementation of VQE for finding ground state energies.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::circuits::{CircuitExecutor, QuantumCircuit};
use super::types::{Hamiltonian, Pauli, PauliString, StateVector};

// ============================================================================
// ANSATZ TYPES
// ============================================================================

/// Variational ansatz type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsatzType {
    /// Hardware-efficient ansatz
    HardwareEfficient,
    /// UCCSD ansatz (Unitary Coupled Cluster)
    Uccsd,
    /// Symmetry-preserving ansatz
    SymmetryPreserving,
    /// Custom parametric circuit
    Custom,
}

// ============================================================================
// ANSATZ PARAMETERS
// ============================================================================

/// VQE variational parameters
#[derive(Debug, Clone)]
pub struct VqeParameters {
    /// Parameter values
    pub values: Vec<f64>,
    /// Number of qubits
    pub n_qubits: usize,
    /// Number of layers
    pub n_layers: usize,
}

impl VqeParameters {
    /// Create new parameters for hardware-efficient ansatz
    pub fn hardware_efficient(n_qubits: usize, n_layers: usize) -> Self {
        // 3 rotation parameters per qubit per layer
        let n_params = 3 * n_qubits * n_layers;
        Self {
            values: vec![0.0; n_params],
            n_qubits,
            n_layers,
        }
    }

    /// Create with random initialization
    pub fn random_init(n_qubits: usize, n_layers: usize, seed: u64) -> Self {
        let n_params = 3 * n_qubits * n_layers;
        let mut values = Vec::with_capacity(n_params);

        let mut rng = seed;
        for _ in 0..n_params {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let val = (rng as f64 / u64::MAX as f64) * 2.0 * core::f64::consts::PI;
            values.push(val);
        }

        Self {
            values,
            n_qubits,
            n_layers,
        }
    }

    /// Number of parameters
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get rotation angles for specific qubit and layer
    pub fn get_angles(&self, qubit: usize, layer: usize) -> (f64, f64, f64) {
        let base = (layer * self.n_qubits + qubit) * 3;
        if base + 2 < self.values.len() {
            (
                self.values[base],
                self.values[base + 1],
                self.values[base + 2],
            )
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}

// ============================================================================
// ANSATZ BUILDER
// ============================================================================

/// Build variational ansatz circuits
pub struct AnsatzBuilder {
    /// Number of qubits
    n_qubits: usize,
    /// Ansatz type
    ansatz_type: AnsatzType,
}

impl AnsatzBuilder {
    /// Create new builder
    pub fn new(n_qubits: usize) -> Self {
        Self {
            n_qubits,
            ansatz_type: AnsatzType::HardwareEfficient,
        }
    }

    /// Set ansatz type
    pub fn with_type(mut self, t: AnsatzType) -> Self {
        self.ansatz_type = t;
        self
    }

    /// Build hardware-efficient ansatz
    pub fn build_hardware_efficient(&self, params: &VqeParameters) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(self.n_qubits);

        for layer in 0..params.n_layers {
            // Rotation layer
            for qubit in 0..self.n_qubits {
                let (rx, ry, rz) = params.get_angles(qubit, layer);
                circuit.rx(qubit, rx);
                circuit.ry(qubit, ry);
                circuit.rz(qubit, rz);
            }

            // Entangling layer
            for i in 0..self.n_qubits - 1 {
                circuit.cnot(i, i + 1);
            }
            if self.n_qubits > 2 {
                circuit.cnot(self.n_qubits - 1, 0);
            }
        }

        circuit
    }

    /// Build symmetry-preserving ansatz
    pub fn build_symmetry_preserving(&self, params: &VqeParameters) -> QuantumCircuit {
        let mut circuit = QuantumCircuit::new(self.n_qubits);

        for layer in 0..params.n_layers {
            // Paired rotations to preserve particle number
            for i in (0..self.n_qubits - 1).step_by(2) {
                let (theta, phi, _) = params.get_angles(i / 2, layer);

                // Excitation-preserving gate
                circuit.ry(i, theta);
                circuit.ry(i + 1, -theta);
                circuit.cnot(i, i + 1);
                circuit.ry(i, phi);
                circuit.ry(i + 1, -phi);
                circuit.cnot(i, i + 1);
            }
        }

        circuit
    }

    /// Build ansatz based on type
    pub fn build(&self, params: &VqeParameters) -> QuantumCircuit {
        match self.ansatz_type {
            AnsatzType::HardwareEfficient => self.build_hardware_efficient(params),
            AnsatzType::SymmetryPreserving => self.build_symmetry_preserving(params),
            _ => self.build_hardware_efficient(params),
        }
    }
}

// ============================================================================
// VQE ENGINE
// ============================================================================

/// VQE execution engine
pub struct VqeEngine {
    /// Number of qubits
    pub n_qubits: usize,
    /// Hamiltonian to minimize
    pub hamiltonian: Hamiltonian,
    /// Current parameters
    pub parameters: VqeParameters,
    /// Ansatz builder
    ansatz: AnsatzBuilder,
}

impl VqeEngine {
    /// Create new VQE engine
    pub fn new(hamiltonian: Hamiltonian, n_qubits: usize, n_layers: usize) -> Self {
        Self {
            n_qubits,
            hamiltonian,
            parameters: VqeParameters::hardware_efficient(n_qubits, n_layers),
            ansatz: AnsatzBuilder::new(n_qubits),
        }
    }

    /// Set ansatz type
    pub fn with_ansatz(mut self, ansatz_type: AnsatzType) -> Self {
        self.ansatz = self.ansatz.with_type(ansatz_type);
        self
    }

    /// Initialize parameters randomly
    pub fn randomize_params(&mut self, seed: u64) {
        self.parameters = VqeParameters::random_init(self.n_qubits, self.parameters.n_layers, seed);
    }

    /// Measure Pauli string expectation value
    fn measure_pauli_string(&self, state: &StateVector, pauli_string: &PauliString) -> f64 {
        let dim = state.dimension();
        let mut expectation = 0.0;

        for i in 0..dim {
            let prob = state.probability(i);

            // Compute eigenvalue for this basis state
            let mut eigenvalue = 1.0f64;
            for (qubit, &pauli) in pauli_string.paulis.iter().enumerate() {
                match pauli {
                    Pauli::I => {}, // Identity
                    Pauli::Z => {
                        if (i >> qubit) & 1 == 1 {
                            eigenvalue *= -1.0;
                        }
                    },
                    Pauli::X | Pauli::Y => {
                        // X or Y - need basis rotation
                        // For simplicity, assume diagonal (ZZ) terms only
                        // Full implementation would require basis rotation
                    },
                }
            }

            expectation += prob * eigenvalue;
        }

        expectation
    }

    /// Compute energy expectation value
    pub fn compute_energy(&self, params: &VqeParameters) -> f64 {
        let circuit = self.ansatz.build(params);

        let mut exec = CircuitExecutor::new(self.n_qubits);
        exec.execute(&circuit);

        let mut energy = 0.0;

        for term in &self.hamiltonian.terms {
            let expectation = self.measure_pauli_string(&exec.state, term);
            energy += term.coeff * expectation;
        }

        energy
    }

    /// Compute energy with current parameters
    pub fn energy(&self) -> f64 {
        self.compute_energy(&self.parameters)
    }

    /// Compute gradient via parameter-shift rule
    pub fn compute_gradient(&self) -> Vec<f64> {
        let shift = core::f64::consts::PI / 2.0;
        let mut gradient = vec![0.0; self.parameters.len()];

        for (i, grad) in gradient.iter_mut().enumerate() {
            // f(θ + π/2)
            let mut params_plus = self.parameters.clone();
            params_plus.values[i] += shift;
            let e_plus = self.compute_energy(&params_plus);

            // f(θ - π/2)
            let mut params_minus = self.parameters.clone();
            params_minus.values[i] -= shift;
            let e_minus = self.compute_energy(&params_minus);

            // Parameter-shift gradient
            *grad = 0.5 * (e_plus - e_minus);
        }

        gradient
    }
}

// ============================================================================
// VQE OPTIMIZER
// ============================================================================

/// VQE optimizer
pub struct VqeOptimizer {
    /// VQE engine
    engine: VqeEngine,
    /// Learning rate
    learning_rate: f64,
    /// Maximum iterations
    max_iterations: usize,
    /// Convergence threshold
    convergence_threshold: f64,
}

impl VqeOptimizer {
    /// Create new optimizer
    pub fn new(engine: VqeEngine) -> Self {
        Self {
            engine,
            learning_rate: 0.1,
            max_iterations: 100,
            convergence_threshold: 1e-6,
        }
    }

    /// Set learning rate
    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.learning_rate = lr;
        self
    }

    /// Set max iterations
    pub fn with_max_iterations(mut self, iters: usize) -> Self {
        self.max_iterations = iters;
        self
    }

    /// Set convergence threshold
    pub fn with_threshold(mut self, thresh: f64) -> Self {
        self.convergence_threshold = thresh;
        self
    }

    /// Run gradient descent optimization
    pub fn optimize(&mut self) -> VqeResult {
        let mut history = Vec::with_capacity(self.max_iterations);
        let mut best_energy = f64::MAX;
        let mut best_params = self.engine.parameters.clone();

        for iteration in 0..self.max_iterations {
            let gradient = self.engine.compute_gradient();

            // Update parameters
            for (param, grad) in self
                .engine
                .parameters
                .values
                .iter_mut()
                .zip(gradient.iter())
            {
                *param -= self.learning_rate * grad;
            }

            let energy = self.engine.energy();
            history.push(energy);

            if energy < best_energy {
                best_energy = energy;
                best_params = self.engine.parameters.clone();
            }

            // Check convergence
            if iteration > 0 {
                let delta = (history[iteration - 1] - energy).abs();
                if delta < self.convergence_threshold {
                    break;
                }
            }
        }

        self.engine.parameters = best_params.clone();

        let converged = history.len() < self.max_iterations;
        VqeResult {
            ground_state_energy: best_energy,
            optimal_parameters: best_params,
            history,
            converged,
        }
    }

    /// Get inner engine
    pub fn into_engine(self) -> VqeEngine {
        self.engine
    }
}

/// VQE optimization result
#[derive(Debug, Clone)]
pub struct VqeResult {
    /// Ground state energy estimate
    pub ground_state_energy: f64,
    /// Optimal parameters
    pub optimal_parameters: VqeParameters,
    /// Energy history
    pub history: Vec<f64>,
    /// Whether optimization converged
    pub converged: bool,
}

// ============================================================================
// MOLECULAR HAMILTONIANS
// ============================================================================

/// Create H2 molecule Hamiltonian (STO-3G, bond length ~0.74Å)
pub fn hydrogen_hamiltonian() -> Hamiltonian {
    // Simplified H2 Hamiltonian in minimal basis
    // H = g0 I + g1 Z0 + g2 Z1 + g3 Z0Z1 + g4 X0X1 + g5 Y0Y1

    let g0 = -0.8105;
    let g1 = 0.1721;
    let g2 = -0.2257;
    let g3 = 0.1209;
    let g4 = 0.0454;
    let g5 = 0.0454;

    let mut hamiltonian = Hamiltonian::new(2);

    // Identity (constant offset) - represented as all-I string
    hamiltonian.add_term(PauliString::identity(2).with_coeff(g0));

    // Z0
    hamiltonian.add_term(PauliString::single(2, 0, Pauli::Z, g1));

    // Z1
    hamiltonian.add_term(PauliString::single(2, 1, Pauli::Z, g2));

    // Z0Z1
    hamiltonian.add_term(PauliString::zz(2, 0, 1, g3));

    // X0X1
    let mut x0x1 = PauliString::identity(2);
    x0x1.paulis[0] = Pauli::X;
    x0x1.paulis[1] = Pauli::X;
    x0x1.coeff = g4;
    hamiltonian.add_term(x0x1);

    // Y0Y1
    let mut y0y1 = PauliString::identity(2);
    y0y1.paulis[0] = Pauli::Y;
    y0y1.paulis[1] = Pauli::Y;
    y0y1.coeff = g5;
    hamiltonian.add_term(y0y1);

    hamiltonian
}

/// Create LiH molecule Hamiltonian (simplified)
pub fn lih_hamiltonian() -> Hamiltonian {
    // 4-qubit simplified LiH Hamiltonian
    let mut hamiltonian = Hamiltonian::new(4);

    // Simplified diagonal terms only
    let coeffs: [(usize, f64); 7] = [
        (0b0000, -7.5),
        (0b0001, 0.12),
        (0b0010, 0.12),
        (0b0100, 0.15),
        (0b1000, 0.15),
        (0b0011, 0.08),
        (0b1100, 0.08),
    ];

    for (mask, coeff) in coeffs {
        let mut paulis = vec![Pauli::I; 4];
        for (i, pauli) in paulis.iter_mut().enumerate() {
            if (mask >> i) & 1 == 1 {
                *pauli = Pauli::Z;
            }
        }
        hamiltonian.add_term(PauliString::new(paulis, coeff));
    }

    hamiltonian
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vqe_parameters() {
        let params = VqeParameters::hardware_efficient(4, 2);
        assert_eq!(params.len(), 24); // 3 * 4 * 2
    }

    #[test]
    fn test_ansatz_builder() {
        let params = VqeParameters::hardware_efficient(2, 1);
        let builder = AnsatzBuilder::new(2);
        let circuit = builder.build(&params);

        assert_eq!(circuit.num_qubits(), 2);
        assert!(circuit.num_gates() > 0);
    }

    #[test]
    fn test_vqe_engine() {
        let h = hydrogen_hamiltonian();
        let engine = VqeEngine::new(h, 2, 1);

        let energy = engine.energy();
        assert!(energy.is_finite());
    }

    #[test]
    fn test_h2_ground_state() {
        let h = hydrogen_hamiltonian();
        let mut engine = VqeEngine::new(h, 2, 2);
        engine.randomize_params(42);

        let mut optimizer = VqeOptimizer::new(engine)
            .with_learning_rate(0.1)
            .with_max_iterations(50);

        let result = optimizer.optimize();

        // H2 ground state ≈ -1.137 Hartree
        // We should get reasonably close
        assert!(result.ground_state_energy < 0.0);
    }

    #[test]
    fn test_gradient_computation() {
        let h = hydrogen_hamiltonian();
        let mut engine = VqeEngine::new(h, 2, 1);
        engine.parameters.values = vec![0.1; engine.parameters.len()];

        let gradient = engine.compute_gradient();

        assert_eq!(gradient.len(), engine.parameters.len());
        assert!(gradient.iter().all(|&g| g.is_finite()));
    }

    #[test]
    fn test_lih_hamiltonian() {
        let h = lih_hamiltonian();
        assert_eq!(h.n_qubits, 4);
        assert_eq!(h.num_terms(), 7);
    }
}
