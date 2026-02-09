//! Virtualization Intelligence
//!
//! Central coordinator for all virtualization intelligence.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    ContainerInfo, ContainerIntelligence, IsolationAnalyzer, MetricsSeries, MigrationOptimizer,
    VirtId, VirtMetrics, VirtResourceScheduler, VmInfo, VmIntelligence, WorkloadInfo,
};

/// Central virtualization intelligence coordinator
pub struct VirtualizationIntelligence {
    /// All workloads
    workloads: BTreeMap<VirtId, WorkloadInfo>,
    /// Metrics per workload
    metrics: BTreeMap<VirtId, VirtMetrics>,
    /// Time series
    series: BTreeMap<VirtId, MetricsSeries>,
    /// VM intelligence
    vm: VmIntelligence,
    /// Container intelligence
    container: ContainerIntelligence,
    /// Migration optimizer
    migration: MigrationOptimizer,
    /// Resource scheduler
    scheduler: VirtResourceScheduler,
    /// Isolation analyzer
    isolation: IsolationAnalyzer,
    /// Total workloads
    total_workloads: AtomicU64,
}

impl VirtualizationIntelligence {
    /// Create new virtualization intelligence
    pub fn new() -> Self {
        Self {
            workloads: BTreeMap::new(),
            metrics: BTreeMap::new(),
            series: BTreeMap::new(),
            vm: VmIntelligence::default(),
            container: ContainerIntelligence::default(),
            migration: MigrationOptimizer::default(),
            scheduler: VirtResourceScheduler::default(),
            isolation: IsolationAnalyzer::default(),
            total_workloads: AtomicU64::new(0),
        }
    }

    /// Register workload
    #[inline]
    pub fn register(&mut self, info: WorkloadInfo) {
        self.metrics.insert(info.id, VirtMetrics::default());
        self.series.insert(info.id, MetricsSeries::default());
        self.workloads.insert(info.id, info);
        self.total_workloads.fetch_add(1, Ordering::Relaxed);
    }

    /// Register VM
    #[inline(always)]
    pub fn register_vm(&mut self, info: VmInfo) {
        self.register(info.base.clone());
        self.vm.register(info);
    }

    /// Register container
    #[inline(always)]
    pub fn register_container(&mut self, info: ContainerInfo) {
        self.register(info.base.clone());
        self.container.register(info);
    }

    /// Update metrics
    #[inline]
    pub fn update_metrics(&mut self, id: VirtId, metrics: VirtMetrics) {
        if let Some(series) = self.series.get_mut(&id) {
            series.add(&metrics);
        }
        self.metrics.insert(id, metrics);
    }

    /// Get workload
    #[inline(always)]
    pub fn get_workload(&self, id: VirtId) -> Option<&WorkloadInfo> {
        self.workloads.get(&id)
    }

    /// Get metrics
    #[inline(always)]
    pub fn get_metrics(&self, id: VirtId) -> Option<&VirtMetrics> {
        self.metrics.get(&id)
    }

    /// Get time series
    #[inline(always)]
    pub fn get_series(&self, id: VirtId) -> Option<&MetricsSeries> {
        self.series.get(&id)
    }

    /// Get VM intelligence
    #[inline(always)]
    pub fn vm(&self) -> &VmIntelligence {
        &self.vm
    }

    /// Get mutable VM intelligence
    #[inline(always)]
    pub fn vm_mut(&mut self) -> &mut VmIntelligence {
        &mut self.vm
    }

    /// Get container intelligence
    #[inline(always)]
    pub fn container(&self) -> &ContainerIntelligence {
        &self.container
    }

    /// Get mutable container intelligence
    #[inline(always)]
    pub fn container_mut(&mut self) -> &mut ContainerIntelligence {
        &mut self.container
    }

    /// Get migration optimizer
    #[inline(always)]
    pub fn migration(&self) -> &MigrationOptimizer {
        &self.migration
    }

    /// Get mutable migration optimizer
    #[inline(always)]
    pub fn migration_mut(&mut self) -> &mut MigrationOptimizer {
        &mut self.migration
    }

    /// Get scheduler
    #[inline(always)]
    pub fn scheduler(&self) -> &VirtResourceScheduler {
        &self.scheduler
    }

    /// Get mutable scheduler
    #[inline(always)]
    pub fn scheduler_mut(&mut self) -> &mut VirtResourceScheduler {
        &mut self.scheduler
    }

    /// Get isolation analyzer
    #[inline(always)]
    pub fn isolation(&self) -> &IsolationAnalyzer {
        &self.isolation
    }

    /// Get mutable isolation analyzer
    #[inline(always)]
    pub fn isolation_mut(&mut self) -> &mut IsolationAnalyzer {
        &mut self.isolation
    }

    /// Get constrained workloads
    #[inline]
    pub fn constrained_workloads(&self) -> Vec<VirtId> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.is_cpu_constrained() || m.is_memory_constrained())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get total workloads
    #[inline(always)]
    pub fn total_workloads(&self) -> u64 {
        self.total_workloads.load(Ordering::Relaxed)
    }

    /// Workload count
    #[inline(always)]
    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }
}

impl Default for VirtualizationIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
