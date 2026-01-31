//! Helper functions for creating common invariants.

use alloc::format;

use super::invariant::{Invariant, InvariantResult};

/// Create a simple boolean invariant
pub fn bool_invariant(
    name: impl Into<alloc::string::String>,
    check: impl Fn() -> bool + Send + Sync + 'static,
    violation_message: impl Into<alloc::string::String> + Clone + Send + Sync + 'static,
) -> Invariant {
    Invariant::new(name, move || {
        InvariantResult::from_bool(check(), violation_message.clone())
    })
}

/// Create a range invariant
pub fn range_invariant<T: PartialOrd + core::fmt::Display + Clone + Send + Sync + 'static>(
    name: impl Into<alloc::string::String>,
    get_value: impl Fn() -> T + Send + Sync + 'static,
    min: T,
    max: T,
) -> Invariant {
    Invariant::new(name, move || {
        let value = get_value();
        if value >= min && value <= max {
            InvariantResult::ok()
        } else {
            InvariantResult::violated(format!("Value {} out of range [{}, {}]", value, min, max))
        }
    })
}
