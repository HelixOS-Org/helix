//! # Cooperative Service Discovery
//!
//! Service registration and discovery for cooperative processes:
//! - Service registration with metadata
//! - Health-aware discovery
//! - Load-balanced service selection
//! - Service versioning
//! - TTL-based expiry

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SERVICE TYPES
// ============================================================================

/// Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Registering
    Registering,
    /// Active
    Active,
    /// Draining (no new requests)
    Draining,
    /// Inactive
    Inactive,
    /// Failed
    Failed,
    /// Expired
    Expired,
}

/// Service type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    /// Compute service
    Compute,
    /// Storage service
    Storage,
    /// Network proxy
    NetworkProxy,
    /// IPC hub
    IpcHub,
    /// Monitoring
    Monitoring,
    /// Custom
    Custom,
}

/// Load balance strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Round robin
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Random
    Random,
    /// Weighted
    Weighted,
}

// ============================================================================
// SERVICE INSTANCE
// ============================================================================

/// Service instance
#[derive(Debug)]
pub struct ServiceInstance {
    /// Instance id
    pub id: u64,
    /// Owner pid
    pub pid: u64,
    /// Service name
    pub name: String,
    /// Version
    pub version: u32,
    /// Type
    pub service_type: ServiceType,
    /// State
    pub state: ServiceState,
    /// Weight (for weighted LB)
    pub weight: u32,
    /// Active connections
    pub active_connections: u32,
    /// Total requests served
    pub total_requests: u64,
    /// Average latency (EMA, ns)
    pub avg_latency_ns: f64,
    /// Error count
    pub errors: u64,
    /// Registration time
    pub registered_at: u64,
    /// Last heartbeat
    pub last_heartbeat: u64,
    /// TTL (ns)
    pub ttl_ns: u64,
    /// Tags: key_hash -> value_hash
    pub tags: LinearMap<u64, 64>,
}

impl ServiceInstance {
    pub fn new(
        id: u64,
        pid: u64,
        name: String,
        version: u32,
        service_type: ServiceType,
        now: u64,
        ttl_ns: u64,
    ) -> Self {
        Self {
            id,
            pid,
            name,
            version,
            service_type,
            state: ServiceState::Registering,
            weight: 100,
            active_connections: 0,
            total_requests: 0,
            avg_latency_ns: 0.0,
            errors: 0,
            registered_at: now,
            last_heartbeat: now,
            ttl_ns,
            tags: LinearMap::new(),
        }
    }

    /// Activate
    #[inline(always)]
    pub fn activate(&mut self) {
        self.state = ServiceState::Active;
    }

    /// Drain
    #[inline(always)]
    pub fn drain(&mut self) {
        self.state = ServiceState::Draining;
    }

    /// Heartbeat
    #[inline(always)]
    pub fn heartbeat(&mut self, now: u64) {
        self.last_heartbeat = now;
    }

    /// Is expired?
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.last_heartbeat) > self.ttl_ns
    }

    /// Record request
    #[inline]
    pub fn record_request(&mut self, latency_ns: u64, success: bool) {
        self.total_requests += 1;
        if !success {
            self.errors += 1;
        }
        let alpha = 0.2;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
    }

    /// Error rate
    #[inline]
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.errors as f64 / self.total_requests as f64
    }

    /// Is healthy? (active, not expired, low error rate)
    #[inline]
    pub fn is_healthy(&self, now: u64) -> bool {
        self.state == ServiceState::Active
            && !self.is_expired(now)
            && self.error_rate() < 0.5
    }
}

// ============================================================================
// SERVICE REGISTRY
// ============================================================================

/// Discovery query
#[derive(Debug)]
pub struct DiscoveryQuery {
    /// Service name (exact match)
    pub name: String,
    /// Minimum version
    pub min_version: Option<u32>,
    /// Service type
    pub service_type: Option<ServiceType>,
    /// Only healthy instances
    pub healthy_only: bool,
}

/// Discovery result
#[derive(Debug)]
pub struct DiscoveryResult {
    /// Matching instances
    pub instances: Vec<u64>,
    /// Selected instance (after LB)
    pub selected: Option<u64>,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Service discovery stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopDiscoveryStats {
    /// Total registered instances
    pub total_instances: usize,
    /// Active instances
    pub active_instances: usize,
    /// Failed instances
    pub failed_instances: usize,
    /// Total queries
    pub total_queries: u64,
    /// Total unique services
    pub unique_services: usize,
}

/// Cooperative service discovery manager
pub struct CoopDiscoveryManager {
    /// Instances
    instances: BTreeMap<u64, ServiceInstance>,
    /// Name index: name_hash -> instance ids
    name_index: BTreeMap<u64, Vec<u64>>,
    /// Load balance strategy
    pub strategy: LoadBalanceStrategy,
    /// Round-robin counter per service
    rr_counters: LinearMap<usize, 64>,
    /// Next instance id
    next_id: u64,
    /// Stats
    stats: CoopDiscoveryStats,
}

impl CoopDiscoveryManager {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            instances: BTreeMap::new(),
            name_index: BTreeMap::new(),
            strategy,
            rr_counters: LinearMap::new(),
            next_id: 1,
            stats: CoopDiscoveryStats::default(),
        }
    }

    fn name_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Register service instance
    pub fn register(
        &mut self,
        pid: u64,
        name: String,
        version: u32,
        service_type: ServiceType,
        now: u64,
        ttl_ns: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let nh = Self::name_hash(&name);
        let mut instance = ServiceInstance::new(id, pid, name, version, service_type, now, ttl_ns);
        instance.activate();

        self.instances.insert(id, instance);
        self.name_index.entry(nh).or_insert_with(Vec::new).push(id);

        self.update_stats();
        id
    }

    /// Deregister
    pub fn deregister(&mut self, id: u64) -> bool {
        if let Some(instance) = self.instances.remove(&id) {
            let nh = Self::name_hash(&instance.name);
            if let Some(ids) = self.name_index.get_mut(&nh) {
                ids.retain(|&i| i != id);
            }
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Heartbeat
    #[inline]
    pub fn heartbeat(&mut self, id: u64, now: u64) -> bool {
        if let Some(instance) = self.instances.get_mut(&id) {
            instance.heartbeat(now);
            true
        } else {
            false
        }
    }

    /// Discover services
    pub fn discover(&mut self, query: &DiscoveryQuery, now: u64) -> DiscoveryResult {
        self.stats.total_queries += 1;
        let nh = Self::name_hash(&query.name);

        let ids = self.name_index.get(&nh).cloned().unwrap_or_default();
        let mut matching = Vec::new();

        for &id in &ids {
            if let Some(inst) = self.instances.get(&id) {
                if query.healthy_only && !inst.is_healthy(now) {
                    continue;
                }
                if let Some(min_ver) = query.min_version {
                    if inst.version < min_ver {
                        continue;
                    }
                }
                if let Some(st) = query.service_type {
                    if inst.service_type != st {
                        continue;
                    }
                }
                matching.push(id);
            }
        }

        let selected = self.select(&matching, nh, now);

        DiscoveryResult {
            instances: matching,
            selected,
        }
    }

    fn select(&mut self, candidates: &[u64], name_hash: u64, now: u64) -> Option<u64> {
        if candidates.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let counter = self.rr_counters.entry(name_hash).or_insert(0);
                let idx = *counter % candidates.len();
                *counter += 1;
                Some(candidates[idx])
            }
            LoadBalanceStrategy::LeastConnections => {
                candidates.iter().copied()
                    .min_by_key(|&id| {
                        self.instances.get(&id).map(|i| i.active_connections).unwrap_or(u32::MAX)
                    })
            }
            LoadBalanceStrategy::Weighted => {
                // Choose highest weight that's healthy
                candidates.iter().copied()
                    .filter(|&id| {
                        self.instances.get(&id).map(|i| i.is_healthy(now)).unwrap_or(false)
                    })
                    .max_by_key(|&id| {
                        self.instances.get(&id).map(|i| i.weight).unwrap_or(0)
                    })
            }
            LoadBalanceStrategy::Random => {
                // Simple hash-based selection
                let idx = (now as usize) % candidates.len();
                Some(candidates[idx])
            }
        }
    }

    /// Expire stale instances
    pub fn expire_stale(&mut self, now: u64) {
        let expired: Vec<u64> = self.instances.iter()
            .filter(|(_, inst)| inst.is_expired(now))
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            if let Some(inst) = self.instances.get_mut(&id) {
                inst.state = ServiceState::Expired;
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_instances = self.instances.len();
        self.stats.active_instances = self.instances.values()
            .filter(|i| i.state == ServiceState::Active).count();
        self.stats.failed_instances = self.instances.values()
            .filter(|i| i.state == ServiceState::Failed).count();
        self.stats.unique_services = self.name_index.len();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopDiscoveryStats {
        &self.stats
    }
}
