//! # Privacy Protection
//!
//! Year 3 EVOLUTION - Q4 - Privacy-preserving distributed evolution

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::NodeId;
use crate::math::F64Ext;

// ============================================================================
// PRIVACY TYPES
// ============================================================================

/// Privacy budget
#[derive(Debug, Clone, Copy)]
pub struct PrivacyBudget {
    /// Epsilon (privacy parameter)
    pub epsilon: f64,
    /// Delta (probability of privacy failure)
    pub delta: f64,
    /// Spent epsilon
    pub spent_epsilon: f64,
    /// Spent delta
    pub spent_delta: f64,
}

impl PrivacyBudget {
    /// Create new budget
    pub fn new(epsilon: f64, delta: f64) -> Self {
        Self {
            epsilon,
            delta,
            spent_epsilon: 0.0,
            spent_delta: 0.0,
        }
    }

    /// Remaining epsilon
    pub fn remaining_epsilon(&self) -> f64 {
        (self.epsilon - self.spent_epsilon).max(0.0)
    }

    /// Remaining delta
    pub fn remaining_delta(&self) -> f64 {
        (self.delta - self.spent_delta).max(0.0)
    }

    /// Can spend
    pub fn can_spend(&self, epsilon: f64, delta: f64) -> bool {
        self.spent_epsilon + epsilon <= self.epsilon && self.spent_delta + delta <= self.delta
    }

    /// Spend budget
    pub fn spend(&mut self, epsilon: f64, delta: f64) -> bool {
        if self.can_spend(epsilon, delta) {
            self.spent_epsilon += epsilon;
            self.spent_delta += delta;
            true
        } else {
            false
        }
    }

    /// Reset budget
    pub fn reset(&mut self) {
        self.spent_epsilon = 0.0;
        self.spent_delta = 0.0;
    }
}

impl Default for PrivacyBudget {
    fn default() -> Self {
        Self::new(1.0, 1e-5)
    }
}

// ============================================================================
// DIFFERENTIAL PRIVACY
// ============================================================================

/// Differential privacy mechanism
pub trait DifferentialPrivacy: Send + Sync {
    /// Add noise to value
    fn add_noise(&self, value: f64, sensitivity: f64, epsilon: f64) -> f64;

    /// Add noise to vector
    fn add_noise_vec(&self, values: &mut [f64], sensitivity: f64, epsilon: f64);

    /// Mechanism name
    fn name(&self) -> &str;
}

/// Laplace mechanism
pub struct LaplaceMechanism {
    /// Random state
    state: AtomicU64,
}

impl LaplaceMechanism {
    /// Create new mechanism
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(0x853c49e6748fea9b),
        }
    }

    /// Generate random number (xorshift)
    fn random(&self) -> f64 {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        (x.wrapping_mul(0x2545F4914F6CDD1D) as f64) / (u64::MAX as f64)
    }

    /// Generate Laplace noise
    fn laplace_noise(&self, scale: f64) -> f64 {
        let u = self.random() - 0.5;
        if u >= 0.0 {
            -scale * (1.0 - 2.0 * u).ln()
        } else {
            scale * (1.0 + 2.0 * u).ln()
        }
    }
}

impl Default for LaplaceMechanism {
    fn default() -> Self {
        Self::new()
    }
}

impl DifferentialPrivacy for LaplaceMechanism {
    fn add_noise(&self, value: f64, sensitivity: f64, epsilon: f64) -> f64 {
        let scale = sensitivity / epsilon;
        value + self.laplace_noise(scale)
    }

    fn add_noise_vec(&self, values: &mut [f64], sensitivity: f64, epsilon: f64) {
        let scale = sensitivity / epsilon;
        for value in values {
            *value += self.laplace_noise(scale);
        }
    }

    fn name(&self) -> &str {
        "Laplace"
    }
}

/// Gaussian mechanism
pub struct GaussianMechanism {
    /// Random state
    state: AtomicU64,
}

impl GaussianMechanism {
    /// Create new mechanism
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(0x12345678deadbeef),
        }
    }

    /// Generate random number
    fn random(&self) -> f64 {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        (x.wrapping_mul(0x2545F4914F6CDD1D) as f64) / (u64::MAX as f64)
    }

    /// Box-Muller transform for Gaussian
    fn gaussian_noise(&self, sigma: f64) -> f64 {
        let u1 = self.random();
        let u2 = self.random();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos();
        z * sigma
    }

    /// Calculate sigma for given epsilon/delta
    fn calculate_sigma(sensitivity: f64, epsilon: f64, delta: f64) -> f64 {
        // Simplified: sigma = sensitivity * sqrt(2 * ln(1.25/delta)) / epsilon
        let c = (2.0 * (1.25 / delta).ln()).sqrt();
        sensitivity * c / epsilon
    }
}

impl Default for GaussianMechanism {
    fn default() -> Self {
        Self::new()
    }
}

impl DifferentialPrivacy for GaussianMechanism {
    fn add_noise(&self, value: f64, sensitivity: f64, epsilon: f64) -> f64 {
        let sigma = Self::calculate_sigma(sensitivity, epsilon, 1e-5);
        value + self.gaussian_noise(sigma)
    }

    fn add_noise_vec(&self, values: &mut [f64], sensitivity: f64, epsilon: f64) {
        let sigma = Self::calculate_sigma(sensitivity, epsilon, 1e-5);
        for value in values {
            *value += self.gaussian_noise(sigma);
        }
    }

    fn name(&self) -> &str {
        "Gaussian"
    }
}

// ============================================================================
// SECURE AGGREGATION
// ============================================================================

/// Secure aggregation protocol
pub struct SecureAggregation {
    /// Session ID
    session_id: u64,
    /// Participants
    participants: Vec<NodeId>,
    /// Masked values
    masked_values: BTreeMap<NodeId, Vec<f64>>,
    /// Masks
    masks: BTreeMap<(NodeId, NodeId), Vec<f64>>,
    /// State
    state: SecAggState,
}

/// Secure aggregation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecAggState {
    /// Collecting keys
    KeyExchange,
    /// Collecting masked values
    MaskedInput,
    /// Aggregating
    Aggregating,
    /// Complete
    Complete,
}

impl SecureAggregation {
    /// Create new secure aggregation
    pub fn new(participants: Vec<NodeId>) -> Self {
        Self {
            session_id: 0,
            participants,
            masked_values: BTreeMap::new(),
            masks: BTreeMap::new(),
            state: SecAggState::KeyExchange,
        }
    }

    /// Generate mask between two participants
    pub fn generate_mask(&mut self, a: NodeId, b: NodeId, length: usize) -> Vec<f64> {
        // Simplified: use XOR of node IDs as seed
        let seed = a.0 ^ b.0;
        let mut mask = Vec::with_capacity(length);
        let mut state = seed;

        for _ in 0..length {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let value = (state.wrapping_mul(0x2545F4914F6CDD1D) as f64) / (u64::MAX as f64);
            mask.push(value - 0.5); // Center around 0
        }

        self.masks.insert((a, b), mask.clone());
        mask
    }

    /// Submit masked value
    pub fn submit_masked(&mut self, node: NodeId, masked: Vec<f64>) {
        self.masked_values.insert(node, masked);

        if self.masked_values.len() == self.participants.len() {
            self.state = SecAggState::Aggregating;
        }
    }

    /// Aggregate (masks cancel out)
    pub fn aggregate(&mut self) -> Option<Vec<f64>> {
        if self.state != SecAggState::Aggregating {
            return None;
        }

        let first = self.masked_values.values().next()?;
        let mut result = vec![0.0; first.len()];

        for values in self.masked_values.values() {
            for (i, v) in values.iter().enumerate() {
                if i < result.len() {
                    result[i] += v;
                }
            }
        }

        self.state = SecAggState::Complete;
        Some(result)
    }

    /// Get state
    pub fn state(&self) -> SecAggState {
        self.state
    }
}

// ============================================================================
// HOMOMORPHIC ENCRYPTION
// ============================================================================

/// Homomorphic encryption (simplified Paillier-like)
pub struct HomomorphicEncryption {
    /// Public key (n = p * q)
    n: u128,
    /// n squared
    n_squared: u128,
    /// Generator g
    g: u128,
    /// Private key lambda
    lambda: Option<u128>,
    /// Mu (for decryption)
    mu: Option<u128>,
}

impl HomomorphicEncryption {
    /// Create with fixed small primes (for demonstration)
    pub fn new_demo() -> Self {
        // Small primes for demo (in practice, use large primes)
        let p: u128 = 61;
        let q: u128 = 53;
        let n = p * q; // 3233
        let n_squared = n * n;
        let lambda = Self::lcm(p - 1, q - 1);
        let g = n + 1;
        let mu = Self::mod_inverse(lambda, n);

        Self {
            n,
            n_squared,
            g,
            lambda: Some(lambda),
            mu,
        }
    }

    /// Create public key only (for encryption)
    pub fn public_only(n: u128, g: u128) -> Self {
        Self {
            n,
            n_squared: n * n,
            g,
            lambda: None,
            mu: None,
        }
    }

    /// Encrypt a value
    pub fn encrypt(&self, m: u64) -> u128 {
        // c = g^m * r^n mod n^2
        // Simplified: assume r = 1
        let gm = Self::mod_pow(self.g, m as u128, self.n_squared);
        gm % self.n_squared
    }

    /// Decrypt a value
    pub fn decrypt(&self, c: u128) -> Option<u64> {
        let lambda = self.lambda?;
        let mu = self.mu?;

        // m = L(c^lambda mod n^2) * mu mod n
        let c_lambda = Self::mod_pow(c, lambda, self.n_squared);
        let l = (c_lambda - 1) / self.n;
        let m = (l * mu) % self.n;

        Some(m as u64)
    }

    /// Add two encrypted values (homomorphic addition)
    pub fn add(&self, c1: u128, c2: u128) -> u128 {
        (c1 * c2) % self.n_squared
    }

    /// Multiply encrypted value by constant
    pub fn mul_const(&self, c: u128, k: u64) -> u128 {
        Self::mod_pow(c, k as u128, self.n_squared)
    }

    fn mod_pow(base: u128, exp: u128, modulus: u128) -> u128 {
        if modulus == 1 {
            return 0;
        }
        let mut result = 1u128;
        let mut base = base % modulus;
        let mut exp = exp;
        while exp > 0 {
            if exp % 2 == 1 {
                result = (result * base) % modulus;
            }
            exp /= 2;
            base = (base * base) % modulus;
        }
        result
    }

    fn gcd(a: u128, b: u128) -> u128 {
        if b == 0 { a } else { Self::gcd(b, a % b) }
    }

    fn lcm(a: u128, b: u128) -> u128 {
        a / Self::gcd(a, b) * b
    }

    fn mod_inverse(a: u128, m: u128) -> Option<u128> {
        let mut mn = (m as i128, a as i128);
        let mut xy = (0i128, 1i128);

        while mn.1 != 0 {
            xy = (xy.1, xy.0 - (mn.0 / mn.1) * xy.1);
            mn = (mn.1, mn.0 % mn.1);
        }

        if mn.0 > 1 {
            None
        } else {
            Some(((xy.0 + m as i128) % m as i128) as u128)
        }
    }
}

// ============================================================================
// PRIVACY MANAGER
// ============================================================================

/// Privacy manager
pub struct PrivacyManager {
    /// Budgets per node
    budgets: BTreeMap<NodeId, PrivacyBudget>,
    /// DP mechanism
    dp_mechanism: Box<dyn DifferentialPrivacy>,
    /// Homomorphic encryption
    he: Option<HomomorphicEncryption>,
    /// Configuration
    config: PrivacyConfig,
    /// Statistics
    stats: PrivacyStats,
}

/// Privacy configuration
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    /// Default epsilon
    pub default_epsilon: f64,
    /// Default delta
    pub default_delta: f64,
    /// Enable DP
    pub enable_dp: bool,
    /// Enable HE
    pub enable_he: bool,
    /// Enable secure aggregation
    pub enable_secagg: bool,
    /// Clip norm
    pub clip_norm: f64,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            default_epsilon: 1.0,
            default_delta: 1e-5,
            enable_dp: true,
            enable_he: false,
            enable_secagg: true,
            clip_norm: 1.0,
        }
    }
}

/// Privacy statistics
#[derive(Debug, Clone, Default)]
pub struct PrivacyStats {
    /// DP queries
    pub dp_queries: u64,
    /// HE operations
    pub he_operations: u64,
    /// SecAgg sessions
    pub secagg_sessions: u64,
    /// Budget exhaustions
    pub budget_exhaustions: u64,
}

impl PrivacyManager {
    /// Create new privacy manager
    pub fn new(config: PrivacyConfig) -> Self {
        let he = if config.enable_he {
            Some(HomomorphicEncryption::new_demo())
        } else {
            None
        };

        Self {
            budgets: BTreeMap::new(),
            dp_mechanism: Box::new(GaussianMechanism::new()),
            he,
            config,
            stats: PrivacyStats::default(),
        }
    }

    /// Get or create budget for node
    pub fn get_budget(&mut self, node: NodeId) -> &mut PrivacyBudget {
        self.budgets.entry(node).or_insert_with(|| {
            PrivacyBudget::new(self.config.default_epsilon, self.config.default_delta)
        })
    }

    /// Apply differential privacy
    pub fn apply_dp(
        &mut self,
        node: NodeId,
        values: &mut [f64],
        sensitivity: f64,
        epsilon: f64,
    ) -> Result<(), PrivacyError> {
        if !self.config.enable_dp {
            return Ok(());
        }

        // Extract config value before borrowing self through get_budget
        let clip_norm = self.config.clip_norm;

        // Check budget first, then release borrow before other operations
        let can_spend = {
            let budget = self.get_budget(node);
            budget.can_spend(epsilon, 0.0)
        };

        if !can_spend {
            self.stats.budget_exhaustions += 1;
            return Err(PrivacyError::BudgetExhausted);
        }

        // Clip values
        let norm: f64 = values.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > clip_norm {
            let scale = clip_norm / norm;
            for v in values.iter_mut() {
                *v *= scale;
            }
        }

        // Add noise
        self.dp_mechanism
            .add_noise_vec(values, sensitivity, epsilon);

        // Now get budget again to spend it
        self.get_budget(node).spend(epsilon, 0.0);
        self.stats.dp_queries += 1;

        Ok(())
    }

    /// Encrypt value with HE
    pub fn he_encrypt(&mut self, value: u64) -> Result<u128, PrivacyError> {
        let he = self.he.as_ref().ok_or(PrivacyError::HENotEnabled)?;
        self.stats.he_operations += 1;
        Ok(he.encrypt(value))
    }

    /// Decrypt value with HE
    pub fn he_decrypt(&mut self, ciphertext: u128) -> Result<u64, PrivacyError> {
        let he = self.he.as_ref().ok_or(PrivacyError::HENotEnabled)?;
        self.stats.he_operations += 1;
        he.decrypt(ciphertext).ok_or(PrivacyError::DecryptionFailed)
    }

    /// Add encrypted values
    pub fn he_add(&self, c1: u128, c2: u128) -> Result<u128, PrivacyError> {
        let he = self.he.as_ref().ok_or(PrivacyError::HENotEnabled)?;
        Ok(he.add(c1, c2))
    }

    /// Create secure aggregation session
    pub fn create_secagg(&mut self, participants: Vec<NodeId>) -> SecureAggregation {
        self.stats.secagg_sessions += 1;
        SecureAggregation::new(participants)
    }

    /// Set DP mechanism
    pub fn set_dp_mechanism(&mut self, mechanism: Box<dyn DifferentialPrivacy>) {
        self.dp_mechanism = mechanism;
    }

    /// Get statistics
    pub fn stats(&self) -> &PrivacyStats {
        &self.stats
    }
}

impl Default for PrivacyManager {
    fn default() -> Self {
        Self::new(PrivacyConfig::default())
    }
}

/// Privacy error
#[derive(Debug)]
pub enum PrivacyError {
    /// Budget exhausted
    BudgetExhausted,
    /// HE not enabled
    HENotEnabled,
    /// Decryption failed
    DecryptionFailed,
    /// Invalid input
    InvalidInput,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_budget() {
        let mut budget = PrivacyBudget::new(1.0, 1e-5);

        assert!(budget.can_spend(0.5, 0.0));
        assert!(budget.spend(0.5, 0.0));
        assert_eq!(budget.remaining_epsilon(), 0.5);

        assert!(budget.spend(0.5, 0.0));
        assert!(!budget.can_spend(0.1, 0.0));
    }

    #[test]
    fn test_laplace_noise() {
        let mechanism = LaplaceMechanism::new();

        let noisy = mechanism.add_noise(100.0, 1.0, 0.1);
        // Should be close to 100 but with noise
        assert!((noisy - 100.0).abs() < 100.0);
    }

    #[test]
    fn test_homomorphic_encryption() {
        let he = HomomorphicEncryption::new_demo();

        let c1 = he.encrypt(5);
        let c2 = he.encrypt(3);

        // Homomorphic addition
        let c_sum = he.add(c1, c2);
        let result = he.decrypt(c_sum).unwrap();
        assert_eq!(result, 8);
    }

    #[test]
    fn test_secure_aggregation() {
        let participants = vec![NodeId(1), NodeId(2), NodeId(3)];
        let mut secagg = SecureAggregation::new(participants.clone());

        // Submit masked values (in practice, masks cancel out)
        secagg.submit_masked(NodeId(1), vec![1.0, 2.0, 3.0]);
        secagg.submit_masked(NodeId(2), vec![4.0, 5.0, 6.0]);
        secagg.submit_masked(NodeId(3), vec![7.0, 8.0, 9.0]);

        let result = secagg.aggregate().unwrap();
        assert_eq!(result, vec![12.0, 15.0, 18.0]);
    }

    #[test]
    fn test_privacy_manager() {
        let mut manager = PrivacyManager::new(PrivacyConfig::default());

        let mut values = vec![0.1, 0.2, 0.3];
        let result = manager.apply_dp(NodeId(1), &mut values, 1.0, 0.5);
        assert!(result.is_ok());

        // Values should be modified
        assert!(values != vec![0.1, 0.2, 0.3]);
    }
}
