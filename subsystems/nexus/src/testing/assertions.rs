//! Test assertion helpers.

use alloc::format;
use alloc::string::String;

/// Assert two values are equal
pub fn assert_eq<T: PartialEq + core::fmt::Debug>(actual: T, expected: T) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("Assertion failed: {:?} != {:?}", actual, expected))
    }
}

/// Assert two values are not equal
pub fn assert_ne<T: PartialEq + core::fmt::Debug>(actual: T, expected: T) -> Result<(), String> {
    if actual != expected {
        Ok(())
    } else {
        Err(format!("Assertion failed: {:?} == {:?}", actual, expected))
    }
}

/// Assert a condition is true
pub fn assert_true(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(format!("Assertion failed: {}", message))
    }
}

/// Assert a condition is false
pub fn assert_false(condition: bool, message: &str) -> Result<(), String> {
    if !condition {
        Ok(())
    } else {
        Err(format!("Assertion failed: expected false, {}", message))
    }
}

/// Assert a value is within range
pub fn assert_in_range<T: PartialOrd + core::fmt::Debug>(
    value: T,
    min: T,
    max: T,
) -> Result<(), String> {
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(format!(
            "Assertion failed: {:?} not in range [{:?}, {:?}]",
            value, min, max
        ))
    }
}

/// Assert a result is Ok
pub fn assert_ok<T, E: core::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|e| format!("Expected Ok, got Err({:?})", e))
}

/// Assert a result is Err
pub fn assert_err<T: core::fmt::Debug, E>(result: Result<T, E>) -> Result<E, String> {
    match result {
        Ok(v) => Err(format!("Expected Err, got Ok({:?})", v)),
        Err(e) => Ok(e),
    }
}
