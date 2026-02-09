//! Domain Definitions
//!
//! Cognitive domain identifiers and flow control.

#![allow(dead_code)]

// ============================================================================
// DOMAIN ENUM
// ============================================================================

/// Domain identifier for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    /// Perception domain
    Sense,
    /// Comprehension domain
    Understand,
    /// Reasoning domain
    Reason,
    /// Decision domain
    Decide,
    /// Execution domain
    Act,
    /// Memory domain
    Memory,
    /// Reflection domain
    Reflect,
    /// Core infrastructure
    Core,
    /// Broadcast to all
    Broadcast,
}

impl Domain {
    /// Get domain name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sense => "sense",
            Self::Understand => "understand",
            Self::Reason => "reason",
            Self::Decide => "decide",
            Self::Act => "act",
            Self::Memory => "memory",
            Self::Reflect => "reflect",
            Self::Core => "core",
            Self::Broadcast => "broadcast",
        }
    }

    /// All domains (excluding Broadcast and Core)
    #[inline]
    pub const fn cognitive_domains() -> [Domain; 7] {
        [
            Self::Sense,
            Self::Understand,
            Self::Reason,
            Self::Decide,
            Self::Act,
            Self::Memory,
            Self::Reflect,
        ]
    }

    /// Check if message flow is allowed
    pub fn can_send_to(&self, target: &Domain) -> bool {
        // Enforce unidirectional flow
        match (self, target) {
            // Perception → Comprehension
            (Domain::Sense, Domain::Understand) => true,
            // Comprehension → Reasoning, Memory
            (Domain::Understand, Domain::Reason) => true,
            (Domain::Understand, Domain::Memory) => true,
            // Reasoning → Decision, Memory
            (Domain::Reason, Domain::Decide) => true,
            (Domain::Reason, Domain::Memory) => true,
            // Decision → Execution, Memory
            (Domain::Decide, Domain::Act) => true,
            (Domain::Decide, Domain::Memory) => true,
            // Execution → Memory
            (Domain::Act, Domain::Memory) => true,
            // Reflection can read from all (observation)
            (_, Domain::Reflect) => true,
            // Memory can be read by all
            (Domain::Memory, _) => true,
            // Core can communicate with all
            (Domain::Core, _) => true,
            (_, Domain::Core) => true,
            // Broadcast goes everywhere
            (_, Domain::Broadcast) => true,
            (Domain::Broadcast, _) => true,
            // Same domain
            (a, b) if a == b => true,
            // Everything else is forbidden
            _ => false,
        }
    }

    /// Is cognitive domain (not infrastructure)
    #[inline(always)]
    pub const fn is_cognitive(&self) -> bool {
        !matches!(self, Self::Core | Self::Broadcast)
    }

    /// Get domain index (for array indexing)
    pub const fn index(&self) -> usize {
        match self {
            Self::Sense => 0,
            Self::Understand => 1,
            Self::Reason => 2,
            Self::Decide => 3,
            Self::Act => 4,
            Self::Memory => 5,
            Self::Reflect => 6,
            Self::Core => 7,
            Self::Broadcast => 8,
        }
    }

    /// From index
    pub const fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Sense),
            1 => Some(Self::Understand),
            2 => Some(Self::Reason),
            3 => Some(Self::Decide),
            4 => Some(Self::Act),
            5 => Some(Self::Memory),
            6 => Some(Self::Reflect),
            7 => Some(Self::Core),
            8 => Some(Self::Broadcast),
            _ => None,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_flow() {
        assert!(Domain::Sense.can_send_to(&Domain::Understand));
        assert!(Domain::Understand.can_send_to(&Domain::Reason));
        assert!(Domain::Reason.can_send_to(&Domain::Decide));
        assert!(Domain::Decide.can_send_to(&Domain::Act));

        // Forbidden flows
        assert!(!Domain::Act.can_send_to(&Domain::Sense));
        assert!(!Domain::Decide.can_send_to(&Domain::Understand));
    }

    #[test]
    fn test_domain_properties() {
        assert!(Domain::Sense.is_cognitive());
        assert!(!Domain::Core.is_cognitive());
        assert!(!Domain::Broadcast.is_cognitive());
    }

    #[test]
    fn test_domain_index() {
        assert_eq!(Domain::Sense.index(), 0);
        assert_eq!(Domain::from_index(0), Some(Domain::Sense));
        assert_eq!(Domain::from_index(99), None);
    }
}
