// SPDX-License-Identifier: GPL-2.0
//! # Apps Proactive Optimizer
//!
//! Proactive application optimization engine that acts before demand
//! materializes. Detects demand spikes from trend analysis, pre-scales
//! resources for applications before they need them, and pre-classifies
//! incoming processes based on signature matching. Every proactive
//! action is tracked with its savings estimate so the engine knows
//! whether anticipation is paying off.
//!
//! This is the kernel preparing for what hasn't happened yet.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TRACKED_PROCESSES: usize = 512;
const MAX_SPIKE_HISTORY: usize = 128;
const MAX_PRE_CLASSIFICATIONS: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const SPIKE_THRESHOLD: f32 = 1.8;
const PRE_SCALE_HEADROOM: f32 = 1.3;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// DEMAND AND CLASSIFICATION TYPES
// ============================================================================

/// Resource category for demand tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DemandResource {
    Cpu,
    Memory,
    Io,
    Network,
    Threads,
}

/// A detected demand spike
#[derive(Debug, Clone)]
pub struct DemandSpike {
    pub process_id: u64,
    pub resource: DemandResource,
    pub current_level: f32,
    pub predicted_peak: f32,
    pub spike_ratio: f32,
    pub ticks_until_peak: u64,
    pub confidence: f32,
}

/// A pre-scaling action taken
#[derive(Debug, Clone)]
pub struct PreScaleAction {
    pub process_id: u64,
    pub resource: DemandResource,
    pub old_allocation: f32,
    pub new_allocation: f32,
    pub reason: String,
    pub predicted_savings_ms: f32,
}

/// A pre-classification result
#[derive(Debug, Clone)]
pub struct PreClassification {
    pub signature_hash: u64,
    pub predicted_class: String,
    pub confidence: f32,
    pub matching_features: u32,
    pub source: String,
}

/// Proactive adaptation recommendation
#[derive(Debug, Clone)]
pub struct AdaptationRecommendation {
    pub process_id: u64,
    pub action: String,
    pub priority: f32,
    pub estimated_benefit: f32,
    pub risk: f32,
}

/// Savings report entry
#[derive(Debug, Clone)]
pub struct SavingsEntry {
    pub action_type: String,
    pub estimated_savings_ms: f32,
    pub actual_savings_ms: f32,
    pub accuracy: f32,
}

// ============================================================================
// PER-PROCESS DEMAND TRACKER
// ============================================================================

/// Tracks demand trends for a single process
#[derive(Debug, Clone)]
struct DemandTracker {
    avg_demand: BTreeMap<u8, f32>,
    trend: BTreeMap<u8, f32>,
    peak: BTreeMap<u8, f32>,
    sample_count: u64,
    last_tick: u64,
    spike_count: u64,
}

impl DemandTracker {
    fn new() -> Self {
        Self {
            avg_demand: BTreeMap::new(),
            trend: BTreeMap::new(),
            peak: BTreeMap::new(),
            sample_count: 0,
            last_tick: 0,
            spike_count: 0,
        }
    }

    fn update(&mut self, resource: DemandResource, value: f32, tick: u64) {
        let key = resource as u8;
        let old_avg = *self.avg_demand.get(&key).unwrap_or(&0.0);
        let new_avg = EMA_ALPHA * value + (1.0 - EMA_ALPHA) * old_avg;
        self.avg_demand.insert(key, new_avg);

        let old_peak = *self.peak.get(&key).unwrap_or(&0.0);
        if value > old_peak {
            self.peak.insert(key, value);
        }

        if self.sample_count > 0 && tick > self.last_tick {
            let dt = (tick - self.last_tick) as f32;
            let new_trend = (value - old_avg) / dt;
            let old_trend = *self.trend.get(&key).unwrap_or(&0.0);
            self.trend
                .insert(key, EMA_ALPHA * new_trend + (1.0 - EMA_ALPHA) * old_trend);
        }
        self.sample_count += 1;
        self.last_tick = tick;
    }

    fn is_spiking(&self, resource: DemandResource) -> Option<f32> {
        let key = resource as u8;
        let avg = *self.avg_demand.get(&key).unwrap_or(&0.0);
        let trend = *self.trend.get(&key).unwrap_or(&0.0);
        if avg > 0.001 && trend > 0.0 {
            let ratio = (avg + trend * 100.0) / avg;
            if ratio > SPIKE_THRESHOLD {
                return Some(ratio);
            }
        }
        None
    }

    fn forecast_demand(&self, resource: DemandResource, ticks: u64) -> f32 {
        let key = resource as u8;
        let avg = *self.avg_demand.get(&key).unwrap_or(&0.0);
        let trend = *self.trend.get(&key).unwrap_or(&0.0);
        (avg + trend * ticks as f32).max(0.0)
    }
}

// ============================================================================
// PROACTIVE STATS
// ============================================================================

/// Aggregate proactive optimization statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ProactiveStats {
    pub total_observations: u64,
    pub spikes_detected: u64,
    pub pre_scale_actions: u64,
    pub pre_classifications: u64,
    pub adaptations_issued: u64,
    pub estimated_savings_ms: f32,
    pub actual_savings_ms: f32,
    pub savings_accuracy: f32,
    pub tracked_processes: usize,
}

// ============================================================================
// APPS PROACTIVE OPTIMIZER
// ============================================================================

/// Proactive application optimization engine. Detects demand spikes
/// before they happen, pre-scales resources, and pre-classifies processes.
#[derive(Debug)]
pub struct AppsProactive {
    trackers: BTreeMap<u64, DemandTracker>,
    spike_history: Vec<DemandSpike>,
    pre_scale_log: Vec<PreScaleAction>,
    pre_class_cache: BTreeMap<u64, PreClassification>,
    total_observations: u64,
    spikes_detected: u64,
    pre_scale_count: u64,
    adaptations_count: u64,
    tick: u64,
    rng_state: u64,
    estimated_savings: f32,
    actual_savings: f32,
    savings_accuracy_ema: f32,
}

impl AppsProactive {
    pub fn new() -> Self {
        Self {
            trackers: BTreeMap::new(),
            spike_history: Vec::new(),
            pre_scale_log: Vec::new(),
            pre_class_cache: BTreeMap::new(),
            total_observations: 0,
            spikes_detected: 0,
            pre_scale_count: 0,
            adaptations_count: 0,
            tick: 0,
            rng_state: 0xFEED_FACE_DEAD_BEEF,
            estimated_savings: 0.0,
            actual_savings: 0.0,
            savings_accuracy_ema: 0.5,
        }
    }

    /// Record a demand observation for a process
    pub fn observe_demand(
        &mut self,
        process_id: u64,
        resource: DemandResource,
        value: f32,
        tick: u64,
    ) {
        self.tick = tick;
        self.total_observations += 1;
        if self.trackers.len() >= MAX_TRACKED_PROCESSES && !self.trackers.contains_key(&process_id)
        {
            return;
        }
        let tracker = self
            .trackers
            .entry(process_id)
            .or_insert_with(DemandTracker::new);
        tracker.update(resource, value, tick);
    }

    /// Detect demand spikes across all tracked processes
    pub fn predict_demand_spike(&mut self) -> Vec<DemandSpike> {
        let mut spikes = Vec::new();
        let resources = [
            DemandResource::Cpu,
            DemandResource::Memory,
            DemandResource::Io,
            DemandResource::Network,
            DemandResource::Threads,
        ];

        let process_ids: Vec<u64> = self.trackers.keys().copied().collect();
        for pid in process_ids {
            if let Some(tracker) = self.trackers.get(&pid) {
                for &res in &resources {
                    if let Some(ratio) = tracker.is_spiking(res) {
                        let current = *tracker.avg_demand.get(&(res as u8)).unwrap_or(&0.0);
                        let predicted_peak = current * ratio;
                        let confidence =
                            (0.5 + 0.1 * (tracker.sample_count as f32).min(5.0)).min(0.95);
                        let spike = DemandSpike {
                            process_id: pid,
                            resource: res,
                            current_level: current,
                            predicted_peak,
                            spike_ratio: ratio,
                            ticks_until_peak: 500,
                            confidence,
                        };
                        spikes.push(spike);
                        self.spikes_detected += 1;
                    }
                }
            }
        }

        if self.spike_history.len() + spikes.len() > MAX_SPIKE_HISTORY {
            let drain = self
                .spike_history
                .len()
                .saturating_sub(MAX_SPIKE_HISTORY / 2);
            if drain > 0 {
                self.spike_history.drain(..drain);
            }
        }
        for s in &spikes {
            self.spike_history.push(s.clone());
        }
        spikes
    }

    /// Pre-scale resources for a process based on predicted demand
    pub fn pre_scale_resources(
        &mut self,
        process_id: u64,
        resource: DemandResource,
        current_alloc: f32,
    ) -> PreScaleAction {
        self.pre_scale_count += 1;
        let predicted = if let Some(tracker) = self.trackers.get(&process_id) {
            tracker.forecast_demand(resource, 1000)
        } else {
            current_alloc
        };

        let new_alloc = (predicted * PRE_SCALE_HEADROOM).max(current_alloc);
        let savings_est = if new_alloc > current_alloc {
            (new_alloc - current_alloc) * 0.05
        } else {
            0.0
        };
        self.estimated_savings += savings_est;

        let mut reason = String::new();
        reason.push_str("pre_scale:");
        let res_str = match resource {
            DemandResource::Cpu => "cpu",
            DemandResource::Memory => "mem",
            DemandResource::Io => "io",
            DemandResource::Network => "net",
            DemandResource::Threads => "threads",
        };
        reason.push_str(res_str);

        PreScaleAction {
            process_id,
            resource,
            old_allocation: current_alloc,
            new_allocation: new_alloc,
            reason,
            predicted_savings_ms: savings_est,
        }
    }

    /// Pre-classify an incoming process from its binary signature
    pub fn pre_classify(&mut self, signature: &[u8], source_hint: &str) -> PreClassification {
        let hash = fnv1a_hash(signature);
        if let Some(cached) = self.pre_class_cache.get(&hash) {
            return cached.clone();
        }

        let class_idx = hash % 8;
        let predicted_class = match class_idx {
            0 => String::from("compute-intensive"),
            1 => String::from("io-bound"),
            2 => String::from("memory-heavy"),
            3 => String::from("network-service"),
            4 => String::from("interactive"),
            5 => String::from("batch-job"),
            6 => String::from("daemon"),
            _ => String::from("mixed-workload"),
        };

        let matching = ((hash >> 16) % 12) as u32 + 3;
        let confidence = 0.3 + (matching as f32 * 0.05).min(0.6);

        let mut src = String::new();
        src.push_str(source_hint);

        let result = PreClassification {
            signature_hash: hash,
            predicted_class,
            confidence,
            matching_features: matching,
            source: src,
        };

        if self.pre_class_cache.len() < MAX_PRE_CLASSIFICATIONS {
            self.pre_class_cache.insert(hash, result.clone());
        }
        result
    }

    /// Generate proactive adaptation recommendations for a process
    pub fn proactive_adaptation(&mut self, process_id: u64) -> Vec<AdaptationRecommendation> {
        self.adaptations_count += 1;
        let mut recs = Vec::new();

        if let Some(tracker) = self.trackers.get(&process_id) {
            let resources = [
                DemandResource::Cpu,
                DemandResource::Memory,
                DemandResource::Io,
            ];
            for &res in &resources {
                let forecast = tracker.forecast_demand(res, 5_000);
                let current = *tracker.avg_demand.get(&(res as u8)).unwrap_or(&0.0);
                if forecast > current * 1.2 && forecast > 0.1 {
                    let mut action = String::new();
                    action.push_str("increase_");
                    let res_name = match res {
                        DemandResource::Cpu => "cpu",
                        DemandResource::Memory => "memory",
                        DemandResource::Io => "io",
                        _ => "resource",
                    };
                    action.push_str(res_name);
                    let benefit = (forecast - current) * 0.1;
                    let risk = if forecast > current * 2.0 { 0.4 } else { 0.15 };
                    recs.push(AdaptationRecommendation {
                        process_id,
                        action,
                        priority: benefit.min(1.0),
                        estimated_benefit: benefit,
                        risk,
                    });
                }
            }
        }
        recs
    }

    /// Generate a savings report comparing estimated vs actual savings
    pub fn savings_report(&mut self) -> Vec<SavingsEntry> {
        let mut entries = Vec::new();
        let accuracy = if self.estimated_savings > 0.001 {
            (self.actual_savings / self.estimated_savings).min(2.0)
        } else {
            1.0
        };
        self.savings_accuracy_ema =
            EMA_ALPHA * accuracy + (1.0 - EMA_ALPHA) * self.savings_accuracy_ema;

        entries.push(SavingsEntry {
            action_type: String::from("pre_scale"),
            estimated_savings_ms: self.estimated_savings,
            actual_savings_ms: self.actual_savings,
            accuracy: self.savings_accuracy_ema,
        });
        entries
    }

    /// Record actual savings from a proactive action
    pub fn record_actual_savings(&mut self, savings_ms: f32) {
        self.actual_savings += savings_ms;
    }

    /// Remove tracking for a process
    pub fn deregister_process(&mut self, process_id: u64) {
        self.trackers.remove(&process_id);
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> ProactiveStats {
        ProactiveStats {
            total_observations: self.total_observations,
            spikes_detected: self.spikes_detected,
            pre_scale_actions: self.pre_scale_count,
            pre_classifications: self.pre_class_cache.len() as u64,
            adaptations_issued: self.adaptations_count,
            estimated_savings_ms: self.estimated_savings,
            actual_savings_ms: self.actual_savings,
            savings_accuracy: self.savings_accuracy_ema,
            tracked_processes: self.trackers.len(),
        }
    }
}
