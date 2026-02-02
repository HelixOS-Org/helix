//! # Priority Management
//!
//! Defines priority levels and priority manipulation.

use core::cmp::Ordering;

/// Thread priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority {
    /// Static priority (0-139)
    /// 0-99: Real-time priorities
    /// 100-139: Normal priorities
    static_priority: u8,
    /// Dynamic priority adjustment
    dynamic_adjustment: i8,
}

impl Priority {
    /// Minimum priority (lowest)
    pub const MIN: Self = Self {
        static_priority: 139,
        dynamic_adjustment: 0,
    };

    /// Maximum priority (highest, real-time)
    pub const MAX: Self = Self {
        static_priority: 0,
        dynamic_adjustment: 0,
    };

    /// Default normal priority
    pub const DEFAULT: Self = Self {
        static_priority: 120,
        dynamic_adjustment: 0,
    };

    /// Idle priority
    pub const IDLE: Self = Self {
        static_priority: 139,
        dynamic_adjustment: 0,
    };

    /// Real-time threshold
    pub const REALTIME_THRESHOLD: u8 = 100;

    /// Create a new priority
    pub const fn new(static_priority: u8) -> Self {
        Self {
            static_priority: if static_priority > 139 {
                139
            } else {
                static_priority
            },
            dynamic_adjustment: 0,
        }
    }

    /// Create a real-time priority
    pub const fn realtime(priority: u8) -> Self {
        Self {
            static_priority: if priority >= Self::REALTIME_THRESHOLD {
                Self::REALTIME_THRESHOLD - 1
            } else {
                priority
            },
            dynamic_adjustment: 0,
        }
    }

    /// Create a normal priority (nice value: -20 to 19)
    pub fn normal(nice: i8) -> Self {
        let nice = nice.clamp(-20, 19);
        let static_priority = (120 + nice) as u8;
        Self {
            static_priority,
            dynamic_adjustment: 0,
        }
    }

    /// Get the static priority
    pub fn static_priority(&self) -> u8 {
        self.static_priority
    }

    /// Get the effective priority (static + dynamic)
    pub fn effective(&self) -> i16 {
        (self.static_priority as i16) + (self.dynamic_adjustment as i16)
    }

    /// Check if this is a real-time priority
    pub fn is_realtime(&self) -> bool {
        self.static_priority < Self::REALTIME_THRESHOLD
    }

    /// Boost priority (for interactive threads)
    pub fn boost(&mut self, amount: i8) {
        self.dynamic_adjustment = self.dynamic_adjustment.saturating_sub(amount);
    }

    /// Penalize priority (for CPU-bound threads)
    pub fn penalize(&mut self, amount: i8) {
        self.dynamic_adjustment = self.dynamic_adjustment.saturating_add(amount);
    }

    /// Reset dynamic adjustment
    pub fn reset_dynamic(&mut self) {
        self.dynamic_adjustment = 0;
    }

    /// Get nice value (for normal priorities)
    pub fn nice(&self) -> i8 {
        if self.is_realtime() {
            -20
        } else {
            (self.static_priority as i8) - 120
        }
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower number = higher priority
        other.effective().cmp(&self.effective())
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityClass {
    /// Idle (only runs when nothing else)
    Idle,
    /// Below normal
    BelowNormal,
    /// Normal
    Normal,
    /// Above normal
    AboveNormal,
    /// High
    High,
    /// Real-time
    Realtime,
}

impl PriorityClass {
    /// Convert to a base priority
    pub fn to_priority(self) -> Priority {
        match self {
            PriorityClass::Idle => Priority::new(139),
            PriorityClass::BelowNormal => Priority::new(130),
            PriorityClass::Normal => Priority::new(120),
            PriorityClass::AboveNormal => Priority::new(110),
            PriorityClass::High => Priority::new(105),
            PriorityClass::Realtime => Priority::realtime(50),
        }
    }
}

impl Default for PriorityClass {
    fn default() -> Self {
        PriorityClass::Normal
    }
}
