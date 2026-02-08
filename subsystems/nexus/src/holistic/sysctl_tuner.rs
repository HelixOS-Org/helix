//! # Holistic Sysctl Tuner
//!
//! Dynamic kernel parameter tuning:
//! - Parameter categories and ranges
//! - Auto-tuning based on workload profiles
//! - Parameter dependency tracking
//! - Change history and rollback
//! - Impact scoring for parameter changes
//! - Safe bounds enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Integer,
    Boolean,
    Percentage,
    Bytes,
    Milliseconds,
    Pages,
    Custom,
}

/// Parameter category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamCategory {
    Vm,
    Kernel,
    Net,
    Fs,
    Debug,
    Sched,
    Ipc,
}

/// Parameter bounds
#[derive(Debug, Clone)]
pub struct ParamBounds {
    pub min_value: i64,
    pub max_value: i64,
    pub default_value: i64,
    pub safe_min: i64,
    pub safe_max: i64,
}

impl ParamBounds {
    pub fn new(min: i64, max: i64, default: i64) -> Self {
        Self { min_value: min, max_value: max, default_value: default, safe_min: min, safe_max: max }
    }

    pub fn with_safe_range(mut self, safe_min: i64, safe_max: i64) -> Self {
        self.safe_min = safe_min;
        self.safe_max = safe_max;
        self
    }

    pub fn clamp(&self, value: i64) -> i64 {
        value.max(self.min_value).min(self.max_value)
    }

    pub fn is_safe(&self, value: i64) -> bool {
        value >= self.safe_min && value <= self.safe_max
    }
}

/// Kernel parameter definition
#[derive(Debug, Clone)]
pub struct SysctlParam {
    pub name: String,
    pub category: ParamCategory,
    pub param_type: ParamType,
    pub current_value: i64,
    pub bounds: ParamBounds,
    pub dependencies: Vec<String>,
    pub impact_score: f64,
    pub change_count: u32,
    pub last_change_ts: u64,
}

impl SysctlParam {
    pub fn new(name: String, cat: ParamCategory, ptype: ParamType, bounds: ParamBounds) -> Self {
        Self {
            name, category: cat, param_type: ptype,
            current_value: bounds.default_value, bounds,
            dependencies: Vec::new(), impact_score: 1.0,
            change_count: 0, last_change_ts: 0,
        }
    }

    pub fn set_value(&mut self, value: i64, ts: u64) -> bool {
        let clamped = self.bounds.clamp(value);
        if clamped != self.current_value {
            self.current_value = clamped;
            self.change_count += 1;
            self.last_change_ts = ts;
            true
        } else { false }
    }

    pub fn is_default(&self) -> bool { self.current_value == self.bounds.default_value }
    pub fn is_safe(&self) -> bool { self.bounds.is_safe(self.current_value) }
    pub fn deviation_pct(&self) -> f64 {
        let range = (self.bounds.max_value - self.bounds.min_value) as f64;
        if range <= 0.0 { 0.0 }
        else { libm::fabs((self.current_value - self.bounds.default_value) as f64) / range * 100.0 }
    }
}

/// Change history entry
#[derive(Debug, Clone)]
pub struct ParamChange {
    pub param_name: String,
    pub old_value: i64,
    pub new_value: i64,
    pub timestamp: u64,
    pub reason: ChangeReason,
}

/// Change reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeReason {
    UserRequest,
    AutoTune,
    WorkloadAdapt,
    PressureResponse,
    Rollback,
}

/// Workload profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadProfile {
    Throughput,
    Latency,
    Balanced,
    MemoryIntensive,
    IoIntensive,
    CpuIntensive,
}

/// Profile-specific tuning recommendations
#[derive(Debug, Clone)]
pub struct TuningRecommendation {
    pub param_name: String,
    pub recommended_value: i64,
    pub profile: WorkloadProfile,
    pub confidence: f64,
}

/// Sysctl tuner stats
#[derive(Debug, Clone, Default)]
pub struct SysctlTunerStats {
    pub total_params: usize,
    pub modified_params: usize,
    pub unsafe_params: usize,
    pub total_changes: u64,
    pub auto_tuned_changes: u64,
    pub rollbacks: u64,
    pub avg_impact_score: f64,
    pub active_profile: u8,
}

/// Holistic sysctl tuner
pub struct HolisticSysctlTuner {
    params: BTreeMap<String, SysctlParam>,
    history: Vec<ParamChange>,
    max_history: usize,
    recommendations: Vec<TuningRecommendation>,
    current_profile: WorkloadProfile,
    stats: SysctlTunerStats,
}

impl HolisticSysctlTuner {
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(), history: Vec::new(),
            max_history: 1000, recommendations: Vec::new(),
            current_profile: WorkloadProfile::Balanced,
            stats: SysctlTunerStats::default(),
        }
    }

    pub fn register_param(&mut self, param: SysctlParam) {
        self.params.insert(param.name.clone(), param);
    }

    pub fn set_value(&mut self, name: &str, value: i64, ts: u64, reason: ChangeReason) -> bool {
        let name_string = String::from(name);
        if let Some(p) = self.params.get_mut(&name_string) {
            let old = p.current_value;
            if p.set_value(value, ts) {
                self.history.push(ParamChange {
                    param_name: name_string, old_value: old, new_value: p.current_value,
                    timestamp: ts, reason,
                });
                if self.history.len() > self.max_history {
                    self.history.remove(0);
                }
                return true;
            }
        }
        false
    }

    pub fn rollback_last(&mut self, ts: u64) -> bool {
        if let Some(change) = self.history.pop() {
            let name = change.param_name.clone();
            if let Some(p) = self.params.get_mut(&name) {
                p.current_value = change.old_value;
                p.change_count += 1;
                p.last_change_ts = ts;
                self.history.push(ParamChange {
                    param_name: name, old_value: change.new_value,
                    new_value: change.old_value, timestamp: ts, reason: ChangeReason::Rollback,
                });
                return true;
            }
        }
        false
    }

    pub fn set_profile(&mut self, profile: WorkloadProfile) {
        self.current_profile = profile;
    }

    pub fn generate_recommendations(&mut self) {
        self.recommendations.clear();
        for (name, param) in &self.params {
            let rec_value = match (self.current_profile, param.category) {
                (WorkloadProfile::Throughput, ParamCategory::Vm) => param.bounds.max_value * 3 / 4,
                (WorkloadProfile::Latency, ParamCategory::Sched) => param.bounds.min_value + (param.bounds.max_value - param.bounds.min_value) / 4,
                (WorkloadProfile::MemoryIntensive, ParamCategory::Vm) => param.bounds.max_value * 9 / 10,
                (WorkloadProfile::IoIntensive, ParamCategory::Fs) => param.bounds.max_value * 8 / 10,
                _ => param.bounds.default_value,
            };
            if rec_value != param.current_value {
                self.recommendations.push(TuningRecommendation {
                    param_name: name.clone(), recommended_value: rec_value,
                    profile: self.current_profile, confidence: 0.7,
                });
            }
        }
    }

    pub fn apply_recommendations(&mut self, ts: u64) {
        let recs: Vec<_> = self.recommendations.drain(..).collect();
        for rec in recs {
            if rec.confidence >= 0.5 {
                self.set_value(&rec.param_name, rec.recommended_value, ts, ChangeReason::AutoTune);
            }
        }
    }

    pub fn params_by_category(&self, cat: ParamCategory) -> Vec<&SysctlParam> {
        self.params.values().filter(|p| p.category == cat).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_params = self.params.len();
        self.stats.modified_params = self.params.values().filter(|p| !p.is_default()).count();
        self.stats.unsafe_params = self.params.values().filter(|p| !p.is_safe()).count();
        self.stats.total_changes = self.history.len() as u64;
        self.stats.auto_tuned_changes = self.history.iter().filter(|h| h.reason == ChangeReason::AutoTune).count() as u64;
        self.stats.rollbacks = self.history.iter().filter(|h| h.reason == ChangeReason::Rollback).count() as u64;
        let impacts: Vec<f64> = self.params.values().map(|p| p.impact_score).collect();
        self.stats.avg_impact_score = if impacts.is_empty() { 0.0 } else { impacts.iter().sum::<f64>() / impacts.len() as f64 };
        self.stats.active_profile = self.current_profile as u8;
    }

    pub fn param(&self, name: &str) -> Option<&SysctlParam> { self.params.get(&String::from(name)) }
    pub fn stats(&self) -> &SysctlTunerStats { &self.stats }
    pub fn history(&self) -> &[ParamChange] { &self.history }
}
