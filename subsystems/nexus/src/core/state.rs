//! NEXUS system state.

/// State of the NEXUS system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NexusState {
    /// Not yet initialized
    Uninitialized = 0,
    /// Currently initializing
    Initializing  = 1,
    /// Running normally
    Running       = 2,
    /// Temporarily paused
    Paused        = 3,
    /// Operating in degraded mode
    Degraded      = 4,
    /// Currently healing
    Healing       = 5,
    /// Shutting down
    ShuttingDown  = 6,
    /// Stopped
    Stopped       = 7,
}

impl NexusState {
    /// Create from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Uninitialized),
            1 => Some(Self::Initializing),
            2 => Some(Self::Running),
            3 => Some(Self::Paused),
            4 => Some(Self::Degraded),
            5 => Some(Self::Healing),
            6 => Some(Self::ShuttingDown),
            7 => Some(Self::Stopped),
            _ => None,
        }
    }

    /// Check if operational
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Running | Self::Degraded | Self::Healing)
    }

    /// Check if accepting events
    pub fn accepts_events(&self) -> bool {
        matches!(
            self,
            Self::Running | Self::Degraded | Self::Healing | Self::Paused
        )
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Uninitialized => "Uninitialized",
            Self::Initializing => "Initializing",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Degraded => "Degraded",
            Self::Healing => "Healing",
            Self::ShuttingDown => "ShuttingDown",
            Self::Stopped => "Stopped",
        }
    }
}
