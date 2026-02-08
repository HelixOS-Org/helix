// SPDX-License-Identifier: GPL-2.0
//! Apps umask_app — file creation mask management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode(pub u32);

impl FileMode {
    pub const S_ISUID: u32 = 0o4000;
    pub const S_ISGID: u32 = 0o2000;
    pub const S_ISVTX: u32 = 0o1000;
    pub const S_IRUSR: u32 = 0o0400;
    pub const S_IWUSR: u32 = 0o0200;
    pub const S_IXUSR: u32 = 0o0100;
    pub const S_IRGRP: u32 = 0o0040;
    pub const S_IWGRP: u32 = 0o0020;
    pub const S_IXGRP: u32 = 0o0010;
    pub const S_IROTH: u32 = 0o0004;
    pub const S_IWOTH: u32 = 0o0002;
    pub const S_IXOTH: u32 = 0o0001;

    pub fn new(mode: u32) -> Self { Self(mode & 0o7777) }
    pub fn apply_umask(&self, umask: &UmaskValue) -> Self { Self(self.0 & !umask.0) }
    pub fn has(&self, perm: u32) -> bool { self.0 & perm != 0 }
    pub fn is_world_writable(&self) -> bool { self.has(Self::S_IWOTH) }
    pub fn is_setuid(&self) -> bool { self.has(Self::S_ISUID) }
}

/// Umask value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmaskValue(pub u32);

impl UmaskValue {
    pub fn new(mask: u32) -> Self { Self(mask & 0o777) }
    pub fn default_value() -> Self { Self(0o022) }
    pub fn restrictive() -> Self { Self(0o077) }
    pub fn permissive() -> Self { Self(0o000) }
}

/// Process umask state
#[derive(Debug)]
pub struct ProcessUmask {
    pub pid: u64,
    pub current: UmaskValue,
    pub change_count: u64,
    pub files_created: u64,
    pub last_change: u64,
    pub history: Vec<(u64, UmaskValue)>,
}

impl ProcessUmask {
    pub fn new(pid: u64) -> Self {
        Self { pid, current: UmaskValue::default_value(), change_count: 0, files_created: 0, last_change: 0, history: Vec::new() }
    }

    pub fn set_umask(&mut self, mask: UmaskValue, now: u64) -> UmaskValue {
        let old = self.current;
        self.current = mask;
        self.change_count += 1;
        self.last_change = now;
        if self.history.len() < 64 { self.history.push((now, mask)); }
        old
    }

    pub fn create_file(&mut self, requested: FileMode) -> FileMode {
        self.files_created += 1;
        requested.apply_umask(&self.current)
    }
}

/// Umask audit event
#[derive(Debug, Clone)]
pub struct UmaskAuditEvent {
    pub pid: u64,
    pub old_mask: UmaskValue,
    pub new_mask: UmaskValue,
    pub timestamp: u64,
    pub suspicious: bool,
}

/// Stats
#[derive(Debug, Clone)]
pub struct UmaskAppStats {
    pub total_processes: u32,
    pub total_changes: u64,
    pub total_files_created: u64,
    pub permissive_count: u32,
    pub restrictive_count: u32,
    pub suspicious_changes: u64,
}

/// Main umask app
pub struct AppUmask {
    processes: BTreeMap<u64, ProcessUmask>,
    audit_log: Vec<UmaskAuditEvent>,
    max_audit: usize,
}

impl AppUmask {
    pub fn new() -> Self { Self { processes: BTreeMap::new(), audit_log: Vec::new(), max_audit: 4096 } }

    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessUmask::new(pid)); }

    pub fn set_umask(&mut self, pid: u64, mask: UmaskValue, now: u64) -> Option<UmaskValue> {
        let proc = self.processes.get_mut(&pid)?;
        let old = proc.set_umask(mask, now);
        let suspicious = mask == UmaskValue::permissive();
        if self.audit_log.len() >= self.max_audit { self.audit_log.drain(..self.max_audit / 2); }
        self.audit_log.push(UmaskAuditEvent { pid, old_mask: old, new_mask: mask, timestamp: now, suspicious });
        Some(old)
    }

    pub fn stats(&self) -> UmaskAppStats {
        let changes: u64 = self.processes.values().map(|p| p.change_count).sum();
        let files: u64 = self.processes.values().map(|p| p.files_created).sum();
        let perm = self.processes.values().filter(|p| p.current == UmaskValue::permissive()).count() as u32;
        let rest = self.processes.values().filter(|p| p.current == UmaskValue::restrictive()).count() as u32;
        let susp = self.audit_log.iter().filter(|e| e.suspicious).count() as u64;
        UmaskAppStats { total_processes: self.processes.len() as u32, total_changes: changes, total_files_created: files, permissive_count: perm, restrictive_count: rest, suspicious_changes: susp }
    }
}

// ============================================================================
// Merged from umask_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmaskV2Scope {
    Global,
    PerUser,
    PerDirectory,
    PerProcess,
}

/// Permission bit field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmaskV2Bits {
    pub value: u32,
}

impl UmaskV2Bits {
    pub fn new(value: u32) -> Self {
        Self { value: value & 0o777 }
    }

    pub fn owner_read(&self) -> bool { (self.value & 0o400) != 0 }
    pub fn owner_write(&self) -> bool { (self.value & 0o200) != 0 }
    pub fn owner_exec(&self) -> bool { (self.value & 0o100) != 0 }
    pub fn group_read(&self) -> bool { (self.value & 0o040) != 0 }
    pub fn group_write(&self) -> bool { (self.value & 0o020) != 0 }
    pub fn group_exec(&self) -> bool { (self.value & 0o010) != 0 }
    pub fn other_read(&self) -> bool { (self.value & 0o004) != 0 }
    pub fn other_write(&self) -> bool { (self.value & 0o002) != 0 }
    pub fn other_exec(&self) -> bool { (self.value & 0o001) != 0 }

    pub fn apply_to_mode(&self, mode: u32) -> u32 {
        mode & !self.value
    }

    pub fn is_restrictive(&self) -> bool {
        self.value >= 0o077
    }
}

/// A per-directory umask override.
#[derive(Debug, Clone)]
pub struct UmaskV2DirOverride {
    pub dir_path_hash: u64,
    pub umask: UmaskV2Bits,
    pub scope: UmaskV2Scope,
    pub apply_to_files: bool,
    pub apply_to_dirs: bool,
    pub apply_count: u64,
}

impl UmaskV2DirOverride {
    pub fn new(dir_path_hash: u64, umask: u32) -> Self {
        Self {
            dir_path_hash,
            umask: UmaskV2Bits::new(umask),
            scope: UmaskV2Scope::PerDirectory,
            apply_to_files: true,
            apply_to_dirs: true,
            apply_count: 0,
        }
    }
}

/// Per-process umask state.
#[derive(Debug, Clone)]
pub struct ProcessUmaskV2State {
    pub pid: u64,
    pub current_umask: UmaskV2Bits,
    pub changes: u64,
    pub inherited_from: Option<u64>,
    pub files_created: u64,
    pub dirs_created: u64,
}

impl ProcessUmaskV2State {
    pub fn new(pid: u64, initial: u32) -> Self {
        Self {
            pid,
            current_umask: UmaskV2Bits::new(initial),
            changes: 0,
            inherited_from: None,
            files_created: 0,
            dirs_created: 0,
        }
    }

    pub fn set_umask(&mut self, new_umask: u32) -> u32 {
        let old = self.current_umask.value;
        self.current_umask = UmaskV2Bits::new(new_umask);
        self.changes += 1;
        old
    }
}

/// Statistics for umask V2 app.
#[derive(Debug, Clone)]
pub struct UmaskV2AppStats {
    pub total_umask_calls: u64,
    pub total_overrides: u64,
    pub restrictive_umasks: u64,
    pub permissive_umasks: u64,
    pub dir_override_hits: u64,
    pub inherited_count: u64,
}

/// Main apps umask V2 manager.
pub struct AppUmaskV2 {
    pub processes: BTreeMap<u64, ProcessUmaskV2State>,
    pub dir_overrides: BTreeMap<u64, UmaskV2DirOverride>,
    pub stats: UmaskV2AppStats,
}

impl AppUmaskV2 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            dir_overrides: BTreeMap::new(),
            stats: UmaskV2AppStats {
                total_umask_calls: 0,
                total_overrides: 0,
                restrictive_umasks: 0,
                permissive_umasks: 0,
                dir_override_hits: 0,
                inherited_count: 0,
            },
        }
    }

    pub fn set_umask(&mut self, pid: u64, new_umask: u32) -> u32 {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessUmaskV2State::new(pid, 0o022));
        let old = state.set_umask(new_umask);
        self.stats.total_umask_calls += 1;
        if UmaskV2Bits::new(new_umask).is_restrictive() {
            self.stats.restrictive_umasks += 1;
        } else {
            self.stats.permissive_umasks += 1;
        }
        old
    }

    pub fn add_dir_override(&mut self, dir_path_hash: u64, umask: u32) {
        let over = UmaskV2DirOverride::new(dir_path_hash, umask);
        self.dir_overrides.insert(dir_path_hash, over);
        self.stats.total_overrides += 1;
    }

    pub fn effective_umask(&mut self, pid: u64, dir_hash: Option<u64>) -> u32 {
        let base = self
            .processes
            .get(&pid)
            .map(|s| s.current_umask.value)
            .unwrap_or(0o022);
        if let Some(dh) = dir_hash {
            if let Some(over) = self.dir_overrides.get_mut(&dh) {
                over.apply_count += 1;
                self.stats.dir_override_hits += 1;
                return over.umask.value;
            }
        }
        base
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

// ============================================================================
// Merged from umask_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmaskV3Context {
    Process,
    Thread,
    Inherited,
    Acl,
    DefaultAcl,
    Namespace,
}

/// Umask v3 change record
#[derive(Debug, Clone)]
pub struct UmaskV3Record {
    pub old_mask: u16,
    pub new_mask: u16,
    pub context: UmaskV3Context,
    pub pid: u32,
    pub tid: u32,
    pub ns_id: u32,
    pub timestamp_ns: u64,
}

impl UmaskV3Record {
    pub fn new(old_mask: u16, new_mask: u16, context: UmaskV3Context) -> Self {
        Self {
            old_mask,
            new_mask,
            context,
            pid: 0,
            tid: 0,
            ns_id: 0,
            timestamp_ns: 0,
        }
    }

    pub fn is_restrictive_change(&self) -> bool { self.new_mask > self.old_mask }
    pub fn is_permissive_change(&self) -> bool { self.new_mask < self.old_mask }
    pub fn effective_permissions(&self, requested: u16) -> u16 { requested & !self.new_mask }
    pub fn blocks_group_write(&self) -> bool { self.new_mask & 0o020 != 0 }
    pub fn blocks_other_all(&self) -> bool { self.new_mask & 0o007 == 0o007 }
    pub fn is_secure_default(&self) -> bool { self.new_mask >= 0o077 }
}

/// Per-thread umask v3 state
#[derive(Debug, Clone)]
pub struct ThreadUmaskV3State {
    pub tid: u32,
    pub current_mask: u16,
    pub acl_mask: Option<u16>,
    pub changes: u64,
    pub files_created: u64,
    pub dirs_created: u64,
}

impl ThreadUmaskV3State {
    pub fn new(tid: u32, mask: u16) -> Self {
        Self { tid, current_mask: mask, acl_mask: None, changes: 0, files_created: 0, dirs_created: 0 }
    }

    pub fn set_mask(&mut self, mask: u16) -> u16 {
        let old = self.current_mask;
        self.current_mask = mask;
        self.changes += 1;
        old
    }

    pub fn effective_mask(&self) -> u16 {
        match self.acl_mask {
            Some(acl) => self.current_mask | acl,
            None => self.current_mask,
        }
    }

    pub fn effective_file_mode(&self, mode: u16) -> u16 {
        mode & !self.effective_mask() & 0o777
    }

    pub fn record_file_create(&mut self) { self.files_created += 1; }
    pub fn record_dir_create(&mut self) { self.dirs_created += 1; }
}

/// Namespace umask policy
#[derive(Debug, Clone)]
pub struct NsUmaskPolicy {
    pub ns_id: u32,
    pub min_mask: u16,
    pub enforced: bool,
    pub thread_count: u32,
}

impl NsUmaskPolicy {
    pub fn new(ns_id: u32, min_mask: u16) -> Self {
        Self { ns_id, min_mask, enforced: true, thread_count: 0 }
    }

    pub fn apply(&self, requested_mask: u16) -> u16 {
        if self.enforced { requested_mask | self.min_mask } else { requested_mask }
    }
}

/// Umask v3 app stats
#[derive(Debug, Clone)]
pub struct UmaskV3AppStats {
    pub total_changes: u64,
    pub restrictive_changes: u64,
    pub permissive_changes: u64,
    pub acl_overrides: u64,
    pub common_masks: BTreeMap<u16, u64>,
}

/// Main app umask v3
#[derive(Debug)]
pub struct AppUmaskV3 {
    pub thread_states: BTreeMap<u32, ThreadUmaskV3State>,
    pub ns_policies: BTreeMap<u32, NsUmaskPolicy>,
    pub stats: UmaskV3AppStats,
    pub default_mask: u16,
}

impl AppUmaskV3 {
    pub fn new(default_mask: u16) -> Self {
        Self {
            thread_states: BTreeMap::new(),
            ns_policies: BTreeMap::new(),
            stats: UmaskV3AppStats {
                total_changes: 0,
                restrictive_changes: 0,
                permissive_changes: 0,
                acl_overrides: 0,
                common_masks: BTreeMap::new(),
            },
            default_mask,
        }
    }

    pub fn record(&mut self, record: &UmaskV3Record) {
        self.stats.total_changes += 1;
        if record.is_restrictive_change() { self.stats.restrictive_changes += 1; }
        else if record.is_permissive_change() { self.stats.permissive_changes += 1; }
        let effective_mask = if let Some(policy) = self.ns_policies.get(&record.ns_id) {
            policy.apply(record.new_mask)
        } else { record.new_mask };
        *self.stats.common_masks.entry(effective_mask).or_insert(0) += 1;
        let state = self.thread_states.entry(record.tid)
            .or_insert_with(|| ThreadUmaskV3State::new(record.tid, self.default_mask));
        state.set_mask(effective_mask);
    }

    pub fn most_common_mask(&self) -> Option<(u16, u64)> {
        self.stats.common_masks.iter().max_by_key(|(_, &c)| c).map(|(&m, &c)| (m, c))
    }
}

// ============================================================================
// Merged from umask_v4_app
// ============================================================================

#[derive(Debug, Clone)]
pub struct UmaskV4Record {
    pub old_mask: u32,
    pub new_mask: u32,
    pub pid: u32,
    pub tid: u32,
}

impl UmaskV4Record {
    pub fn new(new_mask: u32) -> Self {
        Self { old_mask: 0o022, new_mask, pid: 0, tid: 0 }
    }

    pub fn effective_mode(&self, requested: u32) -> u32 {
        requested & !self.new_mask
    }
}

/// Umask v4 app stats
#[derive(Debug, Clone)]
pub struct UmaskV4AppStats {
    pub total_ops: u64,
    pub restrictive_masks: u64,
    pub permissive_masks: u64,
}

/// Main app umask v4
#[derive(Debug)]
pub struct AppUmaskV4 {
    pub stats: UmaskV4AppStats,
    pub default_mask: u32,
}

impl AppUmaskV4 {
    pub fn new() -> Self {
        Self { stats: UmaskV4AppStats { total_ops: 0, restrictive_masks: 0, permissive_masks: 0 }, default_mask: 0o022 }
    }

    pub fn record(&mut self, rec: &UmaskV4Record) {
        self.stats.total_ops += 1;
        if rec.new_mask >= 0o077 {
            self.stats.restrictive_masks += 1;
        } else {
            self.stats.permissive_masks += 1;
        }
    }
}
