//! # Coop Service Discovery
//!
//! Service discovery for cooperative distributed subsystems:
//! - Service registration with health metadata
//! - DNS-like service resolution
//! - Load-balanced endpoint selection
//! - Service version management
//! - Dependency graph construction
//! - Service mesh routing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Registering,
    Healthy,
    Degraded,
    Unhealthy,
    Draining,
    Deregistered,
}

/// Load balance strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LbStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Random,
    ConsistentHash,
    LeastLatency,
}

/// Service endpoint
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub id: u64,
    pub node_id: u64,
    pub address: u64,
    pub port: u16,
    pub weight: u32,
    pub state: ServiceState,
    pub active_connections: u32,
    pub max_connections: u32,
    pub avg_latency_ns: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub last_health_check_ts: u64,
    pub health_check_failures: u32,
}

impl ServiceEndpoint {
    pub fn new(id: u64, node: u64, address: u64, port: u16) -> Self {
        Self {
            id, node_id: node, address, port, weight: 100,
            state: ServiceState::Registering, active_connections: 0,
            max_connections: 1024, avg_latency_ns: 0, success_count: 0,
            error_count: 0, last_health_check_ts: 0, health_check_failures: 0,
        }
    }

    pub fn is_available(&self) -> bool {
        self.state == ServiceState::Healthy && self.active_connections < self.max_connections
    }

    pub fn error_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 { 0.0 } else { self.error_count as f64 / total as f64 }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_connections == 0 { return 1.0; }
        self.active_connections as f64 / self.max_connections as f64
    }

    pub fn record_success(&mut self, latency_ns: u64) {
        self.success_count += 1;
        // EWMA latency
        if self.avg_latency_ns == 0 { self.avg_latency_ns = latency_ns; }
        else { self.avg_latency_ns = (self.avg_latency_ns * 7 + latency_ns) / 8; }
    }

    pub fn record_error(&mut self) { self.error_count += 1; }

    pub fn health_check_pass(&mut self, ts: u64) {
        self.last_health_check_ts = ts;
        self.health_check_failures = 0;
        if self.state == ServiceState::Registering || self.state == ServiceState::Unhealthy {
            self.state = ServiceState::Healthy;
        }
    }

    pub fn health_check_fail(&mut self, ts: u64) {
        self.last_health_check_ts = ts;
        self.health_check_failures += 1;
        if self.health_check_failures >= 3 { self.state = ServiceState::Unhealthy; }
        else if self.health_check_failures >= 1 { self.state = ServiceState::Degraded; }
    }
}

/// Service descriptor
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    pub name: String,
    pub version: u64,
    pub endpoints: Vec<u64>,
    pub lb_strategy: LbStrategy,
    pub rr_index: usize,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub created_ts: u64,
    pub total_requests: u64,
}

impl ServiceDescriptor {
    pub fn new(name: String, version: u64, ts: u64) -> Self {
        Self {
            name, version, endpoints: Vec::new(), lb_strategy: LbStrategy::RoundRobin,
            rr_index: 0, tags: Vec::new(), dependencies: Vec::new(),
            created_ts: ts, total_requests: 0,
        }
    }

    pub fn add_endpoint(&mut self, ep_id: u64) {
        if !self.endpoints.contains(&ep_id) { self.endpoints.push(ep_id); }
    }

    pub fn remove_endpoint(&mut self, ep_id: u64) {
        self.endpoints.retain(|&e| e != ep_id);
    }
}

/// Service dependency edge
#[derive(Debug, Clone)]
pub struct ServiceDepEdge {
    pub from_service: String,
    pub to_service: String,
    pub call_count: u64,
    pub avg_latency_ns: u64,
    pub error_rate: f64,
}

/// Service discovery stats
#[derive(Debug, Clone, Default)]
pub struct ServiceDiscoveryStats {
    pub total_services: usize,
    pub total_endpoints: usize,
    pub healthy_endpoints: usize,
    pub unhealthy_endpoints: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ns: f64,
    pub dependency_edges: usize,
}

/// Coop service discovery
pub struct CoopServiceDiscovery {
    services: BTreeMap<u64, ServiceDescriptor>,
    endpoints: BTreeMap<u64, ServiceEndpoint>,
    dep_edges: Vec<ServiceDepEdge>,
    stats: ServiceDiscoveryStats,
    next_id: u64,
    rng_state: u64,
}

impl CoopServiceDiscovery {
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(), endpoints: BTreeMap::new(),
            dep_edges: Vec::new(), stats: ServiceDiscoveryStats::default(),
            next_id: 1, rng_state: 0xabcdef0123456789,
        }
    }

    fn name_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    fn next_rand(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    pub fn register_service(&mut self, name: String, version: u64, ts: u64) -> u64 {
        let id = Self::name_hash(&name);
        self.services.entry(id).or_insert_with(|| ServiceDescriptor::new(name, version, ts));
        id
    }

    pub fn add_endpoint(&mut self, service_id: u64, node: u64, address: u64, port: u16) -> u64 {
        let ep_id = self.next_id; self.next_id += 1;
        self.endpoints.insert(ep_id, ServiceEndpoint::new(ep_id, node, address, port));
        if let Some(svc) = self.services.get_mut(&service_id) { svc.add_endpoint(ep_id); }
        ep_id
    }

    pub fn remove_endpoint(&mut self, service_id: u64, ep_id: u64) {
        self.endpoints.remove(&ep_id);
        if let Some(svc) = self.services.get_mut(&service_id) { svc.remove_endpoint(ep_id); }
    }

    pub fn resolve(&mut self, service_id: u64) -> Option<u64> {
        let svc = self.services.get_mut(&service_id)?;
        svc.total_requests += 1;
        let available: Vec<u64> = svc.endpoints.iter()
            .filter(|&&ep| self.endpoints.get(&ep).map_or(false, |e| e.is_available()))
            .copied().collect();
        if available.is_empty() { return None; }
        match svc.lb_strategy {
            LbStrategy::RoundRobin => {
                let idx = svc.rr_index % available.len();
                svc.rr_index = svc.rr_index.wrapping_add(1);
                Some(available[idx])
            }
            LbStrategy::LeastConnections => {
                available.iter().min_by_key(|&&ep| self.endpoints.get(&ep).map_or(u32::MAX, |e| e.active_connections)).copied()
            }
            LbStrategy::LeastLatency => {
                available.iter().min_by_key(|&&ep| self.endpoints.get(&ep).map_or(u64::MAX, |e| e.avg_latency_ns)).copied()
            }
            LbStrategy::Random => {
                let idx = self.next_rand() as usize % available.len();
                Some(available[idx])
            }
            _ => Some(available[0]),
        }
    }

    pub fn health_check(&mut self, ep_id: u64, passed: bool, ts: u64) {
        if let Some(ep) = self.endpoints.get_mut(&ep_id) {
            if passed { ep.health_check_pass(ts); } else { ep.health_check_fail(ts); }
        }
    }

    pub fn record_request(&mut self, ep_id: u64, success: bool, latency_ns: u64) {
        if let Some(ep) = self.endpoints.get_mut(&ep_id) {
            if success { ep.record_success(latency_ns); } else { ep.record_error(); }
        }
    }

    pub fn add_dependency(&mut self, from: String, to: String) {
        self.dep_edges.push(ServiceDepEdge { from_service: from, to_service: to, call_count: 0, avg_latency_ns: 0, error_rate: 0.0 });
    }

    pub fn recompute(&mut self) {
        self.stats.total_services = self.services.len();
        self.stats.total_endpoints = self.endpoints.len();
        self.stats.healthy_endpoints = self.endpoints.values().filter(|e| e.state == ServiceState::Healthy).count();
        self.stats.unhealthy_endpoints = self.endpoints.values().filter(|e| e.state == ServiceState::Unhealthy).count();
        self.stats.total_requests = self.services.values().map(|s| s.total_requests).sum();
        self.stats.total_errors = self.endpoints.values().map(|e| e.error_count).sum();
        let lats: Vec<u64> = self.endpoints.values().filter(|e| e.avg_latency_ns > 0).map(|e| e.avg_latency_ns).collect();
        self.stats.avg_latency_ns = if lats.is_empty() { 0.0 } else { lats.iter().sum::<u64>() as f64 / lats.len() as f64 };
        self.stats.dependency_edges = self.dep_edges.len();
    }

    pub fn service(&self, id: u64) -> Option<&ServiceDescriptor> { self.services.get(&id) }
    pub fn endpoint(&self, id: u64) -> Option<&ServiceEndpoint> { self.endpoints.get(&id) }
    pub fn stats(&self) -> &ServiceDiscoveryStats { &self.stats }
}
