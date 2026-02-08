//! # System Topology Awareness
//!
//! Hardware topology modeling:
//! - NUMA topology
//! - Cache hierarchy
//! - Device proximity / interconnect
//! - CPU package / core / thread mapping
//! - Memory controller mapping
//! - Affinity-aware placement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CPU TOPOLOGY
// ============================================================================

/// CPU thread (logical core)
#[derive(Debug, Clone, Copy)]
pub struct LogicalCpu {
    /// Logical CPU ID
    pub id: u32,
    /// Physical core ID
    pub core_id: u32,
    /// Package (socket) ID
    pub package_id: u32,
    /// NUMA node
    pub numa_node: u32,
    /// SMT sibling (if any)
    pub smt_sibling: Option<u32>,
    /// L1 cache ID
    pub l1_cache: u32,
    /// L2 cache ID
    pub l2_cache: u32,
    /// L3 cache ID
    pub l3_cache: u32,
    /// Online status
    pub online: bool,
}

/// Physical core
#[derive(Debug, Clone)]
pub struct PhysicalCore {
    /// Core ID
    pub id: u32,
    /// Package ID
    pub package_id: u32,
    /// Logical CPUs on this core
    pub threads: Vec<u32>,
    /// Base frequency (MHz)
    pub base_freq_mhz: u32,
    /// Max frequency (MHz)
    pub max_freq_mhz: u32,
}

/// CPU package (socket)
#[derive(Debug, Clone)]
pub struct CpuPackage {
    /// Package ID
    pub id: u32,
    /// NUMA node
    pub numa_node: u32,
    /// Physical cores in this package
    pub cores: Vec<u32>,
    /// L3 cache size (bytes)
    pub l3_cache_size: u64,
    /// TDP (watts)
    pub tdp_watts: u32,
}

// ============================================================================
// NUMA TOPOLOGY
// ============================================================================

/// NUMA node
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: u32,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Free memory (bytes)
    pub free_memory: u64,
    /// CPU package IDs on this node
    pub packages: Vec<u32>,
    /// Logical CPU IDs on this node
    pub cpus: Vec<u32>,
    /// Memory controllers
    pub memory_controllers: Vec<u32>,
}

/// Distance between two NUMA nodes (latency relative units)
#[derive(Debug, Clone, Copy)]
pub struct NumaDistance {
    pub source: u32,
    pub target: u32,
    /// Relative latency (10 = local, higher = farther)
    pub distance: u32,
}

/// NUMA policy for allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPolicy {
    /// Allocate on the local node
    Local,
    /// Preferred node, fall back to others
    Preferred(u32),
    /// Interleave across all nodes
    Interleave,
    /// Bind to specific node only
    Bind(u32),
    /// Follow first-touch
    FirstTouch,
    /// Migrate on next touch
    MigrateOnTouch,
}

// ============================================================================
// CACHE HIERARCHY
// ============================================================================

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    L1Data,
    L1Instruction,
    L2Unified,
    L3Unified,
    L4Unified,
}

/// Cache descriptor
#[derive(Debug, Clone)]
pub struct CacheDescriptor {
    /// Cache ID
    pub id: u32,
    /// Cache level
    pub level: CacheLevel,
    /// Size (bytes)
    pub size: u64,
    /// Line size (bytes)
    pub line_size: u32,
    /// Associativity
    pub associativity: u32,
    /// Shared by which CPUs
    pub shared_by: Vec<u32>,
    /// Inclusive (contains lower levels)
    pub inclusive: bool,
}

// ============================================================================
// DEVICE TOPOLOGY
// ============================================================================

/// Device interconnect type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterconnectType {
    /// PCIe
    Pcie,
    /// USB
    Usb,
    /// SATA
    Sata,
    /// NVMe
    Nvme,
    /// Internal bus
    Internal,
    /// Network
    Network,
}

/// Device in the topology
#[derive(Debug, Clone)]
pub struct TopologyDevice {
    /// Device ID
    pub id: u32,
    /// Device type
    pub device_type: u32,
    /// NUMA node
    pub numa_node: u32,
    /// Interconnect type
    pub interconnect: InterconnectType,
    /// PCIe bus/device/function (if applicable)
    pub pci_bdf: Option<(u8, u8, u8)>,
    /// Max bandwidth (bytes/sec)
    pub max_bandwidth: u64,
    /// Latency (nanoseconds)
    pub latency_ns: u64,
}

// ============================================================================
// PROXIMITY
// ============================================================================

/// Proximity between two entities (lower = closer)
#[derive(Debug, Clone, Copy)]
pub struct Proximity {
    /// Source entity
    pub source: u32,
    /// Target entity
    pub target: u32,
    /// Proximity score (0 = same, higher = farther)
    pub score: u32,
}

/// Proximity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProximityLevel {
    /// Same core (SMT sibling)
    SameCore = 0,
    /// Same L2 cache
    SameL2 = 1,
    /// Same L3 cache / package
    SamePackage = 2,
    /// Same NUMA node
    SameNumaNode = 3,
    /// Adjacent NUMA node
    AdjacentNode = 4,
    /// Remote NUMA node
    RemoteNode = 5,
}

impl ProximityLevel {
    /// Migration cost relative units
    pub fn migration_cost(&self) -> u32 {
        match self {
            Self::SameCore => 1,
            Self::SameL2 => 5,
            Self::SamePackage => 20,
            Self::SameNumaNode => 50,
            Self::AdjacentNode => 200,
            Self::RemoteNode => 500,
        }
    }

    /// Cache benefit (probability of cache hits after migration)
    pub fn cache_benefit(&self) -> f64 {
        match self {
            Self::SameCore => 0.95,
            Self::SameL2 => 0.80,
            Self::SamePackage => 0.50,
            Self::SameNumaNode => 0.20,
            Self::AdjacentNode => 0.05,
            Self::RemoteNode => 0.01,
        }
    }
}

// ============================================================================
// TOPOLOGY MANAGER
// ============================================================================

/// System topology manager
pub struct TopologyManager {
    /// Logical CPUs
    cpus: BTreeMap<u32, LogicalCpu>,
    /// Physical cores
    cores: BTreeMap<u32, PhysicalCore>,
    /// CPU packages
    packages: BTreeMap<u32, CpuPackage>,
    /// NUMA nodes
    numa_nodes: BTreeMap<u32, NumaNode>,
    /// NUMA distances
    numa_distances: Vec<NumaDistance>,
    /// Cache descriptors
    caches: BTreeMap<u32, CacheDescriptor>,
    /// Devices
    devices: BTreeMap<u32, TopologyDevice>,
    /// Total logical CPUs
    pub total_cpus: u32,
    /// Total physical cores
    pub total_cores: u32,
    /// Total NUMA nodes
    pub total_numa_nodes: u32,
}

impl TopologyManager {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            cores: BTreeMap::new(),
            packages: BTreeMap::new(),
            numa_nodes: BTreeMap::new(),
            numa_distances: Vec::new(),
            caches: BTreeMap::new(),
            devices: BTreeMap::new(),
            total_cpus: 0,
            total_cores: 0,
            total_numa_nodes: 0,
        }
    }

    /// Register logical CPU
    pub fn add_cpu(&mut self, cpu: LogicalCpu) {
        self.cpus.insert(cpu.id, cpu);
        self.total_cpus = self.cpus.len() as u32;
    }

    /// Register physical core
    pub fn add_core(&mut self, core: PhysicalCore) {
        self.cores.insert(core.id, core);
        self.total_cores = self.cores.len() as u32;
    }

    /// Register package
    pub fn add_package(&mut self, pkg: CpuPackage) {
        self.packages.insert(pkg.id, pkg);
    }

    /// Register NUMA node
    pub fn add_numa_node(&mut self, node: NumaNode) {
        self.numa_nodes.insert(node.id, node);
        self.total_numa_nodes = self.numa_nodes.len() as u32;
    }

    /// Add NUMA distance
    pub fn add_numa_distance(&mut self, source: u32, target: u32, distance: u32) {
        self.numa_distances.push(NumaDistance {
            source,
            target,
            distance,
        });
    }

    /// Register cache
    pub fn add_cache(&mut self, cache: CacheDescriptor) {
        self.caches.insert(cache.id, cache);
    }

    /// Register device
    pub fn add_device(&mut self, device: TopologyDevice) {
        self.devices.insert(device.id, device);
    }

    /// Get NUMA distance between two nodes
    pub fn numa_distance(&self, source: u32, target: u32) -> u32 {
        if source == target {
            return 10; // Local access
        }
        self.numa_distances
            .iter()
            .find(|d| d.source == source && d.target == target)
            .map(|d| d.distance)
            .unwrap_or(100) // Default remote
    }

    /// Determine proximity between two CPUs
    pub fn cpu_proximity(&self, cpu_a: u32, cpu_b: u32) -> ProximityLevel {
        let a = match self.cpus.get(&cpu_a) {
            Some(c) => c,
            None => return ProximityLevel::RemoteNode,
        };
        let b = match self.cpus.get(&cpu_b) {
            Some(c) => c,
            None => return ProximityLevel::RemoteNode,
        };

        if a.core_id == b.core_id {
            ProximityLevel::SameCore
        } else if a.l2_cache == b.l2_cache {
            ProximityLevel::SameL2
        } else if a.l3_cache == b.l3_cache || a.package_id == b.package_id {
            ProximityLevel::SamePackage
        } else if a.numa_node == b.numa_node {
            ProximityLevel::SameNumaNode
        } else {
            let dist = self.numa_distance(a.numa_node, b.numa_node);
            if dist <= 20 {
                ProximityLevel::AdjacentNode
            } else {
                ProximityLevel::RemoteNode
            }
        }
    }

    /// Get CPUs on a NUMA node
    pub fn cpus_on_node(&self, numa_node: u32) -> Vec<u32> {
        self.cpus
            .values()
            .filter(|c| c.numa_node == numa_node && c.online)
            .map(|c| c.id)
            .collect()
    }

    /// Get cores on a package
    pub fn cores_on_package(&self, package_id: u32) -> Vec<u32> {
        self.cores
            .values()
            .filter(|c| c.package_id == package_id)
            .map(|c| c.id)
            .collect()
    }

    /// Find best NUMA node for allocation
    pub fn best_numa_node_for_cpu(&self, cpu_id: u32) -> u32 {
        self.cpus
            .get(&cpu_id)
            .map(|c| c.numa_node)
            .unwrap_or(0)
    }

    /// Get NUMA node with most free memory
    pub fn node_with_most_free_memory(&self) -> Option<u32> {
        self.numa_nodes
            .values()
            .max_by_key(|n| n.free_memory)
            .map(|n| n.id)
    }

    /// Find closest device to CPU
    pub fn closest_device_to_cpu(&self, cpu_id: u32, device_type: u32) -> Option<u32> {
        let cpu_node = self.cpus.get(&cpu_id)?.numa_node;

        let mut best: Option<(u32, u32)> = None;

        for device in self.devices.values() {
            if device.device_type != device_type {
                continue;
            }
            let dist = self.numa_distance(cpu_node, device.numa_node);
            match best {
                None => best = Some((device.id, dist)),
                Some((_, d)) if dist < d => best = Some((device.id, dist)),
                _ => {}
            }
        }

        best.map(|(id, _)| id)
    }

    /// Get topology summary
    pub fn summary(&self) -> TopologySummary {
        let smt_enabled = self
            .cpus
            .values()
            .any(|c| c.smt_sibling.is_some());

        let total_l3_cache: u64 = self
            .caches
            .values()
            .filter(|c| c.level == CacheLevel::L3Unified)
            .map(|c| c.size)
            .sum();

        let total_memory: u64 = self.numa_nodes.values().map(|n| n.total_memory).sum();

        TopologySummary {
            total_cpus: self.total_cpus,
            total_cores: self.total_cores,
            total_packages: self.packages.len() as u32,
            total_numa_nodes: self.total_numa_nodes,
            smt_enabled,
            total_l3_cache,
            total_memory,
            total_devices: self.devices.len() as u32,
        }
    }
}

/// Topology summary
#[derive(Debug, Clone)]
pub struct TopologySummary {
    pub total_cpus: u32,
    pub total_cores: u32,
    pub total_packages: u32,
    pub total_numa_nodes: u32,
    pub smt_enabled: bool,
    pub total_l3_cache: u64,
    pub total_memory: u64,
    pub total_devices: u32,
}
