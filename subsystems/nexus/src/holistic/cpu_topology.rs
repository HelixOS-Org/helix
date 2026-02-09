//! # Holistic CPU Topology
//!
//! CPU topology-aware holistic management:
//! - Core / package / die / NUMA node discovery
//! - Sibling tracking (SMT thread pairs)
//! - Cache sharing topology
//! - Distance matrix between NUMA nodes
//! - Topology-aware placement suggestions
//! - Power-domain grouping

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    L1,
    L2,
    L3,
}

/// CPU core type (hybrid architectures)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreType {
    Performance,
    Efficiency,
    Unknown,
}

/// Logical CPU info
#[derive(Debug, Clone)]
pub struct LogicalCpu {
    pub cpu_id: u32,
    pub core_id: u32,
    pub package_id: u32,
    pub die_id: u32,
    pub numa_node: u32,
    pub core_type: CoreType,
    pub smt_sibling: Option<u32>,
    pub online: bool,
    pub max_freq_khz: u32,
    pub base_freq_khz: u32,
}

impl LogicalCpu {
    pub fn new(cpu_id: u32, core_id: u32, package_id: u32, numa_node: u32) -> Self {
        Self {
            cpu_id,
            core_id,
            package_id,
            die_id: 0,
            numa_node,
            core_type: CoreType::Unknown,
            smt_sibling: None,
            online: true,
            max_freq_khz: 0,
            base_freq_khz: 0,
        }
    }
}

/// Cache domain — group of CPUs sharing a cache
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CacheDomain {
    pub level: CacheLevel,
    pub size_kb: u32,
    pub cpus: Vec<u32>,
    pub ways: u8,
    pub line_size: u16,
}

impl CacheDomain {
    pub fn new(level: CacheLevel, size_kb: u32) -> Self {
        Self {
            level,
            size_kb,
            cpus: Vec::new(),
            ways: 0,
            line_size: 64,
        }
    }

    #[inline]
    pub fn add_cpu(&mut self, cpu_id: u32) {
        if !self.cpus.contains(&cpu_id) {
            self.cpus.push(cpu_id);
        }
    }

    #[inline(always)]
    pub fn contains(&self, cpu_id: u32) -> bool {
        self.cpus.contains(&cpu_id)
    }
}

/// NUMA distance entry
#[derive(Debug, Clone, Copy)]
pub struct NumaDistance {
    pub from_node: u32,
    pub to_node: u32,
    pub distance: u32,
}

/// Package (socket)
#[derive(Debug, Clone)]
pub struct Package {
    pub package_id: u32,
    pub cores: Vec<u32>,
    pub cpus: Vec<u32>,
    pub l3_size_kb: u32,
}

impl Package {
    pub fn new(package_id: u32) -> Self {
        Self {
            package_id,
            cores: Vec::new(),
            cpus: Vec::new(),
            l3_size_kb: 0,
        }
    }

    #[inline(always)]
    pub fn core_count(&self) -> usize {
        self.cores.len()
    }

    #[inline(always)]
    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    #[inline]
    pub fn smt_ratio(&self) -> f64 {
        if self.cores.is_empty() {
            return 1.0;
        }
        self.cpus.len() as f64 / self.cores.len() as f64
    }
}

/// Placement hint based on topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementHint {
    /// Place on same core (SMT sibling)
    SameCore,
    /// Same package, different core
    SamePackage,
    /// Same NUMA node
    SameNuma,
    /// Different NUMA node (for bandwidth)
    CrossNuma,
    /// Any CPU
    Any,
}

/// Topology query result
#[derive(Debug, Clone)]
pub struct TopologyPlacement {
    pub cpu_id: u32,
    pub hint: PlacementHint,
    pub score: f64,
}

/// Holistic topology stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCpuTopologyStats {
    pub total_cpus: usize,
    pub online_cpus: usize,
    pub packages: usize,
    pub numa_nodes: usize,
    pub smt_enabled: bool,
    pub hybrid_arch: bool,
    pub cache_domains: usize,
    pub max_numa_distance: u32,
}

/// Holistic CPU Topology manager
pub struct HolisticCpuTopology {
    cpus: BTreeMap<u32, LogicalCpu>,
    packages: BTreeMap<u32, Package>,
    cache_domains: Vec<CacheDomain>,
    numa_distances: Vec<NumaDistance>,
    numa_nodes: BTreeMap<u32, Vec<u32>>,
    stats: HolisticCpuTopologyStats,
}

impl HolisticCpuTopology {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            packages: BTreeMap::new(),
            cache_domains: Vec::new(),
            numa_distances: Vec::new(),
            numa_nodes: BTreeMap::new(),
            stats: HolisticCpuTopologyStats::default(),
        }
    }

    pub fn add_cpu(&mut self, cpu: LogicalCpu) {
        let pkg_id = cpu.package_id;
        let numa = cpu.numa_node;
        let cpu_id = cpu.cpu_id;
        let core_id = cpu.core_id;

        // Update package
        let pkg = self
            .packages
            .entry(pkg_id)
            .or_insert_with(|| Package::new(pkg_id));
        if !pkg.cpus.contains(&cpu_id) {
            pkg.cpus.push(cpu_id);
        }
        if !pkg.cores.contains(&core_id) {
            pkg.cores.push(core_id);
        }

        // Update NUMA node
        let node_cpus = self.numa_nodes.entry(numa).or_insert_with(Vec::new);
        if !node_cpus.contains(&cpu_id) {
            node_cpus.push(cpu_id);
        }

        self.cpus.insert(cpu_id, cpu);
        self.recompute_stats();
    }

    #[inline(always)]
    pub fn add_cache_domain(&mut self, domain: CacheDomain) {
        self.cache_domains.push(domain);
        self.stats.cache_domains = self.cache_domains.len();
    }

    pub fn set_numa_distance(&mut self, from: u32, to: u32, distance: u32) {
        // Remove existing
        self.numa_distances
            .retain(|d| !(d.from_node == from && d.to_node == to));
        self.numa_distances.push(NumaDistance {
            from_node: from,
            to_node: to,
            distance,
        });

        let max_d = self
            .numa_distances
            .iter()
            .map(|d| d.distance)
            .max()
            .unwrap_or(0);
        self.stats.max_numa_distance = max_d;
    }

    #[inline]
    pub fn get_numa_distance(&self, from: u32, to: u32) -> u32 {
        if from == to {
            return 10;
        } // local
        self.numa_distances
            .iter()
            .find(|d| d.from_node == from && d.to_node == to)
            .map(|d| d.distance)
            .unwrap_or(20)
    }

    /// Find CPUs sharing a cache level with the given CPU
    pub fn cache_siblings(&self, cpu_id: u32, level: CacheLevel) -> Vec<u32> {
        for domain in &self.cache_domains {
            if domain.level == level && domain.contains(cpu_id) {
                return domain
                    .cpus
                    .iter()
                    .copied()
                    .filter(|&c| c != cpu_id)
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get SMT sibling
    #[inline(always)]
    pub fn smt_sibling(&self, cpu_id: u32) -> Option<u32> {
        self.cpus.get(&cpu_id).and_then(|c| c.smt_sibling)
    }

    /// Find best CPUs for placement given a hint
    pub fn suggest_placement(
        &self,
        hint: PlacementHint,
        near_cpu: u32,
        count: usize,
    ) -> Vec<TopologyPlacement> {
        let ref_cpu = match self.cpus.get(&near_cpu) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut candidates: Vec<TopologyPlacement> = Vec::new();

        for (_, cpu) in &self.cpus {
            if !cpu.online || cpu.cpu_id == near_cpu {
                continue;
            }

            let score = match hint {
                PlacementHint::SameCore => {
                    if cpu.core_id == ref_cpu.core_id && cpu.package_id == ref_cpu.package_id {
                        1.0
                    } else {
                        0.0
                    }
                },
                PlacementHint::SamePackage => {
                    if cpu.package_id == ref_cpu.package_id {
                        if cpu.core_id == ref_cpu.core_id {
                            0.9
                        } else {
                            0.7
                        }
                    } else {
                        0.0
                    }
                },
                PlacementHint::SameNuma => {
                    if cpu.numa_node == ref_cpu.numa_node {
                        if cpu.package_id == ref_cpu.package_id {
                            0.9
                        } else {
                            0.7
                        }
                    } else {
                        0.0
                    }
                },
                PlacementHint::CrossNuma => {
                    if cpu.numa_node != ref_cpu.numa_node {
                        let dist = self.get_numa_distance(ref_cpu.numa_node, cpu.numa_node);
                        1.0 / (dist as f64 / 10.0)
                    } else {
                        0.0
                    }
                },
                PlacementHint::Any => 0.5,
            };

            if score > 0.0 {
                candidates.push(TopologyPlacement {
                    cpu_id: cpu.cpu_id,
                    hint,
                    score,
                });
            }
        }

        // Sort by score desc
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        candidates.truncate(count);
        candidates
    }

    /// Get all online CPUs in a NUMA node
    #[inline]
    pub fn numa_cpus(&self, node: u32) -> Vec<u32> {
        self.numa_nodes
            .get(&node)
            .map(|cpus| {
                cpus.iter()
                    .copied()
                    .filter(|&id| self.cpus.get(&id).map(|c| c.online).unwrap_or(false))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if architecture is hybrid (P+E cores)
    #[inline]
    pub fn is_hybrid(&self) -> bool {
        let has_perf = self
            .cpus
            .values()
            .any(|c| c.core_type == CoreType::Performance);
        let has_eff = self
            .cpus
            .values()
            .any(|c| c.core_type == CoreType::Efficiency);
        has_perf && has_eff
    }

    fn recompute_stats(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.online_cpus = self.cpus.values().filter(|c| c.online).count();
        self.stats.packages = self.packages.len();
        self.stats.numa_nodes = self.numa_nodes.len();
        self.stats.smt_enabled = self.cpus.values().any(|c| c.smt_sibling.is_some());
        self.stats.hybrid_arch = self.is_hybrid();
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticCpuTopologyStats {
        &self.stats
    }
}
