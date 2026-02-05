//! # Address Space Management

use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use helix_hal::{PageSize, VirtAddr};
use spin::RwLock;

use super::{AddressSpaceId, PageFlags, VirtualMapper, VmRegion, VmRegionType};
use crate::{MemError, MemResult, Page};

/// Address space
pub struct AddressSpace {
    /// Address space ID
    id: AddressSpaceId,
    /// Regions
    regions: RwLock<BTreeMap<u64, VmRegion>>,
    /// Page table mapper
    mapper: Arc<dyn VirtualMapper>,
    /// Start of user space
    user_start: VirtAddr,
    /// End of user space
    user_end: VirtAddr,
    /// Heap break
    brk: RwLock<VirtAddr>,
}

impl AddressSpace {
    /// Create a new address space
    pub fn new(
        id: AddressSpaceId,
        mapper: Arc<dyn VirtualMapper>,
        user_start: VirtAddr,
        user_end: VirtAddr,
    ) -> Self {
        Self {
            id,
            regions: RwLock::new(BTreeMap::new()),
            mapper,
            user_start,
            user_end,
            brk: RwLock::new(user_start),
        }
    }

    /// Get address space ID
    pub fn id(&self) -> AddressSpaceId {
        self.id
    }

    /// Add a region
    pub fn add_region(&self, region: VmRegion) -> MemResult<()> {
        let mut regions = self.regions.write();

        // Check for overlaps
        for (_, existing) in regions.iter() {
            if existing.overlaps(&region) {
                return Err(MemError::AlreadyMapped);
            }
        }

        regions.insert(region.start.as_u64(), region);
        Ok(())
    }

    /// Remove a region
    pub fn remove_region(&self, start: VirtAddr) -> MemResult<VmRegion> {
        self.regions
            .write()
            .remove(&start.as_u64())
            .ok_or(MemError::NotMapped)
    }

    /// Find region containing address
    pub fn find_region(&self, addr: VirtAddr) -> Option<VmRegion> {
        let regions = self.regions.read();
        for (_, region) in regions.iter() {
            if region.contains(addr) {
                return Some(region.clone());
            }
        }
        None
    }

    /// Allocate anonymous memory
    pub fn mmap_anonymous(
        &self,
        hint: Option<VirtAddr>,
        size: u64,
        flags: PageFlags,
    ) -> MemResult<VirtAddr> {
        let aligned_size = (size + 0xFFF) & !0xFFF; // Page align

        let addr = if let Some(h) = hint {
            // Try to use hint
            if self.is_range_free(h, aligned_size) {
                h
            } else {
                self.find_free_range(aligned_size)?
            }
        } else {
            self.find_free_range(aligned_size)?
        };

        let region = VmRegion {
            start: addr,
            size: aligned_size,
            flags,
            region_type: VmRegionType::Anonymous,
        };

        self.add_region(region)?;
        Ok(addr)
    }

    /// Unmap memory
    pub fn munmap(&self, addr: VirtAddr, size: u64) -> MemResult<()> {
        let _region = self.remove_region(addr)?;

        // Unmap pages
        let page_size = PageSize::Size4KiB.size() as u64;
        let mut current = addr.as_u64();
        let end = current + size;

        while current < end {
            let page = Page::new(VirtAddr::new(current), PageSize::Size4KiB);
            let _ = self.mapper.unmap(page); // Ignore errors for unmapped pages
            current += page_size;
        }

        Ok(())
    }

    /// Change memory protection
    pub fn mprotect(&self, addr: VirtAddr, size: u64, flags: PageFlags) -> MemResult<()> {
        let page_size = PageSize::Size4KiB.size() as u64;
        let mut current = addr.as_u64();
        let end = current + size;

        while current < end {
            let page = Page::new(VirtAddr::new(current), PageSize::Size4KiB);
            self.mapper.update_flags(page, flags)?;
            current += page_size;
        }

        // Update region flags
        let mut regions = self.regions.write();
        if let Some(region) = regions.get_mut(&addr.as_u64()) {
            region.flags = flags;
        }

        Ok(())
    }

    /// Adjust heap break
    pub fn brk(&self, new_brk: Option<VirtAddr>) -> VirtAddr {
        let mut brk = self.brk.write();

        if let Some(new) = new_brk {
            if new >= self.user_start && new < self.user_end {
                *brk = new;
            }
        }

        *brk
    }

    /// Check if a range is free
    fn is_range_free(&self, start: VirtAddr, size: u64) -> bool {
        let end = VirtAddr::new(start.as_u64() + size);

        if start < self.user_start || end > self.user_end {
            return false;
        }

        let regions = self.regions.read();
        for (_, region) in regions.iter() {
            if region.start < end && start < region.end() {
                return false;
            }
        }

        true
    }

    /// Find a free range of given size
    fn find_free_range(&self, size: u64) -> MemResult<VirtAddr> {
        let regions = self.regions.read();

        let mut current = self.user_start.as_u64();

        for (_, region) in regions.iter() {
            if region.start.as_u64() - current >= size {
                return Ok(VirtAddr::new(current));
            }
            current = region.end().as_u64();
        }

        // Check space after last region
        if self.user_end.as_u64() - current >= size {
            return Ok(VirtAddr::new(current));
        }

        Err(MemError::OutOfMemory)
    }
}
