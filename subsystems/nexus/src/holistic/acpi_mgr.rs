// SPDX-License-Identifier: GPL-2.0
//! Holistic acpi_mgr — ACPI table parsing and power management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// ACPI table type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiTableType {
    Rsdp, Rsdt, Xsdt, Fadt, Madt, Dsdt, Ssdt,
    Hpet, Mcfg, Srat, Slit, Bgrt, Dmar, Bert,
}

/// Power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiPowerState {
    S0Working, S1Sleep, S2Sleep, S3Suspend, S4Hibernate, S5SoftOff,
}

/// MADT entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadtEntryType {
    LocalApic, IoApic, InterruptOverride, NmiSource,
    LocalApicNmi, LocalApicOverride, IoSapic, LocalSapic,
    PlatformInterrupt, LocalX2Apic, LocalX2ApicNmi,
}

/// ACPI table header
#[derive(Debug, Clone)]
pub struct AcpiTableHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub address: u64,
}

/// MADT entry
#[derive(Debug, Clone)]
pub struct MadtEntry {
    pub entry_type: MadtEntryType,
    pub processor_id: u8,
    pub apic_id: u32,
    pub flags: u32,
    pub enabled: bool,
}

/// SRAT memory affinity
#[derive(Debug, Clone)]
pub struct SratMemAffinity {
    pub domain: u32,
    pub base_addr: u64,
    pub length: u64,
    pub enabled: bool,
    pub hotplug: bool,
}

/// Stats
#[derive(Debug, Clone)]
pub struct AcpiMgrStats {
    pub tables_parsed: u32,
    pub processors_found: u32,
    pub ioapics_found: u32,
    pub numa_domains: u32,
    pub current_power_state: u8,
}

/// Main ACPI manager
pub struct HolisticAcpiMgr {
    tables: BTreeMap<u32, AcpiTableHeader>,
    madt_entries: Vec<MadtEntry>,
    srat_mem: Vec<SratMemAffinity>,
    power_state: AcpiPowerState,
    next_table_id: u32,
}

impl HolisticAcpiMgr {
    pub fn new() -> Self {
        Self { tables: BTreeMap::new(), madt_entries: Vec::new(), srat_mem: Vec::new(), power_state: AcpiPowerState::S0Working, next_table_id: 1 }
    }

    pub fn add_table(&mut self, header: AcpiTableHeader) -> u32 {
        let id = self.next_table_id; self.next_table_id += 1;
        self.tables.insert(id, header);
        id
    }

    pub fn add_madt_entry(&mut self, entry: MadtEntry) { self.madt_entries.push(entry); }
    pub fn add_srat_mem(&mut self, entry: SratMemAffinity) { self.srat_mem.push(entry); }
    pub fn set_power_state(&mut self, state: AcpiPowerState) { self.power_state = state; }

    pub fn stats(&self) -> AcpiMgrStats {
        let procs = self.madt_entries.iter().filter(|e| matches!(e.entry_type, MadtEntryType::LocalApic | MadtEntryType::LocalX2Apic) && e.enabled).count() as u32;
        let ioapics = self.madt_entries.iter().filter(|e| e.entry_type == MadtEntryType::IoApic).count() as u32;
        let mut domains = Vec::new();
        for s in &self.srat_mem { if !domains.contains(&s.domain) { domains.push(s.domain); } }
        AcpiMgrStats { tables_parsed: self.tables.len() as u32, processors_found: procs, ioapics_found: ioapics, numa_domains: domains.len() as u32, current_power_state: self.power_state as u8 }
    }
}
