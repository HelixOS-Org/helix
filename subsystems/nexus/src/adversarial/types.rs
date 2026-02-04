//! Type definitions and constants for the adversarial defense module.

use alloc::collections::BTreeMap;
use alloc::string::String;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default perturbation epsilon
pub const DEFAULT_EPSILON: f64 = 0.01;

/// Maximum perturbation iterations
pub const MAX_ATTACK_ITER: usize = 100;

/// Number of random samples for detection
pub const DETECTION_SAMPLES: usize = 50;

/// Ensemble size for voting
pub const ENSEMBLE_SIZE: usize = 5;

/// Input dimension limit
pub const MAX_INPUT_DIM: usize = 1024;

// ============================================================================
// ADVERSARIAL PERTURBATION TYPES
// ============================================================================

/// Types of adversarial perturbations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerturbationType {
    /// L-infinity bounded perturbations
    LInf,
    /// L2 bounded perturbations
    L2,
    /// L1 bounded perturbations
    L1,
    /// Patch-based perturbations
    Patch,
    /// Semantic perturbations
    Semantic,
}

// ============================================================================
// DETECTION RESULT
// ============================================================================

/// Detection result
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Is input adversarial?
    pub is_adversarial: bool,
    /// Confidence score
    pub confidence: f64,
    /// Detection method used
    pub method: String,
    /// Additional scores
    pub scores: BTreeMap<String, f64>,
}

impl DetectionResult {
    /// Create a new detection result
    pub fn new(is_adversarial: bool, confidence: f64, method: String) -> Self {
        Self {
            is_adversarial,
            confidence,
            method,
            scores: BTreeMap::new(),
        }
    }
}

// ============================================================================
// KERNEL ATTACK TYPES
// ============================================================================

/// Types of kernel adversarial attacks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAttackType {
    /// Resource exhaustion
    ResourceExhaustion,
    /// Priority manipulation
    PriorityManipulation,
    /// Cache poisoning
    CachePoisoning,
    /// Timing attack
    TimingAttack,
    /// Evasion attack
    Evasion,
}

/// Kernel adversarial event
#[derive(Debug, Clone)]
pub struct AdversarialEvent {
    /// Event timestamp
    pub timestamp: u64,
    /// Attack type
    pub attack_type: KernelAttackType,
    /// Affected component
    pub component: String,
    /// Severity (0-1)
    pub severity: f64,
    /// Was it blocked?
    pub blocked: bool,
}

// ============================================================================
// DEFENSE STATISTICS
// ============================================================================

/// Defense statistics
#[derive(Debug, Clone)]
pub struct DefenseStats {
    /// Total attacks detected
    pub total_attacks: usize,
    /// Blocked attacks
    pub blocked_attacks: usize,
    /// Block rate
    pub block_rate: f64,
    /// Attacks by type
    pub attacks_by_type: BTreeMap<String, usize>,
}
