// SPDX-License-Identifier: GPL-2.0
//! Apps umask_mgr — file creation mask management per process.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Standard umask permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmaskValue(pub u32);

impl UmaskValue {
    pub const NONE: Self = Self(0o000);
    pub const SECURE: Self = Self(0o077);
    pub const DEFAULT: Self = Self(0o022);
    pub const RESTRICTIVE: Self = Self(0o027);
    pub const GROUP_WRITE: Self = Self(0o002);

    pub fn owner_bits(&self) -> u32 { (self.0 >> 6) & 0o7 }
    pub fn group_bits(&self) -> u32 { (self.0 >> 3) & 0o7 }
    pub fn other_bits(&self) -> u32 { self.0 & 0o7 }

    pub fn apply_to_file(&self, mode: u32) -> u32 {
        mode & !self.0 & 0o777
    }

    pub fn apply_to_dir(&self, mode: u32) -> u32 {
        mode & !self.0 & 0o777
    }

    pub fn is_secure(&self) -> bool {
        self.group_bits() == 7 && self.other_bits() == 7
    }

    pub fn blocks_world_read(&self) -> bool { self.0 & 0o004 != 0 }
    pub fn blocks_world_write(&self) -> bool { self.0 & 0o002 != 0 }
    pub fn blocks_world_exec(&self) -> bool { self.0 & 0o001 != 0 }
    pub fn blocks_group_write(&self) -> bool { self.0 & 0o020 != 0 }
}

/// Umask change event
#[derive(Debug, Clone, Copy)]
pub struct UmaskChangeEvent {
    pub pid: u32,
    pub old_mask: UmaskValue,
    pub new_mask: UmaskValue,
    pub timestamp: u64,
}

/// Per-process umask state
#[derive(Debug)]
pub struct ProcessUmaskState {
    pub pid: u32,
    pub current_mask: UmaskValue,
    pub initial_mask: UmaskValue,
    pub change_count: u64,
    pub file_creates: u64,
    pub dir_creates: u64,
    pub last_change: u64,
}

impl ProcessUmaskState {
    pub fn new(pid: u32, mask: UmaskValue) -> Self {
        Self {
            pid, current_mask: mask, initial_mask: mask,
            change_count: 0, file_creates: 0, dir_creates: 0, last_change: 0,
        }
    }

    pub fn set_mask(&mut self, new_mask: UmaskValue, now: u64) -> UmaskValue {
        let old = self.current_mask;
        self.current_mask = new_mask;
        self.change_count += 1;
        self.last_change = now;
        old
    }

    pub fn effective_file_mode(&self, requested: u32) -> u32 {
        self.current_mask.apply_to_file(requested)
    }

    pub fn effective_dir_mode(&self, requested: u32) -> u32 {
        self.current_mask.apply_to_dir(requested)
    }

    pub fn record_file_create(&mut self) { self.file_creates += 1; }
    pub fn record_dir_create(&mut self) { self.dir_creates += 1; }
}

/// Security assessment of a umask
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UmaskSecurityLevel {
    Secure,
    Moderate,
    Permissive,
    Dangerous,
}

impl UmaskSecurityLevel {
    pub fn assess(mask: UmaskValue) -> Self {
        let other = mask.other_bits();
        let group = mask.group_bits();
        if other == 7 && group >= 5 { return Self::Secure; }
        if other >= 2 && group >= 2 { return Self::Moderate; }
        if other > 0 || group > 0 { return Self::Permissive; }
        Self::Dangerous
    }
}

/// Umask policy rule
#[derive(Debug, Clone)]
pub struct UmaskPolicy {
    pub min_mask: UmaskValue,
    pub enforced: bool,
    pub applies_to_uid: Option<u32>,
    pub applies_to_gid: Option<u32>,
}

impl UmaskPolicy {
    pub fn check(&self, mask: UmaskValue) -> bool {
        (mask.0 & self.min_mask.0) == self.min_mask.0
    }
}

/// Umask manager stats
#[derive(Debug, Clone)]
pub struct UmaskMgrStats {
    pub tracked_processes: u32,
    pub total_changes: u64,
    pub total_file_creates: u64,
    pub total_dir_creates: u64,
    pub insecure_masks: u32,
    pub policy_violations: u64,
}

/// Main umask manager
pub struct AppUmaskMgr {
    processes: BTreeMap<u32, ProcessUmaskState>,
    events: Vec<UmaskChangeEvent>,
    max_events: usize,
    policies: Vec<UmaskPolicy>,
    default_mask: UmaskValue,
    policy_violations: u64,
}

impl AppUmaskMgr {
    pub fn new(default_mask: UmaskValue) -> Self {
        Self {
            processes: BTreeMap::new(), events: Vec::new(),
            max_events: 2048, policies: Vec::new(),
            default_mask, policy_violations: 0,
        }
    }

    pub fn create_process(&mut self, pid: u32, mask: Option<UmaskValue>) {
        let m = mask.unwrap_or(self.default_mask);
        self.processes.insert(pid, ProcessUmaskState::new(pid, m));
    }

    pub fn remove_process(&mut self, pid: u32) -> bool {
        self.processes.remove(&pid).is_some()
    }

    pub fn set_umask(&mut self, pid: u32, new_mask: UmaskValue, now: u64) -> Option<UmaskValue> {
        // Check policies
        for policy in &self.policies {
            if policy.enforced && !policy.check(new_mask) {
                self.policy_violations += 1;
                return None;
            }
        }
        let state = self.processes.get_mut(&pid)?;
        let old = state.set_mask(new_mask, now);

        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push(UmaskChangeEvent { pid, old_mask: old, new_mask: new_mask, timestamp: now });
        Some(old)
    }

    pub fn get_umask(&self, pid: u32) -> Option<UmaskValue> {
        self.processes.get(&pid).map(|s| s.current_mask)
    }

    pub fn apply_file_mode(&mut self, pid: u32, requested: u32) -> u32 {
        if let Some(state) = self.processes.get_mut(&pid) {
            state.record_file_create();
            state.effective_file_mode(requested)
        } else {
            self.default_mask.apply_to_file(requested)
        }
    }

    pub fn apply_dir_mode(&mut self, pid: u32, requested: u32) -> u32 {
        if let Some(state) = self.processes.get_mut(&pid) {
            state.record_dir_create();
            state.effective_dir_mode(requested)
        } else {
            self.default_mask.apply_to_dir(requested)
        }
    }

    pub fn fork_umask(&mut self, parent: u32, child: u32) -> bool {
        if let Some(parent_state) = self.processes.get(&parent) {
            let mask = parent_state.current_mask;
            self.processes.insert(child, ProcessUmaskState::new(child, mask));
            true
        } else { false }
    }

    pub fn add_policy(&mut self, policy: UmaskPolicy) {
        self.policies.push(policy);
    }

    pub fn insecure_processes(&self) -> Vec<(u32, UmaskValue, UmaskSecurityLevel)> {
        self.processes.iter()
            .filter_map(|(&pid, state)| {
                let level = UmaskSecurityLevel::assess(state.current_mask);
                if matches!(level, UmaskSecurityLevel::Permissive | UmaskSecurityLevel::Dangerous) {
                    Some((pid, state.current_mask, level))
                } else { None }
            })
            .collect()
    }

    pub fn stats(&self) -> UmaskMgrStats {
        let total_changes: u64 = self.processes.values().map(|p| p.change_count).sum();
        let total_fc: u64 = self.processes.values().map(|p| p.file_creates).sum();
        let total_dc: u64 = self.processes.values().map(|p| p.dir_creates).sum();
        let insecure = self.insecure_processes().len() as u32;
        UmaskMgrStats {
            tracked_processes: self.processes.len() as u32,
            total_changes, total_file_creates: total_fc,
            total_dir_creates: total_dc, insecure_masks: insecure,
            policy_violations: self.policy_violations,
        }
    }
}
