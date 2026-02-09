//! # Bridge IRQ Bridge
//!
//! Bridges interrupt request (IRQ) handling:
//! - IRQ domain management
//! - IRQ descriptor allocation
//! - Affinity configuration
//! - Threaded IRQ support
//! - IRQ chip abstraction
//! - Spurious IRQ detection

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// IRQ trigger type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqTrigger {
    None,
    Rising,
    Falling,
    Both,
    High,
    Low,
}

/// IRQ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    Legacy,
    Msi,
    MsiX,
    Ipi,
    Nmi,
    Pmu,
    Timer,
    Software,
}

/// IRQ handler return
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqReturn {
    None,
    Handled,
    WakeThread,
}

/// IRQ state flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqStateFlag {
    Disabled,
    Masked,
    InProgress,
    Pending,
    Affinity,
    PerCpu,
    NoBalancing,
    NoThread,
}

/// IRQ descriptor
#[derive(Debug, Clone)]
pub struct IrqDesc {
    pub irq: u32,
    pub hwirq: u64,
    pub name: String,
    pub irq_type: IrqType,
    pub trigger: IrqTrigger,
    pub chip_name: String,
    pub domain_id: u64,
    pub affinity_mask: u64,
    pub effective_affinity: u64,
    pub handler_count: u64,
    pub spurious_count: u64,
    pub thread_count: u32,
    pub flags: Vec<IrqStateFlag>,
    pub depth: u32,
    pub last_unhandled: u64,
    pub wakeup_enabled: bool,
}

impl IrqDesc {
    pub fn new(irq: u32, hwirq: u64, name: String, itype: IrqType) -> Self {
        Self {
            irq, hwirq, name, irq_type: itype, trigger: IrqTrigger::None,
            chip_name: String::new(), domain_id: 0, affinity_mask: u64::MAX,
            effective_affinity: 1, handler_count: 0, spurious_count: 0,
            thread_count: 0, flags: Vec::new(), depth: 0,
            last_unhandled: 0, wakeup_enabled: false,
        }
    }

    #[inline(always)]
    pub fn enable(&mut self) { self.flags.retain(|f| *f != IrqStateFlag::Disabled); self.depth = 0; }
    #[inline(always)]
    pub fn disable(&mut self) { if !self.flags.contains(&IrqStateFlag::Disabled) { self.flags.push(IrqStateFlag::Disabled); } self.depth += 1; }
    #[inline(always)]
    pub fn is_disabled(&self) -> bool { self.flags.contains(&IrqStateFlag::Disabled) }
    #[inline(always)]
    pub fn mask(&mut self) { if !self.flags.contains(&IrqStateFlag::Masked) { self.flags.push(IrqStateFlag::Masked); } }
    #[inline(always)]
    pub fn unmask(&mut self) { self.flags.retain(|f| *f != IrqStateFlag::Masked); }
    #[inline(always)]
    pub fn set_affinity(&mut self, mask: u64) { self.affinity_mask = mask; }
    #[inline(always)]
    pub fn handle(&mut self) { self.handler_count += 1; }
    #[inline(always)]
    pub fn spurious(&mut self, ts: u64) { self.spurious_count += 1; self.last_unhandled = ts; }
    #[inline(always)]
    pub fn is_spurious_prone(&self) -> bool { self.handler_count > 0 && self.spurious_count * 100 / (self.handler_count + 1) > 5 }
}

/// IRQ domain
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IrqDomain {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub hwirq_max: u64,
    pub mapped: LinearMap<u32, 64>,
    pub revmap_size: u32,
}

impl IrqDomain {
    pub fn new(id: u64, name: String, hwirq_max: u64) -> Self {
        Self { id, name, parent_id: None, hwirq_max, mapped: LinearMap::new(), revmap_size: 0 }
    }

    #[inline(always)]
    pub fn map(&mut self, hwirq: u64, virq: u32) { self.mapped.insert(hwirq, virq); self.revmap_size = self.mapped.len() as u32; }
    #[inline(always)]
    pub fn unmap(&mut self, hwirq: u64) { self.mapped.remove(hwirq); self.revmap_size = self.mapped.len() as u32; }
    #[inline(always)]
    pub fn resolve(&self, hwirq: u64) -> Option<u32> { self.mapped.get(hwirq).copied() }
}

/// Per-CPU IRQ stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuIrqStats {
    pub cpu_id: u32,
    pub total_irqs: u64,
    pub total_softirqs: u64,
    pub nmi_count: u64,
    pub ipi_count: u64,
    pub timer_count: u64,
    pub per_irq: ArrayMap<u64, 32>,
}

impl CpuIrqStats {
    pub fn new(cpu: u32) -> Self {
        Self { cpu_id: cpu, total_irqs: 0, total_softirqs: 0, nmi_count: 0, ipi_count: 0, timer_count: 0, per_irq: ArrayMap::new(0) }
    }

    #[inline(always)]
    pub fn record(&mut self, irq: u32) {
        self.total_irqs += 1;
        self.per_irq.add(irq as usize, 1);
    }
}

/// IRQ bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IrqBridgeStats {
    pub total_irqs: usize,
    pub total_domains: usize,
    pub total_handled: u64,
    pub total_spurious: u64,
    pub disabled_count: usize,
    pub msi_count: usize,
}

/// Bridge IRQ manager
#[repr(align(64))]
pub struct BridgeIrqBridge {
    descs: BTreeMap<u32, IrqDesc>,
    domains: BTreeMap<u64, IrqDomain>,
    cpu_stats: BTreeMap<u32, CpuIrqStats>,
    stats: IrqBridgeStats,
    next_domain: u64,
    next_virq: u32,
}

impl BridgeIrqBridge {
    pub fn new() -> Self {
        Self { descs: BTreeMap::new(), domains: BTreeMap::new(), cpu_stats: BTreeMap::new(), stats: IrqBridgeStats::default(), next_domain: 1, next_virq: 32 }
    }

    #[inline]
    pub fn create_domain(&mut self, name: String, hwirq_max: u64) -> u64 {
        let id = self.next_domain; self.next_domain += 1;
        self.domains.insert(id, IrqDomain::new(id, name, hwirq_max));
        id
    }

    #[inline]
    pub fn alloc_irq(&mut self, hwirq: u64, name: String, itype: IrqType, domain: u64) -> u32 {
        let virq = self.next_virq; self.next_virq += 1;
        let mut desc = IrqDesc::new(virq, hwirq, name, itype);
        desc.domain_id = domain;
        self.descs.insert(virq, desc);
        if let Some(d) = self.domains.get_mut(&domain) { d.map(hwirq, virq); }
        virq
    }

    #[inline]
    pub fn free_irq(&mut self, virq: u32) {
        if let Some(desc) = self.descs.remove(&virq) {
            if let Some(d) = self.domains.get_mut(&desc.domain_id) { d.unmap(desc.hwirq); }
        }
    }

    #[inline]
    pub fn handle_irq(&mut self, virq: u32, cpu: u32) -> IrqReturn {
        if let Some(desc) = self.descs.get_mut(&virq) {
            if desc.is_disabled() { return IrqReturn::None; }
            desc.handle();
            if let Some(cs) = self.cpu_stats.get_mut(&cpu) { cs.record(virq); }
            if desc.thread_count > 0 { IrqReturn::WakeThread } else { IrqReturn::Handled }
        } else { IrqReturn::None }
    }

    #[inline(always)]
    pub fn set_affinity(&mut self, virq: u32, mask: u64) { if let Some(d) = self.descs.get_mut(&virq) { d.set_affinity(mask); } }
    #[inline(always)]
    pub fn enable(&mut self, virq: u32) { if let Some(d) = self.descs.get_mut(&virq) { d.enable(); } }
    #[inline(always)]
    pub fn disable(&mut self, virq: u32) { if let Some(d) = self.descs.get_mut(&virq) { d.disable(); } }
    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: u32) { self.cpu_stats.insert(cpu, CpuIrqStats::new(cpu)); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_irqs = self.descs.len();
        self.stats.total_domains = self.domains.len();
        self.stats.total_handled = self.descs.values().map(|d| d.handler_count).sum();
        self.stats.total_spurious = self.descs.values().map(|d| d.spurious_count).sum();
        self.stats.disabled_count = self.descs.values().filter(|d| d.is_disabled()).count();
        self.stats.msi_count = self.descs.values().filter(|d| d.irq_type == IrqType::Msi || d.irq_type == IrqType::MsiX).count();
    }

    #[inline(always)]
    pub fn desc(&self, virq: u32) -> Option<&IrqDesc> { self.descs.get(&virq) }
    #[inline(always)]
    pub fn domain(&self, id: u64) -> Option<&IrqDomain> { self.domains.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &IrqBridgeStats { &self.stats }
}
