//! Compliance Framework
//!
//! Compliance frameworks and checks.

use alloc::string::String;
use alloc::vec::Vec;

/// Compliance framework
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplianceFramework {
    /// PCI-DSS
    PciDss,
    /// HIPAA
    Hipaa,
    /// SOX
    Sox,
    /// GDPR
    Gdpr,
    /// CIS Benchmarks
    Cis,
    /// NIST
    Nist,
    /// Custom
    Custom,
}

impl ComplianceFramework {
    /// Get framework name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::PciDss => "PCI-DSS",
            Self::Hipaa => "HIPAA",
            Self::Sox => "SOX",
            Self::Gdpr => "GDPR",
            Self::Cis => "CIS",
            Self::Nist => "NIST",
            Self::Custom => "Custom",
        }
    }

    /// Get all frameworks
    #[inline]
    pub fn all() -> &'static [ComplianceFramework] {
        &[
            Self::PciDss,
            Self::Hipaa,
            Self::Sox,
            Self::Gdpr,
            Self::Cis,
            Self::Nist,
            Self::Custom,
        ]
    }
}

/// Compliance check
#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    /// Check ID
    pub id: String,
    /// Framework
    pub framework: ComplianceFramework,
    /// Description
    pub description: String,
    /// Required audit rules
    pub required_rules: Vec<String>,
    /// Is passing
    pub passing: bool,
    /// Last checked
    pub last_checked: u64,
    /// Violations
    pub violations: Vec<String>,
}

impl ComplianceCheck {
    /// Create new check
    pub fn new(id: String, framework: ComplianceFramework, description: String) -> Self {
        Self {
            id,
            framework,
            description,
            required_rules: Vec::new(),
            passing: true,
            last_checked: 0,
            violations: Vec::new(),
        }
    }

    /// Add required rule
    #[inline(always)]
    pub fn add_required_rule(&mut self, rule: String) {
        self.required_rules.push(rule);
    }

    /// Add violation
    #[inline(always)]
    pub fn add_violation(&mut self, violation: String) {
        self.violations.push(violation);
        self.passing = false;
    }

    /// Update check
    #[inline(always)]
    pub fn update(&mut self, timestamp: u64) {
        self.last_checked = timestamp;
    }

    /// Reset violations
    #[inline(always)]
    pub fn reset(&mut self) {
        self.violations.clear();
        self.passing = true;
    }
}
