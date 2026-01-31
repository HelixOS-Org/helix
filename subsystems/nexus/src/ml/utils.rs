//! ML Utilities and Model Registry
//!
//! Common utilities, RNG, and model registry.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::math;

// ============================================================================
// UTILITIES
// ============================================================================

/// Sigmoid function
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + math::exp(-x))
}

/// Simple LCG random number generator
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// Create new LCG with seed
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Get next random u64
    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    /// Get next random f64 in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        self.next() as f64 / u64::MAX as f64
    }
}

// ============================================================================
// MODEL REGISTRY
// ============================================================================

/// Registry for ML models
pub struct ModelRegistry {
    /// Named models
    models: BTreeMap<String, ModelEntry>,
}

/// A registered model entry
struct ModelEntry {
    /// Model type
    model_type: String,
    /// Created timestamp
    created: u64,
    /// Last used
    last_used: u64,
    /// Usage count
    usage_count: u64,
}

impl ModelRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
        }
    }

    /// Register a model
    pub fn register(&mut self, name: impl Into<String>, model_type: impl Into<String>) {
        let entry = ModelEntry {
            model_type: model_type.into(),
            created: 0, // Would use timestamp
            last_used: 0,
            usage_count: 0,
        };
        self.models.insert(name.into(), entry);
    }

    /// Record usage
    pub fn record_usage(&mut self, name: &str) {
        if let Some(entry) = self.models.get_mut(name) {
            entry.usage_count += 1;
            entry.last_used = 0; // Would use timestamp
        }
    }

    /// List models
    pub fn list(&self) -> Vec<&str> {
        self.models.keys().map(|s| s.as_str()).collect()
    }

    /// Get model type
    pub fn model_type(&self, name: &str) -> Option<&str> {
        self.models.get(name).map(|e| e.model_type.as_str())
    }

    /// Remove model
    pub fn remove(&mut self, name: &str) -> bool {
        self.models.remove(name).is_some()
    }

    /// Model count
    pub fn count(&self) -> usize {
        self.models.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
