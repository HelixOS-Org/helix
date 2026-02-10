// SPDX-License-Identifier: GPL-2.0
//! Apps env_mgr — process environment variable management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Environment variable source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvSource {
    Inherited,
    Explicit,
    SystemDefault,
    SecurityPolicy,
    ProfileConfig,
}

/// Environment variable entry
#[derive(Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub source: EnvSource,
    pub read_only: bool,
    pub sensitive: bool,
    pub access_count: u64,
    pub modified_at: u64,
}

impl EnvVar {
    pub fn new(key: String, value: String, source: EnvSource, now: u64) -> Self {
        Self { key, value, source, read_only: false, sensitive: false, access_count: 0, modified_at: now }
    }

    #[inline(always)]
    pub fn total_size(&self) -> usize {
        self.key.len() + 1 + self.value.len() + 1
    }
}

/// Environment block for a process
#[derive(Debug)]
pub struct ProcessEnvBlock {
    pub pid: u32,
    pub vars: BTreeMap<String, EnvVar>,
    pub max_vars: usize,
    pub max_total_size: usize,
    pub created_at: u64,
}

impl ProcessEnvBlock {
    pub fn new(pid: u32, max_vars: usize, now: u64) -> Self {
        Self { pid, vars: BTreeMap::new(), max_vars, max_total_size: 131072, created_at: now }
    }

    #[inline]
    pub fn get(&mut self, key: &str) -> Option<&str> {
        if let Some(v) = self.vars.get_mut(key) {
            v.access_count += 1;
            Some(&v.value)
        } else { None }
    }

    pub fn set(&mut self, key: String, value: String, source: EnvSource, now: u64) -> bool {
        if let Some(existing) = self.vars.get(&key) {
            if existing.read_only { return false; }
        }
        if !self.vars.contains_key(&key) && self.vars.len() >= self.max_vars {
            return false;
        }
        let total = self.current_size() + key.len() + value.len() + 2;
        if total > self.max_total_size { return false; }
        self.vars.insert(key.clone(), EnvVar::new(key, value, source, now));
        true
    }

    #[inline]
    pub fn unset(&mut self, key: &str) -> bool {
        if let Some(v) = self.vars.get(key) {
            if v.read_only { return false; }
        }
        self.vars.remove(key).is_some()
    }

    #[inline(always)]
    pub fn current_size(&self) -> usize {
        self.vars.values().map(|v| v.total_size()).sum()
    }

    #[inline(always)]
    pub fn count(&self) -> usize { self.vars.len() }

    #[inline(always)]
    pub fn keys(&self) -> Vec<&str> {
        self.vars.keys().map(|k| k.as_str()).collect()
    }

    #[inline]
    pub fn to_envp(&self) -> Vec<String> {
        self.vars.values()
            .map(|v| alloc::format!("{}={}", v.key, v.value))
            .collect()
    }

    #[inline]
    pub fn inherit_from(&mut self, parent: &ProcessEnvBlock, now: u64) {
        for (key, var) in &parent.vars {
            if !var.sensitive {
                let mut new_var = var.clone();
                new_var.source = EnvSource::Inherited;
                new_var.access_count = 0;
                new_var.modified_at = now;
                self.vars.insert(key.clone(), new_var);
            }
        }
    }

    #[inline(always)]
    pub fn clear_sensitive(&mut self) {
        self.vars.retain(|_, v| !v.sensitive);
    }

    #[inline(always)]
    pub fn mark_sensitive(&mut self, key: &str) {
        if let Some(v) = self.vars.get_mut(key) { v.sensitive = true; }
    }

    #[inline(always)]
    pub fn mark_readonly(&mut self, key: &str) {
        if let Some(v) = self.vars.get_mut(key) { v.read_only = true; }
    }
}

/// Environment change event
#[derive(Debug, Clone)]
pub struct EnvChangeEvent {
    pub pid: u32,
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: u64,
}

/// Env manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EnvMgrStats {
    pub tracked_processes: u32,
    pub total_vars: u64,
    pub total_accesses: u64,
    pub total_modifications: u64,
    pub total_size_bytes: u64,
}

/// Main environment manager
pub struct AppEnvMgr {
    blocks: BTreeMap<u32, ProcessEnvBlock>,
    events: VecDeque<EnvChangeEvent>,
    max_events: usize,
    default_max_vars: usize,
    total_accesses: u64,
    total_modifications: u64,
    blocked_vars: Vec<String>,
}

impl AppEnvMgr {
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(), events: VecDeque::new(),
            max_events: 4096, default_max_vars: 1024,
            total_accesses: 0, total_modifications: 0,
            blocked_vars: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn create_block(&mut self, pid: u32, now: u64) {
        let block = ProcessEnvBlock::new(pid, self.default_max_vars, now);
        self.blocks.insert(pid, block);
    }

    #[inline(always)]
    pub fn remove_block(&mut self, pid: u32) -> bool {
        self.blocks.remove(&pid).is_some()
    }

    #[inline]
    pub fn get_var(&mut self, pid: u32, key: &str) -> Option<String> {
        self.total_accesses += 1;
        let block = self.blocks.get_mut(&pid)?;
        block.get(key).map(|s| String::from(s))
    }

    pub fn set_var(&mut self, pid: u32, key: String, value: String, now: u64) -> bool {
        if self.blocked_vars.iter().any(|b| b == &key) { return false; }
        if let Some(block) = self.blocks.get_mut(&pid) {
            let old = block.vars.get(&key).map(|v| v.value.clone());
            if block.set(key.clone(), value.clone(), EnvSource::Explicit, now) {
                self.total_modifications += 1;
                self.record_event(EnvChangeEvent {
                    pid, key, old_value: old, new_value: Some(value), timestamp: now,
                });
                return true;
            }
        }
        false
    }

    pub fn fork_env(&mut self, parent_pid: u32, child_pid: u32, now: u64) -> bool {
        if let Some(parent) = self.blocks.get(&parent_pid) {
            let vars_clone: BTreeMap<String, EnvVar> = parent.vars.clone();
            let max = parent.max_vars;
            let mut child = ProcessEnvBlock::new(child_pid, max, now);
            for (k, mut v) in vars_clone {
                v.source = EnvSource::Inherited;
                v.access_count = 0;
                child.vars.insert(k, v);
            }
            self.blocks.insert(child_pid, child);
            true
        } else { false }
    }

    #[inline(always)]
    pub fn block_var(&mut self, key: String) {
        if !self.blocked_vars.contains(&key) { self.blocked_vars.push(key); }
    }

    fn record_event(&mut self, event: EnvChangeEvent) {
        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push_back(event);
    }

    #[inline]
    pub fn stats(&self) -> EnvMgrStats {
        let total_vars: u64 = self.blocks.values().map(|b| b.count() as u64).sum();
        let total_size: u64 = self.blocks.values().map(|b| b.current_size() as u64).sum();
        EnvMgrStats {
            tracked_processes: self.blocks.len() as u32,
            total_vars, total_accesses: self.total_accesses,
            total_modifications: self.total_modifications,
            total_size_bytes: total_size,
        }
    }
}
