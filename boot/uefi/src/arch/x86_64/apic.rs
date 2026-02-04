//! x86_64 APIC Support
//!
//! Local APIC and I/O APIC implementation.

use core::sync::atomic::{AtomicU32, Ordering};

use super::{rdmsr, wrmsr};
use crate::raw::types::*;

// =============================================================================
// LOCAL APIC CONSTANTS
// =============================================================================

/// Local APIC base MSR
pub const IA32_APIC_BASE: u32 = 0x1B;

/// APIC base flags
pub mod apic_base {
    /// BSP (Bootstrap Processor)
    pub const BSP: u64 = 1 << 8;
    /// APIC Global Enable
    pub const ENABLE: u64 = 1 << 11;
    /// x2APIC mode
    pub const X2APIC: u64 = 1 << 10;
    /// Base address mask
    pub const ADDRESS_MASK: u64 = 0x0000_00FF_FFFF_F000;
}

/// Local APIC register offsets
pub mod reg {
    /// APIC ID
    pub const ID: u32 = 0x020;
    /// APIC Version
    pub const VERSION: u32 = 0x030;
    /// Task Priority Register
    pub const TPR: u32 = 0x080;
    /// Arbitration Priority Register
    pub const APR: u32 = 0x090;
    /// Processor Priority Register
    pub const PPR: u32 = 0x0A0;
    /// EOI Register
    pub const EOI: u32 = 0x0B0;
    /// Remote Read Register
    pub const RRD: u32 = 0x0C0;
    /// Logical Destination Register
    pub const LDR: u32 = 0x0D0;
    /// Destination Format Register
    pub const DFR: u32 = 0x0E0;
    /// Spurious Interrupt Vector Register
    pub const SIVR: u32 = 0x0F0;
    /// In-Service Register (256 bits, 8 registers)
    pub const ISR0: u32 = 0x100;
    pub const ISR1: u32 = 0x110;
    pub const ISR2: u32 = 0x120;
    pub const ISR3: u32 = 0x130;
    pub const ISR4: u32 = 0x140;
    pub const ISR5: u32 = 0x150;
    pub const ISR6: u32 = 0x160;
    pub const ISR7: u32 = 0x170;
    /// Trigger Mode Register (256 bits, 8 registers)
    pub const TMR0: u32 = 0x180;
    pub const TMR1: u32 = 0x190;
    pub const TMR2: u32 = 0x1A0;
    pub const TMR3: u32 = 0x1B0;
    pub const TMR4: u32 = 0x1C0;
    pub const TMR5: u32 = 0x1D0;
    pub const TMR6: u32 = 0x1E0;
    pub const TMR7: u32 = 0x1F0;
    /// Interrupt Request Register (256 bits, 8 registers)
    pub const IRR0: u32 = 0x200;
    pub const IRR1: u32 = 0x210;
    pub const IRR2: u32 = 0x220;
    pub const IRR3: u32 = 0x230;
    pub const IRR4: u32 = 0x240;
    pub const IRR5: u32 = 0x250;
    pub const IRR6: u32 = 0x260;
    pub const IRR7: u32 = 0x270;
    /// Error Status Register
    pub const ESR: u32 = 0x280;
    /// LVT CMCI Register
    pub const LVT_CMCI: u32 = 0x2F0;
    /// Interrupt Command Register (low)
    pub const ICR_LOW: u32 = 0x300;
    /// Interrupt Command Register (high)
    pub const ICR_HIGH: u32 = 0x310;
    /// LVT Timer Register
    pub const LVT_TIMER: u32 = 0x320;
    /// LVT Thermal Sensor Register
    pub const LVT_THERMAL: u32 = 0x330;
    /// LVT Performance Counter Register
    pub const LVT_PMC: u32 = 0x340;
    /// LVT LINT0 Register
    pub const LVT_LINT0: u32 = 0x350;
    /// LVT LINT1 Register
    pub const LVT_LINT1: u32 = 0x360;
    /// LVT Error Register
    pub const LVT_ERROR: u32 = 0x370;
    /// Timer Initial Count Register
    pub const TIMER_ICR: u32 = 0x380;
    /// Timer Current Count Register
    pub const TIMER_CCR: u32 = 0x390;
    /// Timer Divide Configuration Register
    pub const TIMER_DCR: u32 = 0x3E0;
    /// Self IPI Register (x2APIC only)
    pub const SELF_IPI: u32 = 0x3F0;
}

/// LVT flags
pub mod lvt {
    /// Vector mask (bits 0-7)
    pub const VECTOR_MASK: u32 = 0xFF;
    /// Delivery mode mask
    pub const DELIVERY_MASK: u32 = 0x700;
    /// Fixed delivery mode
    pub const DELIVERY_FIXED: u32 = 0x000;
    /// SMI delivery mode
    pub const DELIVERY_SMI: u32 = 0x200;
    /// NMI delivery mode
    pub const DELIVERY_NMI: u32 = 0x400;
    /// ExtINT delivery mode
    pub const DELIVERY_EXTINT: u32 = 0x700;
    /// INIT delivery mode
    pub const DELIVERY_INIT: u32 = 0x500;
    /// Pending status
    pub const PENDING: u32 = 1 << 12;
    /// Polarity (0 = high, 1 = low)
    pub const POLARITY_LOW: u32 = 1 << 13;
    /// Remote IRR
    pub const REMOTE_IRR: u32 = 1 << 14;
    /// Trigger mode (0 = edge, 1 = level)
    pub const TRIGGER_LEVEL: u32 = 1 << 15;
    /// Masked
    pub const MASKED: u32 = 1 << 16;
    /// Timer mode: one-shot
    pub const TIMER_ONESHOT: u32 = 0 << 17;
    /// Timer mode: periodic
    pub const TIMER_PERIODIC: u32 = 1 << 17;
    /// Timer mode: TSC deadline
    pub const TIMER_TSC_DEADLINE: u32 = 2 << 17;
}

/// ICR delivery modes
pub mod icr {
    /// Fixed delivery
    pub const DELIVERY_FIXED: u32 = 0 << 8;
    /// Lowest priority
    pub const DELIVERY_LOWEST: u32 = 1 << 8;
    /// SMI
    pub const DELIVERY_SMI: u32 = 2 << 8;
    /// NMI
    pub const DELIVERY_NMI: u32 = 4 << 8;
    /// INIT
    pub const DELIVERY_INIT: u32 = 5 << 8;
    /// Start Up (SIPI)
    pub const DELIVERY_STARTUP: u32 = 6 << 8;
    /// Logical destination mode
    pub const DEST_LOGICAL: u32 = 1 << 11;
    /// Delivery status pending
    pub const STATUS_PENDING: u32 = 1 << 12;
    /// Level assert
    pub const LEVEL_ASSERT: u32 = 1 << 14;
    /// Level de-assert
    pub const LEVEL_DEASSERT: u32 = 0 << 14;
    /// Edge trigger
    pub const TRIGGER_EDGE: u32 = 0 << 15;
    /// Level trigger
    pub const TRIGGER_LEVEL: u32 = 1 << 15;
    /// Destination shorthand: no shorthand
    pub const DEST_NONE: u32 = 0 << 18;
    /// Destination shorthand: self
    pub const DEST_SELF: u32 = 1 << 18;
    /// Destination shorthand: all including self
    pub const DEST_ALL_INCL: u32 = 2 << 18;
    /// Destination shorthand: all excluding self
    pub const DEST_ALL_EXCL: u32 = 3 << 18;
}

/// Timer divide values
pub mod timer_div {
    /// Divide by 2
    pub const DIV_2: u32 = 0b0000;
    /// Divide by 4
    pub const DIV_4: u32 = 0b0001;
    /// Divide by 8
    pub const DIV_8: u32 = 0b0010;
    /// Divide by 16
    pub const DIV_16: u32 = 0b0011;
    /// Divide by 32
    pub const DIV_32: u32 = 0b1000;
    /// Divide by 64
    pub const DIV_64: u32 = 0b1001;
    /// Divide by 128
    pub const DIV_128: u32 = 0b1010;
    /// Divide by 1
    pub const DIV_1: u32 = 0b1011;
}

// =============================================================================
// LOCAL APIC
// =============================================================================

/// Local APIC interface
pub struct LocalApic {
    /// Base address (virtual)
    base: VirtualAddress,
    /// x2APIC mode
    x2apic: bool,
}

impl LocalApic {
    /// APIC ID for BSP
    pub const BSP_ID: u32 = 0;

    /// Create Local APIC with physical address
    ///
    /// # Safety
    /// Must be called with valid APIC base address mapping
    pub unsafe fn new(base: VirtualAddress) -> Self {
        Self {
            base,
            x2apic: false,
        }
    }

    /// Create x2APIC interface
    pub fn new_x2apic() -> Self {
        Self {
            base: VirtualAddress(0),
            x2apic: true,
        }
    }

    /// Check if x2APIC is supported
    pub fn is_x2apic_supported() -> bool {
        let result = super::cpuid(1, 0);
        (result.ecx & (1 << 21)) != 0
    }

    /// Get APIC base address from MSR
    pub fn get_base_address() -> PhysicalAddress {
        let msr = unsafe { rdmsr(IA32_APIC_BASE) };
        PhysicalAddress(msr & apic_base::ADDRESS_MASK)
    }

    /// Check if APIC is enabled
    pub fn is_enabled() -> bool {
        let msr = unsafe { rdmsr(IA32_APIC_BASE) };
        (msr & apic_base::ENABLE) != 0
    }

    /// Check if this is the BSP
    pub fn is_bsp() -> bool {
        let msr = unsafe { rdmsr(IA32_APIC_BASE) };
        (msr & apic_base::BSP) != 0
    }

    /// Enable APIC
    pub fn enable(&self) {
        unsafe {
            let mut msr = rdmsr(IA32_APIC_BASE);
            msr |= apic_base::ENABLE;
            if self.x2apic {
                msr |= apic_base::X2APIC;
            }
            wrmsr(IA32_APIC_BASE, msr);
        }
    }

    /// Read APIC register
    pub fn read(&self, offset: u32) -> u32 {
        if self.x2apic {
            // x2APIC uses MSRs
            let msr = 0x800 + (offset >> 4);
            unsafe { rdmsr(msr) as u32 }
        } else {
            // xAPIC uses MMIO
            unsafe {
                let ptr = (self.base.0 + offset as u64) as *const u32;
                core::ptr::read_volatile(ptr)
            }
        }
    }

    /// Write APIC register
    pub fn write(&self, offset: u32, value: u32) {
        if self.x2apic {
            // x2APIC uses MSRs
            let msr = 0x800 + (offset >> 4);
            unsafe { wrmsr(msr, value as u64) }
        } else {
            // xAPIC uses MMIO
            unsafe {
                let ptr = (self.base.0 + offset as u64) as *mut u32;
                core::ptr::write_volatile(ptr, value);
            }
        }
    }

    /// Get APIC ID
    pub fn id(&self) -> u32 {
        let value = self.read(reg::ID);
        if self.x2apic {
            value
        } else {
            value >> 24
        }
    }

    /// Get APIC version
    pub fn version(&self) -> ApicVersion {
        let value = self.read(reg::VERSION);
        ApicVersion {
            version: (value & 0xFF) as u8,
            max_lvt_entry: ((value >> 16) & 0xFF) as u8,
            eoi_broadcast_suppression: (value & (1 << 24)) != 0,
        }
    }

    /// Send End Of Interrupt
    pub fn eoi(&self) {
        self.write(reg::EOI, 0);
    }

    /// Set task priority
    pub fn set_task_priority(&self, priority: u8) {
        self.write(reg::TPR, priority as u32);
    }

    /// Get task priority
    pub fn task_priority(&self) -> u8 {
        self.read(reg::TPR) as u8
    }

    /// Set spurious vector and enable APIC
    pub fn set_spurious_vector(&self, vector: u8, enable: bool) {
        let mut value = vector as u32;
        if enable {
            value |= 1 << 8;
        }
        self.write(reg::SIVR, value);
    }

    /// Configure LVT Timer
    pub fn configure_timer(&self, vector: u8, mode: u32) {
        let value = (vector as u32) | mode;
        self.write(reg::LVT_TIMER, value);
    }

    /// Set timer initial count
    pub fn set_timer_initial_count(&self, count: u32) {
        self.write(reg::TIMER_ICR, count);
    }

    /// Get timer current count
    pub fn timer_current_count(&self) -> u32 {
        self.read(reg::TIMER_CCR)
    }

    /// Set timer divide configuration
    pub fn set_timer_divide(&self, divide: u32) {
        self.write(reg::TIMER_DCR, divide);
    }

    /// Mask timer
    pub fn mask_timer(&self) {
        let value = self.read(reg::LVT_TIMER) | lvt::MASKED;
        self.write(reg::LVT_TIMER, value);
    }

    /// Unmask timer
    pub fn unmask_timer(&self) {
        let value = self.read(reg::LVT_TIMER) & !lvt::MASKED;
        self.write(reg::LVT_TIMER, value);
    }

    /// Start one-shot timer
    pub fn start_oneshot(&self, vector: u8, initial_count: u32, divide: u32) {
        self.set_timer_divide(divide);
        self.configure_timer(vector, lvt::TIMER_ONESHOT);
        self.set_timer_initial_count(initial_count);
    }

    /// Start periodic timer
    pub fn start_periodic(&self, vector: u8, initial_count: u32, divide: u32) {
        self.set_timer_divide(divide);
        self.configure_timer(vector, lvt::TIMER_PERIODIC);
        self.set_timer_initial_count(initial_count);
    }

    /// Stop timer
    pub fn stop_timer(&self) {
        self.mask_timer();
        self.set_timer_initial_count(0);
    }

    /// Configure LVT LINT0
    pub fn configure_lint0(&self, vector: u8, delivery_mode: u32, masked: bool) {
        let mut value = (vector as u32) | delivery_mode;
        if masked {
            value |= lvt::MASKED;
        }
        self.write(reg::LVT_LINT0, value);
    }

    /// Configure LVT LINT1
    pub fn configure_lint1(&self, vector: u8, delivery_mode: u32, masked: bool) {
        let mut value = (vector as u32) | delivery_mode;
        if masked {
            value |= lvt::MASKED;
        }
        self.write(reg::LVT_LINT1, value);
    }

    /// Configure LVT Error
    pub fn configure_error(&self, vector: u8) {
        self.write(reg::LVT_ERROR, vector as u32);
    }

    /// Get error status
    pub fn error_status(&self) -> u32 {
        // Write to ESR to update status
        self.write(reg::ESR, 0);
        self.read(reg::ESR)
    }

    /// Clear error status
    pub fn clear_error(&self) {
        self.write(reg::ESR, 0);
    }

    /// Send IPI to specific CPU
    pub fn send_ipi(&self, destination: u32, vector: u8, delivery_mode: u32) {
        if self.x2apic {
            // x2APIC: single 64-bit write to ICR
            let icr = ((destination as u64) << 32) | (vector as u64) | (delivery_mode as u64);
            unsafe { wrmsr(0x830, icr) }
        } else {
            // xAPIC: write high then low
            self.write(reg::ICR_HIGH, destination << 24);
            self.write(reg::ICR_LOW, (vector as u32) | delivery_mode);
        }
    }

    /// Send IPI to self
    pub fn send_self_ipi(&self, vector: u8) {
        if self.x2apic {
            self.write(reg::SELF_IPI, vector as u32);
        } else {
            self.send_ipi(0, vector, icr::DEST_SELF);
        }
    }

    /// Send INIT IPI
    pub fn send_init(&self, destination: u32) {
        self.send_ipi(
            destination,
            0,
            icr::DELIVERY_INIT | icr::LEVEL_ASSERT | icr::TRIGGER_LEVEL,
        );
    }

    /// Send SIPI (Startup IPI)
    pub fn send_sipi(&self, destination: u32, vector: u8) {
        self.send_ipi(
            destination,
            vector,
            icr::DELIVERY_STARTUP | icr::LEVEL_ASSERT,
        );
    }

    /// Broadcast IPI to all CPUs except self
    pub fn broadcast_ipi(&self, vector: u8) {
        self.send_ipi(0, vector, icr::DEST_ALL_EXCL | icr::DELIVERY_FIXED);
    }

    /// Broadcast NMI to all CPUs except self
    pub fn broadcast_nmi(&self) {
        self.send_ipi(0, 0, icr::DEST_ALL_EXCL | icr::DELIVERY_NMI);
    }

    /// Check if ICR is pending
    pub fn is_ipi_pending(&self) -> bool {
        if self.x2apic {
            false // x2APIC delivery is synchronous
        } else {
            (self.read(reg::ICR_LOW) & icr::STATUS_PENDING) != 0
        }
    }

    /// Wait for IPI delivery
    pub fn wait_for_ipi_delivery(&self) {
        while self.is_ipi_pending() {
            core::hint::spin_loop();
        }
    }

    /// Check if interrupt is in service
    pub fn is_in_service(&self, vector: u8) -> bool {
        let reg_offset = reg::ISR0 + ((vector as u32 / 32) * 0x10);
        let bit = vector % 32;
        (self.read(reg_offset) & (1 << bit)) != 0
    }

    /// Check if interrupt is pending
    pub fn is_pending(&self, vector: u8) -> bool {
        let reg_offset = reg::IRR0 + ((vector as u32 / 32) * 0x10);
        let bit = vector % 32;
        (self.read(reg_offset) & (1 << bit)) != 0
    }
}

/// APIC version information
#[derive(Debug, Clone, Copy)]
pub struct ApicVersion {
    /// APIC version
    pub version: u8,
    /// Maximum LVT entry
    pub max_lvt_entry: u8,
    /// EOI broadcast suppression support
    pub eoi_broadcast_suppression: bool,
}

// =============================================================================
// I/O APIC
// =============================================================================

/// I/O APIC register select
const IOREGSEL: u32 = 0x00;
/// I/O APIC register data
const IOWIN: u32 = 0x10;

/// I/O APIC registers
pub mod ioapic_reg {
    /// I/O APIC ID
    pub const ID: u8 = 0x00;
    /// I/O APIC Version
    pub const VER: u8 = 0x01;
    /// I/O APIC Arbitration ID
    pub const ARB: u8 = 0x02;
    /// Redirection table entry base
    pub const REDTBL: u8 = 0x10;
}

/// I/O APIC redirection entry flags
pub mod redir {
    /// Delivery mode: Fixed
    pub const DELIVERY_FIXED: u64 = 0 << 8;
    /// Delivery mode: Lowest priority
    pub const DELIVERY_LOWEST: u64 = 1 << 8;
    /// Delivery mode: SMI
    pub const DELIVERY_SMI: u64 = 2 << 8;
    /// Delivery mode: NMI
    pub const DELIVERY_NMI: u64 = 4 << 8;
    /// Delivery mode: INIT
    pub const DELIVERY_INIT: u64 = 5 << 8;
    /// Delivery mode: ExtINT
    pub const DELIVERY_EXTINT: u64 = 7 << 8;
    /// Destination mode: Physical
    pub const DEST_PHYSICAL: u64 = 0 << 11;
    /// Destination mode: Logical
    pub const DEST_LOGICAL: u64 = 1 << 11;
    /// Delivery status
    pub const DELIVERY_PENDING: u64 = 1 << 12;
    /// Polarity: Active high
    pub const POLARITY_HIGH: u64 = 0 << 13;
    /// Polarity: Active low
    pub const POLARITY_LOW: u64 = 1 << 13;
    /// Remote IRR
    pub const REMOTE_IRR: u64 = 1 << 14;
    /// Trigger mode: Edge
    pub const TRIGGER_EDGE: u64 = 0 << 15;
    /// Trigger mode: Level
    pub const TRIGGER_LEVEL: u64 = 1 << 15;
    /// Masked
    pub const MASKED: u64 = 1 << 16;
}

/// I/O APIC interface
pub struct IoApic {
    /// Base address (virtual)
    base: VirtualAddress,
    /// GSI base
    gsi_base: u32,
}

impl IoApic {
    /// Create I/O APIC with virtual address
    ///
    /// # Safety
    /// Must be called with valid I/O APIC base address mapping
    pub unsafe fn new(base: VirtualAddress, gsi_base: u32) -> Self {
        Self { base, gsi_base }
    }

    /// Read I/O APIC register
    fn read(&self, reg: u8) -> u32 {
        unsafe {
            let regsel = self.base.0 as *mut u32;
            let window = (self.base + IOWIN as u64).0 as *mut u32;
            core::ptr::write_volatile(regsel, reg as u32);
            core::ptr::read_volatile(window)
        }
    }

    /// Write I/O APIC register
    fn write(&self, reg: u8, value: u32) {
        unsafe {
            let regsel = self.base.0 as *mut u32;
            let window = (self.base + IOWIN as u64).0 as *mut u32;
            core::ptr::write_volatile(regsel, reg as u32);
            core::ptr::write_volatile(window, value);
        }
    }

    /// Get I/O APIC ID
    pub fn id(&self) -> u8 {
        ((self.read(ioapic_reg::ID) >> 24) & 0xF) as u8
    }

    /// Set I/O APIC ID
    pub fn set_id(&self, id: u8) {
        let value = (id as u32) << 24;
        self.write(ioapic_reg::ID, value);
    }

    /// Get I/O APIC version
    pub fn version(&self) -> IoApicVersion {
        let value = self.read(ioapic_reg::VER);
        IoApicVersion {
            version: (value & 0xFF) as u8,
            max_redirection_entry: ((value >> 16) & 0xFF) as u8,
        }
    }

    /// Get GSI base
    pub fn gsi_base(&self) -> u32 {
        self.gsi_base
    }

    /// Get number of inputs
    pub fn input_count(&self) -> u8 {
        self.version().max_redirection_entry + 1
    }

    /// Check if GSI is handled by this I/O APIC
    pub fn handles_gsi(&self, gsi: u32) -> bool {
        let max = self.gsi_base + self.input_count() as u32;
        gsi >= self.gsi_base && gsi < max
    }

    /// Read redirection entry
    pub fn read_redirection(&self, index: u8) -> u64 {
        let reg_low = ioapic_reg::REDTBL + (index * 2);
        let reg_high = reg_low + 1;

        let low = self.read(reg_low) as u64;
        let high = self.read(reg_high) as u64;

        low | (high << 32)
    }

    /// Write redirection entry
    pub fn write_redirection(&self, index: u8, value: u64) {
        let reg_low = ioapic_reg::REDTBL + (index * 2);
        let reg_high = reg_low + 1;

        // Write high first, then low to avoid enabling during configuration
        self.write(reg_high, (value >> 32) as u32);
        self.write(reg_low, value as u32);
    }

    /// Configure IRQ
    pub fn configure_irq(&self, irq: u8, vector: u8, flags: u64, destination: u8) {
        let mut entry = vector as u64;
        entry |= flags;
        entry |= (destination as u64) << 56;
        self.write_redirection(irq, entry);
    }

    /// Mask IRQ
    pub fn mask(&self, irq: u8) {
        let entry = self.read_redirection(irq);
        self.write_redirection(irq, entry | redir::MASKED);
    }

    /// Unmask IRQ
    pub fn unmask(&self, irq: u8) {
        let entry = self.read_redirection(irq);
        self.write_redirection(irq, entry & !redir::MASKED);
    }

    /// Check if IRQ is masked
    pub fn is_masked(&self, irq: u8) -> bool {
        (self.read_redirection(irq) & redir::MASKED) != 0
    }

    /// Get vector for IRQ
    pub fn get_vector(&self, irq: u8) -> u8 {
        (self.read_redirection(irq) & 0xFF) as u8
    }

    /// Route legacy PIC IRQ to APIC vector
    pub fn route_legacy_irq(&self, irq: u8, vector: u8, destination: u8) {
        // Legacy IRQs are typically edge-triggered, active-high
        self.configure_irq(
            irq,
            vector,
            redir::DELIVERY_FIXED
                | redir::DEST_PHYSICAL
                | redir::POLARITY_HIGH
                | redir::TRIGGER_EDGE,
            destination,
        );
    }

    /// Disable all IRQs
    pub fn disable_all(&self) {
        let count = self.input_count();
        for i in 0..count {
            self.mask(i);
        }
    }
}

/// I/O APIC version information
#[derive(Debug, Clone, Copy)]
pub struct IoApicVersion {
    /// Version
    pub version: u8,
    /// Maximum redirection entry
    pub max_redirection_entry: u8,
}

// =============================================================================
// APIC INITIALIZATION
// =============================================================================

/// Default APIC base address
pub const DEFAULT_APIC_BASE: PhysicalAddress = PhysicalAddress(0xFEE0_0000);

/// Default I/O APIC base address
pub const DEFAULT_IOAPIC_BASE: PhysicalAddress = PhysicalAddress(0xFEC0_0000);

/// APIC configuration for kernel
#[derive(Debug, Clone)]
pub struct ApicConfig {
    /// Local APIC base address
    pub local_apic_base: PhysicalAddress,
    /// I/O APICs
    pub io_apics: [Option<IoApicConfig>; 8],
    /// Number of I/O APICs
    pub io_apic_count: usize,
    /// Use x2APIC mode
    pub x2apic: bool,
    /// BSP APIC ID
    pub bsp_id: u32,
    /// CPU count
    pub cpu_count: usize,
    /// CPU APIC IDs
    pub cpu_ids: [u32; 256],
}

/// I/O APIC configuration
#[derive(Debug, Clone, Copy)]
pub struct IoApicConfig {
    /// I/O APIC ID
    pub id: u8,
    /// Physical address
    pub address: PhysicalAddress,
    /// GSI base
    pub gsi_base: u32,
}

impl ApicConfig {
    /// Create default configuration
    pub fn new() -> Self {
        Self {
            local_apic_base: DEFAULT_APIC_BASE,
            io_apics: [None; 8],
            io_apic_count: 0,
            x2apic: false,
            bsp_id: 0,
            cpu_count: 1,
            cpu_ids: [0; 256],
        }
    }

    /// Add I/O APIC
    pub fn add_io_apic(&mut self, id: u8, address: PhysicalAddress, gsi_base: u32) {
        if self.io_apic_count < 8 {
            self.io_apics[self.io_apic_count] = Some(IoApicConfig {
                id,
                address,
                gsi_base,
            });
            self.io_apic_count += 1;
        }
    }

    /// Add CPU
    pub fn add_cpu(&mut self, apic_id: u32) {
        if self.cpu_count < 256 {
            self.cpu_ids[self.cpu_count] = apic_id;
            self.cpu_count += 1;
        }
    }
}

impl Default for ApicConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LEGACY PIC DISABLE
// =============================================================================

/// PIC (8259A) ports
pub mod pic {
    /// Master PIC command port
    pub const MASTER_COMMAND: u16 = 0x20;
    /// Master PIC data port
    pub const MASTER_DATA: u16 = 0x21;
    /// Slave PIC command port
    pub const SLAVE_COMMAND: u16 = 0xA0;
    /// Slave PIC data port
    pub const SLAVE_DATA: u16 = 0xA1;
}

/// Disable legacy 8259A PIC
///
/// # Safety
/// Must only be called during early boot before enabling APIC
pub unsafe fn disable_pic() {
    // Mask all IRQs on both PICs
    super::outb(pic::MASTER_DATA, 0xFF);
    super::outb(pic::SLAVE_DATA, 0xFF);
}

/// Remap PIC to vectors 0x20-0x2F and disable
///
/// # Safety
/// Must only be called during early boot
pub unsafe fn remap_and_disable_pic() {
    // Save masks
    let master_mask = super::inb(pic::MASTER_DATA);
    let slave_mask = super::inb(pic::SLAVE_DATA);

    // ICW1: Initialize + ICW4 needed
    super::outb(pic::MASTER_COMMAND, 0x11);
    super::io_wait();
    super::outb(pic::SLAVE_COMMAND, 0x11);
    super::io_wait();

    // ICW2: Vector offset
    super::outb(pic::MASTER_DATA, 0x20); // IRQ 0-7 -> vectors 0x20-0x27
    super::io_wait();
    super::outb(pic::SLAVE_DATA, 0x28); // IRQ 8-15 -> vectors 0x28-0x2F
    super::io_wait();

    // ICW3: Master/slave wiring
    super::outb(pic::MASTER_DATA, 0x04); // Slave at IRQ2
    super::io_wait();
    super::outb(pic::SLAVE_DATA, 0x02); // Slave ID 2
    super::io_wait();

    // ICW4: 8086 mode
    super::outb(pic::MASTER_DATA, 0x01);
    super::io_wait();
    super::outb(pic::SLAVE_DATA, 0x01);
    super::io_wait();

    // Mask all IRQs (disable PIC)
    super::outb(pic::MASTER_DATA, 0xFF);
    super::outb(pic::SLAVE_DATA, 0xFF);
}

// =============================================================================
// AP (APPLICATION PROCESSOR) STARTUP
// =============================================================================

/// AP startup status
static AP_READY: AtomicU32 = AtomicU32::new(0);

/// AP startup trampoline address (must be in low memory)
pub const AP_TRAMPOLINE_ADDR: PhysicalAddress = PhysicalAddress(0x8000);

/// Get number of APs ready
pub fn ap_ready_count() -> u32 {
    AP_READY.load(Ordering::Acquire)
}

/// Signal AP ready
pub fn ap_signal_ready() {
    AP_READY.fetch_add(1, Ordering::AcqRel);
}

/// Reset AP ready counter
pub fn ap_reset_ready() {
    AP_READY.store(0, Ordering::Release);
}

/// AP startup configuration
#[derive(Debug, Clone)]
pub struct ApStartupConfig {
    /// Trampoline physical address
    pub trampoline_addr: PhysicalAddress,
    /// Stack top for AP
    pub stack_top: VirtualAddress,
    /// Entry point for AP
    pub entry_point: VirtualAddress,
    /// Page table physical address
    pub page_table: PhysicalAddress,
    /// GDT physical address
    pub gdt: PhysicalAddress,
}

/// Start an Application Processor
///
/// Returns true if AP responded within timeout
pub fn start_ap(
    apic: &LocalApic,
    ap_id: u32,
    config: &ApStartupConfig,
    timeout_loops: u32,
) -> bool {
    let initial_count = ap_ready_count();

    // Send INIT IPI
    apic.send_init(ap_id);
    apic.wait_for_ipi_delivery();

    // Wait 10ms (approximated by loops)
    for _ in 0..(timeout_loops / 10) {
        core::hint::spin_loop();
    }

    // Send de-assert INIT
    apic.send_ipi(
        ap_id,
        0,
        icr::DELIVERY_INIT | icr::LEVEL_DEASSERT | icr::TRIGGER_LEVEL,
    );
    apic.wait_for_ipi_delivery();

    // Send SIPI (twice as per specification)
    let sipi_vector = (config.trampoline_addr >> 12) as u8;

    for _ in 0..2 {
        apic.send_sipi(ap_id, sipi_vector);
        apic.wait_for_ipi_delivery();

        // Wait for AP to signal ready
        for _ in 0..timeout_loops {
            if ap_ready_count() > initial_count {
                return true;
            }
            core::hint::spin_loop();
        }
    }

    false
}

/// Start all APs
pub fn start_all_aps(apic: &LocalApic, config: &ApStartupConfig, ap_ids: &[u32]) -> usize {
    ap_reset_ready();

    let mut started = 0;
    for &ap_id in ap_ids {
        if ap_id == apic.id() {
            continue; // Skip BSP
        }

        if start_ap(apic, ap_id, config, 100_000) {
            started += 1;
        }
    }

    started
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_apic_config() {
        let mut config = ApicConfig::new();
        assert_eq!(config.cpu_count, 1);

        config.add_cpu(1);
        config.add_cpu(2);
        assert_eq!(config.cpu_count, 3);
        assert_eq!(config.cpu_ids[1], 1);
        assert_eq!(config.cpu_ids[2], 2);

        config.add_io_apic(0, DEFAULT_IOAPIC_BASE, 0);
        assert_eq!(config.io_apic_count, 1);
    }

    #[test]
    fn test_redirection_flags() {
        let entry: u64 = 0x40 | redir::DELIVERY_FIXED | redir::POLARITY_HIGH | redir::TRIGGER_EDGE;
        assert_eq!(entry & 0xFF, 0x40);
        assert_eq!(entry & redir::MASKED, 0);
    }
}
