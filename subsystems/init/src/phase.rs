//! # Initialization Phases
//!
//! This module defines the phases of kernel initialization and the barriers
//! between them. Phases ensure that subsystems are initialized in a predictable
//! order with clear synchronization points.
//!
//! ## Phase Diagram
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                        INITIALIZATION PHASES                                  │
//! │                                                                               │
//! │  ╔═══════════╗    ╔═══════════╗    ╔═══════════╗    ╔═══════════╗    ╔═══════════╗
//! │  ║   BOOT    ║───▶║   EARLY   ║───▶║   CORE    ║───▶║   LATE    ║───▶║  RUNTIME  ║
//! │  ╚═══════════╝    ╚═══════════╝    ╚═══════════╝    ╚═══════════╝    ╚═══════════╝
//! │       │                │                │                │                │
//! │       ▼                ▼                ▼                ▼                ▼
//! │  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐
//! │  │Firmware │      │ Memory  │      │Scheduler│      │ Drivers │      │Userland │
//! │  │Console  │      │   CPU   │      │   IPC   │      │   FS    │      │Services │
//! │  │BootInfo │      │Interrupt│      │ Timers  │      │ Network │      │HotReload│
//! │  └─────────┘      └─────────┘      └─────────┘      └─────────┘      └─────────┘
//! │       │                │                │                │                │
//! │       ▼                ▼                ▼                ▼                ▼
//! │  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐
//! │  │BARRIER 0│      │BARRIER 1│      │BARRIER 2│      │BARRIER 3│      │BARRIER 4│
//! │  │No heap  │      │Basic    │      │Full heap│      │Full     │      │Userspace│
//! │  │No alloc │      │heap only│      │Scheduler│      │services │      │ready    │
//! │  └─────────┘      └─────────┘      └─────────┘      └─────────┘      └─────────┘
//! │                                                                               │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Phase Environments
//!
//! Each phase has specific guarantees about what services are available:
//!
//! | Phase   | Heap | Interrupts | Scheduler | Drivers | Filesystems |
//! |---------|------|------------|-----------|---------|-------------|
//! | Boot    | ❌   | ❌         | ❌        | ❌      | ❌          |
//! | Early   | ✅*  | ❌         | ❌        | ❌      | ❌          |
//! | Core    | ✅   | ✅         | ✅*       | ❌      | ❌          |
//! | Late    | ✅   | ✅         | ✅        | ✅*     | ✅*         |
//! | Runtime | ✅   | ✅         | ✅        | ✅      | ✅          |
//!
//! Legend: ✅ = Available, ✅* = Being initialized, ❌ = Not available

use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::error::{ErrorKind, InitError, InitResult};

// =============================================================================
// PHASE DEFINITION
// =============================================================================

/// Initialization phase enumeration
///
/// Defines the major phases of kernel initialization. Each phase has different
/// guarantees about available services and represents a synchronization point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum InitPhase {
    /// Boot phase - minimal environment
    ///
    /// # Environment
    /// - No heap allocation
    /// - No interrupts
    /// - Only early console available
    /// - Stack-only memory
    ///
    /// # Typical Subsystems
    /// - Firmware handoff
    /// - Early console
    /// - Boot info parsing
    /// - CPU detection (basic)
    Boot    = 0,

    /// Early phase - basic memory available
    ///
    /// # Environment
    /// - Basic heap (bump allocator)
    /// - No interrupts yet
    /// - Physical memory manager ready
    /// - Virtual memory being set up
    ///
    /// # Typical Subsystems
    /// - Physical memory manager
    /// - Virtual memory manager
    /// - Kernel heap
    /// - CPU initialization
    /// - Interrupt setup (but not enabled)
    Early   = 1,

    /// Core phase - kernel services starting
    ///
    /// # Environment
    /// - Full heap available
    /// - Interrupts enabled
    /// - Basic scheduling available
    /// - Timer ticking
    ///
    /// # Typical Subsystems
    /// - Scheduler
    /// - IPC subsystem
    /// - Timer subsystem
    /// - SMP (secondary CPUs starting)
    Core    = 2,

    /// Late phase - device drivers and services
    ///
    /// # Environment
    /// - All core services available
    /// - Can block/sleep
    /// - Can use full threading
    /// - Can access devices
    ///
    /// # Typical Subsystems
    /// - Driver framework
    /// - Device enumeration
    /// - Filesystem mounting
    /// - Network stack
    /// - Security subsystem
    Late    = 3,

    /// Runtime phase - userland preparation
    ///
    /// # Environment
    /// - Full kernel functionality
    /// - Ready for userspace
    /// - Hot-reload enabled
    ///
    /// # Typical Subsystems
    /// - Init process launch
    /// - Service manager
    /// - Runtime configuration
    /// - Debug/tracing services
    Runtime = 4,
}

impl InitPhase {
    /// Get phase name as string
    pub const fn name(&self) -> &'static str {
        match self {
            InitPhase::Boot => "Boot",
            InitPhase::Early => "Early",
            InitPhase::Core => "Core",
            InitPhase::Late => "Late",
            InitPhase::Runtime => "Runtime",
        }
    }

    /// Get phase description
    pub const fn description(&self) -> &'static str {
        match self {
            InitPhase::Boot => "Firmware handoff and minimal setup",
            InitPhase::Early => "Memory initialization and CPU setup",
            InitPhase::Core => "Scheduler, IPC, and timers",
            InitPhase::Late => "Drivers, filesystems, and networking",
            InitPhase::Runtime => "Userland preparation and services",
        }
    }

    /// Get the next phase (or None if this is the last)
    pub const fn next(&self) -> Option<InitPhase> {
        match self {
            InitPhase::Boot => Some(InitPhase::Early),
            InitPhase::Early => Some(InitPhase::Core),
            InitPhase::Core => Some(InitPhase::Late),
            InitPhase::Late => Some(InitPhase::Runtime),
            InitPhase::Runtime => None,
        }
    }

    /// Get the previous phase (or None if this is the first)
    pub const fn previous(&self) -> Option<InitPhase> {
        match self {
            InitPhase::Boot => None,
            InitPhase::Early => Some(InitPhase::Boot),
            InitPhase::Core => Some(InitPhase::Early),
            InitPhase::Late => Some(InitPhase::Core),
            InitPhase::Runtime => Some(InitPhase::Late),
        }
    }

    /// Convert from u32
    pub const fn from_u32(value: u32) -> Self {
        match value {
            0 => InitPhase::Boot,
            1 => InitPhase::Early,
            2 => InitPhase::Core,
            3 => InitPhase::Late,
            4 => InitPhase::Runtime,
            _ => InitPhase::Boot,
        }
    }

    /// Get all phases in order
    pub const fn all() -> [InitPhase; 5] {
        [
            InitPhase::Boot,
            InitPhase::Early,
            InitPhase::Core,
            InitPhase::Late,
            InitPhase::Runtime,
        ]
    }

    /// Check if this phase has heap available
    pub const fn has_heap(&self) -> bool {
        !matches!(self, InitPhase::Boot)
    }

    /// Check if this phase has interrupts
    pub const fn has_interrupts(&self) -> bool {
        matches!(self, InitPhase::Core | InitPhase::Late | InitPhase::Runtime)
    }

    /// Check if this phase has scheduling
    pub const fn has_scheduler(&self) -> bool {
        matches!(self, InitPhase::Core | InitPhase::Late | InitPhase::Runtime)
    }

    /// Check if this phase can use blocking operations
    pub const fn can_block(&self) -> bool {
        matches!(self, InitPhase::Late | InitPhase::Runtime)
    }

    /// Check if this phase supports hot-reload
    pub const fn supports_hot_reload(&self) -> bool {
        matches!(self, InitPhase::Runtime)
    }

    /// Get phase capabilities as flags
    pub fn capabilities(&self) -> PhaseCapabilities {
        match self {
            InitPhase::Boot => PhaseCapabilities::CONSOLE,
            InitPhase::Early => {
                PhaseCapabilities::CONSOLE | PhaseCapabilities::HEAP | PhaseCapabilities::MEMORY
            },
            InitPhase::Core => {
                PhaseCapabilities::CONSOLE
                    | PhaseCapabilities::HEAP
                    | PhaseCapabilities::MEMORY
                    | PhaseCapabilities::INTERRUPTS
                    | PhaseCapabilities::SCHEDULER
                    | PhaseCapabilities::TIMERS
            },
            InitPhase::Late => PhaseCapabilities::all() - PhaseCapabilities::HOT_RELOAD,
            InitPhase::Runtime => PhaseCapabilities::all(),
        }
    }
}

impl fmt::Display for InitPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for InitPhase {
    fn default() -> Self {
        InitPhase::Boot
    }
}

/// Phase order constant array
pub const PHASE_ORDER: [InitPhase; 5] = InitPhase::all();

// =============================================================================
// PHASE CAPABILITIES
// =============================================================================

use bitflags::bitflags;

bitflags! {
    /// Capabilities available during each phase
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PhaseCapabilities: u32 {
        /// Early console available
        const CONSOLE = 1 << 0;
        /// Heap allocation available
        const HEAP = 1 << 1;
        /// Physical/virtual memory management
        const MEMORY = 1 << 2;
        /// Interrupts enabled
        const INTERRUPTS = 1 << 3;
        /// Scheduler running
        const SCHEDULER = 1 << 4;
        /// Timers available
        const TIMERS = 1 << 5;
        /// IPC available
        const IPC = 1 << 6;
        /// Drivers available
        const DRIVERS = 1 << 7;
        /// Filesystems mounted
        const FILESYSTEMS = 1 << 8;
        /// Network stack available
        const NETWORK = 1 << 9;
        /// Security subsystem active
        const SECURITY = 1 << 10;
        /// Userspace ready
        const USERSPACE = 1 << 11;
        /// Hot-reload supported
        const HOT_RELOAD = 1 << 12;
        /// Debug/tracing available
        const DEBUG = 1 << 13;
    }
}

// =============================================================================
// PHASE STATE
// =============================================================================

/// State of a specific phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PhaseState {
    /// Phase not yet started
    Pending  = 0,
    /// Phase currently executing
    Running  = 1,
    /// Phase completed successfully
    Complete = 2,
    /// Phase failed
    Failed   = 3,
    /// Phase skipped (conditional)
    Skipped  = 4,
}

impl PhaseState {
    /// Check if phase is done (complete, failed, or skipped)
    pub fn is_done(&self) -> bool {
        matches!(
            self,
            PhaseState::Complete | PhaseState::Failed | PhaseState::Skipped
        )
    }

    /// Check if phase completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, PhaseState::Complete)
    }
}

// =============================================================================
// PHASE BARRIER
// =============================================================================

/// Synchronization barrier between phases
///
/// Ensures all subsystems in a phase complete before the next phase begins.
/// Supports both blocking (for non-parallel) and spin-wait (for early boot).
#[derive(Debug)]
pub struct PhaseBarrier {
    /// Phase this barrier guards
    phase: InitPhase,

    /// Total subsystems expected in this phase
    expected: AtomicU32,

    /// Subsystems that have completed
    completed: AtomicU32,

    /// Subsystems that have failed
    failed: AtomicU32,

    /// Barrier has been released
    released: AtomicBool,

    /// Start timestamp (microseconds)
    start_time: AtomicU64,

    /// End timestamp (microseconds)
    end_time: AtomicU64,
}

impl PhaseBarrier {
    /// Create new barrier for a phase
    pub const fn new(phase: InitPhase) -> Self {
        Self {
            phase,
            expected: AtomicU32::new(0),
            completed: AtomicU32::new(0),
            failed: AtomicU32::new(0),
            released: AtomicBool::new(false),
            start_time: AtomicU64::new(0),
            end_time: AtomicU64::new(0),
        }
    }

    /// Set the number of expected subsystems
    pub fn set_expected(&self, count: u32) {
        self.expected.store(count, Ordering::SeqCst);
    }

    /// Mark one subsystem as complete
    pub fn mark_complete(&self) {
        let prev = self.completed.fetch_add(1, Ordering::SeqCst);
        self.check_release(prev + 1);
    }

    /// Mark one subsystem as failed
    pub fn mark_failed(&self) {
        self.failed.fetch_add(1, Ordering::SeqCst);
        // Failed subsystem also counts as "processed"
        let prev = self.completed.fetch_add(1, Ordering::SeqCst);
        self.check_release(prev + 1);
    }

    /// Check if barrier should be released
    fn check_release(&self, completed: u32) {
        let expected = self.expected.load(Ordering::SeqCst);
        if completed >= expected {
            self.released.store(true, Ordering::SeqCst);
        }
    }

    /// Wait for barrier to be released
    ///
    /// This is a spin-wait, suitable for early boot when scheduler isn't available.
    pub fn wait(&self) -> InitResult<()> {
        // Fast path: already released
        if self.released.load(Ordering::SeqCst) {
            return self.check_result();
        }

        // Spin wait
        let mut spin_count = 0u64;
        const MAX_SPINS: u64 = 1_000_000_000; // 1 billion spins before timeout

        while !self.released.load(Ordering::SeqCst) {
            core::hint::spin_loop();
            spin_count += 1;

            if spin_count > MAX_SPINS {
                return Err(
                    InitError::new(ErrorKind::Timeout, "Phase barrier wait timed out")
                        .with_phase(self.phase),
                );
            }
        }

        self.check_result()
    }

    /// Wait for barrier with blocking (when scheduler available)
    #[cfg(feature = "parallel")]
    pub fn wait_blocking(&self) -> InitResult<()> {
        // TODO: Use actual blocking primitive when scheduler is available
        self.wait()
    }

    /// Check if any subsystems failed
    fn check_result(&self) -> InitResult<()> {
        let failed = self.failed.load(Ordering::SeqCst);
        if failed > 0 {
            Err(InitError::new(
                ErrorKind::SubsystemFailed,
                "One or more subsystems failed during phase",
            )
            .with_phase(self.phase))
        } else {
            Ok(())
        }
    }

    /// Check if barrier is released
    pub fn is_released(&self) -> bool {
        self.released.load(Ordering::SeqCst)
    }

    /// Get completion count
    pub fn completed(&self) -> u32 {
        self.completed.load(Ordering::SeqCst)
    }

    /// Get failure count
    pub fn failed(&self) -> u32 {
        self.failed.load(Ordering::SeqCst)
    }

    /// Get expected count
    pub fn expected(&self) -> u32 {
        self.expected.load(Ordering::SeqCst)
    }

    /// Get phase
    pub fn phase(&self) -> InitPhase {
        self.phase
    }

    /// Record start time
    pub fn record_start(&self, time: u64) {
        self.start_time.store(time, Ordering::SeqCst);
    }

    /// Record end time
    pub fn record_end(&self, time: u64) {
        self.end_time.store(time, Ordering::SeqCst);
    }

    /// Get phase duration in microseconds
    pub fn duration_us(&self) -> u64 {
        let end = self.end_time.load(Ordering::SeqCst);
        let start = self.start_time.load(Ordering::SeqCst);
        if end > start {
            end - start
        } else {
            0
        }
    }

    /// Reset barrier for reuse
    pub fn reset(&self) {
        self.expected.store(0, Ordering::SeqCst);
        self.completed.store(0, Ordering::SeqCst);
        self.failed.store(0, Ordering::SeqCst);
        self.released.store(false, Ordering::SeqCst);
        self.start_time.store(0, Ordering::SeqCst);
        self.end_time.store(0, Ordering::SeqCst);
    }
}

// =============================================================================
// GLOBAL BARRIERS
// =============================================================================

/// Global phase barriers (one per phase)
static PHASE_BARRIERS: [PhaseBarrier; 5] = [
    PhaseBarrier::new(InitPhase::Boot),
    PhaseBarrier::new(InitPhase::Early),
    PhaseBarrier::new(InitPhase::Core),
    PhaseBarrier::new(InitPhase::Late),
    PhaseBarrier::new(InitPhase::Runtime),
];

/// Get barrier for a phase
pub fn get_barrier(phase: InitPhase) -> &'static PhaseBarrier {
    &PHASE_BARRIERS[phase as usize]
}

/// Wait for a phase barrier
pub fn wait_for_barrier(phase: InitPhase) -> InitResult<()> {
    get_barrier(phase).wait()
}

/// Mark subsystem complete in phase
pub fn mark_phase_complete(phase: InitPhase) {
    get_barrier(phase).mark_complete();
}

/// Mark subsystem failed in phase
pub fn mark_phase_failed(phase: InitPhase) {
    get_barrier(phase).mark_failed();
}

/// Set expected subsystem count for phase
pub fn set_phase_expected(phase: InitPhase, count: u32) {
    get_barrier(phase).set_expected(count);
}

/// Reset all barriers
pub fn reset_all_barriers() {
    for barrier in &PHASE_BARRIERS {
        barrier.reset();
    }
}

// =============================================================================
// PHASE TRANSITION
// =============================================================================

/// Phase transition validator
///
/// Ensures phase transitions follow the correct order and all prerequisites
/// are met before advancing.
pub struct PhaseTransition {
    /// Current phase
    current: InitPhase,

    /// Phase states
    states: [PhaseState; 5],
}

impl PhaseTransition {
    /// Create new transition tracker
    pub const fn new() -> Self {
        Self {
            current: InitPhase::Boot,
            states: [PhaseState::Pending; 5],
        }
    }

    /// Get current phase
    pub fn current(&self) -> InitPhase {
        self.current
    }

    /// Get state of a phase
    pub fn state(&self, phase: InitPhase) -> PhaseState {
        self.states[phase as usize]
    }

    /// Begin a phase
    pub fn begin_phase(&mut self, phase: InitPhase) -> InitResult<()> {
        // Check phase is valid to begin
        if let Some(prev) = phase.previous() {
            if self.states[prev as usize] != PhaseState::Complete {
                return Err(
                    InitError::new(ErrorKind::InvalidState, "Previous phase not complete")
                        .with_phase(phase),
                );
            }
        }

        // Check phase hasn't already run
        if self.states[phase as usize] != PhaseState::Pending {
            return Err(InitError::new(
                ErrorKind::InvalidState,
                "Phase already started or complete",
            )
            .with_phase(phase));
        }

        self.states[phase as usize] = PhaseState::Running;
        self.current = phase;

        Ok(())
    }

    /// Complete a phase
    pub fn complete_phase(&mut self, phase: InitPhase) -> InitResult<()> {
        if self.states[phase as usize] != PhaseState::Running {
            return Err(
                InitError::new(ErrorKind::InvalidState, "Phase not running").with_phase(phase)
            );
        }

        self.states[phase as usize] = PhaseState::Complete;

        Ok(())
    }

    /// Fail a phase
    pub fn fail_phase(&mut self, phase: InitPhase) {
        self.states[phase as usize] = PhaseState::Failed;
    }

    /// Skip a phase
    pub fn skip_phase(&mut self, phase: InitPhase) {
        self.states[phase as usize] = PhaseState::Skipped;
    }

    /// Check if all phases are complete
    pub fn all_complete(&self) -> bool {
        self.states.iter().all(|s| s.is_success())
    }

    /// Check if any phase failed
    pub fn any_failed(&self) -> bool {
        self.states.iter().any(|s| matches!(s, PhaseState::Failed))
    }

    /// Get phases in order with their states
    pub fn phases_with_states(&self) -> [(InitPhase, PhaseState); 5] {
        [
            (InitPhase::Boot, self.states[0]),
            (InitPhase::Early, self.states[1]),
            (InitPhase::Core, self.states[2]),
            (InitPhase::Late, self.states[3]),
            (InitPhase::Runtime, self.states[4]),
        ]
    }
}

impl Default for PhaseTransition {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PHASE REQUIREMENTS
// =============================================================================

/// Requirements for a phase to begin
#[derive(Debug, Clone)]
pub struct PhaseRequirements {
    /// Required capabilities from previous phases
    pub required_capabilities: PhaseCapabilities,

    /// Minimum subsystems that must be initialized
    pub min_subsystems: usize,

    /// Required subsystems by name
    pub required_subsystems: &'static [&'static str],

    /// Optional subsystems that are checked but not required
    pub optional_subsystems: &'static [&'static str],
}

impl PhaseRequirements {
    /// Get requirements for a specific phase
    pub const fn for_phase(phase: InitPhase) -> Self {
        match phase {
            InitPhase::Boot => Self {
                required_capabilities: PhaseCapabilities::empty(),
                min_subsystems: 0,
                required_subsystems: &[],
                optional_subsystems: &["early_console"],
            },
            InitPhase::Early => Self {
                required_capabilities: PhaseCapabilities::CONSOLE,
                min_subsystems: 1,
                required_subsystems: &["firmware", "boot_info"],
                optional_subsystems: &[],
            },
            InitPhase::Core => Self {
                required_capabilities: PhaseCapabilities::CONSOLE
                    .union(PhaseCapabilities::HEAP)
                    .union(PhaseCapabilities::MEMORY),
                min_subsystems: 3,
                required_subsystems: &["pmm", "vmm", "heap", "cpu", "interrupts"],
                optional_subsystems: &[],
            },
            InitPhase::Late => Self {
                required_capabilities: PhaseCapabilities::CONSOLE
                    .union(PhaseCapabilities::HEAP)
                    .union(PhaseCapabilities::MEMORY)
                    .union(PhaseCapabilities::INTERRUPTS)
                    .union(PhaseCapabilities::SCHEDULER),
                min_subsystems: 5,
                required_subsystems: &["scheduler", "ipc", "timers"],
                optional_subsystems: &["smp"],
            },
            InitPhase::Runtime => Self {
                required_capabilities: PhaseCapabilities::all()
                    .difference(PhaseCapabilities::USERSPACE)
                    .difference(PhaseCapabilities::HOT_RELOAD),
                min_subsystems: 8,
                required_subsystems: &["drivers", "vfs"],
                optional_subsystems: &["network", "security"],
            },
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_order() {
        assert!(InitPhase::Boot < InitPhase::Early);
        assert!(InitPhase::Early < InitPhase::Core);
        assert!(InitPhase::Core < InitPhase::Late);
        assert!(InitPhase::Late < InitPhase::Runtime);
    }

    #[test]
    fn test_phase_next() {
        assert_eq!(InitPhase::Boot.next(), Some(InitPhase::Early));
        assert_eq!(InitPhase::Runtime.next(), None);
    }

    #[test]
    fn test_phase_previous() {
        assert_eq!(InitPhase::Boot.previous(), None);
        assert_eq!(InitPhase::Runtime.previous(), Some(InitPhase::Late));
    }

    #[test]
    fn test_phase_capabilities() {
        assert!(!InitPhase::Boot.has_heap());
        assert!(InitPhase::Early.has_heap());
        assert!(!InitPhase::Early.has_interrupts());
        assert!(InitPhase::Core.has_interrupts());
        assert!(InitPhase::Late.can_block());
        assert!(InitPhase::Runtime.supports_hot_reload());
    }

    #[test]
    fn test_barrier_basic() {
        let barrier = PhaseBarrier::new(InitPhase::Early);
        barrier.set_expected(2);

        assert_eq!(barrier.expected(), 2);
        assert_eq!(barrier.completed(), 0);
        assert!(!barrier.is_released());

        barrier.mark_complete();
        assert_eq!(barrier.completed(), 1);
        assert!(!barrier.is_released());

        barrier.mark_complete();
        assert_eq!(barrier.completed(), 2);
        assert!(barrier.is_released());
    }

    #[test]
    fn test_barrier_with_failure() {
        let barrier = PhaseBarrier::new(InitPhase::Core);
        barrier.set_expected(2);

        barrier.mark_complete();
        barrier.mark_failed();

        assert!(barrier.is_released());
        assert_eq!(barrier.failed(), 1);
        assert!(barrier.check_result().is_err());
    }

    #[test]
    fn test_phase_transition() {
        let mut transition = PhaseTransition::new();

        assert!(transition.begin_phase(InitPhase::Boot).is_ok());
        assert!(transition.complete_phase(InitPhase::Boot).is_ok());

        assert!(transition.begin_phase(InitPhase::Early).is_ok());

        // Can't start Core before Early completes
        let mut t2 = PhaseTransition::new();
        t2.states[0] = PhaseState::Complete;
        assert!(t2.begin_phase(InitPhase::Core).is_err());
    }

    #[test]
    fn test_phase_all() {
        let phases = InitPhase::all();
        assert_eq!(phases.len(), 5);
        assert_eq!(phases[0], InitPhase::Boot);
        assert_eq!(phases[4], InitPhase::Runtime);
    }
}
