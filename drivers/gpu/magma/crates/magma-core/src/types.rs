//! # MAGMA Core Types
//!
//! Fundamental type definitions used across the entire driver stack.
//!
//! These types provide:
//! - Strong typing for addresses (CPU vs GPU vs PCI)
//! - Hardware-specific identifiers
//! - Size and alignment guarantees

use core::fmt;
use core::ops::{Add, Sub};

// =============================================================================
// GPU ADDRESS
// =============================================================================

/// GPU Virtual Address
///
/// This is an address in the GPU's virtual address space.
/// It is NOT a CPU pointer and cannot be dereferenced directly.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct GpuAddr(u64);

impl GpuAddr {
    /// Create a new GPU address
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a null GPU address
    #[inline]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Get the raw u64 value
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Check if null
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Check alignment
    #[inline]
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0 & (alignment - 1) == 0
    }

    /// Align up to boundary
    #[inline]
    pub const fn align_up(self, alignment: u64) -> Self {
        let mask = alignment - 1;
        Self((self.0 + mask) & !mask)
    }

    /// Align down to boundary
    #[inline]
    pub const fn align_down(self, alignment: u64) -> Self {
        let mask = alignment - 1;
        Self(self.0 & !mask)
    }

    /// Offset by bytes
    #[inline]
    pub const fn offset(self, bytes: u64) -> Self {
        Self(self.0.wrapping_add(bytes))
    }
}

impl Add<u64> for GpuAddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl Sub<GpuAddr> for GpuAddr {
    type Output = u64;

    fn sub(self, rhs: GpuAddr) -> Self::Output {
        self.0.wrapping_sub(rhs.0)
    }
}

impl fmt::Debug for GpuAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GpuAddr(0x{:016x})", self.0)
    }
}

impl fmt::Display for GpuAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

// =============================================================================
// PHYSICAL ADDRESS (for IOMMU/DMA)
// =============================================================================

/// Physical memory address (for DMA)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Create a new physical address
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw u64 value
    #[inline]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Align up
    #[inline]
    pub const fn align_up(self, alignment: u64) -> Self {
        let mask = alignment - 1;
        Self((self.0 + mask) & !mask)
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr(0x{:016x})", self.0)
    }
}

// =============================================================================
// PCI ADDRESS (BDF)
// =============================================================================

/// PCI Bus:Device.Function address
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PciAddr {
    /// Domain (segment)
    pub domain: u16,
    /// Bus number
    pub bus: u8,
    /// Device number (0-31)
    pub device: u8,
    /// Function number (0-7)
    pub function: u8,
}

impl PciAddr {
    /// Create a new PCI address
    #[inline]
    pub const fn new(domain: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            domain,
            bus,
            device,
            function,
        }
    }

    /// Create from BDF u16 (legacy format)
    #[inline]
    pub const fn from_bdf(bdf: u16) -> Self {
        Self {
            domain: 0,
            bus: ((bdf >> 8) & 0xFF) as u8,
            device: ((bdf >> 3) & 0x1F) as u8,
            function: (bdf & 0x07) as u8,
        }
    }

    /// Convert to BDF u16
    #[inline]
    pub const fn to_bdf(self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }
}

impl fmt::Debug for PciAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PCI({:04x}:{:02x}:{:02x}.{:x})",
            self.domain, self.bus, self.device, self.function
        )
    }
}

impl fmt::Display for PciAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{:x}",
            self.domain, self.bus, self.device, self.function
        )
    }
}

// =============================================================================
// GPU GENERATION
// =============================================================================

/// NVIDIA GPU generation/architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum GpuGeneration {
    /// Unknown/unsupported generation
    Unknown   = 0,
    /// Tesla (GT200)
    Tesla     = 0x50,
    /// Fermi (GF100)
    Fermi     = 0xC0,
    /// Kepler (GK104)
    Kepler    = 0xE0,
    /// Maxwell (GM204)
    Maxwell   = 0x110,
    /// Pascal (GP104)
    Pascal    = 0x130,
    /// Volta (GV100)
    Volta     = 0x140,
    /// Turing (TU104) - RTX 20xx
    Turing    = 0x160,
    /// Ampere (GA102) - RTX 30xx
    Ampere    = 0x170,
    /// Ada Lovelace (AD102) - RTX 40xx
    Ada       = 0x190,
    /// Blackwell (GB202) - RTX 50xx
    Blackwell = 0x1A0,
}

impl GpuGeneration {
    /// Check if this generation has GSP support
    #[inline]
    pub const fn has_gsp(self) -> bool {
        matches!(
            self,
            Self::Turing | Self::Ampere | Self::Ada | Self::Blackwell
        )
    }

    /// Check if this generation supports PCIe Gen4+
    #[inline]
    pub const fn has_pcie_gen4(self) -> bool {
        matches!(self, Self::Ampere | Self::Ada | Self::Blackwell)
    }

    /// Get compute capability major version
    pub const fn compute_capability_major(self) -> u32 {
        match self {
            Self::Turing => 7,
            Self::Ampere => 8,
            Self::Ada => 8,
            Self::Blackwell => 9,
            _ => 0,
        }
    }
}

// =============================================================================
// GPU DEVICE ID
// =============================================================================

/// GPU Device identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuDeviceId {
    /// PCI Vendor ID (always 0x10DE for NVIDIA)
    pub vendor: u16,
    /// PCI Device ID
    pub device: u16,
    /// Subsystem vendor ID
    pub subsystem_vendor: u16,
    /// Subsystem device ID
    pub subsystem_device: u16,
    /// Revision
    pub revision: u8,
}

impl GpuDeviceId {
    /// NVIDIA vendor ID
    pub const NVIDIA_VENDOR_ID: u16 = 0x10DE;

    /// Check if this is an NVIDIA device
    #[inline]
    pub const fn is_nvidia(&self) -> bool {
        self.vendor == Self::NVIDIA_VENDOR_ID
    }

    /// Determine GPU generation from device ID
    pub fn generation(&self) -> GpuGeneration {
        let chip_id = (self.device >> 4) & 0x1FF;

        match chip_id {
            0x160..=0x16F => GpuGeneration::Turing,
            0x170..=0x17F | 0x180..=0x18F => GpuGeneration::Ampere,
            0x190..=0x19F => GpuGeneration::Ada,
            0x1A0..=0x1AF => GpuGeneration::Blackwell,
            _ => GpuGeneration::Unknown,
        }
    }
}

impl fmt::Debug for GpuDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuDeviceId({:04x}:{:04x}, subsys {:04x}:{:04x}, rev {:02x})",
            self.vendor, self.device, self.subsystem_vendor, self.subsystem_device, self.revision
        )
    }
}

// =============================================================================
// SIZE TYPES
// =============================================================================

/// Size in bytes (for VRAM allocations)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ByteSize(u64);

impl ByteSize {
    /// Zero size
    pub const ZERO: Self = Self(0);
    /// 4 KiB
    pub const KIB_4: Self = Self(4 * 1024);
    /// 64 KiB
    pub const KIB_64: Self = Self(64 * 1024);
    /// 2 MiB (huge page)
    pub const MIB_2: Self = Self(2 * 1024 * 1024);
    /// 1 GiB
    pub const GIB_1: Self = Self(1024 * 1024 * 1024);

    /// Create from bytes
    #[inline]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Create from KiB
    #[inline]
    pub const fn from_kib(kib: u64) -> Self {
        Self(kib * 1024)
    }

    /// Create from MiB
    #[inline]
    pub const fn from_mib(mib: u64) -> Self {
        Self(mib * 1024 * 1024)
    }

    /// Create from GiB
    #[inline]
    pub const fn from_gib(gib: u64) -> Self {
        Self(gib * 1024 * 1024 * 1024)
    }

    /// Get as bytes
    #[inline]
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    /// Get as KiB
    #[inline]
    pub const fn as_kib(self) -> u64 {
        self.0 / 1024
    }

    /// Get as MiB
    #[inline]
    pub const fn as_mib(self) -> u64 {
        self.0 / (1024 * 1024)
    }

    /// Align up
    #[inline]
    pub const fn align_up(self, alignment: u64) -> Self {
        let mask = alignment - 1;
        Self((self.0 + mask) & !mask)
    }
}

impl fmt::Debug for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1024 * 1024 * 1024 {
            write!(f, "{} GiB", self.0 / (1024 * 1024 * 1024))
        } else if self.0 >= 1024 * 1024 {
            write!(f, "{} MiB", self.0 / (1024 * 1024))
        } else if self.0 >= 1024 {
            write!(f, "{} KiB", self.0 / 1024)
        } else {
            write!(f, "{} B", self.0)
        }
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// =============================================================================
// HANDLE TYPES
// =============================================================================

/// Opaque handle to a GPU resource
///
/// Handles are type-safe wrappers that prevent mixing different resource types.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle<T> {
    id: u64,
    _marker: core::marker::PhantomData<T>,
}

impl<T> Handle<T> {
    /// Create a new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self {
            id,
            _marker: core::marker::PhantomData,
        }
    }

    /// Create a null handle
    #[inline]
    pub const fn null() -> Self {
        Self::new(0)
    }

    /// Get the raw ID
    #[inline]
    pub const fn id(self) -> u64 {
        self.id
    }

    /// Check if null
    #[inline]
    pub const fn is_null(self) -> bool {
        self.id == 0
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}>(0x{:x})",
            core::any::type_name::<T>(),
            self.id
        )
    }
}

// Marker types for handles
/// Marker for buffer handle
pub struct BufferMarker;
/// Marker for image handle
pub struct ImageMarker;
/// Marker for command buffer handle
pub struct CommandBufferMarker;
/// Marker for shader handle
pub struct ShaderMarker;
/// Marker for pipeline handle
pub struct PipelineMarker;
/// Marker for fence handle
pub struct FenceMarker;
/// Marker for semaphore handle
pub struct SemaphoreMarker;
/// Marker for channel handle
pub struct ChannelMarker;

/// Handle to a GPU buffer
pub type BufferHandle = Handle<BufferMarker>;
/// Handle to a GPU image
pub type ImageHandle = Handle<ImageMarker>;
/// Handle to a command buffer
pub type CommandBufferHandle = Handle<CommandBufferMarker>;
/// Handle to a shader
pub type ShaderHandle = Handle<ShaderMarker>;
/// Handle to a pipeline
pub type PipelineHandle = Handle<PipelineMarker>;
/// Handle to a fence
pub type FenceHandle = Handle<FenceMarker>;
/// Handle to a semaphore
pub type SemaphoreHandle = Handle<SemaphoreMarker>;
/// Handle to a GPU channel
pub type ChannelHandle = Handle<ChannelMarker>;
