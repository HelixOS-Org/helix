// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps readdir — Directory entry enumeration tracking
//!
//! Tracks getdents64/readdir operations with entry caching, seek position
//! management, and per-directory access pattern analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Directory entry type (d_type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirentType {
    Unknown,
    Fifo,
    CharDevice,
    Directory,
    BlockDevice,
    RegularFile,
    Symlink,
    Socket,
    Whiteout,
}

/// A directory entry.
#[derive(Debug, Clone)]
pub struct DirentEntry {
    pub inode: u64,
    pub offset: u64,
    pub entry_type: DirentType,
    pub name: String,
    pub name_len: u16,
}

impl DirentEntry {
    pub fn new(inode: u64, name: String, entry_type: DirentType) -> Self {
        let name_len = name.len() as u16;
        Self {
            inode,
            offset: 0,
            entry_type,
            name,
            name_len,
        }
    }

    pub fn record_size(&self) -> usize {
        // d_ino(8) + d_off(8) + d_reclen(2) + d_type(1) + name + padding
        24 + self.name_len as usize
    }
}

/// Per-directory readdir state.
#[derive(Debug, Clone)]
pub struct ReaddirState {
    pub dir_fd: i32,
    pub pid: u64,
    pub path: Option<String>,
    pub position: u64,
    pub entries_read: u64,
    pub calls_made: u64,
    pub bytes_returned: u64,
    pub is_complete: bool,
    pub cached_entries: Vec<DirentEntry>,
}

impl ReaddirState {
    pub fn new(dir_fd: i32, pid: u64) -> Self {
        Self {
            dir_fd,
            pid,
            path: None,
            position: 0,
            entries_read: 0,
            calls_made: 0,
            bytes_returned: 0,
            is_complete: false,
            cached_entries: Vec::new(),
        }
    }

    pub fn read_entries(&mut self, count: u64, bytes: u64) {
        self.entries_read += count;
        self.bytes_returned += bytes;
        self.calls_made += 1;
        if count == 0 {
            self.is_complete = true;
        }
    }

    pub fn seekdir(&mut self, offset: u64) {
        self.position = offset;
        self.is_complete = false;
    }

    pub fn avg_entries_per_call(&self) -> f64 {
        if self.calls_made == 0 {
            return 0.0;
        }
        self.entries_read as f64 / self.calls_made as f64
    }
}

/// Statistics for readdir app.
#[derive(Debug, Clone)]
pub struct ReaddirAppStats {
    pub total_calls: u64,
    pub total_entries: u64,
    pub total_bytes: u64,
    pub complete_scans: u64,
    pub active_dirs: u64,
    pub seekdir_count: u64,
}

/// Main apps readdir manager.
pub struct AppReaddir {
    pub states: BTreeMap<u64, ReaddirState>, // key = (pid << 32) | fd
    pub next_key: u64,
    pub stats: ReaddirAppStats,
}

impl AppReaddir {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            next_key: 1,
            stats: ReaddirAppStats {
                total_calls: 0,
                total_entries: 0,
                total_bytes: 0,
                complete_scans: 0,
                active_dirs: 0,
                seekdir_count: 0,
            },
        }
    }

    pub fn open_dir(&mut self, pid: u64, dir_fd: i32) -> u64 {
        let key = (pid << 32) | (dir_fd as u64 & 0xFFFFFFFF);
        let state = ReaddirState::new(dir_fd, pid);
        self.states.insert(key, state);
        self.stats.active_dirs += 1;
        key
    }

    pub fn record_getdents(&mut self, key: u64, entries: u64, bytes: u64) -> bool {
        if let Some(state) = self.states.get_mut(&key) {
            state.read_entries(entries, bytes);
            self.stats.total_calls += 1;
            self.stats.total_entries += entries;
            self.stats.total_bytes += bytes;
            if entries == 0 {
                self.stats.complete_scans += 1;
            }
            true
        } else {
            false
        }
    }

    pub fn close_dir(&mut self, key: u64) {
        if self.states.remove(&key).is_some() {
            self.stats.active_dirs = self.stats.active_dirs.saturating_sub(1);
        }
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }
}

// ============================================================================
// Merged from readdir_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirentV2Type {
    Unknown,
    Regular,
    Directory,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Symlink,
    Whiteout,
}

/// Readdir v2 sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaddirV2Sort {
    None,
    NameAsc,
    NameDesc,
    InodeAsc,
    TypeFirst,
}

/// Directory entry v2
#[derive(Debug, Clone)]
pub struct DirentV2Entry {
    pub inode: u64,
    pub name_hash: u64,
    pub entry_type: DirentV2Type,
    pub record_len: u16,
    pub offset: u64,
    pub name_len: u16,
}

impl DirentV2Entry {
    pub fn new(inode: u64, name: &[u8], entry_type: DirentV2Type) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            inode,
            name_hash: h,
            entry_type,
            record_len: (24 + name.len() as u16 + 7) & !7,
            offset: 0,
            name_len: name.len() as u16,
        }
    }

    pub fn is_directory(&self) -> bool {
        self.entry_type == DirentV2Type::Directory
    }

    pub fn is_special(&self) -> bool {
        matches!(self.entry_type, DirentV2Type::CharDevice | DirentV2Type::BlockDevice | DirentV2Type::Socket | DirentV2Type::Fifo)
    }
}

/// Readdir v2 stream
#[derive(Debug, Clone)]
pub struct ReaddirV2Stream {
    pub dir_fd: i32,
    pub position: u64,
    pub entries_read: u64,
    pub total_bytes: u64,
    pub buffer_size: u32,
    pub eof: bool,
    pub sort_order: ReaddirV2Sort,
    pub filter_type: Option<DirentV2Type>,
    pub calls: u64,
}

impl ReaddirV2Stream {
    pub fn new(dir_fd: i32, buffer_size: u32) -> Self {
        Self {
            dir_fd,
            position: 0,
            entries_read: 0,
            total_bytes: 0,
            buffer_size,
            eof: false,
            sort_order: ReaddirV2Sort::None,
            filter_type: None,
            calls: 0,
        }
    }

    pub fn read_entries(&mut self, count: u32, avg_entry_size: u32) {
        self.calls += 1;
        self.entries_read += count as u64;
        self.total_bytes += (count * avg_entry_size) as u64;
        self.position += count as u64;
    }

    pub fn seek(&mut self, position: u64) {
        self.position = position;
        self.eof = false;
    }

    pub fn tell(&self) -> u64 {
        self.position
    }

    pub fn mark_eof(&mut self) {
        self.eof = true;
    }

    pub fn avg_entry_size(&self) -> u64 {
        if self.entries_read == 0 { 0 } else { self.total_bytes / self.entries_read }
    }
}

/// Readdir v2 app stats
#[derive(Debug, Clone)]
pub struct ReaddirV2AppStats {
    pub total_streams: u64,
    pub total_entries_read: u64,
    pub total_calls: u64,
    pub total_bytes: u64,
}

/// Main app readdir v2
#[derive(Debug)]
pub struct AppReaddirV2 {
    pub streams: BTreeMap<i32, ReaddirV2Stream>,
    pub stats: ReaddirV2AppStats,
}

impl AppReaddirV2 {
    pub fn new() -> Self {
        Self {
            streams: BTreeMap::new(),
            stats: ReaddirV2AppStats {
                total_streams: 0,
                total_entries_read: 0,
                total_calls: 0,
                total_bytes: 0,
            },
        }
    }

    pub fn open_dir(&mut self, fd: i32, buffer_size: u32) {
        self.streams.insert(fd, ReaddirV2Stream::new(fd, buffer_size));
        self.stats.total_streams += 1;
    }

    pub fn close_dir(&mut self, fd: i32) -> bool {
        self.streams.remove(&fd).is_some()
    }

    pub fn read(&mut self, fd: i32, count: u32) -> bool {
        if let Some(stream) = self.streams.get_mut(&fd) {
            stream.read_entries(count, 32);
            self.stats.total_entries_read += count as u64;
            self.stats.total_calls += 1;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Merged from readdir_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppDirEntryType {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}

/// Directory entry information
#[derive(Debug, Clone)]
pub struct AppDirEntryV3 {
    pub inode: u64,
    pub name: String,
    pub entry_type: AppDirEntryType,
    pub offset: u64,
    pub record_len: u16,
}

/// Directory listing state
#[derive(Debug, Clone)]
pub struct AppDirStreamV3 {
    pub fd: u64,
    pub path: String,
    pub position: u64,
    pub entries: Vec<AppDirEntryV3>,
    pub eof: bool,
}

/// Stats for readdir operations
#[derive(Debug, Clone)]
pub struct AppReaddirV3Stats {
    pub total_readdirs: u64,
    pub entries_returned: u64,
    pub directories_opened: u64,
    pub readdir_errors: u64,
    pub avg_entries_per_dir: u64,
}

/// Manager for directory listing app operations
pub struct AppReaddirV3Manager {
    streams: BTreeMap<u64, AppDirStreamV3>,
    stats: AppReaddirV3Stats,
    next_fd: u64,
}

impl AppReaddirV3Manager {
    pub fn new() -> Self {
        Self {
            streams: BTreeMap::new(),
            stats: AppReaddirV3Stats {
                total_readdirs: 0,
                entries_returned: 0,
                directories_opened: 0,
                readdir_errors: 0,
                avg_entries_per_dir: 0,
            },
            next_fd: 100,
        }
    }

    pub fn opendir(&mut self, path: &str) -> u64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        let stream = AppDirStreamV3 {
            fd,
            path: String::from(path),
            position: 0,
            entries: Vec::new(),
            eof: false,
        };
        self.streams.insert(fd, stream);
        self.stats.directories_opened += 1;
        fd
    }

    pub fn add_entry(&mut self, fd: u64, name: &str, entry_type: AppDirEntryType, inode: u64) -> bool {
        if let Some(stream) = self.streams.get_mut(&fd) {
            let offset = stream.entries.len() as u64;
            let entry = AppDirEntryV3 {
                inode,
                name: String::from(name),
                entry_type,
                offset,
                record_len: (name.len() + 19) as u16,
            };
            stream.entries.push(entry);
            true
        } else {
            false
        }
    }

    pub fn readdir(&mut self, fd: u64) -> Option<&AppDirEntryV3> {
        self.stats.total_readdirs += 1;
        if let Some(stream) = self.streams.get_mut(&fd) {
            let pos = stream.position as usize;
            if pos < stream.entries.len() {
                stream.position += 1;
                self.stats.entries_returned += 1;
                let stream_ref = self.streams.get(&fd).unwrap();
                Some(&stream_ref.entries[pos])
            } else {
                if let Some(s) = self.streams.get_mut(&fd) {
                    s.eof = true;
                }
                None
            }
        } else {
            self.stats.readdir_errors += 1;
            None
        }
    }

    pub fn closedir(&mut self, fd: u64) -> bool {
        self.streams.remove(&fd).is_some()
    }

    pub fn rewinddir(&mut self, fd: u64) {
        if let Some(stream) = self.streams.get_mut(&fd) {
            stream.position = 0;
            stream.eof = false;
        }
    }

    pub fn stats(&self) -> &AppReaddirV3Stats {
        &self.stats
    }
}
