// SPDX-License-Identifier: GPL-2.0
//! Holistic irq_affinity — IRQ affinity and balancing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Balance mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqBalanceMode {
    None,
    PowerSave,
    Performance,
    Exact,
}

/// IRQ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqSourceType {
    Msi,
    MsiX,
    Legacy,
    Ioapic,
}

/// IRQ descriptor
#[derive(Debug)]
pub struct IrqDesc {
    pub irq: u32,
    pub source: IrqSourceType,
    pub affinity_mask: u64,
    pub effective_affinity: u32,
    pub name_hash: u64,
    pub count: u64,
    pub rate: u64,
    pub spurious: u64,
    pub last_cpu: u32,
    pub enabled: bool,
}

impl IrqDesc {
    pub fn new(irq: u32, source: IrqSourceType) -> Self {
        Self { irq, source, affinity_mask: u64::MAX, effective_affinity: 0, name_hash: irq as u64, count: 0, rate: 0, spurious: 0, last_cpu: 0, enabled: true }
    }

    pub fn handle(&mut self, cpu: u32) { self.count += 1; self.last_cpu = cpu; }
    pub fn set_affinity(&mut self, mask: u64) { self.affinity_mask = mask; }
}

/// CPU IRQ load
#[derive(Debug)]
pub struct CpuIrqLoad {
    pub cpu: u32,
    pub irq_count: u64,
    pub irqs: Vec<u32>,
    pub softirq_time_ns: u64,
}

impl CpuIrqLoad {
    pub fn new(cpu: u32) -> Self { Self { cpu, irq_count: 0, irqs: Vec::new(), softirq_time_ns: 0 } }
    pub fn load(&self) -> u64 { self.irq_count }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IrqAffinityStats {
    pub total_irqs: u32,
    pub total_cpus: u32,
    pub total_interrupts: u64,
    pub balance_mode: u8,
    pub imbalance_ratio: f64,
}

/// Main IRQ affinity manager
pub struct HolisticIrqAffinity {
    irqs: BTreeMap<u32, IrqDesc>,
    cpu_loads: BTreeMap<u32, CpuIrqLoad>,
    mode: IrqBalanceMode,
}

impl HolisticIrqAffinity {
    pub fn new() -> Self { Self { irqs: BTreeMap::new(), cpu_loads: BTreeMap::new(), mode: IrqBalanceMode::Performance } }

    pub fn add_irq(&mut self, irq: u32, source: IrqSourceType) { self.irqs.insert(irq, IrqDesc::new(irq, source)); }
    pub fn add_cpu(&mut self, cpu: u32) { self.cpu_loads.insert(cpu, CpuIrqLoad::new(cpu)); }

    pub fn handle_irq(&mut self, irq: u32, cpu: u32) {
        if let Some(desc) = self.irqs.get_mut(&irq) { desc.handle(cpu); }
        if let Some(load) = self.cpu_loads.get_mut(&cpu) { load.irq_count += 1; }
    }

    pub fn stats(&self) -> IrqAffinityStats {
        let total: u64 = self.irqs.values().map(|i| i.count).sum();
        let loads: Vec<u64> = self.cpu_loads.values().map(|c| c.load()).collect();
        let (min, max) = if loads.is_empty() { (0, 1) } else { (*loads.iter().min().unwrap(), *loads.iter().max().unwrap().max(&1)) };
        let imbalance = if max == 0 { 0.0 } else { (max - min) as f64 / max as f64 };
        IrqAffinityStats { total_irqs: self.irqs.len() as u32, total_cpus: self.cpu_loads.len() as u32, total_interrupts: total, balance_mode: self.mode as u8, imbalance_ratio: imbalance }
    }
}
