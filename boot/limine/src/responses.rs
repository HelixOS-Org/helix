//! # Response Abstractions
//!
//! This module provides safe, high-level response types that wrap the
//! raw Limine protocol responses with proper Rust semantics.
//!
//! ## Design
//!
//! Each response type provides:
//! - Type-safe access to response data
//! - Iterator implementations for collections
//! - Builder patterns for complex types
//! - Conversion to/from raw types

use core::marker::PhantomData;

use crate::memory::{MemoryRegion, MemoryRegionKind, PhysAddr, VirtAddr};
use crate::protocol::raw;

// =============================================================================
// Response Traits
// =============================================================================

/// Trait for response types
pub trait Response {
    /// The raw response type
    type Raw;

    /// Create from raw response
    ///
    /// # Safety
    ///
    /// Raw response must be valid.
    unsafe fn from_raw(raw: *const Self::Raw) -> Option<Self>
    where
        Self: Sized;

    /// Check if response is valid
    fn is_valid(&self) -> bool;
}

/// Trait for iterable responses
pub trait IterableResponse<T>: Response {
    /// Get number of items
    fn count(&self) -> usize;

    /// Get item by index
    fn get(&self, index: usize) -> Option<T>;

    /// Iterate over items
    fn iter(&self) -> ResponseIterator<'_, Self, T>
    where
        Self: Sized,
    {
        ResponseIterator {
            response: self,
            index: 0,
            _marker: PhantomData,
        }
    }
}

/// Generic iterator for responses
pub struct ResponseIterator<'a, R: IterableResponse<T>, T> {
    response: &'a R,
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, R: IterableResponse<T>, T> Iterator for ResponseIterator<'a, R, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.response.get(self.index)?;
        self.index += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.response.count().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a, R: IterableResponse<T>, T> ExactSizeIterator for ResponseIterator<'a, R, T> {}

// =============================================================================
// Bootloader Info Response
// =============================================================================

/// Safe wrapper for bootloader info response
#[derive(Debug)]
pub struct BootloaderInfo {
    name: &'static str,
    version: &'static str,
}

impl BootloaderInfo {
    /// Create from raw pointers
    ///
    /// # Safety
    ///
    /// Pointers must be valid null-terminated strings.
    pub unsafe fn from_raw_ptrs(name: *const u8, version: *const u8) -> Self {
        // SAFETY: Caller guarantees pointers are valid
        unsafe {
            Self {
                name: Self::str_from_ptr(name),
                version: Self::str_from_ptr(version),
            }
        }
    }

    unsafe fn str_from_ptr(ptr: *const u8) -> &'static str {
        if ptr.is_null() {
            return "";
        }
        // SAFETY: Caller guarantees pointer is valid
        unsafe {
            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let bytes = core::slice::from_raw_parts(ptr, len);
            core::str::from_utf8_unchecked(bytes)
        }
    }

    /// Get bootloader name
    pub fn name(&self) -> &str {
        self.name
    }

    /// Get bootloader version
    pub fn version(&self) -> &str {
        self.version
    }

    /// Check if Limine bootloader
    pub fn is_limine(&self) -> bool {
        self.name.contains("imine") || self.name.contains("IMINE")
    }

    /// Parse version components
    pub fn version_parts(&self) -> Option<(u32, u32, u32)> {
        let parts: [&str; 3] = {
            let mut iter = self.version.split('.');
            [
                iter.next()?,
                iter.next().unwrap_or("0"),
                iter.next().unwrap_or("0"),
            ]
        };

        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    }
}

// =============================================================================
// Memory Map Response
// =============================================================================

/// Memory map response wrapper
pub struct MemoryMap {
    entries: *const *const raw::RawMemmapEntry,
    entry_count: u64,
}

impl MemoryMap {
    /// Create from raw response
    ///
    /// # Safety
    ///
    /// Pointers must be valid.
    pub unsafe fn from_raw(entries: *const *const raw::RawMemmapEntry, count: u64) -> Self {
        Self {
            entries,
            entry_count: count,
        }
    }

    /// Get entry count
    pub fn count(&self) -> usize {
        self.entry_count as usize
    }

    /// Get entry by index
    pub fn get(&self, index: usize) -> Option<MemoryRegion> {
        if index >= self.count() {
            return None;
        }

        unsafe {
            let entry = *self.entries.add(index);
            if entry.is_null() {
                return None;
            }

            let raw = &*entry;
            Some(MemoryRegion::new(
                PhysAddr::new(raw.base),
                raw.length,
                MemoryRegionKind::from_limine(raw.entry_type),
            ))
        }
    }

    /// Iterate over entries
    pub fn iter(&self) -> MemoryMapIterator<'_> {
        MemoryMapIterator {
            map: self,
            index: 0,
        }
    }

    /// Get total usable memory
    pub fn total_usable(&self) -> u64 {
        self.iter()
            .filter(|r| r.is_usable())
            .map(|r| r.size())
            .sum()
    }

    /// Get total memory
    pub fn total(&self) -> u64 {
        self.iter().map(|r| r.size()).sum()
    }

    /// Find region containing address
    pub fn find_region(&self, addr: PhysAddr) -> Option<MemoryRegion> {
        self.iter().find(|r| r.contains(addr))
    }

    /// Find largest usable region
    pub fn largest_usable(&self) -> Option<MemoryRegion> {
        self.iter()
            .filter(|r| r.is_usable())
            .max_by_key(|r| r.size())
    }

    /// Get regions of a specific kind
    pub fn regions_of_kind(
        &self,
        target_kind: MemoryRegionKind,
    ) -> impl Iterator<Item = MemoryRegion> + '_ {
        self.iter().filter(move |r| r.region_kind() == target_kind)
    }
}

/// Iterator over memory map entries
pub struct MemoryMapIterator<'a> {
    map: &'a MemoryMap,
    index: usize,
}

impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let region = self.map.get(self.index)?;
        self.index += 1;
        Some(region)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.map.count().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for MemoryMapIterator<'a> {}

// =============================================================================
// HHDM Response
// =============================================================================

/// Higher-half direct map response
#[derive(Debug, Clone, Copy)]
pub struct HhdmInfo {
    offset: u64,
}

impl HhdmInfo {
    /// Create from offset
    pub const fn new(offset: u64) -> Self {
        Self { offset }
    }

    /// Get HHDM offset
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Convert physical address to virtual
    pub const fn phys_to_virt(&self, phys: PhysAddr) -> VirtAddr {
        VirtAddr::new(phys.as_u64().wrapping_add(self.offset))
    }

    /// Convert virtual address to physical
    pub const fn virt_to_phys(&self, virt: VirtAddr) -> Option<PhysAddr> {
        if virt.as_u64() >= self.offset {
            Some(PhysAddr::new(virt.as_u64() - self.offset))
        } else {
            None
        }
    }

    /// Check if address is in HHDM region
    pub const fn contains(&self, virt: VirtAddr) -> bool {
        virt.as_u64() >= self.offset
    }
}

// =============================================================================
// Kernel Address Response
// =============================================================================

/// Kernel address information
#[derive(Debug, Clone, Copy)]
pub struct KernelAddress {
    physical_base: PhysAddr,
    virtual_base: VirtAddr,
}

impl KernelAddress {
    /// Create from addresses
    pub const fn new(physical: PhysAddr, virtual_addr: VirtAddr) -> Self {
        Self {
            physical_base: physical,
            virtual_base: virtual_addr,
        }
    }

    /// Get physical base address
    pub const fn physical_base(&self) -> PhysAddr {
        self.physical_base
    }

    /// Get virtual base address
    pub const fn virtual_base(&self) -> VirtAddr {
        self.virtual_base
    }

    /// Convert kernel virtual address to physical
    pub const fn virt_to_phys(&self, virt: VirtAddr) -> Option<PhysAddr> {
        if virt.as_u64() >= self.virtual_base.as_u64() {
            let offset = virt.as_u64() - self.virtual_base.as_u64();
            Some(PhysAddr::new(self.physical_base.as_u64() + offset))
        } else {
            None
        }
    }

    /// Convert kernel physical address to virtual
    pub const fn phys_to_virt(&self, phys: PhysAddr) -> Option<VirtAddr> {
        if phys.as_u64() >= self.physical_base.as_u64() {
            let offset = phys.as_u64() - self.physical_base.as_u64();
            Some(VirtAddr::new(self.virtual_base.as_u64() + offset))
        } else {
            None
        }
    }
}

// =============================================================================
// SMP Response
// =============================================================================

/// CPU information
#[derive(Debug, Clone, Copy)]
pub struct CpuInfo {
    /// Processor ID
    pub processor_id: u32,
    /// LAPIC ID
    pub lapic_id: u32,
    /// Is BSP
    pub is_bsp: bool,
    /// Raw CPU info pointer
    raw: *const raw::RawSmpInfo,
}

impl CpuInfo {
    /// Create from raw
    unsafe fn from_raw(raw: *const raw::RawSmpInfo, bsp_lapic_id: u64) -> Self {
        // SAFETY: Caller guarantees pointer is valid
        unsafe {
            let info = &*raw;
            Self {
                processor_id: info.processor_id,
                lapic_id: info.lapic_id,
                is_bsp: info.lapic_id as u64 == bsp_lapic_id,
                raw,
            }
        }
    }

    /// Start this CPU
    pub fn start(&self, entry: u64, arg: u64) {
        // SAFETY: We hold a valid pointer to the SmpInfo
        // Write extra_argument first, then goto_address to trigger the CPU
        unsafe {
            let info_ptr = self.raw as *mut raw::RawSmpInfo;
            // Write the argument first using volatile write
            core::ptr::write_volatile(core::ptr::addr_of_mut!((*info_ptr).extra_argument), arg);
            // Write goto_address last (atomic) - this triggers the CPU to start
            (*info_ptr)
                .goto_address
                .store(entry, core::sync::atomic::Ordering::Release);
        }
    }
}

/// SMP response wrapper
pub struct SmpInfo {
    flags: u32,
    bsp_lapic_id: u64,
    cpus: *const *const raw::RawSmpInfo,
    cpu_count: u64,
}

impl SmpInfo {
    /// Create from raw
    ///
    /// # Safety
    ///
    /// Pointers must be valid.
    pub unsafe fn from_raw(
        flags: u32,
        bsp_lapic_id: u64,
        cpus: *const *const raw::RawSmpInfo,
        cpu_count: u64,
    ) -> Self {
        Self {
            flags,
            bsp_lapic_id,
            cpus,
            cpu_count,
        }
    }

    /// Get flags
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Check if x2APIC mode
    pub fn is_x2apic(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Get BSP LAPIC ID
    pub fn bsp_lapic_id(&self) -> u64 {
        self.bsp_lapic_id
    }

    /// Get CPU count
    pub fn cpu_count(&self) -> usize {
        self.cpu_count as usize
    }

    /// Get CPU by index
    pub fn get(&self, index: usize) -> Option<CpuInfo> {
        if index >= self.cpu_count() {
            return None;
        }

        unsafe {
            let cpu = *self.cpus.add(index);
            if cpu.is_null() {
                return None;
            }
            Some(CpuInfo::from_raw(cpu, self.bsp_lapic_id))
        }
    }

    /// Iterate over CPUs
    pub fn iter(&self) -> SmpIterator<'_> {
        SmpIterator {
            info: self,
            index: 0,
        }
    }

    /// Get BSP info
    pub fn bsp(&self) -> Option<CpuInfo> {
        self.iter().find(|c| c.is_bsp)
    }

    /// Iterate over APs (non-BSP CPUs)
    pub fn aps(&self) -> impl Iterator<Item = CpuInfo> + '_ {
        self.iter().filter(|c| !c.is_bsp)
    }
}

/// Iterator over CPUs
pub struct SmpIterator<'a> {
    info: &'a SmpInfo,
    index: usize,
}

impl<'a> Iterator for SmpIterator<'a> {
    type Item = CpuInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let cpu = self.info.get(self.index)?;
        self.index += 1;
        Some(cpu)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.info.cpu_count().saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for SmpIterator<'a> {}

// =============================================================================
// Framebuffer Response
// =============================================================================

/// Framebuffer information
#[derive(Debug)]
pub struct FramebufferInfo {
    /// Framebuffer address
    pub address: VirtAddr,
    /// Width in pixels
    pub width: u64,
    /// Height in pixels
    pub height: u64,
    /// Pitch (bytes per row)
    pub pitch: u64,
    /// Bits per pixel
    pub bpp: u16,
    /// Memory model
    pub memory_model: u8,
    /// Red mask info
    pub red: ColorMask,
    /// Green mask info
    pub green: ColorMask,
    /// Blue mask info
    pub blue: ColorMask,
}

/// Color channel mask
#[derive(Debug, Clone, Copy)]
pub struct ColorMask {
    pub size: u8,
    pub shift: u8,
}

impl ColorMask {
    /// Create from size and shift
    pub const fn new(size: u8, shift: u8) -> Self {
        Self { size, shift }
    }

    /// Get mask value
    pub const fn mask(&self) -> u32 {
        ((1u32 << self.size) - 1) << self.shift
    }

    /// Extract value from pixel
    pub const fn extract(&self, pixel: u32) -> u8 {
        ((pixel >> self.shift) & ((1 << self.size) - 1)) as u8
    }

    /// Pack value into pixel
    pub const fn pack(&self, value: u8) -> u32 {
        ((value as u32) & ((1 << self.size) - 1)) << self.shift
    }
}

impl FramebufferInfo {
    /// Get size in bytes
    pub fn size(&self) -> u64 {
        self.pitch * self.height
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        (self.bpp as usize + 7) / 8
    }

    /// Get pixel offset
    pub fn pixel_offset(&self, x: usize, y: usize) -> usize {
        y * self.pitch as usize + x * self.bytes_per_pixel()
    }

    /// Pack RGB color
    pub fn pack_rgb(&self, r: u8, g: u8, b: u8) -> u32 {
        self.red.pack(r) | self.green.pack(g) | self.blue.pack(b)
    }

    /// Unpack pixel to RGB
    pub fn unpack_rgb(&self, pixel: u32) -> (u8, u8, u8) {
        (
            self.red.extract(pixel),
            self.green.extract(pixel),
            self.blue.extract(pixel),
        )
    }
}

// =============================================================================
// RSDP Response
// =============================================================================

/// RSDP information
#[derive(Debug, Clone, Copy)]
pub struct RsdpInfo {
    /// RSDP address
    pub address: PhysAddr,
    /// ACPI revision (0 = 1.0, 2+ = 2.0+)
    pub revision: u8,
}

impl RsdpInfo {
    /// Create from address
    pub const fn new(address: PhysAddr, revision: u8) -> Self {
        Self { address, revision }
    }

    /// Check if ACPI 2.0+
    pub const fn is_acpi_2(&self) -> bool {
        self.revision >= 2
    }

    /// Get RSDT/XSDT address
    ///
    /// # Safety
    ///
    /// RSDP must be valid.
    pub unsafe fn get_sdt_address(&self) -> u64 {
        // SAFETY: Caller guarantees RSDP is valid
        unsafe {
            if self.is_acpi_2() {
                let xsdp = self.address.as_u64() as *const crate::firmware::Xsdp;
                (*xsdp).xsdt_address
            } else {
                let rsdp = self.address.as_u64() as *const crate::firmware::Rsdp;
                (*rsdp).rsdt_address as u64
            }
        }
    }
}

// =============================================================================
// Boot Time Response
// =============================================================================

/// Boot time information
#[derive(Debug, Clone, Copy)]
pub struct BootTime {
    /// Unix timestamp
    pub unix_time: i64,
}

impl BootTime {
    /// Create from timestamp
    pub const fn new(unix_time: i64) -> Self {
        Self { unix_time }
    }

    /// Get Unix timestamp
    pub const fn unix_timestamp(&self) -> i64 {
        self.unix_time
    }

    /// Calculate approximate date components
    pub fn date(&self) -> (u32, u8, u8) {
        // Simple calculation - not accounting for leap seconds
        let days_since_epoch = self.unix_time / 86400;
        let mut year = 1970u32;
        let mut remaining_days = days_since_epoch as i32;

        loop {
            let days_in_year = if is_leap_year(year) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        let mut month = 1u8;
        let days_in_months = if is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        for &days in &days_in_months {
            if remaining_days < days {
                break;
            }
            remaining_days -= days;
            month += 1;
        }

        (year, month, (remaining_days + 1) as u8)
    }
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_mask() {
        let red = ColorMask::new(8, 16);
        assert_eq!(red.mask(), 0x00FF0000);
        assert_eq!(red.extract(0x00FF0000), 255);
        assert_eq!(red.pack(255), 0x00FF0000);
    }

    #[test]
    fn test_hhdm_info() {
        let hhdm = HhdmInfo::new(0xFFFF800000000000);
        let phys = PhysAddr::new(0x1000);
        let virt = hhdm.phys_to_virt(phys);
        assert_eq!(virt.as_u64(), 0xFFFF800000001000);

        let back = hhdm.virt_to_phys(virt);
        assert_eq!(back, Some(phys));
    }

    #[test]
    fn test_kernel_address() {
        let ka = KernelAddress::new(PhysAddr::new(0x100000), VirtAddr::new(0xFFFFFFFF80000000));

        let virt = VirtAddr::new(0xFFFFFFFF80001000);
        let phys = ka.virt_to_phys(virt);
        assert_eq!(phys, Some(PhysAddr::new(0x101000)));
    }

    #[test]
    fn test_boot_time() {
        // January 1, 2024 00:00:00 UTC
        let time = BootTime::new(1704067200);
        let (year, month, day) = time.date();
        assert_eq!(year, 2024);
        assert_eq!(month, 1);
        assert_eq!(day, 1);
    }
}
