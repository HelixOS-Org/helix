//! File Capabilities
//!
//! File capability set management.

use super::CapabilitySet;

/// File capability set
#[derive(Debug, Clone)]
pub struct FileCaps {
    /// Permitted set
    pub permitted: CapabilitySet,
    /// Inheritable set
    pub inheritable: CapabilitySet,
    /// Effective flag
    pub effective: bool,
    /// Root user ID (for namespace)
    pub rootid: Option<u32>,
    /// Version
    pub version: u8,
}

impl FileCaps {
    /// Create new file caps
    pub fn new() -> Self {
        Self {
            permitted: CapabilitySet::new(),
            inheritable: CapabilitySet::new(),
            effective: false,
            rootid: None,
            version: 3,
        }
    }

    /// Create from sets
    pub fn from_sets(
        permitted: CapabilitySet,
        inheritable: CapabilitySet,
        effective: bool,
    ) -> Self {
        Self {
            permitted,
            inheritable,
            effective,
            rootid: None,
            version: 3,
        }
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.permitted.is_empty() && self.inheritable.is_empty()
    }

    /// Get risk score
    pub fn risk_score(&self) -> f32 {
        let mut score = 0.0f32;

        for cap in self.permitted.iter() {
            score += cap.risk_level().score() as f32;
        }

        if self.effective {
            score *= 1.5;
        }

        score.min(100.0)
    }
}

impl Default for FileCaps {
    fn default() -> Self {
        Self::new()
    }
}
