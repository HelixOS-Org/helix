//! NEXUS intelligence levels.

/// Intelligence level of NEXUS
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NexusLevel {
    /// NEXUS is disabled
    Disabled   = 0,
    /// Passive monitoring only
    Monitoring = 1,
    /// Detection of issues
    Detection  = 2,
    /// Prediction of future issues
    Prediction = 3,
    /// Automatic correction
    Correction = 4,
    /// Self-healing with micro-rollback
    Healing    = 5,
    /// Full autonomous operation
    Autonomous = 6,
}

impl NexusLevel {
    /// Get level from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::Monitoring),
            2 => Some(Self::Detection),
            3 => Some(Self::Prediction),
            4 => Some(Self::Correction),
            5 => Some(Self::Healing),
            6 => Some(Self::Autonomous),
            _ => None,
        }
    }

    /// Check if monitoring is enabled
    pub fn is_monitoring(&self) -> bool {
        *self >= Self::Monitoring
    }

    /// Check if detection is enabled
    pub fn is_detecting(&self) -> bool {
        *self >= Self::Detection
    }

    /// Check if prediction is enabled
    pub fn is_predicting(&self) -> bool {
        *self >= Self::Prediction
    }

    /// Check if correction is enabled
    pub fn is_correcting(&self) -> bool {
        *self >= Self::Correction
    }

    /// Check if healing is enabled
    pub fn is_healing(&self) -> bool {
        *self >= Self::Healing
    }

    /// Check if autonomous operation is enabled
    pub fn is_autonomous(&self) -> bool {
        *self >= Self::Autonomous
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Monitoring => "Monitoring",
            Self::Detection => "Detection",
            Self::Prediction => "Prediction",
            Self::Correction => "Correction",
            Self::Healing => "Healing",
            Self::Autonomous => "Autonomous",
        }
    }
}

impl Default for NexusLevel {
    fn default() -> Self {
        Self::Healing
    }
}
