//! RCU Callback Management
//!
//! This module provides callback information structures and coalescing for efficient batch processing.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::{CallbackId, GracePeriodId, CpuId};

/// Callback information
#[derive(Debug, Clone)]
pub struct CallbackInfo {
    /// Callback ID
    pub id: CallbackId,
    /// Registration timestamp
    pub registered_ns: u64,
    /// Target grace period
    pub target_gp: Option<GracePeriodId>,
    /// CPU that registered callback
    pub source_cpu: CpuId,
    /// Priority
    pub priority: CallbackPriority,
    /// Estimated execution time (nanoseconds)
    pub estimated_exec_ns: u64,
    /// Memory to be freed (bytes)
    pub memory_bytes: u64,
    /// Callback function name/identifier
    pub function_name: String,
}

/// Callback priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallbackPriority {
    /// Background priority
    Background = 0,
    /// Normal priority
    Normal     = 1,
    /// High priority
    High       = 2,
    /// Critical priority
    Critical   = 3,
}

/// Coalesced callback batch
#[derive(Debug, Clone)]
pub struct CallbackBatch {
    /// Batch ID
    pub batch_id: u64,
    /// Callbacks in batch
    pub callbacks: Vec<CallbackId>,
    /// Target grace period
    pub target_gp: GracePeriodId,
    /// Total memory to free
    pub total_memory_bytes: u64,
    /// Total estimated execution time
    pub total_exec_ns: u64,
    /// Highest priority in batch
    pub max_priority: CallbackPriority,
}

/// Callback coalescer for batching RCU callbacks
pub struct CallbackCoalescer {
    /// Pending callbacks
    pending: BTreeMap<CallbackId, CallbackInfo>,
    /// Current batch
    current_batch: Vec<CallbackId>,
    /// Batch size threshold
    batch_threshold: usize,
    /// Time threshold for coalescing (nanoseconds)
    time_threshold_ns: u64,
    /// Memory threshold for coalescing (bytes)
    memory_threshold_bytes: u64,
    /// Last batch timestamp
    last_batch_ns: u64,
    /// Total batches created
    total_batches: u64,
    /// Total callbacks batched
    total_callbacks_batched: u64,
    /// Next batch ID
    next_batch_id: AtomicU64,
}

impl CallbackCoalescer {
    /// Create new callback coalescer
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            current_batch: Vec::new(),
            batch_threshold: 32,
            time_threshold_ns: 1_000_000,        // 1ms
            memory_threshold_bytes: 1024 * 1024, // 1MB
            last_batch_ns: 0,
            total_batches: 0,
            total_callbacks_batched: 0,
            next_batch_id: AtomicU64::new(1),
        }
    }

    /// Add callback to be coalesced
    #[inline]
    pub fn add_callback(&mut self, callback: CallbackInfo) {
        let id = callback.id;
        self.pending.insert(id, callback);
        self.current_batch.push(id);
    }

    /// Check if batch should be flushed
    pub fn should_flush(&self, current_time_ns: u64) -> bool {
        if self.current_batch.len() >= self.batch_threshold {
            return true;
        }

        if current_time_ns - self.last_batch_ns >= self.time_threshold_ns {
            return !self.current_batch.is_empty();
        }

        // Check memory threshold
        let total_memory: u64 = self
            .current_batch
            .iter()
            .filter_map(|id| self.pending.get(id))
            .map(|c| c.memory_bytes)
            .sum();

        total_memory >= self.memory_threshold_bytes
    }

    /// Flush current batch
    pub fn flush_batch(
        &mut self,
        target_gp: GracePeriodId,
        current_time_ns: u64,
    ) -> Option<CallbackBatch> {
        if self.current_batch.is_empty() {
            return None;
        }

        let callbacks = core::mem::take(&mut self.current_batch);

        let mut total_memory = 0u64;
        let mut total_exec = 0u64;
        let mut max_priority = CallbackPriority::Background;

        for id in &callbacks {
            if let Some(cb) = self.pending.get(id) {
                total_memory += cb.memory_bytes;
                total_exec += cb.estimated_exec_ns;
                if cb.priority > max_priority {
                    max_priority = cb.priority;
                }
            }
        }

        self.total_batches += 1;
        self.total_callbacks_batched += callbacks.len() as u64;
        self.last_batch_ns = current_time_ns;

        let batch_id = self.next_batch_id.fetch_add(1, Ordering::Relaxed);

        Some(CallbackBatch {
            batch_id,
            callbacks,
            target_gp,
            total_memory_bytes: total_memory,
            total_exec_ns: total_exec,
            max_priority,
        })
    }

    /// Process completed callbacks
    #[inline]
    pub fn remove_callbacks(&mut self, ids: &[CallbackId]) {
        for id in ids {
            self.pending.remove(id);
        }
    }

    /// Set batch threshold
    #[inline(always)]
    pub fn set_batch_threshold(&mut self, threshold: usize) {
        self.batch_threshold = threshold;
    }

    /// Set time threshold
    #[inline(always)]
    pub fn set_time_threshold(&mut self, threshold_ns: u64) {
        self.time_threshold_ns = threshold_ns;
    }

    /// Get pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get current batch size
    #[inline(always)]
    pub fn current_batch_size(&self) -> usize {
        self.current_batch.len()
    }

    /// Get coalescing ratio
    #[inline]
    pub fn coalescing_ratio(&self) -> f32 {
        if self.total_batches == 0 {
            return 1.0;
        }
        self.total_callbacks_batched as f32 / self.total_batches as f32
    }
}

impl Default for CallbackCoalescer {
    fn default() -> Self {
        Self::new()
    }
}
