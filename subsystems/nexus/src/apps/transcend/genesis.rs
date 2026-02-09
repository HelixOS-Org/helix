// SPDX-License-Identifier: GPL-2.0
//! # Apps Genesis — Dynamic Creation of New Optimization Capabilities
//!
//! Creates entirely new app management capabilities at runtime. When the
//! kernel encounters previously-unknown workload types, the genesis engine
//! dynamically invents new optimization strategies, classifiers, and
//! adaptation mechanisms — expanding the system's intelligence envelope.

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
const EMA_ALPHA_DEN: u64 = 10;
const MAX_STRATEGIES: usize = 256;
const MAX_CLASSIFIERS: usize = 128;
const NOVELTY_THRESHOLD: u64 = 40;

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

/// A dynamically created optimization strategy.
#[derive(Clone, Debug)]
pub struct GenesisStrategy {
    pub strategy_id: u64,
    pub label: String,
    pub trigger_hash: u64,
    pub action_hash: u64,
    pub estimated_gain: u64,
    pub times_applied: u64,
    pub success_rate_ema: u64,
    pub generation: u64,
}

/// A dynamically created workload classifier.
#[derive(Clone, Debug)]
pub struct DynamicClassifier {
    pub classifier_id: u64,
    pub label: String,
    pub feature_hashes: Vec<u64>,
    pub threshold: u64,
    pub accuracy_ema: u64,
    pub classifications_made: u64,
    pub generation: u64,
}

/// Represents a novel workload that the system has not seen before.
#[derive(Clone, Debug)]
pub struct NovelWorkload {
    pub workload_id: u64,
    pub fingerprint: u64,
    pub cpu_profile: u64,
    pub mem_profile: u64,
    pub io_profile: u64,
    pub novelty_score: u64,
    pub adapted: bool,
}

/// A genesis capability — a meta-record of what the engine has created.
#[derive(Clone, Debug)]
pub struct GenesisCapability {
    pub capability_id: u64,
    pub kind: String,
    pub label: String,
    pub generation: u64,
    pub effectiveness: u64,
}

/// Per-workload observation for novelty detection.
#[derive(Clone, Debug)]
pub struct WorkloadObservation {
    pub workload_id: u64,
    pub cpu_ema: u64,
    pub mem_ema: u64,
    pub io_ema: u64,
    pub ipc_ema: u64,
    pub sample_count: u64,
    pub fingerprint: u64,
}

/// Statistics for the genesis engine.
#[derive(Clone, Debug, Default)]
pub struct GenesisStats {
    pub total_strategies: u64,
    pub total_classifiers: u64,
    pub novel_workloads_detected: u64,
    pub novel_workloads_adapted: u64,
    pub total_capabilities: u64,
    pub avg_success_rate_ema: u64,
    pub generation: u64,
}

// ---------------------------------------------------------------------------
// AppsGenesis
// ---------------------------------------------------------------------------

/// Engine that dynamically creates new optimization capabilities for
/// previously-unknown workload types.
pub struct AppsGenesis {
    strategies: BTreeMap<u64, GenesisStrategy>,
    classifiers: BTreeMap<u64, DynamicClassifier>,
    novel_workloads: BTreeMap<u64, NovelWorkload>,
    observations: BTreeMap<u64, WorkloadObservation>,
    capabilities: Vec<GenesisCapability>,
    stats: GenesisStats,
    generation: u64,
    rng: u64,
}

impl AppsGenesis {
    /// Create a new genesis engine.
    pub fn new(seed: u64) -> Self {
        Self {
            strategies: BTreeMap::new(),
            classifiers: BTreeMap::new(),
            novel_workloads: BTreeMap::new(),
            observations: BTreeMap::new(),
            capabilities: Vec::new(),
            stats: GenesisStats::default(),
            generation: 0,
            rng: seed | 1,
        }
    }

    // -- observation --------------------------------------------------------

    /// Observe a workload sample and detect novelty.
    pub fn observe_workload(
        &mut self,
        workload_id: u64,
        cpu: u64,
        mem: u64,
        io: u64,
        ipc: u64,
    ) {
        let fp = self.compute_fingerprint(cpu, mem, io, ipc);
        let obs = self.observations.entry(workload_id).or_insert(WorkloadObservation {
            workload_id,
            cpu_ema: cpu,
            mem_ema: mem,
            io_ema: io,
            ipc_ema: ipc,
            sample_count: 0,
            fingerprint: fp,
        });
        obs.cpu_ema = ema_update(obs.cpu_ema, cpu);
        obs.mem_ema = ema_update(obs.mem_ema, mem);
        obs.io_ema = ema_update(obs.io_ema, io);
        obs.ipc_ema = ema_update(obs.ipc_ema, ipc);
        obs.sample_count += 1;
        obs.fingerprint = ema_update(obs.fingerprint, fp);

        // Novelty detection.
        let novelty = self.compute_novelty(workload_id);
        if novelty > NOVELTY_THRESHOLD && !self.novel_workloads.contains_key(&workload_id) {
            self.novel_workloads.insert(workload_id, NovelWorkload {
                workload_id,
                fingerprint: obs.fingerprint,
                cpu_profile: obs.cpu_ema,
                mem_profile: obs.mem_ema,
                io_profile: obs.io_ema,
                novelty_score: novelty,
                adapted: false,
            });
            self.stats.novel_workloads_detected += 1;
        }
    }

    // -- public API ---------------------------------------------------------

    /// Create a new optimization strategy for a workload.
    pub fn create_strategy(&mut self, workload_id: u64, label: &str) -> Option<GenesisStrategy> {
        if self.strategies.len() >= MAX_STRATEGIES {
            return None;
        }

        self.generation += 1;
        let obs = self.observations.get(&workload_id)?;

        let trigger_hash = fnv1a(&obs.fingerprint.to_le_bytes());
        let action_hash = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let estimated_gain = self.estimate_strategy_gain(obs);

        let strategy_id = trigger_hash ^ action_hash;
        let strategy = GenesisStrategy {
            strategy_id,
            label: String::from(label),
            trigger_hash,
            action_hash,
            estimated_gain,
            times_applied: 0,
            success_rate_ema: 50,
            generation: self.generation,
        };

        self.strategies.insert(strategy_id, strategy.clone());
        self.stats.total_strategies = self.strategies.len() as u64;

        self.register_capability("strategy", label);
        Some(strategy)
    }

    /// Dynamically create a classifier for a novel workload pattern.
    pub fn dynamic_classifier(&mut self, workload_id: u64) -> Option<DynamicClassifier> {
        if self.classifiers.len() >= MAX_CLASSIFIERS {
            return None;
        }

        let obs = self.observations.get(&workload_id)?;
        self.generation += 1;

        let classifier_id = fnv1a(&obs.fingerprint.to_le_bytes()) ^ xorshift64(&mut self.rng);
        let feature_hashes = self.extract_feature_hashes(obs);
        let threshold = self.compute_classification_threshold(obs);

        let label = alloc::format!("classifier_{:x}", classifier_id & 0xFFFF);
        let classifier = DynamicClassifier {
            classifier_id,
            label: label.clone(),
            feature_hashes,
            threshold,
            accuracy_ema: 50,
            classifications_made: 0,
            generation: self.generation,
        };

        self.classifiers.insert(classifier_id, classifier.clone());
        self.stats.total_classifiers = self.classifiers.len() as u64;

        self.register_capability("classifier", &label);
        Some(classifier)
    }

    /// Apply a classifier to a workload sample and return the result.
    pub fn classify_with(&mut self, classifier_id: u64, cpu: u64, mem: u64, io: u64) -> Option<bool> {
        let classifier = self.classifiers.get_mut(&classifier_id)?;
        let fp = self.compute_fingerprint(cpu, mem, io, 0);
        let score = self.evaluate_classifier_score(&classifier.feature_hashes, fp);
        classifier.classifications_made += 1;
        let result = score >= classifier.threshold;
        Some(result)
    }

    /// Adapt the system to handle a novel workload — creates both a classifier
    /// and a strategy, marks the workload as adapted.
    pub fn novel_workload_adapt(&mut self, workload_id: u64) -> bool {
        if !self.novel_workloads.contains_key(&workload_id) {
            return false;
        }

        let created_classifier = self.dynamic_classifier(workload_id).is_some();
        let label = alloc::format!("adapt_{:x}", workload_id & 0xFFFF);
        let created_strategy = self.create_strategy(workload_id, &label).is_some();

        if created_classifier || created_strategy {
            if let Some(nw) = self.novel_workloads.get_mut(&workload_id) {
                nw.adapted = true;
                self.stats.novel_workloads_adapted += 1;
            }
            true
        } else {
            false
        }
    }

    /// Record strategy application result (success = true/false).
    pub fn record_strategy_result(&mut self, strategy_id: u64, success: bool) {
        if let Some(s) = self.strategies.get_mut(&strategy_id) {
            s.times_applied += 1;
            let sample = if success { 100 } else { 0 };
            s.success_rate_ema = ema_update(s.success_rate_ema, sample);
        }
        self.refresh_avg_success();
    }

    /// Return a summary of genesis capabilities created.
    pub fn genesis_capability(&self) -> &[GenesisCapability] {
        &self.capabilities
    }

    /// Return the total count of capabilities created.
    pub fn capability_count(&self) -> u64 {
        self.capabilities.len() as u64
    }

    /// Return current statistics.
    pub fn stats(&self) -> &GenesisStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn compute_fingerprint(&mut self, cpu: u64, mem: u64, io: u64, ipc: u64) -> u64 {
        let mut buf = [0u8; 32];
        buf[0..8].copy_from_slice(&cpu.to_le_bytes());
        buf[8..16].copy_from_slice(&mem.to_le_bytes());
        buf[16..24].copy_from_slice(&io.to_le_bytes());
        buf[24..32].copy_from_slice(&ipc.to_le_bytes());
        fnv1a(&buf)
    }

    fn compute_novelty(&self, workload_id: u64) -> u64 {
        let obs = match self.observations.get(&workload_id) {
            Some(o) => o,
            None => return 0,
        };

        let mut min_distance: u64 = u64::MAX;
        for (other_id, other_obs) in &self.observations {
            if *other_id == workload_id {
                continue;
            }
            let dist = self.fingerprint_distance(obs.fingerprint, other_obs.fingerprint);
            if dist < min_distance {
                min_distance = dist;
            }
        }

        if min_distance == u64::MAX {
            return 80;
        }

        min_distance.min(100)
    }

    fn fingerprint_distance(&self, a: u64, b: u64) -> u64 {
        (a ^ b).count_ones() as u64 * 100 / 64
    }

    fn estimate_strategy_gain(&mut self, obs: &WorkloadObservation) -> u64 {
        let pressure = obs.cpu_ema + obs.mem_ema + obs.io_ema;
        let base_gain = if pressure > 200 { 30 } else if pressure > 100 { 20 } else { 10 };
        let noise = xorshift64(&mut self.rng) % 15;
        (base_gain + noise).min(100)
    }

    fn extract_feature_hashes(&mut self, obs: &WorkloadObservation) -> Vec<u64> {
        let mut hashes = Vec::new();
        hashes.push(fnv1a(&obs.cpu_ema.to_le_bytes()));
        hashes.push(fnv1a(&obs.mem_ema.to_le_bytes()));
        hashes.push(fnv1a(&obs.io_ema.to_le_bytes()));
        hashes.push(obs.fingerprint);
        let combined = obs.cpu_ema ^ obs.mem_ema ^ obs.io_ema ^ xorshift64(&mut self.rng);
        hashes.push(fnv1a(&combined.to_le_bytes()));
        hashes
    }

    fn compute_classification_threshold(&self, obs: &WorkloadObservation) -> u64 {
        let avg = (obs.cpu_ema + obs.mem_ema + obs.io_ema) / 3;
        avg / 2 + 10
    }

    fn evaluate_classifier_score(&self, feature_hashes: &[u64], fp: u64) -> u64 {
        if feature_hashes.is_empty() {
            return 0;
        }
        let mut score: u64 = 0;
        for &fh in feature_hashes {
            let dist = (fh ^ fp).count_ones() as u64;
            let similarity = 64u64.saturating_sub(dist);
            score += similarity * 100 / 64;
        }
        score / feature_hashes.len() as u64
    }

    fn register_capability(&mut self, kind: &str, label: &str) {
        let cap_id = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        self.capabilities.push(GenesisCapability {
            capability_id: cap_id,
            kind: String::from(kind),
            label: String::from(label),
            generation: self.generation,
            effectiveness: 50,
        });
        self.stats.total_capabilities = self.capabilities.len() as u64;
        self.stats.generation = self.generation;
    }

    fn refresh_avg_success(&mut self) {
        if self.strategies.is_empty() {
            return;
        }
        let sum: u64 = self.strategies.values().map(|s| s.success_rate_ema).sum();
        let avg = sum / self.strategies.len() as u64;
        self.stats.avg_success_rate_ema = ema_update(self.stats.avg_success_rate_ema, avg);
    }
}
