// SPDX-License-Identifier: GPL-2.0
//! Bridge memfd_bridge — memfd (anonymous memory-backed file) bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Memfd seal flags
#[derive(Debug, Clone, Copy)]
pub struct SealFlags(pub u32);

impl SealFlags {
    pub const SEAL: Self = Self(0x01);
    pub const SHRINK: Self = Self(0x02);
    pub const GROW: Self = Self(0x04);
    pub const WRITE: Self = Self(0x08);
    pub const FUTURE_WRITE: Self = Self(0x10);
    pub const EXEC: Self = Self(0x20);

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn is_immutable(&self) -> bool {
        self.contains(Self::WRITE) && self.contains(Self::SHRINK) && self.contains(Self::GROW)
    }

    pub fn count_set(&self) -> u32 {
        self.0.count_ones()
    }
}

/// Memfd create flags
#[derive(Debug, Clone, Copy)]
pub struct MemfdFlags(pub u32);

impl MemfdFlags {
    pub const CLOEXEC: Self = Self(0x01);
    pub const ALLOW_SEALING: Self = Self(0x02);
    pub const HUGETLB: Self = Self(0x04);

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// A memfd instance
#[derive(Debug)]
pub struct MemfdInstance {
    pub fd: i32,
    pub name: String,
    pub size: u64,
    pub flags: MemfdFlags,
    pub seals: SealFlags,
    pub owner_pid: u32,
    pub map_count: u32,
    pub shared_with: Vec<u32>,
    pub created: u64,
    pub last_access: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub is_hugetlb: bool,
    pub hugetlb_page_size: u64,
}

impl MemfdInstance {
    pub fn new(fd: i32, name: String, flags: MemfdFlags, pid: u32, now: u64) -> Self {
        Self {
            fd, name, size: 0, flags,
            seals: SealFlags(0),
            owner_pid: pid,
            map_count: 0,
            shared_with: Vec::new(),
            created: now, last_access: now,
            write_count: 0, read_count: 0,
            is_hugetlb: flags.contains(MemfdFlags::HUGETLB),
            hugetlb_page_size: if flags.contains(MemfdFlags::HUGETLB) { 2 * 1024 * 1024 } else { 4096 },
        }
    }

    pub fn can_seal(&self) -> bool {
        self.flags.contains(MemfdFlags::ALLOW_SEALING) && !self.seals.contains(SealFlags::SEAL)
    }

    pub fn add_seal(&mut self, seal: SealFlags) -> bool {
        if !self.can_seal() { return false; }
        self.seals = SealFlags(self.seals.0 | seal.0);
        true
    }

    pub fn is_writable(&self) -> bool {
        !self.seals.contains(SealFlags::WRITE)
    }

    pub fn size_mb(&self) -> f64 {
        self.size as f64 / (1024.0 * 1024.0)
    }

    pub fn sharing_degree(&self) -> usize {
        self.shared_with.len() + 1
    }

    pub fn is_shared(&self) -> bool {
        !self.shared_with.is_empty()
    }

    pub fn idle_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_access)
    }

    pub fn pages(&self) -> u64 {
        let page_size = if self.is_hugetlb { self.hugetlb_page_size } else { 4096 };
        (self.size + page_size - 1) / page_size
    }
}

/// Memfd operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdOp {
    Create,
    Resize,
    Seal,
    Map,
    Unmap,
    Share,
    Close,
}

/// Memfd event
#[derive(Debug, Clone)]
pub struct MemfdEvent {
    pub fd: i32,
    pub op: MemfdOp,
    pub pid: u32,
    pub size: u64,
    pub timestamp: u64,
}

/// Memfd bridge stats
#[derive(Debug, Clone)]
pub struct MemfdBridgeStats {
    pub active_memfds: u32,
    pub total_created: u64,
    pub total_memory_bytes: u64,
    pub sealed_count: u32,
    pub shared_count: u32,
    pub hugetlb_count: u32,
}

/// Main memfd bridge
pub struct BridgeMemfd {
    instances: BTreeMap<i32, MemfdInstance>,
    events: Vec<MemfdEvent>,
    max_events: usize,
    stats: MemfdBridgeStats,
}

impl BridgeMemfd {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            stats: MemfdBridgeStats {
                active_memfds: 0, total_created: 0,
                total_memory_bytes: 0, sealed_count: 0,
                shared_count: 0, hugetlb_count: 0,
            },
        }
    }

    pub fn create(&mut self, fd: i32, name: String, flags: MemfdFlags, pid: u32, now: u64) {
        let inst = MemfdInstance::new(fd, name, flags, pid, now);
        self.stats.total_created += 1;
        self.stats.active_memfds += 1;
        if inst.is_hugetlb { self.stats.hugetlb_count += 1; }
        self.instances.insert(fd, inst);
    }

    pub fn resize(&mut self, fd: i32, new_size: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.seals.contains(SealFlags::SHRINK) && new_size < inst.size { return false; }
            if inst.seals.contains(SealFlags::GROW) && new_size > inst.size { return false; }
            self.stats.total_memory_bytes = self.stats.total_memory_bytes
                .wrapping_sub(inst.size).wrapping_add(new_size);
            inst.size = new_size;
            true
        } else { false }
    }

    pub fn seal(&mut self, fd: i32, seal: SealFlags) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let was_sealed = inst.seals.is_immutable();
            let ok = inst.add_seal(seal);
            if ok && !was_sealed && inst.seals.is_immutable() {
                self.stats.sealed_count += 1;
            }
            ok
        } else { false }
    }

    pub fn share_with(&mut self, fd: i32, target_pid: u32) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if !inst.shared_with.contains(&target_pid) {
                let was_shared = inst.is_shared();
                inst.shared_with.push(target_pid);
                if !was_shared { self.stats.shared_count += 1; }
            }
            true
        } else { false }
    }

    pub fn close(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.remove(&fd) {
            if self.stats.active_memfds > 0 { self.stats.active_memfds -= 1; }
            self.stats.total_memory_bytes = self.stats.total_memory_bytes.saturating_sub(inst.size);
            if inst.is_hugetlb && self.stats.hugetlb_count > 0 { self.stats.hugetlb_count -= 1; }
            if inst.seals.is_immutable() && self.stats.sealed_count > 0 { self.stats.sealed_count -= 1; }
            if inst.is_shared() && self.stats.shared_count > 0 { self.stats.shared_count -= 1; }
            true
        } else { false }
    }

    pub fn record_event(&mut self, event: MemfdEvent) {
        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push(event);
    }

    pub fn largest_memfds(&self, n: usize) -> Vec<(i32, u64)> {
        let mut v: Vec<_> = self.instances.iter()
            .map(|(&fd, inst)| (fd, inst.size))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn unsealed_memfds(&self) -> Vec<i32> {
        self.instances.iter()
            .filter(|(_, inst)| inst.can_seal() && !inst.seals.is_immutable())
            .map(|(&fd, _)| fd)
            .collect()
    }

    pub fn stats(&self) -> &MemfdBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from memfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemfdV2Flags(pub u32);

impl MemfdV2Flags {
    pub const CLOEXEC: u32 = 1 << 0;
    pub const ALLOW_SEALING: u32 = 1 << 1;
    pub const HUGETLB: u32 = 1 << 2;
    pub const NOEXEC_SEAL: u32 = 1 << 3;
    pub const EXEC: u32 = 1 << 4;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Seal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemfdSeals(pub u32);

impl MemfdSeals {
    pub const SEAL: u32 = 1 << 0;
    pub const SHRINK: u32 = 1 << 1;
    pub const GROW: u32 = 1 << 2;
    pub const WRITE: u32 = 1 << 3;
    pub const FUTURE_WRITE: u32 = 1 << 4;
    pub const EXEC: u32 = 1 << 5;

    pub fn none() -> Self { Self(0) }
    pub fn add(&mut self, s: u32) { self.0 |= s; }
    pub fn has(&self, s: u32) -> bool { self.0 & s != 0 }
    pub fn is_sealed(&self) -> bool { self.has(Self::SEAL) }
}

/// Memfd instance
#[derive(Debug)]
pub struct MemfdV2Instance {
    pub id: u64,
    pub fd: i32,
    pub name_hash: u64,
    pub flags: MemfdV2Flags,
    pub seals: MemfdSeals,
    pub size: u64,
    pub resident_pages: u64,
    pub shared_count: u32,
    pub created_at: u64,
    pub last_write: u64,
    pub write_count: u64,
    pub mmap_count: u32,
}

impl MemfdV2Instance {
    pub fn new(id: u64, fd: i32, name_hash: u64, flags: MemfdV2Flags, now: u64) -> Self {
        Self {
            id, fd, name_hash, flags, seals: MemfdSeals::none(),
            size: 0, resident_pages: 0, shared_count: 1,
            created_at: now, last_write: 0, write_count: 0, mmap_count: 0,
        }
    }

    pub fn resize(&mut self, new_size: u64) -> bool {
        if self.seals.has(MemfdSeals::SHRINK) && new_size < self.size { return false; }
        if self.seals.has(MemfdSeals::GROW) && new_size > self.size { return false; }
        self.size = new_size;
        true
    }

    pub fn write(&mut self, now: u64) -> bool {
        if self.seals.has(MemfdSeals::WRITE) { return false; }
        self.write_count += 1;
        self.last_write = now;
        true
    }

    pub fn add_seal(&mut self, seal: u32) -> bool {
        if self.seals.is_sealed() { return false; }
        if !self.flags.has(MemfdV2Flags::ALLOW_SEALING) { return false; }
        self.seals.add(seal);
        true
    }

    pub fn share(&mut self) { self.shared_count += 1; }
    pub fn unshare(&mut self) { self.shared_count = self.shared_count.saturating_sub(1); }
    pub fn mmap(&mut self) { self.mmap_count += 1; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MemfdV2BridgeStats {
    pub total_memfds: u32,
    pub total_bytes: u64,
    pub sealed_count: u32,
    pub hugetlb_count: u32,
    pub total_writes: u64,
    pub total_shares: u32,
}

/// Main memfd v2 bridge
pub struct BridgeMemfdV2 {
    instances: BTreeMap<u64, MemfdV2Instance>,
    next_id: u64,
    next_fd: i32,
}

impl BridgeMemfdV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1, next_fd: 100 } }

    pub fn create(&mut self, name_hash: u64, flags: MemfdV2Flags, now: u64) -> (u64, i32) {
        let id = self.next_id; self.next_id += 1;
        let fd = self.next_fd; self.next_fd += 1;
        self.instances.insert(id, MemfdV2Instance::new(id, fd, name_hash, flags, now));
        (id, fd)
    }

    pub fn resize(&mut self, id: u64, size: u64) -> bool {
        self.instances.get_mut(&id).map(|m| m.resize(size)).unwrap_or(false)
    }

    pub fn add_seal(&mut self, id: u64, seal: u32) -> bool {
        self.instances.get_mut(&id).map(|m| m.add_seal(seal)).unwrap_or(false)
    }

    pub fn stats(&self) -> MemfdV2BridgeStats {
        let bytes: u64 = self.instances.values().map(|m| m.size).sum();
        let sealed = self.instances.values().filter(|m| m.seals.0 != 0).count() as u32;
        let huge = self.instances.values().filter(|m| m.flags.has(MemfdV2Flags::HUGETLB)).count() as u32;
        let writes: u64 = self.instances.values().map(|m| m.write_count).sum();
        let shares: u32 = self.instances.values().map(|m| m.shared_count).sum();
        MemfdV2BridgeStats {
            total_memfds: self.instances.len() as u32, total_bytes: bytes,
            sealed_count: sealed, hugetlb_count: huge,
            total_writes: writes, total_shares: shares,
        }
    }
}

// ============================================================================
// Merged from memfd_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdV3Seal {
    Seal,
    Shrink,
    Grow,
    Write,
    FutureWrite,
    Exec,
}

/// Memfd creation flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdV3Flag {
    CloseExec,
    AllowSealing,
    Hugetlb,
    HugeShift(u32),
    NoexecSeal,
    Exec,
}

/// Memfd backing type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemfdV3Backing {
    Tmpfs,
    Hugetlb2M,
    Hugetlb1G,
    Shmem,
}

/// A memfd V3 instance.
#[derive(Debug, Clone)]
pub struct MemfdV3Instance {
    pub memfd_id: u64,
    pub fd: i32,
    pub name: alloc::string::String,
    pub size: u64,
    pub seals: Vec<MemfdV3Seal>,
    pub flags: Vec<MemfdV3Flag>,
    pub backing: MemfdV3Backing,
    pub is_sealed: bool,
    pub is_executable: bool,
    pub shared_with: Vec<u64>,
    pub write_count: u64,
    pub mmap_count: u64,
    pub owner_pid: u64,
}

impl MemfdV3Instance {
    pub fn new(memfd_id: u64, fd: i32, name: alloc::string::String) -> Self {
        Self {
            memfd_id,
            fd,
            name,
            size: 0,
            seals: Vec::new(),
            flags: Vec::new(),
            backing: MemfdV3Backing::Tmpfs,
            is_sealed: false,
            is_executable: true,
            shared_with: Vec::new(),
            write_count: 0,
            mmap_count: 0,
            owner_pid: 0,
        }
    }

    pub fn add_seal(&mut self, seal: MemfdV3Seal) -> bool {
        if self.is_sealed {
            return false;
        }
        if self.seals.contains(&seal) {
            return false;
        }
        self.seals.push(seal);
        if seal == MemfdV3Seal::Seal {
            self.is_sealed = true;
        }
        if seal == MemfdV3Seal::Exec {
            self.is_executable = false;
        }
        true
    }

    pub fn can_write(&self) -> bool {
        !self.seals.contains(&MemfdV3Seal::Write)
            && !self.seals.contains(&MemfdV3Seal::FutureWrite)
    }

    pub fn can_grow(&self) -> bool {
        !self.seals.contains(&MemfdV3Seal::Grow)
    }

    pub fn can_shrink(&self) -> bool {
        !self.seals.contains(&MemfdV3Seal::Shrink)
    }

    pub fn resize(&mut self, new_size: u64) -> bool {
        if new_size > self.size && !self.can_grow() {
            return false;
        }
        if new_size < self.size && !self.can_shrink() {
            return false;
        }
        self.size = new_size;
        true
    }

    pub fn share_with(&mut self, pid: u64) {
        if !self.shared_with.contains(&pid) {
            self.shared_with.push(pid);
        }
    }
}

/// Statistics for memfd V3 bridge.
#[derive(Debug, Clone)]
pub struct MemfdV3BridgeStats {
    pub total_memfds: u64,
    pub total_sealed: u64,
    pub total_hugetlb: u64,
    pub total_noexec: u64,
    pub total_bytes_allocated: u64,
    pub total_shares: u64,
    pub seal_operations: u64,
}

/// Main bridge memfd V3 manager.
pub struct BridgeMemfdV3 {
    pub instances: BTreeMap<u64, MemfdV3Instance>,
    pub fd_map: BTreeMap<i32, u64>,
    pub next_id: u64,
    pub stats: MemfdV3BridgeStats,
}

impl BridgeMemfdV3 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            fd_map: BTreeMap::new(),
            next_id: 1,
            stats: MemfdV3BridgeStats {
                total_memfds: 0,
                total_sealed: 0,
                total_hugetlb: 0,
                total_noexec: 0,
                total_bytes_allocated: 0,
                total_shares: 0,
                seal_operations: 0,
            },
        }
    }

    pub fn create(
        &mut self,
        fd: i32,
        name: alloc::string::String,
        backing: MemfdV3Backing,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut inst = MemfdV3Instance::new(id, fd, name);
        inst.backing = backing;
        if matches!(
            backing,
            MemfdV3Backing::Hugetlb2M | MemfdV3Backing::Hugetlb1G
        ) {
            self.stats.total_hugetlb += 1;
        }
        self.fd_map.insert(fd, id);
        self.instances.insert(id, inst);
        self.stats.total_memfds += 1;
        id
    }

    pub fn add_seal(&mut self, memfd_id: u64, seal: MemfdV3Seal) -> bool {
        if let Some(inst) = self.instances.get_mut(&memfd_id) {
            let ok = inst.add_seal(seal);
            if ok {
                self.stats.seal_operations += 1;
                if seal == MemfdV3Seal::Seal {
                    self.stats.total_sealed += 1;
                }
                if seal == MemfdV3Seal::Exec {
                    self.stats.total_noexec += 1;
                }
            }
            ok
        } else {
            false
        }
    }

    pub fn resize(&mut self, memfd_id: u64, new_size: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&memfd_id) {
            let old = inst.size;
            let ok = inst.resize(new_size);
            if ok && new_size > old {
                self.stats.total_bytes_allocated += new_size - old;
            }
            ok
        } else {
            false
        }
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
