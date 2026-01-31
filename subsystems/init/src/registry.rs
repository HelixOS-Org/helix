//! # Subsystem Registry
//!
//! This module provides a global registry for tracking and managing subsystems.
//! It supports both static (compile-time) and dynamic (runtime) registration,
//! with thread-safe access patterns.
//!
//! ## Registry Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         SUBSYSTEM REGISTRY                                   │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                    STATIC REGISTRATION                                  │ │
//! │  │                                                                         │ │
//! │  │  #[subsystem]                                                           │ │
//! │  │  impl Subsystem for MySubsystem { ... }                                │ │
//! │  │            │                                                            │ │
//! │  │            ▼                                                            │ │
//! │  │  ┌──────────────────────────────────────────────────────────────────┐  │ │
//! │  │  │  STATIC_REGISTRY: [StaticEntry; MAX_STATIC_SUBSYSTEMS]           │  │ │
//! │  │  │  - Populated at link time via .init_array / constructors         │  │ │
//! │  │  │  - Zero runtime overhead for lookup                              │  │ │
//! │  │  └──────────────────────────────────────────────────────────────────┘  │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                    DYNAMIC REGISTRATION                                 │ │
//! │  │                                                                         │ │
//! │  │  registry.register(Box::new(MySubsystem::new()))?;                     │ │
//! │  │            │                                                            │ │
//! │  │            ▼                                                            │ │
//! │  │  ┌──────────────────────────────────────────────────────────────────┐  │ │
//! │  │  │  DYNAMIC_REGISTRY: BTreeMap<SubsystemId, SubsystemWrapper>       │  │ │
//! │  │  │  - Protected by spinlock                                         │  │ │
//! │  │  │  - Supports hot-reload                                           │  │ │
//! │  │  └──────────────────────────────────────────────────────────────────┘  │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌────────────────────────────────────────────────────────────────────────┐ │
//! │  │                       LOOKUP                                            │ │
//! │  │                                                                         │ │
//! │  │  registry.get(id) ───▶ Check static ───▶ Check dynamic ───▶ Result    │ │
//! │  │                                                                         │ │
//! │  └────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use spin::RwLock;

use crate::dependency::DependencyGraph;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::InitPhase;
use crate::subsystem::{Subsystem, SubsystemId, SubsystemInfo, SubsystemState, SubsystemWrapper};

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of statically registered subsystems
pub const MAX_STATIC_SUBSYSTEMS: usize = 256;

/// Maximum number of dynamically registered subsystems
pub const MAX_DYNAMIC_SUBSYSTEMS: usize = 256;

/// Total maximum subsystems
pub const MAX_SUBSYSTEMS: usize = MAX_STATIC_SUBSYSTEMS + MAX_DYNAMIC_SUBSYSTEMS;

// =============================================================================
// STATIC REGISTRATION
// =============================================================================

/// Entry in the static registry
pub struct StaticEntry {
    /// Factory function to create the subsystem
    pub factory: fn() -> Box<dyn Subsystem>,

    /// Subsystem info (for early access without instantiation)
    pub info: &'static SubsystemInfo,

    /// Whether this entry is valid
    pub valid: bool,
}

impl StaticEntry {
    /// Create new entry
    pub const fn new(factory: fn() -> Box<dyn Subsystem>, info: &'static SubsystemInfo) -> Self {
        Self {
            factory,
            info,
            valid: true,
        }
    }

    /// Empty entry
    pub const fn empty() -> Self {
        Self {
            factory: empty_factory,
            info: &EMPTY_INFO,
            valid: false,
        }
    }
}

fn empty_factory() -> Box<dyn Subsystem> {
    panic!("Empty subsystem factory called");
}

static EMPTY_INFO: SubsystemInfo = SubsystemInfo::new("", InitPhase::Boot);

/// Static registry storage
static mut STATIC_REGISTRY: [StaticEntry; MAX_STATIC_SUBSYSTEMS] = {
    const EMPTY: StaticEntry = StaticEntry::empty();
    [EMPTY; MAX_STATIC_SUBSYSTEMS]
};

/// Number of static entries
static STATIC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Static registry initialized flag
static STATIC_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Register a subsystem statically
///
/// # Safety
/// This should only be called from static constructors.
pub unsafe fn register_static(factory: fn() -> Box<dyn Subsystem>, info: &'static SubsystemInfo) {
    let index = STATIC_COUNT.fetch_add(1, Ordering::SeqCst);
    if index < MAX_STATIC_SUBSYSTEMS {
        STATIC_REGISTRY[index] = StaticEntry::new(factory, info);
    }
}

/// Get static entries (after initialization)
pub fn get_static_entries() -> &'static [StaticEntry] {
    let count = STATIC_COUNT.load(Ordering::Acquire);
    // Safety: STATIC_REGISTRY is only written during static initialization
    unsafe { &STATIC_REGISTRY[..count] }
}

// =============================================================================
// REGISTRY ENTRY
// =============================================================================

/// Entry in the dynamic registry
pub struct RegistryEntry {
    /// The wrapped subsystem
    pub wrapper: SubsystemWrapper,

    /// Registration timestamp
    pub registered_at: u64,

    /// Whether from static registration
    pub is_static: bool,

    /// Registration index (for ordering)
    pub index: usize,
}

impl RegistryEntry {
    /// Create new entry
    pub fn new(subsystem: Box<dyn Subsystem>, index: usize, is_static: bool) -> Self {
        Self {
            wrapper: SubsystemWrapper::new(subsystem),
            registered_at: crate::get_timestamp(),
            is_static,
            index,
        }
    }

    /// Get subsystem ID
    pub fn id(&self) -> SubsystemId {
        self.wrapper.id()
    }

    /// Get subsystem info
    pub fn info(&self) -> &SubsystemInfo {
        self.wrapper.info()
    }

    /// Get current state
    pub fn state(&self) -> SubsystemState {
        self.wrapper.state()
    }
}

// =============================================================================
// SUBSYSTEM REGISTRY
// =============================================================================

/// The main subsystem registry
pub struct SubsystemRegistry {
    /// Dynamic entries by ID
    entries: BTreeMap<SubsystemId, RegistryEntry>,

    /// Entries by name for lookup
    by_name: BTreeMap<&'static str, SubsystemId>,

    /// Entries by phase for iteration
    by_phase: [Vec<SubsystemId>; 5],

    /// Dependency graph
    graph: DependencyGraph,

    /// Total entry count
    count: usize,

    /// Registry is locked (no new registrations)
    locked: bool,

    /// Statistics
    stats: RegistryStats,
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// Total registrations
    pub total_registered: usize,
    /// Static registrations
    pub static_registered: usize,
    /// Dynamic registrations
    pub dynamic_registered: usize,
    /// Failed registrations
    pub registration_failures: usize,
    /// Successful initializations
    pub init_success: usize,
    /// Failed initializations
    pub init_failed: usize,
}

impl SubsystemRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_phase: Default::default(),
            graph: DependencyGraph::new(),
            count: 0,
            locked: false,
            stats: RegistryStats::default(),
        }
    }

    /// Initialize from static registry
    pub fn init_from_static(&mut self) -> InitResult<()> {
        for entry in get_static_entries() {
            if entry.valid {
                let subsystem = (entry.factory)();
                self.register_internal(subsystem, true)?;
            }
        }
        STATIC_INITIALIZED.store(true, Ordering::Release);
        Ok(())
    }

    /// Register a subsystem dynamically
    pub fn register(&mut self, subsystem: Box<dyn Subsystem>) -> InitResult<SubsystemId> {
        self.register_internal(subsystem, false)
    }

    /// Internal registration
    fn register_internal(
        &mut self,
        subsystem: Box<dyn Subsystem>,
        is_static: bool,
    ) -> InitResult<SubsystemId> {
        if self.locked {
            return Err(InitError::new(
                ErrorKind::InvalidState,
                "Registry is locked",
            ));
        }

        if self.count >= MAX_SUBSYSTEMS {
            self.stats.registration_failures += 1;
            return Err(InitError::new(
                ErrorKind::ResourceExhausted,
                "Maximum subsystems reached",
            ));
        }

        let info = subsystem.info();
        let id = info.id;
        let name = info.name;
        let phase = info.phase;

        // Check for duplicates
        if self.entries.contains_key(&id) {
            self.stats.registration_failures += 1;
            return Err(
                InitError::new(ErrorKind::AlreadyExists, "Subsystem already registered")
                    .with_subsystem(id),
            );
        }

        // Add to dependency graph
        self.graph.add_subsystem(info)?;

        // Create entry
        let entry = RegistryEntry::new(subsystem, self.count, is_static);

        // Insert into maps
        self.by_name.insert(name, id);
        self.by_phase[phase as usize].push(id);
        self.entries.insert(id, entry);

        self.count += 1;
        self.stats.total_registered += 1;
        if is_static {
            self.stats.static_registered += 1;
        } else {
            self.stats.dynamic_registered += 1;
        }

        Ok(id)
    }

    /// Unregister a subsystem
    pub fn unregister(&mut self, id: SubsystemId) -> InitResult<()> {
        if self.locked {
            return Err(InitError::new(
                ErrorKind::InvalidState,
                "Registry is locked",
            ));
        }

        let entry = self.entries.remove(&id).ok_or_else(|| {
            InitError::new(ErrorKind::NotFound, "Subsystem not found").with_subsystem(id)
        })?;

        // Remove from name index
        self.by_name.remove(entry.info().name);

        // Remove from phase index
        let phase = entry.info().phase;
        self.by_phase[phase as usize].retain(|&x| x != id);

        // Remove from graph
        self.graph.remove_subsystem(id)?;

        self.count -= 1;

        Ok(())
    }

    /// Get a subsystem by ID
    pub fn get(&self, id: SubsystemId) -> Option<&RegistryEntry> {
        self.entries.get(&id)
    }

    /// Get mutable subsystem by ID
    pub fn get_mut(&mut self, id: SubsystemId) -> Option<&mut RegistryEntry> {
        self.entries.get_mut(&id)
    }

    /// Get subsystem by name
    pub fn get_by_name(&self, name: &str) -> Option<&RegistryEntry> {
        self.by_name.get(name).and_then(|id| self.entries.get(id))
    }

    /// Get mutable subsystem by name
    pub fn get_by_name_mut(&mut self, name: &str) -> Option<&mut RegistryEntry> {
        if let Some(&id) = self.by_name.get(name) {
            self.entries.get_mut(&id)
        } else {
            None
        }
    }

    /// Check if subsystem exists
    pub fn contains(&self, id: SubsystemId) -> bool {
        self.entries.contains_key(&id)
    }

    /// Check if subsystem exists by name
    pub fn contains_name(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Get all subsystems in a phase
    pub fn get_phase(&self, phase: InitPhase) -> Vec<&RegistryEntry> {
        self.by_phase[phase as usize]
            .iter()
            .filter_map(|id| self.entries.get(id))
            .collect()
    }

    /// Get all subsystem IDs in a phase
    pub fn get_phase_ids(&self, phase: InitPhase) -> &[SubsystemId] {
        &self.by_phase[phase as usize]
    }

    /// Get initialization order (topological sort)
    pub fn get_init_order(&self) -> InitResult<Vec<SubsystemId>> {
        let mut graph = self.graph.clone();
        graph.validate()?;
        Ok(graph
            .topological_order()
            .map(|o| o.to_vec())
            .unwrap_or_default())
    }

    /// Get initialization order for a specific phase
    pub fn get_phase_order(&self, phase: InitPhase) -> Vec<SubsystemId> {
        self.graph.phase_order(phase)
    }

    /// Get parallel batches for a phase
    pub fn get_parallel_batches(&self, phase: InitPhase) -> Vec<Vec<SubsystemId>> {
        self.graph.get_parallel_batches(phase)
    }

    /// Validate dependencies
    pub fn validate(&mut self) -> InitResult<()> {
        self.graph.validate()
    }

    /// Lock the registry (prevent new registrations)
    pub fn lock(&mut self) {
        self.locked = true;
    }

    /// Unlock the registry
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get all entries
    pub fn entries(&self) -> impl Iterator<Item = &RegistryEntry> {
        self.entries.values()
    }

    /// Get mutable entries
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut RegistryEntry> {
        self.entries.values_mut()
    }

    /// Get dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Get statistics
    pub fn stats(&self) -> &RegistryStats {
        &self.stats
    }

    /// Update init stats
    pub fn record_init_result(&mut self, success: bool) {
        if success {
            self.stats.init_success += 1;
        } else {
            self.stats.init_failed += 1;
        }
    }

    /// Get subsystems by state
    pub fn get_by_state(&self, state: SubsystemState) -> Vec<SubsystemId> {
        self.entries
            .iter()
            .filter(|(_, e)| e.state() == state)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get active subsystems
    pub fn get_active(&self) -> Vec<SubsystemId> {
        self.get_by_state(SubsystemState::Active)
    }

    /// Get failed subsystems
    pub fn get_failed(&self) -> Vec<SubsystemId> {
        self.get_by_state(SubsystemState::Failed)
    }

    /// Find subsystems providing a capability
    pub fn find_by_capability(&self, cap: crate::phase::PhaseCapabilities) -> Vec<SubsystemId> {
        self.entries
            .iter()
            .filter(|(_, e)| e.info().provides.contains(cap))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Generate DOT graph for visualization
    pub fn to_dot(&self) -> String {
        self.graph.to_dot()
    }
}

impl Default for SubsystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GLOBAL REGISTRY
// =============================================================================

/// Global subsystem registry
static GLOBAL_REGISTRY: RwLock<Option<SubsystemRegistry>> = RwLock::new(None);

/// Global registry initialized flag
static GLOBAL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the global registry
pub fn init_global_registry() -> InitResult<()> {
    if GLOBAL_INITIALIZED.load(Ordering::Acquire) {
        return Err(InitError::new(
            ErrorKind::AlreadyExists,
            "Global registry already initialized",
        ));
    }

    let mut registry = SubsystemRegistry::new();
    registry.init_from_static()?;

    let mut guard = GLOBAL_REGISTRY.write();
    *guard = Some(registry);

    GLOBAL_INITIALIZED.store(true, Ordering::Release);

    Ok(())
}

/// Check if global registry is initialized
pub fn is_global_initialized() -> bool {
    GLOBAL_INITIALIZED.load(Ordering::Acquire)
}

/// Access global registry for reading
pub fn with_registry<F, R>(f: F) -> InitResult<R>
where
    F: FnOnce(&SubsystemRegistry) -> R,
{
    if !is_global_initialized() {
        return Err(InitError::new(
            ErrorKind::InvalidState,
            "Global registry not initialized",
        ));
    }

    let guard = GLOBAL_REGISTRY.read();
    let registry = guard
        .as_ref()
        .ok_or_else(|| InitError::new(ErrorKind::InvalidState, "Global registry is None"))?;

    Ok(f(registry))
}

/// Access global registry for writing
pub fn with_registry_mut<F, R>(f: F) -> InitResult<R>
where
    F: FnOnce(&mut SubsystemRegistry) -> R,
{
    if !is_global_initialized() {
        return Err(InitError::new(
            ErrorKind::InvalidState,
            "Global registry not initialized",
        ));
    }

    let mut guard = GLOBAL_REGISTRY.write();
    let registry = guard
        .as_mut()
        .ok_or_else(|| InitError::new(ErrorKind::InvalidState, "Global registry is None"))?;

    Ok(f(registry))
}

/// Register a subsystem globally
pub fn register_global(subsystem: Box<dyn Subsystem>) -> InitResult<SubsystemId> {
    with_registry_mut(|r| r.register(subsystem))?
}

/// Get a subsystem from global registry
pub fn get_global(id: SubsystemId) -> InitResult<bool> {
    with_registry(|r| r.contains(id))
}

/// Get a subsystem by name from global registry
pub fn get_global_by_name(name: &str) -> InitResult<Option<SubsystemId>> {
    with_registry(|r| r.by_name.get(name).copied())
}

// =============================================================================
// REGISTRY SNAPSHOT
// =============================================================================

/// A snapshot of registry state for diagnostics
#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    /// Timestamp
    pub timestamp: u64,

    /// Total subsystems
    pub total: usize,

    /// By phase counts
    pub by_phase: [usize; 5],

    /// By state counts
    pub by_state: BTreeMap<SubsystemState, usize>,

    /// Subsystem summaries
    pub subsystems: Vec<SubsystemSummary>,

    /// Statistics
    pub stats: RegistryStats,
}

/// Summary of a single subsystem
#[derive(Debug, Clone)]
pub struct SubsystemSummary {
    /// ID
    pub id: SubsystemId,
    /// Name
    pub name: &'static str,
    /// Phase
    pub phase: InitPhase,
    /// State
    pub state: SubsystemState,
    /// Is essential
    pub essential: bool,
    /// Init duration
    pub init_duration_us: u64,
}

impl SubsystemRegistry {
    /// Take a snapshot of the registry
    pub fn snapshot(&self) -> RegistrySnapshot {
        let mut by_state: BTreeMap<SubsystemState, usize> = BTreeMap::new();
        let mut subsystems = Vec::new();

        for entry in self.entries.values() {
            let state = entry.state();
            *by_state.entry(state).or_insert(0) += 1;

            subsystems.push(SubsystemSummary {
                id: entry.id(),
                name: entry.info().name,
                phase: entry.info().phase,
                state,
                essential: entry.info().essential,
                init_duration_us: entry.wrapper.init_duration_us(),
            });
        }

        RegistrySnapshot {
            timestamp: crate::get_timestamp(),
            total: self.count,
            by_phase: [
                self.by_phase[0].len(),
                self.by_phase[1].len(),
                self.by_phase[2].len(),
                self.by_phase[3].len(),
                self.by_phase[4].len(),
            ],
            by_state,
            subsystems,
            stats: self.stats.clone(),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InitContext;
    use crate::phase::PhaseCapabilities;

    struct TestSubsystem {
        info: SubsystemInfo,
    }

    impl TestSubsystem {
        fn new(name: &'static str, phase: InitPhase) -> Self {
            Self {
                info: SubsystemInfo::new(name, phase),
            }
        }
    }

    impl Subsystem for TestSubsystem {
        fn info(&self) -> &SubsystemInfo {
            &self.info
        }

        fn init(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_register() {
        let mut registry = SubsystemRegistry::new();

        let sub = Box::new(TestSubsystem::new("test1", InitPhase::Boot));
        let id = registry.register(sub).unwrap();

        assert!(registry.contains(id));
        assert!(registry.contains_name("test1"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_duplicate() {
        let mut registry = SubsystemRegistry::new();

        let sub1 = Box::new(TestSubsystem::new("test", InitPhase::Boot));
        let sub2 = Box::new(TestSubsystem::new("test", InitPhase::Boot));

        registry.register(sub1).unwrap();
        assert!(registry.register(sub2).is_err());
    }

    #[test]
    fn test_registry_phase() {
        let mut registry = SubsystemRegistry::new();

        registry
            .register(Box::new(TestSubsystem::new("boot1", InitPhase::Boot)))
            .unwrap();
        registry
            .register(Box::new(TestSubsystem::new("boot2", InitPhase::Boot)))
            .unwrap();
        registry
            .register(Box::new(TestSubsystem::new("early1", InitPhase::Early)))
            .unwrap();

        let boot_entries = registry.get_phase(InitPhase::Boot);
        assert_eq!(boot_entries.len(), 2);

        let early_entries = registry.get_phase(InitPhase::Early);
        assert_eq!(early_entries.len(), 1);
    }

    #[test]
    fn test_registry_lock() {
        let mut registry = SubsystemRegistry::new();

        let sub1 = Box::new(TestSubsystem::new("test1", InitPhase::Boot));
        registry.register(sub1).unwrap();

        registry.lock();

        let sub2 = Box::new(TestSubsystem::new("test2", InitPhase::Boot));
        assert!(registry.register(sub2).is_err());

        registry.unlock();

        let sub3 = Box::new(TestSubsystem::new("test3", InitPhase::Boot));
        assert!(registry.register(sub3).is_ok());
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = SubsystemRegistry::new();

        let sub = Box::new(TestSubsystem::new("test", InitPhase::Boot));
        let id = registry.register(sub).unwrap();

        assert_eq!(registry.len(), 1);

        registry.unregister(id).unwrap();

        assert_eq!(registry.len(), 0);
        assert!(!registry.contains(id));
    }

    #[test]
    fn test_registry_snapshot() {
        let mut registry = SubsystemRegistry::new();

        registry
            .register(Box::new(TestSubsystem::new("boot1", InitPhase::Boot)))
            .unwrap();
        registry
            .register(Box::new(TestSubsystem::new("early1", InitPhase::Early)))
            .unwrap();

        let snapshot = registry.snapshot();

        assert_eq!(snapshot.total, 2);
        assert_eq!(snapshot.by_phase[0], 1);
        assert_eq!(snapshot.by_phase[1], 1);
        assert_eq!(snapshot.subsystems.len(), 2);
    }
}
