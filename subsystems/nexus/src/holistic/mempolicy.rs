// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic Memory Policy — NUMA memory placement policies
//!
//! Implements per-process and per-VMA NUMA memory policies including bind,
//! interleave, preferred, local, and weighted-interleave placement strategies.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// NUMA memory placement policy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MempolicyMode {
    Default,
    Bind,
    Interleave,
    Preferred,
    Local,
    WeightedInterleave,
}

/// Policy application scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MempolicyScope {
    Process,
    Vma,
    SharedPolicy,
    System,
}

/// Policy flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MempolicyFlag {
    Static,
    Relative,
    MoveAll,
    MoveExisting,
}

/// A NUMA node mask.
#[derive(Debug, Clone)]
pub struct NumaNodemask {
    pub bits: Vec<u64>,
    pub max_nodes: u32,
}

impl NumaNodemask {
    pub fn new(max_nodes: u32) -> Self {
        let words = ((max_nodes + 63) / 64) as usize;
        Self {
            bits: alloc::vec![0u64; words],
            max_nodes,
        }
    }

    #[inline]
    pub fn set_node(&mut self, node: u32) {
        if node < self.max_nodes {
            let word = (node / 64) as usize;
            let bit = node % 64;
            if word < self.bits.len() {
                self.bits[word] |= 1u64 << bit;
            }
        }
    }

    #[inline]
    pub fn clear_node(&mut self, node: u32) {
        if node < self.max_nodes {
            let word = (node / 64) as usize;
            let bit = node % 64;
            if word < self.bits.len() {
                self.bits[word] &= !(1u64 << bit);
            }
        }
    }

    pub fn test_node(&self, node: u32) -> bool {
        if node >= self.max_nodes {
            return false;
        }
        let word = (node / 64) as usize;
        let bit = node % 64;
        if word < self.bits.len() {
            (self.bits[word] & (1u64 << bit)) != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn weight(&self) -> u32 {
        let mut count = 0u32;
        for &word in &self.bits {
            count += word.count_ones();
        }
        count
    }

    pub fn first_set(&self) -> Option<u32> {
        for (i, &word) in self.bits.iter().enumerate() {
            if word != 0 {
                let bit = word.trailing_zeros();
                let node = i as u32 * 64 + bit;
                if node < self.max_nodes {
                    return Some(node);
                }
            }
        }
        None
    }
}

/// Weighted interleave configuration.
#[derive(Debug, Clone)]
pub struct WeightedInterleave {
    pub node_weights: ArrayMap<u32, 32>,
    pub total_weight: u32,
    pub interleave_index: u64,
}

impl WeightedInterleave {
    pub fn new() -> Self {
        Self {
            node_weights: ArrayMap::new(0),
            total_weight: 0,
            interleave_index: 0,
        }
    }

    #[inline]
    pub fn set_weight(&mut self, node: u32, weight: u32) {
        if let Some(old) = self.node_weights.insert(node, weight) {
            self.total_weight = self.total_weight - old + weight;
        } else {
            self.total_weight += weight;
        }
    }

    pub fn next_node(&mut self) -> Option<u32> {
        if self.total_weight == 0 {
            return None;
        }
        let idx = (self.interleave_index % self.total_weight as u64) as u32;
        self.interleave_index += 1;
        let mut cumulative = 0u32;
        for (&node, &weight) in &self.node_weights {
            cumulative += weight;
            if idx < cumulative {
                return Some(node);
            }
        }
        self.node_weights.keys().next().copied()
    }
}

/// A memory policy instance.
#[derive(Debug, Clone)]
pub struct MempolicyInstance {
    pub policy_id: u64,
    pub mode: MempolicyMode,
    pub scope: MempolicyScope,
    pub nodemask: NumaNodemask,
    pub preferred_node: Option<u32>,
    pub flags: Vec<MempolicyFlag>,
    pub weighted: Option<WeightedInterleave>,
    pub reference_count: u32,
    pub hit_count: u64,
    pub miss_count: u64,
    pub fallback_count: u64,
}

impl MempolicyInstance {
    pub fn new(policy_id: u64, mode: MempolicyMode, max_nodes: u32) -> Self {
        Self {
            policy_id,
            mode,
            scope: MempolicyScope::Process,
            nodemask: NumaNodemask::new(max_nodes),
            preferred_node: None,
            flags: Vec::new(),
            weighted: None,
            reference_count: 1,
            hit_count: 0,
            miss_count: 0,
            fallback_count: 0,
        }
    }

    pub fn select_node(&mut self) -> Option<u32> {
        match self.mode {
            MempolicyMode::Default | MempolicyMode::Local => {
                self.hit_count += 1;
                Some(0) // Local node
            }
            MempolicyMode::Preferred => {
                if let Some(node) = self.preferred_node {
                    self.hit_count += 1;
                    Some(node)
                } else {
                    self.fallback_count += 1;
                    Some(0)
                }
            }
            MempolicyMode::Bind => {
                if let Some(node) = self.nodemask.first_set() {
                    self.hit_count += 1;
                    Some(node)
                } else {
                    self.miss_count += 1;
                    None
                }
            }
            MempolicyMode::Interleave => {
                let w = self.nodemask.weight();
                if w == 0 {
                    self.miss_count += 1;
                    return None;
                }
                self.hit_count += 1;
                let idx = (self.hit_count % w as u64) as u32;
                let mut seen = 0u32;
                for node in 0..self.nodemask.max_nodes {
                    if self.nodemask.test_node(node) {
                        if seen == idx {
                            return Some(node);
                        }
                        seen += 1;
                    }
                }
                None
            }
            MempolicyMode::WeightedInterleave => {
                if let Some(ref mut wi) = self.weighted {
                    if let Some(node) = wi.next_node() {
                        self.hit_count += 1;
                        return Some(node);
                    }
                }
                self.miss_count += 1;
                None
            }
        }
    }
}

/// Statistics for the memory policy manager.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MempolicyStats {
    pub total_policies: u64,
    pub bind_policies: u64,
    pub interleave_policies: u64,
    pub preferred_policies: u64,
    pub weighted_policies: u64,
    pub total_allocations: u64,
    pub local_allocations: u64,
    pub remote_allocations: u64,
    pub fallback_allocations: u64,
}

/// Main holistic memory policy manager.
pub struct HolisticMempolicy {
    pub policies: BTreeMap<u64, MempolicyInstance>,
    pub process_policies: LinearMap<u64, 64>, // pid → policy_id
    pub max_numa_nodes: u32,
    pub next_policy_id: u64,
    pub stats: MempolicyStats,
}

impl HolisticMempolicy {
    pub fn new(max_numa_nodes: u32) -> Self {
        Self {
            policies: BTreeMap::new(),
            process_policies: LinearMap::new(),
            max_numa_nodes,
            next_policy_id: 1,
            stats: MempolicyStats {
                total_policies: 0,
                bind_policies: 0,
                interleave_policies: 0,
                preferred_policies: 0,
                weighted_policies: 0,
                total_allocations: 0,
                local_allocations: 0,
                remote_allocations: 0,
                fallback_allocations: 0,
            },
        }
    }

    pub fn create_policy(&mut self, mode: MempolicyMode) -> u64 {
        let id = self.next_policy_id;
        self.next_policy_id += 1;
        let policy = MempolicyInstance::new(id, mode, self.max_numa_nodes);
        self.policies.insert(id, policy);
        self.stats.total_policies += 1;
        match mode {
            MempolicyMode::Bind => self.stats.bind_policies += 1,
            MempolicyMode::Interleave => self.stats.interleave_policies += 1,
            MempolicyMode::Preferred => self.stats.preferred_policies += 1,
            MempolicyMode::WeightedInterleave => self.stats.weighted_policies += 1,
            _ => {}
        }
        id
    }

    #[inline]
    pub fn set_process_policy(&mut self, pid: u64, policy_id: u64) -> bool {
        if self.policies.contains_key(&policy_id) {
            self.process_policies.insert(pid, policy_id);
            true
        } else {
            false
        }
    }

    pub fn allocate_for_process(&mut self, pid: u64) -> Option<u32> {
        let policy_id = self.process_policies.get(pid).copied()?;
        let policy = self.policies.get_mut(&policy_id)?;
        let node = policy.select_node();
        self.stats.total_allocations += 1;
        if let Some(n) = node {
            if n == 0 {
                self.stats.local_allocations += 1;
            } else {
                self.stats.remote_allocations += 1;
            }
        } else {
            self.stats.fallback_allocations += 1;
        }
        node
    }

    #[inline(always)]
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    #[inline(always)]
    pub fn process_binding_count(&self) -> usize {
        self.process_policies.len()
    }
}
