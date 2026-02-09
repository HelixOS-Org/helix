// SPDX-License-Identifier: GPL-2.0
//! # Apps Identity
//!
//! Application engine identity declaration. Declares what this engine IS,
//! what workload types it CAN classify, its version and evolution history.
//! The identity module provides a stable fingerprint that uniquely identifies
//! this apps engine instance and its accumulated classification capabilities.
//!
//! Identity is not just a label — it is the sum of all supported workloads,
//! all matured classification capabilities, and all evolutionary milestones.
//! An engine that knows its identity can reason about what classifications
//! it should attempt and which ones exceed its current competence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAPABILITIES: usize = 128;
const MAX_EVOLUTION_LOG: usize = 256;
const MAX_WORKLOAD_TYPES: usize = 64;
const VERSION_MAJOR: u16 = 5;
const VERSION_MINOR: u16 = 0;
const VERSION_PATCH: u16 = 1;
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

/// Xorshift64 PRNG for unique nonce generation
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// CAPABILITY DECLARATION
// ============================================================================

/// Maturity level of a classification capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityMaturity {
    /// Just introduced, experimental
    Nascent    = 0,
    /// Functional but not battle-tested
    Developing = 1,
    /// Reliable in common scenarios
    Mature     = 2,
    /// Proven across edge cases, highly optimized
    Mastered   = 3,
}

/// A declared classification capability
#[derive(Debug, Clone)]
pub struct ClassificationCapability {
    pub name: String,
    pub id: u64,
    pub maturity: CapabilityMaturity,
    /// Performance level (0.0 – 1.0)
    pub performance: f32,
    /// Reliability score (0.0 – 1.0)
    pub reliability: f32,
    /// Version when this capability was introduced
    pub introduced_version: (u16, u16, u16),
    /// Tick when declared
    pub declared_tick: u64,
    /// Last tick when performance was updated
    pub last_update_tick: u64,
    /// Number of performance updates
    pub updates: u64,
}

/// An evolution event in the engine's history
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    pub id: u64,
    pub tick: u64,
    pub event_type: EvolutionType,
    pub description: String,
    pub fingerprint_before: u64,
    pub fingerprint_after: u64,
}

/// Types of evolution events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionType {
    CapabilityAdded,
    CapabilityMatured,
    CapabilityDegraded,
    CapabilityRemoved,
    WorkloadAdded,
    WorkloadRemoved,
    VersionBump,
}

/// A supported workload type
#[derive(Debug, Clone)]
pub struct SupportedWorkload {
    pub name: String,
    pub id: u64,
    /// How well this workload type is supported (0.0 – 1.0)
    pub support_level: f32,
    /// Number of classifications performed for this workload
    pub classifications: u64,
    /// Tick when first supported
    pub since_tick: u64,
}

// ============================================================================
// IDENTITY STATS
// ============================================================================

/// Aggregate identity statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct IdentityStats {
    pub capability_count: usize,
    pub nascent_count: usize,
    pub developing_count: usize,
    pub mature_count: usize,
    pub mastered_count: usize,
    pub supported_workloads: usize,
    pub avg_performance: f32,
    pub avg_reliability: f32,
    pub evolution_events: usize,
    pub identity_age_ticks: u64,
}

// ============================================================================
// APPS IDENTITY
// ============================================================================

/// Apps engine identity — domain declaration, workload support, capability
/// vector, evolution tracking, and identity fingerprinting.
#[derive(Debug)]
pub struct AppsIdentity {
    /// Domain this engine operates in
    domain: String,
    /// Classification capabilities keyed by FNV hash
    capabilities: BTreeMap<u64, ClassificationCapability>,
    /// Supported workload types keyed by FNV hash
    workloads: BTreeMap<u64, SupportedWorkload>,
    /// Evolution log
    evolution_log: Vec<EvolutionEvent>,
    /// Monotonic tick
    tick: u64,
    /// Tick when identity was created
    birth_tick: u64,
    /// Current capability fingerprint (XOR of all capability hashes)
    fingerprint: u64,
    /// Version tuple
    version: (u16, u16, u16),
    /// PRNG state for nonce generation
    rng_state: u64,
}

impl AppsIdentity {
    pub fn new() -> Self {
        Self {
            domain: String::from("application-understanding"),
            capabilities: BTreeMap::new(),
            workloads: BTreeMap::new(),
            evolution_log: Vec::new(),
            tick: 0,
            birth_tick: 0,
            fingerprint: FNV_OFFSET,
            version: (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH),
            rng_state: 0xFEED_FACE_DEAD_C0DE,
        }
    }

    /// Declare the engine's operational domain
    #[inline]
    pub fn declare_domain(&mut self, domain: &str) {
        self.tick += 1;
        let old_fp = self.fingerprint;
        self.domain = String::from(domain);
        self.fingerprint ^= fnv1a_hash(domain.as_bytes());
        self.log_evolution(
            EvolutionType::VersionBump,
            "Domain declaration updated",
            old_fp,
        );
    }

    /// Get the list of supported workload types
    #[inline]
    pub fn supported_workloads(&self) -> Vec<(String, f32, u64)> {
        self.workloads
            .values()
            .map(|w| (w.name.clone(), w.support_level, w.classifications))
            .collect()
    }

    /// Add or update a supported workload type
    pub fn add_workload(&mut self, name: &str, support_level: f32) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let old_fp = self.fingerprint;
        let tick = self.tick;

        let is_new = !self.workloads.contains_key(&id);
        let wl = self
            .workloads
            .entry(id)
            .or_insert_with(|| SupportedWorkload {
                name: String::from(name),
                id,
                support_level: 0.0,
                classifications: 0,
                since_tick: tick,
            });
        wl.support_level = support_level.max(0.0).min(1.0);

        if is_new && self.workloads.len() <= MAX_WORKLOAD_TYPES {
            self.fingerprint ^= id;
            self.log_evolution(EvolutionType::WorkloadAdded, name, old_fp);
        }
    }

    /// Compute the capability vector: a summary of all capabilities
    #[inline]
    pub fn capability_vector(&self) -> Vec<(String, CapabilityMaturity, f32, f32)> {
        self.capabilities
            .values()
            .map(|c| (c.name.clone(), c.maturity, c.performance, c.reliability))
            .collect()
    }

    /// Add or update a classification capability
    pub fn add_capability(
        &mut self,
        name: &str,
        maturity: CapabilityMaturity,
        performance: f32,
        reliability: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let old_fp = self.fingerprint;
        let tick = self.tick;

        let is_new = !self.capabilities.contains_key(&id);
        let cap = self
            .capabilities
            .entry(id)
            .or_insert_with(|| ClassificationCapability {
                name: String::from(name),
                id,
                maturity,
                performance: 0.0,
                reliability: 0.0,
                introduced_version: self.version,
                declared_tick: tick,
                last_update_tick: tick,
                updates: 0,
            });

        let old_maturity = cap.maturity;
        cap.maturity = maturity;
        cap.performance = performance.max(0.0).min(1.0);
        cap.reliability = reliability.max(0.0).min(1.0);
        cap.last_update_tick = tick;
        cap.updates += 1;

        if is_new && self.capabilities.len() <= MAX_CAPABILITIES {
            self.fingerprint ^= id;
            self.log_evolution(EvolutionType::CapabilityAdded, name, old_fp);
        } else if maturity > old_maturity {
            self.fingerprint = self.fingerprint.wrapping_add(id);
            self.log_evolution(EvolutionType::CapabilityMatured, name, old_fp);
        } else if maturity < old_maturity {
            self.log_evolution(EvolutionType::CapabilityDegraded, name, old_fp);
        }
    }

    /// Get the evolution log
    #[inline(always)]
    pub fn evolution_log(&self) -> &[EvolutionEvent] {
        &self.evolution_log
    }

    /// Compute a stable identity signature (FNV hash of fingerprint + version)
    #[inline]
    pub fn identity_signature(&self) -> u64 {
        let mut sig = self.fingerprint;
        sig ^= (self.version.0 as u64) << 32;
        sig ^= (self.version.1 as u64) << 16;
        sig ^= self.version.2 as u64;
        sig ^= fnv1a_hash(self.domain.as_bytes());
        sig
    }

    /// Internal helper to log evolution events
    fn log_evolution(&mut self, event_type: EvolutionType, description: &str, old_fp: u64) {
        if self.evolution_log.len() >= MAX_EVOLUTION_LOG {
            return;
        }
        let nonce = xorshift64(&mut self.rng_state);
        let event = EvolutionEvent {
            id: nonce,
            tick: self.tick,
            event_type,
            description: String::from(description),
            fingerprint_before: old_fp,
            fingerprint_after: self.fingerprint,
        };
        self.evolution_log.push(event);
    }

    /// Compute aggregate identity statistics
    pub fn stats(&self) -> IdentityStats {
        let n = self.capabilities.len();
        let nascent = self
            .capabilities
            .values()
            .filter(|c| c.maturity == CapabilityMaturity::Nascent)
            .count();
        let developing = self
            .capabilities
            .values()
            .filter(|c| c.maturity == CapabilityMaturity::Developing)
            .count();
        let mature = self
            .capabilities
            .values()
            .filter(|c| c.maturity == CapabilityMaturity::Mature)
            .count();
        let mastered = self
            .capabilities
            .values()
            .filter(|c| c.maturity == CapabilityMaturity::Mastered)
            .count();
        let avg_perf = if n > 0 {
            self.capabilities
                .values()
                .map(|c| c.performance)
                .sum::<f32>()
                / n as f32
        } else {
            0.0
        };
        let avg_rel = if n > 0 {
            self.capabilities
                .values()
                .map(|c| c.reliability)
                .sum::<f32>()
                / n as f32
        } else {
            0.0
        };

        IdentityStats {
            capability_count: n,
            nascent_count: nascent,
            developing_count: developing,
            mature_count: mature,
            mastered_count: mastered,
            supported_workloads: self.workloads.len(),
            avg_performance: avg_perf,
            avg_reliability: avg_rel,
            evolution_events: self.evolution_log.len(),
            identity_age_ticks: self.tick.saturating_sub(self.birth_tick),
        }
    }
}
