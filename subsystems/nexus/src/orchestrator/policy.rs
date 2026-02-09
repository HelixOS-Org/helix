//! Orchestrator Policy System
//!
//! System policies for balancing performance, power, security, and reliability.

// ============================================================================
// POLICY TYPES
// ============================================================================

/// Policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyType {
    /// Performance focus
    Performance,
    /// Power saving
    PowerSaving,
    /// Balanced
    Balanced,
    /// Security focus
    Security,
    /// Reliability focus
    Reliability,
}

impl PolicyType {
    /// Get policy name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::PowerSaving => "power_saving",
            Self::Balanced => "balanced",
            Self::Security => "security",
            Self::Reliability => "reliability",
        }
    }
}

// ============================================================================
// SYSTEM POLICY
// ============================================================================

/// System policy
#[derive(Debug, Clone)]
pub struct SystemPolicy {
    /// Policy type
    pub policy_type: PolicyType,
    /// Performance weight (0-100)
    pub performance_weight: u8,
    /// Power weight (0-100)
    pub power_weight: u8,
    /// Security weight (0-100)
    pub security_weight: u8,
    /// Reliability weight (0-100)
    pub reliability_weight: u8,
}

impl SystemPolicy {
    /// Create performance policy
    #[inline]
    pub fn performance() -> Self {
        Self {
            policy_type: PolicyType::Performance,
            performance_weight: 90,
            power_weight: 20,
            security_weight: 50,
            reliability_weight: 60,
        }
    }

    /// Create power saving policy
    #[inline]
    pub fn power_saving() -> Self {
        Self {
            policy_type: PolicyType::PowerSaving,
            performance_weight: 40,
            power_weight: 90,
            security_weight: 50,
            reliability_weight: 60,
        }
    }

    /// Create balanced policy
    #[inline]
    pub fn balanced() -> Self {
        Self {
            policy_type: PolicyType::Balanced,
            performance_weight: 60,
            power_weight: 60,
            security_weight: 60,
            reliability_weight: 60,
        }
    }

    /// Create security policy
    #[inline]
    pub fn security() -> Self {
        Self {
            policy_type: PolicyType::Security,
            performance_weight: 40,
            power_weight: 50,
            security_weight: 95,
            reliability_weight: 80,
        }
    }

    /// Create reliability policy
    #[inline]
    pub fn reliability() -> Self {
        Self {
            policy_type: PolicyType::Reliability,
            performance_weight: 50,
            power_weight: 50,
            security_weight: 70,
            reliability_weight: 95,
        }
    }

    /// Get total weight
    #[inline]
    pub fn total_weight(&self) -> u16 {
        self.performance_weight as u16
            + self.power_weight as u16
            + self.security_weight as u16
            + self.reliability_weight as u16
    }

    /// Normalize weights to sum to 100
    pub fn normalized_weights(&self) -> (f32, f32, f32, f32) {
        let total = self.total_weight() as f32;
        if total == 0.0 {
            return (25.0, 25.0, 25.0, 25.0);
        }
        (
            self.performance_weight as f32 / total * 100.0,
            self.power_weight as f32 / total * 100.0,
            self.security_weight as f32 / total * 100.0,
            self.reliability_weight as f32 / total * 100.0,
        )
    }
}

impl Default for SystemPolicy {
    fn default() -> Self {
        Self::balanced()
    }
}
