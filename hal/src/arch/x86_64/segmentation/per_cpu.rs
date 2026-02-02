//! # Per-CPU Segmentation
//!
//! Per-CPU GDT and TSS management for SMP systems.
//!
//! ## Overview
//!
//! Each CPU core needs its own GDT and TSS because:
//!
//! 1. **RSP0**: Each CPU needs a different kernel stack
//! 2. **IST stacks**: Each CPU needs independent interrupt stacks
//! 3. **TSS busy bit**: Only one CPU can use a TSS at a time
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Per-CPU Segmentation                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │   CPU 0                          CPU 1                          │
//! │   ┌──────────┐                   ┌──────────┐                   │
//! │   │   GDT₀   │                   │   GDT₁   │                   │
//! │   ├──────────┤                   ├──────────┤                   │
//! │   │   TSS₀   │                   │   TSS₁   │                   │
//! │   │ ┌──────┐ │                   │ ┌──────┐ │                   │
//! │   │ │ RSP0 │ │                   │ │ RSP0 │ │                   │
//! │   │ ├──────┤ │                   │ ├──────┤ │                   │
//! │   │ │ IST1 │ │                   │ │ IST1 │ │                   │
//! │   │ │  ... │ │                   │ │  ... │ │                   │
//! │   │ │ IST7 │ │                   │ │ IST7 │ │                   │
//! │   │ └──────┘ │                   │ └──────┘ │                   │
//! │   └──────────┘                   └──────────┘                   │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use core::sync::atomic::{AtomicUsize, Ordering};

use super::gdt::Gdt;
use super::selectors::TSS_SELECTOR;
use super::tss::{IstIndex, Tss, IST_DOUBLE_FAULT_SIZE, IST_STACK_SIZE, KERNEL_STACK_SIZE};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of CPUs supported
pub const MAX_CPUS: usize = 256;

/// Size of per-CPU stack area (kernel stack + IST stacks)
const PER_CPU_STACK_SIZE: usize = KERNEL_STACK_SIZE
    + IST_DOUBLE_FAULT_SIZE  // IST1: Double Fault (larger)
    + IST_STACK_SIZE * 6; // IST2-IST7

// =============================================================================
// STATIC DATA
// =============================================================================

/// Number of initialized CPUs
static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Per-CPU GDT storage
static mut GDTS: [Gdt; MAX_CPUS] = {
    const EMPTY_GDT: Gdt = Gdt::new();
    [EMPTY_GDT; MAX_CPUS]
};

/// Per-CPU TSS storage
static mut TSSS: [Tss; MAX_CPUS] = {
    const EMPTY_TSS: Tss = Tss::new();
    [EMPTY_TSS; MAX_CPUS]
};

/// Per-CPU stack storage
///
/// Layout per CPU:
/// - [0..KERNEL_STACK_SIZE): Kernel stack
/// - [KERNEL_STACK_SIZE..]: IST stacks
#[repr(C, align(4096))]
struct PerCpuStacks {
    stacks: [[u8; PER_CPU_STACK_SIZE]; MAX_CPUS],
}

static mut STACKS: PerCpuStacks = PerCpuStacks {
    stacks: [[0; PER_CPU_STACK_SIZE]; MAX_CPUS],
};

// =============================================================================
// PER-CPU SEGMENTATION
// =============================================================================

/// Per-CPU segmentation data
pub struct PerCpuSegmentation {
    /// CPU ID
    cpu_id: usize,
}

impl PerCpuSegmentation {
    /// Get the GDT for this CPU
    pub fn gdt(&self) -> &Gdt {
        unsafe { &GDTS[self.cpu_id] }
    }

    /// Get the GDT for this CPU (mutable)
    pub fn gdt_mut(&mut self) -> &mut Gdt {
        unsafe { &mut GDTS[self.cpu_id] }
    }

    /// Get the TSS for this CPU
    pub fn tss(&self) -> &Tss {
        unsafe { &TSSS[self.cpu_id] }
    }

    /// Get the TSS for this CPU (mutable)
    pub fn tss_mut(&mut self) -> &mut Tss {
        unsafe { &mut TSSS[self.cpu_id] }
    }

    /// Get kernel stack top for this CPU
    pub fn kernel_stack_top(&self) -> u64 {
        unsafe {
            let base = STACKS.stacks[self.cpu_id].as_ptr() as u64;
            // Stack top is at end of kernel stack region, 16-byte aligned
            (base + KERNEL_STACK_SIZE as u64) & !0xF
        }
    }

    /// Get IST stack top for this CPU
    pub fn ist_stack_top(&self, ist: IstIndex) -> u64 {
        unsafe {
            let base = STACKS.stacks[self.cpu_id].as_ptr() as u64;
            let mut offset = KERNEL_STACK_SIZE;

            // Add offsets for previous IST stacks
            for i in 1..=(ist as usize) {
                offset += if i == 1 {
                    IST_DOUBLE_FAULT_SIZE
                } else {
                    IST_STACK_SIZE
                };
            }

            // Stack top at end of this IST region, 16-byte aligned
            (base + offset as u64) & !0xF
        }
    }

    /// CPU ID
    pub fn cpu_id(&self) -> usize {
        self.cpu_id
    }
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize segmentation for the Bootstrap Processor (CPU 0)
///
/// # Safety
/// Must be called exactly once during early boot on BSP.
pub unsafe fn init_bsp() {
    unsafe { init_cpu(0) };
    log::info!("Segmentation: BSP initialized");
}

/// Initialize segmentation for an Application Processor
///
/// # Safety
/// Must be called exactly once per AP during SMP initialization.
pub unsafe fn init_ap(cpu_id: usize) {
    assert!(cpu_id > 0, "CPU 0 should use init_bsp");
    assert!(cpu_id < MAX_CPUS, "CPU ID exceeds MAX_CPUS");

    unsafe { init_cpu(cpu_id) };
    log::debug!("Segmentation: AP {} initialized", cpu_id);
}

/// Internal CPU initialization
unsafe fn init_cpu(cpu_id: usize) {
    // Get per-CPU references
    let gdt = unsafe { &mut GDTS[cpu_id] };
    let tss = unsafe { &mut TSSS[cpu_id] };

    // Initialize GDT
    *gdt = Gdt::new();

    // Initialize TSS
    *tss = Tss::new();

    // Calculate stack addresses
    let base = unsafe { STACKS.stacks[cpu_id].as_ptr() as u64 };

    // Set kernel stack (RSP0)
    let kernel_stack_top = (base + KERNEL_STACK_SIZE as u64) & !0xF;
    tss.set_kernel_stack(kernel_stack_top);

    // Set IST stacks
    let mut offset = KERNEL_STACK_SIZE;

    // IST1: Double Fault (larger stack)
    offset += IST_DOUBLE_FAULT_SIZE;
    tss.set_ist(IstIndex::DoubleFault, (base + offset as u64) & !0xF);

    // IST2: NMI
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::Nmi, (base + offset as u64) & !0xF);

    // IST3: Machine Check
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::MachineCheck, (base + offset as u64) & !0xF);

    // IST4: Debug
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::Debug, (base + offset as u64) & !0xF);

    // IST5-7: Reserved
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::Reserved5, (base + offset as u64) & !0xF);
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::Reserved6, (base + offset as u64) & !0xF);
    offset += IST_STACK_SIZE;
    tss.set_ist(IstIndex::Reserved7, (base + offset as u64) & !0xF);

    // Link TSS to GDT
    gdt.set_tss(tss as *const Tss);

    // Load GDT
    unsafe { gdt.load_and_reload_segments() };

    // Clear TSS busy bit and load TSS
    gdt.clear_tss_busy();
    unsafe { super::tss::load_tss(TSS_SELECTOR) };

    // Increment CPU count
    CPU_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Get the number of initialized CPUs
pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Acquire)
}

/// Get per-CPU segmentation for a specific CPU
///
/// # Safety
/// The CPU must have been initialized.
pub unsafe fn get_cpu(cpu_id: usize) -> PerCpuSegmentation {
    assert!(
        cpu_id < CPU_COUNT.load(Ordering::Acquire),
        "CPU not initialized"
    );
    PerCpuSegmentation { cpu_id }
}

/// Get per-CPU segmentation for the current CPU
///
/// This requires per-CPU data to be set up (GS base pointing to per-CPU area).
/// For BSP before per-CPU data is ready, use `get_cpu(0)`.
#[cfg(feature = "percpu")]
pub fn current() -> PerCpuSegmentation {
    // TODO: Read CPU ID from per-CPU data
    unsafe { get_cpu(0) }
}

// =============================================================================
// DYNAMIC TSS UPDATES
// =============================================================================

/// Update kernel stack for a CPU
///
/// Called during context switch to set the new kernel stack for the process.
///
/// # Safety
/// Must be called with a valid stack pointer.
pub unsafe fn set_kernel_stack(cpu_id: usize, stack_top: u64) {
    assert!(cpu_id < MAX_CPUS);
    unsafe { TSSS[cpu_id].set_kernel_stack(stack_top) };
}

/// Update kernel stack for current CPU
///
/// # Safety
/// Must be called with a valid stack pointer.
#[cfg(feature = "percpu")]
pub unsafe fn set_current_kernel_stack(stack_top: u64) {
    // TODO: Get current CPU ID from per-CPU data
    unsafe { set_kernel_stack(0, stack_top) };
}

// =============================================================================
// DEBUG
// =============================================================================

/// Dump segmentation info for debugging
pub fn dump_info(cpu_id: usize) {
    if cpu_id >= CPU_COUNT.load(Ordering::Acquire) {
        log::warn!("CPU {} not initialized", cpu_id);
        return;
    }

    unsafe {
        let gdt = &GDTS[cpu_id];
        let tss = &TSSS[cpu_id];

        // Copy fields from packed struct to avoid unaligned references
        let rsp0 = { tss.rsp0 };
        let ist0 = { tss.ist[0] };
        let ist1 = { tss.ist[1] };
        let ist2 = { tss.ist[2] };
        let ist3 = { tss.ist[3] };

        log::debug!("=== CPU {} Segmentation ===", cpu_id);
        log::debug!("GDT: {:?}", gdt.descriptor());
        log::debug!("TSS RSP0: {:#018x}", rsp0);
        log::debug!("IST1 (DF): {:#018x}", ist0);
        log::debug!("IST2 (NMI): {:#018x}", ist1);
        log::debug!("IST3 (MC): {:#018x}", ist2);
        log::debug!("IST4 (DB): {:#018x}", ist3);
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_cpu_stack_size() {
        // Ensure we have enough stack space
        assert!(
            PER_CPU_STACK_SIZE >= KERNEL_STACK_SIZE + IST_DOUBLE_FAULT_SIZE + 6 * IST_STACK_SIZE
        );
    }

    #[test]
    fn test_max_cpus() {
        assert!(MAX_CPUS >= 1);
        assert!(MAX_CPUS <= 4096); // Reasonable upper limit
    }
}
