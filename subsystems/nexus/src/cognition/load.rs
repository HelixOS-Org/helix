//! # Cognitive Load Balancer
//!
//! Balances cognitive load across domains and resources.
//! Implements adaptive load distribution.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// LOAD TYPES
// ============================================================================

/// Load information for a domain
#[derive(Debug, Clone)]
pub struct DomainLoad {
    /// Domain ID
    pub domain: DomainId,
    /// Current load (0.0 - 1.0)
    pub current_load: f64,
    /// Pending items
    pub pending_items: u64,
    /// Active tasks
    pub active_tasks: u64,
    /// Capacity (items/cycle)
    pub capacity: u64,
    /// Last update time
    pub last_update: Timestamp,
    /// Health status
    pub healthy: bool,
    /// Weight (for weighted distribution)
    pub weight: f64,
}

impl DomainLoad {
    /// Check if overloaded
    pub fn is_overloaded(&self, threshold: f64) -> bool {
        self.current_load > threshold
    }

    /// Check if underloaded
    pub fn is_underloaded(&self, threshold: f64) -> bool {
        self.current_load < threshold
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> u64 {
        self.capacity.saturating_sub(self.active_tasks)
    }

    /// Calculate effective weight (weight * available capacity)
    pub fn effective_weight(&self) -> f64 {
        if !self.healthy {
            return 0.0;
        }
        self.weight * (1.0 - self.current_load)
    }
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Round-robin
    RoundRobin,
    /// Least loaded
    LeastLoaded,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Adaptive (learns from performance)
    Adaptive,
    /// Hash-based (consistent)
    HashBased,
    /// Random
    Random,
    /// Priority-based
    Priority,
}

/// Load balancer decision
#[derive(Debug, Clone)]
pub struct LoadBalanceDecision {
    /// Selected domain
    pub domain: DomainId,
    /// Strategy used
    pub strategy: LoadBalanceStrategy,
    /// Load at selection time
    pub load_at_selection: f64,
    /// Reason
    pub reason: String,
}

// ============================================================================
// LOAD BALANCER
// ============================================================================

/// Cognitive load balancer
pub struct LoadBalancer {
    /// Domain loads
    loads: BTreeMap<DomainId, DomainLoad>,
    /// Strategy
    strategy: LoadBalanceStrategy,
    /// Round-robin index
    rr_index: usize,
    /// Weighted RR state
    weighted_state: BTreeMap<DomainId, i64>,
    /// Overload threshold
    overload_threshold: f64,
    /// Underload threshold
    underload_threshold: f64,
    /// Configuration
    config: LoadBalancerConfig,
    /// Statistics
    stats: LoadBalancerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Default capacity
    pub default_capacity: u64,
    /// Default weight
    pub default_weight: f64,
    /// Overload threshold
    pub overload_threshold: f64,
    /// Underload threshold
    pub underload_threshold: f64,
    /// Enable adaptive learning
    pub enable_adaptive: bool,
    /// Load update interval (ns)
    pub update_interval_ns: u64,
    /// Health check timeout (ns)
    pub health_timeout_ns: u64,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            default_capacity: 100,
            default_weight: 1.0,
            overload_threshold: 0.8,
            underload_threshold: 0.2,
            enable_adaptive: true,
            update_interval_ns: 1_000_000_000, // 1 second
            health_timeout_ns: 5_000_000_000,  // 5 seconds
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct LoadBalancerStats {
    /// Total selections
    pub total_selections: u64,
    /// Selections per strategy
    pub strategy_selections: BTreeMap<String, u64>,
    /// Overload events
    pub overload_events: u64,
    /// Rebalance events
    pub rebalance_events: u64,
    /// Average load
    pub avg_load: f64,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(strategy: LoadBalanceStrategy, config: LoadBalancerConfig) -> Self {
        Self {
            loads: BTreeMap::new(),
            strategy,
            rr_index: 0,
            weighted_state: BTreeMap::new(),
            overload_threshold: config.overload_threshold,
            underload_threshold: config.underload_threshold,
            config,
            stats: LoadBalancerStats::default(),
        }
    }

    /// Register a domain
    pub fn register_domain(&mut self, domain: DomainId, capacity: Option<u64>, weight: Option<f64>) {
        let load = DomainLoad {
            domain,
            current_load: 0.0,
            pending_items: 0,
            active_tasks: 0,
            capacity: capacity.unwrap_or(self.config.default_capacity),
            last_update: Timestamp::now(),
            healthy: true,
            weight: weight.unwrap_or(self.config.default_weight),
        };

        self.loads.insert(domain, load);
        self.weighted_state.insert(domain, 0);
    }

    /// Unregister a domain
    pub fn unregister_domain(&mut self, domain: DomainId) -> bool {
        self.weighted_state.remove(&domain);
        self.loads.remove(&domain).is_some()
    }

    /// Update domain load
    pub fn update_load(
        &mut self,
        domain: DomainId,
        current_load: f64,
        pending: u64,
        active: u64,
    ) {
        if let Some(load) = self.loads.get_mut(&domain) {
            load.current_load = current_load.clamp(0.0, 1.0);
            load.pending_items = pending;
            load.active_tasks = active;
            load.last_update = Timestamp::now();

            // Check for overload
            if load.is_overloaded(self.overload_threshold) {
                self.stats.overload_events += 1;
            }
        }
    }

    /// Set domain health
    pub fn set_healthy(&mut self, domain: DomainId, healthy: bool) {
        if let Some(load) = self.loads.get_mut(&domain) {
            load.healthy = healthy;
        }
    }

    /// Select a domain for work
    pub fn select(&mut self) -> Option<LoadBalanceDecision> {
        let healthy_domains: Vec<_> = self.loads.values()
            .filter(|l| l.healthy)
            .collect();

        if healthy_domains.is_empty() {
            return None;
        }

        let decision = match self.strategy {
            LoadBalanceStrategy::RoundRobin => self.select_round_robin(&healthy_domains),
            LoadBalanceStrategy::LeastLoaded => self.select_least_loaded(&healthy_domains),
            LoadBalanceStrategy::WeightedRoundRobin => self.select_weighted_rr(&healthy_domains),
            LoadBalanceStrategy::Adaptive => self.select_adaptive(&healthy_domains),
            LoadBalanceStrategy::Random => self.select_random(&healthy_domains),
            LoadBalanceStrategy::Priority => self.select_priority(&healthy_domains),
            LoadBalanceStrategy::HashBased => self.select_round_robin(&healthy_domains), // Fallback
        };

        if let Some(ref d) = decision {
            self.stats.total_selections += 1;
            *self.stats.strategy_selections
                .entry(format!("{:?}", self.strategy))
                .or_default() += 1;
        }

        decision
    }

    /// Select with hash key (for consistent hashing)
    pub fn select_with_key(&self, key: &str) -> Option<LoadBalanceDecision> {
        let healthy_domains: Vec<_> = self.loads.values()
            .filter(|l| l.healthy)
            .collect();

        if healthy_domains.is_empty() {
            return None;
        }

        // Simple consistent hashing
        let hash: usize = key.bytes().map(|b| b as usize).sum();
        let idx = hash % healthy_domains.len();
        let selected = healthy_domains[idx];

        Some(LoadBalanceDecision {
            domain: selected.domain,
            strategy: LoadBalanceStrategy::HashBased,
            load_at_selection: selected.current_load,
            reason: format!("Hash-based selection for key: {}", key),
        })
    }

    /// Round-robin selection
    fn select_round_robin(&mut self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        if domains.is_empty() {
            return None;
        }

        let idx = self.rr_index % domains.len();
        self.rr_index = (self.rr_index + 1) % domains.len();

        let selected = domains[idx];
        Some(LoadBalanceDecision {
            domain: selected.domain,
            strategy: LoadBalanceStrategy::RoundRobin,
            load_at_selection: selected.current_load,
            reason: "Round-robin selection".into(),
        })
    }

    /// Least loaded selection
    fn select_least_loaded(&self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        let selected = domains.iter()
            .min_by(|a, b| {
                a.current_load.partial_cmp(&b.current_load).unwrap_or(core::cmp::Ordering::Equal)
            })?;

        Some(LoadBalanceDecision {
            domain: selected.domain,
            strategy: LoadBalanceStrategy::LeastLoaded,
            load_at_selection: selected.current_load,
            reason: format!("Least loaded at {:.1}%", selected.current_load * 100.0),
        })
    }

    /// Weighted round-robin selection
    fn select_weighted_rr(&mut self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        // Smooth weighted round-robin
        let mut max_weight = i64::MIN;
        let mut selected = None;

        // Update weights
        for domain_load in domains {
            let weight = self.weighted_state.entry(domain_load.domain).or_default();
            *weight += (domain_load.weight * 100.0) as i64;

            if *weight > max_weight {
                max_weight = *weight;
                selected = Some(domain_load);
            }
        }

        // Decrease selected weight
        if let Some(sel) = selected {
            let total_weight: i64 = domains.iter()
                .map(|d| (d.weight * 100.0) as i64)
                .sum();

            if let Some(weight) = self.weighted_state.get_mut(&sel.domain) {
                *weight -= total_weight;
            }

            return Some(LoadBalanceDecision {
                domain: sel.domain,
                strategy: LoadBalanceStrategy::WeightedRoundRobin,
                load_at_selection: sel.current_load,
                reason: format!("Weighted selection (weight: {:.2})", sel.weight),
            });
        }

        None
    }

    /// Adaptive selection (combines metrics)
    fn select_adaptive(&self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        // Score = (1 - load) * weight * available_capacity
        let scored: Vec<_> = domains.iter()
            .map(|d| {
                let score = d.effective_weight() * d.available_capacity() as f64;
                (d, score)
            })
            .collect();

        let selected = scored.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal))?;

        Some(LoadBalanceDecision {
            domain: selected.0.domain,
            strategy: LoadBalanceStrategy::Adaptive,
            load_at_selection: selected.0.current_load,
            reason: format!("Adaptive score: {:.2}", selected.1),
        })
    }

    /// Random selection
    fn select_random(&self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        if domains.is_empty() {
            return None;
        }

        // Simple pseudo-random
        let idx = (Timestamp::now().raw() as usize) % domains.len();
        let selected = domains[idx];

        Some(LoadBalanceDecision {
            domain: selected.domain,
            strategy: LoadBalanceStrategy::Random,
            load_at_selection: selected.current_load,
            reason: "Random selection".into(),
        })
    }

    /// Priority selection (lowest load with highest weight)
    fn select_priority(&self, domains: &[&DomainLoad]) -> Option<LoadBalanceDecision> {
        let selected = domains.iter()
            .filter(|d| !d.is_overloaded(self.overload_threshold))
            .max_by(|a, b| {
                a.weight.partial_cmp(&b.weight).unwrap_or(core::cmp::Ordering::Equal)
            })
            .or_else(|| domains.iter().min_by(|a, b| {
                a.current_load.partial_cmp(&b.current_load).unwrap_or(core::cmp::Ordering::Equal)
            }))?;

        Some(LoadBalanceDecision {
            domain: selected.domain,
            strategy: LoadBalanceStrategy::Priority,
            load_at_selection: selected.current_load,
            reason: format!("Priority weight: {:.2}", selected.weight),
        })
    }

    /// Check if rebalancing is needed
    pub fn needs_rebalance(&self) -> bool {
        let loads: Vec<f64> = self.loads.values()
            .filter(|l| l.healthy)
            .map(|l| l.current_load)
            .collect();

        if loads.len() < 2 {
            return false;
        }

        let max = loads.iter().cloned().fold(0.0_f64, f64::max);
        let min = loads.iter().cloned().fold(1.0_f64, f64::min);

        // Need rebalance if load difference > 50%
        max - min > 0.5
    }

    /// Get overloaded domains
    pub fn overloaded_domains(&self) -> Vec<DomainId> {
        self.loads.values()
            .filter(|l| l.is_overloaded(self.overload_threshold))
            .map(|l| l.domain)
            .collect()
    }

    /// Get underloaded domains
    pub fn underloaded_domains(&self) -> Vec<DomainId> {
        self.loads.values()
            .filter(|l| l.healthy && l.is_underloaded(self.underload_threshold))
            .map(|l| l.domain)
            .collect()
    }

    /// Get load for domain
    pub fn get_load(&self, domain: DomainId) -> Option<&DomainLoad> {
        self.loads.get(&domain)
    }

    /// Get all loads
    pub fn all_loads(&self) -> Vec<&DomainLoad> {
        self.loads.values().collect()
    }

    /// Get average load
    pub fn average_load(&self) -> f64 {
        let healthy: Vec<_> = self.loads.values()
            .filter(|l| l.healthy)
            .collect();

        if healthy.is_empty() {
            return 0.0;
        }

        healthy.iter().map(|l| l.current_load).sum::<f64>() / healthy.len() as f64
    }

    /// Set strategy
    pub fn set_strategy(&mut self, strategy: LoadBalanceStrategy) {
        self.strategy = strategy;
    }

    /// Get statistics
    pub fn stats(&self) -> &LoadBalancerStats {
        &self.stats
    }

    /// Get domain count
    pub fn domain_count(&self) -> usize {
        self.loads.len()
    }

    /// Get healthy domain count
    pub fn healthy_count(&self) -> usize {
        self.loads.values().filter(|l| l.healthy).count()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self::new(LoadBalanceStrategy::LeastLoaded, LoadBalancerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let config = LoadBalancerConfig::default();
        let mut balancer = LoadBalancer::new(LoadBalanceStrategy::RoundRobin, config);

        balancer.register_domain(DomainId::new(1), None, None);
        balancer.register_domain(DomainId::new(2), None, None);
        balancer.register_domain(DomainId::new(3), None, None);

        let d1 = balancer.select().unwrap().domain.as_u64();
        let d2 = balancer.select().unwrap().domain.as_u64();
        let d3 = balancer.select().unwrap().domain.as_u64();
        let d4 = balancer.select().unwrap().domain.as_u64();

        // Should cycle
        assert_eq!(d1, d4);
        assert_ne!(d1, d2);
        assert_ne!(d2, d3);
    }

    #[test]
    fn test_least_loaded() {
        let config = LoadBalancerConfig::default();
        let mut balancer = LoadBalancer::new(LoadBalanceStrategy::LeastLoaded, config);

        balancer.register_domain(DomainId::new(1), None, None);
        balancer.register_domain(DomainId::new(2), None, None);
        balancer.register_domain(DomainId::new(3), None, None);

        // Set different loads
        balancer.update_load(DomainId::new(1), 0.8, 0, 0);
        balancer.update_load(DomainId::new(2), 0.3, 0, 0);
        balancer.update_load(DomainId::new(3), 0.5, 0, 0);

        let decision = balancer.select().unwrap();
        assert_eq!(decision.domain.as_u64(), 2); // Least loaded
    }

    #[test]
    fn test_unhealthy_excluded() {
        let config = LoadBalancerConfig::default();
        let mut balancer = LoadBalancer::new(LoadBalanceStrategy::LeastLoaded, config);

        balancer.register_domain(DomainId::new(1), None, None);
        balancer.register_domain(DomainId::new(2), None, None);

        balancer.update_load(DomainId::new(1), 0.1, 0, 0);
        balancer.update_load(DomainId::new(2), 0.5, 0, 0);

        // Mark domain 1 as unhealthy
        balancer.set_healthy(DomainId::new(1), false);

        let decision = balancer.select().unwrap();
        assert_eq!(decision.domain.as_u64(), 2); // Domain 1 excluded
    }

    #[test]
    fn test_consistent_hashing() {
        let config = LoadBalancerConfig::default();
        let mut balancer = LoadBalancer::new(LoadBalanceStrategy::HashBased, config);

        balancer.register_domain(DomainId::new(1), None, None);
        balancer.register_domain(DomainId::new(2), None, None);
        balancer.register_domain(DomainId::new(3), None, None);

        let d1 = balancer.select_with_key("user123").unwrap().domain;
        let d2 = balancer.select_with_key("user123").unwrap().domain;

        // Same key should give same result
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_overload_detection() {
        let config = LoadBalancerConfig {
            overload_threshold: 0.8,
            ..Default::default()
        };
        let mut balancer = LoadBalancer::new(LoadBalanceStrategy::LeastLoaded, config);

        balancer.register_domain(DomainId::new(1), None, None);
        balancer.update_load(DomainId::new(1), 0.9, 0, 0);

        let overloaded = balancer.overloaded_domains();
        assert_eq!(overloaded.len(), 1);
    }
}
