//! # Helix OS Subsystem Initialization Framework
//!
//! A **revolutionary**, industrial-grade initialization framework designed to
//! orchestrate the startup sequence of a modern operating system kernel.
//!
//! ## Philosophy
//!
//! ```text
//! ╔═══════════════════════════════════════════════════════════════════════════════╗
//! ║                                                                               ║
//! ║  "Initialization is not just about starting things up — it's about           ║
//! ║   establishing invariants that the entire system depends upon."              ║
//! ║                                                                               ║
//! ║  The Helix Init Framework treats initialization as a first-class concern,    ║
//! ║  with the same rigor applied to memory safety, concurrency, and security.    ║
//! ║                                                                               ║
//! ╚═══════════════════════════════════════════════════════════════════════════════╝
//! ```
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────────┐
//! │                     HELIX INIT FRAMEWORK ARCHITECTURE                           │
//! │                     ═════════════════════════════════                           │
//! │                                                                                 │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐  │
//! │  │                         INITIALIZATION TIMELINE                           │  │
//! │  │                                                                           │  │
//! │  │   BOOT          EARLY          CORE           LATE          RUNTIME      │  │
//! │  │    │              │              │              │              │         │  │
//! │  │    ▼              ▼              ▼              ▼              ▼         │  │
//! │  │  ┌───┐  ────▶  ┌───┐  ────▶  ┌───┐  ────▶  ┌───┐  ────▶  ┌───┐         │  │
//! │  │  │ B │ BARRIER │ E │ BARRIER │ C │ BARRIER │ L │ BARRIER │ R │         │  │
//! │  │  └───┘         └───┘         └───┘         └───┘         └───┘         │  │
//! │  │                                                                           │  │
//! │  │  Firmware      Memory         Scheduler      Filesystems   Userland     │  │
//! │  │  Console       Interrupts     IPC            Network       Services     │  │
//! │  │  Boot Info     Timers         Drivers        Security      Hot-reload   │  │
//! │  └──────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                                 │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐  │
//! │  │                         DEPENDENCY GRAPH (DAG)                            │  │
//! │  │                                                                           │  │
//! │  │        ┌─────────┐                                                        │  │
//! │  │        │ Memory  │◄─────────┬──────────┬──────────┐                      │  │
//! │  │        └────┬────┘          │          │          │                      │  │
//! │  │             │               │          │          │                      │  │
//! │  │             ▼               ▼          ▼          ▼                      │  │
//! │  │     ┌───────────┐    ┌──────────┐ ┌────────┐ ┌─────────┐                │  │
//! │  │     │ Scheduler │    │Interrupts│ │ Timers │ │   IPC   │                │  │
//! │  │     └─────┬─────┘    └────┬─────┘ └───┬────┘ └────┬────┘                │  │
//! │  │           │               │           │           │                      │  │
//! │  │           ▼               ▼           ▼           ▼                      │  │
//! │  │        ┌──────────────────────────────────────────────┐                  │  │
//! │  │        │              Driver Framework                 │                  │  │
//! │  │        └──────────────────┬───────────────────────────┘                  │  │
//! │  │                           │                                               │  │
//! │  │                           ▼                                               │  │
//! │  │     ┌────────────┐  ┌───────────┐  ┌──────────┐                          │  │
//! │  │     │ Filesystem │  │  Network  │  │ Security │                          │  │
//! │  │     └─────┬──────┘  └─────┬─────┘  └────┬─────┘                          │  │
//! │  │           └───────────────┼─────────────┘                                │  │
//! │  │                           ▼                                               │  │
//! │  │                    ┌────────────┐                                         │  │
//! │  │                    │  Userland  │                                         │  │
//! │  │                    └────────────┘                                         │  │
//! │  └──────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                                 │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐  │
//! │  │                           REGISTRY                                        │  │
//! │  │                                                                           │  │
//! │  │   ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │  │
//! │  │   │  Boot   │ │ Memory  │ │   CPU   │ │  Timer  │ │ Driver  │ ...       │  │
//! │  │   │ Subsys  │ │ Subsys  │ │ Subsys  │ │ Subsys  │ │ Subsys  │           │  │
//! │  │   └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │  │
//! │  │        │           │           │           │           │                 │  │
//! │  │        └───────────┴───────────┴───────────┴───────────┘                 │  │
//! │  │                                │                                          │  │
//! │  │                                ▼                                          │  │
//! │  │                    ┌───────────────────────┐                              │  │
//! │  │                    │   Global Registry     │                              │  │
//! │  │                    │   (Compile-time +     │                              │  │
//! │  │                    │    Runtime dynamic)   │                              │  │
//! │  │                    └───────────────────────┘                              │  │
//! │  └──────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                                 │
//! │  ┌──────────────────────────────────────────────────────────────────────────┐  │
//! │  │                      EXECUTOR MODES                                       │  │
//! │  │                                                                           │  │
//! │  │   Sequential          Parallel           Lazy              Conditional   │  │
//! │  │   ┌─┐─┐─┐─┐          ┌─┐ ┌─┐ ┌─┐        ┌─┐               ┌─┐           │  │
//! │  │   │A│B│C│D│          │A│ │B│ │C│        │?│──▶┌─┐         │?│──▶┌─┐     │  │
//! │  │   └─┴─┴─┴─┘          └─┘ └─┘ └─┘        └─┘   │A│         └─┘   │A│     │  │
//! │  │                       ║   ║   ║               └─┘               └─┘     │  │
//! │  │                       ╚═══╩═══╝           On-demand        If condition │  │
//! │  └──────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                                 │
//! └─────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Initialization Phases
//!
//! | Phase     | Order | Environment                    | Typical Subsystems          |
//! |-----------|-------|--------------------------------|-----------------------------|
//! | **Boot**  | 0     | No heap, no interrupts         | Firmware, Console           |
//! | **Early** | 1     | Basic heap, no scheduler       | Memory, CPU, Interrupts     |
//! | **Core**  | 2     | Full heap, basic scheduling    | Scheduler, IPC, Timers      |
//! | **Late**  | 3     | Full kernel services           | Drivers, FS, Network        |
//! | **Runtime** | 4   | Ready for userspace            | Security, Userland          |
//!
//! ## Key Features
//!
//! - **Dependency Graph**: Subsystems declare dependencies; framework ensures correct order
//! - **Phase Barriers**: Hard synchronization points between major phases
//! - **Rollback Support**: Failed subsystems can trigger cleanup of dependent systems
//! - **Parallel Execution**: Independent subsystems can initialize concurrently
//! - **Lazy Initialization**: Subsystems can defer init until first use
//! - **Conditional Init**: Subsystems can be enabled/disabled at runtime
//! - **Hot-reload Ready**: Late-phase subsystems can be reloaded
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use helix_init::{
//!     Subsystem, SubsystemRegistry, InitPhase, InitResult,
//!     subsystem, requires, provides,
//! };
//!
//! #[subsystem(
//!     name = "memory_manager",
//!     phase = InitPhase::Early,
//!     priority = 100,
//! )]
//! #[requires("boot_info", "console")]
//! #[provides("pmm", "vmm", "heap")]
//! pub struct MemoryManager;
//!
//! impl Subsystem for MemoryManager {
//!     fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
//!         // Initialize physical memory manager
//!         let pmm = Pmm::init(ctx.boot_info())?;
//!         ctx.provide("pmm", pmm)?;
//!
//!         // Initialize virtual memory manager
//!         let vmm = Vmm::init(&pmm)?;
//!         ctx.provide("vmm", vmm)?;
//!
//!         // Initialize kernel heap
//!         let heap = Heap::init(&vmm)?;
//!         ctx.provide("heap", heap)?;
//!
//!         Ok(())
//!     }
//!
//!     fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
//!         // Cleanup in reverse order
//!         ctx.take::<Heap>("heap")?.shutdown()?;
//!         ctx.take::<Vmm>("vmm")?.shutdown()?;
//!         ctx.take::<Pmm>("pmm")?.shutdown()?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Adding a New Subsystem
//!
//! 1. **Define the subsystem struct** with `#[subsystem]` attribute
//! 2. **Declare dependencies** with `#[requires(...)]`
//! 3. **Declare provisions** with `#[provides(...)]`
//! 4. **Implement `Subsystem` trait** with `init()` and optional `shutdown()`
//! 5. **Register** using `registry.register::<MySubsystem>()`
//!
//! ## Invariants
//!
//! 1. **Dependency Satisfaction**: A subsystem's `init()` is never called before all
//!    its dependencies have completed successfully.
//!
//! 2. **Phase Ordering**: Boot → Early → Core → Late → Runtime. No subsystem in a
//!    later phase runs before all subsystems in earlier phases complete.
//!
//! 3. **Rollback Guarantee**: If a subsystem fails, its `shutdown()` is called (if
//!    partially initialized), then all dependent subsystems are rolled back.
//!
//! 4. **No Cycles**: The dependency graph is a DAG. Cycles are detected at registration
//!    time and cause a compile-time error (where possible) or runtime panic.
//!
//! 5. **Thread Safety**: The registry and executor are thread-safe. Subsystem init
//!    functions may run in parallel if they have no dependencies on each other.

#![no_std]
#![feature(const_type_name)]
#![feature(const_type_id)]
#![feature(negative_impls)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]

// =============================================================================
// EXTERNAL DEPENDENCIES
// =============================================================================

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use spin::{Mutex, Once, RwLock};

// =============================================================================
// MODULE DECLARATIONS
// =============================================================================

/// Phase definitions and barriers
pub mod phase;

/// Dependency graph and topological sort
pub mod dependency;

/// Subsystem registry
pub mod registry;

/// Initialization executor
pub mod executor;

/// Error types and rollback support
pub mod error;

/// Subsystem trait and types
pub mod subsystem;

/// Initialization context
pub mod context;

/// Macros for subsystem declaration
pub mod macros;

/// Metrics and tracing
#[cfg(feature = "metrics")]
pub mod metrics;

// -----------------------------------------------------------------------------
// Subsystem implementations by category
// -----------------------------------------------------------------------------

/// Boot subsystems (firmware, console, boot info)
pub mod subsystems {
    pub mod boot;
    pub mod cpu;
    pub mod debug;
    pub mod drivers;
    pub mod filesystem;
    pub mod interrupts;
    pub mod ipc;
    pub mod memory;
    pub mod network;
    pub mod scheduler;
    pub mod security;
    pub mod timers;
    pub mod userland;
}

// =============================================================================
// RE-EXPORTS
// =============================================================================

pub use context::{ContextBuilder, InitContext, ResourceHandle};
pub use dependency::{DependencyEdge, DependencyGraph, DependencyNode};
pub use error::{ErrorKind, InitError, InitResult, RollbackChain};
pub use executor::{ExecutionMode, ExecutionResult, ExecutorConfig, InitExecutor};
pub use phase::{InitPhase, PhaseBarrier, PhaseState, PHASE_ORDER};
pub use registry::{RegistryEntry, SubsystemRegistry, GLOBAL_REGISTRY};
pub use subsystem::{Subsystem, SubsystemFlags, SubsystemInfo, SubsystemState};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Framework version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of subsystems
pub const MAX_SUBSYSTEMS: usize = 512;

/// Maximum dependency depth (to detect infinite loops)
pub const MAX_DEPENDENCY_DEPTH: usize = 64;

/// Maximum rollback chain length
pub const MAX_ROLLBACK_CHAIN: usize = 128;

/// Default initialization timeout (in microseconds)
pub const DEFAULT_INIT_TIMEOUT_US: u64 = 10_000_000; // 10 seconds

/// Magic value for initialized subsystem
pub const SUBSYSTEM_MAGIC: u64 = 0x48454C49585F5355; // "HELIX_SU"

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global initialization state
static INIT_STATE: RwLock<GlobalInitState> = RwLock::new(GlobalInitState::new());

/// Global initialization complete flag
static INIT_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Current phase counter
static CURRENT_PHASE: AtomicU32 = AtomicU32::new(0);

/// Initialization start timestamp
static INIT_START_TIME: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// GLOBAL INIT STATE
// =============================================================================

/// Global initialization state machine
#[derive(Debug)]
pub struct GlobalInitState {
    /// Current overall state
    pub state: InitializationState,

    /// Current phase
    pub phase: InitPhase,

    /// Number of subsystems registered
    pub registered_count: usize,

    /// Number of subsystems initialized
    pub initialized_count: usize,

    /// Number of subsystems failed
    pub failed_count: usize,

    /// Whether initialization is in progress
    pub in_progress: bool,

    /// Whether rollback is in progress
    pub rolling_back: bool,

    /// Error that caused failure (if any)
    pub last_error: Option<InitError>,

    /// Total initialization time (microseconds)
    pub total_time_us: u64,

    /// Per-phase timing
    pub phase_times: [u64; 5],
}

impl GlobalInitState {
    /// Create new global state
    pub const fn new() -> Self {
        Self {
            state: InitializationState::NotStarted,
            phase: InitPhase::Boot,
            registered_count: 0,
            initialized_count: 0,
            failed_count: 0,
            in_progress: false,
            rolling_back: false,
            last_error: None,
            total_time_us: 0,
            phase_times: [0; 5],
        }
    }
}

/// Overall initialization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializationState {
    /// Not yet started
    NotStarted,
    /// Currently initializing
    InProgress,
    /// Completed successfully
    Complete,
    /// Failed with error
    Failed,
    /// Rolling back after failure
    RollingBack,
    /// Shutdown in progress
    ShuttingDown,
    /// Fully shutdown
    Shutdown,
}

// =============================================================================
// CORE API FUNCTIONS
// =============================================================================

/// Initialize the Helix OS kernel
///
/// This is the main entry point for kernel initialization. It:
/// 1. Validates the registry and builds the dependency graph
/// 2. Executes each phase in order with barriers between them
/// 3. Handles failures with automatic rollback
///
/// # Arguments
/// * `config` - Executor configuration
///
/// # Returns
/// * `Ok(())` if all subsystems initialized successfully
/// * `Err(InitError)` if any subsystem failed
///
/// # Example
/// ```rust,ignore
/// use helix_init::{initialize_kernel, ExecutorConfig};
///
/// fn kernel_main() -> ! {
///     let config = ExecutorConfig::default();
///
///     if let Err(e) = initialize_kernel(config) {
///         panic!("Kernel initialization failed: {:?}", e);
///     }
///
///     // Kernel is now fully initialized
///     kernel_main_loop();
/// }
/// ```
pub fn initialize_kernel(config: ExecutorConfig) -> InitResult<()> {
    // Record start time
    let start = get_timestamp();
    INIT_START_TIME.store(start, Ordering::SeqCst);

    // Update global state
    {
        let mut state = INIT_STATE.write();
        if state.in_progress {
            return Err(InitError::new(
                ErrorKind::AlreadyInitializing,
                "Initialization already in progress",
            ));
        }
        state.in_progress = true;
        state.state = InitializationState::InProgress;
    }

    // Build dependency graph
    let registry = registry::GLOBAL_REGISTRY.read();
    let graph = dependency::DependencyGraph::build(&registry)?;
    drop(registry);

    // Validate the graph (check for cycles, missing deps)
    graph.validate()?;

    // Create executor
    let mut executor = executor::InitExecutor::new(config);

    // Execute each phase
    let phases = [
        InitPhase::Boot,
        InitPhase::Early,
        InitPhase::Core,
        InitPhase::Late,
        InitPhase::Runtime,
    ];

    for phase in phases {
        CURRENT_PHASE.store(phase as u32, Ordering::SeqCst);

        let phase_start = get_timestamp();

        match executor.execute_phase(phase, &graph) {
            Ok(results) => {
                let phase_time = get_timestamp() - phase_start;

                let mut state = INIT_STATE.write();
                state.phase = phase;
                state.initialized_count += results.success_count;
                state.phase_times[phase as usize] = phase_time;
            },
            Err(e) => {
                // Rollback on failure
                let mut state = INIT_STATE.write();
                state.rolling_back = true;
                state.state = InitializationState::RollingBack;
                state.failed_count += 1;
                state.last_error = Some(e.clone());
                drop(state);

                // Execute rollback
                let _ = executor.rollback(&graph);

                // Update final state
                let mut state = INIT_STATE.write();
                state.rolling_back = false;
                state.state = InitializationState::Failed;
                state.in_progress = false;

                return Err(e);
            },
        }

        // Phase barrier - wait for all subsystems to complete
        phase::wait_for_barrier(phase)?;
    }

    // Mark complete
    let total_time = get_timestamp() - start;
    {
        let mut state = INIT_STATE.write();
        state.state = InitializationState::Complete;
        state.in_progress = false;
        state.total_time_us = total_time;
    }

    INIT_COMPLETE.store(true, Ordering::SeqCst);

    Ok(())
}

/// Shutdown the kernel in reverse order
///
/// Calls `shutdown()` on each subsystem in reverse initialization order.
pub fn shutdown_kernel() -> InitResult<()> {
    {
        let mut state = INIT_STATE.write();
        if state.state != InitializationState::Complete {
            return Err(InitError::new(
                ErrorKind::InvalidState,
                "Cannot shutdown: kernel not fully initialized",
            ));
        }
        state.state = InitializationState::ShuttingDown;
    }

    // Build graph and get reverse order
    let registry = registry::GLOBAL_REGISTRY.read();
    let graph = dependency::DependencyGraph::build(&registry)?;
    drop(registry);

    let order = graph.reverse_topological_order()?;

    // Shutdown each subsystem
    for id in order {
        let mut registry = registry::GLOBAL_REGISTRY.write();
        if let Some(entry) = registry.get_mut(&id) {
            if entry.state == SubsystemState::Initialized {
                let mut ctx = context::InitContext::new();
                let _ = entry.subsystem.shutdown(&mut ctx);
                entry.state = SubsystemState::Shutdown;
            }
        }
    }

    // Update state
    {
        let mut state = INIT_STATE.write();
        state.state = InitializationState::Shutdown;
    }

    INIT_COMPLETE.store(false, Ordering::SeqCst);

    Ok(())
}

/// Check if kernel is fully initialized
pub fn is_initialized() -> bool {
    INIT_COMPLETE.load(Ordering::SeqCst)
}

/// Get current initialization phase
pub fn current_phase() -> InitPhase {
    InitPhase::from_u32(CURRENT_PHASE.load(Ordering::SeqCst))
}

/// Get global initialization state
pub fn get_state() -> InitializationState {
    INIT_STATE.read().state
}

/// Get initialization statistics
pub fn get_stats() -> InitStats {
    let state = INIT_STATE.read();
    InitStats {
        state: state.state,
        phase: state.phase,
        registered: state.registered_count,
        initialized: state.initialized_count,
        failed: state.failed_count,
        total_time_us: state.total_time_us,
        phase_times: state.phase_times,
    }
}

/// Initialization statistics
#[derive(Debug, Clone)]
pub struct InitStats {
    pub state: InitializationState,
    pub phase: InitPhase,
    pub registered: usize,
    pub initialized: usize,
    pub failed: usize,
    pub total_time_us: u64,
    pub phase_times: [u64; 5],
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get current timestamp in microseconds
///
/// Platform-specific implementation
#[inline]
fn get_timestamp() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let tsc: u64;
        unsafe {
            core::arch::asm!(
                "rdtsc",
                "shl rdx, 32",
                "or rax, rdx",
                out("rax") tsc,
                out("rdx") _,
                options(nostack, nomem)
            );
        }
        // Assume 3GHz CPU, convert to microseconds
        tsc / 3000
    }

    #[cfg(target_arch = "aarch64")]
    {
        let cnt: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, CNTPCT_EL0",
                out(reg) cnt,
                options(nostack, nomem)
            );
        }
        // Assume 1MHz counter
        cnt
    }

    #[cfg(target_arch = "riscv64")]
    {
        let time: u64;
        unsafe {
            core::arch::asm!(
                "rdtime {}",
                out(reg) time,
                options(nostack, nomem)
            );
        }
        // Assume 10MHz timebase
        time / 10
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        0
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_state_initial() {
        let state = GlobalInitState::new();
        assert_eq!(state.state, InitializationState::NotStarted);
        assert_eq!(state.phase, InitPhase::Boot);
        assert_eq!(state.registered_count, 0);
        assert!(!state.in_progress);
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_constants() {
        assert!(MAX_SUBSYSTEMS >= 256);
        assert!(MAX_DEPENDENCY_DEPTH >= 32);
        assert!(DEFAULT_INIT_TIMEOUT_US >= 1_000_000);
    }
}
