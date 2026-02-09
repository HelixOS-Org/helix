// SPDX-License-Identifier: GPL-2.0
//! # Apps Omniscient — Perfect Application Understanding
//!
//! Provides total, complete understanding of every application running in the
//! kernel. Maintains a full behavioral taxonomy, knowledge graph of app
//! relationships, and ensures zero unknowns across the application space.
//!
//! The engine tracks every application's behavior, resource patterns, IPC
//! interactions, and lifecycle events, building a comprehensive model that
//! drives downstream optimization.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_TAXONOMY_DEPTH: usize = 16;
const KNOWLEDGE_EDGE_CAP: usize = 4096;

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

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Classification of a single application behavior dimension.
#[derive(Clone, Debug)]
pub struct BehaviorClass {
    pub class_id: u64,
    pub label: String,
    pub confidence: u64,
    pub sample_count: u64,
}

/// A directed edge in the knowledge graph connecting two apps.
#[derive(Clone, Debug)]
pub struct KnowledgeEdge {
    pub src_app: u64,
    pub dst_app: u64,
    pub relation_hash: u64,
    pub weight: u64,
}

/// Per-application behavioral profile.
#[derive(Clone, Debug)]
pub struct AppProfile {
    pub app_id: u64,
    pub name: String,
    pub cpu_ema: u64,
    pub mem_ema: u64,
    pub io_ema: u64,
    pub ipc_ema: u64,
    pub classes: Vec<BehaviorClass>,
    pub observation_count: u64,
    pub last_event_tick: u64,
}

/// Taxonomy node in the behavioral classification tree.
#[derive(Clone, Debug)]
pub struct TaxonomyNode {
    pub node_id: u64,
    pub parent_id: u64,
    pub depth: usize,
    pub label: String,
    pub member_count: u64,
}

/// Statistics for the omniscient engine.
#[derive(Clone, Debug, Default)]
pub struct OmniscientStats {
    pub total_apps_tracked: u64,
    pub total_observations: u64,
    pub taxonomy_nodes: u64,
    pub knowledge_edges: u64,
    pub unknown_count: u64,
    pub completeness_pct: u64,
    pub omniscience_score: u64,
}

// ---------------------------------------------------------------------------
// AppsOmniscient
// ---------------------------------------------------------------------------

/// Engine providing total understanding of every application in the system.
pub struct AppsOmniscient {
    profiles: BTreeMap<u64, AppProfile>,
    taxonomy: BTreeMap<u64, TaxonomyNode>,
    edges: Vec<KnowledgeEdge>,
    stats: OmniscientStats,
    rng: u64,
    tick: u64,
}

impl AppsOmniscient {
    /// Create a new omniscient engine.
    pub fn new(seed: u64) -> Self {
        let root_node = TaxonomyNode {
            node_id: 0,
            parent_id: 0,
            depth: 0,
            label: String::from("root"),
            member_count: 0,
        };
        let mut taxonomy = BTreeMap::new();
        taxonomy.insert(0, root_node);

        Self {
            profiles: BTreeMap::new(),
            taxonomy,
            edges: Vec::new(),
            stats: OmniscientStats { taxonomy_nodes: 1, ..Default::default() },
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Ingest an observation for a given application.
    pub fn observe(&mut self, app_id: u64, name: &str, cpu: u64, mem: u64, io: u64, ipc: u64) {
        self.tick += 1;
        let profile = self.profiles.entry(app_id).or_insert_with(|| {
            self.stats.total_apps_tracked += 1;
            AppProfile {
                app_id,
                name: String::from(name),
                cpu_ema: cpu,
                mem_ema: mem,
                io_ema: io,
                ipc_ema: ipc,
                classes: Vec::new(),
                observation_count: 0,
                last_event_tick: 0,
            }
        });
        profile.cpu_ema = ema_update(profile.cpu_ema, cpu);
        profile.mem_ema = ema_update(profile.mem_ema, mem);
        profile.io_ema = ema_update(profile.io_ema, io);
        profile.ipc_ema = ema_update(profile.ipc_ema, ipc);
        profile.observation_count += 1;
        profile.last_event_tick = self.tick;
        self.stats.total_observations += 1;

        self.classify_app(app_id);
        self.refresh_stats();
    }

    /// Record a relationship between two applications.
    pub fn record_relation(&mut self, src: u64, dst: u64, relation: &str) {
        if self.edges.len() >= KNOWLEDGE_EDGE_CAP {
            return;
        }
        let rh = fnv1a(relation.as_bytes());
        let weight = self.compute_edge_weight(src, dst);
        self.edges.push(KnowledgeEdge {
            src_app: src,
            dst_app: dst,
            relation_hash: rh,
            weight,
        });
        self.stats.knowledge_edges = self.edges.len() as u64;
    }

    /// Return total understanding across all applications (0–100).
    pub fn total_understanding(&self) -> u64 {
        if self.stats.total_apps_tracked == 0 {
            return 0;
        }
        let classified = self.profiles.values()
            .filter(|p| !p.classes.is_empty())
            .count() as u64;
        let ratio = classified * 100 / self.stats.total_apps_tracked;
        let obs_factor = if self.stats.total_observations > 1000 { 100 } else {
            self.stats.total_observations * 100 / 1000
        };
        (ratio + obs_factor) / 2
    }

    /// Build the application knowledge graph as a vector of edges.
    pub fn app_knowledge_graph(&self) -> Vec<(u64, u64, u64)> {
        self.edges.iter().map(|e| (e.src_app, e.dst_app, e.weight)).collect()
    }

    /// Measure completeness of behavioral classification (0–100).
    pub fn behavior_completeness(&self) -> u64 {
        if self.stats.total_apps_tracked == 0 {
            return 100;
        }
        let total_classes: u64 = self.profiles.values()
            .map(|p| p.classes.len() as u64)
            .sum();
        let expected = self.stats.total_apps_tracked * 4; // 4 dimensions
        if expected == 0 { return 100; }
        let pct = total_classes * 100 / expected;
        if pct > 100 { 100 } else { pct }
    }

    /// Return the count of completely unknown applications (no classification).
    pub fn unknown_app_zero(&self) -> u64 {
        self.profiles.values()
            .filter(|p| p.classes.is_empty() && p.observation_count < 3)
            .count() as u64
    }

    /// Compute the overall omniscience metric (0–100).
    pub fn omniscience_metric(&self) -> u64 {
        let understanding = self.total_understanding();
        let completeness = self.behavior_completeness();
        let zero_penalty = if self.stats.total_apps_tracked > 0 {
            self.unknown_app_zero() * 100 / self.stats.total_apps_tracked
        } else {
            0
        };
        let graph_density = if self.stats.total_apps_tracked > 1 {
            let max_edges = self.stats.total_apps_tracked * (self.stats.total_apps_tracked - 1);
            if max_edges == 0 { 0 } else {
                (self.stats.knowledge_edges * 100).min(max_edges * 100) / max_edges
            }
        } else {
            0
        };
        let raw = (understanding + completeness + graph_density) / 3;
        if raw > zero_penalty { raw - zero_penalty } else { 0 }
    }

    /// Return a snapshot of current statistics.
    pub fn stats(&self) -> &OmniscientStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn classify_app(&mut self, app_id: u64) {
        let profile = match self.profiles.get(&app_id) {
            Some(p) => p,
            None => return,
        };
        if profile.observation_count < 3 {
            return;
        }

        let mut new_classes = Vec::new();

        // CPU class
        let cpu_label = if profile.cpu_ema > 80 {
            "cpu_intensive"
        } else if profile.cpu_ema > 30 {
            "cpu_moderate"
        } else {
            "cpu_light"
        };
        let cpu_hash = fnv1a(cpu_label.as_bytes());
        new_classes.push(BehaviorClass {
            class_id: cpu_hash,
            label: String::from(cpu_label),
            confidence: profile.observation_count.min(100),
            sample_count: profile.observation_count,
        });

        // Memory class
        let mem_label = if profile.mem_ema > 70 {
            "mem_heavy"
        } else if profile.mem_ema > 25 {
            "mem_moderate"
        } else {
            "mem_light"
        };
        let mem_hash = fnv1a(mem_label.as_bytes());
        new_classes.push(BehaviorClass {
            class_id: mem_hash,
            label: String::from(mem_label),
            confidence: profile.observation_count.min(100),
            sample_count: profile.observation_count,
        });

        // IO class
        let io_label = if profile.io_ema > 60 {
            "io_bound"
        } else if profile.io_ema > 20 {
            "io_moderate"
        } else {
            "io_minimal"
        };
        let io_hash = fnv1a(io_label.as_bytes());
        new_classes.push(BehaviorClass {
            class_id: io_hash,
            label: String::from(io_label),
            confidence: profile.observation_count.min(100),
            sample_count: profile.observation_count,
        });

        // IPC class
        let ipc_label = if profile.ipc_ema > 50 {
            "ipc_heavy"
        } else if profile.ipc_ema > 15 {
            "ipc_moderate"
        } else {
            "ipc_isolated"
        };
        let ipc_hash = fnv1a(ipc_label.as_bytes());
        new_classes.push(BehaviorClass {
            class_id: ipc_hash,
            label: String::from(ipc_label),
            confidence: profile.observation_count.min(100),
            sample_count: profile.observation_count,
        });

        if let Some(p) = self.profiles.get_mut(&app_id) {
            p.classes = new_classes;
        }

        self.ensure_taxonomy_nodes(app_id);
    }

    fn ensure_taxonomy_nodes(&mut self, app_id: u64) {
        let profile = match self.profiles.get(&app_id) {
            Some(p) => p,
            None => return,
        };
        for cls in &profile.classes {
            if !self.taxonomy.contains_key(&cls.class_id) && self.taxonomy.len() < MAX_TAXONOMY_DEPTH * 64 {
                let node = TaxonomyNode {
                    node_id: cls.class_id,
                    parent_id: 0,
                    depth: 1,
                    label: cls.label.clone(),
                    member_count: 1,
                };
                self.taxonomy.insert(cls.class_id, node);
                self.stats.taxonomy_nodes = self.taxonomy.len() as u64;
            } else if let Some(tn) = self.taxonomy.get_mut(&cls.class_id) {
                tn.member_count += 1;
            }
        }
    }

    fn compute_edge_weight(&mut self, src: u64, dst: u64) -> u64 {
        let src_obs = self.profiles.get(&src).map(|p| p.observation_count).unwrap_or(1);
        let dst_obs = self.profiles.get(&dst).map(|p| p.observation_count).unwrap_or(1);
        let noise = xorshift64(&mut self.rng) % 10;
        ((src_obs + dst_obs) / 2).saturating_add(noise)
    }

    fn refresh_stats(&mut self) {
        self.stats.unknown_count = self.unknown_app_zero();
        self.stats.completeness_pct = self.behavior_completeness();
        self.stats.omniscience_score = self.omniscience_metric();
    }
}
