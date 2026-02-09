//! # Cognitive Lifecycle Management
//!
//! Manages the lifecycle of cognitive components.
//! Handles initialization, startup, shutdown, and cleanup.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// LIFECYCLE TYPES
// ============================================================================

/// Component lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Not yet created
    Uninitialized,
    /// Being created
    Initializing,
    /// Created but not started
    Initialized,
    /// Starting up
    Starting,
    /// Running normally
    Running,
    /// Paused
    Paused,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Error state
    Failed,
    /// Being destroyed
    Destroying,
    /// Destroyed
    Destroyed,
}

impl LifecycleState {
    /// Check if operational
    #[inline(always)]
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }

    /// Check if terminal
    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed | Self::Destroyed)
    }

    /// Get valid transitions
    pub fn valid_transitions(&self) -> &'static [LifecycleState] {
        match self {
            Self::Uninitialized => &[Self::Initializing],
            Self::Initializing => &[Self::Initialized, Self::Failed],
            Self::Initialized => &[Self::Starting, Self::Destroying],
            Self::Starting => &[Self::Running, Self::Failed],
            Self::Running => &[Self::Paused, Self::Stopping, Self::Failed],
            Self::Paused => &[Self::Running, Self::Stopping],
            Self::Stopping => &[Self::Stopped, Self::Failed],
            Self::Stopped => &[Self::Starting, Self::Destroying],
            Self::Failed => &[Self::Destroying, Self::Starting], // Recovery possible
            Self::Destroying => &[Self::Destroyed],
            Self::Destroyed => &[],
        }
    }

    /// Check if transition is valid
    #[inline(always)]
    pub fn can_transition_to(&self, next: LifecycleState) -> bool {
        self.valid_transitions().contains(&next)
    }
}

/// Lifecycle event
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    /// Event ID
    pub id: u64,
    /// Component ID
    pub component_id: u64,
    /// Previous state
    pub from_state: LifecycleState,
    /// New state
    pub to_state: LifecycleState,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Reason
    pub reason: String,
    /// Duration in previous state (ns)
    pub duration_ns: u64,
}

/// Lifecycle hook
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleHook {
    /// Before state change
    PreTransition,
    /// After state change
    PostTransition,
    /// On initialization
    OnInit,
    /// On start
    OnStart,
    /// On stop
    OnStop,
    /// On destroy
    OnDestroy,
    /// On error
    OnError,
}

/// Component lifecycle info
#[derive(Debug, Clone)]
pub struct ComponentLifecycle {
    /// Component ID
    pub id: u64,
    /// Component name
    pub name: String,
    /// Owner domain
    pub owner: DomainId,
    /// Current state
    pub state: LifecycleState,
    /// State entry time
    pub state_since: Timestamp,
    /// Creation time
    pub created: Timestamp,
    /// Transition history
    pub history: VecDeque<LifecycleEvent>,
    /// Failure count
    pub failure_count: u32,
    /// Last error
    pub last_error: Option<String>,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Dependents
    pub dependents: Vec<u64>,
}

impl ComponentLifecycle {
    /// Get state duration
    #[inline(always)]
    pub fn state_duration(&self) -> u64 {
        Timestamp::now().elapsed_since(self.state_since)
    }

    /// Get uptime
    #[inline(always)]
    pub fn uptime(&self) -> u64 {
        Timestamp::now().elapsed_since(self.created)
    }

    /// Check if all dependencies are running
    #[inline]
    pub fn dependencies_satisfied(&self, manager: &LifecycleManager) -> bool {
        self.dependencies.iter().all(|dep| {
            manager.get_state(*dep)
                .map(|s| s == LifecycleState::Running)
                .unwrap_or(false)
        })
    }
}

// ============================================================================
// LIFECYCLE MANAGER
// ============================================================================

/// Manages component lifecycles
pub struct LifecycleManager {
    /// Components
    components: BTreeMap<u64, ComponentLifecycle>,
    /// Components by name
    by_name: BTreeMap<String, u64>,
    /// Next component ID
    next_id: AtomicU64,
    /// Next event ID
    next_event_id: AtomicU64,
    /// Registered hooks
    hooks: Vec<RegisteredHook>,
    /// Configuration
    config: LifecycleConfig,
    /// Statistics
    stats: LifecycleStats,
}

/// Registered hook
#[derive(Debug, Clone)]
struct RegisteredHook {
    /// Hook ID
    id: u64,
    /// Hook type
    hook_type: LifecycleHook,
    /// Target component (None = all)
    target: Option<u64>,
    /// Callback tag
    callback_tag: String,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Maximum history per component
    pub max_history: usize,
    /// Enable automatic recovery
    pub auto_recovery: bool,
    /// Maximum recovery attempts
    pub max_recovery_attempts: u32,
    /// Startup timeout (ns)
    pub startup_timeout_ns: u64,
    /// Shutdown timeout (ns)
    pub shutdown_timeout_ns: u64,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            auto_recovery: true,
            max_recovery_attempts: 3,
            startup_timeout_ns: 30_000_000_000,   // 30 seconds
            shutdown_timeout_ns: 10_000_000_000,  // 10 seconds
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct LifecycleStats {
    /// Total components created
    pub total_created: u64,
    /// Total transitions
    pub total_transitions: u64,
    /// Total failures
    pub total_failures: u64,
    /// Total recoveries
    pub total_recoveries: u64,
    /// Currently running
    pub running_count: u64,
    /// Currently failed
    pub failed_count: u64,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            components: BTreeMap::new(),
            by_name: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            next_event_id: AtomicU64::new(1),
            hooks: Vec::new(),
            config,
            stats: LifecycleStats::default(),
        }
    }

    /// Register a component
    pub fn register(
        &mut self,
        name: &str,
        owner: DomainId,
        dependencies: Vec<u64>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let component = ComponentLifecycle {
            id,
            name: name.into(),
            owner,
            state: LifecycleState::Uninitialized,
            state_since: now,
            created: now,
            history: VecDeque::new(),
            failure_count: 0,
            last_error: None,
            dependencies,
            dependents: Vec::new(),
        };

        // Update dependents
        for dep in &component.dependencies {
            if let Some(dep_comp) = self.components.get_mut(dep) {
                dep_comp.dependents.push(id);
            }
        }

        self.components.insert(id, component);
        self.by_name.insert(name.into(), id);
        self.stats.total_created += 1;

        id
    }

    /// Transition component state
    pub fn transition(
        &mut self,
        id: u64,
        to_state: LifecycleState,
        reason: &str,
    ) -> Result<(), &'static str> {
        let component = self.components.get_mut(&id)
            .ok_or("Component not found")?;

        if !component.state.can_transition_to(to_state) {
            return Err("Invalid state transition");
        }

        let from_state = component.state;
        let now = Timestamp::now();
        let duration = now.elapsed_since(component.state_since);

        // Create event
        let event = LifecycleEvent {
            id: self.next_event_id.fetch_add(1, Ordering::Relaxed),
            component_id: id,
            from_state,
            to_state,
            timestamp: now,
            reason: reason.into(),
            duration_ns: duration,
        };

        // Update history
        if component.history.len() >= self.config.max_history {
            component.history.pop_front().unwrap();
        }
        component.history.push(event);

        // Update state
        component.state = to_state;
        component.state_since = now;

        // Track failures
        if to_state == LifecycleState::Failed {
            component.failure_count += 1;
            component.last_error = Some(reason.into());
            self.stats.total_failures += 1;
        }

        self.stats.total_transitions += 1;
        self.update_counts();

        Ok(())
    }

    /// Initialize a component
    #[inline]
    pub fn initialize(&mut self, id: u64) -> Result<(), &'static str> {
        self.transition(id, LifecycleState::Initializing, "Starting initialization")?;
        // In real implementation, run initialization logic
        self.transition(id, LifecycleState::Initialized, "Initialization complete")
    }

    /// Start a component
    pub fn start(&mut self, id: u64) -> Result<(), &'static str> {
        // Check dependencies first
        let deps_ok = self.components.get(&id)
            .map(|c| c.dependencies_satisfied(self))
            .unwrap_or(false);

        if !deps_ok {
            return Err("Dependencies not satisfied");
        }

        self.transition(id, LifecycleState::Starting, "Starting component")?;
        // In real implementation, run startup logic
        self.transition(id, LifecycleState::Running, "Component started")
    }

    /// Stop a component
    pub fn stop(&mut self, id: u64) -> Result<(), &'static str> {
        // Check if any dependents are running
        let has_running_dependents = self.components.get(&id)
            .map(|c| {
                c.dependents.iter().any(|dep| {
                    self.components.get(dep)
                        .map(|d| d.state == LifecycleState::Running)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if has_running_dependents {
            return Err("Cannot stop: dependents still running");
        }

        self.transition(id, LifecycleState::Stopping, "Stopping component")?;
        // In real implementation, run shutdown logic
        self.transition(id, LifecycleState::Stopped, "Component stopped")
    }

    /// Pause a component
    #[inline(always)]
    pub fn pause(&mut self, id: u64) -> Result<(), &'static str> {
        self.transition(id, LifecycleState::Paused, "Component paused")
    }

    /// Resume a component
    #[inline(always)]
    pub fn resume(&mut self, id: u64) -> Result<(), &'static str> {
        self.transition(id, LifecycleState::Running, "Component resumed")
    }

    /// Fail a component
    #[inline(always)]
    pub fn fail(&mut self, id: u64, reason: &str) -> Result<(), &'static str> {
        self.transition(id, LifecycleState::Failed, reason)
    }

    /// Destroy a component
    pub fn destroy(&mut self, id: u64) -> Result<(), &'static str> {
        self.transition(id, LifecycleState::Destroying, "Destroying component")?;
        self.transition(id, LifecycleState::Destroyed, "Component destroyed")?;

        // Remove from tracking
        if let Some(component) = self.components.remove(&id) {
            self.by_name.remove(&component.name);

            // Remove from dependents lists
            for dep in &component.dependencies {
                if let Some(dep_comp) = self.components.get_mut(dep) {
                    dep_comp.dependents.retain(|&d| d != id);
                }
            }
        }

        Ok(())
    }

    /// Attempt recovery
    pub fn recover(&mut self, id: u64) -> Result<(), &'static str> {
        let component = self.components.get(&id)
            .ok_or("Component not found")?;

        if component.state != LifecycleState::Failed {
            return Err("Component not in failed state");
        }

        if component.failure_count >= self.config.max_recovery_attempts {
            return Err("Maximum recovery attempts exceeded");
        }

        self.stats.total_recoveries += 1;
        self.start(id)
    }

    /// Get component state
    #[inline(always)]
    pub fn get_state(&self, id: u64) -> Option<LifecycleState> {
        self.components.get(&id).map(|c| c.state)
    }

    /// Get component
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&ComponentLifecycle> {
        self.components.get(&id)
    }

    /// Get component by name
    #[inline(always)]
    pub fn get_by_name(&self, name: &str) -> Option<&ComponentLifecycle> {
        self.by_name.get(name).and_then(|id| self.components.get(id))
    }

    /// Get components in state
    #[inline]
    pub fn in_state(&self, state: LifecycleState) -> Vec<&ComponentLifecycle> {
        self.components.values()
            .filter(|c| c.state == state)
            .collect()
    }

    /// Get components by owner
    #[inline]
    pub fn by_owner(&self, owner: DomainId) -> Vec<&ComponentLifecycle> {
        self.components.values()
            .filter(|c| c.owner == owner)
            .collect()
    }

    /// Register a hook
    pub fn register_hook(
        &mut self,
        hook_type: LifecycleHook,
        target: Option<u64>,
        callback_tag: &str,
    ) -> u64 {
        let id = self.hooks.len() as u64 + 1;

        self.hooks.push(RegisteredHook {
            id,
            hook_type,
            target,
            callback_tag: callback_tag.into(),
        });

        id
    }

    /// Get startup order (topological sort)
    #[inline]
    pub fn startup_order(&self) -> Vec<u64> {
        let mut order = Vec::new();
        let mut visited = BTreeMap::new();

        for id in self.components.keys() {
            self.visit_for_order(*id, &mut visited, &mut order);
        }

        order
    }

    fn visit_for_order(
        &self,
        id: u64,
        visited: &mut BTreeMap<u64, bool>,
        order: &mut Vec<u64>,
    ) {
        if visited.get(&id) == Some(&true) {
            return;
        }

        visited.insert(id, true);

        if let Some(component) = self.components.get(&id) {
            for dep in &component.dependencies {
                self.visit_for_order(*dep, visited, order);
            }
        }

        order.push(id);
    }

    /// Get shutdown order (reverse of startup)
    #[inline]
    pub fn shutdown_order(&self) -> Vec<u64> {
        let mut order = self.startup_order();
        order.reverse();
        order
    }

    /// Update running/failed counts
    fn update_counts(&mut self) {
        self.stats.running_count = self.components.values()
            .filter(|c| c.state == LifecycleState::Running)
            .count() as u64;

        self.stats.failed_count = self.components.values()
            .filter(|c| c.state == LifecycleState::Failed)
            .count() as u64;
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &LifecycleStats {
        &self.stats
    }

    /// Get component count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.components.len()
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new(LifecycleConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(LifecycleState::Uninitialized.can_transition_to(LifecycleState::Initializing));
        assert!(LifecycleState::Running.can_transition_to(LifecycleState::Paused));
        assert!(!LifecycleState::Destroyed.can_transition_to(LifecycleState::Running));
    }

    #[test]
    fn test_component_lifecycle() {
        let mut manager = LifecycleManager::default();
        let domain = DomainId::new(1);

        let id = manager.register("test_component", domain, Vec::new());

        assert_eq!(manager.get_state(id), Some(LifecycleState::Uninitialized));

        manager.initialize(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Initialized));

        manager.start(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Running));

        manager.pause(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Paused));

        manager.resume(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Running));

        manager.stop(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Stopped));
    }

    #[test]
    fn test_dependencies() {
        let mut manager = LifecycleManager::default();
        let domain = DomainId::new(1);

        let dep_id = manager.register("dependency", domain, Vec::new());
        let main_id = manager.register("main", domain, vec![dep_id]);

        // Initialize both
        manager.initialize(dep_id).unwrap();
        manager.initialize(main_id).unwrap();

        // Can't start main without dependency running
        assert!(manager.start(main_id).is_err());

        // Start dependency first
        manager.start(dep_id).unwrap();

        // Now main can start
        manager.start(main_id).unwrap();

        // Can't stop dependency while main is running
        assert!(manager.stop(dep_id).is_err());

        // Stop main first
        manager.stop(main_id).unwrap();

        // Now dependency can stop
        manager.stop(dep_id).unwrap();
    }

    #[test]
    fn test_startup_order() {
        let mut manager = LifecycleManager::default();
        let domain = DomainId::new(1);

        let a = manager.register("a", domain, Vec::new());
        let b = manager.register("b", domain, vec![a]);
        let c = manager.register("c", domain, vec![a, b]);

        let order = manager.startup_order();

        // a should come before b, b should come before c
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_c = order.iter().position(|&x| x == c).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_recovery() {
        let mut manager = LifecycleManager::default();
        let domain = DomainId::new(1);

        let id = manager.register("recoverable", domain, Vec::new());
        manager.initialize(id).unwrap();
        manager.start(id).unwrap();

        // Simulate failure
        manager.fail(id, "Test failure").unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Failed));

        // Recover
        manager.recover(id).unwrap();
        assert_eq!(manager.get_state(id), Some(LifecycleState::Running));
    }
}
