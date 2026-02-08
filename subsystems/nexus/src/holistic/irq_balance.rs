//! # Holistic IRQ Balance Engine
//!
//! System-wide interrupt distribution optimization:
//! - IRQ-to-CPU affinity management
//! - NAPI/polling mode switching for high-throughput
//! - MSI-X vector distribution
//! - IRQ coalescing tuning
//! - Cross-CPU interrupt migration
//! - Cache-affinity aware IRQ placement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IRQ type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    LevelTriggered,
    EdgeTriggered,
    Msi,
    MsiX,
    Ipi,
    Timer,
    Spurious,
}

/// IRQ delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqDeliveryMode {
    Fixed,
    LowestPriority,
    Smi,
    Nmi,
    Init,
    ExtInt,
}

/// IRQ coalescing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceStrategy {
    Disabled,
    Static(u32),        // fixed interval in us
    Adaptive,
    EthtoolCompatible,
}

/// Per-IRQ descriptor
#[derive(Debug, Clone)]
pub struct IrqDescriptor {
    pub irq_number: u32,
    pub irq_type: IrqType,
    pub delivery: IrqDeliveryMode,
    pub affinity_cpu: u32,
    pub device_id: u32,
    pub handler_ns_avg: u64,
    pub handler_ns_max: u64,
    pub fire_count: u64,
    pub spurious_count: u64,
    pub coalesce: CoalesceStrategy,
    pub last_migration_ns: u64,
}

impl IrqDescriptor {
    pub fn new(irq: u32, irq_type: IrqType, device_id: u32) -> Self {
        Self {
            irq_number: irq,
            irq_type,
            delivery: IrqDeliveryMode::LowestPriority,
            affinity_cpu: 0,
            device_id,
            handler_ns_avg: 0,
            handler_ns_max: 0,
            fire_count: 0,
            spurious_count: 0,
            coalesce: CoalesceStrategy::Disabled,
            last_migration_ns: 0,
        }
    }

    pub fn is_high_rate(&self) -> bool {
        self.fire_count > 100_000 // per second approximation
    }

    pub fn cpu_time_ns(&self) -> u64 {
        self.handler_ns_avg * self.fire_count
    }

    pub fn spurious_ratio(&self) -> f64 {
        if self.fire_count == 0 { return 0.0; }
        self.spurious_count as f64 / self.fire_count as f64
    }
}

/// Per-CPU IRQ load
#[derive(Debug, Clone)]
pub struct CpuIrqLoad {
    pub cpu_id: u32,
    pub assigned_irqs: Vec<u32>,
    pub total_irq_time_ns: u64,
    pub total_irq_count: u64,
    pub softirq_time_ns: u64,
    pub numa_node: u32,
}

impl CpuIrqLoad {
    pub fn new(cpu_id: u32, numa_node: u32) -> Self {
        Self {
            cpu_id,
            assigned_irqs: Vec::new(),
            total_irq_time_ns: 0,
            total_irq_count: 0,
            softirq_time_ns: 0,
            numa_node,
        }
    }

    pub fn total_overhead_ns(&self) -> u64 {
        self.total_irq_time_ns + self.softirq_time_ns
    }
}

/// IRQ migration suggestion
#[derive(Debug, Clone)]
pub struct IrqMigration {
    pub irq_number: u32,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub expected_benefit_ns: u64,
    pub reason: IrqMigrationReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqMigrationReason {
    LoadBalance,
    NumaLocality,
    CacheAffinity,
    PowerSaving,
    IsolationRequired,
}

/// MSI-X vector assignment
#[derive(Debug, Clone)]
pub struct MsiXAssignment {
    pub device_id: u32,
    pub vectors: Vec<(u32, u32)>, // (vector_idx, cpu_id)
    pub max_vectors: u32,
}

impl MsiXAssignment {
    pub fn new(device_id: u32, max_vectors: u32) -> Self {
        Self {
            device_id,
            vectors: Vec::new(),
            max_vectors,
        }
    }

    pub fn spread_across(&mut self, cpus: &[u32]) {
        self.vectors.clear();
        for (i, &cpu) in cpus.iter().enumerate() {
            if i as u32 >= self.max_vectors { break; }
            self.vectors.push((i as u32, cpu));
        }
    }
}

/// Holistic IRQ Balance stats
#[derive(Debug, Clone, Default)]
pub struct HolisticIrqBalanceStats {
    pub total_irqs: usize,
    pub total_cpus: usize,
    pub max_cpu_irq_load_ns: u64,
    pub min_cpu_irq_load_ns: u64,
    pub imbalance_ratio: f64,
    pub total_migrations: u64,
    pub high_rate_irqs: usize,
}

/// Holistic IRQ Balance Engine
pub struct HolisticIrqBalance {
    irqs: BTreeMap<u32, IrqDescriptor>,
    cpus: BTreeMap<u32, CpuIrqLoad>,
    msix: BTreeMap<u32, MsiXAssignment>,
    migrations: u64,
    stats: HolisticIrqBalanceStats,
}

impl HolisticIrqBalance {
    pub fn new() -> Self {
        Self {
            irqs: BTreeMap::new(),
            cpus: BTreeMap::new(),
            msix: BTreeMap::new(),
            migrations: 0,
            stats: HolisticIrqBalanceStats::default(),
        }
    }

    pub fn register_irq(&mut self, desc: IrqDescriptor) {
        let cpu = desc.affinity_cpu;
        let irq = desc.irq_number;
        self.irqs.insert(irq, desc);
        if let Some(cpu_load) = self.cpus.get_mut(&cpu) {
            if !cpu_load.assigned_irqs.contains(&irq) {
                cpu_load.assigned_irqs.push(irq);
            }
        }
    }

    pub fn register_cpu(&mut self, load: CpuIrqLoad) {
        self.cpus.insert(load.cpu_id, load);
    }

    pub fn register_msix(&mut self, assignment: MsiXAssignment) {
        self.msix.insert(assignment.device_id, assignment);
    }

    /// Compute IRQ load per CPU
    fn compute_loads(&mut self) {
        for cpu in self.cpus.values_mut() {
            cpu.total_irq_time_ns = 0;
            cpu.total_irq_count = 0;
            for &irq_num in &cpu.assigned_irqs {
                if let Some(irq) = self.irqs.get(&irq_num) {
                    cpu.total_irq_time_ns += irq.cpu_time_ns();
                    cpu.total_irq_count += irq.fire_count;
                }
            }
        }
    }

    /// Generate migration suggestions to balance IRQ load
    pub fn compute_balance(&mut self) -> Vec<IrqMigration> {
        self.compute_loads();
        let mut suggestions = Vec::new();

        let max_load = self.cpus.values().map(|c| c.total_irq_time_ns).max().unwrap_or(0);
        let min_load = self.cpus.values().map(|c| c.total_irq_time_ns).min().unwrap_or(0);
        let avg_load = if self.cpus.is_empty() { 0 } else {
            self.cpus.values().map(|c| c.total_irq_time_ns).sum::<u64>() / self.cpus.len() as u64
        };

        // Find overloaded CPUs
        let threshold = avg_load + avg_load / 4;
        let overloaded: Vec<u32> = self.cpus.iter()
            .filter(|(_, c)| c.total_irq_time_ns > threshold)
            .map(|(&id, _)| id)
            .collect();

        let underloaded: Vec<u32> = self.cpus.iter()
            .filter(|(_, c)| c.total_irq_time_ns < avg_load)
            .map(|(&id, _)| id)
            .collect();

        for &src_cpu in &overloaded {
            if underloaded.is_empty() { break; }
            let irqs_on_cpu: Vec<u32> = self.cpus.get(&src_cpu)
                .map(|c| c.assigned_irqs.clone())
                .unwrap_or_default();

            for &irq_num in &irqs_on_cpu {
                if let Some(irq) = self.irqs.get(&irq_num) {
                    // Find best target CPU (NUMA-local and underloaded)
                    let src_numa = self.cpus.get(&src_cpu).map(|c| c.numa_node).unwrap_or(0);
                    let best_target = underloaded.iter()
                        .filter(|&&cpu| {
                            self.cpus.get(&cpu).map(|c| c.numa_node).unwrap_or(0) == src_numa
                        })
                        .min_by_key(|&&cpu| {
                            self.cpus.get(&cpu).map(|c| c.total_irq_time_ns).unwrap_or(u64::MAX)
                        })
                        .or_else(|| underloaded.first())
                        .copied();

                    if let Some(target) = best_target {
                        suggestions.push(IrqMigration {
                            irq_number: irq_num,
                            from_cpu: src_cpu,
                            to_cpu: target,
                            expected_benefit_ns: irq.cpu_time_ns(),
                            reason: IrqMigrationReason::LoadBalance,
                        });
                        break; // One migration per overloaded CPU per pass
                    }
                }
            }
        }

        self.recompute(max_load, min_load);
        suggestions
    }

    fn recompute(&mut self, max_load: u64, min_load: u64) {
        self.stats.total_irqs = self.irqs.len();
        self.stats.total_cpus = self.cpus.len();
        self.stats.max_cpu_irq_load_ns = max_load;
        self.stats.min_cpu_irq_load_ns = min_load;
        let avg = if self.cpus.is_empty() { 1 } else {
            self.cpus.values().map(|c| c.total_irq_time_ns).sum::<u64>() / self.cpus.len() as u64
        };
        self.stats.imbalance_ratio = if avg > 0 {
            (max_load - min_load) as f64 / avg as f64
        } else { 0.0 };
        self.stats.total_migrations = self.migrations;
        self.stats.high_rate_irqs = self.irqs.values().filter(|i| i.is_high_rate()).count();
    }

    pub fn irq(&self, num: u32) -> Option<&IrqDescriptor> { self.irqs.get(&num) }
    pub fn cpu_load(&self, id: u32) -> Option<&CpuIrqLoad> { self.cpus.get(&id) }
    pub fn stats(&self) -> &HolisticIrqBalanceStats { &self.stats }
}
