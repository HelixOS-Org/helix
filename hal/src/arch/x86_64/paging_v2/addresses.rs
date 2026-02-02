//! # Address Types
//!
//! This module provides type-safe physical and virtual address types,
//! as well as frame and page abstractions.

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::{PAGE_SIZE_1G, PAGE_SIZE_2M, PAGE_SIZE_4K, PHYS_ADDR_MASK};

// =============================================================================
// Page Size
// =============================================================================

/// Page size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PageSize {
    /// 4 KB page
    Size4K,
    /// 2 MB huge page
    Size2M,
    /// 1 GB huge page
    Size1G,
}

impl PageSize {
    /// Get the size in bytes
    #[inline]
    pub const fn size(self) -> usize {
        match self {
            PageSize::Size4K => PAGE_SIZE_4K,
            PageSize::Size2M => PAGE_SIZE_2M,
            PageSize::Size1G => PAGE_SIZE_1G,
        }
    }

    /// Get the page table level that handles this size
    #[inline]
    pub const fn level(self) -> u8 {
        match self {
            PageSize::Size4K => 1, // PT level
            PageSize::Size2M => 2, // PD level
            PageSize::Size1G => 3, // PDPT level
        }
    }

    /// Get the number of 4K pages in this page size
    #[inline]
    pub const fn pages_4k(self) -> usize {
        match self {
            PageSize::Size4K => 1,
            PageSize::Size2M => 512,
            PageSize::Size1G => 512 * 512,
        }
    }

    /// Get the alignment mask
    #[inline]
    pub const fn mask(self) -> u64 {
        !(self.size() as u64 - 1)
    }

    /// Check if an address is aligned to this page size
    #[inline]
    pub const fn is_aligned(self, addr: u64) -> bool {
        addr & !self.mask() == 0
    }

    /// Align an address down to this page size
    #[inline]
    pub const fn align_down(self, addr: u64) -> u64 {
        addr & self.mask()
    }

    /// Align an address up to this page size
    #[inline]
    pub const fn align_up(self, addr: u64) -> u64 {
        let size = self.size() as u64;
        (addr + size - 1) & self.mask()
    }
}

impl fmt::Display for PageSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageSize::Size4K => write!(f, "4KB"),
            PageSize::Size2M => write!(f, "2MB"),
            PageSize::Size1G => write!(f, "1GB"),
        }
    }
}

// =============================================================================
// Physical Address
// =============================================================================

/// A physical memory address
///
/// This type ensures addresses are properly masked and provides
/// convenient conversion methods.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    /// Create a new physical address
    ///
    /// The address is masked to ensure only valid bits are set.
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr & PHYS_ADDR_MASK)
    }

    /// Create a physical address without masking
    ///
    /// # Safety
    ///
    /// The caller must ensure the address is valid.
    #[inline]
    pub const unsafe fn new_unchecked(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a null (zero) physical address
    #[inline]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a null address
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get the address as a usize (may truncate on 32-bit, but we're x86_64)
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Check if the address is aligned to the given page size
    #[inline]
    pub const fn is_aligned(self, page_size: PageSize) -> bool {
        page_size.is_aligned(self.0)
    }

    /// Check if the address is page-aligned (4KB)
    #[inline]
    pub const fn is_page_aligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    /// Align the address down to the given page size
    #[inline]
    pub const fn align_down(self, page_size: PageSize) -> Self {
        Self(page_size.align_down(self.0))
    }

    /// Align the address up to the given page size
    #[inline]
    pub const fn align_up(self, page_size: PageSize) -> Self {
        Self(page_size.align_up(self.0))
    }

    /// Get the frame containing this address
    #[inline]
    pub const fn containing_frame(self, page_size: PageSize) -> Frame {
        Frame::containing_address(self, page_size)
    }

    /// Add an offset to this address
    #[inline]
    pub const fn offset(self, offset: u64) -> Self {
        Self::new(self.0.wrapping_add(offset))
    }

    /// Calculate the page offset (offset within a 4KB page)
    #[inline]
    pub const fn page_offset(self) -> u16 {
        (self.0 & 0xFFF) as u16
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysicalAddress({:#x})", self.0)
    }
}

impl fmt::Display for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl fmt::LowerHex for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<u64> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl AddAssign<u64> for PhysicalAddress {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl Sub<u64> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl SubAssign<u64> for PhysicalAddress {
    #[inline]
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl Sub<PhysicalAddress> for PhysicalAddress {
    type Output = u64;

    #[inline]
    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        self.0 - rhs.0
    }
}

impl From<u64> for PhysicalAddress {
    #[inline]
    fn from(addr: u64) -> Self {
        Self::new(addr)
    }
}

impl From<PhysicalAddress> for u64 {
    #[inline]
    fn from(addr: PhysicalAddress) -> Self {
        addr.0
    }
}

// =============================================================================
// Virtual Address
// =============================================================================

/// A virtual (linear) memory address
///
/// This type provides methods for extracting page table indices
/// and checking canonical form.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct VirtualAddress(u64);

impl VirtualAddress {
    /// Create a new virtual address
    ///
    /// The address is canonicalized (sign-extended from bit 47 or 56).
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Create a virtual address, truncating to canonical form for 4-level paging
    #[inline]
    pub const fn new_truncate_4level(addr: u64) -> Self {
        let mask = 1u64 << 47;
        if addr & mask != 0 {
            Self(addr | 0xFFFF_0000_0000_0000)
        } else {
            Self(addr & 0x0000_FFFF_FFFF_FFFF)
        }
    }

    /// Create a null (zero) virtual address
    #[inline]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a null address
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the raw address value
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Get the address as a pointer
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Get the address as a mutable pointer
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Check if the address is canonical for 4-level paging
    #[inline]
    pub const fn is_canonical_4level(self) -> bool {
        let canonical = if self.0 & (1 << 47) != 0 {
            self.0 | 0xFFFF_0000_0000_0000
        } else {
            self.0 & 0x0000_FFFF_FFFF_FFFF
        };
        self.0 == canonical
    }

    /// Check if the address is canonical for 5-level paging
    #[inline]
    pub const fn is_canonical_5level(self) -> bool {
        let canonical = if self.0 & (1 << 56) != 0 {
            self.0 | 0xFE00_0000_0000_0000
        } else {
            self.0 & 0x01FF_FFFF_FFFF_FFFF
        };
        self.0 == canonical
    }

    /// Check if the address is in kernel space (higher half)
    #[inline]
    pub const fn is_kernel_space(self) -> bool {
        // For both 4-level and 5-level, kernel space has the high bit set
        self.0 & (1 << 63) != 0
    }

    /// Check if the address is in user space (lower half)
    #[inline]
    pub const fn is_user_space(self) -> bool {
        self.0 & (1 << 63) == 0
    }

    /// Get the page offset (bits 0-11)
    #[inline]
    pub const fn page_offset(self) -> u16 {
        (self.0 & 0xFFF) as u16
    }

    /// Get the PT index (bits 12-20)
    #[inline]
    pub const fn pt_index(self) -> u16 {
        ((self.0 >> 12) & 0x1FF) as u16
    }

    /// Get the PD index (bits 21-29)
    #[inline]
    pub const fn pd_index(self) -> u16 {
        ((self.0 >> 21) & 0x1FF) as u16
    }

    /// Get the PDPT index (bits 30-38)
    #[inline]
    pub const fn pdpt_index(self) -> u16 {
        ((self.0 >> 30) & 0x1FF) as u16
    }

    /// Get the PML4 index (bits 39-47)
    #[inline]
    pub const fn pml4_index(self) -> u16 {
        ((self.0 >> 39) & 0x1FF) as u16
    }

    /// Get the PML5 index (bits 48-56)
    #[inline]
    pub const fn pml5_index(self) -> u16 {
        ((self.0 >> 48) & 0x1FF) as u16
    }

    /// Get the index for a specific page table level
    ///
    /// Level 1 = PT, Level 2 = PD, Level 3 = PDPT, Level 4 = PML4, Level 5 = PML5
    #[inline]
    pub const fn table_index(self, level: u8) -> u16 {
        let shift = 12 + (level - 1) as u64 * 9;
        ((self.0 >> shift) & 0x1FF) as u16
    }

    /// Check if the address is aligned to the given page size
    #[inline]
    pub const fn is_aligned(self, page_size: PageSize) -> bool {
        page_size.is_aligned(self.0)
    }

    /// Align the address down to the given page size
    #[inline]
    pub const fn align_down(self, page_size: PageSize) -> Self {
        Self(page_size.align_down(self.0))
    }

    /// Align the address up to the given page size
    #[inline]
    pub const fn align_up(self, page_size: PageSize) -> Self {
        Self(page_size.align_up(self.0))
    }

    /// Get the page containing this address
    #[inline]
    pub const fn containing_page(self, page_size: PageSize) -> Page {
        Page::containing_address(self, page_size)
    }

    /// Add an offset to this address
    #[inline]
    pub const fn offset(self, offset: i64) -> Self {
        Self(self.0.wrapping_add(offset as u64))
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtualAddress({:#018x})", self.0)
    }
}

impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::LowerHex for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Add<u64> for VirtualAddress {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for VirtualAddress {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl Sub<u64> for VirtualAddress {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u64> for VirtualAddress {
    #[inline]
    fn sub_assign(&mut self, rhs: u64) {
        *self = *self - rhs;
    }
}

impl Sub<VirtualAddress> for VirtualAddress {
    type Output = i64;

    #[inline]
    fn sub(self, rhs: VirtualAddress) -> Self::Output {
        (self.0 as i64).wrapping_sub(rhs.0 as i64)
    }
}

impl From<u64> for VirtualAddress {
    #[inline]
    fn from(addr: u64) -> Self {
        Self::new(addr)
    }
}

impl From<VirtualAddress> for u64 {
    #[inline]
    fn from(addr: VirtualAddress) -> Self {
        addr.0
    }
}

impl<T> From<*const T> for VirtualAddress {
    #[inline]
    fn from(ptr: *const T) -> Self {
        Self::new(ptr as u64)
    }
}

impl<T> From<*mut T> for VirtualAddress {
    #[inline]
    fn from(ptr: *mut T) -> Self {
        Self::new(ptr as u64)
    }
}

// =============================================================================
// Frame (Physical Page)
// =============================================================================

/// A physical memory frame
///
/// Represents an aligned region of physical memory of a specific size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame {
    /// Starting physical address (aligned)
    start: PhysicalAddress,
    /// Page size
    size: PageSize,
}

impl Frame {
    /// Create a frame from an aligned physical address
    ///
    /// # Panics
    ///
    /// Panics if the address is not aligned to the page size.
    #[inline]
    pub fn new(addr: PhysicalAddress, size: PageSize) -> Self {
        assert!(addr.is_aligned(size), "Frame address not aligned");
        Self { start: addr, size }
    }

    /// Create a frame without checking alignment
    ///
    /// # Safety
    ///
    /// The address must be aligned to the page size.
    #[inline]
    pub const unsafe fn new_unchecked(addr: PhysicalAddress, size: PageSize) -> Self {
        Self { start: addr, size }
    }

    /// Get the frame containing the given address
    #[inline]
    pub const fn containing_address(addr: PhysicalAddress, size: PageSize) -> Self {
        Self {
            start: addr.align_down(size),
            size,
        }
    }

    /// Create a 4KB frame from a frame number
    #[inline]
    pub const fn from_number(number: u64) -> Self {
        Self {
            start: PhysicalAddress::new(number * PAGE_SIZE_4K as u64),
            size: PageSize::Size4K,
        }
    }

    /// Get the starting physical address
    #[inline]
    pub const fn start_address(self) -> PhysicalAddress {
        self.start
    }

    /// Get the ending physical address (exclusive)
    #[inline]
    pub const fn end_address(self) -> PhysicalAddress {
        PhysicalAddress::new(self.start.as_u64() + self.size.size() as u64)
    }

    /// Get the page size
    #[inline]
    pub const fn size(self) -> PageSize {
        self.size
    }

    /// Get the frame number (for 4KB frames)
    #[inline]
    pub const fn number(self) -> u64 {
        self.start.as_u64() / PAGE_SIZE_4K as u64
    }

    /// Check if this frame contains the given address
    #[inline]
    pub const fn contains(self, addr: PhysicalAddress) -> bool {
        addr.as_u64() >= self.start.as_u64()
            && addr.as_u64() < self.start.as_u64() + self.size.size() as u64
    }

    /// Get the next frame
    #[inline]
    pub const fn next(self) -> Self {
        Self {
            start: PhysicalAddress::new(self.start.as_u64() + self.size.size() as u64),
            size: self.size,
        }
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame({}, {})", self.start, self.size)
    }
}

// =============================================================================
// Page (Virtual Page)
// =============================================================================

/// A virtual memory page
///
/// Represents an aligned region of virtual memory of a specific size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Page {
    /// Starting virtual address (aligned)
    start: VirtualAddress,
    /// Page size
    size: PageSize,
}

impl Page {
    /// Create a page from an aligned virtual address
    ///
    /// # Panics
    ///
    /// Panics if the address is not aligned to the page size.
    #[inline]
    pub fn new(addr: VirtualAddress, size: PageSize) -> Self {
        assert!(addr.is_aligned(size), "Page address not aligned");
        Self { start: addr, size }
    }

    /// Create a page without checking alignment
    ///
    /// # Safety
    ///
    /// The address must be aligned to the page size.
    #[inline]
    pub const unsafe fn new_unchecked(addr: VirtualAddress, size: PageSize) -> Self {
        Self { start: addr, size }
    }

    /// Get the page containing the given address
    #[inline]
    pub const fn containing_address(addr: VirtualAddress, size: PageSize) -> Self {
        Self {
            start: addr.align_down(size),
            size,
        }
    }

    /// Create a 4KB page from a page number
    #[inline]
    pub const fn from_number(number: u64) -> Self {
        Self {
            start: VirtualAddress::new(number * PAGE_SIZE_4K as u64),
            size: PageSize::Size4K,
        }
    }

    /// Get the starting virtual address
    #[inline]
    pub const fn start_address(self) -> VirtualAddress {
        self.start
    }

    /// Get the ending virtual address (exclusive)
    #[inline]
    pub const fn end_address(self) -> VirtualAddress {
        VirtualAddress::new(self.start.as_u64() + self.size.size() as u64)
    }

    /// Get the page size
    #[inline]
    pub const fn size(self) -> PageSize {
        self.size
    }

    /// Get the page number (for 4KB pages)
    #[inline]
    pub const fn number(self) -> u64 {
        self.start.as_u64() / PAGE_SIZE_4K as u64
    }

    /// Check if this page contains the given address
    #[inline]
    pub const fn contains(self, addr: VirtualAddress) -> bool {
        addr.as_u64() >= self.start.as_u64()
            && addr.as_u64() < self.start.as_u64() + self.size.size() as u64
    }

    /// Get the next page
    #[inline]
    pub const fn next(self) -> Self {
        Self {
            start: VirtualAddress::new(self.start.as_u64() + self.size.size() as u64),
            size: self.size,
        }
    }

    /// Get the index into the page table at the given level
    #[inline]
    pub const fn table_index(self, level: u8) -> u16 {
        self.start.table_index(level)
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({}, {})", self.start, self.size)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_size_alignment() {
        assert!(PageSize::Size4K.is_aligned(0x1000));
        assert!(!PageSize::Size4K.is_aligned(0x1001));
        assert!(PageSize::Size2M.is_aligned(0x200000));
        assert!(PageSize::Size1G.is_aligned(0x40000000));
    }

    #[test]
    fn test_virtual_address_indices() {
        let addr = VirtualAddress::new(0xFFFF_8000_0123_4567);
        assert_eq!(addr.page_offset(), 0x567);
        assert_eq!(addr.pt_index(), 0x234 >> 3);
    }

    #[test]
    fn test_canonical_addresses() {
        let kernel = VirtualAddress::new(0xFFFF_8000_0000_0000);
        let user = VirtualAddress::new(0x0000_7FFF_FFFF_FFFF);

        assert!(kernel.is_canonical_4level());
        assert!(user.is_canonical_4level());
        assert!(kernel.is_kernel_space());
        assert!(user.is_user_space());
    }
}
