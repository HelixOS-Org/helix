// SPDX-License-Identifier: GPL-2.0
//! Holistic iommu — I/O memory management unit tracking.

extern crate alloc;

use alloc::collections::BTreeMap;

/// IOMMU domain type
use alloc::vec::Vec;
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuDomainType {
    #[default]
    Dma,
    Identity,
    Blocked,
    Unmanaged,
}

/// IOMMU mapping
#[derive(Default, Debug)]
pub struct IommuMapping {
    pub iova: u64,
    pub phys_addr: u64,
    pub size: u64,
    pub prot: u32,

    pub paddr: u64,
}

/// IOMMU domain
#[derive(Default, Debug)]
pub struct IommuDomain {
    pub id: u64,
    pub domain_type: IommuDomainType,
    pub mappings: BTreeMap<u64, IommuMapping>,
    pub total_mapped: u64,
    pub total_unmapped: u64,
    pub page_faults: u64,

    pub devices: alloc::vec::Vec<u32>,
    pub fault_count: u64,
    pub pt_level: u32,
    pub total_mapped_bytes: u64,
}

impl IommuDomain {
    pub fn new(id: u64, dtype: IommuDomainType) -> Self {
        Self { id, domain_type: dtype, mappings: BTreeMap::new(), total_mapped: 0, total_unmapped: 0, page_faults: 0, ..Default::default() }
    }

    #[inline(always)]
    pub fn map(&mut self, iova: u64, phys: u64, size: u64, prot: u32) {
        self.mappings.insert(iova, IommuMapping { iova, phys_addr: phys, size, prot, ..Default::default() });
        self.total_mapped += size;
    }

    #[inline]
    pub fn unmap(&mut self, iova: u64) -> Option<u64> {
        if let Some(m) = self.mappings.remove(&iova) {
            self.total_unmapped += m.size;
            Some(m.size)
        } else { None }
    }

    /// Attach a device to this domain (V2 API)
    #[inline]
    pub fn attach_device(&mut self, bdf: u64) {
        self.devices.push(bdf as u32);
    }
}

/// IOMMU device
#[derive(Debug)]
pub struct IommuDevice {
    pub dev_id: u32,
    pub domain_id: u64,
    pub group_id: u32,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IommuStats {
    pub total_domains: u32,
    pub total_devices: u32,
    pub total_mapped_bytes: u64,
    pub total_page_faults: u64,
}

/// Main holistic IOMMU
pub struct HolisticIommu {
    domains: BTreeMap<u64, IommuDomain>,
    devices: BTreeMap<u32, IommuDevice>,
    next_id: u64,
}

impl HolisticIommu {
    pub fn new() -> Self { Self { domains: BTreeMap::new(), devices: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create_domain(&mut self, dtype: IommuDomainType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.domains.insert(id, IommuDomain::new(id, dtype));
        id
    }

    #[inline(always)]
    pub fn attach_device(&mut self, dev_id: u32, domain: u64, group: u32) {
        self.devices.insert(dev_id, IommuDevice { dev_id, domain_id: domain, group_id: group });
    }

    #[inline(always)]
    pub fn map(&mut self, domain: u64, iova: u64, phys: u64, size: u64, prot: u32) {
        if let Some(d) = self.domains.get_mut(&domain) { d.map(iova, phys, size, prot); }
    }

    #[inline(always)]
    pub fn unmap(&mut self, domain: u64, iova: u64) {
        if let Some(d) = self.domains.get_mut(&domain) { d.unmap(iova); }
    }

    #[inline]
    pub fn stats(&self) -> IommuStats {
        let mapped: u64 = self.domains.values().map(|d| d.total_mapped - d.total_unmapped).sum();
        let faults: u64 = self.domains.values().map(|d| d.page_faults).sum();
        IommuStats { total_domains: self.domains.len() as u32, total_devices: self.devices.len() as u32, total_mapped_bytes: mapped, total_page_faults: faults }
    }
}

// ============================================================================
// Merged from iommu_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuType {
    IntelVtD,
    AmdVi,
    ArmSmmu,
    VirtioIommu,
}

/// Page table level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoptLevel {
    Level3,
    Level4,
    Level5,
}



/// IO mapping
#[derive(Debug, Clone)]
pub struct IoMapping {
    pub iova: u64,
    pub paddr: u64,
    pub size: u64,
    pub flags: u32,
}

/// IOMMU fault
#[derive(Debug, Clone)]
pub struct IommuFault {
    pub domain_id: u64,
    pub device_bdf: u64,
    pub faulting_iova: u64,
    pub fault_type: IommuFaultType,
    pub timestamp: u64,
}

/// Fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuFaultType {
    PageNotPresent,
    WriteProtect,
    DeviceAcs,
    IrqRemap,
    Timeout,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IommuV2Stats {
    pub total_domains: u32,
    pub total_devices: u32,
    pub total_mappings: u64,
    pub total_mapped_bytes: u64,
    pub total_faults: u64,
    pub fault_rate: f64,
}

/// Main IOMMU v2 manager
pub struct HolisticIommuV2 {
    iommu_type: IommuType,
    domains: BTreeMap<u64, IommuDomain>,
    faults: Vec<IommuFault>,
    next_domain_id: u64,
    total_faults: u64,
}

impl HolisticIommuV2 {
    pub fn new(iommu_type: IommuType) -> Self {
        Self { iommu_type, domains: BTreeMap::new(), faults: Vec::new(), next_domain_id: 1, total_faults: 0 }
    }

    #[inline]
    pub fn create_domain(&mut self, dtype: IommuDomainType, _level: IoptLevel) -> u64 {
        let id = self.next_domain_id;
        self.next_domain_id += 1;
        self.domains.insert(id, IommuDomain::new(id, dtype));
        id
    }

    #[inline(always)]
    pub fn attach(&mut self, domain: u64, bdf: u64) {
        if let Some(d) = self.domains.get_mut(&domain) { d.attach_device(bdf); }
    }

    #[inline(always)]
    pub fn map_iova(&mut self, domain: u64, iova: u64, paddr: u64, size: u64, flags: u32) {
        if let Some(d) = self.domains.get_mut(&domain) { d.map(iova, paddr, size, flags); }
    }

    #[inline(always)]
    pub fn unmap_iova(&mut self, domain: u64, iova: u64) -> Option<u64> {
        self.domains.get_mut(&domain)?.unmap(iova)
    }

    #[inline]
    pub fn report_fault(&mut self, fault: IommuFault) {
        if let Some(d) = self.domains.get_mut(&fault.domain_id) { d.fault_count += 1; }
        self.total_faults += 1;
        if self.faults.len() > 4096 { self.faults.drain(..2048); }
        self.faults.push(fault);
    }

    #[inline]
    pub fn stats(&self) -> IommuV2Stats {
        let devices: u32 = self.domains.values().map(|d| d.devices.len() as u32).sum();
        let mappings: u64 = self.domains.values().map(|d| d.mappings.len() as u64).sum();
        let bytes: u64 = self.domains.values().map(|d| d.total_mapped_bytes).sum();
        IommuV2Stats {
            total_domains: self.domains.len() as u32, total_devices: devices,
            total_mappings: mappings, total_mapped_bytes: bytes,
            total_faults: self.total_faults,
            fault_rate: if mappings == 0 { 0.0 } else { self.total_faults as f64 / mappings as f64 },
        }
    }
}
