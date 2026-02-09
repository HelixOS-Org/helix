// SPDX-License-Identifier: GPL-2.0
//! Apps shmem_mgr — shared memory management and tracking per application.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Shared memory segment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmemType {
    /// POSIX shm_open
    PosixShm,
    /// System V shmget
    SysVShm,
    /// memfd_create
    Memfd,
    /// Anonymous mmap MAP_SHARED
    AnonShared,
    /// tmpfs-backed
    Tmpfs,
    /// hugetlbfs-backed
    Hugetlb,
}

/// Shared memory permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShmemPerms(pub u32);

impl ShmemPerms {
    pub const OWNER_READ: u32 = 0o400;
    pub const OWNER_WRITE: u32 = 0o200;
    pub const GROUP_READ: u32 = 0o040;
    pub const GROUP_WRITE: u32 = 0o020;
    pub const OTHER_READ: u32 = 0o004;
    pub const OTHER_WRITE: u32 = 0o002;

    #[inline(always)]
    pub fn has(&self, perm: u32) -> bool {
        self.0 & perm != 0
    }

    #[inline(always)]
    pub fn is_world_readable(&self) -> bool {
        self.has(Self::OTHER_READ)
    }

    #[inline(always)]
    pub fn is_world_writable(&self) -> bool {
        self.has(Self::OTHER_WRITE)
    }

    #[inline(always)]
    pub fn security_risk(&self) -> bool {
        self.is_world_writable()
    }
}

/// A shared memory segment
#[derive(Debug, Clone)]
pub struct ShmemSegment {
    pub id: u64,
    pub name: Option<String>,
    pub shm_type: ShmemType,
    pub size_bytes: u64,
    pub perms: ShmemPerms,
    pub creator_pid: u64,
    pub created_ns: u64,
    pub sealed: bool,
    attachments: Vec<ShmemAttachment>,
    pub page_faults: u64,
    pub resident_pages: u64,
    pub swap_pages: u64,
}

impl ShmemSegment {
    pub fn new(id: u64, shm_type: ShmemType, size_bytes: u64, creator: u64) -> Self {
        Self {
            id,
            name: None,
            shm_type,
            size_bytes,
            perms: ShmemPerms(0o600),
            creator_pid: creator,
            created_ns: 0,
            sealed: false,
            attachments: Vec::new(),
            page_faults: 0,
            resident_pages: 0,
            swap_pages: 0,
        }
    }

    #[inline(always)]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    #[inline]
    pub fn attach(&mut self, pid: u64, writable: bool, addr: u64, timestamp_ns: u64) {
        self.attachments.push(ShmemAttachment {
            pid,
            writable,
            virt_addr: addr,
            attach_ns: timestamp_ns,
            access_count: 0,
        });
    }

    #[inline]
    pub fn detach(&mut self, pid: u64) -> bool {
        if let Some(pos) = self.attachments.iter().position(|a| a.pid == pid) {
            self.attachments.swap_remove(pos);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    #[inline(always)]
    pub fn writer_count(&self) -> usize {
        self.attachments.iter().filter(|a| a.writable).count()
    }

    #[inline(always)]
    pub fn is_orphan(&self) -> bool {
        self.attachments.is_empty()
    }

    #[inline]
    pub fn resident_ratio(&self) -> f64 {
        let total = self.size_bytes / 4096;
        if total == 0 { return 0.0; }
        self.resident_pages as f64 / total as f64
    }

    #[inline]
    pub fn swap_ratio(&self) -> f64 {
        let total = self.resident_pages + self.swap_pages;
        if total == 0 { return 0.0; }
        self.swap_pages as f64 / total as f64
    }

    #[inline]
    pub fn is_shared_between(&self, pid_a: u64, pid_b: u64) -> bool {
        let has_a = self.attachments.iter().any(|a| a.pid == pid_a);
        let has_b = self.attachments.iter().any(|a| a.pid == pid_b);
        has_a && has_b
    }

    #[inline(always)]
    pub fn page_count(&self) -> u64 {
        (self.size_bytes + 4095) / 4096
    }

    #[inline(always)]
    pub fn total_access_count(&self) -> u64 {
        self.attachments.iter().map(|a| a.access_count).sum()
    }
}

/// A process attachment to a shared segment
#[derive(Debug, Clone)]
pub struct ShmemAttachment {
    pub pid: u64,
    pub writable: bool,
    pub virt_addr: u64,
    pub attach_ns: u64,
    pub access_count: u64,
}

/// Per-app shared memory profile
#[derive(Debug)]
pub struct AppShmemProfile {
    pub pid: u64,
    pub owned_segments: Vec<u64>,
    pub attached_segments: Vec<u64>,
    pub total_shared_bytes: u64,
    pub ipc_partners: Vec<u64>,
}

impl AppShmemProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            owned_segments: Vec::new(),
            attached_segments: Vec::new(),
            total_shared_bytes: 0,
            ipc_partners: Vec::new(),
        }
    }

    #[inline]
    pub fn add_owned(&mut self, seg_id: u64, size: u64) {
        if !self.owned_segments.contains(&seg_id) {
            self.owned_segments.push(seg_id);
            self.total_shared_bytes += size;
        }
    }

    #[inline]
    pub fn add_attached(&mut self, seg_id: u64, size: u64) {
        if !self.attached_segments.contains(&seg_id) {
            self.attached_segments.push(seg_id);
            self.total_shared_bytes += size;
        }
    }

    #[inline]
    pub fn add_partner(&mut self, pid: u64) {
        if pid != self.pid && !self.ipc_partners.contains(&pid) {
            self.ipc_partners.push(pid);
        }
    }

    #[inline(always)]
    pub fn total_segments(&self) -> usize {
        self.owned_segments.len() + self.attached_segments.len()
    }
}

/// Shmem manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ShmemMgrStats {
    pub total_segments: u64,
    pub total_bytes: u64,
    pub total_attachments: u64,
    pub orphan_segments: u64,
    pub security_warnings: u64,
    pub total_page_faults: u64,
}

/// Main shared memory manager
pub struct AppShmemMgr {
    segments: BTreeMap<u64, ShmemSegment>,
    name_to_id: BTreeMap<String, u64>,
    profiles: BTreeMap<u64, AppShmemProfile>,
    next_id: u64,
    stats: ShmemMgrStats,
}

impl AppShmemMgr {
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            profiles: BTreeMap::new(),
            next_id: 1,
            stats: ShmemMgrStats {
                total_segments: 0,
                total_bytes: 0,
                total_attachments: 0,
                orphan_segments: 0,
                security_warnings: 0,
                total_page_faults: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_app(&mut self, pid: u64) {
        self.profiles.insert(pid, AppShmemProfile::new(pid));
    }

    pub fn create_segment(&mut self, creator: u64, shm_type: ShmemType, size: u64, name: Option<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut seg = ShmemSegment::new(id, shm_type, size, creator);
        if let Some(ref n) = name {
            self.name_to_id.insert(n.clone(), id);
            seg = seg.with_name(n.clone());
        }
        if seg.perms.security_risk() {
            self.stats.security_warnings += 1;
        }
        self.segments.insert(id, seg);
        if let Some(prof) = self.profiles.get_mut(&creator) {
            prof.add_owned(id, size);
        }
        self.stats.total_segments += 1;
        self.stats.total_bytes += size;
        id
    }

    pub fn attach(&mut self, seg_id: u64, pid: u64, writable: bool, addr: u64, timestamp_ns: u64) -> bool {
        if let Some(seg) = self.segments.get_mut(&seg_id) {
            if seg.sealed && writable { return false; }
            seg.attach(pid, writable, addr, timestamp_ns);
            self.stats.total_attachments += 1;
            if let Some(prof) = self.profiles.get_mut(&pid) {
                prof.add_attached(seg_id, seg.size_bytes);
            }
            // Register IPC partners
            let pids: Vec<u64> = seg.attachments.iter().map(|a| a.pid).collect();
            for &p in &pids {
                if let Some(prof) = self.profiles.get_mut(&p) {
                    for &other in &pids {
                        prof.add_partner(other);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    pub fn detach(&mut self, seg_id: u64, pid: u64) -> bool {
        if let Some(seg) = self.segments.get_mut(&seg_id) {
            if seg.detach(pid) {
                self.stats.total_attachments = self.stats.total_attachments.saturating_sub(1);
                if seg.is_orphan() {
                    self.stats.orphan_segments += 1;
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn destroy_segment(&mut self, seg_id: u64) -> bool {
        if let Some(seg) = self.segments.remove(&seg_id) {
            self.stats.total_bytes = self.stats.total_bytes.saturating_sub(seg.size_bytes);
            self.stats.total_segments = self.stats.total_segments.saturating_sub(1);
            if let Some(ref n) = seg.name {
                self.name_to_id.remove(n);
            }
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn seal_segment(&mut self, seg_id: u64) {
        if let Some(seg) = self.segments.get_mut(&seg_id) {
            seg.sealed = true;
        }
    }

    #[inline]
    pub fn record_fault(&mut self, seg_id: u64) {
        if let Some(seg) = self.segments.get_mut(&seg_id) {
            seg.page_faults += 1;
            self.stats.total_page_faults += 1;
        }
    }

    #[inline(always)]
    pub fn lookup_by_name(&self, name: &str) -> Option<u64> {
        self.name_to_id.get(name).copied()
    }

    #[inline]
    pub fn largest_segments(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.segments.iter().map(|(&id, s)| (id, s.size_bytes)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    #[inline]
    pub fn orphan_segments(&self) -> Vec<u64> {
        self.segments.iter()
            .filter(|(_, s)| s.is_orphan())
            .map(|(&id, _)| id)
            .collect()
    }

    #[inline]
    pub fn security_risks(&self) -> Vec<u64> {
        self.segments.iter()
            .filter(|(_, s)| s.perms.security_risk())
            .map(|(&id, _)| id)
            .collect()
    }

    #[inline(always)]
    pub fn get_segment(&self, id: u64) -> Option<&ShmemSegment> {
        self.segments.get(&id)
    }

    #[inline(always)]
    pub fn get_profile(&self, pid: u64) -> Option<&AppShmemProfile> {
        self.profiles.get(&pid)
    }

    #[inline(always)]
    pub fn stats(&self) -> &ShmemMgrStats {
        &self.stats
    }
}
