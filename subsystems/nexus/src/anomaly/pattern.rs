//! Anomaly pattern library

#![allow(dead_code)]

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::types::AnomalySeverity;
use crate::math;

// ============================================================================
// ANOMALY PATTERN
// ============================================================================

/// A known pattern that indicates a problem
#[derive(Debug, Clone)]
pub struct AnomalyPattern {
    /// Pattern ID
    pub id: u32,
    /// Pattern name
    pub name: String,
    /// Description
    pub description: String,
    /// Metrics involved
    pub metrics: Vec<String>,
    /// Pattern sequence (simplified)
    pub sequence: Vec<f64>,
    /// Severity when matched
    pub severity: AnomalySeverity,
}

// ============================================================================
// PATTERN LIBRARY
// ============================================================================

/// Library of known anomaly patterns
pub struct PatternLibrary {
    patterns: Vec<AnomalyPattern>,
}

impl PatternLibrary {
    /// Create a new pattern library
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Add a pattern
    #[inline(always)]
    pub fn add(&mut self, pattern: AnomalyPattern) {
        self.patterns.push(pattern);
    }

    /// Match against a sequence
    #[inline]
    pub fn match_pattern(&self, metric: &str, values: &[f64]) -> Option<&AnomalyPattern> {
        for pattern in &self.patterns {
            if pattern.metrics.contains(&metric.to_string()) {
                // Simple pattern matching (DTW would be better but complex)
                if self.simple_match(&pattern.sequence, values) {
                    return Some(pattern);
                }
            }
        }
        None
    }

    /// Simple pattern matching
    fn simple_match(&self, pattern: &[f64], values: &[f64]) -> bool {
        if pattern.len() > values.len() {
            return false;
        }

        // Check last N values against pattern
        let offset = values.len() - pattern.len();
        let window = &values[offset..];

        // Calculate correlation
        let corr = self.correlation(pattern, window);
        corr > 0.8
    }

    /// Calculate correlation coefficient
    fn correlation(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let n = a.len() as f64;
        let sum_a: f64 = a.iter().sum();
        let sum_b: f64 = b.iter().sum();
        let sum_ab: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let sum_a2: f64 = a.iter().map(|x| x * x).sum();
        let sum_b2: f64 = b.iter().map(|x| x * x).sum();

        let numerator = n * sum_ab - sum_a * sum_b;
        let denominator = math::sqrt((n * sum_a2 - sum_a * sum_a) * (n * sum_b2 - sum_b * sum_b));

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
}

impl Default for PatternLibrary {
    fn default() -> Self {
        Self::new()
    }
}
