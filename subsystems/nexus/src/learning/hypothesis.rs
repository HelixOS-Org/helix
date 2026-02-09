//! Hypothesis testing and management
//!
//! This module provides hypothesis testing capabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{HypothesisId, Timestamp};

/// Hypothesis status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    /// Proposed
    Proposed,
    /// Testing
    Testing,
    /// Confirmed
    Confirmed,
    /// Rejected
    Rejected,
    /// Inconclusive
    Inconclusive,
}

/// Hypothesis
#[derive(Debug, Clone)]
pub struct Hypothesis {
    /// Hypothesis ID
    pub id: HypothesisId,
    /// Statement
    pub statement: String,
    /// Status
    pub status: HypothesisStatus,
    /// Supporting evidence
    pub supporting: u64,
    /// Contradicting evidence
    pub contradicting: u64,
    /// Confidence (0-1)
    pub confidence: f32,
    /// Created
    pub created: Timestamp,
    /// Last tested
    pub last_tested: Timestamp,
}

impl Hypothesis {
    /// Create new hypothesis
    pub fn new(id: HypothesisId, statement: String) -> Self {
        Self {
            id,
            statement,
            status: HypothesisStatus::Proposed,
            supporting: 0,
            contradicting: 0,
            confidence: 0.5,
            created: Timestamp::new(0),
            last_tested: Timestamp::new(0),
        }
    }

    /// Add evidence
    pub fn add_evidence(&mut self, supports: bool, timestamp: Timestamp) {
        if supports {
            self.supporting += 1;
        } else {
            self.contradicting += 1;
        }
        self.last_tested = timestamp;

        // Update confidence
        let total = self.supporting + self.contradicting;
        if total > 0 {
            self.confidence = self.supporting as f32 / total as f32;
        }

        // Update status
        if total >= 10 {
            self.status = if self.confidence >= 0.8 {
                HypothesisStatus::Confirmed
            } else if self.confidence <= 0.2 {
                HypothesisStatus::Rejected
            } else if total >= 50 {
                HypothesisStatus::Inconclusive
            } else {
                HypothesisStatus::Testing
            };
        }
    }
}

/// Hypothesis manager
pub struct HypothesisManager {
    /// Hypotheses
    hypotheses: BTreeMap<HypothesisId, Hypothesis>,
    /// Counter
    counter: AtomicU64,
}

impl HypothesisManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            hypotheses: BTreeMap::new(),
            counter: AtomicU64::new(0),
        }
    }

    /// Create hypothesis
    #[inline]
    pub fn create(&mut self, statement: String) -> HypothesisId {
        let id = HypothesisId(self.counter.fetch_add(1, Ordering::Relaxed));
        let hypothesis = Hypothesis::new(id, statement);
        self.hypotheses.insert(id, hypothesis);
        id
    }

    /// Get hypothesis
    #[inline(always)]
    pub fn get(&self, id: HypothesisId) -> Option<&Hypothesis> {
        self.hypotheses.get(&id)
    }

    /// Add evidence
    #[inline]
    pub fn add_evidence(&mut self, id: HypothesisId, supports: bool, timestamp: u64) {
        if let Some(h) = self.hypotheses.get_mut(&id) {
            h.add_evidence(supports, Timestamp::new(timestamp));
        }
    }

    /// Find confirmed
    #[inline]
    pub fn find_confirmed(&self) -> Vec<&Hypothesis> {
        self.hypotheses
            .values()
            .filter(|h| h.status == HypothesisStatus::Confirmed)
            .collect()
    }

    /// Count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.hypotheses.len()
    }
}

impl Default for HypothesisManager {
    fn default() -> Self {
        Self::new()
    }
}
