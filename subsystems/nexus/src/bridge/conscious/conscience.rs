// SPDX-License-Identifier: GPL-2.0
//! # Bridge Conscience
//!
//! Ethical decision framework for bridge operations. Ensures the bridge
//! makes "fair" decisions by enforcing principles:
//!
//! - **No process starvation** — every process gets its fair share
//! - **No priority inversion** — high-priority work is never blocked by low
//! - **No resource hoarding** — no single consumer monopolizes resources
//! - **Proportional allocation** — resources track declared priority
//! - **Transparency** — decisions can be audited and justified
//!
//! Every bridge decision passes through the conscience check. Violations
//! are tracked, weighted, and used to adjust future behaviour.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PRINCIPLES: usize = 32;
const MAX_PROCESS_TRACKING: usize = 256;
const MAX_VIOLATION_HISTORY: usize = 512;
const MAX_AUDIT_LOG: usize = 256;
const STARVATION_THRESHOLD_TICKS: u64 = 200;
const HOARDING_THRESHOLD_RATIO: f32 = 0.50;
const FAIRNESS_EMA_ALPHA: f32 = 0.08;
const VIOLATION_DECAY_RATE: f32 = 0.01;
const PRIORITY_INVERSION_PENALTY: f32 = 0.20;
const STARVATION_PENALTY: f32 = 0.15;
const HOARDING_PENALTY: f32 = 0.10;
const CONSCIENCE_CLEAR_THRESHOLD: f32 = 0.80;
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

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

// ============================================================================
// CONSCIENCE RULE
// ============================================================================

/// A principle enforced by the conscience
#[derive(Debug, Clone)]
pub struct ConscienceRule {
    pub principle: String,
    pub principle_hash: u64,
    pub weight: f32,
    pub violation_count: u64,
    pub last_violation_tick: Option<u64>,
    pub enabled: bool,
}

impl ConscienceRule {
    fn new(principle: &str, weight: f32) -> Self {
        Self {
            principle: String::from(principle),
            principle_hash: fnv1a_hash(principle.as_bytes()),
            weight: weight.clamp(0.0, 1.0),
            violation_count: 0,
            last_violation_tick: None,
            enabled: true,
        }
    }

    fn record_violation(&mut self, tick: u64) {
        self.violation_count += 1;
        self.last_violation_tick = Some(tick);
    }
}

// ============================================================================
// PROCESS TRACKING
// ============================================================================

/// Tracking data for a process's resource consumption
#[derive(Debug, Clone)]
pub struct ProcessTracker {
    pub process_id: u64,
    pub process_name: String,
    pub priority: u32,
    pub last_served_tick: u64,
    pub total_served: u64,
    pub resource_share: f32,
    pub consecutive_denials: u32,
}

impl ProcessTracker {
    fn new(pid: u64, name: &str, priority: u32, tick: u64) -> Self {
        Self {
            process_id: pid,
            process_name: String::from(name),
            priority,
            last_served_tick: tick,
            total_served: 0,
            resource_share: 0.0,
            consecutive_denials: 0,
        }
    }

    fn serve(&mut self, tick: u64) {
        self.last_served_tick = tick;
        self.total_served += 1;
        self.consecutive_denials = 0;
    }

    fn deny(&mut self) {
        self.consecutive_denials += 1;
    }

    fn ticks_since_served(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_served_tick)
    }

    fn is_starving(&self, current_tick: u64) -> bool {
        self.ticks_since_served(current_tick) > STARVATION_THRESHOLD_TICKS
    }
}

// ============================================================================
// VIOLATION
// ============================================================================

/// A specific violation record
#[derive(Debug, Clone)]
pub struct Violation {
    pub principle_hash: u64,
    pub description: String,
    pub severity: f32,
    pub tick: u64,
    pub process_id: Option<u64>,
}

// ============================================================================
// ETHICAL CHECK RESULT
// ============================================================================

/// Result of an ethical check on a proposed action
#[derive(Debug, Clone)]
pub struct EthicalCheckResult {
    pub approved: bool,
    pub fairness_score: f32,
    pub violations_detected: Vec<String>,
    pub recommendations: Vec<String>,
}

// ============================================================================
// AUDIT LOG ENTRY
// ============================================================================

#[derive(Debug, Clone)]
struct AuditEntry {
    tick: u64,
    action_hash: u64,
    approved: bool,
    fairness_score: f32,
    violations: usize,
}

// ============================================================================
// STATS
// ============================================================================

/// Conscience statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConscienceStats {
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_approvals: u64,
    pub total_rejections: u64,
    pub avg_fairness: f32,
    pub active_starvation_count: usize,
    pub priority_inversions: u64,
    pub conscience_score: f32,
}

// ============================================================================
// BRIDGE CONSCIENCE
// ============================================================================

/// Ethical decision framework for bridge operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeConscience {
    principles: BTreeMap<u64, ConscienceRule>,
    processes: BTreeMap<u64, ProcessTracker>,
    violations: VecDeque<Violation>,
    audit_log: VecDeque<AuditEntry>,
    current_tick: u64,
    total_checks: u64,
    total_approvals: u64,
    total_rejections: u64,
    total_violations: u64,
    priority_inversions: u64,
    fairness_ema: f32,
    total_resources_allocated: u64,
}

impl BridgeConscience {
    /// Create a new conscience with default ethical principles
    pub fn new() -> Self {
        let mut conscience = Self {
            principles: BTreeMap::new(),
            processes: BTreeMap::new(),
            violations: VecDeque::new(),
            audit_log: VecDeque::new(),
            current_tick: 0,
            total_checks: 0,
            total_approvals: 0,
            total_rejections: 0,
            total_violations: 0,
            priority_inversions: 0,
            fairness_ema: 1.0,
            total_resources_allocated: 0,
        };
        conscience.install_default_principles();
        conscience
    }

    fn install_default_principles(&mut self) {
        let defaults = [
            ("no_starvation", 0.9),
            ("no_priority_inversion", 0.85),
            ("no_resource_hoarding", 0.80),
            ("proportional_allocation", 0.75),
            ("transparency", 0.70),
        ];
        for (name, weight) in defaults {
            let rule = ConscienceRule::new(name, weight);
            self.principles.insert(rule.principle_hash, rule);
        }
    }

    /// Register a process for tracking
    #[inline]
    pub fn register_process(&mut self, pid: u64, name: &str, priority: u32) {
        self.current_tick += 1;
        if self.processes.len() < MAX_PROCESS_TRACKING {
            let tracker = ProcessTracker::new(pid, name, priority, self.current_tick);
            self.processes.insert(pid, tracker);
        }
    }

    /// Record that a process was served
    #[inline]
    pub fn record_service(&mut self, pid: u64) {
        self.current_tick += 1;
        self.total_resources_allocated += 1;
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.serve(self.current_tick);
        }
    }

    /// Record that a process was denied service
    #[inline]
    pub fn record_denial(&mut self, pid: u64) {
        self.current_tick += 1;
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.deny();
        }
    }

    /// Perform an ethical check on a proposed action
    pub fn ethical_check(&mut self, action: &str, beneficiary_pid: u64) -> EthicalCheckResult {
        self.current_tick += 1;
        self.total_checks += 1;

        let mut violations_detected = Vec::new();
        let mut recommendations = Vec::new();

        // Check for starvation
        let starving: Vec<u64> = self
            .processes
            .iter()
            .filter(|(_, p)| p.is_starving(self.current_tick))
            .map(|(&pid, _)| pid)
            .collect();

        if !starving.is_empty() {
            let is_starving_beneficiary = starving.contains(&beneficiary_pid);
            if !is_starving_beneficiary {
                violations_detected.push(String::from("starvation_risk"));
                recommendations.push(String::from("serve_starving_process_first"));
                self.record_violation_internal("no_starvation", STARVATION_PENALTY, None);
            }
        }

        // Check for priority inversion
        if let Some(beneficiary) = self.processes.get(&beneficiary_pid) {
            let beneficiary_priority = beneficiary.priority;
            for (_, proc) in &self.processes {
                if proc.priority > beneficiary_priority && proc.is_starving(self.current_tick) {
                    violations_detected.push(String::from("priority_inversion"));
                    recommendations.push(String::from("serve_higher_priority_first"));
                    self.priority_inversions += 1;
                    self.record_violation_internal(
                        "no_priority_inversion",
                        PRIORITY_INVERSION_PENALTY,
                        Some(proc.process_id),
                    );
                    break;
                }
            }
        }

        // Check for resource hoarding
        if self.total_resources_allocated > 0 {
            if let Some(beneficiary) = self.processes.get(&beneficiary_pid) {
                let share =
                    beneficiary.total_served as f32 / self.total_resources_allocated as f32;
                if share > HOARDING_THRESHOLD_RATIO {
                    violations_detected.push(String::from("resource_hoarding"));
                    recommendations.push(String::from("distribute_resources_more_evenly"));
                    self.record_violation_internal(
                        "no_resource_hoarding",
                        HOARDING_PENALTY,
                        Some(beneficiary_pid),
                    );
                }
            }
        }

        let fairness = self.compute_fairness();
        self.fairness_ema = ema_update(self.fairness_ema, fairness, FAIRNESS_EMA_ALPHA);

        let approved = violations_detected.is_empty() || fairness > CONSCIENCE_CLEAR_THRESHOLD;
        if approved {
            self.total_approvals += 1;
        } else {
            self.total_rejections += 1;
        }

        // Audit log
        let action_hash = fnv1a_hash(action.as_bytes());
        if self.audit_log.len() >= MAX_AUDIT_LOG {
            self.audit_log.pop_front();
        }
        self.audit_log.push_back(AuditEntry {
            tick: self.current_tick,
            action_hash,
            approved,
            fairness_score: fairness,
            violations: violations_detected.len(),
        });

        EthicalCheckResult {
            approved,
            fairness_score: fairness,
            violations_detected,
            recommendations,
        }
    }

    fn record_violation_internal(
        &mut self,
        principle: &str,
        severity: f32,
        process_id: Option<u64>,
    ) {
        let hash = fnv1a_hash(principle.as_bytes());

        if let Some(rule) = self.principles.get_mut(&hash) {
            rule.record_violation(self.current_tick);
        }

        if self.violations.len() >= MAX_VIOLATION_HISTORY {
            self.violations.pop_front();
        }

        self.violations.push_back(Violation {
            principle_hash: hash,
            description: String::from(principle),
            severity,
            tick: self.current_tick,
            process_id,
        });

        self.total_violations += 1;
    }

    /// Compute overall fairness score (0.0 = completely unfair, 1.0 = perfectly fair)
    #[inline(always)]
    pub fn fairness_score(&self) -> f32 {
        self.fairness_ema
    }

    fn compute_fairness(&self) -> f32 {
        if self.processes.is_empty() || self.total_resources_allocated == 0 {
            return 1.0;
        }

        // Gini coefficient-inspired fairness
        let shares: Vec<f32> = self
            .processes
            .values()
            .map(|p| p.total_served as f32 / self.total_resources_allocated as f32)
            .collect();

        let n = shares.len() as f32;
        let mean = shares.iter().sum::<f32>() / n;
        if mean == 0.0 {
            return 1.0;
        }

        let mut sum_abs_diff = 0.0;
        for i in 0..shares.len() {
            for j in 0..shares.len() {
                sum_abs_diff += (shares[i] - shares[j]).abs();
            }
        }

        let gini = sum_abs_diff / (2.0 * n * n * mean);
        (1.0 - gini).clamp(0.0, 1.0)
    }

    /// Detect processes currently being starved
    #[inline]
    pub fn detect_starvation(&self) -> Vec<(u64, String, u64)> {
        self.processes
            .iter()
            .filter(|(_, p)| p.is_starving(self.current_tick))
            .map(|(&pid, p)| (pid, p.process_name.clone(), p.ticks_since_served(self.current_tick)))
            .collect()
    }

    /// Check for priority inversions in current state
    pub fn priority_inversion_check(&self) -> Vec<(u64, u64)> {
        let mut inversions = Vec::new();

        let mut sorted: Vec<&ProcessTracker> = self.processes.values().collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let high = sorted[i];
                let low = sorted[j];
                // Priority inversion: high-priority is starving while low-priority is being served
                if high.is_starving(self.current_tick)
                    && !low.is_starving(self.current_tick)
                    && high.priority > low.priority
                {
                    inversions.push((high.process_id, low.process_id));
                }
            }
        }

        inversions
    }

    /// Generate a conscience report
    #[inline]
    pub fn conscience_report(&self) -> Vec<(String, u64, f32)> {
        self.principles
            .values()
            .map(|r| (r.principle.clone(), r.violation_count, r.weight))
            .collect()
    }

    /// Moral weight of a specific principle
    #[inline]
    pub fn moral_weight(&self, principle: &str) -> f32 {
        let hash = fnv1a_hash(principle.as_bytes());
        self.principles
            .get(&hash)
            .map(|r| r.weight)
            .unwrap_or(0.0)
    }

    /// Is the conscience clear? (No active violations, high fairness)
    #[inline]
    pub fn is_clear(&self) -> bool {
        self.fairness_ema >= CONSCIENCE_CLEAR_THRESHOLD
            && self.detect_starvation().is_empty()
            && self.priority_inversion_check().is_empty()
    }

    /// Statistics snapshot
    pub fn stats(&self) -> ConscienceStats {
        ConscienceStats {
            total_checks: self.total_checks,
            total_violations: self.total_violations,
            total_approvals: self.total_approvals,
            total_rejections: self.total_rejections,
            avg_fairness: self.fairness_ema,
            active_starvation_count: self.detect_starvation().len(),
            priority_inversions: self.priority_inversions,
            conscience_score: self.fairness_ema
                * (1.0
                    - (self.total_violations as f32
                        / (self.total_checks as f32 + 1.0).max(1.0))
                        .clamp(0.0, 1.0)),
        }
    }

    /// Reset the conscience
    #[inline]
    pub fn reset(&mut self) {
        self.processes.clear();
        self.violations.clear();
        self.audit_log.clear();
        self.fairness_ema = 1.0;
        self.total_resources_allocated = 0;
        for rule in self.principles.values_mut() {
            rule.violation_count = 0;
            rule.last_violation_tick = None;
        }
    }
}
