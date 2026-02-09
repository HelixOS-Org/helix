//! # Coop Service Mesh
//!
//! Service mesh protocol for inter-process communication:
//! - Service registration and discovery
//! - Load balancing across service instances
//! - Health checking with circuit breaker
//! - Request routing with path-based rules
//! - Service mesh observability (latency, error rates)
//! - Retry and timeout policies

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Registering
    Registering,
    /// Active and healthy
    Active,
    /// Degraded (some health checks failing)
    Degraded,
    /// Draining (no new connections)
    Draining,
    /// Down
    Down,
}

/// Load balance strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Round robin
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Weighted random
    WeightedRandom,
    /// Least latency
    LeastLatency,
    /// Consistent hash
    ConsistentHash,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation
    Closed,
    /// Failures exceeded threshold, blocking requests
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Service instance
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub instance_id: u64,
    pub pid: u64,
    pub state: ServiceState,
    pub weight: u32,
    pub active_connections: u32,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_latency_ns: u64,
    pub last_health_check_ns: u64,
    pub health_check_failures: u32,
    pub circuit: CircuitState,
    pub circuit_failure_count: u32,
    pub circuit_threshold: u32,
    pub circuit_open_ns: u64,
    pub circuit_half_open_timeout_ns: u64,
}

impl ServiceInstance {
    pub fn new(id: u64, pid: u64, weight: u32) -> Self {
        Self {
            instance_id: id,
            pid,
            state: ServiceState::Active,
            weight,
            active_connections: 0,
            total_requests: 0,
            total_errors: 0,
            total_latency_ns: 0,
            last_health_check_ns: 0,
            health_check_failures: 0,
            circuit: CircuitState::Closed,
            circuit_failure_count: 0,
            circuit_threshold: 5,
            circuit_open_ns: 0,
            circuit_half_open_timeout_ns: 10_000_000_000, // 10s
        }
    }

    #[inline]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ns as f64 / self.total_requests as f64
        }
    }

    #[inline]
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_errors as f64 / self.total_requests as f64
        }
    }

    pub fn record_request(&mut self, latency_ns: u64, success: bool) {
        self.total_requests += 1;
        self.total_latency_ns += latency_ns;
        if !success {
            self.total_errors += 1;
            self.circuit_failure_count += 1;
            if self.circuit_failure_count >= self.circuit_threshold {
                self.circuit = CircuitState::Open;
            }
        } else {
            self.circuit_failure_count = 0;
            if self.circuit == CircuitState::HalfOpen {
                self.circuit = CircuitState::Closed;
            }
        }
    }

    #[inline]
    pub fn check_circuit(&mut self, now_ns: u64) {
        if self.circuit == CircuitState::Open {
            if now_ns - self.circuit_open_ns > self.circuit_half_open_timeout_ns {
                self.circuit = CircuitState::HalfOpen;
            }
        }
    }

    #[inline(always)]
    pub fn is_available(&self) -> bool {
        self.state == ServiceState::Active && self.circuit != CircuitState::Open
    }

    pub fn health_check(&mut self, success: bool, now_ns: u64) {
        self.last_health_check_ns = now_ns;
        if success {
            self.health_check_failures = 0;
            if self.state == ServiceState::Degraded {
                self.state = ServiceState::Active;
            }
        } else {
            self.health_check_failures += 1;
            if self.health_check_failures >= 3 {
                self.state = ServiceState::Down;
            } else if self.health_check_failures >= 1 {
                self.state = ServiceState::Degraded;
            }
        }
    }
}

/// Service definition
#[derive(Debug)]
pub struct Service {
    pub service_id: u64,
    pub name: String,
    pub strategy: LoadBalanceStrategy,
    instances: Vec<ServiceInstance>,
    /// Round-robin counter
    rr_counter: usize,
    /// PRNG for weighted random
    rng_state: u64,
    pub retry_count: u32,
    pub timeout_ns: u64,
}

impl Service {
    pub fn new(id: u64, name: String, strategy: LoadBalanceStrategy) -> Self {
        Self {
            service_id: id,
            name,
            strategy,
            instances: Vec::new(),
            rr_counter: 0,
            rng_state: 0xcbf29ce484222325 ^ id,
            retry_count: 3,
            timeout_ns: 5_000_000_000,
        }
    }

    #[inline(always)]
    pub fn add_instance(&mut self, instance: ServiceInstance) {
        self.instances.push(instance);
    }

    #[inline(always)]
    pub fn remove_instance(&mut self, instance_id: u64) {
        self.instances.retain(|i| i.instance_id != instance_id);
    }

    /// Select an instance based on strategy
    pub fn select_instance(&mut self) -> Option<u64> {
        let available: Vec<usize> = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, i)| i.is_available())
            .map(|(idx, _)| idx)
            .collect();

        if available.is_empty() {
            return None;
        }

        let idx = match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                self.rr_counter += 1;
                available[self.rr_counter % available.len()]
            },
            LoadBalanceStrategy::LeastConnections => *available
                .iter()
                .min_by_key(|&&i| self.instances[i].active_connections)
                .unwrap(),
            LoadBalanceStrategy::LeastLatency => *available
                .iter()
                .min_by(|&&a, &&b| {
                    self.instances[a]
                        .avg_latency_ns()
                        .partial_cmp(&self.instances[b].avg_latency_ns())
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .unwrap(),
            LoadBalanceStrategy::WeightedRandom => {
                let total_weight: u32 = available.iter().map(|&i| self.instances[i].weight).sum();
                // xorshift64
                self.rng_state ^= self.rng_state << 13;
                self.rng_state ^= self.rng_state >> 7;
                self.rng_state ^= self.rng_state << 17;
                let target = (self.rng_state % total_weight as u64) as u32;
                let mut cumulative = 0u32;
                let mut chosen = available[0];
                for &i in &available {
                    cumulative += self.instances[i].weight;
                    if cumulative > target {
                        chosen = i;
                        break;
                    }
                }
                chosen
            },
            LoadBalanceStrategy::ConsistentHash => {
                // Simple hash-based selection
                available[self.rng_state as usize % available.len()]
            },
        };

        Some(self.instances[idx].instance_id)
    }

    #[inline(always)]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    #[inline(always)]
    pub fn healthy_count(&self) -> usize {
        self.instances.iter().filter(|i| i.is_available()).count()
    }

    #[inline(always)]
    pub fn total_requests(&self) -> u64 {
        self.instances.iter().map(|i| i.total_requests).sum()
    }

    #[inline(always)]
    pub fn total_errors(&self) -> u64 {
        self.instances.iter().map(|i| i.total_errors).sum()
    }
}

/// Service mesh stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopServiceMeshStats {
    pub total_services: usize,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ns: f64,
    pub circuit_open_count: usize,
}

/// Coop Service Mesh
pub struct CoopServiceMesh {
    services: BTreeMap<u64, Service>,
    stats: CoopServiceMeshStats,
    next_service_id: u64,
    next_instance_id: u64,
}

impl CoopServiceMesh {
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            stats: CoopServiceMeshStats::default(),
            next_service_id: 1,
            next_instance_id: 1,
        }
    }

    #[inline]
    pub fn register_service(&mut self, name: String, strategy: LoadBalanceStrategy) -> u64 {
        let id = self.next_service_id;
        self.next_service_id += 1;
        self.services.insert(id, Service::new(id, name, strategy));
        self.update_stats();
        id
    }

    #[inline]
    pub fn add_instance(&mut self, service_id: u64, pid: u64, weight: u32) -> u64 {
        let inst_id = self.next_instance_id;
        self.next_instance_id += 1;
        if let Some(svc) = self.services.get_mut(&service_id) {
            svc.add_instance(ServiceInstance::new(inst_id, pid, weight));
        }
        self.update_stats();
        inst_id
    }

    #[inline(always)]
    pub fn route(&mut self, service_id: u64) -> Option<u64> {
        self.services.get_mut(&service_id)?.select_instance()
    }

    fn update_stats(&mut self) {
        self.stats.total_services = self.services.len();
        self.stats.total_instances = self.services.values().map(|s| s.instance_count()).sum();
        self.stats.healthy_instances = self.services.values().map(|s| s.healthy_count()).sum();
        self.stats.total_requests = self.services.values().map(|s| s.total_requests()).sum();
        self.stats.total_errors = self.services.values().map(|s| s.total_errors()).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopServiceMeshStats {
        &self.stats
    }
}
