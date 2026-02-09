//! Seccomp Manager
//!
//! Filter management and process attachment.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Architecture, FilterAction, FilterId, Pid, SeccompFilter};

/// Seccomp manager
pub struct SeccompManager {
    /// Filters
    filters: BTreeMap<FilterId, SeccompFilter>,
    /// Process filters
    process_filters: BTreeMap<Pid, FilterId>,
    /// Next filter ID
    next_filter_id: AtomicU64,
    /// Total filters created
    total_created: AtomicU64,
    /// Total violations
    total_violations: AtomicU64,
}

impl SeccompManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            filters: BTreeMap::new(),
            process_filters: BTreeMap::new(),
            next_filter_id: AtomicU64::new(1),
            total_created: AtomicU64::new(0),
            total_violations: AtomicU64::new(0),
        }
    }

    /// Create filter
    #[inline]
    pub fn create_filter(
        &mut self,
        arch: Architecture,
        default_action: FilterAction,
        timestamp: u64,
    ) -> FilterId {
        let id = FilterId::new(self.next_filter_id.fetch_add(1, Ordering::Relaxed));
        let filter = SeccompFilter::new(id, arch, default_action, timestamp);
        self.filters.insert(id, filter);
        self.total_created.fetch_add(1, Ordering::Relaxed);
        id
    }

    /// Get filter
    #[inline(always)]
    pub fn get_filter(&self, id: FilterId) -> Option<&SeccompFilter> {
        self.filters.get(&id)
    }

    /// Get filter mutably
    #[inline(always)]
    pub fn get_filter_mut(&mut self, id: FilterId) -> Option<&mut SeccompFilter> {
        self.filters.get_mut(&id)
    }

    /// Delete filter
    #[inline(always)]
    pub fn delete_filter(&mut self, id: FilterId) -> bool {
        self.filters.remove(&id).is_some()
    }

    /// Attach filter to process
    #[inline]
    pub fn attach(&mut self, filter_id: FilterId, pid: Pid) -> bool {
        if let Some(filter) = self.filters.get_mut(&filter_id) {
            filter.attached_pid = Some(pid);
            filter.activate();
            self.process_filters.insert(pid, filter_id);
            true
        } else {
            false
        }
    }

    /// Detach filter from process
    #[inline]
    pub fn detach(&mut self, pid: Pid) -> Option<FilterId> {
        if let Some(filter_id) = self.process_filters.remove(&pid) {
            if let Some(filter) = self.filters.get_mut(&filter_id) {
                filter.attached_pid = None;
                filter.deactivate();
            }
            Some(filter_id)
        } else {
            None
        }
    }

    /// Get process filter
    #[inline]
    pub fn get_process_filter(&self, pid: Pid) -> Option<&SeccompFilter> {
        self.process_filters
            .get(&pid)
            .and_then(|id| self.filters.get(id))
    }

    /// Record violation
    #[inline(always)]
    pub fn record_violation(&self, _filter_id: FilterId) {
        self.total_violations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get filter count
    #[inline(always)]
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    /// Get total filters created
    #[inline(always)]
    pub fn total_created(&self) -> u64 {
        self.total_created.load(Ordering::Relaxed)
    }

    /// Get total violations
    #[inline(always)]
    pub fn total_violations(&self) -> u64 {
        self.total_violations.load(Ordering::Relaxed)
    }

    /// List all filters
    #[inline(always)]
    pub fn list_filters(&self) -> impl Iterator<Item = &SeccompFilter> {
        self.filters.values()
    }
}

impl Default for SeccompManager {
    fn default() -> Self {
        Self::new()
    }
}
