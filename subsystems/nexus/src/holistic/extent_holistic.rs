// SPDX-License-Identifier: GPL-2.0
//! Holistic extent — extent-based allocation analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Extent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtentState {
    Allocated,
    Unwritten,
    Delayed,
    Hole,
}

/// Extent record
#[derive(Debug, Clone)]
pub struct ExtentRecord {
    pub inode: u64,
    pub logical_block: u64,
    pub physical_block: u64,
    pub length: u64,
    pub state: ExtentState,
    pub depth: u8,
}

impl ExtentRecord {
    pub fn new(inode: u64, logical: u64, physical: u64, length: u64) -> Self {
        Self { inode, logical_block: logical, physical_block: physical, length, state: ExtentState::Allocated, depth: 0 }
    }

    pub fn is_contiguous_with(&self, other: &ExtentRecord) -> bool {
        self.physical_block + self.length == other.physical_block
    }

    pub fn size_bytes(&self) -> u64 { self.length * 4096 }
}

/// File fragmentation analysis
#[derive(Debug, Clone)]
pub struct FragmentationAnalysis {
    pub inode: u64,
    pub extent_count: u64,
    pub total_blocks: u64,
    pub contiguous_runs: u64,
    pub holes: u64,
}

impl FragmentationAnalysis {
    pub fn new(inode: u64) -> Self {
        Self { inode, extent_count: 0, total_blocks: 0, contiguous_runs: 0, holes: 0 }
    }

    pub fn add_extent(&mut self, ext: &ExtentRecord) {
        self.extent_count += 1;
        self.total_blocks += ext.length;
        if ext.state == ExtentState::Hole { self.holes += 1; }
    }

    pub fn fragmentation_ratio(&self) -> f64 {
        if self.extent_count <= 1 { 0.0 }
        else { 1.0 - (1.0 / self.extent_count as f64) }
    }
}

/// Holistic extent stats
#[derive(Debug, Clone)]
pub struct HolisticExtentStats {
    pub total_extents: u64,
    pub total_blocks: u64,
    pub fragmented_files: u64,
    pub holes: u64,
}

/// Main holistic extent
#[derive(Debug)]
pub struct HolisticExtent {
    pub files: BTreeMap<u64, FragmentationAnalysis>,
    pub stats: HolisticExtentStats,
}

impl HolisticExtent {
    pub fn new() -> Self {
        Self { files: BTreeMap::new(), stats: HolisticExtentStats { total_extents: 0, total_blocks: 0, fragmented_files: 0, holes: 0 } }
    }

    pub fn record_extent(&mut self, ext: &ExtentRecord) {
        self.stats.total_extents += 1;
        self.stats.total_blocks += ext.length;
        if ext.state == ExtentState::Hole { self.stats.holes += 1; }
        let fa = self.files.entry(ext.inode).or_insert_with(|| FragmentationAnalysis::new(ext.inode));
        fa.add_extent(ext);
        if fa.extent_count > 1 { self.stats.fragmented_files += 1; }
    }
}

// ============================================================================
// Merged from extent_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticExtentV2Metric {
    FragmentationRatio,
    AvgExtentSize,
    ExtentsPerInode,
    AllocationLatency,
    FreeSpaceContiguity,
    SplitRate,
    MergeRate,
}

/// Extent analysis sample
#[derive(Debug, Clone)]
pub struct HolisticExtentV2Sample {
    pub metric: HolisticExtentV2Metric,
    pub value: u64,
    pub inode: u64,
    pub timestamp: u64,
}

/// Extent health assessment
#[derive(Debug, Clone)]
pub struct HolisticExtentV2Health {
    pub fragmentation_score: u64,
    pub allocation_efficiency: u64,
    pub contiguity_score: u64,
    pub overall: u64,
}

/// Stats for extent analysis
#[derive(Debug, Clone)]
pub struct HolisticExtentV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub fragmentation_warnings: u64,
    pub defrag_recommendations: u64,
}

/// Manager for extent holistic analysis
pub struct HolisticExtentV2Manager {
    samples: Vec<HolisticExtentV2Sample>,
    per_inode: BTreeMap<u64, Vec<HolisticExtentV2Sample>>,
    health: HolisticExtentV2Health,
    stats: HolisticExtentV2Stats,
}

impl HolisticExtentV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            per_inode: BTreeMap::new(),
            health: HolisticExtentV2Health {
                fragmentation_score: 0,
                allocation_efficiency: 100,
                contiguity_score: 100,
                overall: 100,
            },
            stats: HolisticExtentV2Stats {
                samples: 0,
                analyses: 0,
                fragmentation_warnings: 0,
                defrag_recommendations: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticExtentV2Metric, value: u64, inode: u64) {
        let sample = HolisticExtentV2Sample {
            metric,
            value,
            inode,
            timestamp: self.samples.len() as u64,
        };
        self.per_inode.entry(inode).or_insert_with(Vec::new).push(sample.clone());
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticExtentV2Health {
        self.stats.analyses += 1;
        let frag: Vec<&HolisticExtentV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticExtentV2Metric::FragmentationRatio))
            .collect();
        if !frag.is_empty() {
            let avg: u64 = frag.iter().map(|s| s.value).sum::<u64>() / frag.len() as u64;
            self.health.fragmentation_score = avg.min(100);
            if avg > 50 {
                self.stats.fragmentation_warnings += 1;
                self.stats.defrag_recommendations += 1;
            }
        }
        self.health.overall = (self.health.allocation_efficiency + self.health.contiguity_score + (100 - self.health.fragmentation_score)) / 3;
        &self.health
    }

    pub fn analyze_inode(&self, inode: u64) -> Option<u64> {
        self.per_inode.get(&inode).map(|samples| {
            if samples.is_empty() { 0 } else {
                samples.iter().map(|s| s.value).sum::<u64>() / samples.len() as u64
            }
        })
    }

    pub fn stats(&self) -> &HolisticExtentV2Stats {
        &self.stats
    }
}
