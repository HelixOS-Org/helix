//! Kernel-level adversarial defense system.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::adversarial::defense::InputPurifier;
use crate::adversarial::detection::{FeatureSqueezer, LIDDetector};
use crate::adversarial::ensemble::EnsembleDefense;
use crate::adversarial::types::{AdversarialEvent, DefenseStats, DetectionResult, KernelAttackType};

/// Kernel adversarial defense manager
pub struct KernelAdversarialDefense {
    /// Feature squeezer
    pub squeezer: FeatureSqueezer,
    /// LID detector
    pub lid_detector: LIDDetector,
    /// Input purifier
    pub purifier: InputPurifier,
    /// Ensemble defense
    pub ensemble: EnsembleDefense,
    /// Event log
    pub events: Vec<AdversarialEvent>,
    /// Is defense active?
    pub active: bool,
    /// Reference samples for detection
    reference_samples: VecDeque<Vec<f64>>,
}

impl KernelAdversarialDefense {
    /// Create a new kernel defense system
    pub fn new() -> Self {
        Self {
            squeezer: FeatureSqueezer::new(),
            lid_detector: LIDDetector::new(20),
            purifier: InputPurifier::new(),
            ensemble: EnsembleDefense::new(5),
            events: Vec::new(),
            active: true,
            reference_samples: VecDeque::new(),
        }
    }

    /// Add reference sample
    pub fn add_reference(&mut self, sample: Vec<f64>) {
        self.reference_samples.push_back(sample);

        // Limit size
        while self.reference_samples.len() > 1000 {
            self.reference_samples.pop_front();
        }
    }

    /// Detect potential attack
    pub fn detect(&mut self, input: &[f64], timestamp: u64) -> DetectionResult {
        if !self.active {
            return DetectionResult::new(false, 0.0, String::from("Disabled"));
        }

        // LID detection
        let lid_result = if !self.reference_samples.is_empty() {
            self.lid_detector.detect(input, &self.reference_samples)
        } else {
            DetectionResult::new(false, 0.0, String::from("NoReference"))
        };

        // Combined detection
        let is_adversarial = lid_result.is_adversarial;

        if is_adversarial {
            let event = AdversarialEvent {
                timestamp,
                attack_type: KernelAttackType::Evasion,
                component: String::from("unknown"),
                severity: lid_result.confidence,
                blocked: true,
            };
            self.events.push(event);
        }

        lid_result
    }

    /// Purify suspicious input
    pub fn purify<F>(&self, input: &[f64], energy_fn: F, seed: u64) -> Vec<f64>
    where
        F: FnMut(&[f64]) -> (f64, Vec<f64>),
    {
        if !self.active {
            return input.to_vec();
        }

        self.purifier.purify(input, energy_fn, seed)
    }

    /// Get defense statistics
    pub fn get_stats(&self) -> DefenseStats {
        let total = self.events.len();
        let blocked = self.events.iter().filter(|e| e.blocked).count();

        let by_type: BTreeMap<String, usize> =
            self.events.iter().fold(BTreeMap::new(), |mut acc, e| {
                let key = format!("{:?}", e.attack_type);
                *acc.entry(key).or_insert(0) += 1;
                acc
            });

        DefenseStats {
            total_attacks: total,
            blocked_attacks: blocked,
            block_rate: if total > 0 {
                blocked as f64 / total as f64
            } else {
                1.0
            },
            attacks_by_type: by_type,
        }
    }
}

impl Default for KernelAdversarialDefense {
    fn default() -> Self {
        Self::new()
    }
}
