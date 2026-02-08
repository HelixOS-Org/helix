// SPDX-License-Identifier: GPL-2.0
//! Holistic cpufreq_gov — CPU frequency governor.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorType {
    Performance,
    Powersave,
    Ondemand,
    Conservative,
    Schedutil,
    Userspace,
}

/// CPU frequency state
#[derive(Debug)]
pub struct CpuFreqState {
    pub cpu_id: u32,
    pub current_freq_khz: u64,
    pub min_freq_khz: u64,
    pub max_freq_khz: u64,
    pub governor: GovernorType,
    pub load_percent: u32,
    pub transitions: u64,
    pub time_in_state_ms: BTreeMap<u64, u64>,
}

impl CpuFreqState {
    pub fn new(cpu_id: u32, min: u64, max: u64) -> Self {
        Self { cpu_id, current_freq_khz: max, min_freq_khz: min, max_freq_khz: max, governor: GovernorType::Schedutil, load_percent: 0, transitions: 0, time_in_state_ms: BTreeMap::new() }
    }

    pub fn set_freq(&mut self, freq_khz: u64) {
        let f = freq_khz.clamp(self.min_freq_khz, self.max_freq_khz);
        if f != self.current_freq_khz { self.transitions += 1; self.current_freq_khz = f; }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_freq_khz == 0 { 0.0 } else { self.current_freq_khz as f64 / self.max_freq_khz as f64 }
    }
}

/// Frequency transition event
#[derive(Debug, Clone)]
pub struct FreqTransition {
    pub cpu_id: u32,
    pub old_freq: u64,
    pub new_freq: u64,
    pub timestamp: u64,
    pub reason_hash: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct CpuFreqGovStats {
    pub total_cpus: u32,
    pub total_transitions: u64,
    pub avg_freq_khz: u64,
    pub avg_utilization: f64,
}

/// Main cpufreq governor
pub struct HolisticCpuFreqGov {
    cpus: BTreeMap<u32, CpuFreqState>,
    transitions: u64,
}

impl HolisticCpuFreqGov {
    pub fn new() -> Self { Self { cpus: BTreeMap::new(), transitions: 0 } }

    pub fn register_cpu(&mut self, cpu_id: u32, min: u64, max: u64) {
        self.cpus.insert(cpu_id, CpuFreqState::new(cpu_id, min, max));
    }

    pub fn update_load(&mut self, cpu_id: u32, load: u32) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.load_percent = load;
            match cpu.governor {
                GovernorType::Performance => cpu.set_freq(cpu.max_freq_khz),
                GovernorType::Powersave => cpu.set_freq(cpu.min_freq_khz),
                GovernorType::Schedutil | GovernorType::Ondemand => {
                    let range = cpu.max_freq_khz - cpu.min_freq_khz;
                    let target = cpu.min_freq_khz + (range * load as u64) / 100;
                    cpu.set_freq(target);
                }
                GovernorType::Conservative => {
                    let step = (cpu.max_freq_khz - cpu.min_freq_khz) / 20;
                    if load > 80 { cpu.set_freq(cpu.current_freq_khz + step); }
                    else if load < 20 { cpu.set_freq(cpu.current_freq_khz.saturating_sub(step)); }
                }
                GovernorType::Userspace => {}
            }
        }
    }

    pub fn stats(&self) -> CpuFreqGovStats {
        let transitions: u64 = self.cpus.values().map(|c| c.transitions).sum();
        let freqs: Vec<u64> = self.cpus.values().map(|c| c.current_freq_khz).collect();
        let avg_freq = if freqs.is_empty() { 0 } else { freqs.iter().sum::<u64>() / freqs.len() as u64 };
        let utils: Vec<f64> = self.cpus.values().map(|c| c.utilization()).collect();
        let avg_util = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        CpuFreqGovStats { total_cpus: self.cpus.len() as u32, total_transitions: transitions, avg_freq_khz: avg_freq, avg_utilization: avg_util }
    }
}
