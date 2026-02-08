//! # Bridge Procfs Bridge
//!
//! Bridges procfs reads/writes between kernel and userspace:
//! - Per-process proc entries
//! - System-wide proc entries
//! - Seq file emulation
//! - Dynamic proc file registration
//! - Access control and filtering

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Proc entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcEntryType {
    Dir,
    File,
    Symlink,
    SeqFile,
    BinaryFile,
}

/// Proc namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcNamespace {
    Global,
    PerProcess,
    PerThread,
    Net,
    Sys,
}

/// Single proc entry
#[derive(Debug, Clone)]
pub struct ProcEntry {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub entry_type: ProcEntryType,
    pub ns: ProcNamespace,
    pub mode: u16,
    pub owner_uid: u32,
    pub owner_gid: u32,
    pub data: Vec<u8>,
    pub data_gen: u64,
    pub children: Vec<u64>,
    pub read_count: u64,
    pub write_count: u64,
    pub seq_pos: u64,
}

impl ProcEntry {
    pub fn new(id: u64, name: String, etype: ProcEntryType, ns: ProcNamespace, parent: Option<u64>) -> Self {
        Self {
            id, name, parent_id: parent, entry_type: etype, ns, mode: 0o444,
            owner_uid: 0, owner_gid: 0, data: Vec::new(), data_gen: 0,
            children: Vec::new(), read_count: 0, write_count: 0, seq_pos: 0,
        }
    }

    pub fn read(&mut self, offset: usize, len: usize) -> &[u8] {
        self.read_count += 1;
        let end = (offset + len).min(self.data.len());
        if offset >= self.data.len() { return &[]; }
        &self.data[offset..end]
    }

    pub fn write(&mut self, data: &[u8]) -> bool {
        if self.mode & 0o200 == 0 { return false; }
        self.data = data.into();
        self.data_gen += 1;
        self.write_count += 1;
        true
    }

    pub fn set_data(&mut self, data: Vec<u8>) { self.data = data; self.data_gen += 1; }
    pub fn size(&self) -> usize { self.data.len() }
}

/// Per-process proc state
#[derive(Debug, Clone)]
pub struct ProcessProcState {
    pub pid: u64,
    pub cmdline: String,
    pub status_data: Vec<u8>,
    pub stat_data: Vec<u8>,
    pub maps_data: Vec<u8>,
    pub fd_count: u32,
    pub entry_ids: Vec<u64>,
}

impl ProcessProcState {
    pub fn new(pid: u64, cmdline: String) -> Self {
        Self { pid, cmdline, status_data: Vec::new(), stat_data: Vec::new(), maps_data: Vec::new(), fd_count: 0, entry_ids: Vec::new() }
    }
}

/// Access check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcAccessResult {
    Allowed,
    Denied,
    Hidden,
    Filtered,
}

/// Proc bridge stats
#[derive(Debug, Clone, Default)]
pub struct ProcfsBridgeStats {
    pub total_entries: usize,
    pub per_process_entries: usize,
    pub total_reads: u64,
    pub total_writes: u64,
    pub access_denied: u64,
    pub total_data_bytes: u64,
}

/// Bridge procfs proxy
pub struct BridgeProcfsBridge {
    entries: BTreeMap<u64, ProcEntry>,
    processes: BTreeMap<u64, ProcessProcState>,
    stats: ProcfsBridgeStats,
    next_id: u64,
}

impl BridgeProcfsBridge {
    pub fn new() -> Self {
        let mut entries = BTreeMap::new();
        entries.insert(0, ProcEntry::new(0, String::from("/proc"), ProcEntryType::Dir, ProcNamespace::Global, None));
        Self { entries, processes: BTreeMap::new(), stats: ProcfsBridgeStats::default(), next_id: 1 }
    }

    pub fn register_entry(&mut self, name: String, etype: ProcEntryType, ns: ProcNamespace, parent: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let entry = ProcEntry::new(id, name, etype, ns, Some(parent));
        self.entries.insert(id, entry);
        if let Some(p) = self.entries.get_mut(&parent) { p.children.push(id); }
        id
    }

    pub fn unregister_entry(&mut self, id: u64) -> bool {
        if id == 0 { return false; }
        let parent = self.entries.get(&id).and_then(|e| e.parent_id);
        let has_children = self.entries.get(&id).map(|e| !e.children.is_empty()).unwrap_or(true);
        if has_children { return false; }
        self.entries.remove(&id);
        if let Some(pid) = parent { if let Some(p) = self.entries.get_mut(&pid) { p.children.retain(|&c| c != id); } }
        true
    }

    pub fn register_process(&mut self, pid: u64, cmdline: String) {
        self.processes.insert(pid, ProcessProcState::new(pid, cmdline));
    }

    pub fn unregister_process(&mut self, pid: u64) {
        if let Some(p) = self.processes.remove(&pid) {
            for eid in &p.entry_ids { self.entries.remove(eid); }
        }
    }

    pub fn update_process_status(&mut self, pid: u64, status: Vec<u8>) {
        if let Some(p) = self.processes.get_mut(&pid) { p.status_data = status; }
    }

    pub fn update_process_stat(&mut self, pid: u64, stat: Vec<u8>) {
        if let Some(p) = self.processes.get_mut(&pid) { p.stat_data = stat; }
    }

    pub fn read_entry(&mut self, id: u64, offset: usize, len: usize) -> Option<Vec<u8>> {
        self.entries.get_mut(&id).map(|e| e.read(offset, len).to_vec())
    }

    pub fn write_entry(&mut self, id: u64, data: &[u8]) -> bool {
        self.entries.get_mut(&id).map(|e| e.write(data)).unwrap_or(false)
    }

    pub fn set_entry_data(&mut self, id: u64, data: Vec<u8>) {
        if let Some(e) = self.entries.get_mut(&id) { e.set_data(data); }
    }

    pub fn check_access(&self, id: u64, uid: u32, want_write: bool) -> ProcAccessResult {
        if let Some(e) = self.entries.get(&id) {
            if want_write && e.mode & 0o200 == 0 { return ProcAccessResult::Denied; }
            if !want_write && e.mode & 0o444 == 0 { return ProcAccessResult::Denied; }
            if uid != 0 && uid != e.owner_uid && e.mode & 0o004 == 0 { return ProcAccessResult::Hidden; }
            ProcAccessResult::Allowed
        } else { ProcAccessResult::Denied }
    }

    pub fn list_children(&self, id: u64) -> Vec<u64> { self.entries.get(&id).map(|e| e.children.clone()).unwrap_or_default() }

    pub fn recompute(&mut self) {
        self.stats.total_entries = self.entries.len();
        self.stats.per_process_entries = self.processes.values().map(|p| p.entry_ids.len()).sum();
        self.stats.total_reads = self.entries.values().map(|e| e.read_count).sum();
        self.stats.total_writes = self.entries.values().map(|e| e.write_count).sum();
        self.stats.total_data_bytes = self.entries.values().map(|e| e.data.len() as u64).sum();
    }

    pub fn entry(&self, id: u64) -> Option<&ProcEntry> { self.entries.get(&id) }
    pub fn process(&self, pid: u64) -> Option<&ProcessProcState> { self.processes.get(&pid) }
    pub fn stats(&self) -> &ProcfsBridgeStats { &self.stats }
}
