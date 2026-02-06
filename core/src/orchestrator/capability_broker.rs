//! # Capability Broker
//!
//! The capability broker is responsible for distributing, validating,
//! and managing capabilities throughout the system.
//!
//! Capabilities are unforgeable tokens that grant specific rights to
//! resources or operations.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use spin::RwLock;

use crate::{CapabilityError, KernelError, KernelResult};

/// Unique capability identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(u64);

impl CapabilityId {
    /// Create a new capability ID
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

bitflags! {
    /// Rights that can be granted by a capability
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CapabilityRights: u64 {
        /// Read access
        const READ = 1 << 0;
        /// Write access
        const WRITE = 1 << 1;
        /// Execute access
        const EXECUTE = 1 << 2;
        /// Delete the resource
        const DELETE = 1 << 3;
        /// Duplicate the capability
        const DUPLICATE = 1 << 4;
        /// Transfer the capability
        const TRANSFER = 1 << 5;
        /// Grant capabilities to others
        const GRANT = 1 << 6;
        /// Revoke granted capabilities
        const REVOKE = 1 << 7;
        /// Manage the resource
        const MANAGE = 1 << 8;
        /// Full rights
        const ALL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits()
                  | Self::DELETE.bits() | Self::DUPLICATE.bits() | Self::TRANSFER.bits()
                  | Self::GRANT.bits() | Self::REVOKE.bits() | Self::MANAGE.bits();
    }
}

/// Resource type for capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceType {
    /// Memory region
    Memory,
    /// Process
    Process,
    /// Thread
    Thread,
    /// I/O port
    IoPort,
    /// Interrupt
    Interrupt,
    /// File descriptor
    FileDescriptor,
    /// IPC channel
    IpcChannel,
    /// Device
    Device,
    /// Module
    Module,
    /// Custom resource
    Custom(u32),
}

/// A capability token
#[derive(Debug, Clone)]
pub struct Capability {
    /// Unique identifier
    id: CapabilityId,
    /// Resource type
    resource_type: ResourceType,
    /// Resource identifier (type-specific)
    resource_id: u64,
    /// Granted rights
    rights: CapabilityRights,
    /// Parent capability (if derived)
    parent: Option<CapabilityId>,
    /// Owner (holder of this capability)
    owner: u64,
    /// Creation timestamp
    _created_at: u64,
    /// Expiration timestamp (0 = never)
    expires_at: u64,
    /// Is this capability revoked?
    revoked: bool,
}

impl Capability {
    /// Get the capability ID
    pub fn id(&self) -> CapabilityId {
        self.id
    }

    /// Get the resource type
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Get the resource ID
    pub fn resource_id(&self) -> u64 {
        self.resource_id
    }

    /// Get the rights
    pub fn rights(&self) -> CapabilityRights {
        self.rights
    }

    /// Check if capability has specific rights
    pub fn has_rights(&self, rights: CapabilityRights) -> bool {
        self.rights.contains(rights)
    }

    /// Check if capability is valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        !self.revoked && (self.expires_at == 0 || self.expires_at > current_time)
    }
}

/// The capability broker
pub struct CapabilityBroker {
    /// All capabilities in the system
    capabilities: RwLock<BTreeMap<CapabilityId, Capability>>,

    /// Capabilities by owner
    by_owner: RwLock<BTreeMap<u64, Vec<CapabilityId>>>,

    /// Capabilities by resource
    by_resource: RwLock<BTreeMap<(ResourceType, u64), Vec<CapabilityId>>>,
}

impl Default for CapabilityBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityBroker {
    /// Create a new capability broker
    pub const fn new() -> Self {
        Self {
            capabilities: RwLock::new(BTreeMap::new()),
            by_owner: RwLock::new(BTreeMap::new()),
            by_resource: RwLock::new(BTreeMap::new()),
        }
    }

    /// Create a new capability for a resource
    pub fn create(
        &self,
        owner: u64,
        resource_type: ResourceType,
        resource_id: u64,
        rights: CapabilityRights,
    ) -> KernelResult<CapabilityId> {
        let cap = Capability {
            id: CapabilityId::new(),
            resource_type,
            resource_id,
            rights,
            parent: None,
            owner,
            _created_at: 0, // TODO: get current time
            expires_at: 0,
            revoked: false,
        };

        let id = cap.id;

        self.capabilities.write().insert(id, cap);
        self.by_owner.write().entry(owner).or_default().push(id);
        self.by_resource
            .write()
            .entry((resource_type, resource_id))
            .or_default()
            .push(id);

        Ok(id)
    }

    /// Derive a new capability from an existing one with reduced rights
    pub fn derive(
        &self,
        parent_id: CapabilityId,
        new_owner: u64,
        rights: CapabilityRights,
    ) -> KernelResult<CapabilityId> {
        let parent = self
            .capabilities
            .read()
            .get(&parent_id)
            .ok_or(KernelError::CapabilityError(CapabilityError::NotFound))?
            .clone();

        // Can only derive with equal or lesser rights
        if !parent.rights.contains(rights) {
            return Err(KernelError::CapabilityError(
                CapabilityError::InsufficientRights,
            ));
        }

        // Parent must have GRANT right
        if !parent.rights.contains(CapabilityRights::GRANT) {
            return Err(KernelError::CapabilityError(
                CapabilityError::InsufficientRights,
            ));
        }

        let cap = Capability {
            id: CapabilityId::new(),
            resource_type: parent.resource_type,
            resource_id: parent.resource_id,
            rights,
            parent: Some(parent_id),
            owner: new_owner,
            _created_at: 0,
            expires_at: parent.expires_at,
            revoked: false,
        };

        let id = cap.id;

        self.capabilities.write().insert(id, cap);
        self.by_owner.write().entry(new_owner).or_default().push(id);

        Ok(id)
    }

    /// Validate a capability
    pub fn validate(
        &self,
        id: CapabilityId,
        required_rights: CapabilityRights,
    ) -> KernelResult<()> {
        let caps = self.capabilities.read();
        let cap = caps
            .get(&id)
            .ok_or(KernelError::CapabilityError(CapabilityError::NotFound))?;

        if cap.revoked {
            return Err(KernelError::CapabilityError(CapabilityError::Revoked));
        }

        if !cap.rights.contains(required_rights) {
            return Err(KernelError::CapabilityError(
                CapabilityError::InsufficientRights,
            ));
        }

        Ok(())
    }

    /// Revoke a capability and all its derivatives
    pub fn revoke(&self, id: CapabilityId) -> KernelResult<()> {
        let mut caps = self.capabilities.write();

        let cap = caps
            .get_mut(&id)
            .ok_or(KernelError::CapabilityError(CapabilityError::NotFound))?;

        cap.revoked = true;

        // Revoke all derivatives
        let derivatives: Vec<CapabilityId> = caps
            .iter()
            .filter(|(_, c)| c.parent == Some(id))
            .map(|(id, _)| *id)
            .collect();

        drop(caps);

        for deriv_id in derivatives {
            let _ = self.revoke(deriv_id);
        }

        Ok(())
    }

    /// Get all capabilities for an owner
    pub fn get_by_owner(&self, owner: u64) -> Vec<CapabilityId> {
        self.by_owner
            .read()
            .get(&owner)
            .cloned()
            .unwrap_or_default()
    }

    /// Get capability details
    pub fn get(&self, id: CapabilityId) -> Option<Capability> {
        self.capabilities.read().get(&id).cloned()
    }

    /// Transfer a capability to a new owner
    pub fn transfer(&self, id: CapabilityId, new_owner: u64) -> KernelResult<()> {
        let mut caps = self.capabilities.write();
        let cap = caps
            .get_mut(&id)
            .ok_or(KernelError::CapabilityError(CapabilityError::NotFound))?;

        if !cap.rights.contains(CapabilityRights::TRANSFER) {
            return Err(KernelError::CapabilityError(
                CapabilityError::InsufficientRights,
            ));
        }

        let old_owner = cap.owner;
        cap.owner = new_owner;

        drop(caps);

        // Update ownership indices
        let mut by_owner = self.by_owner.write();
        if let Some(old_caps) = by_owner.get_mut(&old_owner) {
            old_caps.retain(|&c| c != id);
        }
        by_owner.entry(new_owner).or_default().push(id);

        Ok(())
    }
}
