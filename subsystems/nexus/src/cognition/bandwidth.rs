//! # Cognitive Bandwidth Management
//!
//! Manages the cognitive processing bandwidth across domains.
//! Ensures fair resource allocation and prevents cognitive overload.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// BANDWIDTH TYPES
// ============================================================================

/// Bandwidth allocation for a domain
#[derive(Debug, Clone)]
pub struct BandwidthAllocation {
    /// Domain ID
    pub domain_id: DomainId,
    /// Allocated cycles per tick
    pub cycles_per_tick: u64,
    /// Priority weight (1-100)
    pub weight: u32,
    /// Minimum guaranteed bandwidth
    pub min_bandwidth: u64,
    /// Maximum allowed bandwidth
    pub max_bandwidth: u64,
    /// Current usage
    pub current_usage: u64,
    /// Accumulated debt (for fair scheduling)
    pub debt: i64,
}

/// Bandwidth request
#[derive(Debug, Clone)]
pub struct BandwidthRequest {
    /// Requesting domain
    pub domain_id: DomainId,
    /// Requested cycles
    pub cycles: u64,
    /// Priority
    pub priority: BandwidthPriority,
    /// Deadline (optional)
    pub deadline: Option<Timestamp>,
    /// Can be deferred
    pub deferrable: bool,
}

/// Bandwidth priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BandwidthPriority {
    /// Background processing
    Background = 0,
    /// Normal priority
    Normal     = 1,
    /// Elevated priority
    Elevated   = 2,
    /// High priority
    High       = 3,
    /// Critical - must execute
    Critical   = 4,
}

/// Result of a bandwidth request
#[derive(Debug, Clone)]
pub struct BandwidthGrant {
    /// Request ID
    pub request_id: u64,
    /// Granted cycles
    pub granted_cycles: u64,
    /// Start time
    pub start_time: Timestamp,
    /// Must complete by
    pub deadline: Timestamp,
    /// Was request fully satisfied
    pub fully_satisfied: bool,
}

// ============================================================================
// BANDWIDTH MANAGER
// ============================================================================

/// Manages bandwidth allocation
pub struct BandwidthManager {
    /// Total available bandwidth (cycles per tick)
    total_bandwidth: u64,
    /// Allocations by domain
    allocations: BTreeMap<DomainId, BandwidthAllocation>,
    /// Pending requests
    pending_requests: Vec<BandwidthRequest>,
    /// Next request ID
    next_request_id: AtomicU64,
    /// Current tick
    current_tick: u64,
    /// Configuration
    config: BandwidthConfig,
    /// Statistics
    stats: BandwidthStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct BandwidthConfig {
    /// Total cycles per tick
    pub total_cycles: u64,
    /// Reserved for critical operations
    pub reserved_cycles: u64,
    /// Enable fair scheduling
    pub fair_scheduling: bool,
    /// Debt limit for fair scheduling
    pub max_debt: i64,
    /// Allow borrowing from future ticks
    pub allow_borrowing: bool,
    /// Maximum borrow amount
    pub max_borrow: u64,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            total_cycles: 1_000_000,
            reserved_cycles: 100_000,
            fair_scheduling: true,
            max_debt: 500_000,
            allow_borrowing: true,
            max_borrow: 200_000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct BandwidthStats {
    /// Total requests received
    pub total_requests: u64,
    /// Requests fully satisfied
    pub satisfied_requests: u64,
    /// Requests partially satisfied
    pub partial_requests: u64,
    /// Requests denied
    pub denied_requests: u64,
    /// Total cycles allocated
    pub total_allocated: u64,
    /// Total cycles used
    pub total_used: u64,
    /// Average utilization
    pub avg_utilization: f32,
    /// Peak utilization
    pub peak_utilization: f32,
}

impl BandwidthManager {
    /// Create a new bandwidth manager
    pub fn new(config: BandwidthConfig) -> Self {
        Self {
            total_bandwidth: config.total_cycles,
            allocations: BTreeMap::new(),
            pending_requests: Vec::new(),
            next_request_id: AtomicU64::new(1),
            current_tick: 0,
            config,
            stats: BandwidthStats::default(),
        }
    }

    /// Register a domain
    pub fn register_domain(
        &mut self,
        domain_id: DomainId,
        weight: u32,
        min_bandwidth: u64,
        max_bandwidth: u64,
    ) {
        let allocation = BandwidthAllocation {
            domain_id,
            cycles_per_tick: 0,
            weight,
            min_bandwidth,
            max_bandwidth,
            current_usage: 0,
            debt: 0,
        };
        self.allocations.insert(domain_id, allocation);
        self.recalculate_allocations();
    }

    /// Unregister a domain
    pub fn unregister_domain(&mut self, domain_id: DomainId) {
        self.allocations.remove(&domain_id);
        self.recalculate_allocations();
    }

    /// Request bandwidth
    pub fn request(&mut self, request: BandwidthRequest) -> Option<BandwidthGrant> {
        self.stats.total_requests += 1;
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        // Get allocation for domain
        let allocation = match self.allocations.get_mut(&request.domain_id) {
            Some(a) => a,
            None => {
                self.stats.denied_requests += 1;
                return None;
            },
        };

        // Calculate available bandwidth
        let available = allocation
            .cycles_per_tick
            .saturating_sub(allocation.current_usage);
        let can_borrow = if self.config.allow_borrowing {
            self.config
                .max_borrow
                .saturating_sub((-allocation.debt).max(0) as u64)
        } else {
            0
        };

        let total_available = available + can_borrow;

        // Handle critical requests
        if request.priority == BandwidthPriority::Critical {
            let granted = request.cycles.min(total_available);
            if granted > 0 {
                allocation.current_usage += granted;
                if granted > available {
                    allocation.debt -= (granted - available) as i64;
                }
                self.stats.satisfied_requests += 1;
                self.stats.total_allocated += granted;

                return Some(BandwidthGrant {
                    request_id,
                    granted_cycles: granted,
                    start_time: Timestamp::now(),
                    deadline: request.deadline.unwrap_or(Timestamp::now()),
                    fully_satisfied: granted >= request.cycles,
                });
            }
        }

        // Normal request handling
        if request.cycles <= available {
            allocation.current_usage += request.cycles;
            self.stats.satisfied_requests += 1;
            self.stats.total_allocated += request.cycles;

            Some(BandwidthGrant {
                request_id,
                granted_cycles: request.cycles,
                start_time: Timestamp::now(),
                deadline: request.deadline.unwrap_or(Timestamp::now()),
                fully_satisfied: true,
            })
        } else if request.deferrable {
            // Defer to next tick
            self.pending_requests.push(request);
            None
        } else {
            // Partial allocation
            let granted = available;
            if granted > 0 {
                allocation.current_usage += granted;
                self.stats.partial_requests += 1;
                self.stats.total_allocated += granted;

                Some(BandwidthGrant {
                    request_id,
                    granted_cycles: granted,
                    start_time: Timestamp::now(),
                    deadline: request.deadline.unwrap_or(Timestamp::now()),
                    fully_satisfied: false,
                })
            } else {
                self.stats.denied_requests += 1;
                None
            }
        }
    }

    /// Report actual usage
    pub fn report_usage(&mut self, domain_id: DomainId, cycles_used: u64) {
        if let Some(allocation) = self.allocations.get_mut(&domain_id) {
            self.stats.total_used += cycles_used;

            // Update debt for fair scheduling
            if self.config.fair_scheduling {
                let expected = allocation.cycles_per_tick;
                if cycles_used < expected {
                    // Build credit
                    allocation.debt += (expected - cycles_used) as i64;
                    allocation.debt = allocation.debt.min(self.config.max_debt);
                }
            }
        }
    }

    /// Start new tick
    pub fn new_tick(&mut self) {
        self.current_tick += 1;

        // Reset usage counters
        for allocation in self.allocations.values_mut() {
            allocation.current_usage = 0;
        }

        // Calculate utilization
        let utilization = if self.total_bandwidth > 0 {
            self.stats.total_used as f32 / self.total_bandwidth as f32
        } else {
            0.0
        };

        self.stats.avg_utilization = (self.stats.avg_utilization * (self.current_tick - 1) as f32
            + utilization)
            / self.current_tick as f32;

        if utilization > self.stats.peak_utilization {
            self.stats.peak_utilization = utilization;
        }

        // Process pending requests
        let pending = core::mem::take(&mut self.pending_requests);
        for request in pending {
            // Try again
            let _ = self.request(request);
        }
    }

    /// Recalculate allocations based on weights
    fn recalculate_allocations(&mut self) {
        let total_weight: u32 = self.allocations.values().map(|a| a.weight).sum();
        if total_weight == 0 {
            return;
        }

        let available = self.total_bandwidth - self.config.reserved_cycles;

        for allocation in self.allocations.values_mut() {
            let fair_share = (available as u64 * allocation.weight as u64) / total_weight as u64;
            allocation.cycles_per_tick = fair_share
                .max(allocation.min_bandwidth)
                .min(allocation.max_bandwidth);
        }
    }

    /// Get allocation for domain
    pub fn get_allocation(&self, domain_id: DomainId) -> Option<&BandwidthAllocation> {
        self.allocations.get(&domain_id)
    }

    /// Get total available bandwidth
    pub fn total_available(&self) -> u64 {
        let used: u64 = self.allocations.values().map(|a| a.current_usage).sum();
        self.total_bandwidth.saturating_sub(used)
    }

    /// Get statistics
    pub fn stats(&self) -> &BandwidthStats {
        &self.stats
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }
}

// ============================================================================
// BANDWIDTH THROTTLE
// ============================================================================

/// Throttles bandwidth usage
pub struct BandwidthThrottle {
    /// Maximum rate (cycles per second)
    max_rate: u64,
    /// Current window start
    window_start: Timestamp,
    /// Cycles used in current window
    window_usage: u64,
    /// Window duration (nanoseconds)
    window_duration_ns: u64,
}

impl BandwidthThrottle {
    /// Create a new throttle
    pub fn new(max_cycles_per_second: u64) -> Self {
        Self {
            max_rate: max_cycles_per_second,
            window_start: Timestamp::now(),
            window_usage: 0,
            window_duration_ns: 1_000_000_000, // 1 second
        }
    }

    /// Check if can proceed
    pub fn can_proceed(&mut self, cycles: u64) -> bool {
        let now = Timestamp::now();

        // Check if window expired
        if now.elapsed_since(self.window_start) >= self.window_duration_ns {
            self.window_start = now;
            self.window_usage = 0;
        }

        self.window_usage + cycles <= self.max_rate
    }

    /// Record usage
    pub fn record(&mut self, cycles: u64) {
        self.window_usage += cycles;
    }

    /// Get remaining budget
    pub fn remaining(&self) -> u64 {
        self.max_rate.saturating_sub(self.window_usage)
    }

    /// Get utilization
    pub fn utilization(&self) -> f32 {
        if self.max_rate > 0 {
            self.window_usage as f32 / self.max_rate as f32
        } else {
            1.0
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandwidth_manager() {
        let config = BandwidthConfig::default();
        let mut manager = BandwidthManager::new(config);

        manager.register_domain(DomainId::new(1), 50, 10000, 500000);
        manager.register_domain(DomainId::new(2), 50, 10000, 500000);

        let alloc1 = manager.get_allocation(DomainId::new(1)).unwrap();
        let alloc2 = manager.get_allocation(DomainId::new(2)).unwrap();

        // Should have roughly equal allocations
        assert!(alloc1.cycles_per_tick > 0);
        assert!(alloc2.cycles_per_tick > 0);
    }

    #[test]
    fn test_bandwidth_request() {
        let config = BandwidthConfig::default();
        let mut manager = BandwidthManager::new(config);

        manager.register_domain(DomainId::new(1), 100, 10000, 500000);

        let request = BandwidthRequest {
            domain_id: DomainId::new(1),
            cycles: 1000,
            priority: BandwidthPriority::Normal,
            deadline: None,
            deferrable: false,
        };

        let grant = manager.request(request);
        assert!(grant.is_some());
        assert!(grant.unwrap().fully_satisfied);
    }

    #[test]
    fn test_throttle() {
        let mut throttle = BandwidthThrottle::new(1000);

        assert!(throttle.can_proceed(500));
        throttle.record(500);

        assert!(throttle.can_proceed(500));
        throttle.record(500);

        assert!(!throttle.can_proceed(1));
        assert_eq!(throttle.remaining(), 0);
    }
}
