//! # Bridge Versioning
//!
//! Syscall API versioning and compatibility:
//! - API version negotiation
//! - Backward compatibility shims
//! - Feature flags
//! - Deprecation tracking
//! - Version migration support
//! - ABI stability checks

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// API VERSION
// ============================================================================

/// Syscall API version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    /// Major version (breaking changes)
    pub major: u16,
    /// Minor version (new features)
    pub minor: u16,
    /// Patch version (bug fixes)
    pub patch: u16,
}

impl ApiVersion {
    pub const CURRENT: Self = Self {
        major: 4,
        minor: 1,
        patch: 0,
    };

    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Check if compatible (same major, >= minor)
    #[inline(always)]
    pub fn is_compatible(&self, other: &ApiVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }

    /// Check if backward compatible
    #[inline]
    pub fn is_backward_compatible(&self, older: &ApiVersion) -> bool {
        if self.major > older.major {
            return false; // Breaking change
        }
        if self.major == older.major {
            return self.minor >= older.minor;
        }
        false
    }

    /// Encode as u64
    #[inline(always)]
    pub fn encode(&self) -> u64 {
        ((self.major as u64) << 32) | ((self.minor as u64) << 16) | (self.patch as u64)
    }

    /// Decode from u64
    #[inline]
    pub fn decode(encoded: u64) -> Self {
        Self {
            major: (encoded >> 32) as u16,
            minor: (encoded >> 16) as u16,
            patch: encoded as u16,
        }
    }
}

// ============================================================================
// FEATURE FLAGS
// ============================================================================

/// Feature flag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallFeature {
    /// Async syscalls
    AsyncSyscalls,
    /// Batched syscalls
    BatchedSyscalls,
    /// Zero-copy I/O
    ZeroCopyIo,
    /// Cooperative scheduling hints
    CoopHints,
    /// Memory-mapped syscalls
    MmapSyscalls,
    /// Extended error codes
    ExtendedErrors,
    /// Syscall tracing
    SyscallTracing,
    /// Capability-based security
    Capabilities,
    /// User-space interrupts
    UserInterrupts,
    /// IPC fast path
    IpcFastPath,
    /// Large page support
    LargePages,
    /// NUMA hints
    NumaHints,
}

/// Feature availability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStatus {
    /// Available and stable
    Stable,
    /// Available but experimental
    Experimental,
    /// Deprecated (will be removed)
    Deprecated,
    /// Removed
    Removed,
    /// Not yet available
    Unavailable,
}

/// Feature info
#[derive(Debug, Clone)]
pub struct FeatureInfo {
    /// Feature
    pub feature: SyscallFeature,
    /// Status
    pub status: FeatureStatus,
    /// Since version
    pub since: ApiVersion,
    /// Deprecated since (if applicable)
    pub deprecated_since: Option<ApiVersion>,
    /// Removed since (if applicable)
    pub removed_since: Option<ApiVersion>,
    /// Usage count
    pub usage_count: u64,
}

impl FeatureInfo {
    pub fn new(feature: SyscallFeature, status: FeatureStatus, since: ApiVersion) -> Self {
        Self {
            feature,
            status,
            since,
            deprecated_since: None,
            removed_since: None,
            usage_count: 0,
        }
    }

    #[inline]
    pub fn is_available(&self) -> bool {
        matches!(
            self.status,
            FeatureStatus::Stable | FeatureStatus::Experimental | FeatureStatus::Deprecated
        )
    }

    #[inline(always)]
    pub fn record_usage(&mut self) {
        self.usage_count += 1;
    }
}

// ============================================================================
// SYSCALL ENTRY
// ============================================================================

/// Syscall definition
#[derive(Debug, Clone)]
pub struct SyscallDefinition {
    /// Syscall number
    pub number: u32,
    /// Number of arguments
    pub arg_count: u8,
    /// Since version
    pub since: ApiVersion,
    /// Deprecated since
    pub deprecated_since: Option<ApiVersion>,
    /// Replacement syscall (if deprecated)
    pub replacement: Option<u32>,
    /// Required features
    pub required_features: Vec<SyscallFeature>,
}

impl SyscallDefinition {
    pub fn new(number: u32, arg_count: u8, since: ApiVersion) -> Self {
        Self {
            number,
            arg_count,
            since,
            deprecated_since: None,
            replacement: None,
            required_features: Vec::new(),
        }
    }

    #[inline]
    pub fn deprecate(mut self, since: ApiVersion, replacement: u32) -> Self {
        self.deprecated_since = Some(since);
        self.replacement = Some(replacement);
        self
    }

    #[inline(always)]
    pub fn is_deprecated(&self) -> bool {
        self.deprecated_since.is_some()
    }

    #[inline(always)]
    pub fn is_available_at(&self, version: &ApiVersion) -> bool {
        version.is_backward_compatible(&self.since)
    }
}

// ============================================================================
// COMPATIBILITY SHIM
// ============================================================================

/// Compatibility shim type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShimType {
    /// Translate arguments
    ArgTranslation,
    /// Translate return value
    ReturnTranslation,
    /// Full emulation
    FullEmulation,
    /// Redirect to new syscall
    Redirect,
}

/// Compatibility shim
#[derive(Debug, Clone)]
pub struct CompatShim {
    /// Old syscall number
    pub old_number: u32,
    /// New syscall number
    pub new_number: u32,
    /// Shim type
    pub shim_type: ShimType,
    /// Old version range
    pub min_version: ApiVersion,
    pub max_version: ApiVersion,
    /// Active
    pub active: bool,
    /// Invocations
    pub invocations: u64,
}

impl CompatShim {
    pub fn new(
        old_number: u32,
        new_number: u32,
        shim_type: ShimType,
        min_version: ApiVersion,
        max_version: ApiVersion,
    ) -> Self {
        Self {
            old_number,
            new_number,
            shim_type,
            min_version,
            max_version,
            active: true,
            invocations: 0,
        }
    }

    #[inline(always)]
    pub fn applies_to(&self, version: &ApiVersion) -> bool {
        self.active && version >= &self.min_version && version <= &self.max_version
    }

    #[inline(always)]
    pub fn invoke(&mut self) {
        self.invocations += 1;
    }
}

// ============================================================================
// VERSION MANAGER
// ============================================================================

/// Versioning stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct VersioningStats {
    /// Current API version
    pub current_version: u64,
    /// Registered syscalls
    pub registered_syscalls: usize,
    /// Deprecated syscalls
    pub deprecated_count: usize,
    /// Active shims
    pub active_shims: usize,
    /// Feature count
    pub feature_count: usize,
    /// Shim invocations
    pub shim_invocations: u64,
}

/// Bridge versioning manager
#[repr(align(64))]
pub struct BridgeVersionManager {
    /// Syscall definitions (number → def)
    syscalls: BTreeMap<u32, SyscallDefinition>,
    /// Feature registry
    features: BTreeMap<u8, FeatureInfo>,
    /// Compatibility shims
    shims: Vec<CompatShim>,
    /// Process API versions (pid → version)
    process_versions: BTreeMap<u64, ApiVersion>,
    /// Stats
    stats: VersioningStats,
}

impl BridgeVersionManager {
    pub fn new() -> Self {
        Self {
            syscalls: BTreeMap::new(),
            features: BTreeMap::new(),
            shims: Vec::new(),
            process_versions: BTreeMap::new(),
            stats: VersioningStats {
                current_version: ApiVersion::CURRENT.encode(),
                ..Default::default()
            },
        }
    }

    /// Register syscall
    #[inline(always)]
    pub fn register_syscall(&mut self, def: SyscallDefinition) {
        self.syscalls.insert(def.number, def);
        self.update_stats();
    }

    /// Register feature
    #[inline(always)]
    pub fn register_feature(&mut self, info: FeatureInfo) {
        self.features.insert(info.feature as u8, info);
        self.stats.feature_count = self.features.len();
    }

    /// Add shim
    #[inline(always)]
    pub fn add_shim(&mut self, shim: CompatShim) {
        self.shims.push(shim);
        self.stats.active_shims = self.shims.iter().filter(|s| s.active).count();
    }

    /// Set process version
    #[inline(always)]
    pub fn set_process_version(&mut self, pid: u64, version: ApiVersion) {
        self.process_versions.insert(pid, version);
    }

    /// Check syscall availability for process
    pub fn check_syscall(&self, pid: u64, syscall_nr: u32) -> bool {
        let version = self
            .process_versions
            .get(&pid)
            .copied()
            .unwrap_or(ApiVersion::CURRENT);

        if let Some(def) = self.syscalls.get(&syscall_nr) {
            def.is_available_at(&version)
        } else {
            false
        }
    }

    /// Find applicable shim
    pub fn find_shim(&mut self, pid: u64, syscall_nr: u32) -> Option<u32> {
        let version = self
            .process_versions
            .get(&pid)
            .copied()
            .unwrap_or(ApiVersion::CURRENT);

        for shim in &mut self.shims {
            if shim.old_number == syscall_nr && shim.applies_to(&version) {
                shim.invoke();
                self.stats.shim_invocations += 1;
                return Some(shim.new_number);
            }
        }
        None
    }

    /// Check feature availability
    #[inline]
    pub fn check_feature(&mut self, feature: SyscallFeature) -> bool {
        if let Some(info) = self.features.get_mut(&(feature as u8)) {
            if info.is_available() {
                info.record_usage();
                return true;
            }
        }
        false
    }

    /// Get deprecated syscalls
    #[inline(always)]
    pub fn deprecated_syscalls(&self) -> Vec<&SyscallDefinition> {
        self.syscalls.values().filter(|s| s.is_deprecated()).collect()
    }

    fn update_stats(&mut self) {
        self.stats.registered_syscalls = self.syscalls.len();
        self.stats.deprecated_count = self.syscalls.values().filter(|s| s.is_deprecated()).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &VersioningStats {
        &self.stats
    }
}
