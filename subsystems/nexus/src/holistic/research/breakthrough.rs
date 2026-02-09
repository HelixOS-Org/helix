// SPDX-License-Identifier: GPL-2.0
//! # Holistic Breakthrough Detector — System-Level Discovery Detection
//!
//! Detects when research across the NEXUS kernel produces a genuine
//! SYSTEM-LEVEL breakthrough — a finding so significant that it changes
//! how the kernel optimises itself. Individual subsystem improvements
//! are incremental; this engine watches for cross-domain cascades,
//! paradigm-shifting insights, and historically significant discoveries.
//!
//! ## Capabilities
//!
//! - **System breakthrough detection** — identify breakthrough-class findings
//! - **Cross-domain breakthrough** — breakthroughs spanning multiple domains
//! - **Paradigm impact scoring** — quantify impact on kernel philosophy
//! - **Breakthrough cascade tracking** — secondary effects of breakthroughs
//! - **Historical ranking** — rank breakthroughs against all-time records
//! - **Breakthrough prediction** — forecast upcoming breakthrough potential
//!
//! The engine that recognises when a kernel revolution has occurred.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_BREAKTHROUGHS: usize = 512;
const MAX_CASCADE_DEPTH: usize = 8;
const MAX_PREDICTIONS: usize = 128;
const MAX_HISTORY: usize = 1024;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const BREAKTHROUGH_THRESHOLD: f32 = 0.80;
const CROSS_DOMAIN_THRESHOLD: f32 = 0.65;
const PARADIGM_SHIFT_THRESHOLD: f32 = 0.90;
const CASCADE_DECAY: f32 = 0.85;
const PREDICTION_HORIZON: u64 = 200;
const HISTORICAL_SIGNIFICANCE_FLOOR: f32 = 0.50;
const MOMENTUM_WINDOW: usize = 64;

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

/// Domain in which a breakthrough occurs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreakthroughDomain {
    Scheduling,
    Memory,
    Ipc,
    FileSystem,
    Networking,
    Trust,
    Energy,
    Hardware,
    SystemWide,
    Emergent,
}

/// Magnitude classification of a breakthrough
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreakthroughMagnitude {
    Minor,
    Notable,
    Significant,
    Major,
    Paradigmatic,
    Revolutionary,
}

/// A detected system-level breakthrough
#[derive(Debug, Clone)]
pub struct Breakthrough {
    pub id: u64,
    pub domain: BreakthroughDomain,
    pub description: String,
    pub magnitude: BreakthroughMagnitude,
    pub effect_size: f32,
    pub confidence: f32,
    pub domains_affected: Vec<BreakthroughDomain>,
    pub paradigm_impact: f32,
    pub historical_rank: u32,
    pub detected_tick: u64,
    pub hash: u64,
}

/// Cascade effect from a breakthrough
#[derive(Debug, Clone)]
pub struct BreakthroughCascade {
    pub id: u64,
    pub source_breakthrough: u64,
    pub affected_domain: BreakthroughDomain,
    pub cascade_depth: u32,
    pub effect_strength: f32,
    pub propagation_tick: u64,
}

/// Breakthrough prediction
#[derive(Debug, Clone)]
pub struct BreakthroughPrediction {
    pub id: u64,
    pub predicted_domain: BreakthroughDomain,
    pub probability: f32,
    pub estimated_magnitude: BreakthroughMagnitude,
    pub momentum_score: f32,
    pub predicted_tick: u64,
    pub created_tick: u64,
}

/// Historical record for ranking
#[derive(Debug, Clone)]
pub struct HistoricalRecord {
    pub breakthrough_id: u64,
    pub effect_size: f32,
    pub domains_count: u64,
    pub paradigm_impact: f32,
    pub composite_score: f32,
    pub tick: u64,
}

/// Research momentum tracker per domain
#[derive(Debug, Clone)]
pub struct DomainMomentum {
    pub domain: BreakthroughDomain,
    pub recent_effects: VecDeque<f32>,
    pub momentum_ema: f32,
    pub acceleration: f32,
    pub breakthrough_count: u64,
    pub last_breakthrough_tick: u64,
}

/// Breakthrough detection statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BreakthroughStats {
    pub total_detected: u64,
    pub cross_domain_breakthroughs: u64,
    pub paradigmatic_breakthroughs: u64,
    pub total_cascades: u64,
    pub max_cascade_depth: u32,
    pub avg_effect_size_ema: f32,
    pub avg_paradigm_impact_ema: f32,
    pub prediction_accuracy_ema: f32,
    pub momentum_global_ema: f32,
    pub historical_records: u64,
    pub predictions_made: u64,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC BREAKTHROUGH DETECTOR
// ============================================================================

/// System-level breakthrough detection engine
pub struct HolisticBreakthroughDetector {
    breakthroughs: BTreeMap<u64, Breakthrough>,
    cascades: Vec<BreakthroughCascade>,
    predictions: Vec<BreakthroughPrediction>,
    history: VecDeque<HistoricalRecord>,
    domain_momentum: BTreeMap<u64, DomainMomentum>,
    rng_state: u64,
    tick: u64,
    stats: BreakthroughStats,
}

impl HolisticBreakthroughDetector {
    /// Create a new holistic breakthrough detector
    pub fn new(seed: u64) -> Self {
        Self {
            breakthroughs: BTreeMap::new(),
            cascades: Vec::new(),
            predictions: Vec::new(),
            history: VecDeque::new(),
            domain_momentum: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: BreakthroughStats {
                total_detected: 0,
                cross_domain_breakthroughs: 0,
                paradigmatic_breakthroughs: 0,
                total_cascades: 0,
                max_cascade_depth: 0,
                avg_effect_size_ema: 0.0,
                avg_paradigm_impact_ema: 0.0,
                prediction_accuracy_ema: 0.5,
                momentum_global_ema: 0.0,
                historical_records: 0,
                predictions_made: 0,
                last_tick: 0,
            },
        }
    }

    /// Initialize momentum tracking for a domain
    #[inline]
    pub fn register_domain(&mut self, domain: BreakthroughDomain) {
        let key = domain as u64;
        if self.domain_momentum.contains_key(&key) { return; }
        self.domain_momentum.insert(key, DomainMomentum {
            domain, recent_effects: VecDeque::new(), momentum_ema: 0.0,
            acceleration: 0.0, breakthrough_count: 0, last_breakthrough_tick: 0,
        });
    }

    /// Feed a research effect for momentum tracking
    pub fn feed_effect(&mut self, domain: BreakthroughDomain, effect: f32) {
        let key = domain as u64;
        if let Some(momentum) = self.domain_momentum.get_mut(&key) {
            if momentum.recent_effects.len() >= MOMENTUM_WINDOW {
                momentum.recent_effects.pop_front().unwrap();
            }
            let old_momentum = momentum.momentum_ema;
            momentum.recent_effects.push(effect);
            momentum.momentum_ema = momentum.momentum_ema
                * (1.0 - EMA_ALPHA) + effect * EMA_ALPHA;
            momentum.acceleration = momentum.momentum_ema - old_momentum;
        }
    }

    /// Detect a system-level breakthrough
    pub fn system_breakthrough(&mut self, domain: BreakthroughDomain,
                                description: String, effect: f32, confidence: f32)
        -> Option<Breakthrough>
    {
        if effect < BREAKTHROUGH_THRESHOLD || confidence < 0.5 { return None; }
        let magnitude = if effect >= 0.98 { BreakthroughMagnitude::Revolutionary }
            else if effect >= 0.95 { BreakthroughMagnitude::Paradigmatic }
            else if effect >= 0.90 { BreakthroughMagnitude::Major }
            else if effect >= 0.85 { BreakthroughMagnitude::Significant }
            else if effect >= 0.82 { BreakthroughMagnitude::Notable }
            else { BreakthroughMagnitude::Minor };
        let paradigm_impact = if magnitude == BreakthroughMagnitude::Paradigmatic
            || magnitude == BreakthroughMagnitude::Revolutionary {
            effect * confidence
        } else {
            effect * confidence * 0.5
        };
        let id = self.stats.total_detected;
        let hash = fnv1a_hash(description.as_bytes());
        let rank = self.compute_historical_rank(effect, paradigm_impact);
        let bt = Breakthrough {
            id, domain, description, magnitude, effect_size: effect,
            confidence, domains_affected: alloc::vec![domain],
            paradigm_impact, historical_rank: rank,
            detected_tick: self.tick, hash,
        };
        if self.breakthroughs.len() >= MAX_BREAKTHROUGHS {
            let oldest = self.breakthroughs.keys().next().copied();
            if let Some(k) = oldest { self.breakthroughs.remove(&k); }
        }
        self.breakthroughs.insert(id, bt.clone());
        self.history.push_back(HistoricalRecord {
            breakthrough_id: id, effect_size: effect,
            domains_count: 1, paradigm_impact,
            composite_score: effect * confidence * paradigm_impact,
            tick: self.tick,
        });
        if self.history.len() > MAX_HISTORY { self.history.pop_front(); }
        self.stats.total_detected += 1;
        self.stats.avg_effect_size_ema = self.stats.avg_effect_size_ema
            * (1.0 - EMA_ALPHA) + effect * EMA_ALPHA;
        self.stats.avg_paradigm_impact_ema = self.stats.avg_paradigm_impact_ema
            * (1.0 - EMA_ALPHA) + paradigm_impact * EMA_ALPHA;
        self.stats.historical_records = self.history.len() as u64;
        if magnitude == BreakthroughMagnitude::Paradigmatic
            || magnitude == BreakthroughMagnitude::Revolutionary {
            self.stats.paradigmatic_breakthroughs += 1;
        }
        if let Some(m) = self.domain_momentum.get_mut(&(domain as u64)) {
            m.breakthrough_count += 1;
            m.last_breakthrough_tick = self.tick;
        }
        self.stats.last_tick = self.tick;
        Some(bt)
    }

    /// Detect cross-domain breakthroughs spanning multiple subsystems
    pub fn cross_domain_breakthrough(&mut self, domains: Vec<BreakthroughDomain>,
                                      description: String, effect: f32, confidence: f32)
        -> Option<Breakthrough>
    {
        if domains.len() < 2 || effect < CROSS_DOMAIN_THRESHOLD { return None; }
        let cross_bonus = (domains.len() as f32 - 1.0) * 0.1;
        let adjusted_effect = (effect + cross_bonus).min(1.0);
        let magnitude = if adjusted_effect >= 0.95 { BreakthroughMagnitude::Revolutionary }
            else if adjusted_effect >= 0.88 { BreakthroughMagnitude::Paradigmatic }
            else if adjusted_effect >= 0.80 { BreakthroughMagnitude::Major }
            else if adjusted_effect >= 0.72 { BreakthroughMagnitude::Significant }
            else { BreakthroughMagnitude::Notable };
        let paradigm_impact = adjusted_effect * confidence * (1.0 + cross_bonus);
        let id = self.stats.total_detected;
        let hash = fnv1a_hash(description.as_bytes());
        let rank = self.compute_historical_rank(adjusted_effect, paradigm_impact);
        let primary = domains.first().copied().unwrap_or(BreakthroughDomain::SystemWide);
        let bt = Breakthrough {
            id, domain: primary, description, magnitude,
            effect_size: adjusted_effect, confidence,
            domains_affected: domains.clone(), paradigm_impact,
            historical_rank: rank, detected_tick: self.tick, hash,
        };
        self.breakthroughs.insert(id, bt.clone());
        self.stats.total_detected += 1;
        self.stats.cross_domain_breakthroughs += 1;
        self.history.push_back(HistoricalRecord {
            breakthrough_id: id, effect_size: adjusted_effect,
            domains_count: domains.len() as u64, paradigm_impact,
            composite_score: adjusted_effect * confidence * paradigm_impact,
            tick: self.tick,
        });
        self.stats.last_tick = self.tick;
        Some(bt)
    }

    /// Assess paradigm impact of a breakthrough
    #[inline]
    pub fn paradigm_impact(&self, breakthrough_id: u64) -> f32 {
        self.breakthroughs.get(&breakthrough_id)
            .map(|bt| bt.paradigm_impact)
            .unwrap_or(0.0)
    }

    /// Trace the cascade of secondary effects from a breakthrough
    pub fn breakthrough_cascade(&mut self, breakthrough_id: u64) -> Vec<BreakthroughCascade> {
        let bt = match self.breakthroughs.get(&breakthrough_id) {
            Some(b) => b.clone(),
            None => return Vec::new(),
        };
        let mut cascades = Vec::new();
        let all_domains = [
            BreakthroughDomain::Scheduling, BreakthroughDomain::Memory,
            BreakthroughDomain::Ipc, BreakthroughDomain::FileSystem,
            BreakthroughDomain::Networking, BreakthroughDomain::Trust,
            BreakthroughDomain::Energy, BreakthroughDomain::Hardware,
        ];
        let mut current_strength = bt.effect_size;
        for depth in 1..=MAX_CASCADE_DEPTH {
            if current_strength < 0.05 { break; }
            for &domain in &all_domains {
                if bt.domains_affected.contains(&domain) { continue; }
                let noise = xorshift_f32(&mut self.rng_state) * 0.1;
                let effect = current_strength * (0.3 + noise);
                if effect > 0.05 {
                    let cas_id = self.stats.total_cascades;
                    let cascade = BreakthroughCascade {
                        id: cas_id, source_breakthrough: breakthrough_id,
                        affected_domain: domain, cascade_depth: depth as u32,
                        effect_strength: effect, propagation_tick: self.tick,
                    };
                    cascades.push(cascade.clone());
                    self.cascades.push(cascade);
                    self.stats.total_cascades += 1;
                    if depth as u32 > self.stats.max_cascade_depth {
                        self.stats.max_cascade_depth = depth as u32;
                    }
                }
            }
            current_strength *= CASCADE_DECAY;
        }
        cascades
    }

    /// Rank a breakthrough historically
    #[inline]
    pub fn historical_rank(&self, breakthrough_id: u64) -> u32 {
        self.breakthroughs.get(&breakthrough_id)
            .map(|bt| bt.historical_rank)
            .unwrap_or(0)
    }

    fn compute_historical_rank(&self, effect: f32, paradigm_impact: f32) -> u32 {
        let composite = effect * paradigm_impact;
        let higher = self.history.iter()
            .filter(|h| h.composite_score > composite).count();
        (higher as u32) + 1
    }

    /// Predict upcoming breakthroughs based on momentum
    pub fn breakthrough_prediction(&mut self) -> Vec<BreakthroughPrediction> {
        let mut predictions = Vec::new();
        for momentum in self.domain_momentum.values() {
            if momentum.momentum_ema < 0.3 { continue; }
            let probability = (momentum.momentum_ema * 0.5
                + momentum.acceleration.max(0.0) * 2.0).min(1.0);
            if probability < 0.2 { continue; }
            let est_magnitude = if probability > 0.8 { BreakthroughMagnitude::Major }
                else if probability > 0.6 { BreakthroughMagnitude::Significant }
                else if probability > 0.4 { BreakthroughMagnitude::Notable }
                else { BreakthroughMagnitude::Minor };
            let id = self.stats.predictions_made;
            let noise = xorshift64(&mut self.rng_state) % PREDICTION_HORIZON;
            predictions.push(BreakthroughPrediction {
                id, predicted_domain: momentum.domain, probability,
                estimated_magnitude: est_magnitude,
                momentum_score: momentum.momentum_ema,
                predicted_tick: self.tick + noise,
                created_tick: self.tick,
            });
            self.stats.predictions_made += 1;
        }
        predictions.sort_by(|a, b| b.probability.partial_cmp(&a.probability)
            .unwrap_or(core::cmp::Ordering::Equal));
        predictions.truncate(MAX_PREDICTIONS);
        let global_momentum: f32 = self.domain_momentum.values()
            .map(|m| m.momentum_ema).sum::<f32>()
            / (self.domain_momentum.len().max(1) as f32);
        self.stats.momentum_global_ema = self.stats.momentum_global_ema
            * (1.0 - EMA_ALPHA) + global_momentum * EMA_ALPHA;
        for p in &predictions {
            if self.predictions.len() < MAX_PREDICTIONS {
                self.predictions.push(p.clone());
            }
        }
        predictions
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Validate a previous prediction against actual outcome
    #[inline]
    pub fn validate_prediction(&mut self, prediction_idx: usize,
                                actual_breakthrough: bool) {
        if prediction_idx >= self.predictions.len() { return; }
        let correct = if actual_breakthrough {
            self.predictions[prediction_idx].probability >= 0.5
        } else {
            self.predictions[prediction_idx].probability < 0.5
        };
        let accuracy_delta = if correct { 1.0f32 } else { 0.0 };
        self.stats.prediction_accuracy_ema = self.stats.prediction_accuracy_ema
            * (1.0 - EMA_ALPHA) + accuracy_delta * EMA_ALPHA;
    }

    /// Get the top-N breakthroughs by composite historical score
    #[inline]
    pub fn top_breakthroughs(&self, n: usize) -> Vec<&Breakthrough> {
        let mut sorted: Vec<&Breakthrough> = self.breakthroughs.values().collect();
        sorted.sort_by(|a, b| {
            let score_a = a.effect_size * a.confidence * a.paradigm_impact;
            let score_b = b.effect_size * b.confidence * b.paradigm_impact;
            score_b.partial_cmp(&score_a).unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    /// Get domain momentum for a specific domain
    #[inline(always)]
    pub fn domain_momentum(&self, domain: BreakthroughDomain) -> Option<&DomainMomentum> {
        self.domain_momentum.get(&(domain as u64))
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &BreakthroughStats { &self.stats }

    /// Get all detected breakthroughs
    #[inline(always)]
    pub fn breakthroughs(&self) -> &BTreeMap<u64, Breakthrough> { &self.breakthroughs }

    /// Get cascade log
    #[inline(always)]
    pub fn cascade_log(&self) -> &[BreakthroughCascade] { &self.cascades }

    /// Get all predictions
    #[inline(always)]
    pub fn predictions(&self) -> &[BreakthroughPrediction] { &self.predictions }

    /// Get historical records
    #[inline(always)]
    pub fn historical_records(&self) -> &[HistoricalRecord] { &self.history }

    /// Get all domain momentum entries
    #[inline(always)]
    pub fn all_domain_momentum(&self) -> &BTreeMap<u64, DomainMomentum> {
        &self.domain_momentum
    }
}
