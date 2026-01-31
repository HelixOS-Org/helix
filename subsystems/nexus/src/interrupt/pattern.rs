//! Interrupt pattern detection
//!
//! This module provides pattern detection for interrupts, identifying
//! periodic, burst, random, and correlated patterns for predictive optimization.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::Irq;

/// Pattern types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterruptPattern {
    /// Periodic interrupts
    Periodic { period_ns: u64 },
    /// Burst interrupts
    Burst { avg_burst_size: u32 },
    /// Random interrupts
    Random,
    /// Correlated with another IRQ
    Correlated { other_irq: Irq },
}

/// Detects interrupt patterns
pub struct InterruptPatternDetector {
    /// History per IRQ
    history: BTreeMap<Irq, Vec<u64>>,
    /// Detected patterns
    patterns: BTreeMap<Irq, InterruptPattern>,
    /// Pattern confidence
    confidence: BTreeMap<Irq, f64>,
    /// Max history
    max_history: usize,
}

impl InterruptPatternDetector {
    /// Create new detector
    pub fn new(max_history: usize) -> Self {
        Self {
            history: BTreeMap::new(),
            patterns: BTreeMap::new(),
            confidence: BTreeMap::new(),
            max_history,
        }
    }

    /// Record interrupt timestamp
    pub fn record(&mut self, irq: Irq, timestamp: u64) {
        let history = self.history.entry(irq).or_insert_with(Vec::new);
        history.push(timestamp);

        if history.len() > self.max_history {
            history.remove(0);
        }

        if history.len() >= 10 {
            self.detect_pattern(irq);
        }
    }

    /// Detect pattern for IRQ
    fn detect_pattern(&mut self, irq: Irq) {
        if let Some(history) = self.history.get(&irq) {
            if history.len() < 10 {
                return;
            }

            // Calculate inter-arrival times
            let deltas: Vec<u64> = history.windows(2).map(|w| w[1] - w[0]).collect();

            // Check for periodic pattern
            if let Some(period) = self.detect_periodic(&deltas) {
                self.patterns
                    .insert(irq, InterruptPattern::Periodic { period_ns: period });
                self.confidence.insert(irq, 0.9);
                return;
            }

            // Check for burst pattern
            if let Some(burst_size) = self.detect_burst(&deltas) {
                self.patterns.insert(irq, InterruptPattern::Burst {
                    avg_burst_size: burst_size,
                });
                self.confidence.insert(irq, 0.8);
                return;
            }

            // Default to random
            self.patterns.insert(irq, InterruptPattern::Random);
            self.confidence.insert(irq, 0.5);
        }
    }

    /// Detect periodic pattern
    fn detect_periodic(&self, deltas: &[u64]) -> Option<u64> {
        if deltas.is_empty() {
            return None;
        }

        let mean: f64 = deltas.iter().map(|&d| d as f64).sum::<f64>() / deltas.len() as f64;
        let variance: f64 = deltas
            .iter()
            .map(|&d| {
                let diff = d as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / deltas.len() as f64;

        let cv = variance.sqrt() / mean; // Coefficient of variation

        if cv < 0.1 { Some(mean as u64) } else { None }
    }

    /// Detect burst pattern
    fn detect_burst(&self, deltas: &[u64]) -> Option<u32> {
        if deltas.len() < 5 {
            return None;
        }

        let mean: f64 = deltas.iter().map(|&d| d as f64).sum::<f64>() / deltas.len() as f64;
        let burst_threshold = mean * 0.1;

        let mut burst_count = 0u32;
        let mut in_burst = false;
        let mut current_burst = 0u32;

        for &delta in deltas {
            if (delta as f64) < burst_threshold {
                if !in_burst {
                    in_burst = true;
                    burst_count += 1;
                }
                current_burst += 1;
            } else {
                in_burst = false;
                current_burst = 0;
            }
        }

        if burst_count >= 3 {
            Some((deltas.len() as u32 / burst_count).max(1))
        } else {
            None
        }
    }

    /// Get detected pattern
    pub fn get_pattern(&self, irq: Irq) -> Option<InterruptPattern> {
        self.patterns.get(&irq).copied()
    }

    /// Get pattern confidence
    pub fn get_confidence(&self, irq: Irq) -> f64 {
        self.confidence.get(&irq).copied().unwrap_or(0.0)
    }

    /// Predict next interrupt time
    pub fn predict_next(&self, irq: Irq) -> Option<u64> {
        match self.patterns.get(&irq)? {
            InterruptPattern::Periodic { period_ns } => {
                let last = self.history.get(&irq)?.last()?;
                Some(last + period_ns)
            },
            _ => None,
        }
    }
}

impl Default for InterruptPatternDetector {
    fn default() -> Self {
        Self::new(100)
    }
}
