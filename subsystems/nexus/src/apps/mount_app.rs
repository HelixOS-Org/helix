// SPDX-License-Identifier: GPL-2.0
//! Apps mount_app — filesystem mount management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Mount flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountAppFlag {
    ReadOnly,
    NoSuid,
    NoDev,
    NoExec,
    Synchronous,
    Remount,
    MandLock,
    DirSync,
    NoAtime,
    NoDirAtime,
    Bind,
    Move,
    Recursive,
    Silent,
    Relatime,
    StrictAtime,
    LazyTime,
}

/// Mount entry
#[derive(Debug)]
pub struct MountEntry {
    pub id: u64,
    pub source_hash: u64,
    pub target_hash: u64,
    pub fstype: String,
    pub flags: u64,
    pub mount_time: u64,
    pub reads: u64,
    pub writes: u64,
    pub parent_id: Option<u64>,
}

impl MountEntry {
    pub fn new(id: u64, source: u64, target: u64, fstype: String, flags: u64, now: u64) -> Self {
        Self { id, source_hash: source, target_hash: target, fstype, flags, mount_time: now, reads: 0, writes: 0, parent_id: None }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MountAppStats {
    pub total_mounts: u32,
    pub readonly_mounts: u32,
    pub bind_mounts: u32,
    pub total_io: u64,
}

/// Main mount app
pub struct AppMount {
    mounts: BTreeMap<u64, MountEntry>,
    next_id: u64,
}

impl AppMount {
    pub fn new() -> Self { Self { mounts: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn mount(&mut self, source: u64, target: u64, fstype: String, flags: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.mounts.insert(id, MountEntry::new(id, source, target, fstype, flags, now));
        id
    }

    #[inline(always)]
    pub fn umount(&mut self, id: u64) -> bool { self.mounts.remove(&id).is_some() }

    #[inline]
    pub fn stats(&self) -> MountAppStats {
        let ro = self.mounts.values().filter(|m| m.flags & 1 != 0).count() as u32;
        let bind = self.mounts.values().filter(|m| m.flags & (1 << 12) != 0).count() as u32;
        let io: u64 = self.mounts.values().map(|m| m.reads + m.writes).sum();
        MountAppStats { total_mounts: self.mounts.len() as u32, readonly_mounts: ro, bind_mounts: bind, total_io: io }
    }
}
