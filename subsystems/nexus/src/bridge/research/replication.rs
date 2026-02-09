// SPDX-License-Identifier: GPL-2.0
//! # Bridge Replication — Result Replication Engine
//!
//! A finding that cannot be replicated is not a finding. This module
//! implements systematic replication of bridge research results: for each
//! confirmed discovery, the engine re-runs the experiment under matching
//! conditions and compares the outcome. It tracks replication rates, detects
//! replication crises (when too many findings fail to replicate), identifies
//! robust findings, and generates comprehensive replication reports.
//!
//! Reproducibility is the bedrock of the bridge's self-research.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 1024;
const MAX_ATTEMPTS_PER_FINDING: usize = 32;
const MAX_HISTORY: usize = 4096;
const MATCH_THRESHOLD: f32 = 0.85;
const REPLICATION_SUCCESS_THRESHOLD: f32 = 0.70;
const CRISIS_THRESHOLD: f32 = 0.50;
const ROBUST_THRESHOLD: f32 = 0.90;
const CONDITION_TOLERANCE: f32 = 0.15;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIN_REPLICATIONS: usize = 3;
const CRISIS_WINDOW: usize = 50;

// ============================================================================
// HELPERS
// ============================================================================

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

fn abs_f32(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

// ============================================================================
// TYPES
// ============================================================================

/// An original finding to be replicated.
#[derive(Clone)]
struct OriginalFinding {
    id: u64,
    description: String,
    original_effect: f32,
    conditions: Vec<(String, f32)>,
    sample_size: usize,
    tick: u64,
    attempts: Vec<ReplicationAttemptInternal>,
    replication_rate: f32,
    robust: bool,
}

/// Internal replication attempt record.
#[derive(Clone)]
struct ReplicationAttemptInternal {
    attempt_id: u64,
    replicated_effect: f32,
    conditions_matched: bool,
    match_score: f32,
    replicated: bool,
    tick: u64,
}

/// Public replication attempt result.
#[derive(Clone)]
pub struct ReplicationAttempt {
    pub original_finding: u64,
    pub replicated: bool,
    pub conditions: Vec<(String, f32)>,
    pub match_score: f32,
    pub original_effect: f32,
    pub replicated_effect: f32,
    pub effect_deviation: f32,
}

/// Replication report for a finding.
#[derive(Clone)]
pub struct ReplicationReport {
    pub finding_id: u64,
    pub description: String,
    pub total_attempts: usize,
    pub successful_replications: usize,
    pub replication_rate: f32,
    pub avg_match_score: f32,
    pub is_robust: bool,
    pub avg_effect_deviation: f32,
    pub best_match_score: f32,
    pub worst_match_score: f32,
}

/// Replication engine statistics.
#[derive(Clone)]
#[repr(align(64))]
pub struct ReplicationStats {
    pub total_findings_tracked: u64,
    pub total_attempts: u64,
    pub successful_replications: u64,
    pub failed_replications: u64,
    pub overall_replication_rate_ema: f32,
    pub avg_match_score_ema: f32,
    pub robust_findings: u64,
    pub fragile_findings: u64,
    pub crisis_detected: bool,
    pub crisis_severity: f32,
}

/// Crisis analysis result.
#[derive(Clone)]
pub struct CrisisAnalysis {
    pub in_crisis: bool,
    pub recent_replication_rate: f32,
    pub failing_findings: Vec<u64>,
    pub severity: f32,
    pub recommendation: String,
}

// ============================================================================
// BRIDGE REPLICATION ENGINE
// ============================================================================

/// Result replication engine for bridge research.
#[repr(align(64))]
pub struct BridgeReplication {
    findings: BTreeMap<u64, OriginalFinding>,
    stats: ReplicationStats,
    recent_results: VecDeque<bool>, // recent replication successes
    rng_state: u64,
    tick: u64,
}

impl BridgeReplication {
    /// Create a new replication engine.
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            stats: ReplicationStats {
                total_findings_tracked: 0,
                total_attempts: 0,
                successful_replications: 0,
                failed_replications: 0,
                overall_replication_rate_ema: 0.5,
                avg_match_score_ema: 0.5,
                robust_findings: 0,
                fragile_findings: 0,
                crisis_detected: false,
                crisis_severity: 0.0,
            },
            recent_results: VecDeque::new(),
            rng_state: seed ^ 0x4E011CA010E001,
            tick: 0,
        }
    }

    /// Register an original finding for replication tracking.
    pub fn register_finding(
        &mut self,
        description: &str,
        original_effect: f32,
        conditions: &[(String, f32)],
        sample_size: usize,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;

        if self.findings.len() >= MAX_FINDINGS {
            self.evict_least_replicated();
        }

        let mut stored_conditions = Vec::new();
        for (name, val) in conditions {
            stored_conditions.push((name.clone(), *val));
        }

        self.findings.insert(
            id,
            OriginalFinding {
                id,
                description: String::from(description),
                original_effect,
                conditions: stored_conditions,
                sample_size,
                tick: self.tick,
                attempts: Vec::new(),
                replication_rate: 0.0,
                robust: false,
            },
        );
        self.stats.total_findings_tracked += 1;
        id
    }

    /// Attempt to replicate a finding.
    pub fn replicate_finding(
        &mut self,
        finding_id: u64,
        replicated_effect: f32,
        conditions: &[(String, f32)],
    ) -> ReplicationAttempt {
        self.tick += 1;
        self.stats.total_attempts += 1;

        let (original_effect, original_conditions, description) =
            match self.findings.get(&finding_id) {
                Some(f) => (f.original_effect, f.conditions.clone(), f.description.clone()),
                None => {
                    return ReplicationAttempt {
                        original_finding: finding_id,
                        replicated: false,
                        conditions: Vec::new(),
                        match_score: 0.0,
                        original_effect: 0.0,
                        replicated_effect,
                        effect_deviation: 1.0,
                    };
                }
            };

        // Check condition matching
        let conditions_match = self.conditions_match_internal(&original_conditions, conditions);

        // Compute effect match score
        let effect_deviation = if abs_f32(original_effect) > 1e-9 {
            abs_f32(replicated_effect - original_effect) / abs_f32(original_effect)
        } else {
            abs_f32(replicated_effect)
        };

        let effect_match = (1.0 - effect_deviation).max(0.0);
        let match_score = conditions_match * 0.4 + effect_match * 0.6;
        let replicated = match_score >= MATCH_THRESHOLD;

        // Record attempt
        let attempt_id = fnv1a_hash(description.as_bytes()) ^ self.tick ^ (self.stats.total_attempts as u64);
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if finding.attempts.len() < MAX_ATTEMPTS_PER_FINDING {
                finding.attempts.push(ReplicationAttemptInternal {
                    attempt_id,
                    replicated_effect,
                    conditions_matched: conditions_match >= MATCH_THRESHOLD,
                    match_score,
                    replicated,
                    tick: self.tick,
                });
            }
            // Update replication rate
            let total = finding.attempts.len();
            let successes = finding.attempts.iter().filter(|a| a.replicated).count();
            finding.replication_rate = if total > 0 {
                successes as f32 / total as f32
            } else {
                0.0
            };
            finding.robust =
                finding.replication_rate >= ROBUST_THRESHOLD && total >= MIN_REPLICATIONS;
        }

        // Update stats
        if replicated {
            self.stats.successful_replications += 1;
        } else {
            self.stats.failed_replications += 1;
        }
        self.stats.overall_replication_rate_ema = self.stats.overall_replication_rate_ema
            * (1.0 - EMA_ALPHA)
            + if replicated { 1.0 } else { 0.0 } * EMA_ALPHA;
        self.stats.avg_match_score_ema =
            self.stats.avg_match_score_ema * (1.0 - EMA_ALPHA) + match_score * EMA_ALPHA;

        // Track for crisis detection
        if self.recent_results.len() >= MAX_HISTORY {
            self.recent_results.pop_front();
        }
        self.recent_results.push_back(replicated);

        // Update robust/fragile counts
        self.update_robustness_counts();

        // Check for crisis
        self.check_crisis();

        let mut cond_out = Vec::new();
        for (n, v) in conditions {
            cond_out.push((n.clone(), *v));
        }

        ReplicationAttempt {
            original_finding: finding_id,
            replicated,
            conditions: cond_out,
            match_score,
            original_effect,
            replicated_effect,
            effect_deviation,
        }
    }

    /// Get the overall replication rate.
    #[inline]
    pub fn replication_rate(&self) -> f32 {
        if self.stats.total_attempts == 0 {
            return 0.0;
        }
        self.stats.successful_replications as f32 / self.stats.total_attempts as f32
    }

    /// Check if conditions match between original and replication.
    #[inline(always)]
    pub fn conditions_match(
        &self,
        original: &[(String, f32)],
        replication: &[(String, f32)],
    ) -> f32 {
        self.conditions_match_internal(original, replication)
    }

    /// Check for a replication crisis.
    pub fn replication_crisis_check(&self) -> CrisisAnalysis {
        let window = self.recent_results.len().min(CRISIS_WINDOW);
        if window == 0 {
            return CrisisAnalysis {
                in_crisis: false,
                recent_replication_rate: 0.0,
                failing_findings: Vec::new(),
                severity: 0.0,
                recommendation: String::from("insufficient data"),
            };
        }

        let recent = &self.recent_results[self.recent_results.len() - window..];
        let successes = recent.iter().filter(|&&r| r).count();
        let rate = successes as f32 / window as f32;
        let in_crisis = rate < CRISIS_THRESHOLD;

        let mut failing = Vec::new();
        for finding in self.findings.values() {
            if finding.attempts.len() >= MIN_REPLICATIONS
                && finding.replication_rate < CRISIS_THRESHOLD
            {
                failing.push(finding.id);
            }
        }

        let severity = if in_crisis {
            ((CRISIS_THRESHOLD - rate) / CRISIS_THRESHOLD).min(1.0)
        } else {
            0.0
        };

        let recommendation = if in_crisis && severity > 0.5 {
            String::from("CRITICAL: halt new research, audit methodology and measurement")
        } else if in_crisis {
            String::from("WARNING: increase sample sizes and tighten methodology checks")
        } else {
            String::from("OK: replication rates within acceptable bounds")
        };

        CrisisAnalysis {
            in_crisis,
            recent_replication_rate: rate,
            failing_findings: failing,
            severity,
            recommendation,
        }
    }

    /// Get all robust (reliably replicated) findings.
    #[inline]
    pub fn robust_findings(&self) -> Vec<u64> {
        self.findings
            .values()
            .filter(|f| f.robust)
            .map(|f| f.id)
            .collect()
    }

    /// Generate a replication report for a finding.
    pub fn replication_report(&self, finding_id: u64) -> ReplicationReport {
        match self.findings.get(&finding_id) {
            Some(f) => {
                let total = f.attempts.len();
                let successes = f.attempts.iter().filter(|a| a.replicated).count();
                let avg_match = if total > 0 {
                    f.attempts.iter().map(|a| a.match_score).sum::<f32>() / total as f32
                } else {
                    0.0
                };
                let avg_dev = if total > 0 {
                    let sum_dev: f32 = f
                        .attempts
                        .iter()
                        .map(|a| {
                            if abs_f32(f.original_effect) > 1e-9 {
                                abs_f32(a.replicated_effect - f.original_effect)
                                    / abs_f32(f.original_effect)
                            } else {
                                0.0
                            }
                        })
                        .sum();
                    sum_dev / total as f32
                } else {
                    0.0
                };
                let best = f
                    .attempts
                    .iter()
                    .map(|a| a.match_score)
                    .fold(0.0f32, |a, b| if b > a { b } else { a });
                let worst = f
                    .attempts
                    .iter()
                    .map(|a| a.match_score)
                    .fold(1.0f32, |a, b| if b < a { b } else { a });

                ReplicationReport {
                    finding_id,
                    description: f.description.clone(),
                    total_attempts: total,
                    successful_replications: successes,
                    replication_rate: f.replication_rate,
                    avg_match_score: avg_match,
                    is_robust: f.robust,
                    avg_effect_deviation: avg_dev,
                    best_match_score: best,
                    worst_match_score: if total > 0 { worst } else { 0.0 },
                }
            }
            None => ReplicationReport {
                finding_id,
                description: String::from("not_found"),
                total_attempts: 0,
                successful_replications: 0,
                replication_rate: 0.0,
                avg_match_score: 0.0,
                is_robust: false,
                avg_effect_deviation: 0.0,
                best_match_score: 0.0,
                worst_match_score: 0.0,
            },
        }
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ReplicationStats {
        &self.stats
    }

    /// Number of tracked findings.
    #[inline(always)]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn conditions_match_internal(
        &self,
        original: &[(String, f32)],
        replication: &[(String, f32)],
    ) -> f32 {
        if original.is_empty() && replication.is_empty() {
            return 1.0;
        }
        if original.is_empty() || replication.is_empty() {
            return 0.5;
        }

        let mut matched = 0u32;
        let mut total_deviation = 0.0f32;
        let total = original.len();

        for (name, val) in original {
            let name_hash = fnv1a_hash(name.as_bytes());
            let mut found = false;
            for (rname, rval) in replication {
                let rname_hash = fnv1a_hash(rname.as_bytes());
                if name_hash == rname_hash {
                    let dev = if abs_f32(*val) > 1e-9 {
                        abs_f32(rval - val) / abs_f32(*val)
                    } else {
                        abs_f32(*rval)
                    };
                    total_deviation += dev;
                    if dev <= CONDITION_TOLERANCE {
                        matched += 1;
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                total_deviation += 1.0;
            }
        }

        let presence_score = matched as f32 / total.max(1) as f32;
        let deviation_score =
            (1.0 - total_deviation / total.max(1) as f32).max(0.0);
        (presence_score * 0.6 + deviation_score * 0.4).max(0.0).min(1.0)
    }

    fn update_robustness_counts(&mut self) {
        let mut robust = 0u64;
        let mut fragile = 0u64;
        for f in self.findings.values() {
            if f.attempts.len() >= MIN_REPLICATIONS {
                if f.robust {
                    robust += 1;
                } else if f.replication_rate < CRISIS_THRESHOLD {
                    fragile += 1;
                }
            }
        }
        self.stats.robust_findings = robust;
        self.stats.fragile_findings = fragile;
    }

    fn check_crisis(&mut self) {
        let window = self.recent_results.len().min(CRISIS_WINDOW);
        if window < 10 {
            return;
        }
        let recent = &self.recent_results[self.recent_results.len() - window..];
        let rate = recent.iter().filter(|&&r| r).count() as f32 / window as f32;
        self.stats.crisis_detected = rate < CRISIS_THRESHOLD;
        self.stats.crisis_severity = if self.stats.crisis_detected {
            ((CRISIS_THRESHOLD - rate) / CRISIS_THRESHOLD).min(1.0)
        } else {
            0.0
        };
    }

    fn evict_least_replicated(&mut self) {
        let worst = self
            .findings
            .values()
            .min_by(|a, b| {
                a.replication_rate
                    .partial_cmp(&b.replication_rate)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|f| f.id);
        if let Some(wid) = worst {
            self.findings.remove(&wid);
        }
    }
}
