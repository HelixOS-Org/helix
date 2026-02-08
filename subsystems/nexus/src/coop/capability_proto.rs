//! # Coop Capability Protocol
//!
//! Capability-based cooperative access control:
//! - Fine-grained capability tokens
//! - Delegation chains with attenuation
//! - Revocation propagation
//! - Capability memorization (caching)
//! - Cross-process capability transfer
//! - Confinement enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Capability right
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapRight {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
    /// Grant (delegate) access
    Grant,
    /// Revoke access
    Revoke,
    /// Inspect metadata
    Inspect,
    /// Create child objects
    Create,
    /// Destroy objects
    Destroy,
}

/// Object type for capability targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapObjectType {
    /// Memory region
    Memory,
    /// File descriptor
    File,
    /// Network socket
    Socket,
    /// IPC channel
    IpcChannel,
    /// Device
    Device,
    /// Process
    Process,
    /// Thread
    Thread,
    /// Semaphore
    Semaphore,
}

/// A capability token
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    pub cap_id: u64,
    pub object_type: CapObjectType,
    pub object_id: u64,
    /// Bitmask of CapRight
    pub rights: u32,
    pub owner_pid: u64,
    pub created_ns: u64,
    pub expiry_ns: u64,
    /// Parent capability (for delegation chain)
    pub parent_cap: Option<u64>,
    /// Generation (increment on revoke to invalidate children)
    pub generation: u32,
    /// How many times this can be further delegated
    pub delegation_depth: u8,
    pub revoked: bool,
}

impl CapabilityToken {
    pub fn new(
        cap_id: u64,
        object_type: CapObjectType,
        object_id: u64,
        rights: u32,
        owner_pid: u64,
        now_ns: u64,
    ) -> Self {
        Self {
            cap_id,
            object_type,
            object_id,
            rights,
            owner_pid,
            created_ns: now_ns,
            expiry_ns: now_ns + 3600_000_000_000, // 1h default
            parent_cap: None,
            generation: 0,
            delegation_depth: 3,
            revoked: false,
        }
    }

    /// Check if this capability grants a specific right
    pub fn has_right(&self, right: CapRight) -> bool {
        self.rights & (1 << (right as u32)) != 0
    }

    /// Is this capability still valid?
    pub fn is_valid(&self, now_ns: u64) -> bool {
        !self.revoked && now_ns < self.expiry_ns
    }

    /// Create an attenuated delegation
    pub fn delegate(&self, new_owner: u64, attenuated_rights: u32, now_ns: u64) -> Option<CapabilityToken> {
        if self.delegation_depth == 0 || self.revoked {
            return None;
        }
        if !self.has_right(CapRight::Grant) {
            return None;
        }
        // Attenuate: new rights must be subset
        let effective_rights = attenuated_rights & self.rights;

        // Generate cap_id via FNV-1a
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.cap_id;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= new_owner;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= now_ns;
        hash = hash.wrapping_mul(0x100000001b3);

        Some(CapabilityToken {
            cap_id: hash,
            object_type: self.object_type,
            object_id: self.object_id,
            rights: effective_rights,
            owner_pid: new_owner,
            created_ns: now_ns,
            expiry_ns: self.expiry_ns, // inherit parent expiry
            parent_cap: Some(self.cap_id),
            generation: self.generation,
            delegation_depth: self.delegation_depth - 1,
            revoked: false,
        })
    }

    /// Rights as bitmask value
    pub fn rights_count(&self) -> u32 {
        let mut r = self.rights;
        let mut count = 0;
        while r > 0 {
            count += r & 1;
            r >>= 1;
        }
        count
    }
}

/// Per-process capability table
#[derive(Debug)]
pub struct ProcessCapTable {
    pub pid: u64,
    caps: BTreeMap<u64, CapabilityToken>,
    pub grants_given: u64,
    pub grants_received: u64,
    pub revocations: u64,
    pub access_checks: u64,
    pub access_denials: u64,
}

impl ProcessCapTable {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            caps: BTreeMap::new(),
            grants_given: 0,
            grants_received: 0,
            revocations: 0,
            access_checks: 0,
            access_denials: 0,
        }
    }

    pub fn insert(&mut self, cap: CapabilityToken) {
        self.caps.insert(cap.cap_id, cap);
        self.grants_received += 1;
    }

    pub fn check_access(&mut self, object_type: CapObjectType, object_id: u64, right: CapRight, now_ns: u64) -> bool {
        self.access_checks += 1;
        let has = self.caps.values().any(|c| {
            c.object_type == object_type
                && c.object_id == object_id
                && c.has_right(right)
                && c.is_valid(now_ns)
        });
        if !has {
            self.access_denials += 1;
        }
        has
    }

    pub fn revoke(&mut self, cap_id: u64) -> bool {
        if let Some(cap) = self.caps.get_mut(&cap_id) {
            cap.revoked = true;
            self.revocations += 1;
            true
        } else {
            false
        }
    }

    pub fn cleanup_expired(&mut self, now_ns: u64) {
        self.caps.retain(|_, cap| cap.is_valid(now_ns));
    }

    pub fn cap_count(&self) -> usize {
        self.caps.len()
    }

    pub fn denial_rate(&self) -> f64 {
        if self.access_checks == 0 { 0.0 } else {
            self.access_denials as f64 / self.access_checks as f64
        }
    }
}

/// Capability protocol stats
#[derive(Debug, Clone, Default)]
pub struct CoopCapProtocolStats {
    pub tracked_processes: usize,
    pub total_capabilities: usize,
    pub total_delegations: u64,
    pub total_revocations: u64,
    pub avg_denial_rate: f64,
}

/// Coop Capability Protocol
pub struct CoopCapProtocol {
    processes: BTreeMap<u64, ProcessCapTable>,
    stats: CoopCapProtocolStats,
    next_cap_id: u64,
}

impl CoopCapProtocol {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: CoopCapProtocolStats::default(),
            next_cap_id: 1,
        }
    }

    pub fn register(&mut self, pid: u64) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessCapTable::new(pid));
    }

    /// Grant a new root capability
    pub fn grant_root(&mut self, pid: u64, obj_type: CapObjectType, obj_id: u64, rights: u32, now_ns: u64) -> u64 {
        let cap_id = self.next_cap_id;
        self.next_cap_id += 1;
        let cap = CapabilityToken::new(cap_id, obj_type, obj_id, rights, pid, now_ns);
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.insert(cap);
        }
        cap_id
    }

    /// Delegate capability from one process to another
    pub fn delegate(
        &mut self,
        from_pid: u64,
        to_pid: u64,
        cap_id: u64,
        attenuated_rights: u32,
        now_ns: u64,
    ) -> Option<u64> {
        let delegated = {
            let from_proc = self.processes.get(&from_pid)?;
            let cap = from_proc.caps.get(&cap_id)?;
            cap.delegate(to_pid, attenuated_rights, now_ns)
        };
        if let Some(new_cap) = delegated {
            let new_id = new_cap.cap_id;
            if let Some(to_proc) = self.processes.get_mut(&to_pid) {
                to_proc.insert(new_cap);
            }
            if let Some(from_proc) = self.processes.get_mut(&from_pid) {
                from_proc.grants_given += 1;
            }
            self.update_stats();
            Some(new_id)
        } else {
            None
        }
    }

    /// Revoke and propagate
    pub fn revoke_cascade(&mut self, cap_id: u64) {
        // Revoke in all processes
        for proc in self.processes.values_mut() {
            proc.revoke(cap_id);
            // Also revoke children
            let children: Vec<u64> = proc.caps.iter()
                .filter(|(_, c)| c.parent_cap == Some(cap_id))
                .map(|(&id, _)| id)
                .collect();
            for child_id in children {
                proc.revoke(child_id);
            }
        }
        self.update_stats();
    }

    pub fn check_access(&mut self, pid: u64, obj_type: CapObjectType, obj_id: u64, right: CapRight, now_ns: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.check_access(obj_type, obj_id, right, now_ns)
        } else {
            false
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_capabilities = self.processes.values()
            .map(|p| p.cap_count()).sum();
        self.stats.total_delegations = self.processes.values()
            .map(|p| p.grants_given).sum();
        self.stats.total_revocations = self.processes.values()
            .map(|p| p.revocations).sum();
        if !self.processes.is_empty() {
            self.stats.avg_denial_rate = self.processes.values()
                .map(|p| p.denial_rate())
                .sum::<f64>() / self.processes.len() as f64;
        }
    }

    pub fn stats(&self) -> &CoopCapProtocolStats {
        &self.stats
    }
}
