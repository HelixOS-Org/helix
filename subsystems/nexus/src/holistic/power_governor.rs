//! # Holistic Power Governor
//!
//! System-wide power/energy management:
//! - CPU frequency scaling (DVFS)
//! - Power domain management
//! - Energy budget enforcement
//! - Thermal-aware power limiting
//! - C-state management
//! - Package power capping (RAPL-style)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// POWER TYPES
// ============================================================================

/// Power policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPolicy {
    /// Maximum performance (all cores at max)
    Performance,
    /// Balanced (dynamic scaling)
    Balanced,
    /// Power save (aggressive downclocking)
    PowerSave,
    /// Battery critical (emergency low power)
    Emergency,
}

/// C-state depth
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CState {
    /// Active
    C0,
    /// Halt (fast wake)
    C1,
    /// Stop clock
    C2,
    /// Sleep (L1 may be lost)
    C3,
    /// Deep sleep (LLC may be lost)
    C6,
    /// Package sleep
    C7,
}

/// Frequency scaling governor action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqAction {
    /// Step up frequency
    StepUp,
    /// Step down frequency
    StepDown,
    /// Jump to max
    JumpMax,
    /// Jump to min
    JumpMin,
    /// Hold current
    Hold,
}

// ============================================================================
// CPU FREQUENCY STATE
// ============================================================================

/// Per-CPU frequency state
#[derive(Debug, Clone)]
pub struct CpuFreqState {
    /// CPU ID
    pub cpu_id: u32,
    /// Current frequency (MHz)
    pub current_mhz: u32,
    /// Min frequency (MHz)
    pub min_mhz: u32,
    /// Max frequency (MHz)
    pub max_mhz: u32,
    /// Available frequencies
    pub available_freqs: Vec<u32>,
    /// Current C-state
    pub current_cstate: CState,
    /// Utilization (0..1)
    pub utilization: f64,
    /// Utilization EMA
    pub util_ema: f64,
    /// Energy consumed (µJ)
    pub energy_uj: u64,
    /// Residency in C1+ (ratio)
    pub idle_residency: f64,
}

impl CpuFreqState {
    pub fn new(cpu_id: u32, min_mhz: u32, max_mhz: u32) -> Self {
        // Generate available frequencies (steps of 100MHz)
        let mut freqs = Vec::new();
        let mut f = min_mhz;
        while f <= max_mhz {
            freqs.push(f);
            f += 100;
        }
        if freqs.is_empty() || *freqs.last().unwrap() != max_mhz {
            freqs.push(max_mhz);
        }
        Self {
            cpu_id,
            current_mhz: max_mhz,
            min_mhz,
            max_mhz,
            available_freqs: freqs,
            current_cstate: CState::C0,
            utilization: 0.0,
            util_ema: 0.0,
            energy_uj: 0,
            idle_residency: 0.0,
        }
    }

    /// Update utilization
    pub fn update_utilization(&mut self, util: f64) {
        self.utilization = util.max(0.0).min(1.0);
        self.util_ema = 0.8 * self.util_ema + 0.2 * self.utilization;
    }

    /// Decide frequency action based on policy
    pub fn decide_action(&self, policy: PowerPolicy) -> FreqAction {
        match policy {
            PowerPolicy::Performance => {
                if self.current_mhz < self.max_mhz {
                    FreqAction::JumpMax
                } else {
                    FreqAction::Hold
                }
            }
            PowerPolicy::Balanced => {
                if self.util_ema > 0.8 {
                    FreqAction::StepUp
                } else if self.util_ema < 0.3 {
                    FreqAction::StepDown
                } else {
                    FreqAction::Hold
                }
            }
            PowerPolicy::PowerSave => {
                if self.util_ema > 0.9 {
                    FreqAction::StepUp
                } else if self.util_ema < 0.5 {
                    FreqAction::StepDown
                } else {
                    FreqAction::Hold
                }
            }
            PowerPolicy::Emergency => {
                if self.current_mhz > self.min_mhz {
                    FreqAction::JumpMin
                } else {
                    FreqAction::Hold
                }
            }
        }
    }

    /// Apply action
    pub fn apply_action(&mut self, action: FreqAction) {
        match action {
            FreqAction::StepUp => {
                if let Some(pos) = self.available_freqs.iter().position(|&f| f == self.current_mhz) {
                    if pos + 1 < self.available_freqs.len() {
                        self.current_mhz = self.available_freqs[pos + 1];
                    }
                }
            }
            FreqAction::StepDown => {
                if let Some(pos) = self.available_freqs.iter().position(|&f| f == self.current_mhz) {
                    if pos > 0 {
                        self.current_mhz = self.available_freqs[pos - 1];
                    }
                }
            }
            FreqAction::JumpMax => self.current_mhz = self.max_mhz,
            FreqAction::JumpMin => self.current_mhz = self.min_mhz,
            FreqAction::Hold => {}
        }
    }

    /// Estimated power (simplified cubic model: P ~ V^2 * f ~ f^3)
    pub fn estimated_power_mw(&self) -> u64 {
        let freq_ratio = self.current_mhz as f64 / self.max_mhz as f64;
        let base_power = 15000.0; // 15W TDP per core at max
        (base_power * freq_ratio * freq_ratio * freq_ratio * self.utilization) as u64
    }
}

// ============================================================================
// POWER DOMAIN
// ============================================================================

/// Power domain (package/socket level)
#[derive(Debug)]
pub struct PowerDomain {
    /// Domain ID
    pub domain_id: u32,
    /// Power cap (mW, 0 = unlimited)
    pub power_cap_mw: u64,
    /// Current power (mW EMA)
    pub power_ema_mw: f64,
    /// Energy counter (µJ)
    pub energy_uj: u64,
    /// CPUs in domain
    pub cpus: Vec<u32>,
    /// Thermal throttle active
    pub thermal_throttle: bool,
    /// Temperature (milli-Celsius)
    pub temperature_mc: u32,
    /// Thermal limit (milli-Celsius)
    pub thermal_limit_mc: u32,
}

impl PowerDomain {
    pub fn new(domain_id: u32) -> Self {
        Self {
            domain_id,
            power_cap_mw: 0,
            power_ema_mw: 0.0,
            energy_uj: 0,
            cpus: Vec::new(),
            thermal_throttle: false,
            temperature_mc: 40000,
            thermal_limit_mc: 95000,
        }
    }

    /// Set power cap
    pub fn set_power_cap(&mut self, cap_mw: u64) {
        self.power_cap_mw = cap_mw;
    }

    /// Update power
    pub fn update_power(&mut self, current_mw: u64) {
        self.power_ema_mw = 0.7 * self.power_ema_mw + 0.3 * current_mw as f64;
    }

    /// Check thermal
    pub fn check_thermal(&mut self, temp_mc: u32) {
        self.temperature_mc = temp_mc;
        self.thermal_throttle = temp_mc > self.thermal_limit_mc;
    }

    /// Is over budget?
    pub fn is_over_budget(&self) -> bool {
        self.power_cap_mw > 0 && self.power_ema_mw > self.power_cap_mw as f64
    }

    /// Budget headroom (mW, negative = over)
    pub fn headroom_mw(&self) -> f64 {
        if self.power_cap_mw == 0 {
            return f64::INFINITY;
        }
        self.power_cap_mw as f64 - self.power_ema_mw
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Power governor stats
#[derive(Debug, Clone, Default)]
pub struct HolisticPowerGovernorStats {
    /// Active CPUs
    pub active_cpus: usize,
    /// Power domains
    pub power_domains: usize,
    /// Current policy
    pub policy: u8,
    /// Total power (mW)
    pub total_power_mw: u64,
    /// Average frequency ratio
    pub avg_freq_ratio: f64,
    /// Domains over budget
    pub over_budget_domains: usize,
    /// Thermal throttled domains
    pub thermal_throttled: usize,
}

/// System-wide power governor
pub struct HolisticPowerGovernor {
    /// CPU states
    cpus: BTreeMap<u32, CpuFreqState>,
    /// Power domains
    domains: BTreeMap<u32, PowerDomain>,
    /// Current policy
    policy: PowerPolicy,
    /// Stats
    stats: HolisticPowerGovernorStats,
}

impl HolisticPowerGovernor {
    pub fn new(policy: PowerPolicy) -> Self {
        Self {
            cpus: BTreeMap::new(),
            domains: BTreeMap::new(),
            policy,
            stats: HolisticPowerGovernorStats::default(),
        }
    }

    /// Register CPU
    pub fn register_cpu(&mut self, cpu_id: u32, min_mhz: u32, max_mhz: u32, domain_id: u32) {
        self.cpus.insert(cpu_id, CpuFreqState::new(cpu_id, min_mhz, max_mhz));
        let domain = self.domains.entry(domain_id).or_insert_with(|| PowerDomain::new(domain_id));
        domain.cpus.push(cpu_id);
        self.update_stats();
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: PowerPolicy) {
        self.policy = policy;
        self.update_stats();
    }

    /// Update CPU utilization
    pub fn update_cpu_util(&mut self, cpu_id: u32, util: f64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.update_utilization(util);
        }
    }

    /// Tick governor (evaluate and apply frequency changes)
    pub fn tick(&mut self) {
        let policy = self.policy;
        let cpu_ids: Vec<u32> = self.cpus.keys().copied().collect();

        for &cpu_id in &cpu_ids {
            if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
                let mut effective_policy = policy;
                // Check if domain is over budget or thermal throttled
                for domain in self.domains.values() {
                    if domain.cpus.contains(&cpu_id) {
                        if domain.thermal_throttle || domain.is_over_budget() {
                            effective_policy = PowerPolicy::Emergency;
                        }
                        break;
                    }
                }
                let action = cpu.decide_action(effective_policy);
                cpu.apply_action(action);
            }
        }

        // Update domain power
        for domain in self.domains.values_mut() {
            let mut total_mw = 0u64;
            for &cpu_id in &domain.cpus {
                if let Some(cpu) = self.cpus.get(&cpu_id) {
                    total_mw += cpu.estimated_power_mw();
                }
            }
            domain.update_power(total_mw);
        }

        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_cpus = self.cpus.len();
        self.stats.power_domains = self.domains.len();
        self.stats.policy = self.policy as u8;

        let mut total_power = 0u64;
        let mut freq_ratio_sum = 0.0f64;
        for cpu in self.cpus.values() {
            total_power += cpu.estimated_power_mw();
            if cpu.max_mhz > 0 {
                freq_ratio_sum += cpu.current_mhz as f64 / cpu.max_mhz as f64;
            }
        }
        self.stats.total_power_mw = total_power;
        self.stats.avg_freq_ratio = if !self.cpus.is_empty() {
            freq_ratio_sum / self.cpus.len() as f64
        } else {
            0.0
        };
        self.stats.over_budget_domains = self.domains.values().filter(|d| d.is_over_budget()).count();
        self.stats.thermal_throttled = self.domains.values().filter(|d| d.thermal_throttle).count();
    }

    /// Stats
    pub fn stats(&self) -> &HolisticPowerGovernorStats {
        &self.stats
    }
}
