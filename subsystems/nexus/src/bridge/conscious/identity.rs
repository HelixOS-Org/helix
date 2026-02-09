// SPDX-License-Identifier: GPL-2.0
//! # Bridge Identity
//!
//! Bridge identity and capability declaration. Declares what the bridge IS
//! and what it CAN DO. Version-aware, tracks capability evolution over time.
//! The identity module provides a stable fingerprint that uniquely identifies
//! this bridge instance and its accumulated capabilities.
//!
//! Identity is not just a name — it is the sum of all capabilities, all
//! lessons learned, and all evolution undergone. A bridge that knows its
//! identity can reason about what it should attempt and what it should avoid.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAPABILITIES: usize = 128;
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

/// Maturity level of a capability
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

/// A declared capability of the bridge
#[derive(Debug, Clone)]
pub struct DeclaredCapability {
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

/// An evolution event in the bridge's history
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    pub id: u64,
    pub tick: u64,
    pub event_type: EvolutionType,
    pub description: String,
    /// Capability fingerprint before this event
    pub fingerprint_before: u64,
    /// Capability fingerprint after this event
    pub fingerprint_after: u64,
}

/// Types of evolution events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionType {
    CapabilityAdded,
    CapabilityMatured,
    CapabilityDegraded,
    CapabilityRemoved,
    VersionBump,
    IdentityReset,
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
    pub avg_performance: f32,
    pub avg_reliability: f32,
    pub evolution_events: usize,
    pub identity_hash: u64,
    pub identity_age_ticks: u64,
}

// ============================================================================
// BRIDGE IDENTITY ENGINE
// ============================================================================

/// Bridge identity: what the bridge IS, what it CAN DO, and how it has
/// evolved over time. Provides a stable fingerprint and version tracking.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeIdentity {
    /// Declared capabilities (keyed by FNV hash)
    capabilities: BTreeMap<u64, DeclaredCapability>,
    /// Evolution log
    evolution_log: Vec<EvolutionEvent>,
    evo_write_idx: usize,
    /// Current version
    version: (u16, u16, u16),
    /// Tick of identity creation
    birth_tick: u64,
    /// Monotonic tick
    tick: u64,
    /// PRNG state for nonce generation
    rng_state: u64,
    /// Cached capability fingerprint
    cached_fingerprint: u64,
    /// Tick of last fingerprint computation
    fingerprint_tick: u64,
}

impl BridgeIdentity {
    pub fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
            evolution_log: Vec::new(),
            evo_write_idx: 0,
            version: (VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH),
            birth_tick: 0,
            tick: 0,
            rng_state: 0xBAAD_CAFE_DEAD_BEEF,
            cached_fingerprint: 0,
            fingerprint_tick: 0,
        }
    }

    /// Declare a new capability or update an existing one
    pub fn declare_capability(
        &mut self,
        name: &str,
        maturity: CapabilityMaturity,
        performance: f32,
        reliability: f32,
    ) {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes());
        let fp_before = self.capability_fingerprint();

        let is_new = !self.capabilities.contains_key(&id);
        let old_maturity = self.capabilities.get(&id).map(|c| c.maturity);

        let cap = self
            .capabilities
            .entry(id)
            .or_insert_with(|| DeclaredCapability {
                name: String::from(name),
                id,
                maturity,
                performance: 0.0,
                reliability: 0.0,
                introduced_version: self.version,
                declared_tick: self.tick,
                last_update_tick: self.tick,
                updates: 0,
            });

        cap.performance = performance.max(0.0).min(1.0);
        cap.reliability = reliability.max(0.0).min(1.0);
        cap.maturity = maturity;
        cap.last_update_tick = self.tick;
        cap.updates += 1;

        // Log evolution event
        let event_type = if is_new {
            EvolutionType::CapabilityAdded
        } else if let Some(old) = old_maturity {
            if maturity > old {
                EvolutionType::CapabilityMatured
            } else if maturity < old {
                EvolutionType::CapabilityDegraded
            } else {
                // No evolution event for same-level update
                return;
            }
        } else {
            EvolutionType::CapabilityAdded
        };

        let fp_after = self.compute_fingerprint();
        self.log_evolution(event_type, String::from(name), fp_before, fp_after);
    }

    /// Human-readable version string
    pub fn version_string(&self) -> String {
        let (major, minor, patch) = self.version;
        let mut s = String::new();
        // Manual number-to-string for no_std
        s.push_str("helix-bridge-v");
        push_number(&mut s, major as u64);
        s.push('.');
        push_number(&mut s, minor as u64);
        s.push('.');
        push_number(&mut s, patch as u64);
        s
    }

    /// Compute a fingerprint of all current capabilities
    #[inline]
    pub fn capability_fingerprint(&mut self) -> u64 {
        if self.tick == self.fingerprint_tick && self.cached_fingerprint != 0 {
            return self.cached_fingerprint;
        }
        self.cached_fingerprint = self.compute_fingerprint();
        self.fingerprint_tick = self.tick;
        self.cached_fingerprint
    }

    fn compute_fingerprint(&self) -> u64 {
        let mut hash = FNV_OFFSET;
        for (id, cap) in &self.capabilities {
            hash ^= *id;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= cap.maturity as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= (cap.performance * 1000.0) as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= (cap.reliability * 1000.0) as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Full evolution history
    #[inline(always)]
    pub fn evolution_history(&self) -> &[EvolutionEvent] {
        &self.evolution_log
    }

    /// Stable identity hash: combines version, birth, and capability fingerprint
    pub fn identity_hash(&mut self) -> u64 {
        let fp = self.capability_fingerprint();
        let mut hash = FNV_OFFSET;
        hash ^= self.version.0 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= self.version.1 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= self.version.2 as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= self.birth_tick;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= fp;
        hash = hash.wrapping_mul(FNV_PRIME);
        hash
    }

    /// Record an evolution event
    fn log_evolution(
        &mut self,
        event_type: EvolutionType,
        description: String,
        fp_before: u64,
        fp_after: u64,
    ) {
        let nonce = xorshift64(&mut self.rng_state);
        let event = EvolutionEvent {
            id: nonce,
            tick: self.tick,
            event_type,
            description,
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

    /// Bump the version (minor)
    pub fn bump_version(&mut self) {
        let fp_before = self.capability_fingerprint();
        self.version.1 += 1;
        self.tick += 1;
        let fp_after = self.compute_fingerprint();
        self.log_evolution(
            EvolutionType::VersionBump,
            self.version_string(),
            fp_before,
            fp_after,
        );
    }

    /// Remove a capability declaration
    pub fn remove_capability(&mut self, name: &str) {
        let id = fnv1a_hash(name.as_bytes());
        let fp_before = self.capability_fingerprint();
        if self.capabilities.remove(&id).is_some() {
            self.tick += 1;
            let fp_after = self.compute_fingerprint();
            self.log_evolution(
                EvolutionType::CapabilityRemoved,
                String::from(name),
                fp_before,
                fp_after,
            );
        }
    }

    /// Compute aggregate identity statistics
    pub fn stats(&mut self) -> IdentityStats {
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

        let avg_perf = if self.capabilities.is_empty() {
            0.0
        } else {
            self.capabilities
                .values()
                .map(|c| c.performance)
                .sum::<f32>()
                / self.capabilities.len() as f32
        };
        let avg_rel = if self.capabilities.is_empty() {
            0.0
        } else {
            self.capabilities
                .values()
                .map(|c| c.reliability)
                .sum::<f32>()
                / self.capabilities.len() as f32
        };

        let id_hash = self.identity_hash();

        IdentityStats {
            capability_count: self.capabilities.len(),
            nascent_count: nascent,
            developing_count: developing,
            mature_count: mature,
            mastered_count: mastered,
            avg_performance: avg_perf,
            avg_reliability: avg_rel,
            evolution_events: self.evolution_log.len(),
            identity_hash: id_hash,
            identity_age_ticks: self.tick.saturating_sub(self.birth_tick),
        }
    }

    /// List all capabilities sorted by maturity (highest first)
    #[inline]
    pub fn capability_roster(&self) -> Vec<(String, CapabilityMaturity, f32)> {
        let mut roster: Vec<(String, CapabilityMaturity, f32)> = self
            .capabilities
            .values()
            .map(|c| (c.name.clone(), c.maturity, c.performance))
            .collect();
        roster.sort_by(|a, b| b.1.cmp(&a.1));
        roster
    }
}

/// Helper: push a u64 as decimal into a String (no_std friendly)
fn push_number(s: &mut String, mut n: u64) {
    if n == 0 {
        s.push('0');
        return;
    }
    let mut digits = Vec::new();
    while n > 0 {
        digits.push((b'0' + (n % 10) as u8) as char);
        n /= 10;
    }
    for ch in digits.into_iter().rev() {
        s.push(ch);
    }
}
