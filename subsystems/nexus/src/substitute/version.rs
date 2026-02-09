//! Module version for compatibility checking.

/// Module version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
}

impl ModuleVersion {
    /// Create a new version
    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Check ABI compatibility (same major version)
    #[inline(always)]
    pub fn is_abi_compatible(&self, other: &ModuleVersion) -> bool {
        self.major == other.major
    }

    /// Check if this is newer
    #[inline(always)]
    pub fn is_newer_than(&self, other: &ModuleVersion) -> bool {
        self > other
    }
}

impl core::fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
