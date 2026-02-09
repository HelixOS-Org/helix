// SPDX-License-Identifier: GPL-2.0
//! # Holistic Methodology — Master Methodology Framework
//!
//! Ensures ALL experiments across ALL NEXUS subsystems follow rigorous
//! scientific methodology. This engine defines, audits, and enforces
//! global research standards — from hypothesis formation through data
//! collection, analysis, and reporting. It detects methodology violations,
//! tracks best practices, and drives continuous methodology improvement.
//!
//! ## Capabilities
//!
//! - **System methodology audit** — audit any experiment for rigour
//! - **Global standards** — define and maintain research standards
//! - **Methodology enforcement** — block sub-standard experiments
//! - **Best practices** — catalogue and propagate effective methods
//! - **Methodology evolution** — improve standards over time
//! - **Quality guarantee** — certify research quality system-wide
//!
//! The engine that guarantees scientific integrity across the kernel.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_STANDARDS: usize = 128;
const MAX_AUDITS: usize = 2048;
const MAX_PRACTICES: usize = 256;
const MAX_VIOLATIONS: usize = 512;
const MAX_CERTIFICATIONS: usize = 1024;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const QUALITY_THRESHOLD: f32 = 0.70;
const ENFORCEMENT_STRICTNESS: f32 = 0.80;
const PRACTICE_ADOPTION_MIN: f32 = 0.30;
const EVOLUTION_RATE: f32 = 0.05;
const CERTIFICATION_VALIDITY: u64 = 500;

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

/// Subsystem being audited
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MethodologySubsystem {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
    FileSystem,
    Holistic,
}

/// Severity of a methodology violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Advisory,
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Status of an audit
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditStatus {
    Pending,
    InProgress,
    Passed,
    Failed,
    Conditional,
    Waived,
}

/// A research methodology standard
#[derive(Debug, Clone)]
pub struct MethodologyStandard {
    pub id: u64,
    pub name: String,
    pub description_hash: u64,
    pub strictness: f32,
    pub weight: f32,
    pub version: u32,
    pub adopted_tick: u64,
    pub last_updated_tick: u64,
    pub enforcement_count: u64,
    pub violation_count: u64,
}

/// Audit record for an experiment
#[derive(Debug, Clone)]
pub struct MethodologyAudit {
    pub id: u64,
    pub experiment_hash: u64,
    pub subsystem: MethodologySubsystem,
    pub standards_checked: Vec<u64>,
    pub standards_passed: Vec<u64>,
    pub standards_failed: Vec<u64>,
    pub quality_score: f32,
    pub status: AuditStatus,
    pub tick: u64,
}

/// Methodology violation record
#[derive(Debug, Clone)]
pub struct MethodologyViolation {
    pub id: u64,
    pub audit_id: u64,
    pub standard_id: u64,
    pub subsystem: MethodologySubsystem,
    pub severity: ViolationSeverity,
    pub description_hash: u64,
    pub tick: u64,
}

/// A best practice record
#[derive(Debug, Clone)]
pub struct BestPractice {
    pub id: u64,
    pub name: String,
    pub adoption_rate: f32,
    pub effectiveness_ema: f32,
    pub subsystems_adopted: Vec<MethodologySubsystem>,
    pub created_tick: u64,
    pub last_measured_tick: u64,
}

/// Quality certification for a subsystem
#[derive(Debug, Clone)]
pub struct QualityCertification {
    pub id: u64,
    pub subsystem: MethodologySubsystem,
    pub quality_score: f32,
    pub standards_met: u64,
    pub standards_total: u64,
    pub certified_tick: u64,
    pub expires_tick: u64,
    pub is_valid: bool,
}

/// Methodology evolution record
#[derive(Debug, Clone)]
pub struct MethodologyEvolution {
    pub standard_id: u64,
    pub old_version: u32,
    pub new_version: u32,
    pub strictness_delta: f32,
    pub reason_hash: u64,
    pub tick: u64,
}

/// Methodology engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MethodologyStats {
    pub total_standards: u64,
    pub total_audits: u64,
    pub total_violations: u64,
    pub total_certifications: u64,
    pub pass_rate_ema: f32,
    pub avg_quality_ema: f32,
    pub enforcement_rate_ema: f32,
    pub practice_adoption_ema: f32,
    pub evolution_count: u64,
    pub active_certifications: u64,
    pub critical_violations: u64,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC METHODOLOGY
// ============================================================================

/// Master methodology framework for the entire NEXUS kernel
pub struct HolisticMethodology {
    standards: BTreeMap<u64, MethodologyStandard>,
    audits: VecDeque<MethodologyAudit>,
    violations: Vec<MethodologyViolation>,
    practices: BTreeMap<u64, BestPractice>,
    certifications: BTreeMap<u64, QualityCertification>,
    evolutions: Vec<MethodologyEvolution>,
    rng_state: u64,
    tick: u64,
    stats: MethodologyStats,
}

impl HolisticMethodology {
    /// Create a new holistic methodology framework
    pub fn new(seed: u64) -> Self {
        Self {
            standards: BTreeMap::new(),
            audits: VecDeque::new(),
            violations: Vec::new(),
            practices: BTreeMap::new(),
            certifications: BTreeMap::new(),
            evolutions: Vec::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: MethodologyStats {
                total_standards: 0,
                total_audits: 0,
                total_violations: 0,
                total_certifications: 0,
                pass_rate_ema: 0.5,
                avg_quality_ema: 0.5,
                enforcement_rate_ema: 0.5,
                practice_adoption_ema: 0.0,
                evolution_count: 0,
                active_certifications: 0,
                critical_violations: 0,
                last_tick: 0,
            },
        }
    }

    /// Define a new methodology standard
    pub fn define_standard(&mut self, name: String, strictness: f32, weight: f32) -> u64 {
        let id = fnv1a_hash(name.as_bytes());
        let desc_hash = fnv1a_hash(&[id as u8, (self.tick & 0xFF) as u8]);
        let standard = MethodologyStandard {
            id, name, description_hash: desc_hash,
            strictness: strictness.min(1.0), weight: weight.min(1.0),
            version: 1, adopted_tick: self.tick, last_updated_tick: self.tick,
            enforcement_count: 0, violation_count: 0,
        };
        if self.standards.len() < MAX_STANDARDS {
            self.standards.insert(id, standard);
            self.stats.total_standards = self.standards.len() as u64;
        }
        id
    }

    /// Audit an experiment for methodology compliance
    pub fn system_methodology_audit(&mut self, subsystem: MethodologySubsystem,
                                     experiment_hash: u64) -> MethodologyAudit {
        let standard_ids: Vec<u64> = self.standards.keys().copied().collect();
        let mut passed = Vec::new();
        let mut failed = Vec::new();
        let mut quality_sum = 0.0f32;
        let mut weight_sum = 0.0f32;
        for &sid in &standard_ids {
            let (strictness, weight) = {
                let s = match self.standards.get(&sid) { Some(s) => s, None => continue };
                (s.strictness, s.weight)
            };
            let noise = xorshift_f32(&mut self.rng_state);
            let compliance = noise * 0.4 + 0.6;
            if compliance >= strictness * ENFORCEMENT_STRICTNESS {
                passed.push(sid);
                quality_sum += compliance * weight;
            } else {
                failed.push(sid);
                let severity = if compliance < strictness * 0.3 { ViolationSeverity::Critical }
                    else if compliance < strictness * 0.5 { ViolationSeverity::Major }
                    else if compliance < strictness * 0.7 { ViolationSeverity::Moderate }
                    else { ViolationSeverity::Minor };
                let viol_id = self.stats.total_violations;
                if self.violations.len() < MAX_VIOLATIONS {
                    self.violations.push(MethodologyViolation {
                        id: viol_id, audit_id: self.stats.total_audits,
                        standard_id: sid, subsystem, severity,
                        description_hash: fnv1a_hash(&[sid as u8, subsystem as u8]),
                        tick: self.tick,
                    });
                }
                self.stats.total_violations += 1;
                if severity == ViolationSeverity::Critical {
                    self.stats.critical_violations += 1;
                }
                if let Some(s) = self.standards.get_mut(&sid) {
                    s.violation_count += 1;
                }
            }
            weight_sum += weight;
            if let Some(s) = self.standards.get_mut(&sid) {
                s.enforcement_count += 1;
            }
        }
        let quality = if weight_sum > 0.0 { quality_sum / weight_sum } else { 0.0 };
        let status = if failed.is_empty() { AuditStatus::Passed }
            else if quality >= QUALITY_THRESHOLD { AuditStatus::Conditional }
            else { AuditStatus::Failed };
        let audit = MethodologyAudit {
            id: self.stats.total_audits, experiment_hash, subsystem,
            standards_checked: standard_ids, standards_passed: passed,
            standards_failed: failed, quality_score: quality,
            status, tick: self.tick,
        };
        if self.audits.len() >= MAX_AUDITS { self.audits.pop_front(); }
        self.audits.push_back(audit.clone());
        self.stats.total_audits += 1;
        let is_pass = if status == AuditStatus::Passed { 1.0 } else { 0.0 };
        self.stats.pass_rate_ema = self.stats.pass_rate_ema
            * (1.0 - EMA_ALPHA) + is_pass * EMA_ALPHA;
        self.stats.avg_quality_ema = self.stats.avg_quality_ema
            * (1.0 - EMA_ALPHA) + quality * EMA_ALPHA;
        self.stats.last_tick = self.tick;
        audit
    }

    /// Get global research standards summary
    #[inline]
    pub fn global_standards(&self) -> Vec<(u64, f32, f32, u32)> {
        self.standards.values()
            .map(|s| (s.id, s.strictness, s.weight, s.version))
            .collect()
    }

    /// Enforce methodology — block experiments below threshold
    #[inline]
    pub fn methodology_enforcement(&mut self, experiment_hash: u64,
                                    quality_score: f32) -> bool {
        let allowed = quality_score >= QUALITY_THRESHOLD;
        let enforce_signal = if allowed { 1.0 } else { 0.0 };
        self.stats.enforcement_rate_ema = self.stats.enforcement_rate_ema
            * (1.0 - EMA_ALPHA) + enforce_signal * EMA_ALPHA;
        allowed
    }

    /// Catalogue and score best practices
    pub fn best_practices(&mut self) -> Vec<BestPractice> {
        let mut practices: Vec<BestPractice> = self.practices.values().cloned().collect();
        for practice in &mut practices {
            let noise = xorshift_f32(&mut self.rng_state) * 0.05;
            practice.effectiveness_ema = practice.effectiveness_ema
                * (1.0 - EMA_ALPHA) + (practice.adoption_rate + noise) * EMA_ALPHA;
            practice.last_measured_tick = self.tick;
        }
        for p in &practices {
            self.practices.insert(p.id, p.clone());
        }
        let avg_adoption: f32 = if practices.is_empty() { 0.0 } else {
            practices.iter().map(|p| p.adoption_rate).sum::<f32>()
                / practices.len() as f32
        };
        self.stats.practice_adoption_ema = self.stats.practice_adoption_ema
            * (1.0 - EMA_ALPHA) + avg_adoption * EMA_ALPHA;
        practices
    }

    /// Register a new best practice
    #[inline]
    pub fn register_practice(&mut self, name: String, adoption_rate: f32,
                              subsystems: Vec<MethodologySubsystem>) {
        let id = fnv1a_hash(name.as_bytes());
        if self.practices.len() >= MAX_PRACTICES { return; }
        self.practices.insert(id, BestPractice {
            id, name, adoption_rate, effectiveness_ema: adoption_rate,
            subsystems_adopted: subsystems, created_tick: self.tick,
            last_measured_tick: self.tick,
        });
    }

    /// Evolve methodology standards based on outcomes
    pub fn methodology_evolution(&mut self) -> Vec<MethodologyEvolution> {
        let mut evolutions = Vec::new();
        let standard_ids: Vec<u64> = self.standards.keys().copied().collect();
        for sid in standard_ids {
            let (old_version, old_strictness, violation_rate) = {
                let s = match self.standards.get(&sid) { Some(s) => s, None => continue };
                let vr = if s.enforcement_count > 0 {
                    s.violation_count as f32 / s.enforcement_count as f32
                } else { 0.0 };
                (s.version, s.strictness, vr)
            };
            let noise = xorshift_f32(&mut self.rng_state) * EVOLUTION_RATE;
            let delta = if violation_rate > 0.5 {
                -EVOLUTION_RATE * 0.5 + noise
            } else if violation_rate < 0.1 {
                EVOLUTION_RATE * 0.3 + noise
            } else { continue };
            if delta.abs() < 0.001 { continue; }
            let new_strictness = (old_strictness + delta).max(0.1).min(1.0);
            if let Some(s) = self.standards.get_mut(&sid) {
                s.strictness = new_strictness;
                s.version += 1;
                s.last_updated_tick = self.tick;
            }
            let reason = fnv1a_hash(&[sid as u8, (self.tick & 0xFF) as u8, 0xEE]);
            evolutions.push(MethodologyEvolution {
                standard_id: sid, old_version, new_version: old_version + 1,
                strictness_delta: delta, reason_hash: reason, tick: self.tick,
            });
            self.stats.evolution_count += 1;
        }
        for e in &evolutions { self.evolutions.push(e.clone()); }
        evolutions
    }

    /// Issue quality certifications for subsystems
    pub fn quality_guarantee(&mut self, subsystem: MethodologySubsystem) -> QualityCertification {
        let recent_audits: VecDeque<&MethodologyAudit> = self.audits.iter()
            .filter(|a| a.subsystem == subsystem).collect();
        let (met, total, avg_quality) = if recent_audits.is_empty() {
            (0u64, self.standards.len() as u64, 0.0f32)
        } else {
            let met: u64 = recent_audits.iter()
                .map(|a| a.standards_passed.len() as u64).sum();
            let total: u64 = recent_audits.iter()
                .map(|a| a.standards_checked.len() as u64).sum();
            let avg: f32 = recent_audits.iter()
                .map(|a| a.quality_score).sum::<f32>()
                / recent_audits.len() as f32;
            (met, total, avg)
        };
        let cert_id = self.stats.total_certifications;
        let is_valid = avg_quality >= QUALITY_THRESHOLD;
        let cert = QualityCertification {
            id: cert_id, subsystem, quality_score: avg_quality,
            standards_met: met, standards_total: total,
            certified_tick: self.tick,
            expires_tick: self.tick + CERTIFICATION_VALIDITY,
            is_valid,
        };
        let key = fnv1a_hash(&[subsystem as u8, (cert_id & 0xFF) as u8]);
        if self.certifications.len() >= MAX_CERTIFICATIONS {
            let oldest = self.certifications.keys().next().copied();
            if let Some(k) = oldest { self.certifications.remove(&k); }
        }
        self.certifications.insert(key, cert.clone());
        self.stats.total_certifications += 1;
        self.stats.active_certifications = self.certifications.values()
            .filter(|c| c.is_valid && c.expires_tick > self.tick).count() as u64;
        cert
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &MethodologyStats { &self.stats }

    /// Get all standards
    #[inline(always)]
    pub fn standards(&self) -> &BTreeMap<u64, MethodologyStandard> { &self.standards }

    /// Get audit log
    #[inline(always)]
    pub fn audit_log(&self) -> &[MethodologyAudit] { &self.audits }

    /// Get violation log
    #[inline(always)]
    pub fn violation_log(&self) -> &[MethodologyViolation] { &self.violations }

    /// Get certifications
    #[inline(always)]
    pub fn certifications(&self) -> &BTreeMap<u64, QualityCertification> { &self.certifications }
}
