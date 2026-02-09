// SPDX-License-Identifier: GPL-2.0
//! # Holistic Timeline Projector
//!
//! System-wide timeline projection engine. Projects the complete system state
//! forward in time, maintaining a "predicted future" that is continuously
//! validated against actual observations. Supports timeline branching,
//! merging, and course correction — enabling the kernel to *live in the
//! future* and course-correct when reality diverges from expectations.
//!
//! This is the temporal backbone of the NEXUS prediction framework.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_TIMELINE_POINTS: usize = 512;
const MAX_BRANCHES: usize = 32;
const MAX_CORRECTIONS: usize = 128;
const MAX_ACCURACY_SAMPLES: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const DIVERGENCE_THRESHOLD: f32 = 0.15;
const CORRECTION_STRENGTH: f32 = 0.25;
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
// TIMELINE STATUS
// ============================================================================

/// Status of a timeline branch
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimelineStatus {
    Active,
    Merged,
    Invalidated,
    Archived,
}

/// Reason for a course correction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrectionReason {
    CpuDivergence,
    MemoryDivergence,
    IoDivergence,
    NetworkDivergence,
    ProcessDivergence,
    ThermalDivergence,
    ExternalShock,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// A single point in the projected timeline
#[derive(Debug, Clone)]
pub struct TimelinePoint {
    pub tick: u64,
    pub cpu_projected: f32,
    pub mem_projected: f32,
    pub io_projected: f32,
    pub net_projected: f32,
    pub process_count: u32,
    pub confidence: f32,
    pub validated: bool,
}

/// A timeline branch — an alternative future
#[derive(Debug, Clone)]
pub struct TimelineBranch {
    pub id: u64,
    pub parent_id: u64,
    pub branch_tick: u64,
    pub status: TimelineStatus,
    pub points: Vec<TimelinePoint>,
    pub description: String,
    pub probability: f32,
}

/// Comparison between predicted and actual system state
#[derive(Debug, Clone)]
pub struct PredictedVsActual {
    pub tick: u64,
    pub predicted_cpu: f32,
    pub actual_cpu: f32,
    pub predicted_mem: f32,
    pub actual_mem: f32,
    pub predicted_io: f32,
    pub actual_io: f32,
    pub error_magnitude: f32,
}

/// A course correction applied to the timeline
#[derive(Debug, Clone)]
pub struct CourseCorrection {
    pub id: u64,
    pub tick: u64,
    pub reason: CorrectionReason,
    pub error_before: f32,
    pub error_after: f32,
    pub adjustment_magnitude: f32,
    pub affected_points: u32,
}

/// Accuracy measurement for the timeline projection
#[derive(Debug, Clone)]
pub struct AccuracySample {
    pub tick: u64,
    pub horizon_ticks: u64,
    pub accuracy: f32,
    pub dimension: String,
}

/// Result of merging two timeline branches
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub branch_a: u64,
    pub branch_b: u64,
    pub merged_points: usize,
    pub conflict_count: u32,
    pub result_confidence: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate timeline projection statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct TimelineStats {
    pub total_projections: u64,
    pub total_branches: u64,
    pub total_merges: u64,
    pub total_corrections: u64,
    pub avg_accuracy: f32,
    pub avg_confidence: f32,
    pub active_branches: usize,
    pub correction_rate: f32,
}

// ============================================================================
// HOLISTIC TIMELINE
// ============================================================================

/// System-wide timeline projection engine. Maintains a predicted future that
/// is continuously validated and corrected against reality.
#[derive(Debug)]
pub struct HolisticTimeline {
    main_timeline: Vec<TimelinePoint>,
    branches: BTreeMap<u64, TimelineBranch>,
    corrections: BTreeMap<u64, CourseCorrection>,
    accuracy_samples: Vec<AccuracySample>,
    comparison_log: Vec<PredictedVsActual>,
    total_projections: u64,
    total_branches: u64,
    total_merges: u64,
    total_corrections: u64,
    tick: u64,
    rng_state: u64,
    accuracy_ema: f32,
    confidence_ema: f32,
}

impl HolisticTimeline {
    pub fn new() -> Self {
        Self {
            main_timeline: Vec::new(),
            branches: BTreeMap::new(),
            corrections: BTreeMap::new(),
            accuracy_samples: Vec::new(),
            comparison_log: Vec::new(),
            total_projections: 0,
            total_branches: 0,
            total_merges: 0,
            total_corrections: 0,
            tick: 0,
            rng_state: 0xD1BE_11BE_CA0F_3C70,
            accuracy_ema: 0.5,
            confidence_ema: 0.5,
        }
    }

    /// Project the complete system state forward by a number of ticks
    pub fn project_system(
        &mut self,
        current_cpu: f32,
        current_mem: f32,
        current_io: f32,
        current_net: f32,
        process_count: u32,
        horizon_ticks: u64,
    ) -> Vec<TimelinePoint> {
        self.tick += 1;
        self.total_projections += 1;

        let mut points = Vec::new();
        let mut cpu = current_cpu;
        let mut mem = current_mem;
        let mut io = current_io;
        let mut net = current_net;
        let mut procs = process_count;

        let capped = (horizon_ticks as usize).min(MAX_TIMELINE_POINTS);
        for t in 0..capped as u64 {
            let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 2000.0 - 0.025;
            let decay = 1.0 / (1.0 + t as f32 * 0.01);

            cpu = (cpu + noise * 10.0 * decay).clamp(0.0, 100.0);
            mem = (mem + noise * 0.02 * decay).clamp(0.0, 1.0);
            io = (io + noise * 5.0 * decay).clamp(0.0, 100.0);
            net = (net + noise * 5.0 * decay).clamp(0.0, 100.0);

            if mem > 0.9 {
                procs = procs.saturating_sub(1);
            }

            let conf = (1.0 / (1.0 + t as f32 * 0.05)).clamp(0.01, 1.0);

            points.push(TimelinePoint {
                tick: self.tick + t,
                cpu_projected: cpu,
                mem_projected: mem,
                io_projected: io,
                net_projected: net,
                process_count: procs,
                confidence: conf,
                validated: false,
            });
        }

        self.confidence_ema = if let Some(last) = points.last() {
            EMA_ALPHA * last.confidence + (1.0 - EMA_ALPHA) * self.confidence_ema
        } else {
            self.confidence_ema
        };

        for p in &points {
            self.main_timeline.push(p.clone());
        }
        while self.main_timeline.len() > MAX_TIMELINE_POINTS {
            self.main_timeline.remove(0);
        }

        points
    }

    /// Create a branch — an alternative future scenario
    pub fn timeline_branch(
        &mut self,
        parent_id: u64,
        description: String,
        probability: f32,
        adjustment_cpu: f32,
        adjustment_mem: f32,
    ) -> TimelineBranch {
        self.tick += 1;
        self.total_branches += 1;

        let mut branch_points = Vec::new();
        for point in self.main_timeline.iter().filter(|p| p.tick >= self.tick) {
            let mut bp = point.clone();
            bp.cpu_projected = (bp.cpu_projected + adjustment_cpu).clamp(0.0, 100.0);
            bp.mem_projected = (bp.mem_projected + adjustment_mem).clamp(0.0, 1.0);
            bp.confidence *= probability.clamp(0.01, 1.0);
            branch_points.push(bp);
        }

        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let branch = TimelineBranch {
            id,
            parent_id,
            branch_tick: self.tick,
            status: TimelineStatus::Active,
            points: branch_points,
            description,
            probability: probability.clamp(0.0, 1.0),
        };

        self.branches.insert(id, branch.clone());
        if self.branches.len() > MAX_BRANCHES {
            let mut to_remove = None;
            for (&bid, b) in &self.branches {
                if b.status == TimelineStatus::Archived || b.status == TimelineStatus::Invalidated {
                    to_remove = Some(bid);
                    break;
                }
            }
            if let Some(bid) = to_remove {
                self.branches.remove(&bid);
            } else if let Some((&oldest, _)) = self.branches.iter().next() {
                self.branches.remove(&oldest);
            }
        }

        branch
    }

    /// Merge two timeline branches into a weighted combination
    pub fn merge_predictions(&mut self, branch_a_id: u64, branch_b_id: u64) -> MergeResult {
        self.total_merges += 1;

        let points_a = self.branches.get(&branch_a_id).map(|b| b.points.clone());
        let points_b = self.branches.get(&branch_b_id).map(|b| b.points.clone());
        let prob_a = self.branches.get(&branch_a_id).map(|b| b.probability).unwrap_or(0.5);
        let prob_b = self.branches.get(&branch_b_id).map(|b| b.probability).unwrap_or(0.5);

        let mut merged_count = 0;
        let mut conflicts = 0_u32;
        let total_prob = prob_a + prob_b;
        let wa = if total_prob > 0.0 { prob_a / total_prob } else { 0.5 };
        let wb = 1.0 - wa;

        if let (Some(pa), Some(pb)) = (points_a, points_b) {
            let len = pa.len().min(pb.len());
            for i in 0..len {
                let cpu_diff = (pa[i].cpu_projected - pb[i].cpu_projected).abs();
                if cpu_diff > DIVERGENCE_THRESHOLD * 100.0 {
                    conflicts += 1;
                }
                merged_count += 1;
            }

            // Apply merged values back to main timeline
            for i in 0..len.min(self.main_timeline.len()) {
                let idx = self.main_timeline.len().saturating_sub(len) + i;
                if idx < self.main_timeline.len() && i < pa.len() && i < pb.len() {
                    self.main_timeline[idx].cpu_projected =
                        pa[i].cpu_projected * wa + pb[i].cpu_projected * wb;
                    self.main_timeline[idx].mem_projected =
                        pa[i].mem_projected * wa + pb[i].mem_projected * wb;
                }
            }
        }

        if let Some(ba) = self.branches.get_mut(&branch_a_id) {
            ba.status = TimelineStatus::Merged;
        }
        if let Some(bb) = self.branches.get_mut(&branch_b_id) {
            bb.status = TimelineStatus::Merged;
        }

        let result_conf = (wa * prob_a + wb * prob_b).clamp(0.0, 1.0);

        MergeResult {
            branch_a: branch_a_id,
            branch_b: branch_b_id,
            merged_points: merged_count,
            conflict_count: conflicts,
            result_confidence: result_conf,
        }
    }

    /// Measure overall timeline accuracy
    pub fn timeline_accuracy(&self) -> f32 {
        if self.accuracy_samples.is_empty() {
            return self.accuracy_ema;
        }
        let sum: f32 = self.accuracy_samples.iter().map(|s| s.accuracy).sum();
        sum / self.accuracy_samples.len() as f32
    }

    /// Compare predicted vs actual and log the comparison
    pub fn predicted_vs_actual(
        &mut self,
        actual_cpu: f32,
        actual_mem: f32,
        actual_io: f32,
    ) -> PredictedVsActual {
        self.tick += 1;

        let predicted = self
            .main_timeline
            .iter()
            .min_by_key(|p| {
                let diff = if p.tick >= self.tick {
                    p.tick - self.tick
                } else {
                    self.tick - p.tick
                };
                diff
            })
            .cloned();

        let (pred_cpu, pred_mem, pred_io) = predicted
            .map(|p| (p.cpu_projected, p.mem_projected, p.io_projected))
            .unwrap_or((50.0, 0.5, 50.0));

        let cpu_err = (pred_cpu - actual_cpu).abs() / 100.0;
        let mem_err = (pred_mem - actual_mem).abs();
        let io_err = (pred_io - actual_io).abs() / 100.0;
        let error_mag = (cpu_err + mem_err + io_err) / 3.0;

        let accuracy = 1.0 - error_mag.clamp(0.0, 1.0);
        self.accuracy_ema = EMA_ALPHA * accuracy + (1.0 - EMA_ALPHA) * self.accuracy_ema;

        self.accuracy_samples.push(AccuracySample {
            tick: self.tick,
            horizon_ticks: 1,
            accuracy,
            dimension: String::from("composite"),
        });
        while self.accuracy_samples.len() > MAX_ACCURACY_SAMPLES {
            self.accuracy_samples.remove(0);
        }

        let comparison = PredictedVsActual {
            tick: self.tick,
            predicted_cpu: pred_cpu,
            actual_cpu,
            predicted_mem: pred_mem,
            actual_mem,
            predicted_io: pred_io,
            actual_io,
            error_magnitude: error_mag,
        };

        self.comparison_log.push(comparison.clone());
        while self.comparison_log.len() > MAX_ACCURACY_SAMPLES {
            self.comparison_log.remove(0);
        }

        comparison
    }

    /// Apply course corrections when reality diverges from predictions
    pub fn course_correction(&mut self) -> Vec<CourseCorrection> {
        let mut corrections = Vec::new();

        for cmp in &self.comparison_log {
            if cmp.error_magnitude < DIVERGENCE_THRESHOLD {
                continue;
            }

            let reason = if (cmp.predicted_cpu - cmp.actual_cpu).abs() / 100.0 > DIVERGENCE_THRESHOLD {
                CorrectionReason::CpuDivergence
            } else if (cmp.predicted_mem - cmp.actual_mem).abs() > DIVERGENCE_THRESHOLD {
                CorrectionReason::MemoryDivergence
            } else {
                CorrectionReason::IoDivergence
            };

            let id = fnv1a_hash(format!("corr-{}-{:?}", cmp.tick, reason).as_bytes());

            if self.corrections.contains_key(&id) {
                continue;
            }

            let adj = cmp.error_magnitude * CORRECTION_STRENGTH;
            let mut affected = 0_u32;

            for point in &mut self.main_timeline {
                if point.tick >= cmp.tick && !point.validated {
                    match reason {
                        CorrectionReason::CpuDivergence => {
                            let dir = if cmp.actual_cpu > cmp.predicted_cpu { 1.0 } else { -1.0 };
                            point.cpu_projected =
                                (point.cpu_projected + dir * adj * 100.0).clamp(0.0, 100.0);
                        }
                        CorrectionReason::MemoryDivergence => {
                            let dir = if cmp.actual_mem > cmp.predicted_mem { 1.0 } else { -1.0 };
                            point.mem_projected =
                                (point.mem_projected + dir * adj).clamp(0.0, 1.0);
                        }
                        _ => {
                            point.io_projected =
                                (point.io_projected + adj * 50.0).clamp(0.0, 100.0);
                        }
                    }
                    affected += 1;
                }
            }

            let correction = CourseCorrection {
                id,
                tick: cmp.tick,
                reason,
                error_before: cmp.error_magnitude,
                error_after: (cmp.error_magnitude - adj).clamp(0.0, 1.0),
                adjustment_magnitude: adj,
                affected_points: affected,
            };

            self.corrections.insert(id, correction.clone());
            self.total_corrections += 1;
            corrections.push(correction);
        }

        while self.corrections.len() > MAX_CORRECTIONS {
            if let Some((&oldest, _)) = self.corrections.iter().next() {
                self.corrections.remove(&oldest);
            }
        }

        corrections
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> TimelineStats {
        let active = self
            .branches
            .values()
            .filter(|b| b.status == TimelineStatus::Active)
            .count();

        let correction_rate = if self.total_projections > 0 {
            self.total_corrections as f32 / self.total_projections as f32
        } else {
            0.0
        };

        TimelineStats {
            total_projections: self.total_projections,
            total_branches: self.total_branches,
            total_merges: self.total_merges,
            total_corrections: self.total_corrections,
            avg_accuracy: self.accuracy_ema,
            avg_confidence: self.confidence_ema,
            active_branches: active,
            correction_rate,
        }
    }
}
