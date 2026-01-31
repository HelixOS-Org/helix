//! Crypto Intelligence Module
//!
//! This module provides AI-powered cryptographic subsystem analysis and optimization.
//! It includes algorithm management, key lifecycle tracking, hardware acceleration detection,
//! and intelligent security recommendations for cryptographic operations.

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod algorithm;
mod hardware;
mod intelligence;
mod key;
mod manager;
mod types;

// Re-exports
pub use algorithm::{AlgorithmInfo, KnownAlgorithms};
pub use hardware::{HwCryptoDetector, HwCryptoFeature};
pub use intelligence::{
    CryptoAction, CryptoAnalysis, CryptoIntelligence, CryptoIssue, CryptoIssueType,
    CryptoRecommendation,
};
pub use key::{KeyInfo, KeyManager, KeyState, KeyType};
pub use manager::CryptoManager;
pub use types::{
    AlgorithmId, AlgorithmStatus, AlgorithmType, CipherMode, KeyId, Priority, SecurityStrength,
    TransformId,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_strength() {
        assert!(SecurityStrength::BITS_256.is_secure());
        assert!(SecurityStrength::BITS_128.is_secure());
        assert!(!SecurityStrength::new(64).is_secure());
        assert!(SecurityStrength::new(64).is_weak());
    }

    #[test]
    fn test_algorithm_info() {
        let mut alg = AlgorithmInfo::new(
            AlgorithmId::new(1),
            alloc::string::String::from("aes-256-gcm"),
            AlgorithmType::Aead,
        );
        alg.strength = SecurityStrength::BITS_256;
        alg.mode = Some(CipherMode::Gcm);

        assert!(alg.is_secure());
        alg.record_use(1024);
        assert_eq!(alg.use_count(), 1);
    }

    #[test]
    fn test_key_manager() {
        let mut km = KeyManager::new();

        let id = km.register(
            KeyType::Symmetric,
            alloc::string::String::from("aes-256"),
            256,
            1000,
        );
        assert!(km.get(&id).is_some());

        assert!(km.activate(id));
        assert_eq!(km.active_count(), 1);

        assert!(km.revoke(id));
        assert_eq!(km.active_count(), 0);
    }

    #[test]
    fn test_known_algorithms() {
        assert_eq!(KnownAlgorithms::status("md5"), AlgorithmStatus::Broken);
        assert_eq!(KnownAlgorithms::status("sha1"), AlgorithmStatus::Deprecated);
        assert_eq!(KnownAlgorithms::status("aes-256"), AlgorithmStatus::Active);
    }

    #[test]
    fn test_crypto_intelligence() {
        let mut intel = CryptoIntelligence::new();

        intel.register_algorithm(
            alloc::string::String::from("aes-256-gcm"),
            AlgorithmType::Aead,
        );
        intel.record_operation("aes-256-gcm", 4096);

        let analysis = intel.analyze();
        assert!(analysis.security_score > 50.0);
    }
}
