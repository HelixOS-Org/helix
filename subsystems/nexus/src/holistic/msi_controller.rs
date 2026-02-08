// SPDX-License-Identifier: GPL-2.0
//! Holistic msi_controller — MSI/MSI-X interrupt controller management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// MSI type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsiType {
    Msi,
    MsiX,
    ImsEntry,
}

/// MSI delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsiDeliveryMode {
    Fixed,
    LowestPriority,
    Smi,
    Nmi,
    Init,
    ExtInt,
}

/// MSI vector entry
#[derive(Debug, Clone)]
pub struct MsiVector {
    pub index: u32,
    pub vector: u8,
    pub dest_apic: u32,
    pub delivery_mode: MsiDeliveryMode,
    pub masked: bool,
    pub pending: bool,
    pub trigger_count: u64,
    pub last_trigger: u64,
}

impl MsiVector {
    pub fn new(index: u32, vector: u8, dest: u32) -> Self {
        Self {
            index, vector, dest_apic: dest, delivery_mode: MsiDeliveryMode::Fixed,
            masked: false, pending: false, trigger_count: 0, last_trigger: 0,
        }
    }

    pub fn trigger(&mut self, now: u64) {
        if self.masked { self.pending = true; return; }
        self.trigger_count += 1;
        self.last_trigger = now;
    }

    pub fn mask(&mut self) { self.masked = true; }
    pub fn unmask(&mut self, now: u64) {
        self.masked = false;
        if self.pending { self.pending = false; self.trigger(now); }
    }
}

/// MSI device capability
#[derive(Debug)]
pub struct MsiDevice {
    pub bdf: u64,
    pub msi_type: MsiType,
    pub vectors: Vec<MsiVector>,
    pub max_vectors: u32,
    pub enabled: bool,
    pub total_interrupts: u64,
}

impl MsiDevice {
    pub fn new(bdf: u64, msi_type: MsiType, max_vectors: u32) -> Self {
        Self { bdf, msi_type, vectors: Vec::new(), max_vectors, enabled: false, total_interrupts: 0 }
    }

    pub fn allocate_vector(&mut self, vector: u8, dest: u32) -> Option<u32> {
        if self.vectors.len() as u32 >= self.max_vectors { return None; }
        let idx = self.vectors.len() as u32;
        self.vectors.push(MsiVector::new(idx, vector, dest));
        Some(idx)
    }

    pub fn enable(&mut self) { self.enabled = true; }
    pub fn disable(&mut self) { self.enabled = false; }

    pub fn trigger(&mut self, index: u32, now: u64) {
        if !self.enabled { return; }
        if let Some(v) = self.vectors.get_mut(index as usize) {
            v.trigger(now);
            self.total_interrupts += 1;
        }
    }
}

/// IRQ domain for MSI
#[derive(Debug, Clone)]
pub struct MsiIrqDomain {
    pub id: u32,
    pub base_vector: u8,
    pub vector_count: u32,
    pub allocated: u32,
}

impl MsiIrqDomain {
    pub fn new(id: u32, base: u8, count: u32) -> Self {
        Self { id, base_vector: base, vector_count: count, allocated: 0 }
    }

    pub fn allocate(&mut self) -> Option<u8> {
        if self.allocated >= self.vector_count { return None; }
        let v = self.base_vector.wrapping_add(self.allocated as u8);
        self.allocated += 1;
        Some(v)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MsiControllerStats {
    pub total_devices: u32,
    pub total_vectors: u32,
    pub total_interrupts: u64,
    pub masked_vectors: u32,
    pub pending_vectors: u32,
    pub irq_domains: u32,
}

/// Main MSI controller
pub struct HolisticMsiController {
    devices: BTreeMap<u64, MsiDevice>,
    irq_domains: BTreeMap<u32, MsiIrqDomain>,
    next_domain: u32,
}

impl HolisticMsiController {
    pub fn new() -> Self {
        Self { devices: BTreeMap::new(), irq_domains: BTreeMap::new(), next_domain: 0 }
    }

    pub fn register_device(&mut self, bdf: u64, msi_type: MsiType, max_vectors: u32) {
        self.devices.insert(bdf, MsiDevice::new(bdf, msi_type, max_vectors));
    }

    pub fn allocate_vector(&mut self, bdf: u64, vector: u8, dest: u32) -> Option<u32> {
        self.devices.get_mut(&bdf)?.allocate_vector(vector, dest)
    }

    pub fn enable_device(&mut self, bdf: u64) {
        if let Some(d) = self.devices.get_mut(&bdf) { d.enable(); }
    }

    pub fn trigger(&mut self, bdf: u64, index: u32, now: u64) {
        if let Some(d) = self.devices.get_mut(&bdf) { d.trigger(index, now); }
    }

    pub fn create_irq_domain(&mut self, base: u8, count: u32) -> u32 {
        let id = self.next_domain;
        self.next_domain += 1;
        self.irq_domains.insert(id, MsiIrqDomain::new(id, base, count));
        id
    }

    pub fn stats(&self) -> MsiControllerStats {
        let vectors: u32 = self.devices.values().map(|d| d.vectors.len() as u32).sum();
        let ints: u64 = self.devices.values().map(|d| d.total_interrupts).sum();
        let masked: u32 = self.devices.values().flat_map(|d| &d.vectors).filter(|v| v.masked).count() as u32;
        let pending: u32 = self.devices.values().flat_map(|d| &d.vectors).filter(|v| v.pending).count() as u32;
        MsiControllerStats {
            total_devices: self.devices.len() as u32, total_vectors: vectors,
            total_interrupts: ints, masked_vectors: masked,
            pending_vectors: pending, irq_domains: self.irq_domains.len() as u32,
        }
    }
}
