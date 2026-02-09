// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Replication — Replication of Cooperation Findings
//!
//! Ensures that cooperation findings are reproducible across different
//! conditions. When a fairness improvement or trust model is validated,
//! the replication engine re-runs the experiment under varied environments:
//! different load levels, subsystem counts, contention patterns, and time
//! horizons. Tracks replication success rates, environmental sensitivity,
//! and identifies which findings are robust and which are fragile. Only
//! robust, replicated findings should be deployed in production.
//!
//! The engine that makes sure cooperation improvements are real.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_REPLICATIONS: usize = 512;
const MAX_FINDINGS: usize = 256;
const MAX_ENVIRONMENTS: usize = 32;
const REPLICATION_THRESHOLD: f32 = 0.66;
const STRONG_REPLICATION: f32 = 0.85;
const EFFECT_TOLERANCE: f32 = 0.30;
const MIN_REPLICATIONS_REQUIRED: usize = 3;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const ENVIRONMENT_VARIATION: f32 = 0.20;
const CONFIDENCE_BOOST_PER_REPLICATION: f32 = 0.05;
const FRAGILITY_THRESHOLD: f32 = 0.40;
const ROBUST_THRESHOLD: f32 = 0.80;
const MAX_ROBUST_STRATEGIES: usize = 64;

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
// REPLICATION TYPES
// ============================================================================

/// Domain of the finding being replicated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplicationDomain {
    FairnessImprovement,
    TrustModel,
    ContentionReduction,
    SharingStrategy,
    NegotiationProtocol,
    AuctionMechanism,
    CoalitionFormation,
}

/// Outcome of a single replication attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplicationOutcome {
    Replicated,
    PartialReplication,
    FailedToReplicate,
    Inconclusive,
}

/// An environment configuration for replication
#[derive(Debug, Clone)]
pub struct ReplicationEnvironment {
    pub id: u64,
    pub name: String,
    pub load_level: f32,
    pub subsystem_count: u32,
    pub contention_factor: f32,
    pub time_horizon: u64,
    pub noise_level: f32,
}

/// A single replication attempt
#[derive(Debug, Clone)]
pub struct ReplicationAttempt {
    pub id: u64,
    pub finding_id: u64,
    pub environment_id: u64,
    pub original_effect: f32,
    pub replicated_effect: f32,
    pub effect_ratio: f32,
    pub outcome: ReplicationOutcome,
    pub tick: u64,
    pub domain: ReplicationDomain,
}

/// A finding submitted for replication
#[derive(Debug, Clone)]
pub struct ReplicableFinding {
    pub id: u64,
    pub domain: ReplicationDomain,
    pub description: String,
    pub original_effect_size: f32,
    pub original_sample_size: usize,
    pub replications: Vec<ReplicationAttempt>,
    pub replication_rate: f32,
    pub confidence: f32,
    pub robust: bool,
    pub created_tick: u64,
}

/// A robust strategy that has been successfully replicated
#[derive(Debug, Clone)]
pub struct RobustStrategy {
    pub finding_id: u64,
    pub domain: ReplicationDomain,
    pub description: String,
    pub replication_rate: f32,
    pub avg_effect_size: f32,
    pub environments_tested: usize,
    pub confidence: f32,
}

// ============================================================================
// REPLICATION STATS
// ============================================================================

/// Aggregate statistics for the replication engine
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ReplicationStats {
    pub total_replications: u64,
    pub successful_replications: u64,
    pub failed_replications: u64,
    pub partial_replications: u64,
    pub avg_replication_rate_ema: f32,
    pub avg_effect_ratio_ema: f32,
    pub robust_findings: u64,
    pub fragile_findings: u64,
    pub environments_tested: u64,
    pub findings_tracked: u64,
}

// ============================================================================
// COOPERATION REPLICATION
// ============================================================================

/// Replication engine for cooperation research findings
#[derive(Debug)]
pub struct CoopReplication {
    findings: BTreeMap<u64, ReplicableFinding>,
    environments: Vec<ReplicationEnvironment>,
    all_attempts: VecDeque<ReplicationAttempt>,
    robust_strategies: Vec<RobustStrategy>,
    domain_replication_rates: LinearMap<f32, 64>,
    rng_state: u64,
    tick: u64,
    stats: ReplicationStats,
}

impl CoopReplication {
    /// Create a new replication engine with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            environments: Vec::new(),
            all_attempts: VecDeque::new(),
            robust_strategies: Vec::new(),
            domain_replication_rates: LinearMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: ReplicationStats::default(),
        }
    }

    /// Register an environment for replication testing
    pub fn register_environment(&mut self, name: String, load: f32, subsystems: u32, contention: f32) -> u64 {
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let env = ReplicationEnvironment {
            id,
            name,
            load_level: load.min(1.0).max(0.0),
            subsystem_count: subsystems,
            contention_factor: contention.min(1.0).max(0.0),
            time_horizon: 10000,
            noise_level: xorshift_f32(&mut self.rng_state) * 0.1,
        };
        if self.environments.len() < MAX_ENVIRONMENTS {
            self.environments.push(env);
            self.stats.environments_tested = self.environments.len() as u64;
        }
        id
    }

    /// Attempt to replicate a cooperation finding
    pub fn replicate_cooperation_finding(
        &mut self,
        finding_id: u64,
        domain: ReplicationDomain,
        description: String,
        original_effect: f32,
        original_sample_size: usize,
    ) -> Vec<ReplicationAttempt> {
        self.tick += 1;
        // Register finding if new
        if !self.findings.contains_key(&finding_id) {
            if self.findings.len() >= MAX_FINDINGS {
                self.evict_oldest_finding();
            }
            let finding = ReplicableFinding {
                id: finding_id,
                domain,
                description: description.clone(),
                original_effect_size: original_effect,
                original_sample_size,
                replications: Vec::new(),
                replication_rate: 0.0,
                confidence: 0.5,
                robust: false,
                created_tick: self.tick,
            };
            self.findings.insert(finding_id, finding);
            self.stats.findings_tracked = self.findings.len() as u64;
        }

        let mut attempts: Vec<ReplicationAttempt> = Vec::new();
        let envs: Vec<ReplicationEnvironment> = self.environments.clone();
        for env in &envs {
            let attempt = self.run_replication(finding_id, env, original_effect);
            attempts.push(attempt);
        }
        // Update finding with replication results
        self.update_finding_replication(finding_id, &attempts);
        // Check if finding is now robust
        self.check_robust_status(finding_id);
        attempts
    }

    /// Match environments for replication — find similar environments
    pub fn environment_matching(&self, target_load: f32, target_contention: f32) -> Vec<&ReplicationEnvironment> {
        let mut matches: Vec<(&ReplicationEnvironment, f32)> = self
            .environments
            .iter()
            .map(|env| {
                let load_diff = (env.load_level - target_load).abs();
                let contention_diff = (env.contention_factor - target_contention).abs();
                let distance = load_diff + contention_diff;
                (env, distance)
            })
            .collect();
        matches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        matches.iter().map(|(env, _)| *env).collect()
    }

    /// Attempt fairness-specific replication
    pub fn fairness_replication(
        &mut self,
        finding_id: u64,
        original_fairness_effect: f32,
    ) -> Option<ReplicationAttempt> {
        self.tick += 1;
        if self.environments.is_empty() {
            return None;
        }
        let env_idx = (xorshift64(&mut self.rng_state) as usize) % self.environments.len();
        let env = self.environments[env_idx].clone();
        let attempt = self.run_replication(finding_id, &env, original_fairness_effect);
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            finding.replications.push(attempt.clone());
            self.recompute_replication_rate(finding_id);
        }
        Some(attempt)
    }

    /// Attempt trust model reproduction
    pub fn trust_reproduction(
        &mut self,
        finding_id: u64,
        original_trust_effect: f32,
    ) -> Option<ReplicationAttempt> {
        self.tick += 1;
        if self.environments.is_empty() {
            return None;
        }
        let env_idx = (xorshift64(&mut self.rng_state) as usize) % self.environments.len();
        let env = self.environments[env_idx].clone();
        let attempt = self.run_replication(finding_id, &env, original_trust_effect);
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            finding.replications.push(attempt.clone());
            self.recompute_replication_rate(finding_id);
        }
        Some(attempt)
    }

    /// Get the replication confidence for a finding
    #[inline]
    pub fn replication_confidence(&self, finding_id: u64) -> f32 {
        match self.findings.get(&finding_id) {
            Some(f) => f.confidence,
            None => 0.0,
        }
    }

    /// Get all robust strategies that have been successfully replicated
    #[inline(always)]
    pub fn robust_strategies(&self) -> &[RobustStrategy] {
        &self.robust_strategies
    }

    /// Get current replication statistics
    #[inline(always)]
    pub fn stats(&self) -> &ReplicationStats {
        &self.stats
    }

    /// Number of findings being tracked
    #[inline(always)]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// Total replication attempts across all findings
    #[inline(always)]
    pub fn total_attempts(&self) -> usize {
        self.all_attempts.len()
    }

    /// Get the replication rate for a specific domain
    #[inline]
    pub fn domain_replication_rate(&self, domain: ReplicationDomain) -> f32 {
        self.domain_replication_rates
            .get(&(domain as u64))
            .copied()
            .unwrap_or(0.0)
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    #[inline]
    fn run_replication(
        &mut self,
        finding_id: u64,
        env: &ReplicationEnvironment,
        original_effect: f32,
    ) -> ReplicationAttempt {
        // Simulate replication with environmental variation
        let noise = (xorshift_f32(&mut self.rng_state) - 0.5) * 2.0 * ENVIRONMENT_VARIATION;
        let env_modifier = 1.0 - env.contention_factor * 0.3 - env.noise_level;
        let replicated_effect = original_effect * env_modifier + noise * original_effect.abs();
        let effect_ratio = if original_effect.abs() > 0.001 {
            replicated_effect / original_effect
        } else {
            0.0
        };
        let outcome = if (effect_ratio - 1.0).abs() <= EFFECT_TOLERANCE {
            ReplicationOutcome::Replicated
        } else if effect_ratio > 0.5 && effect_ratio < 1.5 {
            ReplicationOutcome::PartialReplication
        } else if replicated_effect.abs() < 0.01 {
            ReplicationOutcome::FailedToReplicate
        } else {
            ReplicationOutcome::Inconclusive
        };
        let attempt_id = fnv1a_hash(&self.tick.to_le_bytes()) ^ fnv1a_hash(&env.id.to_le_bytes());
        let domain = self
            .findings
            .get(&finding_id)
            .map(|f| f.domain)
            .unwrap_or(ReplicationDomain::FairnessImprovement);
        let attempt = ReplicationAttempt {
            id: attempt_id,
            finding_id,
            environment_id: env.id,
            original_effect,
            replicated_effect,
            effect_ratio,
            outcome,
            tick: self.tick,
            domain,
        };
        self.stats.total_replications += 1;
        match outcome {
            ReplicationOutcome::Replicated => self.stats.successful_replications += 1,
            ReplicationOutcome::FailedToReplicate => self.stats.failed_replications += 1,
            ReplicationOutcome::PartialReplication => self.stats.partial_replications += 1,
            ReplicationOutcome::Inconclusive => {}
        }
        self.stats.avg_effect_ratio_ema =
            EMA_ALPHA * effect_ratio.abs() + (1.0 - EMA_ALPHA) * self.stats.avg_effect_ratio_ema;
        if self.all_attempts.len() >= MAX_REPLICATIONS {
            self.all_attempts.pop_front();
        }
        self.all_attempts.push_back(attempt.clone());
        attempt
    }

    fn update_finding_replication(&mut self, finding_id: u64, attempts: &[ReplicationAttempt]) {
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            for attempt in attempts {
                finding.replications.push(attempt.clone());
            }
        }
        self.recompute_replication_rate(finding_id);
    }

    #[inline]
    fn recompute_replication_rate(&mut self, finding_id: u64) {
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if finding.replications.is_empty() {
                finding.replication_rate = 0.0;
                return;
            }
            let replicated = finding
                .replications
                .iter()
                .filter(|a| matches!(a.outcome, ReplicationOutcome::Replicated))
                .count();
            let partial = finding
                .replications
                .iter()
                .filter(|a| matches!(a.outcome, ReplicationOutcome::PartialReplication))
                .count();
            let total = finding.replications.len() as f32;
            finding.replication_rate = (replicated as f32 + partial as f32 * 0.5) / total;
            // Update confidence based on replications
            let base_confidence = 0.5;
            let replication_bonus =
                CONFIDENCE_BOOST_PER_REPLICATION * replicated as f32;
            finding.confidence = (base_confidence + replication_bonus).min(1.0);
            // Update domain replication rate
            let domain_key = finding.domain as u64;
            let prev_rate = self
                .domain_replication_rates
                .get(&domain_key)
                .copied()
                .unwrap_or(0.5);
            let new_rate =
                EMA_ALPHA * finding.replication_rate + (1.0 - EMA_ALPHA) * prev_rate;
            self.domain_replication_rates.insert(domain_key, new_rate);
            self.stats.avg_replication_rate_ema =
                EMA_ALPHA * finding.replication_rate
                    + (1.0 - EMA_ALPHA) * self.stats.avg_replication_rate_ema;
        }
    }

    fn check_robust_status(&mut self, finding_id: u64) {
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if finding.replications.len() >= MIN_REPLICATIONS_REQUIRED {
                if finding.replication_rate >= ROBUST_THRESHOLD {
                    finding.robust = true;
                    self.stats.robust_findings += 1;
                    // Add to robust strategies
                    let avg_effect: f32 = if finding.replications.is_empty() {
                        finding.original_effect_size
                    } else {
                        finding
                            .replications
                            .iter()
                            .map(|a| a.replicated_effect)
                            .sum::<f32>()
                            / finding.replications.len() as f32
                    };
                    if self.robust_strategies.len() < MAX_ROBUST_STRATEGIES {
                        self.robust_strategies.push(RobustStrategy {
                            finding_id: finding.id,
                            domain: finding.domain,
                            description: finding.description.clone(),
                            replication_rate: finding.replication_rate,
                            avg_effect_size: avg_effect,
                            environments_tested: finding.replications.len(),
                            confidence: finding.confidence,
                        });
                    }
                } else if finding.replication_rate < FRAGILITY_THRESHOLD {
                    finding.robust = false;
                    self.stats.fragile_findings += 1;
                }
            }
        }
    }

    fn evict_oldest_finding(&mut self) {
        let oldest_id = self
            .findings
            .values()
            .min_by_key(|f| f.created_tick)
            .map(|f| f.id);
        if let Some(id) = oldest_id {
            self.findings.remove(&id);
        }
    }
}
