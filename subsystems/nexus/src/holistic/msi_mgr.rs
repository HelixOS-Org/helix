// SPDX-License-Identifier: GPL-2.0
//! Holistic msi_mgr — MSI/MSI-X interrupt manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// MSI type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsiType {
    Msi,
    MsiX,
    Legacy,
}

/// MSI entry
#[derive(Debug)]
pub struct MsiEntry {
    pub vector: u32,
    pub msi_type: MsiType,
    pub target_cpu: u32,
    pub addr: u64,
    pub data: u32,
    pub masked: bool,
    pub pending: bool,
    pub irq_count: u64,
}

impl MsiEntry {
    pub fn new(vector: u32, mt: MsiType, cpu: u32) -> Self {
        Self { vector, msi_type: mt, target_cpu: cpu, addr: 0xFEE0_0000 | ((cpu as u64) << 12), data: vector as u32, masked: false, pending: false, irq_count: 0 }
    }
}

/// Device MSI config
#[derive(Debug)]
pub struct DeviceMsi {
    pub device_id: u64,
    pub entries: Vec<MsiEntry>,
    pub max_vectors: u32,
    pub allocated_vectors: u32,
    pub total_irqs: u64,
}

impl DeviceMsi {
    pub fn new(dev: u64, max: u32) -> Self { Self { device_id: dev, entries: Vec::new(), max_vectors: max, allocated_vectors: 0, total_irqs: 0 } }

    pub fn allocate(&mut self, vector: u32, mt: MsiType, cpu: u32) -> bool {
        if self.allocated_vectors >= self.max_vectors { return false; }
        self.entries.push(MsiEntry::new(vector, mt, cpu));
        self.allocated_vectors += 1;
        true
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MsiMgrStats {
    pub total_devices: u32,
    pub total_vectors: u32,
    pub msi_vectors: u32,
    pub msix_vectors: u32,
    pub total_irqs: u64,
}

/// Main MSI manager
pub struct HolisticMsiMgr {
    devices: BTreeMap<u64, DeviceMsi>,
    next_vector: u32,
}

impl HolisticMsiMgr {
    pub fn new() -> Self { Self { devices: BTreeMap::new(), next_vector: 32 } }

    pub fn register_device(&mut self, dev_id: u64, max_vectors: u32) {
        self.devices.insert(dev_id, DeviceMsi::new(dev_id, max_vectors));
    }

    pub fn allocate_vector(&mut self, dev_id: u64, mt: MsiType, cpu: u32) -> Option<u32> {
        let vec = self.next_vector; self.next_vector += 1;
        let dev = self.devices.get_mut(&dev_id)?;
        if dev.allocate(vec, mt, cpu) { Some(vec) } else { None }
    }

    pub fn stats(&self) -> MsiMgrStats {
        let vecs: u32 = self.devices.values().map(|d| d.allocated_vectors).sum();
        let msi: u32 = self.devices.values().flat_map(|d| d.entries.iter()).filter(|e| e.msi_type == MsiType::Msi).count() as u32;
        let msix: u32 = self.devices.values().flat_map(|d| d.entries.iter()).filter(|e| e.msi_type == MsiType::MsiX).count() as u32;
        let irqs: u64 = self.devices.values().map(|d| d.total_irqs).sum();
        MsiMgrStats { total_devices: self.devices.len() as u32, total_vectors: vecs, msi_vectors: msi, msix_vectors: msix, total_irqs: irqs }
    }
}
