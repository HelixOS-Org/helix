// SPDX-License-Identifier: GPL-2.0
//! # Holistic Meta-Cognition
//!
//! The highest level of meta-cognition. Thinks about the ENTIRE system's
//! thinking. Evaluates the cognitive architecture itself — are the right
//! subsystems paying attention to the right things? Are there systemic
//! biases distorting the kernel's reasoning? Where should the architecture
//! evolve next?
//!
//! Meta-cognition is the kernel thinking about how it thinks.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SUBSYSTEMS: usize = 16;
const MAX_BIASES: usize = 128;
const MAX_OPTIMIZATIONS: usize = 64;
const MAX_HISTORY: usize = 256;
const EMA_ALPHA: f32 = 0.08;
const ATTENTION_IMBALANCE_THRESHOLD: f32 = 0.25;
const BIAS_SIGNIFICANCE: f32 = 0.10;
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
// COGNITIVE COMPONENT
// ============================================================================

/// A cognitive subsystem within the architecture
#[derive(Debug, Clone)]
pub struct CognitiveComponent {
    pub name: String,
    pub id: u64,
    pub attention_share: f32,
    pub processing_quality: f32,
    pub decision_throughput: u64,
    pub error_rate: f32,
    pub last_update_tick: u64,
    pub importance_weight: f32,
}

/// A systemic bias detected across the architecture
#[derive(Debug, Clone)]
pub struct SystemicBias {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub affected_components: Vec<u64>,
    pub magnitude: f32,
    pub direction: f32,
    pub detection_tick: u64,
    pub confidence: f32,
    pub sample_count: u64,
}

/// A meta-optimization recommendation
#[derive(Debug, Clone)]
pub struct MetaOptimization {
    pub id: u64,
    pub description: String,
    pub target_component: u64,
    pub expected_improvement: f32,
    pub priority: f32,
    pub tick_proposed: u64,
    pub applied: bool,
}

/// Snapshot of architecture evaluation
#[derive(Debug, Clone, Copy)]
pub struct ArchitectureSnapshot {
    pub tick: u64,
    pub overall_score: f32,
    pub attention_balance: f32,
    pub bias_severity: f32,
    pub optimization_potential: f32,
}

/// Evolution direction recommendation
#[derive(Debug, Clone)]
pub struct EvolutionDirection {
    pub primary_focus: String,
    pub secondary_focus: String,
    pub components_to_strengthen: Vec<u64>,
    pub components_to_prune: Vec<u64>,
    pub confidence: f32,
    pub horizon_ticks: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate meta-cognition statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct MetaCognitionStats {
    pub component_count: usize,
    pub bias_count: usize,
    pub pending_optimizations: usize,
    pub architecture_score: f32,
    pub attention_balance: f32,
    pub total_bias_magnitude: f32,
    pub evolution_velocity: f32,
    pub cognitive_efficiency: f32,
}

// ============================================================================
// HOLISTIC META-COGNITION
// ============================================================================

/// The highest-level meta-cognition engine. Evaluates the cognitive
/// architecture, balances attention distribution, identifies systemic
/// biases, and recommends architectural evolution.
#[derive(Debug)]
pub struct HolisticMetaCognition {
    components: BTreeMap<u64, CognitiveComponent>,
    biases: BTreeMap<u64, SystemicBias>,
    optimizations: BTreeMap<u64, MetaOptimization>,
    history: Vec<ArchitectureSnapshot>,
    tick: u64,
    rng_state: u64,
    architecture_score_ema: f32,
    attention_balance_ema: f32,
    bias_severity_ema: f32,
    efficiency_ema: f32,
}

impl HolisticMetaCognition {
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
            biases: BTreeMap::new(),
            optimizations: BTreeMap::new(),
            history: Vec::new(),
            tick: 0,
            rng_state: 0x1234_ABCD_EF56_7890,
            architecture_score_ema: 0.5,
            attention_balance_ema: 0.5,
            bias_severity_ema: 0.0,
            efficiency_ema: 0.5,
        }
    }

    /// Register a cognitive component in the architecture
    pub fn register_component(
        &mut self,
        name: String,
        attention_share: f32,
        importance: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.components.len() >= MAX_SUBSYSTEMS {
            return id;
        }

        let component = CognitiveComponent {
            name,
            id,
            attention_share: attention_share.clamp(0.0, 1.0),
            processing_quality: 0.5,
            decision_throughput: 0,
            error_rate: 0.0,
            last_update_tick: self.tick,
            importance_weight: importance.clamp(0.0, 1.0),
        };
        self.components.insert(id, component);
        id
    }

    /// Update a component's performance metrics
    #[inline]
    pub fn update_component(
        &mut self,
        component_id: u64,
        quality: f32,
        throughput: u64,
        error_rate: f32,
    ) {
        self.tick += 1;
        if let Some(comp) = self.components.get_mut(&component_id) {
            comp.processing_quality =
                EMA_ALPHA * quality.clamp(0.0, 1.0) + (1.0 - EMA_ALPHA) * comp.processing_quality;
            comp.decision_throughput = throughput;
            comp.error_rate =
                EMA_ALPHA * error_rate.clamp(0.0, 1.0) + (1.0 - EMA_ALPHA) * comp.error_rate;
            comp.last_update_tick = self.tick;
        }
    }

    /// Evaluate the entire cognitive architecture
    #[inline]
    pub fn architecture_evaluation(&mut self) -> f32 {
        if self.components.is_empty() {
            return 0.0;
        }

        let total_quality: f32 = self
            .components
            .values()
            .map(|c| c.processing_quality * c.importance_weight)
            .sum();
        let total_weight: f32 = self.components.values().map(|c| c.importance_weight).sum();
        let weighted_quality = if total_weight > 0.0 {
            total_quality / total_weight
        } else {
            0.0
        };

        let error_penalty: f32 = self
            .components
            .values()
            .map(|c| c.error_rate * c.importance_weight)
            .sum::<f32>()
            / total_weight.max(0.01);

        let score = (weighted_quality * 0.7 + (1.0 - error_penalty) * 0.3).clamp(0.0, 1.0);
        self.architecture_score_ema =
            EMA_ALPHA * score + (1.0 - EMA_ALPHA) * self.architecture_score_ema;

        let snapshot = ArchitectureSnapshot {
            tick: self.tick,
            overall_score: self.architecture_score_ema,
            attention_balance: self.attention_balance_ema,
            bias_severity: self.bias_severity_ema,
            optimization_potential: 1.0 - self.architecture_score_ema,
        };
        if self.history.len() < MAX_HISTORY {
            self.history.push(snapshot);
        } else {
            let idx = (self.tick as usize) % MAX_HISTORY;
            self.history[idx] = snapshot;
        }

        self.architecture_score_ema
    }

    /// Analyze attention distribution across components
    #[inline]
    pub fn attention_distribution(&mut self) -> BTreeMap<u64, f32> {
        let total_attention: f32 = self.components.values().map(|c| c.attention_share).sum();
        let mut distribution = BTreeMap::new();

        if self.components.is_empty() || total_attention <= 0.0 {
            return distribution;
        }

        let ideal_share = 1.0 / self.components.len() as f32;
        let mut imbalance_sum = 0.0f32;

        for comp in self.components.values() {
            let normalized = comp.attention_share / total_attention;
            distribution.insert(comp.id, normalized);
            let deviation = (normalized - ideal_share * comp.importance_weight).abs();
            imbalance_sum += deviation;
        }

        let balance = (1.0 - imbalance_sum / self.components.len() as f32).clamp(0.0, 1.0);
        self.attention_balance_ema =
            EMA_ALPHA * balance + (1.0 - EMA_ALPHA) * self.attention_balance_ema;

        distribution
    }

    /// Scan for systemic biases across the architecture
    pub fn systemic_bias_scan(&mut self) -> Vec<SystemicBias> {
        self.tick += 1;
        let mut found = Vec::new();

        // Check for attention bias: some components getting disproportionate attention
        let total_attention: f32 = self.components.values().map(|c| c.attention_share).sum();
        if total_attention > 0.0 {
            for comp in self.components.values() {
                let share = comp.attention_share / total_attention;
                let expected = comp.importance_weight
                    / self
                        .components
                        .values()
                        .map(|c| c.importance_weight)
                        .sum::<f32>()
                        .max(0.01);
                let deviation = share - expected;
                if deviation.abs() > ATTENTION_IMBALANCE_THRESHOLD {
                    let id = fnv1a_hash(comp.name.as_bytes()) ^ self.tick;
                    let bias = SystemicBias {
                        id,
                        name: String::from("attention_bias"),
                        description: String::from("Disproportionate attention allocation"),
                        affected_components: alloc::vec![comp.id],
                        magnitude: deviation.abs(),
                        direction: deviation,
                        detection_tick: self.tick,
                        confidence: (comp.last_update_tick as f32 / self.tick as f32).min(1.0),
                        sample_count: comp.decision_throughput,
                    };
                    if self.biases.len() < MAX_BIASES {
                        self.biases.insert(id, bias.clone());
                    }
                    found.push(bias);
                }
            }
        }

        // Check for quality bias: high-error components getting too much trust
        for comp in self.components.values() {
            if comp.error_rate > BIAS_SIGNIFICANCE && comp.processing_quality > 0.7 {
                let id = fnv1a_hash(&comp.id.to_le_bytes()) ^ self.tick;
                let bias = SystemicBias {
                    id,
                    name: String::from("quality_overestimation"),
                    description: String::from("Component quality rated above actual performance"),
                    affected_components: alloc::vec![comp.id],
                    magnitude: comp.error_rate,
                    direction: comp.processing_quality - (1.0 - comp.error_rate),
                    detection_tick: self.tick,
                    confidence: 0.6,
                    sample_count: comp.decision_throughput,
                };
                if self.biases.len() < MAX_BIASES {
                    self.biases.insert(id, bias.clone());
                }
                found.push(bias);
            }
        }

        let severity = if found.is_empty() {
            0.0
        } else {
            found.iter().map(|b| b.magnitude).sum::<f32>() / found.len() as f32
        };
        self.bias_severity_ema = EMA_ALPHA * severity + (1.0 - EMA_ALPHA) * self.bias_severity_ema;

        found
    }

    /// Propose meta-optimizations for the architecture
    pub fn meta_optimization(&mut self) -> Vec<MetaOptimization> {
        self.tick += 1;
        let mut proposals = Vec::new();

        for comp in self.components.values() {
            if comp.processing_quality < 0.5 && comp.importance_weight > 0.3 {
                let expected = comp.importance_weight - comp.processing_quality;
                let id = fnv1a_hash(&comp.id.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
                let opt = MetaOptimization {
                    id,
                    description: String::from("Improve processing quality of critical component"),
                    target_component: comp.id,
                    expected_improvement: expected.clamp(0.0, 1.0),
                    priority: comp.importance_weight * (1.0 - comp.processing_quality),
                    tick_proposed: self.tick,
                    applied: false,
                };
                if self.optimizations.len() < MAX_OPTIMIZATIONS {
                    self.optimizations.insert(id, opt.clone());
                }
                proposals.push(opt);
            }

            if comp.error_rate > 0.15 {
                let id = fnv1a_hash(&[comp.id as u8]) ^ xorshift64(&mut self.rng_state);
                let opt = MetaOptimization {
                    id,
                    description: String::from("Reduce error rate in component"),
                    target_component: comp.id,
                    expected_improvement: comp.error_rate * 0.5,
                    priority: comp.error_rate * comp.importance_weight,
                    tick_proposed: self.tick,
                    applied: false,
                };
                if self.optimizations.len() < MAX_OPTIMIZATIONS {
                    self.optimizations.insert(id, opt.clone());
                }
                proposals.push(opt);
            }
        }

        proposals.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        proposals
    }

    /// Overall cognitive architecture score (0.0 – 1.0)
    #[inline]
    pub fn cognitive_architecture_score(&self) -> f32 {
        self.architecture_score_ema * 0.40
            + self.attention_balance_ema * 0.25
            + (1.0 - self.bias_severity_ema) * 0.20
            + self.efficiency_ema * 0.15
    }

    /// Recommend the direction of architectural evolution
    pub fn evolution_direction(&self) -> EvolutionDirection {
        let mut weakest: Vec<(u64, f32)> = self
            .components
            .values()
            .map(|c| (c.id, c.processing_quality * c.importance_weight))
            .collect();
        weakest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        let strengthen: Vec<u64> = weakest.iter().take(3).map(|(id, _)| *id).collect();
        let prune: Vec<u64> = self
            .components
            .values()
            .filter(|c| c.importance_weight < 0.1 && c.processing_quality < 0.3)
            .map(|c| c.id)
            .collect();

        let velocity = if self.history.len() >= 2 {
            let last = self.history[self.history.len() - 1].overall_score;
            let prev = self.history[self.history.len() - 2].overall_score;
            last - prev
        } else {
            0.0
        };

        self.efficiency_ema;

        EvolutionDirection {
            primary_focus: String::from("Strengthen weakest critical components"),
            secondary_focus: String::from("Reduce systemic biases"),
            components_to_strengthen: strengthen,
            components_to_prune: prune,
            confidence: self.architecture_score_ema,
            horizon_ticks: ((1.0 / (velocity.abs() + 0.001)) * 100.0) as u64,
        }
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> MetaCognitionStats {
        let pending = self.optimizations.values().filter(|o| !o.applied).count();
        let total_bias: f32 = self.biases.values().map(|b| b.magnitude).sum();

        let velocity = if self.history.len() >= 2 {
            let last = self.history[self.history.len() - 1].overall_score;
            let prev = self.history[self.history.len() - 2].overall_score;
            last - prev
        } else {
            0.0
        };

        MetaCognitionStats {
            component_count: self.components.len(),
            bias_count: self.biases.len(),
            pending_optimizations: pending,
            architecture_score: self.architecture_score_ema,
            attention_balance: self.attention_balance_ema,
            total_bias_magnitude: total_bias,
            evolution_velocity: velocity,
            cognitive_efficiency: self.efficiency_ema,
        }
    }
}
