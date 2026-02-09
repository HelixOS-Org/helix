// SPDX-License-Identifier: GPL-2.0
//! Holistic psi_monitor — Pressure Stall Information monitoring and alerting.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// PSI resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PsiResource {
    Cpu,
    Memory,
    Io,
}

/// PSI measurement type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsiType {
    /// Some tasks stalled
    Some,
    /// All tasks stalled
    Full,
}

/// Time window for PSI averages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PsiWindow {
    Avg10,
    Avg60,
    Avg300,
    Total,
}

impl PsiWindow {
    #[inline]
    pub fn seconds(&self) -> u64 {
        match self {
            Self::Avg10 => 10,
            Self::Avg60 => 60,
            Self::Avg300 => 300,
            Self::Total => u64::MAX,
        }
    }
}

/// PSI readings for one resource
#[derive(Debug, Clone)]
pub struct PsiReading {
    pub resource: PsiResource,
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,
    pub some_total_us: u64,
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
    pub full_total_us: u64,
    pub timestamp: u64,
}

impl PsiReading {
    pub fn new(resource: PsiResource) -> Self {
        Self {
            resource,
            some_avg10: 0.0, some_avg60: 0.0, some_avg300: 0.0,
            some_total_us: 0,
            full_avg10: 0.0, full_avg60: 0.0, full_avg300: 0.0,
            full_total_us: 0,
            timestamp: 0,
        }
    }

    #[inline]
    pub fn get_some(&self, window: PsiWindow) -> f64 {
        match window {
            PsiWindow::Avg10 => self.some_avg10,
            PsiWindow::Avg60 => self.some_avg60,
            PsiWindow::Avg300 => self.some_avg300,
            PsiWindow::Total => self.some_total_us as f64,
        }
    }

    #[inline]
    pub fn get_full(&self, window: PsiWindow) -> f64 {
        match window {
            PsiWindow::Avg10 => self.full_avg10,
            PsiWindow::Avg60 => self.full_avg60,
            PsiWindow::Avg300 => self.full_avg300,
            PsiWindow::Total => self.full_total_us as f64,
        }
    }

    #[inline]
    pub fn max_pressure(&self) -> f64 {
        let vals = [
            self.some_avg10, self.some_avg60, self.some_avg300,
            self.full_avg10, self.full_avg60, self.full_avg300,
        ];
        vals.iter().cloned().fold(0.0_f64, f64::max)
    }

    #[inline(always)]
    pub fn is_critical(&self) -> bool {
        self.full_avg10 > 50.0 || self.some_avg10 > 80.0
    }
}

/// PSI trigger configuration
#[derive(Debug, Clone)]
pub struct PsiTrigger {
    pub id: u32,
    pub resource: PsiResource,
    pub psi_type: PsiType,
    pub threshold_pct: f64,
    pub window_us: u64,
    pub enabled: bool,
    pub fire_count: u64,
    pub last_fired: u64,
    pub cooldown_us: u64,
}

impl PsiTrigger {
    pub fn new(id: u32, resource: PsiResource, psi_type: PsiType,
               threshold_pct: f64, window_us: u64) -> Self {
        Self {
            id, resource, psi_type,
            threshold_pct, window_us,
            enabled: true, fire_count: 0,
            last_fired: 0, cooldown_us: 1_000_000,
        }
    }

    #[inline]
    pub fn should_fire(&self, reading: &PsiReading, now: u64) -> bool {
        if !self.enabled { return false; }
        if now.saturating_sub(self.last_fired) < self.cooldown_us { return false; }

        let value = match self.psi_type {
            PsiType::Some => reading.some_avg10,
            PsiType::Full => reading.full_avg10,
        };
        value >= self.threshold_pct
    }
}

/// PSI alert event
#[derive(Debug, Clone)]
pub struct PsiAlert {
    pub trigger_id: u32,
    pub resource: PsiResource,
    pub psi_type: PsiType,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: u64,
    pub cgroup_id: Option<u64>,
}

/// Per-cgroup PSI
#[derive(Debug)]
pub struct CgroupPsi {
    pub cgroup_id: u64,
    pub cgroup_name: String,
    pub cpu: PsiReading,
    pub memory: PsiReading,
    pub io: PsiReading,
}

impl CgroupPsi {
    pub fn new(cgroup_id: u64, name: String) -> Self {
        Self {
            cgroup_id, cgroup_name: name,
            cpu: PsiReading::new(PsiResource::Cpu),
            memory: PsiReading::new(PsiResource::Memory),
            io: PsiReading::new(PsiResource::Io),
        }
    }

    #[inline]
    pub fn reading(&self, resource: PsiResource) -> &PsiReading {
        match resource {
            PsiResource::Cpu => &self.cpu,
            PsiResource::Memory => &self.memory,
            PsiResource::Io => &self.io,
        }
    }

    #[inline(always)]
    pub fn overall_pressure(&self) -> f64 {
        let vals = [self.cpu.max_pressure(), self.memory.max_pressure(), self.io.max_pressure()];
        vals.iter().cloned().fold(0.0_f64, f64::max)
    }

    #[inline(always)]
    pub fn has_pressure(&self) -> bool {
        self.overall_pressure() > 0.0
    }
}

/// PSI monitor stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PsiMonitorStats {
    pub trigger_count: u32,
    pub active_triggers: u32,
    pub total_alerts: u64,
    pub cgroup_count: u32,
    pub peak_cpu_some: f64,
    pub peak_memory_some: f64,
    pub peak_io_some: f64,
}

/// Main PSI monitor
pub struct HolisticPsiMonitor {
    system_psi: BTreeMap<u8, PsiReading>,
    cgroup_psi: BTreeMap<u64, CgroupPsi>,
    triggers: BTreeMap<u32, PsiTrigger>,
    alerts: VecDeque<PsiAlert>,
    max_alerts: usize,
    next_trigger_id: u32,
    stats: PsiMonitorStats,
}

impl HolisticPsiMonitor {
    pub fn new() -> Self {
        Self {
            system_psi: BTreeMap::new(),
            cgroup_psi: BTreeMap::new(),
            triggers: BTreeMap::new(),
            alerts: VecDeque::new(),
            max_alerts: 4096,
            next_trigger_id: 1,
            stats: PsiMonitorStats {
                trigger_count: 0, active_triggers: 0,
                total_alerts: 0, cgroup_count: 0,
                peak_cpu_some: 0.0, peak_memory_some: 0.0,
                peak_io_some: 0.0,
            },
        }
    }

    pub fn update_system_psi(&mut self, reading: PsiReading) {
        let key = reading.resource as u8;
        match reading.resource {
            PsiResource::Cpu => {
                if reading.some_avg10 > self.stats.peak_cpu_some {
                    self.stats.peak_cpu_some = reading.some_avg10;
                }
            }
            PsiResource::Memory => {
                if reading.some_avg10 > self.stats.peak_memory_some {
                    self.stats.peak_memory_some = reading.some_avg10;
                }
            }
            PsiResource::Io => {
                if reading.some_avg10 > self.stats.peak_io_some {
                    self.stats.peak_io_some = reading.some_avg10;
                }
            }
        }
        self.system_psi.insert(key, reading);
    }

    pub fn update_cgroup_psi(&mut self, cgroup_id: u64, name: String,
                              resource: PsiResource, reading: PsiReading) {
        let entry = self.cgroup_psi.entry(cgroup_id)
            .or_insert_with(|| {
                self.stats.cgroup_count += 1;
                CgroupPsi::new(cgroup_id, name)
            });
        match resource {
            PsiResource::Cpu => entry.cpu = reading,
            PsiResource::Memory => entry.memory = reading,
            PsiResource::Io => entry.io = reading,
        }
    }

    #[inline]
    pub fn add_trigger(&mut self, resource: PsiResource, psi_type: PsiType,
                        threshold_pct: f64, window_us: u64) -> u32 {
        let id = self.next_trigger_id;
        self.next_trigger_id += 1;
        let trigger = PsiTrigger::new(id, resource, psi_type, threshold_pct, window_us);
        self.triggers.insert(id, trigger);
        self.stats.trigger_count += 1;
        self.stats.active_triggers += 1;
        id
    }

    pub fn check_triggers(&mut self, now: u64) -> Vec<PsiAlert> {
        let mut fired = Vec::new();
        let readings: Vec<PsiReading> = self.system_psi.values().cloned().collect();

        for trigger in self.triggers.values_mut() {
            if !trigger.enabled { continue; }
            for reading in &readings {
                if reading.resource != trigger.resource { continue; }
                if trigger.should_fire(reading, now) {
                    let value = match trigger.psi_type {
                        PsiType::Some => reading.some_avg10,
                        PsiType::Full => reading.full_avg10,
                    };
                    fired.push(PsiAlert {
                        trigger_id: trigger.id,
                        resource: trigger.resource,
                        psi_type: trigger.psi_type,
                        value, threshold: trigger.threshold_pct,
                        timestamp: now, cgroup_id: None,
                    });
                    trigger.fire_count += 1;
                    trigger.last_fired = now;
                }
            }
        }

        self.stats.total_alerts += fired.len() as u64;
        for alert in &fired {
            if self.alerts.len() >= self.max_alerts {
                self.alerts.pop_front();
            }
            self.alerts.push_back(alert.clone());
        }
        fired
    }

    #[inline]
    pub fn highest_pressure_cgroups(&self, n: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<_> = self.cgroup_psi.iter()
            .map(|(&id, cg)| (id, cg.overall_pressure()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    #[inline]
    pub fn critical_resources(&self) -> Vec<PsiResource> {
        self.system_psi.values()
            .filter(|r| r.is_critical())
            .map(|r| r.resource)
            .collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &PsiMonitorStats {
        &self.stats
    }
}
