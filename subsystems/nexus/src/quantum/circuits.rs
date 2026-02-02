//! # Quantum Circuits
//!
//! Circuit representation and execution for quantum simulation.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::gates::{
    GateType, apply_cnot, apply_cz, apply_h, apply_rx, apply_ry, apply_rz, apply_swap,
    apply_toffoli, apply_x, apply_y, apply_z,
};
use super::types::StateVector;

// ============================================================================
// CIRCUIT INSTRUCTION
// ============================================================================

/// Single instruction in a quantum circuit
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Gate type
    pub gate: GateType,
    /// Target qubits
    pub targets: Vec<usize>,
    /// Optional parameter (angle for rotation gates)
    pub parameter: Option<f64>,
}

impl Instruction {
    /// Create new single-qubit instruction
    pub fn single(gate: GateType, qubit: usize) -> Self {
        Self {
            gate,
            targets: alloc::vec![qubit],
            parameter: None,
        }
    }

    /// Create new two-qubit instruction
    pub fn two_qubit(gate: GateType, q1: usize, q2: usize) -> Self {
        Self {
            gate,
            targets: alloc::vec![q1, q2],
            parameter: None,
        }
    }

    /// Create new parametric instruction
    pub fn parametric(gate: GateType, qubit: usize, theta: f64) -> Self {
        Self {
            gate,
            targets: alloc::vec![qubit],
            parameter: Some(theta),
        }
    }

    /// Create CNOT instruction
    pub fn cnot(control: usize, target: usize) -> Self {
        Self::two_qubit(GateType::CX, control, target)
    }

    /// Create Toffoli instruction
    pub fn toffoli(ctrl1: usize, ctrl2: usize, target: usize) -> Self {
        Self {
            gate: GateType::CCX,
            targets: alloc::vec![ctrl1, ctrl2, target],
            parameter: None,
        }
    }
}

// ============================================================================
// QUANTUM CIRCUIT
// ============================================================================

/// Quantum circuit representation
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    /// Number of qubits
    pub n_qubits: usize,
    /// List of instructions
    pub instructions: Vec<Instruction>,
    /// Circuit depth (computed lazily)
    depth_cache: Option<usize>,
}

impl QuantumCircuit {
    /// Create new empty circuit
    pub fn new(n_qubits: usize) -> Self {
        Self {
            n_qubits,
            instructions: Vec::new(),
            depth_cache: None,
        }
    }

    /// Create circuit with capacity
    pub fn with_capacity(n_qubits: usize, capacity: usize) -> Self {
        Self {
            n_qubits,
            instructions: Vec::with_capacity(capacity),
            depth_cache: None,
        }
    }

    /// Get number of qubits
    pub fn num_qubits(&self) -> usize {
        self.n_qubits
    }

    /// Get number of gates
    pub fn num_gates(&self) -> usize {
        self.instructions.len()
    }

    /// Add instruction to circuit
    fn add_instruction(&mut self, inst: Instruction) {
        self.instructions.push(inst);
        self.depth_cache = None; // Invalidate cache
    }

    /// Add X gate
    pub fn x(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::X, qubit));
        self
    }

    /// Add Y gate
    pub fn y(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::Y, qubit));
        self
    }

    /// Add Z gate
    pub fn z(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::Z, qubit));
        self
    }

    /// Add Hadamard gate
    pub fn h(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::H, qubit));
        self
    }

    /// Add S gate
    pub fn s(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::S, qubit));
        self
    }

    /// Add T gate
    pub fn t(&mut self, qubit: usize) -> &mut Self {
        self.add_instruction(Instruction::single(GateType::T, qubit));
        self
    }

    /// Add Rx rotation
    pub fn rx(&mut self, qubit: usize, theta: f64) -> &mut Self {
        self.add_instruction(Instruction::parametric(GateType::Rx, qubit, theta));
        self
    }

    /// Add Ry rotation
    pub fn ry(&mut self, qubit: usize, theta: f64) -> &mut Self {
        self.add_instruction(Instruction::parametric(GateType::Ry, qubit, theta));
        self
    }

    /// Add Rz rotation
    pub fn rz(&mut self, qubit: usize, theta: f64) -> &mut Self {
        self.add_instruction(Instruction::parametric(GateType::Rz, qubit, theta));
        self
    }

    /// Add CNOT gate
    pub fn cnot(&mut self, control: usize, target: usize) -> &mut Self {
        self.add_instruction(Instruction::cnot(control, target));
        self
    }

    /// Add CZ gate
    pub fn cz(&mut self, q1: usize, q2: usize) -> &mut Self {
        self.add_instruction(Instruction::two_qubit(GateType::CZ, q1, q2));
        self
    }

    /// Add SWAP gate
    pub fn swap(&mut self, q1: usize, q2: usize) -> &mut Self {
        self.add_instruction(Instruction::two_qubit(GateType::SWAP, q1, q2));
        self
    }

    /// Add Toffoli gate
    pub fn ccx(&mut self, ctrl1: usize, ctrl2: usize, target: usize) -> &mut Self {
        self.add_instruction(Instruction::toffoli(ctrl1, ctrl2, target));
        self
    }

    /// Append another circuit
    pub fn append(&mut self, other: &QuantumCircuit) -> &mut Self {
        for inst in &other.instructions {
            self.instructions.push(inst.clone());
        }
        self.depth_cache = None;
        self
    }

    /// Calculate circuit depth
    pub fn depth(&mut self) -> usize {
        if let Some(d) = self.depth_cache {
            return d;
        }

        // Track depth per qubit
        let mut qubit_depth = alloc::vec![0usize; self.n_qubits];

        for inst in &self.instructions {
            // Find max depth among targets
            let max_depth = inst
                .targets
                .iter()
                .filter(|&&q| q < self.n_qubits)
                .map(|&q| qubit_depth[q])
                .max()
                .unwrap_or(0);

            // Update all target qubits to new depth
            for &q in &inst.targets {
                if q < self.n_qubits {
                    qubit_depth[q] = max_depth + 1;
                }
            }
        }

        let depth = qubit_depth.into_iter().max().unwrap_or(0);
        self.depth_cache = Some(depth);
        depth
    }

    /// Clear all instructions
    pub fn clear(&mut self) {
        self.instructions.clear();
        self.depth_cache = Some(0);
    }

    /// Reverse circuit (inverse)
    pub fn inverse(&self) -> QuantumCircuit {
        let mut inv = QuantumCircuit::new(self.n_qubits);

        for inst in self.instructions.iter().rev() {
            let inv_inst = match inst.gate {
                // Self-inverse gates
                GateType::I
                | GateType::X
                | GateType::Y
                | GateType::Z
                | GateType::H
                | GateType::CX
                | GateType::CZ
                | GateType::SWAP
                | GateType::CCX => inst.clone(),

                // Inverse rotation
                GateType::Rx | GateType::Ry | GateType::Rz | GateType::P => Instruction {
                    gate: inst.gate,
                    targets: inst.targets.clone(),
                    parameter: inst.parameter.map(|p| -p),
                },

                // S† is inverse of S
                GateType::S => Instruction::single(GateType::Sdg, inst.targets[0]),
                GateType::Sdg => Instruction::single(GateType::S, inst.targets[0]),

                // T† is inverse of T
                GateType::T => Instruction::single(GateType::Tdg, inst.targets[0]),
                GateType::Tdg => Instruction::single(GateType::T, inst.targets[0]),
            };

            inv.instructions.push(inv_inst);
        }

        inv
    }
}

// ============================================================================
// CIRCUIT EXECUTOR
// ============================================================================

/// Execute a quantum circuit on a state vector
pub struct CircuitExecutor {
    /// Current state
    pub state: StateVector,
}

impl CircuitExecutor {
    /// Create executor with n qubits in |0...0⟩ state
    pub fn new(n_qubits: usize) -> Self {
        Self {
            state: StateVector::new(n_qubits),
        }
    }

    /// Create executor from existing state
    pub fn from_state(state: StateVector) -> Self {
        Self { state }
    }

    /// Reset to |0...0⟩
    pub fn reset(&mut self) {
        self.state = StateVector::new(self.state.n_qubits);
    }

    /// Execute single instruction
    pub fn execute_instruction(&mut self, inst: &Instruction) {
        match inst.gate {
            GateType::I => {},
            GateType::X => apply_x(&mut self.state, inst.targets[0]),
            GateType::Y => apply_y(&mut self.state, inst.targets[0]),
            GateType::Z => apply_z(&mut self.state, inst.targets[0]),
            GateType::H => apply_h(&mut self.state, inst.targets[0]),

            GateType::S => {
                let theta = core::f64::consts::PI / 2.0;
                apply_rz(&mut self.state, inst.targets[0], theta);
            },
            GateType::Sdg => {
                let theta = -core::f64::consts::PI / 2.0;
                apply_rz(&mut self.state, inst.targets[0], theta);
            },
            GateType::T => {
                let theta = core::f64::consts::PI / 4.0;
                apply_rz(&mut self.state, inst.targets[0], theta);
            },
            GateType::Tdg => {
                let theta = -core::f64::consts::PI / 4.0;
                apply_rz(&mut self.state, inst.targets[0], theta);
            },
            GateType::P => {
                if let Some(theta) = inst.parameter {
                    apply_rz(&mut self.state, inst.targets[0], theta);
                }
            },

            GateType::Rx => {
                if let Some(theta) = inst.parameter {
                    apply_rx(&mut self.state, inst.targets[0], theta);
                }
            },
            GateType::Ry => {
                if let Some(theta) = inst.parameter {
                    apply_ry(&mut self.state, inst.targets[0], theta);
                }
            },
            GateType::Rz => {
                if let Some(theta) = inst.parameter {
                    apply_rz(&mut self.state, inst.targets[0], theta);
                }
            },

            GateType::CX => {
                apply_cnot(&mut self.state, inst.targets[0], inst.targets[1]);
            },
            GateType::CZ => {
                apply_cz(&mut self.state, inst.targets[0], inst.targets[1]);
            },
            GateType::SWAP => {
                apply_swap(&mut self.state, inst.targets[0], inst.targets[1]);
            },
            GateType::CCX => {
                apply_toffoli(
                    &mut self.state,
                    inst.targets[0],
                    inst.targets[1],
                    inst.targets[2],
                );
            },
        }
    }

    /// Execute entire circuit
    pub fn execute(&mut self, circuit: &QuantumCircuit) {
        for inst in &circuit.instructions {
            self.execute_instruction(inst);
        }
    }

    /// Get probability of measuring specific outcome
    pub fn probability(&self, outcome: usize) -> f64 {
        self.state.probability(outcome)
    }

    /// Get all probabilities
    pub fn probabilities(&self) -> Vec<f64> {
        (0..self.state.dimension())
            .map(|i| self.probability(i))
            .collect()
    }

    /// Measure all qubits (returns classical bitstring)
    pub fn measure(&self, rng_seed: u64) -> usize {
        let probs = self.probabilities();

        // Simple PRNG
        let mut x = rng_seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let rand = (x as f64) / (u64::MAX as f64);

        let mut cumsum = 0.0;
        for (i, &p) in probs.iter().enumerate() {
            cumsum += p;
            if rand < cumsum {
                return i;
            }
        }

        probs.len() - 1
    }

    /// Get expectation value of Z on qubit
    pub fn expectation_z(&self, qubit: usize) -> f64 {
        let dim = self.state.dimension();
        let mask = 1 << qubit;

        let mut exp = 0.0;
        for i in 0..dim {
            let p = self.probability(i);
            if (i & mask) == 0 {
                exp += p; // |0⟩ eigenvalue +1
            } else {
                exp -= p; // |1⟩ eigenvalue -1
            }
        }

        exp
    }
}

// ============================================================================
// CIRCUIT BUILDERS
// ============================================================================

/// Create GHZ state circuit
pub fn ghz_circuit(n_qubits: usize) -> QuantumCircuit {
    let mut circuit = QuantumCircuit::new(n_qubits);

    circuit.h(0);
    for i in 0..n_qubits - 1 {
        circuit.cnot(i, i + 1);
    }

    circuit
}

/// Create W state circuit (approximate)
pub fn w_circuit(n_qubits: usize) -> QuantumCircuit {
    let mut circuit = QuantumCircuit::new(n_qubits);

    if n_qubits == 0 {
        return circuit;
    }

    // Start with |100...0⟩
    circuit.x(0);

    // Distribute amplitude
    for i in 0..n_qubits - 1 {
        let remaining = (n_qubits - i) as f64;
        let theta = 2.0 * libm::acos(libm::sqrt((remaining - 1.0) / remaining));
        circuit.ry(i, theta);
        circuit.cnot(i, i + 1);
        circuit.x(i);
    }

    circuit
}

/// Create QFT circuit
pub fn qft_circuit(n_qubits: usize) -> QuantumCircuit {
    let mut circuit = QuantumCircuit::new(n_qubits);

    for i in 0..n_qubits {
        circuit.h(i);

        for j in (i + 1)..n_qubits {
            let k = j - i;
            let theta = core::f64::consts::PI / (1 << k) as f64;
            // Controlled phase
            circuit.rz(j, theta / 2.0);
            circuit.cnot(i, j);
            circuit.rz(j, -theta / 2.0);
            circuit.cnot(i, j);
        }
    }

    // Swap qubits to reverse order
    for i in 0..n_qubits / 2 {
        circuit.swap(i, n_qubits - 1 - i);
    }

    circuit
}

/// Create random circuit layer
pub fn random_layer(n_qubits: usize, seed: u64) -> QuantumCircuit {
    let mut circuit = QuantumCircuit::new(n_qubits);
    let mut rng = seed;

    // Random single-qubit rotations
    for i in 0..n_qubits {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let theta = (rng as f64 / u64::MAX as f64) * 2.0 * core::f64::consts::PI;

        rng ^= rng << 13;
        let gate_choice = rng % 3;

        match gate_choice {
            0 => circuit.rx(i, theta),
            1 => circuit.ry(i, theta),
            _ => circuit.rz(i, theta),
        };
    }

    // Entangling layer
    for i in (0..n_qubits - 1).step_by(2) {
        circuit.cnot(i, i + 1);
    }
    for i in (1..n_qubits - 1).step_by(2) {
        circuit.cnot(i, i + 1);
    }

    circuit
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_builder() {
        let mut circuit = QuantumCircuit::new(2);
        circuit.h(0).cnot(0, 1);

        assert_eq!(circuit.num_gates(), 2);
        assert_eq!(circuit.num_qubits(), 2);
    }

    #[test]
    fn test_bell_circuit() {
        let mut circuit = QuantumCircuit::new(2);
        circuit.h(0).cnot(0, 1);

        let mut exec = CircuitExecutor::new(2);
        exec.execute(&circuit);

        assert!((exec.probability(0b00) - 0.5).abs() < 1e-10);
        assert!((exec.probability(0b11) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_ghz_circuit() {
        let circuit = ghz_circuit(3);

        let mut exec = CircuitExecutor::new(3);
        exec.execute(&circuit);

        // GHZ state: (|000⟩ + |111⟩)/√2
        assert!((exec.probability(0b000) - 0.5).abs() < 1e-10);
        assert!((exec.probability(0b111) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_circuit_depth() {
        let mut circuit = QuantumCircuit::new(3);
        circuit.h(0).h(1).h(2); // Depth 1 (parallel)
        circuit.cnot(0, 1); // Depth 2
        circuit.cnot(1, 2); // Depth 3

        assert_eq!(circuit.depth(), 3);
    }

    #[test]
    fn test_circuit_inverse() {
        let mut circuit = QuantumCircuit::new(2);
        circuit.rx(0, 1.0).ry(1, 0.5).cnot(0, 1);

        let inv = circuit.inverse();

        // Execute circuit then inverse should return to |00⟩
        let mut exec = CircuitExecutor::new(2);
        exec.execute(&circuit);
        exec.execute(&inv);

        assert!((exec.probability(0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_expectation_z() {
        let mut exec = CircuitExecutor::new(1);

        // |0⟩ has ⟨Z⟩ = +1
        assert!((exec.expectation_z(0) - 1.0).abs() < 1e-10);

        // |1⟩ has ⟨Z⟩ = -1
        exec.execute_instruction(&Instruction::single(GateType::X, 0));
        assert!((exec.expectation_z(0) + 1.0).abs() < 1e-10);

        // |+⟩ has ⟨Z⟩ = 0
        exec.reset();
        exec.execute_instruction(&Instruction::single(GateType::H, 0));
        assert!(exec.expectation_z(0).abs() < 1e-10);
    }
}
