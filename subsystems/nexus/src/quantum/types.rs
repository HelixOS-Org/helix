//! # Quantum Types and Core Structures
//!
//! Core type definitions for quantum-inspired computing in the kernel.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of qubits supported
pub const MAX_QUBITS: usize = 20;

/// Default number of QAOA layers
pub const DEFAULT_QAOA_LAYERS: usize = 3;

/// Default number of VQE iterations
pub const DEFAULT_VQE_ITERATIONS: usize = 100;

/// Convergence threshold
pub const CONVERGENCE_THRESHOLD: f64 = 1e-6;

// ============================================================================
// COMPLEX NUMBER
// ============================================================================

/// Complex number for quantum amplitude representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    /// Real part
    pub re: f64,
    /// Imaginary part
    pub im: f64,
}

impl Complex {
    /// Zero complex number
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    /// One (real unit)
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };

    /// Imaginary unit
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    /// Create a new complex number
    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Create from polar coordinates
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            re: r * libm::cos(theta),
            im: r * libm::sin(theta),
        }
    }

    /// Complex conjugate
    #[inline]
    pub const fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    /// Magnitude squared
    #[inline]
    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Magnitude (absolute value)
    #[inline]
    pub fn norm(self) -> f64 {
        libm::sqrt(self.norm_sq())
    }

    /// Phase angle
    #[inline]
    pub fn arg(self) -> f64 {
        libm::atan2(self.im, self.re)
    }

    /// Addition
    #[inline]
    pub fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    /// Subtraction
    #[inline]
    pub fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    /// Multiplication
    #[inline]
    pub fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    /// Division
    pub fn div(self, other: Self) -> Self {
        let denom = other.norm_sq();
        if denom < 1e-15 {
            return Self::ZERO;
        }
        Self {
            re: (self.re * other.re + self.im * other.im) / denom,
            im: (self.im * other.re - self.re * other.im) / denom,
        }
    }

    /// Scale by real number
    #[inline]
    pub fn scale(self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    /// Complex exponential
    pub fn exp(self) -> Self {
        let exp_re = libm::exp(self.re);
        Self {
            re: exp_re * libm::cos(self.im),
            im: exp_re * libm::sin(self.im),
        }
    }
}

impl Default for Complex {
    fn default() -> Self {
        Self::ZERO
    }
}

// ============================================================================
// QUBIT STATE
// ============================================================================

/// Single qubit state |ψ⟩ = α|0⟩ + β|1⟩
#[derive(Debug, Clone)]
pub struct QubitState {
    /// Amplitude for |0⟩
    pub alpha: Complex,
    /// Amplitude for |1⟩
    pub beta: Complex,
}

impl QubitState {
    /// |0⟩ state
    pub const ZERO: Self = Self {
        alpha: Complex::ONE,
        beta: Complex::ZERO,
    };

    /// |1⟩ state
    pub const ONE: Self = Self {
        alpha: Complex::ZERO,
        beta: Complex::ONE,
    };

    /// Create new qubit state
    pub fn new(alpha: Complex, beta: Complex) -> Self {
        Self { alpha, beta }
    }

    /// Create |+⟩ = (|0⟩ + |1⟩)/√2
    pub fn plus() -> Self {
        let inv_sqrt2 = 1.0 / libm::sqrt(2.0);
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(inv_sqrt2, 0.0),
        }
    }

    /// Create |-⟩ = (|0⟩ - |1⟩)/√2
    pub fn minus() -> Self {
        let inv_sqrt2 = 1.0 / libm::sqrt(2.0);
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(-inv_sqrt2, 0.0),
        }
    }

    /// Probability of measuring |0⟩
    #[inline]
    pub fn prob_zero(&self) -> f64 {
        self.alpha.norm_sq()
    }

    /// Probability of measuring |1⟩
    #[inline]
    pub fn prob_one(&self) -> f64 {
        self.beta.norm_sq()
    }

    /// Normalize the state
    pub fn normalize(&mut self) {
        let norm = libm::sqrt(self.alpha.norm_sq() + self.beta.norm_sq());
        if norm > 1e-15 {
            self.alpha = self.alpha.scale(1.0 / norm);
            self.beta = self.beta.scale(1.0 / norm);
        }
    }

    /// Check if normalized
    pub fn is_normalized(&self) -> bool {
        let total = self.alpha.norm_sq() + self.beta.norm_sq();
        (total - 1.0).abs() < 1e-10
    }
}

impl Default for QubitState {
    fn default() -> Self {
        Self::ZERO
    }
}

// ============================================================================
// QUANTUM STATE VECTOR
// ============================================================================

/// Multi-qubit quantum state as state vector
#[derive(Debug, Clone)]
pub struct StateVector {
    /// Number of qubits
    pub n_qubits: usize,
    /// Amplitudes (2^n complex numbers)
    pub amplitudes: Vec<Complex>,
}

impl StateVector {
    /// Create new state in |00...0⟩
    pub fn new(n_qubits: usize) -> Self {
        let dim = 1 << n_qubits;
        let mut amplitudes = vec![Complex::ZERO; dim];
        amplitudes[0] = Complex::ONE;

        Self {
            n_qubits,
            amplitudes,
        }
    }

    /// Create from amplitudes
    pub fn from_amplitudes(n_qubits: usize, amplitudes: Vec<Complex>) -> Self {
        let dim = 1 << n_qubits;
        let mut amps = amplitudes;
        amps.resize(dim, Complex::ZERO);

        Self {
            n_qubits,
            amplitudes: amps,
        }
    }

    /// State dimension (2^n)
    #[inline]
    pub fn dimension(&self) -> usize {
        self.amplitudes.len()
    }

    /// Get amplitude for basis state
    #[inline]
    pub fn get(&self, index: usize) -> Complex {
        self.amplitudes.get(index).copied().unwrap_or(Complex::ZERO)
    }

    /// Set amplitude for basis state
    #[inline]
    pub fn set(&mut self, index: usize, value: Complex) {
        if index < self.amplitudes.len() {
            self.amplitudes[index] = value;
        }
    }

    /// Normalize the state
    pub fn normalize(&mut self) {
        let norm_sq: f64 = self.amplitudes.iter().map(|c| c.norm_sq()).sum();

        if norm_sq > 1e-15 {
            let norm = libm::sqrt(norm_sq);
            for amp in &mut self.amplitudes {
                *amp = amp.scale(1.0 / norm);
            }
        }
    }

    /// Probability of measuring basis state
    #[inline]
    pub fn probability(&self, index: usize) -> f64 {
        self.get(index).norm_sq()
    }

    /// Get all probabilities
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.iter().map(|c| c.norm_sq()).collect()
    }

    /// Measure all qubits (collapse to basis state)
    pub fn measure(&self, rng_state: u64) -> usize {
        let probs = self.probabilities();
        let random = (rng_state as f64) / (u64::MAX as f64);

        let mut cumulative = 0.0;
        for (i, &p) in probs.iter().enumerate() {
            cumulative += p;
            if random <= cumulative {
                return i;
            }
        }

        probs.len() - 1
    }

    /// Inner product ⟨self|other⟩
    pub fn inner_product(&self, other: &StateVector) -> Complex {
        self.amplitudes
            .iter()
            .zip(other.amplitudes.iter())
            .fold(Complex::ZERO, |acc, (&a, &b)| acc.add(a.conj().mul(b)))
    }

    /// Expectation value of diagonal observable
    pub fn expectation_diagonal(&self, diagonal: &[f64]) -> f64 {
        self.amplitudes
            .iter()
            .zip(diagonal.iter())
            .map(|(amp, &d)| amp.norm_sq() * d)
            .sum()
    }
}

// ============================================================================
// PAULI OPERATORS
// ============================================================================

/// Pauli operator type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pauli {
    /// Identity
    I,
    /// Pauli-X (bit flip)
    X,
    /// Pauli-Y
    Y,
    /// Pauli-Z (phase flip)
    Z,
}

impl Pauli {
    /// Get 2x2 matrix representation
    pub fn matrix(&self) -> [[Complex; 2]; 2] {
        match self {
            Pauli::I => [[Complex::ONE, Complex::ZERO], [Complex::ZERO, Complex::ONE]],
            Pauli::X => [[Complex::ZERO, Complex::ONE], [Complex::ONE, Complex::ZERO]],
            Pauli::Y => [[Complex::ZERO, Complex::new(0.0, -1.0)], [
                Complex::new(0.0, 1.0),
                Complex::ZERO,
            ]],
            Pauli::Z => [[Complex::ONE, Complex::ZERO], [
                Complex::ZERO,
                Complex::new(-1.0, 0.0),
            ]],
        }
    }

    /// Eigenvalue for |0⟩
    pub fn eigenvalue_zero(&self) -> f64 {
        match self {
            Pauli::I => 1.0,
            Pauli::X => 1.0, // Actually eigenstates are |+⟩, |-⟩
            Pauli::Y => 1.0,
            Pauli::Z => 1.0,
        }
    }

    /// Eigenvalue for |1⟩
    pub fn eigenvalue_one(&self) -> f64 {
        match self {
            Pauli::I => 1.0,
            Pauli::X => -1.0,
            Pauli::Y => -1.0,
            Pauli::Z => -1.0,
        }
    }
}

// ============================================================================
// PAULI STRING (TENSOR PRODUCT)
// ============================================================================

/// Pauli string (tensor product of Paulis)
#[derive(Debug, Clone)]
pub struct PauliString {
    /// Pauli operators for each qubit
    pub paulis: Vec<Pauli>,
    /// Coefficient
    pub coeff: f64,
}

impl PauliString {
    /// Create new Pauli string
    pub fn new(paulis: Vec<Pauli>, coeff: f64) -> Self {
        Self { paulis, coeff }
    }

    /// Create identity string
    pub fn identity(n_qubits: usize) -> Self {
        Self {
            paulis: vec![Pauli::I; n_qubits],
            coeff: 1.0,
        }
    }

    /// Create single-qubit Pauli
    pub fn single(n_qubits: usize, qubit: usize, pauli: Pauli, coeff: f64) -> Self {
        let mut paulis = vec![Pauli::I; n_qubits];
        if qubit < n_qubits {
            paulis[qubit] = pauli;
        }
        Self { paulis, coeff }
    }

    /// Create ZZ coupling term
    pub fn zz(n_qubits: usize, q1: usize, q2: usize, coeff: f64) -> Self {
        let mut paulis = vec![Pauli::I; n_qubits];
        if q1 < n_qubits {
            paulis[q1] = Pauli::Z;
        }
        if q2 < n_qubits {
            paulis[q2] = Pauli::Z;
        }
        Self { paulis, coeff }
    }

    /// Number of qubits
    pub fn n_qubits(&self) -> usize {
        self.paulis.len()
    }

    /// Check if identity
    pub fn is_identity(&self) -> bool {
        self.paulis.iter().all(|&p| p == Pauli::I)
    }

    /// Return copy with new coefficient
    pub fn with_coeff(mut self, coeff: f64) -> Self {
        self.coeff = coeff;
        self
    }

    /// Compute expectation value for computational basis state
    pub fn expectation_basis(&self, basis_state: usize) -> f64 {
        let mut parity = 0;

        for (i, &pauli) in self.paulis.iter().enumerate() {
            if pauli == Pauli::Z {
                let bit = (basis_state >> i) & 1;
                parity ^= bit;
            }
        }

        self.coeff * if parity == 0 { 1.0 } else { -1.0 }
    }
}

// ============================================================================
// HAMILTONIAN
// ============================================================================

/// Quantum Hamiltonian as sum of Pauli strings
#[derive(Debug, Clone)]
pub struct Hamiltonian {
    /// Number of qubits
    pub n_qubits: usize,
    /// Terms in the Hamiltonian
    pub terms: Vec<PauliString>,
}

impl Hamiltonian {
    /// Create empty Hamiltonian
    pub fn new(n_qubits: usize) -> Self {
        Self {
            n_qubits,
            terms: Vec::new(),
        }
    }

    /// Add a term
    pub fn add_term(&mut self, term: PauliString) {
        self.terms.push(term);
    }

    /// Create Ising Hamiltonian: H = -Σ J_ij Z_i Z_j - Σ h_i Z_i
    pub fn ising(couplings: &[(usize, usize, f64)], fields: &[(usize, f64)]) -> Self {
        let n_qubits = couplings
            .iter()
            .flat_map(|&(i, j, _)| [i, j])
            .chain(fields.iter().map(|&(i, _)| i))
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);

        let mut h = Self::new(n_qubits);

        for &(i, j, jij) in couplings {
            h.add_term(PauliString::zz(n_qubits, i, j, -jij));
        }

        for &(i, hi) in fields {
            h.add_term(PauliString::single(n_qubits, i, Pauli::Z, -hi));
        }

        h
    }

    /// Number of terms
    pub fn num_terms(&self) -> usize {
        self.terms.len()
    }

    /// Compute energy for basis state
    pub fn energy_basis(&self, basis_state: usize) -> f64 {
        self.terms
            .iter()
            .map(|term| term.expectation_basis(basis_state))
            .sum()
    }

    /// Find ground state energy by brute force (small systems only)
    pub fn ground_state_brute_force(&self) -> (usize, f64) {
        let dim = 1 << self.n_qubits;
        let mut best_state = 0;
        let mut best_energy = f64::INFINITY;

        for state in 0..dim {
            let energy = self.energy_basis(state);
            if energy < best_energy {
                best_energy = energy;
                best_state = state;
            }
        }

        (best_state, best_energy)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);

        let sum = a.add(b);
        assert!((sum.re - 4.0).abs() < 1e-10);
        assert!((sum.im - 6.0).abs() < 1e-10);

        let prod = a.mul(b);
        assert!((prod.re - (-5.0)).abs() < 1e-10);
        assert!((prod.im - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_exp() {
        let z = Complex::new(0.0, core::f64::consts::PI);
        let exp_z = z.exp();

        // e^(iπ) = -1
        assert!((exp_z.re - (-1.0)).abs() < 1e-10);
        assert!(exp_z.im.abs() < 1e-10);
    }

    #[test]
    fn test_qubit_state() {
        let q = QubitState::ZERO;
        assert!((q.prob_zero() - 1.0).abs() < 1e-10);
        assert!(q.prob_one().abs() < 1e-10);

        let plus = QubitState::plus();
        assert!((plus.prob_zero() - 0.5).abs() < 1e-10);
        assert!((plus.prob_one() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_state_vector() {
        let state = StateVector::new(2);

        assert_eq!(state.n_qubits, 2);
        assert_eq!(state.dimension(), 4);
        assert!((state.probability(0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pauli_string() {
        let zz = PauliString::zz(2, 0, 1, 1.0);

        // |00⟩ → +1, |01⟩ → -1, |10⟩ → -1, |11⟩ → +1
        assert!((zz.expectation_basis(0b00) - 1.0).abs() < 1e-10);
        assert!((zz.expectation_basis(0b01) - (-1.0)).abs() < 1e-10);
        assert!((zz.expectation_basis(0b10) - (-1.0)).abs() < 1e-10);
        assert!((zz.expectation_basis(0b11) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ising_hamiltonian() {
        let h = Hamiltonian::ising(
            &[(0, 1, 1.0)], // J_01 = 1
            &[],
        );

        assert_eq!(h.n_qubits, 2);

        // Ground state should be |00⟩ or |11⟩ with energy -1
        let (_, gs_energy) = h.ground_state_brute_force();
        assert!((gs_energy - (-1.0)).abs() < 1e-10);
    }
}
