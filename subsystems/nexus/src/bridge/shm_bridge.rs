//! # Bridge SHM Bridge
//!
//! System V shared memory (shmget/shmat/shmdt/shmctl) bridging:
//! - Shared memory segment creation and management
//! - Attach/detach tracking per process
//! - Permission and key management
//! - Segment size and residency tracking
//! - Hugepage-backed SHM support
//! - IPC namespace isolation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// SHM permission flags
#[derive(Debug, Clone, Copy)]
pub struct ShmPerm {
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u16,
}

impl ShmPerm {
    pub fn new(uid: u32, gid: u32, mode: u16) -> Self {
        Self { uid, gid, cuid: uid, cgid: gid, mode }
    }

    pub fn check_read(&self, uid: u32, gid: u32) -> bool {
        if uid == self.uid { return self.mode & 0o400 != 0; }
        if gid == self.gid { return self.mode & 0o040 != 0; }
        self.mode & 0o004 != 0
    }

    pub fn check_write(&self, uid: u32, gid: u32) -> bool {
        if uid == self.uid { return self.mode & 0o200 != 0; }
        if gid == self.gid { return self.mode & 0o020 != 0; }
        self.mode & 0o002 != 0
    }
}

/// SHM segment state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmState {
    Active,
    MarkedForDestroy,
    Locked,
    Swapped,
}

/// Attachment info for a process
#[derive(Debug, Clone)]
pub struct ShmAttach {
    pub pid: u64,
    pub attach_addr: u64,
    pub readonly: bool,
    pub attach_ts: u64,
}

/// Shared memory segment
#[derive(Debug, Clone)]
pub struct ShmSegment {
    pub shm_id: u32,
    pub key: i32,
    pub perm: ShmPerm,
    pub size: usize,
    pub state: ShmState,
    pub hugepage: bool,
    pub ns_id: u64,
    pub attachments: Vec<ShmAttach>,
    pub creator_pid: u64,
    pub last_attach_pid: u64,
    pub last_detach_pid: u64,
    pub attach_time: u64,
    pub detach_time: u64,
    pub change_time: u64,
    pub nattch: u32,
    pub resident_pages: u64,
    pub swapped_pages: u64,
}

impl ShmSegment {
    pub fn new(id: u32, key: i32, perm: ShmPerm, size: usize, pid: u64, ts: u64) -> Self {
        Self {
            shm_id: id, key, perm, size, state: ShmState::Active,
            hugepage: false, ns_id: 0, attachments: Vec::new(),
            creator_pid: pid, last_attach_pid: 0, last_detach_pid: 0,
            attach_time: 0, detach_time: 0, change_time: ts,
            nattch: 0, resident_pages: 0, swapped_pages: 0,
        }
    }

    pub fn attach(&mut self, pid: u64, addr: u64, readonly: bool, ts: u64) {
        self.attachments.push(ShmAttach { pid, attach_addr: addr, readonly, attach_ts: ts });
        self.nattch += 1;
        self.last_attach_pid = pid;
        self.attach_time = ts;
    }

    pub fn detach(&mut self, pid: u64, addr: u64, ts: u64) -> bool {
        let before = self.attachments.len();
        self.attachments.retain(|a| !(a.pid == pid && a.attach_addr == addr));
        if self.attachments.len() < before {
            self.nattch = self.nattch.saturating_sub(1);
            self.last_detach_pid = pid;
            self.detach_time = ts;
            true
        } else { false }
    }

    pub fn detach_all_for_pid(&mut self, pid: u64, ts: u64) -> u32 {
        let before = self.attachments.len();
        self.attachments.retain(|a| a.pid != pid);
        let removed = (before - self.attachments.len()) as u32;
        if removed > 0 {
            self.nattch = self.nattch.saturating_sub(removed);
            self.last_detach_pid = pid;
            self.detach_time = ts;
        }
        removed
    }

    pub fn is_private(&self) -> bool { self.key == 0 }
    pub fn total_memory(&self) -> usize { self.size }
    pub fn resident_ratio(&self) -> f64 {
        let total_pages = (self.size + 4095) / 4096;
        if total_pages == 0 { return 0.0; }
        self.resident_pages as f64 / total_pages as f64
    }
}

/// SHM operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmOp {
    Create,
    Attach,
    Detach,
    Remove,
    Lock,
    Unlock,
    SetPerm,
}

/// SHM bridge stats
#[derive(Debug, Clone, Default)]
pub struct ShmBridgeStats {
    pub total_segments: usize,
    pub total_memory: usize,
    pub total_attachments: u32,
    pub hugepage_segments: usize,
    pub marked_destroy: usize,
    pub total_ops: u64,
    pub peak_segments: usize,
    pub peak_memory: usize,
}

/// Bridge SHM manager
pub struct BridgeShmBridge {
    segments: BTreeMap<u32, ShmSegment>,
    key_to_id: BTreeMap<i32, u32>,
    next_id: u32,
    stats: ShmBridgeStats,
}

impl BridgeShmBridge {
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(), key_to_id: BTreeMap::new(),
            next_id: 1, stats: ShmBridgeStats::default(),
        }
    }

    pub fn shmget(&mut self, key: i32, size: usize, uid: u32, gid: u32, mode: u16, pid: u64, ts: u64) -> u32 {
        if key != 0 {
            if let Some(&existing) = self.key_to_id.get(&key) {
                return existing;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let seg = ShmSegment::new(id, key, ShmPerm::new(uid, gid, mode), size, pid, ts);
        self.segments.insert(id, seg);
        if key != 0 { self.key_to_id.insert(key, id); }
        self.stats.total_ops += 1;
        id
    }

    pub fn shmat(&mut self, shm_id: u32, pid: u64, addr: u64, readonly: bool, ts: u64) -> bool {
        if let Some(seg) = self.segments.get_mut(&shm_id) {
            if seg.state == ShmState::MarkedForDestroy { return false; }
            seg.attach(pid, addr, readonly, ts);
            self.stats.total_ops += 1;
            true
        } else { false }
    }

    pub fn shmdt(&mut self, shm_id: u32, pid: u64, addr: u64, ts: u64) -> bool {
        if let Some(seg) = self.segments.get_mut(&shm_id) {
            let ok = seg.detach(pid, addr, ts);
            if ok { self.stats.total_ops += 1; }
            // auto-destroy if marked and no attachments
            if seg.state == ShmState::MarkedForDestroy && seg.nattch == 0 {
                let key = seg.key;
                self.segments.remove(&shm_id);
                if key != 0 { self.key_to_id.remove(&key); }
            }
            ok
        } else { false }
    }

    pub fn shmctl_rmid(&mut self, shm_id: u32, ts: u64) -> bool {
        if let Some(seg) = self.segments.get_mut(&shm_id) {
            seg.change_time = ts;
            if seg.nattch == 0 {
                let key = seg.key;
                self.segments.remove(&shm_id);
                if key != 0 { self.key_to_id.remove(&key); }
            } else {
                seg.state = ShmState::MarkedForDestroy;
            }
            self.stats.total_ops += 1;
            true
        } else { false }
    }

    pub fn cleanup_pid(&mut self, pid: u64, ts: u64) {
        let ids: Vec<u32> = self.segments.keys().copied().collect();
        for id in ids {
            if let Some(seg) = self.segments.get_mut(&id) {
                seg.detach_all_for_pid(pid, ts);
                if seg.state == ShmState::MarkedForDestroy && seg.nattch == 0 {
                    let key = seg.key;
                    self.segments.remove(&id);
                    if key != 0 { self.key_to_id.remove(&key); }
                }
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_segments = self.segments.len();
        self.stats.total_memory = self.segments.values().map(|s| s.size).sum();
        self.stats.total_attachments = self.segments.values().map(|s| s.nattch).sum();
        self.stats.hugepage_segments = self.segments.values().filter(|s| s.hugepage).count();
        self.stats.marked_destroy = self.segments.values().filter(|s| s.state == ShmState::MarkedForDestroy).count();
        if self.stats.total_segments > self.stats.peak_segments { self.stats.peak_segments = self.stats.total_segments; }
        if self.stats.total_memory > self.stats.peak_memory { self.stats.peak_memory = self.stats.total_memory; }
    }

    pub fn segment(&self, id: u32) -> Option<&ShmSegment> { self.segments.get(&id) }
    pub fn stats(&self) -> &ShmBridgeStats { &self.stats }
}

// ============================================================================
// Merged from shm_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmV2Op {
    Shmget,
    Shmat,
    Shmdt,
    Shmctl,
}

/// Shm v2 flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmV2Flag {
    None,
    IpcCreat,
    IpcExcl,
    ShmRdonly,
    ShmRnd,
    ShmRemap,
    ShmHugetlb,
}

/// Shm v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmV2Result {
    Success,
    PermissionDenied,
    NoMemory,
    InvalidId,
    Error,
}

/// Shm v2 record
#[derive(Debug, Clone)]
pub struct ShmV2Record {
    pub op: ShmV2Op,
    pub result: ShmV2Result,
    pub shmid: i32,
    pub size: u64,
    pub key: u32,
    pub flags: u32,
}

impl ShmV2Record {
    pub fn new(op: ShmV2Op, size: u64) -> Self {
        Self { op, result: ShmV2Result::Success, shmid: -1, size, key: 0, flags: 0 }
    }
}

/// Shm v2 bridge stats
#[derive(Debug, Clone)]
pub struct ShmV2BridgeStats {
    pub total_ops: u64,
    pub segments_created: u64,
    pub attaches: u64,
    pub total_bytes: u64,
    pub errors: u64,
}

/// Main bridge shm v2
#[derive(Debug)]
pub struct BridgeShmV2 {
    pub stats: ShmV2BridgeStats,
}

impl BridgeShmV2 {
    pub fn new() -> Self {
        Self { stats: ShmV2BridgeStats { total_ops: 0, segments_created: 0, attaches: 0, total_bytes: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &ShmV2Record) {
        self.stats.total_ops += 1;
        match rec.op {
            ShmV2Op::Shmget => { self.stats.segments_created += 1; self.stats.total_bytes += rec.size; }
            ShmV2Op::Shmat => self.stats.attaches += 1,
            _ => {}
        }
        if rec.result != ShmV2Result::Success { self.stats.errors += 1; }
    }
}
