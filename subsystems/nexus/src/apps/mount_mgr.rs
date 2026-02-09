// SPDX-License-Identifier: GPL-2.0
//! Apps mount_mgr — per-application mount namespace and filesystem mount tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Mount type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    Bind,
    RBind,
    Move,
    Remount,
    NewFs,
    Overlay,
    Tmpfs,
    Proc,
    Sysfs,
    Devpts,
    Cgroup,
}

/// Mount propagation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationType {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Mount flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountFlags(pub u32);

impl MountFlags {
    pub const RDONLY: u32 = 1 << 0;
    pub const NOSUID: u32 = 1 << 1;
    pub const NODEV: u32 = 1 << 2;
    pub const NOEXEC: u32 = 1 << 3;
    pub const SYNCHRONOUS: u32 = 1 << 4;
    pub const NOATIME: u32 = 1 << 5;
    pub const NODIRATIME: u32 = 1 << 6;
    pub const RELATIME: u32 = 1 << 7;
    pub const STRICTATIME: u32 = 1 << 8;
    pub const LAZYTIME: u32 = 1 << 9;
    pub const DIRSYNC: u32 = 1 << 10;
    pub const MANDLOCK: u32 = 1 << 11;

    #[inline(always)]
    pub fn contains(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    #[inline(always)]
    pub fn is_readonly(&self) -> bool {
        self.contains(Self::RDONLY)
    }

    #[inline]
    pub fn security_score(&self) -> u32 {
        let mut score = 0u32;
        if self.contains(Self::NOSUID) { score += 2; }
        if self.contains(Self::NODEV) { score += 2; }
        if self.contains(Self::NOEXEC) { score += 3; }
        if self.contains(Self::RDONLY) { score += 3; }
        score
    }
}

/// A mount point entry
#[derive(Debug, Clone)]
pub struct MountEntry {
    pub mount_id: u64,
    pub parent_id: u64,
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub mount_type: MountType,
    pub flags: MountFlags,
    pub propagation: PropagationType,
    pub mount_ns: u64,
    pub created_ns: u64,
    pub io_reads: u64,
    pub io_writes: u64,
    pub errors: u32,
}

impl MountEntry {
    pub fn new(mount_id: u64, mount_point: String, fs_type: String, mount_type: MountType) -> Self {
        Self {
            mount_id,
            parent_id: 0,
            device: String::new(),
            mount_point,
            fs_type,
            mount_type,
            flags: MountFlags(0),
            propagation: PropagationType::Private,
            mount_ns: 0,
            created_ns: 0,
            io_reads: 0,
            io_writes: 0,
            errors: 0,
        }
    }

    #[inline]
    pub fn record_io(&mut self, is_write: bool) {
        if is_write {
            self.io_writes += 1;
        } else {
            self.io_reads += 1;
        }
    }

    #[inline(always)]
    pub fn total_io(&self) -> u64 {
        self.io_reads + self.io_writes
    }

    #[inline]
    pub fn write_ratio(&self) -> f64 {
        let total = self.total_io();
        if total == 0 { return 0.0; }
        self.io_writes as f64 / total as f64
    }

    #[inline(always)]
    pub fn is_pseudo_fs(&self) -> bool {
        matches!(self.mount_type, MountType::Proc | MountType::Sysfs | MountType::Devpts | MountType::Cgroup)
    }

    #[inline(always)]
    pub fn is_overlay_or_tmpfs(&self) -> bool {
        matches!(self.mount_type, MountType::Overlay | MountType::Tmpfs)
    }

    #[inline(always)]
    pub fn is_child_of(&self, path: &str) -> bool {
        self.mount_point.starts_with(path)
    }
}

/// Mount namespace descriptor
#[derive(Debug)]
pub struct MountNamespace {
    pub ns_id: u64,
    pub owner_pid: u64,
    mounts: Vec<u64>,
    pub created_ns: u64,
}

impl MountNamespace {
    pub fn new(ns_id: u64, owner_pid: u64, created_ns: u64) -> Self {
        Self {
            ns_id,
            owner_pid,
            mounts: Vec::new(),
            created_ns,
        }
    }

    #[inline]
    pub fn add_mount(&mut self, mount_id: u64) {
        if !self.mounts.contains(&mount_id) {
            self.mounts.push(mount_id);
        }
    }

    #[inline]
    pub fn remove_mount(&mut self, mount_id: u64) {
        if let Some(pos) = self.mounts.iter().position(|&m| m == mount_id) {
            self.mounts.swap_remove(pos);
        }
    }

    #[inline(always)]
    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }
}

/// Per-app mount state
#[derive(Debug)]
#[repr(align(64))]
pub struct AppMountState {
    pub pid: u64,
    pub namespace_id: u64,
    pub mount_ops: u64,
    pub umount_ops: u64,
    pub mount_failures: u64,
    pub chroot_path: Option<String>,
    pub pivot_root_done: bool,
}

impl AppMountState {
    pub fn new(pid: u64, ns_id: u64) -> Self {
        Self {
            pid,
            namespace_id: ns_id,
            mount_ops: 0,
            umount_ops: 0,
            mount_failures: 0,
            chroot_path: None,
            pivot_root_done: false,
        }
    }

    #[inline(always)]
    pub fn total_ops(&self) -> u64 {
        self.mount_ops + self.umount_ops
    }

    #[inline(always)]
    pub fn failure_rate(&self) -> f64 {
        if self.mount_ops == 0 { return 0.0; }
        self.mount_failures as f64 / self.mount_ops as f64
    }

    #[inline(always)]
    pub fn is_sandboxed(&self) -> bool {
        self.chroot_path.is_some() || self.pivot_root_done
    }
}

/// Mount manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MountMgrStats {
    pub total_mounts: u64,
    pub total_namespaces: u64,
    pub total_mount_ops: u64,
    pub total_umount_ops: u64,
    pub total_failures: u64,
    pub pseudo_fs_count: u64,
}

/// Main mount manager
pub struct AppMountMgr {
    mounts: BTreeMap<u64, MountEntry>,
    namespaces: BTreeMap<u64, MountNamespace>,
    app_states: BTreeMap<u64, AppMountState>,
    next_mount_id: u64,
    next_ns_id: u64,
    stats: MountMgrStats,
}

impl AppMountMgr {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            namespaces: BTreeMap::new(),
            app_states: BTreeMap::new(),
            next_mount_id: 1,
            next_ns_id: 1,
            stats: MountMgrStats {
                total_mounts: 0,
                total_namespaces: 0,
                total_mount_ops: 0,
                total_umount_ops: 0,
                total_failures: 0,
                pseudo_fs_count: 0,
            },
        }
    }

    #[inline]
    pub fn create_namespace(&mut self, owner_pid: u64, timestamp_ns: u64) -> u64 {
        let id = self.next_ns_id;
        self.next_ns_id += 1;
        self.namespaces.insert(id, MountNamespace::new(id, owner_pid, timestamp_ns));
        self.stats.total_namespaces += 1;
        id
    }

    #[inline(always)]
    pub fn register_app(&mut self, pid: u64, ns_id: u64) {
        self.app_states.insert(pid, AppMountState::new(pid, ns_id));
    }

    pub fn mount(&mut self, ns_id: u64, pid: u64, mount_point: String, fs_type: String, mount_type: MountType, flags: MountFlags) -> Option<u64> {
        self.stats.total_mount_ops += 1;
        if let Some(app) = self.app_states.get_mut(&pid) {
            app.mount_ops += 1;
        }

        let id = self.next_mount_id;
        self.next_mount_id += 1;

        let mut entry = MountEntry::new(id, mount_point, fs_type, mount_type);
        entry.flags = flags;
        entry.mount_ns = ns_id;
        if entry.is_pseudo_fs() {
            self.stats.pseudo_fs_count += 1;
        }
        self.mounts.insert(id, entry);

        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.add_mount(id);
        }
        self.stats.total_mounts += 1;
        Some(id)
    }

    pub fn umount(&mut self, mount_id: u64, pid: u64) -> bool {
        self.stats.total_umount_ops += 1;
        if let Some(app) = self.app_states.get_mut(&pid) {
            app.umount_ops += 1;
        }
        if let Some(entry) = self.mounts.remove(&mount_id) {
            if let Some(ns) = self.namespaces.get_mut(&entry.mount_ns) {
                ns.remove_mount(mount_id);
            }
            if entry.is_pseudo_fs() {
                self.stats.pseudo_fs_count = self.stats.pseudo_fs_count.saturating_sub(1);
            }
            self.stats.total_mounts = self.stats.total_mounts.saturating_sub(1);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn record_io(&mut self, mount_id: u64, is_write: bool) {
        if let Some(entry) = self.mounts.get_mut(&mount_id) {
            entry.record_io(is_write);
        }
    }

    #[inline]
    pub fn set_chroot(&mut self, pid: u64, path: String) {
        if let Some(app) = self.app_states.get_mut(&pid) {
            app.chroot_path = Some(path);
        }
    }

    #[inline]
    pub fn set_pivot_root(&mut self, pid: u64) {
        if let Some(app) = self.app_states.get_mut(&pid) {
            app.pivot_root_done = true;
        }
    }

    #[inline(always)]
    pub fn mounts_under(&self, path: &str) -> Vec<&MountEntry> {
        self.mounts.values().filter(|m| m.is_child_of(path)).collect()
    }

    #[inline]
    pub fn least_secure_mounts(&self, top: usize) -> Vec<(u64, u32)> {
        let mut v: Vec<(u64, u32)> = self.mounts.iter()
            .map(|(&id, m)| (id, m.flags.security_score()))
            .collect();
        v.sort_by_key(|&(_, score)| score);
        v.truncate(top);
        v
    }

    #[inline]
    pub fn busiest_mounts(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.mounts.iter()
            .map(|(&id, m)| (id, m.total_io()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    #[inline(always)]
    pub fn get_mount(&self, id: u64) -> Option<&MountEntry> {
        self.mounts.get(&id)
    }

    #[inline(always)]
    pub fn get_app_state(&self, pid: u64) -> Option<&AppMountState> {
        self.app_states.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &MountMgrStats {
        &self.stats
    }
}
