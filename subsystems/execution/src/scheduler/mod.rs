//! # Scheduler Framework
//!
//! This module defines the scheduler FRAMEWORK, not a specific scheduler.
//! Actual scheduler implementations are provided as modules.

pub mod metrics;
pub mod priority;
pub mod queue;
pub mod traits;

use alloc::sync::Arc;

pub use priority::*;
use spin::RwLock;
pub use traits::*;

use crate::{ExecError, ExecResult, ThreadId};

/// Scheduler framework
///
/// This struct holds the current scheduler implementation
/// and provides a stable interface for the rest of the kernel.
pub struct SchedulerFramework {
    /// Current scheduler implementation
    scheduler: RwLock<Option<Arc<dyn Scheduler>>>,
    /// Scheduler metrics
    metrics: metrics::SchedulerMetrics,
    /// Load balancer (for SMP)
    load_balancer: RwLock<Option<Arc<dyn LoadBalancer>>>,
}

impl SchedulerFramework {
    /// Create a new scheduler framework
    pub const fn new() -> Self {
        Self {
            scheduler: RwLock::new(None),
            metrics: metrics::SchedulerMetrics::new(),
            load_balancer: RwLock::new(None),
        }
    }

    /// Set the scheduler implementation
    pub fn set_scheduler(&self, scheduler: Arc<dyn Scheduler>) {
        log::info!("Setting scheduler: {}", scheduler.name());
        *self.scheduler.write() = Some(scheduler);
    }

    /// Get the current scheduler
    pub fn scheduler(&self) -> Option<Arc<dyn Scheduler>> {
        self.scheduler.read().clone()
    }

    /// Set the load balancer
    pub fn set_load_balancer(&self, balancer: Arc<dyn LoadBalancer>) {
        *self.load_balancer.write() = Some(balancer);
    }

    /// Pick the next thread to run
    pub fn pick_next(&self, cpu: usize) -> Option<ThreadId> {
        let scheduler = self.scheduler.read();
        scheduler.as_ref()?.pick_next(cpu)
    }

    /// Add a thread to the scheduler
    pub fn add_thread(&self, thread: SchedulableThread) -> ExecResult<()> {
        let scheduler = self.scheduler.read();
        scheduler
            .as_ref()
            .ok_or(ExecError::Internal)?
            .add_thread(thread)
    }

    /// Remove a thread from the scheduler
    pub fn remove_thread(&self, id: ThreadId) -> ExecResult<()> {
        let scheduler = self.scheduler.read();
        scheduler
            .as_ref()
            .ok_or(ExecError::Internal)?
            .remove_thread(id)
    }

    /// Notify that a thread is ready to run
    pub fn thread_ready(&self, id: ThreadId) -> ExecResult<()> {
        let scheduler = self.scheduler.read();
        scheduler
            .as_ref()
            .ok_or(ExecError::Internal)?
            .thread_ready(id)
    }

    /// Notify that a thread is blocking
    pub fn thread_block(&self, id: ThreadId) -> ExecResult<()> {
        let scheduler = self.scheduler.read();
        scheduler
            .as_ref()
            .ok_or(ExecError::Internal)?
            .thread_block(id)
    }

    /// Timer tick notification
    pub fn tick(&self, cpu: usize) {
        if let Some(scheduler) = self.scheduler.read().as_ref() {
            scheduler.tick(cpu);
        }
        self.metrics.record_tick();
    }

    /// Yield the current thread
    pub fn yield_current(&self, cpu: usize) {
        if let Some(scheduler) = self.scheduler.read().as_ref() {
            scheduler.yield_thread(cpu);
        }
    }

    /// Update thread priority
    pub fn set_priority(&self, id: ThreadId, priority: Priority) -> ExecResult<()> {
        let scheduler = self.scheduler.read();
        scheduler
            .as_ref()
            .ok_or(ExecError::Internal)?
            .set_priority(id, priority)
    }

    /// Get scheduler metrics
    pub fn metrics(&self) -> &metrics::SchedulerMetrics {
        &self.metrics
    }

    /// Trigger load balancing
    pub fn balance_load(&self) {
        if let Some(balancer) = self.load_balancer.read().as_ref() {
            if let Some(scheduler) = self.scheduler.read().as_ref() {
                balancer.balance(scheduler.as_ref());
            }
        }
    }
}

/// Global scheduler framework
static FRAMEWORK: SchedulerFramework = SchedulerFramework::new();

/// Get the scheduler framework
pub fn framework() -> &'static SchedulerFramework {
    &FRAMEWORK
}
