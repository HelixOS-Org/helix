//! # Apps CPU Flame Profiler
//!
//! CPU stack flame graph data collection:
//! - Stack sample collection and aggregation
//! - Hot path detection
//! - Function-level CPU attribution
//! - Flame graph tree construction
//! - Differential flame analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function address
    pub address: u64,
    /// Symbol name hash (FNV-1a)
    pub symbol_hash: u64,
    /// Module hash
    pub module_hash: u64,
    /// Is kernel frame?
    pub is_kernel: bool,
}

impl StackFrame {
    pub fn new(address: u64, symbol: &str, module: &str) -> Self {
        Self {
            address,
            symbol_hash: Self::fnv_hash(symbol),
            module_hash: Self::fnv_hash(module),
            is_kernel: address >= 0xffff800000000000,
        }
    }

    fn fnv_hash(s: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in s.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Stack sample
#[derive(Debug, Clone)]
pub struct StackSample {
    /// Stack frames (bottom to top)
    pub frames: Vec<StackFrame>,
    /// PID
    pub pid: u64,
    /// TID
    pub tid: u64,
    /// Timestamp (ns)
    pub timestamp_ns: u64,
    /// CPU
    pub cpu: u32,
    /// Weight (for weighted sampling)
    pub weight: u32,
}

/// Flame node in aggregated tree
#[derive(Debug, Clone)]
pub struct FlameNode {
    /// Symbol hash
    pub symbol_hash: u64,
    /// Address
    pub address: u64,
    /// Self count (samples ending here)
    pub self_count: u64,
    /// Total count (samples passing through)
    pub total_count: u64,
    /// Children
    pub children: BTreeMap<u64, FlameNode>,
}

impl FlameNode {
    pub fn new(symbol_hash: u64, address: u64) -> Self {
        Self {
            symbol_hash,
            address,
            self_count: 0,
            total_count: 0,
            children: BTreeMap::new(),
        }
    }

    /// Insert stack (recursive)
    pub fn insert(&mut self, frames: &[StackFrame], weight: u64) {
        self.total_count += weight;
        if frames.is_empty() {
            self.self_count += weight;
            return;
        }
        let frame = &frames[0];
        let child = self.children
            .entry(frame.symbol_hash)
            .or_insert_with(|| FlameNode::new(frame.symbol_hash, frame.address));
        child.insert(&frames[1..], weight);
    }

    /// Self percentage
    pub fn self_pct(&self) -> f64 {
        if self.total_count == 0 { return 0.0; }
        self.self_count as f64 / self.total_count as f64 * 100.0
    }

    /// Depth
    pub fn depth(&self) -> usize {
        if self.children.is_empty() {
            return 1;
        }
        1 + self.children.values().map(|c| c.depth()).max().unwrap_or(0)
    }

    /// Hottest child
    pub fn hottest_child(&self) -> Option<&FlameNode> {
        self.children.values().max_by_key(|c| c.total_count)
    }
}

/// Hot path
#[derive(Debug, Clone)]
pub struct HotPath {
    /// Frame hashes along the path
    pub path: Vec<u64>,
    /// Sample count
    pub count: u64,
    /// Percentage of total
    pub percentage: f64,
}

/// Flame profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppFlameProfilerStats {
    pub total_samples: u64,
    pub unique_stacks: usize,
    pub max_depth: usize,
    pub hot_paths: usize,
    pub kernel_pct: f64,
}

/// Per-process flame profiler
pub struct AppFlameProfiler {
    /// Flame tree root
    root: FlameNode,
    /// Total samples
    total_samples: u64,
    /// Kernel samples
    kernel_samples: u64,
    /// Unique stack hashes
    unique_stacks: BTreeMap<u64, u64>,
    /// Stats
    stats: AppFlameProfilerStats,
}

impl AppFlameProfiler {
    pub fn new() -> Self {
        Self {
            root: FlameNode::new(0, 0),
            total_samples: 0,
            kernel_samples: 0,
            unique_stacks: BTreeMap::new(),
            stats: AppFlameProfilerStats::default(),
        }
    }

    /// Add sample
    pub fn add_sample(&mut self, sample: &StackSample) {
        let weight = sample.weight.max(1) as u64;
        self.total_samples += weight;

        if sample.frames.iter().any(|f| f.is_kernel) {
            self.kernel_samples += weight;
        }

        // Hash the full stack for uniqueness
        let mut stack_hash: u64 = 0xcbf29ce484222325;
        for frame in &sample.frames {
            stack_hash ^= frame.symbol_hash;
            stack_hash = stack_hash.wrapping_mul(0x100000001b3);
        }
        *self.unique_stacks.entry(stack_hash).or_insert(0) += weight;

        // Insert into flame tree (reverse order: bottom frame first)
        self.root.insert(&sample.frames, weight);
        self.update_stats();
    }

    /// Get hot paths
    pub fn hot_paths(&self, top_n: usize) -> Vec<HotPath> {
        let mut paths: Vec<HotPath> = self.unique_stacks.iter()
            .map(|(&hash, &count)| HotPath {
                path: alloc::vec![hash],
                count,
                percentage: if self.total_samples > 0 {
                    count as f64 / self.total_samples as f64 * 100.0
                } else { 0.0 },
            })
            .collect();
        paths.sort_by(|a, b| b.count.cmp(&a.count));
        paths.truncate(top_n);
        paths
    }

    /// Get hottest path from root
    pub fn hottest_trace(&self) -> Vec<u64> {
        let mut path = Vec::new();
        let mut node = &self.root;
        while let Some(child) = node.hottest_child() {
            path.push(child.symbol_hash);
            node = child;
        }
        path
    }

    fn update_stats(&mut self) {
        self.stats.total_samples = self.total_samples;
        self.stats.unique_stacks = self.unique_stacks.len();
        self.stats.max_depth = self.root.depth();
        self.stats.kernel_pct = if self.total_samples > 0 {
            self.kernel_samples as f64 / self.total_samples as f64 * 100.0
        } else { 0.0 };
    }

    pub fn stats(&self) -> &AppFlameProfilerStats {
        &self.stats
    }
}
