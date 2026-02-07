//! # Global System State & Snapshot
//!
//! Maintains a holistic view of the entire system's state, aggregating
//! metrics from CPU, memory, I/O, network, and all active processes.

use alloc::vec::Vec;

// ============================================================================
// OPTIMIZATION GOALS
// ============================================================================

/// High-level optimization goal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationGoal {
    /// Maximize throughput
    Throughput,
    /// Minimize latency
    Latency,
    /// Minimize energy consumption
    Energy,
    /// Balance performance and energy
    Balanced,
    /// Maximize fairness across processes
    Fairness,
    /// Maximize QoS for interactive workloads
    Interactive,
    /// Maximize reliability
    Reliability,
}

// ============================================================================
// SYSTEM SNAPSHOT
// ============================================================================

/// CPU state snapshot
#[derive(Debug, Clone, Copy)]
pub struct CpuSnapshot {
    /// Total cores
    pub cores: u32,
    /// Per-core utilization average (0.0 - 1.0)
    pub utilization: f64,
    /// User-space CPU time ratio
    pub user_ratio: f64,
    /// Kernel-space CPU time ratio
    pub kernel_ratio: f64,
    /// Idle ratio
    pub idle_ratio: f64,
    /// Context switches per second
    pub context_switches_per_sec: u64,
    /// Run queue length
    pub run_queue_length: u32,
}

/// Memory state snapshot
#[derive(Debug, Clone, Copy)]
pub struct MemorySnapshot {
    /// Total physical memory (bytes)
    pub total: u64,
    /// Used memory (bytes)
    pub used: u64,
    /// Free memory (bytes)
    pub free: u64,
    /// Cached memory (bytes)
    pub cached: u64,
    /// Page faults per second
    pub page_faults_per_sec: u64,
    /// Swap usage (bytes)
    pub swap_used: u64,
    /// Memory pressure (0.0 - 1.0)
    pub pressure: f64,
}

/// I/O state snapshot
#[derive(Debug, Clone, Copy)]
pub struct IoSnapshot {
    /// Read throughput (bytes/sec)
    pub read_bps: u64,
    /// Write throughput (bytes/sec)
    pub write_bps: u64,
    /// I/O operations per second
    pub iops: u64,
    /// Average I/O latency (microseconds)
    pub avg_latency_us: u64,
    /// I/O queue depth
    pub queue_depth: u32,
    /// I/O pressure (0.0 - 1.0)
    pub pressure: f64,
}

/// Network state snapshot
#[derive(Debug, Clone, Copy)]
pub struct NetworkSnapshot {
    /// Receive throughput (bytes/sec)
    pub rx_bps: u64,
    /// Transmit throughput (bytes/sec)
    pub tx_bps: u64,
    /// Packets per second
    pub pps: u64,
    /// Dropped packets
    pub dropped: u64,
    /// Network pressure (0.0 - 1.0)
    pub pressure: f64,
}

/// Process summary for holistic view
#[derive(Debug, Clone)]
pub struct ProcessSummary {
    /// Process ID
    pub pid: u64,
    /// CPU usage (0.0 - 1.0)
    pub cpu_usage: f64,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// I/O rate (bytes/sec)
    pub io_rate: u64,
    /// Priority
    pub priority: u8,
    /// Whether the process is cooperative
    pub is_cooperative: bool,
    /// Cooperation health score (if cooperative)
    pub coop_score: f64,
}

/// Complete system state snapshot
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    /// CPU state
    pub cpu: CpuSnapshot,
    /// Memory state
    pub memory: MemorySnapshot,
    /// I/O state
    pub io: IoSnapshot,
    /// Network state
    pub network: NetworkSnapshot,
    /// Process summaries (top processes)
    pub processes: Vec<ProcessSummary>,
    /// Total process count
    pub total_processes: u32,
    /// Active cooperation sessions
    pub active_coop_sessions: u32,
    /// System uptime (ms)
    pub uptime_ms: u64,
    /// Timestamp of this snapshot
    pub timestamp: u64,
    /// CPU cores
    pub cpu_cores: u32,
    /// Total memory
    pub total_memory: u64,
}

impl SystemSnapshot {
    /// Create a new snapshot with minimal info
    pub fn new(cpu_cores: u32, total_memory: u64) -> Self {
        Self {
            cpu: CpuSnapshot {
                cores: cpu_cores,
                utilization: 0.0,
                user_ratio: 0.0,
                kernel_ratio: 0.0,
                idle_ratio: 1.0,
                context_switches_per_sec: 0,
                run_queue_length: 0,
            },
            memory: MemorySnapshot {
                total: total_memory,
                used: 0,
                free: total_memory,
                cached: 0,
                page_faults_per_sec: 0,
                swap_used: 0,
                pressure: 0.0,
            },
            io: IoSnapshot {
                read_bps: 0,
                write_bps: 0,
                iops: 0,
                avg_latency_us: 0,
                queue_depth: 0,
                pressure: 0.0,
            },
            network: NetworkSnapshot {
                rx_bps: 0,
                tx_bps: 0,
                pps: 0,
                dropped: 0,
                pressure: 0.0,
            },
            processes: Vec::new(),
            total_processes: 0,
            active_coop_sessions: 0,
            uptime_ms: 0,
            timestamp: 0,
            cpu_cores,
            total_memory,
        }
    }

    /// Overall system pressure (0.0 - 1.0)
    pub fn overall_pressure(&self) -> f64 {
        let cpu_pressure = self.cpu.utilization;
        let mem_pressure = self.memory.pressure;
        let io_pressure = self.io.pressure;
        let net_pressure = self.network.pressure;

        // Weighted average with CPU and memory weighted higher
        cpu_pressure * 0.35 + mem_pressure * 0.35 + io_pressure * 0.2 + net_pressure * 0.1
    }

    /// Determine the dominant bottleneck
    pub fn dominant_bottleneck(&self) -> BottleneckType {
        let pressures = [
            (BottleneckType::Cpu, self.cpu.utilization),
            (BottleneckType::Memory, self.memory.pressure),
            (BottleneckType::Io, self.io.pressure),
            (BottleneckType::Network, self.network.pressure),
        ];

        pressures
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(t, _)| *t)
            .unwrap_or(BottleneckType::None)
    }
}

/// Type of system bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckType {
    None,
    Cpu,
    Memory,
    Io,
    Network,
}
