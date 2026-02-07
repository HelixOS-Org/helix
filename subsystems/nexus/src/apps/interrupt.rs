//! # App Interrupt Profiler
//!
//! Track per-process interrupt and softirq impact:
//! - IRQ affinity per process
//! - Softirq time attribution
//! - Interrupt storm detection
//! - NAPI/poll mode tracking
//! - Context switch from IRQ cost

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// IRQ TYPES
// ============================================================================

/// IRQ category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqCategory {
    /// Timer interrupt
    Timer,
    /// Disk I/O
    DiskIo,
    /// Network
    Network,
    /// GPU
    Gpu,
    /// USB
    Usb,
    /// IPI (inter-processor)
    Ipi,
    /// Other
    Other,
}

/// Softirq type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftirqType {
    /// HI softirq
    Hi,
    /// Timer
    Timer,
    /// Net TX
    NetTx,
    /// Net RX
    NetRx,
    /// Block
    Block,
    /// IRQ poll
    IrqPoll,
    /// Tasklet
    Tasklet,
    /// Scheduler
    Sched,
    /// RCU
    Rcu,
}

/// Storm severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StormSeverity {
    /// No storm
    None,
    /// Light (above normal)
    Light,
    /// Moderate
    Moderate,
    /// Severe
    Severe,
}

// ============================================================================
// IRQ STATS
// ============================================================================

/// Per-IRQ statistics
#[derive(Debug, Clone, Default)]
pub struct IrqStats {
    /// IRQ number
    pub irq_number: u32,
    /// Category
    pub category_val: u8,
    /// Total invocations
    pub invocations: u64,
    /// Total time (ns)
    pub total_ns: u64,
    /// Min time (ns)
    pub min_ns: u64,
    /// Max time (ns)
    pub max_ns: u64,
    /// CPU affinity mask
    pub cpu_affinity: u64,
}

impl IrqStats {
    pub fn new(irq_number: u32, category: IrqCategory) -> Self {
        Self {
            irq_number,
            category_val: category as u8,
            invocations: 0,
            total_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
            cpu_affinity: 0,
        }
    }

    /// Record invocation
    pub fn record(&mut self, duration_ns: u64, cpu: u32) {
        self.invocations += 1;
        self.total_ns += duration_ns;
        if duration_ns < self.min_ns {
            self.min_ns = duration_ns;
        }
        if duration_ns > self.max_ns {
            self.max_ns = duration_ns;
        }
        if cpu < 64 {
            self.cpu_affinity |= 1u64 << cpu;
        }
    }

    /// Average duration
    pub fn avg_ns(&self) -> u64 {
        if self.invocations == 0 {
            0
        } else {
            self.total_ns / self.invocations
        }
    }

    /// Rate (per second, given time window)
    pub fn rate(&self, window_ns: u64) -> f64 {
        if window_ns == 0 {
            return 0.0;
        }
        self.invocations as f64 / (window_ns as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// SOFTIRQ STATS
// ============================================================================

/// Per-softirq statistics
#[derive(Debug, Clone, Default)]
pub struct SoftirqStats {
    /// Type
    pub softirq_type: u8,
    /// Count
    pub count: u64,
    /// Total time (ns)
    pub total_ns: u64,
    /// EMA time (ns)
    pub ema_ns: f64,
}

impl SoftirqStats {
    pub fn new(stype: SoftirqType) -> Self {
        Self {
            softirq_type: stype as u8,
            ..Default::default()
        }
    }

    /// Record
    pub fn record(&mut self, duration_ns: u64) {
        self.count += 1;
        self.total_ns += duration_ns;
        self.ema_ns = 0.9 * self.ema_ns + 0.1 * duration_ns as f64;
    }
}

// ============================================================================
// PER-PROCESS IRQ IMPACT
// ============================================================================

/// Process IRQ impact tracker
#[derive(Debug)]
pub struct ProcessIrqImpact {
    /// Process ID
    pub pid: u64,
    /// IRQ time stolen from this process (ns)
    pub irq_stolen_ns: u64,
    /// Softirq time stolen (ns)
    pub softirq_stolen_ns: u64,
    /// Context switches caused by IRQ
    pub irq_context_switches: u64,
    /// IRQs while running
    pub irqs_while_running: u64,
    /// Most impactful IRQ
    pub top_irq: u32,
    /// Top IRQ time (ns)
    pub top_irq_ns: u64,
}

impl ProcessIrqImpact {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            irq_stolen_ns: 0,
            softirq_stolen_ns: 0,
            irq_context_switches: 0,
            irqs_while_running: 0,
            top_irq: 0,
            top_irq_ns: 0,
        }
    }

    /// Record IRQ impact
    pub fn record_irq(&mut self, irq: u32, duration_ns: u64, caused_switch: bool) {
        self.irq_stolen_ns += duration_ns;
        self.irqs_while_running += 1;
        if caused_switch {
            self.irq_context_switches += 1;
        }
        // Track top IRQ
        // (simplified: last wins if same duration)
        if duration_ns > self.top_irq_ns {
            self.top_irq = irq;
            self.top_irq_ns = duration_ns;
        }
    }

    /// Record softirq impact
    pub fn record_softirq(&mut self, duration_ns: u64) {
        self.softirq_stolen_ns += duration_ns;
    }

    /// Total stolen time
    pub fn total_stolen_ns(&self) -> u64 {
        self.irq_stolen_ns + self.softirq_stolen_ns
    }
}

// ============================================================================
// STORM DETECTOR
// ============================================================================

/// IRQ storm detector
#[derive(Debug)]
pub struct StormDetector {
    /// Window (ns)
    window_ns: u64,
    /// Current window start
    window_start: u64,
    /// IRQ counts in window
    window_counts: BTreeMap<u32, u64>,
    /// Storm threshold (per window)
    threshold: u64,
    /// Storms detected
    pub storms_detected: u64,
}

impl StormDetector {
    pub fn new(window_ns: u64, threshold: u64) -> Self {
        Self {
            window_ns,
            window_start: 0,
            window_counts: BTreeMap::new(),
            threshold,
            storms_detected: 0,
        }
    }

    /// Record IRQ
    pub fn record(&mut self, irq: u32, now: u64) -> StormSeverity {
        if now >= self.window_start + self.window_ns {
            self.window_counts.clear();
            self.window_start = now;
        }
        *self.window_counts.entry(irq).or_insert(0) += 1;
        let count = self.window_counts[&irq];
        if count > self.threshold * 3 {
            self.storms_detected += 1;
            StormSeverity::Severe
        } else if count > self.threshold * 2 {
            StormSeverity::Moderate
        } else if count > self.threshold {
            StormSeverity::Light
        } else {
            StormSeverity::None
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Interrupt profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppInterruptStats {
    /// Tracked IRQs
    pub tracked_irqs: usize,
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total IRQ invocations
    pub total_irqs: u64,
    /// Storms detected
    pub storms: u64,
}

/// App interrupt profiler
pub struct AppInterruptProfiler {
    /// IRQ stats
    irqs: BTreeMap<u32, IrqStats>,
    /// Softirq stats
    softirqs: BTreeMap<u8, SoftirqStats>,
    /// Per-process impact
    processes: BTreeMap<u64, ProcessIrqImpact>,
    /// Storm detector
    storm_detector: StormDetector,
    /// Stats
    stats: AppInterruptStats,
}

impl AppInterruptProfiler {
    pub fn new() -> Self {
        Self {
            irqs: BTreeMap::new(),
            softirqs: BTreeMap::new(),
            processes: BTreeMap::new(),
            storm_detector: StormDetector::new(1_000_000_000, 10000), // 1s, 10k threshold
            stats: AppInterruptStats::default(),
        }
    }

    /// Record IRQ
    pub fn record_irq(
        &mut self,
        irq: u32,
        category: IrqCategory,
        cpu: u32,
        duration_ns: u64,
        affected_pid: Option<u64>,
        caused_switch: bool,
        now: u64,
    ) {
        let stats = self
            .irqs
            .entry(irq)
            .or_insert_with(|| IrqStats::new(irq, category));
        stats.record(duration_ns, cpu);
        self.storm_detector.record(irq, now);
        if let Some(pid) = affected_pid {
            let impact = self
                .processes
                .entry(pid)
                .or_insert_with(|| ProcessIrqImpact::new(pid));
            impact.record_irq(irq, duration_ns, caused_switch);
        }
        self.stats.total_irqs += 1;
        self.update_stats();
    }

    /// Record softirq
    pub fn record_softirq(
        &mut self,
        stype: SoftirqType,
        duration_ns: u64,
        affected_pid: Option<u64>,
    ) {
        let stats = self
            .softirqs
            .entry(stype as u8)
            .or_insert_with(|| SoftirqStats::new(stype));
        stats.record(duration_ns);
        if let Some(pid) = affected_pid {
            let impact = self
                .processes
                .entry(pid)
                .or_insert_with(|| ProcessIrqImpact::new(pid));
            impact.record_softirq(duration_ns);
        }
    }

    /// Get process IRQ impact
    pub fn process_impact(&self, pid: u64) -> Option<&ProcessIrqImpact> {
        self.processes.get(&pid)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_irqs = self.irqs.len();
        self.stats.tracked_processes = self.processes.len();
        self.stats.storms = self.storm_detector.storms_detected;
    }

    /// Stats
    pub fn stats(&self) -> &AppInterruptStats {
        &self.stats
    }
}
