// SPDX-License-Identifier: GPL-2.0
//! Holistic irq_domain — IRQ domain hierarchy and mapping management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IRQ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    Edge,
    Level,
    Msi,
    MsiX,
    Ipi,
    Nmi,
    Smi,
    Pmi,
}

/// IRQ trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqTrigger {
    RisingEdge,
    FallingEdge,
    BothEdges,
    HighLevel,
    LowLevel,
}

/// IRQ delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqDelivery {
    Fixed,
    LowestPriority,
    Smi,
    Nmi,
    Init,
    ExtInt,
}

/// IRQ state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqState {
    Inactive,
    Active,
    Pending,
    Masked,
    InService,
    Disabled,
}

/// IRQ descriptor within a domain
#[derive(Debug, Clone)]
pub struct IrqDesc {
    pub hwirq: u32,
    pub virq: u32,
    pub irq_type: IrqType,
    pub trigger: IrqTrigger,
    pub delivery: IrqDelivery,
    pub state: IrqState,
    pub target_cpu: u32,
    pub affinity_mask: u64,
    pub fire_count: u64,
    pub last_fire: u64,
    pub handler_time_ns: u64,
    pub spurious_count: u32,
}

impl IrqDesc {
    pub fn new(hwirq: u32, virq: u32, irq_type: IrqType) -> Self {
        Self {
            hwirq, virq, irq_type,
            trigger: IrqTrigger::RisingEdge,
            delivery: IrqDelivery::Fixed,
            state: IrqState::Inactive,
            target_cpu: 0, affinity_mask: 1,
            fire_count: 0, last_fire: 0,
            handler_time_ns: 0, spurious_count: 0,
        }
    }

    #[inline]
    pub fn fire(&mut self, now: u64) {
        self.fire_count += 1;
        self.last_fire = now;
        self.state = IrqState::InService;
    }

    #[inline(always)]
    pub fn eoi(&mut self, handler_ns: u64) {
        self.handler_time_ns += handler_ns;
        self.state = IrqState::Inactive;
    }

    #[inline(always)]
    pub fn mask(&mut self) { self.state = IrqState::Masked; }
    #[inline(always)]
    pub fn unmask(&mut self) { self.state = IrqState::Inactive; }

    #[inline(always)]
    pub fn avg_handler_ns(&self) -> u64 {
        if self.fire_count == 0 { 0 } else { self.handler_time_ns / self.fire_count }
    }

    #[inline(always)]
    pub fn rate_per_sec(&self, window_ns: u64) -> f64 {
        if window_ns == 0 { return 0.0; }
        self.fire_count as f64 / (window_ns as f64 / 1_000_000_000.0)
    }
}

/// IRQ domain (interrupt controller hierarchy)
#[derive(Debug)]
pub struct IrqDomain {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub name_hash: u64,
    pub hwirq_base: u32,
    pub hwirq_count: u32,
    pub virq_base: u32,
    pub irqs: BTreeMap<u32, IrqDesc>,
    pub total_fires: u64,
}

impl IrqDomain {
    pub fn new(id: u32, hwirq_base: u32, count: u32, virq_base: u32) -> Self {
        Self {
            id, parent_id: None, name_hash: id as u64,
            hwirq_base, hwirq_count: count, virq_base,
            irqs: BTreeMap::new(), total_fires: 0,
        }
    }

    #[inline]
    pub fn map_irq(&mut self, hwirq: u32, irq_type: IrqType) -> u32 {
        let virq = self.virq_base + (hwirq - self.hwirq_base);
        self.irqs.insert(hwirq, IrqDesc::new(hwirq, virq, irq_type));
        virq
    }

    #[inline(always)]
    pub fn unmap_irq(&mut self, hwirq: u32) -> bool { self.irqs.remove(&hwirq).is_some() }

    #[inline]
    pub fn fire_irq(&mut self, hwirq: u32, now: u64) -> bool {
        if let Some(desc) = self.irqs.get_mut(&hwirq) {
            desc.fire(now);
            self.total_fires += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn busiest_irqs(&self, n: usize) -> Vec<(u32, u64)> {
        let mut v: Vec<_> = self.irqs.iter().map(|(&hw, d)| (hw, d.fire_count)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }
}

/// IRQ domain stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IrqDomainStats {
    pub total_domains: u32,
    pub total_mapped_irqs: u32,
    pub total_fires: u64,
    pub total_spurious: u64,
    pub avg_handler_ns: u64,
}

/// Main IRQ domain manager
pub struct HolisticIrqDomain {
    domains: BTreeMap<u32, IrqDomain>,
    next_id: u32,
    next_virq: u32,
}

impl HolisticIrqDomain {
    pub fn new() -> Self {
        Self { domains: BTreeMap::new(), next_id: 1, next_virq: 32 }
    }

    #[inline]
    pub fn create_domain(&mut self, hwirq_base: u32, count: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let virq_base = self.next_virq;
        self.next_virq += count;
        self.domains.insert(id, IrqDomain::new(id, hwirq_base, count, virq_base));
        id
    }

    #[inline(always)]
    pub fn set_parent(&mut self, domain_id: u32, parent_id: u32) {
        if let Some(d) = self.domains.get_mut(&domain_id) { d.parent_id = Some(parent_id); }
    }

    #[inline(always)]
    pub fn map_irq(&mut self, domain_id: u32, hwirq: u32, irq_type: IrqType) -> Option<u32> {
        self.domains.get_mut(&domain_id).map(|d| d.map_irq(hwirq, irq_type))
    }

    #[inline(always)]
    pub fn fire(&mut self, domain_id: u32, hwirq: u32, now: u64) -> bool {
        self.domains.get_mut(&domain_id).map(|d| d.fire_irq(hwirq, now)).unwrap_or(false)
    }

    pub fn stats(&self) -> IrqDomainStats {
        let total_irqs: u32 = self.domains.values().map(|d| d.irqs.len() as u32).sum();
        let total_fires: u64 = self.domains.values().map(|d| d.total_fires).sum();
        let all_descs: Vec<&IrqDesc> = self.domains.values().flat_map(|d| d.irqs.values()).collect();
        let total_spurious: u64 = all_descs.iter().map(|d| d.spurious_count as u64).sum();
        let avg_handler = if all_descs.is_empty() { 0 } else {
            all_descs.iter().map(|d| d.avg_handler_ns()).sum::<u64>() / all_descs.len() as u64
        };
        IrqDomainStats {
            total_domains: self.domains.len() as u32, total_mapped_irqs: total_irqs,
            total_fires, total_spurious, avg_handler_ns: avg_handler,
        }
    }
}
