// SPDX-License-Identifier: GPL-2.0
//! Apps prlimit_app — process resource limits management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlimitResource {
    Cpu,       // CPU time
    Fsize,     // File size
    Data,      // Data segment
    Stack,     // Stack
    Core,      // Core file
    Rss,       // RSS
    Nproc,     // Processes
    Nofile,    // Open files
    Memlock,   // Locked memory
    As,        // Address space
    Locks,     // File locks
    Sigpending, // Pending signals
    Msgqueue,  // Message queue bytes
    Nice,      // Nice priority
    Rtprio,    // RT priority
    Rttime,    // RT CPU time
}

/// Resource limit
#[derive(Debug, Clone, Copy)]
pub struct Rlimit {
    pub soft: u64,
    pub hard: u64,
}

impl Rlimit {
    pub fn new(soft: u64, hard: u64) -> Self { Self { soft, hard } }
    pub fn unlimited() -> Self { Self { soft: u64::MAX, hard: u64::MAX } }
    pub fn is_unlimited(&self) -> bool { self.hard == u64::MAX }
}

/// Process limits
#[derive(Debug)]
pub struct ProcessLimits {
    pub pid: u64,
    pub limits: BTreeMap<u8, Rlimit>,
    pub change_count: u64,
}

impl ProcessLimits {
    pub fn new(pid: u64) -> Self {
        let mut limits = BTreeMap::new();
        for i in 0..16u8 { limits.insert(i, Rlimit::unlimited()); }
        Self { pid, limits, change_count: 0 }
    }

    pub fn get(&self, resource: RlimitResource) -> Rlimit {
        self.limits.get(&(resource as u8)).copied().unwrap_or(Rlimit::unlimited())
    }

    pub fn set(&mut self, resource: RlimitResource, limit: Rlimit) -> bool {
        if limit.soft > limit.hard { return false; }
        if let Some(cur) = self.limits.get(&(resource as u8)) {
            if limit.hard > cur.hard { return false; }
        }
        self.limits.insert(resource as u8, limit);
        self.change_count += 1;
        true
    }

    pub fn check(&self, resource: RlimitResource, usage: u64) -> bool {
        let l = self.get(resource);
        usage <= l.soft
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PrlimitAppStats {
    pub tracked_processes: u32,
    pub total_changes: u64,
    pub unlimited_limits: u32,
}

/// Main prlimit app
pub struct AppPrlimit {
    processes: BTreeMap<u64, ProcessLimits>,
}

impl AppPrlimit {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessLimits::new(pid)); }

    pub fn set_limit(&mut self, pid: u64, resource: RlimitResource, limit: Rlimit) -> bool {
        self.processes.get_mut(&pid).map_or(false, |p| p.set(resource, limit))
    }

    pub fn get_limit(&self, pid: u64, resource: RlimitResource) -> Option<Rlimit> {
        Some(self.processes.get(&pid)?.get(resource))
    }

    pub fn stats(&self) -> PrlimitAppStats {
        let changes: u64 = self.processes.values().map(|p| p.change_count).sum();
        let unlimited: u32 = self.processes.values().map(|p| p.limits.values().filter(|l| l.is_unlimited()).count() as u32).sum();
        PrlimitAppStats { tracked_processes: self.processes.len() as u32, total_changes: changes, unlimited_limits: unlimited }
    }
}
