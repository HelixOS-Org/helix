//! Organ types and functional units.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::cell::Tissue;
use super::types::MorphogenType;

/// Organ types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganType {
    /// CPU management organ
    ProcessingCore,
    /// Memory management organ
    MemoryPool,
    /// I/O subsystem organ
    IoSubsystem,
    /// Network stack organ
    NetworkStack,
    /// Security monitoring organ
    SecurityMonitor,
}

/// Organ function status
#[derive(Debug, Clone, Default)]
pub struct OrganFunction {
    /// Current capacity
    pub capacity: f64,
    /// Current load
    pub load: f64,
    /// Efficiency
    pub efficiency: f64,
    /// Fault rate
    pub fault_rate: f64,
}

/// Organ (functional unit composed of tissues)
#[derive(Debug, Clone)]
pub struct Organ {
    /// Organ identifier
    pub id: u64,
    /// Organ type
    pub organ_type: OrganType,
    /// Tissues in this organ
    pub tissues: Vec<Tissue>,
    /// Organ function
    pub function: OrganFunction,
}

impl Organ {
    /// Create a new organ
    pub fn new(id: u64, organ_type: OrganType) -> Self {
        Self {
            id,
            organ_type,
            tissues: Vec::new(),
            function: OrganFunction::default(),
        }
    }

    /// Add tissue to organ
    #[inline(always)]
    pub fn add_tissue(&mut self, tissue: Tissue) {
        self.tissues.push(tissue);
    }

    /// Update organ
    pub fn update(&mut self, morphogens: &BTreeMap<MorphogenType, f64>, dt: f64) {
        for tissue in &mut self.tissues {
            tissue.update(morphogens, dt);
        }

        // Calculate organ function
        let total_health: f64 = self.tissues.iter().map(|t| t.state.health).sum();
        let total_throughput: f64 = self.tissues.iter().map(|t| t.state.throughput).sum();
        let num_tissues = self.tissues.len() as f64;

        if num_tissues > 0.0 {
            self.function.capacity = total_health / num_tissues;
            self.function.load = total_throughput / (total_health + 1.0);
            self.function.efficiency = (1.0 - self.function.load / 2.0).max(0.1);
            self.function.fault_rate = (1.0 - total_health / num_tissues) * 0.1;
        }
    }

    /// Get total cell count
    #[inline(always)]
    pub fn cell_count(&self) -> usize {
        self.tissues.iter().map(|t| t.size()).sum()
    }
}
