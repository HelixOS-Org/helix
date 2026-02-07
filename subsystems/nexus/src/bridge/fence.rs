//! # Bridge Fence System
//!
//! Memory and ordering fence management for syscall processing:
//! - Memory barrier coordination
//! - Ordering guarantee enforcement
//! - Fence point tracking
//! - Dependency ordering
//! - Completion notifications

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FENCE TYPES
// ============================================================================

/// Fence type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FenceType {
    /// Read barrier
    Read,
    /// Write barrier
    Write,
    /// Full barrier (read + write)
    Full,
    /// Acquire semantics
    Acquire,
    /// Release semantics
    Release,
    /// Sequential consistency
    SeqCst,
}

/// Fence state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceState {
    /// Pending (not yet signaled)
    Pending,
    /// Signaled (complete)
    Signaled,
    /// Timed out
    TimedOut,
    /// Cancelled
    Cancelled,
}

/// Fence scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceScope {
    /// Thread-local
    Thread,
    /// Process-wide
    Process,
    /// System-wide
    System,
    /// Device (DMA)
    Device,
}

// ============================================================================
// FENCE POINT
// ============================================================================

/// A fence point
#[derive(Debug, Clone)]
pub struct FencePoint {
    /// Fence id
    pub id: u64,
    /// Type
    pub fence_type: FenceType,
    /// Scope
    pub scope: FenceScope,
    /// State
    pub state: FenceState,
    /// Created timestamp
    pub created_at: u64,
    /// Signaled timestamp
    pub signaled_at: Option<u64>,
    /// Timeout (ns from creation)
    pub timeout_ns: u64,
    /// Waiters count
    pub waiters: u32,
    /// Dependencies (fence ids that must complete first)
    pub dependencies: Vec<u64>,
}

impl FencePoint {
    pub fn new(id: u64, fence_type: FenceType, scope: FenceScope, now: u64) -> Self {
        Self {
            id,
            fence_type,
            scope,
            state: FenceState::Pending,
            created_at: now,
            signaled_at: None,
            timeout_ns: 10_000_000_000, // 10s default
            waiters: 0,
            dependencies: Vec::new(),
        }
    }

    /// Signal completion
    pub fn signal(&mut self, now: u64) {
        if self.state == FenceState::Pending {
            self.state = FenceState::Signaled;
            self.signaled_at = Some(now);
        }
    }

    /// Check timeout
    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.state == FenceState::Pending {
            if now.saturating_sub(self.created_at) >= self.timeout_ns {
                self.state = FenceState::TimedOut;
                return true;
            }
        }
        false
    }

    /// Is complete?
    pub fn is_complete(&self) -> bool {
        self.state == FenceState::Signaled
    }

    /// Latency (if signaled)
    pub fn latency_ns(&self) -> Option<u64> {
        self.signaled_at.map(|s| s.saturating_sub(self.created_at))
    }

    /// Add dependency
    pub fn add_dependency(&mut self, fence_id: u64) {
        if !self.dependencies.contains(&fence_id) {
            self.dependencies.push(fence_id);
        }
    }
}

// ============================================================================
// FENCE CHAIN
// ============================================================================

/// An ordered chain of fences
#[derive(Debug)]
pub struct FenceChain {
    /// Chain id
    pub id: u64,
    /// Fence ids in order
    pub fences: Vec<u64>,
    /// Current position (index of next to signal)
    pub position: usize,
}

impl FenceChain {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            fences: Vec::new(),
            position: 0,
        }
    }

    /// Add fence to chain
    pub fn push(&mut self, fence_id: u64) {
        self.fences.push(fence_id);
    }

    /// Current fence
    pub fn current(&self) -> Option<u64> {
        self.fences.get(self.position).copied()
    }

    /// Advance to next
    pub fn advance(&mut self) -> Option<u64> {
        if self.position < self.fences.len() {
            self.position += 1;
        }
        self.current()
    }

    /// Is complete?
    pub fn is_complete(&self) -> bool {
        self.position >= self.fences.len()
    }

    /// Remaining count
    pub fn remaining(&self) -> usize {
        self.fences.len().saturating_sub(self.position)
    }
}

// ============================================================================
// FENCE POOL
// ============================================================================

/// Pool of reusable fence objects
#[derive(Debug)]
pub struct FencePool {
    /// Available fence ids
    available: Vec<u64>,
    /// Next id
    next_id: u64,
    /// Pool capacity
    pub capacity: usize,
}

impl FencePool {
    pub fn new(capacity: usize) -> Self {
        Self {
            available: Vec::new(),
            next_id: 1,
            capacity,
        }
    }

    /// Allocate fence id
    pub fn allocate(&mut self) -> u64 {
        if let Some(id) = self.available.pop() {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        }
    }

    /// Return fence id to pool
    pub fn release(&mut self, id: u64) {
        if self.available.len() < self.capacity {
            self.available.push(id);
        }
    }

    /// Available count
    pub fn available_count(&self) -> usize {
        self.available.len()
    }
}

// ============================================================================
// FENCE ENGINE
// ============================================================================

/// Fence stats
#[derive(Debug, Clone, Default)]
pub struct BridgeFenceStats {
    /// Active fences
    pub active_fences: usize,
    /// Total created
    pub total_created: u64,
    /// Total signaled
    pub total_signaled: u64,
    /// Total timed out
    pub total_timed_out: u64,
    /// Average latency (ns)
    pub avg_latency_ns: f64,
}

/// Bridge fence manager
pub struct BridgeFenceManager {
    /// Active fences
    fences: BTreeMap<u64, FencePoint>,
    /// Fence chains
    chains: BTreeMap<u64, FenceChain>,
    /// Fence pool
    pool: FencePool,
    /// Next chain id
    next_chain_id: u64,
    /// Stats
    stats: BridgeFenceStats,
    /// Latency accumulator
    latency_sum: f64,
    latency_count: u64,
}

impl BridgeFenceManager {
    pub fn new() -> Self {
        Self {
            fences: BTreeMap::new(),
            chains: BTreeMap::new(),
            pool: FencePool::new(1024),
            next_chain_id: 1,
            stats: BridgeFenceStats::default(),
            latency_sum: 0.0,
            latency_count: 0,
        }
    }

    /// Create fence
    pub fn create_fence(
        &mut self,
        fence_type: FenceType,
        scope: FenceScope,
        now: u64,
    ) -> u64 {
        let id = self.pool.allocate();
        let fence = FencePoint::new(id, fence_type, scope, now);
        self.fences.insert(id, fence);
        self.stats.total_created += 1;
        self.update_stats();
        id
    }

    /// Signal fence
    pub fn signal(&mut self, id: u64, now: u64) -> bool {
        if let Some(fence) = self.fences.get_mut(&id) {
            fence.signal(now);
            if let Some(lat) = fence.latency_ns() {
                self.latency_sum += lat as f64;
                self.latency_count += 1;
            }
            self.stats.total_signaled += 1;
            true
        } else {
            false
        }
    }

    /// Check timeouts
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();
        for (id, fence) in self.fences.iter_mut() {
            if fence.check_timeout(now) {
                timed_out.push(*id);
                self.stats.total_timed_out += 1;
            }
        }
        timed_out
    }

    /// Clean up completed fences
    pub fn cleanup(&mut self) {
        let complete_ids: Vec<u64> = self.fences.iter()
            .filter(|(_, f)| f.state != FenceState::Pending)
            .map(|(&id, _)| id)
            .collect();
        for id in complete_ids {
            self.fences.remove(&id);
            self.pool.release(id);
        }
        self.update_stats();
    }

    /// Create chain
    pub fn create_chain(&mut self) -> u64 {
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.chains.insert(id, FenceChain::new(id));
        id
    }

    /// Get fence
    pub fn fence(&self, id: u64) -> Option<&FencePoint> {
        self.fences.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_fences = self.fences.len();
        if self.latency_count > 0 {
            self.stats.avg_latency_ns = self.latency_sum / self.latency_count as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &BridgeFenceStats {
        &self.stats
    }
}
