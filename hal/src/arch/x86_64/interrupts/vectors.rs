//! # Interrupt Vector Definitions
//!
//! This module defines all interrupt vectors used in x86_64 systems,
//! including CPU exceptions, IRQs, system calls, and device interrupts.
//!
//! ## Vector Layout
//!
//! ```text
//! Vector Range   Purpose                  Notes
//! ─────────────────────────────────────────────────────────────
//! 0x00-0x1F      CPU Exceptions          Intel-reserved
//! 0x20-0x2F      Legacy PIC IRQs         Remapped from 0x00-0x0F
//! 0x30-0x3F      System Vectors          IPIs, syscall fallback
//! 0x40-0xEF      Device Interrupts       APIC, MSI, MSI-X
//! 0xF0-0xFE      Reserved                Future use
//! 0xFF           Spurious Interrupt      APIC spurious
//! ```

use core::fmt;

// =============================================================================
// CPU Exception Vectors (0x00-0x1F)
// =============================================================================

/// CPU Exception Vectors
///
/// These are hardware-defined exception numbers that cannot be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionVector {
    /// #DE - Divide Error (fault, no error code)
    DivideError          = 0x00,

    /// #DB - Debug Exception (fault/trap, no error code)
    Debug                = 0x01,

    /// NMI - Non-Maskable Interrupt (interrupt, no error code)
    NonMaskableInterrupt = 0x02,

    /// #BP - Breakpoint (trap, no error code)
    Breakpoint           = 0x03,

    /// #OF - Overflow (trap, no error code)
    Overflow             = 0x04,

    /// #BR - Bound Range Exceeded (fault, no error code)
    BoundRangeExceeded   = 0x05,

    /// #UD - Invalid Opcode (fault, no error code)
    InvalidOpcode        = 0x06,

    /// #NM - Device Not Available (fault, no error code)
    DeviceNotAvailable   = 0x07,

    /// #DF - Double Fault (abort, error code = 0)
    DoubleFault          = 0x08,

    /// Coprocessor Segment Overrun (abort, no error code)
    /// Legacy - not generated on modern CPUs
    CoprocessorSegmentOverrun = 0x09,

    /// #TS - Invalid TSS (fault, error code)
    InvalidTss           = 0x0A,

    /// #NP - Segment Not Present (fault, error code)
    SegmentNotPresent    = 0x0B,

    /// #SS - Stack Segment Fault (fault, error code)
    StackSegmentFault    = 0x0C,

    /// #GP - General Protection Fault (fault, error code)
    GeneralProtection    = 0x0D,

    /// #PF - Page Fault (fault, error code)
    PageFault            = 0x0E,

    /// Reserved (0x0F)
    Reserved0F           = 0x0F,

    /// #MF - x87 FPU Floating-Point Error (fault, no error code)
    X87FloatingPoint     = 0x10,

    /// #AC - Alignment Check (fault, error code = 0)
    AlignmentCheck       = 0x11,

    /// #MC - Machine Check (abort, no error code)
    MachineCheck         = 0x12,

    /// #XM/#XF - SIMD Floating-Point Exception (fault, no error code)
    SimdFloatingPoint    = 0x13,

    /// #VE - Virtualization Exception (fault, no error code)
    VirtualizationException = 0x14,

    /// #CP - Control Protection Exception (fault, error code)
    ControlProtection    = 0x15,

    // 0x16-0x1B: Reserved
    Reserved16           = 0x16,
    Reserved17           = 0x17,
    Reserved18           = 0x18,
    Reserved19           = 0x19,
    Reserved1A           = 0x1A,
    Reserved1B           = 0x1B,

    /// #HV - Hypervisor Injection Exception (fault, no error code)
    HypervisorInjection  = 0x1C,

    /// #VC - VMM Communication Exception (fault, error code)
    VmmCommunication     = 0x1D,

    /// #SX - Security Exception (fault, error code)
    SecurityException    = 0x1E,

    /// Reserved (0x1F)
    Reserved1F           = 0x1F,
}

impl ExceptionVector {
    /// Check if this exception pushes an error code
    #[inline]
    pub const fn has_error_code(self) -> bool {
        matches!(
            self,
            ExceptionVector::DoubleFault
                | ExceptionVector::InvalidTss
                | ExceptionVector::SegmentNotPresent
                | ExceptionVector::StackSegmentFault
                | ExceptionVector::GeneralProtection
                | ExceptionVector::PageFault
                | ExceptionVector::AlignmentCheck
                | ExceptionVector::ControlProtection
                | ExceptionVector::VmmCommunication
                | ExceptionVector::SecurityException
        )
    }

    /// Check if this is a fault (can be resumed)
    #[inline]
    pub const fn is_fault(self) -> bool {
        matches!(
            self,
            ExceptionVector::DivideError
                | ExceptionVector::Debug
                | ExceptionVector::BoundRangeExceeded
                | ExceptionVector::InvalidOpcode
                | ExceptionVector::DeviceNotAvailable
                | ExceptionVector::InvalidTss
                | ExceptionVector::SegmentNotPresent
                | ExceptionVector::StackSegmentFault
                | ExceptionVector::GeneralProtection
                | ExceptionVector::PageFault
                | ExceptionVector::X87FloatingPoint
                | ExceptionVector::AlignmentCheck
                | ExceptionVector::SimdFloatingPoint
                | ExceptionVector::VirtualizationException
                | ExceptionVector::ControlProtection
                | ExceptionVector::HypervisorInjection
                | ExceptionVector::VmmCommunication
                | ExceptionVector::SecurityException
        )
    }

    /// Check if this is a trap (instruction completed)
    #[inline]
    pub const fn is_trap(self) -> bool {
        matches!(
            self,
            ExceptionVector::Debug | ExceptionVector::Breakpoint | ExceptionVector::Overflow
        )
    }

    /// Check if this is an abort (cannot continue)
    #[inline]
    pub const fn is_abort(self) -> bool {
        matches!(
            self,
            ExceptionVector::DoubleFault
                | ExceptionVector::MachineCheck
                | ExceptionVector::CoprocessorSegmentOverrun
        )
    }

    /// Get the mnemonic for this exception
    pub const fn mnemonic(self) -> &'static str {
        match self {
            ExceptionVector::DivideError => "#DE",
            ExceptionVector::Debug => "#DB",
            ExceptionVector::NonMaskableInterrupt => "NMI",
            ExceptionVector::Breakpoint => "#BP",
            ExceptionVector::Overflow => "#OF",
            ExceptionVector::BoundRangeExceeded => "#BR",
            ExceptionVector::InvalidOpcode => "#UD",
            ExceptionVector::DeviceNotAvailable => "#NM",
            ExceptionVector::DoubleFault => "#DF",
            ExceptionVector::CoprocessorSegmentOverrun => "---",
            ExceptionVector::InvalidTss => "#TS",
            ExceptionVector::SegmentNotPresent => "#NP",
            ExceptionVector::StackSegmentFault => "#SS",
            ExceptionVector::GeneralProtection => "#GP",
            ExceptionVector::PageFault => "#PF",
            ExceptionVector::X87FloatingPoint => "#MF",
            ExceptionVector::AlignmentCheck => "#AC",
            ExceptionVector::MachineCheck => "#MC",
            ExceptionVector::SimdFloatingPoint => "#XM",
            ExceptionVector::VirtualizationException => "#VE",
            ExceptionVector::ControlProtection => "#CP",
            ExceptionVector::HypervisorInjection => "#HV",
            ExceptionVector::VmmCommunication => "#VC",
            ExceptionVector::SecurityException => "#SX",
            _ => "---",
        }
    }

    /// Get a human-readable name for this exception
    pub const fn name(self) -> &'static str {
        match self {
            ExceptionVector::DivideError => "Divide Error",
            ExceptionVector::Debug => "Debug Exception",
            ExceptionVector::NonMaskableInterrupt => "Non-Maskable Interrupt",
            ExceptionVector::Breakpoint => "Breakpoint",
            ExceptionVector::Overflow => "Overflow",
            ExceptionVector::BoundRangeExceeded => "Bound Range Exceeded",
            ExceptionVector::InvalidOpcode => "Invalid Opcode",
            ExceptionVector::DeviceNotAvailable => "Device Not Available",
            ExceptionVector::DoubleFault => "Double Fault",
            ExceptionVector::CoprocessorSegmentOverrun => "Coprocessor Segment Overrun",
            ExceptionVector::InvalidTss => "Invalid TSS",
            ExceptionVector::SegmentNotPresent => "Segment Not Present",
            ExceptionVector::StackSegmentFault => "Stack Segment Fault",
            ExceptionVector::GeneralProtection => "General Protection Fault",
            ExceptionVector::PageFault => "Page Fault",
            ExceptionVector::X87FloatingPoint => "x87 FPU Error",
            ExceptionVector::AlignmentCheck => "Alignment Check",
            ExceptionVector::MachineCheck => "Machine Check",
            ExceptionVector::SimdFloatingPoint => "SIMD Floating-Point",
            ExceptionVector::VirtualizationException => "Virtualization Exception",
            ExceptionVector::ControlProtection => "Control Protection",
            ExceptionVector::HypervisorInjection => "Hypervisor Injection",
            ExceptionVector::VmmCommunication => "VMM Communication",
            ExceptionVector::SecurityException => "Security Exception",
            _ => "Reserved",
        }
    }

    /// Convert from vector number
    pub const fn from_vector(vector: u8) -> Option<Self> {
        if vector <= 0x1F {
            // Safety: all values 0x00-0x1F are valid enum variants
            Some(unsafe { core::mem::transmute(vector) })
        } else {
            None
        }
    }

    /// Get recommended IST for this exception (0 = no IST)
    pub const fn recommended_ist(self) -> u8 {
        match self {
            ExceptionVector::DoubleFault => 1, // IST1 - dedicated stack
            ExceptionVector::NonMaskableInterrupt => 2, // IST2
            ExceptionVector::MachineCheck => 3, // IST3
            ExceptionVector::Debug => 4,       // IST4
            _ => 0,                            // Use current stack
        }
    }
}

// =============================================================================
// Legacy PIC IRQ Vectors (0x20-0x2F)
// =============================================================================

/// Legacy PIC IRQ Vectors
///
/// When using the 8259 PIC, IRQs 0-15 are remapped to vectors 0x20-0x2F
/// to avoid conflicting with CPU exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IrqVector {
    /// IRQ 0 - PIT Timer
    Timer        = 0x20,

    /// IRQ 1 - Keyboard
    Keyboard     = 0x21,

    /// IRQ 2 - Cascade (PIC2)
    Cascade      = 0x22,

    /// IRQ 3 - COM2
    Com2         = 0x23,

    /// IRQ 4 - COM1
    Com1         = 0x24,

    /// IRQ 5 - LPT2 / Sound Card
    Lpt2         = 0x25,

    /// IRQ 6 - Floppy Disk
    Floppy       = 0x26,

    /// IRQ 7 - LPT1 / Spurious
    Lpt1         = 0x27,

    /// IRQ 8 - RTC
    Rtc          = 0x28,

    /// IRQ 9 - ACPI / Legacy redirect
    Acpi         = 0x29,

    /// IRQ 10 - Free
    Free10       = 0x2A,

    /// IRQ 11 - Free
    Free11       = 0x2B,

    /// IRQ 12 - PS/2 Mouse
    Mouse        = 0x2C,

    /// IRQ 13 - FPU / Coprocessor
    Fpu          = 0x2D,

    /// IRQ 14 - Primary ATA
    PrimaryAta   = 0x2E,

    /// IRQ 15 - Secondary ATA
    SecondaryAta = 0x2F,
}

impl IrqVector {
    /// Get the IRQ number (0-15)
    #[inline]
    pub const fn irq_number(self) -> u8 {
        self as u8 - 0x20
    }

    /// Convert from IRQ number
    #[inline]
    pub const fn from_irq(irq: u8) -> Option<Self> {
        if irq < 16 {
            Some(unsafe { core::mem::transmute(0x20 + irq) })
        } else {
            None
        }
    }

    /// Get IRQ name
    pub const fn name(self) -> &'static str {
        match self {
            IrqVector::Timer => "Timer",
            IrqVector::Keyboard => "Keyboard",
            IrqVector::Cascade => "Cascade",
            IrqVector::Com2 => "COM2",
            IrqVector::Com1 => "COM1",
            IrqVector::Lpt2 => "LPT2",
            IrqVector::Floppy => "Floppy",
            IrqVector::Lpt1 => "LPT1",
            IrqVector::Rtc => "RTC",
            IrqVector::Acpi => "ACPI",
            IrqVector::Free10 => "Free",
            IrqVector::Free11 => "Free",
            IrqVector::Mouse => "Mouse",
            IrqVector::Fpu => "FPU",
            IrqVector::PrimaryAta => "Primary ATA",
            IrqVector::SecondaryAta => "Secondary ATA",
        }
    }
}

// =============================================================================
// System Vectors (0x30-0x3F)
// =============================================================================

/// System Vectors
///
/// These vectors are used for IPIs, syscalls, and other system purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SystemVector {
    /// IPI: Reschedule request
    IpiReschedule   = 0x30,

    /// IPI: TLB shootdown
    IpiTlbShootdown = 0x31,

    /// IPI: Halt processor
    IpiHalt         = 0x32,

    /// IPI: Call function
    IpiCallFunction = 0x33,

    /// IPI: Generic notification
    IpiNotify       = 0x34,

    /// IPI: Profiling
    IpiProfile      = 0x35,

    // 0x36-0x7F: Reserved for future system use
    /// System call (INT 0x80 fallback)
    Syscall         = 0x80,
}

impl SystemVector {
    /// Check if this is an IPI vector
    #[inline]
    pub const fn is_ipi(self) -> bool {
        (self as u8) >= 0x30 && (self as u8) <= 0x3F
    }
}

// =============================================================================
// Device Vectors (0x40-0xEF)
// =============================================================================

/// Common Device Vectors
///
/// These are commonly used APIC vector assignments.
pub mod device {
    /// APIC Timer
    pub const APIC_TIMER: u8 = 0x40;

    /// Thermal Sensor
    pub const THERMAL: u8 = 0x41;

    /// Performance Monitoring
    pub const PERF_COUNTER: u8 = 0x42;

    /// CMCI (Corrected Machine Check Interrupt)
    pub const CMCI: u8 = 0x43;

    /// First MSI vector
    pub const MSI_BASE: u8 = 0x50;

    /// Last MSI vector
    pub const MSI_END: u8 = 0xDF;

    /// Number of available MSI vectors
    pub const MSI_COUNT: u8 = MSI_END - MSI_BASE + 1;
}

/// Reserved Vectors
pub mod reserved {
    /// APIC Error
    pub const APIC_ERROR: u8 = 0xFE;

    /// Spurious Interrupt
    pub const SPURIOUS: u8 = 0xFF;
}

// =============================================================================
// Generic Vector Type
// =============================================================================

/// Generic Interrupt Vector
///
/// This enum can represent any type of interrupt vector.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vector {
    /// CPU Exception (0x00-0x1F)
    Exception(ExceptionVector),

    /// Legacy PIC IRQ (0x20-0x2F)
    Irq(IrqVector),

    /// System Vector (IPIs, syscall)
    System(SystemVector),

    /// Device Interrupt (APIC, MSI)
    Device(u8),

    /// Spurious Interrupt
    Spurious,
}

impl Vector {
    /// Get the raw vector number
    #[inline]
    pub const fn as_u8(&self) -> u8 {
        match self {
            Vector::Exception(e) => *e as u8,
            Vector::Irq(i) => *i as u8,
            Vector::System(s) => *s as u8,
            Vector::Device(d) => *d,
            Vector::Spurious => 0xFF,
        }
    }

    /// Create from raw vector number
    pub const fn from_u8(vector: u8) -> Self {
        match vector {
            0x00..=0x1F => {
                if let Some(e) = ExceptionVector::from_vector(vector) {
                    Vector::Exception(e)
                } else {
                    Vector::Device(vector)
                }
            },
            0x20..=0x2F => {
                if let Some(i) = IrqVector::from_irq(vector - 0x20) {
                    Vector::Irq(i)
                } else {
                    Vector::Device(vector)
                }
            },
            0x30 => Vector::System(SystemVector::IpiReschedule),
            0x31 => Vector::System(SystemVector::IpiTlbShootdown),
            0x32 => Vector::System(SystemVector::IpiHalt),
            0x33 => Vector::System(SystemVector::IpiCallFunction),
            0x80 => Vector::System(SystemVector::Syscall),
            0xFF => Vector::Spurious,
            _ => Vector::Device(vector),
        }
    }
}

impl fmt::Debug for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Vector::Exception(e) => write!(f, "Exception({:?} [{:#04x}])", e, *e as u8),
            Vector::Irq(i) => write!(f, "IRQ({:?} [{:#04x}])", i, *i as u8),
            Vector::System(s) => write!(f, "System({:?} [{:#04x}])", s, *s as u8),
            Vector::Device(d) => write!(f, "Device({:#04x})", d),
            Vector::Spurious => write!(f, "Spurious"),
        }
    }
}

impl From<u8> for Vector {
    fn from(v: u8) -> Self {
        Vector::from_u8(v)
    }
}

impl From<Vector> for u8 {
    fn from(v: Vector) -> Self {
        v.as_u8()
    }
}

impl From<ExceptionVector> for Vector {
    fn from(e: ExceptionVector) -> Self {
        Vector::Exception(e)
    }
}

impl From<IrqVector> for Vector {
    fn from(i: IrqVector) -> Self {
        Vector::Irq(i)
    }
}

impl From<SystemVector> for Vector {
    fn from(s: SystemVector) -> Self {
        Vector::System(s)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_error_codes() {
        assert!(!ExceptionVector::DivideError.has_error_code());
        assert!(ExceptionVector::PageFault.has_error_code());
        assert!(ExceptionVector::GeneralProtection.has_error_code());
        assert!(ExceptionVector::DoubleFault.has_error_code());
    }

    #[test]
    fn test_irq_numbers() {
        assert_eq!(IrqVector::Timer.irq_number(), 0);
        assert_eq!(IrqVector::Keyboard.irq_number(), 1);
        assert_eq!(IrqVector::SecondaryAta.irq_number(), 15);
    }

    #[test]
    fn test_vector_conversion() {
        assert_eq!(Vector::from_u8(0x00).as_u8(), 0x00);
        assert_eq!(Vector::from_u8(0x20).as_u8(), 0x20);
        assert_eq!(Vector::from_u8(0xFF).as_u8(), 0xFF);
    }
}
