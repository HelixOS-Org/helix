//! # Holistic Hotplug Manager
//!
//! CPU and memory hotplug management at system level:
//! - Online / offline CPU management
//! - Memory hotplug (add/remove sections)
//! - Workload migration before offline
//! - Capacity recalculation on topology changes
//! - Hotplug policy (aggressive, conservative, manual)
//! - Event notification for subsystem coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Hotplug resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugResource {
    Cpu,
    Memory,
    IoDevice,
    NumaNode,
}

/// Hotplug direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugAction {
    Online,
    Offline,
    /// Preparing to go offline (drain)
    Draining,
    /// Failed to online/offline
    Failed,
}

/// Hotplug policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugPolicy {
    /// Aggressively offline idle resources
    Aggressive,
    /// Only offline when clearly idle for sustained period
    Conservative,
    /// Manual only
    Manual,
    /// Power-aware: offline for power savings
    PowerAware,
}

/// CPU hotplug state
#[derive(Debug, Clone)]
pub struct CpuHotplugState {
    pub cpu_id: u32,
    pub online: bool,
    pub draining: bool,
    pub load_avg: f64,
    pub idle_duration_ms: u64,
    pub tasks_pinned: u32,
    pub irqs_assigned: u32,
    pub last_online_ts: u64,
    pub last_offline_ts: u64,
    pub online_count: u64,
    pub offline_count: u64,
}

impl CpuHotplugState {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            online: true,
            draining: false,
            load_avg: 0.0,
            idle_duration_ms: 0,
            tasks_pinned: 0,
            irqs_assigned: 0,
            last_online_ts: 0,
            last_offline_ts: 0,
            online_count: 1,
            offline_count: 0,
        }
    }

    /// Can this CPU be offlined safely?
    pub fn can_offline(&self) -> bool {
        self.online && !self.draining && self.tasks_pinned == 0 && self.load_avg < 0.1
    }

    /// Needs migration before offline
    pub fn needs_migration(&self) -> bool {
        self.tasks_pinned > 0 || self.irqs_assigned > 0
    }
}

/// Memory section hotplug state
#[derive(Debug, Clone)]
pub struct MemorySection {
    pub section_id: u32,
    pub start_addr: u64,
    pub size_bytes: u64,
    pub online: bool,
    pub numa_node: u32,
    pub pages_in_use: u64,
    pub pages_total: u64,
    pub removable: bool,
}

impl MemorySection {
    pub fn new(section_id: u32, start_addr: u64, size_bytes: u64, numa_node: u32) -> Self {
        let pages_total = size_bytes / 4096;
        Self {
            section_id,
            start_addr,
            size_bytes,
            online: true,
            numa_node,
            pages_in_use: 0,
            pages_total,
            removable: true,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.pages_total == 0 {
            return 0.0;
        }
        self.pages_in_use as f64 / self.pages_total as f64
    }

    pub fn can_offline(&self) -> bool {
        self.online && self.removable && self.pages_in_use == 0
    }
}

/// Hotplug event
#[derive(Debug, Clone)]
pub struct HotplugEvent {
    pub resource: HotplugResource,
    pub action: HotplugAction,
    pub resource_id: u32,
    pub timestamp: u64,
    pub success: bool,
    pub migration_needed: bool,
}

/// Hotplug stats
#[derive(Debug, Clone, Default)]
pub struct HolisticHotplugStats {
    pub total_cpus: usize,
    pub online_cpus: usize,
    pub draining_cpus: usize,
    pub memory_sections: usize,
    pub online_memory_bytes: u64,
    pub total_events: usize,
    pub failed_events: usize,
    pub policy: u8,
}

/// Holistic Hotplug Manager
pub struct HolisticHotplugMgr {
    cpus: BTreeMap<u32, CpuHotplugState>,
    memory_sections: BTreeMap<u32, MemorySection>,
    events: Vec<HotplugEvent>,
    policy: HotplugPolicy,
    idle_threshold_ms: u64,
    max_events: usize,
    stats: HolisticHotplugStats,
}

impl HolisticHotplugMgr {
    pub fn new(policy: HotplugPolicy) -> Self {
        Self {
            cpus: BTreeMap::new(),
            memory_sections: BTreeMap::new(),
            events: Vec::new(),
            policy,
            idle_threshold_ms: 5000,
            max_events: 256,
            stats: HolisticHotplugStats::default(),
        }
    }

    pub fn register_cpu(&mut self, cpu_id: u32) {
        self.cpus.insert(cpu_id, CpuHotplugState::new(cpu_id));
        self.recompute();
    }

    pub fn update_cpu_load(&mut self, cpu_id: u32, load: f64, idle_ms: u64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.load_avg = load;
            cpu.idle_duration_ms = idle_ms;
        }
    }

    pub fn set_pinned_tasks(&mut self, cpu_id: u32, count: u32) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.tasks_pinned = count;
        }
    }

    /// Attempt to offline a CPU
    pub fn offline_cpu(&mut self, cpu_id: u32, now: u64) -> HotplugEvent {
        let success;
        let migration;

        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            if cpu.needs_migration() {
                cpu.draining = true;
                migration = true;
                success = false; // needs drain first
            } else if cpu.can_offline() {
                cpu.online = false;
                cpu.draining = false;
                cpu.last_offline_ts = now;
                cpu.offline_count += 1;
                migration = false;
                success = true;
            } else {
                migration = false;
                success = false;
            }
        } else {
            migration = false;
            success = false;
        }

        let event = HotplugEvent {
            resource: HotplugResource::Cpu,
            action: if success {
                HotplugAction::Offline
            } else {
                HotplugAction::Failed
            },
            resource_id: cpu_id,
            timestamp: now,
            success,
            migration_needed: migration,
        };
        self.record_event(event.clone());
        self.recompute();
        event
    }

    /// Bring CPU online
    pub fn online_cpu(&mut self, cpu_id: u32, now: u64) -> HotplugEvent {
        let success = if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            if !cpu.online {
                cpu.online = true;
                cpu.draining = false;
                cpu.last_online_ts = now;
                cpu.online_count += 1;
                true
            } else {
                false
            }
        } else {
            false
        };

        let event = HotplugEvent {
            resource: HotplugResource::Cpu,
            action: if success {
                HotplugAction::Online
            } else {
                HotplugAction::Failed
            },
            resource_id: cpu_id,
            timestamp: now,
            success,
            migration_needed: false,
        };
        self.record_event(event.clone());
        self.recompute();
        event
    }

    /// Complete drain (after task migration)
    pub fn complete_drain(&mut self, cpu_id: u32, now: u64) -> bool {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            if cpu.draining {
                cpu.draining = false;
                cpu.online = false;
                cpu.last_offline_ts = now;
                cpu.offline_count += 1;
                self.recompute();
                return true;
            }
        }
        false
    }

    pub fn register_memory(&mut self, section: MemorySection) {
        self.memory_sections.insert(section.section_id, section);
        self.recompute();
    }

    pub fn update_memory_usage(&mut self, section_id: u32, pages_in_use: u64) {
        if let Some(sec) = self.memory_sections.get_mut(&section_id) {
            sec.pages_in_use = pages_in_use;
        }
    }

    /// Suggest CPUs to offline based on policy
    pub fn suggest_offline(&self) -> Vec<u32> {
        match self.policy {
            HotplugPolicy::Manual => Vec::new(),
            HotplugPolicy::Aggressive => self
                .cpus
                .values()
                .filter(|c| c.online && c.load_avg < 0.05 && c.idle_duration_ms > 1000)
                .map(|c| c.cpu_id)
                .collect(),
            HotplugPolicy::Conservative => self
                .cpus
                .values()
                .filter(|c| c.can_offline() && c.idle_duration_ms > self.idle_threshold_ms)
                .map(|c| c.cpu_id)
                .collect(),
            HotplugPolicy::PowerAware => {
                // Offline CPUs with lowest load, keep minimum online
                let online_count = self.cpus.values().filter(|c| c.online).count();
                let min_online = 2;
                if online_count <= min_online {
                    return Vec::new();
                }

                let mut candidates: Vec<&CpuHotplugState> =
                    self.cpus.values().filter(|c| c.can_offline()).collect();
                candidates.sort_by(|a, b| {
                    a.load_avg
                        .partial_cmp(&b.load_avg)
                        .unwrap_or(core::cmp::Ordering::Equal)
                });

                let max_offline = online_count - min_online;
                candidates
                    .into_iter()
                    .take(max_offline)
                    .map(|c| c.cpu_id)
                    .collect()
            },
        }
    }

    fn record_event(&mut self, event: HotplugEvent) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.online_cpus = self.cpus.values().filter(|c| c.online).count();
        self.stats.draining_cpus = self.cpus.values().filter(|c| c.draining).count();
        self.stats.memory_sections = self.memory_sections.len();
        self.stats.online_memory_bytes = self
            .memory_sections
            .values()
            .filter(|s| s.online)
            .map(|s| s.size_bytes)
            .sum();
        self.stats.total_events = self.events.len();
        self.stats.failed_events = self.events.iter().filter(|e| !e.success).count();
        self.stats.policy = self.policy as u8;
    }

    pub fn stats(&self) -> &HolisticHotplugStats {
        &self.stats
    }

    pub fn events(&self) -> &[HotplugEvent] {
        &self.events
    }
}
