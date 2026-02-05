//! # Kernel Orchestrator
//!
//! The orchestrator is the central coordination point of the kernel.
//! It manages the lifecycle of all subsystems and modules.

pub mod capability_broker;
pub mod lifecycle;
pub mod panic_handler;
pub mod resource_broker;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::RwLock;

use crate::{KernelError, KernelEvent, KernelResult};

/// The kernel orchestrator
///
/// This is the central coordination point for the entire kernel.
/// It manages subsystems, modules, and their interactions.
pub struct KernelOrchestrator {
    /// Registered subsystems
    subsystems: RwLock<BTreeMap<String, Arc<dyn Subsystem>>>,

    /// Boot configuration
    config: RwLock<BootConfiguration>,

    /// Event dispatcher
    event_dispatcher: EventDispatcher,

    /// Is the orchestrator initialized?
    initialized: AtomicBool,
}

impl KernelOrchestrator {
    /// Create a new kernel orchestrator
    pub const fn new() -> Self {
        Self {
            subsystems: RwLock::new(BTreeMap::new()),
            // Config will be initialized via init()
            config: RwLock::new(BootConfiguration::empty()),
            event_dispatcher: EventDispatcher::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the orchestrator with boot configuration
    pub fn init(&self, config: BootConfiguration) -> KernelResult<()> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Err(KernelError::AlreadyExists);
        }

        *self.config.write() = config;

        crate::set_kernel_state(crate::KernelState::Initializing);

        log::info!("Helix Kernel Orchestrator initializing...");

        Ok(())
    }

    /// Register a subsystem
    pub fn register_subsystem(
        &self,
        name: String,
        subsystem: Arc<dyn Subsystem>,
    ) -> KernelResult<()> {
        let mut subsystems = self.subsystems.write();

        if subsystems.contains_key(&name) {
            return Err(KernelError::AlreadyExists);
        }

        subsystems.insert(name.clone(), subsystem);

        log::debug!("Registered subsystem: {}", name);

        Ok(())
    }

    /// Get a subsystem by name
    pub fn get_subsystem(&self, name: &str) -> Option<Arc<dyn Subsystem>> {
        self.subsystems.read().get(name).cloned()
    }

    /// Initialize all registered subsystems in dependency order
    pub fn init_subsystems(&self) -> KernelResult<()> {
        let subsystems = self.subsystems.read();

        // Build dependency graph and topological sort
        let order = self.resolve_init_order(&subsystems)?;

        for name in order {
            if let Some(subsystem) = subsystems.get(&name) {
                log::info!("Initializing subsystem: {}", name);
                subsystem.init()?;
            }
        }

        Ok(())
    }

    /// Resolve initialization order based on dependencies
    fn resolve_init_order(
        &self,
        subsystems: &BTreeMap<String, Arc<dyn Subsystem>>,
    ) -> KernelResult<Vec<String>> {
        // Topological sort of subsystems based on dependencies
        let mut result = Vec::new();
        let mut visited = BTreeMap::new();

        for name in subsystems.keys() {
            self.visit_subsystem(name, subsystems, &mut visited, &mut result)?;
        }

        Ok(result)
    }

    fn visit_subsystem(
        &self,
        name: &str,
        subsystems: &BTreeMap<String, Arc<dyn Subsystem>>,
        visited: &mut BTreeMap<String, VisitState>,
        result: &mut Vec<String>,
    ) -> KernelResult<()> {
        match visited.get(name) {
            Some(VisitState::Visiting) => {
                return Err(KernelError::InvalidArgument); // Cycle detected
            },
            Some(VisitState::Visited) => {
                return Ok(());
            },
            None => {},
        }

        visited.insert(name.to_string(), VisitState::Visiting);

        if let Some(subsystem) = subsystems.get(name) {
            for dep in subsystem.dependencies() {
                self.visit_subsystem(dep, subsystems, visited, result)?;
            }
        }

        visited.insert(name.to_string(), VisitState::Visited);
        result.push(name.to_string());

        Ok(())
    }

    /// Shutdown all subsystems
    pub fn shutdown(&self) -> KernelResult<()> {
        crate::set_kernel_state(crate::KernelState::ShuttingDown);

        let subsystems = self.subsystems.read();

        // Shutdown in reverse order
        let order = self.resolve_init_order(&subsystems)?;

        for name in order.into_iter().rev() {
            if let Some(subsystem) = subsystems.get(&name) {
                log::info!("Shutting down subsystem: {}", name);
                if let Err(e) = subsystem.shutdown() {
                    log::error!("Error shutting down {}: {:?}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Dispatch an event to all listeners
    pub fn dispatch_event(&self, event: KernelEvent) {
        self.event_dispatcher.dispatch(event);
    }

    /// Get the boot configuration
    pub fn config(&self) -> BootConfiguration {
        self.config.read().clone()
    }
}

#[derive(Debug, Clone, Copy)]
enum VisitState {
    Visiting,
    Visited,
}

/// Subsystem trait
///
/// All major kernel subsystems must implement this trait.
pub trait Subsystem: Send + Sync {
    /// Get the subsystem name
    fn name(&self) -> &'static str;

    /// Get the subsystem version
    fn version(&self) -> &'static str;

    /// Get subsystem dependencies
    fn dependencies(&self) -> &[&'static str] {
        &[]
    }

    /// Initialize the subsystem
    fn init(&self) -> KernelResult<()>;

    /// Shutdown the subsystem
    fn shutdown(&self) -> KernelResult<()>;

    /// Suspend the subsystem (for power management)
    fn suspend(&self) -> KernelResult<()> {
        Ok(())
    }

    /// Resume the subsystem
    fn resume(&self) -> KernelResult<()> {
        Ok(())
    }

    /// Health check
    fn is_healthy(&self) -> bool {
        true
    }
}

/// Boot configuration
#[derive(Debug, Clone, Default)]
pub struct BootConfiguration {
    /// Command line arguments
    pub command_line: String,
    /// Initial memory map
    pub memory_map: Vec<MemoryMapEntry>,
    /// Boot modules
    pub boot_modules: Vec<BootModule>,
    /// Debug mode enabled
    pub debug_mode: bool,
    /// Verbose logging enabled
    pub verbose: bool,
    /// Number of CPUs to use (0 = all)
    pub cpu_count: usize,
    /// Custom configuration options
    pub custom: BTreeMap<String, String>,
}

impl BootConfiguration {
    /// Create an empty configuration (for const initialization)
    pub const fn empty() -> Self {
        Self {
            command_line: String::new(),
            memory_map: Vec::new(),
            boot_modules: Vec::new(),
            debug_mode: false,
            verbose: false,
            cpu_count: 0,
            custom: BTreeMap::new(),
        }
    }
}

/// Memory map entry from bootloader
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    /// Start address
    pub start: u64,
    /// Size in bytes
    pub size: u64,
    /// Entry type
    pub kind: MemoryMapKind,
}

/// Memory map entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapKind {
    /// Usable RAM
    Usable,
    /// Reserved
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Bootloader reserved
    BootloaderReserved,
    /// Kernel
    Kernel,
    /// Framebuffer
    Framebuffer,
}

/// Boot module information
#[derive(Debug, Clone)]
pub struct BootModule {
    /// Module name
    pub name: String,
    /// Physical address
    pub address: u64,
    /// Size in bytes
    pub size: usize,
    /// Command line for this module
    pub command_line: String,
}

/// Event dispatcher
struct EventDispatcher {
    listeners: RwLock<Vec<Arc<dyn crate::KernelEventListener>>>,
}

impl EventDispatcher {
    const fn new() -> Self {
        Self {
            listeners: RwLock::new(Vec::new()),
        }
    }

    fn dispatch(&self, event: KernelEvent) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_event(&event);
        }
    }
}
