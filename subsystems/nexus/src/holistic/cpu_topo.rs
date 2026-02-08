// SPDX-License-Identifier: GPL-2.0
//! Holistic cpu_topo_v2 — advanced CPU topology discovery and representation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CPU cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    L1d,
    L1i,
    L2,
    L3,
    L4,
}

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    Data,
    Instruction,
    Unified,
}

/// Cache descriptor
#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub level: CacheLevel,
    pub cache_type: CacheType,
    pub size_kb: u32,
    pub line_size: u32,
    pub ways: u32,
    pub sets: u32,
    pub shared_cpus: Vec<u32>,
    pub inclusive: bool,
}

impl CacheInfo {
    pub fn total_size(&self) -> u64 { self.size_kb as u64 * 1024 }
    pub fn associativity(&self) -> u32 { self.ways }
}

/// CPU micro-architecture info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroArch {
    Unknown,
    IntelSkylake,
    IntelIceLake,
    IntelAlderLake,
    IntelSapphireRapids,
    AmdZen2,
    AmdZen3,
    AmdZen4,
    ArmCortexA76,
    ArmCortexA78,
    ArmCortexX2,
    ArmNeoverse,
}

/// CPU feature flags
#[derive(Debug, Clone)]
pub struct CpuFeatures {
    pub bits: [u64; 4],
}

impl CpuFeatures {
    pub const SSE: u32 = 0;
    pub const SSE2: u32 = 1;
    pub const AVX: u32 = 2;
    pub const AVX2: u32 = 3;
    pub const AVX512: u32 = 4;
    pub const AES_NI: u32 = 5;
    pub const TSC: u32 = 6;
    pub const INVARIANT_TSC: u32 = 7;
    pub const RDRAND: u32 = 8;
    pub const RDSEED: u32 = 9;
    pub const CX16: u32 = 10;
    pub const POPCNT: u32 = 11;
    pub const BMI1: u32 = 12;
    pub const BMI2: u32 = 13;
    pub const XSAVE: u32 = 14;
    pub const SMT: u32 = 15;

    pub fn empty() -> Self { Self { bits: [0; 4] } }
    pub fn set(&mut self, feat: u32) { if feat < 256 { self.bits[feat as usize / 64] |= 1 << (feat % 64); } }
    pub fn has(&self, feat: u32) -> bool {
        if feat >= 256 { false } else { (self.bits[feat as usize / 64] >> (feat % 64)) & 1 != 0 }
    }
}

/// Logical CPU descriptor
#[derive(Debug, Clone)]
pub struct LogicalCpu {
    pub cpu_id: u32,
    pub core_id: u32,
    pub package_id: u32,
    pub die_id: u32,
    pub numa_node: u32,
    pub smt_sibling: Option<u32>,
    pub microarch: MicroArch,
    pub base_freq_mhz: u32,
    pub max_freq_mhz: u32,
    pub current_freq_mhz: u32,
    pub online: bool,
    pub caches: Vec<CacheInfo>,
    pub features: CpuFeatures,
}

impl LogicalCpu {
    pub fn new(cpu_id: u32, core_id: u32, package_id: u32, node: u32) -> Self {
        Self {
            cpu_id, core_id, package_id, die_id: 0, numa_node: node,
            smt_sibling: None, microarch: MicroArch::Unknown,
            base_freq_mhz: 0, max_freq_mhz: 0, current_freq_mhz: 0,
            online: true, caches: Vec::new(), features: CpuFeatures::empty(),
        }
    }

    pub fn is_smt_primary(&self) -> bool { self.smt_sibling.is_some() && self.cpu_id < self.smt_sibling.unwrap() }
    pub fn cache_size(&self, level: CacheLevel) -> u32 {
        self.caches.iter().find(|c| c.level == level).map(|c| c.size_kb).unwrap_or(0)
    }
}

/// Topology level distances (for scheduling cost estimation)
#[derive(Debug, Clone)]
pub struct TopologyDistances {
    pub same_core: u32,
    pub same_package: u32,
    pub same_die: u32,
    pub same_numa: u32,
    pub cross_numa: u32,
}

impl TopologyDistances {
    pub fn default_x86() -> Self {
        Self { same_core: 1, same_package: 10, same_die: 5, same_numa: 20, cross_numa: 100 }
    }

    pub fn distance(&self, cpu_a: &LogicalCpu, cpu_b: &LogicalCpu) -> u32 {
        if cpu_a.cpu_id == cpu_b.cpu_id { return 0; }
        if cpu_a.core_id == cpu_b.core_id && cpu_a.package_id == cpu_b.package_id { return self.same_core; }
        if cpu_a.die_id == cpu_b.die_id && cpu_a.package_id == cpu_b.package_id { return self.same_die; }
        if cpu_a.package_id == cpu_b.package_id { return self.same_package; }
        if cpu_a.numa_node == cpu_b.numa_node { return self.same_numa; }
        self.cross_numa
    }
}

/// CPU topology stats
#[derive(Debug, Clone)]
pub struct CpuTopoV2Stats {
    pub total_cpus: u32,
    pub online_cpus: u32,
    pub total_cores: u32,
    pub total_packages: u32,
    pub total_numa_nodes: u32,
    pub smt_enabled: bool,
    pub total_l3_kb: u64,
}

/// Main topology manager
pub struct HolisticCpuTopoV2 {
    cpus: BTreeMap<u32, LogicalCpu>,
    distances: TopologyDistances,
}

impl HolisticCpuTopoV2 {
    pub fn new() -> Self {
        Self { cpus: BTreeMap::new(), distances: TopologyDistances::default_x86() }
    }

    pub fn add_cpu(&mut self, cpu: LogicalCpu) { self.cpus.insert(cpu.cpu_id, cpu); }

    pub fn get_cpu(&self, id: u32) -> Option<&LogicalCpu> { self.cpus.get(&id) }

    pub fn online_cpus(&self) -> Vec<u32> {
        self.cpus.values().filter(|c| c.online).map(|c| c.cpu_id).collect()
    }

    pub fn cpus_in_package(&self, pkg: u32) -> Vec<u32> {
        self.cpus.values().filter(|c| c.package_id == pkg).map(|c| c.cpu_id).collect()
    }

    pub fn cpus_on_node(&self, node: u32) -> Vec<u32> {
        self.cpus.values().filter(|c| c.numa_node == node).map(|c| c.cpu_id).collect()
    }

    pub fn distance(&self, a: u32, b: u32) -> u32 {
        let cpu_a = match self.cpus.get(&a) { Some(c) => c, None => return u32::MAX };
        let cpu_b = match self.cpus.get(&b) { Some(c) => c, None => return u32::MAX };
        self.distances.distance(cpu_a, cpu_b)
    }

    pub fn stats(&self) -> CpuTopoV2Stats {
        let online = self.cpus.values().filter(|c| c.online).count() as u32;
        let cores: Vec<_> = self.cpus.values().map(|c| (c.package_id, c.core_id)).collect();
        let mut unique_cores = cores.clone();
        unique_cores.sort();
        unique_cores.dedup();
        let mut pkgs: Vec<_> = self.cpus.values().map(|c| c.package_id).collect();
        pkgs.sort(); pkgs.dedup();
        let mut nodes: Vec<_> = self.cpus.values().map(|c| c.numa_node).collect();
        nodes.sort(); nodes.dedup();
        let smt = self.cpus.values().any(|c| c.smt_sibling.is_some());
        let l3: u64 = self.cpus.values().map(|c| c.cache_size(CacheLevel::L3) as u64).sum();
        CpuTopoV2Stats {
            total_cpus: self.cpus.len() as u32, online_cpus: online,
            total_cores: unique_cores.len() as u32, total_packages: pkgs.len() as u32,
            total_numa_nodes: nodes.len() as u32, smt_enabled: smt, total_l3_kb: l3,
        }
    }
}
