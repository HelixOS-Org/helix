//! Kprobe Core Types
//!
//! Fundamental types for dynamic kernel probing.

use alloc::string::String;

/// Kprobe identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KprobeId(pub u64);

impl KprobeId {
    /// Create a new kprobe ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Kretprobe identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KretprobeId(pub u64);

impl KretprobeId {
    /// Create a new kretprobe ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Probe address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProbeAddress(pub u64);

impl ProbeAddress {
    /// Create a new probe address
    #[inline(always)]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Check if address is aligned
    #[inline(always)]
    pub fn is_aligned(&self, alignment: u64) -> bool {
        self.0 % alignment == 0
    }

    /// Get offset from base
    #[inline(always)]
    pub fn offset_from(&self, base: u64) -> u64 {
        self.0.saturating_sub(base)
    }
}

/// Symbol information
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Symbol name
    pub name: String,
    /// Symbol address
    pub address: ProbeAddress,
    /// Symbol size
    pub size: u64,
    /// Module name (if in module)
    pub module: Option<String>,
    /// Is function
    pub is_function: bool,
    /// Is exported
    pub is_exported: bool,
}

impl SymbolInfo {
    /// Create new symbol info
    pub fn new(name: String, address: ProbeAddress, size: u64) -> Self {
        Self {
            name,
            address,
            size,
            module: None,
            is_function: true,
            is_exported: false,
        }
    }

    /// Check if address is within symbol
    #[inline(always)]
    pub fn contains(&self, addr: ProbeAddress) -> bool {
        addr.raw() >= self.address.raw() && addr.raw() < self.address.raw() + self.size
    }

    /// Get offset within symbol
    #[inline]
    pub fn offset(&self, addr: ProbeAddress) -> Option<u64> {
        if self.contains(addr) {
            Some(addr.raw() - self.address.raw())
        } else {
            None
        }
    }
}

/// Kprobe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KprobeState {
    /// Registered but not armed
    Registered,
    /// Armed and active
    Armed,
    /// Temporarily disabled
    Disabled,
    /// Hit and processing
    Firing,
    /// Error state
    Error,
}

/// Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// x86_64
    X86_64,
    /// AArch64
    Aarch64,
    /// RISC-V 64
    Riscv64,
}

impl Architecture {
    /// Get breakpoint instruction
    #[inline]
    pub fn breakpoint_instruction(&self) -> &'static [u8] {
        match self {
            Self::X86_64 => &[0xCC], // INT3
            Self::Aarch64 => &[0x00, 0x00, 0x20, 0xD4], // BRK #0
            Self::Riscv64 => &[0x73, 0x00, 0x10, 0x00], // EBREAK
        }
    }

    /// Get NOP instruction
    #[inline]
    pub fn nop_instruction(&self) -> &'static [u8] {
        match self {
            Self::X86_64 => &[0x90], // NOP
            Self::Aarch64 => &[0x1F, 0x20, 0x03, 0xD5], // NOP
            Self::Riscv64 => &[0x13, 0x00, 0x00, 0x00], // ADDI x0, x0, 0
        }
    }

    /// Get instruction alignment
    #[inline]
    pub fn instruction_alignment(&self) -> u64 {
        match self {
            Self::X86_64 => 1,
            Self::Aarch64 => 4,
            Self::Riscv64 => 2, // Compressed instructions
        }
    }
}
