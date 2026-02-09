//! Per-task and system-wide energy profiling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{CState, PState};

// ============================================================================
// TASK ENERGY
// ============================================================================

/// Task energy usage
#[derive(Debug, Clone, Default)]
pub struct TaskEnergy {
    /// Task ID
    pub task_id: u64,
    /// CPU energy (arbitrary units)
    pub cpu_energy: f64,
    /// Memory energy
    pub memory_energy: f64,
    /// I/O energy
    pub io_energy: f64,
    /// Network energy
    pub network_energy: f64,
    /// Total runtime (nanoseconds)
    pub runtime_ns: u64,
}

impl TaskEnergy {
    /// Total energy
    #[inline(always)]
    pub fn total(&self) -> f64 {
        self.cpu_energy + self.memory_energy + self.io_energy + self.network_energy
    }

    /// Energy per nanosecond
    #[inline]
    pub fn power(&self) -> f64 {
        if self.runtime_ns > 0 {
            self.total() / self.runtime_ns as f64
        } else {
            0.0
        }
    }
}

// ============================================================================
// SYSTEM ENERGY
// ============================================================================

/// System-wide energy tracking
#[derive(Debug, Clone, Default)]
pub struct SystemEnergy {
    /// Total CPU energy
    pub cpu_total: f64,
    /// Total memory energy
    pub memory_total: f64,
    /// Total I/O energy
    pub io_total: f64,
    /// Idle energy
    pub idle_energy: f64,
    /// Time tracked (nanoseconds)
    pub time_ns: u64,
}

// ============================================================================
// POWER SENSOR
// ============================================================================

/// Power sensor
#[derive(Debug, Clone)]
pub struct PowerSensor {
    /// Sensor ID
    pub id: u32,
    /// Sensor name
    pub name: String,
    /// Current power (milliwatts)
    pub power_mw: u32,
    /// Energy counter (microjoules)
    pub energy_uj: u64,
}

// ============================================================================
// ENERGY PROFILER
// ============================================================================

/// Per-task energy profiling
pub struct EnergyProfiler {
    /// Task energy usage
    task_energy: BTreeMap<u64, TaskEnergy>,
    /// System-wide energy
    system_energy: SystemEnergy,
    /// Power sensors (if available)
    power_sensors: Vec<PowerSensor>,
    /// Total operations
    total_ops: AtomicU64,
}

impl EnergyProfiler {
    /// Create new energy profiler
    pub fn new() -> Self {
        Self {
            task_energy: BTreeMap::new(),
            system_energy: SystemEnergy::default(),
            power_sensors: Vec::new(),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Record CPU usage for task
    #[inline]
    pub fn record_cpu(&mut self, task_id: u64, cpu_cycles: u64, p_state: &PState) {
        let energy = self.task_energy.entry(task_id).or_default();
        energy.task_id = task_id;

        // Estimate energy based on P-state
        let joules = cpu_cycles as f64 * p_state.relative_power * 0.000001;
        energy.cpu_energy += joules;

        self.system_energy.cpu_total += joules;
        self.total_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record memory access
    #[inline]
    pub fn record_memory(&mut self, task_id: u64, accesses: u64) {
        let energy = self.task_energy.entry(task_id).or_default();

        // Rough estimate: 1nJ per access
        let joules = accesses as f64 * 0.000000001;
        energy.memory_energy += joules;

        self.system_energy.memory_total += joules;
    }

    /// Record I/O
    #[inline]
    pub fn record_io(&mut self, task_id: u64, bytes: u64) {
        let energy = self.task_energy.entry(task_id).or_default();

        // Rough estimate: 10nJ per byte
        let joules = bytes as f64 * 0.00000001;
        energy.io_energy += joules;

        self.system_energy.io_total += joules;
    }

    /// Record runtime
    #[inline]
    pub fn record_runtime(&mut self, task_id: u64, runtime_ns: u64) {
        let energy = self.task_energy.entry(task_id).or_default();
        energy.runtime_ns += runtime_ns;
        self.system_energy.time_ns += runtime_ns;
    }

    /// Record idle time
    #[inline]
    pub fn record_idle(&mut self, duration_ns: u64, c_state: CState) {
        let power_factor = c_state.power_reduction();
        let joules = duration_ns as f64 * power_factor * 0.00000001;
        self.system_energy.idle_energy += joules;
        self.system_energy.time_ns += duration_ns;
    }

    /// Get task energy
    #[inline(always)]
    pub fn get_task_energy(&self, task_id: u64) -> Option<&TaskEnergy> {
        self.task_energy.get(&task_id)
    }

    /// Get top energy consumers
    #[inline]
    pub fn top_consumers(&self, n: usize) -> Vec<&TaskEnergy> {
        let mut tasks: Vec<_> = self.task_energy.values().collect();
        tasks.sort_by(|a, b| b.total().partial_cmp(&a.total()).unwrap());
        tasks.into_iter().take(n).collect()
    }

    /// Get system energy
    #[inline(always)]
    pub fn system_energy(&self) -> &SystemEnergy {
        &self.system_energy
    }

    /// Get average power (watts)
    pub fn average_power(&self) -> f64 {
        let total_energy = self.system_energy.cpu_total
            + self.system_energy.memory_total
            + self.system_energy.io_total
            + self.system_energy.idle_energy;

        let time_s = self.system_energy.time_ns as f64 / 1_000_000_000.0;
        if time_s > 0.0 {
            total_energy / time_s
        } else {
            0.0
        }
    }

    /// Add power sensor
    #[inline(always)]
    pub fn add_sensor(&mut self, sensor: PowerSensor) {
        self.power_sensors.push(sensor);
    }

    /// Update sensor reading
    #[inline]
    pub fn update_sensor(&mut self, sensor_id: u32, power_mw: u32, energy_uj: u64) {
        if let Some(sensor) = self.power_sensors.iter_mut().find(|s| s.id == sensor_id) {
            sensor.power_mw = power_mw;
            sensor.energy_uj = energy_uj;
        }
    }

    /// Get total sensor power
    #[inline(always)]
    pub fn sensor_power(&self) -> u32 {
        self.power_sensors.iter().map(|s| s.power_mw).sum()
    }
}

impl Default for EnergyProfiler {
    fn default() -> Self {
        Self::new()
    }
}
