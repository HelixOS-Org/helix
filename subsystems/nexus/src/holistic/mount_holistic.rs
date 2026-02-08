// SPDX-License-Identifier: GPL-2.0
//! Holistic mount — mount point management with namespace and propagation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mount type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    Normal,
    Bind,
    Rbind,
    Move,
    Remount,
    Overlay,
}

/// Mount propagation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountPropagation {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Mount flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountFlag {
    ReadOnly,
    NoSuid,
    NoDev,
    NoExec,
    Synchronous,
    MandLock,
    DirSync,
    NoAtime,
    RelatimeAtime,
    StrictAtime,
    NoBdi,
}

/// Mount point
#[derive(Debug, Clone)]
pub struct MountPoint {
    pub mount_id: u64,
    pub parent_id: u64,
    pub mount_type: MountType,
    pub propagation: MountPropagation,
    pub source_hash: u64,
    pub target_hash: u64,
    pub flags: u32,
    pub fs_type_hash: u64,
    pub namespace_id: u64,
    pub children: Vec<u64>,
    pub ref_count: u32,
}

impl MountPoint {
    pub fn new(mount_id: u64, source: &[u8], target: &[u8], mount_type: MountType) -> Self {
        let hash = |data: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in data { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self {
            mount_id, parent_id: 0, mount_type, propagation: MountPropagation::Private,
            source_hash: hash(source), target_hash: hash(target), flags: 0,
            fs_type_hash: 0, namespace_id: 0, children: Vec::new(), ref_count: 1,
        }
    }

    pub fn add_child(&mut self, child_id: u64) { self.children.push(child_id); }
    pub fn is_readonly(&self) -> bool { self.flags & 1 != 0 }
    pub fn is_shared(&self) -> bool { self.propagation == MountPropagation::Shared }
}

/// Mount holistic stats
#[derive(Debug, Clone)]
pub struct HolisticMountStats {
    pub total_mounts: u64,
    pub bind_mounts: u64,
    pub overlay_mounts: u64,
    pub shared_mounts: u64,
    pub namespaces: u64,
}

/// Main holistic mount
#[derive(Debug)]
pub struct HolisticMount {
    pub mounts: BTreeMap<u64, MountPoint>,
    pub stats: HolisticMountStats,
}

impl HolisticMount {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            stats: HolisticMountStats { total_mounts: 0, bind_mounts: 0, overlay_mounts: 0, shared_mounts: 0, namespaces: 0 },
        }
    }

    pub fn mount(&mut self, mp: MountPoint) {
        self.stats.total_mounts += 1;
        match mp.mount_type {
            MountType::Bind | MountType::Rbind => self.stats.bind_mounts += 1,
            MountType::Overlay => self.stats.overlay_mounts += 1,
            _ => {}
        }
        if mp.is_shared() { self.stats.shared_mounts += 1; }
        self.mounts.insert(mp.mount_id, mp);
    }

    pub fn umount(&mut self, mount_id: u64) -> bool {
        self.mounts.remove(&mount_id).is_some()
    }
}

// ============================================================================
// Merged from mount_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticMountV2Metric {
    MountCount,
    NamespaceDepth,
    PropagationEvents,
    MountLatency,
    UnmountLatency,
    BindMountChain,
}

/// Mount analysis sample
#[derive(Debug, Clone)]
pub struct HolisticMountV2Sample {
    pub metric: HolisticMountV2Metric,
    pub value: u64,
    pub mount_id: u64,
    pub timestamp: u64,
}

/// Mount health assessment
#[derive(Debug, Clone)]
pub struct HolisticMountV2Health {
    pub namespace_health: u64,
    pub performance_score: u64,
    pub complexity_score: u64,
    pub overall: u64,
}

/// Stats for mount holistic analysis
#[derive(Debug, Clone)]
pub struct HolisticMountV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub complexity_warnings: u64,
    pub namespace_alerts: u64,
}

/// Manager for mount holistic analysis
pub struct HolisticMountV2Manager {
    samples: Vec<HolisticMountV2Sample>,
    health: HolisticMountV2Health,
    stats: HolisticMountV2Stats,
}

impl HolisticMountV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticMountV2Health {
                namespace_health: 100,
                performance_score: 100,
                complexity_score: 0,
                overall: 100,
            },
            stats: HolisticMountV2Stats {
                samples: 0,
                analyses: 0,
                complexity_warnings: 0,
                namespace_alerts: 0,
            },
        }
    }

    pub fn record(&mut self, metric: HolisticMountV2Metric, value: u64, mount_id: u64) {
        let sample = HolisticMountV2Sample {
            metric,
            value,
            mount_id,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticMountV2Health {
        self.stats.analyses += 1;
        let depth_samples: u64 = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticMountV2Metric::NamespaceDepth))
            .map(|s| s.value)
            .sum();
        let count = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticMountV2Metric::NamespaceDepth))
            .count() as u64;
        if count > 0 {
            let avg_depth = depth_samples / count;
            self.health.complexity_score = avg_depth.min(100);
            if avg_depth > 32 {
                self.stats.complexity_warnings += 1;
            }
        }
        self.health.overall = (self.health.namespace_health + self.health.performance_score) / 2;
        &self.health
    }

    pub fn stats(&self) -> &HolisticMountV2Stats {
        &self.stats
    }
}
