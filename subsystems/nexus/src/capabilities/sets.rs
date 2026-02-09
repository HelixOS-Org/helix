//! Capability Sets
//!
//! Bitmask-based capability set operations.

use alloc::vec::Vec;

use super::Capability;

/// Capability set (bitmask)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    /// Lower 32 bits
    pub cap0: u32,
    /// Upper 32 bits
    pub cap1: u32,
}

impl CapabilitySet {
    /// Empty set
    pub const EMPTY: Self = Self { cap0: 0, cap1: 0 };

    /// Full set (all capabilities)
    pub const FULL: Self = Self {
        cap0: u32::MAX,
        cap1: u32::MAX,
    };

    /// Create new capability set
    #[inline(always)]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// Check if capability is set
    #[inline]
    pub fn has(&self, cap: Capability) -> bool {
        let bit = cap.number();
        if bit < 32 {
            (self.cap0 & (1 << bit)) != 0
        } else {
            (self.cap1 & (1 << (bit - 32))) != 0
        }
    }

    /// Set capability
    #[inline]
    pub fn set(&mut self, cap: Capability) {
        let bit = cap.number();
        if bit < 32 {
            self.cap0 |= 1 << bit;
        } else {
            self.cap1 |= 1 << (bit - 32);
        }
    }

    /// Clear capability
    #[inline]
    pub fn clear(&mut self, cap: Capability) {
        let bit = cap.number();
        if bit < 32 {
            self.cap0 &= !(1 << bit);
        } else {
            self.cap1 &= !(1 << (bit - 32));
        }
    }

    /// Toggle capability
    #[inline]
    pub fn toggle(&mut self, cap: Capability) {
        if self.has(cap) {
            self.clear(cap);
        } else {
            self.set(cap);
        }
    }

    /// Union with another set
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            cap0: self.cap0 | other.cap0,
            cap1: self.cap1 | other.cap1,
        }
    }

    /// Intersection with another set
    #[inline]
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            cap0: self.cap0 & other.cap0,
            cap1: self.cap1 & other.cap1,
        }
    }

    /// Difference (capabilities in self but not in other)
    #[inline]
    pub fn difference(&self, other: &Self) -> Self {
        Self {
            cap0: self.cap0 & !other.cap0,
            cap1: self.cap1 & !other.cap1,
        }
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.cap0 == 0 && self.cap1 == 0
    }

    /// Is full (all capabilities set)
    #[inline]
    pub fn is_full(&self) -> bool {
        let mask0 = u32::MAX;
        let mask1 = (1u32 << 9) - 1; // Caps 32-40
        (self.cap0 & mask0) == mask0 && (self.cap1 & mask1) == mask1
    }

    /// Count capabilities
    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.cap0.count_ones() + self.cap1.count_ones()
    }

    /// Iterate over set capabilities
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        Capability::all().iter().filter(|c| self.has(**c)).copied()
    }

    /// Get list of capabilities
    #[inline(always)]
    pub fn to_list(&self) -> Vec<Capability> {
        self.iter().collect()
    }

    /// From list of capabilities
    #[inline]
    pub fn from_list(caps: &[Capability]) -> Self {
        let mut set = Self::new();
        for cap in caps {
            set.set(*cap);
        }
        set
    }
}

/// Capability set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapSetType {
    /// Effective capabilities
    Effective,
    /// Permitted capabilities
    Permitted,
    /// Inheritable capabilities
    Inheritable,
    /// Bounding set
    Bounding,
    /// Ambient capabilities
    Ambient,
}

impl CapSetType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Effective => "effective",
            Self::Permitted => "permitted",
            Self::Inheritable => "inheritable",
            Self::Bounding => "bounding",
            Self::Ambient => "ambient",
        }
    }
}
