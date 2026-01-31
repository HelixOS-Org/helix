//! # AArch64 GIC (Generic Interrupt Controller) Driver
//!
//! Supports GICv2 and GICv3 interrupt controllers.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::{BootContext, InterruptControllerType};
use crate::error::{BootError, BootResult};

// =============================================================================
// GIC BASE ADDRESSES (common defaults)
// =============================================================================

/// Default GICD base (GICv2)
pub const GICD_BASE_DEFAULT: u64 = 0x08000000;
/// Default GICC base (GICv2)
pub const GICC_BASE_DEFAULT: u64 = 0x08010000;
/// Default GICR base (GICv3)
pub const GICR_BASE_DEFAULT: u64 = 0x080A0000;

// =============================================================================
// GICD REGISTERS (Distributor)
// =============================================================================

/// Distributor Control Register
pub const GICD_CTLR: u64 = 0x000;
/// Interrupt Controller Type Register
pub const GICD_TYPER: u64 = 0x004;
/// Distributor Implementer Identification Register
pub const GICD_IIDR: u64 = 0x008;
/// Interrupt Group Registers (banked)
pub const GICD_IGROUPR: u64 = 0x080;
/// Interrupt Set-Enable Registers
pub const GICD_ISENABLER: u64 = 0x100;
/// Interrupt Clear-Enable Registers
pub const GICD_ICENABLER: u64 = 0x180;
/// Interrupt Set-Pending Registers
pub const GICD_ISPENDR: u64 = 0x200;
/// Interrupt Clear-Pending Registers
pub const GICD_ICPENDR: u64 = 0x280;
/// Interrupt Set-Active Registers
pub const GICD_ISACTIVER: u64 = 0x300;
/// Interrupt Clear-Active Registers
pub const GICD_ICACTIVER: u64 = 0x380;
/// Interrupt Priority Registers
pub const GICD_IPRIORITYR: u64 = 0x400;
/// Interrupt Processor Targets Registers
pub const GICD_ITARGETSR: u64 = 0x800;
/// Interrupt Configuration Registers
pub const GICD_ICFGR: u64 = 0xC00;
/// Software Generated Interrupt Register
pub const GICD_SGIR: u64 = 0xF00;
/// SGI Clear-Pending Registers
pub const GICD_CPENDSGIR: u64 = 0xF10;
/// SGI Set-Pending Registers
pub const GICD_SPENDSGIR: u64 = 0xF20;

// =============================================================================
// GICC REGISTERS (CPU Interface - GICv2)
// =============================================================================

/// CPU Interface Control Register
pub const GICC_CTLR: u64 = 0x000;
/// Priority Mask Register
pub const GICC_PMR: u64 = 0x004;
/// Binary Point Register
pub const GICC_BPR: u64 = 0x008;
/// Interrupt Acknowledge Register
pub const GICC_IAR: u64 = 0x00C;
/// End of Interrupt Register
pub const GICC_EOIR: u64 = 0x010;
/// Running Priority Register
pub const GICC_RPR: u64 = 0x014;
/// Highest Priority Pending Interrupt Register
pub const GICC_HPPIR: u64 = 0x018;
/// Aliased Binary Point Register
pub const GICC_ABPR: u64 = 0x01C;
/// Aliased Interrupt Acknowledge Register
pub const GICC_AIAR: u64 = 0x020;
/// Aliased End of Interrupt Register
pub const GICC_AEOIR: u64 = 0x024;
/// Aliased Highest Priority Pending Interrupt Register
pub const GICC_AHPPIR: u64 = 0x028;
/// CPU Interface Identification Register
pub const GICC_IIDR: u64 = 0x00FC;
/// Deactivate Interrupt Register
pub const GICC_DIR: u64 = 0x1000;

// =============================================================================
// GICR REGISTERS (Redistributor - GICv3)
// =============================================================================

/// Redistributor Control Register
pub const GICR_CTLR: u64 = 0x000;
/// Implementer Identification Register
pub const GICR_IIDR: u64 = 0x004;
/// Redistributor Type Register
pub const GICR_TYPER: u64 = 0x008;
/// Redistributor Wake Register
pub const GICR_WAKER: u64 = 0x014;
/// Redistributor frame size
pub const GICR_FRAME_SIZE: u64 = 0x20000;
/// SGI base offset
pub const GICR_SGI_BASE: u64 = 0x10000;

// =============================================================================
// INTERRUPT TYPES
// =============================================================================

/// Software Generated Interrupts (0-15)
pub const SGI_START: u32 = 0;
pub const SGI_END: u32 = 15;
/// Private Peripheral Interrupts (16-31)
pub const PPI_START: u32 = 16;
pub const PPI_END: u32 = 31;
/// Shared Peripheral Interrupts (32-1019)
pub const SPI_START: u32 = 32;
/// Maximum number of interrupts
pub const MAX_IRQS: u32 = 1020;

// =============================================================================
// GIC STATE
// =============================================================================

/// GICD base address
static GICD_BASE: AtomicU64 = AtomicU64::new(GICD_BASE_DEFAULT);
/// GICC base address (GICv2)
static GICC_BASE: AtomicU64 = AtomicU64::new(GICC_BASE_DEFAULT);
/// GICR base address (GICv3)
static GICR_BASE: AtomicU64 = AtomicU64::new(GICR_BASE_DEFAULT);
/// GIC version (2 or 3)
static mut GIC_VERSION: u8 = 2;
/// Number of interrupt lines
static mut NUM_IRQS: u32 = 0;

// =============================================================================
// GIC ACCESS
// =============================================================================

/// Read GICD register
unsafe fn gicd_read(offset: u64) -> u32 {
    let addr = GICD_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::read_volatile(addr as *const u32)
}

/// Write GICD register
unsafe fn gicd_write(offset: u64, value: u32) {
    let addr = GICD_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::write_volatile(addr as *mut u32, value);
}

/// Read GICC register (GICv2)
unsafe fn gicc_read(offset: u64) -> u32 {
    let addr = GICC_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::read_volatile(addr as *const u32)
}

/// Write GICC register (GICv2)
unsafe fn gicc_write(offset: u64, value: u32) {
    let addr = GICC_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::write_volatile(addr as *mut u32, value);
}

/// Get GICR base for current CPU (GICv3)
unsafe fn get_gicr_base() -> u64 {
    // Find redistributor for current CPU
    let affinity = get_cpu_affinity();
    let base = GICR_BASE.load(Ordering::SeqCst);

    let mut offset: u64 = 0;
    loop {
        let typer = core::ptr::read_volatile((base + offset + GICR_TYPER) as *const u64);
        let aff = (typer >> 32) as u32;

        if aff == affinity as u32 {
            return base + offset;
        }

        // Check if this is the last redistributor
        if typer & (1 << 4) != 0 {
            break;
        }

        offset += GICR_FRAME_SIZE;
    }

    // Fallback to first redistributor
    base
}

/// Read GICR register (GICv3)
unsafe fn gicr_read(offset: u64) -> u32 {
    let base = get_gicr_base();
    core::ptr::read_volatile((base + offset) as *const u32)
}

/// Write GICR register (GICv3)
unsafe fn gicr_write(offset: u64, value: u32) {
    let base = get_gicr_base();
    core::ptr::write_volatile((base + offset) as *mut u32, value);
}

// =============================================================================
// GICv3 SYSTEM REGISTERS
// =============================================================================

/// Read ICC_SRE_EL1
fn read_icc_sre_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, S3_0_C12_C12_5", // ICC_SRE_EL1
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write ICC_SRE_EL1
fn write_icc_sre_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr S3_0_C12_C12_5, {}", // ICC_SRE_EL1
            in(reg) value,
            options(nomem, nostack)
        );
    }
    isb();
}

/// Read ICC_PMR_EL1
fn read_icc_pmr_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, S3_0_C4_C6_0", // ICC_PMR_EL1
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write ICC_PMR_EL1
fn write_icc_pmr_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr S3_0_C4_C6_0, {}", // ICC_PMR_EL1
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read ICC_IAR1_EL1
fn read_icc_iar1_el1() -> u32 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, S3_0_C12_C12_0", // ICC_IAR1_EL1
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value as u32
}

/// Write ICC_EOIR1_EL1
fn write_icc_eoir1_el1(value: u32) {
    unsafe {
        core::arch::asm!(
            "msr S3_0_C12_C12_1, {}", // ICC_EOIR1_EL1
            in(reg) value as u64,
            options(nomem, nostack)
        );
    }
}

/// Read ICC_IGRPEN1_EL1
fn read_icc_igrpen1_el1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, S3_0_C12_C12_7", // ICC_IGRPEN1_EL1
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write ICC_IGRPEN1_EL1
fn write_icc_igrpen1_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr S3_0_C12_C12_7, {}", // ICC_IGRPEN1_EL1
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Write ICC_SGI1R_EL1 (send SGI)
fn write_icc_sgi1r_el1(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr S3_0_C12_C11_5, {}", // ICC_SGI1R_EL1
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

// =============================================================================
// GIC DETECTION
// =============================================================================

/// Detect GIC version
pub unsafe fn detect_gic_version() -> u8 {
    // Try GICv3 first by reading ICC_SRE_EL1
    let sre = read_icc_sre_el1();

    // If SRE bit can be set, it's GICv3
    write_icc_sre_el1(sre | 1);
    let new_sre = read_icc_sre_el1();

    if new_sre & 1 != 0 {
        3
    } else {
        2
    }
}

// =============================================================================
// GIC INITIALIZATION
// =============================================================================

/// Initialize GIC
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Detect GIC version
    GIC_VERSION = detect_gic_version();

    // Get base addresses from device tree or use defaults
    if let Some(ref dt_info) = ctx.boot_info.device_tree {
        // TODO: Parse device tree for GIC addresses
    }

    // Read number of interrupt lines
    let typer = gicd_read(GICD_TYPER);
    NUM_IRQS = ((typer & 0x1F) + 1) * 32;
    if NUM_IRQS > MAX_IRQS {
        NUM_IRQS = MAX_IRQS;
    }

    // Initialize based on version
    if GIC_VERSION >= 3 {
        init_gicv3(ctx)?;
    } else {
        init_gicv2(ctx)?;
    }

    Ok(())
}

/// Initialize GICv2
unsafe fn init_gicv2(ctx: &mut BootContext) -> BootResult<()> {
    // Disable distributor
    gicd_write(GICD_CTLR, 0);

    // Configure all SPIs
    for i in (SPI_START..NUM_IRQS).step_by(32) {
        let idx = i / 32;
        // Disable
        gicd_write(GICD_ICENABLER + (idx as u64 * 4), 0xFFFFFFFF);
        // Clear pending
        gicd_write(GICD_ICPENDR + (idx as u64 * 4), 0xFFFFFFFF);
        // Set group 0
        gicd_write(GICD_IGROUPR + (idx as u64 * 4), 0);
    }

    // Set priority for all interrupts
    for i in (0..NUM_IRQS).step_by(4) {
        gicd_write(GICD_IPRIORITYR + (i as u64), 0xA0A0A0A0);
    }

    // Route all SPIs to CPU 0
    for i in (SPI_START..NUM_IRQS).step_by(4) {
        gicd_write(GICD_ITARGETSR + (i as u64), 0x01010101);
    }

    // Enable distributor
    gicd_write(GICD_CTLR, 1);

    // Initialize CPU interface
    init_gicv2_cpu_interface();

    ctx.interrupt_state.controller_type = InterruptControllerType::GicV2;
    ctx.arch_data.arm.gic_version = 2;
    ctx.arch_data.arm.gicd_base = GICD_BASE.load(Ordering::SeqCst);
    ctx.arch_data.arm.gicc_base = GICC_BASE.load(Ordering::SeqCst);

    Ok(())
}

/// Initialize GICv2 CPU interface
unsafe fn init_gicv2_cpu_interface() {
    // Disable CPU interface
    gicc_write(GICC_CTLR, 0);

    // Set priority mask (allow all)
    gicc_write(GICC_PMR, 0xFF);

    // Set binary point
    gicc_write(GICC_BPR, 0);

    // Enable CPU interface
    gicc_write(GICC_CTLR, 1);
}

/// Initialize GICv3
unsafe fn init_gicv3(ctx: &mut BootContext) -> BootResult<()> {
    // Enable system register access
    let sre = read_icc_sre_el1();
    write_icc_sre_el1(sre | 1);

    // Disable distributor
    gicd_write(GICD_CTLR, 0);
    dsb();

    // Wait for RWP
    while gicd_read(GICD_CTLR) & (1 << 31) != 0 {
        core::hint::spin_loop();
    }

    // Configure SPIs (same as GICv2)
    for i in (SPI_START..NUM_IRQS).step_by(32) {
        let idx = i / 32;
        gicd_write(GICD_ICENABLER + (idx as u64 * 4), 0xFFFFFFFF);
        gicd_write(GICD_ICPENDR + (idx as u64 * 4), 0xFFFFFFFF);
        gicd_write(GICD_IGROUPR + (idx as u64 * 4), 0xFFFFFFFF); // Group 1 for GICv3
    }

    // Set priority for SPIs
    for i in (SPI_START..NUM_IRQS).step_by(4) {
        gicd_write(GICD_IPRIORITYR + (i as u64), 0xA0A0A0A0);
    }

    // Enable distributor (Group 0 and Group 1)
    gicd_write(GICD_CTLR, (1 << 0) | (1 << 1) | (1 << 4)); // EnableGrp0, EnableGrp1NS, ARE_NS
    dsb();

    // Initialize redistributor for this CPU
    init_gicv3_redistributor();

    // Initialize CPU interface
    init_gicv3_cpu_interface();

    ctx.interrupt_state.controller_type = InterruptControllerType::GicV3;
    ctx.arch_data.arm.gic_version = 3;
    ctx.arch_data.arm.gicd_base = GICD_BASE.load(Ordering::SeqCst);
    ctx.arch_data.arm.gicr_base = GICR_BASE.load(Ordering::SeqCst);

    Ok(())
}

/// Initialize GICv3 redistributor
unsafe fn init_gicv3_redistributor() {
    let base = get_gicr_base();
    let sgi_base = base + GICR_SGI_BASE;

    // Wake up redistributor
    let waker = core::ptr::read_volatile((base + GICR_WAKER) as *const u32);
    core::ptr::write_volatile((base + GICR_WAKER) as *mut u32, waker & !(1 << 1));

    // Wait for ChildrenAsleep to clear
    while core::ptr::read_volatile((base + GICR_WAKER) as *const u32) & (1 << 2) != 0 {
        core::hint::spin_loop();
    }

    // Configure SGIs and PPIs
    // Disable all
    core::ptr::write_volatile((sgi_base + 0x180) as *mut u32, 0xFFFFFFFF);
    // Clear pending
    core::ptr::write_volatile((sgi_base + 0x280) as *mut u32, 0xFFFFFFFF);
    // Set group 1
    core::ptr::write_volatile((sgi_base + 0x080) as *mut u32, 0xFFFFFFFF);

    // Set priority for SGIs and PPIs
    for i in (0..32).step_by(4) {
        core::ptr::write_volatile((sgi_base + 0x400 + i) as *mut u32, 0xA0A0A0A0);
    }

    dsb();
}

/// Initialize GICv3 CPU interface
unsafe fn init_gicv3_cpu_interface() {
    // Set priority mask (allow all)
    write_icc_pmr_el1(0xFF);

    // Enable Group 1 interrupts
    write_icc_igrpen1_el1(1);

    isb();
}

// =============================================================================
// INTERRUPT MANAGEMENT
// =============================================================================

/// Enable an interrupt
pub unsafe fn enable_irq(irq: u32) {
    if irq >= NUM_IRQS {
        return;
    }

    let reg = irq / 32;
    let bit = irq % 32;

    if irq < SPI_START && GIC_VERSION >= 3 {
        // SGI/PPI - use redistributor
        let base = get_gicr_base() + GICR_SGI_BASE;
        let addr = base + 0x100 + (reg as u64 * 4);
        core::ptr::write_volatile(addr as *mut u32, 1 << bit);
    } else {
        gicd_write(GICD_ISENABLER + (reg as u64 * 4), 1 << bit);
    }
}

/// Disable an interrupt
pub unsafe fn disable_irq(irq: u32) {
    if irq >= NUM_IRQS {
        return;
    }

    let reg = irq / 32;
    let bit = irq % 32;

    if irq < SPI_START && GIC_VERSION >= 3 {
        let base = get_gicr_base() + GICR_SGI_BASE;
        let addr = base + 0x180 + (reg as u64 * 4);
        core::ptr::write_volatile(addr as *mut u32, 1 << bit);
    } else {
        gicd_write(GICD_ICENABLER + (reg as u64 * 4), 1 << bit);
    }
}

/// Set interrupt priority
pub unsafe fn set_priority(irq: u32, priority: u8) {
    if irq >= NUM_IRQS {
        return;
    }

    let reg = irq / 4;
    let shift = (irq % 4) * 8;

    if irq < SPI_START && GIC_VERSION >= 3 {
        let base = get_gicr_base() + GICR_SGI_BASE;
        let addr = base + 0x400 + (reg as u64 * 4);
        let mut val = core::ptr::read_volatile(addr as *const u32);
        val &= !(0xFF << shift);
        val |= (priority as u32) << shift;
        core::ptr::write_volatile(addr as *mut u32, val);
    } else {
        let val = gicd_read(GICD_IPRIORITYR + (reg as u64 * 4));
        let new_val = (val & !(0xFF << shift)) | ((priority as u32) << shift);
        gicd_write(GICD_IPRIORITYR + (reg as u64 * 4), new_val);
    }
}

/// Acknowledge interrupt
pub unsafe fn ack_irq() -> u32 {
    if GIC_VERSION >= 3 {
        read_icc_iar1_el1()
    } else {
        gicc_read(GICC_IAR)
    }
}

/// End of interrupt
pub unsafe fn eoi(irq: u32) {
    if GIC_VERSION >= 3 {
        write_icc_eoir1_el1(irq);
    } else {
        gicc_write(GICC_EOIR, irq);
    }
}

/// Send SGI (Software Generated Interrupt)
pub unsafe fn send_sgi(sgi: u8, target_list: u16) {
    if GIC_VERSION >= 3 {
        // GICv3 uses ICC_SGI1R_EL1
        let value = ((sgi as u64) << 24) | (target_list as u64);
        write_icc_sgi1r_el1(value);
    } else {
        // GICv2 uses GICD_SGIR
        let value = ((target_list as u32) << 16) | (sgi as u32);
        gicd_write(GICD_SGIR, value);
    }
}

/// Send SGI to all other CPUs
pub unsafe fn send_sgi_others(sgi: u8) {
    if GIC_VERSION >= 3 {
        // IRM=1 for all other PEs
        let value = ((sgi as u64) << 24) | (1 << 40);
        write_icc_sgi1r_el1(value);
    } else {
        let value = (1 << 24) | (sgi as u32); // Target filter = 01 (all except self)
        gicd_write(GICD_SGIR, value);
    }
}

/// Get GIC version
pub fn get_gic_version() -> u8 {
    unsafe { GIC_VERSION }
}

/// Get number of supported IRQs
pub fn get_num_irqs() -> u32 {
    unsafe { NUM_IRQS }
}
