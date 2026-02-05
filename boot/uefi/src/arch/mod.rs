//! Architecture-Specific Code
//!
//! Platform-specific implementations for x86_64 and aarch64.

#[cfg(target_arch = "aarch64")]
pub mod aarch64;
#[cfg(target_arch = "x86_64")]
pub mod x86_64;

use crate::error::Result;
use crate::raw::types::VirtualAddress;

// =============================================================================
// ARCHITECTURE DETECTION
// =============================================================================

/// Target architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Architecture {
    /// x86-64 (AMD64)
    X86_64  = 0,
    /// `AArch64` (ARM64)
    Aarch64 = 1,
    /// RISC-V 64-bit
    RiscV64 = 2,
    /// Unknown architecture
    Unknown = 255,
}

impl Architecture {
    /// Get current architecture at compile time
    #[cfg(target_arch = "x86_64")]
    pub const fn current() -> Self {
        Self::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    pub const fn current() -> Self {
        Self::Aarch64
    }

    #[cfg(target_arch = "riscv64")]
    pub const fn current() -> Self {
        Self::RiscV64
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    pub const fn current() -> Self {
        Self::Unknown
    }

    /// Get architecture name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::RiscV64 => "riscv64",
            Self::Unknown => "unknown",
        }
    }

    /// Get page size
    pub const fn page_size(&self) -> u64 {
        match self {
            Self::X86_64 | Self::Aarch64 | Self::RiscV64 | Self::Unknown => 4096, // Can be 4K, 16K, or 64K
        }
    }

    /// Get virtual address bits
    pub const fn virtual_address_bits(&self) -> u8 {
        match self {
            Self::X86_64 | Self::Aarch64 | Self::RiscV64 | Self::Unknown => 48, // 57 with 5-level paging
        }
    }

    /// Get physical address bits
    pub const fn physical_address_bits(&self) -> u8 {
        match self {
            Self::X86_64 => 52, // MAXPHYADDR varies
            Self::RiscV64 => 56,
            Self::Aarch64 | Self::Unknown => 48,
        }
    }
}

// =============================================================================
// ARCHITECTURE TRAIT
// =============================================================================

/// Architecture-specific operations
pub trait ArchOperations {
    /// Initialize architecture
    fn init() -> Result<()>;

    /// Get current CPU ID
    fn current_cpu_id() -> u32;

    /// Halt CPU
    fn halt() -> !;

    /// Enable interrupts
    fn enable_interrupts();

    /// Disable interrupts
    fn disable_interrupts();

    /// Check if interrupts are enabled
    fn interrupts_enabled() -> bool;

    /// Read timestamp counter
    fn read_timestamp() -> u64;

    /// Invalidate TLB entry
    fn invalidate_tlb_entry(addr: VirtualAddress);

    /// Invalidate entire TLB
    fn invalidate_tlb_all();

    /// Memory barrier (full)
    fn memory_barrier();

    /// Read barrier
    fn read_barrier();

    /// Write barrier
    fn write_barrier();
}

// =============================================================================
// COMMON CPU FEATURES
// =============================================================================

/// Common CPU feature flags
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFeatures(u64);

impl CpuFeatures {
    /// SSE support
    pub const SSE: Self = Self(1 << 0);
    /// SSE2 support
    pub const SSE2: Self = Self(1 << 1);
    /// SSE3 support
    pub const SSE3: Self = Self(1 << 2);
    /// SSE4.1 support
    pub const SSE4_1: Self = Self(1 << 3);
    /// SSE4.2 support
    pub const SSE4_2: Self = Self(1 << 4);
    /// AVX support
    pub const AVX: Self = Self(1 << 5);
    /// AVX2 support
    pub const AVX2: Self = Self(1 << 6);
    /// AVX-512 support
    pub const AVX512: Self = Self(1 << 7);
    /// AES-NI support
    pub const AES: Self = Self(1 << 8);
    /// RDRAND support
    pub const RDRAND: Self = Self(1 << 9);
    /// RDSEED support
    pub const RDSEED: Self = Self(1 << 10);
    /// SHA extensions
    pub const SHA: Self = Self(1 << 11);
    /// TSC support
    pub const TSC: Self = Self(1 << 12);
    /// Invariant TSC
    pub const TSC_INVARIANT: Self = Self(1 << 13);
    /// NX/XD bit support
    pub const NX: Self = Self(1 << 14);
    /// 1GB pages
    pub const PAGE_1GB: Self = Self(1 << 15);
    /// 5-level paging
    pub const LA57: Self = Self(1 << 16);
    /// SMEP (Supervisor Mode Execution Prevention)
    pub const SMEP: Self = Self(1 << 17);
    /// SMAP (Supervisor Mode Access Prevention)
    pub const SMAP: Self = Self(1 << 18);
    /// UMIP (User Mode Instruction Prevention)
    pub const UMIP: Self = Self(1 << 19);
    /// PKU (Protection Keys for User pages)
    pub const PKU: Self = Self(1 << 20);
    /// CET (Control-flow Enforcement Technology)
    pub const CET: Self = Self(1 << 21);
    /// XSAVE support
    pub const XSAVE: Self = Self(1 << 22);
    /// FSGSBASE support
    pub const FSGSBASE: Self = Self(1 << 23);
    /// x2APIC support
    pub const X2APIC: Self = Self(1 << 24);
    /// PCID support
    pub const PCID: Self = Self(1 << 25);
    /// INVPCID support
    pub const INVPCID: Self = Self(1 << 26);

    /// Check if a feature is present
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Add a feature flag
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl CpuFeatures {
    /// Detect features for current architecture
    #[cfg(target_arch = "x86_64")]
    pub fn detect() -> Self {
        x86_64::cpu::detect_features()
    }

    #[cfg(target_arch = "aarch64")]
    pub fn detect() -> Self {
        aarch64::cpu::detect_features()
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    pub fn detect() -> Self {
        Self::default()
    }
}

// =============================================================================
// MEMORY MODEL
// =============================================================================

/// Memory model configuration
#[derive(Debug, Clone, Copy)]
pub struct MemoryModel {
    /// Page size (typically 4096)
    pub page_size: u64,
    /// Large page size (2MB on `x86_64`)
    pub large_page_size: u64,
    /// Huge page size (1GB on `x86_64`)
    pub huge_page_size: Option<u64>,
    /// Physical address width
    pub physical_address_bits: u8,
    /// Virtual address width
    pub virtual_address_bits: u8,
    /// Canonical address check required
    pub canonical_addresses: bool,
    /// Higher half kernel address
    pub higher_half_start: VirtualAddress,
    /// Kernel text segment base
    pub kernel_base: VirtualAddress,
    /// Physical memory map base
    pub physical_map_base: VirtualAddress,
    /// Page table recursion address
    pub recursive_mapping: Option<VirtualAddress>,
}

impl MemoryModel {
    /// `x86_64` memory model
    pub const X86_64: Self = Self {
        page_size: 4096,
        large_page_size: 2 * 1024 * 1024,
        huge_page_size: Some(1024 * 1024 * 1024),
        physical_address_bits: 52,
        virtual_address_bits: 48,
        canonical_addresses: true,
        higher_half_start: VirtualAddress(0xFFFF_8000_0000_0000),
        kernel_base: VirtualAddress(0xFFFF_FFFF_8000_0000),
        physical_map_base: VirtualAddress(0xFFFF_8800_0000_0000),
        recursive_mapping: Some(VirtualAddress(0xFFFF_FF00_0000_0000)),
    };

    /// `AArch64` memory model (4K pages)
    pub const AARCH64_4K: Self = Self {
        page_size: 4096,
        large_page_size: 2 * 1024 * 1024,
        huge_page_size: Some(1024 * 1024 * 1024),
        physical_address_bits: 48,
        virtual_address_bits: 48,
        canonical_addresses: true,
        higher_half_start: VirtualAddress(0xFFFF_0000_0000_0000),
        kernel_base: VirtualAddress(0xFFFF_FFFF_8000_0000),
        physical_map_base: VirtualAddress(0xFFFF_8000_0000_0000),
        recursive_mapping: None,
    };

    /// Get for current architecture
    pub const fn current() -> Self {
        match Architecture::current() {
            Architecture::Aarch64 => Self::AARCH64_4K,
            Architecture::X86_64 | Architecture::Unknown | Architecture::RiscV64 => Self::X86_64,
        }
    }

    /// Check if address is canonical
    pub fn is_canonical(&self, addr: VirtualAddress) -> bool {
        if !self.canonical_addresses {
            return true;
        }

        let bits = self.virtual_address_bits;
        let sign_bit = 1u64 << (bits - 1);
        let mask = !((1u64 << bits) - 1);

        if (addr & sign_bit) != 0 {
            // Negative address: all high bits must be 1
            (addr & mask) == mask
        } else {
            // Positive address: all high bits must be 0
            (addr & mask) == 0
        }
    }

    /// Canonicalize address
    pub const fn canonicalize(&self, addr: VirtualAddress) -> VirtualAddress {
        let bits = self.virtual_address_bits;
        let sign_bit = 1u64 << (bits - 1);

        if (addr.0 & sign_bit) != 0 {
            VirtualAddress(addr.0 | !((1u64 << bits) - 1))
        } else {
            VirtualAddress(addr.0 & ((1u64 << bits) - 1))
        }
    }

    /// Get page count for size
    pub const fn page_count(&self, size: u64) -> u64 {
        size.div_ceil(self.page_size)
    }

    /// Align up to page boundary
    pub const fn page_align_up(&self, addr: u64) -> u64 {
        (addr + self.page_size - 1) & !(self.page_size - 1)
    }

    /// Align down to page boundary
    pub const fn page_align_down(&self, addr: u64) -> u64 {
        addr & !(self.page_size - 1)
    }
}

// =============================================================================
// REGISTER CONTEXT
// =============================================================================

/// CPU register context (for context switching)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct RegisterContext {
    /// General purpose registers (architecture-specific)
    pub gpr: [u64; 32],
    /// Instruction pointer / Program counter
    pub ip: u64,
    /// Stack pointer
    pub sp: u64,
    /// Flags / CPSR
    pub flags: u64,
    /// Segment selectors (`x86_64`) or EL (aarch64)
    pub extra: [u64; 8],
}

impl RegisterContext {
    /// Create new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create for entry point
    pub fn for_entry(entry: VirtualAddress, stack: VirtualAddress) -> Self {
        let mut ctx = Self::new();
        ctx.ip = entry.0;
        ctx.sp = stack.0;
        ctx
    }
}

// =============================================================================
// PLATFORM INIT
// =============================================================================

/// Platform initialization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitState {
    /// Not initialized
    None,
    /// Early init (minimal)
    Early,
    /// CPU initialized
    Cpu,
    /// Memory initialized
    Memory,
    /// Interrupts initialized
    Interrupts,
    /// Fully initialized
    Complete,
}

/// Platform initializer
pub struct PlatformInit {
    state: InitState,
    features: CpuFeatures,
    memory_model: MemoryModel,
}

impl PlatformInit {
    /// Create new platform initializer
    pub fn new() -> Self {
        Self {
            state: InitState::None,
            features: CpuFeatures::default(),
            memory_model: MemoryModel::current(),
        }
    }

    /// Get current state
    pub const fn state(&self) -> InitState {
        self.state
    }

    /// Get CPU features
    pub const fn features(&self) -> &CpuFeatures {
        &self.features
    }

    /// Get memory model
    pub const fn memory_model(&self) -> &MemoryModel {
        &self.memory_model
    }

    /// Perform early initialization
    pub fn early_init(&mut self) -> Result<()> {
        #[cfg(target_arch = "x86_64")]
        x86_64::early_init()?;

        #[cfg(target_arch = "aarch64")]
        aarch64::early_init()?;

        self.state = InitState::Early;
        Ok(())
    }

    /// Initialize CPU
    pub fn init_cpu(&mut self) -> Result<()> {
        self.features = CpuFeatures::detect();

        #[cfg(target_arch = "x86_64")]
        x86_64::init_cpu(&self.features)?;

        #[cfg(target_arch = "aarch64")]
        aarch64::init_cpu(&self.features)?;

        self.state = InitState::Cpu;
        Ok(())
    }

    /// Initialize memory subsystem
    pub fn init_memory(&mut self) -> Result<()> {
        #[cfg(target_arch = "x86_64")]
        x86_64::init_memory(&self.memory_model)?;

        #[cfg(target_arch = "aarch64")]
        aarch64::init_memory(&self.memory_model)?;

        self.state = InitState::Memory;
        Ok(())
    }

    /// Initialize interrupts
    pub fn init_interrupts(&mut self) -> Result<()> {
        #[cfg(target_arch = "x86_64")]
        x86_64::init_interrupts()?;

        #[cfg(target_arch = "aarch64")]
        aarch64::init_interrupts()?;

        self.state = InitState::Interrupts;
        Ok(())
    }

    /// Complete initialization
    pub fn complete(&mut self) -> Result<()> {
        self.state = InitState::Complete;
        Ok(())
    }
}

impl Default for PlatformInit {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_architecture() {
        let arch = Architecture::current();
        assert!(!arch.name().is_empty());
        assert!(arch.page_size() >= 4096);
    }

    #[test]
    fn test_memory_model() {
        let model = MemoryModel::current();
        assert_eq!(model.page_size, 4096);

        // Test canonical addresses
        assert!(model.is_canonical(0x0000_7FFF_FFFF_FFFF));
        assert!(model.is_canonical(0xFFFF_8000_0000_0000));
    }

    #[test]
    fn test_page_alignment() {
        let model = MemoryModel::X86_64;

        assert_eq!(model.page_align_up(0), 0);
        assert_eq!(model.page_align_up(1), 4096);
        assert_eq!(model.page_align_up(4096), 4096);
        assert_eq!(model.page_align_up(4097), 8192);

        assert_eq!(model.page_align_down(0), 0);
        assert_eq!(model.page_align_down(4095), 0);
        assert_eq!(model.page_align_down(4096), 4096);
    }
}
