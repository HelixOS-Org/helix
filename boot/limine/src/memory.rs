//! # Memory Model Abstractions
//!
//! This module provides high-level memory abstractions including physical
//! and virtual address types, HHDM-based address translation, and memory
//! region management.
//!
//! ## Features
//!
//! - Type-safe physical and virtual address handling
//! - HHDM-based address translation
//! - Memory region abstraction with iterator support
//! - Page-level operations
//!
//! ## Example
//!
//! ```rust,no_run
//! use helix_limine::memory::{PhysAddr, VirtAddr, HHDM};
//!
//! // Create typed addresses
//! let phys = PhysAddr::new(0x1000);
//! let virt = VirtAddr::new(0xFFFF_8000_0000_1000);
//!
//! // Translation (requires HHDM)
//! let virt = HHDM::phys_to_virt(phys);
//! let phys = HHDM::virt_to_phys(virt).unwrap();
//! ```

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

// =============================================================================
// Physical Address
// =============================================================================

/// A physical memory address
///
/// Physical addresses are used for hardware-level memory access and
/// must be translated to virtual addresses before being dereferenced.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// The null physical address
    pub const NULL: Self = Self(0);

    /// Maximum valid physical address (52-bit)
    pub const MAX: Self = Self(0x000F_FFFF_FFFF_FFFF);

    /// Page size (4 KB)
    pub const PAGE_SIZE: u64 = 4096;

    /// Large page size (2 MB)
    pub const LARGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

    /// Huge page size (1 GB)
    pub const HUGE_PAGE_SIZE: u64 = 1024 * 1024 * 1024;

    /// Create a new physical address
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create from a raw pointer
    ///
    /// # Safety
    ///
    /// The pointer must represent a valid physical address.
    #[inline]
    pub unsafe fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as u64)
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Check if the address is null
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Check if the address is page-aligned
    #[inline]
    pub const fn is_page_aligned(self) -> bool {
        self.0 & (Self::PAGE_SIZE - 1) == 0
    }

    /// Check if aligned to the given alignment
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 & (align - 1) == 0
    }

    /// Align the address down to a page boundary
    #[inline]
    pub const fn page_align_down(self) -> Self {
        Self(self.0 & !(Self::PAGE_SIZE - 1))
    }

    /// Align the address up to a page boundary
    #[inline]
    pub const fn page_align_up(self) -> Self {
        Self((self.0 + Self::PAGE_SIZE - 1) & !(Self::PAGE_SIZE - 1))
    }

    /// Align down to arbitrary alignment
    #[inline]
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }

    /// Align up to arbitrary alignment
    #[inline]
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Get the page offset
    #[inline]
    pub const fn page_offset(self) -> u64 {
        self.0 & (Self::PAGE_SIZE - 1)
    }

    /// Get the page number
    #[inline]
    pub const fn page_number(self) -> u64 {
        self.0 / Self::PAGE_SIZE
    }

    /// Calculate offset to another address
    #[inline]
    pub const fn offset_from(self, other: Self) -> i64 {
        self.0 as i64 - other.0 as i64
    }

    /// Check if this address is within a range
    #[inline]
    pub const fn is_within(self, base: Self, length: u64) -> bool {
        self.0 >= base.0 && self.0 < base.0 + length
    }

    /// Add an offset (checked)
    #[inline]
    pub const fn checked_add(self, offset: u64) -> Option<Self> {
        match self.0.checked_add(offset) {
            Some(addr) => Some(Self(addr)),
            None => None,
        }
    }

    /// Subtract an offset (checked)
    #[inline]
    pub const fn checked_sub(self, offset: u64) -> Option<Self> {
        match self.0.checked_sub(offset) {
            Some(addr) => Some(Self(addr)),
            None => None,
        }
    }
}

impl Add<u64> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for PhysAddr {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for PhysAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u64> for PhysAddr {
    #[inline]
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: PhysAddr) -> u64 {
        self.0 - rhs.0
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr({:#018x})", self.0)
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl From<u64> for PhysAddr {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl From<PhysAddr> for u64 {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

// =============================================================================
// Virtual Address
// =============================================================================

/// A virtual memory address
///
/// Virtual addresses are used for CPU-level memory access and go through
/// the MMU's page table translation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// The null virtual address
    pub const NULL: Self = Self(0);

    /// Page size (4 KB)
    pub const PAGE_SIZE: u64 = 4096;

    /// Start of higher half (canonical address)
    pub const HIGHER_HALF_START: Self = Self(0xFFFF_8000_0000_0000);

    /// Create a new virtual address
    ///
    /// The address must be canonical (sign-extended from bit 47 or 56).
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a new canonical virtual address
    ///
    /// Non-canonical addresses are made canonical by sign-extension.
    #[inline]
    pub const fn new_canonical(addr: u64) -> Self {
        // Sign-extend from bit 47 for 4-level paging
        let sign_bit = (addr >> 47) & 1;
        if sign_bit == 1 {
            Self(addr | 0xFFFF_0000_0000_0000)
        } else {
            Self(addr & 0x0000_FFFF_FFFF_FFFF)
        }
    }

    /// Create from a raw pointer
    #[inline]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self(ptr as u64)
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Convert to a raw pointer
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Convert to a mutable raw pointer
    #[inline]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Check if the address is null
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Check if the address is in the higher half
    #[inline]
    pub const fn is_higher_half(self) -> bool {
        self.0 >= Self::HIGHER_HALF_START.0
    }

    /// Check if the address is in the lower half
    #[inline]
    pub const fn is_lower_half(self) -> bool {
        !self.is_higher_half()
    }

    /// Check if the address is page-aligned
    #[inline]
    pub const fn is_page_aligned(self) -> bool {
        self.0 & (Self::PAGE_SIZE - 1) == 0
    }

    /// Check if aligned to the given alignment
    #[inline]
    pub const fn is_aligned(self, align: u64) -> bool {
        self.0 & (align - 1) == 0
    }

    /// Align the address down to a page boundary
    #[inline]
    pub const fn page_align_down(self) -> Self {
        Self(self.0 & !(Self::PAGE_SIZE - 1))
    }

    /// Align the address up to a page boundary
    #[inline]
    pub const fn page_align_up(self) -> Self {
        Self((self.0 + Self::PAGE_SIZE - 1) & !(Self::PAGE_SIZE - 1))
    }

    /// Get the page offset
    #[inline]
    pub const fn page_offset(self) -> u64 {
        self.0 & (Self::PAGE_SIZE - 1)
    }

    /// Get page table indices for this address (4-level paging)
    ///
    /// Returns (pml4_index, pdpt_index, pd_index, pt_index)
    #[inline]
    pub const fn page_table_indices(self) -> (u16, u16, u16, u16) {
        let pml4 = ((self.0 >> 39) & 0x1FF) as u16;
        let pdpt = ((self.0 >> 30) & 0x1FF) as u16;
        let pd = ((self.0 >> 21) & 0x1FF) as u16;
        let pt = ((self.0 >> 12) & 0x1FF) as u16;
        (pml4, pdpt, pd, pt)
    }

    /// Add an offset (checked)
    #[inline]
    pub const fn checked_add(self, offset: u64) -> Option<Self> {
        match self.0.checked_add(offset) {
            Some(addr) => Some(Self(addr)),
            None => None,
        }
    }

    /// Subtract an offset (checked)
    #[inline]
    pub const fn checked_sub(self, offset: u64) -> Option<Self> {
        match self.0.checked_sub(offset) {
            Some(addr) => Some(Self(addr)),
            None => None,
        }
    }
}

impl Add<u64> for VirtAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for VirtAddr {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for VirtAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u64> for VirtAddr {
    #[inline]
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: VirtAddr) -> u64 {
        self.0 - rhs.0
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtAddr({:#018x})", self.0)
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl From<u64> for VirtAddr {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl From<VirtAddr> for u64 {
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}

impl<T> From<*const T> for VirtAddr {
    fn from(ptr: *const T) -> Self {
        Self(ptr as u64)
    }
}

impl<T> From<*mut T> for VirtAddr {
    fn from(ptr: *mut T) -> Self {
        Self(ptr as u64)
    }
}

// =============================================================================
// HHDM (Higher Half Direct Map)
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

/// Global HHDM offset storage
static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Higher Half Direct Map utilities
///
/// The HHDM is a direct mapping of all physical memory at a high virtual
/// address. This allows easy and efficient translation between physical
/// and virtual addresses.
pub struct HHDM;

impl HHDM {
    /// Initialize the HHDM with the offset from the bootloader
    ///
    /// This should be called once during early boot with the value
    /// from the HHDM response.
    pub fn init(offset: u64) {
        HHDM_OFFSET.store(offset, Ordering::Release);
    }

    /// Get the HHDM offset
    ///
    /// Returns 0 if not initialized.
    pub fn offset() -> u64 {
        HHDM_OFFSET.load(Ordering::Acquire)
    }

    /// Check if HHDM has been initialized
    pub fn is_initialized() -> bool {
        HHDM_OFFSET.load(Ordering::Acquire) != 0
    }

    /// Convert a physical address to virtual using HHDM
    pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
        VirtAddr::new(phys.as_u64() + Self::offset())
    }

    /// Convert a virtual address to physical using HHDM
    ///
    /// Returns None if the address is not in the HHDM range.
    pub fn virt_to_phys(virt: VirtAddr) -> Option<PhysAddr> {
        let offset = Self::offset();
        let addr = virt.as_u64();

        if addr >= offset {
            Some(PhysAddr::new(addr - offset))
        } else {
            None
        }
    }

    /// Get a reference to physical memory
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical address is valid and
    /// points to properly initialized memory of type T.
    pub unsafe fn phys_ref<T>(phys: PhysAddr) -> &'static T {
        let virt = Self::phys_to_virt(phys);
        unsafe { &*virt.as_ptr() }
    }

    /// Get a mutable reference to physical memory
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical address is valid and
    /// points to properly initialized memory of type T, and that
    /// no other references to this memory exist.
    pub unsafe fn phys_ref_mut<T>(phys: PhysAddr) -> &'static mut T {
        let virt = Self::phys_to_virt(phys);
        unsafe { &mut *virt.as_mut_ptr() }
    }

    /// Get a slice of physical memory
    ///
    /// # Safety
    ///
    /// Same requirements as phys_ref, plus the length must be valid.
    pub unsafe fn phys_slice<T>(phys: PhysAddr, len: usize) -> &'static [T] {
        let virt = Self::phys_to_virt(phys);
        unsafe { core::slice::from_raw_parts(virt.as_ptr(), len) }
    }

    /// Get a mutable slice of physical memory
    ///
    /// # Safety
    ///
    /// Same requirements as phys_ref_mut, plus the length must be valid.
    pub unsafe fn phys_slice_mut<T>(phys: PhysAddr, len: usize) -> &'static mut [T] {
        let virt = Self::phys_to_virt(phys);
        unsafe { core::slice::from_raw_parts_mut(virt.as_mut_ptr(), len) }
    }
}

// =============================================================================
// Memory Region
// =============================================================================

/// A memory region descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Base physical address
    pub base: PhysAddr,
    /// Length in bytes
    pub length: u64,
    /// Region type
    pub kind: MemoryRegionKind,
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(base: PhysAddr, length: u64, kind: MemoryRegionKind) -> Self {
        Self { base, length, kind }
    }

    /// Get the end address (exclusive)
    pub fn end(&self) -> PhysAddr {
        self.base + self.length
    }

    /// Get the size in bytes
    pub fn size(&self) -> u64 {
        self.length
    }

    /// Get the region kind
    pub fn region_kind(&self) -> MemoryRegionKind {
        self.kind
    }

    /// Check if this region contains an address
    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr >= self.base && addr < self.end()
    }

    /// Check if this region overlaps with another
    pub fn overlaps(&self, other: &Self) -> bool {
        self.base < other.end() && other.base < self.end()
    }

    /// Check if this is usable memory
    pub fn is_usable(&self) -> bool {
        self.kind == MemoryRegionKind::Usable
    }

    /// Check if this memory can be reclaimed
    pub fn is_reclaimable(&self) -> bool {
        matches!(
            self.kind,
            MemoryRegionKind::BootloaderReclaimable | MemoryRegionKind::AcpiReclaimable
        )
    }

    /// Get the size in pages
    pub fn page_count(&self) -> u64 {
        (self.length + PhysAddr::PAGE_SIZE - 1) / PhysAddr::PAGE_SIZE
    }
}

impl fmt::Display for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} ({:?}, {} KB)",
            self.base,
            self.end(),
            self.kind,
            self.length / 1024
        )
    }
}

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MemoryRegionKind {
    /// Usable RAM
    Usable,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Bootloader reclaimable
    BootloaderReclaimable,
    /// Kernel and modules
    KernelAndModules,
    /// Framebuffer
    Framebuffer,
    /// Unknown
    Unknown,
}

impl MemoryRegionKind {
    /// Get a human-readable name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Usable => "Usable",
            Self::Reserved => "Reserved",
            Self::AcpiReclaimable => "ACPI Reclaimable",
            Self::AcpiNvs => "ACPI NVS",
            Self::BadMemory => "Bad Memory",
            Self::BootloaderReclaimable => "Bootloader Reclaimable",
            Self::KernelAndModules => "Kernel and Modules",
            Self::Framebuffer => "Framebuffer",
            Self::Unknown => "Unknown",
        }
    }

    /// Convert from Limine memory type constant
    pub const fn from_limine(ty: u64) -> Self {
        match ty {
            0 => Self::Usable,
            1 => Self::Reserved,
            2 => Self::AcpiReclaimable,
            3 => Self::AcpiNvs,
            4 => Self::BadMemory,
            5 => Self::BootloaderReclaimable,
            6 => Self::KernelAndModules,
            7 => Self::Framebuffer,
            _ => Self::Unknown,
        }
    }

    /// Convert to Limine memory type constant
    pub const fn to_limine(self) -> u64 {
        match self {
            Self::Usable => 0,
            Self::Reserved => 1,
            Self::AcpiReclaimable => 2,
            Self::AcpiNvs => 3,
            Self::BadMemory => 4,
            Self::BootloaderReclaimable => 5,
            Self::KernelAndModules => 6,
            Self::Framebuffer => 7,
            Self::Unknown => 0xFF,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phys_addr() {
        let addr = PhysAddr::new(0x1000);
        assert!(addr.is_page_aligned());
        assert_eq!(addr.page_number(), 1);
        assert_eq!(addr.page_offset(), 0);

        let addr2 = PhysAddr::new(0x1234);
        assert!(!addr2.is_page_aligned());
        assert_eq!(addr2.page_align_down().as_u64(), 0x1000);
        assert_eq!(addr2.page_align_up().as_u64(), 0x2000);
    }

    #[test]
    fn test_virt_addr() {
        let lower = VirtAddr::new(0x0000_7FFF_FFFF_FFFF);
        assert!(lower.is_lower_half());

        let higher = VirtAddr::new(0xFFFF_8000_0000_0000);
        assert!(higher.is_higher_half());
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::new(PhysAddr::new(0x1000), 0x2000, MemoryRegionKind::Usable);

        assert!(region.contains(PhysAddr::new(0x1500)));
        assert!(!region.contains(PhysAddr::new(0x4000)));
        assert!(region.is_usable());
    }
}
