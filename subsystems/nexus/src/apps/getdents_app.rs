// SPDX-License-Identifier: GPL-2.0
//! Apps getdents_app — directory entry reading.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Directory entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentType {
    Unknown,
    RegularFile,
    Directory,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Symlink,
}

/// Directory entry
#[derive(Debug)]
pub struct DentEntry {
    pub inode: u64,
    pub offset: u64,
    pub name_hash: u64,
    pub d_type: DentType,
}

/// Directory read session
#[derive(Debug)]
pub struct DirReadSession {
    pub fd: u64,
    pub pid: u64,
    pub entries_read: u64,
    pub bytes_read: u64,
    pub calls: u64,
    pub position: u64,
    pub eof: bool,
}

impl DirReadSession {
    pub fn new(fd: u64, pid: u64) -> Self {
        Self { fd, pid, entries_read: 0, bytes_read: 0, calls: 0, position: 0, eof: false }
    }

    #[inline]
    pub fn read(&mut self, count: u64, bytes: u64) {
        self.entries_read += count;
        self.bytes_read += bytes;
        self.calls += 1;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GetdentsAppStats {
    pub total_sessions: u32,
    pub total_entries_read: u64,
    pub total_bytes_read: u64,
    pub total_calls: u64,
    pub avg_entries_per_call: f64,
}

/// Main getdents app
pub struct AppGetdents {
    sessions: BTreeMap<u64, DirReadSession>,
}

impl AppGetdents {
    pub fn new() -> Self { Self { sessions: BTreeMap::new() } }

    #[inline(always)]
    pub fn open(&mut self, fd: u64, pid: u64) { self.sessions.insert(fd, DirReadSession::new(fd, pid)); }

    #[inline(always)]
    pub fn read(&mut self, fd: u64, count: u64, bytes: u64) {
        if let Some(s) = self.sessions.get_mut(&fd) { s.read(count, bytes); }
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.sessions.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> GetdentsAppStats {
        let entries: u64 = self.sessions.values().map(|s| s.entries_read).sum();
        let bytes: u64 = self.sessions.values().map(|s| s.bytes_read).sum();
        let calls: u64 = self.sessions.values().map(|s| s.calls).sum();
        let avg = if calls == 0 { 0.0 } else { entries as f64 / calls as f64 };
        GetdentsAppStats { total_sessions: self.sessions.len() as u32, total_entries_read: entries, total_bytes_read: bytes, total_calls: calls, avg_entries_per_call: avg }
    }
}

// ============================================================================
// Merged from getdents_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DentTypeV2 {
    Unknown,
    Regular,
    Directory,
    CharDev,
    BlockDev,
    Fifo,
    Socket,
    Symlink,
}

/// Directory entry v2
#[derive(Debug)]
pub struct DentEntryV2 {
    pub inode: u64,
    pub name_hash: u64,
    pub dtype: DentTypeV2,
    pub record_len: u16,
    pub offset: u64,
}

/// Directory read session v2
#[derive(Debug)]
pub struct DirSessionV2 {
    pub fd: u64,
    pub dir_inode: u64,
    pub position: u64,
    pub entries_read: u64,
    pub bytes_read: u64,
    pub completed: bool,
}

impl DirSessionV2 {
    pub fn new(fd: u64, inode: u64) -> Self {
        Self { fd, dir_inode: inode, position: 0, entries_read: 0, bytes_read: 0, completed: false }
    }

    #[inline]
    pub fn read_entries(&mut self, count: u32, bytes: u64) {
        self.entries_read += count as u64;
        self.bytes_read += bytes;
        if bytes == 0 { self.completed = true; }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GetdentsV2AppStats {
    pub active_sessions: u32,
    pub total_entries_read: u64,
    pub total_bytes: u64,
    pub completed_scans: u32,
}

/// Main app getdents v2
pub struct AppGetdentsV2 {
    sessions: BTreeMap<u64, DirSessionV2>,
}

impl AppGetdentsV2 {
    pub fn new() -> Self { Self { sessions: BTreeMap::new() } }

    #[inline(always)]
    pub fn open_dir(&mut self, fd: u64, inode: u64) {
        self.sessions.insert(fd, DirSessionV2::new(fd, inode));
    }

    #[inline(always)]
    pub fn read(&mut self, fd: u64, count: u32, bytes: u64) {
        if let Some(s) = self.sessions.get_mut(&fd) { s.read_entries(count, bytes); }
    }

    #[inline(always)]
    pub fn close_dir(&mut self, fd: u64) { self.sessions.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> GetdentsV2AppStats {
        let entries: u64 = self.sessions.values().map(|s| s.entries_read).sum();
        let bytes: u64 = self.sessions.values().map(|s| s.bytes_read).sum();
        let completed = self.sessions.values().filter(|s| s.completed).count() as u32;
        GetdentsV2AppStats { active_sessions: self.sessions.len() as u32, total_entries_read: entries, total_bytes: bytes, completed_scans: completed }
    }
}
