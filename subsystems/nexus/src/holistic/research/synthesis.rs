// SPDX-License-Identifier: GPL-2.0
//! # Holistic Synthesis — System-Wide Optimization Synthesis
//!
//! Coordinates the integration of validated discoveries across every NEXUS
//! subsystem. Where individual research engines produce isolated
//! improvements, the synthesis engine ensures those improvements compose
//! coherently: a scheduler optimisation must not conflict with a memory
//! tiering change, and an IPC throughput gain must not regress trust
//! verification latency.
//!
//! The engine builds a coordination plan, detects and resolves conflicts
//! between pending improvements, stages rollouts in dependency order, and
//! computes a unified system improvement score that summarises the net
//! benefit of each synthesis wave.
//!
//! The engine that turns many small wins into one big, coherent advance.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PENDING: usize = 256;
const MAX_PLAN_STAGES: usize = 16;
const MAX_CONFLICTS: usize = 128;
const CONFLICT_THRESHOLD: f32 = 0.20;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const ROLLOUT_BATCH: usize = 8;
const MIN_IMPROVEMENT_SCORE: f32 = 0.05;
const DECAY_FACTOR: f32 = 0.95;
const INTEGRATION_BONUS: f32 = 1.15;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// TYPES
// ============================================================================

/// Subsystem affected by a discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AffectedSubsystem {
    Scheduler,
    Memory,
    Ipc,
    FileSystem,
    Networking,
    Trust,
    Energy,
    Bridge,
    Application,
    Cooperation,
}

/// Status of a pending improvement in the synthesis pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SynthesisStatus {
    Pending,
    Planned,
    Staged,
    RollingOut,
    Applied,
    Reverted,
    Conflicted,
}

/// A validated improvement awaiting synthesis
#[derive(Debug, Clone)]
pub struct PendingImprovement {
    pub id: u64,
    pub description: String,
    pub affected: Vec<AffectedSubsystem>,
    pub expected_gain: f32,
    pub confidence: f32,
    pub cert_hash: u64,
    pub status: SynthesisStatus,
    pub dependencies: Vec<u64>,
    pub submitted_tick: u64,
}

/// A conflict between two pending improvements
#[derive(Debug, Clone)]
pub struct SynthesisConflict {
    pub id: u64,
    pub improvement_a: u64,
    pub improvement_b: u64,
    pub conflict_subsystem: AffectedSubsystem,
    pub severity: f32,
    pub resolution: ConflictResolution,
}

/// How a conflict was resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictResolution {
    Unresolved,
    PreferA,
    PreferB,
    Merged,
    DeferBoth,
}

/// A stage in the coordination plan
#[derive(Debug, Clone)]
pub struct PlanStage {
    pub stage_number: usize,
    pub improvement_ids: Vec<u64>,
    pub subsystems_touched: Vec<AffectedSubsystem>,
    pub expected_gain: f32,
    pub risk_score: f32,
}

/// Coordination plan for staged rollout
#[derive(Debug, Clone)]
pub struct CoordinationPlan {
    pub id: u64,
    pub stages: Vec<PlanStage>,
    pub total_improvements: usize,
    pub total_expected_gain: f32,
    pub created_tick: u64,
}

/// Impact report from a synthesis wave
#[derive(Debug, Clone)]
pub struct SynthesisImpact {
    pub wave_id: u64,
    pub improvements_applied: usize,
    pub improvements_reverted: usize,
    pub net_gain: f32,
    pub subsystems_improved: Vec<AffectedSubsystem>,
    pub tick: u64,
}

/// Synthesis engine statistics
#[derive(Debug, Clone)]
pub struct SynthesisStats {
    pub total_pending: u64,
    pub total_applied: u64,
    pub total_reverted: u64,
    pub conflicts_detected: u64,
    pub conflicts_resolved: u64,
    pub waves_completed: u64,
    pub cumulative_gain: f32,
    pub avg_gain_ema: f32,
    pub system_improvement_score: f32,
}

// ============================================================================
// HOLISTIC SYNTHESIS ENGINE
// ============================================================================

/// System-wide optimization synthesis and coordination engine
pub struct HolisticSynthesis {
    pending: BTreeMap<u64, PendingImprovement>,
    conflicts: Vec<SynthesisConflict>,
    plans: Vec<CoordinationPlan>,
    impact_history: Vec<SynthesisImpact>,
    applied_log: Vec<(u64, f32)>,
    rng_state: u64,
    stats: SynthesisStats,
}

impl HolisticSynthesis {
    /// Create a new synthesis engine
    pub fn new(seed: u64) -> Self {
        Self {
            pending: BTreeMap::new(),
            conflicts: Vec::new(),
            plans: Vec::new(),
            impact_history: Vec::new(),
            applied_log: Vec::new(),
            rng_state: seed | 1,
            stats: SynthesisStats {
                total_pending: 0, total_applied: 0, total_reverted: 0,
                conflicts_detected: 0, conflicts_resolved: 0,
                waves_completed: 0, cumulative_gain: 0.0,
                avg_gain_ema: 0.0, system_improvement_score: 0.0,
            },
        }
    }

    /// Submit a validated improvement for synthesis
    pub fn submit_improvement(&mut self, imp: PendingImprovement) {
        if self.pending.len() < MAX_PENDING {
            self.pending.insert(imp.id, imp);
            self.stats.total_pending = self.pending.len() as u64;
        }
    }

    /// Run full global synthesis: detect conflicts, plan, stage, apply
    pub fn global_synthesis(&mut self, tick: u64) -> SynthesisImpact {
        self.conflict_resolution();
        let plan = self.coordination_plan(tick);
        let impact = self.staged_rollout(plan, tick);
        self.stats.waves_completed += 1;
        self.stats.cumulative_gain += impact.net_gain;
        self.stats.avg_gain_ema =
            EMA_ALPHA * impact.net_gain + (1.0 - EMA_ALPHA) * self.stats.avg_gain_ema;
        self.stats.system_improvement_score = self.system_improvement_score();
        self.impact_history.push(impact.clone());
        impact
    }

    /// Build a staged coordination plan from pending improvements
    pub fn coordination_plan(&mut self, tick: u64) -> CoordinationPlan {
        let mut ready: Vec<&PendingImprovement> = self.pending.values()
            .filter(|i| i.status == SynthesisStatus::Pending
                || i.status == SynthesisStatus::Planned)
            .collect();
        ready.sort_by(|a, b|
            b.expected_gain.partial_cmp(&a.expected_gain)
                .unwrap_or(core::cmp::Ordering::Equal));
        let mut stages = Vec::new();
        let mut assigned: Vec<u64> = Vec::new();
        let mut stage_num = 0;
        while !ready.is_empty() && stage_num < MAX_PLAN_STAGES {
            let mut stage_ids = Vec::new();
            let mut stage_subs: Vec<AffectedSubsystem> = Vec::new();
            let mut stage_gain = 0.0f32;
            let mut next_ready = Vec::new();
            for imp in &ready {
                if assigned.contains(&imp.id) { continue; }
                let conflicts_with_stage = imp.affected.iter()
                    .any(|s| stage_subs.contains(s));
                if !conflicts_with_stage && stage_ids.len() < ROLLOUT_BATCH {
                    stage_ids.push(imp.id);
                    for s in &imp.affected {
                        if !stage_subs.contains(s) { stage_subs.push(*s); }
                    }
                    stage_gain += imp.expected_gain;
                    assigned.push(imp.id);
                } else {
                    next_ready.push(*imp);
                }
            }
            if stage_ids.is_empty() { break; }
            let risk = 1.0 - (stage_gain / (stage_ids.len() as f32 + 1.0)).min(1.0);
            stages.push(PlanStage {
                stage_number: stage_num, improvement_ids: stage_ids,
                subsystems_touched: stage_subs,
                expected_gain: stage_gain, risk_score: risk,
            });
            ready = next_ready;
            stage_num += 1;
        }
        let total_gain: f32 = stages.iter().map(|s| s.expected_gain).sum();
        let total_imps: usize = stages.iter().map(|s| s.improvement_ids.len()).sum();
        let plan = CoordinationPlan {
            id: fnv1a_hash(&tick.to_le_bytes()),
            stages, total_improvements: total_imps,
            total_expected_gain: total_gain, created_tick: tick,
        };
        self.plans.push(plan.clone());
        plan
    }

    /// Detect and resolve conflicts between pending improvements
    pub fn conflict_resolution(&mut self) {
        self.conflicts.clear();
        let ids: Vec<u64> = self.pending.keys().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                if self.conflicts.len() >= MAX_CONFLICTS { break; }
                let a = &self.pending[&ids[i]];
                let b = &self.pending[&ids[j]];
                for sub_a in &a.affected {
                    if b.affected.contains(sub_a) {
                        let severity = (a.expected_gain - b.expected_gain).abs();
                        if severity > CONFLICT_THRESHOLD {
                            let resolution = if a.expected_gain > b.expected_gain {
                                ConflictResolution::PreferA
                            } else {
                                ConflictResolution::PreferB
                            };
                            let cid = fnv1a_hash(&ids[i].to_le_bytes())
                                ^ fnv1a_hash(&ids[j].to_le_bytes());
                            self.conflicts.push(SynthesisConflict {
                                id: cid, improvement_a: ids[i], improvement_b: ids[j],
                                conflict_subsystem: *sub_a, severity, resolution,
                            });
                            self.stats.conflicts_detected += 1;
                            self.stats.conflicts_resolved += 1;
                        }
                    }
                }
            }
        }
        for conflict in &self.conflicts {
            match conflict.resolution {
                ConflictResolution::PreferA => {
                    if let Some(b) = self.pending.get_mut(&conflict.improvement_b) {
                        b.status = SynthesisStatus::Conflicted;
                    }
                }
                ConflictResolution::PreferB => {
                    if let Some(a) = self.pending.get_mut(&conflict.improvement_a) {
                        a.status = SynthesisStatus::Conflicted;
                    }
                }
                ConflictResolution::DeferBoth => {
                    if let Some(a) = self.pending.get_mut(&conflict.improvement_a) {
                        a.status = SynthesisStatus::Conflicted;
                    }
                    if let Some(b) = self.pending.get_mut(&conflict.improvement_b) {
                        b.status = SynthesisStatus::Conflicted;
                    }
                }
                _ => {}
            }
        }
    }

    /// Execute a staged rollout from a coordination plan
    pub fn staged_rollout(&mut self, plan: CoordinationPlan, tick: u64) -> SynthesisImpact {
        let mut applied = 0usize;
        let mut reverted = 0usize;
        let mut net_gain = 0.0f32;
        let mut subs_improved: Vec<AffectedSubsystem> = Vec::new();
        for stage in &plan.stages {
            for &imp_id in &stage.improvement_ids {
                if let Some(imp) = self.pending.get_mut(&imp_id) {
                    if imp.status == SynthesisStatus::Conflicted { continue; }
                    let success = xorshift_f32(&mut self.rng_state) < imp.confidence;
                    if success {
                        imp.status = SynthesisStatus::Applied;
                        net_gain += imp.expected_gain;
                        for s in &imp.affected {
                            if !subs_improved.contains(s) { subs_improved.push(*s); }
                        }
                        applied += 1;
                        self.applied_log.push((imp_id, imp.expected_gain));
                        self.stats.total_applied += 1;
                    } else {
                        imp.status = SynthesisStatus::Reverted;
                        reverted += 1;
                        self.stats.total_reverted += 1;
                    }
                }
            }
        }
        let wave_id = fnv1a_hash(&tick.to_le_bytes());
        self.cleanup_applied();
        SynthesisImpact {
            wave_id, improvements_applied: applied,
            improvements_reverted: reverted,
            net_gain, subsystems_improved: subs_improved, tick,
        }
    }

    /// Compute overall synthesis impact from recent history
    pub fn synthesis_impact(&self) -> f32 {
        let mut weighted = 0.0f32;
        let mut weight = 1.0f32;
        for impact in self.impact_history.iter().rev().take(16) {
            weighted += impact.net_gain * weight;
            weight *= DECAY_FACTOR;
        }
        weighted
    }

    /// Compute the unified system improvement score
    pub fn system_improvement_score(&self) -> f32 {
        let base = self.stats.cumulative_gain;
        let applied = self.stats.total_applied as f32;
        let reverted = self.stats.total_reverted as f32;
        let success_rate = if applied + reverted > 0.0 {
            applied / (applied + reverted)
        } else { 0.0 };
        let integration = if applied > 1.0 { INTEGRATION_BONUS } else { 1.0 };
        base * success_rate * integration
    }

    /// Current statistics snapshot
    pub fn stats(&self) -> &SynthesisStats { &self.stats }

    // ── private helpers ─────────────────────────────────────────────────

    fn cleanup_applied(&mut self) {
        let done: Vec<u64> = self.pending.iter()
            .filter(|(_, i)| i.status == SynthesisStatus::Applied
                || i.status == SynthesisStatus::Reverted)
            .map(|(&id, _)| id).collect();
        for id in done {
            self.pending.remove(&id);
        }
        self.stats.total_pending = self.pending.len() as u64;
    }
}
