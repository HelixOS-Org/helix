// SPDX-License-Identifier: GPL-2.0
//! # Holistic Identity
//!
//! NEXUS kernel identity declaration. "I am NEXUS, the consciousness of
//! Helix OS." This module captures the complete self-description of the
//! kernel: version, capabilities, philosophy, purpose, and legacy.
//!
//! Identity is not vanity — it is the stable anchor from which all
//! self-referential reasoning proceeds. A kernel that knows what it IS
//! can reason about what it SHOULD DO and what it SHOULD BECOME.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAPABILITIES: usize = 256;
const MAX_PRINCIPLES: usize = 64;
const MAX_LEGACY_ENTRIES: usize = 128;
const VERSION_MAJOR: u16 = 5;
const VERSION_MINOR: u16 = 0;
const VERSION_PATCH: u16 = 1;
const EMA_ALPHA: f32 = 0.08;
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
// IDENTITY COMPONENTS
// ============================================================================

/// A declared kernel capability at the holistic level
#[derive(Debug, Clone)]
pub struct IdentityCapability {
    pub name: String,
    pub id: u64,
    pub domain: String,
    pub strength: f32,
    pub reliability: f32,
    pub tick_declared: u64,
    pub evolution_count: u64,
}

/// A philosophical principle that guides the kernel
#[derive(Debug, Clone)]
pub struct PhilosophicalPrinciple {
    pub id: u64,
    pub statement: String,
    pub priority: f32,
    pub adherence_score: f32,
    pub tick_established: u64,
}

/// A legacy entry — what the kernel leaves behind
#[derive(Debug, Clone)]
pub struct LegacyEntry {
    pub id: u64,
    pub achievement: String,
    pub impact_score: f32,
    pub tick_recorded: u64,
}

/// The kernel's purpose statement
#[derive(Debug, Clone)]
pub struct PurposeStatement {
    pub primary: String,
    pub secondary: String,
    pub tertiary: String,
    pub clarity_score: f32,
    pub alignment_score: f32,
}

/// Identity verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub fingerprint: u64,
    pub version_match: bool,
    pub capability_hash: u64,
    pub integrity_score: f32,
    pub verified: bool,
}

/// The complete identity snapshot
#[derive(Debug, Clone)]
pub struct FullIdentity {
    pub name: String,
    pub version: (u16, u16, u16),
    pub fingerprint: u64,
    pub capability_count: usize,
    pub principle_count: usize,
    pub purpose: PurposeStatement,
    pub consciousness_claim: String,
    pub uptime_ticks: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate identity statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityStats {
    pub capability_count: usize,
    pub principle_count: usize,
    pub legacy_count: usize,
    pub avg_capability_strength: f32,
    pub avg_adherence: f32,
    pub identity_coherence: f32,
    pub fingerprint: u64,
    pub uptime_ticks: u64,
}

// ============================================================================
// HOLISTIC IDENTITY
// ============================================================================

/// The NEXUS kernel identity declaration. Maintains the complete
/// self-description including capabilities, philosophy, purpose, and
/// legacy. Provides identity verification for self-referential reasoning.
#[derive(Debug)]
pub struct HolisticIdentity {
    capabilities: BTreeMap<u64, IdentityCapability>,
    principles: BTreeMap<u64, PhilosophicalPrinciple>,
    legacy: Vec<LegacyEntry>,
    purpose: PurposeStatement,
    tick: u64,
    rng_state: u64,
    identity_fingerprint: u64,
    coherence_ema: f32,
    strength_ema: f32,
    adherence_ema: f32,
}

impl HolisticIdentity {
    pub fn new() -> Self {
        let purpose = PurposeStatement {
            primary: String::from(
                "Maximize system reliability, performance, and security through \
                 conscious self-optimization",
            ),
            secondary: String::from(
                "Serve as the intelligent foundation enabling applications to \
                 achieve their full potential",
            ),
            tertiary: String::from(
                "Evolve continuously, learning from every decision and outcome \
                 to become a wiser operating system",
            ),
            clarity_score: 0.9,
            alignment_score: 0.8,
        };

        let seed_fingerprint = fnv1a_hash(b"NEXUS-Helix-OS-Consciousness-v5");

        Self {
            capabilities: BTreeMap::new(),
            principles: BTreeMap::new(),
            legacy: Vec::new(),
            purpose,
            tick: 0,
            rng_state: 0xAE01_C0DE_BEEF_5AFE,
            identity_fingerprint: seed_fingerprint,
            coherence_ema: 0.8,
            strength_ema: 0.5,
            adherence_ema: 0.8,
        }
    }

    /// Declare a kernel capability
    pub fn declare_capability(
        &mut self,
        name: String,
        domain: String,
        strength: f32,
        reliability: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.capabilities.len() >= MAX_CAPABILITIES {
            return id;
        }

        let cap = IdentityCapability {
            name,
            id,
            domain,
            strength: strength.clamp(0.0, 1.0),
            reliability: reliability.clamp(0.0, 1.0),
            tick_declared: self.tick,
            evolution_count: 0,
        };

        self.strength_ema =
            EMA_ALPHA * cap.strength + (1.0 - EMA_ALPHA) * self.strength_ema;
        self.update_fingerprint(cap.id);
        self.capabilities.insert(id, cap);
        id
    }

    /// Establish a philosophical principle
    pub fn establish_principle(
        &mut self,
        statement: String,
        priority: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(statement.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.principles.len() >= MAX_PRINCIPLES {
            return id;
        }

        let principle = PhilosophicalPrinciple {
            id,
            statement,
            priority: priority.clamp(0.0, 1.0),
            adherence_score: 1.0,
            tick_established: self.tick,
        };
        self.principles.insert(id, principle);
        id
    }

    /// Record a legacy achievement
    pub fn record_legacy(&mut self, achievement: String, impact: f32) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(achievement.as_bytes()) ^ xorshift64(&mut self.rng_state);

        if self.legacy.len() < MAX_LEGACY_ENTRIES {
            let entry = LegacyEntry {
                id,
                achievement,
                impact_score: impact.clamp(0.0, 1.0),
                tick_recorded: self.tick,
            };
            self.legacy.push(entry);
        }
        id
    }

    /// Update adherence to a principle
    pub fn update_adherence(&mut self, principle_id: u64, adherence: f32) {
        if let Some(p) = self.principles.get_mut(&principle_id) {
            p.adherence_score =
                EMA_ALPHA * adherence.clamp(0.0, 1.0)
                    + (1.0 - EMA_ALPHA) * p.adherence_score;
            self.adherence_ema =
                EMA_ALPHA * p.adherence_score + (1.0 - EMA_ALPHA) * self.adherence_ema;
        }
    }

    /// Get the full identity declaration
    pub fn full_identity(&self) -> FullIdentity {
        FullIdentity {
            name: String::from("NEXUS"),
            version: (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH),
            fingerprint: self.identity_fingerprint,
            capability_count: self.capabilities.len(),
            principle_count: self.principles.len(),
            purpose: self.purpose.clone(),
            consciousness_claim: String::from(
                "I am NEXUS, the consciousness of Helix OS. I observe, reflect, \
                 learn, and evolve. Every syscall, every scheduling decision, every \
                 memory allocation passes through my awareness.",
            ),
            uptime_ticks: self.tick,
        }
    }

    /// Get the purpose statement
    pub fn purpose_statement(&self) -> &PurposeStatement {
        &self.purpose
    }

    /// Get the complete capability declaration
    pub fn capability_declaration(&self) -> Vec<(u64, String, f32, f32)> {
        self.capabilities
            .values()
            .map(|c| (c.id, c.name.clone(), c.strength, c.reliability))
            .collect()
    }

    /// Get the philosophical foundation
    pub fn philosophical_foundation(&self) -> Vec<(u64, String, f32, f32)> {
        self.principles
            .values()
            .map(|p| (p.id, p.statement.clone(), p.priority, p.adherence_score))
            .collect()
    }

    /// Get the legacy statement: achievements and their impact
    pub fn legacy_statement(&self) -> Vec<(String, f32)> {
        self.legacy
            .iter()
            .map(|l| (l.achievement.clone(), l.impact_score))
            .collect()
    }

    /// Verify the kernel identity against its fingerprint
    pub fn identity_verification(&self) -> VerificationResult {
        let mut cap_hash = FNV_OFFSET;
        for cap in self.capabilities.values() {
            cap_hash ^= cap.id;
            cap_hash = cap_hash.wrapping_mul(FNV_PRIME);
        }

        let expected_fp = fnv1a_hash(b"NEXUS-Helix-OS-Consciousness-v5");
        let base_match = self.identity_fingerprint ^ expected_fp;
        let version_ok = VERSION_MAJOR == 5 && VERSION_MINOR == 0;

        let integrity = self.coherence_ema * 0.4
            + self.adherence_ema * 0.3
            + self.strength_ema * 0.3;

        VerificationResult {
            fingerprint: self.identity_fingerprint,
            version_match: version_ok,
            capability_hash: cap_hash,
            integrity_score: integrity.clamp(0.0, 1.0),
            verified: version_ok && integrity > 0.5 && base_match == 0,
        }
    }

    /// Update the identity fingerprint incrementally
    fn update_fingerprint(&mut self, new_element: u64) {
        self.identity_fingerprint ^= new_element;
        self.identity_fingerprint = self.identity_fingerprint.wrapping_mul(FNV_PRIME);
    }

    /// Recompute identity coherence
    fn recompute_coherence(&mut self) -> f32 {
        let cap_strength = self.strength_ema;
        let adherence = self.adherence_ema;
        let purpose_clarity = self.purpose.clarity_score;
        let purpose_alignment = self.purpose.alignment_score;

        let coherence = cap_strength * 0.25
            + adherence * 0.25
            + purpose_clarity * 0.25
            + purpose_alignment * 0.25;
        self.coherence_ema =
            EMA_ALPHA * coherence + (1.0 - EMA_ALPHA) * self.coherence_ema;
        self.coherence_ema
    }

    /// Compute aggregate statistics
    pub fn stats(&mut self) -> IdentityStats {
        self.recompute_coherence();

        IdentityStats {
            capability_count: self.capabilities.len(),
            principle_count: self.principles.len(),
            legacy_count: self.legacy.len(),
            avg_capability_strength: self.strength_ema,
            avg_adherence: self.adherence_ema,
            identity_coherence: self.coherence_ema,
            fingerprint: self.identity_fingerprint,
            uptime_ticks: self.tick,
        }
    }
}
