//! # AArch64 PSCI (Power State Coordination Interface) Driver
//!
//! Implements PSCI for power management and SMP boot.
//! Supports PSCI 0.2+ specification.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// PSCI FUNCTION IDs (SMCCC calling convention)
// =============================================================================

/// PSCI version
pub const PSCI_VERSION: u32 = 0x84000000;
/// CPU suspend (32-bit)
pub const CPU_SUSPEND_32: u32 = 0x84000001;
/// CPU off
pub const CPU_OFF: u32 = 0x84000002;
/// CPU on (32-bit)
pub const CPU_ON_32: u32 = 0x84000003;
/// Affinity info (32-bit)
pub const AFFINITY_INFO_32: u32 = 0x84000004;
/// Migrate (32-bit)
pub const MIGRATE_32: u32 = 0x84000005;
/// Migrate info type
pub const MIGRATE_INFO_TYPE: u32 = 0x84000006;
/// Migrate info up CPU (32-bit)
pub const MIGRATE_INFO_UP_CPU_32: u32 = 0x84000007;
/// System off
pub const SYSTEM_OFF: u32 = 0x84000008;
/// System reset
pub const SYSTEM_RESET: u32 = 0x84000009;
/// PSCI features
pub const PSCI_FEATURES: u32 = 0x8400000A;
/// CPU freeze
pub const CPU_FREEZE: u32 = 0x8400000B;
/// CPU default suspend (32-bit)
pub const CPU_DEFAULT_SUSPEND_32: u32 = 0x8400000C;
/// Node hardware state (32-bit)
pub const NODE_HW_STATE_32: u32 = 0x8400000D;
/// System suspend (32-bit)
pub const SYSTEM_SUSPEND_32: u32 = 0x8400000E;
/// PSCI set suspend mode
pub const PSCI_SET_SUSPEND_MODE: u32 = 0x8400000F;
/// PSCI stats residency (32-bit)
pub const PSCI_STAT_RESIDENCY_32: u32 = 0x84000010;
/// PSCI stats count (32-bit)
pub const PSCI_STAT_COUNT_32: u32 = 0x84000011;
/// System reset 2 (32-bit)
pub const SYSTEM_RESET2_32: u32 = 0x84000012;
/// Memory protect
pub const MEM_PROTECT: u32 = 0x84000013;
/// Memory protect check range (32-bit)
pub const MEM_PROTECT_CHECK_RANGE_32: u32 = 0x84000014;

// 64-bit function IDs (SMC64)
/// CPU suspend (64-bit)
pub const CPU_SUSPEND_64: u32 = 0xC4000001;
/// CPU on (64-bit)
pub const CPU_ON_64: u32 = 0xC4000003;
/// Affinity info (64-bit)
pub const AFFINITY_INFO_64: u32 = 0xC4000004;
/// Migrate (64-bit)
pub const MIGRATE_64: u32 = 0xC4000005;
/// Migrate info up CPU (64-bit)
pub const MIGRATE_INFO_UP_CPU_64: u32 = 0xC4000007;
/// CPU default suspend (64-bit)
pub const CPU_DEFAULT_SUSPEND_64: u32 = 0xC400000C;
/// Node hardware state (64-bit)
pub const NODE_HW_STATE_64: u32 = 0xC400000D;
/// System suspend (64-bit)
pub const SYSTEM_SUSPEND_64: u32 = 0xC400000E;
/// PSCI stats residency (64-bit)
pub const PSCI_STAT_RESIDENCY_64: u32 = 0xC4000010;
/// PSCI stats count (64-bit)
pub const PSCI_STAT_COUNT_64: u32 = 0xC4000011;
/// System reset 2 (64-bit)
pub const SYSTEM_RESET2_64: u32 = 0xC4000012;
/// Memory protect check range (64-bit)
pub const MEM_PROTECT_CHECK_RANGE_64: u32 = 0xC4000014;

// =============================================================================
// PSCI RETURN VALUES
// =============================================================================

/// Success
pub const PSCI_SUCCESS: i32 = 0;
/// Not supported
pub const PSCI_NOT_SUPPORTED: i32 = -1;
/// Invalid parameters
pub const PSCI_INVALID_PARAMS: i32 = -2;
/// Denied
pub const PSCI_DENIED: i32 = -3;
/// Already on
pub const PSCI_ALREADY_ON: i32 = -4;
/// On pending
pub const PSCI_ON_PENDING: i32 = -5;
/// Internal failure
pub const PSCI_INTERNAL_FAILURE: i32 = -6;
/// Not present
pub const PSCI_NOT_PRESENT: i32 = -7;
/// Disabled
pub const PSCI_DISABLED: i32 = -8;
/// Invalid address
pub const PSCI_INVALID_ADDRESS: i32 = -9;

// =============================================================================
// AFFINITY INFO VALUES
// =============================================================================

/// CPU is on
pub const AFFINITY_ON: u32 = 0;
/// CPU is off
pub const AFFINITY_OFF: u32 = 1;
/// CPU is on pending
pub const AFFINITY_ON_PENDING: u32 = 2;

// =============================================================================
// CPU SUSPEND POWER STATE
// =============================================================================

/// Standby (retention)
pub const POWER_STATE_STANDBY: u32 = 0;
/// Powerdown
pub const POWER_STATE_POWERDOWN: u32 = 1;

/// State type: standby
pub const STATE_TYPE_STANDBY: u32 = 0 << 16;
/// State type: powerdown
pub const STATE_TYPE_POWERDOWN: u32 = 1 << 16;

/// Power level: core
pub const POWER_LEVEL_CORE: u32 = 0;
/// Power level: cluster
pub const POWER_LEVEL_CLUSTER: u32 = 1;
/// Power level: system
pub const POWER_LEVEL_SYSTEM: u32 = 2;

// =============================================================================
// PSCI CONDUIT
// =============================================================================

/// PSCI calling convention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PsciConduit {
    /// Not detected
    None,
    /// SMC (Secure Monitor Call)
    Smc,
    /// HVC (Hypervisor Call)
    Hvc,
}

/// Current PSCI conduit
static mut PSCI_CONDUIT: PsciConduit = PsciConduit::None;
/// PSCI version
static PSCI_VER: AtomicU32 = AtomicU32::new(0);
/// Number of online CPUs
static ONLINE_CPUS: AtomicU32 = AtomicU32::new(1);

// =============================================================================
// SMCCC CALLS
// =============================================================================

/// Make SMC call
#[inline]
unsafe fn smc_call(func: u32, arg0: u64, arg1: u64, arg2: u64) -> i64 {
    let result: i64;
    core::arch::asm!(
        "smc #0",
        inout("x0") func as u64 => result,
        inout("x1") arg0 => _,
        inout("x2") arg1 => _,
        inout("x3") arg2 => _,
        // Clobber all other argument registers
        out("x4") _,
        out("x5") _,
        out("x6") _,
        out("x7") _,
        options(nomem, nostack)
    );
    result
}

/// Make HVC call
#[inline]
unsafe fn hvc_call(func: u32, arg0: u64, arg1: u64, arg2: u64) -> i64 {
    let result: i64;
    core::arch::asm!(
        "hvc #0",
        inout("x0") func as u64 => result,
        inout("x1") arg0 => _,
        inout("x2") arg1 => _,
        inout("x3") arg2 => _,
        out("x4") _,
        out("x5") _,
        out("x6") _,
        out("x7") _,
        options(nomem, nostack)
    );
    result
}

/// Make PSCI call using detected conduit
#[inline]
pub unsafe fn psci_call(func: u32, arg0: u64, arg1: u64, arg2: u64) -> i64 {
    match PSCI_CONDUIT {
        PsciConduit::Smc => smc_call(func, arg0, arg1, arg2),
        PsciConduit::Hvc => hvc_call(func, arg0, arg1, arg2),
        PsciConduit::None => PSCI_NOT_SUPPORTED as i64,
    }
}

// =============================================================================
// PSCI DETECTION AND INITIALIZATION
// =============================================================================

/// Detect PSCI conduit
unsafe fn detect_conduit() -> PsciConduit {
    // Try SMC first
    let result = smc_call(PSCI_VERSION, 0, 0, 0);
    if result >= 0 {
        return PsciConduit::Smc;
    }

    // Try HVC
    let result = hvc_call(PSCI_VERSION, 0, 0, 0);
    if result >= 0 {
        return PsciConduit::Hvc;
    }

    PsciConduit::None
}

/// Get PSCI version
pub unsafe fn get_version() -> u32 {
    let cached = PSCI_VER.load(Ordering::SeqCst);
    if cached != 0 {
        return cached;
    }

    let version = psci_call(PSCI_VERSION, 0, 0, 0);
    if version >= 0 {
        PSCI_VER.store(version as u32, Ordering::SeqCst);
        version as u32
    } else {
        0
    }
}

/// Check if PSCI feature is supported
pub unsafe fn is_feature_supported(func: u32) -> bool {
    let result = psci_call(PSCI_FEATURES, func as u64, 0, 0);
    result >= 0
}

/// Initialize PSCI
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Detect conduit
    PSCI_CONDUIT = detect_conduit();

    if PSCI_CONDUIT == PsciConduit::None {
        // PSCI not available - might be bare metal or unsupported
        return Ok(());
    }

    // Get version
    let version = get_version();
    let major = (version >> 16) & 0xFFFF;
    let minor = version & 0xFFFF;

    // Store PSCI info in context
    ctx.arch_data.arm.psci_conduit = match PSCI_CONDUIT {
        PsciConduit::Smc => 1,
        PsciConduit::Hvc => 2,
        PsciConduit::None => 0,
    };
    ctx.arch_data.arm.psci_version = version;

    Ok(())
}

// =============================================================================
// CPU POWER MANAGEMENT
// =============================================================================

/// Turn on a CPU
///
/// # Arguments
/// * `target_cpu` - MPIDR of target CPU
/// * `entry_point` - Physical address of entry point
/// * `context_id` - Context ID passed to entry point in x0
pub unsafe fn cpu_on(target_cpu: u64, entry_point: u64, context_id: u64) -> i32 {
    let result = psci_call(CPU_ON_64, target_cpu, entry_point, context_id);
    result as i32
}

/// Turn off calling CPU (does not return on success)
pub unsafe fn cpu_off() -> i32 {
    let result = psci_call(CPU_OFF, 0, 0, 0);
    result as i32
}

/// Suspend calling CPU
///
/// # Arguments
/// * `power_state` - Power state to enter
/// * `entry_point` - Physical address of resume entry point
/// * `context_id` - Context ID passed on resume
pub unsafe fn cpu_suspend(power_state: u32, entry_point: u64, context_id: u64) -> i32 {
    let result = psci_call(CPU_SUSPEND_64, power_state as u64, entry_point, context_id);
    result as i32
}

/// Get affinity info for a CPU
pub unsafe fn affinity_info(target_affinity: u64, lowest_affinity_level: u32) -> i32 {
    let result = psci_call(
        AFFINITY_INFO_64,
        target_affinity,
        lowest_affinity_level as u64,
        0,
    );
    result as i32
}

/// Check if CPU is online
pub unsafe fn is_cpu_online(mpidr: u64) -> bool {
    let result = affinity_info(mpidr & 0xFF_FFFF_FFFF, 0);
    result == AFFINITY_ON as i32
}

// =============================================================================
// SYSTEM POWER MANAGEMENT
// =============================================================================

/// Power off the system (does not return)
pub unsafe fn system_off() -> ! {
    psci_call(SYSTEM_OFF, 0, 0, 0);

    // Should not reach here
    loop {
        wfi();
    }
}

/// Reset the system (does not return)
pub unsafe fn system_reset() -> ! {
    psci_call(SYSTEM_RESET, 0, 0, 0);

    // Should not reach here
    loop {
        wfi();
    }
}

/// Reset the system with reason (PSCI 1.1+)
pub unsafe fn system_reset2(reset_type: u32, cookie: u64) -> ! {
    psci_call(SYSTEM_RESET2_64, reset_type as u64, cookie, 0);

    // Fallback to regular reset
    system_reset()
}

/// Suspend the entire system
pub unsafe fn system_suspend(entry_point: u64, context_id: u64) -> i32 {
    let result = psci_call(SYSTEM_SUSPEND_64, entry_point, context_id, 0);
    result as i32
}

// =============================================================================
// SMP BOOT
// =============================================================================

/// AP (secondary CPU) entry trampoline
/// This is placed in low memory and jumped to by the firmware
#[repr(C)]
pub struct ApTrampoline {
    /// Entry point to jump to
    pub entry_point: u64,
    /// Stack pointer
    pub stack_pointer: u64,
    /// Page table pointer (TTBR0)
    pub page_table: u64,
    /// CPU ID
    pub cpu_id: u64,
    /// Ready flag (set by AP when ready)
    pub ready: AtomicU32,
}

/// Global trampoline data
static mut AP_TRAMPOLINES: [ApTrampoline; 256] =
    unsafe { core::mem::MaybeUninit::zeroed().assume_init() };

/// Start secondary CPU
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID (0-255)
/// * `mpidr` - MPIDR value for the CPU
/// * `entry_point` - Entry function pointer
/// * `stack_top` - Top of stack for this CPU
/// * `page_table` - Physical address of page table
pub unsafe fn start_cpu(
    cpu_id: u32,
    mpidr: u64,
    entry_point: extern "C" fn(u64),
    stack_top: u64,
    page_table: u64,
) -> BootResult<()> {
    if cpu_id as usize >= 256 {
        return Err(BootError::InvalidParameter("CPU ID out of range".into()));
    }

    // Setup trampoline data
    let trampoline = &mut AP_TRAMPOLINES[cpu_id as usize];
    trampoline.entry_point = entry_point as u64;
    trampoline.stack_pointer = stack_top;
    trampoline.page_table = page_table;
    trampoline.cpu_id = cpu_id as u64;
    trampoline.ready.store(0, Ordering::SeqCst);

    // Data barrier to ensure trampoline is visible
    dsb();

    // Get trampoline address
    let trampoline_addr = trampoline as *const _ as u64;

    // Call PSCI CPU_ON
    let result = cpu_on(
        mpidr & 0xFF_FFFF_FFFF, // Mask to valid MPIDR bits
        ap_trampoline_entry as u64,
        trampoline_addr,
    );

    if result != PSCI_SUCCESS {
        return Err(BootError::CpuStartFailed(cpu_id, result));
    }

    // Wait for CPU to come online (with timeout)
    let timeout = 1000000; // ~1 second
    for _ in 0..timeout {
        if trampoline.ready.load(Ordering::SeqCst) != 0 {
            ONLINE_CPUS.fetch_add(1, Ordering::SeqCst);
            return Ok(());
        }
        core::hint::spin_loop();
    }

    Err(BootError::CpuStartTimeout(cpu_id))
}

/// AP trampoline entry point
/// Called by firmware with x0 = trampoline address
#[naked]
unsafe extern "C" fn ap_trampoline_entry() {
    core::arch::asm!(
        // x0 contains trampoline address
        "mov x19, x0", // Save trampoline address
        // Load stack pointer
        "ldr x1, [x19, #8]", // stack_pointer
        "mov sp, x1",
        // Load page table
        "ldr x1, [x19, #16]", // page_table
        "msr TTBR0_EL1, x1",
        "isb",
        // Load CPU ID into x0 for entry function
        "ldr x0, [x19, #24]", // cpu_id
        // Signal ready
        "mov x1, #1",
        "str w1, [x19, #32]", // ready
        "dsb sy",
        "sev", // Wake up waiting CPUs
        // Load and jump to entry point
        "ldr x1, [x19, #0]", // entry_point
        "br x1",
        options(noreturn)
    );
}

/// Get number of online CPUs
pub fn get_online_cpu_count() -> u32 {
    ONLINE_CPUS.load(Ordering::SeqCst)
}

/// Park calling CPU (low-power wait)
pub fn park_cpu() {
    loop {
        wfe();
    }
}

// =============================================================================
// MIGRATION (for uniprocessor systems)
// =============================================================================

/// Get migration type
pub unsafe fn migrate_info_type() -> i32 {
    let result = psci_call(MIGRATE_INFO_TYPE, 0, 0, 0);
    result as i32
}

/// Migrate to specified CPU (if supported)
pub unsafe fn migrate(target_cpu: u64) -> i32 {
    let result = psci_call(MIGRATE_64, target_cpu, 0, 0);
    result as i32
}

/// Get migration target CPU
pub unsafe fn migrate_info_up_cpu() -> u64 {
    let result = psci_call(MIGRATE_INFO_UP_CPU_64, 0, 0, 0);
    result as u64
}

// =============================================================================
// MEMORY PROTECTION
// =============================================================================

/// Enable/disable memory protection
pub unsafe fn mem_protect(enable: bool) -> i32 {
    let result = psci_call(MEM_PROTECT, enable as u64, 0, 0);
    result as i32
}

/// Check if memory range is protected
pub unsafe fn mem_protect_check_range(base: u64, length: u64) -> i32 {
    let result = psci_call(MEM_PROTECT_CHECK_RANGE_64, base, length, 0);
    result as i32
}

// =============================================================================
// STATISTICS
// =============================================================================

/// Get power state residency statistics
pub unsafe fn stat_residency(target_cpu: u64, power_state: u32) -> u64 {
    let result = psci_call(PSCI_STAT_RESIDENCY_64, target_cpu, power_state as u64, 0);
    result as u64
}

/// Get power state entry count
pub unsafe fn stat_count(target_cpu: u64, power_state: u32) -> u64 {
    let result = psci_call(PSCI_STAT_COUNT_64, target_cpu, power_state as u64, 0);
    result as u64
}

// =============================================================================
// NODE HARDWARE STATE
// =============================================================================

/// Hardware state: off
pub const HW_STATE_OFF: u32 = 0;
/// Hardware state: standby
pub const HW_STATE_STANDBY: u32 = 1;
/// Hardware state: on
pub const HW_STATE_ON: u32 = 2;

/// Get node hardware state
pub unsafe fn node_hw_state(target_cpu: u64, power_level: u32) -> i32 {
    let result = psci_call(NODE_HW_STATE_64, target_cpu, power_level as u64, 0);
    result as i32
}

// =============================================================================
// SUSPEND MODE
// =============================================================================

/// Suspend mode: platform coordinated
pub const SUSPEND_MODE_PC: u32 = 0;
/// Suspend mode: OS initiated
pub const SUSPEND_MODE_OSI: u32 = 1;

/// Set suspend mode
pub unsafe fn set_suspend_mode(mode: u32) -> i32 {
    let result = psci_call(PSCI_SET_SUSPEND_MODE, mode as u64, 0, 0);
    result as i32
}

// =============================================================================
// PSCI VERSION HELPERS
// =============================================================================

/// PSCI version info
#[derive(Debug, Clone, Copy)]
pub struct PsciVersion {
    pub major: u16,
    pub minor: u16,
}

impl PsciVersion {
    /// Parse version from raw value
    pub fn from_raw(raw: u32) -> Self {
        Self {
            major: ((raw >> 16) & 0xFFFF) as u16,
            minor: (raw & 0xFFFF) as u16,
        }
    }

    /// Check if version is at least the specified version
    pub fn at_least(&self, major: u16, minor: u16) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }
}

/// Get parsed PSCI version
pub fn get_psci_version() -> PsciVersion {
    PsciVersion::from_raw(PSCI_VER.load(Ordering::SeqCst))
}

/// Get current conduit type
pub fn get_conduit() -> PsciConduit {
    unsafe { PSCI_CONDUIT }
}
