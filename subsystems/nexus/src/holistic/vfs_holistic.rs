// SPDX-License-Identifier: GPL-2.0
//! Holistic VFS — virtual filesystem switch with mount resolution and path walk

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// VFS operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsOpType {
    Open,
    Close,
    Read,
    Write,
    Stat,
    Readdir,
    Mkdir,
    Rmdir,
    Unlink,
    Rename,
    Link,
    Symlink,
    Mount,
    Umount,
    Ioctl,
    Fsync,
}

/// VFS filesystem type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsFsType {
    Ext4,
    Btrfs,
    Xfs,
    Tmpfs,
    Procfs,
    Sysfs,
    Devtmpfs,
    Nfs,
    Fuse,
    Overlayfs,
}

/// Path walk state
#[derive(Debug, Clone)]
pub struct PathWalkState {
    pub components_resolved: u32,
    pub symlinks_followed: u32,
    pub mounts_crossed: u32,
    pub total_lookups: u64,
    pub cache_hits: u64,
}

impl PathWalkState {
    pub fn new() -> Self {
        Self { components_resolved: 0, symlinks_followed: 0, mounts_crossed: 0, total_lookups: 0, cache_hits: 0 }
    }

    pub fn resolve_component(&mut self, cached: bool) {
        self.components_resolved += 1;
        self.total_lookups += 1;
        if cached { self.cache_hits += 1; }
    }

    pub fn follow_symlink(&mut self) { self.symlinks_followed += 1; }
    pub fn cross_mount(&mut self) { self.mounts_crossed += 1; }

    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_lookups == 0 { 0.0 } else { self.cache_hits as f64 / self.total_lookups as f64 }
    }
}

/// VFS operation record
#[derive(Debug, Clone)]
pub struct VfsOpRecord {
    pub op: VfsOpType,
    pub path_hash: u64,
    pub fs_type: VfsFsType,
    pub inode: u64,
    pub latency_ns: u64,
    pub bytes_transferred: u64,
    pub walk: PathWalkState,
}

impl VfsOpRecord {
    pub fn new(op: VfsOpType, path: &[u8], fs_type: VfsFsType) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, path_hash: h, fs_type, inode: 0, latency_ns: 0, bytes_transferred: 0, walk: PathWalkState::new() }
    }
}

/// VFS stats
#[derive(Debug, Clone)]
pub struct HolisticVfsStats {
    pub total_ops: u64,
    pub total_bytes: u64,
    pub path_walks: u64,
    pub symlinks_followed: u64,
    pub ops_by_type: BTreeMap<u8, u64>,
}

/// Main holistic VFS
#[derive(Debug)]
pub struct HolisticVfs {
    pub stats: HolisticVfsStats,
}

impl HolisticVfs {
    pub fn new() -> Self {
        Self {
            stats: HolisticVfsStats {
                total_ops: 0, total_bytes: 0, path_walks: 0, symlinks_followed: 0, ops_by_type: BTreeMap::new(),
            },
        }
    }

    pub fn record_op(&mut self, record: &VfsOpRecord) {
        self.stats.total_ops += 1;
        self.stats.total_bytes += record.bytes_transferred;
        self.stats.path_walks += 1;
        self.stats.symlinks_followed += record.walk.symlinks_followed as u64;
        *self.stats.ops_by_type.entry(record.op as u8).or_insert(0) += 1;
    }
}

// ============================================================================
// Merged from vfs_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticVfsV2Dimension {
    Throughput,
    Latency,
    CacheEfficiency,
    LockContention,
    PathResolution,
    NamespaceOverhead,
    MountComplexity,
}

/// VFS analysis data point
#[derive(Debug, Clone)]
pub struct HolisticVfsV2Sample {
    pub dimension: HolisticVfsV2Dimension,
    pub value: u64,
    pub timestamp: u64,
    pub weight: u32,
}

/// VFS holistic health score
#[derive(Debug, Clone)]
pub struct HolisticVfsV2Health {
    pub overall_score: u64,
    pub throughput_score: u64,
    pub latency_score: u64,
    pub cache_score: u64,
    pub contention_score: u64,
}

/// Stats for VFS holistic analysis
#[derive(Debug, Clone)]
pub struct HolisticVfsV2Stats {
    pub samples_collected: u64,
    pub analyses_run: u64,
    pub anomalies_detected: u64,
    pub recommendations: u64,
    pub health_checks: u64,
}

/// Manager for VFS holistic analysis
pub struct HolisticVfsV2Manager {
    samples: BTreeMap<u64, Vec<HolisticVfsV2Sample>>,
    health: HolisticVfsV2Health,
    next_id: u64,
    stats: HolisticVfsV2Stats,
}

impl HolisticVfsV2Manager {
    pub fn new() -> Self {
        Self {
            samples: BTreeMap::new(),
            health: HolisticVfsV2Health {
                overall_score: 100,
                throughput_score: 100,
                latency_score: 100,
                cache_score: 100,
                contention_score: 100,
            },
            next_id: 1,
            stats: HolisticVfsV2Stats {
                samples_collected: 0,
                analyses_run: 0,
                anomalies_detected: 0,
                recommendations: 0,
                health_checks: 0,
            },
        }
    }

    pub fn record_sample(&mut self, dimension: HolisticVfsV2Dimension, value: u64) {
        let id = self.next_id;
        self.next_id += 1;
        let sample = HolisticVfsV2Sample {
            dimension,
            value,
            timestamp: id.wrapping_mul(31),
            weight: 1,
        };
        self.samples.entry(dimension as u64).or_insert_with(Vec::new).push(sample);
        self.stats.samples_collected += 1;
    }

    pub fn analyze(&mut self) -> &HolisticVfsV2Health {
        self.stats.analyses_run += 1;
        let mut total = 0u64;
        let mut count = 0u64;
        for (_, samples) in &self.samples {
            for s in samples {
                total += s.value;
                count += 1;
            }
        }
        if count > 0 {
            self.health.overall_score = (total / count).min(100);
        }
        &self.health
    }

    pub fn health_check(&mut self) -> &HolisticVfsV2Health {
        self.stats.health_checks += 1;
        &self.health
    }

    pub fn stats(&self) -> &HolisticVfsV2Stats {
        &self.stats
    }
}
