// SPDX-License-Identifier: GPL-2.0
//! Bridge mnt_ns_bridge — mount namespace and filesystem mount point management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Mount propagation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountPropagation {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Mount flags
#[derive(Debug, Clone, Copy)]
pub struct MountFlags {
    pub bits: u64,
}

impl MountFlags {
    pub const RDONLY: u64 = 1 << 0;
    pub const NOSUID: u64 = 1 << 1;
    pub const NODEV: u64 = 1 << 2;
    pub const NOEXEC: u64 = 1 << 3;
    pub const SYNCHRONOUS: u64 = 1 << 4;
    pub const REMOUNT: u64 = 1 << 5;
    pub const MANDLOCK: u64 = 1 << 6;
    pub const NOATIME: u64 = 1 << 10;
    pub const NODIRATIME: u64 = 1 << 11;
    pub const BIND: u64 = 1 << 12;
    pub const MOVE: u64 = 1 << 13;
    pub const LAZYTIME: u64 = 1 << 25;

    pub fn new(bits: u64) -> Self { Self { bits } }
    pub fn has(&self, flag: u64) -> bool { self.bits & flag != 0 }
    pub fn is_readonly(&self) -> bool { self.has(Self::RDONLY) }
}

/// Filesystem type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsType {
    Ext4,
    Btrfs,
    Xfs,
    Tmpfs,
    Proc,
    Sysfs,
    Devtmpfs,
    Overlay,
    Nfs,
    Cifs,
    Fuse,
    Unknown,
}

/// Mount point entry
#[derive(Debug, Clone)]
pub struct MountPoint {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub ns_id: u64,
    pub source: String,
    pub target: String,
    pub fs_type: FsType,
    pub flags: MountFlags,
    pub propagation: MountPropagation,
    pub peer_group: u32,
    pub mounted_at: u64,
}

impl MountPoint {
    pub fn new(id: u64, ns_id: u64, source: String, target: String, fs_type: FsType) -> Self {
        Self {
            id, parent_id: None, ns_id, source, target, fs_type,
            flags: MountFlags::new(0), propagation: MountPropagation::Private,
            peer_group: 0, mounted_at: 0,
        }
    }

    pub fn fnv_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in self.target.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Mount namespace
#[derive(Debug, Clone)]
pub struct MountNamespace {
    pub id: u64,
    pub owner_pid: u64,
    pub mount_ids: Vec<u64>,
    pub created_at: u64,
    pub ref_count: u32,
}

impl MountNamespace {
    pub fn new(id: u64, owner: u64, now: u64) -> Self {
        Self { id, owner_pid: owner, mount_ids: Vec::new(), created_at: now, ref_count: 1 }
    }

    pub fn add_mount(&mut self, mount_id: u64) { self.mount_ids.push(mount_id); }
    pub fn remove_mount(&mut self, mount_id: u64) { self.mount_ids.retain(|&id| id != mount_id); }
}

/// Mount event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountEventType {
    Mount,
    Umount,
    Remount,
    Move,
    PropagationChange,
}

/// Bridge stats
#[derive(Debug, Clone)]
pub struct MntNsBridgeStats {
    pub total_namespaces: u32,
    pub total_mounts: u32,
    pub total_events: u64,
    pub shared_mounts: u32,
    pub readonly_mounts: u32,
}

/// Main mount namespace bridge
pub struct BridgeMntNs {
    namespaces: BTreeMap<u64, MountNamespace>,
    mounts: BTreeMap<u64, MountPoint>,
    next_ns_id: u64,
    next_mount_id: u64,
    total_events: u64,
}

impl BridgeMntNs {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(), mounts: BTreeMap::new(),
            next_ns_id: 1, next_mount_id: 1, total_events: 0,
        }
    }

    pub fn create_namespace(&mut self, owner: u64, now: u64) -> u64 {
        let id = self.next_ns_id;
        self.next_ns_id += 1;
        self.namespaces.insert(id, MountNamespace::new(id, owner, now));
        id
    }

    pub fn mount(&mut self, ns_id: u64, source: String, target: String, fs_type: FsType, now: u64) -> Option<u64> {
        let mid = self.next_mount_id;
        self.next_mount_id += 1;
        let mut mp = MountPoint::new(mid, ns_id, source, target, fs_type);
        mp.mounted_at = now;
        self.mounts.insert(mid, mp);
        if let Some(ns) = self.namespaces.get_mut(&ns_id) { ns.add_mount(mid); }
        self.total_events += 1;
        Some(mid)
    }

    pub fn umount(&mut self, mount_id: u64) -> bool {
        if let Some(mp) = self.mounts.remove(&mount_id) {
            if let Some(ns) = self.namespaces.get_mut(&mp.ns_id) { ns.remove_mount(mount_id); }
            self.total_events += 1;
            true
        } else { false }
    }

    pub fn stats(&self) -> MntNsBridgeStats {
        let shared = self.mounts.values().filter(|m| m.propagation == MountPropagation::Shared).count() as u32;
        let ro = self.mounts.values().filter(|m| m.flags.is_readonly()).count() as u32;
        MntNsBridgeStats {
            total_namespaces: self.namespaces.len() as u32,
            total_mounts: self.mounts.len() as u32,
            total_events: self.total_events,
            shared_mounts: shared, readonly_mounts: ro,
        }
    }
}
