//! Policy Analysis
//!
//! LSM policy analysis and statistics.

use alloc::string::String;
use alloc::vec::Vec;

use super::{ObjectClass, Permission};

/// Policy rule
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Source type
    pub source: String,
    /// Target type
    pub target: String,
    /// Object class
    pub class: ObjectClass,
    /// Permissions
    pub permissions: Vec<Permission>,
    /// Is allow (vs deny)
    pub is_allow: bool,
}

impl PolicyRule {
    /// Create new rule
    pub fn new(
        source: String,
        target: String,
        class: ObjectClass,
        permissions: Vec<Permission>,
    ) -> Self {
        Self {
            source,
            target,
            class,
            permissions,
            is_allow: true,
        }
    }

    /// Create deny rule
    pub fn deny(
        source: String,
        target: String,
        class: ObjectClass,
        permissions: Vec<Permission>,
    ) -> Self {
        Self {
            source,
            target,
            class,
            permissions,
            is_allow: false,
        }
    }
}

/// Policy statistics
#[derive(Debug, Clone, Default)]
pub struct PolicyStats {
    /// Total rules
    pub total_rules: usize,
    /// Allow rules
    pub allow_rules: usize,
    /// Deny rules
    pub deny_rules: usize,
    /// Type count
    pub type_count: usize,
    /// Role count
    pub role_count: usize,
    /// User count
    pub user_count: usize,
    /// Unconfined types
    pub unconfined_types: usize,
}

/// Policy complexity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyComplexity {
    /// Minimal
    Minimal,
    /// Simple
    Simple,
    /// Moderate
    Moderate,
    /// Complex
    Complex,
    /// VeryComplex
    VeryComplex,
}

impl PolicyComplexity {
    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::VeryComplex => "very_complex",
        }
    }

    /// From rule count
    pub fn from_rule_count(count: usize) -> Self {
        match count {
            0..=100 => Self::Minimal,
            101..=1000 => Self::Simple,
            1001..=10000 => Self::Moderate,
            10001..=100000 => Self::Complex,
            _ => Self::VeryComplex,
        }
    }
}
