// SPDX-License-Identifier: GPL-2.0
//! Apps chroot_app — chroot/pivot filesystem root.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Chroot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChrootState {
    Normal,
    Chrooted,
    PivotRoot,
    Escaped,
}

/// Chroot entry
#[derive(Debug)]
pub struct ChrootEntry {
    pub pid: u64,
    pub state: ChrootState,
    pub root_path_hash: u64,
    pub old_root_hash: u64,
    pub depth: u32,
    pub created_at: u64,
    pub escapes_blocked: u64,
}

impl ChrootEntry {
    pub fn new(pid: u64, root_hash: u64, now: u64) -> Self {
        Self { pid, state: ChrootState::Chrooted, root_path_hash: root_hash, old_root_hash: 0, depth: 1, created_at: now, escapes_blocked: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ChrootAppStats {
    pub total_chroots: u32,
    pub active_chroots: u32,
    pub total_escapes_blocked: u64,
    pub max_depth: u32,
}

/// Main chroot app
pub struct AppChroot {
    entries: BTreeMap<u64, ChrootEntry>,
}

impl AppChroot {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn chroot(&mut self, pid: u64, root_hash: u64, now: u64) {
        let mut entry = ChrootEntry::new(pid, root_hash, now);
        if let Some(old) = self.entries.get(&pid) { entry.depth = old.depth + 1; entry.old_root_hash = old.root_path_hash; }
        self.entries.insert(pid, entry);
    }

    pub fn exit(&mut self, pid: u64) { self.entries.remove(&pid); }

    pub fn stats(&self) -> ChrootAppStats {
        let active = self.entries.values().filter(|e| e.state == ChrootState::Chrooted).count() as u32;
        let escapes: u64 = self.entries.values().map(|e| e.escapes_blocked).sum();
        let depth = self.entries.values().map(|e| e.depth).max().unwrap_or(0);
        ChrootAppStats { total_chroots: self.entries.len() as u32, active_chroots: active, total_escapes_blocked: escapes, max_depth: depth }
    }
}
