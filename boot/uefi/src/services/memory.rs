//! Memory Services
//!
//! Safe wrappers for UEFI memory allocation and management.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;

use crate::raw::memory::{MemoryDescriptor, MemoryMapKey, MemoryType};
use crate::raw::types::{AllocateType, PhysicalAddress, Status};

// =============================================================================
// MEMORY ALLOCATION
// =============================================================================

/// Allocate memory from UEFI
///
/// # Safety
/// Must only be called while boot services are available.
pub unsafe fn allocate_pool(memory_type: MemoryType, size: usize) -> Result<*mut u8, Status> {
    let bs = super::boot_services();
    let mut buffer: *mut u8 = core::ptr::null_mut();

    let status = (bs.allocate_pool)(memory_type, size, &mut buffer);

    status.to_status_result_with(buffer)
}

/// Free pool memory
///
/// # Safety
/// Must only be called with pointers returned by `allocate_pool`.
pub unsafe fn free_pool(buffer: *mut u8) -> Result<(), Status> {
    if buffer.is_null() {
        return Ok(());
    }

    let bs = super::boot_services();
    let status = (bs.free_pool)(buffer);
    status.to_status_result()
}

/// Allocate pages
///
/// # Safety
/// Must only be called while boot services are available.
pub unsafe fn allocate_pages(
    alloc_type: AllocateType,
    memory_type: MemoryType,
    pages: usize,
    address: &mut PhysicalAddress,
) -> Result<(), Status> {
    let bs = super::boot_services();
    let status = (bs.allocate_pages)(alloc_type, memory_type, pages, address);
    status.to_status_result()
}

/// Free pages
///
/// # Safety
/// Must only be called with addresses returned by `allocate_pages`.
pub unsafe fn free_pages(address: PhysicalAddress, pages: usize) -> Result<(), Status> {
    let bs = super::boot_services();
    let status = (bs.free_pages)(address, pages);
    status.to_status_result()
}

// =============================================================================
// MEMORY MAP
// =============================================================================

/// Memory map wrapper
pub struct MemoryMap {
    /// Buffer containing the memory map
    buffer: *mut u8,
    /// Buffer size
    _buffer_size: usize,
    /// Map size (actual used)
    map_size: usize,
    /// Memory map key
    key: MemoryMapKey,
    /// Descriptor size
    descriptor_size: usize,
    /// Descriptor version
    descriptor_version: u32,
}

impl MemoryMap {
    /// Get the memory map
    ///
    /// # Safety
    /// Must only be called while boot services are available.
    pub unsafe fn get() -> Result<Self, Status> {
        let bs = super::boot_services();

        // First call to get size
        let mut map_size = 0;
        let mut key_value: usize = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = (bs.get_memory_map)(
            &mut map_size,
            core::ptr::null_mut(),
            &mut key_value,
            &mut descriptor_size,
            &mut descriptor_version,
        );

        if status != Status::BUFFER_TOO_SMALL {
            return Err(status);
        }

        // Add extra space for map changes during allocation
        let buffer_size = map_size + 4 * descriptor_size;
        let buffer = allocate_pool(MemoryType::LoaderData, buffer_size)?;

        // Get actual map
        let mut actual_size = buffer_size;
        let status = (bs.get_memory_map)(
            &mut actual_size,
            buffer.cast::<MemoryDescriptor>(),
            &mut key_value,
            &mut descriptor_size,
            &mut descriptor_version,
        );

        if !status.is_success() {
            free_pool(buffer)?;
            return Err(status);
        }

        Ok(Self {
            buffer,
            _buffer_size: buffer_size,
            map_size: actual_size,
            key: MemoryMapKey(key_value),
            descriptor_size,
            descriptor_version,
        })
    }

    /// Get memory map key
    pub const fn key(&self) -> MemoryMapKey {
        self.key
    }

    /// Get descriptor size
    pub const fn descriptor_size(&self) -> usize {
        self.descriptor_size
    }

    /// Get descriptor version
    pub const fn descriptor_version(&self) -> u32 {
        self.descriptor_version
    }

    /// Get number of entries
    pub const fn entry_count(&self) -> usize {
        self.map_size / self.descriptor_size
    }

    /// Iterate over memory descriptors
    #[allow(clippy::iter_without_into_iter)]
    pub const fn iter(&self) -> MemoryMapIterator<'_> {
        MemoryMapIterator {
            map: self,
            index: 0,
        }
    }

    /// Get total memory statistics
    pub fn statistics(&self) -> MemoryStatistics {
        let mut stats = MemoryStatistics::default();

        for desc in self.iter() {
            let size = desc.number_of_pages * 4096;
            stats.total_pages += desc.number_of_pages;

            let mt = desc.memory_type;
            if mt == MemoryType::ConventionalMemory as u32 {
                stats.conventional_pages += desc.number_of_pages;
                stats.usable_memory += size;
            } else if mt == MemoryType::LoaderCode as u32 || mt == MemoryType::LoaderData as u32 {
                stats.loader_pages += desc.number_of_pages;
                stats.usable_memory += size; // Reclaimable after boot
            } else if mt == MemoryType::BootServicesCode as u32
                || mt == MemoryType::BootServicesData as u32
            {
                stats.boot_services_pages += desc.number_of_pages;
                stats.usable_memory += size; // Reclaimable after ExitBootServices
            } else if mt == MemoryType::RuntimeServicesCode as u32
                || mt == MemoryType::RuntimeServicesData as u32
            {
                stats.runtime_services_pages += desc.number_of_pages;
            } else if mt == MemoryType::AcpiReclaimMemory as u32 {
                stats.acpi_reclaim_pages += desc.number_of_pages;
            } else if mt == MemoryType::AcpiNvsMemory as u32 {
                stats.acpi_nvs_pages += desc.number_of_pages;
            } else if mt == MemoryType::MemoryMappedIo as u32
                || mt == MemoryType::MemoryMappedIoPortSpace as u32
            {
                stats.mmio_pages += desc.number_of_pages;
            } else if mt == MemoryType::ReservedMemory as u32 {
                stats.reserved_pages += desc.number_of_pages;
            }
        }

        stats
    }

    /// Find largest free region
    pub fn largest_free_region(&self) -> Option<(PhysicalAddress, u64)> {
        let mut largest: Option<(PhysicalAddress, u64)> = None;

        for desc in self.iter() {
            if desc.memory_type == MemoryType::ConventionalMemory as u32 {
                let size = desc.number_of_pages * 4096;
                match largest {
                    Some((_, s)) if size > s => {
                        largest = Some((desc.physical_start, size));
                    },
                    None => {
                        largest = Some((desc.physical_start, size));
                    },
                    _ => {},
                }
            }
        }

        largest
    }

    /// Find memory at specific address
    pub fn find_at(&self, address: PhysicalAddress) -> Option<&MemoryDescriptor> {
        for desc in self.iter() {
            let start = desc.physical_start;
            let end = start + desc.number_of_pages * 4096;
            if address >= start && address < end {
                return Some(desc);
            }
        }
        None
    }
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        if !self.buffer.is_null() && super::boot_services_available() {
            unsafe {
                let _ = free_pool(self.buffer);
            }
        }
    }
}

/// Memory map iterator
pub struct MemoryMapIterator<'a> {
    map: &'a MemoryMap,
    index: usize,
}

impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = &'a MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.map.entry_count() {
            return None;
        }

        let offset = self.index * self.map.descriptor_size;
        self.index += 1;

        unsafe {
            let ptr = self.map.buffer.add(offset).cast::<MemoryDescriptor>();
            Some(&*ptr)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.map.entry_count() - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for MemoryMapIterator<'_> {}

/// Memory statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStatistics {
    /// Total pages in system
    pub total_pages: u64,
    /// Conventional memory pages
    pub conventional_pages: u64,
    /// Loader pages
    pub loader_pages: u64,
    /// Boot services pages
    pub boot_services_pages: u64,
    /// Runtime services pages
    pub runtime_services_pages: u64,
    /// ACPI reclaim pages
    pub acpi_reclaim_pages: u64,
    /// ACPI NVS pages
    pub acpi_nvs_pages: u64,
    /// MMIO pages
    pub mmio_pages: u64,
    /// Reserved pages
    pub reserved_pages: u64,
    /// Total usable memory (available after `ExitBootServices`)
    pub usable_memory: u64,
}

impl MemoryStatistics {
    /// Get total memory in bytes
    pub const fn total_memory(&self) -> u64 {
        self.total_pages * 4096
    }

    /// Get free memory in bytes (conventional only)
    pub const fn free_memory(&self) -> u64 {
        self.conventional_pages * 4096
    }

    /// Get memory that will be available after boot
    pub const fn available_after_boot(&self) -> u64 {
        (self.conventional_pages + self.boot_services_pages + self.loader_pages) * 4096
    }
}

// =============================================================================
// VIRTUAL MEMORY
// =============================================================================

/// Set virtual address map
///
/// # Safety
/// Must be called only once after `ExitBootServices`.
pub unsafe fn set_virtual_address_map(
    memory_map: &mut [MemoryDescriptor],
    descriptor_size: usize,
) -> Result<(), Status> {
    let rs = super::runtime_services();

    let map_size = memory_map.len() * descriptor_size;
    let status = (rs.set_virtual_address_map)(
        map_size,
        descriptor_size,
        1, // descriptor version
        memory_map.as_mut_ptr(),
    );

    status.to_status_result()
}

/// Convert physical address to virtual (for runtime services)
///
/// # Safety
/// Must be called only after `SetVirtualAddressMap`.
pub unsafe fn convert_pointer(
    debug_disposition: usize,
    address: *mut *mut core::ffi::c_void,
) -> Result<(), Status> {
    let rs = super::runtime_services();
    let status = (rs.convert_pointer)(debug_disposition, address);
    status.to_status_result()
}

// =============================================================================
// UEFI ALLOCATOR
// =============================================================================

/// UEFI memory allocator for use with Rust's alloc crate
pub struct UefiAllocator;

unsafe impl GlobalAlloc for UefiAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !super::boot_services_available() {
            return core::ptr::null_mut();
        }

        // UEFI allocations are 8-byte aligned, so we might need extra
        let align = layout.align();
        if align <= 8 {
            // Simple case
            allocate_pool(MemoryType::LoaderData, layout.size()).unwrap_or(core::ptr::null_mut())
        } else {
            // Need to allocate extra and align manually
            let total_size = layout.size() + align;
            let raw =
                allocate_pool(MemoryType::LoaderData, total_size).unwrap_or(core::ptr::null_mut());

            if raw.is_null() {
                return core::ptr::null_mut();
            }

            // Align the pointer
            let raw_addr = raw as usize;
            let aligned_addr = (raw_addr + align - 1) & !(align - 1);
            let aligned = aligned_addr as *mut u8;

            // Store original pointer before aligned address
            if aligned_addr > raw_addr {
                let header = aligned.cast::<*mut u8>().sub(1);
                *header = raw;
            }

            aligned
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !super::boot_services_available() || ptr.is_null() {
            return;
        }

        let align = layout.align();
        if align <= 8 {
            let _ = free_pool(ptr);
        } else {
            // Retrieve original pointer
            let header = ptr.cast::<*mut u8>().sub(1);
            let _ = free_pool(*header);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);

        if !new_ptr.is_null() && !ptr.is_null() {
            let copy_size = layout.size().min(new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
            self.dealloc(ptr, layout);
        }

        new_ptr
    }
}

// Note: Global allocator defined in lib.rs (BootAllocator)
// UefiAllocator is available for explicit use when needed
static _ALLOCATOR: UefiAllocator = UefiAllocator;

// =============================================================================
// MEMORY REGION HELPERS
// =============================================================================

/// Memory region for kernel handoff
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Start address
    pub start: PhysicalAddress,
    /// End address (exclusive)
    pub end: PhysicalAddress,
    /// Region type
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region
    pub const fn new(
        start: PhysicalAddress,
        end: PhysicalAddress,
        region_type: MemoryRegionType,
    ) -> Self {
        Self {
            start,
            end,
            region_type,
        }
    }

    /// Get size in bytes
    pub const fn size(&self) -> u64 {
        self.end.0 - self.start.0
    }

    /// Get size in pages (4K)
    pub const fn pages(&self) -> u64 {
        self.size() / 4096
    }

    /// Check if address is in region
    pub const fn contains(&self, addr: PhysicalAddress) -> bool {
        addr.0 >= self.start.0 && addr.0 < self.end.0
    }

    /// Check if this region overlaps with another
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start.0 < other.end.0 && other.start.0 < self.end.0
    }
}

/// Memory region type (simplified for kernel)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryRegionType {
    /// Usable RAM
    Usable          = 0,
    /// Reserved/unusable
    Reserved        = 1,
    /// ACPI reclaimable
    AcpiReclaimable = 2,
    /// ACPI NVS
    AcpiNvs         = 3,
    /// Memory-mapped I/O
    Mmio            = 4,
    /// Kernel code/data
    Kernel          = 5,
    /// Bootloader data
    Bootloader      = 6,
    /// Framebuffer
    Framebuffer     = 7,
}

impl From<MemoryType> for MemoryRegionType {
    fn from(uefi_type: MemoryType) -> Self {
        match uefi_type {
            MemoryType::ConventionalMemory
            | MemoryType::BootServicesCode
            | MemoryType::BootServicesData => Self::Usable,
            MemoryType::LoaderCode | MemoryType::LoaderData => Self::Bootloader,
            MemoryType::RuntimeServicesCode | MemoryType::RuntimeServicesData => Self::Reserved,
            MemoryType::AcpiReclaimMemory => Self::AcpiReclaimable,
            MemoryType::AcpiNvsMemory => Self::AcpiNvs,
            MemoryType::MemoryMappedIo | MemoryType::MemoryMappedIoPortSpace => Self::Mmio,
            _ => Self::Reserved,
        }
    }
}

// =============================================================================
// PAGE ALLOCATOR
// =============================================================================

/// Page-aligned allocator for large allocations
pub struct PageAllocator;

impl PageAllocator {
    /// Allocate pages
    ///
    /// # Safety
    /// Must only be called while boot services are available.
    pub unsafe fn allocate(pages: usize) -> Option<NonNull<u8>> {
        let mut address = PhysicalAddress(0);
        allocate_pages(
            AllocateType::AllocateAnyPages,
            MemoryType::LoaderData,
            pages,
            &mut address,
        )
        .ok()?;

        NonNull::new(address.0 as *mut u8)
    }

    /// Allocate pages at specific address
    ///
    /// # Safety
    /// Must only be called while boot services are available.
    pub unsafe fn allocate_at(address: PhysicalAddress, pages: usize) -> Result<(), Status> {
        let mut addr = address;
        allocate_pages(
            AllocateType::AllocateAddress,
            MemoryType::LoaderData,
            pages,
            &mut addr,
        )
    }

    /// Allocate pages below address
    ///
    /// # Safety
    /// Must only be called while boot services are available.
    pub unsafe fn allocate_below(
        max_address: PhysicalAddress,
        pages: usize,
    ) -> Option<PhysicalAddress> {
        let mut address = max_address;
        allocate_pages(
            AllocateType::AllocateMaxAddress,
            MemoryType::LoaderData,
            pages,
            &mut address,
        )
        .ok()?;

        Some(address)
    }

    /// Free pages
    ///
    /// # Safety
    /// Must only be called with addresses returned by `PageAllocator` methods.
    pub unsafe fn free(address: PhysicalAddress, pages: usize) -> Result<(), Status> {
        free_pages(address, pages)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::new(0x1000, 0x5000, MemoryRegionType::Usable);
        assert_eq!(region.size(), 0x4000);
        assert_eq!(region.pages(), 4);
        assert!(region.contains(0x2000));
        assert!(!region.contains(0x5000));
    }

    #[test]
    fn test_region_overlap() {
        let r1 = MemoryRegion::new(0x1000, 0x3000, MemoryRegionType::Usable);
        let r2 = MemoryRegion::new(0x2000, 0x4000, MemoryRegionType::Usable);
        let r3 = MemoryRegion::new(0x4000, 0x5000, MemoryRegionType::Usable);

        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3));
    }
}
