// SPDX-License-Identifier: GPL-2.0
//! # Holistic Self-Model
//!
//! The COMPLETE kernel self-model. Unifies bridge, application, and cooperative
//! self-models into a single coherent self-image. Tracks all capabilities across
//! every subsystem, known limitations, the performance envelope, and the
//! improvement trajectory over time.
//!
//! This is the kernel looking in the mirror and seeing its WHOLE self — not
//! just one subsystem, but the unified organism that is NEXUS.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAPABILITIES: usize = 512;
const MAX_LIMITATIONS: usize = 256;
const MAX_SUBSYSTEMS: usize = 32;
const MAX_HISTORY: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const CONSISTENCY_THRESHOLD: f32 = 0.15;
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
// SUBSYSTEM DOMAIN
// ============================================================================

/// Which subsystem a capability or limitation belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemDomain {
    Bridge,
    Application,
    Cooperative,
    Memory,
    Scheduler,
    Filesystem,
    Network,
    Security,
    Hardware,
    Holistic,
}

/// Maturity level across the entire kernel
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnifiedMaturity {
    Nascent = 0,
    Developing = 1,
    Functional = 2,
    Mature = 3,
    Mastered = 4,
}

// ============================================================================
// CAPABILITY & LIMITATION
// ============================================================================

/// A unified capability spanning subsystem boundaries
#[derive(Debug, Clone)]
pub struct UnifiedCapability {
    pub name: String,
    pub id: u64,
    pub domain: SubsystemDomain,
    pub maturity: UnifiedMaturity,
    pub performance: f32,
    pub reliability: f32,
    pub cross_subsystem_impact: f32,
    pub tick_introduced: u64,
    pub tick_updated: u64,
    pub update_count: u64,
}

/// A known limitation of the kernel
#[derive(Debug, Clone)]
pub struct KnownLimitation {
    pub name: String,
    pub id: u64,
    pub domain: SubsystemDomain,
    pub severity: f32,
    pub workaround_available: bool,
    pub improvement_potential: f32,
    pub tick_discovered: u64,
}

/// Performance envelope boundary
#[derive(Debug, Clone, Copy)]
pub struct EnvelopeBound {
    pub metric_id: u64,
    pub lower: f32,
    pub upper: f32,
    pub current: f32,
    pub headroom: f32,
}

/// Improvement trajectory sample
#[derive(Debug, Clone, Copy)]
pub struct TrajectoryPoint {
    pub tick: u64,
    pub overall_score: f32,
    pub capability_count: u32,
    pub limitation_count: u32,
    pub maturity_avg: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate self-model statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfModelStats {
    pub total_capabilities: usize,
    pub total_limitations: usize,
    pub avg_maturity: f32,
    pub avg_performance: f32,
    pub avg_reliability: f32,
    pub overall_self_score: f32,
    pub consistency_score: f32,
    pub envelope_headroom: f32,
    pub improvement_velocity: f32,
}

// ============================================================================
// HOLISTIC SELF-MODEL
// ============================================================================

/// The complete, unified self-model for the entire NEXUS kernel.
/// Integrates capabilities and limitations from bridge, apps, and coop
/// into one coherent self-image with performance envelope tracking.
#[derive(Debug)]
pub struct HolisticSelfModel {
    capabilities: BTreeMap<u64, UnifiedCapability>,
    limitations: BTreeMap<u64, KnownLimitation>,
    envelope: BTreeMap<u64, EnvelopeBound>,
    trajectory: Vec<TrajectoryPoint>,
    subsystem_scores: BTreeMap<u8, f32>,
    tick: u64,
    rng_state: u64,
    overall_maturity_ema: f32,
    overall_performance_ema: f32,
    overall_reliability_ema: f32,
    consistency_ema: f32,
}

impl HolisticSelfModel {
    pub fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            limitations: BTreeMap::new(),
            envelope: BTreeMap::new(),
            trajectory: Vec::new(),
            subsystem_scores: BTreeMap::new(),
            tick: 0,
            rng_state: 0xDEAD_BEEF_CAFE_1234,
            overall_maturity_ema: 0.0,
            overall_performance_ema: 0.5,
            overall_reliability_ema: 0.5,
            consistency_ema: 1.0,
        }
    }

    /// Register a unified capability from any subsystem
    pub fn register_capability(
        &mut self,
        name: String,
        domain: SubsystemDomain,
        maturity: UnifiedMaturity,
        performance: f32,
        reliability: f32,
        cross_impact: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.capabilities.len() >= MAX_CAPABILITIES {
            return id;
        }

        let cap = UnifiedCapability {
            name,
            id,
            domain,
            maturity,
            performance: performance.clamp(0.0, 1.0),
            reliability: reliability.clamp(0.0, 1.0),
            cross_subsystem_impact: cross_impact.clamp(0.0, 1.0),
            tick_introduced: self.tick,
            tick_updated: self.tick,
            update_count: 1,
        };

        self.overall_performance_ema =
            EMA_ALPHA * cap.performance + (1.0 - EMA_ALPHA) * self.overall_performance_ema;
        self.overall_reliability_ema =
            EMA_ALPHA * cap.reliability + (1.0 - EMA_ALPHA) * self.overall_reliability_ema;

        let mat_val = cap.maturity as u8 as f32 / 4.0;
        self.overall_maturity_ema =
            EMA_ALPHA * mat_val + (1.0 - EMA_ALPHA) * self.overall_maturity_ema;

        let domain_key = cap.domain as u8;
        let entry = self.subsystem_scores.entry(domain_key).or_insert(0.5);
        *entry = EMA_ALPHA * cap.performance + (1.0 - EMA_ALPHA) * *entry;

        self.capabilities.insert(id, cap);
        id
    }

    /// Register a known limitation
    pub fn register_limitation(
        &mut self,
        name: String,
        domain: SubsystemDomain,
        severity: f32,
        workaround: bool,
        improvement_potential: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.limitations.len() >= MAX_LIMITATIONS {
            return id;
        }

        let lim = KnownLimitation {
            name,
            id,
            domain,
            severity: severity.clamp(0.0, 1.0),
            workaround_available: workaround,
            improvement_potential: improvement_potential.clamp(0.0, 1.0),
            tick_discovered: self.tick,
        };
        self.limitations.insert(id, lim);
        id
    }

    /// Record a performance envelope boundary for a metric
    pub fn record_envelope(&mut self, metric_name: &str, lower: f32, upper: f32, current: f32) {
        let metric_id = fnv1a_hash(metric_name.as_bytes());
        let headroom = if upper > lower {
            ((current - lower) / (upper - lower)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let bound = EnvelopeBound {
            metric_id,
            lower,
            upper,
            current,
            headroom,
        };
        self.envelope.insert(metric_id, bound);
    }

    /// Full unified self-assessment across all subsystems
    pub fn unified_self_assessment(&mut self) -> SelfModelStats {
        self.tick += 1;
        let cap_count = self.capabilities.len();
        let lim_count = self.limitations.len();

        let headroom_avg = if self.envelope.is_empty() {
            0.5
        } else {
            self.envelope.values().map(|e| e.headroom).sum::<f32>()
                / self.envelope.len() as f32
        };

        let velocity = if self.trajectory.len() >= 2 {
            let last = self.trajectory[self.trajectory.len() - 1].overall_score;
            let prev = self.trajectory[self.trajectory.len() - 2].overall_score;
            last - prev
        } else {
            0.0
        };

        let overall = self.overall_performance_ema * 0.35
            + self.overall_reliability_ema * 0.30
            + self.overall_maturity_ema * 0.20
            + self.consistency_ema * 0.15;

        let point = TrajectoryPoint {
            tick: self.tick,
            overall_score: overall,
            capability_count: cap_count as u32,
            limitation_count: lim_count as u32,
            maturity_avg: self.overall_maturity_ema,
        };
        if self.trajectory.len() < MAX_HISTORY {
            self.trajectory.push(point);
        } else {
            let idx = (self.tick as usize) % MAX_HISTORY;
            self.trajectory[idx] = point;
        }

        SelfModelStats {
            total_capabilities: cap_count,
            total_limitations: lim_count,
            avg_maturity: self.overall_maturity_ema,
            avg_performance: self.overall_performance_ema,
            avg_reliability: self.overall_reliability_ema,
            overall_self_score: overall,
            consistency_score: self.consistency_ema,
            envelope_headroom: headroom_avg,
            improvement_velocity: velocity,
        }
    }

    /// Produce the full capability matrix: domain → list of capabilities
    pub fn capability_matrix(&self) -> BTreeMap<u8, Vec<(u64, f32, f32)>> {
        let mut matrix: BTreeMap<u8, Vec<(u64, f32, f32)>> = BTreeMap::new();
        for cap in self.capabilities.values() {
            matrix
                .entry(cap.domain as u8)
                .or_insert_with(Vec::new)
                .push((cap.id, cap.performance, cap.reliability));
        }
        matrix
    }

    /// Map all known limitations by domain
    pub fn limitation_map(&self) -> BTreeMap<u8, Vec<(u64, f32, bool)>> {
        let mut map: BTreeMap<u8, Vec<(u64, f32, bool)>> = BTreeMap::new();
        for lim in self.limitations.values() {
            map.entry(lim.domain as u8)
                .or_insert_with(Vec::new)
                .push((lim.id, lim.severity, lim.workaround_available));
        }
        map
    }

    /// Current performance envelope boundaries
    pub fn performance_envelope(&self) -> Vec<EnvelopeBound> {
        self.envelope.values().cloned().collect()
    }

    /// Compute the improvement vector: direction and magnitude of growth
    pub fn improvement_vector(&self) -> (f32, f32) {
        if self.trajectory.len() < 3 {
            return (0.0, 0.0);
        }
        let n = self.trajectory.len();
        let recent = &self.trajectory[n.saturating_sub(5)..n];
        let direction = if recent.len() >= 2 {
            recent.last().map_or(0.0, |l| l.overall_score)
                - recent.first().map_or(0.0, |f| f.overall_score)
        } else {
            0.0
        };
        let magnitude = recent.iter().map(|p| p.overall_score).sum::<f32>()
            / recent.len() as f32;
        (direction, magnitude)
    }

    /// Check self-model consistency across subsystems
    pub fn self_consistency_check(&mut self) -> f32 {
        if self.subsystem_scores.len() < 2 {
            self.consistency_ema = 1.0;
            return 1.0;
        }

        let scores: Vec<f32> = self.subsystem_scores.values().copied().collect();
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>()
            / scores.len() as f32;
        let std_dev = if variance > 0.0 {
            f32_sqrt(variance)
        } else {
            0.0
        };

        let consistency = if std_dev < CONSISTENCY_THRESHOLD {
            1.0
        } else {
            (1.0 - (std_dev - CONSISTENCY_THRESHOLD) * 2.0).clamp(0.0, 1.0)
        };

        self.consistency_ema =
            EMA_ALPHA * consistency + (1.0 - EMA_ALPHA) * self.consistency_ema;
        self.consistency_ema
    }

    /// Get the trajectory history for plotting improvement over time
    pub fn trajectory_history(&self) -> &[TrajectoryPoint] {
        &self.trajectory
    }

    /// Get per-subsystem scores
    pub fn subsystem_scores(&self) -> &BTreeMap<u8, f32> {
        &self.subsystem_scores
    }
}

/// Integer-only square root approximation via Newton's method
fn f32_sqrt(val: f32) -> f32 {
    if val <= 0.0 {
        return 0.0;
    }
    let mut guess = val * 0.5;
    for _ in 0..8 {
        if guess <= 0.0 {
            return 0.0;
        }
        guess = (guess + val / guess) * 0.5;
    }
    guess
}
