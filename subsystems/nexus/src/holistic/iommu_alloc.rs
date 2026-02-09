// SPDX-License-Identifier: GPL-2.0
//! Holistic iommu_alloc — IOMMU address space allocator.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IOMMU mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuMapType {
    Identity,
    Translated,
    Swiotlb,
    Passthrough,
}

/// IOMMU address region
#[derive(Debug)]
pub struct IovaRegion {
    pub iova_start: u64,
    pub iova_size: u64,
    pub phys_addr: u64,
    pub map_type: IommuMapType,
    pub device_id: u64,
    pub read: bool,
    pub write: bool,
}

/// IOMMU domain allocator
#[derive(Debug)]
pub struct IommuAllocDomain {
    pub id: u64,
    pub regions: Vec<IovaRegion>,
    pub next_iova: u64,
    pub total_mapped: u64,
    pub map_count: u64,
    pub unmap_count: u64,
    pub faults: u64,
}

impl IommuAllocDomain {
    pub fn new(id: u64, base_iova: u64) -> Self {
        Self { id, regions: Vec::new(), next_iova: base_iova, total_mapped: 0, map_count: 0, unmap_count: 0, faults: 0 }
    }

    #[inline]
    pub fn map(&mut self, phys: u64, size: u64, dev: u64, mt: IommuMapType) -> u64 {
        let iova = self.next_iova;
        self.next_iova += size;
        self.regions.push(IovaRegion { iova_start: iova, iova_size: size, phys_addr: phys, map_type: mt, device_id: dev, read: true, write: true });
        self.total_mapped += size;
        self.map_count += 1;
        iova
    }

    #[inline]
    pub fn unmap(&mut self, iova: u64) -> bool {
        if let Some(idx) = self.regions.iter().position(|r| r.iova_start == iova) {
            let reg = self.regions.remove(idx);
            self.total_mapped -= reg.iova_size;
            self.unmap_count += 1;
            true
        } else { false }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IommuAllocStats {
    pub total_domains: u32,
    pub total_regions: u32,
    pub total_mapped_bytes: u64,
    pub total_maps: u64,
    pub total_unmaps: u64,
    pub total_faults: u64,
}

/// Main IOMMU allocator
pub struct HolisticIommuAlloc {
    domains: BTreeMap<u64, IommuAllocDomain>,
    next_id: u64,
}

impl HolisticIommuAlloc {
    pub fn new() -> Self { Self { domains: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create_domain(&mut self, base_iova: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.domains.insert(id, IommuAllocDomain::new(id, base_iova));
        id
    }

    #[inline]
    pub fn stats(&self) -> IommuAllocStats {
        let regions: u32 = self.domains.values().map(|d| d.regions.len() as u32).sum();
        let mapped: u64 = self.domains.values().map(|d| d.total_mapped).sum();
        let maps: u64 = self.domains.values().map(|d| d.map_count).sum();
        let unmaps: u64 = self.domains.values().map(|d| d.unmap_count).sum();
        let faults: u64 = self.domains.values().map(|d| d.faults).sum();
        IommuAllocStats { total_domains: self.domains.len() as u32, total_regions: regions, total_mapped_bytes: mapped, total_maps: maps, total_unmaps: unmaps, total_faults: faults }
    }
}
