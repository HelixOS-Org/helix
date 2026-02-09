//! ACPI Parser
//!
//! ACPI table parsing and management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{AcpiRevision, AcpiSignature, AcpiTableInfo};

/// MADT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadtEntryType {
    /// Local APIC
    LocalApic,
    /// I/O APIC
    IoApic,
    /// Interrupt Source Override
    InterruptOverride,
    /// NMI Source
    NmiSource,
    /// Local APIC NMI
    LocalApicNmi,
    /// Local APIC Address Override
    LocalApicOverride,
    /// I/O SAPIC
    IoSapic,
    /// Local SAPIC
    LocalSapic,
    /// Platform Interrupt Sources
    PlatformInterrupt,
    /// Local x2APIC
    LocalX2Apic,
    /// Local x2APIC NMI
    LocalX2ApicNmi,
    /// GIC CPU Interface
    GicCpu,
    /// GIC Distributor
    GicDistributor,
    /// GIC MSI Frame
    GicMsiFrame,
    /// GIC Redistributor
    GicRedistributor,
    /// GIC Interrupt Translation Service
    GicIts,
    /// Unknown type
    Unknown(u8),
}

/// MADT entry
#[derive(Debug, Clone)]
pub struct MadtEntry {
    /// Entry type
    pub entry_type: MadtEntryType,
    /// Entry data
    pub data: Vec<u8>,
    /// Processor/APIC ID (if applicable)
    pub processor_id: Option<u32>,
    /// Flags
    pub flags: u32,
}

/// SRAT entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SratEntryType {
    /// Processor Local APIC/SAPIC Affinity
    ProcessorAffinity,
    /// Memory Affinity
    MemoryAffinity,
    /// Processor Local x2APIC Affinity
    X2ApicAffinity,
    /// GICC Affinity
    GiccAffinity,
    /// Unknown
    Unknown(u8),
}

/// SRAT entry (System Resource Affinity Table)
#[derive(Debug, Clone)]
pub struct SratEntry {
    /// Entry type
    pub entry_type: SratEntryType,
    /// Proximity domain
    pub proximity_domain: u32,
    /// Flags
    pub flags: u32,
    /// Additional data
    pub data: Vec<u8>,
}

/// ACPI parsing result
#[derive(Debug, Clone)]
pub struct AcpiParseResult {
    /// ACPI revision
    pub revision: AcpiRevision,
    /// Detected tables
    pub tables: Vec<AcpiTableInfo>,
    /// MADT entries
    pub madt_entries: Vec<MadtEntry>,
    /// SRAT entries
    pub srat_entries: Vec<SratEntry>,
    /// CPU count
    pub cpu_count: u32,
    /// NUMA node count
    pub numa_node_count: u32,
    /// Parse errors
    pub errors: Vec<String>,
}

/// ACPI parser
pub struct AcpiParser {
    /// RSDP address
    rsdp_address: Option<u64>,
    /// Parsed tables
    tables: BTreeMap<AcpiSignature, AcpiTableInfo>,
    /// MADT entries
    madt_entries: Vec<MadtEntry>,
    /// SRAT entries
    srat_entries: Vec<SratEntry>,
    /// ACPI revision
    revision: AcpiRevision,
    /// Parse errors
    errors: Vec<String>,
    /// Tables parsed count
    tables_parsed: AtomicU64,
}

impl AcpiParser {
    /// Create new ACPI parser
    pub fn new() -> Self {
        Self {
            rsdp_address: None,
            tables: BTreeMap::new(),
            madt_entries: Vec::new(),
            srat_entries: Vec::new(),
            revision: AcpiRevision::V1_0,
            errors: Vec::new(),
            tables_parsed: AtomicU64::new(0),
        }
    }

    /// Set RSDP address
    #[inline(always)]
    pub fn set_rsdp(&mut self, address: u64) {
        self.rsdp_address = Some(address);
    }

    /// Parse ACPI tables (simplified)
    pub fn parse(&mut self) -> AcpiParseResult {
        let tables: Vec<AcpiTableInfo> = self.tables.values().cloned().collect();
        let cpu_count = self.madt_entries.iter()
            .filter(|e| matches!(e.entry_type, MadtEntryType::LocalApic | MadtEntryType::LocalX2Apic))
            .count() as u32;
        let numa_node_count = self.srat_entries.iter()
            .map(|e| e.proximity_domain)
            .collect::<alloc::collections::BTreeSet<_>>()
            .len() as u32;

        AcpiParseResult {
            revision: self.revision,
            tables,
            madt_entries: self.madt_entries.clone(),
            srat_entries: self.srat_entries.clone(),
            cpu_count,
            numa_node_count,
            errors: self.errors.clone(),
        }
    }

    /// Register table
    #[inline(always)]
    pub fn register_table(&mut self, info: AcpiTableInfo) {
        self.tables.insert(info.signature, info);
        self.tables_parsed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get table info
    #[inline(always)]
    pub fn get_table(&self, signature: AcpiSignature) -> Option<&AcpiTableInfo> {
        self.tables.get(&signature)
    }

    /// Check if table exists
    #[inline(always)]
    pub fn has_table(&self, signature: AcpiSignature) -> bool {
        self.tables.contains_key(&signature)
    }

    /// Add MADT entry
    #[inline(always)]
    pub fn add_madt_entry(&mut self, entry: MadtEntry) {
        self.madt_entries.push(entry);
    }

    /// Add SRAT entry
    #[inline(always)]
    pub fn add_srat_entry(&mut self, entry: SratEntry) {
        self.srat_entries.push(entry);
    }

    /// Set revision
    #[inline(always)]
    pub fn set_revision(&mut self, revision: AcpiRevision) {
        self.revision = revision;
    }

    /// Get table count
    #[inline(always)]
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Get CPU count from MADT
    #[inline]
    pub fn cpu_count(&self) -> u32 {
        self.madt_entries.iter()
            .filter(|e| matches!(e.entry_type, MadtEntryType::LocalApic | MadtEntryType::LocalX2Apic))
            .filter(|e| e.flags & 1 != 0)
            .count() as u32
    }
}

impl Default for AcpiParser {
    fn default() -> Self {
        Self::new()
    }
}
