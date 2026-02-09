// SPDX-License-Identifier: GPL-2.0
//! # Bridge Genesis — Dynamic Capability Creation
//!
//! The bridge creates entirely new capabilities that were never programmed.
//! New syscall optimisation patterns are discovered at runtime through
//! pattern mining, research integration, and evolutionary exploration.
//! Each genesis event produces a new `Capability` that can be activated,
//! measured, and retired independently.
//!
//! FNV-1a hashing indexes capabilities; xorshift64 drives stochastic
//! exploration for novel patterns; EMA tracks capability effectiveness.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAPABILITIES: usize = 256;
const MAX_GENESIS_LOG: usize = 512;
const MAX_PATTERNS_PER_CAP: usize = 16;
const MAX_EXTENSIONS: usize = 64;
const EFFECTIVENESS_THRESHOLD: f32 = 0.60;
const EMA_ALPHA: f32 = 0.10;
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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// GENESIS TYPES
// ============================================================================

/// Origin of a capability's creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityOrigin {
    PatternMining,
    ResearchIntegration,
    EvolutionarySynthesis,
    AnomalyResponse,
    UserHint,
    CrossDomainTransfer,
}

/// Lifecycle status of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityStatus {
    Proposed,
    Testing,
    Active,
    Degraded,
    Retired,
}

/// A syscall pattern that a capability targets.
#[derive(Debug, Clone)]
pub struct SyscallPattern {
    pub pattern_id: u64,
    pub syscall_ids: Vec<u32>,
    pub frequency: f32,
    pub avg_latency_ns: f32,
    pub description: String,
}

/// A dynamically created capability.
#[derive(Debug, Clone)]
pub struct Capability {
    pub capability_id: u64,
    pub name: String,
    pub origin: CapabilityOrigin,
    pub status: CapabilityStatus,
    pub patterns: Vec<SyscallPattern>,
    pub effectiveness: f32,
    pub activation_count: u64,
    pub success_count: u64,
    pub latency_improvement_ema: f32,
    pub created_tick: u64,
    pub last_used_tick: u64,
    pub research_source: Option<u64>,
}

/// Record of a genesis event.
#[derive(Debug, Clone)]
pub struct GenesisEvent {
    pub event_id: u64,
    pub capability_id: u64,
    pub origin: CapabilityOrigin,
    pub description: String,
    pub confidence: f32,
    pub tick: u64,
}

/// A dynamic extension added to the bridge at runtime.
#[derive(Debug, Clone)]
pub struct DynamicExtension {
    pub extension_id: u64,
    pub name: String,
    pub capability_ids: Vec<u64>,
    pub combined_effectiveness: f32,
    pub tick: u64,
}

/// Inventory summary of all capabilities.
#[derive(Debug, Clone)]
pub struct CapabilityInventory {
    pub total: usize,
    pub active: usize,
    pub testing: usize,
    pub retired: usize,
    pub by_origin: Vec<(CapabilityOrigin, usize)>,
    pub avg_effectiveness: f32,
    pub total_activations: u64,
}

// ============================================================================
// GENESIS STATS
// ============================================================================

/// Aggregate statistics for the genesis engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenesisStats {
    pub total_capabilities_created: u64,
    pub total_genesis_events: u64,
    pub active_capabilities: u32,
    pub total_activations: u64,
    pub avg_effectiveness_ema: f32,
    pub avg_latency_improvement_ema: f32,
    pub total_extensions: u32,
    pub capabilities_from_research: u64,
}

// ============================================================================
// PATTERN MINER
// ============================================================================

#[derive(Debug)]
struct PatternMiner {
    observed: BTreeMap<u64, (Vec<u32>, u64, f32)>,
}

impl PatternMiner {
    fn new() -> Self {
        Self {
            observed: BTreeMap::new(),
        }
    }

    fn observe(&mut self, syscall_ids: &[u32], latency_ns: f32) {
        let hash = fnv1a_hash(
            &syscall_ids.iter().flat_map(|s| s.to_le_bytes()).collect::<Vec<u8>>(),
        );
        let entry = self.observed.entry(hash).or_insert_with(|| {
            (syscall_ids.to_vec(), 0, 0.0)
        });
        entry.1 += 1;
        entry.2 = EMA_ALPHA * latency_ns + (1.0 - EMA_ALPHA) * entry.2;
    }

    fn frequent_patterns(&self, min_freq: u64) -> Vec<SyscallPattern> {
        self.observed
            .iter()
            .filter(|(_, (_, count, _))| *count >= min_freq)
            .map(|(&hash, (ids, count, latency))| SyscallPattern {
                pattern_id: hash,
                syscall_ids: ids.clone(),
                frequency: *count as f32,
                avg_latency_ns: *latency,
                description: String::from("mined-pattern"),
            })
            .collect()
    }
}

// ============================================================================
// BRIDGE GENESIS
// ============================================================================

/// Dynamic capability creation engine. Discovers, creates, tests, and
/// manages new syscall optimisation capabilities at runtime.
#[derive(Debug)]
pub struct BridgeGenesis {
    capabilities: BTreeMap<u64, Capability>,
    genesis_log: Vec<GenesisEvent>,
    extensions: BTreeMap<u64, DynamicExtension>,
    miner: PatternMiner,
    tick: u64,
    rng_state: u64,
    stats: GenesisStats,
}

impl BridgeGenesis {
    pub fn new(seed: u64) -> Self {
        Self {
            capabilities: BTreeMap::new(),
            genesis_log: Vec::new(),
            extensions: BTreeMap::new(),
            miner: PatternMiner::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: GenesisStats::default(),
        }
    }

    /// Create a brand-new capability from a name, origin, and set of
    /// target syscall patterns.
    pub fn create_capability(
        &mut self,
        name: String,
        origin: CapabilityOrigin,
        patterns: Vec<SyscallPattern>,
        confidence: f32,
    ) -> Capability {
        self.tick += 1;
        self.stats.total_capabilities_created += 1;
        self.stats.total_genesis_events += 1;
        let cid = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let cap = Capability {
            capability_id: cid,
            name: name.clone(),
            origin,
            status: CapabilityStatus::Proposed,
            patterns: patterns.into_iter().take(MAX_PATTERNS_PER_CAP).collect(),
            effectiveness: 0.0,
            activation_count: 0,
            success_count: 0,
            latency_improvement_ema: 0.0,
            created_tick: self.tick,
            last_used_tick: self.tick,
            research_source: None,
        };

        // Record genesis event.
        let eid = xorshift64(&mut self.rng_state);
        if self.genesis_log.len() < MAX_GENESIS_LOG {
            self.genesis_log.push(GenesisEvent {
                event_id: eid,
                capability_id: cid,
                origin,
                description: name,
                confidence: confidence.max(0.0).min(1.0),
                tick: self.tick,
            });
        }

        // Evict least-effective capability if at capacity.
        if self.capabilities.len() >= MAX_CAPABILITIES && !self.capabilities.contains_key(&cid) {
            if let Some((&evict_id, _)) = self.capabilities.iter()
                .filter(|(_, c)| c.status == CapabilityStatus::Retired || c.status == CapabilityStatus::Degraded)
                .min_by(|a, b| a.1.effectiveness.partial_cmp(&b.1.effectiveness).unwrap_or(core::cmp::Ordering::Equal))
            {
                self.capabilities.remove(&evict_id);
            } else if let Some((&evict_id, _)) = self.capabilities.iter()
                .min_by(|a, b| a.1.effectiveness.partial_cmp(&b.1.effectiveness).unwrap_or(core::cmp::Ordering::Equal))
            {
                self.capabilities.remove(&evict_id);
            }
        }

        self.capabilities.insert(cid, cap.clone());
        self.recount_active();
        cap
    }

    /// Create a capability derived from a research discovery.
    pub fn capability_from_research(
        &mut self,
        name: String,
        research_id: u64,
        patterns: Vec<SyscallPattern>,
        confidence: f32,
    ) -> Capability {
        self.stats.capabilities_from_research += 1;
        let mut cap = self.create_capability(
            name,
            CapabilityOrigin::ResearchIntegration,
            patterns,
            confidence,
        );
        cap.research_source = Some(research_id);
        if let Some(stored) = self.capabilities.get_mut(&cap.capability_id) {
            stored.research_source = Some(research_id);
        }
        cap
    }

    /// Register a dynamic extension that bundles multiple capabilities.
    pub fn dynamic_extension(
        &mut self,
        name: String,
        capability_ids: Vec<u64>,
    ) -> DynamicExtension {
        self.tick += 1;
        let eid = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let mut total_eff: f32 = 0.0;
        let mut count = 0u32;
        for &cid in &capability_ids {
            if let Some(cap) = self.capabilities.get(&cid) {
                total_eff += cap.effectiveness;
                count += 1;
            }
        }
        let combined = if count > 0 { total_eff / count as f32 } else { 0.0 };

        let ext = DynamicExtension {
            extension_id: eid,
            name,
            capability_ids,
            combined_effectiveness: combined,
            tick: self.tick,
        };

        if self.extensions.len() >= MAX_EXTENSIONS {
            if let Some(&oldest) = self.extensions.keys().next() {
                self.extensions.remove(&oldest);
            }
        }
        self.extensions.insert(eid, ext.clone());
        self.stats.total_extensions = self.extensions.len() as u32;
        ext
    }

    /// Record a genesis event for an existing capability (e.g. mutation).
    pub fn genesis_event(
        &mut self,
        capability_id: u64,
        description: String,
        confidence: f32,
    ) -> Option<GenesisEvent> {
        self.tick += 1;
        if !self.capabilities.contains_key(&capability_id) {
            return None;
        }

        self.stats.total_genesis_events += 1;
        let eid = xorshift64(&mut self.rng_state);
        let origin = self.capabilities.get(&capability_id)
            .map(|c| c.origin)
            .unwrap_or(CapabilityOrigin::PatternMining);

        let event = GenesisEvent {
            event_id: eid,
            capability_id,
            origin,
            description,
            confidence: confidence.max(0.0).min(1.0),
            tick: self.tick,
        };

        if self.genesis_log.len() < MAX_GENESIS_LOG {
            self.genesis_log.push(event.clone());
        }
        Some(event)
    }

    /// Produce a full capability inventory report.
    pub fn capability_inventory(&self) -> CapabilityInventory {
        let total = self.capabilities.len();
        let active = self.capabilities.values().filter(|c| c.status == CapabilityStatus::Active).count();
        let testing = self.capabilities.values().filter(|c| c.status == CapabilityStatus::Testing).count();
        let retired = self.capabilities.values().filter(|c| c.status == CapabilityStatus::Retired).count();

        let mut origin_counts: BTreeMap<u8, usize> = BTreeMap::new();
        for cap in self.capabilities.values() {
            *origin_counts.entry(cap.origin as u8).or_insert(0) += 1;
        }

        let origins = [
            CapabilityOrigin::PatternMining,
            CapabilityOrigin::ResearchIntegration,
            CapabilityOrigin::EvolutionarySynthesis,
            CapabilityOrigin::AnomalyResponse,
            CapabilityOrigin::UserHint,
            CapabilityOrigin::CrossDomainTransfer,
        ];
        let by_origin: Vec<(CapabilityOrigin, usize)> = origins
            .iter()
            .map(|&o| (o, *origin_counts.get(&(o as u8)).unwrap_or(&0)))
            .collect();

        let avg_eff = if total > 0 {
            self.capabilities.values().map(|c| c.effectiveness).sum::<f32>() / total as f32
        } else {
            0.0
        };
        let total_act: u64 = self.capabilities.values().map(|c| c.activation_count).sum();

        CapabilityInventory {
            total,
            active,
            testing,
            retired,
            by_origin,
            avg_effectiveness: avg_eff,
            total_activations: total_act,
        }
    }

    /// Activate a capability for use.
    pub fn activate_capability(&mut self, capability_id: u64) -> bool {
        if let Some(cap) = self.capabilities.get_mut(&capability_id) {
            if cap.status == CapabilityStatus::Proposed || cap.status == CapabilityStatus::Testing {
                cap.status = CapabilityStatus::Active;
                self.recount_active();
                return true;
            }
        }
        false
    }

    /// Record usage of a capability with outcome feedback.
    pub fn record_usage(
        &mut self,
        capability_id: u64,
        success: bool,
        latency_improvement: f32,
    ) {
        if let Some(cap) = self.capabilities.get_mut(&capability_id) {
            cap.activation_count += 1;
            cap.last_used_tick = self.tick;
            if success {
                cap.success_count += 1;
            }
            let outcome = if success { 1.0 } else { 0.0 };
            cap.effectiveness = EMA_ALPHA * outcome + (1.0 - EMA_ALPHA) * cap.effectiveness;
            cap.latency_improvement_ema =
                EMA_ALPHA * latency_improvement + (1.0 - EMA_ALPHA) * cap.latency_improvement_ema;

            self.stats.total_activations += 1;
            self.stats.avg_effectiveness_ema =
                EMA_ALPHA * cap.effectiveness + (1.0 - EMA_ALPHA) * self.stats.avg_effectiveness_ema;
            self.stats.avg_latency_improvement_ema = EMA_ALPHA * cap.latency_improvement_ema
                + (1.0 - EMA_ALPHA) * self.stats.avg_latency_improvement_ema;

            // Auto-degrade if effectiveness drops below threshold.
            if cap.effectiveness < EFFECTIVENESS_THRESHOLD * 0.5 && cap.activation_count > 20 {
                cap.status = CapabilityStatus::Degraded;
                self.recount_active();
            }
        }
    }

    /// Observe a syscall pattern for the pattern miner.
    pub fn observe_pattern(&mut self, syscall_ids: &[u32], latency_ns: f32) {
        self.miner.observe(syscall_ids, latency_ns);
    }

    /// Auto-discover capabilities from frequently observed patterns.
    pub fn auto_discover(&mut self, min_frequency: u64) -> Vec<Capability> {
        let patterns = self.miner.frequent_patterns(min_frequency);
        let mut created = Vec::new();
        for pattern in patterns {
            let name = pattern.description.clone();
            let cap = self.create_capability(
                name,
                CapabilityOrigin::PatternMining,
                alloc::vec![pattern],
                0.5,
            );
            created.push(cap);
        }
        created
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> GenesisStats {
        self.stats
    }

    // ---- internal helpers ----

    fn recount_active(&mut self) {
        self.stats.active_capabilities = self
            .capabilities
            .values()
            .filter(|c| c.status == CapabilityStatus::Active)
            .count() as u32;
    }
}
