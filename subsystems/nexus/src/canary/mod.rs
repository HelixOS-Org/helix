//! # Canary Invariants
//!
//! System invariant monitoring and violation detection.
//!
//! ## Key Features
//!
//! - **Invariant Definition**: Define system invariants
//! - **Continuous Monitoring**: Check invariants continuously
//! - **Violation Response**: Automatic response to violations
//! - **Health Integration**: Integrate with health system

#![allow(dead_code)]

extern crate alloc;

mod canary;
mod helpers;
mod invariant;
mod monitor;

// Re-export invariant types
// Re-export canary
pub use canary::Canary;
// Re-export helpers
pub use helpers::{bool_invariant, range_invariant};
pub use invariant::{Invariant, InvariantCheck, InvariantResult};
// Re-export monitor
pub use monitor::{CanaryMonitor, CanaryStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use super::*;

    #[test]
    fn test_invariant_result() {
        let ok = InvariantResult::ok();
        assert!(ok.holds);

        let violated = InvariantResult::violated("error");
        assert!(!violated.holds);
        assert_eq!(violated.message, Some("error".into()));
    }

    #[test]
    fn test_invariant() {
        let mut inv = Invariant::new("always_true", || InvariantResult::ok());

        let check = inv.check();
        assert!(!check.violated());
        assert_eq!(inv.total_checks, 1);
        assert_eq!(inv.consecutive_failures(), 0);
    }

    #[test]
    fn test_failing_invariant() {
        let mut inv = Invariant::new("always_false", || InvariantResult::violated("always fails"));

        inv.check();
        assert_eq!(inv.consecutive_failures(), 1);

        inv.check();
        assert_eq!(inv.consecutive_failures(), 2);
    }

    #[test]
    fn test_canary() {
        let canary = Canary::new(12345);
        assert!(canary.check());
        assert_eq!(canary.value(), 12345);

        // Simulate corruption
        canary.value.store(99999, Ordering::SeqCst);
        assert!(!canary.check());

        canary.reset();
        assert!(canary.check());
    }

    #[test]
    fn test_canary_monitor() {
        let mut monitor = CanaryMonitor::new();

        // Add canary
        monitor.add_canary("test", Canary::new(12345));
        assert!(monitor.all_canaries_intact());

        // Add invariant
        let inv = Invariant::new("test_inv", || InvariantResult::ok()).with_interval(0); // Always check
        monitor.add_invariant(inv);

        let results = monitor.check_all();
        assert_eq!(results.len(), 1);
        assert!(!results[0].violated());
    }

    #[test]
    fn test_bool_invariant() {
        let mut inv = bool_invariant("positive", || 5 > 0, "Value must be positive");

        let check = inv.check();
        assert!(!check.violated());
    }

    #[test]
    fn test_range_invariant() {
        let mut inv = range_invariant("in_range", || 50, 0, 100);

        let check = inv.check();
        assert!(!check.violated());
    }
}
