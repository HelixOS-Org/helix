// SPDX-License-Identifier: GPL-2.0
//! # Holistic Replication — System-Wide Reproducibility Engine
//!
//! Ensures ALL findings across ALL NEXUS subsystems are reproducible.
//! A finding that cannot be replicated is not knowledge — it is noise.
//! This engine orchestrates systematic replication attempts, monitors
//! replication crisis indicators, filters unreliable findings, and
//! mandates replication before any discovery gains canonical status.
//!
//! ## Capabilities
//!
//! - **System replication** — orchestrate replication of any finding
//! - **Replication crisis monitor** — detect system-wide replication failures
//! - **Robust knowledge filter** — only pass findings that replicate
//! - **Replication rate tracking** — per-subsystem and global rates
//! - **Unreliable finding detection** — flag findings that fail replication
//! - **Replication mandate** — enforce replication before knowledge promotion
//!
//! The engine that separates signal from noise across the entire kernel.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 2048;
const MAX_ATTEMPTS: usize = 8192;
const MAX_UNRELIABLE: usize = 512;
const MAX_MANDATES: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const REPLICATION_SUCCESS_THRESHOLD: f32 = 0.80;
const CRISIS_THRESHOLD: f32 = 0.50;
const ROBUST_CONFIDENCE: f32 = 0.85;
const MIN_REPLICATIONS: u64 = 3;
const UNRELIABLE_THRESHOLD: f32 = 0.30;
const EFFECT_TOLERANCE: f32 = 0.20;
const MANDATE_STRICTNESS: f32 = 0.70;

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

/// Subsystem that produced a finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplicationSubsystem {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
    FileSystem,
    Networking,
}

/// Status of a replication attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplicationStatus {
    Pending,
    InProgress,
    Replicated,
    PartialReplication,
    FailedReplication,
    Inconclusive,
}

/// Reliability classification of a finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReliabilityClass {
    Unreliable,
    Questionable,
    Tentative,
    Reliable,
    Robust,
    Canonical,
}

/// A finding subject to replication
#[derive(Debug, Clone)]
pub struct ReplicableFinding {
    pub id: u64,
    pub subsystem: ReplicationSubsystem,
    pub description: String,
    pub original_effect: f32,
    pub original_confidence: f32,
    pub replication_attempts: u64,
    pub successful_replications: u64,
    pub replication_rate: f32,
    pub reliability: ReliabilityClass,
    pub first_reported_tick: u64,
    pub last_attempt_tick: u64,
    pub hash: u64,
}

/// A single replication attempt
#[derive(Debug, Clone)]
pub struct ReplicationAttempt {
    pub id: u64,
    pub finding_id: u64,
    pub replicated_effect: f32,
    pub effect_difference: f32,
    pub status: ReplicationStatus,
    pub replicating_subsystem: ReplicationSubsystem,
    pub confidence: f32,
    pub tick: u64,
}

/// An unreliable finding that should be flagged
#[derive(Debug, Clone)]
pub struct UnreliableFinding {
    pub finding_id: u64,
    pub subsystem: ReplicationSubsystem,
    pub replication_rate: f32,
    pub attempts: u64,
    pub last_failure_tick: u64,
    pub flagged_tick: u64,
}

/// Replication mandate — requirement before knowledge promotion
#[derive(Debug, Clone)]
pub struct ReplicationMandate {
    pub id: u64,
    pub finding_id: u64,
    pub required_replications: u64,
    pub completed_replications: u64,
    pub mandate_strictness: f32,
    pub is_satisfied: bool,
    pub created_tick: u64,
}

/// Per-subsystem replication rate
#[derive(Debug, Clone)]
pub struct SubsystemReplicationRate {
    pub subsystem: ReplicationSubsystem,
    pub total_findings: u64,
    pub replicated_findings: u64,
    pub replication_rate: f32,
    pub avg_effect_diff_ema: f32,
    pub crisis_flag: bool,
}

/// Replication engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReplicationStats {
    pub total_findings_tracked: u64,
    pub total_attempts: u64,
    pub successful_replications: u64,
    pub failed_replications: u64,
    pub global_replication_rate_ema: f32,
    pub avg_effect_difference_ema: f32,
    pub crisis_level: f32,
    pub robust_findings: u64,
    pub unreliable_findings: u64,
    pub mandates_issued: u64,
    pub mandates_satisfied: u64,
    pub canonical_findings: u64,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC REPLICATION
// ============================================================================

/// System-wide replication engine for ensuring reproducibility
pub struct HolisticReplication {
    findings: BTreeMap<u64, ReplicableFinding>,
    attempts: VecDeque<ReplicationAttempt>,
    unreliable: Vec<UnreliableFinding>,
    mandates: BTreeMap<u64, ReplicationMandate>,
    subsystem_rates: BTreeMap<u64, SubsystemReplicationRate>,
    rng_state: u64,
    tick: u64,
    stats: ReplicationStats,
}

impl HolisticReplication {
    /// Create a new holistic replication engine
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            attempts: VecDeque::new(),
            unreliable: Vec::new(),
            mandates: BTreeMap::new(),
            subsystem_rates: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: ReplicationStats {
                total_findings_tracked: 0,
                total_attempts: 0,
                successful_replications: 0,
                failed_replications: 0,
                global_replication_rate_ema: 0.5,
                avg_effect_difference_ema: 0.0,
                crisis_level: 0.0,
                robust_findings: 0,
                unreliable_findings: 0,
                mandates_issued: 0,
                mandates_satisfied: 0,
                canonical_findings: 0,
                last_tick: 0,
            },
        }
    }

    /// Register a finding for replication tracking
    pub fn register_finding(&mut self, subsystem: ReplicationSubsystem,
                             description: String, effect: f32, confidence: f32) -> u64 {
        let id = self.stats.total_findings_tracked;
        let hash = fnv1a_hash(description.as_bytes());
        let finding = ReplicableFinding {
            id, subsystem, description, original_effect: effect,
            original_confidence: confidence, replication_attempts: 0,
            successful_replications: 0, replication_rate: 0.0,
            reliability: ReliabilityClass::Tentative,
            first_reported_tick: self.tick, last_attempt_tick: 0, hash,
        };
        if self.findings.len() >= MAX_FINDINGS {
            let oldest = self.findings.keys().next().copied();
            if let Some(k) = oldest { self.findings.remove(&k); }
        }
        self.findings.insert(id, finding);
        self.stats.total_findings_tracked += 1;
        id
    }

    /// Attempt to replicate a finding
    pub fn system_replication(&mut self, finding_id: u64,
                               replicating_subsystem: ReplicationSubsystem) -> ReplicationAttempt {
        let (original_effect, finding_subsystem) = match self.findings.get(&finding_id) {
            Some(f) => (f.original_effect, f.subsystem),
            None => {
                return ReplicationAttempt {
                    id: 0, finding_id, replicated_effect: 0.0,
                    effect_difference: 1.0, status: ReplicationStatus::Inconclusive,
                    replicating_subsystem, confidence: 0.0, tick: self.tick,
                };
            }
        };
        let noise = xorshift_f32(&mut self.rng_state);
        let replicated_effect = original_effect * (0.7 + noise * 0.6);
        let effect_diff = (replicated_effect - original_effect).abs();
        let relative_diff = if original_effect.abs() > 0.001 {
            effect_diff / original_effect.abs()
        } else { effect_diff };
        let status = if relative_diff <= EFFECT_TOLERANCE {
            ReplicationStatus::Replicated
        } else if relative_diff <= EFFECT_TOLERANCE * 2.0 {
            ReplicationStatus::PartialReplication
        } else {
            ReplicationStatus::FailedReplication
        };
        let confidence = (1.0 - relative_diff).max(0.0).min(1.0);
        let attempt_id = self.stats.total_attempts;
        let attempt = ReplicationAttempt {
            id: attempt_id, finding_id, replicated_effect,
            effect_difference: effect_diff, status,
            replicating_subsystem, confidence, tick: self.tick,
        };
        if self.attempts.len() >= MAX_ATTEMPTS { self.attempts.pop_front(); }
        self.attempts.push_back(attempt.clone());
        self.stats.total_attempts += 1;
        let is_success = status == ReplicationStatus::Replicated
            || status == ReplicationStatus::PartialReplication;
        if is_success {
            self.stats.successful_replications += 1;
        } else {
            self.stats.failed_replications += 1;
        }
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            finding.replication_attempts += 1;
            if is_success { finding.successful_replications += 1; }
            finding.replication_rate = if finding.replication_attempts > 0 {
                finding.successful_replications as f32 / finding.replication_attempts as f32
            } else { 0.0 };
            finding.last_attempt_tick = self.tick;
            finding.reliability = if finding.replication_rate >= 0.9
                && finding.successful_replications >= MIN_REPLICATIONS * 2 {
                ReliabilityClass::Canonical
            } else if finding.replication_rate >= REPLICATION_SUCCESS_THRESHOLD
                && finding.successful_replications >= MIN_REPLICATIONS {
                ReliabilityClass::Robust
            } else if finding.replication_rate >= 0.6 {
                ReliabilityClass::Reliable
            } else if finding.replication_rate >= 0.4 {
                ReliabilityClass::Tentative
            } else if finding.replication_rate >= UNRELIABLE_THRESHOLD {
                ReliabilityClass::Questionable
            } else {
                ReliabilityClass::Unreliable
            };
            if finding.reliability == ReliabilityClass::Canonical {
                self.stats.canonical_findings += 1;
            }
        }
        let success_rate = if self.stats.total_attempts > 0 {
            self.stats.successful_replications as f32 / self.stats.total_attempts as f32
        } else { 0.5 };
        self.stats.global_replication_rate_ema = self.stats.global_replication_rate_ema
            * (1.0 - EMA_ALPHA) + success_rate * EMA_ALPHA;
        self.stats.avg_effect_difference_ema = self.stats.avg_effect_difference_ema
            * (1.0 - EMA_ALPHA) + effect_diff * EMA_ALPHA;
        let key = finding_subsystem as u64;
        let rate = self.subsystem_rates.entry(key).or_insert(SubsystemReplicationRate {
            subsystem: finding_subsystem, total_findings: 0,
            replicated_findings: 0, replication_rate: 0.0,
            avg_effect_diff_ema: 0.0, crisis_flag: false,
        });
        rate.total_findings += 1;
        if is_success { rate.replicated_findings += 1; }
        rate.replication_rate = rate.replicated_findings as f32 / rate.total_findings.max(1) as f32;
        rate.avg_effect_diff_ema = rate.avg_effect_diff_ema
            * (1.0 - EMA_ALPHA) + effect_diff * EMA_ALPHA;
        rate.crisis_flag = rate.replication_rate < CRISIS_THRESHOLD;
        self.stats.last_tick = self.tick;
        attempt
    }

    /// Monitor for replication crisis — system-wide failure to replicate
    pub fn replication_crisis_monitor(&mut self) -> f32 {
        let mut crisis_domains = 0u64;
        let mut total_domains = 0u64;
        for rate in self.subsystem_rates.values() {
            if rate.total_findings >= MIN_REPLICATIONS {
                total_domains += 1;
                if rate.crisis_flag { crisis_domains += 1; }
            }
        }
        let crisis_level = if total_domains > 0 {
            crisis_domains as f32 / total_domains as f32
        } else { 0.0 };
        self.stats.crisis_level = self.stats.crisis_level
            * (1.0 - EMA_ALPHA) + crisis_level * EMA_ALPHA;
        crisis_level
    }

    /// Filter to only robust, reproducible knowledge
    pub fn robust_knowledge_filter(&mut self) -> Vec<u64> {
        let mut robust_ids = Vec::new();
        for finding in self.findings.values() {
            match finding.reliability {
                ReliabilityClass::Robust | ReliabilityClass::Canonical => {
                    if finding.replication_rate >= ROBUST_CONFIDENCE
                        && finding.successful_replications >= MIN_REPLICATIONS {
                        robust_ids.push(finding.id);
                    }
                }
                _ => {}
            }
        }
        self.stats.robust_findings = robust_ids.len() as u64;
        robust_ids
    }

    /// Get per-subsystem replication rate tracking
    #[inline(always)]
    pub fn replication_rate_tracking(&self) -> Vec<SubsystemReplicationRate> {
        self.subsystem_rates.values().cloned().collect()
    }

    /// Detect unreliable findings that consistently fail replication
    pub fn unreliable_finding_detection(&mut self) -> Vec<UnreliableFinding> {
        let mut newly_flagged = Vec::new();
        for finding in self.findings.values() {
            if finding.replication_attempts >= MIN_REPLICATIONS
                && finding.replication_rate < UNRELIABLE_THRESHOLD {
                let already_flagged = self.unreliable.iter()
                    .any(|u| u.finding_id == finding.id);
                if !already_flagged {
                    let uf = UnreliableFinding {
                        finding_id: finding.id, subsystem: finding.subsystem,
                        replication_rate: finding.replication_rate,
                        attempts: finding.replication_attempts,
                        last_failure_tick: finding.last_attempt_tick,
                        flagged_tick: self.tick,
                    };
                    newly_flagged.push(uf.clone());
                    if self.unreliable.len() < MAX_UNRELIABLE {
                        self.unreliable.push(uf);
                    }
                }
            }
        }
        self.stats.unreliable_findings = self.unreliable.len() as u64;
        newly_flagged
    }

    /// Issue or check a replication mandate
    pub fn replication_mandate(&mut self, finding_id: u64) -> ReplicationMandate {
        if let Some(mandate) = self.mandates.get(&finding_id) {
            return mandate.clone();
        }
        let required = MIN_REPLICATIONS;
        let completed = self.findings.get(&finding_id)
            .map(|f| f.successful_replications).unwrap_or(0);
        let is_satisfied = completed >= required;
        let mandate = ReplicationMandate {
            id: self.stats.mandates_issued,
            finding_id, required_replications: required,
            completed_replications: completed,
            mandate_strictness: MANDATE_STRICTNESS,
            is_satisfied, created_tick: self.tick,
        };
        if self.mandates.len() >= MAX_MANDATES {
            let oldest = self.mandates.keys().next().copied();
            if let Some(k) = oldest { self.mandates.remove(&k); }
        }
        self.mandates.insert(finding_id, mandate.clone());
        self.stats.mandates_issued += 1;
        if is_satisfied { self.stats.mandates_satisfied += 1; }
        mandate
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &ReplicationStats { &self.stats }

    /// Get all tracked findings
    #[inline(always)]
    pub fn findings(&self) -> &BTreeMap<u64, ReplicableFinding> { &self.findings }

    /// Get replication attempt log
    #[inline(always)]
    pub fn attempt_log(&self) -> &[ReplicationAttempt] { &self.attempts }

    /// Get unreliable findings list
    #[inline(always)]
    pub fn unreliable_findings(&self) -> &[UnreliableFinding] { &self.unreliable }

    /// Get all mandates
    #[inline(always)]
    pub fn mandates(&self) -> &BTreeMap<u64, ReplicationMandate> { &self.mandates }

    /// Get subsystem rates
    #[inline(always)]
    pub fn subsystem_rates(&self) -> &BTreeMap<u64, SubsystemReplicationRate> {
        &self.subsystem_rates
    }
}
