//! Crypto manager for algorithm and key management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::algorithm::{AlgorithmInfo, KnownAlgorithms};
use super::hardware::HwCryptoDetector;
use super::key::KeyManager;
use super::types::{AlgorithmId, AlgorithmType};

// ============================================================================
// CRYPTO MANAGER
// ============================================================================

/// Crypto manager
pub struct CryptoManager {
    /// Algorithms
    algorithms: BTreeMap<String, AlgorithmInfo>,
    /// Key manager
    key_manager: KeyManager,
    /// Hardware detector
    hw_detector: HwCryptoDetector,
    /// Next algorithm ID
    next_alg_id: AtomicU64,
    /// Total operations
    total_operations: AtomicU64,
    /// Total bytes
    total_bytes: AtomicU64,
}

impl CryptoManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            algorithms: BTreeMap::new(),
            key_manager: KeyManager::new(),
            hw_detector: HwCryptoDetector::new(),
            next_alg_id: AtomicU64::new(1),
            total_operations: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    /// Register algorithm
    pub fn register_algorithm(&mut self, name: String, alg_type: AlgorithmType) -> AlgorithmId {
        let id = AlgorithmId::new(self.next_alg_id.fetch_add(1, Ordering::Relaxed));
        let mut alg = AlgorithmInfo::new(id, name.clone(), alg_type);

        // Set known properties
        alg.strength = KnownAlgorithms::strength(&name);
        alg.status = KnownAlgorithms::status(&name);

        self.algorithms.insert(name, alg);
        id
    }

    /// Get algorithm
    pub fn get_algorithm(&self, name: &str) -> Option<&AlgorithmInfo> {
        self.algorithms.get(name)
    }

    /// Get algorithm mutably
    pub fn get_algorithm_mut(&mut self, name: &str) -> Option<&mut AlgorithmInfo> {
        self.algorithms.get_mut(name)
    }

    /// Record operation
    pub fn record_operation(&self, alg_name: &str, bytes: u64) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);

        if let Some(alg) = self.algorithms.get(alg_name) {
            alg.record_use(bytes);
        }
    }

    /// Get key manager
    pub fn key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    /// Get key manager mutably
    pub fn key_manager_mut(&mut self) -> &mut KeyManager {
        &mut self.key_manager
    }

    /// Get hardware detector
    pub fn hw_detector(&self) -> &HwCryptoDetector {
        &self.hw_detector
    }

    /// Get hardware detector mutably
    pub fn hw_detector_mut(&mut self) -> &mut HwCryptoDetector {
        &mut self.hw_detector
    }

    /// Get deprecated algorithms in use
    pub fn deprecated_in_use(&self) -> Vec<&AlgorithmInfo> {
        self.algorithms
            .values()
            .filter(|a| a.is_deprecated() && a.use_count() > 0)
            .collect()
    }

    /// Get algorithms
    pub fn algorithms(&self) -> &BTreeMap<String, AlgorithmInfo> {
        &self.algorithms
    }

    /// Get total operations
    pub fn total_operations(&self) -> u64 {
        self.total_operations.load(Ordering::Relaxed)
    }

    /// Get total bytes
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }
}

impl Default for CryptoManager {
    fn default() -> Self {
        Self::new()
    }
}
