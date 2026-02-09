// SPDX-License-Identifier: GPL-2.0
//! # Apps Conscience
//!
//! Fairness framework for application resource allocation. The conscience
//! module acts as an ethical guardian within the apps consciousness layer,
//! ensuring that:
//!
//! - No application is starved of resources indefinitely
//! - Priority classes are respected proportionally
//! - SLA commitments are honored with quantified compliance
//! - Resource allocation decisions are auditable and justified
//!
//! The conscience computes a composite **conscience score** for the overall
//! system and per-app **fairness indices**. It detects starvation patterns
//! (an app receiving < threshold resources for > threshold duration) and
//! raises alerts. SLA compliance is tracked as a running EMA of whether
//! each app's guarantees are met each tick.
//!
//! This module provides the moral compass for resource allocation — the
//! engine's ability to evaluate whether its decisions are *fair*.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_APPS: usize = 1024;
const STARVATION_THRESHOLD: f32 = 0.1;
const STARVATION_DURATION: u64 = 50;
const SLA_DEFAULT: f32 = 0.9;
const PRIORITY_CLASSES: usize = 8;
const MAX_VIOLATION_LOG: usize = 256;
const CONSCIENCE_DECAY: f32 = 0.998;
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

/// Xorshift64 PRNG for tie-breaking in allocation
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// FAIRNESS TYPES
// ============================================================================

/// Per-app fairness record
#[derive(Debug, Clone)]
pub struct AppFairness {
    pub app_id: u64,
    pub app_name: String,
    pub priority_class: u8,
    pub sla_target: f32,
    pub sla_compliance: f32,
    pub resource_share: f32,
    pub expected_share: f32,
    pub fairness_index: f32,
    pub starved: bool,
    pub starved_ticks: u64,
    pub total_ticks: u64,
    pub violations: u64,
    pub last_tick: u64,
    compliance_history: Vec<f32>,
    write_idx: usize,
}

impl AppFairness {
    fn new(app_id: u64, app_name: String, priority_class: u8, sla_target: f32) -> Self {
        Self {
            app_id,
            app_name,
            priority_class,
            sla_target,
            sla_compliance: 1.0,
            resource_share: 0.0,
            expected_share: 0.0,
            fairness_index: 1.0,
            starved: false,
            starved_ticks: 0,
            total_ticks: 0,
            violations: 0,
            last_tick: 0,
            compliance_history: Vec::new(),
            write_idx: 0,
        }
    }

    #[inline]
    fn update(
        &mut self,
        resource_share: f32,
        expected_share: f32,
        sla_met: bool,
        tick: u64,
    ) {
        self.total_ticks += 1;
        self.last_tick = tick;
        self.resource_share = resource_share;
        self.expected_share = expected_share;

        // SLA compliance EMA
        let sla_raw = if sla_met { 1.0_f32 } else { 0.0 };
        self.sla_compliance =
            EMA_ALPHA * sla_raw + (1.0 - EMA_ALPHA) * self.sla_compliance;

        if !sla_met {
            self.violations += 1;
        }

        // Fairness index: ratio of actual to expected share
        if expected_share > 0.001 {
            let ratio = resource_share / expected_share;
            self.fairness_index =
                EMA_ALPHA * ratio.min(2.0) + (1.0 - EMA_ALPHA) * self.fairness_index;
        }

        // Starvation detection
        if resource_share < STARVATION_THRESHOLD {
            self.starved_ticks += 1;
            if self.starved_ticks >= STARVATION_DURATION {
                self.starved = true;
            }
        } else {
            self.starved_ticks = 0;
            self.starved = false;
        }

        // History ring
        if self.compliance_history.len() < 128 {
            self.compliance_history.push(sla_raw);
        } else {
            self.compliance_history[self.write_idx] = sla_raw;
        }
        self.write_idx = (self.write_idx + 1) % 128;
    }

    fn compliance_trend(&self) -> f32 {
        if self.compliance_history.len() < 4 {
            return 0.0;
        }
        let len = self.compliance_history.len();
        let mid = len / 2;
        let first: f32 =
            self.compliance_history[..mid].iter().sum::<f32>() / mid as f32;
        let second: f32 =
            self.compliance_history[mid..].iter().sum::<f32>() / (len - mid) as f32;
        second - first
    }
}

/// A logged fairness violation
#[derive(Debug, Clone)]
pub struct FairnessViolation {
    pub tick: u64,
    pub app_id: u64,
    pub violation_type: String,
    pub severity: f32,
    pub details: String,
}

/// Per-priority-class fairness aggregate
#[derive(Debug, Clone)]
pub struct PriorityClassFairness {
    pub priority_class: u8,
    pub app_count: usize,
    pub mean_share: f32,
    pub mean_sla_compliance: f32,
    pub starved_count: usize,
}

// ============================================================================
// STATS
// ============================================================================

/// Overall conscience statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConscienceStats {
    pub total_apps: usize,
    pub conscience_score: f32,
    pub mean_fairness: f32,
    pub mean_sla_compliance: f32,
    pub starved_app_count: usize,
    pub total_violations: u64,
    pub violation_rate: f32,
    pub priority_fairness: Vec<PriorityClassFairness>,
}

// ============================================================================
// APPS CONSCIENCE
// ============================================================================

/// Fairness framework for resource allocation
#[derive(Debug)]
pub struct AppsConscience {
    apps: BTreeMap<u64, AppFairness>,
    violations: Vec<FairnessViolation>,
    viol_write_idx: usize,
    tick: u64,
    total_evaluations: u64,
    total_violations: u64,
    conscience_score: f32,
    rng_state: u64,
}

impl AppsConscience {
    pub fn new(seed: u64) -> Self {
        Self {
            apps: BTreeMap::new(),
            violations: Vec::new(),
            viol_write_idx: 0,
            tick: 0,
            total_evaluations: 0,
            total_violations: 0,
            conscience_score: 1.0,
            rng_state: if seed == 0 { 0xC05C_CAFE_1234_5678 } else { seed },
        }
    }

    /// Register an app with its priority class and SLA target
    pub fn register_app(
        &mut self,
        app_id: u64,
        app_name: &str,
        priority_class: u8,
        sla_target: f32,
    ) {
        let sla = if sla_target <= 0.0 || sla_target > 1.0 {
            SLA_DEFAULT
        } else {
            sla_target
        };
        let pclass = priority_class.min((PRIORITY_CLASSES - 1) as u8);

        self.apps.entry(app_id).or_insert_with(|| {
            AppFairness::new(app_id, String::from(app_name), pclass, sla)
        });

        if self.apps.len() > MAX_APPS {
            self.evict_oldest(app_id);
        }
    }

    /// Perform a fairness check on an app's resource allocation
    pub fn fairness_check(
        &mut self,
        app_id: u64,
        resource_share: f32,
        expected_share: f32,
        sla_met: bool,
    ) -> f32 {
        self.tick += 1;
        self.total_evaluations += 1;

        if let Some(app) = self.apps.get_mut(&app_id) {
            app.update(resource_share, expected_share, sla_met, self.tick);

            if !sla_met {
                self.total_violations += 1;
                self.log_violation(
                    app_id,
                    String::from("sla_breach"),
                    1.0 - app.sla_compliance,
                    String::from("SLA target not met"),
                );
            }

            if app.starved {
                self.log_violation(
                    app_id,
                    String::from("starvation"),
                    1.0,
                    String::from("App starved of resources"),
                );
            }

            self.recompute_conscience();
            app.fairness_index
        } else {
            0.0
        }
    }

    /// Check SLA compliance for a specific app
    #[inline(always)]
    pub fn sla_compliance(&self, app_id: u64) -> Option<(f32, f32)> {
        let app = self.apps.get(&app_id)?;
        Some((app.sla_compliance, app.compliance_trend()))
    }

    /// Detect all currently starved apps
    #[inline]
    pub fn starvation_detection(&self) -> Vec<(u64, u64)> {
        let mut starved = Vec::new();
        for (id, app) in &self.apps {
            if app.starved {
                starved.push((*id, app.starved_ticks));
            }
        }
        starved.sort_by(|a, b| b.1.cmp(&a.1));
        starved
    }

    /// Check whether priorities are being respected
    pub fn priority_respect(&self) -> Vec<PriorityClassFairness> {
        let mut by_class: BTreeMap<u8, Vec<&AppFairness>> = BTreeMap::new();
        for (_, app) in &self.apps {
            by_class.entry(app.priority_class).or_insert_with(Vec::new).push(app);
        }

        let mut result = Vec::new();
        for (pclass, apps) in &by_class {
            let n = apps.len().max(1) as f32;
            let mean_share: f32 = apps.iter().map(|a| a.resource_share).sum::<f32>() / n;
            let mean_sla: f32 = apps.iter().map(|a| a.sla_compliance).sum::<f32>() / n;
            let starved: usize = apps.iter().filter(|a| a.starved).count();

            result.push(PriorityClassFairness {
                priority_class: *pclass,
                app_count: apps.len(),
                mean_share,
                mean_sla_compliance: mean_sla,
                starved_count: starved,
            });
        }

        // Verify monotonicity: higher priority should get >= share
        // (this is informational — the data is returned for the caller to evaluate)
        result.sort_by(|a, b| a.priority_class.cmp(&b.priority_class));
        result
    }

    /// Compute the overall conscience score (0.0 = terrible, 1.0 = perfect)
    #[inline(always)]
    pub fn conscience_score(&self) -> f32 {
        self.conscience_score
    }

    /// Recommend an ethical allocation adjustment for a starved app
    #[inline]
    pub fn ethical_allocation(&self, app_id: u64) -> Option<f32> {
        let app = self.apps.get(&app_id)?;
        if !app.starved {
            return None;
        }
        // Recommend at least the expected share
        let recommended = app.expected_share.max(STARVATION_THRESHOLD * 2.0);
        Some(recommended)
    }

    /// Get the fairness record for a specific app
    #[inline(always)]
    pub fn app_fairness(&self, app_id: u64) -> Option<&AppFairness> {
        self.apps.get(&app_id)
    }

    /// Get all violations
    #[inline(always)]
    pub fn violation_log(&self) -> &[FairnessViolation] {
        &self.violations
    }

    /// Full stats
    pub fn stats(&self) -> ConscienceStats {
        let n = self.apps.len().max(1) as f32;
        let mut fairness_sum = 0.0_f32;
        let mut sla_sum = 0.0_f32;
        let mut starved = 0usize;

        for (_, app) in &self.apps {
            fairness_sum += app.fairness_index;
            sla_sum += app.sla_compliance;
            if app.starved {
                starved += 1;
            }
        }

        let violation_rate = if self.total_evaluations > 0 {
            self.total_violations as f32 / self.total_evaluations as f32
        } else {
            0.0
        };

        ConscienceStats {
            total_apps: self.apps.len(),
            conscience_score: self.conscience_score,
            mean_fairness: fairness_sum / n,
            mean_sla_compliance: sla_sum / n,
            starved_app_count: starved,
            total_violations: self.total_violations,
            violation_rate,
            priority_fairness: self.priority_respect(),
        }
    }

    /// Decay conscience tracking
    #[inline]
    pub fn decay(&mut self) {
        for (_, app) in self.apps.iter_mut() {
            app.sla_compliance *= CONSCIENCE_DECAY;
            if app.starved_ticks > 0 && !app.starved {
                app.starved_ticks = app.starved_ticks.saturating_sub(1);
            }
        }
        self.recompute_conscience();
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    #[inline]
    fn recompute_conscience(&mut self) {
        if self.apps.is_empty() {
            self.conscience_score = 1.0;
            return;
        }

        let n = self.apps.len() as f32;
        let mut fairness_sum = 0.0_f32;
        let mut sla_sum = 0.0_f32;
        let mut starved_penalty = 0.0_f32;

        for (_, app) in &self.apps {
            fairness_sum += app.fairness_index.min(1.0);
            sla_sum += app.sla_compliance;
            if app.starved {
                starved_penalty += 0.1;
            }
        }

        let mean_fairness = fairness_sum / n;
        let mean_sla = sla_sum / n;
        let raw = 0.4 * mean_fairness + 0.4 * mean_sla - starved_penalty;
        self.conscience_score =
            EMA_ALPHA * raw.clamp(0.0, 1.0)
                + (1.0 - EMA_ALPHA) * self.conscience_score;
    }

    fn log_violation(
        &mut self,
        app_id: u64,
        violation_type: String,
        severity: f32,
        details: String,
    ) {
        let viol = FairnessViolation {
            tick: self.tick,
            app_id,
            violation_type,
            severity,
            details,
        };

        if self.violations.len() < MAX_VIOLATION_LOG {
            self.violations.push(viol);
        } else {
            self.violations[self.viol_write_idx] = viol;
        }
        self.viol_write_idx = (self.viol_write_idx + 1) % MAX_VIOLATION_LOG;
    }

    fn evict_oldest(&mut self, keep_id: u64) {
        let mut oldest_tick = u64::MAX;
        let mut oldest_id = 0u64;
        for (id, app) in &self.apps {
            if *id != keep_id && app.last_tick < oldest_tick {
                oldest_tick = app.last_tick;
                oldest_id = *id;
            }
        }
        if oldest_id != 0 {
            self.apps.remove(&oldest_id);
        }
    }
}
