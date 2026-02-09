// SPDX-License-Identifier: GPL-2.0
//! # Apps Abstraction — Dynamic Application Abstraction Creation
//!
//! Discovers and creates new application categories that were never
//! predefined. Instead of relying solely on a fixed taxonomy, this engine
//! observes emergent behavior patterns across running applications and
//! groups them into novel abstract categories.
//!
//! The abstraction layer continuously compresses the taxonomy, evaluates
//! utility of each abstraction, and tracks how abstractions evolve over
//! time as the workload mix changes.

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
const MAX_ABSTRACTIONS: usize = 512;
const MAX_CATEGORIES: usize = 256;
const MIN_CLUSTER_SIZE: usize = 2;
const DISTANCE_MERGE_THRESHOLD: u64 = 20;
const UTILITY_DECAY_RATE: u64 = 3;

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

/// A behavioural fingerprint vector for a single application.
#[derive(Clone, Debug)]
pub struct AppFingerprint {
    pub app_id: u64,
    pub cpu_feature: u64,
    pub mem_feature: u64,
    pub io_feature: u64,
    pub ipc_feature: u64,
    pub composite_hash: u64,
    pub sample_count: u64,
}

/// An emergent category discovered by clustering.
#[derive(Clone, Debug)]
pub struct EmergentCategory {
    pub category_id: u64,
    pub label: String,
    pub centroid_hash: u64,
    pub member_ids: Vec<u64>,
    pub cohesion: u64,
    pub creation_tick: u64,
    pub last_used_tick: u64,
}

/// An abstraction that groups one or more categories at a higher level.
#[derive(Clone, Debug)]
pub struct Abstraction {
    pub abstraction_id: u64,
    pub label: String,
    pub child_category_ids: Vec<u64>,
    pub utility: u64,
    pub compression_ratio: u64,
    pub generation: u64,
    pub evolved_from: Option<u64>,
}

/// A taxonomy compression event log entry.
#[derive(Clone, Debug)]
pub struct CompressionEvent {
    pub event_id: u64,
    pub merged_ids: Vec<u64>,
    pub result_id: u64,
    pub saved_entries: u64,
    pub tick: u64,
}

/// Tracks evolution of a single abstraction across generations.
#[derive(Clone, Debug)]
pub struct AbstractionHistory {
    pub abstraction_id: u64,
    pub utility_trace: Vec<u64>,
    pub member_count_trace: Vec<u64>,
    pub generation_count: u64,
}

/// Aggregated statistics for the abstraction engine.
#[derive(Clone, Debug, Default)]
pub struct AbstractionStats {
    pub total_abstractions: u64,
    pub total_categories: u64,
    pub total_fingerprints: u64,
    pub compressions_performed: u64,
    pub avg_utility_ema: u64,
    pub avg_cohesion_ema: u64,
    pub evolution_events: u64,
    pub taxonomy_depth: u64,
}

// ---------------------------------------------------------------------------
// AppsAbstraction
// ---------------------------------------------------------------------------

/// Engine for dynamically discovering and managing emergent application
/// abstractions and categories.
pub struct AppsAbstraction {
    fingerprints: BTreeMap<u64, AppFingerprint>,
    categories: BTreeMap<u64, EmergentCategory>,
    abstractions: BTreeMap<u64, Abstraction>,
    compressions: Vec<CompressionEvent>,
    histories: BTreeMap<u64, AbstractionHistory>,
    stats: AbstractionStats,
    generation: u64,
    rng: u64,
    tick: u64,
}

impl AppsAbstraction {
    /// Create a new abstraction engine.
    pub fn new(seed: u64) -> Self {
        Self {
            fingerprints: BTreeMap::new(),
            categories: BTreeMap::new(),
            abstractions: BTreeMap::new(),
            compressions: Vec::new(),
            histories: BTreeMap::new(),
            stats: AbstractionStats::default(),
            generation: 0,
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- fingerprint observation --------------------------------------------

    /// Observe an application sample and update its fingerprint.
    pub fn observe_app(&mut self, app_id: u64, cpu: u64, mem: u64, io: u64, ipc: u64) {
        let composite = self.compute_composite(cpu, mem, io, ipc);
        let fp = self.fingerprints.entry(app_id).or_insert(AppFingerprint {
            app_id,
            cpu_feature: cpu,
            mem_feature: mem,
            io_feature: io,
            ipc_feature: ipc,
            composite_hash: composite,
            sample_count: 0,
        });
        fp.cpu_feature = ema_update(fp.cpu_feature, cpu);
        fp.mem_feature = ema_update(fp.mem_feature, mem);
        fp.io_feature = ema_update(fp.io_feature, io);
        fp.ipc_feature = ema_update(fp.ipc_feature, ipc);
        fp.composite_hash = ema_update(fp.composite_hash, composite);
        fp.sample_count += 1;
        self.stats.total_fingerprints = self.fingerprints.len() as u64;
    }

    // -- public API ---------------------------------------------------------

    /// Create a higher-level abstraction from a set of categories.
    pub fn create_app_abstraction(
        &mut self,
        label: &str,
        category_ids: &[u64],
    ) -> Option<u64> {
        if self.abstractions.len() >= MAX_ABSTRACTIONS || category_ids.is_empty() {
            return None;
        }
        // Verify all categories exist.
        let valid_ids: Vec<u64> = category_ids
            .iter()
            .copied()
            .filter(|id| self.categories.contains_key(id))
            .collect();
        if valid_ids.is_empty() {
            return None;
        }

        self.generation += 1;
        let abs_id = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let utility = self.estimate_utility(&valid_ids);
        let compression_ratio = self.compression_ratio(&valid_ids);

        let abs = Abstraction {
            abstraction_id: abs_id,
            label: String::from(label),
            child_category_ids: valid_ids,
            utility,
            compression_ratio,
            generation: self.generation,
            evolved_from: None,
        };
        self.abstractions.insert(abs_id, abs);
        self.init_history(abs_id, utility);
        self.stats.total_abstractions = self.abstractions.len() as u64;
        self.refresh_avg_utility();
        Some(abs_id)
    }

    /// Discover a new emergent category by clustering similar apps.
    pub fn discover_category(&mut self) -> Option<u64> {
        if self.categories.len() >= MAX_CATEGORIES || self.fingerprints.len() < MIN_CLUSTER_SIZE {
            return None;
        }

        self.tick += 1;
        // Find a pair of unassigned apps with closest fingerprints.
        let assigned: Vec<u64> = self
            .categories
            .values()
            .flat_map(|c| c.member_ids.iter().copied())
            .collect();

        let unassigned: Vec<u64> = self
            .fingerprints
            .keys()
            .copied()
            .filter(|id| !assigned.contains(id))
            .collect();

        if unassigned.len() < MIN_CLUSTER_SIZE {
            return None;
        }

        let (best_a, best_b, best_dist) = self.closest_pair(&unassigned);
        if best_dist > DISTANCE_MERGE_THRESHOLD * 2 {
            return None;
        }

        // Expand cluster with other close apps.
        let mut members = Vec::new();
        members.push(best_a);
        members.push(best_b);

        let centroid = self.cluster_centroid(&members);
        for &uid in &unassigned {
            if uid == best_a || uid == best_b {
                continue;
            }
            if let Some(fp) = self.fingerprints.get(&uid) {
                let dist = self.hash_distance(fp.composite_hash, centroid);
                if dist <= DISTANCE_MERGE_THRESHOLD {
                    members.push(uid);
                }
            }
        }

        let cat_id = fnv1a(&centroid.to_le_bytes()) ^ xorshift64(&mut self.rng);
        let cohesion = self.measure_cohesion(&members);
        let label = alloc::format!("emergent_{:x}", cat_id & 0xFFFF);
        let category = EmergentCategory {
            category_id: cat_id,
            label,
            centroid_hash: centroid,
            member_ids: members,
            cohesion,
            creation_tick: self.tick,
            last_used_tick: self.tick,
        };
        self.categories.insert(cat_id, category);
        self.stats.total_categories = self.categories.len() as u64;
        self.stats.avg_cohesion_ema = ema_update(self.stats.avg_cohesion_ema, cohesion);
        Some(cat_id)
    }

    /// Return the utility score of an abstraction (0–100).
    pub fn abstraction_utility(&self, abstraction_id: u64) -> Option<u64> {
        self.abstractions.get(&abstraction_id).map(|a| a.utility)
    }

    /// Compress the taxonomy by merging categories that are close together.
    pub fn compress_taxonomy(&mut self) -> u64 {
        self.tick += 1;
        let mut merges: u64 = 0;
        let cat_ids: Vec<u64> = self.categories.keys().copied().collect();

        let mut i = 0;
        while i < cat_ids.len() {
            let mut j = i + 1;
            while j < cat_ids.len() {
                let id_a = cat_ids[i];
                let id_b = cat_ids[j];
                let dist = self.category_distance(id_a, id_b);
                if dist <= DISTANCE_MERGE_THRESHOLD {
                    if self.merge_categories(id_a, id_b) {
                        merges += 1;
                    }
                }
                j += 1;
            }
            i += 1;
        }

        self.stats.compressions_performed += merges;
        self.stats.total_categories = self.categories.len() as u64;
        merges
    }

    /// Group apps by emergent behaviour patterns and assign to existing or new
    /// categories.
    pub fn emergent_grouping(&mut self) -> u64 {
        self.tick += 1;
        let mut grouped: u64 = 0;

        let app_ids: Vec<u64> = self.fingerprints.keys().copied().collect();
        for app_id in app_ids {
            let fp = match self.fingerprints.get(&app_id) {
                Some(f) => f.composite_hash,
                None => continue,
            };
            let best_cat = self.find_best_category(fp);
            if let Some(cat_id) = best_cat {
                if let Some(cat) = self.categories.get_mut(&cat_id) {
                    if !cat.member_ids.contains(&app_id) {
                        cat.member_ids.push(app_id);
                        cat.last_used_tick = self.tick;
                        grouped += 1;
                    }
                }
            }
        }
        grouped
    }

    /// Evolve an abstraction by re-evaluating its utility and potentially
    /// splitting or merging child categories.
    pub fn abstraction_evolution(&mut self, abstraction_id: u64) -> bool {
        let abs = match self.abstractions.get(&abstraction_id) {
            Some(a) => a.clone(),
            None => return false,
        };
        self.generation += 1;

        // Re-calculate utility.
        let new_utility = self.estimate_utility(&abs.child_category_ids);
        let new_compression = self.compression_ratio(&abs.child_category_ids);

        // Prune dead categories.
        let live_children: Vec<u64> = abs
            .child_category_ids
            .iter()
            .copied()
            .filter(|id| self.categories.contains_key(id))
            .collect();

        if live_children.is_empty() {
            self.abstractions.remove(&abstraction_id);
            self.stats.total_abstractions = self.abstractions.len() as u64;
            return false;
        }

        if let Some(a) = self.abstractions.get_mut(&abstraction_id) {
            a.utility = new_utility;
            a.compression_ratio = new_compression;
            a.child_category_ids = live_children;
            a.generation = self.generation;
        }

        self.update_history(abstraction_id, new_utility);
        self.stats.evolution_events += 1;
        self.refresh_avg_utility();
        true
    }

    /// Return current statistics.
    pub fn stats(&self) -> &AbstractionStats {
        &self.stats
    }

    /// Return the history of an abstraction.
    pub fn get_history(&self, abstraction_id: u64) -> Option<&AbstractionHistory> {
        self.histories.get(&abstraction_id)
    }

    /// Return the taxonomy depth (max nesting of abstractions).
    pub fn taxonomy_depth(&self) -> u64 {
        self.stats.taxonomy_depth
    }

    // -- internal -----------------------------------------------------------

    fn compute_composite(&mut self, cpu: u64, mem: u64, io: u64, ipc: u64) -> u64 {
        let mut buf = [0u8; 32];
        buf[0..8].copy_from_slice(&cpu.to_le_bytes());
        buf[8..16].copy_from_slice(&mem.to_le_bytes());
        buf[16..24].copy_from_slice(&io.to_le_bytes());
        buf[24..32].copy_from_slice(&ipc.to_le_bytes());
        fnv1a(&buf)
    }

    fn hash_distance(&self, a: u64, b: u64) -> u64 {
        (a ^ b).count_ones() as u64 * 100 / 64
    }

    fn closest_pair(&self, ids: &[u64]) -> (u64, u64, u64) {
        let mut best_a = 0u64;
        let mut best_b = 0u64;
        let mut best_dist = u64::MAX;
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let fa = self.fingerprints.get(&ids[i]);
                let fb = self.fingerprints.get(&ids[j]);
                if let (Some(a), Some(b)) = (fa, fb) {
                    let d = self.hash_distance(a.composite_hash, b.composite_hash);
                    if d < best_dist {
                        best_dist = d;
                        best_a = ids[i];
                        best_b = ids[j];
                    }
                }
            }
        }
        (best_a, best_b, best_dist)
    }

    fn cluster_centroid(&self, members: &[u64]) -> u64 {
        if members.is_empty() {
            return 0;
        }
        let mut acc: u64 = 0;
        for &mid in members {
            if let Some(fp) = self.fingerprints.get(&mid) {
                acc ^= fp.composite_hash;
            }
        }
        fnv1a(&acc.to_le_bytes())
    }

    fn measure_cohesion(&self, members: &[u64]) -> u64 {
        if members.len() < 2 {
            return 100;
        }
        let centroid = self.cluster_centroid(members);
        let total_dist: u64 = members
            .iter()
            .filter_map(|id| self.fingerprints.get(id))
            .map(|fp| self.hash_distance(fp.composite_hash, centroid))
            .sum();
        let avg_dist = total_dist / members.len() as u64;
        100u64.saturating_sub(avg_dist)
    }

    fn category_distance(&self, cat_a: u64, cat_b: u64) -> u64 {
        let ca = self.categories.get(&cat_a);
        let cb = self.categories.get(&cat_b);
        match (ca, cb) {
            (Some(a), Some(b)) => self.hash_distance(a.centroid_hash, b.centroid_hash),
            _ => u64::MAX,
        }
    }

    fn merge_categories(&mut self, keep_id: u64, remove_id: u64) -> bool {
        let removed = match self.categories.remove(&remove_id) {
            Some(c) => c,
            None => return false,
        };
        if let Some(keeper) = self.categories.get_mut(&keep_id) {
            for mid in removed.member_ids {
                if !keeper.member_ids.contains(&mid) {
                    keeper.member_ids.push(mid);
                }
            }
            keeper.cohesion = self.measure_cohesion(&keeper.member_ids);
            keeper.last_used_tick = self.tick;

            let event_id = fnv1a(&keep_id.to_le_bytes()) ^ fnv1a(&remove_id.to_le_bytes());
            self.compressions.push(CompressionEvent {
                event_id,
                merged_ids: alloc::vec![keep_id, remove_id],
                result_id: keep_id,
                saved_entries: 1,
                tick: self.tick,
            });
            return true;
        }
        false
    }

    fn find_best_category(&self, composite: u64) -> Option<u64> {
        let mut best_id = None;
        let mut best_dist = u64::MAX;
        for (cat_id, cat) in &self.categories {
            let d = self.hash_distance(cat.centroid_hash, composite);
            if d < best_dist && d <= DISTANCE_MERGE_THRESHOLD {
                best_dist = d;
                best_id = Some(*cat_id);
            }
        }
        best_id
    }

    fn estimate_utility(&self, category_ids: &[u64]) -> u64 {
        if category_ids.is_empty() {
            return 0;
        }
        let total_members: usize = category_ids
            .iter()
            .filter_map(|id| self.categories.get(id))
            .map(|c| c.member_ids.len())
            .sum();
        let avg_cohesion: u64 = category_ids
            .iter()
            .filter_map(|id| self.categories.get(id))
            .map(|c| c.cohesion)
            .sum::<u64>()
            .checked_div(category_ids.len() as u64)
            .unwrap_or(0);
        let size_bonus = (total_members as u64).min(30);
        (avg_cohesion / 2 + size_bonus).min(100)
    }

    fn compression_ratio(&self, category_ids: &[u64]) -> u64 {
        let total: usize = category_ids
            .iter()
            .filter_map(|id| self.categories.get(id))
            .map(|c| c.member_ids.len())
            .sum();
        if total == 0 {
            return 0;
        }
        let categories = category_ids.len() as u64;
        (total as u64 * 100) / (categories + total as u64)
    }

    fn init_history(&mut self, abs_id: u64, utility: u64) {
        self.histories.insert(abs_id, AbstractionHistory {
            abstraction_id: abs_id,
            utility_trace: alloc::vec![utility],
            member_count_trace: alloc::vec![0],
            generation_count: 1,
        });
    }

    fn update_history(&mut self, abs_id: u64, utility: u64) {
        if let Some(h) = self.histories.get_mut(&abs_id) {
            h.utility_trace.push(utility);
            let members = self
                .abstractions
                .get(&abs_id)
                .map(|a| a.child_category_ids.len() as u64)
                .unwrap_or(0);
            h.member_count_trace.push(members);
            h.generation_count += 1;
        }
        self.stats.taxonomy_depth = self.compute_taxonomy_depth();
    }

    fn compute_taxonomy_depth(&self) -> u64 {
        // Simple heuristic: count abstractions containing other abstractions' categories.
        let mut depth: u64 = 1;
        for abs in self.abstractions.values() {
            let child_abs_count = abs
                .child_category_ids
                .iter()
                .filter(|id| self.abstractions.contains_key(id))
                .count() as u64;
            let d = 1 + child_abs_count;
            if d > depth {
                depth = d;
            }
        }
        depth
    }

    fn refresh_avg_utility(&mut self) {
        if self.abstractions.is_empty() {
            return;
        }
        let sum: u64 = self.abstractions.values().map(|a| a.utility).sum();
        let avg = sum / self.abstractions.len() as u64;
        self.stats.avg_utility_ema = ema_update(self.stats.avg_utility_ema, avg);
    }
}
