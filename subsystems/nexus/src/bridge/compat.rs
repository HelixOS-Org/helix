//! # Syscall Compatibility Layer
//!
//! Provides cross-version and cross-ABI compatibility for the syscall bridge:
//! - POSIX → Helix syscall translation
//! - Linux → Helix syscall mapping
//! - ABI versioning & negotiation
//! - Deprecated syscall emulation
//! - Compatibility profiles

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// ABI VERSION MANAGEMENT
// ============================================================================

/// ABI version identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AbiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl AbiVersion {
    #[inline]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Current Helix ABI version
    pub const CURRENT: Self = Self::new(4, 0, 0);

    /// Minimum supported ABI version
    pub const MIN_SUPPORTED: Self = Self::new(1, 0, 0);

    /// Check compatibility
    #[inline(always)]
    pub fn is_compatible(&self, other: &AbiVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }

    /// Check if this version supports a feature introduced in `since`
    pub fn supports(&self, since: &AbiVersion) -> bool {
        if self.major > since.major {
            return true;
        }
        if self.major == since.major {
            if self.minor > since.minor {
                return true;
            }
            if self.minor == since.minor {
                return self.patch >= since.patch;
            }
        }
        false
    }
}

// ============================================================================
// COMPATIBILITY PROFILES
// ============================================================================

/// Compatibility profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatProfile {
    /// Native Helix ABI
    Native,
    /// POSIX-compatible layer
    Posix,
    /// Linux-compatible layer
    LinuxCompat,
    /// Minimal (embedded) profile
    Minimal,
    /// Legacy Helix (pre-Year 4)
    LegacyHelix,
    /// Custom profile
    Custom,
}

/// Compatibility mode configuration
#[derive(Debug, Clone)]
pub struct CompatConfig {
    /// Active profile
    pub profile: CompatProfile,
    /// ABI version
    pub abi_version: AbiVersion,
    /// Whether to emulate deprecated syscalls
    pub emulate_deprecated: bool,
    /// Whether to log compatibility translations
    pub log_translations: bool,
    /// Max translation overhead allowed (µs)
    pub max_overhead_us: u64,
    /// Allowed syscall types
    pub allowed_syscalls: Vec<SyscallType>,
    /// Blocked syscall types
    pub blocked_syscalls: Vec<SyscallType>,
}

impl CompatConfig {
    #[inline]
    pub fn native() -> Self {
        Self {
            profile: CompatProfile::Native,
            abi_version: AbiVersion::CURRENT,
            emulate_deprecated: false,
            log_translations: false,
            max_overhead_us: 10,
            allowed_syscalls: Vec::new(),
            blocked_syscalls: Vec::new(),
        }
    }

    #[inline]
    pub fn posix() -> Self {
        Self {
            profile: CompatProfile::Posix,
            abi_version: AbiVersion::CURRENT,
            emulate_deprecated: true,
            log_translations: true,
            max_overhead_us: 50,
            allowed_syscalls: Vec::new(),
            blocked_syscalls: Vec::new(),
        }
    }

    #[inline]
    pub fn linux_compat() -> Self {
        Self {
            profile: CompatProfile::LinuxCompat,
            abi_version: AbiVersion::CURRENT,
            emulate_deprecated: true,
            log_translations: true,
            max_overhead_us: 100,
            allowed_syscalls: Vec::new(),
            blocked_syscalls: Vec::new(),
        }
    }
}

// ============================================================================
// SYSCALL MAPPING TABLE
// ============================================================================

/// Foreign syscall number to Helix mapping
#[derive(Debug, Clone)]
pub struct SyscallMapping {
    /// Source syscall number (foreign ABI)
    pub source_number: u32,
    /// Source ABI name
    pub source_name: String,
    /// Target Helix syscall type
    pub target: SyscallType,
    /// Whether argument rewriting is needed
    pub needs_arg_rewrite: bool,
    /// Argument mapping (source arg idx → target arg idx)
    pub arg_map: Vec<(usize, usize)>,
    /// Translation complexity (0-100)
    pub complexity: u8,
    /// Whether this is a deprecated mapping
    pub deprecated: bool,
    /// ABI version this mapping was introduced
    pub since: AbiVersion,
}

/// Mapping table for a foreign ABI
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MappingTable {
    /// Profile
    pub profile: CompatProfile,
    /// Mappings (keyed by foreign syscall number)
    mappings: BTreeMap<u32, SyscallMapping>,
    /// Fallback handler type
    pub fallback: FallbackAction,
    /// Statistics
    pub hits: u64,
    pub misses: u64,
}

/// Action when no mapping exists
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackAction {
    /// Return -ENOSYS
    ReturnError,
    /// Pass through unmodified
    PassThrough,
    /// Kill the process
    KillProcess,
    /// Log and return error
    LogAndError,
}

impl MappingTable {
    pub fn new(profile: CompatProfile) -> Self {
        Self {
            profile,
            mappings: BTreeMap::new(),
            fallback: FallbackAction::ReturnError,
            hits: 0,
            misses: 0,
        }
    }

    /// Add a mapping
    #[inline(always)]
    pub fn add_mapping(&mut self, mapping: SyscallMapping) {
        self.mappings.insert(mapping.source_number, mapping);
    }

    /// Lookup a mapping
    #[inline]
    pub fn lookup(&mut self, foreign_number: u32) -> Option<&SyscallMapping> {
        if let Some(mapping) = self.mappings.get(&foreign_number) {
            self.hits += 1;
            Some(mapping)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Number of mappings
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Hit rate
    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ============================================================================
// ARGUMENT REWRITER
// ============================================================================

/// Argument transformation type
#[derive(Debug, Clone)]
pub enum ArgTransform {
    /// Pass argument unchanged
    Identity,
    /// Map to different position
    Reposition(usize),
    /// Add constant
    AddConstant(i64),
    /// Bitwise mask
    Mask(u64),
    /// Map flags from one convention to another
    MapFlags(Vec<FlagMapping>),
    /// Pointer indirection adjustment
    PointerAdjust(i64),
    /// NULL → default value substitution
    NullDefault(u64),
    /// Clamp to range
    Clamp(u64, u64),
}

/// Flag mapping entry
#[derive(Debug, Clone, Copy)]
pub struct FlagMapping {
    /// Source flag bit
    pub source_bit: u64,
    /// Target flag bit
    pub target_bit: u64,
    /// Whether to invert
    pub invert: bool,
}

/// Argument rewriter for a specific syscall translation
#[derive(Debug, Clone)]
pub struct ArgRewriter {
    /// Transforms per argument (index = target arg position)
    transforms: Vec<ArgTransform>,
    /// Return value transform
    return_transform: ReturnTransform,
}

/// Return value transformation
#[derive(Debug, Clone)]
pub enum ReturnTransform {
    /// Pass through unchanged
    Identity,
    /// Negate error codes
    NegateErrors,
    /// Map specific error codes
    MapErrors(Vec<(i64, i64)>),
    /// Boolean to errno
    BoolToErrno,
}

impl ArgRewriter {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
            return_transform: ReturnTransform::Identity,
        }
    }

    /// Add a transform for a target argument
    #[inline(always)]
    pub fn add_transform(&mut self, transform: ArgTransform) {
        self.transforms.push(transform);
    }

    /// Set return transform
    #[inline(always)]
    pub fn set_return_transform(&mut self, transform: ReturnTransform) {
        self.return_transform = transform;
    }

    /// Rewrite arguments from source ABI to Helix ABI
    pub fn rewrite(&self, source_args: &[u64]) -> Vec<u64> {
        let mut result = Vec::with_capacity(self.transforms.len());

        for (i, transform) in self.transforms.iter().enumerate() {
            let value = match transform {
                ArgTransform::Identity => {
                    if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    }
                },
                ArgTransform::Reposition(src_idx) => {
                    if *src_idx < source_args.len() {
                        source_args[*src_idx]
                    } else {
                        0
                    }
                },
                ArgTransform::AddConstant(c) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    (src as i64 + c) as u64
                },
                ArgTransform::Mask(mask) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    src & mask
                },
                ArgTransform::MapFlags(mappings) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    let mut result = 0u64;
                    for mapping in mappings {
                        let bit_set = (src & mapping.source_bit) != 0;
                        let should_set = if mapping.invert { !bit_set } else { bit_set };
                        if should_set {
                            result |= mapping.target_bit;
                        }
                    }
                    result
                },
                ArgTransform::PointerAdjust(offset) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    if src == 0 {
                        0
                    } else {
                        (src as i64 + offset) as u64
                    }
                },
                ArgTransform::NullDefault(default) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    if src == 0 { *default } else { src }
                },
                ArgTransform::Clamp(min, max) => {
                    let src = if i < source_args.len() {
                        source_args[i]
                    } else {
                        0
                    };
                    if src < *min {
                        *min
                    } else if src > *max {
                        *max
                    } else {
                        src
                    }
                },
            };
            result.push(value);
        }

        result
    }

    /// Rewrite return value
    pub fn rewrite_return(&self, value: i64) -> i64 {
        match &self.return_transform {
            ReturnTransform::Identity => value,
            ReturnTransform::NegateErrors => {
                if value < 0 {
                    -value
                } else {
                    value
                }
            },
            ReturnTransform::MapErrors(map) => {
                for &(src, dst) in map {
                    if value == src {
                        return dst;
                    }
                }
                value
            },
            ReturnTransform::BoolToErrno => {
                if value == 0 {
                    -1
                } else {
                    0
                }
            },
        }
    }
}

// ============================================================================
// COMPAT LAYER ENGINE
// ============================================================================

/// Compatibility translation result
#[derive(Debug, Clone)]
pub struct TranslationResult {
    /// Target Helix syscall type
    pub target_type: SyscallType,
    /// Rewritten arguments
    pub args: Vec<u64>,
    /// Whether this was a direct mapping
    pub direct_mapping: bool,
    /// Translation overhead (ns)
    pub overhead_ns: u64,
    /// Whether deprecated
    pub deprecated: bool,
}

/// The compatibility layer engine
#[repr(align(64))]
pub struct CompatLayer {
    /// Mapping tables per profile
    tables: BTreeMap<u8, MappingTable>,
    /// Per-process profile assignments
    process_profiles: BTreeMap<u64, CompatProfile>,
    /// Default profile
    default_profile: CompatProfile,
    /// Argument rewriters per (profile, syscall_number) pair
    rewriters: BTreeMap<(u8, u32), ArgRewriter>,
    /// Config
    config: CompatConfig,
    /// Total translations performed
    pub total_translations: u64,
    /// Total direct hits (no rewrite needed)
    pub total_direct: u64,
}

impl CompatLayer {
    pub fn new(config: CompatConfig) -> Self {
        let default_profile = config.profile;
        Self {
            tables: BTreeMap::new(),
            process_profiles: BTreeMap::new(),
            default_profile,
            rewriters: BTreeMap::new(),
            config,
            total_translations: 0,
            total_direct: 0,
        }
    }

    /// Register a mapping table
    #[inline(always)]
    pub fn register_table(&mut self, table: MappingTable) {
        self.tables.insert(table.profile as u8, table);
    }

    /// Register an argument rewriter
    #[inline(always)]
    pub fn register_rewriter(
        &mut self,
        profile: CompatProfile,
        syscall_number: u32,
        rewriter: ArgRewriter,
    ) {
        self.rewriters
            .insert((profile as u8, syscall_number), rewriter);
    }

    /// Set profile for a process
    #[inline(always)]
    pub fn set_process_profile(&mut self, pid: u64, profile: CompatProfile) {
        self.process_profiles.insert(pid, profile);
    }

    /// Get profile for a process
    #[inline]
    pub fn get_process_profile(&self, pid: u64) -> CompatProfile {
        self.process_profiles
            .get(&pid)
            .copied()
            .unwrap_or(self.default_profile)
    }

    /// Translate a foreign syscall
    pub fn translate(
        &mut self,
        pid: u64,
        foreign_number: u32,
        args: &[u64],
    ) -> Result<TranslationResult, CompatError> {
        self.total_translations += 1;
        let profile = self.get_process_profile(pid);

        let table = self
            .tables
            .get_mut(&(profile as u8))
            .ok_or(CompatError::NoMappingTable)?;

        let mapping = table
            .lookup(foreign_number)
            .ok_or(CompatError::UnknownSyscall(foreign_number))?;

        let target_type = mapping.target;
        let deprecated = mapping.deprecated;
        let needs_rewrite = mapping.needs_arg_rewrite;

        if !needs_rewrite {
            self.total_direct += 1;
            return Ok(TranslationResult {
                target_type,
                args: args.to_vec(),
                direct_mapping: true,
                overhead_ns: 0,
                deprecated,
            });
        }

        // Apply argument rewriting
        let rewriter_key = (profile as u8, foreign_number);
        let rewritten = if let Some(rewriter) = self.rewriters.get(&rewriter_key) {
            rewriter.rewrite(args)
        } else {
            args.to_vec()
        };

        Ok(TranslationResult {
            target_type,
            args: rewritten,
            direct_mapping: false,
            overhead_ns: 0,
            deprecated,
        })
    }

    /// Remove process data
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.process_profiles.remove(&pid);
    }

    /// Direct hit rate
    #[inline]
    pub fn direct_rate(&self) -> f64 {
        if self.total_translations == 0 {
            0.0
        } else {
            self.total_direct as f64 / self.total_translations as f64
        }
    }
}

/// Compatibility error
#[derive(Debug, Clone)]
pub enum CompatError {
    /// No mapping table for the profile
    NoMappingTable,
    /// Unknown foreign syscall number
    UnknownSyscall(u32),
    /// ABI version mismatch
    VersionMismatch(AbiVersion, AbiVersion),
    /// Syscall blocked by policy
    Blocked,
    /// Translation overhead too high
    OverheadExceeded,
    /// Deprecated and emulation disabled
    DeprecatedNotEmulated,
}
