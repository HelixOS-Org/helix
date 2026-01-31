//! Crypto intelligence for security analysis and recommendations.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::algorithm::AlgorithmInfo;
use super::manager::CryptoManager;
use super::types::{AlgorithmId, AlgorithmStatus, AlgorithmType, CipherMode};

// ============================================================================
// CRYPTO ANALYSIS
// ============================================================================

/// Crypto analysis
#[derive(Debug, Clone)]
pub struct CryptoAnalysis {
    /// Security score (0-100)
    pub security_score: f32,
    /// Hardware acceleration available
    pub hw_available: bool,
    /// Issues detected
    pub issues: Vec<CryptoIssue>,
    /// Recommendations
    pub recommendations: Vec<CryptoRecommendation>,
}

/// Crypto issue
#[derive(Debug, Clone)]
pub struct CryptoIssue {
    /// Issue type
    pub issue_type: CryptoIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Algorithm
    pub algorithm: Option<String>,
}

/// Crypto issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoIssueType {
    /// Weak algorithm in use
    WeakAlgorithm,
    /// Deprecated algorithm in use
    DeprecatedAlgorithm,
    /// Broken algorithm in use
    BrokenAlgorithm,
    /// Weak key size
    WeakKeySize,
    /// No hardware acceleration
    NoHwAcceleration,
    /// Expired key
    ExpiredKey,
    /// ECB mode used
    EcbMode,
    /// Missing authenticated encryption
    MissingAuth,
}

/// Crypto recommendation
#[derive(Debug, Clone)]
pub struct CryptoRecommendation {
    /// Action
    pub action: CryptoAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Crypto action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoAction {
    /// Upgrade algorithm
    UpgradeAlgorithm,
    /// Increase key size
    IncreaseKeySize,
    /// Enable hardware acceleration
    EnableHwAccel,
    /// Rotate key
    RotateKey,
    /// Use authenticated mode
    UseAuthMode,
}

// ============================================================================
// CRYPTO INTELLIGENCE
// ============================================================================

/// Crypto Intelligence
pub struct CryptoIntelligence {
    /// Manager
    manager: CryptoManager,
}

impl CryptoIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: CryptoManager::new(),
        }
    }

    /// Register algorithm
    pub fn register_algorithm(&mut self, name: String, alg_type: AlgorithmType) -> AlgorithmId {
        self.manager.register_algorithm(name, alg_type)
    }

    /// Record operation
    pub fn record_operation(&self, alg_name: &str, bytes: u64) {
        self.manager.record_operation(alg_name, bytes);
    }

    /// Analyze security
    pub fn analyze(&self) -> CryptoAnalysis {
        let mut security_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        let hw_available = self.manager.hw_detector().has_aes_acceleration();

        // Check for deprecated algorithms
        for alg in self.manager.deprecated_in_use() {
            let severity = if alg.status == AlgorithmStatus::Broken {
                10
            } else {
                7
            };
            security_score -= if severity == 10 { 30.0 } else { 15.0 };

            issues.push(CryptoIssue {
                issue_type: if severity == 10 {
                    CryptoIssueType::BrokenAlgorithm
                } else {
                    CryptoIssueType::DeprecatedAlgorithm
                },
                severity,
                description: alloc::format!("Algorithm {} is {}", alg.name, alg.status.name()),
                algorithm: Some(alg.name.clone()),
            });

            recommendations.push(CryptoRecommendation {
                action: CryptoAction::UpgradeAlgorithm,
                expected_improvement: 20.0,
                reason: alloc::format!("Replace {} with a modern algorithm", alg.name),
            });
        }

        // Check for weak keys
        for key in self.manager.key_manager().active_keys() {
            if key.strength().is_weak() {
                security_score -= 20.0;
                issues.push(CryptoIssue {
                    issue_type: CryptoIssueType::WeakKeySize,
                    severity: 8,
                    description: alloc::format!(
                        "Key {} has weak strength ({} bits)",
                        key.id.raw(),
                        key.size_bits
                    ),
                    algorithm: Some(key.algorithm.clone()),
                });
            }
        }

        // Check for hardware acceleration
        if !hw_available {
            issues.push(CryptoIssue {
                issue_type: CryptoIssueType::NoHwAcceleration,
                severity: 3,
                description: String::from("No hardware crypto acceleration detected"),
                algorithm: None,
            });
            recommendations.push(CryptoRecommendation {
                action: CryptoAction::EnableHwAccel,
                expected_improvement: 10.0,
                reason: String::from("Enable hardware acceleration for better performance"),
            });
        }

        // Check for ECB mode
        for alg in self.manager.algorithms().values() {
            if alg.mode == Some(CipherMode::Ecb) && alg.use_count() > 0 {
                security_score -= 25.0;
                issues.push(CryptoIssue {
                    issue_type: CryptoIssueType::EcbMode,
                    severity: 9,
                    description: alloc::format!("Algorithm {} uses insecure ECB mode", alg.name),
                    algorithm: Some(alg.name.clone()),
                });
                recommendations.push(CryptoRecommendation {
                    action: CryptoAction::UseAuthMode,
                    expected_improvement: 25.0,
                    reason: String::from("Use GCM or other authenticated mode instead of ECB"),
                });
            }
        }

        security_score = security_score.max(0.0);

        CryptoAnalysis {
            security_score,
            hw_available,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &CryptoManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut CryptoManager {
        &mut self.manager
    }
}

impl Default for CryptoIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
