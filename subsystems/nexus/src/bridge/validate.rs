//! # Syscall Validation Engine
//!
//! Comprehensive syscall argument validation:
//! - Type-safe argument checking
//! - Pointer validation
//! - Range checking
//! - Permission validation
//! - Sanitization rules
//! - Validation caching for trusted processes

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// VALIDATION TYPES
// ============================================================================

/// Validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationResult {
    /// Arguments are valid
    Valid,
    /// Warning - suspicious but allowed
    Warning,
    /// Invalid - syscall should be rejected
    Invalid,
    /// Needs further checking
    NeedsCheck,
}

/// Validation error category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationError {
    /// Null pointer
    NullPointer,
    /// Unaligned pointer
    UnalignedPointer,
    /// Pointer out of user space
    KernelPointer,
    /// Buffer too large
    BufferTooLarge,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid flags
    InvalidFlags,
    /// Invalid file descriptor
    InvalidFd,
    /// Invalid mode
    InvalidMode,
    /// Integer overflow
    IntegerOverflow,
    /// Path traversal attempt
    PathTraversal,
    /// Null byte in string
    NullByteInString,
    /// String too long
    StringTooLong,
    /// Invalid combination
    InvalidCombination,
    /// Permission denied
    PermissionDenied,
    /// Resource limit exceeded
    ResourceLimit,
}

/// Argument type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    /// Integer value
    Integer,
    /// Unsigned integer
    UnsignedInteger,
    /// File descriptor
    FileDescriptor,
    /// Pointer to user buffer
    UserPointer,
    /// Pointer to user string
    UserString,
    /// Flags bitfield
    Flags,
    /// Mode (permissions)
    Mode,
    /// Size / length
    Size,
    /// Offset
    Offset,
    /// Signal number
    Signal,
    /// PID
    Pid,
}

// ============================================================================
// VALIDATION RULES
// ============================================================================

/// Argument validation rule
#[derive(Debug, Clone)]
pub struct ArgRule {
    /// Argument index (0-5)
    pub arg_index: u8,
    /// Expected type
    pub arg_type: ArgType,
    /// Minimum value (for integers)
    pub min_value: Option<u64>,
    /// Maximum value (for integers)
    pub max_value: Option<u64>,
    /// Required alignment (for pointers)
    pub alignment: Option<u64>,
    /// Allowed flags mask (for flags)
    pub allowed_flags: Option<u64>,
    /// Maximum size (for buffers/strings)
    pub max_size: Option<u64>,
    /// Is optional (can be 0/NULL)
    pub optional: bool,
}

impl ArgRule {
    pub fn integer(index: u8) -> Self {
        Self {
            arg_index: index,
            arg_type: ArgType::Integer,
            min_value: None,
            max_value: None,
            alignment: None,
            allowed_flags: None,
            max_size: None,
            optional: false,
        }
    }

    pub fn fd(index: u8) -> Self {
        Self {
            arg_index: index,
            arg_type: ArgType::FileDescriptor,
            min_value: Some(0),
            max_value: Some(1024 * 1024),
            alignment: None,
            allowed_flags: None,
            max_size: None,
            optional: false,
        }
    }

    pub fn user_pointer(index: u8) -> Self {
        Self {
            arg_index: index,
            arg_type: ArgType::UserPointer,
            min_value: None,
            max_value: None,
            alignment: Some(1),
            allowed_flags: None,
            max_size: None,
            optional: false,
        }
    }

    pub fn flags(index: u8, allowed: u64) -> Self {
        Self {
            arg_index: index,
            arg_type: ArgType::Flags,
            min_value: None,
            max_value: None,
            alignment: None,
            allowed_flags: Some(allowed),
            max_size: None,
            optional: false,
        }
    }

    pub fn size(index: u8, max: u64) -> Self {
        Self {
            arg_index: index,
            arg_type: ArgType::Size,
            min_value: Some(0),
            max_value: Some(max),
            alignment: None,
            allowed_flags: None,
            max_size: Some(max),
            optional: false,
        }
    }

    #[inline(always)]
    pub fn with_optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// Syscall validation spec
#[derive(Debug, Clone)]
pub struct SyscallValidationSpec {
    /// Syscall number
    pub syscall_nr: u32,
    /// Argument rules
    pub rules: Vec<ArgRule>,
    /// Custom validation flags
    pub custom_flags: u64,
}

impl SyscallValidationSpec {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            rules: Vec::new(),
            custom_flags: 0,
        }
    }

    #[inline(always)]
    pub fn add_rule(&mut self, rule: ArgRule) {
        self.rules.push(rule);
    }
}

// ============================================================================
// VALIDATION CONTEXT
// ============================================================================

/// Context for validation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ValidationContext {
    /// Process ID
    pub pid: u64,
    /// User space address range (start, end)
    pub user_space_range: (u64, u64),
    /// Process capability flags
    pub capabilities: u64,
    /// Process trust level
    pub trust_level: u32,
    /// Max file descriptors for process
    pub max_fds: u32,
    /// Max buffer size
    pub max_buffer_size: u64,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            pid: 0,
            user_space_range: (0x1000, 0x0000_7FFF_FFFF_FFFF),
            capabilities: 0,
            trust_level: 0,
            max_fds: 1024,
            max_buffer_size: 1024 * 1024 * 256, // 256MB
        }
    }
}

// ============================================================================
// VALIDATION REPORT
// ============================================================================

/// Single validation finding
#[derive(Debug, Clone)]
pub struct ValidationFinding {
    /// Argument index
    pub arg_index: u8,
    /// Error
    pub error: ValidationError,
    /// Actual value
    pub actual_value: u64,
    /// Expected constraint
    pub expected: u64,
}

/// Complete validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Syscall number
    pub syscall_nr: u32,
    /// Overall result
    pub result: ValidationResult,
    /// Findings
    pub findings: Vec<ValidationFinding>,
    /// Validation time (nanoseconds)
    pub validation_time_ns: u64,
}

// ============================================================================
// VALIDATION CACHE
// ============================================================================

/// Cached validation result for trusted processes
#[derive(Debug, Clone)]
struct CachedValidation {
    /// Syscall number
    syscall_nr: u32,
    /// Argument signature (hash-like)
    arg_signature: u64,
    /// Result
    result: ValidationResult,
    /// Timestamp
    cached_at: u64,
    /// Expiry
    expires_at: u64,
}

/// Validation cache
struct ValidCache {
    /// Cache entries (pid -> entries)
    entries: BTreeMap<u64, Vec<CachedValidation>>,
    /// Max per process
    max_per_process: usize,
    /// Hits
    hits: u64,
    /// Misses
    misses: u64,
}

impl ValidCache {
    fn new(max_per_process: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_per_process,
            hits: 0,
            misses: 0,
        }
    }

    fn lookup(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        arg_sig: u64,
        current_time: u64,
    ) -> Option<ValidationResult> {
        if let Some(entries) = self.entries.get(&pid) {
            for entry in entries {
                if entry.syscall_nr == syscall_nr
                    && entry.arg_signature == arg_sig
                    && current_time < entry.expires_at
                {
                    self.hits += 1;
                    return Some(entry.result);
                }
            }
        }
        self.misses += 1;
        None
    }

    fn insert(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        arg_sig: u64,
        result: ValidationResult,
        timestamp: u64,
    ) {
        let entries = self.entries.entry(pid).or_insert_with(Vec::new);
        if entries.len() >= self.max_per_process {
            entries.pop_front();
        }
        entries.push(CachedValidation {
            syscall_nr,
            arg_signature: arg_sig,
            result,
            cached_at: timestamp,
            expires_at: timestamp + 5000, // 5s TTL
        });
    }
}

// ============================================================================
// VALIDATION ENGINE
// ============================================================================

/// Validation statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ValidationStats {
    /// Total validations
    pub total: u64,
    /// Valid
    pub valid: u64,
    /// Invalid
    pub invalid: u64,
    /// Warnings
    pub warnings: u64,
    /// By error type
    pub by_error: BTreeMap<u8, u64>,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

/// Syscall validation engine
#[repr(align(64))]
pub struct ValidationEngine {
    /// Validation specs per syscall
    specs: BTreeMap<u32, SyscallValidationSpec>,
    /// Validation cache
    cache: ValidCache,
    /// Min trust level for cache bypass
    pub cache_bypass_trust: u32,
    /// Statistics
    pub stats: ValidationStats,
    /// Enabled
    pub enabled: bool,
}

impl ValidationEngine {
    pub fn new() -> Self {
        Self {
            specs: BTreeMap::new(),
            cache: ValidCache::new(32),
            cache_bypass_trust: 5,
            stats: ValidationStats::default(),
            enabled: true,
        }
    }

    /// Register validation spec
    #[inline(always)]
    pub fn register_spec(&mut self, spec: SyscallValidationSpec) {
        self.specs.insert(spec.syscall_nr, spec);
    }

    /// Validate syscall arguments
    pub fn validate(
        &mut self,
        syscall_nr: u32,
        args: &[u64; 6],
        ctx: &ValidationContext,
        timestamp: u64,
    ) -> ValidationReport {
        if !self.enabled {
            return ValidationReport {
                syscall_nr,
                result: ValidationResult::Valid,
                findings: Vec::new(),
                validation_time_ns: 0,
            };
        }

        self.stats.total += 1;

        // Check cache for high-trust processes
        if ctx.trust_level >= self.cache_bypass_trust {
            let sig = self.compute_arg_signature(args);
            if let Some(result) = self.cache.lookup(ctx.pid, syscall_nr, sig, timestamp) {
                return ValidationReport {
                    syscall_nr,
                    result,
                    findings: Vec::new(),
                    validation_time_ns: 10,
                };
            }
        }

        let spec = match self.specs.get(&syscall_nr) {
            Some(s) => s,
            None => {
                return ValidationReport {
                    syscall_nr,
                    result: ValidationResult::Valid,
                    findings: Vec::new(),
                    validation_time_ns: 5,
                };
            }
        };

        let mut findings = Vec::new();
        let mut worst = ValidationResult::Valid;

        for rule in &spec.rules {
            let idx = rule.arg_index as usize;
            if idx >= 6 {
                continue;
            }
            let value = args[idx];

            // Skip optional args that are zero
            if rule.optional && value == 0 {
                continue;
            }

            match rule.arg_type {
                ArgType::UserPointer | ArgType::UserString => {
                    if value == 0 && !rule.optional {
                        findings.push(ValidationFinding {
                            arg_index: rule.arg_index,
                            error: ValidationError::NullPointer,
                            actual_value: value,
                            expected: 0,
                        });
                        worst = ValidationResult::Invalid;
                    } else if value != 0 {
                        // Check user space range
                        if value < ctx.user_space_range.0 || value > ctx.user_space_range.1 {
                            findings.push(ValidationFinding {
                                arg_index: rule.arg_index,
                                error: ValidationError::KernelPointer,
                                actual_value: value,
                                expected: ctx.user_space_range.1,
                            });
                            worst = ValidationResult::Invalid;
                        }

                        // Check alignment
                        if let Some(align) = rule.alignment {
                            if align > 0 && value % align != 0 {
                                findings.push(ValidationFinding {
                                    arg_index: rule.arg_index,
                                    error: ValidationError::UnalignedPointer,
                                    actual_value: value,
                                    expected: align,
                                });
                                worst = worst.max(ValidationResult::Warning);
                            }
                        }
                    }
                }
                ArgType::FileDescriptor => {
                    if value > ctx.max_fds as u64 {
                        findings.push(ValidationFinding {
                            arg_index: rule.arg_index,
                            error: ValidationError::InvalidFd,
                            actual_value: value,
                            expected: ctx.max_fds as u64,
                        });
                        worst = ValidationResult::Invalid;
                    }
                }
                ArgType::Flags => {
                    if let Some(allowed) = rule.allowed_flags {
                        if value & !allowed != 0 {
                            findings.push(ValidationFinding {
                                arg_index: rule.arg_index,
                                error: ValidationError::InvalidFlags,
                                actual_value: value,
                                expected: allowed,
                            });
                            worst = ValidationResult::Invalid;
                        }
                    }
                }
                ArgType::Size => {
                    if let Some(max) = rule.max_value {
                        if value > max {
                            findings.push(ValidationFinding {
                                arg_index: rule.arg_index,
                                error: ValidationError::BufferTooLarge,
                                actual_value: value,
                                expected: max,
                            });
                            worst = ValidationResult::Invalid;
                        }
                    }
                }
                ArgType::Integer | ArgType::UnsignedInteger => {
                    if let Some(min) = rule.min_value {
                        if value < min {
                            findings.push(ValidationFinding {
                                arg_index: rule.arg_index,
                                error: ValidationError::IntegerOverflow,
                                actual_value: value,
                                expected: min,
                            });
                            worst = ValidationResult::Invalid;
                        }
                    }
                    if let Some(max) = rule.max_value {
                        if value > max {
                            findings.push(ValidationFinding {
                                arg_index: rule.arg_index,
                                error: ValidationError::IntegerOverflow,
                                actual_value: value,
                                expected: max,
                            });
                            worst = ValidationResult::Invalid;
                        }
                    }
                }
                _ => {}
            }
        }

        match worst {
            ValidationResult::Valid => self.stats.valid += 1,
            ValidationResult::Invalid => self.stats.invalid += 1,
            ValidationResult::Warning => self.stats.warnings += 1,
            _ => {}
        }

        for f in &findings {
            *self.stats.by_error.entry(f.error as u8).or_insert(0) += 1;
        }

        // Cache result for trusted processes
        if ctx.trust_level >= self.cache_bypass_trust {
            let sig = self.compute_arg_signature(args);
            self.cache.insert(ctx.pid, syscall_nr, sig, worst, timestamp);
        }

        // Update cache hit rate
        let total_cache = self.cache.hits + self.cache.misses;
        if total_cache > 0 {
            self.stats.cache_hit_rate = self.cache.hits as f64 / total_cache as f64;
        }

        ValidationReport {
            syscall_nr,
            result: worst,
            findings,
            validation_time_ns: 50,
        }
    }

    /// Compute simple argument signature
    fn compute_arg_signature(&self, args: &[u64; 6]) -> u64 {
        let mut sig: u64 = 0;
        for (i, &arg) in args.iter().enumerate() {
            sig ^= arg.rotate_left((i * 11) as u32);
        }
        sig
    }

    /// Spec count
    #[inline(always)]
    pub fn spec_count(&self) -> usize {
        self.specs.len()
    }
}

impl PartialOrd for ValidationResult {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValidationResult {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}
