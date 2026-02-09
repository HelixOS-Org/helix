// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Identity
//!
//! Cooperation engine identity declaration. Declares what protocols are
//! supported, what fairness guarantees are offered, and what cooperation
//! philosophy the engine follows. Tracks identity evolution over time
//! and provides cryptographic-style identity proofs via FNV fingerprinting.
//!
//! Identity is not just a name — it is the sum of all protocols, all
//! fairness commitments, and all philosophical principles that define how
//! this cooperation engine interacts with the world.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PROTOCOLS: usize = 128;
const MAX_GUARANTEES: usize = 64;
const MAX_EVOLUTION_LOG: usize = 256;
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

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// PROTOCOL DECLARATION TYPES
// ============================================================================

/// Maturity level of a supported protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtocolMaturity {
    /// Experimental, may change
    Experimental = 0,
    /// Functional but not widely tested
    Beta         = 1,
    /// Stable and reliable
    Stable       = 2,
    /// Battle-proven, fully optimized
    Proven       = 3,
}

/// A declared supported protocol
#[derive(Debug, Clone)]
pub struct SupportedProtocol {
    pub name: String,
    pub id: u64,
    pub maturity: ProtocolMaturity,
    /// Performance level (0.0 – 1.0)
    pub performance: f32,
    /// Reliability score (0.0 – 1.0)
    pub reliability: f32,
    /// Version when this protocol was introduced
    pub introduced_version: (u16, u16, u16),
    /// Tick when declared
    pub declared_tick: u64,
    /// Number of updates to this protocol
    pub updates: u64,
}

/// A fairness guarantee offered by this cooperation engine
#[derive(Debug, Clone)]
pub struct FairnessGuarantee {
    pub name: String,
    pub id: u64,
    /// Guaranteed minimum fairness level (0.0 – 1.0)
    pub minimum_level: f32,
    /// Observed actual level (0.0 – 1.0)
    pub actual_level: f32,
    /// Number of times this guarantee has been tested
    pub test_count: u64,
    /// Number of violations
    pub violation_count: u64,
    /// Is this guarantee currently being met?
    pub met: bool,
}

/// Cooperation philosophy principle
#[derive(Debug, Clone)]
pub struct PhilosophyPrinciple {
    pub name: String,
    pub id: u64,
    /// Adherence score (0.0 – 1.0)
    pub adherence: f32,
    /// Priority weight
    pub weight: f32,
    /// Description
    pub description: String,
}

/// An evolution event in the cooperation engine's history
#[derive(Debug, Clone)]
pub struct CoopEvolutionEvent {
    pub id: u64,
    pub tick: u64,
    pub event_type: CoopEvolutionType,
    pub description: String,
    pub fingerprint_before: u64,
    pub fingerprint_after: u64,
}

/// Types of cooperation evolution events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopEvolutionType {
    ProtocolAdded,
    ProtocolMatured,
    ProtocolDeprecated,
    GuaranteeStrengthened,
    GuaranteeViolated,
    PhilosophyEvolved,
    VersionBump,
}

// ============================================================================
// IDENTITY STATS
// ============================================================================

/// Aggregate identity statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityStats {
    pub protocol_count: usize,
    pub experimental_count: usize,
    pub beta_count: usize,
    pub stable_count: usize,
    pub proven_count: usize,
    pub guarantee_count: usize,
    pub guarantees_met: usize,
    pub avg_performance: f32,
    pub avg_reliability: f32,
    pub identity_hash: u64,
    pub evolution_events: usize,
}

// ============================================================================
// COOPERATION IDENTITY ENGINE
// ============================================================================

/// Cooperation engine identity: what protocols it supports, what fairness
/// guarantees it offers, what philosophy it follows, and how it has evolved.
#[derive(Debug)]
pub struct CoopIdentity {
    /// Declared protocols (keyed by FNV hash)
    protocols: BTreeMap<u64, SupportedProtocol>,
    /// Fairness guarantees (keyed by FNV hash)
    guarantees: BTreeMap<u64, FairnessGuarantee>,
    /// Philosophy principles (keyed by FNV hash)
    philosophy: BTreeMap<u64, PhilosophyPrinciple>,
    /// Evolution log
    evolution_log: Vec<CoopEvolutionEvent>,
    evo_write_idx: usize,
    /// Current version
    version: (u16, u16, u16),
    /// Tick of identity creation
    birth_tick: u64,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for nonce generation
    rng_state: u64,
    /// Cached fingerprint
    cached_fingerprint: u64,
    /// Tick of last fingerprint computation
    fingerprint_tick: u64,
}

impl CoopIdentity {
    pub fn new() -> Self {
        Self {
            protocols: BTreeMap::new(),
            guarantees: BTreeMap::new(),
            philosophy: BTreeMap::new(),
            evolution_log: Vec::new(),
            evo_write_idx: 0,
            version: (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH),
            birth_tick: 0,
            tick: 0,
            rng_state: 0x1D3A_C00B_CAFE_F00D,
            cached_fingerprint: 0,
            fingerprint_tick: 0,
        }
    }

    /// Declare a supported protocol or update an existing one
    pub fn declare_protocols(
        &mut self,
        name: &str,
        maturity: ProtocolMaturity,
        performance: f32,
        reliability: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let fp_before = self.compute_fingerprint();

        let is_new = !self.protocols.contains_key(&id);
        let old_maturity = self.protocols.get(&id).map(|p| p.maturity);

        let proto = self.protocols.entry(id).or_insert_with(|| SupportedProtocol {
            name: String::from(name),
            id,
            maturity,
            performance: 0.0,
            reliability: 0.0,
            introduced_version: self.version,
            declared_tick: self.tick,
            updates: 0,
        });

        proto.performance = performance.max(0.0).min(1.0);
        proto.reliability = reliability.max(0.0).min(1.0);
        proto.maturity = maturity;
        proto.updates += 1;

        let fp_after = self.compute_fingerprint();

        // Log evolution
        let evo_type = if is_new {
            CoopEvolutionType::ProtocolAdded
        } else if old_maturity.map(|m| maturity > m).unwrap_or(false) {
            CoopEvolutionType::ProtocolMatured
        } else {
            return; // No significant evolution event
        };

        let nonce = xorshift64(&mut self.rng_state);
        let event = CoopEvolutionEvent {
            id: nonce,
            tick: self.tick,
            event_type: evo_type,
            description: String::from(name),
            fingerprint_before: fp_before,
            fingerprint_after: fp_after,
        };

        if self.evolution_log.len() < MAX_EVOLUTION_LOG {
            self.evolution_log.push(event);
        } else {
            self.evolution_log[self.evo_write_idx] = event;
        }
        self.evo_write_idx = (self.evo_write_idx + 1) % MAX_EVOLUTION_LOG;
    }

    /// Declare or update a fairness guarantee
    pub fn fairness_guarantee(
        &mut self,
        name: &str,
        minimum_level: f32,
        actual_level: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        let guarantee = self.guarantees.entry(id).or_insert_with(|| FairnessGuarantee {
            name: String::from(name),
            id,
            minimum_level: minimum_level.max(0.0).min(1.0),
            actual_level: 0.5,
            test_count: 0,
            violation_count: 0,
            met: true,
        });

        guarantee.test_count += 1;
        guarantee.actual_level = actual_level.max(0.0).min(1.0);
        guarantee.minimum_level = minimum_level.max(0.0).min(1.0);

        let was_met = guarantee.met;
        guarantee.met = guarantee.actual_level >= guarantee.minimum_level;
        if !guarantee.met {
            guarantee.violation_count += 1;

            if was_met {
                // Transition from met → violated
                let fp_before = self.compute_fingerprint();
                let nonce = xorshift64(&mut self.rng_state);
                let event = CoopEvolutionEvent {
                    id: nonce,
                    tick: self.tick,
                    event_type: CoopEvolutionType::GuaranteeViolated,
                    description: String::from(name),
                    fingerprint_before: fp_before,
                    fingerprint_after: self.compute_fingerprint(),
                };
                if self.evolution_log.len() < MAX_EVOLUTION_LOG {
                    self.evolution_log.push(event);
                } else {
                    self.evolution_log[self.evo_write_idx] = event;
                }
                self.evo_write_idx = (self.evo_write_idx + 1) % MAX_EVOLUTION_LOG;
            }
        }
    }

    /// Declare or update a cooperation philosophy principle
    pub fn cooperation_philosophy(
        &mut self,
        name: &str,
        description: &str,
        adherence: f32,
        weight: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());

        let principle = self.philosophy.entry(id).or_insert_with(|| PhilosophyPrinciple {
            name: String::from(name),
            id,
            adherence: 0.5,
            weight: weight.max(0.0).min(1.0),
            description: String::from(description),
        });

        principle.adherence = adherence.max(0.0).min(1.0);
        principle.weight = weight.max(0.0).min(1.0);
    }

    /// Get evolution history as a summary: (event_count, maturation_count, violation_count)
    pub fn evolution_history(&self) -> (usize, usize, usize) {
        let total = self.evolution_log.len();
        let matured = self
            .evolution_log
            .iter()
            .filter(|e| e.event_type == CoopEvolutionType::ProtocolMatured)
            .count();
        let violated = self
            .evolution_log
            .iter()
            .filter(|e| e.event_type == CoopEvolutionType::GuaranteeViolated)
            .count();
        (total, matured, violated)
    }

    /// Produce an identity proof: a fingerprint that uniquely identifies this engine
    pub fn identity_proof(&mut self) -> u64 {
        self.tick += 1;

        // Combine protocol fingerprint, guarantee state, and philosophy
        let proto_fp = self.compute_fingerprint();

        let mut guarantee_fp = FNV_OFFSET;
        for g in self.guarantees.values() {
            guarantee_fp ^= g.id;
            guarantee_fp = guarantee_fp.wrapping_mul(FNV_PRIME);
            guarantee_fp ^= (g.actual_level * 1000.0) as u64;
            guarantee_fp = guarantee_fp.wrapping_mul(FNV_PRIME);
        }

        let mut philosophy_fp = FNV_OFFSET;
        for p in self.philosophy.values() {
            philosophy_fp ^= p.id;
            philosophy_fp = philosophy_fp.wrapping_mul(FNV_PRIME);
            philosophy_fp ^= (p.adherence * 1000.0) as u64;
            philosophy_fp = philosophy_fp.wrapping_mul(FNV_PRIME);
        }

        // Combine all three fingerprints
        let combined = proto_fp
            .wrapping_mul(FNV_PRIME)
            ^ guarantee_fp.wrapping_mul(FNV_PRIME)
            ^ philosophy_fp;

        // Mix in version and birth tick
        let version_bits = (self.version.0 as u64) << 32
            | (self.version.1 as u64) << 16
            | (self.version.2 as u64);

        combined ^ version_bits ^ self.birth_tick.wrapping_mul(FNV_PRIME)
    }

    /// Compute a fingerprint over all declared protocols
    fn compute_fingerprint(&mut self) -> u64 {
        if self.tick == self.fingerprint_tick && self.cached_fingerprint != 0 {
            return self.cached_fingerprint;
        }

        let mut fp = FNV_OFFSET;
        for proto in self.protocols.values() {
            fp ^= proto.id;
            fp = fp.wrapping_mul(FNV_PRIME);
            fp ^= proto.maturity as u64;
            fp = fp.wrapping_mul(FNV_PRIME);
            fp ^= (proto.performance * 1000.0) as u64;
            fp = fp.wrapping_mul(FNV_PRIME);
        }

        self.cached_fingerprint = fp;
        self.fingerprint_tick = self.tick;
        fp
    }

    /// Get aggregate identity statistics
    pub fn stats(&mut self) -> IdentityStats {
        let mut experimental = 0_usize;
        let mut beta = 0_usize;
        let mut stable = 0_usize;
        let mut proven = 0_usize;
        let mut total_perf = 0.0_f32;
        let mut total_rel = 0.0_f32;

        for proto in self.protocols.values() {
            match proto.maturity {
                ProtocolMaturity::Experimental => experimental += 1,
                ProtocolMaturity::Beta => beta += 1,
                ProtocolMaturity::Stable => stable += 1,
                ProtocolMaturity::Proven => proven += 1,
            }
            total_perf += proto.performance;
            total_rel += proto.reliability;
        }

        let count = self.protocols.len().max(1) as f32;
        let guarantees_met = self.guarantees.values().filter(|g| g.met).count();

        IdentityStats {
            protocol_count: self.protocols.len(),
            experimental_count: experimental,
            beta_count: beta,
            stable_count: stable,
            proven_count: proven,
            guarantee_count: self.guarantees.len(),
            guarantees_met,
            avg_performance: total_perf / count,
            avg_reliability: total_rel / count,
            identity_hash: self.compute_fingerprint(),
            evolution_events: self.evolution_log.len(),
        }
    }
}
