//! Central I/O intelligence coordinator.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::prefetch::PrefetchEngine;
use super::request::IoRequest;
use super::scheduler::IoScheduler;
use super::types::DeviceType;

// ============================================================================
// DEVICE INFO
// ============================================================================

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device ID
    pub id: u32,
    /// Device type
    pub device_type: DeviceType,
    /// Device name
    pub name: String,
    /// Capacity in bytes
    pub capacity: u64,
    /// Is removable
    pub removable: bool,
}

// ============================================================================
// I/O INTELLIGENCE
// ============================================================================

/// Central I/O intelligence coordinator
pub struct IoIntelligence {
    /// I/O scheduler
    scheduler: IoScheduler,
    /// Prefetch engine
    prefetch: PrefetchEngine,
    /// Device registry
    devices: BTreeMap<u32, DeviceInfo>,
    /// Total I/O operations
    total_ops: AtomicU64,
}

impl IoIntelligence {
    /// Create new I/O intelligence
    pub fn new() -> Self {
        Self {
            scheduler: IoScheduler::default(),
            prefetch: PrefetchEngine::new(),
            devices: BTreeMap::new(),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Register device
    #[inline]
    pub fn register_device(&mut self, info: DeviceInfo) {
        self.scheduler
            .latency_predictor_mut()
            .register_device(info.id, info.device_type);
        self.devices.insert(info.id, info);
    }

    /// Submit I/O request
    pub fn submit(&mut self, request: IoRequest) -> Vec<(u64, u32)> {
        self.total_ops.fetch_add(1, Ordering::Relaxed);

        // Check for prefetch opportunities
        let prefetches = self.prefetch.record_access(
            request.device_id,
            request.process_id,
            request.offset,
            request.size,
            request.is_read(),
        );

        // Submit to scheduler
        self.scheduler.submit(request);

        prefetches
    }

    /// Dispatch next request
    #[inline(always)]
    pub fn dispatch(&mut self, device_id: u32) -> Option<IoRequest> {
        self.scheduler.dispatch(device_id)
    }

    /// Complete request
    #[inline(always)]
    pub fn complete(&mut self, request: &IoRequest) {
        self.scheduler.complete(request);
    }

    /// Get scheduler
    #[inline(always)]
    pub fn scheduler(&self) -> &IoScheduler {
        &self.scheduler
    }

    /// Get mutable scheduler
    #[inline(always)]
    pub fn scheduler_mut(&mut self) -> &mut IoScheduler {
        &mut self.scheduler
    }

    /// Get prefetch engine
    #[inline(always)]
    pub fn prefetch(&self) -> &PrefetchEngine {
        &self.prefetch
    }

    /// Get mutable prefetch engine
    #[inline(always)]
    pub fn prefetch_mut(&mut self) -> &mut PrefetchEngine {
        &mut self.prefetch
    }

    /// Get total operations
    #[inline(always)]
    pub fn total_operations(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }
}

impl Default for IoIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
