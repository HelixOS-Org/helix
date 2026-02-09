//! # Bridge Capability Manager
//!
//! Fine-grained capability-based access control for syscall bridge:
//! - Hierarchical capability tokens
//! - Capability derivation and attenuation
//! - Time-bounded capabilities
//! - Revocation propagation
//! - Delegation chains

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CAPABILITY TYPES
// ============================================================================

/// Capability type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute
    Execute,
    /// Create resource
    Create,
    /// Delete resource
    Delete,
    /// Grant to others
    Grant,
    /// Admin operations
    Admin,
}

/// Capability state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityState {
    /// Active and valid
    Active,
    /// Suspended temporarily
    Suspended,
    /// Revoked permanently
    Revoked,
    /// Expired
    Expired,
}

/// Resource type the capability protects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedResource {
    /// File descriptor
    FileDescriptor,
    /// Memory region
    MemoryRegion,
    /// Network socket
    Socket,
    /// Process management
    Process,
    /// Device access
    Device,
    /// IPC channel
    IpcChannel,
    /// Syscall class
    SyscallClass,
}

// ============================================================================
// CAPABILITY TOKEN
// ============================================================================

/// Capability token
#[derive(Debug, Clone)]
pub struct CapabilityToken {
    /// Unique ID
    pub id: u64,
    /// Owner process
    pub owner_pid: u64,
    /// Capability type
    pub cap_type: CapabilityType,
    /// Protected resource
    pub resource: ProtectedResource,
    /// Resource identifier
    pub resource_id: u64,
    /// State
    pub state: CapabilityState,
    /// Creation timestamp
    pub created_ns: u64,
    /// Expiry timestamp (0 = no expiry)
    pub expires_ns: u64,
    /// Parent capability (for derivation chains)
    pub parent_id: Option<u64>,
    /// Derivation depth
    pub depth: u32,
    /// Max derivation depth
    pub max_depth: u32,
    /// Can be delegated
    pub delegatable: bool,
}

impl CapabilityToken {
    pub fn new(
        id: u64,
        owner_pid: u64,
        cap_type: CapabilityType,
        resource: ProtectedResource,
        resource_id: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            owner_pid,
            cap_type,
            resource,
            resource_id,
            state: CapabilityState::Active,
            created_ns: now,
            expires_ns: 0,
            parent_id: None,
            depth: 0,
            max_depth: 5,
            delegatable: true,
        }
    }

    /// Check if active
    #[inline(always)]
    pub fn is_valid(&self, now: u64) -> bool {
        self.state == CapabilityState::Active && (self.expires_ns == 0 || now < self.expires_ns)
    }

    /// Derive attenuated capability
    pub fn derive(&self, new_id: u64, new_owner: u64, now: u64) -> Option<CapabilityToken> {
        if !self.delegatable || self.depth >= self.max_depth {
            return None;
        }
        if !self.is_valid(now) {
            return None;
        }
        Some(CapabilityToken {
            id: new_id,
            owner_pid: new_owner,
            cap_type: self.cap_type,
            resource: self.resource,
            resource_id: self.resource_id,
            state: CapabilityState::Active,
            created_ns: now,
            expires_ns: self.expires_ns, // inherit parent expiry
            parent_id: Some(self.id),
            depth: self.depth + 1,
            max_depth: self.max_depth,
            delegatable: self.depth + 1 < self.max_depth,
        })
    }

    /// Revoke
    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = CapabilityState::Revoked;
    }

    /// Suspend
    #[inline]
    pub fn suspend(&mut self) {
        if self.state == CapabilityState::Active {
            self.state = CapabilityState::Suspended;
        }
    }

    /// Resume
    #[inline]
    pub fn resume(&mut self) {
        if self.state == CapabilityState::Suspended {
            self.state = CapabilityState::Active;
        }
    }

    /// Check expiry
    #[inline]
    pub fn check_expiry(&mut self, now: u64) {
        if self.expires_ns > 0 && now >= self.expires_ns && self.state == CapabilityState::Active {
            self.state = CapabilityState::Expired;
        }
    }
}

// ============================================================================
// CAPABILITY TABLE
// ============================================================================

/// Per-process capability table
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessCapabilityTable {
    /// Process ID
    pub pid: u64,
    /// Capabilities owned
    capabilities: BTreeMap<u64, CapabilityToken>,
    /// Next capability ID
    next_id: u64,
}

impl ProcessCapabilityTable {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            capabilities: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Add capability
    #[inline(always)]
    pub fn add(&mut self, cap: CapabilityToken) {
        self.capabilities.insert(cap.id, cap);
    }

    /// Check permission
    #[inline]
    pub fn has_permission(
        &self,
        resource: ProtectedResource,
        resource_id: u64,
        cap_type: CapabilityType,
        now: u64,
    ) -> bool {
        self.capabilities.values().any(|c| {
            c.resource == resource
                && c.resource_id == resource_id
                && c.cap_type == cap_type
                && c.is_valid(now)
        })
    }

    /// Get capability
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&CapabilityToken> {
        self.capabilities.get(&id)
    }

    /// Get mutable
    #[inline(always)]
    pub fn get_mut(&mut self, id: u64) -> Option<&mut CapabilityToken> {
        self.capabilities.get_mut(&id)
    }

    /// Revoke by ID
    #[inline]
    pub fn revoke(&mut self, id: u64) -> bool {
        if let Some(cap) = self.capabilities.get_mut(&id) {
            cap.revoke();
            true
        } else {
            false
        }
    }

    /// Count active
    #[inline]
    pub fn active_count(&self) -> usize {
        self.capabilities
            .values()
            .filter(|c| c.state == CapabilityState::Active)
            .count()
    }

    /// Cleanup expired/revoked
    #[inline]
    pub fn cleanup(&mut self, now: u64) {
        for cap in self.capabilities.values_mut() {
            cap.check_expiry(now);
        }
        self.capabilities.retain(|_, c| {
            c.state != CapabilityState::Revoked && c.state != CapabilityState::Expired
        });
    }

    /// Allocate next ID
    #[inline]
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Capability manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeCapabilityStats {
    /// Total processes
    pub processes: usize,
    /// Total capabilities
    pub total_capabilities: usize,
    /// Active capabilities
    pub active_capabilities: usize,
    /// Permission checks
    pub permission_checks: u64,
    /// Grants
    pub grants: u64,
    /// Revocations
    pub revocations: u64,
    /// Derivations
    pub derivations: u64,
}

/// Bridge capability manager
#[repr(align(64))]
pub struct BridgeCapabilityManager {
    /// Per-process tables
    tables: BTreeMap<u64, ProcessCapabilityTable>,
    /// Global next ID
    next_global_id: u64,
    /// Stats
    stats: BridgeCapabilityStats,
}

impl BridgeCapabilityManager {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            next_global_id: 1_000_000,
            stats: BridgeCapabilityStats::default(),
        }
    }

    /// Grant capability
    pub fn grant(
        &mut self,
        pid: u64,
        cap_type: CapabilityType,
        resource: ProtectedResource,
        resource_id: u64,
        now: u64,
    ) -> u64 {
        let id = self.alloc_id();
        let cap = CapabilityToken::new(id, pid, cap_type, resource, resource_id, now);
        let table = self
            .tables
            .entry(pid)
            .or_insert_with(|| ProcessCapabilityTable::new(pid));
        table.add(cap);
        self.stats.grants += 1;
        self.update_stats();
        id
    }

    /// Grant with expiry
    pub fn grant_timed(
        &mut self,
        pid: u64,
        cap_type: CapabilityType,
        resource: ProtectedResource,
        resource_id: u64,
        now: u64,
        duration_ns: u64,
    ) -> u64 {
        let id = self.alloc_id();
        let mut cap = CapabilityToken::new(id, pid, cap_type, resource, resource_id, now);
        cap.expires_ns = now + duration_ns;
        let table = self
            .tables
            .entry(pid)
            .or_insert_with(|| ProcessCapabilityTable::new(pid));
        table.add(cap);
        self.stats.grants += 1;
        self.update_stats();
        id
    }

    /// Check permission
    #[inline]
    pub fn check(
        &mut self,
        pid: u64,
        resource: ProtectedResource,
        resource_id: u64,
        cap_type: CapabilityType,
        now: u64,
    ) -> bool {
        self.stats.permission_checks += 1;
        self.tables
            .get(&pid)
            .map(|t| t.has_permission(resource, resource_id, cap_type, now))
            .unwrap_or(false)
    }

    /// Delegate capability
    pub fn delegate(&mut self, from_pid: u64, cap_id: u64, to_pid: u64, now: u64) -> Option<u64> {
        let parent = self.tables.get(&from_pid)?.get(cap_id)?.clone();
        let new_id = self.alloc_id();
        let derived = parent.derive(new_id, to_pid, now)?;
        let table = self
            .tables
            .entry(to_pid)
            .or_insert_with(|| ProcessCapabilityTable::new(to_pid));
        table.add(derived);
        self.stats.derivations += 1;
        self.update_stats();
        Some(new_id)
    }

    /// Revoke capability and all derivatives
    pub fn revoke_cascade(&mut self, pid: u64, cap_id: u64) {
        if let Some(table) = self.tables.get_mut(&pid) {
            table.revoke(cap_id);
        }
        // Cascade: revoke all children
        let mut to_revoke = Vec::new();
        for (other_pid, table) in &self.tables {
            for (id, cap) in &table.capabilities {
                if cap.parent_id == Some(cap_id) {
                    to_revoke.push((*other_pid, *id));
                }
            }
        }
        for (rpid, rid) in to_revoke {
            self.revoke_cascade(rpid, rid);
        }
        self.stats.revocations += 1;
        self.update_stats();
    }

    /// Cleanup expired
    #[inline]
    pub fn cleanup(&mut self, now: u64) {
        for table in self.tables.values_mut() {
            table.cleanup(now);
        }
        self.tables.retain(|_, t| !t.capabilities.is_empty());
        self.update_stats();
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_global_id;
        self.next_global_id += 1;
        id
    }

    fn update_stats(&mut self) {
        self.stats.processes = self.tables.len();
        self.stats.total_capabilities = self.tables.values().map(|t| t.capabilities.len()).sum();
        self.stats.active_capabilities = self.tables.values().map(|t| t.active_count()).sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeCapabilityStats {
        &self.stats
    }
}
