//! # Cooperative Service Registry
//!
//! Service discovery and registration for cooperative subsystems:
//! - Service advertisement and capability publication
//! - Name-based and capability-based lookup
//! - Health-aware routing with liveness checks
//! - Versioned service endpoints
//! - Dependency graph tracking
//! - Graceful deregistration with drain support

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Draining,
    Down,
}

/// Service capability tag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCapability {
    Compute,
    Storage,
    Network,
    Security,
    Monitoring,
    Messaging,
    Scheduling,
    Memory,
    Custom(u32),
}

/// Service endpoint descriptor
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub service_id: u64,
    pub name: String,
    pub version_major: u16,
    pub version_minor: u16,
    pub version_patch: u16,
    pub capabilities: Vec<ServiceCapability>,
    pub health: ServiceHealth,
    pub registered_ns: u64,
    pub last_heartbeat_ns: u64,
    pub heartbeat_interval_ns: u64,
    pub load_score: u32, // 0-1000
    pub max_concurrent: u32,
    pub active_requests: u32,
    pub total_served: u64,
    pub dependencies: Vec<u64>, // service IDs this depends on
    pub metadata: LinearMap<u64, 64>,
}

impl ServiceEndpoint {
    pub fn new(service_id: u64, name: String, now_ns: u64) -> Self {
        Self {
            service_id,
            name,
            version_major: 0,
            version_minor: 1,
            version_patch: 0,
            capabilities: Vec::new(),
            health: ServiceHealth::Healthy,
            registered_ns: now_ns,
            last_heartbeat_ns: now_ns,
            heartbeat_interval_ns: 5_000_000_000, // 5s default
            load_score: 0,
            max_concurrent: 100,
            active_requests: 0,
            total_served: 0,
            dependencies: Vec::new(),
            metadata: LinearMap::new(),
        }
    }

    #[inline(always)]
    pub fn is_available(&self) -> bool {
        matches!(
            self.health,
            ServiceHealth::Healthy | ServiceHealth::Degraded
        ) && self.active_requests < self.max_concurrent
    }

    #[inline(always)]
    pub fn is_alive(&self, now_ns: u64) -> bool {
        let timeout = self.heartbeat_interval_ns * 3;
        now_ns.saturating_sub(self.last_heartbeat_ns) < timeout
    }

    #[inline(always)]
    pub fn heartbeat(&mut self, now_ns: u64, load: u32) {
        self.last_heartbeat_ns = now_ns;
        self.load_score = load;
    }

    #[inline(always)]
    pub fn has_capability(&self, cap: ServiceCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    #[inline(always)]
    pub fn version_tuple(&self) -> (u16, u16, u16) {
        (self.version_major, self.version_minor, self.version_patch)
    }
}

/// Lookup result with routing score
#[derive(Debug, Clone)]
pub struct LookupResult {
    pub service_id: u64,
    pub score: u32,
}

/// Dependency status for a service
#[derive(Debug, Clone)]
pub struct DependencyStatus {
    pub service_id: u64,
    pub satisfied: bool,
    pub missing_deps: Vec<u64>,
    pub degraded_deps: Vec<u64>,
}

/// Service event
#[derive(Debug, Clone)]
pub struct ServiceEvent {
    pub service_id: u64,
    pub event_type: ServiceEventType,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEventType {
    Registered,
    Deregistered,
    HealthChanged,
    Heartbeat,
    DependencyDown,
    Draining,
}

/// Cooperative Service Registry
pub struct CoopServiceRegistry {
    services: BTreeMap<u64, ServiceEndpoint>,
    name_index: LinearMap<u64, 64>,     // name_hash -> service_id
    cap_index: BTreeMap<u32, Vec<u64>>, // cap_discriminant -> [service_ids]
    events: VecDeque<ServiceEvent>,
    max_events: usize,
}

impl CoopServiceRegistry {
    pub fn new(max_events: usize) -> Self {
        Self {
            services: BTreeMap::new(),
            name_index: LinearMap::new(),
            cap_index: BTreeMap::new(),
            events: VecDeque::new(),
            max_events,
        }
    }

    fn hash_name(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn cap_key(cap: &ServiceCapability) -> u32 {
        match cap {
            ServiceCapability::Compute => 0,
            ServiceCapability::Storage => 1,
            ServiceCapability::Network => 2,
            ServiceCapability::Security => 3,
            ServiceCapability::Monitoring => 4,
            ServiceCapability::Messaging => 5,
            ServiceCapability::Scheduling => 6,
            ServiceCapability::Memory => 7,
            ServiceCapability::Custom(v) => 100 + v,
        }
    }

    pub fn register(&mut self, endpoint: ServiceEndpoint, now_ns: u64) {
        let id = endpoint.service_id;
        let name_hash = Self::hash_name(&endpoint.name);
        self.name_index.insert(name_hash, id);

        for cap in &endpoint.capabilities {
            let key = Self::cap_key(cap);
            self.cap_index.entry(key).or_insert_with(Vec::new).push(id);
        }

        self.emit_event(id, ServiceEventType::Registered, now_ns);
        self.services.insert(id, endpoint);
    }

    pub fn deregister(&mut self, service_id: u64, now_ns: u64) -> bool {
        if let Some(ep) = self.services.remove(&service_id) {
            let name_hash = Self::hash_name(&ep.name);
            self.name_index.remove(name_hash);
            for cap in &ep.capabilities {
                let key = Self::cap_key(cap);
                if let Some(list) = self.cap_index.get_mut(&key) {
                    list.retain(|&id| id != service_id);
                }
            }
            self.emit_event(service_id, ServiceEventType::Deregistered, now_ns);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn drain(&mut self, service_id: u64, now_ns: u64) {
        if let Some(ep) = self.services.get_mut(&service_id) {
            ep.health = ServiceHealth::Draining;
            self.emit_event(service_id, ServiceEventType::Draining, now_ns);
        }
    }

    #[inline]
    pub fn heartbeat(&mut self, service_id: u64, load: u32, now_ns: u64) {
        if let Some(ep) = self.services.get_mut(&service_id) {
            ep.heartbeat(now_ns, load);
        }
    }

    /// Lookup by name
    #[inline(always)]
    pub fn lookup_by_name(&self, name: &str) -> Option<&ServiceEndpoint> {
        let hash = Self::hash_name(name);
        self.name_index
            .get(hash)
            .and_then(|id| self.services.get(id))
    }

    /// Lookup by capability — returns available services sorted by load
    pub fn lookup_by_capability(&self, cap: ServiceCapability, now_ns: u64) -> Vec<LookupResult> {
        let key = Self::cap_key(&cap);
        let mut results = Vec::new();
        if let Some(ids) = self.cap_index.get(&key) {
            for &id in ids {
                if let Some(ep) = self.services.get(&id) {
                    if ep.is_available() && ep.is_alive(now_ns) {
                        let score = 1000u32.saturating_sub(ep.load_score);
                        results.push(LookupResult {
                            service_id: id,
                            score,
                        });
                    }
                }
            }
        }
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Check dependency satisfaction
    pub fn check_dependencies(&self, service_id: u64, now_ns: u64) -> Option<DependencyStatus> {
        let ep = self.services.get(&service_id)?;
        let mut missing = Vec::new();
        let mut degraded = Vec::new();

        for &dep_id in &ep.dependencies {
            if let Some(dep) = self.services.get(&dep_id) {
                if !dep.is_alive(now_ns) || dep.health == ServiceHealth::Down {
                    missing.push(dep_id);
                } else if dep.health == ServiceHealth::Degraded
                    || dep.health == ServiceHealth::Unhealthy
                {
                    degraded.push(dep_id);
                }
            } else {
                missing.push(dep_id);
            }
        }

        Some(DependencyStatus {
            service_id,
            satisfied: missing.is_empty(),
            missing_deps: missing,
            degraded_deps: degraded,
        })
    }

    /// Check for stale services and mark them down
    pub fn sweep_stale(&mut self, now_ns: u64) {
        let stale: Vec<u64> = self
            .services
            .iter()
            .filter(|(_, ep)| !ep.is_alive(now_ns) && ep.health != ServiceHealth::Down)
            .map(|(&id, _)| id)
            .collect();

        for id in stale {
            if let Some(ep) = self.services.get_mut(&id) {
                ep.health = ServiceHealth::Down;
            }
            self.emit_event(id, ServiceEventType::HealthChanged, now_ns);
        }
    }

    fn emit_event(&mut self, service_id: u64, event_type: ServiceEventType, ts: u64) {
        self.events.push_back(ServiceEvent {
            service_id,
            event_type,
            timestamp_ns: ts,
        });
        while self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    #[inline(always)]
    pub fn service(&self, id: u64) -> Option<&ServiceEndpoint> {
        self.services.get(&id)
    }

    #[inline(always)]
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    #[inline(always)]
    pub fn healthy_count(&self) -> usize {
        self.services
            .values()
            .filter(|ep| ep.health == ServiceHealth::Healthy)
            .count()
    }
}
