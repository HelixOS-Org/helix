//! Virtual Memory Management
//!
//! Advanced virtual memory management for UEFI boot environment.
//! Provides virtual address space management, mapping, and protection.

use crate::error::{Error, Result};
use crate::memory::paging::{PageFlags, PageTableManager};
use crate::memory::regions::{MemoryRegion, RegionType};
use crate::raw::types::{PhysicalAddress, VirtualAddress};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// VIRTUAL MEMORY CONSTANTS
// =============================================================================

/// Page size (`4 KiB`)
pub const PAGE_SIZE: u64 = 0x1000;

/// Large page size (`2 MiB`)
pub const LARGE_PAGE_SIZE: u64 = 0x0020_0000;

/// Huge page size (`1 GiB`)
pub const HUGE_PAGE_SIZE: u64 = 0x4000_0000;

/// Canonical address mask for `x86_64`
pub const CANONICAL_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

/// Higher half start
pub const HIGHER_HALF_START: u64 = 0xFFFF_8000_0000_0000;

// =============================================================================
// VIRTUAL ADDRESS SPACE
// =============================================================================

/// Virtual address space manager
pub struct VirtualAddressSpace {
    /// Address space ID
    id: u64,
    /// Allocated regions
    regions: BTreeMap<VirtualAddress, VirtualRegion>,
    /// Free regions
    free_regions: Vec<FreeRegion>,
    /// Page table manager
    page_tables: Option<PageTableManager>,
    /// Space limits
    limits: AddressSpaceLimits,
    /// Statistics
    stats: VirtualMemoryStats,
}

impl VirtualAddressSpace {
    /// Create new address space
    pub fn new(id: u64) -> Self {
        Self {
            id,
            regions: BTreeMap::new(),
            free_regions: Vec::new(),
            page_tables: None,
            limits: AddressSpaceLimits::default(),
            stats: VirtualMemoryStats::default(),
        }
    }

    /// Create with limits
    pub fn with_limits(id: u64, limits: AddressSpaceLimits) -> Self {
        let mut space = Self::new(id);
        space.limits = limits;
        space.initialize_free_regions();
        space
    }

    /// Initialize free regions from limits
    fn initialize_free_regions(&mut self) {
        self.free_regions.clear();

        // User space free region
        if self.limits.user_end > self.limits.user_start {
            self.free_regions.push(FreeRegion {
                base: self.limits.user_start,
                size: self.limits.user_end - self.limits.user_start,
            });
        }

        // Kernel space free region
        if self.limits.kernel_end > self.limits.kernel_start {
            self.free_regions.push(FreeRegion {
                base: self.limits.kernel_start,
                size: self.limits.kernel_end - self.limits.kernel_start,
            });
        }
    }

    /// Get address space ID
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Allocate virtual region
    pub fn allocate(&mut self, size: u64, flags: VirtualRegionFlags) -> Result<VirtualAddress> {
        self.allocate_aligned(size, PAGE_SIZE, flags)
    }

    /// Allocate virtual region with alignment
    pub fn allocate_aligned(
        &mut self,
        size: u64,
        alignment: u64,
        flags: VirtualRegionFlags,
    ) -> Result<VirtualAddress> {
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Find suitable free region
        let mut found_idx = None;
        let mut found_addr = 0;

        for (idx, free) in self.free_regions.iter().enumerate() {
            let aligned_base = (free.base.0 + alignment - 1) & !(alignment - 1);
            let offset = aligned_base - free.base.0;

            if free.size >= offset + aligned_size {
                found_idx = Some(idx);
                found_addr = aligned_base;
                break;
            }
        }

        let idx = found_idx.ok_or(Error::OutOfMemory)?;

        // Split free region
        let free = &mut self.free_regions[idx];
        let aligned_base = found_addr;
        let offset = aligned_base - free.base.0;

        if offset > 0 {
            // Keep prefix
            let new_free = FreeRegion {
                base: VirtualAddress(aligned_base + aligned_size),
                size: free.size - offset - aligned_size,
            };
            free.size = offset;
            if new_free.size > 0 {
                self.free_regions.push(new_free);
            }
        } else {
            // Update in place
            free.base = VirtualAddress(aligned_base + aligned_size);
            free.size -= aligned_size;
            if free.size == 0 {
                self.free_regions.remove(idx);
            }
        }

        // Create virtual region
        let region = VirtualRegion {
            base: VirtualAddress(aligned_base),
            size: aligned_size,
            flags,
            name: String::new(),
            physical_backing: None,
        };

        self.regions.insert(VirtualAddress(aligned_base), region);
        self.stats.allocated_bytes += aligned_size;
        self.stats.region_count += 1;

        Ok(VirtualAddress(aligned_base))
    }

    /// Allocate at specific address
    pub fn allocate_at(
        &mut self,
        address: VirtualAddress,
        size: u64,
        flags: VirtualRegionFlags,
    ) -> Result<()> {
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let end = address.0 + aligned_size;

        // Check not already allocated
        if self.find_region(address).is_some() {
            return Err(Error::AlreadyExists);
        }

        // Find and split free region
        let mut found_idx = None;

        for (idx, free) in self.free_regions.iter().enumerate() {
            if free.base.0 <= address.0 && free.base.0 + free.size >= end {
                found_idx = Some(idx);
                break;
            }
        }

        let idx = found_idx.ok_or(Error::NotFound)?;

        // Split free region
        let free = self.free_regions.remove(idx);

        // Prefix free region
        if free.base.0 < address.0 {
            self.free_regions.push(FreeRegion {
                base: free.base,
                size: address.0 - free.base.0,
            });
        }

        // Suffix free region
        if free.base.0 + free.size > end {
            self.free_regions.push(FreeRegion {
                base: VirtualAddress(end),
                size: (free.base.0 + free.size) - end,
            });
        }

        // Create virtual region
        let region = VirtualRegion {
            base: address,
            size: aligned_size,
            flags,
            name: String::new(),
            physical_backing: None,
        };

        self.regions.insert(address, region);
        self.stats.allocated_bytes += aligned_size;
        self.stats.region_count += 1;

        Ok(())
    }

    /// Free virtual region
    pub fn free(&mut self, address: VirtualAddress) -> Result<()> {
        let region = self.regions.remove(&address).ok_or(Error::NotFound)?;

        self.stats.allocated_bytes -= region.size;
        self.stats.region_count -= 1;

        // Add back to free list
        self.free_regions.push(FreeRegion {
            base: region.base,
            size: region.size,
        });

        // Merge adjacent free regions
        self.merge_free_regions();

        Ok(())
    }

    /// Merge adjacent free regions
    fn merge_free_regions(&mut self) {
        if self.free_regions.len() < 2 {
            return;
        }

        self.free_regions.sort_by_key(|r| r.base.0);

        let mut merged = Vec::new();
        let mut current = self.free_regions[0].clone();

        for region in &self.free_regions[1..] {
            if current.base.0 + current.size == region.base.0 {
                current.size += region.size;
            } else {
                merged.push(current);
                current = region.clone();
            }
        }
        merged.push(current);

        self.free_regions = merged;
    }

    /// Find region containing address
    pub fn find_region(&self, address: VirtualAddress) -> Option<&VirtualRegion> {
        for (base, region) in &self.regions {
            if base.0 <= address.0 && address.0 < base.0 + region.size {
                return Some(region);
            }
        }
        None
    }

    /// Find region containing address (mutable)
    pub fn find_region_mut(&mut self, address: VirtualAddress) -> Option<&mut VirtualRegion> {
        for (base, region) in &mut self.regions {
            if base.0 <= address.0 && address.0 < base.0 + region.size {
                return Some(region);
            }
        }
        None
    }

    /// Map region to physical memory
    pub fn map_physical(
        &mut self,
        virtual_addr: VirtualAddress,
        physical_addr: PhysicalAddress,
        size: u64,
        flags: VirtualRegionFlags,
    ) -> Result<()> {
        // Allocate virtual region if not exists
        if self.find_region(virtual_addr).is_none() {
            self.allocate_at(virtual_addr, size, flags)?;
        }

        // Update backing
        if let Some(region) = self.find_region_mut(virtual_addr) {
            region.physical_backing = Some(physical_addr);
        }

        // Map in page tables
        if let Some(ref mut pt) = self.page_tables {
            let page_flags = flags.to_page_flags();
            unsafe {
                pt.map_range(virtual_addr, physical_addr, size, page_flags)?;
            }
        }

        self.stats.mapped_bytes += size;

        Ok(())
    }

    /// Unmap region
    pub fn unmap(&mut self, virtual_addr: VirtualAddress) -> Result<()> {
        let region = self.find_region(virtual_addr).ok_or(Error::NotFound)?;
        let size = region.size;

        // Unmap from page tables
        if let Some(ref mut pt) = self.page_tables {
            unsafe {
                pt.unmap_range(virtual_addr, size)?;
            }
        }

        self.stats.mapped_bytes -= size;

        // Update backing
        if let Some(region) = self.find_region_mut(virtual_addr) {
            region.physical_backing = None;
        }

        Ok(())
    }

    /// Change protection
    pub fn protect(&mut self, address: VirtualAddress, flags: VirtualRegionFlags) -> Result<()> {
        let region = self.find_region_mut(address).ok_or(Error::NotFound)?;

        region.flags = flags;

        // Update page tables
        if let Some(ref mut _pt) = self.page_tables {
            // Would need to update PTEs
        }

        Ok(())
    }

    /// Set page table manager
    pub fn set_page_tables(&mut self, pt: PageTableManager) {
        self.page_tables = Some(pt);
    }

    /// Get statistics
    pub const fn stats(&self) -> &VirtualMemoryStats {
        &self.stats
    }

    /// Get all regions
    pub fn regions(&self) -> impl Iterator<Item = &VirtualRegion> {
        self.regions.values()
    }

    /// Get region count
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

// =============================================================================
// VIRTUAL REGION
// =============================================================================

/// Virtual memory region
#[derive(Debug, Clone)]
pub struct VirtualRegion {
    /// Base virtual address
    pub base: VirtualAddress,
    /// Size in bytes
    pub size: u64,
    /// Protection flags
    pub flags: VirtualRegionFlags,
    /// Region name
    pub name: String,
    /// Physical backing address
    pub physical_backing: Option<PhysicalAddress>,
}

impl VirtualRegion {
    /// Get end address
    pub fn end(&self) -> VirtualAddress {
        self.base + self.size
    }

    /// Check if contains address
    pub fn contains(&self, address: VirtualAddress) -> bool {
        address >= self.base && address < self.end()
    }

    /// Get size in pages
    pub const fn pages(&self) -> u64 {
        self.size.div_ceil(PAGE_SIZE)
    }

    /// Check if mapped
    pub const fn is_mapped(&self) -> bool {
        self.physical_backing.is_some()
    }
}

// =============================================================================
// VIRTUAL REGION FLAGS
// =============================================================================

/// Virtual region protection flags
#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualRegionFlags {
    /// Readable
    pub read: bool,
    /// Writable
    pub write: bool,
    /// Executable
    pub execute: bool,
    /// User accessible
    pub user: bool,
    /// Global (not flushed on CR3 change)
    pub global: bool,
    /// Cacheable
    pub cached: bool,
    /// Write through
    pub write_through: bool,
    /// No cache
    pub no_cache: bool,
}

impl VirtualRegionFlags {
    /// Read only
    pub fn read_only() -> Self {
        Self {
            read: true,
            cached: true,
            ..Default::default()
        }
    }

    /// Read/write
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            cached: true,
            ..Default::default()
        }
    }

    /// Executable read only
    pub fn exec_read_only() -> Self {
        Self {
            read: true,
            execute: true,
            cached: true,
            ..Default::default()
        }
    }

    /// User read/write
    pub fn user_read_write() -> Self {
        Self {
            read: true,
            write: true,
            user: true,
            cached: true,
            ..Default::default()
        }
    }

    /// Device memory (uncached)
    pub fn device() -> Self {
        Self {
            read: true,
            write: true,
            no_cache: true,
            ..Default::default()
        }
    }

    /// Convert to page flags
    pub fn to_page_flags(&self) -> PageFlags {
        let mut flags = PageFlags::empty();

        if self.write {
            flags.writable = true;
        }
        if self.user {
            flags.user = true;
        }
        if !self.execute {
            flags.no_execute = true;
        }
        if self.global {
            flags.global = true;
        }
        if self.write_through {
            flags.write_through = true;
        }
        if self.no_cache {
            flags.cache_disable = true;
        }

        flags
    }
}

// =============================================================================
// FREE REGION
// =============================================================================

/// Free virtual memory region
#[derive(Debug, Clone)]
struct FreeRegion {
    /// Base address
    base: VirtualAddress,
    /// Size
    size: u64,
}

// =============================================================================
// ADDRESS SPACE LIMITS
// =============================================================================

/// Address space limits
#[derive(Debug, Clone)]
pub struct AddressSpaceLimits {
    /// User space start
    pub user_start: VirtualAddress,
    /// User space end
    pub user_end: VirtualAddress,
    /// Kernel space start
    pub kernel_start: VirtualAddress,
    /// Kernel space end
    pub kernel_end: VirtualAddress,
}

impl Default for AddressSpaceLimits {
    fn default() -> Self {
        Self::x86_64_default()
    }
}

impl AddressSpaceLimits {
    /// Default `x86_64` layout
    pub const fn x86_64_default() -> Self {
        Self {
            user_start: VirtualAddress(0x0000_0000_0001_0000),
            user_end: VirtualAddress(0x0000_7FFF_FFFF_FFFF),
            kernel_start: VirtualAddress(0xFFFF_8000_0000_0000),
            kernel_end: VirtualAddress(0xFFFF_FFFF_FFFF_FFFF),
        }
    }

    /// `AArch64` default layout
    pub const fn aarch64_default() -> Self {
        Self {
            user_start: VirtualAddress(0x0000_0000_0001_0000),
            user_end: VirtualAddress(0x0000_FFFF_FFFF_FFFF),
            kernel_start: VirtualAddress(0xFFFF_0000_0000_0000),
            kernel_end: VirtualAddress(0xFFFF_FFFF_FFFF_FFFF),
        }
    }
}

// =============================================================================
// VIRTUAL MEMORY STATS
// =============================================================================

/// Virtual memory statistics
#[derive(Debug, Clone, Default)]
pub struct VirtualMemoryStats {
    /// Total allocated bytes
    pub allocated_bytes: u64,
    /// Total mapped bytes
    pub mapped_bytes: u64,
    /// Region count
    pub region_count: u64,
    /// Page table memory
    pub page_table_bytes: u64,
}

// =============================================================================
// IDENTITY MAPPING
// =============================================================================

/// Identity mapping helper
pub struct IdentityMapper {
    /// Page table manager
    page_tables: PageTableManager,
    /// Mapped regions
    mappings: Vec<IdentityMapping>,
}

impl IdentityMapper {
    /// Create new identity mapper
    pub fn new() -> Self {
        Self {
            page_tables: PageTableManager::new(),
            mappings: Vec::new(),
        }
    }

    /// Map physical range as identity
    pub fn map_range(
        &mut self,
        physical_addr: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Result<()> {
        // Map physical == virtual
        unsafe {
            self.page_tables.map_range(
                VirtualAddress(physical_addr.0),
                physical_addr,
                size,
                flags,
            )?;
        }

        self.mappings.push(IdentityMapping {
            address: physical_addr,
            size,
            flags,
        });

        Ok(())
    }

    /// Map memory region as identity
    pub fn map_region(&mut self, region: &MemoryRegion) -> Result<()> {
        let flags = match region.region_type {
            RegionType::AcpiReclaimable => PageFlags::kernel_rodata(),
            RegionType::Mmio => PageFlags::device(),
            RegionType::Kernel => PageFlags::kernel_code(),
            _ => PageFlags::kernel_data(),
        };

        self.map_range(region.base, region.size, flags)
    }

    /// Get page tables
    pub const fn page_tables(&self) -> &PageTableManager {
        &self.page_tables
    }

    /// Get page tables (mutable)
    pub fn page_tables_mut(&mut self) -> &mut PageTableManager {
        &mut self.page_tables
    }

    /// Get mappings
    pub fn mappings(&self) -> &[IdentityMapping] {
        &self.mappings
    }
}

impl Default for IdentityMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// Identity mapping entry
#[derive(Debug, Clone)]
pub struct IdentityMapping {
    /// Physical/virtual address
    pub address: PhysicalAddress,
    /// Size
    pub size: u64,
    /// Flags
    pub flags: PageFlags,
}

// =============================================================================
// HIGHER HALF MAPPER
// =============================================================================

/// Higher half kernel mapper
pub struct HigherHalfMapper {
    /// Base virtual address
    base: VirtualAddress,
    /// Current offset
    offset: u64,
    /// Page table manager
    page_tables: PageTableManager,
    /// Mappings
    mappings: Vec<HigherHalfMapping>,
}

impl HigherHalfMapper {
    /// Create new higher half mapper
    pub fn new(base: VirtualAddress) -> Self {
        Self {
            base,
            offset: 0,
            page_tables: PageTableManager::new(),
            mappings: Vec::new(),
        }
    }

    /// Create with default kernel base
    pub fn default_kernel() -> Self {
        Self::new(VirtualAddress(0xFFFF_8000_0000_0000))
    }

    /// Map physical range to higher half
    pub fn map_range(
        &mut self,
        physical_addr: PhysicalAddress,
        size: u64,
        flags: PageFlags,
    ) -> Result<VirtualAddress> {
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let virtual_addr = self.base + self.offset;

        unsafe {
            self.page_tables
                .map_range(virtual_addr, physical_addr, aligned_size, flags)?;
        }

        self.mappings.push(HigherHalfMapping {
            virtual_addr,
            physical_addr,
            size: aligned_size,
            flags,
        });

        self.offset += aligned_size;

        Ok(virtual_addr)
    }

    /// Map kernel image
    pub fn map_kernel(
        &mut self,
        physical_base: PhysicalAddress,
        code_size: u64,
        data_size: u64,
        rodata_size: u64,
    ) -> Result<KernelMapping> {
        let code_virt = self.map_range(physical_base, code_size, PageFlags::kernel_code())?;

        let data_virt = self.map_range(
            physical_base + code_size,
            data_size,
            PageFlags::kernel_data(),
        )?;

        let rodata_virt = self.map_range(
            physical_base + code_size + data_size,
            rodata_size,
            PageFlags::kernel_rodata(),
        )?;

        Ok(KernelMapping {
            code_virtual: code_virt,
            code_physical: physical_base,
            code_size,
            data_virtual: data_virt,
            data_physical: physical_base + code_size,
            data_size,
            rodata_virtual: rodata_virt,
            rodata_physical: physical_base + code_size + data_size,
            rodata_size,
        })
    }

    /// Get page tables
    pub const fn page_tables(&self) -> &PageTableManager {
        &self.page_tables
    }

    /// Get current virtual address
    pub fn current_address(&self) -> VirtualAddress {
        self.base + self.offset
    }

    /// Get mappings
    pub fn mappings(&self) -> &[HigherHalfMapping] {
        &self.mappings
    }
}

/// Higher half mapping entry
#[derive(Debug, Clone)]
pub struct HigherHalfMapping {
    /// Virtual address
    pub virtual_addr: VirtualAddress,
    /// Physical address
    pub physical_addr: PhysicalAddress,
    /// Size
    pub size: u64,
    /// Flags
    pub flags: PageFlags,
}

/// Kernel section mapping
#[derive(Debug, Clone)]
pub struct KernelMapping {
    /// Code section virtual
    pub code_virtual: VirtualAddress,
    /// Code section physical
    pub code_physical: PhysicalAddress,
    /// Code size
    pub code_size: u64,
    /// Data section virtual
    pub data_virtual: VirtualAddress,
    /// Data section physical
    pub data_physical: PhysicalAddress,
    /// Data size
    pub data_size: u64,
    /// Rodata section virtual
    pub rodata_virtual: VirtualAddress,
    /// Rodata section physical
    pub rodata_physical: PhysicalAddress,
    /// Rodata size
    pub rodata_size: u64,
}

impl KernelMapping {
    /// Get total size
    pub const fn total_size(&self) -> u64 {
        self.code_size + self.data_size + self.rodata_size
    }
}

// =============================================================================
// PHYSICAL MAPPING
// =============================================================================

/// Direct physical memory mapping
pub struct PhysicalMapper {
    /// Base virtual address for physical map
    base: VirtualAddress,
    /// Size of physical memory mapped
    size: u64,
    /// Page table manager
    page_tables: PageTableManager,
}

impl PhysicalMapper {
    /// Create new physical mapper
    pub fn new(base: VirtualAddress) -> Self {
        Self {
            base,
            size: 0,
            page_tables: PageTableManager::new(),
        }
    }

    /// Default physical map base
    pub fn default_base() -> Self {
        // Common higher half physical map offset
        Self::new(VirtualAddress(0xFFFF_8800_0000_0000))
    }

    /// Map all physical memory
    pub fn map_all(&mut self, physical_end: PhysicalAddress) -> Result<()> {
        // Use huge pages for efficiency
        let aligned_end = (physical_end.0 + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);

        // Map physical memory with huge pages
        let flags = PageFlags {
            writable: true,
            no_execute: true,
            global: true,
            ..Default::default()
        };

        // For now just track the intention
        // Actual mapping would use huge pages
        self.size = aligned_end;

        // Map with 2MB pages for better TLB usage
        let mut phys = 0u64;
        while phys < aligned_end {
            let virt = VirtualAddress(self.base.0 + phys);
            unsafe {
                self.page_tables
                    .map_range(virt, PhysicalAddress(phys), LARGE_PAGE_SIZE, flags)?;
            }
            phys += LARGE_PAGE_SIZE;
        }

        Ok(())
    }

    /// Convert physical to virtual
    pub const fn phys_to_virt(&self, physical: PhysicalAddress) -> Option<VirtualAddress> {
        if physical.0 < self.size {
            Some(VirtualAddress(self.base.0 + physical.0))
        } else {
            None
        }
    }

    /// Convert virtual to physical
    pub const fn virt_to_phys(&self, virtual_addr: VirtualAddress) -> Option<PhysicalAddress> {
        if virtual_addr.0 >= self.base.0 && virtual_addr.0 < self.base.0 + self.size {
            Some(PhysicalAddress(virtual_addr.0 - self.base.0))
        } else {
            None
        }
    }

    /// Get base address
    pub const fn base(&self) -> VirtualAddress {
        self.base
    }

    /// Get mapped size
    pub const fn size(&self) -> u64 {
        self.size
    }
}

// =============================================================================
// TLB MANAGEMENT
// =============================================================================

/// TLB management utilities
pub mod tlb {
    use crate::raw::types::VirtualAddress;

    /// Flush entire TLB
    #[inline]
    pub fn flush_all() {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // Read and write CR3 to flush TLB
            let cr3: u64;
            core::arch::asm!("mov {}, cr3", out(reg) cr3);
            core::arch::asm!("mov cr3, {}", in(reg) cr3);
        }
    }

    /// Flush single page
    #[inline]
    pub fn flush_page(addr: VirtualAddress) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("invlpg [{}]", in(reg) addr.0 as usize);
        }
    }

    /// Flush range of pages
    pub fn flush_range(start: VirtualAddress, size: u64) {
        const PAGE_SIZE: u64 = 4096;
        let mut addr = start;
        let end = start + size;

        while addr < end {
            flush_page(addr);
            addr += PAGE_SIZE;
        }
    }

    /// Flush TLB for PCID
    #[inline]
    pub fn flush_pcid(pcid: u16) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // INVPCID instruction
            let descriptor: [u64; 2] = [u64::from(pcid), 0];
            core::arch::asm!(
                "invpcid {}, [{}]",
                in(reg) 0u64, // type 0 = single address
                in(reg) descriptor.as_ptr(),
            );
        }
    }

    /// Check if PCID supported
    pub fn pcid_supported() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // Check CPUID for PCID support (leaf 1, ecx bit 17)
            let result = crate::arch::x86_64::cpuid(1, 0);
            (result.ecx & (1 << 17)) != 0
        }

        #[cfg(not(target_arch = "x86_64"))]
        false
    }

    /// Check if INVPCID supported
    pub fn invpcid_supported() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // Check CPUID for INVPCID support (leaf 7, subleaf 0, ebx bit 10)
            let result = crate::arch::x86_64::cpuid(7, 0);
            (result.ebx & (1 << 10)) != 0
        }

        #[cfg(not(target_arch = "x86_64"))]
        false
    }
}

// =============================================================================
// MEMORY PROTECTION KEYS
// =============================================================================

/// Memory protection keys (`x86_64` PKU)
pub mod pku {
    /// Protection key rights
    #[derive(Debug, Clone, Copy)]
    pub struct ProtectionKeyRights {
        /// Access disabled
        pub access_disabled: bool,
        /// Write disabled
        pub write_disabled: bool,
    }

    /// Set protection key rights
    pub fn set_key_rights(key: u8, rights: ProtectionKeyRights) {
        #[cfg(target_arch = "x86_64")]
        {
            let mut pkru = read_pkru();
            let shift = u32::from(key) * 2;
            let mask = 0b11u32 << shift;

            pkru &= !mask;
            if rights.access_disabled {
                pkru |= 1 << shift;
            }
            if rights.write_disabled {
                pkru |= 2 << shift;
            }

            write_pkru(pkru);
        }
    }

    /// Read PKRU register
    #[cfg(target_arch = "x86_64")]
    fn read_pkru() -> u32 {
        let pkru: u32;
        unsafe {
            core::arch::asm!(
                "xor ecx, ecx",
                "rdpkru",
                out("eax") pkru,
                out("edx") _,
                out("ecx") _,
            );
        }
        pkru
    }

    /// Write PKRU register
    #[cfg(target_arch = "x86_64")]
    fn write_pkru(pkru: u32) {
        unsafe {
            core::arch::asm!(
                "xor ecx, ecx",
                "xor edx, edx",
                "wrpkru",
                in("eax") pkru,
            );
        }
    }

    /// Check if PKU supported
    pub fn pku_supported() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // Check CPUID for PKU support (leaf 7, subleaf 0, ecx bit 3)
            let result = crate::arch::x86_64::cpuid(7, 0);
            (result.ecx & (1 << 3)) != 0
        }

        #[cfg(not(target_arch = "x86_64"))]
        false
    }
}
// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_address_space() {
        let mut space = VirtualAddressSpace::with_limits(1, AddressSpaceLimits::x86_64_default());

        // Allocate region
        let addr = space
            .allocate(0x10000, VirtualRegionFlags::read_write())
            .unwrap();
        assert!(addr > 0);

        // Find region
        assert!(space.find_region(addr).is_some());
        assert!(space.find_region(addr + 0x1000).is_some());

        // Free region
        space.free(addr).unwrap();
        assert!(space.find_region(addr).is_none());
    }

    #[test]
    fn test_virtual_region_flags() {
        let flags = VirtualRegionFlags::read_write();
        assert!(flags.read);
        assert!(flags.write);
        assert!(!flags.execute);

        let page_flags = flags.to_page_flags();
        assert!(page_flags.writable);
        assert!(page_flags.no_execute);
    }

    #[test]
    fn test_physical_mapper() {
        let mapper = PhysicalMapper::new(0xFFFF_8800_0000_0000);

        // Before mapping
        assert!(mapper.phys_to_virt(0x1000).is_none());
    }
}
