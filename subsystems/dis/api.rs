//! # DIS Public API
//!
//! This module provides the public API for interacting with the Dynamic Intent
//! Scheduling system. It offers a clean, ergonomic interface for creating tasks,
//! managing intents, and controlling scheduler behavior.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use dis::{DIS, Intent, IntentClass};
//!
//! // Create a real-time audio task
//! let task = DIS::create_task("audio_processor")
//!     .with_intent(Intent::realtime()
//!         .latency_target(Latency::UltraLow)
//!         .cpu_budget(CpuBudget::Full)
//!         .build())
//!     .spawn()?;
//!
//! // Create an interactive UI task
//! let ui_task = DIS::create_task("ui_renderer")
//!     .with_intent(Intent::interactive()
//!         .build())
//!     .spawn()?;
//!
//! // Create a batch processing task
//! let batch_task = DIS::create_task("data_processor")
//!     .with_intent(Intent::batch()
//!         .cpu_budget(CpuBudget::Unlimited)
//!         .build())
//!     .spawn()?;
//! ```
//!
//! ## Intent Templates
//!
//! The API provides convenient templates for common use cases:
//!
//! - `Intent::realtime()` - For audio, video, real-time control
//! - `Intent::interactive()` - For UI, games, user-facing apps
//! - `Intent::normal()` - For general workloads
//! - `Intent::batch()` - For background processing
//! - `Intent::daemon()` - For system services
//!
//! ## Policy API
//!
//! Dynamic policies can be added and removed at runtime:
//!
//! ```rust,ignore
//! // Add a policy to boost interactive tasks under load
//! DIS::add_policy(Policy::new("boost_interactive")
//!     .when(System::CpuLoad > 80)
//!     .and(Task::IsInteractive)
//!     .then(Action::BoostPriority(5))
//!     .build_unchecked());
//! ```

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::RwLock;

use super::executor::Executor;
use super::intent::{Intent, IntentBuilder, IntentClass};
use super::isolation::{Capability, CapabilitySet, DomainId, DomainType, SecurityManager};
use super::optimizer::OptimizationHint;
use super::policy::{Policy, PolicyBuilder, PolicyId};
use super::queues::QueueManager;
use super::scheduler::{DISScheduler, SchedulerConfig, SchedulingDecision};
use super::stats::{StatsSummary, SystemStats, TaskStats};
use super::{CpuId, DISResult, Nanoseconds, Task, TaskId, TaskState};

// =============================================================================
// DIS Handle
// =============================================================================

/// Handle to a DIS task
#[derive(Debug, Clone)]
pub struct TaskHandle {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
    /// Intent class
    pub intent_class: IntentClass,
}

impl TaskHandle {
    /// Create new handle
    pub fn new(id: TaskId, name: &str, intent_class: IntentClass) -> Self {
        Self {
            id,
            name: name.to_string(),
            intent_class,
        }
    }

    /// Get task ID
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get task name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get intent class
    pub fn class(&self) -> IntentClass {
        self.intent_class
    }
}

// =============================================================================
// Task Builder
// =============================================================================

/// Builder for creating DIS tasks
pub struct TaskBuilder<'a> {
    dis: &'a DIS,
    name: String,
    intent: Option<Intent>,
    domain: Option<DomainId>,
    cpu_affinity: Option<Vec<CpuId>>,
    priority: Option<i8>,
    time_slice: Option<Nanoseconds>,
    capabilities: Option<CapabilitySet>,
}

impl<'a> TaskBuilder<'a> {
    /// Create new task builder
    pub fn new(dis: &'a DIS, name: &str) -> Self {
        Self {
            dis,
            name: name.to_string(),
            intent: None,
            domain: None,
            cpu_affinity: None,
            priority: None,
            time_slice: None,
            capabilities: None,
        }
    }

    /// Set task intent
    pub fn with_intent(mut self, intent: Intent) -> Self {
        self.intent = Some(intent);
        self
    }

    /// Set security domain
    pub fn in_domain(mut self, domain: DomainId) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Set CPU affinity
    pub fn on_cpus(mut self, cpus: Vec<CpuId>) -> Self {
        self.cpu_affinity = Some(cpus);
        self
    }

    /// Set initial priority
    pub fn priority(mut self, priority: i8) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set time slice
    pub fn time_slice(mut self, slice: Nanoseconds) -> Self {
        self.time_slice = Some(slice);
        self
    }

    /// Set capabilities
    pub fn capabilities(mut self, caps: CapabilitySet) -> Self {
        self.capabilities = Some(caps);
        self
    }

    /// Create as real-time task
    pub fn realtime(mut self) -> Self {
        self.intent = Some(
            IntentBuilder::new()
            .class(IntentClass::RealTime)
            .latency_ms(1)  // Ultra-low latency: 1ms
            .build_unchecked(),
        );
        self
    }

    /// Create as interactive task
    pub fn interactive(mut self) -> Self {
        self.intent = Some(
            IntentBuilder::new()
            .class(IntentClass::Interactive)
            .latency_ms(16)  // Low latency: 16ms (60fps)
            .build_unchecked(),
        );
        self
    }

    /// Create as batch task
    pub fn batch(mut self) -> Self {
        self.intent = Some(
            IntentBuilder::new()
                .class(IntentClass::Batch)
                .build_unchecked(),
        );
        self
    }

    /// Create as background task
    pub fn background(mut self) -> Self {
        self.intent = Some(
            IntentBuilder::new()
                .class(IntentClass::Background)
                .build_unchecked(),
        );
        self
    }

    /// Spawn the task
    pub fn spawn(self) -> DISResult<TaskHandle> {
        let intent = self.intent.unwrap_or_else(|| {
            IntentBuilder::new()
                .class(IntentClass::Interactive)
                .build_unchecked()
        });

        let intent_class = intent.class;
        let task_id = self.dis.create_task_internal(&self.name, intent)?;

        // Apply optional settings
        if let Some(priority) = self.priority {
            self.dis.set_priority(task_id, priority)?;
        }

        if let Some(slice) = self.time_slice {
            self.dis.set_time_slice(task_id, slice)?;
        }

        if let Some(cpus) = self.cpu_affinity {
            self.dis.set_cpu_affinity(task_id, cpus)?;
        }

        if let Some(domain) = self.domain {
            self.dis.set_domain(task_id, domain)?;
        }

        Ok(TaskHandle::new(task_id, &self.name, intent_class))
    }
}

// =============================================================================
// DIS Main Interface
// =============================================================================

/// Dynamic Intent Scheduling main interface
pub struct DIS {
    /// The scheduler
    scheduler: RwLock<DISScheduler>,
    /// Security manager
    security: RwLock<SecurityManager>,
    /// Queue manager
    queues: RwLock<QueueManager>,
    /// Executor
    executor: RwLock<Executor>,
    /// Initialized flag
    initialized: AtomicBool,
}

impl DIS {
    /// Create new DIS instance
    pub fn new() -> Self {
        Self {
            scheduler: RwLock::new(DISScheduler::default()),
            security: RwLock::new(SecurityManager::new()),
            queues: RwLock::new(QueueManager::default()),
            executor: RwLock::new(Executor::new()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SchedulerConfig) -> Self {
        Self {
            scheduler: RwLock::new(DISScheduler::new(config)),
            security: RwLock::new(SecurityManager::new()),
            queues: RwLock::new(QueueManager::default()),
            executor: RwLock::new(Executor::new()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize DIS
    pub fn init(&self) -> DISResult<()> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already initialized
        }

        // Additional initialization if needed
        Ok(())
    }

    // =========================================================================
    // Task Management
    // =========================================================================

    /// Create task builder
    pub fn create_task(&self, name: &str) -> TaskBuilder<'_> {
        TaskBuilder::new(self, name)
    }

    /// Create task with intent (internal)
    fn create_task_internal(&self, name: &str, intent: Intent) -> DISResult<TaskId> {
        self.scheduler.read().create_task(name, intent)
    }

    /// Destroy task
    pub fn destroy_task(&self, task_id: TaskId) -> DISResult<()> {
        self.scheduler.read().destroy_task(task_id)
    }

    /// Get task
    pub fn get_task(&self, task_id: TaskId) -> Option<Task> {
        self.scheduler.read().get_task(task_id)
    }

    /// Set task state
    pub fn set_state(&self, task_id: TaskId, state: TaskState) -> DISResult<()> {
        self.scheduler.read().set_task_state(task_id, state)
    }

    /// Set task priority
    pub fn set_priority(&self, task_id: TaskId, priority: i8) -> DISResult<()> {
        // Would update task priority
        let _ = (task_id, priority);
        Ok(())
    }

    /// Set time slice
    pub fn set_time_slice(&self, task_id: TaskId, slice: Nanoseconds) -> DISResult<()> {
        self.executor.read().reset_time_slice(task_id, slice)
    }

    /// Set CPU affinity
    pub fn set_cpu_affinity(&self, task_id: TaskId, cpus: Vec<CpuId>) -> DISResult<()> {
        let _ = (task_id, cpus);
        Ok(())
    }

    /// Set security domain
    pub fn set_domain(&self, task_id: TaskId, domain: DomainId) -> DISResult<()> {
        let _ = (task_id, domain);
        Ok(())
    }

    /// Yield current task
    pub fn yield_now(&self) {
        // Would trigger rescheduling
    }

    /// Sleep for duration
    pub fn sleep(&self, duration: Nanoseconds) {
        let _ = duration;
        // Would block current task for duration
    }

    // =========================================================================
    // Intent Management
    // =========================================================================

    /// Update task intent
    pub fn update_intent(&self, task_id: TaskId, intent: Intent) -> DISResult<()> {
        let scheduler = self.scheduler.read();
        let engine = scheduler.intent_engine().write();
        engine.register(intent)?;
        Ok(())
    }

    /// Get intent engine
    pub fn intent_engine(&self) -> IntentEngineHandle<'_> {
        IntentEngineHandle { dis: self }
    }

    // =========================================================================
    // Policy Management
    // =========================================================================

    /// Add scheduling policy
    pub fn add_policy(&self, policy: Policy) -> PolicyId {
        self.scheduler
            .read()
            .policy_engine()
            .read()
            .register(policy)
    }

    /// Remove policy
    pub fn remove_policy(&self, policy_id: PolicyId) -> bool {
        self.scheduler
            .read()
            .policy_engine()
            .read()
            .unregister(policy_id)
            .is_some()
    }

    /// Get policy engine handle
    pub fn policy_engine(&self) -> PolicyEngineHandle<'_> {
        PolicyEngineHandle { dis: self }
    }

    // =========================================================================
    // Security Management
    // =========================================================================

    /// Create security domain
    pub fn create_domain(
        &self,
        name: &str,
        domain_type: DomainType,
        parent: DomainId,
    ) -> DISResult<DomainId> {
        self.security
            .read()
            .create_domain(name, domain_type, parent)
    }

    /// Check capability
    pub fn check_capability(&self, task_id: TaskId, cap: Capability) -> bool {
        self.security.read().check_cap(task_id, cap)
    }

    /// Grant capability
    pub fn grant_capability(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        self.security.read().grant_cap(task_id, cap)
    }

    /// Revoke capability
    pub fn revoke_capability(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        self.security.read().revoke_cap(task_id, cap)
    }

    /// Get security manager handle
    pub fn security(&self) -> SecurityHandle<'_> {
        SecurityHandle { dis: self }
    }

    // =========================================================================
    // Scheduling
    // =========================================================================

    /// Schedule on CPU
    pub fn schedule(&self, cpu: CpuId) -> SchedulingDecision {
        self.scheduler.read().schedule(cpu)
    }

    /// Timer tick
    pub fn tick(&self, cpu: CpuId) {
        self.scheduler.read().tick(cpu)
    }

    /// Request reschedule
    pub fn resched(&self, cpu: CpuId) {
        let _ = cpu;
        // Would set need_resched flag
    }

    // =========================================================================
    // Statistics and Monitoring
    // =========================================================================

    /// Get task statistics
    pub fn task_stats(&self, task_id: TaskId) -> Option<TaskStats> {
        self.scheduler
            .read()
            .stats_collector()
            .read()
            .get_task_stats(task_id)
    }

    /// Get system statistics
    pub fn system_stats(&self) -> SystemStats {
        self.scheduler
            .read()
            .stats_collector()
            .read()
            .get_system_stats()
    }

    /// Get scheduler statistics
    pub fn scheduler_stats(&self) -> super::scheduler::DISStatistics {
        self.scheduler.read().statistics()
    }

    /// Get stats summary
    pub fn stats_summary(&self) -> StatsSummary {
        self.scheduler.read().stats_collector().read().summary()
    }

    /// Get optimizer statistics
    pub fn optimizer_stats(&self) -> super::optimizer::OptimizerStatistics {
        self.scheduler.read().optimizer().read().statistics()
    }

    // =========================================================================
    // Configuration
    // =========================================================================

    /// Get configuration
    pub fn config(&self) -> SchedulerConfig {
        self.scheduler.read().config()
    }

    /// Update configuration
    pub fn set_config(&self, config: SchedulerConfig) {
        self.scheduler.read().set_config(config)
    }

    /// Enable/disable adaptive optimization
    pub fn set_adaptive_optimization(&self, enabled: bool) {
        let mut config = self.config();
        config.adaptive_optimization = enabled;
        self.set_config(config);
    }

    /// Enable/disable load balancing
    pub fn set_load_balancing(&self, enabled: bool) {
        let mut config = self.config();
        config.load_balancing = enabled;
        self.set_config(config);
    }
}

impl Default for DIS {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Sub-handles for ergonomic access
// =============================================================================

/// Handle to intent engine
pub struct IntentEngineHandle<'a> {
    dis: &'a DIS,
}

impl<'a> IntentEngineHandle<'a> {
    /// Create intent builder
    pub fn build(&self) -> IntentBuilder {
        IntentBuilder::new()
    }

    /// Register intent
    pub fn register(&self, intent: Intent) -> DISResult<()> {
        self.dis
            .scheduler
            .read()
            .intent_engine()
            .write()
            .register(intent)?;
        Ok(())
    }

    /// Get intent templates
    pub fn templates(&self) -> IntentTemplates {
        IntentTemplates
    }
}

/// Intent templates
pub struct IntentTemplates;

impl IntentTemplates {
    /// Real-time audio processing
    pub fn audio(&self) -> Intent {
        super::intent::templates::audio()
    }

    /// Video playback at 60 FPS
    pub fn video_60fps(&self) -> Intent {
        super::intent::templates::video_60fps()
    }

    /// UI rendering
    pub fn ui(&self) -> Intent {
        super::intent::templates::ui()
    }

    /// Web server
    pub fn webserver(&self) -> Intent {
        super::intent::templates::webserver()
    }

    /// Database
    pub fn database(&self) -> Intent {
        super::intent::templates::database()
    }

    /// Batch processing
    pub fn batch(&self) -> Intent {
        super::intent::templates::batch()
    }

    /// Background task
    pub fn background(&self) -> Intent {
        super::intent::templates::background()
    }
}

/// Handle to policy engine
pub struct PolicyEngineHandle<'a> {
    dis: &'a DIS,
}

impl<'a> PolicyEngineHandle<'a> {
    /// Create policy builder
    pub fn build(&self, name: &str) -> PolicyBuilder {
        PolicyBuilder::new(name)
    }

    /// Add policy
    pub fn add(&self, policy: Policy) -> PolicyId {
        self.dis.add_policy(policy)
    }

    /// Remove policy
    pub fn remove(&self, id: PolicyId) -> bool {
        self.dis.remove_policy(id)
    }

    /// Get policy templates
    pub fn templates(&self) -> PolicyTemplates {
        PolicyTemplates
    }
}

/// Policy templates
pub struct PolicyTemplates;

impl PolicyTemplates {
    /// Real-time policy
    pub fn realtime(&self) -> Policy {
        super::policy::templates::realtime()
    }

    /// Interactive policy
    pub fn interactive(&self) -> Policy {
        super::policy::templates::interactive()
    }

    /// Server policy
    pub fn server(&self) -> Policy {
        super::policy::templates::server()
    }

    /// Batch policy
    pub fn batch(&self) -> Policy {
        super::policy::templates::batch()
    }
}

/// Handle to security manager
pub struct SecurityHandle<'a> {
    dis: &'a DIS,
}

impl<'a> SecurityHandle<'a> {
    /// Create domain
    pub fn create_domain(
        &self,
        name: &str,
        domain_type: DomainType,
        parent: DomainId,
    ) -> DISResult<DomainId> {
        self.dis.create_domain(name, domain_type, parent)
    }

    /// Check capability
    pub fn check(&self, task_id: TaskId, cap: Capability) -> bool {
        self.dis.check_capability(task_id, cap)
    }

    /// Grant capability
    pub fn grant(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        self.dis.grant_capability(task_id, cap)
    }

    /// Revoke capability
    pub fn revoke(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        self.dis.revoke_capability(task_id, cap)
    }
}

// =============================================================================
// Global DIS Instance
// =============================================================================

/// Global DIS instance
static DIS_INSTANCE: RwLock<Option<DIS>> = RwLock::new(None);

/// Initialize global DIS instance
pub fn init() -> DISResult<()> {
    let mut instance = DIS_INSTANCE.write();
    if instance.is_none() {
        let dis = DIS::new();
        dis.init()?;
        *instance = Some(dis);
    }
    Ok(())
}

/// Initialize with custom config
pub fn init_with_config(config: SchedulerConfig) -> DISResult<()> {
    let mut instance = DIS_INSTANCE.write();
    if instance.is_none() {
        let dis = DIS::with_config(config);
        dis.init()?;
        *instance = Some(dis);
    }
    Ok(())
}

/// Get global DIS instance
pub fn get() -> &'static DIS {
    // Note: This is a simplified version. In real code,
    // would need proper static initialization.
    unsafe {
        let ptr = DIS_INSTANCE.read().as_ref().unwrap() as *const DIS;
        &*ptr
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Create a task using the global DIS instance
pub fn create_task(name: &str) -> TaskBuilder<'static> {
    get().create_task(name)
}

/// Destroy a task
pub fn destroy_task(task_id: TaskId) -> DISResult<()> {
    get().destroy_task(task_id)
}

/// Add a policy
pub fn add_policy(policy: Policy) -> PolicyId {
    get().add_policy(policy)
}

/// Remove a policy
pub fn remove_policy(id: PolicyId) -> bool {
    get().remove_policy(id)
}

/// Yield current task
pub fn yield_now() {
    get().yield_now()
}

/// Sleep for duration
pub fn sleep(duration: Nanoseconds) {
    get().sleep(duration)
}

/// Get task statistics
pub fn task_stats(task_id: TaskId) -> Option<TaskStats> {
    get().task_stats(task_id)
}

/// Get system statistics
pub fn system_stats() -> SystemStats {
    get().system_stats()
}

// =============================================================================
// Events
// =============================================================================

/// DIS event types
#[derive(Debug, Clone)]
pub enum DISEvent {
    /// Task created
    TaskCreated(TaskId, String),
    /// Task destroyed
    TaskDestroyed(TaskId),
    /// Task state changed
    TaskStateChanged(TaskId, TaskState, TaskState),
    /// Policy added
    PolicyAdded(PolicyId),
    /// Policy removed
    PolicyRemoved(PolicyId),
    /// Optimization applied
    OptimizationApplied(TaskId, OptimizationHint),
    /// Security violation
    SecurityViolation(TaskId, Capability),
}

/// Event listener trait
pub trait DISEventListener: Send + Sync {
    fn on_event(&self, event: &DISEvent);
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dis_creation() {
        let dis = DIS::new();
        dis.init().unwrap();
    }

    #[test]
    fn test_task_builder() {
        let dis = DIS::new();
        dis.init().unwrap();

        let handle = dis
            .create_task("test_task")
            .interactive()
            .priority(50)
            .spawn()
            .unwrap();

        assert_eq!(handle.name(), "test_task");
        assert_eq!(handle.class(), IntentClass::Interactive);
    }

    #[test]
    fn test_policy_management() {
        let dis = DIS::new();
        dis.init().unwrap();

        let policy = dis.policy_engine().templates().interactive();
        let id = dis.add_policy(policy);

        assert!(dis.remove_policy(id));
    }
}
