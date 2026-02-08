// SPDX-License-Identifier: GPL-2.0
//! Holistic superblock — filesystem superblock management and registration

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Superblock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuperblockState {
    New,
    Active,
    ReadOnly,
    Frozen,
    Unmounting,
    Dead,
}

/// Superblock feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbFeature {
    Journal,
    Encryption,
    Compression,
    Dedup,
    Quota,
    Acl,
    Xattr,
    Casefold,
    Verity,
    Inline,
}

/// Filesystem superblock
#[derive(Debug, Clone)]
pub struct HolisticSuperblock {
    pub sb_id: u64,
    pub fs_type_hash: u64,
    pub dev_id: u64,
    pub state: SuperblockState,
    pub block_size: u32,
    pub max_file_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub features: u32,
    pub mount_count: u32,
    pub mount_time_ns: u64,
}

impl HolisticSuperblock {
    pub fn new(sb_id: u64, fs_type: &[u8], block_size: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in fs_type { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            sb_id, fs_type_hash: h, dev_id: 0, state: SuperblockState::New,
            block_size, max_file_size: u64::MAX, total_blocks: 0, free_blocks: 0,
            total_inodes: 0, free_inodes: 0, features: 0, mount_count: 0, mount_time_ns: 0,
        }
    }

    pub fn activate(&mut self) { self.state = SuperblockState::Active; self.mount_count += 1; }
    pub fn freeze(&mut self) { self.state = SuperblockState::Frozen; }
    pub fn thaw(&mut self) { self.state = SuperblockState::Active; }

    pub fn enable_feature(&mut self, feature: SbFeature) { self.features |= 1u32 << (feature as u32); }
    pub fn has_feature(&self, feature: SbFeature) -> bool { self.features & (1u32 << (feature as u32)) != 0 }

    pub fn used_blocks(&self) -> u64 { self.total_blocks.saturating_sub(self.free_blocks) }
    pub fn usage_pct(&self) -> f64 {
        if self.total_blocks == 0 { 0.0 } else { self.used_blocks() as f64 / self.total_blocks as f64 }
    }

    pub fn inode_usage_pct(&self) -> f64 {
        if self.total_inodes == 0 { 0.0 }
        else { (self.total_inodes - self.free_inodes) as f64 / self.total_inodes as f64 }
    }
}

/// Superblock holistic stats
#[derive(Debug, Clone)]
pub struct HolisticSbStats {
    pub total_superblocks: u64,
    pub active: u64,
    pub frozen: u64,
    pub total_capacity_blocks: u64,
    pub total_free_blocks: u64,
}

/// Main holistic superblock manager
#[derive(Debug)]
pub struct HolisticSuperblockMgr {
    pub superblocks: BTreeMap<u64, HolisticSuperblock>,
    pub stats: HolisticSbStats,
}

impl HolisticSuperblockMgr {
    pub fn new() -> Self {
        Self {
            superblocks: BTreeMap::new(),
            stats: HolisticSbStats { total_superblocks: 0, active: 0, frozen: 0, total_capacity_blocks: 0, total_free_blocks: 0 },
        }
    }

    pub fn register(&mut self, sb: HolisticSuperblock) {
        self.stats.total_superblocks += 1;
        self.stats.total_capacity_blocks += sb.total_blocks;
        self.stats.total_free_blocks += sb.free_blocks;
        self.superblocks.insert(sb.sb_id, sb);
    }

    pub fn activate(&mut self, sb_id: u64) -> bool {
        if let Some(sb) = self.superblocks.get_mut(&sb_id) {
            sb.activate();
            self.stats.active += 1;
            true
        } else { false }
    }

    pub fn overall_usage_pct(&self) -> f64 {
        if self.stats.total_capacity_blocks == 0 { 0.0 }
        else { (self.stats.total_capacity_blocks - self.stats.total_free_blocks) as f64 / self.stats.total_capacity_blocks as f64 }
    }
}

// ============================================================================
// Merged from superblock_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticSbV2Metric {
    FreeBlockRatio,
    FreeInodeRatio,
    FragmentationIndex,
    SyncFrequency,
    ErrorRate,
    UsageGrowthRate,
}

/// Superblock analysis sample
#[derive(Debug, Clone)]
pub struct HolisticSbV2Sample {
    pub metric: HolisticSbV2Metric,
    pub value: u64,
    pub sb_id: u64,
    pub timestamp: u64,
}

/// Superblock health
#[derive(Debug, Clone)]
pub struct HolisticSbV2Health {
    pub space_health: u64,
    pub inode_health: u64,
    pub integrity_score: u64,
    pub overall: u64,
}

/// Stats for superblock analysis
#[derive(Debug, Clone)]
pub struct HolisticSbV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub space_warnings: u64,
    pub integrity_alerts: u64,
}

/// Manager for superblock holistic analysis
pub struct HolisticSuperblockV2Manager {
    samples: Vec<HolisticSbV2Sample>,
    per_sb: BTreeMap<u64, Vec<HolisticSbV2Sample>>,
    health: HolisticSbV2Health,
    stats: HolisticSbV2Stats,
}

impl HolisticSuperblockV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            per_sb: BTreeMap::new(),
            health: HolisticSbV2Health {
                space_health: 100,
                inode_health: 100,
                integrity_score: 100,
                overall: 100,
            },
            stats: HolisticSbV2Stats {
                samples: 0,
                analyses: 0,
                space_warnings: 0,
                integrity_alerts: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticSbV2Metric, value: u64, sb_id: u64) {
        let sample = HolisticSbV2Sample {
            metric,
            value,
            sb_id,
            timestamp: self.samples.len() as u64,
        };
        self.per_sb.entry(sb_id).or_insert_with(Vec::new).push(sample.clone());
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticSbV2Health {
        self.stats.analyses += 1;
        let space_samples: Vec<&HolisticSbV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticSbV2Metric::FreeBlockRatio))
            .collect();
        if !space_samples.is_empty() {
            let avg: u64 = space_samples.iter().map(|s| s.value).sum::<u64>() / space_samples.len() as u64;
            self.health.space_health = avg.min(100);
            if avg < 10 {
                self.stats.space_warnings += 1;
            }
        }
        self.health.overall = (self.health.space_health + self.health.inode_health + self.health.integrity_score) / 3;
        &self.health
    }

    pub fn stats(&self) -> &HolisticSbV2Stats {
        &self.stats
    }
}
