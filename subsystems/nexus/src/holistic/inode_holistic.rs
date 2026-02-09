// SPDX-License-Identifier: GPL-2.0
//! Holistic inode — inode lifecycle management with allocation and eviction

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Inode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
}

/// Inode state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeState {
    New,
    Active,
    Dirty,
    Locked,
    Freeing,
    Evicted,
    Reclaiming,
}

/// Inode data
#[derive(Debug, Clone)]
pub struct HolisticInode {
    pub ino: u64,
    pub inode_type: InodeType,
    pub state: InodeState,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub blocks: u64,
    pub nlink: u32,
    pub ref_count: u32,
    pub atime_ns: u64,
    pub mtime_ns: u64,
    pub ctime_ns: u64,
    pub dirty_pages: u32,
    pub generation: u64,
}

impl HolisticInode {
    pub fn new(ino: u64, inode_type: InodeType, mode: u16) -> Self {
        Self {
            ino, inode_type, state: InodeState::New, mode, uid: 0, gid: 0,
            size: 0, blocks: 0, nlink: 1, ref_count: 1, atime_ns: 0,
            mtime_ns: 0, ctime_ns: 0, dirty_pages: 0, generation: 0,
        }
    }

    #[inline(always)]
    pub fn mark_dirty(&mut self) { self.state = InodeState::Dirty; }
    #[inline(always)]
    pub fn activate(&mut self) { self.state = InodeState::Active; }
    #[inline(always)]
    pub fn grab(&mut self) { self.ref_count += 1; }
    #[inline(always)]
    pub fn put(&mut self) { self.ref_count = self.ref_count.saturating_sub(1); }

    #[inline(always)]
    pub fn is_orphan(&self) -> bool { self.nlink == 0 }
    #[inline(always)]
    pub fn is_reclaimable(&self) -> bool { self.ref_count == 0 && self.state != InodeState::Dirty }

    #[inline(always)]
    pub fn block_usage_bytes(&self) -> u64 { self.blocks * 512 }
}

/// Inode allocator
#[derive(Debug, Clone)]
pub struct InodeAllocator {
    pub next_ino: u64,
    pub free_list: Vec<u64>,
    pub total_allocated: u64,
    pub total_freed: u64,
}

impl InodeAllocator {
    pub fn new(start_ino: u64) -> Self {
        Self { next_ino: start_ino, free_list: Vec::new(), total_allocated: 0, total_freed: 0 }
    }

    #[inline]
    pub fn alloc(&mut self) -> u64 {
        self.total_allocated += 1;
        if let Some(ino) = self.free_list.pop() { ino }
        else { let ino = self.next_ino; self.next_ino += 1; ino }
    }

    #[inline(always)]
    pub fn free(&mut self, ino: u64) {
        self.free_list.push(ino);
        self.total_freed += 1;
    }

    #[inline(always)]
    pub fn in_use(&self) -> u64 { self.total_allocated - self.total_freed }
}

/// Holistic inode stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticInodeStats {
    pub total_inodes: u64,
    pub dirty_inodes: u64,
    pub orphan_inodes: u64,
    pub evictions: u64,
    pub active: u64,
}

/// Main holistic inode manager
#[derive(Debug)]
pub struct HolisticInodeMgr {
    pub inodes: BTreeMap<u64, HolisticInode>,
    pub allocator: InodeAllocator,
    pub stats: HolisticInodeStats,
}

impl HolisticInodeMgr {
    pub fn new() -> Self {
        Self {
            inodes: BTreeMap::new(),
            allocator: InodeAllocator::new(1),
            stats: HolisticInodeStats { total_inodes: 0, dirty_inodes: 0, orphan_inodes: 0, evictions: 0, active: 0 },
        }
    }

    #[inline]
    pub fn create(&mut self, inode_type: InodeType, mode: u16) -> u64 {
        let ino = self.allocator.alloc();
        self.inodes.insert(ino, HolisticInode::new(ino, inode_type, mode));
        self.stats.total_inodes += 1;
        self.stats.active += 1;
        ino
    }

    pub fn evict(&mut self, ino: u64) -> bool {
        if let Some(inode) = self.inodes.get(&ino) {
            if inode.is_reclaimable() {
                self.inodes.remove(&ino);
                self.allocator.free(ino);
                self.stats.evictions += 1;
                self.stats.active -= 1;
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Merged from inode_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticInodeV2Metric {
    AllocationRate,
    FreeRate,
    CacheHitRatio,
    DirtyRatio,
    WritebackLatency,
    FragmentationIndex,
    LockWaitTime,
}

/// Inode analysis sample
#[derive(Debug, Clone)]
pub struct HolisticInodeV2Sample {
    pub metric: HolisticInodeV2Metric,
    pub value: u64,
    pub inode_range_start: u64,
    pub inode_range_end: u64,
    pub timestamp: u64,
}

/// Inode health assessment
#[derive(Debug, Clone)]
pub struct HolisticInodeV2Health {
    pub allocation_health: u64,
    pub cache_health: u64,
    pub fragmentation_health: u64,
    pub overall: u64,
}

/// Stats for inode holistic analysis
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticInodeV2Stats {
    pub samples_collected: u64,
    pub analyses_run: u64,
    pub threshold_alerts: u64,
    pub optimization_suggestions: u64,
}

/// Manager for inode holistic analysis
pub struct HolisticInodeV2Manager {
    samples: Vec<HolisticInodeV2Sample>,
    health: HolisticInodeV2Health,
    thresholds: LinearMap<u64, 64>,
    stats: HolisticInodeV2Stats,
}

impl HolisticInodeV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticInodeV2Health {
                allocation_health: 100,
                cache_health: 100,
                fragmentation_health: 100,
                overall: 100,
            },
            thresholds: LinearMap::new(),
            stats: HolisticInodeV2Stats {
                samples_collected: 0,
                analyses_run: 0,
                threshold_alerts: 0,
                optimization_suggestions: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, metric: HolisticInodeV2Metric, value: u64, range_start: u64, range_end: u64) {
        let sample = HolisticInodeV2Sample {
            metric,
            value,
            inode_range_start: range_start,
            inode_range_end: range_end,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples_collected += 1;
    }

    #[inline(always)]
    pub fn set_threshold(&mut self, metric: HolisticInodeV2Metric, threshold: u64) {
        self.thresholds.insert(metric as u64, threshold);
    }

    pub fn analyze(&mut self) -> &HolisticInodeV2Health {
        self.stats.analyses_run += 1;
        if !self.samples.is_empty() {
            let sum: u64 = self.samples.iter().map(|s| s.value).sum();
            let avg = sum / self.samples.len() as u64;
            self.health.overall = avg.min(100);
        }
        for sample in &self.samples {
            if let Some(&threshold) = self.thresholds.get(&(sample.metric as u64)) {
                if sample.value > threshold {
                    self.stats.threshold_alerts += 1;
                }
            }
        }
        &self.health
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticInodeV2Stats {
        &self.stats
    }
}
