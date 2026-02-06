//! Diagnostic and Testing Framework
//!
//! Hardware diagnostics, memory testing, and boot verification.

use core::fmt;

// =============================================================================
// DIAGNOSTIC RESULTS
// =============================================================================

/// Test result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TestResult {
    /// Test passed
    Pass,
    /// Test failed
    Fail,
    /// Test skipped
    #[default]
    Skip,
    /// Test not supported
    NotSupported,
    /// Test timed out
    Timeout,
    /// Test error
    Error,
}

impl TestResult {
    /// Returns `true` if the test result indicates success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Pass | Self::Skip | Self::NotSupported)
    }

    /// Returns `true` if the test result indicates failure.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Fail | Self::Error | Self::Timeout)
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Skip => write!(f, "SKIP"),
            Self::NotSupported => write!(f, "N/A"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// Diagnostic report
#[derive(Clone)]
pub struct DiagnosticReport {
    /// Test name
    pub name: &'static str,
    /// Result
    pub result: TestResult,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Details
    pub details: Option<&'static str>,
    /// Error code
    pub error_code: Option<u32>,
}

impl DiagnosticReport {
    /// Creates a diagnostic report indicating a passed test.
    #[must_use]
    pub fn pass(name: &'static str, duration_us: u64) -> Self {
        Self {
            name,
            result: TestResult::Pass,
            duration_us,
            details: None,
            error_code: None,
        }
    }

    /// Creates a diagnostic report indicating a failed test.
    #[must_use]
    pub fn fail(name: &'static str, details: &'static str) -> Self {
        Self {
            name,
            result: TestResult::Fail,
            duration_us: 0,
            details: Some(details),
            error_code: None,
        }
    }

    /// Creates a diagnostic report indicating a skipped test.
    #[must_use]
    pub fn skip(name: &'static str, reason: &'static str) -> Self {
        Self {
            name,
            result: TestResult::Skip,
            duration_us: 0,
            details: Some(reason),
            error_code: None,
        }
    }
}

// =============================================================================
// CPU DIAGNOSTICS
// =============================================================================

/// CPU diagnostic utilities for testing CPU features and capabilities.
pub struct CpuDiagnostics;

impl CpuDiagnostics {
    /// Runs all CPU diagnostic tests and returns the results.
    #[must_use]
    pub fn run_all() -> CpuTestResults {
        CpuTestResults {
            vendor: Self::get_vendor(),
            family: Self::get_family(),
            model: Self::get_model(),
            stepping: Self::get_stepping(),
            features: Self::detect_features(),
            cache_info: Self::get_cache_info(),
        }
    }

    /// Gets the CPU vendor identification.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn get_vendor() -> CpuVendor {
        let result = crate::arch::x86_64::cpuid(0, 0);
        let part1 = result.ebx;
        let part3 = result.ecx;
        let part2 = result.edx;

        // Combine vendor string (ebx + edx + ecx order)
        let vendor_bytes: [u8; 12] = [
            part1 as u8,
            (part1 >> 8) as u8,
            (part1 >> 16) as u8,
            (part1 >> 24) as u8,
            part2 as u8,
            (part2 >> 8) as u8,
            (part2 >> 16) as u8,
            (part2 >> 24) as u8,
            part3 as u8,
            (part3 >> 8) as u8,
            (part3 >> 16) as u8,
            (part3 >> 24) as u8,
        ];
        if &vendor_bytes == b"GenuineIntel" {
            CpuVendor::Intel
        } else if &vendor_bytes == b"AuthenticAMD" {
            CpuVendor::Amd
        } else {
            CpuVendor::Unknown
        }
    }

    /// Gets the CPU vendor identification.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn get_vendor() -> CpuVendor {
        CpuVendor::Unknown
    }

    /// Gets the CPU family number.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn get_family() -> u8 {
        let result = crate::arch::x86_64::cpuid(1, 0);
        let eax = result.eax;

        let base_family = ((eax >> 8) & 0xF) as u8;
        let ext_family = ((eax >> 20) & 0xFF) as u8;

        if base_family == 0xF {
            base_family + ext_family
        } else {
            base_family
        }
    }

    /// Gets the CPU family number.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn get_family() -> u8 {
        0
    }

    /// Gets the CPU model number.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn get_model() -> u8 {
        let result = crate::arch::x86_64::cpuid(1, 0);
        let eax = result.eax;

        let base_model = ((eax >> 4) & 0xF) as u8;
        let ext_model = ((eax >> 16) & 0xF) as u8;

        (ext_model << 4) | base_model
    }

    /// Gets the CPU model number.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn get_model() -> u8 {
        0
    }

    /// Gets the CPU stepping revision.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn get_stepping() -> u8 {
        let result = crate::arch::x86_64::cpuid(1, 0);
        (result.eax & 0xF) as u8
    }

    /// Gets the CPU stepping revision.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn get_stepping() -> u8 {
        0
    }

    /// Detects available CPU features.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn detect_features() -> CpuFeatures {
        let result1 = crate::arch::x86_64::cpuid(1, 0);
        let basic_ecx = result1.ecx;
        let standard_edx = result1.edx;

        let result7 = crate::arch::x86_64::cpuid(7, 0);
        let extended_ebx = result7.ebx;
        let structured_ecx = result7.ecx;

        let result_ext = crate::arch::x86_64::cpuid(0x8000_0001, 0);
        let amd_ext_caps = result_ext.edx;

        CpuFeatures {
            // EDX features (CPUID 1)
            fpu: standard_edx & (1 << 0) != 0,
            pae: standard_edx & (1 << 6) != 0,
            msr: standard_edx & (1 << 5) != 0,
            apic: standard_edx & (1 << 9) != 0,
            mmx: standard_edx & (1 << 23) != 0,
            sse: standard_edx & (1 << 25) != 0,
            sse2: standard_edx & (1 << 26) != 0,

            // ECX features (CPUID 1)
            sse3: basic_ecx & (1 << 0) != 0,
            ssse3: basic_ecx & (1 << 9) != 0,
            sse4_1: basic_ecx & (1 << 19) != 0,
            sse4_2: basic_ecx & (1 << 20) != 0,
            popcnt: basic_ecx & (1 << 23) != 0,
            aes: basic_ecx & (1 << 25) != 0,
            avx: basic_ecx & (1 << 28) != 0,
            x2apic: basic_ecx & (1 << 21) != 0,
            xsave: basic_ecx & (1 << 26) != 0,

            // EBX features (CPUID 7)
            avx2: extended_ebx & (1 << 5) != 0,
            smep: extended_ebx & (1 << 7) != 0,
            smap: extended_ebx & (1 << 20) != 0,
            sha: extended_ebx & (1 << 29) != 0,

            // ECX features (CPUID 7)
            umip: structured_ecx & (1 << 2) != 0,

            // Extended features (CPUID 0x80000001)
            nx: amd_ext_caps & (1 << 20) != 0,
            long_mode: amd_ext_caps & (1 << 29) != 0,
            gbyte_pages: amd_ext_caps & (1 << 26) != 0,
        }
    }

    /// Detects available CPU features.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn detect_features() -> CpuFeatures {
        CpuFeatures::default()
    }

    /// Gets CPU cache information.
    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn get_cache_info() -> CacheInfo {
        // Try to get cache info from CPUID 4
        let mut data_cache_size = 0u32;
        let mut inst_cache_size = 0u32;
        let mut l2_size = 0u32;
        let mut l3_size = 0u32;

        for i in 0..16 {
            let result = crate::arch::x86_64::cpuid(4, i);
            let reg_a = result.eax;
            let reg_b = result.ebx;
            let reg_c = result.ecx;

            let cache_type = reg_a & 0x1F;
            if cache_type == 0 {
                break;
            }

            let level = (reg_a >> 5) & 0x7;
            let ways = ((reg_b >> 22) & 0x3FF) + 1;
            let partitions = ((reg_b >> 12) & 0x3FF) + 1;
            let line_size = (reg_b & 0xFFF) + 1;
            let sets = reg_c + 1;

            let size = ways * partitions * line_size * sets;

            match (level, cache_type) {
                (1, 1) => data_cache_size = size, // L1 Data
                (1, 2) => inst_cache_size = size, // L1 Instruction
                (2, 3) => l2_size = size,         // L2 Unified
                (3, 3) => l3_size = size,         // L3 Unified
                _ => {},
            }
        }

        CacheInfo {
            l1_data_kb: data_cache_size / 1024,
            l1_inst_kb: inst_cache_size / 1024,
            l2_kb: l2_size / 1024,
            l3_kb: l3_size / 1024,
        }
    }

    /// Gets CPU cache information.
    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn get_cache_info() -> CacheInfo {
        CacheInfo::default()
    }

    /// Tests the CPUID instruction availability.
    #[must_use]
    pub fn test_cpuid() -> DiagnosticReport {
        #[cfg(target_arch = "x86_64")]
        {
            let result = crate::arch::x86_64::cpuid(0, 0);

            if result.eax > 0 {
                DiagnosticReport::pass("CPUID", 0)
            } else {
                DiagnosticReport::fail("CPUID", "CPUID returned invalid max leaf")
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        DiagnosticReport::skip("CPUID", "Not x86_64")
    }

    /// Tests long mode (64-bit) support.
    #[must_use]
    pub fn test_long_mode() -> DiagnosticReport {
        let features = Self::detect_features();

        if features.long_mode {
            DiagnosticReport::pass("Long Mode", 0)
        } else {
            DiagnosticReport::fail("Long Mode", "64-bit mode not supported")
        }
    }

    /// Tests NX (No-Execute) bit support.
    #[must_use]
    pub fn test_nx_bit() -> DiagnosticReport {
        let features = Self::detect_features();

        if features.nx {
            DiagnosticReport::pass("NX Bit", 0)
        } else {
            DiagnosticReport::fail("NX Bit", "Execute disable not supported")
        }
    }
}

/// CPU vendor identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    /// Intel Corporation.
    Intel,
    /// Advanced Micro Devices.
    Amd,
    /// Unknown or unsupported vendor.
    Unknown,
}

impl fmt::Display for CpuVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intel => write!(f, "Intel"),
            Self::Amd => write!(f, "AMD"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Results from CPU diagnostic tests.
pub struct CpuTestResults {
    /// CPU vendor identification.
    pub vendor: CpuVendor,
    /// CPU family number.
    pub family: u8,
    /// CPU model number.
    pub model: u8,
    /// CPU stepping revision.
    pub stepping: u8,
    /// Detected CPU features.
    pub features: CpuFeatures,
    /// CPU cache information.
    pub cache_info: CacheInfo,
}

/// Detected CPU feature flags.
#[derive(Default)]
pub struct CpuFeatures {
    /// Floating Point Unit on-chip.
    pub fpu: bool,
    /// Physical Address Extension.
    pub pae: bool,
    /// Model Specific Registers.
    pub msr: bool,
    /// Advanced Programmable Interrupt Controller.
    pub apic: bool,
    /// MMX instruction set.
    pub mmx: bool,
    /// Streaming SIMD Extensions.
    pub sse: bool,
    /// Streaming SIMD Extensions 2.
    pub sse2: bool,

    /// Streaming SIMD Extensions 3.
    pub sse3: bool,
    /// Supplemental Streaming SIMD Extensions 3.
    pub ssse3: bool,
    /// Streaming SIMD Extensions 4.1.
    pub sse4_1: bool,
    /// Streaming SIMD Extensions 4.2.
    pub sse4_2: bool,
    /// Population count instruction.
    pub popcnt: bool,
    /// AES instruction set.
    pub aes: bool,
    /// Advanced Vector Extensions.
    pub avx: bool,
    /// Advanced Vector Extensions 2.
    pub avx2: bool,
    /// x2APIC support.
    pub x2apic: bool,
    /// XSAVE/XRSTOR feature.
    pub xsave: bool,

    /// Supervisor Mode Execution Prevention.
    pub smep: bool,
    /// Supervisor Mode Access Prevention.
    pub smap: bool,
    /// SHA instruction extensions.
    pub sha: bool,
    /// User-Mode Instruction Prevention.
    pub umip: bool,

    /// No-Execute bit support.
    pub nx: bool,
    /// Long mode (64-bit) support.
    pub long_mode: bool,
    /// 1 GiB page support.
    pub gbyte_pages: bool,
}

/// CPU cache size information.
#[derive(Default)]
pub struct CacheInfo {
    /// L1 data cache size in kilobytes.
    pub l1_data_kb: u32,
    /// L1 instruction cache size in kilobytes.
    pub l1_inst_kb: u32,
    /// L2 cache size in kilobytes.
    pub l2_kb: u32,
    /// L3 cache size in kilobytes.
    pub l3_kb: u32,
}

// =============================================================================
// MEMORY DIAGNOSTICS
// =============================================================================

/// Memory test patterns for memory diagnostics.
pub mod test_pattern {
    /// All bits set to zero.
    pub const ALL_ZEROS: u64 = 0x0000_0000_0000_0000;
    /// All bits set to one.
    pub const ALL_ONES: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    /// Alternating bits pattern (0xAA...).
    pub const ALTERNATING_A: u64 = 0xAAAA_AAAA_AAAA_AAAA;
    /// Alternating bits pattern (0x55...).
    pub const ALTERNATING_5: u64 = 0x5555_5555_5555_5555;
    /// Walking ones pattern array.
    pub const WALKING_ONE: [u64; 64] = {
        let mut arr = [0u64; 64];
        let mut i = 0;
        while i < 64 {
            arr[i] = 1u64 << i;
            i += 1;
        }
        arr
    };
}

/// Memory testing utilities for verifying memory integrity.
pub struct MemoryTest;

impl MemoryTest {
    /// Tests a memory range by writing and verifying a pattern.
    #[must_use]
    pub fn test_pattern(start: *mut u64, count: usize, pattern: u64) -> TestResult {
        // Write pattern
        for i in 0..count {
            unsafe {
                core::ptr::write_volatile(start.add(i), pattern);
            }
        }

        // Memory barrier
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Verify pattern
        for i in 0..count {
            let value = unsafe { core::ptr::read_volatile(start.add(i)) };
            if value != pattern {
                return TestResult::Fail;
            }
        }

        TestResult::Pass
    }

    /// Performs a walking ones test on the memory range.
    #[must_use]
    pub fn test_walking_ones(start: *mut u64, count: usize) -> TestResult {
        for &pattern in &test_pattern::WALKING_ONE {
            if Self::test_pattern(start, count.min(1024), pattern) != TestResult::Pass {
                return TestResult::Fail;
            }
        }
        TestResult::Pass
    }

    /// Performs an address test to check for addressing issues.
    #[must_use]
    pub fn test_address(start: *mut u64, count: usize) -> TestResult {
        // Write addresses
        for i in 0..count {
            let addr = start as u64 + (i as u64 * 8);
            unsafe {
                core::ptr::write_volatile(start.add(i), addr);
            }
        }

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Verify addresses
        for i in 0..count {
            let expected = start as u64 + (i as u64 * 8);
            let value = unsafe { core::ptr::read_volatile(start.add(i)) };
            if value != expected {
                return TestResult::Fail;
            }
        }

        TestResult::Pass
    }

    /// Performs a random pattern test using a linear congruential generator.
    #[must_use]
    pub fn test_random(start: *mut u64, count: usize, seed: u64) -> TestResult {
        let mut rng = SimpleRng::new(seed);

        // Write random values
        for i in 0..count {
            unsafe {
                core::ptr::write_volatile(start.add(i), rng.next());
            }
        }

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Verify with same sequence
        let mut rng = SimpleRng::new(seed);
        for i in 0..count {
            let expected = rng.next();
            let value = unsafe { core::ptr::read_volatile(start.add(i)) };
            if value != expected {
                return TestResult::Fail;
            }
        }

        TestResult::Pass
    }

    /// Performs a quick memory test covering the main patterns.
    #[must_use]
    pub fn quick_test(start: *mut u64, size_bytes: usize) -> MemoryTestResult {
        let count = size_bytes / 8;
        let mut result = MemoryTestResult::new();

        // All zeros
        result.zeros = Self::test_pattern(start, count, test_pattern::ALL_ZEROS);

        // All ones
        result.ones = Self::test_pattern(start, count, test_pattern::ALL_ONES);

        // Alternating
        result.alternating = Self::test_pattern(start, count, test_pattern::ALTERNATING_A);

        // Address test
        result.address = Self::test_address(start, count);

        result
    }

    /// Performs a thorough memory test including all patterns.
    #[must_use]
    pub fn full_test(start: *mut u64, size_bytes: usize) -> MemoryTestResult {
        let count = size_bytes / 8;
        let mut result = Self::quick_test(start, size_bytes);

        // Walking ones
        result.walking = Self::test_walking_ones(start, count);

        // Random
        result.random = Self::test_random(start, count, 0x1234_5678_DEAD_BEEF);

        result
    }
}

/// Simple random number generator using Linear Congruential Generator.
struct SimpleRng {
    /// Current state of the generator.
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // LCG parameters from Numerical Recipes
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }
}

/// Results from memory diagnostic tests.
#[derive(Default)]
pub struct MemoryTestResult {
    /// Result of the all-zeros pattern test.
    pub zeros: TestResult,
    /// Result of the all-ones pattern test.
    pub ones: TestResult,
    /// Result of the alternating bits pattern test.
    pub alternating: TestResult,
    /// Result of the address verification test.
    pub address: TestResult,
    /// Result of the walking ones test.
    pub walking: TestResult,
    /// Result of the random pattern test.
    pub random: TestResult,
}

impl MemoryTestResult {
    fn new() -> Self {
        Self {
            zeros: TestResult::Skip,
            ones: TestResult::Skip,
            alternating: TestResult::Skip,
            address: TestResult::Skip,
            walking: TestResult::Skip,
            random: TestResult::Skip,
        }
    }

    /// Returns `true` if all tests passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.zeros.is_success()
            && self.ones.is_success()
            && self.alternating.is_success()
            && self.address.is_success()
            && self.walking.is_success()
            && self.random.is_success()
    }

    /// Counts the number of failed tests.
    #[must_use]
    pub fn failure_count(&self) -> usize {
        let mut count = 0;
        if self.zeros.is_failure() {
            count += 1;
        }
        if self.ones.is_failure() {
            count += 1;
        }
        if self.alternating.is_failure() {
            count += 1;
        }
        if self.address.is_failure() {
            count += 1;
        }
        if self.walking.is_failure() {
            count += 1;
        }
        if self.random.is_failure() {
            count += 1;
        }
        count
    }
}

// =============================================================================
// BOOT DIAGNOSTICS
// =============================================================================

/// Boot stage enumeration for tracking boot progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BootStage {
    /// Entry point reached.
    Entry            = 0,
    /// UEFI services initialized.
    UefiInit         = 1,
    /// Console initialized.
    Console          = 2,
    /// Memory map obtained.
    MemoryMap        = 3,
    /// Graphics initialized.
    Graphics         = 4,
    /// Configuration loaded.
    Config           = 5,
    /// Kernel found.
    KernelFound      = 6,
    /// Kernel loaded.
    KernelLoaded     = 7,
    /// Kernel verified.
    KernelVerified   = 8,
    /// Exit boot services.
    ExitBootServices = 9,
    /// Jumping to kernel.
    JumpToKernel     = 10,
}

impl BootStage {
    /// Returns the human-readable name of this boot stage.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Entry => "Entry",
            Self::UefiInit => "UEFI Init",
            Self::Console => "Console",
            Self::MemoryMap => "Memory Map",
            Self::Graphics => "Graphics",
            Self::Config => "Config",
            Self::KernelFound => "Kernel Found",
            Self::KernelLoaded => "Kernel Loaded",
            Self::KernelVerified => "Kernel Verified",
            Self::ExitBootServices => "Exit Boot Services",
            Self::JumpToKernel => "Jump to Kernel",
        }
    }

    /// Returns the progress percentage for this boot stage.
    #[must_use]
    pub fn progress(&self) -> u8 {
        match self {
            Self::Entry => 0,
            Self::UefiInit => 10,
            Self::Console => 20,
            Self::MemoryMap => 30,
            Self::Graphics => 40,
            Self::Config => 50,
            Self::KernelFound => 60,
            Self::KernelLoaded => 70,
            Self::KernelVerified => 80,
            Self::ExitBootServices => 90,
            Self::JumpToKernel => 100,
        }
    }
}

/// Tracks boot progress through various stages.
pub struct BootProgress {
    /// Current boot stage.
    current_stage: BootStage,
    /// Timestamps for each stage (if available).
    stage_times: [u64; 11],
    /// Errors encountered during boot.
    errors: [Option<BootError>; 16],
    /// Number of errors recorded.
    error_count: usize,
}

impl Default for BootProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl BootProgress {
    /// Creates a new boot progress tracker.
    #[must_use]
    pub const fn new() -> Self {
        const NONE_ERROR: Option<BootError> = None;

        Self {
            current_stage: BootStage::Entry,
            stage_times: [0; 11],
            errors: [NONE_ERROR; 16],
            error_count: 0,
        }
    }

    /// Advances to the specified boot stage.
    pub fn advance(&mut self, stage: BootStage, timestamp: u64) {
        self.current_stage = stage;
        self.stage_times[stage as usize] = timestamp;
    }

    /// Records an error that occurred during boot.
    pub fn record_error(&mut self, error: BootError) {
        if self.error_count < 16 {
            self.errors[self.error_count] = Some(error);
            self.error_count += 1;
        }
    }

    /// Returns the current boot stage.
    #[must_use]
    pub fn current_stage(&self) -> BootStage {
        self.current_stage
    }

    /// Returns the current progress percentage.
    #[must_use]
    pub fn progress(&self) -> u8 {
        self.current_stage.progress()
    }

    /// Returns `true` if any errors have been recorded.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Returns an iterator over the recorded errors.
    pub fn errors(&self) -> impl Iterator<Item = &BootError> {
        self.errors.iter().filter_map(|e| e.as_ref())
    }
}

/// Represents an error that occurred during boot.
#[derive(Debug, Clone)]
pub struct BootError {
    /// The boot stage where the error occurred.
    pub stage: BootStage,
    /// Numeric error code.
    pub code: u32,
    /// Human-readable error message.
    pub message: &'static str,
    /// Whether this error is fatal.
    pub fatal: bool,
}

impl BootError {
    /// Creates a new boot error.
    #[must_use]
    pub fn new(stage: BootStage, code: u32, message: &'static str, fatal: bool) -> Self {
        Self {
            stage,
            code,
            message,
            fatal,
        }
    }
}

// =============================================================================
// CHECKSUM UTILITIES
// =============================================================================

/// Calculates the CRC32 checksum of the given data.
#[must_use]
pub fn crc32(data: &[u8]) -> u32 {
    const CRC32_TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut j = 0;
            while j < 8 {
                if c & 1 != 0 {
                    c = 0xEDB8_8320 ^ (c >> 1);
                } else {
                    c >>= 1;
                }
                j += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };

    let mut crc = 0xFFFF_FFFF_u32;

    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = CRC32_TABLE[index] ^ (crc >> 8);
    }

    !crc
}

/// Calculates the Adler-32 checksum of the given data.
#[must_use]
pub fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;

    let mut a: u32 = 1;
    let mut b: u32 = 0;

    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

/// Calculates a simple XOR checksum of all bytes.
#[must_use]
pub fn simple_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

/// Calculates a sum-based checksum (two's complement).
#[must_use]
pub fn sum_checksum(data: &[u8]) -> u8 {
    let sum: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    (!sum).wrapping_add(1)
}

// =============================================================================
// TIMING UTILITIES
// =============================================================================

/// Reads the Time Stamp Counter.
#[cfg(target_arch = "x86_64")]
#[must_use]
pub fn read_tsc() -> u64 {
    let (low, high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Reads the Time Stamp Counter.
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
pub fn read_tsc() -> u64 {
    0
}

/// Estimates the TSC frequency (approximate).
#[cfg(target_arch = "x86_64")]
#[must_use]
pub fn estimate_tsc_frequency() -> u64 {
    // Try to get from CPUID if available
    let result = crate::arch::x86_64::cpuid(0x15, 0);
    let tsc_ratio_denom = result.eax;
    let tsc_ratio_numer = result.ebx;
    let crystal_freq = result.ecx;

    if tsc_ratio_denom != 0 && tsc_ratio_numer != 0 {
        // TSC frequency = (core crystal clock * ebx) / eax
        if crystal_freq != 0 {
            return (crystal_freq as u64 * tsc_ratio_numer as u64) / tsc_ratio_denom as u64;
        }
    }

    // Fallback: assume 2.5 GHz (common)
    2_500_000_000
}

/// Estimates the TSC frequency (approximate).
#[cfg(not(target_arch = "x86_64"))]
#[must_use]
pub fn estimate_tsc_frequency() -> u64 {
    1_000_000_000 // 1 GHz default
}

/// Performs a simple delay using the TSC.
pub fn tsc_delay(cycles: u64) {
    let start = read_tsc();
    while read_tsc() - start < cycles {
        core::hint::spin_loop();
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        let data = b"Hello, World!";
        let crc = crc32(data);
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_adler32() {
        let data = b"Wikipedia";
        let checksum = adler32(data);
        assert_eq!(checksum, 0x11E60398);
    }

    #[test]
    fn test_boot_stage_progress() {
        assert_eq!(BootStage::Entry.progress(), 0);
        assert_eq!(BootStage::JumpToKernel.progress(), 100);
    }

    #[test]
    fn test_test_result() {
        assert!(TestResult::Pass.is_success());
        assert!(TestResult::Fail.is_failure());
        assert!(TestResult::Skip.is_success());
    }
}
