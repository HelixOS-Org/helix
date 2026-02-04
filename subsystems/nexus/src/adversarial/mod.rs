//! # Adversarial Defense Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary adversarial robustness system that
//! protects kernel AI models from adversarial attacks, ensuring security
//! and reliability of AI-powered kernel decisions.
//!
//! ## Key Features
//!
//! - **Adversarial Attack Detection**: Detect malicious inputs
//! - **Adversarial Training**: Train models to be robust
//! - **Input Purification**: Clean potentially adversarial inputs
//! - **Certified Defenses**: Provable robustness guarantees
//! - **Ensemble Diversity**: Multiple models for robustness
//! - **Anomaly Detection**: Detect out-of-distribution inputs
//!
//! ## Kernel Applications
//!
//! - Protect scheduler from adversarial workloads
//! - Secure resource allocation from manipulation
//! - Defend intrusion detection from evasion attacks
//! - Robust memory management under adversarial conditions

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod auto_attack;
mod cw;
mod defense;
mod detection;
mod ensemble;
mod fgsm;
mod kernel_defense;
mod perturbation;
mod pgd;
mod types;
mod utils;

// ============================================================================
// PUBLIC RE-EXPORTS
// ============================================================================

// Types and constants
pub use types::{
    AdversarialEvent, DefenseStats, DetectionResult, KernelAttackType, PerturbationType,
    DEFAULT_EPSILON, DETECTION_SAMPLES, ENSEMBLE_SIZE, MAX_ATTACK_ITER, MAX_INPUT_DIM,
};

// Perturbation
pub use perturbation::Perturbation;

// Attack implementations
pub use auto_attack::AutoAttack;
pub use cw::CWAttack;
pub use fgsm::FGSM;
pub use pgd::PGD;

// Detection mechanisms
pub use detection::{FeatureSqueezer, LIDDetector, MahalanobisDetector};

// Defense mechanisms
pub use defense::{AdversarialTraining, InputPurifier, RandomizedSmoothing};

// Ensemble defense
pub use ensemble::EnsembleDefense;

// Kernel-level defense
pub use kernel_defense::KernelAdversarialDefense;

// Utility functions (internal use, but exposed for testing)
pub use utils::{box_muller, inv_normal_cdf, lcg_next, sign};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perturbation() {
        let mut pert = Perturbation::new(10, PerturbationType::LInf, 0.1);

        pert.delta = vec![0.2, -0.2, 0.05, 0.0, 0.15, -0.1, 0.0, 0.0, 0.0, 0.0];
        pert.project();

        for &d in &pert.delta {
            assert!(d >= -0.1 && d <= 0.1);
        }
    }

    #[test]
    fn test_l2_projection() {
        let mut pert = Perturbation::new(3, PerturbationType::L2, 1.0);

        pert.delta = vec![1.0, 1.0, 1.0];
        pert.project();

        assert!(pert.l2_norm() <= 1.0 + 1e-10);
    }

    #[test]
    fn test_fgsm() {
        let fgsm = FGSM::new(0.1);
        let input = vec![0.5; 10];
        let gradient = vec![1.0, -1.0, 0.5, -0.5, 0.0, 1.0, -1.0, 0.5, -0.5, 0.0];

        let pert = fgsm.attack(&input, &gradient);

        assert_eq!(pert.delta.len(), 10);
        assert!((pert.delta[0] - 0.1).abs() < 1e-10);
        assert!((pert.delta[1] + 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_pgd() {
        let pgd = PGD::new(0.1, 0.01, 10);
        let input = vec![0.5; 10];

        let pert = pgd.attack(&input, |x| vec![1.0; x.len()], 12345);

        assert_eq!(pert.delta.len(), 10);
        assert!(pert.linf_norm() <= 0.1 + 1e-10);
    }

    #[test]
    fn test_cw_attack() {
        let cw = CWAttack::new();

        assert!(cw.max_iterations > 0);
        assert!(cw.learning_rate > 0.0);
    }

    #[test]
    fn test_auto_attack() {
        let auto = AutoAttack::new(0.1);
        let input = vec![0.5; 10];

        let pert = auto.attack(&input, |x| vec![1.0; x.len()], 12345);

        assert_eq!(pert.delta.len(), 10);
    }

    #[test]
    fn test_feature_squeezer() {
        let squeezer = FeatureSqueezer::new();
        let input = vec![0.5, 0.25, 0.75, 0.125];

        let squeezed = squeezer.squeeze_bits(&input);

        assert_eq!(squeezed.len(), input.len());
    }

    #[test]
    fn test_feature_squeezer_blur() {
        let squeezer = FeatureSqueezer::new();
        let input = vec![0.0, 1.0, 0.0, 1.0, 0.0];

        let blurred = squeezer.squeeze_blur(&input);

        assert_eq!(blurred.len(), input.len());
        // Blurred values should be smoothed
        assert!(blurred[0] < 0.5);
    }

    #[test]
    fn test_lid_detector() {
        let mut detector = LIDDetector::new(5);

        let clean_samples: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0; 5]).collect();

        detector.fit(&clean_samples);

        let test = vec![0.5; 5];
        let result = detector.detect(&test, &clean_samples);

        assert!(result.scores.contains_key("lid"));
    }

    #[test]
    fn test_mahalanobis_detector() {
        let mut detector = MahalanobisDetector::new(5);

        let features: Vec<Vec<f64>> = (0..100).map(|i| vec![i as f64 / 100.0; 5]).collect();
        let labels: Vec<usize> = (0..100).map(|i| i % 3).collect();

        detector.fit(&features, &labels);

        assert_eq!(detector.means.len(), 3);
    }

    #[test]
    fn test_input_purifier() {
        let purifier = InputPurifier::new();
        let input = vec![0.5; 10];

        let purified = purifier.purify_median(&input, 3);

        assert_eq!(purified.len(), input.len());
    }

    #[test]
    fn test_randomized_smoothing() {
        let smoother = RandomizedSmoothing::new(0.25);
        let input = vec![0.5; 10];

        let (pred, confidence) = smoother.predict(&input, |_| 0, 12345);

        assert_eq!(pred, 0);
        assert!(confidence > 0.0);
    }

    #[test]
    fn test_adversarial_training() {
        let trainer = AdversarialTraining::new(0.1);
        let inputs = vec![vec![0.5; 10], vec![0.3; 10]];

        let adv_batch = trainer.generate_adversarial_batch(&inputs, |x| vec![1.0; x.len()], 12345);

        assert_eq!(adv_batch.len(), 2);
    }

    #[test]
    fn test_ensemble_defense() {
        let mut ensemble = EnsembleDefense::new(3);

        let predictions = vec![vec![0.9, 0.1], vec![0.8, 0.2], vec![0.85, 0.15]];

        let avg = ensemble.vote(&predictions);

        assert_eq!(avg.len(), 2);
        assert!(avg[0] > 0.8);
    }

    #[test]
    fn test_ensemble_variance() {
        let mut ensemble = EnsembleDefense::new(3);

        let predictions = vec![vec![0.9, 0.1], vec![0.8, 0.2], vec![0.85, 0.15]];

        ensemble.vote(&predictions);
        let variance = ensemble.prediction_variance();

        assert_eq!(variance.len(), 2);
        assert!(variance[0] < 0.1);
    }

    #[test]
    fn test_kernel_adversarial_defense() {
        let mut defense = KernelAdversarialDefense::new();

        // Add reference samples
        for i in 0..10 {
            defense.add_reference(vec![i as f64 / 10.0; 5]);
        }

        let input = vec![0.5; 5];
        let result = defense.detect(&input, 1000);

        assert!(result.method.len() > 0);
    }

    #[test]
    fn test_defense_stats() {
        let defense = KernelAdversarialDefense::new();
        let stats = defense.get_stats();

        assert_eq!(stats.total_attacks, 0);
        assert!((stats.block_rate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_inv_normal_cdf() {
        // Should be approximately 0 for p=0.5
        assert!(inv_normal_cdf(0.5).abs() < 0.1);

        // Should be positive for p>0.5
        assert!(inv_normal_cdf(0.9) > 0.0);

        // Should be negative for p<0.5
        assert!(inv_normal_cdf(0.1) < 0.0);
    }
}
