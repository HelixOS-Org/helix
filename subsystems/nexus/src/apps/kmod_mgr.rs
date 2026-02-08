// SPDX-License-Identifier: GPL-2.0
//! Apps kmod_mgr — kernel module dependency and lifecycle tracking per application.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmodState {
    Unloaded,
    Loading,
    Live,
    Unloading,
    Failed,
    Blocked,
}

/// Module type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmodType {
    Builtin,
    Loadable,
    Platform,
    Firmware,
    Crypto,
    Filesystem,
    Network,
    Block,
    Char,
    Usb,
}

/// Module dependency edge
#[derive(Debug, Clone)]
pub struct KmodDep {
    pub from: u64,
    pub to: u64,
    pub soft: bool,
}

/// Module reference count tracker
#[derive(Debug, Clone)]
pub struct KmodRefcount {
    pub holder_pid: u64,
    pub count: u32,
    pub timestamp_ns: u64,
}

/// A kernel module descriptor
#[derive(Debug, Clone)]
pub struct KmodInfo {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub mod_type: KmodType,
    pub state: KmodState,
    pub base_addr: u64,
    pub size_bytes: u64,
    pub init_size: u64,
    pub core_size: u64,
    pub load_timestamp_ns: u64,
    pub taints: u32,
    pub refcount: u32,
    pub param_count: u32,
    deps: Vec<KmodDep>,
    holders: Vec<KmodRefcount>,
    load_count: u64,
    fault_count: u64,
}

impl KmodInfo {
    pub fn new(id: u64, name: String, mod_type: KmodType) -> Self {
        Self {
            id,
            name,
            version: String::new(),
            mod_type,
            state: KmodState::Unloaded,
            base_addr: 0,
            size_bytes: 0,
            init_size: 0,
            core_size: 0,
            load_timestamp_ns: 0,
            taints: 0,
            refcount: 0,
            param_count: 0,
            deps: Vec::new(),
            holders: Vec::new(),
            load_count: 0,
            fault_count: 0,
        }
    }

    pub fn add_dependency(&mut self, dep: KmodDep) {
        self.deps.push(dep);
    }

    pub fn dep_count(&self) -> usize {
        self.deps.len()
    }

    pub fn hard_dep_count(&self) -> usize {
        self.deps.iter().filter(|d| !d.soft).count()
    }

    pub fn add_holder(&mut self, pid: u64, timestamp_ns: u64) {
        self.refcount += 1;
        if let Some(h) = self.holders.iter_mut().find(|h| h.holder_pid == pid) {
            h.count += 1;
            h.timestamp_ns = timestamp_ns;
        } else {
            self.holders.push(KmodRefcount {
                holder_pid: pid,
                count: 1,
                timestamp_ns,
            });
        }
    }

    pub fn release_holder(&mut self, pid: u64) -> bool {
        if let Some(pos) = self.holders.iter().position(|h| h.holder_pid == pid) {
            self.holders[pos].count = self.holders[pos].count.saturating_sub(1);
            if self.holders[pos].count == 0 {
                self.holders.swap_remove(pos);
            }
            self.refcount = self.refcount.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn can_unload(&self) -> bool {
        self.refcount == 0 && self.state == KmodState::Live
    }

    pub fn is_tainted(&self) -> bool {
        self.taints != 0
    }

    pub fn memory_total(&self) -> u64 {
        if self.size_bytes > 0 {
            self.size_bytes
        } else {
            self.init_size + self.core_size
        }
    }

    pub fn fault_rate_per_load(&self) -> f64 {
        if self.load_count == 0 { return 0.0; }
        self.fault_count as f64 / self.load_count as f64
    }
}

/// Per-app module usage tracking
#[derive(Debug)]
pub struct AppKmodUsage {
    pub pid: u64,
    pub requested_modules: Vec<u64>,
    pub denied_modules: Vec<u64>,
    pub auto_loaded: u32,
}

impl AppKmodUsage {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            requested_modules: Vec::new(),
            denied_modules: Vec::new(),
            auto_loaded: 0,
        }
    }

    pub fn request_module(&mut self, mod_id: u64, granted: bool) {
        if granted {
            if !self.requested_modules.contains(&mod_id) {
                self.requested_modules.push(mod_id);
            }
        } else {
            if !self.denied_modules.contains(&mod_id) {
                self.denied_modules.push(mod_id);
            }
        }
    }

    pub fn denial_rate(&self) -> f64 {
        let total = self.requested_modules.len() + self.denied_modules.len();
        if total == 0 { return 0.0; }
        self.denied_modules.len() as f64 / total as f64
    }
}

/// Kmod manager stats
#[derive(Debug, Clone)]
pub struct KmodMgrStats {
    pub total_modules: u64,
    pub live_modules: u64,
    pub total_loads: u64,
    pub total_unloads: u64,
    pub load_failures: u64,
    pub dep_cycles_detected: u64,
    pub total_memory_bytes: u64,
    pub tainted_count: u64,
}

/// Main kmod manager
pub struct AppKmodMgr {
    modules: BTreeMap<u64, KmodInfo>,
    name_to_id: BTreeMap<String, u64>,
    app_usage: BTreeMap<u64, AppKmodUsage>,
    next_id: u64,
    stats: KmodMgrStats,
}

impl AppKmodMgr {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            app_usage: BTreeMap::new(),
            next_id: 1,
            stats: KmodMgrStats {
                total_modules: 0,
                live_modules: 0,
                total_loads: 0,
                total_unloads: 0,
                load_failures: 0,
                dep_cycles_detected: 0,
                total_memory_bytes: 0,
                tainted_count: 0,
            },
        }
    }

    pub fn register_module(&mut self, name: String, mod_type: KmodType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.name_to_id.insert(name.clone(), id);
        self.modules.insert(id, KmodInfo::new(id, name, mod_type));
        self.stats.total_modules += 1;
        id
    }

    pub fn add_dependency(&mut self, from: u64, to: u64, soft: bool) -> bool {
        // Check for cycle: simple DFS
        if self.has_path(to, from) {
            self.stats.dep_cycles_detected += 1;
            return false;
        }
        if let Some(m) = self.modules.get_mut(&from) {
            m.add_dependency(KmodDep { from, to, soft });
            true
        } else {
            false
        }
    }

    fn has_path(&self, from: u64, to: u64) -> bool {
        if from == to { return true; }
        let mut visited = Vec::new();
        let mut stack = alloc::vec![from];
        while let Some(current) = stack.pop() {
            if current == to { return true; }
            if visited.contains(&current) { continue; }
            visited.push(current);
            if let Some(m) = self.modules.get(&current) {
                for dep in &m.deps {
                    stack.push(dep.to);
                }
            }
        }
        false
    }

    pub fn load_module(&mut self, id: u64, base_addr: u64, size: u64, timestamp_ns: u64) -> bool {
        if let Some(m) = self.modules.get_mut(&id) {
            // Check all hard deps are live
            let hard_deps: Vec<u64> = m.deps.iter().filter(|d| !d.soft).map(|d| d.to).collect();
            for dep_id in &hard_deps {
                if let Some(dep) = self.modules.get(dep_id) {
                    if dep.state != KmodState::Live {
                        self.stats.load_failures += 1;
                        return false;
                    }
                }
            }
            let m = self.modules.get_mut(&id).unwrap();
            m.state = KmodState::Live;
            m.base_addr = base_addr;
            m.size_bytes = size;
            m.load_timestamp_ns = timestamp_ns;
            m.load_count += 1;
            self.stats.total_loads += 1;
            self.stats.live_modules += 1;
            self.stats.total_memory_bytes += size;
            true
        } else {
            false
        }
    }

    pub fn unload_module(&mut self, id: u64) -> bool {
        if let Some(m) = self.modules.get_mut(&id) {
            if !m.can_unload() { return false; }
            self.stats.total_memory_bytes = self.stats.total_memory_bytes.saturating_sub(m.size_bytes);
            m.state = KmodState::Unloaded;
            m.base_addr = 0;
            self.stats.total_unloads += 1;
            self.stats.live_modules = self.stats.live_modules.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn request_module_for_app(&mut self, pid: u64, mod_name: &str, granted: bool) {
        let usage = self.app_usage.entry(pid).or_insert_with(|| AppKmodUsage::new(pid));
        if let Some(&id) = self.name_to_id.get(mod_name) {
            usage.request_module(id, granted);
        }
    }

    pub fn record_fault(&mut self, id: u64) {
        if let Some(m) = self.modules.get_mut(&id) {
            m.fault_count += 1;
        }
    }

    pub fn modules_by_memory(&self) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.modules.iter()
            .filter(|(_, m)| m.state == KmodState::Live)
            .map(|(&id, m)| (id, m.memory_total()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    pub fn faultiest_modules(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.modules.iter()
            .filter(|(_, m)| m.load_count > 0)
            .map(|(&id, m)| (id, m.fault_rate_per_load()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_module(&self, id: u64) -> Option<&KmodInfo> {
        self.modules.get(&id)
    }

    pub fn stats(&self) -> &KmodMgrStats {
        &self.stats
    }
}
