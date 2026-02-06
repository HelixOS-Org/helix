//! # 8259 Programmable Interrupt Controller (PIC)
//!
//! The legacy dual PIC system (master + slave) for handling hardware interrupts.
//! IRQs 0-7 are on the master, IRQs 8-15 are on the slave.
//!
//! ## Remapping
//!
//! The PICs are remapped to vectors 0x20-0x2F to avoid conflict with CPU exceptions.

use core::sync::atomic::{AtomicBool, Ordering};

/// PIC I/O ports
mod ports {
    /// Master PIC command port
    pub const MASTER_CMD: u16 = 0x20;
    /// Master PIC data port
    pub const MASTER_DATA: u16 = 0x21;
    /// Slave PIC command port
    pub const SLAVE_CMD: u16 = 0xA0;
    /// Slave PIC data port
    pub const SLAVE_DATA: u16 = 0xA1;
}

/// PIC commands
mod cmd {
    /// End of Interrupt command byte
    pub const EOI: u8 = 0x20;
    /// ICW1: Initialization command
    pub const ICW1_INIT: u8 = 0x10;
    /// ICW1: ICW4 will be sent
    pub const ICW1_ICW4: u8 = 0x01;
    /// ICW4: 8086/88 mode
    pub const ICW4_8086: u8 = 0x01;
}

/// Vector offset for remapped IRQs
pub const IRQ_OFFSET: u8 = 32;

/// Is the PIC initialized?
static PIC_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// IRQ numbers for the 8259 PIC
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Irq {
    /// IRQ0: Programmable Interval Timer (PIT)
    Timer        = 0,
    /// IRQ1: PS/2 Keyboard
    Keyboard     = 1,
    /// IRQ2: Cascade (used internally for slave PIC connection)
    Cascade      = 2,
    /// IRQ3: COM2 serial port
    Com2         = 3,
    /// IRQ4: COM1 serial port
    Com1         = 4,
    /// IRQ5: LPT2 parallel port
    Lpt2         = 5,
    /// IRQ6: Floppy disk controller
    Floppy       = 6,
    /// IRQ7: LPT1 parallel port (or spurious interrupt)
    Lpt1         = 7,
    /// IRQ8: Real-Time Clock (RTC)
    RtcClock     = 8,
    /// IRQ9: ACPI/legacy free (redirected from IRQ2)
    Acpi         = 9,
    /// IRQ10: Available for peripherals
    Available1   = 10,
    /// IRQ11: Available for peripherals
    Available2   = 11,
    /// IRQ12: PS/2 Mouse
    Mouse        = 12,
    /// IRQ13: FPU/Coprocessor
    Coprocessor  = 13,
    /// IRQ14: Primary ATA hard disk
    PrimaryAta   = 14,
    /// IRQ15: Secondary ATA hard disk
    SecondaryAta = 15,
}

impl Irq {
    /// Get the interrupt vector for this IRQ
    pub const fn vector(self) -> u8 {
        IRQ_OFFSET + self as u8
    }

    /// Check if this is a slave IRQ (8-15)
    pub const fn is_slave(self) -> bool {
        (self as u8) >= 8
    }
}

/// Initialize and remap the PICs
///
/// Remaps IRQs 0-15 to vectors 0x20-0x2F
///
/// # Safety
/// Must be called only once during early boot.
pub unsafe fn init() {
    // Save current masks
    let master_mask = unsafe { inb(ports::MASTER_DATA) };
    let slave_mask = unsafe { inb(ports::SLAVE_DATA) };

    unsafe {
        // Start initialization sequence (ICW1)
        outb(ports::MASTER_CMD, cmd::ICW1_INIT | cmd::ICW1_ICW4);
        io_wait();
        outb(ports::SLAVE_CMD, cmd::ICW1_INIT | cmd::ICW1_ICW4);
        io_wait();

        // ICW2: Set vector offsets
        outb(ports::MASTER_DATA, IRQ_OFFSET); // Master: IRQ 0-7 -> 0x20-0x27
        io_wait();
        outb(ports::SLAVE_DATA, IRQ_OFFSET + 8); // Slave: IRQ 8-15 -> 0x28-0x2F
        io_wait();

        // ICW3: Configure cascading
        outb(ports::MASTER_DATA, 0x04); // Master: slave on IRQ2
        io_wait();
        outb(ports::SLAVE_DATA, 0x02); // Slave: cascade identity
        io_wait();

        // ICW4: Set mode
        outb(ports::MASTER_DATA, cmd::ICW4_8086);
        io_wait();
        outb(ports::SLAVE_DATA, cmd::ICW4_8086);
        io_wait();

        // Restore masks (all interrupts masked initially)
        outb(ports::MASTER_DATA, master_mask);
        outb(ports::SLAVE_DATA, slave_mask);
    }

    PIC_INITIALIZED.store(true, Ordering::Release);
    log::info!(
        "PIC remapped: IRQs 0-15 -> vectors 0x{:02X}-0x{:02X}",
        IRQ_OFFSET,
        IRQ_OFFSET + 15
    );
}

/// Enable a specific IRQ
pub fn enable_irq(irq: Irq) {
    let irq_num = irq as u8;

    unsafe {
        if irq_num < 8 {
            // Master PIC
            let mask = inb(ports::MASTER_DATA);
            outb(ports::MASTER_DATA, mask & !(1 << irq_num));
        } else {
            // Slave PIC - also need to enable cascade on master
            let mask = inb(ports::SLAVE_DATA);
            outb(ports::SLAVE_DATA, mask & !(1 << (irq_num - 8)));
            // Ensure IRQ2 (cascade) is enabled on master
            let master_mask = inb(ports::MASTER_DATA);
            outb(ports::MASTER_DATA, master_mask & !(1 << 2));
        }
    }

    log::debug!("IRQ{} enabled (vector 0x{:02X})", irq_num, irq.vector());
}

/// Disable a specific IRQ
pub fn disable_irq(irq: Irq) {
    let irq_num = irq as u8;

    unsafe {
        if irq_num < 8 {
            let mask = inb(ports::MASTER_DATA);
            outb(ports::MASTER_DATA, mask | (1 << irq_num));
        } else {
            let mask = inb(ports::SLAVE_DATA);
            outb(ports::SLAVE_DATA, mask | (1 << (irq_num - 8)));
        }
    }
}

/// Send End of Interrupt signal
///
/// Must be called at the end of every IRQ handler.
pub fn end_of_interrupt(irq: Irq) {
    unsafe {
        if irq.is_slave() {
            // Send EOI to both slave and master
            outb(ports::SLAVE_CMD, cmd::EOI);
        }
        // Always send EOI to master
        outb(ports::MASTER_CMD, cmd::EOI);
    }
}

/// Mask all IRQs (disable all interrupts)
pub fn disable_all() {
    unsafe {
        outb(ports::MASTER_DATA, 0xFF);
        outb(ports::SLAVE_DATA, 0xFF);
    }
}

/// Enable all IRQs (unmask all interrupts)
pub fn enable_all() {
    unsafe {
        outb(ports::MASTER_DATA, 0x00);
        outb(ports::SLAVE_DATA, 0x00);
    }
}

/// Check if PIC is initialized
pub fn is_initialized() -> bool {
    PIC_INITIALIZED.load(Ordering::Acquire)
}

// =============================================================================
// I/O Helpers
// =============================================================================

#[inline]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Small I/O delay for PIC operations
#[inline]
fn io_wait() {
    unsafe {
        // Write to unused port 0x80 for delay
        core::arch::asm!(
            "out 0x80, al",
            in("al") 0u8,
            options(nomem, nostack, preserves_flags)
        );
    }
}
