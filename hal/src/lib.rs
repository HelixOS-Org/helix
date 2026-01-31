//! # Helix HAL - Hardware Abstraction Layer
//!
//! This crate defines the core traits and abstractions for hardware interaction.
//! All architecture-specific implementations must implement these traits.
//!
//! ## Design Philosophy
//!
//! The HAL is designed to be:
//! - **Complete**: Abstracts all hardware details needed by the kernel
//! - **Minimal**: Only exposes what's necessary
//! - **Safe**: Encapsulates all unsafe operations
//! - **Extensible**: New architectures can be added easily
//!
//! ## Kernel Relocation
//!
//! The HAL provides comprehensive support for kernel relocation:
//!
//! - [`relocation`]: ELF relocation engine for PIE kernels
//! - [`kaslr`]: Kernel Address Space Layout Randomization
//!
//! See the [Kernel Relocation Architecture](../docs/architecture/KERNEL_RELOCATION.md)
//! document for details.

#![no_std]
#![feature(negative_impls)]
#![feature(auto_traits)]
#![feature(naked_functions)]
#![feature(asm_experimental_arch)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod cpu;
pub mod mmu;
pub mod interrupts;
pub mod firmware;

// Kernel relocation support
pub mod relocation;
pub mod kaslr;

// Architecture-specific implementations
pub mod arch;

// Stub implementations (for fallback)
pub mod arch_stubs;

use core::fmt::Debug;

/// Result type for HAL operations
pub type HalResult<T> = Result<T, HalError>;

/// Errors that can occur in HAL operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalError {
    /// The operation is not supported on this architecture
    NotSupported,
    /// Invalid parameter provided
    InvalidParameter,
    /// Hardware reported an error
    HardwareError,
    /// Resource is not available
    ResourceBusy,
    /// Operation timed out
    Timeout,
    /// Address is invalid or not aligned
    InvalidAddress,
    /// Memory allocation failed
    OutOfMemory,
    /// Permission denied
    PermissionDenied,
    /// Feature not initialized
    NotInitialized,
}

/// Marker trait for architecture-specific HAL implementations
pub auto trait ArchSpecific {}

/// The main HAL trait that architecture implementations must provide
///
/// This trait aggregates all sub-traits needed for a complete HAL implementation.
pub trait HardwareAbstractionLayer: Send + Sync + 'static {
    /// The CPU abstraction type
    type Cpu: cpu::CpuAbstraction;

    /// The MMU abstraction type
    type Mmu: mmu::MmuAbstraction;

    /// The interrupt controller abstraction type
    type InterruptController: interrupts::InterruptController;

    /// The firmware interface type
    type Firmware: firmware::FirmwareInterface;

    /// Get the CPU abstraction
    fn cpu(&self) -> &Self::Cpu;

    /// Get the MMU abstraction
    fn mmu(&self) -> &Self::Mmu;

    /// Get the interrupt controller
    fn interrupt_controller(&self) -> &Self::InterruptController;

    /// Get the firmware interface
    fn firmware(&self) -> &Self::Firmware;

    /// Get the architecture name
    fn arch_name(&self) -> &'static str;

    /// Get the architecture version/revision
    fn arch_version(&self) -> &'static str;

    /// Initialize early hardware (called very early in boot)
    fn early_init(&mut self) -> HalResult<()>;

    /// Full hardware initialization
    fn init(&mut self) -> HalResult<()>;

    /// Halt the system
    fn halt(&self) -> !;

    /// Reboot the system
    fn reboot(&self) -> !;

    /// Shutdown the system
    fn shutdown(&self) -> !;
}

/// Physical address type (architecture-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Create a new physical address
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if the address is aligned to the given alignment
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 % align == 0
    }

    /// Align the address up to the given alignment
    #[inline]
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Align the address down to the given alignment
    #[inline]
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }

    /// Add an offset to the address
    #[inline]
    pub const fn add(self, offset: u64) -> Self {
        Self(self.0 + offset)
    }

    /// Subtract an offset from the address
    #[inline]
    pub const fn sub(self, offset: u64) -> Self {
        Self(self.0 - offset)
    }
}

/// Virtual address type (architecture-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Create a new virtual address
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if the address is aligned to the given alignment
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 % align == 0
    }

    /// Align the address up to the given alignment
    #[inline]
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Align the address down to the given alignment
    #[inline]
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }

    /// Convert to a raw pointer
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Convert to a raw mutable pointer
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}

/// Page size enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PageSize {
    /// 4 KiB page
    Size4KiB,
    /// 2 MiB page (large page)
    Size2MiB,
    /// 1 GiB page (huge page)
    Size1GiB,
}

impl PageSize {
    /// Get the size in bytes
    #[inline]
    pub const fn size(self) -> u64 {
        match self {
            PageSize::Size4KiB => 4 * 1024,
            PageSize::Size2MiB => 2 * 1024 * 1024,
            PageSize::Size1GiB => 1024 * 1024 * 1024,
        }
    }
}
