//! # Cooperation Registry
//!
//! A capability/service registry for cooperative processes:
//! - Processes register capabilities they offer
//! - Kernel tracks available services
//! - Enables discovery of cooperative partners
//! - Resource sharing coordination
//! - Deduplication of services across processes

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CAPABILITY REGISTRATION
// ============================================================================

/// Category of cooperative capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityCategory {
    /// Memory sharing (can share buffers)
    MemorySharing,
    /// Computation offloading
    Compute,
    /// I/O multiplexing
    IoMultiplex,
    /// Caching services
    Caching,
    /// Network proxying
    NetworkProxy,
    /// File system services
    FileService,
    /// Signal routing
    SignalRouting,
    /// Timer coalescing
    TimerCoalescing,
    /// Lock coordination
    LockCoordination,
    /// Event brokering
    EventBrokering,
    /// Resource pooling
    ResourcePooling,
    /// Checkpoint/restore support
    CheckpointRestore,
    /// Custom capability
    Custom,
}

/// Status of a registered capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// Available for use
    Available,
    /// Temporarily busy
    Busy,
    /// Degraded performance
    Degraded,
    /// Offline/disabled
    Offline,
    /// Being unregistered
    Removing,
}

/// A registered cooperative capability
#[derive(Debug, Clone)]
pub struct RegisteredCapability {
    /// Capability ID
    pub id: u64,
    /// Process providing this capability
    pub provider_pid: u64,
    /// Category
    pub category: CapabilityCategory,
    /// Human-readable name
    pub name: String,
    /// Version
    pub version: u32,
    /// Status
    pub status: CapabilityStatus,
    /// Capacity (0 = unlimited)
    pub capacity: u32,
    /// Current users
    pub current_users: u32,
    /// Registration time
    pub registered_at: u64,
    /// Last health check
    pub last_health_check: u64,
    /// Times used
    pub use_count: u64,
    /// Average response time (microseconds)
    pub avg_response_us: u64,
    /// Priority (higher = preferred)
    pub priority: u8,
}

impl RegisteredCapability {
    pub fn new(
        id: u64,
        provider_pid: u64,
        category: CapabilityCategory,
        name: String,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            provider_pid,
            category,
            name,
            version: 1,
            status: CapabilityStatus::Available,
            capacity: 0,
            current_users: 0,
            registered_at: timestamp,
            last_health_check: timestamp,
            use_count: 0,
            avg_response_us: 0,
            priority: 128,
        }
    }

    /// Is available for use?
    pub fn is_available(&self) -> bool {
        self.status == CapabilityStatus::Available
            && (self.capacity == 0 || self.current_users < self.capacity)
    }

    /// Record a usage
    pub fn record_use(&mut self, response_us: u64) {
        self.use_count += 1;
        // Rolling average
        if self.avg_response_us == 0 {
            self.avg_response_us = response_us;
        } else {
            self.avg_response_us = (self.avg_response_us * 7 + response_us) / 8;
        }
    }

    /// Acquire (increment user count)
    pub fn acquire(&mut self) -> bool {
        if self.is_available() {
            self.current_users += 1;
            true
        } else {
            false
        }
    }

    /// Release (decrement user count)
    pub fn release(&mut self) {
        if self.current_users > 0 {
            self.current_users -= 1;
        }
    }
}

// ============================================================================
// SERVICE DESCRIPTOR
// ============================================================================

/// Description of a service offered by a cooperative process
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    /// Service name
    pub name: String,
    /// Category
    pub category: CapabilityCategory,
    /// Required capabilities
    pub requirements: Vec<CapabilityCategory>,
    /// Max concurrent users
    pub max_users: u32,
    /// Priority
    pub priority: u8,
}

// ============================================================================
// REGISTRY
// ============================================================================

/// Capability/service registry
pub struct CapabilityRegistry {
    /// All registered capabilities
    capabilities: BTreeMap<u64, RegisteredCapability>,
    /// Category → capability IDs index
    category_index: BTreeMap<u8, Vec<u64>>,
    /// PID → capability IDs
    pid_index: BTreeMap<u64, Vec<u64>>,
    /// Next capability ID
    next_id: u64,
    /// Max registrations per process
    max_per_process: usize,
    /// Total registrations
    pub total_registrations: u64,
    /// Total lookups
    pub total_lookups: u64,
    /// Total unregistrations
    pub total_unregistrations: u64,
}

impl CapabilityRegistry {
    pub fn new(max_per_process: usize) -> Self {
        Self {
            capabilities: BTreeMap::new(),
            category_index: BTreeMap::new(),
            pid_index: BTreeMap::new(),
            next_id: 1,
            max_per_process,
            total_registrations: 0,
            total_lookups: 0,
            total_unregistrations: 0,
        }
    }

    /// Register a capability
    pub fn register(
        &mut self,
        pid: u64,
        category: CapabilityCategory,
        name: String,
        timestamp: u64,
    ) -> Option<u64> {
        // Check per-process limit
        let pid_caps = self.pid_index.entry(pid).or_insert_with(Vec::new);
        if pid_caps.len() >= self.max_per_process {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;

        let cap = RegisteredCapability::new(id, pid, category, name, timestamp);
        self.capabilities.insert(id, cap);

        pid_caps.push(id);

        let cat_key = category as u8;
        self.category_index
            .entry(cat_key)
            .or_insert_with(Vec::new)
            .push(id);

        self.total_registrations += 1;
        Some(id)
    }

    /// Unregister a capability
    pub fn unregister(&mut self, id: u64) -> bool {
        if let Some(cap) = self.capabilities.remove(&id) {
            // Remove from pid index
            if let Some(pids) = self.pid_index.get_mut(&cap.provider_pid) {
                pids.retain(|&cid| cid != id);
            }
            // Remove from category index
            let cat_key = cap.category as u8;
            if let Some(cats) = self.category_index.get_mut(&cat_key) {
                cats.retain(|&cid| cid != id);
            }
            self.total_unregistrations += 1;
            true
        } else {
            false
        }
    }

    /// Unregister all capabilities for a process
    pub fn unregister_pid(&mut self, pid: u64) {
        if let Some(cap_ids) = self.pid_index.remove(&pid) {
            for id in cap_ids {
                if let Some(cap) = self.capabilities.remove(&id) {
                    let cat_key = cap.category as u8;
                    if let Some(cats) = self.category_index.get_mut(&cat_key) {
                        cats.retain(|&cid| cid != id);
                    }
                    self.total_unregistrations += 1;
                }
            }
        }
    }

    /// Find capabilities by category
    pub fn find_by_category(&self, category: CapabilityCategory) -> Vec<&RegisteredCapability> {
        let cat_key = category as u8;
        if let Some(ids) = self.category_index.get(&cat_key) {
            ids.iter()
                .filter_map(|id| self.capabilities.get(id))
                .filter(|cap| cap.is_available())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find best capability by category (highest priority, lowest response time)
    pub fn find_best(&mut self, category: CapabilityCategory) -> Option<u64> {
        self.total_lookups += 1;
        let cat_key = category as u8;

        let ids = self.category_index.get(&cat_key)?;

        let mut best_id: Option<u64> = None;
        let mut best_score = 0u64;

        for &id in ids {
            if let Some(cap) = self.capabilities.get(&id) {
                if !cap.is_available() {
                    continue;
                }
                // Score: priority * 1000 - response_time
                let score = (cap.priority as u64) * 1000
                    + 1000u64.saturating_sub(cap.avg_response_us / 10);
                if score > best_score {
                    best_score = score;
                    best_id = Some(id);
                }
            }
        }

        best_id
    }

    /// Get capability by ID
    pub fn get(&self, id: u64) -> Option<&RegisteredCapability> {
        self.capabilities.get(&id)
    }

    /// Get mutable capability by ID
    pub fn get_mut(&mut self, id: u64) -> Option<&mut RegisteredCapability> {
        self.capabilities.get_mut(&id)
    }

    /// Get all capabilities for a process
    pub fn get_for_pid(&self, pid: u64) -> Vec<&RegisteredCapability> {
        if let Some(ids) = self.pid_index.get(&pid) {
            ids.iter()
                .filter_map(|id| self.capabilities.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Total registered capabilities
    pub fn total_count(&self) -> usize {
        self.capabilities.len()
    }

    /// Available capabilities count
    pub fn available_count(&self) -> usize {
        self.capabilities.values().filter(|c| c.is_available()).count()
    }

    /// Health check — mark offline any cap not checked recently
    pub fn health_check(&mut self, current_time: u64, timeout_ms: u64) {
        for cap in self.capabilities.values_mut() {
            if current_time.saturating_sub(cap.last_health_check) > timeout_ms {
                cap.status = CapabilityStatus::Offline;
            }
        }
    }

    /// Update health check timestamp
    pub fn update_health(&mut self, id: u64, timestamp: u64) {
        if let Some(cap) = self.capabilities.get_mut(&id) {
            cap.last_health_check = timestamp;
            if cap.status == CapabilityStatus::Offline {
                cap.status = CapabilityStatus::Available;
            }
        }
    }
}

// ============================================================================
// SERVICE DIRECTORY
// ============================================================================

/// High-level service directory built on capability registry
pub struct ServiceDirectory {
    /// Underlying registry
    registry: CapabilityRegistry,
    /// Service dependencies
    dependencies: BTreeMap<u64, Vec<u64>>,
}

impl ServiceDirectory {
    pub fn new(max_per_process: usize) -> Self {
        Self {
            registry: CapabilityRegistry::new(max_per_process),
            dependencies: BTreeMap::new(),
        }
    }

    /// Register a service
    pub fn register_service(
        &mut self,
        pid: u64,
        descriptor: &ServiceDescriptor,
        timestamp: u64,
    ) -> Option<u64> {
        let id = self.registry.register(
            pid,
            descriptor.category,
            descriptor.name.clone(),
            timestamp,
        )?;

        if let Some(cap) = self.registry.get_mut(id) {
            cap.priority = descriptor.priority;
            cap.capacity = descriptor.max_users;
        }

        // Resolve dependencies
        let mut dep_ids = Vec::new();
        for req in &descriptor.requirements {
            if let Some(dep_id) = self.registry.find_best(*req) {
                dep_ids.push(dep_id);
            }
        }
        if !dep_ids.is_empty() {
            self.dependencies.insert(id, dep_ids);
        }

        Some(id)
    }

    /// Check if all deps for a service are available
    pub fn deps_satisfied(&self, service_id: u64) -> bool {
        if let Some(deps) = self.dependencies.get(&service_id) {
            deps.iter().all(|dep_id| {
                self.registry
                    .get(*dep_id)
                    .map_or(false, |c| c.is_available())
            })
        } else {
            true
        }
    }

    /// Get registry
    pub fn registry(&self) -> &CapabilityRegistry {
        &self.registry
    }

    /// Get mutable registry
    pub fn registry_mut(&mut self) -> &mut CapabilityRegistry {
        &mut self.registry
    }
}
