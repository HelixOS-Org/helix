//! # Holistic Scheduling Orchestrator
//!
//! System-wide scheduling decisions with global perspective:
//! - Cross-CPU load balancing
//! - NUMA-aware scheduling
//! - Energy-aware placement
//! - Priority inversion avoidance
//! - Real-time deadline management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SCHEDULING TYPES
// ============================================================================

/// Scheduling domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedDomain {
    /// Single CPU
    Cpu,
    /// CPU cluster (shared L2)
    Cluster,
    /// NUMA node
    Numa,
    /// Socket
    Socket,
    /// System-wide
    System,
}

/// Task class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticTaskClass {
    /// Batch processing
    Batch,
    /// Interactive
    Interactive,
    /// Real-time
    Realtime,
    /// Idle
    Idle,
    /// Deadline-constrained
    Deadline,
    /// Latency-sensitive
    LatencySensitive,
}

/// Placement decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementReason {
    /// Load balance
    LoadBalance,
    /// Affinity
    Affinity,
    /// NUMA locality
    NumaLocality,
    /// Cache warmth
    CacheWarmth,
    /// Energy saving
    EnergySaving,
    /// Isolation
    Isolation,
}

// ============================================================================
// CPU MODEL
// ============================================================================

/// CPU state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuState {
    /// CPU id
    pub id: u32,
    /// NUMA node
    pub numa_node: u32,
    /// Socket
    pub socket: u32,
    /// Current load (0.0-1.0)
    pub load: f64,
    /// Run queue length
    pub runqueue_len: u32,
    /// IPC (instructions per cycle)
    pub ipc: f64,
    /// Frequency (MHz)
    pub frequency_mhz: u32,
    /// Is idle?
    pub is_idle: bool,
    /// Assigned tasks
    pub assigned_tasks: Vec<u64>,
    /// Power state
    pub power_state: u8,
}

impl CpuState {
    pub fn new(id: u32, numa_node: u32, socket: u32) -> Self {
        Self {
            id,
            numa_node,
            socket,
            load: 0.0,
            runqueue_len: 0,
            ipc: 0.0,
            frequency_mhz: 0,
            is_idle: true,
            assigned_tasks: Vec::new(),
            power_state: 0,
        }
    }

    /// Capacity (inverse of load)
    #[inline(always)]
    pub fn capacity(&self) -> f64 {
        1.0 - self.load
    }
}

/// Task descriptor for scheduling
#[derive(Debug, Clone)]
pub struct SchedTask {
    /// Task id
    pub id: u64,
    /// Class
    pub class: HolisticTaskClass,
    /// Current CPU
    pub current_cpu: Option<u32>,
    /// Preferred NUMA node
    pub preferred_numa: Option<u32>,
    /// Load weight
    pub weight: u32,
    /// Deadline (ns, 0 = no deadline)
    pub deadline_ns: u64,
    /// Last migration time
    pub last_migration: u64,
    /// Migration cooldown (ns)
    pub migration_cooldown_ns: u64,
}

impl SchedTask {
    pub fn new(id: u64, class: HolisticTaskClass) -> Self {
        Self {
            id,
            class,
            current_cpu: None,
            preferred_numa: None,
            weight: 1024,
            deadline_ns: 0,
            last_migration: 0,
            migration_cooldown_ns: 5_000_000, // 5ms
        }
    }

    /// Can migrate now?
    #[inline(always)]
    pub fn can_migrate(&self, now: u64) -> bool {
        now.saturating_sub(self.last_migration) >= self.migration_cooldown_ns
    }
}

// ============================================================================
// LOAD IMBALANCE DETECTION
// ============================================================================

/// Imbalance info
#[derive(Debug, Clone)]
pub struct LoadImbalance {
    /// Domain
    pub domain: SchedDomain,
    /// Busiest CPU
    pub busiest_cpu: u32,
    /// Idlest CPU
    pub idlest_cpu: u32,
    /// Imbalance amount
    pub imbalance: f64,
    /// Tasks to migrate
    pub tasks_to_move: Vec<u64>,
}

/// Load balancer
#[derive(Debug)]
pub struct HolisticLoadBalancer {
    /// Imbalance threshold
    pub threshold: f64,
    /// Last balance time
    pub last_balance: u64,
    /// Balance interval (ns)
    pub interval_ns: u64,
    /// Migrations performed
    pub total_migrations: u64,
}

impl HolisticLoadBalancer {
    pub fn new() -> Self {
        Self {
            threshold: 0.25,
            last_balance: 0,
            interval_ns: 4_000_000, // 4ms
            total_migrations: 0,
        }
    }

    /// Detect imbalance across CPUs
    pub fn detect_imbalance(&self, cpus: &[CpuState]) -> Option<LoadImbalance> {
        if cpus.len() < 2 {
            return None;
        }

        let mut busiest_idx = 0;
        let mut idlest_idx = 0;
        let mut max_load = 0.0f64;
        let mut min_load = 1.0f64;

        for (i, cpu) in cpus.iter().enumerate() {
            if cpu.load > max_load {
                max_load = cpu.load;
                busiest_idx = i;
            }
            if cpu.load < min_load {
                min_load = cpu.load;
                idlest_idx = i;
            }
        }

        let imbalance = max_load - min_load;
        if imbalance > self.threshold {
            Some(LoadImbalance {
                domain: SchedDomain::System,
                busiest_cpu: cpus[busiest_idx].id,
                idlest_cpu: cpus[idlest_idx].id,
                imbalance,
                tasks_to_move: Vec::new(),
            })
        } else {
            None
        }
    }

    /// Should balance now?
    #[inline(always)]
    pub fn should_balance(&self, now: u64) -> bool {
        now.saturating_sub(self.last_balance) >= self.interval_ns
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Scheduling stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticSchedulingStats {
    /// Total CPUs
    pub total_cpus: usize,
    /// Total tasks
    pub total_tasks: usize,
    /// Average load
    pub avg_load: f64,
    /// Load std deviation
    pub load_stddev: f64,
    /// Total migrations
    pub total_migrations: u64,
    /// Deadline misses
    pub deadline_misses: u64,
}

/// Holistic scheduling orchestrator
pub struct HolisticSchedulingEngine {
    /// CPU states
    cpus: BTreeMap<u32, CpuState>,
    /// Tasks
    tasks: BTreeMap<u64, SchedTask>,
    /// Load balancer
    pub balancer: HolisticLoadBalancer,
    /// Stats
    stats: HolisticSchedulingStats,
}

impl HolisticSchedulingEngine {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            tasks: BTreeMap::new(),
            balancer: HolisticLoadBalancer::new(),
            stats: HolisticSchedulingStats::default(),
        }
    }

    /// Register CPU
    #[inline(always)]
    pub fn add_cpu(&mut self, cpu: CpuState) {
        self.cpus.insert(cpu.id, cpu);
        self.update_stats();
    }

    /// Add task
    #[inline(always)]
    pub fn add_task(&mut self, task: SchedTask) {
        self.tasks.insert(task.id, task);
        self.update_stats();
    }

    /// Remove task
    #[inline]
    pub fn remove_task(&mut self, task_id: u64) {
        if let Some(task) = self.tasks.remove(&task_id) {
            if let Some(cpu_id) = task.current_cpu {
                if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
                    cpu.assigned_tasks.retain(|&t| t != task_id);
                }
            }
        }
        self.update_stats();
    }

    /// Place task on best CPU
    pub fn place_task(&mut self, task_id: u64, now: u64) -> Option<(u32, PlacementReason)> {
        let task = self.tasks.get(&task_id)?;
        let preferred_numa = task.preferred_numa;

        // Find best CPU
        let mut best_cpu: Option<u32> = None;
        let mut best_score = f64::MIN;
        let mut reason = PlacementReason::LoadBalance;

        for cpu in self.cpus.values() {
            let mut score = cpu.capacity() * 100.0;

            // NUMA bonus
            if let Some(pn) = preferred_numa {
                if cpu.numa_node == pn {
                    score += 50.0;
                    reason = PlacementReason::NumaLocality;
                }
            }

            // Idle bonus
            if cpu.is_idle {
                score += 30.0;
            }

            if score > best_score {
                best_score = score;
                best_cpu = Some(cpu.id);
            }
        }

        if let Some(cpu_id) = best_cpu {
            if let Some(task) = self.tasks.get_mut(&task_id) {
                task.current_cpu = Some(cpu_id);
                task.last_migration = now;
            }
            if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
                cpu.assigned_tasks.push(task_id);
            }
            return Some((cpu_id, reason));
        }
        None
    }

    /// Balance load
    pub fn balance(&mut self, now: u64) -> Vec<(u64, u32, u32)> {
        let mut migrations = Vec::new();
        if !self.balancer.should_balance(now) {
            return migrations;
        }
        self.balancer.last_balance = now;

        let cpu_list: Vec<CpuState> = self.cpus.values().cloned().collect();
        if let Some(imbalance) = self.balancer.detect_imbalance(&cpu_list) {
            // Find tasks on busiest that can migrate
            let busiest_tasks: Vec<u64> = self.cpus.get(&imbalance.busiest_cpu)
                .map(|c| c.assigned_tasks.clone())
                .unwrap_or_default();

            for &tid in busiest_tasks.iter().take(1) {
                if let Some(task) = self.tasks.get(&tid) {
                    if task.can_migrate(now) {
                        migrations.push((tid, imbalance.busiest_cpu, imbalance.idlest_cpu));
                    }
                }
            }
        }

        // Execute migrations
        for &(tid, from, to) in &migrations {
            if let Some(cpu) = self.cpus.get_mut(&from) {
                cpu.assigned_tasks.retain(|&t| t != tid);
            }
            if let Some(cpu) = self.cpus.get_mut(&to) {
                cpu.assigned_tasks.push(tid);
            }
            if let Some(task) = self.tasks.get_mut(&tid) {
                task.current_cpu = Some(to);
                task.last_migration = now;
            }
            self.balancer.total_migrations += 1;
        }

        self.update_stats();
        migrations
    }

    /// Update CPU load
    #[inline]
    pub fn update_cpu_load(&mut self, cpu_id: u32, load: f64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.load = load;
            cpu.is_idle = load < 0.01;
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.total_tasks = self.tasks.len();
        if !self.cpus.is_empty() {
            let loads: Vec<f64> = self.cpus.values().map(|c| c.load).collect();
            let sum: f64 = loads.iter().sum();
            self.stats.avg_load = sum / loads.len() as f64;
            let variance: f64 = loads.iter()
                .map(|l| (l - self.stats.avg_load) * (l - self.stats.avg_load))
                .sum::<f64>() / loads.len() as f64;
            self.stats.load_stddev = libm::sqrt(variance);
        }
        self.stats.total_migrations = self.balancer.total_migrations;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticSchedulingStats {
        &self.stats
    }
}
