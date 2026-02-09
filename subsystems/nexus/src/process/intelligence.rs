//! Process Intelligence
//!
//! Central coordinator for process analysis.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    BehaviorEvent, KillRecommendation, LifecycleManager, ProcessBehaviorAnalyzer, ProcessId,
    ProcessMetrics, ProcessState, PriorityOptimizer, ResourcePrediction, ResourcePredictor,
};

/// Central process intelligence coordinator
pub struct ProcessIntelligence {
    /// Behavior analyzer
    behavior: ProcessBehaviorAnalyzer,
    /// Resource predictor
    resource: ResourcePredictor,
    /// Priority optimizer
    priority: PriorityOptimizer,
    /// Lifecycle manager
    lifecycle: LifecycleManager,
    /// Total processes tracked
    total_tracked: AtomicU64,
}

impl ProcessIntelligence {
    /// Create new process intelligence
    pub fn new() -> Self {
        Self {
            behavior: ProcessBehaviorAnalyzer::default(),
            resource: ResourcePredictor::default(),
            priority: PriorityOptimizer::default(),
            lifecycle: LifecycleManager::default(),
            total_tracked: AtomicU64::new(0),
        }
    }

    /// Process started
    #[inline]
    pub fn process_started(&mut self, pid: ProcessId, name: &str) {
        self.behavior.record_start(pid, name);
        self.lifecycle.process_created(pid);
        self.total_tracked.fetch_add(1, Ordering::Relaxed);
    }

    /// Process exited
    #[inline(always)]
    pub fn process_exited(&mut self, pid: ProcessId, exit_code: i32) {
        self.behavior.record_exit(pid, exit_code);
        self.lifecycle.update_state(pid, ProcessState::Zombie);
    }

    /// Record metrics
    #[inline(always)]
    pub fn record_metrics(&mut self, metrics: ProcessMetrics) -> Option<BehaviorEvent> {
        self.behavior.record_metrics(metrics)
    }

    /// Get resource prediction
    #[inline(always)]
    pub fn predict_resources(&mut self, pid: ProcessId) -> Option<ResourcePrediction> {
        let profile = self.behavior.get_profile(pid)?;
        Some(self.resource.predict(profile))
    }

    /// Suggest priority
    #[inline]
    pub fn suggest_priority(&mut self, pid: ProcessId, current: i8) -> i8 {
        if let Some(profile) = self.behavior.get_profile(pid) {
            self.priority.suggest_priority(profile, current)
        } else {
            current
        }
    }

    /// Find kill candidates
    #[inline]
    pub fn find_kill_candidates(&mut self, memory_needed: u64) -> Vec<KillRecommendation> {
        let profiles: BTreeMap<_, _> = self
            .behavior
            .all_profiles()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        self.lifecycle.find_kill_candidates(memory_needed, &profiles)
    }

    /// Get behavior analyzer
    #[inline(always)]
    pub fn behavior(&self) -> &ProcessBehaviorAnalyzer {
        &self.behavior
    }

    /// Get lifecycle manager
    #[inline(always)]
    pub fn lifecycle(&self) -> &LifecycleManager {
        &self.lifecycle
    }

    /// Get mutable lifecycle manager
    #[inline(always)]
    pub fn lifecycle_mut(&mut self) -> &mut LifecycleManager {
        &mut self.lifecycle
    }

    /// Cleanup
    #[inline(always)]
    pub fn cleanup(&mut self) {
        self.lifecycle.cleanup();
    }
}

impl Default for ProcessIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
