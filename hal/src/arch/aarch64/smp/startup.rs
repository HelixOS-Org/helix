//! # Secondary CPU Startup
//!
//! This module handles bringing secondary CPUs (Application Processors) online
//! using either PSCI or platform-specific methods like spin tables.
//!
//! ## Boot Flow
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                     Secondary CPU Startup Flow                         │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │                                                                        │
//! │   BSP (CPU 0)                          AP (CPU N)                      │
//! │   ──────────                           ──────────                      │
//! │                                                                        │
//! │   1. Initialize core systems                                           │
//! │      (GIC, MMU, timers, etc.)                                          │
//! │                                                                        │
//! │   2. For each AP:                                                      │
//! │      ┌─────────────────────────┐                                       │
//! │      │ Allocate AP stack       │                                       │
//! │      │ Initialize per-CPU data │                                       │
//! │      │ Call PSCI CPU_ON        │───────────▶ Power on                  │
//! │      └─────────────────────────┘             │                         │
//! │                                              ▼                         │
//! │                                        3. AP starts at                 │
//! │                                           entry point                  │
//! │                                              │                         │
//! │                                              ▼                         │
//! │                                        4. Set up MMU                   │
//! │                                           with shared                  │
//! │                                           page tables                  │
//! │                                              │                         │
//! │                                              ▼                         │
//! │                                        5. Initialize                   │
//! │                                           local GIC                    │
//! │                                           (redistributor)              │
//! │                                              │                         │
//! │                                              ▼                         │
//! │                                        6. Initialize                   │
//! │   5. Wait for AP ready    ◀────────────   per-CPU data                │
//! │      signal                               and signal                   │
//! │                                           ready                        │
//! │                                              │                         │
//! │   6. Continue to next AP                     ▼                         │
//! │                                        7. Enter idle                   │
//! │                                           or scheduler                 │
//! │                                                                        │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Platform Support
//!
//! | Platform      | Method        | Notes                                 |
//! |---------------|---------------|---------------------------------------|
//! | QEMU virt     | PSCI          | Standard, works out of box            |
//! | Raspberry Pi  | Spin Table    | Uses mailbox at 0xD8/0xE0/0xE8/0xF0   |
//! | ARM FVP       | PSCI          | Full PSCI support                     |
//! | Server SoCs   | PSCI/ACPI     | ACPI MADT + PSCI                      |

use core::arch::asm;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::mpidr::Mpidr;
use super::percpu::{self, PerCpuData};
use super::psci::{self, Psci, PsciError};
use super::{CpuState, CpuTopology, SmpError, DEFAULT_AP_STACK_SIZE, MAX_CPUS};

// ============================================================================
// AP Startup State
// ============================================================================

/// AP startup state for synchronization
#[repr(C, align(64))] // Cache line aligned
pub struct ApStartupState {
    /// CPU ID being started
    pub cpu_id: AtomicU32,
    /// Entry point address
    pub entry_point: AtomicU64,
    /// Stack pointer for the AP
    pub stack_ptr: AtomicU64,
    /// Page table base (TTBR0 or TTBR1)
    pub page_table: AtomicU64,
    /// Ready flag (AP sets when initialized)
    pub ready: AtomicU32,
    /// Error code (if startup failed)
    pub error: AtomicU32,
    /// Context data passed to AP
    pub context: AtomicU64,
}

impl ApStartupState {
    /// Create a new startup state
    pub const fn new() -> Self {
        Self {
            cpu_id: AtomicU32::new(0xFFFF_FFFF),
            entry_point: AtomicU64::new(0),
            stack_ptr: AtomicU64::new(0),
            page_table: AtomicU64::new(0),
            ready: AtomicU32::new(0),
            error: AtomicU32::new(0),
            context: AtomicU64::new(0),
        }
    }

    /// Reset state for a new CPU
    pub fn reset(&self, cpu_id: u32, entry: u64, stack: u64) {
        self.ready.store(0, Ordering::Release);
        self.error.store(0, Ordering::Release);
        self.stack_ptr.store(stack, Ordering::Release);
        self.entry_point.store(entry, Ordering::Release);
        self.cpu_id.store(cpu_id, Ordering::Release);
    }

    /// Wait for the AP to signal ready
    pub fn wait_ready(&self, timeout_us: u64) -> Result<(), SmpError> {
        let start = read_counter();
        let freq = read_counter_freq();
        let timeout_ticks = (timeout_us * freq) / 1_000_000;

        while self.ready.load(Ordering::Acquire) == 0 {
            if read_counter() - start > timeout_ticks {
                return Err(SmpError::Timeout);
            }
            core::hint::spin_loop();
        }

        if self.error.load(Ordering::Acquire) != 0 {
            Err(SmpError::PsciFailed(
                self.error.load(Ordering::Acquire) as i32
            ))
        } else {
            Ok(())
        }
    }

    /// Signal that the AP is ready
    pub fn signal_ready(&self) {
        // Memory barrier before signaling
        core::sync::atomic::fence(Ordering::Release);
        self.ready.store(1, Ordering::Release);
    }

    /// Signal an error
    pub fn signal_error(&self, code: u32) {
        self.error.store(code, Ordering::Release);
        self.ready.store(1, Ordering::Release);
    }
}

impl Default for ApStartupState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global AP startup state
static AP_STARTUP: ApStartupState = ApStartupState::new();

// ============================================================================
// Counter Access
// ============================================================================

/// Read the system counter
#[inline]
fn read_counter() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntvct_el0", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read the counter frequency
#[inline]
fn read_counter_freq() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) value, options(nomem, nostack));
    }
    value
}

// ============================================================================
// CPU Startup
// ============================================================================

/// Start a secondary CPU using PSCI
///
/// # Arguments
///
/// - `cpu_id`: Linear CPU ID
/// - `mpidr`: Target CPU's MPIDR
/// - `psci`: PSCI interface
/// - `entry`: Physical address of entry point
/// - `stack_top`: Stack top address for the AP
///
/// # Returns
///
/// Ok(()) on success, or an error if startup failed.
pub fn start_cpu_psci(
    cpu_id: u32,
    mpidr: Mpidr,
    psci: &Psci,
    entry: u64,
    stack_top: u64,
) -> Result<(), SmpError> {
    // Set up startup state
    AP_STARTUP.reset(cpu_id, entry, stack_top);

    // Issue PSCI CPU_ON
    // Context ID (x0 on AP) will be the CPU ID
    let result = psci.cpu_on(mpidr, entry, cpu_id as u64);

    match result {
        Ok(()) => {
            // Wait for AP to signal ready (10 second timeout)
            AP_STARTUP.wait_ready(10_000_000)
        },
        Err(PsciError::AlreadyOn) => {
            // CPU is already on, this might be okay
            Err(SmpError::AlreadyOnline)
        },
        Err(e) => Err(SmpError::PsciFailed(e as i32)),
    }
}

/// Start a secondary CPU using spin table
///
/// This is used on platforms like Raspberry Pi that don't use PSCI.
///
/// # Arguments
///
/// - `cpu_id`: Linear CPU ID
/// - `mailbox_addr`: Address of the spin table mailbox for this CPU
/// - `entry`: Entry point address
/// - `stack_top`: Stack top address for the AP
pub fn start_cpu_spin_table(
    cpu_id: u32,
    mailbox_addr: usize,
    entry: u64,
    stack_top: u64,
) -> Result<(), SmpError> {
    AP_STARTUP.reset(cpu_id, entry, stack_top);

    // Write entry point to mailbox
    unsafe {
        let mailbox = mailbox_addr as *mut u64;

        // Memory barrier before write
        asm!("dmb sy", options(nomem, nostack));

        // Write entry point
        core::ptr::write_volatile(mailbox, entry);

        // Clean cache for the mailbox address
        asm!(
            "dc cvac, {addr}",
            "dsb sy",
            "sev",
            addr = in(reg) mailbox_addr,
            options(nomem, nostack),
        );
    }

    // Wait for AP to signal ready
    AP_STARTUP.wait_ready(10_000_000)
}

// ============================================================================
// AP Entry Point
// ============================================================================

/// Low-level AP entry point (called from assembly)
///
/// This is the first Rust code executed on a secondary CPU after
/// basic setup (stack, MMU if needed).
///
/// # Safety
///
/// Must only be called from AP startup assembly.
#[no_mangle]
pub unsafe extern "C" fn ap_rust_entry(cpu_id: u64) -> ! {
    let cpu_id = cpu_id as u32;

    // Initialize per-CPU data
    percpu::init_percpu_ap(cpu_id);

    // Initialize local GIC (redistributor)
    // This would call into the GIC module
    // gic::init_cpu();

    // Initialize local timer
    // timer::init_cpu();

    // Signal ready
    AP_STARTUP.signal_ready();

    // Enter the scheduler or idle loop
    ap_idle_loop()
}

/// AP idle loop
fn ap_idle_loop() -> ! {
    loop {
        // Wait for event (interrupt or SEV)
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }

        // Check for work (reschedule, IPI, etc.)
        if let Some(percpu) = PerCpuData::try_current() {
            if percpu.need_resched {
                percpu.need_resched = false;
                // Would call scheduler here
            }
        }
    }
}

// ============================================================================
// Startup Manager
// ============================================================================

/// Configuration for starting secondary CPUs
#[derive(Debug, Clone)]
pub struct StartupConfig {
    /// Stack size for each AP
    pub stack_size: usize,
    /// Use PSCI for startup
    pub use_psci: bool,
    /// PSCI interface
    pub psci: Option<Psci>,
    /// Spin table mailbox addresses (for non-PSCI platforms)
    pub spin_table_addrs: [u64; MAX_CPUS],
    /// AP entry point (physical address)
    pub entry_point: u64,
    /// Timeout for each CPU start (microseconds)
    pub timeout_us: u64,
}

impl StartupConfig {
    /// Create default config for PSCI systems
    pub fn psci_default(entry_point: u64) -> Self {
        Self {
            stack_size: DEFAULT_AP_STACK_SIZE,
            use_psci: true,
            psci: Some(Psci::smc()),
            spin_table_addrs: [0; MAX_CPUS],
            entry_point,
            timeout_us: 10_000_000, // 10 seconds
        }
    }

    /// Create config for Raspberry Pi (spin table)
    pub fn rpi_spin_table(entry_point: u64) -> Self {
        let mut addrs = [0u64; MAX_CPUS];
        // RPi4 mailbox addresses
        addrs[0] = 0xD8; // CPU 0 (unused, BSP)
        addrs[1] = 0xE0; // CPU 1
        addrs[2] = 0xE8; // CPU 2
        addrs[3] = 0xF0; // CPU 3

        Self {
            stack_size: DEFAULT_AP_STACK_SIZE,
            use_psci: false,
            psci: None,
            spin_table_addrs: addrs,
            entry_point,
            timeout_us: 5_000_000, // 5 seconds
        }
    }
}

/// Manager for bringing up secondary CPUs
pub struct StartupManager {
    config: StartupConfig,
    started_count: AtomicU32,
}

impl StartupManager {
    /// Create a new startup manager
    pub const fn new(config: StartupConfig) -> Self {
        Self {
            config,
            started_count: AtomicU32::new(1), // BSP is already started
        }
    }

    /// Start all secondary CPUs
    pub fn start_all(&self, topology: &mut CpuTopology) -> Result<u32, SmpError> {
        let mut started = 0u32;

        for cpu_id in 1..topology.num_cpus {
            if let Some(cpu_info) = topology.get_cpu(cpu_id) {
                if cpu_info.state == CpuState::Offline {
                    match self.start_cpu(cpu_id, cpu_info.mpidr, topology) {
                        Ok(()) => {
                            started += 1;
                        },
                        Err(e) => {
                            // Log error but continue with other CPUs
                            // In a real kernel: log::warn!("Failed to start CPU {}: {:?}", cpu_id, e);
                            let _ = e;
                        },
                    }
                }
            }
        }

        self.started_count.fetch_add(started, Ordering::SeqCst);
        Ok(started)
    }

    /// Start a specific CPU
    fn start_cpu(
        &self,
        cpu_id: u32,
        mpidr: Mpidr,
        topology: &mut CpuTopology,
    ) -> Result<(), SmpError> {
        // Allocate stack (in a real kernel, this would use the memory allocator)
        let stack_top = self.allocate_stack(cpu_id)?;

        // Update CPU state
        if let Some(cpu_info) = topology.get_cpu_mut(cpu_id) {
            cpu_info.state = CpuState::Starting;
            cpu_info.stack_base = Some(stack_top.saturating_sub(self.config.stack_size));
        }

        // Start the CPU
        let result = if self.config.use_psci {
            if let Some(ref psci) = self.config.psci {
                start_cpu_psci(
                    cpu_id,
                    mpidr,
                    psci,
                    self.config.entry_point,
                    stack_top as u64,
                )
            } else {
                Err(SmpError::NotSupported)
            }
        } else {
            let mailbox = self.config.spin_table_addrs[cpu_id as usize];
            if mailbox != 0 {
                start_cpu_spin_table(
                    cpu_id,
                    mailbox as usize,
                    self.config.entry_point,
                    stack_top as u64,
                )
            } else {
                Err(SmpError::NotSupported)
            }
        };

        // Update state based on result
        if let Some(cpu_info) = topology.get_cpu_mut(cpu_id) {
            cpu_info.state = match &result {
                Ok(()) => CpuState::Online,
                Err(_) => CpuState::Offline,
            };
        }

        result
    }

    /// Allocate a stack for an AP
    ///
    /// In a real kernel, this would use the memory allocator.
    fn allocate_stack(&self, cpu_id: u32) -> Result<usize, SmpError> {
        // Placeholder: In reality, allocate from physical memory
        // Each CPU gets a separate stack region

        // Example: Stack region starts at some address and grows down
        // This is just a placeholder calculation
        let base_stack_region = 0xFFFF_0000_0010_0000usize; // Example address
        let stack_top = base_stack_region + ((cpu_id as usize + 1) * self.config.stack_size);

        Ok(stack_top)
    }

    /// Get the number of online CPUs
    pub fn online_count(&self) -> u32 {
        self.started_count.load(Ordering::SeqCst)
    }
}

// ============================================================================
// Assembly Entry Point (placeholder documentation)
// ============================================================================

/// Assembly entry point for APs
///
/// This would be implemented in assembly and should:
///
/// 1. Disable interrupts
/// 2. Set up stack pointer from startup state
/// 3. Enable MMU with shared page tables
/// 4. Enable caches
/// 5. Call `ap_rust_entry` with CPU ID
///
/// Example assembly (pseudocode):
///
/// ```asm
/// .global ap_entry_asm
/// ap_entry_asm:
///     // x0 contains CPU ID (context from PSCI)
///
///     // Disable interrupts
///     msr daifset, #0xf
///
///     // Load stack pointer from startup state
///     adrp x1, AP_STARTUP
///     add x1, x1, :lo12:AP_STARTUP
///     ldr x2, [x1, #16]  // stack_ptr offset
///     mov sp, x2
///
///     // Set up MMU (load page tables, etc.)
///     ldr x2, [x1, #24]  // page_table offset
///     msr ttbr0_el1, x2
///     isb
///
///     // Enable MMU
///     mrs x2, sctlr_el1
///     orr x2, x2, #1     // M bit
///     orr x2, x2, #4     // C bit
///     orr x2, x2, #(1 << 12) // I bit
///     msr sctlr_el1, x2
///     isb
///
///     // Call Rust entry
///     bl ap_rust_entry
/// ```
#[doc(hidden)]
pub const _AP_ENTRY_ASM_DOC: () = ();
