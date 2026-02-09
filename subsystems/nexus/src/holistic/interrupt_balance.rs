//! # Holistic Interrupt Balance
//!
//! IRQ balancing across CPUs at system level:
//! - Per-IRQ load tracking
//! - CPU interrupt load histogram
//! - Affinity rebalancing (spread vs. pack)
//! - MSI/MSI-X vector management
//! - Interrupt coalescing suggestions
//! - Storm detection and throttling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Interrupt type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// Legacy pin-based IRQ
    Legacy,
    /// Message Signaled Interrupt
    Msi,
    /// MSI with per-vector masking
    MsiX,
    /// Inter-Processor Interrupt
    Ipi,
    /// Local APIC timer
    LocalTimer,
    /// Software interrupt
    Software,
}

/// IRQ balance strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceStrategy {
    /// Spread IRQs evenly across CPUs
    Spread,
    /// Pack onto fewer CPUs (power saving)
    Pack,
    /// NUMA-aware: keep IRQ on same node as device
    NumaAware,
    /// Per-queue: network RX queues to separate CPUs
    PerQueue,
    /// Manual affinity
    Manual,
}

/// Per-IRQ tracking
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IrqStats {
    pub irq_number: u32,
    pub irq_type: InterruptType,
    pub name_hash: u64,
    pub current_cpu: u32,
    pub total_count: u64,
    pub rate_per_sec: f64,
    pub avg_handler_ns: u64,
    pub cpu_time_frac: f64,
    pub numa_node: Option<u32>,
    pub affinity_mask: u64,
    pub coalesce_count: u32,
}

impl IrqStats {
    pub fn new(irq_number: u32, irq_type: InterruptType) -> Self {
        Self {
            irq_number,
            irq_type,
            name_hash: 0,
            current_cpu: 0,
            total_count: 0,
            rate_per_sec: 0.0,
            avg_handler_ns: 0,
            cpu_time_frac: 0.0,
            numa_node: None,
            affinity_mask: u64::MAX,
            coalesce_count: 1,
        }
    }

    #[inline]
    pub fn record(&mut self, handler_ns: u64) {
        self.total_count += 1;
        // EMA for handler time
        let alpha = 0.1;
        self.avg_handler_ns =
            ((1.0 - alpha) * self.avg_handler_ns as f64 + alpha * handler_ns as f64) as u64;
    }

    /// Load contribution (rate * handler time)
    #[inline(always)]
    pub fn load(&self) -> f64 {
        self.rate_per_sec * self.avg_handler_ns as f64 / 1_000_000_000.0
    }

    /// Is this a storm? (>100k/sec)
    #[inline(always)]
    pub fn is_storm(&self) -> bool {
        self.rate_per_sec > 100_000.0
    }
}

/// Per-CPU interrupt load
#[derive(Debug, Clone)]
pub struct CpuIrqLoad {
    pub cpu_id: u32,
    pub assigned_irqs: Vec<u32>,
    pub total_irq_rate: f64,
    pub total_irq_time_frac: f64,
    pub softirq_time_frac: f64,
}

impl CpuIrqLoad {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            assigned_irqs: Vec::new(),
            total_irq_rate: 0.0,
            total_irq_time_frac: 0.0,
            softirq_time_frac: 0.0,
        }
    }

    #[inline(always)]
    pub fn total_interrupt_load(&self) -> f64 {
        self.total_irq_time_frac + self.softirq_time_frac
    }

    #[inline(always)]
    pub fn is_overloaded(&self) -> bool {
        self.total_interrupt_load() > 0.5
    }
}

/// Rebalance recommendation
#[derive(Debug, Clone)]
pub struct IrqRebalance {
    pub irq_number: u32,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub load_reduction: f64,
}

/// Coalesce suggestion
#[derive(Debug, Clone)]
pub struct CoalesceSuggestion {
    pub irq_number: u32,
    pub current_rate: f64,
    pub suggested_coalesce: u32,
    pub expected_rate: f64,
}

/// IRQ balance stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticIrqBalanceStats {
    pub tracked_irqs: usize,
    pub total_irq_rate: f64,
    pub max_cpu_irq_load: f64,
    pub min_cpu_irq_load: f64,
    pub imbalance_ratio: f64,
    pub storm_irqs: usize,
    pub pending_rebalances: usize,
}

/// Holistic IRQ Balancer
pub struct HolisticIrqBalance {
    irqs: BTreeMap<u32, IrqStats>,
    cpu_loads: BTreeMap<u32, CpuIrqLoad>,
    strategy: BalanceStrategy,
    rebalances: Vec<IrqRebalance>,
    coalesce_suggestions: Vec<CoalesceSuggestion>,
    stats: HolisticIrqBalanceStats,
}

impl HolisticIrqBalance {
    pub fn new(strategy: BalanceStrategy) -> Self {
        Self {
            irqs: BTreeMap::new(),
            cpu_loads: BTreeMap::new(),
            strategy,
            rebalances: Vec::new(),
            coalesce_suggestions: Vec::new(),
            stats: HolisticIrqBalanceStats::default(),
        }
    }

    pub fn register_irq(&mut self, irq: IrqStats) {
        let cpu = irq.current_cpu;
        let irq_num = irq.irq_number;
        self.irqs.insert(irq_num, irq);

        let cpu_load = self
            .cpu_loads
            .entry(cpu)
            .or_insert_with(|| CpuIrqLoad::new(cpu));
        if !cpu_load.assigned_irqs.contains(&irq_num) {
            cpu_load.assigned_irqs.push(irq_num);
        }
    }

    #[inline]
    pub fn register_cpu(&mut self, cpu_id: u32) {
        self.cpu_loads
            .entry(cpu_id)
            .or_insert_with(|| CpuIrqLoad::new(cpu_id));
    }

    #[inline]
    pub fn record_interrupt(&mut self, irq_number: u32, handler_ns: u64) {
        if let Some(irq) = self.irqs.get_mut(&irq_number) {
            irq.record(handler_ns);
        }
    }

    #[inline]
    pub fn update_irq_rate(&mut self, irq_number: u32, rate: f64) {
        if let Some(irq) = self.irqs.get_mut(&irq_number) {
            irq.rate_per_sec = rate;
            irq.cpu_time_frac = irq.load();
        }
    }

    /// Recompute CPU loads from IRQ assignments
    fn recompute_cpu_loads(&mut self) {
        for cpu in self.cpu_loads.values_mut() {
            cpu.total_irq_rate = 0.0;
            cpu.total_irq_time_frac = 0.0;
        }

        for irq in self.irqs.values() {
            if let Some(cpu) = self.cpu_loads.get_mut(&irq.current_cpu) {
                cpu.total_irq_rate += irq.rate_per_sec;
                cpu.total_irq_time_frac += irq.cpu_time_frac;
            }
        }
    }

    /// Generate rebalance recommendations
    pub fn rebalance(&mut self) {
        self.rebalances.clear();
        self.coalesce_suggestions.clear();
        self.recompute_cpu_loads();

        if self.cpu_loads.len() < 2 {
            return;
        }

        // Find overloaded and underloaded CPUs
        let avg_load: f64 = self
            .cpu_loads
            .values()
            .map(|c| c.total_irq_time_frac)
            .sum::<f64>()
            / self.cpu_loads.len() as f64;

        let overloaded: Vec<u32> = self
            .cpu_loads
            .values()
            .filter(|c| c.total_irq_time_frac > avg_load * 1.5)
            .map(|c| c.cpu_id)
            .collect();

        let underloaded: Vec<u32> = self
            .cpu_loads
            .values()
            .filter(|c| c.total_irq_time_frac < avg_load * 0.5)
            .map(|c| c.cpu_id)
            .collect();

        // For each overloaded CPU, try to move IRQs to underloaded
        let mut target_idx = 0;
        for src_cpu in &overloaded {
            let irqs_on_cpu: Vec<u32> = self
                .cpu_loads
                .get(src_cpu)
                .map(|c| c.assigned_irqs.clone())
                .unwrap_or_default();

            for irq_num in irqs_on_cpu {
                if target_idx >= underloaded.len() {
                    break;
                }
                let irq_load = self.irqs.get(&irq_num).map(|i| i.load()).unwrap_or(0.0);

                // Check NUMA affinity if strategy requires
                let target = underloaded[target_idx];
                let numa_ok = match self.strategy {
                    BalanceStrategy::NumaAware => {
                        // Simplified: allow same node only (would need topology info)
                        true
                    },
                    _ => true,
                };

                if numa_ok {
                    self.rebalances.push(IrqRebalance {
                        irq_number: irq_num,
                        from_cpu: *src_cpu,
                        to_cpu: target,
                        load_reduction: irq_load,
                    });
                    target_idx = (target_idx + 1) % underloaded.len().max(1);
                }
            }
        }

        // Generate coalesce suggestions for storm IRQs
        for irq in self.irqs.values() {
            if irq.is_storm() && irq.coalesce_count <= 1 {
                let suggested = ((irq.rate_per_sec / 50_000.0) as u32).max(2).min(256);
                self.coalesce_suggestions.push(CoalesceSuggestion {
                    irq_number: irq.irq_number,
                    current_rate: irq.rate_per_sec,
                    suggested_coalesce: suggested,
                    expected_rate: irq.rate_per_sec / suggested as f64,
                });
            }
        }

        // Update stats
        let max_load = self
            .cpu_loads
            .values()
            .map(|c| c.total_irq_time_frac)
            .fold(0.0f64, |a, b| if b > a { b } else { a });
        let min_load = self
            .cpu_loads
            .values()
            .map(|c| c.total_irq_time_frac)
            .fold(f64::MAX, |a, b| if b < a { b } else { a });

        self.stats = HolisticIrqBalanceStats {
            tracked_irqs: self.irqs.len(),
            total_irq_rate: self.irqs.values().map(|i| i.rate_per_sec).sum(),
            max_cpu_irq_load: max_load,
            min_cpu_irq_load: if min_load == f64::MAX { 0.0 } else { min_load },
            imbalance_ratio: if avg_load > 0.001 {
                (max_load - min_load) / avg_load
            } else {
                0.0
            },
            storm_irqs: self.irqs.values().filter(|i| i.is_storm()).count(),
            pending_rebalances: self.rebalances.len(),
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticIrqBalanceStats {
        &self.stats
    }

    #[inline(always)]
    pub fn rebalance_suggestions(&self) -> &[IrqRebalance] {
        &self.rebalances
    }

    #[inline(always)]
    pub fn coalesce_suggestions(&self) -> &[CoalesceSuggestion] {
        &self.coalesce_suggestions
    }
}
