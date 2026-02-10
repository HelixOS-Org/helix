// SPDX-License-Identifier: GPL-2.0
//! Apps CPU frequency manager — per-app DVFS profiling and governor hints.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::fast::array_map::ArrayMap;

/// CPU frequency governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqGovernor {
    Performance,
    Powersave,
    Ondemand,
    Conservative,
    Schedutil,
    Userspace,
}

/// Frequency transition direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqTransition {
    Up,
    Down,
    Same,
}

/// Per-CPU frequency domain
#[derive(Debug)]
pub struct FreqDomain {
    pub domain_id: u32,
    pub cpus: Vec<u32>,
    pub min_freq_khz: u32,
    pub max_freq_khz: u32,
    pub current_freq_khz: u32,
    pub governor: FreqGovernor,
    pub transition_count: u64,
    pub time_in_freq: ArrayMap<u64, 32>,
    last_transition_ns: u64,
}

impl FreqDomain {
    pub fn new(domain_id: u32, cpus: Vec<u32>, min_khz: u32, max_khz: u32) -> Self {
        Self {
            domain_id,
            cpus,
            min_freq_khz: min_khz,
            max_freq_khz: max_khz,
            current_freq_khz: max_khz,
            governor: FreqGovernor::Schedutil,
            transition_count: 0,
            time_in_freq: ArrayMap::new(0),
            last_transition_ns: 0,
        }
    }

    pub fn set_frequency(&mut self, freq_khz: u32, now_ns: u64) -> FreqTransition {
        let clamped = freq_khz.clamp(self.min_freq_khz, self.max_freq_khz);
        let direction = if clamped > self.current_freq_khz {
            FreqTransition::Up
        } else if clamped < self.current_freq_khz {
            FreqTransition::Down
        } else {
            FreqTransition::Same
        };

        // Record time spent at old frequency
        if self.last_transition_ns > 0 {
            let duration = now_ns.saturating_sub(self.last_transition_ns);
            *self.time_in_freq.entry(self.current_freq_khz).or_insert(0) += duration;
        }

        self.current_freq_khz = clamped;
        self.last_transition_ns = now_ns;
        if direction != FreqTransition::Same {
            self.transition_count += 1;
        }
        direction
    }

    #[inline(always)]
    pub fn utilization_ratio(&self) -> f64 {
        if self.max_freq_khz == 0 {
            return 0.0;
        }
        self.current_freq_khz as f64 / self.max_freq_khz as f64
    }

    #[inline]
    pub fn dominant_frequency(&self) -> u32 {
        self.time_in_freq
            .iter()
            .max_by_key(|(_, time)| time)
            .map(|(freq, _)| freq as u32)
            .unwrap_or(self.current_freq_khz)
    }

    #[inline(always)]
    pub fn available_headroom_khz(&self) -> u32 {
        self.max_freq_khz.saturating_sub(self.current_freq_khz)
    }
}

/// Per-app frequency preference
#[derive(Debug)]
pub struct AppFreqProfile {
    pub pid: u64,
    pub preferred_min_khz: Option<u32>,
    pub preferred_max_khz: Option<u32>,
    pub avg_utilization: f64,
    pub samples: u64,
    pub latency_sensitive: bool,
    pub freq_histogram: ArrayMap<u64, 32>,
    total_util_sum: f64,
}

impl AppFreqProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            preferred_min_khz: None,
            preferred_max_khz: None,
            avg_utilization: 0.0,
            samples: 0,
            latency_sensitive: false,
            freq_histogram: ArrayMap::new(0),
            total_util_sum: 0.0,
        }
    }

    #[inline]
    pub fn record_sample(&mut self, freq_khz: u32, utilization: f64) {
        self.samples += 1;
        self.total_util_sum += utilization;
        self.avg_utilization = self.total_util_sum / self.samples as f64;
        self.freq_histogram.add(freq_khz as usize, 1);
    }

    #[inline]
    pub fn suggest_governor(&self) -> FreqGovernor {
        if self.latency_sensitive {
            FreqGovernor::Performance
        } else if self.avg_utilization < 0.2 {
            FreqGovernor::Powersave
        } else if self.avg_utilization > 0.8 {
            FreqGovernor::Performance
        } else {
            FreqGovernor::Schedutil
        }
    }

    #[inline]
    pub fn most_common_freq(&self) -> u32 {
        self.freq_histogram
            .iter()
            .max_by_key(|(_, count)| count)
            .map(|(freq, _)| freq as u32)
            .unwrap_or(0)
    }
}

/// Energy model entry for a frequency point
#[derive(Debug, Clone)]
pub struct EnergyPoint {
    pub freq_khz: u32,
    pub power_mw: u32,
    pub capacity: u32,
}

impl EnergyPoint {
    #[inline(always)]
    pub fn efficiency(&self) -> f64 {
        if self.power_mw == 0 {
            return 0.0;
        }
        self.capacity as f64 / self.power_mw as f64
    }
}

/// CPU frequency manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuFreqMgrStats {
    pub domains_managed: u64,
    pub transitions_total: u64,
    pub apps_profiled: u64,
    pub governor_hints: u64,
    pub energy_saved_est_mwh: u64,
}

/// Main apps CPU frequency manager
pub struct AppCpuFreqMgr {
    domains: BTreeMap<u32, FreqDomain>,
    profiles: BTreeMap<u64, AppFreqProfile>,
    energy_model: Vec<EnergyPoint>,
    stats: CpuFreqMgrStats,
}

impl AppCpuFreqMgr {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            profiles: BTreeMap::new(),
            energy_model: Vec::new(),
            stats: CpuFreqMgrStats {
                domains_managed: 0,
                transitions_total: 0,
                apps_profiled: 0,
                governor_hints: 0,
                energy_saved_est_mwh: 0,
            },
        }
    }

    #[inline(always)]
    pub fn add_domain(&mut self, domain_id: u32, cpus: Vec<u32>, min_khz: u32, max_khz: u32) {
        self.domains.insert(
            domain_id,
            FreqDomain::new(domain_id, cpus, min_khz, max_khz),
        );
        self.stats.domains_managed += 1;
    }

    #[inline(always)]
    pub fn set_energy_model(&mut self, points: Vec<EnergyPoint>) {
        self.energy_model = points;
    }

    #[inline]
    pub fn set_frequency(
        &mut self,
        domain_id: u32,
        freq_khz: u32,
        now_ns: u64,
    ) -> Option<FreqTransition> {
        let domain = self.domains.get_mut(&domain_id)?;
        let transition = domain.set_frequency(freq_khz, now_ns);
        if transition != FreqTransition::Same {
            self.stats.transitions_total += 1;
        }
        Some(transition)
    }

    #[inline]
    pub fn set_governor(&mut self, domain_id: u32, governor: FreqGovernor) -> bool {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.governor = governor;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn register_app(&mut self, pid: u64) {
        if !self.profiles.contains_key(&pid) {
            self.profiles.insert(pid, AppFreqProfile::new(pid));
            self.stats.apps_profiled += 1;
        }
    }

    #[inline]
    pub fn sample_app(&mut self, pid: u64, cpu_id: u32, utilization: f64) {
        // Find the domain for this CPU
        let freq_khz = self
            .domains
            .values()
            .find(|d| d.cpus.contains(&cpu_id))
            .map(|d| d.current_freq_khz)
            .unwrap_or(0);

        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_sample(freq_khz, utilization);
        }
    }

    #[inline]
    pub fn suggest_governor_for_app(&mut self, pid: u64) -> Option<FreqGovernor> {
        let profile = self.profiles.get(&pid)?;
        self.stats.governor_hints += 1;
        Some(profile.suggest_governor())
    }

    #[inline]
    pub fn most_efficient_freq(&self) -> Option<u32> {
        self.energy_model
            .iter()
            .max_by(|a, b| {
                a.efficiency()
                    .partial_cmp(&b.efficiency())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|e| e.freq_khz)
    }

    #[inline]
    pub fn domain_info(&self, domain_id: u32) -> Option<(u32, u32, f64)> {
        self.domains
            .get(&domain_id)
            .map(|d| (d.current_freq_khz, d.max_freq_khz, d.utilization_ratio()))
    }

    pub fn total_power_estimate_mw(&self) -> u32 {
        let mut total = 0u32;
        for domain in self.domains.values() {
            let closest = self.energy_model.iter().min_by_key(|e| {
                let diff = e.freq_khz as i64 - domain.current_freq_khz as i64;
                if diff < 0 {
                    (-diff) as u64
                } else {
                    diff as u64
                }
            });
            if let Some(point) = closest {
                total += point.power_mw * domain.cpus.len() as u32;
            }
        }
        total
    }

    #[inline(always)]
    pub fn stats(&self) -> &CpuFreqMgrStats {
        &self.stats
    }
}
