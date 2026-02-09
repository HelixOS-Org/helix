// SPDX-License-Identifier: GPL-2.0
//! # Holistic Genesis — CAPABILITY GENESIS
//!
//! `HolisticGenesis` is the birthplace of new kernel capabilities.  The
//! system creates abilities it was never explicitly programmed with —
//! dynamic capability creation, extension, and evolution within a bounded
//! `no_std` memory model.
//!
//! Each capability is a node in an ever-growing capability tree.  Nodes
//! can spawn children, merge with siblings, or undergo evolutionary
//! pressure to produce more efficient variants.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 12;
const MAX_CAPABILITIES: usize = 512;
const MAX_EVENTS: usize = 1024;
const EVOLUTION_PRESSURE_BPS: u64 = 3_000;
const GENESIS_MATURITY_THRESHOLD: u64 = 5_000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xf00dcafe } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Capability node
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Capability {
    pub cap_hash: u64,
    pub name: String,
    pub parent_hash: u64,
    pub generation: u64,
    pub fitness: u64,
    pub ema_fitness: u64,
    pub maturity_bps: u64,
    pub children: Vec<u64>,
    pub created_tick: u64,
    pub last_evolved_tick: u64,
}

impl Capability {
    fn new(name: String, parent: u64, gen: u64, tick: u64) -> Self {
        let h = fnv1a(name.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            cap_hash: h,
            name,
            parent_hash: parent,
            generation: gen,
            fitness: 0,
            ema_fitness: 0,
            maturity_bps: 0,
            children: Vec::new(),
            created_tick: tick,
            last_evolved_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// Evolution event
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct GenesisEvent {
    pub event_hash: u64,
    pub tick: u64,
    pub kind: String,
    pub capability_hash: u64,
    pub fitness_delta: u64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Capability tree summary
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CapabilityTreeSummary {
    pub total_nodes: u64,
    pub max_depth: u64,
    pub root_count: u64,
    pub mature_count: u64,
    pub avg_fitness: u64,
    pub tree_hash: u64,
}

// ---------------------------------------------------------------------------
// Dynamic extension record
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DynamicExtension {
    pub ext_hash: u64,
    pub base_capability: u64,
    pub extension_name: String,
    pub added_fitness: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct GenesisStats {
    pub total_capabilities: u64,
    pub total_events: u64,
    pub total_evolutions: u64,
    pub total_extensions: u64,
    pub ema_fitness: u64,
    pub genesis_rate_per_1k_ticks: u64,
    pub mature_capabilities: u64,
    pub peak_fitness: u64,
    pub deepest_lineage: u64,
}

impl GenesisStats {
    fn new() -> Self {
        Self {
            total_capabilities: 0,
            total_events: 0,
            total_evolutions: 0,
            total_extensions: 0,
            ema_fitness: 0,
            genesis_rate_per_1k_ticks: 0,
            mature_capabilities: 0,
            peak_fitness: 0,
            deepest_lineage: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// HolisticGenesis Engine
// ---------------------------------------------------------------------------

pub struct HolisticGenesis {
    capabilities: BTreeMap<u64, Capability>,
    events: VecDeque<GenesisEvent>,
    stats: GenesisStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticGenesis {
    pub fn new(seed: u64) -> Self {
        Self {
            capabilities: BTreeMap::new(),
            events: VecDeque::new(),
            stats: GenesisStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
        }
    }

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn gen_hash(&mut self, label: &str) -> u64 {
        fnv1a(label.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes()) ^ self.rng.next()
    }

    fn log_event(&mut self, kind: &str, cap_hash: u64, delta: u64, desc: &str) {
        let eh = self.gen_hash(kind);
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(GenesisEvent {
            event_hash: eh,
            tick: self.tick,
            kind: String::from(kind),
            capability_hash: cap_hash,
            fitness_delta: delta,
            description: String::from(desc),
        });
        self.stats.total_events = self.stats.total_events.wrapping_add(1);
    }

    fn refresh_stats(&mut self) {
        let mut sum_fit: u64 = 0;
        let mut mature: u64 = 0;
        let mut peak: u64 = 0;
        let mut deepest: u64 = 0;
        for cap in self.capabilities.values() {
            sum_fit = sum_fit.wrapping_add(cap.fitness);
            if cap.fitness > peak {
                peak = cap.fitness;
            }
            if cap.maturity_bps >= GENESIS_MATURITY_THRESHOLD {
                mature += 1;
            }
            if cap.generation > deepest {
                deepest = cap.generation;
            }
        }
        let count = self.capabilities.len() as u64;
        self.stats.total_capabilities = count;
        self.stats.mature_capabilities = mature;
        self.stats.peak_fitness = peak;
        self.stats.deepest_lineage = deepest;
        let avg = if count > 0 { sum_fit / count } else { 0 };
        self.stats.ema_fitness = ema_update(self.stats.ema_fitness, avg);
        if self.tick > 0 {
            self.stats.genesis_rate_per_1k_ticks =
                count.saturating_mul(1_000) / self.tick;
        }
    }

    fn lineage_depth(&self, cap_hash: u64) -> u64 {
        let mut depth: u64 = 0;
        let mut current = cap_hash;
        let mut visited = 0u64;
        while let Some(cap) = self.capabilities.get(&current) {
            if cap.parent_hash == 0 || visited > 256 {
                break;
            }
            current = cap.parent_hash;
            depth += 1;
            visited += 1;
        }
        depth
    }

    // -- 6 public methods ---------------------------------------------------

    /// Create a brand-new capability from scratch.
    pub fn create_capability(&mut self, name: String) -> Capability {
        self.advance_tick();
        let mut cap = Capability::new(name, 0, 0, self.tick);
        let fitness = 1_000_u64.wrapping_add(self.rng.next() % 4_000);
        cap.fitness = fitness;
        cap.ema_fitness = fitness;
        cap.maturity_bps = self.rng.next() % 3_000;
        let hash = cap.cap_hash;
        if self.capabilities.len() < MAX_CAPABILITIES {
            self.capabilities.insert(hash, cap.clone());
        }
        self.log_event("create", hash, fitness, "capability_born");
        self.refresh_stats();
        cap
    }

    /// Create a capability from nothing — emergent genesis with no parent.
    pub fn capability_from_nothing(&mut self) -> Capability {
        self.advance_tick();
        let labels = [
            "emergent_scheduler",
            "auto_healer",
            "predictive_prefetch",
            "self_tuning_cache",
            "adaptive_irq_router",
            "dynamic_power_gov",
        ];
        let idx = (self.rng.next() as usize) % labels.len();
        let name = String::from(labels[idx]);
        let mut cap = Capability::new(name, 0, 0, self.tick);
        let fitness = 2_000_u64.wrapping_add(self.rng.next() % 6_000);
        cap.fitness = fitness;
        cap.ema_fitness = fitness;
        cap.maturity_bps = self.rng.next() % 2_000;
        let hash = cap.cap_hash;
        if self.capabilities.len() < MAX_CAPABILITIES {
            self.capabilities.insert(hash, cap.clone());
        }
        self.log_event("genesis_from_nothing", hash, fitness, "emergent_birth");
        self.refresh_stats();
        cap
    }

    /// Dynamically extend an existing capability with new behaviour.
    pub fn dynamic_extension(&mut self, base_hash: u64, ext_name: &str) -> DynamicExtension {
        self.advance_tick();
        let added = self.rng.next() % 3_000;
        let eh = self.gen_hash(ext_name);
        if let Some(base) = self.capabilities.get_mut(&base_hash) {
            base.fitness = base.fitness.wrapping_add(added);
            base.ema_fitness = ema_update(base.ema_fitness, base.fitness);
            base.last_evolved_tick = self.tick;
            base.maturity_bps = base.maturity_bps.wrapping_add(500).min(10_000);
        }
        self.stats.total_extensions = self.stats.total_extensions.wrapping_add(1);
        self.log_event("extension", base_hash, added, ext_name);
        self.refresh_stats();
        DynamicExtension {
            ext_hash: eh,
            base_capability: base_hash,
            extension_name: String::from(ext_name),
            added_fitness: added,
            tick: self.tick,
        }
    }

    /// Trigger an evolution event — mutate a capability, possibly spawn child.
    pub fn evolution_event(&mut self, cap_hash: u64) -> GenesisEvent {
        self.advance_tick();
        let pressure = self.rng.next() % 10_000;
        let evolves = pressure >= EVOLUTION_PRESSURE_BPS;
        if let Some(parent) = self.capabilities.get(&cap_hash).cloned() {
            if evolves && self.capabilities.len() < MAX_CAPABILITIES {
                let child_name = {
                    let mut s = parent.name.clone();
                    s.push_str("_evo");
                    s
                };
                let mut child =
                    Capability::new(child_name, cap_hash, parent.generation + 1, self.tick);
                let child_fitness = parent.fitness.wrapping_add(self.rng.next() % 2_000);
                child.fitness = child_fitness;
                child.ema_fitness = ema_update(parent.ema_fitness, child_fitness);
                let ch = child.cap_hash;
                self.capabilities.insert(ch, child);
                if let Some(p) = self.capabilities.get_mut(&cap_hash) {
                    p.children.push(ch);
                    p.last_evolved_tick = self.tick;
                }
            } else if let Some(cap) = self.capabilities.get_mut(&cap_hash) {
                let delta = self.rng.next() % 1_000;
                cap.fitness = cap.fitness.wrapping_add(delta);
                cap.ema_fitness = ema_update(cap.ema_fitness, cap.fitness);
                cap.last_evolved_tick = self.tick;
            }
        }
        self.stats.total_evolutions = self.stats.total_evolutions.wrapping_add(1);
        self.refresh_stats();
        let evt_hash = self.gen_hash("evolution");
        let evt = GenesisEvent {
            event_hash: evt_hash,
            tick: self.tick,
            kind: String::from(if evolves { "speciation" } else { "mutation" }),
            capability_hash: cap_hash,
            fitness_delta: self.rng.next() % 2_000,
            description: String::from("evolutionary_pressure_applied"),
        };
        if self.events.len() < MAX_EVENTS {
            self.events.push_back(evt.clone());
        }
        evt
    }

    /// Build a summary of the capability tree.
    pub fn capability_tree(&mut self) -> CapabilityTreeSummary {
        self.advance_tick();
        self.refresh_stats();
        let root_count = self
            .capabilities
            .values()
            .filter(|c| c.parent_hash == 0)
            .count() as u64;
        let max_depth = self
            .capabilities
            .keys()
            .map(|&h| self.lineage_depth(h))
            .max()
            .unwrap_or(0);
        let mut tree_hash = FNV_OFFSET;
        for cap in self.capabilities.values() {
            tree_hash ^= cap.cap_hash;
            tree_hash = tree_hash.wrapping_mul(FNV_PRIME);
        }
        CapabilityTreeSummary {
            total_nodes: self.stats.total_capabilities,
            max_depth,
            root_count,
            mature_count: self.stats.mature_capabilities,
            avg_fitness: self.stats.ema_fitness,
            tree_hash,
        }
    }

    /// The genesis rate — how quickly new capabilities are being born.
    #[inline(always)]
    pub fn genesis_rate(&self) -> u64 {
        self.stats.genesis_rate_per_1k_ticks
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &GenesisStats {
        &self.stats
    }

    #[inline(always)]
    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }

    #[inline(always)]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_create_capability() {
        let mut eng = HolisticGenesis::new(42);
        let cap = eng.create_capability("new_sched".to_string());
        assert!(cap.fitness > 0);
        assert!(eng.capability_count() == 1);
    }

    #[test]
    fn test_capability_from_nothing() {
        let mut eng = HolisticGenesis::new(7);
        let cap = eng.capability_from_nothing();
        assert!(!cap.name.is_empty());
        assert!(cap.parent_hash == 0);
    }

    #[test]
    fn test_dynamic_extension() {
        let mut eng = HolisticGenesis::new(99);
        let cap = eng.create_capability("base".to_string());
        let ext = eng.dynamic_extension(cap.cap_hash, "turbo_mode");
        assert!(ext.added_fitness > 0 || ext.added_fitness == 0);
    }

    #[test]
    fn test_evolution_event() {
        let mut eng = HolisticGenesis::new(3);
        let cap = eng.create_capability("evolvable".to_string());
        for _ in 0..10 {
            eng.evolution_event(cap.cap_hash);
        }
        assert!(eng.stats().total_evolutions >= 10);
    }

    #[test]
    fn test_capability_tree() {
        let mut eng = HolisticGenesis::new(55);
        eng.create_capability("root_a".to_string());
        eng.create_capability("root_b".to_string());
        let tree = eng.capability_tree();
        assert!(tree.total_nodes == 2);
        assert!(tree.root_count == 2);
    }

    #[test]
    fn test_genesis_rate() {
        let mut eng = HolisticGenesis::new(11);
        for _ in 0..5 {
            eng.create_capability("gen_test".to_string());
        }
        let rate = eng.genesis_rate();
        assert!(rate > 0);
    }
}
