//! Prefetch prediction based on access patterns.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::pattern::PatternDetector;
use super::types::{AccessPattern, AccessRecord, AccessType};
use crate::core::NexusTimestamp;

// ============================================================================
// PREFETCH PREDICTOR
// ============================================================================

/// Predicts future memory accesses for prefetching
pub struct PrefetchPredictor {
    /// Pattern detector
    detector: PatternDetector,
    /// Prediction table
    predictions: BTreeMap<u64, PredictionEntry>,
    /// Max predictions
    max_predictions: usize,
    /// Accuracy tracking
    hits: AtomicU64,
    /// Miss tracking
    misses: AtomicU64,
}

/// Prediction table entry
#[derive(Debug, Clone)]
struct PredictionEntry {
    /// Predicted next address
    next_address: u64,
    /// Predicted stride
    stride: i64,
    /// Confidence
    confidence: f64,
    /// Correct predictions
    hits: u32,
    /// Total predictions
    total: u32,
}

impl PrefetchPredictor {
    /// Create new prefetch predictor
    pub fn new() -> Self {
        Self {
            detector: PatternDetector::new(500),
            predictions: BTreeMap::new(),
            max_predictions: 1024,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Record access and update predictions
    pub fn record_access(&mut self, address: u64, access_type: AccessType) {
        let record = AccessRecord {
            address,
            access_type,
            size: 8,
            timestamp: NexusTimestamp::now().raw(),
        };

        // Check if we predicted this access
        let page = address / 4096;
        if let Some(entry) = self.predictions.get_mut(&page) {
            let delta = (entry.next_address as i64 - address as i64).abs();
            if delta < 64 {
                // Hit!
                entry.hits += 1;
                self.hits.fetch_add(1, Ordering::Relaxed);
            } else {
                self.misses.fetch_add(1, Ordering::Relaxed);
            }
            entry.total += 1;
            entry.confidence = entry.hits as f64 / entry.total as f64;
        }

        self.detector.record(record);

        // Update predictions based on pattern
        self.update_predictions(address);
    }

    /// Update prediction table
    fn update_predictions(&mut self, current_address: u64) {
        let (pattern, confidence) = self.detector.detect_pattern();

        if !pattern.is_prefetchable() || confidence < 0.5 {
            return;
        }

        let stride = match pattern {
            AccessPattern::Sequential => 64,
            AccessPattern::ReverseSequential => -64,
            AccessPattern::Strided { stride } => stride,
            _ => return,
        };

        let page = current_address / 4096;
        let next_address = (current_address as i64 + stride) as u64;

        self.predictions.insert(page, PredictionEntry {
            next_address,
            stride,
            confidence,
            hits: 0,
            total: 0,
        });

        // Evict old predictions
        while self.predictions.len() > self.max_predictions {
            // Remove lowest confidence entry
            if let Some((&key, _)) = self
                .predictions
                .iter()
                .min_by(|a, b| a.1.confidence.partial_cmp(&b.1.confidence).unwrap())
            {
                self.predictions.remove(&key);
            }
        }
    }

    /// Get addresses to prefetch
    pub fn get_prefetch_addresses(&self, current_address: u64, count: usize) -> Vec<u64> {
        let (pattern, confidence) = self.detector.detect_pattern();

        if !pattern.is_prefetchable() || confidence < 0.5 {
            return Vec::new();
        }

        let stride = match pattern {
            AccessPattern::Sequential => 64i64,
            AccessPattern::ReverseSequential => -64,
            AccessPattern::Strided { stride } => stride,
            _ => return Vec::new(),
        };

        (1..=count)
            .map(|i| (current_address as i64 + stride * i as i64) as u64)
            .collect()
    }

    /// Get accuracy
    pub fn accuracy(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Get current pattern
    pub fn current_pattern(&self) -> (AccessPattern, f64) {
        self.detector.detect_pattern()
    }
}

impl Default for PrefetchPredictor {
    fn default() -> Self {
        Self::new()
    }
}
