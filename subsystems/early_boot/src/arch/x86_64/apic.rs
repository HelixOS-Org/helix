//! # x86_64 APIC and I/O APIC
//!
//! Local APIC, x2APIC, and I/O APIC initialization for interrupt handling.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::{BootContext, InterruptControllerType};
use crate::error::{BootError, BootResult};

// =============================================================================
// APIC REGISTER OFFSETS
// =============================================================================

/// Local APIC ID register
pub const LAPIC_ID: u32 = 0x020;
/// Local APIC version register
pub const LAPIC_VERSION: u32 = 0x030;
/// Task Priority Register
pub const LAPIC_TPR: u32 = 0x080;
/// Arbitration Priority Register
pub const LAPIC_APR: u32 = 0x090;
/// Processor Priority Register
pub const LAPIC_PPR: u32 = 0x0A0;
/// End of Interrupt register
pub const LAPIC_EOI: u32 = 0x0B0;
/// Remote Read Register
pub const LAPIC_RRD: u32 = 0x0C0;
/// Logical Destination Register
pub const LAPIC_LDR: u32 = 0x0D0;
/// Destination Format Register
pub const LAPIC_DFR: u32 = 0x0E0;
/// Spurious Interrupt Vector Register
pub const LAPIC_SVR: u32 = 0x0F0;
/// In-Service Register (ISR) base
pub const LAPIC_ISR: u32 = 0x100;
/// Trigger Mode Register (TMR) base
pub const LAPIC_TMR: u32 = 0x180;
/// Interrupt Request Register (IRR) base
pub const LAPIC_IRR: u32 = 0x200;
/// Error Status Register
pub const LAPIC_ESR: u32 = 0x280;
/// LVT Corrected Machine Check Interrupt
pub const LAPIC_LVT_CMCI: u32 = 0x2F0;
/// Interrupt Command Register (low)
pub const LAPIC_ICR_LOW: u32 = 0x300;
/// Interrupt Command Register (high)
pub const LAPIC_ICR_HIGH: u32 = 0x310;
/// LVT Timer
pub const LAPIC_LVT_TIMER: u32 = 0x320;
/// LVT Thermal Sensor
pub const LAPIC_LVT_THERMAL: u32 = 0x330;
/// LVT Performance Monitoring
pub const LAPIC_LVT_PERF: u32 = 0x340;
/// LVT LINT0
pub const LAPIC_LVT_LINT0: u32 = 0x350;
/// LVT LINT1
pub const LAPIC_LVT_LINT1: u32 = 0x360;
/// LVT Error
pub const LAPIC_LVT_ERROR: u32 = 0x370;
/// Initial Count Register (for timer)
pub const LAPIC_TIMER_ICR: u32 = 0x380;
/// Current Count Register (for timer)
pub const LAPIC_TIMER_CCR: u32 = 0x390;
/// Divide Configuration Register (for timer)
pub const LAPIC_TIMER_DCR: u32 = 0x3E0;

// =============================================================================
// APIC MSR ADDRESSES (x2APIC)
// =============================================================================

/// x2APIC base MSR
pub const X2APIC_MSR_BASE: u32 = 0x800;

/// Convert LAPIC register offset to x2APIC MSR
pub const fn lapic_to_x2apic_msr(reg: u32) -> u32 {
    X2APIC_MSR_BASE + (reg >> 4)
}

// =============================================================================
// LVT FLAGS
// =============================================================================

/// LVT masked (interrupt disabled)
pub const LVT_MASKED: u32 = 1 << 16;
/// LVT timer mode: one-shot
pub const LVT_TIMER_ONESHOT: u32 = 0 << 17;
/// LVT timer mode: periodic
pub const LVT_TIMER_PERIODIC: u32 = 1 << 17;
/// LVT timer mode: TSC-deadline
pub const LVT_TIMER_TSC_DEADLINE: u32 = 2 << 17;
/// LVT delivery status pending
pub const LVT_DELIVERY_PENDING: u32 = 1 << 12;
/// LVT trigger mode: edge
pub const LVT_EDGE: u32 = 0 << 15;
/// LVT trigger mode: level
pub const LVT_LEVEL: u32 = 1 << 15;

// =============================================================================
// ICR FLAGS
// =============================================================================

/// ICR delivery mode: fixed
pub const ICR_FIXED: u32 = 0 << 8;
/// ICR delivery mode: lowest priority
pub const ICR_LOWEST: u32 = 1 << 8;
/// ICR delivery mode: SMI
pub const ICR_SMI: u32 = 2 << 8;
/// ICR delivery mode: NMI
pub const ICR_NMI: u32 = 4 << 8;
/// ICR delivery mode: INIT
pub const ICR_INIT: u32 = 5 << 8;
/// ICR delivery mode: SIPI
pub const ICR_SIPI: u32 = 6 << 8;
/// ICR level: deassert
pub const ICR_DEASSERT: u32 = 0 << 14;
/// ICR level: assert
pub const ICR_ASSERT: u32 = 1 << 14;
/// ICR trigger mode: edge
pub const ICR_TRIGGER_EDGE: u32 = 0 << 15;
/// ICR trigger mode: level
pub const ICR_TRIGGER_LEVEL: u32 = 1 << 15;
/// ICR shorthand: none
pub const ICR_NO_SHORTHAND: u32 = 0 << 18;
/// ICR shorthand: self
pub const ICR_SELF: u32 = 1 << 18;
/// ICR shorthand: all including self
pub const ICR_ALL_INCLUDING_SELF: u32 = 2 << 18;
/// ICR shorthand: all excluding self
pub const ICR_ALL_EXCLUDING_SELF: u32 = 3 << 18;

// =============================================================================
// TIMER DIVIDER VALUES
// =============================================================================

/// Divide by 1
pub const TIMER_DIV_1: u32 = 0b1011;
/// Divide by 2
pub const TIMER_DIV_2: u32 = 0b0000;
/// Divide by 4
pub const TIMER_DIV_4: u32 = 0b0001;
/// Divide by 8
pub const TIMER_DIV_8: u32 = 0b0010;
/// Divide by 16
pub const TIMER_DIV_16: u32 = 0b0011;
/// Divide by 32
pub const TIMER_DIV_32: u32 = 0b1000;
/// Divide by 64
pub const TIMER_DIV_64: u32 = 0b1001;
/// Divide by 128
pub const TIMER_DIV_128: u32 = 0b1010;

// =============================================================================
// APIC STATE
// =============================================================================

/// APIC base address
static APIC_BASE: AtomicU64 = AtomicU64::new(LAPIC_BASE as u64);

/// Whether x2APIC mode is enabled
static mut X2APIC_ENABLED: bool = false;

/// APIC frequency in Hz
static APIC_FREQUENCY: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// I/O APIC STRUCTURES
// =============================================================================

/// I/O APIC registers
pub const IOAPIC_REG_ID: u32 = 0x00;
pub const IOAPIC_REG_VER: u32 = 0x01;
pub const IOAPIC_REG_ARB: u32 = 0x02;
pub const IOAPIC_REG_REDTBL_BASE: u32 = 0x10;

/// I/O APIC descriptor
#[derive(Debug, Clone, Copy)]
pub struct IoApic {
    /// Base address
    pub base: u64,
    /// Global System Interrupt base
    pub gsi_base: u32,
    /// Number of redirection entries
    pub max_entries: u32,
}

/// I/O APIC redirection entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IoApicRedirectionEntry {
    pub low: u32,
    pub high: u32,
}

impl IoApicRedirectionEntry {
    /// Create a new redirection entry
    pub fn new(vector: u8, dest_apic: u8, masked: bool) -> Self {
        let low = (vector as u32) | if masked { 1 << 16 } else { 0 };
        let high = (dest_apic as u32) << 24;
        Self { low, high }
    }

    /// Set destination mode (0 = physical, 1 = logical)
    pub fn set_dest_mode(&mut self, logical: bool) {
        if logical {
            self.low |= 1 << 11;
        } else {
            self.low &= !(1 << 11);
        }
    }

    /// Set trigger mode (0 = edge, 1 = level)
    pub fn set_trigger_mode(&mut self, level: bool) {
        if level {
            self.low |= 1 << 15;
        } else {
            self.low &= !(1 << 15);
        }
    }

    /// Set polarity (0 = high, 1 = low)
    pub fn set_polarity(&mut self, low: bool) {
        if low {
            self.low |= 1 << 13;
        } else {
            self.low &= !(1 << 13);
        }
    }
}

/// Static I/O APIC list
static mut IOAPICS: [Option<IoApic>; 8] = [None; 8];
static mut IOAPIC_COUNT: usize = 0;

// =============================================================================
// LOCAL APIC ACCESS
// =============================================================================

/// Read from local APIC register (xAPIC mode)
unsafe fn lapic_read_xapic(reg: u32) -> u32 {
    let addr = APIC_BASE.load(Ordering::SeqCst) + reg as u64;
    core::ptr::read_volatile(addr as *const u32)
}

/// Write to local APIC register (xAPIC mode)
unsafe fn lapic_write_xapic(reg: u32, value: u32) {
    let addr = APIC_BASE.load(Ordering::SeqCst) + reg as u64;
    core::ptr::write_volatile(addr as *mut u32, value);
}

/// Read from local APIC register (x2APIC mode)
unsafe fn lapic_read_x2apic(reg: u32) -> u32 {
    let msr = lapic_to_x2apic_msr(reg);
    rdmsr(msr) as u32
}

/// Write to local APIC register (x2APIC mode)
unsafe fn lapic_write_x2apic(reg: u32, value: u32) {
    let msr = lapic_to_x2apic_msr(reg);
    wrmsr(msr, value as u64);
}

/// Read from local APIC register (auto-selects mode)
pub unsafe fn lapic_read(reg: u32) -> u32 {
    if X2APIC_ENABLED {
        lapic_read_x2apic(reg)
    } else {
        lapic_read_xapic(reg)
    }
}

/// Write to local APIC register (auto-selects mode)
pub unsafe fn lapic_write(reg: u32, value: u32) {
    if X2APIC_ENABLED {
        lapic_write_x2apic(reg, value)
    } else {
        lapic_write_xapic(reg, value)
    }
}

// =============================================================================
// I/O APIC ACCESS
// =============================================================================

/// Read from I/O APIC register
pub unsafe fn ioapic_read(ioapic: &IoApic, reg: u32) -> u32 {
    let ioregsel = ioapic.base as *mut u32;
    let iowin = (ioapic.base + 0x10) as *mut u32;

    core::ptr::write_volatile(ioregsel, reg);
    core::ptr::read_volatile(iowin)
}

/// Write to I/O APIC register
pub unsafe fn ioapic_write(ioapic: &IoApic, reg: u32, value: u32) {
    let ioregsel = ioapic.base as *mut u32;
    let iowin = (ioapic.base + 0x10) as *mut u32;

    core::ptr::write_volatile(ioregsel, reg);
    core::ptr::write_volatile(iowin, value);
}

/// Read I/O APIC redirection entry
pub unsafe fn ioapic_read_redirect(ioapic: &IoApic, irq: u8) -> IoApicRedirectionEntry {
    let reg = IOAPIC_REG_REDTBL_BASE + (irq as u32 * 2);
    let low = ioapic_read(ioapic, reg);
    let high = ioapic_read(ioapic, reg + 1);
    IoApicRedirectionEntry { low, high }
}

/// Write I/O APIC redirection entry
pub unsafe fn ioapic_write_redirect(ioapic: &IoApic, irq: u8, entry: IoApicRedirectionEntry) {
    let reg = IOAPIC_REG_REDTBL_BASE + (irq as u32 * 2);
    ioapic_write(ioapic, reg, entry.low);
    ioapic_write(ioapic, reg + 1, entry.high);
}

// =============================================================================
// APIC INITIALIZATION
// =============================================================================

/// Check if APIC is present
pub unsafe fn has_apic() -> bool {
    let (_, _, _, edx) = cpuid(1, 0);
    (edx & (1 << 9)) != 0
}

/// Check if x2APIC is supported
pub unsafe fn has_x2apic() -> bool {
    let (_, _, ecx, _) = cpuid(1, 0);
    (ecx & (1 << 21)) != 0
}

/// Get APIC base address from MSR
pub unsafe fn get_apic_base() -> u64 {
    let msr_value = rdmsr(MSR_APIC_BASE);
    msr_value & 0xFFFF_FFFF_FFFF_F000
}

/// Enable the local APIC
pub unsafe fn enable_lapic() {
    let msr_value = rdmsr(MSR_APIC_BASE);
    wrmsr(MSR_APIC_BASE, msr_value | (1 << 11)); // Enable bit
}

/// Enable x2APIC mode
pub unsafe fn enable_x2apic() {
    let msr_value = rdmsr(MSR_APIC_BASE);
    wrmsr(MSR_APIC_BASE, msr_value | (1 << 10) | (1 << 11)); // x2APIC + enable
    X2APIC_ENABLED = true;
}

/// Initialize the local APIC
pub unsafe fn init_lapic(ctx: &mut BootContext) -> BootResult<()> {
    if !has_apic() {
        return Err(BootError::HardwareNotSupported);
    }

    // Get APIC base
    let base = get_apic_base();
    APIC_BASE.store(base, Ordering::SeqCst);

    // Enable APIC
    enable_lapic();

    // Try x2APIC if supported and configured
    if ctx.config.prefer_x2apic && has_x2apic() {
        enable_x2apic();
        ctx.interrupt_state.controller_type = InterruptControllerType::X2apic;
    } else {
        ctx.interrupt_state.controller_type = InterruptControllerType::Apic;
    }

    // Set spurious interrupt vector and enable APIC
    let svr = lapic_read(LAPIC_SVR);
    lapic_write(LAPIC_SVR, svr | 0xFF | (1 << 8)); // Vector 0xFF, enable APIC

    // Clear task priority to accept all interrupts
    lapic_write(LAPIC_TPR, 0);

    // Set destination format to flat model
    if !X2APIC_ENABLED {
        lapic_write(LAPIC_DFR, 0xFFFFFFFF); // Flat model

        // Set logical destination
        let apic_id = (lapic_read(LAPIC_ID) >> 24) as u8;
        lapic_write(LAPIC_LDR, (1u32 << apic_id) << 24);
    }

    // Mask all LVT entries initially
    lapic_write(LAPIC_LVT_TIMER, LVT_MASKED);
    lapic_write(LAPIC_LVT_THERMAL, LVT_MASKED);
    lapic_write(LAPIC_LVT_PERF, LVT_MASKED);
    lapic_write(LAPIC_LVT_LINT0, LVT_MASKED);
    lapic_write(LAPIC_LVT_LINT1, LVT_MASKED);
    lapic_write(LAPIC_LVT_ERROR, LVT_MASKED);

    // Enable error interrupt
    lapic_write(LAPIC_ESR, 0); // Clear errors
    lapic_write(LAPIC_LVT_ERROR, 0xFC); // Vector 0xFC

    // Clear any pending interrupts
    lapic_write(LAPIC_EOI, 0);

    // Store APIC info in context
    ctx.arch_data.x86.apic_base = base;
    ctx.arch_data.x86.x2apic_enabled = X2APIC_ENABLED;
    ctx.arch_data.x86.bsp_apic_id = get_apic_id() as u32;

    Ok(())
}

// =============================================================================
// I/O APIC INITIALIZATION
// =============================================================================

/// Initialize I/O APIC
pub unsafe fn init_ioapic(ctx: &mut BootContext) -> BootResult<()> {
    // Find I/O APIC from ACPI tables
    let ioapic_base = find_ioapic_base(ctx)?;

    // Create I/O APIC descriptor
    let version = {
        let ioregsel = ioapic_base as *mut u32;
        let iowin = (ioapic_base + 0x10) as *mut u32;
        core::ptr::write_volatile(ioregsel, IOAPIC_REG_VER);
        core::ptr::read_volatile(iowin)
    };

    let max_entries = ((version >> 16) & 0xFF) + 1;

    let ioapic = IoApic {
        base: ioapic_base,
        gsi_base: 0,
        max_entries,
    };

    // Store in list
    IOAPICS[0] = Some(ioapic);
    IOAPIC_COUNT = 1;

    // Mask all I/O APIC entries
    for i in 0..max_entries {
        let mut entry = IoApicRedirectionEntry::new(0, 0, true);
        ioapic_write_redirect(&ioapic, i as u8, entry);
    }

    // Set up ISA IRQ mappings (IRQ 0-15)
    let apic_id = get_apic_id() as u8;

    // Timer (IRQ 0) -> Vector 32
    configure_ioapic_irq(&ioapic, 0, 32, apic_id, false)?;

    // Keyboard (IRQ 1) -> Vector 33
    configure_ioapic_irq(&ioapic, 1, 33, apic_id, false)?;

    // COM2 (IRQ 3) -> Vector 35
    configure_ioapic_irq(&ioapic, 3, 35, apic_id, false)?;

    // COM1 (IRQ 4) -> Vector 36
    configure_ioapic_irq(&ioapic, 4, 36, apic_id, false)?;

    // Store in context
    ctx.arch_data.x86.ioapic_base = ioapic_base;
    ctx.arch_data.x86.ioapic_count = 1;

    Ok(())
}

/// Find I/O APIC base address from ACPI or use default
fn find_ioapic_base(_ctx: &BootContext) -> BootResult<u64> {
    // TODO: Parse ACPI MADT table for I/O APIC info
    // For now, use default address
    Ok(IOAPIC_BASE as u64)
}

/// Configure an I/O APIC IRQ
unsafe fn configure_ioapic_irq(
    ioapic: &IoApic,
    irq: u8,
    vector: u8,
    dest_apic: u8,
    masked: bool,
) -> BootResult<()> {
    if irq as u32 >= ioapic.max_entries {
        return Err(BootError::InvalidParameter);
    }

    let entry = IoApicRedirectionEntry::new(vector, dest_apic, masked);
    ioapic_write_redirect(ioapic, irq, entry);

    Ok(())
}

// =============================================================================
// INTERRUPT CONTROL
// =============================================================================

/// Send End of Interrupt
pub unsafe fn send_eoi() {
    lapic_write(LAPIC_EOI, 0);
}

/// Send IPI (Inter-Processor Interrupt)
pub unsafe fn send_ipi(dest: u8, vector: u8, flags: u32) {
    if X2APIC_ENABLED {
        // x2APIC uses single MSR write
        let icr = ((dest as u64) << 32) | ((flags | vector as u32) as u64);
        wrmsr(lapic_to_x2apic_msr(LAPIC_ICR_LOW), icr);
    } else {
        // xAPIC uses two register writes
        lapic_write(LAPIC_ICR_HIGH, (dest as u32) << 24);
        lapic_write(LAPIC_ICR_LOW, flags | vector as u32);
    }
}

/// Wait for IPI delivery
pub unsafe fn wait_ipi_delivery() {
    if !X2APIC_ENABLED {
        while lapic_read(LAPIC_ICR_LOW) & (1 << 12) != 0 {
            core::hint::spin_loop();
        }
    }
}

/// Send INIT IPI
pub unsafe fn send_init_ipi(dest: u8) {
    send_ipi(dest, 0, ICR_INIT | ICR_ASSERT | ICR_TRIGGER_LEVEL);
    wait_ipi_delivery();

    // De-assert INIT
    send_ipi(dest, 0, ICR_INIT | ICR_DEASSERT | ICR_TRIGGER_LEVEL);
    wait_ipi_delivery();
}

/// Send SIPI (Startup IPI)
pub unsafe fn send_sipi(dest: u8, vector: u8) {
    send_ipi(dest, vector, ICR_SIPI | ICR_ASSERT);
    wait_ipi_delivery();
}

/// Send NMI to another processor
pub unsafe fn send_nmi(dest: u8) {
    send_ipi(dest, 0, ICR_NMI | ICR_ASSERT);
    wait_ipi_delivery();
}

// =============================================================================
// APIC TIMER
// =============================================================================

/// Calibrate APIC timer frequency
pub unsafe fn calibrate_apic_timer() -> u64 {
    // Use PIT to calibrate APIC timer
    const PIT_FREQUENCY: u64 = 1193182;
    const CALIBRATION_MS: u64 = 10;

    // Set APIC timer to one-shot mode with maximum count
    lapic_write(LAPIC_TIMER_DCR, TIMER_DIV_16);
    lapic_write(LAPIC_LVT_TIMER, LVT_MASKED | 0xFE);
    lapic_write(LAPIC_TIMER_ICR, 0xFFFFFFFF);

    // Wait using PIT
    let pit_count = (PIT_FREQUENCY * CALIBRATION_MS) / 1000;
    pit_wait(pit_count as u16);

    // Read APIC timer count
    let elapsed = 0xFFFFFFFF - lapic_read(LAPIC_TIMER_CCR);

    // Calculate frequency
    let frequency = (elapsed as u64 * 16 * 1000) / CALIBRATION_MS;

    APIC_FREQUENCY.store(frequency, Ordering::SeqCst);

    frequency
}

/// Wait using PIT (Programmable Interval Timer)
unsafe fn pit_wait(count: u16) {
    const PIT_CH2_DATA: u16 = 0x42;
    const PIT_CMD: u16 = 0x43;
    const PIT_CH2_GATE: u16 = 0x61;

    // Configure PIT channel 2 for one-shot mode
    outb(PIT_CMD, 0xB0); // Channel 2, lobyte/hibyte, mode 0
    outb(PIT_CH2_DATA, (count & 0xFF) as u8);
    outb(PIT_CH2_DATA, (count >> 8) as u8);

    // Enable channel 2 gate
    let gate = inb(PIT_CH2_GATE);
    outb(PIT_CH2_GATE, gate | 1);

    // Wait for count to reach 0
    while inb(PIT_CH2_GATE) & 0x20 == 0 {
        core::hint::spin_loop();
    }

    // Disable channel 2 gate
    outb(PIT_CH2_GATE, gate);
}

/// Set up APIC timer for periodic interrupts
pub unsafe fn setup_apic_timer(frequency_hz: u64, vector: u8) {
    let apic_freq = APIC_FREQUENCY.load(Ordering::SeqCst);
    if apic_freq == 0 {
        return;
    }

    let count = apic_freq / frequency_hz;

    lapic_write(LAPIC_TIMER_DCR, TIMER_DIV_16);
    lapic_write(LAPIC_LVT_TIMER, LVT_TIMER_PERIODIC | vector as u32);
    lapic_write(LAPIC_TIMER_ICR, count as u32);
}

/// Stop APIC timer
pub unsafe fn stop_apic_timer() {
    lapic_write(LAPIC_LVT_TIMER, LVT_MASKED);
    lapic_write(LAPIC_TIMER_ICR, 0);
}

// =============================================================================
// LEGACY PIC
// =============================================================================

/// Master PIC command port
const PIC1_CMD: u16 = 0x20;
/// Master PIC data port
const PIC1_DATA: u16 = 0x21;
/// Slave PIC command port
const PIC2_CMD: u16 = 0xA0;
/// Slave PIC data port
const PIC2_DATA: u16 = 0xA1;

/// Disable the legacy 8259 PIC
pub unsafe fn disable_pic() {
    // Initialize PICs
    outb(PIC1_CMD, 0x11); // ICW1: init + ICW4 needed
    outb(PIC2_CMD, 0x11);

    // ICW2: interrupt vector offsets
    outb(PIC1_DATA, 0x20); // Master: vectors 0x20-0x27
    outb(PIC2_DATA, 0x28); // Slave: vectors 0x28-0x2F

    // ICW3: tell Master/Slave relationship
    outb(PIC1_DATA, 0x04); // Master: slave on IRQ2
    outb(PIC2_DATA, 0x02); // Slave: cascade identity

    // ICW4: 8086 mode
    outb(PIC1_DATA, 0x01);
    outb(PIC2_DATA, 0x01);

    // Mask all interrupts
    outb(PIC1_DATA, 0xFF);
    outb(PIC2_DATA, 0xFF);
}

/// Send EOI to legacy PIC
pub unsafe fn pic_send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_CMD, 0x20);
    }
    outb(PIC1_CMD, 0x20);
}

// =============================================================================
// APIC QUERIES
// =============================================================================

/// Get local APIC ID
pub unsafe fn get_apic_id() -> u32 {
    if X2APIC_ENABLED {
        lapic_read(LAPIC_ID)
    } else {
        lapic_read(LAPIC_ID) >> 24
    }
}

/// Get APIC version
pub unsafe fn get_apic_version() -> u32 {
    lapic_read(LAPIC_VERSION) & 0xFF
}

/// Get maximum LVT entry
pub unsafe fn get_max_lvt() -> u32 {
    ((lapic_read(LAPIC_VERSION) >> 16) & 0xFF) + 1
}

/// Check if we're on the BSP
pub unsafe fn is_bsp() -> bool {
    (rdmsr(MSR_APIC_BASE) & (1 << 8)) != 0
}

/// Get APIC frequency
pub fn get_apic_frequency() -> u64 {
    APIC_FREQUENCY.load(Ordering::SeqCst)
}
