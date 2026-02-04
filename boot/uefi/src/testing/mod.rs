//! Testing and Validation Framework for Helix UEFI Bootloader
//!
//! This module provides comprehensive testing infrastructure including
//! hardware validation, boot verification, and diagnostic tests.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Testing Framework                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Hardware Tests                                │   │
//! │  │  Memory │ CPU │ Storage │ Network │ Graphics │ USB              │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Boot Verification                             │   │
//! │  │  Secure Boot │ Chain of Trust │ Signature │ Integrity           │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Diagnostics                                   │   │
//! │  │  POST │ BIST │ Error Detection │ Recovery                       │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Reporting                                     │   │
//! │  │  Results │ Logs │ Statistics │ Export                           │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// TEST RESULT
// =============================================================================

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed
    Pass,
    /// Test failed
    Fail,
    /// Test skipped
    Skip,
    /// Test timed out
    Timeout,
    /// Test aborted
    Abort,
    /// Test not run
    NotRun,
    /// Test in progress
    Running,
    /// Test completed with warnings
    Warning,
}

impl TestStatus {
    /// Check if test passed (including with warnings)
    pub const fn is_pass(&self) -> bool {
        matches!(self, TestStatus::Pass | TestStatus::Warning)
    }

    /// Check if test failed
    pub const fn is_fail(&self) -> bool {
        matches!(
            self,
            TestStatus::Fail | TestStatus::Timeout | TestStatus::Abort
        )
    }
}

impl fmt::Display for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Pass => write!(f, "PASS"),
            TestStatus::Fail => write!(f, "FAIL"),
            TestStatus::Skip => write!(f, "SKIP"),
            TestStatus::Timeout => write!(f, "TIMEOUT"),
            TestStatus::Abort => write!(f, "ABORT"),
            TestStatus::NotRun => write!(f, "NOT RUN"),
            TestStatus::Running => write!(f, "RUNNING"),
            TestStatus::Warning => write!(f, "WARNING"),
        }
    }
}

impl Default for TestStatus {
    fn default() -> Self {
        TestStatus::NotRun
    }
}

/// Test result with details
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Status
    pub status: TestStatus,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Error code (if failed)
    pub error_code: u32,
    /// Message
    pub message: [u8; 128],
    /// Message length
    pub message_len: usize,
}

impl TestResult {
    /// Create new passing result
    pub const fn pass(duration_us: u64) -> Self {
        Self {
            status: TestStatus::Pass,
            duration_us,
            error_code: 0,
            message: [0u8; 128],
            message_len: 0,
        }
    }

    /// Create new failing result
    pub const fn fail(error_code: u32) -> Self {
        Self {
            status: TestStatus::Fail,
            duration_us: 0,
            error_code,
            message: [0u8; 128],
            message_len: 0,
        }
    }

    /// Create skipped result
    pub const fn skip() -> Self {
        Self {
            status: TestStatus::Skip,
            duration_us: 0,
            error_code: 0,
            message: [0u8; 128],
            message_len: 0,
        }
    }
}

impl Default for TestResult {
    fn default() -> Self {
        Self {
            status: TestStatus::NotRun,
            duration_us: 0,
            error_code: 0,
            message: [0u8; 128],
            message_len: 0,
        }
    }
}

// =============================================================================
// TEST CATEGORIES
// =============================================================================

/// Test category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCategory {
    /// Memory tests
    Memory,
    /// CPU tests
    Cpu,
    /// Storage tests
    Storage,
    /// Network tests
    Network,
    /// Graphics tests
    Graphics,
    /// USB tests
    Usb,
    /// Security tests
    Security,
    /// Boot tests
    Boot,
    /// Firmware tests
    Firmware,
    /// Integration tests
    Integration,
    /// Performance tests
    Performance,
    /// Stress tests
    Stress,
}

impl fmt::Display for TestCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestCategory::Memory => write!(f, "Memory"),
            TestCategory::Cpu => write!(f, "CPU"),
            TestCategory::Storage => write!(f, "Storage"),
            TestCategory::Network => write!(f, "Network"),
            TestCategory::Graphics => write!(f, "Graphics"),
            TestCategory::Usb => write!(f, "USB"),
            TestCategory::Security => write!(f, "Security"),
            TestCategory::Boot => write!(f, "Boot"),
            TestCategory::Firmware => write!(f, "Firmware"),
            TestCategory::Integration => write!(f, "Integration"),
            TestCategory::Performance => write!(f, "Performance"),
            TestCategory::Stress => write!(f, "Stress"),
        }
    }
}

/// Test severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestSeverity {
    /// Informational
    Info,
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical
    Critical,
}

impl Default for TestSeverity {
    fn default() -> Self {
        TestSeverity::Medium
    }
}

// =============================================================================
// MEMORY TESTS
// =============================================================================

/// Memory test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTest {
    /// Walking ones pattern
    WalkingOnes,
    /// Walking zeros pattern
    WalkingZeros,
    /// Checkerboard pattern
    Checkerboard,
    /// Address in address
    AddressInAddress,
    /// Random data
    Random,
    /// All zeros
    AllZeros,
    /// All ones
    AllOnes,
    /// Sequential
    Sequential,
    /// Block move
    BlockMove,
    /// Cache test
    Cache,
    /// ECC test
    Ecc,
}

impl MemoryTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            MemoryTest::WalkingOnes => "Walking Ones",
            MemoryTest::WalkingZeros => "Walking Zeros",
            MemoryTest::Checkerboard => "Checkerboard",
            MemoryTest::AddressInAddress => "Address-in-Address",
            MemoryTest::Random => "Random Data",
            MemoryTest::AllZeros => "All Zeros",
            MemoryTest::AllOnes => "All Ones",
            MemoryTest::Sequential => "Sequential",
            MemoryTest::BlockMove => "Block Move",
            MemoryTest::Cache => "Cache",
            MemoryTest::Ecc => "ECC",
        }
    }

    /// Get test pattern (for simple patterns)
    pub const fn pattern(&self) -> u64 {
        match self {
            MemoryTest::AllZeros => 0x0000000000000000,
            MemoryTest::AllOnes => 0xFFFFFFFFFFFFFFFF,
            MemoryTest::Checkerboard => 0xAAAAAAAAAAAAAAAA,
            _ => 0,
        }
    }
}

/// Memory test configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryTestConfig {
    /// Tests to run
    pub tests: u32,
    /// Start address
    pub start_addr: u64,
    /// End address
    pub end_addr: u64,
    /// Pattern seed for random tests
    pub seed: u64,
    /// Number of passes
    pub passes: u32,
    /// Timeout in seconds
    pub timeout_secs: u32,
    /// Stop on first error
    pub stop_on_error: bool,
}

impl Default for MemoryTestConfig {
    fn default() -> Self {
        Self {
            tests: 0xFFFF, // All tests
            start_addr: 0,
            end_addr: 0, // Will be detected
            seed: 0x12345678,
            passes: 1,
            timeout_secs: 300,
            stop_on_error: false,
        }
    }
}

/// Memory test result
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryTestResult {
    /// Bytes tested
    pub bytes_tested: u64,
    /// Errors found
    pub errors: u32,
    /// First error address
    pub first_error_addr: u64,
    /// Expected value at error
    pub expected: u64,
    /// Actual value at error
    pub actual: u64,
    /// Test that failed
    pub failed_test: Option<MemoryTest>,
}

// =============================================================================
// CPU TESTS
// =============================================================================

/// CPU test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuTest {
    /// Integer ALU
    IntAlu,
    /// Floating point
    FloatingPoint,
    /// SIMD (SSE/AVX)
    Simd,
    /// Cache coherency
    CacheCoherency,
    /// Branch prediction
    BranchPrediction,
    /// Instruction decoding
    InstructionDecode,
    /// TLB
    Tlb,
    /// Microcode
    Microcode,
    /// Temperature
    Temperature,
    /// Frequency
    Frequency,
}

impl CpuTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            CpuTest::IntAlu => "Integer ALU",
            CpuTest::FloatingPoint => "Floating Point",
            CpuTest::Simd => "SIMD",
            CpuTest::CacheCoherency => "Cache Coherency",
            CpuTest::BranchPrediction => "Branch Prediction",
            CpuTest::InstructionDecode => "Instruction Decode",
            CpuTest::Tlb => "TLB",
            CpuTest::Microcode => "Microcode",
            CpuTest::Temperature => "Temperature",
            CpuTest::Frequency => "Frequency",
        }
    }
}

/// CPU feature verification
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFeatureCheck {
    /// Has SSE
    pub sse: bool,
    /// Has SSE2
    pub sse2: bool,
    /// Has SSE3
    pub sse3: bool,
    /// Has SSSE3
    pub ssse3: bool,
    /// Has SSE4.1
    pub sse41: bool,
    /// Has SSE4.2
    pub sse42: bool,
    /// Has AVX
    pub avx: bool,
    /// Has AVX2
    pub avx2: bool,
    /// Has AVX-512
    pub avx512: bool,
    /// Has AES-NI
    pub aesni: bool,
    /// Has SHA extensions
    pub sha: bool,
    /// Has RDRAND
    pub rdrand: bool,
    /// Has RDSEED
    pub rdseed: bool,
    /// Has TSC
    pub tsc: bool,
    /// Has invariant TSC
    pub tsc_invariant: bool,
    /// Has APIC
    pub apic: bool,
    /// Has x2APIC
    pub x2apic: bool,
    /// Has 1GB pages
    pub huge_pages: bool,
    /// Has NX bit
    pub nx: bool,
    /// Has SMEP
    pub smep: bool,
    /// Has SMAP
    pub smap: bool,
}

// =============================================================================
// STORAGE TESTS
// =============================================================================

/// Storage test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageTest {
    /// Read speed
    ReadSpeed,
    /// Write speed
    WriteSpeed,
    /// Random read
    RandomRead,
    /// Random write
    RandomWrite,
    /// Sequential read
    SequentialRead,
    /// Sequential write
    SequentialWrite,
    /// Verify data
    Verify,
    /// SMART check
    Smart,
    /// Surface scan
    SurfaceScan,
    /// Partition check
    PartitionCheck,
    /// Filesystem check
    FilesystemCheck,
}

impl StorageTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            StorageTest::ReadSpeed => "Read Speed",
            StorageTest::WriteSpeed => "Write Speed",
            StorageTest::RandomRead => "Random Read",
            StorageTest::RandomWrite => "Random Write",
            StorageTest::SequentialRead => "Sequential Read",
            StorageTest::SequentialWrite => "Sequential Write",
            StorageTest::Verify => "Data Verification",
            StorageTest::Smart => "SMART Check",
            StorageTest::SurfaceScan => "Surface Scan",
            StorageTest::PartitionCheck => "Partition Check",
            StorageTest::FilesystemCheck => "Filesystem Check",
        }
    }
}

/// Storage test result
#[derive(Debug, Clone, Copy, Default)]
pub struct StorageTestResult {
    /// Read speed (KB/s)
    pub read_speed_kbps: u64,
    /// Write speed (KB/s)
    pub write_speed_kbps: u64,
    /// IOPS (read)
    pub read_iops: u32,
    /// IOPS (write)
    pub write_iops: u32,
    /// Average latency (microseconds)
    pub avg_latency_us: u32,
    /// Max latency (microseconds)
    pub max_latency_us: u32,
    /// Bad sectors found
    pub bad_sectors: u32,
    /// SMART status OK
    pub smart_ok: bool,
}

// =============================================================================
// NETWORK TESTS
// =============================================================================

/// Network test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTest {
    /// Link status
    LinkStatus,
    /// DHCP
    Dhcp,
    /// DNS resolution
    Dns,
    /// Ping
    Ping,
    /// TCP connection
    TcpConnect,
    /// HTTP request
    HttpRequest,
    /// PXE boot
    PxeBoot,
    /// Throughput
    Throughput,
    /// Loopback
    Loopback,
}

impl NetworkTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            NetworkTest::LinkStatus => "Link Status",
            NetworkTest::Dhcp => "DHCP",
            NetworkTest::Dns => "DNS Resolution",
            NetworkTest::Ping => "Ping",
            NetworkTest::TcpConnect => "TCP Connection",
            NetworkTest::HttpRequest => "HTTP Request",
            NetworkTest::PxeBoot => "PXE Boot",
            NetworkTest::Throughput => "Throughput",
            NetworkTest::Loopback => "Loopback",
        }
    }
}

/// Network test result
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkTestResult {
    /// Link up
    pub link_up: bool,
    /// Link speed (Mbps)
    pub link_speed_mbps: u32,
    /// IP address obtained
    pub ip_obtained: bool,
    /// Ping latency (ms)
    pub ping_latency_ms: u32,
    /// Packet loss percentage
    pub packet_loss_percent: u8,
    /// Throughput (Mbps)
    pub throughput_mbps: u32,
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

/// Security test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityTest {
    /// Secure Boot status
    SecureBootStatus,
    /// Key verification
    KeyVerification,
    /// Signature check
    SignatureCheck,
    /// Certificate chain
    CertificateChain,
    /// TPM presence
    TpmPresence,
    /// TPM PCR values
    TpmPcr,
    /// Measured boot
    MeasuredBoot,
    /// Password protection
    PasswordProtection,
    /// Secure variables
    SecureVariables,
}

impl SecurityTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            SecurityTest::SecureBootStatus => "Secure Boot Status",
            SecurityTest::KeyVerification => "Key Verification",
            SecurityTest::SignatureCheck => "Signature Check",
            SecurityTest::CertificateChain => "Certificate Chain",
            SecurityTest::TpmPresence => "TPM Presence",
            SecurityTest::TpmPcr => "TPM PCR Values",
            SecurityTest::MeasuredBoot => "Measured Boot",
            SecurityTest::PasswordProtection => "Password Protection",
            SecurityTest::SecureVariables => "Secure Variables",
        }
    }
}

/// Security test result
#[derive(Debug, Clone, Copy, Default)]
pub struct SecurityTestResult {
    /// Secure Boot enabled
    pub secure_boot_enabled: bool,
    /// Secure Boot mode
    pub secure_boot_mode: u8,
    /// TPM version (0 = not present)
    pub tpm_version: u8,
    /// Keys verified
    pub keys_valid: bool,
    /// Signature verified
    pub signature_valid: bool,
    /// Certificate chain valid
    pub chain_valid: bool,
}

// =============================================================================
// GRAPHICS TESTS
// =============================================================================

/// Graphics test type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsTest {
    /// Mode enumeration
    ModeEnum,
    /// Resolution set
    ResolutionSet,
    /// Color depth
    ColorDepth,
    /// Framebuffer access
    FramebufferAccess,
    /// Text rendering
    TextRendering,
    /// Image rendering
    ImageRendering,
    /// Animation
    Animation,
    /// EDID read
    EdidRead,
}

impl GraphicsTest {
    /// Get test name
    pub const fn name(&self) -> &'static str {
        match self {
            GraphicsTest::ModeEnum => "Mode Enumeration",
            GraphicsTest::ResolutionSet => "Resolution Set",
            GraphicsTest::ColorDepth => "Color Depth",
            GraphicsTest::FramebufferAccess => "Framebuffer Access",
            GraphicsTest::TextRendering => "Text Rendering",
            GraphicsTest::ImageRendering => "Image Rendering",
            GraphicsTest::Animation => "Animation",
            GraphicsTest::EdidRead => "EDID Read",
        }
    }
}

/// Graphics test result
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphicsTestResult {
    /// Modes available
    pub modes_count: u32,
    /// Max horizontal resolution
    pub max_h_res: u32,
    /// Max vertical resolution
    pub max_v_res: u32,
    /// Max color depth
    pub max_depth: u8,
    /// Framebuffer address
    pub framebuffer_addr: u64,
    /// Framebuffer size
    pub framebuffer_size: u64,
}

// =============================================================================
// TEST SUITE
// =============================================================================

/// Test suite configuration
#[derive(Debug, Clone, Copy)]
pub struct TestSuiteConfig {
    /// Categories to run
    pub categories: u32,
    /// Minimum severity to run
    pub min_severity: TestSeverity,
    /// Stop on first failure
    pub stop_on_fail: bool,
    /// Timeout per test (seconds)
    pub timeout_secs: u32,
    /// Enable verbose output
    pub verbose: bool,
    /// Run destructive tests
    pub destructive: bool,
}

impl Default for TestSuiteConfig {
    fn default() -> Self {
        Self {
            categories: 0xFFFFFFFF,
            min_severity: TestSeverity::Low,
            stop_on_fail: false,
            timeout_secs: 60,
            verbose: false,
            destructive: false,
        }
    }
}

/// Test suite summary
#[derive(Debug, Clone, Copy, Default)]
pub struct TestSuiteSummary {
    /// Total tests
    pub total: u32,
    /// Passed
    pub passed: u32,
    /// Failed
    pub failed: u32,
    /// Skipped
    pub skipped: u32,
    /// Warnings
    pub warnings: u32,
    /// Total duration (microseconds)
    pub total_duration_us: u64,
}

impl TestSuiteSummary {
    /// Calculate pass rate
    pub fn pass_rate(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f32 / self.total as f32) * 100.0
    }

    /// Check if all passed
    pub const fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

// =============================================================================
// ASSERTION HELPERS
// =============================================================================

/// Assertion type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertType {
    /// Values equal
    Equal,
    /// Values not equal
    NotEqual,
    /// Value less than
    LessThan,
    /// Value less than or equal
    LessEqual,
    /// Value greater than
    GreaterThan,
    /// Value greater than or equal
    GreaterEqual,
    /// Value is true
    True,
    /// Value is false
    False,
    /// Value is null/zero
    Null,
    /// Value is not null/zero
    NotNull,
    /// Value in range
    InRange,
}

/// Assertion result
#[derive(Debug, Clone, Copy)]
pub struct Assertion {
    /// Assertion type
    pub assert_type: AssertType,
    /// Passed
    pub passed: bool,
    /// Expected value
    pub expected: u64,
    /// Actual value
    pub actual: u64,
    /// Line number
    pub line: u32,
}

impl Assertion {
    /// Create passed assertion
    pub const fn pass(assert_type: AssertType, actual: u64) -> Self {
        Self {
            assert_type,
            passed: true,
            expected: 0,
            actual,
            line: 0,
        }
    }

    /// Create failed assertion
    pub const fn fail(assert_type: AssertType, expected: u64, actual: u64, line: u32) -> Self {
        Self {
            assert_type,
            passed: false,
            expected,
            actual,
            line,
        }
    }
}

// =============================================================================
// ERROR CODES
// =============================================================================

/// Test error codes
pub mod error_codes {
    // Memory errors (0x1xxx)
    pub const MEM_ALLOCATION_FAILED: u32 = 0x1001;
    pub const MEM_PATTERN_MISMATCH: u32 = 0x1002;
    pub const MEM_ADDRESS_ERROR: u32 = 0x1003;
    pub const MEM_ECC_ERROR: u32 = 0x1004;
    pub const MEM_PARITY_ERROR: u32 = 0x1005;

    // CPU errors (0x2xxx)
    pub const CPU_EXCEPTION: u32 = 0x2001;
    pub const CPU_FEATURE_MISSING: u32 = 0x2002;
    pub const CPU_TEMP_HIGH: u32 = 0x2003;
    pub const CPU_FREQ_ERROR: u32 = 0x2004;

    // Storage errors (0x3xxx)
    pub const STORAGE_NOT_FOUND: u32 = 0x3001;
    pub const STORAGE_READ_ERROR: u32 = 0x3002;
    pub const STORAGE_WRITE_ERROR: u32 = 0x3003;
    pub const STORAGE_SMART_FAIL: u32 = 0x3004;
    pub const STORAGE_BAD_SECTOR: u32 = 0x3005;

    // Network errors (0x4xxx)
    pub const NET_NO_LINK: u32 = 0x4001;
    pub const NET_DHCP_FAIL: u32 = 0x4002;
    pub const NET_DNS_FAIL: u32 = 0x4003;
    pub const NET_TIMEOUT: u32 = 0x4004;

    // Security errors (0x5xxx)
    pub const SEC_BOOT_DISABLED: u32 = 0x5001;
    pub const SEC_KEY_INVALID: u32 = 0x5002;
    pub const SEC_SIG_INVALID: u32 = 0x5003;
    pub const SEC_CHAIN_INVALID: u32 = 0x5004;
    pub const SEC_TPM_MISSING: u32 = 0x5005;

    // Graphics errors (0x6xxx)
    pub const GFX_NO_OUTPUT: u32 = 0x6001;
    pub const GFX_MODE_FAIL: u32 = 0x6002;
    pub const GFX_FB_ERROR: u32 = 0x6003;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_status() {
        assert!(TestStatus::Pass.is_pass());
        assert!(TestStatus::Warning.is_pass());
        assert!(!TestStatus::Fail.is_pass());
        assert!(TestStatus::Fail.is_fail());
    }

    #[test]
    fn test_result() {
        let result = TestResult::pass(1000);
        assert_eq!(result.status, TestStatus::Pass);
        assert_eq!(result.duration_us, 1000);
    }

    #[test]
    fn test_memory_test_pattern() {
        assert_eq!(MemoryTest::AllZeros.pattern(), 0);
        assert_eq!(MemoryTest::AllOnes.pattern(), 0xFFFFFFFFFFFFFFFF);
    }

    #[test]
    fn test_suite_summary() {
        let summary = TestSuiteSummary {
            total: 100,
            passed: 95,
            failed: 3,
            skipped: 2,
            warnings: 5,
            total_duration_us: 1000000,
        };
        assert!((summary.pass_rate() - 95.0).abs() < 0.01);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_assertion() {
        let passed = Assertion::pass(AssertType::Equal, 42);
        assert!(passed.passed);

        let failed = Assertion::fail(AssertType::Equal, 42, 0, 100);
        assert!(!failed.passed);
        assert_eq!(failed.expected, 42);
        assert_eq!(failed.actual, 0);
    }
}
