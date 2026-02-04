//! Version Types
//!
//! Semantic versioning for data structures and APIs.

#![allow(dead_code)]

// ============================================================================
// VERSION
// ============================================================================

/// Version number for data structures
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
}

impl Version {
    /// Create new version
    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Initial version (1.0.0)
    pub const INITIAL: Self = Self::new(1, 0, 0);

    /// Zero version (0.0.0)
    pub const ZERO: Self = Self::new(0, 0, 0);

    /// Check compatibility (same major version)
    #[inline]
    pub const fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }

    /// Bump major version
    pub const fn bump_major(self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }

    /// Bump minor version
    pub const fn bump_minor(self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Bump patch version
    pub const fn bump_patch(self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Is this a pre-release version (0.x.x)?
    pub const fn is_prerelease(&self) -> bool {
        self.major == 0
    }

    /// Compare versions
    pub const fn cmp_version(&self, other: &Self) -> core::cmp::Ordering {
        if self.major != other.major {
            if self.major < other.major {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        } else if self.minor != other.minor {
            if self.minor < other.minor {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        } else if self.patch != other.patch {
            if self.patch < other.patch {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        } else {
            core::cmp::Ordering::Equal
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::INITIAL
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ============================================================================
// VERSION REQUIREMENT
// ============================================================================

/// Version requirement for dependencies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionReq {
    /// Minimum version (inclusive)
    pub min: Version,
    /// Maximum version (exclusive) - None means no upper bound
    pub max: Option<Version>,
}

impl VersionReq {
    /// Create exact version requirement
    pub const fn exact(version: Version) -> Self {
        Self {
            min: version,
            max: Some(Version::new(
                version.major,
                version.minor,
                version.patch + 1,
            )),
        }
    }

    /// Create compatible requirement (same major version)
    pub const fn compatible(version: Version) -> Self {
        Self {
            min: version,
            max: Some(Version::new(version.major + 1, 0, 0)),
        }
    }

    /// Create minimum version requirement
    pub const fn at_least(version: Version) -> Self {
        Self {
            min: version,
            max: None,
        }
    }

    /// Create any version requirement
    pub const fn any() -> Self {
        Self {
            min: Version::ZERO,
            max: None,
        }
    }

    /// Check if version matches requirement
    pub fn matches(&self, version: Version) -> bool {
        if version.cmp_version(&self.min) == core::cmp::Ordering::Less {
            return false;
        }
        if let Some(max) = self.max {
            if version.cmp_version(&max) != core::cmp::Ordering::Less {
                return false;
            }
        }
        true
    }
}

impl Default for VersionReq {
    fn default() -> Self {
        Self::any()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_bump() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.bump_patch(), Version::new(1, 2, 4));
        assert_eq!(v.bump_minor(), Version::new(1, 3, 0));
        assert_eq!(v.bump_major(), Version::new(2, 0, 0));
    }

    #[test]
    fn test_version_compatible() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 5, 0);
        let v3 = Version::new(2, 0, 0);
        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));
    }

    #[test]
    fn test_version_req() {
        let req = VersionReq::compatible(Version::new(1, 0, 0));
        assert!(req.matches(Version::new(1, 0, 0)));
        assert!(req.matches(Version::new(1, 5, 3)));
        assert!(!req.matches(Version::new(0, 9, 0)));
        assert!(!req.matches(Version::new(2, 0, 0)));
    }
}
