// SPDX-License-Identifier: GPL-2.0
//! Apps remap_pfn_app — PFN remapping for device memory.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Remap type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemapPfnType {
    DeviceMemory,
    Framebuffer,
    PciBar,
    IoMemory,
}

/// PFN mapping
#[derive(Debug, Clone)]
pub struct PfnMapping {
    pub id: u64,
    pub virt_addr: u64,
    pub pfn_start: u64,
    pub page_count: u64,
    pub remap_type: RemapPfnType,
    pub write_combine: bool,
    pub uncacheable: bool,
    pub owner_pid: u64,
}

impl PfnMapping {
    pub fn size(&self) -> u64 { self.page_count * 4096 }
    pub fn phys_addr(&self) -> u64 { self.pfn_start * 4096 }
    pub fn contains_virt(&self, addr: u64) -> bool { addr >= self.virt_addr && addr < self.virt_addr + self.size() }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RemapPfnAppStats {
    pub total_mappings: u32,
    pub total_pages: u64,
    pub device_mappings: u32,
    pub framebuffer_mappings: u32,
    pub total_bytes_mapped: u64,
}

/// Main remap PFN app
pub struct AppRemapPfn {
    mappings: BTreeMap<u64, PfnMapping>,
    next_id: u64,
}

impl AppRemapPfn {
    pub fn new() -> Self { Self { mappings: BTreeMap::new(), next_id: 1 } }

    pub fn remap(&mut self, virt_addr: u64, pfn: u64, pages: u64, remap_type: RemapPfnType, pid: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.mappings.insert(id, PfnMapping { id, virt_addr, pfn_start: pfn, page_count: pages, remap_type, write_combine: false, uncacheable: true, owner_pid: pid });
        id
    }

    pub fn unmap(&mut self, id: u64) { self.mappings.remove(&id); }

    pub fn find_by_virt(&self, addr: u64) -> Option<&PfnMapping> {
        self.mappings.values().find(|m| m.contains_virt(addr))
    }

    pub fn stats(&self) -> RemapPfnAppStats {
        let pages: u64 = self.mappings.values().map(|m| m.page_count).sum();
        let dev = self.mappings.values().filter(|m| m.remap_type == RemapPfnType::DeviceMemory).count() as u32;
        let fb = self.mappings.values().filter(|m| m.remap_type == RemapPfnType::Framebuffer).count() as u32;
        RemapPfnAppStats { total_mappings: self.mappings.len() as u32, total_pages: pages, device_mappings: dev, framebuffer_mappings: fb, total_bytes_mapped: pages * 4096 }
    }
}
