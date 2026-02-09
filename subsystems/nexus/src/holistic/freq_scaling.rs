// SPDX-License-Identifier: GPL-2.0
//! Holistic freq_scaling — CPU frequency scaling (DVFS).

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Frequency governor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqGovernor {
    Performance,
    Powersave,
    OnDemand,
    Conservative,
    Schedutil,
    Userspace,
}

/// Scaling state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingState {
    Active,
    Inactive,
    Boosted,
    Throttled,
}

/// Frequency domain
#[derive(Debug)]
pub struct FreqDomain {
    pub id: u32,
    pub cpus: Vec<u32>,
    pub current_freq_khz: u64,
    pub min_freq_khz: u64,
    pub max_freq_khz: u64,
    pub available_freqs: Vec<u64>,
    pub governor: FreqGovernor,
    pub state: ScalingState,
    pub transitions: u64,
    pub time_in_state: LinearMap<u64, 64>,
    pub last_transition: u64,
}

impl FreqDomain {
    pub fn new(id: u32, min: u64, max: u64) -> Self {
        Self {
            id, cpus: Vec::new(), current_freq_khz: max,
            min_freq_khz: min, max_freq_khz: max,
            available_freqs: Vec::new(), governor: FreqGovernor::Schedutil,
            state: ScalingState::Active, transitions: 0,
            time_in_state: LinearMap::new(), last_transition: 0,
        }
    }

    #[inline]
    pub fn set_freq(&mut self, freq_khz: u64, now: u64) -> bool {
        if freq_khz < self.min_freq_khz || freq_khz > self.max_freq_khz { return false; }
        let duration = now.saturating_sub(self.last_transition);
        *self.time_in_state.entry(self.current_freq_khz).or_insert(0) += duration;
        self.current_freq_khz = freq_khz;
        self.transitions += 1;
        self.last_transition = now;
        true
    }

    #[inline(always)]
    pub fn boost(&mut self, now: u64) {
        self.set_freq(self.max_freq_khz, now);
        self.state = ScalingState::Boosted;
    }

    #[inline(always)]
    pub fn throttle(&mut self, now: u64) {
        self.set_freq(self.min_freq_khz, now);
        self.state = ScalingState::Throttled;
    }

    #[inline(always)]
    pub fn freq_ratio(&self) -> f64 {
        if self.max_freq_khz == 0 { return 0.0; }
        self.current_freq_khz as f64 / self.max_freq_khz as f64
    }

    #[inline]
    pub fn avg_freq_khz(&self) -> u64 {
        let total_time: u64 = self.time_in_state.values().sum();
        if total_time == 0 { return self.current_freq_khz; }
        let weighted: u64 = self.time_in_state.iter().map(|(&f, &t)| f * t / 1000).sum();
        weighted * 1000 / total_time
    }
}

/// Energy performance preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyPref {
    Default,
    Performance,
    BalancePerformance,
    BalancePower,
    Power,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FreqScalingStats {
    pub total_domains: u32,
    pub total_cpus: u32,
    pub avg_freq_ratio: f64,
    pub total_transitions: u64,
    pub boosted_domains: u32,
    pub throttled_domains: u32,
}

/// Main frequency scaling manager
pub struct HolisticFreqScaling {
    domains: BTreeMap<u32, FreqDomain>,
    next_id: u32,
    global_governor: FreqGovernor,
}

impl HolisticFreqScaling {
    pub fn new(governor: FreqGovernor) -> Self {
        Self { domains: BTreeMap::new(), next_id: 0, global_governor: governor }
    }

    #[inline]
    pub fn add_domain(&mut self, min: u64, max: u64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut domain = FreqDomain::new(id, min, max);
        domain.governor = self.global_governor;
        self.domains.insert(id, domain);
        id
    }

    #[inline(always)]
    pub fn set_freq(&mut self, domain: u32, freq_khz: u64, now: u64) -> bool {
        self.domains.get_mut(&domain).map(|d| d.set_freq(freq_khz, now)).unwrap_or(false)
    }

    #[inline(always)]
    pub fn boost_domain(&mut self, domain: u32, now: u64) {
        if let Some(d) = self.domains.get_mut(&domain) { d.boost(now); }
    }

    #[inline(always)]
    pub fn throttle_domain(&mut self, domain: u32, now: u64) {
        if let Some(d) = self.domains.get_mut(&domain) { d.throttle(now); }
    }

    pub fn stats(&self) -> FreqScalingStats {
        let cpus: u32 = self.domains.values().map(|d| d.cpus.len() as u32).sum();
        let ratios: Vec<f64> = self.domains.values().map(|d| d.freq_ratio()).collect();
        let avg = if ratios.is_empty() { 0.0 } else { ratios.iter().sum::<f64>() / ratios.len() as f64 };
        let transitions: u64 = self.domains.values().map(|d| d.transitions).sum();
        let boosted = self.domains.values().filter(|d| d.state == ScalingState::Boosted).count() as u32;
        let throttled = self.domains.values().filter(|d| d.state == ScalingState::Throttled).count() as u32;
        FreqScalingStats {
            total_domains: self.domains.len() as u32, total_cpus: cpus,
            avg_freq_ratio: avg, total_transitions: transitions,
            boosted_domains: boosted, throttled_domains: throttled,
        }
    }
}
