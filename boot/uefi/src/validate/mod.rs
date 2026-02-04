//! Validation and Integrity Checking System
//!
//! This module provides comprehensive validation, integrity verification,
//! and sanity checking capabilities for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - File integrity verification (checksums, signatures)
//! - Configuration validation
//! - Memory range validation
//! - Boot entry validation
//! - Pre-boot checks
//! - Hardware capability validation
//! - Security policy validation

#![no_std]

use core::fmt;

// =============================================================================
// VALIDATION RESULT
// =============================================================================

/// Validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Valid/passed
    Valid,
    /// Invalid/failed
    Invalid,
    /// Warning (valid but with issues)
    Warning,
    /// Skipped (not applicable)
    Skipped,
    /// Error during validation
    Error,
    /// Not yet validated
    Pending,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        ValidationStatus::Pending
    }
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationStatus::Valid => write!(f, "Valid"),
            ValidationStatus::Invalid => write!(f, "Invalid"),
            ValidationStatus::Warning => write!(f, "Warning"),
            ValidationStatus::Skipped => write!(f, "Skipped"),
            ValidationStatus::Error => write!(f, "Error"),
            ValidationStatus::Pending => write!(f, "Pending"),
        }
    }
}

/// Validation result
#[derive(Debug, Clone, Copy)]
pub struct ValidationResult {
    /// Status
    pub status: ValidationStatus,
    /// Error code (if failed)
    pub error_code: u32,
    /// Warning count
    pub warnings: u16,
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Duration (microseconds)
    pub duration_us: u32,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            status: ValidationStatus::Pending,
            error_code: 0,
            warnings: 0,
            timestamp_us: 0,
            duration_us: 0,
        }
    }
}

impl ValidationResult {
    /// Create success result
    pub const fn valid() -> Self {
        Self {
            status: ValidationStatus::Valid,
            error_code: 0,
            warnings: 0,
            timestamp_us: 0,
            duration_us: 0,
        }
    }

    /// Create invalid result
    pub const fn invalid(error_code: u32) -> Self {
        Self {
            status: ValidationStatus::Invalid,
            error_code,
            warnings: 0,
            timestamp_us: 0,
            duration_us: 0,
        }
    }

    /// Create warning result
    pub const fn warning(count: u16) -> Self {
        Self {
            status: ValidationStatus::Warning,
            error_code: 0,
            warnings: count,
            timestamp_us: 0,
            duration_us: 0,
        }
    }

    /// Check if passed (valid or warning)
    pub const fn passed(&self) -> bool {
        matches!(
            self.status,
            ValidationStatus::Valid | ValidationStatus::Warning
        )
    }

    /// Check if failed
    pub const fn failed(&self) -> bool {
        matches!(
            self.status,
            ValidationStatus::Invalid | ValidationStatus::Error
        )
    }
}

// =============================================================================
// CHECKSUM TYPES
// =============================================================================

/// Checksum algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumType {
    /// No checksum
    None,
    /// CRC32
    Crc32,
    /// CRC32C (Castagnoli)
    Crc32c,
    /// Adler32
    Adler32,
    /// Fletcher16
    Fletcher16,
    /// Fletcher32
    Fletcher32,
    /// MD5
    Md5,
    /// SHA-1
    Sha1,
    /// SHA-256
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
    /// XXHash32
    XxHash32,
    /// XXHash64
    XxHash64,
}

impl Default for ChecksumType {
    fn default() -> Self {
        ChecksumType::Crc32
    }
}

/// Checksum value
#[derive(Debug, Clone, Copy)]
pub enum ChecksumValue {
    /// 16-bit value
    U16(u16),
    /// 32-bit value
    U32(u32),
    /// 64-bit value
    U64(u64),
    /// 128-bit value (MD5)
    U128([u8; 16]),
    /// 160-bit value (SHA-1)
    U160([u8; 20]),
    /// 256-bit value (SHA-256)
    U256([u8; 32]),
    /// 384-bit value (SHA-384)
    U384([u8; 48]),
    /// 512-bit value (SHA-512)
    U512([u8; 64]),
}

impl Default for ChecksumValue {
    fn default() -> Self {
        ChecksumValue::U32(0)
    }
}

impl ChecksumValue {
    /// Get size in bytes
    pub const fn size(&self) -> usize {
        match self {
            ChecksumValue::U16(_) => 2,
            ChecksumValue::U32(_) => 4,
            ChecksumValue::U64(_) => 8,
            ChecksumValue::U128(_) => 16,
            ChecksumValue::U160(_) => 20,
            ChecksumValue::U256(_) => 32,
            ChecksumValue::U384(_) => 48,
            ChecksumValue::U512(_) => 64,
        }
    }

    /// Compare checksums
    pub fn equals(&self, other: &ChecksumValue) -> bool {
        match (self, other) {
            (ChecksumValue::U16(a), ChecksumValue::U16(b)) => a == b,
            (ChecksumValue::U32(a), ChecksumValue::U32(b)) => a == b,
            (ChecksumValue::U64(a), ChecksumValue::U64(b)) => a == b,
            (ChecksumValue::U128(a), ChecksumValue::U128(b)) => a == b,
            (ChecksumValue::U160(a), ChecksumValue::U160(b)) => a == b,
            (ChecksumValue::U256(a), ChecksumValue::U256(b)) => a == b,
            (ChecksumValue::U384(a), ChecksumValue::U384(b)) => a == b,
            (ChecksumValue::U512(a), ChecksumValue::U512(b)) => a == b,
            _ => false,
        }
    }
}

/// Checksum specification
#[derive(Debug, Clone, Copy)]
pub struct ChecksumSpec {
    /// Algorithm
    pub algorithm: ChecksumType,
    /// Expected value
    pub expected: ChecksumValue,
    /// Offset in file (for embedded checksums)
    pub offset: Option<u64>,
    /// Range start (None = entire file)
    pub range_start: Option<u64>,
    /// Range end (None = end of file)
    pub range_end: Option<u64>,
}

impl Default for ChecksumSpec {
    fn default() -> Self {
        Self {
            algorithm: ChecksumType::Crc32,
            expected: ChecksumValue::U32(0),
            offset: None,
            range_start: None,
            range_end: None,
        }
    }
}

// =============================================================================
// FILE VALIDATION
// =============================================================================

/// File validation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileValidationFlags(u16);

impl FileValidationFlags {
    pub const NONE: FileValidationFlags = FileValidationFlags(0);
    pub const CHECK_EXISTS: FileValidationFlags = FileValidationFlags(1);
    pub const CHECK_READABLE: FileValidationFlags = FileValidationFlags(2);
    pub const CHECK_SIZE: FileValidationFlags = FileValidationFlags(4);
    pub const CHECK_CHECKSUM: FileValidationFlags = FileValidationFlags(8);
    pub const CHECK_SIGNATURE: FileValidationFlags = FileValidationFlags(16);
    pub const CHECK_MAGIC: FileValidationFlags = FileValidationFlags(32);
    pub const CHECK_HEADER: FileValidationFlags = FileValidationFlags(64);
    pub const CHECK_FORMAT: FileValidationFlags = FileValidationFlags(128);
    pub const ALL: FileValidationFlags = FileValidationFlags(0xFF);

    /// Get raw value
    pub const fn raw(&self) -> u16 {
        self.0
    }

    /// Check flag
    pub const fn has(&self, flag: FileValidationFlags) -> bool {
        self.0 & flag.0 != 0
    }

    /// Combine flags
    pub const fn with(self, other: FileValidationFlags) -> FileValidationFlags {
        FileValidationFlags(self.0 | other.0)
    }
}

/// Maximum file path length
pub const MAX_FILE_PATH: usize = 256;

/// File validation request
#[derive(Debug, Clone, Copy)]
pub struct FileValidation {
    /// File path
    pub path: [u8; MAX_FILE_PATH],
    /// Path length
    pub path_len: usize,
    /// Validation flags
    pub flags: FileValidationFlags,
    /// Minimum size
    pub min_size: u64,
    /// Maximum size
    pub max_size: u64,
    /// Checksum spec
    pub checksum: ChecksumSpec,
    /// Expected magic bytes
    pub magic: [u8; 16],
    /// Magic length
    pub magic_len: usize,
    /// Magic offset
    pub magic_offset: u64,
}

impl Default for FileValidation {
    fn default() -> Self {
        Self {
            path: [0u8; MAX_FILE_PATH],
            path_len: 0,
            flags: FileValidationFlags::CHECK_EXISTS,
            min_size: 0,
            max_size: u64::MAX,
            checksum: ChecksumSpec::default(),
            magic: [0u8; 16],
            magic_len: 0,
            magic_offset: 0,
        }
    }
}

/// File validation result
#[derive(Debug, Clone, Copy, Default)]
pub struct FileValidationResult {
    /// Overall result
    pub result: ValidationResult,
    /// File exists
    pub exists: bool,
    /// File readable
    pub readable: bool,
    /// Actual file size
    pub actual_size: u64,
    /// Size valid
    pub size_valid: bool,
    /// Checksum valid
    pub checksum_valid: bool,
    /// Signature valid
    pub signature_valid: bool,
    /// Magic valid
    pub magic_valid: bool,
    /// Header valid
    pub header_valid: bool,
}

// =============================================================================
// MEMORY VALIDATION
// =============================================================================

/// Memory validation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryValidationType {
    /// Range is valid (non-null, aligned, in bounds)
    Range,
    /// Memory is readable
    Readable,
    /// Memory is writable
    Writable,
    /// Memory is executable
    Executable,
    /// Memory is available
    Available,
    /// Memory is contiguous
    Contiguous,
    /// Memory matches pattern
    Pattern,
    /// Memory is zeroed
    Zeroed,
}

impl Default for MemoryValidationType {
    fn default() -> Self {
        MemoryValidationType::Range
    }
}

/// Memory validation request
#[derive(Debug, Clone, Copy)]
pub struct MemoryValidation {
    /// Start address
    pub address: u64,
    /// Size
    pub size: u64,
    /// Validation type
    pub validation_type: MemoryValidationType,
    /// Required alignment
    pub alignment: u64,
    /// Pattern (for pattern check)
    pub pattern: [u8; 16],
    /// Pattern length
    pub pattern_len: usize,
}

impl Default for MemoryValidation {
    fn default() -> Self {
        Self {
            address: 0,
            size: 0,
            validation_type: MemoryValidationType::Range,
            alignment: 1,
            pattern: [0u8; 16],
            pattern_len: 0,
        }
    }
}

impl MemoryValidation {
    /// Create range validation
    pub const fn range(address: u64, size: u64) -> Self {
        Self {
            address,
            size,
            validation_type: MemoryValidationType::Range,
            alignment: 1,
            pattern: [0u8; 16],
            pattern_len: 0,
        }
    }

    /// Create aligned range validation
    pub const fn aligned(address: u64, size: u64, alignment: u64) -> Self {
        Self {
            address,
            size,
            validation_type: MemoryValidationType::Range,
            alignment,
            pattern: [0u8; 16],
            pattern_len: 0,
        }
    }
}

/// Memory validation result
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryValidationResult {
    /// Overall result
    pub result: ValidationResult,
    /// Address is valid
    pub address_valid: bool,
    /// Size is valid
    pub size_valid: bool,
    /// Alignment correct
    pub aligned: bool,
    /// Permission check passed
    pub permission_ok: bool,
    /// Actual memory type found
    pub memory_type: u8,
}

// =============================================================================
// BOOT ENTRY VALIDATION
// =============================================================================

/// Entry validation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntryValidationFlags(u16);

impl EntryValidationFlags {
    pub const NONE: EntryValidationFlags = EntryValidationFlags(0);
    pub const CHECK_PATH: EntryValidationFlags = EntryValidationFlags(1);
    pub const CHECK_KERNEL: EntryValidationFlags = EntryValidationFlags(2);
    pub const CHECK_INITRD: EntryValidationFlags = EntryValidationFlags(4);
    pub const CHECK_ARGS: EntryValidationFlags = EntryValidationFlags(8);
    pub const CHECK_DEVICE: EntryValidationFlags = EntryValidationFlags(16);
    pub const CHECK_SIGNATURE: EntryValidationFlags = EntryValidationFlags(32);
    pub const CHECK_BOOTABLE: EntryValidationFlags = EntryValidationFlags(64);
    pub const ALL: EntryValidationFlags = EntryValidationFlags(0x7F);

    /// Get raw value
    pub const fn raw(&self) -> u16 {
        self.0
    }

    /// Check flag
    pub const fn has(&self, flag: EntryValidationFlags) -> bool {
        self.0 & flag.0 != 0
    }

    /// Combine flags
    pub const fn with(self, other: EntryValidationFlags) -> EntryValidationFlags {
        EntryValidationFlags(self.0 | other.0)
    }
}

/// Boot entry validation result
#[derive(Debug, Clone, Copy, Default)]
pub struct EntryValidationResult {
    /// Overall result
    pub result: ValidationResult,
    /// Entry index
    pub entry_index: u16,
    /// Path valid
    pub path_valid: bool,
    /// Kernel exists
    pub kernel_exists: bool,
    /// Kernel valid
    pub kernel_valid: bool,
    /// Initrd exists
    pub initrd_exists: bool,
    /// Initrd valid
    pub initrd_valid: bool,
    /// Args valid
    pub args_valid: bool,
    /// Device accessible
    pub device_ok: bool,
    /// Signature valid
    pub signature_ok: bool,
    /// Entry is bootable
    pub bootable: bool,
}

// =============================================================================
// CONFIG VALIDATION
// =============================================================================

/// Configuration validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValidationType {
    /// Syntax check
    Syntax,
    /// Semantic check
    Semantic,
    /// Reference check
    References,
    /// Range check
    Ranges,
    /// Compatibility check
    Compatibility,
    /// Security check
    Security,
    /// Full validation
    Full,
}

impl Default for ConfigValidationType {
    fn default() -> Self {
        ConfigValidationType::Full
    }
}

/// Config validation error
#[derive(Debug, Clone, Copy)]
pub struct ConfigError {
    /// Error type
    pub error_type: ConfigErrorType,
    /// Line number (if applicable)
    pub line: u32,
    /// Column (if applicable)
    pub column: u32,
    /// Error code
    pub code: u16,
}

impl Default for ConfigError {
    fn default() -> Self {
        Self {
            error_type: ConfigErrorType::Unknown,
            line: 0,
            column: 0,
            code: 0,
        }
    }
}

/// Config error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorType {
    /// Unknown error
    Unknown,
    /// Syntax error
    Syntax,
    /// Unknown key
    UnknownKey,
    /// Invalid value
    InvalidValue,
    /// Missing required
    MissingRequired,
    /// Duplicate key
    Duplicate,
    /// Invalid reference
    InvalidReference,
    /// Out of range
    OutOfRange,
    /// Incompatible
    Incompatible,
    /// Security issue
    Security,
}

impl Default for ConfigErrorType {
    fn default() -> Self {
        ConfigErrorType::Unknown
    }
}

/// Maximum config errors to track
pub const MAX_CONFIG_ERRORS: usize = 16;

/// Config validation result
#[derive(Debug, Clone, Copy)]
pub struct ConfigValidationResult {
    /// Overall result
    pub result: ValidationResult,
    /// Errors
    pub errors: [ConfigError; MAX_CONFIG_ERRORS],
    /// Error count
    pub error_count: usize,
    /// Total entries validated
    pub entries_checked: u32,
    /// Valid entries
    pub entries_valid: u32,
}

impl Default for ConfigValidationResult {
    fn default() -> Self {
        Self {
            result: ValidationResult::default(),
            errors: [ConfigError::default(); MAX_CONFIG_ERRORS],
            error_count: 0,
            entries_checked: 0,
            entries_valid: 0,
        }
    }
}

impl ConfigValidationResult {
    /// Add error
    pub fn add_error(&mut self, error: ConfigError) {
        if self.error_count < MAX_CONFIG_ERRORS {
            self.errors[self.error_count] = error;
            self.error_count += 1;
        }
    }

    /// Check if has errors
    pub const fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

// =============================================================================
// PRE-BOOT CHECKS
// =============================================================================

/// Pre-boot check type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreBootCheck {
    /// Memory sufficient
    Memory,
    /// CPU capabilities
    Cpu,
    /// Storage accessible
    Storage,
    /// Graphics available
    Graphics,
    /// Network available
    Network,
    /// Security status
    Security,
    /// Configuration valid
    Config,
    /// Boot entry valid
    BootEntry,
    /// Dependencies met
    Dependencies,
    /// Custom check
    Custom(u16),
}

impl Default for PreBootCheck {
    fn default() -> Self {
        PreBootCheck::Memory
    }
}

/// Pre-boot check result
#[derive(Debug, Clone, Copy, Default)]
pub struct PreBootCheckResult {
    /// Check type
    pub check_type: PreBootCheck,
    /// Result
    pub result: ValidationResult,
    /// Is critical
    pub critical: bool,
    /// Can continue
    pub can_continue: bool,
}

/// Maximum pre-boot checks
pub const MAX_PREBOOT_CHECKS: usize = 16;

/// Pre-boot validation suite
#[derive(Debug)]
pub struct PreBootValidation {
    /// Checks to run
    checks: [PreBootCheck; MAX_PREBOOT_CHECKS],
    /// Check count
    check_count: usize,
    /// Results
    results: [PreBootCheckResult; MAX_PREBOOT_CHECKS],
    /// Result count
    result_count: usize,
    /// Overall status
    pub overall: ValidationStatus,
}

impl Default for PreBootValidation {
    fn default() -> Self {
        Self::new()
    }
}

impl PreBootValidation {
    /// Create new validation suite
    pub const fn new() -> Self {
        Self {
            checks: [PreBootCheck::Memory; MAX_PREBOOT_CHECKS],
            check_count: 0,
            results: [PreBootCheckResult {
                check_type: PreBootCheck::Memory,
                result: ValidationResult {
                    status: ValidationStatus::Pending,
                    error_code: 0,
                    warnings: 0,
                    timestamp_us: 0,
                    duration_us: 0,
                },
                critical: false,
                can_continue: true,
            }; MAX_PREBOOT_CHECKS],
            result_count: 0,
            overall: ValidationStatus::Pending,
        }
    }

    /// Add check to run
    pub fn add_check(&mut self, check: PreBootCheck) -> bool {
        if self.check_count >= MAX_PREBOOT_CHECKS {
            return false;
        }
        self.checks[self.check_count] = check;
        self.check_count += 1;
        true
    }

    /// Add result
    pub fn add_result(&mut self, result: PreBootCheckResult) -> bool {
        if self.result_count >= MAX_PREBOOT_CHECKS {
            return false;
        }
        self.results[self.result_count] = result;
        self.result_count += 1;
        true
    }

    /// Check if all passed
    pub fn all_passed(&self) -> bool {
        for i in 0..self.result_count {
            if !self.results[i].result.passed() {
                return false;
            }
        }
        self.result_count > 0
    }

    /// Check if can continue
    pub fn can_continue(&self) -> bool {
        for i in 0..self.result_count {
            if self.results[i].critical && !self.results[i].can_continue {
                return false;
            }
        }
        true
    }

    /// Count failures
    pub fn failure_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.result_count {
            if self.results[i].result.failed() {
                count += 1;
            }
        }
        count
    }

    /// Get result count
    pub const fn len(&self) -> usize {
        self.result_count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.result_count == 0
    }
}

// =============================================================================
// HARDWARE CAPABILITY VALIDATION
// =============================================================================

/// Required CPU feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFeature {
    /// Long mode (64-bit)
    LongMode,
    /// PAE
    Pae,
    /// SSE
    Sse,
    /// SSE2
    Sse2,
    /// SSE3
    Sse3,
    /// SSSE3
    Ssse3,
    /// SSE4.1
    Sse41,
    /// SSE4.2
    Sse42,
    /// AVX
    Avx,
    /// AVX2
    Avx2,
    /// AVX512F
    Avx512f,
    /// AES-NI
    AesNi,
    /// SHA extensions
    Sha,
    /// FPU
    Fpu,
    /// NX bit
    Nx,
    /// SMEP
    Smep,
    /// SMAP
    Smap,
    /// XSAVE
    Xsave,
    /// RDRAND
    Rdrand,
    /// RDSEED
    Rdseed,
    /// TSC
    Tsc,
    /// APIC
    Apic,
    /// X2APIC
    X2apic,
}

impl Default for CpuFeature {
    fn default() -> Self {
        CpuFeature::LongMode
    }
}

/// Hardware requirements
#[derive(Debug, Clone, Copy)]
pub struct HardwareRequirements {
    /// Required CPU features
    pub cpu_features: [CpuFeature; 16],
    /// Feature count
    pub feature_count: usize,
    /// Minimum memory (bytes)
    pub min_memory: u64,
    /// Recommended memory (bytes)
    pub recommended_memory: u64,
    /// Minimum CPU cores
    pub min_cores: u8,
    /// Graphics required
    pub graphics_required: bool,
    /// Framebuffer required
    pub framebuffer_required: bool,
    /// Minimum resolution width
    pub min_width: u16,
    /// Minimum resolution height
    pub min_height: u16,
    /// Network required
    pub network_required: bool,
    /// Storage required (bytes)
    pub storage_required: u64,
    /// TPM required
    pub tpm_required: bool,
    /// Secure boot required
    pub secure_boot_required: bool,
}

impl Default for HardwareRequirements {
    fn default() -> Self {
        Self {
            cpu_features: [CpuFeature::LongMode; 16],
            feature_count: 0,
            min_memory: 64 * 1024 * 1024,          // 64 MB
            recommended_memory: 256 * 1024 * 1024, // 256 MB
            min_cores: 1,
            graphics_required: false,
            framebuffer_required: false,
            min_width: 640,
            min_height: 480,
            network_required: false,
            storage_required: 0,
            tpm_required: false,
            secure_boot_required: false,
        }
    }
}

/// Hardware validation result
#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareValidationResult {
    /// Overall result
    pub result: ValidationResult,
    /// CPU features met
    pub cpu_ok: bool,
    /// Missing features
    pub missing_features: [CpuFeature; 8],
    /// Missing feature count
    pub missing_count: usize,
    /// Memory sufficient
    pub memory_ok: bool,
    /// Actual memory (bytes)
    pub actual_memory: u64,
    /// Core count met
    pub cores_ok: bool,
    /// Actual cores
    pub actual_cores: u8,
    /// Graphics met
    pub graphics_ok: bool,
    /// Network met
    pub network_ok: bool,
    /// Storage met
    pub storage_ok: bool,
    /// TPM met
    pub tpm_ok: bool,
    /// Secure boot met
    pub secure_boot_ok: bool,
}

// =============================================================================
// VALIDATION SUITE
// =============================================================================

/// Validation suite for comprehensive validation
#[derive(Debug)]
pub struct ValidationSuite {
    /// Pre-boot validation
    pub pre_boot: PreBootValidation,
    /// Hardware validation result
    pub hardware: HardwareValidationResult,
    /// Config validation result
    pub config: ConfigValidationResult,
    /// Entry validation results
    pub entries: [EntryValidationResult; 16],
    /// Entry result count
    pub entry_count: usize,
    /// Overall status
    pub overall: ValidationStatus,
    /// Total checks run
    pub total_checks: u32,
    /// Passed checks
    pub passed_checks: u32,
    /// Failed checks
    pub failed_checks: u32,
    /// Warnings
    pub warning_count: u32,
}

impl Default for ValidationSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationSuite {
    /// Create new suite
    pub fn new() -> Self {
        Self {
            pre_boot: PreBootValidation::new(),
            hardware: HardwareValidationResult::default(),
            config: ConfigValidationResult::default(),
            entries: [EntryValidationResult::default(); 16],
            entry_count: 0,
            overall: ValidationStatus::Pending,
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            warning_count: 0,
        }
    }

    /// Add entry result
    pub fn add_entry_result(&mut self, result: EntryValidationResult) -> bool {
        if self.entry_count >= 16 {
            return false;
        }
        self.entries[self.entry_count] = result;
        self.entry_count += 1;
        self.update_counters(&result.result);
        true
    }

    /// Update counters from result
    fn update_counters(&mut self, result: &ValidationResult) {
        self.total_checks += 1;
        match result.status {
            ValidationStatus::Valid => self.passed_checks += 1,
            ValidationStatus::Warning => {
                self.passed_checks += 1;
                self.warning_count += result.warnings as u32;
            },
            ValidationStatus::Invalid | ValidationStatus::Error => {
                self.failed_checks += 1;
            },
            _ => {},
        }
    }

    /// Calculate overall status
    pub fn finalize(&mut self) {
        if self.failed_checks > 0 {
            self.overall = ValidationStatus::Invalid;
        } else if self.warning_count > 0 {
            self.overall = ValidationStatus::Warning;
        } else if self.passed_checks > 0 {
            self.overall = ValidationStatus::Valid;
        } else {
            self.overall = ValidationStatus::Pending;
        }
    }

    /// Get success rate (percent)
    pub fn success_rate(&self) -> u8 {
        if self.total_checks == 0 {
            return 0;
        }
        ((self.passed_checks * 100) / self.total_checks) as u8
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.passed());
        assert!(!valid.failed());

        let invalid = ValidationResult::invalid(1);
        assert!(!invalid.passed());
        assert!(invalid.failed());

        let warning = ValidationResult::warning(2);
        assert!(warning.passed());
        assert!(!warning.failed());
    }

    #[test]
    fn test_checksum_value() {
        let crc = ChecksumValue::U32(0x12345678);
        assert_eq!(crc.size(), 4);

        let sha256 = ChecksumValue::U256([0u8; 32]);
        assert_eq!(sha256.size(), 32);

        let a = ChecksumValue::U32(100);
        let b = ChecksumValue::U32(100);
        assert!(a.equals(&b));
    }

    #[test]
    fn test_file_validation_flags() {
        let flags = FileValidationFlags::CHECK_EXISTS.with(FileValidationFlags::CHECK_CHECKSUM);
        assert!(flags.has(FileValidationFlags::CHECK_EXISTS));
        assert!(flags.has(FileValidationFlags::CHECK_CHECKSUM));
        assert!(!flags.has(FileValidationFlags::CHECK_SIGNATURE));
    }

    #[test]
    fn test_pre_boot_validation() {
        let mut validation = PreBootValidation::new();
        assert!(validation.is_empty());

        validation.add_check(PreBootCheck::Memory);
        validation.add_check(PreBootCheck::Cpu);

        validation.add_result(PreBootCheckResult {
            check_type: PreBootCheck::Memory,
            result: ValidationResult::valid(),
            critical: true,
            can_continue: true,
        });

        assert!(!validation.is_empty());
        assert!(validation.can_continue());
    }

    #[test]
    fn test_validation_suite() {
        let mut suite = ValidationSuite::new();

        suite.add_entry_result(EntryValidationResult {
            result: ValidationResult::valid(),
            entry_index: 0,
            bootable: true,
            ..Default::default()
        });

        suite.finalize();
        assert_eq!(suite.overall, ValidationStatus::Valid);
        assert_eq!(suite.success_rate(), 100);
    }
}
