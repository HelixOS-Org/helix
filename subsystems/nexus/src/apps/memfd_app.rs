// SPDX-License-Identifier: GPL-2.0
//! Apps memfd_app — memfd_create application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Memfd flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdFlag {
    CloseOnExec,
    AllowSealing,
    HugeTlb,
    HugeTlb2Mb,
    HugeTlb1Gb,
}

/// Memfd seal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdSeal {
    SealSeal,
    SealShrink,
    SealGrow,
    SealWrite,
    SealFutureWrite,
    SealExec,
}

/// Memfd instance
#[derive(Debug)]
pub struct MemfdInstance {
    pub fd: u64,
    pub name_hash: u64,
    pub flags: u32,
    pub seals: u32,
    pub size: u64,
    pub mapped: bool,
    pub shared_count: u32,
    pub created_at: u64,
}

impl MemfdInstance {
    pub fn new(fd: u64, name: u64, flags: u32, now: u64) -> Self {
        Self { fd, name_hash: name, flags, seals: 0, size: 0, mapped: false, shared_count: 0, created_at: now }
    }

    pub fn add_seal(&mut self, seal: MemfdSeal) {
        let bit = match seal {
            MemfdSeal::SealSeal => 1,
            MemfdSeal::SealShrink => 2,
            MemfdSeal::SealGrow => 4,
            MemfdSeal::SealWrite => 8,
            MemfdSeal::SealFutureWrite => 16,
            MemfdSeal::SealExec => 32,
        };
        self.seals |= bit;
    }

    pub fn is_sealed(&self) -> bool { self.seals & 1 != 0 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MemfdAppStats {
    pub total_memfds: u32,
    pub sealed_count: u32,
    pub mapped_count: u32,
    pub total_size: u64,
}

/// Main app memfd
pub struct AppMemfd {
    memfds: BTreeMap<u64, MemfdInstance>,
    next_fd: u64,
}

impl AppMemfd {
    pub fn new() -> Self { Self { memfds: BTreeMap::new(), next_fd: 1 } }

    pub fn create(&mut self, name: u64, flags: u32, now: u64) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.memfds.insert(fd, MemfdInstance::new(fd, name, flags, now));
        fd
    }

    pub fn seal(&mut self, fd: u64, seal: MemfdSeal) {
        if let Some(m) = self.memfds.get_mut(&fd) { m.add_seal(seal); }
    }

    pub fn resize(&mut self, fd: u64, new_size: u64) {
        if let Some(m) = self.memfds.get_mut(&fd) { m.size = new_size; }
    }

    pub fn close(&mut self, fd: u64) { self.memfds.remove(&fd); }

    pub fn stats(&self) -> MemfdAppStats {
        let sealed = self.memfds.values().filter(|m| m.is_sealed()).count() as u32;
        let mapped = self.memfds.values().filter(|m| m.mapped).count() as u32;
        let size: u64 = self.memfds.values().map(|m| m.size).sum();
        MemfdAppStats { total_memfds: self.memfds.len() as u32, sealed_count: sealed, mapped_count: mapped, total_size: size }
    }
}
