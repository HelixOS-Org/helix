//! # Resource Broker
//!
//! Manages system resources and their allocation across subsystems.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::RwLock;

use crate::{KernelError, KernelResult};

/// Resource identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(u64);

impl ResourceId {
    /// Create a new resource ID
    fn _new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Resource class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceClass {
    /// CPU time
    CpuTime,
    /// Physical memory
    PhysicalMemory,
    /// Virtual address space
    VirtualAddressSpace,
    /// File descriptors
    FileDescriptors,
    /// Network sockets
    NetworkSockets,
    /// I/O bandwidth
    IoBandwidth,
    /// IRQ lines
    Irq,
    /// Custom resource
    Custom(u32),
}

/// Resource limits
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimit {
    /// Current usage
    pub current: u64,
    /// Soft limit
    pub soft: u64,
    /// Hard limit
    pub hard: u64,
}

impl ResourceLimit {
    /// Check if the current usage is at or over the soft limit
    pub fn at_soft_limit(&self) -> bool {
        self.current >= self.soft
    }

    /// Check if the current usage is at or over the hard limit
    pub fn at_hard_limit(&self) -> bool {
        self.current >= self.hard
    }

    /// Try to allocate resources
    pub fn try_allocate(&mut self, amount: u64) -> bool {
        if self.current + amount > self.hard {
            false
        } else {
            self.current += amount;
            true
        }
    }

    /// Release resources
    pub fn release(&mut self, amount: u64) {
        self.current = self.current.saturating_sub(amount);
    }
}

impl Default for ResourceLimit {
    fn default() -> Self {
        Self {
            current: 0,
            soft: u64::MAX,
            hard: u64::MAX,
        }
    }
}

/// Resource quota for an entity
#[derive(Debug, Clone)]
pub struct ResourceQuota {
    /// Entity ID (process, container, etc.)
    _entity_id: u64,
    /// Limits by resource class
    limits: BTreeMap<ResourceClass, ResourceLimit>,
}

impl ResourceQuota {
    /// Create a new quota
    pub fn new(entity_id: u64) -> Self {
        Self {
            _entity_id: entity_id,
            limits: BTreeMap::new(),
        }
    }

    /// Set a limit for a resource class
    pub fn set_limit(&mut self, class: ResourceClass, soft: u64, hard: u64) {
        self.limits.insert(class, ResourceLimit {
            current: 0,
            soft,
            hard,
        });
    }

    /// Get the limit for a resource class
    pub fn get_limit(&self, class: ResourceClass) -> Option<&ResourceLimit> {
        self.limits.get(&class)
    }

    /// Get the limit for a resource class (mutable)
    pub fn get_limit_mut(&mut self, class: ResourceClass) -> Option<&mut ResourceLimit> {
        self.limits.get_mut(&class)
    }
}

/// The resource broker
pub struct ResourceBroker {
    /// Quotas by entity
    quotas: RwLock<BTreeMap<u64, ResourceQuota>>,

    /// Global resource limits
    global_limits: RwLock<BTreeMap<ResourceClass, ResourceLimit>>,

    /// Resource providers
    providers: RwLock<BTreeMap<ResourceClass, Arc<dyn ResourceProvider>>>,
}

impl Default for ResourceBroker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceBroker {
    /// Create a new resource broker
    pub const fn new() -> Self {
        Self {
            quotas: RwLock::new(BTreeMap::new()),
            global_limits: RwLock::new(BTreeMap::new()),
            providers: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a resource provider
    pub fn register_provider(&self, class: ResourceClass, provider: Arc<dyn ResourceProvider>) {
        self.providers.write().insert(class, provider);
    }

    /// Set global limits for a resource class
    pub fn set_global_limit(&self, class: ResourceClass, soft: u64, hard: u64) {
        self.global_limits.write().insert(class, ResourceLimit {
            current: 0,
            soft,
            hard,
        });
    }

    /// Create a quota for an entity
    pub fn create_quota(&self, entity_id: u64) -> KernelResult<()> {
        let mut quotas = self.quotas.write();
        if quotas.contains_key(&entity_id) {
            return Err(KernelError::AlreadyExists);
        }
        quotas.insert(entity_id, ResourceQuota::new(entity_id));
        Ok(())
    }

    /// Set quota limits for an entity
    pub fn set_quota_limit(
        &self,
        entity_id: u64,
        class: ResourceClass,
        soft: u64,
        hard: u64,
    ) -> KernelResult<()> {
        let mut quotas = self.quotas.write();
        let quota = quotas.get_mut(&entity_id).ok_or(KernelError::NotFound)?;
        quota.set_limit(class, soft, hard);
        Ok(())
    }

    /// Request resource allocation
    pub fn allocate(&self, entity_id: u64, class: ResourceClass, amount: u64) -> KernelResult<()> {
        // Check global limits
        {
            let mut global = self.global_limits.write();
            if let Some(limit) = global.get_mut(&class) {
                if !limit.try_allocate(amount) {
                    return Err(KernelError::OutOfMemory);
                }
            }
        }

        // Check entity quota
        {
            let mut quotas = self.quotas.write();
            if let Some(quota) = quotas.get_mut(&entity_id) {
                if let Some(limit) = quota.get_limit_mut(class) {
                    if !limit.try_allocate(amount) {
                        // Rollback global allocation
                        #[allow(clippy::excessive_nesting)]
                        if let Some(limit) = self.global_limits.write().get_mut(&class) {
                            limit.release(amount);
                        }
                        return Err(KernelError::OutOfMemory);
                    }
                }
            }
        }

        Ok(())
    }

    /// Release allocated resources
    pub fn release(&self, entity_id: u64, class: ResourceClass, amount: u64) {
        // Release from entity quota
        if let Some(quota) = self.quotas.write().get_mut(&entity_id) {
            if let Some(limit) = quota.get_limit_mut(class) {
                limit.release(amount);
            }
        }

        // Release from global limits
        if let Some(limit) = self.global_limits.write().get_mut(&class) {
            limit.release(amount);
        }
    }

    /// Get resource usage for an entity
    pub fn get_usage(&self, entity_id: u64, class: ResourceClass) -> Option<ResourceLimit> {
        self.quotas
            .read()
            .get(&entity_id)
            .and_then(|q| q.get_limit(class).copied())
    }

    /// Get global resource usage
    pub fn get_global_usage(&self, class: ResourceClass) -> Option<ResourceLimit> {
        self.global_limits.read().get(&class).copied()
    }
}

/// Resource provider trait
pub trait ResourceProvider: Send + Sync {
    /// Get the total available resources
    fn total(&self) -> u64;

    /// Get currently used resources
    fn used(&self) -> u64;

    /// Get available resources
    fn available(&self) -> u64 {
        self.total().saturating_sub(self.used())
    }

    /// Try to reserve resources
    fn reserve(&self, amount: u64) -> KernelResult<()>;

    /// Release reserved resources
    fn release(&self, amount: u64);
}
