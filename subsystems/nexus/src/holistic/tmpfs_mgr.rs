// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic tmpfs manager — Shared memory filesystem tracking
//!
//! Models tmpfs/shmem with page allocation tracking, swap usage,
//! huge page policy, and per-mount size limit enforcement.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Tmpfs huge page policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmpfsHugePolicy {
    Never,
    Always,
    Within,
    Advise,
}

/// Tmpfs mount state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmpfsMountState {
    Active,
    Full,
    ReadOnly,
    Unmounted,
}

/// A tmpfs mount instance.
#[derive(Debug, Clone)]
pub struct TmpfsMountInstance {
    pub mount_id: u64,
    pub name: String,
    pub state: TmpfsMountState,
    pub max_bytes: u64,
    pub used_bytes: u64,
    pub max_inodes: u64,
    pub used_inodes: u64,
    pub huge_policy: TmpfsHugePolicy,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub pages_allocated: u64,
    pub pages_swapped: u64,
    pub huge_pages: u64,
}

impl TmpfsMountInstance {
    pub fn new(mount_id: u64, name: String, max_bytes: u64) -> Self {
        Self {
            mount_id,
            name,
            state: TmpfsMountState::Active,
            max_bytes,
            used_bytes: 0,
            max_inodes: u64::MAX,
            used_inodes: 0,
            huge_policy: TmpfsHugePolicy::Never,
            mode: 0o1777,
            uid: 0,
            gid: 0,
            pages_allocated: 0,
            pages_swapped: 0,
            huge_pages: 0,
        }
    }

    #[inline]
    pub fn allocate(&mut self, bytes: u64) -> bool {
        if self.used_bytes + bytes > self.max_bytes {
            self.state = TmpfsMountState::Full;
            return false;
        }
        self.used_bytes += bytes;
        self.pages_allocated += (bytes + 4095) / 4096;
        true
    }

    #[inline]
    pub fn free(&mut self, bytes: u64) {
        self.used_bytes = self.used_bytes.saturating_sub(bytes);
        let pages = (bytes + 4095) / 4096;
        self.pages_allocated = self.pages_allocated.saturating_sub(pages);
        if self.state == TmpfsMountState::Full {
            self.state = TmpfsMountState::Active;
        }
    }

    #[inline(always)]
    pub fn swap_out(&mut self, pages: u64) {
        self.pages_swapped += pages;
        self.pages_allocated = self.pages_allocated.saturating_sub(pages);
    }

    #[inline(always)]
    pub fn swap_in(&mut self, pages: u64) {
        self.pages_allocated += pages;
        self.pages_swapped = self.pages_swapped.saturating_sub(pages);
    }

    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.max_bytes == 0 {
            return 0.0;
        }
        self.used_bytes as f64 / self.max_bytes as f64
    }
}

/// Statistics for tmpfs manager.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TmpfsMgrStats {
    pub total_mounts: u64,
    pub total_bytes_used: u64,
    pub total_pages: u64,
    pub total_swapped: u64,
    pub total_huge_pages: u64,
    pub full_mounts: u64,
}

/// Main holistic tmpfs manager.
pub struct HolisticTmpfsMgr {
    pub mounts: BTreeMap<u64, TmpfsMountInstance>,
    pub next_mount_id: u64,
    pub stats: TmpfsMgrStats,
}

impl HolisticTmpfsMgr {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            next_mount_id: 1,
            stats: TmpfsMgrStats {
                total_mounts: 0,
                total_bytes_used: 0,
                total_pages: 0,
                total_swapped: 0,
                total_huge_pages: 0,
                full_mounts: 0,
            },
        }
    }

    #[inline]
    pub fn create_mount(&mut self, name: String, max_bytes: u64) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id += 1;
        let mount = TmpfsMountInstance::new(id, name, max_bytes);
        self.mounts.insert(id, mount);
        self.stats.total_mounts += 1;
        id
    }

    pub fn allocate(&mut self, mount_id: u64, bytes: u64) -> bool {
        if let Some(mount) = self.mounts.get_mut(&mount_id) {
            let ok = mount.allocate(bytes);
            if ok {
                self.stats.total_bytes_used += bytes;
                self.stats.total_pages += (bytes + 4095) / 4096;
            }
            ok
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }
}
