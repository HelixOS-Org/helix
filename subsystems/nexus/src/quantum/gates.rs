//! # Quantum Gates
//!
//! Quantum gate definitions and operations for the kernel quantum engine.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use super::types::{Complex, StateVector};

// ============================================================================
// GATE TYPE
// ============================================================================

/// Quantum gate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateType {
    /// Identity
    I,
    /// Pauli-X (NOT)
    X,
    /// Pauli-Y
    Y,
    /// Pauli-Z
    Z,
    /// Hadamard
    H,
    /// S gate (√Z)
    S,
    /// S-dagger
    Sdg,
    /// T gate (fourth root of Z)
    T,
    /// T-dagger
    Tdg,
    /// Phase gate
    P,
    /// Rotation X
    Rx,
    /// Rotation Y
    Ry,
    /// Rotation Z
    Rz,
    /// CNOT (Controlled-X)
    CX,
    /// Controlled-Z
    CZ,
    /// SWAP
    SWAP,
    /// Toffoli (CCX)
    CCX,
}

// ============================================================================
// SINGLE-QUBIT GATES
// ============================================================================

/// Apply single-qubit gate to state vector
pub fn apply_single_qubit_gate(
    state: &mut StateVector,
    qubit: usize,
    gate: [[Complex; 2]; 2],
) {
    let n = state.n_qubits;
    if qubit >= n {
        return;
    }
    
    let dim = state.dimension();
    let mask = 1 << qubit;
    
    for i in 0..dim {
        if i & mask == 0 {
            let j = i | mask;
            
            let a = state.amplitudes[i];
            let b = state.amplitudes[j];
            
            // Apply 2x2 matrix
            state.amplitudes[i] = gate[0][0].mul(a).add(gate[0][1].mul(b));
            state.amplitudes[j] = gate[1][0].mul(a).add(gate[1][1].mul(b));
        }
    }
}

/// Identity gate matrix
pub fn gate_i() -> [[Complex; 2]; 2] {
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, Complex::ONE],
    ]
}

/// Pauli-X gate matrix
pub fn gate_x() -> [[Complex; 2]; 2] {
    [
        [Complex::ZERO, Complex::ONE],
        [Complex::ONE, Complex::ZERO],
    ]
}

/// Pauli-Y gate matrix
pub fn gate_y() -> [[Complex; 2]; 2] {
    [
        [Complex::ZERO, Complex::new(0.0, -1.0)],
        [Complex::new(0.0, 1.0), Complex::ZERO],
    ]
}

/// Pauli-Z gate matrix
pub fn gate_z() -> [[Complex; 2]; 2] {
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, Complex::new(-1.0, 0.0)],
    ]
}

/// Hadamard gate matrix
pub fn gate_h() -> [[Complex; 2]; 2] {
    let inv_sqrt2 = 1.0 / libm::sqrt(2.0);
    let h = Complex::new(inv_sqrt2, 0.0);
    [
        [h, h],
        [h, h.scale(-1.0)],
    ]
}

/// S gate matrix (√Z)
pub fn gate_s() -> [[Complex; 2]; 2] {
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, Complex::I],
    ]
}

/// S-dagger gate matrix
pub fn gate_sdg() -> [[Complex; 2]; 2] {
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, Complex::new(0.0, -1.0)],
    ]
}

/// T gate matrix
pub fn gate_t() -> [[Complex; 2]; 2] {
    let phase = Complex::from_polar(1.0, core::f64::consts::PI / 4.0);
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, phase],
    ]
}

/// T-dagger gate matrix
pub fn gate_tdg() -> [[Complex; 2]; 2] {
    let phase = Complex::from_polar(1.0, -core::f64::consts::PI / 4.0);
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, phase],
    ]
}

/// Phase gate P(θ)
pub fn gate_p(theta: f64) -> [[Complex; 2]; 2] {
    let phase = Complex::from_polar(1.0, theta);
    [
        [Complex::ONE, Complex::ZERO],
        [Complex::ZERO, phase],
    ]
}

/// Rotation X gate Rx(θ)
pub fn gate_rx(theta: f64) -> [[Complex; 2]; 2] {
    let cos = Complex::new(libm::cos(theta / 2.0), 0.0);
    let sin = Complex::new(0.0, -libm::sin(theta / 2.0));
    [
        [cos, sin],
        [sin, cos],
    ]
}

/// Rotation Y gate Ry(θ)
pub fn gate_ry(theta: f64) -> [[Complex; 2]; 2] {
    let cos = Complex::new(libm::cos(theta / 2.0), 0.0);
    let sin_pos = Complex::new(libm::sin(theta / 2.0), 0.0);
    let sin_neg = Complex::new(-libm::sin(theta / 2.0), 0.0);
    [
        [cos, sin_neg],
        [sin_pos, cos],
    ]
}

/// Rotation Z gate Rz(θ)
pub fn gate_rz(theta: f64) -> [[Complex; 2]; 2] {
    let neg = Complex::from_polar(1.0, -theta / 2.0);
    let pos = Complex::from_polar(1.0, theta / 2.0);
    [
        [neg, Complex::ZERO],
        [Complex::ZERO, pos],
    ]
}

// ============================================================================
// TWO-QUBIT GATES
// ============================================================================

/// Apply CNOT gate (control, target)
pub fn apply_cnot(state: &mut StateVector, control: usize, target: usize) {
    let n = state.n_qubits;
    if control >= n || target >= n || control == target {
        return;
    }
    
    let dim = state.dimension();
    let ctrl_mask = 1 << control;
    let tgt_mask = 1 << target;
    
    for i in 0..dim {
        // Only swap if control is |1⟩ and target is |0⟩
        if (i & ctrl_mask) != 0 && (i & tgt_mask) == 0 {
            let j = i | tgt_mask;
            state.amplitudes.swap(i, j);
        }
    }
}

/// Apply CZ gate (controlled-Z)
pub fn apply_cz(state: &mut StateVector, qubit1: usize, qubit2: usize) {
    let n = state.n_qubits;
    if qubit1 >= n || qubit2 >= n || qubit1 == qubit2 {
        return;
    }
    
    let dim = state.dimension();
    let mask1 = 1 << qubit1;
    let mask2 = 1 << qubit2;
    
    for i in 0..dim {
        // Apply -1 phase when both qubits are |1⟩
        if (i & mask1) != 0 && (i & mask2) != 0 {
            state.amplitudes[i] = state.amplitudes[i].scale(-1.0);
        }
    }
}

/// Apply SWAP gate
pub fn apply_swap(state: &mut StateVector, qubit1: usize, qubit2: usize) {
    let n = state.n_qubits;
    if qubit1 >= n || qubit2 >= n || qubit1 == qubit2 {
        return;
    }
    
    let dim = state.dimension();
    let mask1 = 1 << qubit1;
    let mask2 = 1 << qubit2;
    
    for i in 0..dim {
        let bit1 = (i & mask1) != 0;
        let bit2 = (i & mask2) != 0;
        
        // Only swap if bits are different and this is the "smaller" index
        if bit1 != bit2 && !bit1 {
            let j = (i | mask1) & !mask2;
            state.amplitudes.swap(i, j);
        }
    }
}

/// Apply Controlled-RZ gate
pub fn apply_crz(state: &mut StateVector, control: usize, target: usize, theta: f64) {
    let n = state.n_qubits;
    if control >= n || target >= n || control == target {
        return;
    }
    
    let dim = state.dimension();
    let ctrl_mask = 1 << control;
    let tgt_mask = 1 << target;
    
    let phase_neg = Complex::from_polar(1.0, -theta / 2.0);
    let phase_pos = Complex::from_polar(1.0, theta / 2.0);
    
    for i in 0..dim {
        if (i & ctrl_mask) != 0 {
            if (i & tgt_mask) == 0 {
                state.amplitudes[i] = state.amplitudes[i].mul(phase_neg);
            } else {
                state.amplitudes[i] = state.amplitudes[i].mul(phase_pos);
            }
        }
    }
}

/// Apply Controlled-RY gate
pub fn apply_cry(state: &mut StateVector, control: usize, target: usize, theta: f64) {
    let n = state.n_qubits;
    if control >= n || target >= n || control == target {
        return;
    }
    
    let dim = state.dimension();
    let ctrl_mask = 1 << control;
    let tgt_mask = 1 << target;
    
    let cos = libm::cos(theta / 2.0);
    let sin = libm::sin(theta / 2.0);
    
    for i in 0..dim {
        // Only apply when control is |1⟩ and target is |0⟩
        if (i & ctrl_mask) != 0 && (i & tgt_mask) == 0 {
            let j = i | tgt_mask;
            
            let a = state.amplitudes[i];
            let b = state.amplitudes[j];
            
            state.amplitudes[i] = a.scale(cos).add(b.scale(-sin));
            state.amplitudes[j] = a.scale(sin).add(b.scale(cos));
        }
    }
}

// ============================================================================
// MULTI-QUBIT GATES
// ============================================================================

/// Apply Toffoli gate (CCX)
pub fn apply_toffoli(
    state: &mut StateVector,
    control1: usize,
    control2: usize,
    target: usize,
) {
    let n = state.n_qubits;
    if control1 >= n || control2 >= n || target >= n {
        return;
    }
    if control1 == control2 || control1 == target || control2 == target {
        return;
    }
    
    let dim = state.dimension();
    let ctrl1_mask = 1 << control1;
    let ctrl2_mask = 1 << control2;
    let tgt_mask = 1 << target;
    
    for i in 0..dim {
        // Only flip target if both controls are |1⟩
        if (i & ctrl1_mask) != 0 && (i & ctrl2_mask) != 0 && (i & tgt_mask) == 0 {
            let j = i | tgt_mask;
            state.amplitudes.swap(i, j);
        }
    }
}

// ============================================================================
// GATE APPLICATION HELPERS
// ============================================================================

/// Apply X gate to qubit
pub fn apply_x(state: &mut StateVector, qubit: usize) {
    apply_single_qubit_gate(state, qubit, gate_x());
}

/// Apply Y gate to qubit
pub fn apply_y(state: &mut StateVector, qubit: usize) {
    apply_single_qubit_gate(state, qubit, gate_y());
}

/// Apply Z gate to qubit
pub fn apply_z(state: &mut StateVector, qubit: usize) {
    apply_single_qubit_gate(state, qubit, gate_z());
}

/// Apply Hadamard gate to qubit
pub fn apply_h(state: &mut StateVector, qubit: usize) {
    apply_single_qubit_gate(state, qubit, gate_h());
}

/// Apply Rx gate to qubit
pub fn apply_rx(state: &mut StateVector, qubit: usize, theta: f64) {
    apply_single_qubit_gate(state, qubit, gate_rx(theta));
}

/// Apply Ry gate to qubit
pub fn apply_ry(state: &mut StateVector, qubit: usize, theta: f64) {
    apply_single_qubit_gate(state, qubit, gate_ry(theta));
}

/// Apply Rz gate to qubit
pub fn apply_rz(state: &mut StateVector, qubit: usize, theta: f64) {
    apply_single_qubit_gate(state, qubit, gate_rz(theta));
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_x_gate() {
        let mut state = StateVector::new(1);
        apply_x(&mut state, 0);
        
        // |0⟩ → |1⟩
        assert!(state.probability(0) < 1e-10);
        assert!((state.probability(1) - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_hadamard() {
        let mut state = StateVector::new(1);
        apply_h(&mut state, 0);
        
        // |0⟩ → |+⟩ = (|0⟩ + |1⟩)/√2
        assert!((state.probability(0) - 0.5).abs() < 1e-10);
        assert!((state.probability(1) - 0.5).abs() < 1e-10);
    }
    
    #[test]
    fn test_cnot() {
        // Start with |10⟩
        let mut state = StateVector::new(2);
        apply_x(&mut state, 1);  // |00⟩ → |10⟩
        
        apply_cnot(&mut state, 1, 0);
        
        // |10⟩ → |11⟩
        assert!((state.probability(0b11) - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_bell_state() {
        let mut state = StateVector::new(2);
        
        // Create Bell state |Φ+⟩ = (|00⟩ + |11⟩)/√2
        apply_h(&mut state, 0);
        apply_cnot(&mut state, 0, 1);
        
        assert!((state.probability(0b00) - 0.5).abs() < 1e-10);
        assert!((state.probability(0b11) - 0.5).abs() < 1e-10);
        assert!(state.probability(0b01) < 1e-10);
        assert!(state.probability(0b10) < 1e-10);
    }
    
    #[test]
    fn test_rotation_gates() {
        let mut state = StateVector::new(1);
        
        // Rx(π) ≈ X
        apply_rx(&mut state, 0, core::f64::consts::PI);
        
        assert!(state.probability(0) < 1e-10);
        assert!((state.probability(1) - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_swap() {
        let mut state = StateVector::new(2);
        apply_x(&mut state, 0);  // |00⟩ → |01⟩
        
        apply_swap(&mut state, 0, 1);
        
        // |01⟩ → |10⟩
        assert!((state.probability(0b10) - 1.0).abs() < 1e-10);
    }
}
