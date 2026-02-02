//! # Helix Relocation Subsystem
//!
//! Industrial-grade kernel relocation for the Helix OS framework.
//!
//! ## Overview
//!
//! This subsystem provides complete ELF relocation capabilities, enabling:
//! - Position Independent Executable (PIE) kernels
//! - Kernel Address Space Layout Randomization (KASLR)
//! - Dynamic module loading
//! - Multi-boot protocol support
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    RELOCATION SUBSYSTEM                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   Context   │  │   Engine    │  │      Consumers      │  │
//! │  │  (state)    │──│  (apply)    │──│  (kernel, modules)  │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_relocation::{
//!     KaslrManager, RelocatableKernel, RelocationContext,
//! };
//!
//! // Create KASLR manager and generate random base
//! let mut kaslr = KaslrManager::new_secure();
//! let load_addr = kaslr.generate_load_address(kernel_size)?;
//!
//! // Create relocation context
//! let ctx = RelocationContext::builder()
//!     .phys_base(0x100000)
//!     .virt_base(load_addr)
//!     .link_base(0xFFFFFFFF80000000)
//!     .kernel_size(kernel_size)
//!     .build()?;
//!
//! // Apply relocations
//! let mut kernel = RelocatableKernel::new(ctx);
//! unsafe { kernel.apply_all()? };
//!
//! // Verify and finalize
//! kernel.verify_integrity()?;
//! let relocated = kernel.finalize();
//! ```
//!
//! ## Features
//!
//! - `kaslr`: Enable KASLR support with hardware entropy
//! - `debug`: Enable detailed debug logging
//! - `stats`: Collect relocation statistics
//! - `validation`: Enable integrity verification
//!
//! ## Safety
//!
//! This crate operates on raw memory and requires careful use:
//! - All relocation functions are `unsafe`
//! - Caller must ensure valid ELF data
//! - Memory must be writable during relocation

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

// Only use alloc if available
#[cfg(feature = "alloc")]
extern crate alloc;

// ============================================================================
// MODULES
// ============================================================================

/// Core relocation context and state management
pub mod context;

/// ELF parsing and structure definitions
pub mod elf;

/// Relocation engine and application logic
pub mod engine;

/// Architecture-specific relocation implementations
pub mod arch;

/// KASLR (Kernel Address Space Layout Randomization)
pub mod kaslr;

/// Boot protocol integrations
pub mod boot;

/// Validation and integrity checking
pub mod validation;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use arch::current_arch;
pub use boot::{BootAdapter, BootContext};
pub use context::{
    BootProtocol, KernelState, RelocationContext, RelocationContextBuilder, RelocationStrategy,
};
pub use elf::{Elf64Dyn, Elf64Header, Elf64Phdr, Elf64Rela, Elf64Shdr, Elf64Sym, ElfInfo};
pub use engine::{EarlyRelocator, FullRelocator, Relocatable, RelocatableKernel, RelocationEngine};
pub use kaslr::{EntropyCollector, EntropyQuality, Kaslr, KaslrConfig};

// ============================================================================
// ERROR TYPES
// ============================================================================

/// Relocation error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocError {
    /// Invalid ELF magic number
    InvalidElfMagic,
    /// Invalid ELF class (expected 64-bit)
    InvalidElfClass,
    /// Invalid ELF machine type
    InvalidElfMachine,
    /// Unsupported relocation type
    UnsupportedRelocType(u32),
    /// Relocation target out of bounds
    OutOfBounds(u64),
    /// Arithmetic overflow during calculation
    Overflow(u64),
    /// Required section not found
    SectionNotFound(&'static str),
    /// Symbol not found
    SymbolNotFound(u32),
    /// No relocations found (may be OK for static binaries)
    NoRelocations,
    /// Too many relocations
    TooManyRelocations(usize),
    /// Too many errors occurred
    TooManyErrors(usize),
    /// Integrity check failed
    IntegrityFailed(&'static str),
    /// Insufficient entropy for KASLR
    InsufficientEntropy,
    /// Context not initialized
    NotInitialized,
    /// Already finalized
    AlreadyFinalized,
    /// Invalid address
    InvalidAddress,
    /// Invalid kernel layout
    InvalidKernelLayout,
    /// Misaligned access
    MisalignedAccess(u64),
    /// Invalid alignment
    InvalidAlignment {
        /// Required alignment
        required: u64,
        /// Actual alignment
        actual: u64,
    },
}

impl core::fmt::Display for RelocError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidElfMagic => write!(f, "Invalid ELF magic"),
            Self::InvalidElfClass => write!(f, "Invalid ELF class (need 64-bit)"),
            Self::InvalidElfMachine => write!(f, "Invalid ELF machine type"),
            Self::UnsupportedRelocType(t) => write!(f, "Unsupported relocation type: {}", t),
            Self::OutOfBounds(offset) => write!(f, "Out of bounds: offset 0x{:x}", offset),
            Self::Overflow(offset) => write!(f, "Overflow at 0x{:x}", offset),
            Self::SectionNotFound(s) => write!(f, "Section not found: {}", s),
            Self::SymbolNotFound(i) => write!(f, "Symbol not found: index {}", i),
            Self::NoRelocations => write!(f, "No relocations found"),
            Self::TooManyRelocations(n) => write!(f, "Too many relocations: {}", n),
            Self::TooManyErrors(n) => write!(f, "Too many errors: {}", n),
            Self::IntegrityFailed(msg) => write!(f, "Integrity check failed: {}", msg),
            Self::InsufficientEntropy => write!(f, "Insufficient entropy for KASLR"),
            Self::NotInitialized => write!(f, "Context not initialized"),
            Self::AlreadyFinalized => write!(f, "Already finalized"),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::InvalidKernelLayout => write!(f, "Invalid kernel layout"),
            Self::MisalignedAccess(addr) => write!(f, "Misaligned access at 0x{:x}", addr),
            Self::InvalidAlignment { required, actual } => {
                write!(
                    f,
                    "Invalid alignment: need 0x{:x}, got 0x{:x}",
                    required, actual
                )
            },
        }
    }
}

/// Result type for relocation operations
pub type RelocResult<T> = Result<T, RelocError>;

// ============================================================================
// ADDRESS TYPES
// ============================================================================

/// Physical address (usize wrapper for type safety)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    /// Create new physical address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get raw value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if address is page-aligned
    pub const fn is_page_aligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    /// Check if address is huge-page-aligned (2MB)
    pub const fn is_huge_aligned(self) -> bool {
        self.0 & 0x1FFFFF == 0
    }

    /// Align up to page boundary
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }

    /// Align up to huge page boundary
    pub const fn align_up_huge(self) -> Self {
        Self((self.0 + 0x1FFFFF) & !0x1FFFFF)
    }
}

impl core::ops::Add<u64> for PhysAddr {
    type Output = Self;
    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<PhysAddr> for PhysAddr {
    type Output = u64;
    fn sub(self, rhs: PhysAddr) -> u64 {
        self.0 - rhs.0
    }
}

/// Virtual address (usize wrapper for type safety)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

impl VirtAddr {
    /// Create new virtual address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get raw value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if this is a higher-half address
    pub const fn is_higher_half(self) -> bool {
        self.0 >= 0xFFFF_8000_0000_0000
    }

    /// Check if address is page-aligned
    pub const fn is_page_aligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    /// Align up to page boundary
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }

    /// Align up to huge page boundary (2MB)
    pub const fn align_up_huge(self) -> Self {
        Self((self.0 + 0x1FFFFF) & !0x1FFFFF)
    }
}

impl core::ops::Add<u64> for VirtAddr {
    type Output = Self;
    fn add(self, rhs: u64) -> Self {
        Self(self.0.wrapping_add(rhs))
    }
}

impl core::ops::Sub<VirtAddr> for VirtAddr {
    type Output = i64;
    fn sub(self, rhs: VirtAddr) -> i64 {
        self.0.wrapping_sub(rhs.0) as i64
    }
}

// ============================================================================
// RELOCATION STATISTICS
// ============================================================================

/// Statistics from relocation process
#[derive(Debug, Default, Clone, Copy)]
pub struct RelocationStats {
    /// Total relocations processed
    pub total: usize,
    /// Successfully applied
    pub applied: usize,
    /// Skipped (already correct or not needed)
    pub skipped: usize,
    /// Errors encountered
    pub errors: usize,
    /// R_X86_64_RELATIVE count
    pub relative_count: usize,
    /// R_X86_64_64 count
    pub absolute_count: usize,
    /// R_X86_64_PC32 count
    pub pc32_count: usize,
    /// GOT entries patched
    pub got_entries: usize,
    /// PLT entries patched
    pub plt_entries: usize,
    /// Time taken (cycles or nanoseconds)
    pub time_taken: u64,
}

impl RelocationStats {
    /// Create empty stats
    pub const fn new() -> Self {
        Self {
            total: 0,
            applied: 0,
            skipped: 0,
            errors: 0,
            relative_count: 0,
            absolute_count: 0,
            pc32_count: 0,
            got_entries: 0,
            plt_entries: 0,
            time_taken: 0,
        }
    }

    /// Merge two stats
    pub fn merge(&mut self, other: &Self) {
        self.total += other.total;
        self.applied += other.applied;
        self.skipped += other.skipped;
        self.errors += other.errors;
        self.relative_count += other.relative_count;
        self.absolute_count += other.absolute_count;
        self.pc32_count += other.pc32_count;
        self.got_entries += other.got_entries;
        self.plt_entries += other.plt_entries;
        self.time_taken += other.time_taken;
    }
}

// ============================================================================
// VERSION INFO
// ============================================================================

/// Subsystem version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Subsystem name
pub const NAME: &str = "helix-relocation";

/// Get version info string
pub fn version_string() -> &'static str {
    concat!("helix-relocation v", env!("CARGO_PKG_VERSION"))
}
