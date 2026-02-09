// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Empathy Engine
//!
//! Understanding each process's cooperation needs from the inside. The empathy
//! engine models what each process requires from cooperation — guaranteed
//! latency, fair share, burst capacity, predictable allocation — and tracks
//! how well those needs are being met. This enables proactive cooperation
//! adjustments before dissatisfaction becomes contention.
//!
//! ## Empathy Profiles
//!
//! Each process has an `EmpathyProfile` capturing its inferred cooperation
//! needs, current satisfaction level, and need-conflict status with other
//! processes. The engine continuously updates these profiles using EMA
//! smoothing and detects cross-process conflicts early.
//!
//! ## Key Methods
//!
//! - `empathize_with_process()` — Build/update empathy profile for a process
//! - `infer_cooperation_needs()` — Infer needs from behavioral signals
//! - `satisfaction_model()` — Model how satisfied a process is with cooperation
//! - `need_conflict_detection()` — Detect conflicting needs between processes
//! - `empathy_score()` — Compute empathy accuracy for a process
//! - `cross_process_understanding()` — How well do processes understand each other?

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_PROCESSES: usize = 512;
const MAX_NEED_HISTORY: usize = 64;
const SATISFACTION_THRESHOLD: f32 = 0.7;
const CONFLICT_THRESHOLD: f32 = 0.6;
const NEED_DECAY: f32 = 0.99;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Xorshift64 PRNG for need inference noise
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// COOPERATION NEED KIND
// ============================================================================

/// Types of cooperation needs a process can have
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopNeedKind {
    /// Guaranteed latency bounds
    LatencyGuarantee,
    /// Fair proportional share of resources
    FairShare,
    /// Burst capacity when needed
    BurstCapacity,
    /// Predictable, stable allocation
    PredictableAllocation,
    /// Isolation from noisy neighbors
    NoiseIsolation,
    /// Priority access during critical sections
    CriticalPriority,
}

impl CoopNeedKind {
    pub fn all() -> &'static [CoopNeedKind] {
        &[
            CoopNeedKind::LatencyGuarantee,
            CoopNeedKind::FairShare,
            CoopNeedKind::BurstCapacity,
            CoopNeedKind::PredictableAllocation,
            CoopNeedKind::NoiseIsolation,
            CoopNeedKind::CriticalPriority,
        ]
    }
}

// ============================================================================
// COOPERATION NEED
// ============================================================================

/// A single inferred cooperation need with intensity and satisfaction
#[derive(Debug, Clone)]
pub struct CoopNeed {
    pub kind: CoopNeedKind,
    /// How strongly this process needs this (0.0–1.0)
    pub intensity: f32,
    /// How well this need is currently satisfied (0.0–1.0)
    pub satisfaction: f32,
    /// Gap = intensity - satisfaction (positive = unmet)
    pub gap: f32,
    /// Confidence in the inference (0.0–1.0)
    pub confidence: f32,
    /// EMA-smoothed intensity
    pub smoothed_intensity: f32,
    /// Number of observations supporting this need
    pub observations: u64,
    /// History of intensity observations
    history: Vec<f32>,
    write_idx: usize,
}

impl CoopNeed {
    pub fn new(kind: CoopNeedKind) -> Self {
        let mut history = Vec::with_capacity(MAX_NEED_HISTORY);
        for _ in 0..MAX_NEED_HISTORY {
            history.push(0.0);
        }
        Self {
            kind,
            intensity: 0.0,
            satisfaction: 0.5,
            gap: 0.0,
            confidence: 0.0,
            smoothed_intensity: 0.0,
            observations: 0,
            history,
            write_idx: 0,
        }
    }

    /// Observe a new need intensity signal
    pub fn observe_intensity(&mut self, raw: f32) {
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };
        self.intensity = clamped;
        self.smoothed_intensity += EMA_ALPHA * (clamped - self.smoothed_intensity);
        self.history[self.write_idx] = clamped;
        self.write_idx = (self.write_idx + 1) % MAX_NEED_HISTORY;
        self.observations += 1;
        // Confidence grows with observations
        let conf_raw = (self.observations as f32 / 20.0).min(1.0);
        self.confidence += EMA_ALPHA * (conf_raw - self.confidence);
        self.update_gap();
    }

    /// Update satisfaction level
    pub fn update_satisfaction(&mut self, sat: f32) {
        let clamped = if sat < 0.0 { 0.0 } else if sat > 1.0 { 1.0 } else { sat };
        self.satisfaction += EMA_ALPHA * (clamped - self.satisfaction);
        self.update_gap();
    }

    fn update_gap(&mut self) {
        self.gap = (self.smoothed_intensity - self.satisfaction).max(0.0);
    }

    /// Decay intensity over time
    pub fn decay(&mut self) {
        self.smoothed_intensity *= NEED_DECAY;
        self.update_gap();
    }

    /// Average intensity from history
    pub fn history_average(&self) -> f32 {
        let count = if self.observations < MAX_NEED_HISTORY as u64 {
            self.observations as usize
        } else {
            MAX_NEED_HISTORY
        };
        if count == 0 {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += self.history[i];
        }
        sum / count as f32
    }
}

// ============================================================================
// EMPATHY PROFILE
// ============================================================================

/// Complete empathy profile for a single cooperating process
#[derive(Debug, Clone)]
pub struct EmpathyProfile {
    pub process_id: u64,
    pub cooperation_needs: BTreeMap<u8, CoopNeed>,
    /// Overall satisfaction (EMA-smoothed aggregate)
    pub overall_satisfaction: f32,
    /// Empathy accuracy score — how well we predicted this process's needs
    pub empathy_accuracy: f32,
    /// Number of empathy evaluations
    pub evaluation_count: u64,
    /// Tick of last evaluation
    pub last_evaluation_tick: u64,
    /// Dominant unmet need
    pub dominant_unmet: CoopNeedKind,
    /// Total need gap across all needs
    pub total_gap: f32,
}

impl EmpathyProfile {
    pub fn new(process_id: u64) -> Self {
        let mut cooperation_needs = BTreeMap::new();
        for kind in CoopNeedKind::all() {
            cooperation_needs.insert(*kind as u8, CoopNeed::new(*kind));
        }
        Self {
            process_id,
            cooperation_needs,
            overall_satisfaction: 0.5,
            empathy_accuracy: 0.0,
            evaluation_count: 0,
            last_evaluation_tick: 0,
            dominant_unmet: CoopNeedKind::FairShare,
            total_gap: 0.0,
        }
    }

    /// Recompute aggregate metrics
    pub fn recompute_aggregates(&mut self) {
        let mut total_sat = 0.0f32;
        let mut total_gap = 0.0f32;
        let mut worst_gap = 0.0f32;
        let mut worst_kind = CoopNeedKind::FairShare;
        let mut count = 0usize;

        for (_, need) in self.cooperation_needs.iter() {
            total_sat += need.satisfaction;
            total_gap += need.gap;
            if need.gap > worst_gap {
                worst_gap = need.gap;
                worst_kind = need.kind;
            }
            count += 1;
        }

        if count > 0 {
            let avg_sat = total_sat / count as f32;
            self.overall_satisfaction += EMA_ALPHA * (avg_sat - self.overall_satisfaction);
        }
        self.total_gap = total_gap;
        self.dominant_unmet = worst_kind;
    }
}

// ============================================================================
// NEED CONFLICT
// ============================================================================

/// A detected conflict between the needs of two processes
#[derive(Debug, Clone)]
pub struct NeedConflict {
    pub conflict_id: u64,
    pub process_a: u64,
    pub process_b: u64,
    pub need_kind: CoopNeedKind,
    /// Severity of the conflict (0.0–1.0)
    pub severity: f32,
    /// Description of the conflict
    pub description: String,
    /// Tick when detected
    pub detected_tick: u64,
}

// ============================================================================
// EMPATHY STATS
// ============================================================================

#[derive(Debug, Clone)]
pub struct CoopEmpathyStats {
    pub tracked_processes: usize,
    pub total_evaluations: u64,
    pub avg_satisfaction: f32,
    pub avg_empathy_accuracy: f32,
    pub active_conflicts: usize,
    pub total_conflicts_detected: u64,
    pub worst_satisfaction_process: u64,
    pub avg_need_gap: f32,
    pub cross_understanding_score: f32,
}

impl CoopEmpathyStats {
    pub fn new() -> Self {
        Self {
            tracked_processes: 0,
            total_evaluations: 0,
            avg_satisfaction: 0.5,
            avg_empathy_accuracy: 0.0,
            active_conflicts: 0,
            total_conflicts_detected: 0,
            worst_satisfaction_process: 0,
            avg_need_gap: 0.0,
            cross_understanding_score: 0.0,
        }
    }
}

// ============================================================================
// COOPERATION EMPATHY ENGINE
// ============================================================================

/// Engine for modeling each process's cooperation needs
pub struct CoopEmpathyEngine {
    profiles: BTreeMap<u64, EmpathyProfile>,
    conflicts: Vec<NeedConflict>,
    pub stats: CoopEmpathyStats,
    rng_state: u64,
    tick: u64,
    /// EMA-smoothed cross-understanding score
    cross_understanding_ema: f32,
}

impl CoopEmpathyEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            conflicts: Vec::new(),
            stats: CoopEmpathyStats::new(),
            rng_state: seed | 1,
            tick: 0,
            cross_understanding_ema: 0.0,
        }
    }

    // ========================================================================
    // EMPATHIZE WITH PROCESS
    // ========================================================================

    /// Build or update the empathy profile for a process
    pub fn empathize_with_process(
        &mut self,
        process_id: u64,
        latency_need: f32,
        fairness_need: f32,
        burst_need: f32,
        predictability_need: f32,
        isolation_need: f32,
        priority_need: f32,
    ) {
        self.tick += 1;

        if !self.profiles.contains_key(&process_id) {
            if self.profiles.len() >= MAX_PROCESSES {
                return;
            }
            self.profiles.insert(process_id, EmpathyProfile::new(process_id));
        }

        let tick = self.tick;
        if let Some(profile) = self.profiles.get_mut(&process_id) {
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::LatencyGuarantee as u8)) {
                n.observe_intensity(latency_need);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::FairShare as u8)) {
                n.observe_intensity(fairness_need);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::BurstCapacity as u8)) {
                n.observe_intensity(burst_need);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::PredictableAllocation as u8)) {
                n.observe_intensity(predictability_need);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::NoiseIsolation as u8)) {
                n.observe_intensity(isolation_need);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::CriticalPriority as u8)) {
                n.observe_intensity(priority_need);
            }
            profile.recompute_aggregates();
            profile.evaluation_count += 1;
            profile.last_evaluation_tick = tick;
        }

        self.stats.total_evaluations += 1;
        self.update_global_stats();
    }

    // ========================================================================
    // INFER COOPERATION NEEDS
    // ========================================================================

    /// Infer cooperation needs from behavioral signals
    ///
    /// Takes observed latency sensitivity, allocation variance tolerance,
    /// burst frequency, and contention reactions to infer needs.
    pub fn infer_cooperation_needs(
        &mut self,
        process_id: u64,
        latency_sensitivity: f32,
        variance_tolerance: f32,
        burst_frequency: f32,
        contention_reaction: f32,
    ) -> Vec<(CoopNeedKind, f32)> {
        let mut inferred = Vec::new();

        // High latency sensitivity → needs latency guarantee
        let lat_need = latency_sensitivity;
        inferred.push((CoopNeedKind::LatencyGuarantee, lat_need));

        // Low variance tolerance → needs predictable allocation
        let predict_need = 1.0 - variance_tolerance;
        inferred.push((CoopNeedKind::PredictableAllocation, predict_need));

        // High burst frequency → needs burst capacity
        inferred.push((CoopNeedKind::BurstCapacity, burst_frequency));

        // High contention reaction → needs noise isolation
        inferred.push((CoopNeedKind::NoiseIsolation, contention_reaction));

        // Fair share is always somewhat needed
        let fair_need = 0.3 + contention_reaction * 0.4;
        inferred.push((CoopNeedKind::FairShare, fair_need.min(1.0)));

        // Priority need from combined signals
        let prio_need = (latency_sensitivity * 0.5 + contention_reaction * 0.5).min(1.0);
        inferred.push((CoopNeedKind::CriticalPriority, prio_need));

        // Apply inferred needs
        if let Some(profile) = self.profiles.get_mut(&process_id) {
            for (kind, intensity) in inferred.iter() {
                if let Some(need) = profile.cooperation_needs.get_mut(&(*kind as u8)) {
                    need.observe_intensity(*intensity);
                }
            }
            profile.recompute_aggregates();
        }

        inferred
    }

    // ========================================================================
    // SATISFACTION MODEL
    // ========================================================================

    /// Model how satisfied a process is with current cooperation
    pub fn satisfaction_model(
        &mut self,
        process_id: u64,
        latency_met: f32,
        fairness_met: f32,
        burst_met: f32,
        predictability_met: f32,
        isolation_met: f32,
        priority_met: f32,
    ) -> f32 {
        if let Some(profile) = self.profiles.get_mut(&process_id) {
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::LatencyGuarantee as u8)) {
                n.update_satisfaction(latency_met);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::FairShare as u8)) {
                n.update_satisfaction(fairness_met);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::BurstCapacity as u8)) {
                n.update_satisfaction(burst_met);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::PredictableAllocation as u8)) {
                n.update_satisfaction(predictability_met);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::NoiseIsolation as u8)) {
                n.update_satisfaction(isolation_met);
            }
            if let Some(n) = profile.cooperation_needs.get_mut(&(CoopNeedKind::CriticalPriority as u8)) {
                n.update_satisfaction(priority_met);
            }
            profile.recompute_aggregates();
            return profile.overall_satisfaction;
        }
        0.0
    }

    // ========================================================================
    // NEED CONFLICT DETECTION
    // ========================================================================

    /// Detect conflicting needs between pairs of processes
    pub fn need_conflict_detection(&mut self) -> Vec<NeedConflict> {
        self.tick += 1;
        let mut new_conflicts = Vec::new();
        let process_ids: Vec<u64> = self.profiles.keys().copied().collect();

        for i in 0..process_ids.len() {
            for j in (i + 1)..process_ids.len() {
                let pid_a = process_ids[i];
                let pid_b = process_ids[j];
                let conflicts = self.detect_pair_conflicts(pid_a, pid_b);
                for conflict in conflicts {
                    new_conflicts.push(conflict);
                }
            }
        }

        self.stats.total_conflicts_detected += new_conflicts.len() as u64;
        self.conflicts = new_conflicts.clone();
        self.stats.active_conflicts = self.conflicts.len();
        new_conflicts
    }

    fn detect_pair_conflicts(&self, pid_a: u64, pid_b: u64) -> Vec<NeedConflict> {
        let mut conflicts = Vec::new();
        let profile_a = match self.profiles.get(&pid_a) {
            Some(p) => p,
            None => return conflicts,
        };
        let profile_b = match self.profiles.get(&pid_b) {
            Some(p) => p,
            None => return conflicts,
        };

        for kind in CoopNeedKind::all() {
            let key = *kind as u8;
            let need_a = profile_a.cooperation_needs.get(&key);
            let need_b = profile_b.cooperation_needs.get(&key);
            if let (Some(a), Some(b)) = (need_a, need_b) {
                // Conflict when both have high intensity for the same scarce resource
                if a.smoothed_intensity > CONFLICT_THRESHOLD && b.smoothed_intensity > CONFLICT_THRESHOLD {
                    let severity = (a.smoothed_intensity + b.smoothed_intensity) / 2.0;
                    let mut id_buf = [0u8; 17];
                    let a_bytes = pid_a.to_le_bytes();
                    let b_bytes = pid_b.to_le_bytes();
                    id_buf[..8].copy_from_slice(&a_bytes);
                    id_buf[8..16].copy_from_slice(&b_bytes);
                    id_buf[16] = key;
                    let conflict_id = fnv1a_hash(&id_buf);

                    let mut desc = String::new();
                    desc.push_str("conflict_on_need_");
                    let digit = (key % 10) + b'0';
                    desc.push(digit as char);

                    conflicts.push(NeedConflict {
                        conflict_id,
                        process_a: pid_a,
                        process_b: pid_b,
                        need_kind: *kind,
                        severity,
                        description: desc,
                        detected_tick: self.tick,
                    });
                }
            }
        }

        conflicts
    }

    // ========================================================================
    // EMPATHY SCORE
    // ========================================================================

    /// Compute empathy accuracy for a specific process
    ///
    /// Measures how well our need predictions match actual satisfaction outcomes.
    pub fn empathy_score(&mut self, process_id: u64) -> f32 {
        if let Some(profile) = self.profiles.get_mut(&process_id) {
            let mut accuracy_sum = 0.0f32;
            let mut count = 0usize;

            for (_, need) in profile.cooperation_needs.iter() {
                if need.observations > 0 {
                    // Accuracy = 1 - gap (smaller gap = better empathy)
                    let acc = 1.0 - need.gap;
                    accuracy_sum += acc;
                    count += 1;
                }
            }

            if count > 0 {
                let raw_accuracy = accuracy_sum / count as f32;
                profile.empathy_accuracy += EMA_ALPHA * (raw_accuracy - profile.empathy_accuracy);
                return profile.empathy_accuracy;
            }
        }
        0.0
    }

    // ========================================================================
    // CROSS-PROCESS UNDERSTANDING
    // ========================================================================

    /// How well do all processes understand each other's needs?
    ///
    /// Measures global empathy: low conflicts + high satisfaction = high understanding.
    pub fn cross_process_understanding(&mut self) -> f32 {
        let proc_count = self.profiles.len();
        if proc_count == 0 {
            return 0.0;
        }

        let mut total_sat = 0.0f32;
        let mut total_accuracy = 0.0f32;
        for (_, profile) in self.profiles.iter() {
            total_sat += profile.overall_satisfaction;
            total_accuracy += profile.empathy_accuracy;
        }

        let avg_sat = total_sat / proc_count as f32;
        let avg_acc = total_accuracy / proc_count as f32;
        let conflict_penalty = if proc_count > 1 {
            (self.conflicts.len() as f32 / (proc_count * (proc_count - 1) / 2) as f32).min(1.0)
        } else {
            0.0
        };

        let raw = avg_sat * 0.4 + avg_acc * 0.4 + (1.0 - conflict_penalty) * 0.2;
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };

        self.cross_understanding_ema += EMA_ALPHA * (clamped - self.cross_understanding_ema);
        self.stats.cross_understanding_score = self.cross_understanding_ema;
        self.cross_understanding_ema
    }

    // ========================================================================
    // MAINTENANCE
    // ========================================================================

    /// Decay all needs over time
    pub fn decay_all(&mut self) {
        for (_, profile) in self.profiles.iter_mut() {
            for (_, need) in profile.cooperation_needs.iter_mut() {
                need.decay();
            }
        }
    }

    /// Prune stale profiles
    pub fn prune_stale(&mut self, max_age: u64) {
        let cutoff = if self.tick > max_age { self.tick - max_age } else { 0 };
        let stale: Vec<u64> = self.profiles.iter()
            .filter(|(_, p)| p.last_evaluation_tick < cutoff)
            .map(|(k, _)| *k)
            .collect();
        for key in stale {
            self.profiles.remove(&key);
        }
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    pub fn profile(&self, process_id: u64) -> Option<&EmpathyProfile> {
        self.profiles.get(&process_id)
    }

    pub fn process_count(&self) -> usize {
        self.profiles.len()
    }

    pub fn snapshot_stats(&self) -> CoopEmpathyStats {
        self.stats.clone()
    }

    fn update_global_stats(&mut self) {
        let count = self.profiles.len();
        self.stats.tracked_processes = count;
        if count == 0 {
            return;
        }
        let mut total_sat = 0.0f32;
        let mut total_acc = 0.0f32;
        let mut total_gap = 0.0f32;
        let mut worst_sat = f32::MAX;
        let mut worst_pid = 0u64;

        for (pid, profile) in self.profiles.iter() {
            total_sat += profile.overall_satisfaction;
            total_acc += profile.empathy_accuracy;
            total_gap += profile.total_gap;
            if profile.overall_satisfaction < worst_sat {
                worst_sat = profile.overall_satisfaction;
                worst_pid = *pid;
            }
        }
        let inv = 1.0 / count as f32;
        self.stats.avg_satisfaction = total_sat * inv;
        self.stats.avg_empathy_accuracy = total_acc * inv;
        self.stats.avg_need_gap = total_gap * inv;
        self.stats.worst_satisfaction_process = worst_pid;
    }
}
