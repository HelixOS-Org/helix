// SPDX-License-Identifier: GPL-2.0
//! # Apps Replication — Replication of App Research Findings
//!
//! Ensures discovered app patterns are genuinely reproducible by running
//! independent replications under varying conditions. Each finding is
//! replicated multiple times and the match quality between original and
//! replication is assessed. Only findings that survive replication are
//! promoted to robust-pattern status.
//!
//! The engine that proves research results aren't just flukes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 512;
const MAX_REPLICATIONS_PER_FINDING: usize = 16;
const MAX_ROBUST_PATTERNS: usize = 256;
const MIN_REPLICATIONS: usize = 3;
const REPLICATION_SUCCESS_THRESHOLD: f32 = 0.70;
const MATCH_QUALITY_GOOD: f32 = 0.80;
const MATCH_QUALITY_FAIR: f32 = 0.60;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CONDITION_VARIATION: f32 = 0.15;
const MAX_REPORT_HISTORY: usize = 256;
const ROBUSTNESS_THRESHOLD: f32 = 0.75;

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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x * 0.5;
    for _ in 0..12 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

// ============================================================================
// TYPES
// ============================================================================

/// Status of a replication attempt.
#[derive(Clone, Copy, PartialEq)]
pub enum ReplicationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Inconclusive,
}

/// Match quality classification.
#[derive(Clone, Copy, PartialEq)]
pub enum MatchQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    NoMatch,
}

/// The original finding to be replicated.
#[derive(Clone)]
pub struct OriginalFinding {
    pub finding_id: u64,
    pub title: String,
    pub original_effect: f32,
    pub original_confidence: f32,
    pub original_conditions_hash: u64,
    pub registered_tick: u64,
}

/// A single replication attempt.
#[derive(Clone)]
pub struct ReplicationAttempt {
    pub attempt_id: u64,
    pub finding_id: u64,
    pub replicated_effect: f32,
    pub conditions_hash: u64,
    pub condition_variation: f32,
    pub status: ReplicationStatus,
    pub match_score: f32,
    pub match_quality: MatchQuality,
    pub attempt_tick: u64,
}

/// Conditions under which a replication is performed.
#[derive(Clone)]
pub struct ReplicationConditions {
    pub condition_id: u64,
    pub finding_id: u64,
    pub variation_factor: f32,
    pub parameters_changed: u32,
    pub environment_hash: u64,
    pub description: String,
}

/// A robust pattern that survived replication.
#[derive(Clone)]
pub struct RobustPattern {
    pub pattern_id: u64,
    pub finding_id: u64,
    pub title: String,
    pub replication_count: usize,
    pub success_count: usize,
    pub avg_match: f32,
    pub robustness_score: f32,
    pub condition_diversity: f32,
    pub promoted_tick: u64,
}

/// Replication report for a finding.
#[derive(Clone)]
pub struct ReplicationReport {
    pub finding_id: u64,
    pub title: String,
    pub total_attempts: usize,
    pub successful: usize,
    pub failed: usize,
    pub inconclusive: usize,
    pub replication_rate: f32,
    pub avg_match_quality: f32,
    pub is_robust: bool,
    pub report_tick: u64,
}

/// Engine-level stats.
#[derive(Clone)]
#[repr(align(64))]
pub struct ReplicationStats {
    pub findings_registered: u64,
    pub attempts_total: u64,
    pub attempts_succeeded: u64,
    pub attempts_failed: u64,
    pub robust_patterns_found: u64,
    pub ema_replication_rate: f32,
    pub ema_match_quality: f32,
    pub ema_robustness: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Replication engine for app research findings.
pub struct AppsReplication {
    findings: BTreeMap<u64, OriginalFinding>,
    attempts: BTreeMap<u64, Vec<ReplicationAttempt>>,
    conditions: BTreeMap<u64, Vec<ReplicationConditions>>,
    robust: BTreeMap<u64, RobustPattern>,
    report_history: VecDeque<ReplicationReport>,
    stats: ReplicationStats,
    rng_state: u64,
    tick: u64,
}

impl AppsReplication {
    /// Create a new replication engine.
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            attempts: BTreeMap::new(),
            conditions: BTreeMap::new(),
            robust: BTreeMap::new(),
            report_history: VecDeque::new(),
            stats: ReplicationStats {
                findings_registered: 0,
                attempts_total: 0,
                attempts_succeeded: 0,
                attempts_failed: 0,
                robust_patterns_found: 0,
                ema_replication_rate: 0.0,
                ema_match_quality: 0.0,
                ema_robustness: 0.0,
            },
            rng_state: seed ^ 0x56ce2d81fa394b07,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Register a finding and attempt replication with varied conditions.
    pub fn replicate_finding(
        &mut self,
        title: &str,
        original_effect: f32,
        original_confidence: f32,
        replicated_effect: f32,
    ) -> ReplicationAttempt {
        self.tick += 1;
        let finding_id = fnv1a_hash(title.as_bytes());

        // Register finding if new
        if !self.findings.contains_key(&finding_id) {
            self.stats.findings_registered += 1;
            let original = OriginalFinding {
                finding_id,
                title: String::from(title),
                original_effect,
                original_confidence,
                original_conditions_hash: fnv1a_hash(&finding_id.to_le_bytes()),
                registered_tick: self.tick,
            };
            if self.findings.len() >= MAX_FINDINGS {
                if let Some(oldest) = self.findings.keys().next().cloned() {
                    self.findings.remove(&oldest);
                    self.attempts.remove(&oldest);
                    self.conditions.remove(&oldest);
                }
            }
            self.findings.insert(finding_id, original);
        }

        // Create replication attempt
        self.stats.attempts_total += 1;
        let attempt_id = fnv1a_hash(&self.stats.attempts_total.to_le_bytes()) ^ self.tick;
        let cond_variation = xorshift_f32(&mut self.rng_state) * CONDITION_VARIATION;
        let cond_hash = xorshift64(&mut self.rng_state);

        // Compute match quality
        let diff = abs_f32(original_effect - replicated_effect);
        let max_mag = abs_f32(original_effect).max(abs_f32(replicated_effect)).max(0.01);
        let match_score = 1.0 - (diff / max_mag).min(1.0);

        let match_quality = if match_score >= 0.90 {
            MatchQuality::Excellent
        } else if match_score >= MATCH_QUALITY_GOOD {
            MatchQuality::Good
        } else if match_score >= MATCH_QUALITY_FAIR {
            MatchQuality::Fair
        } else if match_score >= 0.30 {
            MatchQuality::Poor
        } else {
            MatchQuality::NoMatch
        };

        let status = if match_score >= REPLICATION_SUCCESS_THRESHOLD {
            self.stats.attempts_succeeded += 1;
            ReplicationStatus::Succeeded
        } else if match_score >= MATCH_QUALITY_FAIR {
            ReplicationStatus::Inconclusive
        } else {
            self.stats.attempts_failed += 1;
            ReplicationStatus::Failed
        };

        let attempt = ReplicationAttempt {
            attempt_id,
            finding_id,
            replicated_effect,
            conditions_hash: cond_hash,
            condition_variation: cond_variation,
            status,
            match_score,
            match_quality,
            attempt_tick: self.tick,
        };

        let attempt_list = self.attempts.entry(finding_id).or_insert_with(Vec::new);
        if attempt_list.len() >= MAX_REPLICATIONS_PER_FINDING {
            attempt_list.pop_front();
        }
        attempt_list.push(attempt.clone());

        // Update EMAs
        let rate = self.stats.attempts_succeeded as f32 / self.stats.attempts_total.max(1) as f32;
        self.stats.ema_replication_rate =
            EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.ema_replication_rate;
        self.stats.ema_match_quality =
            EMA_ALPHA * match_score + (1.0 - EMA_ALPHA) * self.stats.ema_match_quality;

        // Check for promotion to robust pattern
        self.check_robustness(finding_id);

        attempt
    }

    /// Generate varied conditions for a replication.
    pub fn replication_conditions(&mut self, finding_id: u64, description: &str) -> Option<ReplicationConditions> {
        if !self.findings.contains_key(&finding_id) {
            return None;
        }
        self.tick += 1;
        let cond_id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        let variation = xorshift_f32(&mut self.rng_state) * CONDITION_VARIATION * 2.0;
        let params_changed = (xorshift64(&mut self.rng_state) % 5) as u32 + 1;

        let cond = ReplicationConditions {
            condition_id: cond_id,
            finding_id,
            variation_factor: variation,
            parameters_changed: params_changed,
            environment_hash: xorshift64(&mut self.rng_state),
            description: String::from(description),
        };

        let cond_list = self.conditions.entry(finding_id).or_insert_with(Vec::new);
        if cond_list.len() >= MAX_REPLICATIONS_PER_FINDING {
            cond_list.pop_front();
        }
        cond_list.push(cond.clone());
        Some(cond)
    }

    /// Compute the match quality between original and replicated results.
    pub fn match_quality(&self, finding_id: u64) -> Option<(f32, MatchQuality)> {
        let attempts = self.attempts.get(&finding_id)?;
        if attempts.is_empty() {
            return None;
        }

        let mut total_match = 0.0f32;
        for attempt in attempts {
            total_match += attempt.match_score;
        }
        let avg = total_match / attempts.len() as f32;

        let quality = if avg >= 0.90 {
            MatchQuality::Excellent
        } else if avg >= MATCH_QUALITY_GOOD {
            MatchQuality::Good
        } else if avg >= MATCH_QUALITY_FAIR {
            MatchQuality::Fair
        } else if avg >= 0.30 {
            MatchQuality::Poor
        } else {
            MatchQuality::NoMatch
        };

        Some((avg, quality))
    }

    /// Compute the overall replication rate for a specific finding.
    #[inline]
    pub fn replication_rate(&self, finding_id: u64) -> Option<f32> {
        let attempts = self.attempts.get(&finding_id)?;
        if attempts.is_empty() {
            return Some(0.0);
        }
        let success_count = attempts
            .iter()
            .filter(|a| a.status == ReplicationStatus::Succeeded)
            .count();
        Some(success_count as f32 / attempts.len() as f32)
    }

    /// Get all robust patterns that have survived replication.
    #[inline(always)]
    pub fn robust_patterns(&self) -> Vec<RobustPattern> {
        self.robust.values().cloned().collect()
    }

    /// Generate a replication report for a finding.
    pub fn replication_report(&mut self, finding_id: u64) -> Option<ReplicationReport> {
        let finding = self.findings.get(&finding_id)?;
        let attempts = self.attempts.get(&finding_id)?;

        let total = attempts.len();
        let mut success = 0usize;
        let mut fail = 0usize;
        let mut inconclusive = 0usize;
        let mut match_sum = 0.0f32;

        for attempt in attempts {
            match attempt.status {
                ReplicationStatus::Succeeded => success += 1,
                ReplicationStatus::Failed => fail += 1,
                ReplicationStatus::Inconclusive => inconclusive += 1,
                _ => {}
            }
            match_sum += attempt.match_score;
        }

        let rep_rate = if total > 0 {
            success as f32 / total as f32
        } else {
            0.0
        };
        let avg_match = if total > 0 {
            match_sum / total as f32
        } else {
            0.0
        };
        let is_robust = rep_rate >= ROBUSTNESS_THRESHOLD && total >= MIN_REPLICATIONS;

        let report = ReplicationReport {
            finding_id,
            title: finding.title.clone(),
            total_attempts: total,
            successful: success,
            failed: fail,
            inconclusive,
            replication_rate: rep_rate,
            avg_match_quality: avg_match,
            is_robust,
            report_tick: self.tick,
        };

        if self.report_history.len() >= MAX_REPORT_HISTORY {
            self.report_history.pop_front();
        }
        self.report_history.push_back(report.clone());
        Some(report)
    }

    /// Return engine stats.
    #[inline(always)]
    pub fn stats(&self) -> &ReplicationStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    #[inline]
    fn check_robustness(&mut self, finding_id: u64) {
        let attempts = match self.attempts.get(&finding_id) {
            Some(a) => a,
            None => return,
        };
        let finding = match self.findings.get(&finding_id) {
            Some(f) => f,
            None => return,
        };

        if attempts.len() < MIN_REPLICATIONS {
            return;
        }

        let total = attempts.len();
        let success = attempts
            .iter()
            .filter(|a| a.status == ReplicationStatus::Succeeded)
            .count();
        let rate = success as f32 / total as f32;

        if rate < ROBUSTNESS_THRESHOLD {
            return;
        }

        let mut match_sum = 0.0f32;
        let mut cond_hashes: Vec<u64> = Vec::new();
        for attempt in attempts {
            match_sum += attempt.match_score;
            if !cond_hashes.contains(&attempt.conditions_hash) {
                cond_hashes.push(attempt.conditions_hash);
            }
        }
        let avg_match = match_sum / total as f32;
        let cond_diversity = cond_hashes.len() as f32 / total as f32;
        let robustness = rate * 0.5 + avg_match * 0.3 + cond_diversity * 0.2;

        self.stats.ema_robustness =
            EMA_ALPHA * robustness + (1.0 - EMA_ALPHA) * self.stats.ema_robustness;

        if robustness >= ROBUSTNESS_THRESHOLD && !self.robust.contains_key(&finding_id) {
            self.stats.robust_patterns_found += 1;
            let pattern = RobustPattern {
                pattern_id: finding_id,
                finding_id,
                title: finding.title.clone(),
                replication_count: total,
                success_count: success,
                avg_match,
                robustness_score: robustness,
                condition_diversity: cond_diversity,
                promoted_tick: self.tick,
            };

            if self.robust.len() >= MAX_ROBUST_PATTERNS {
                let mut min_id = 0u64;
                let mut min_rob = f32::MAX;
                for (rid, r) in self.robust.iter() {
                    if r.robustness_score < min_rob {
                        min_rob = r.robustness_score;
                        min_id = *rid;
                    }
                }
                self.robust.remove(&min_id);
            }
            self.robust.insert(finding_id, pattern);
        }
    }
}
