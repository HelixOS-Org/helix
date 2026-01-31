//! # Interrupt Subsystem
//!
//! Interrupt descriptor table (IDT), interrupt controllers, and exception handling.
//! Supports x86_64 (APIC/IOAPIC), AArch64 (GIC), and RISC-V (PLIC/CLINT).

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// INTERRUPT HANDLER TYPES
// =============================================================================

/// Interrupt handler function type
pub type InterruptHandler = fn(vector: u8, context: &InterruptContext);

/// Interrupt context (saved registers)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptContext {
    // x86_64 registers
    #[cfg(target_arch = "x86_64")]
    pub rax: u64,
    #[cfg(target_arch = "x86_64")]
    pub rbx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rcx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rdx: u64,
    #[cfg(target_arch = "x86_64")]
    pub rsi: u64,
    #[cfg(target_arch = "x86_64")]
    pub rdi: u64,
    #[cfg(target_arch = "x86_64")]
    pub rbp: u64,
    #[cfg(target_arch = "x86_64")]
    pub r8: u64,
    #[cfg(target_arch = "x86_64")]
    pub r9: u64,
    #[cfg(target_arch = "x86_64")]
    pub r10: u64,
    #[cfg(target_arch = "x86_64")]
    pub r11: u64,
    #[cfg(target_arch = "x86_64")]
    pub r12: u64,
    #[cfg(target_arch = "x86_64")]
    pub r13: u64,
    #[cfg(target_arch = "x86_64")]
    pub r14: u64,
    #[cfg(target_arch = "x86_64")]
    pub r15: u64,
    #[cfg(target_arch = "x86_64")]
    pub rip: u64,
    #[cfg(target_arch = "x86_64")]
    pub cs: u64,
    #[cfg(target_arch = "x86_64")]
    pub rflags: u64,
    #[cfg(target_arch = "x86_64")]
    pub rsp: u64,
    #[cfg(target_arch = "x86_64")]
    pub ss: u64,

    // Common fields
    pub vector: u8,
    pub error_code: u64,
}

impl Default for InterruptContext {
    fn default() -> Self {
        Self {
            #[cfg(target_arch = "x86_64")]
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            #[cfg(target_arch = "x86_64")]
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            #[cfg(target_arch = "x86_64")]
            rip: 0,
            cs: 0,
            rflags: 0,
            rsp: 0,
            ss: 0,
            vector: 0,
            error_code: 0,
        }
    }
}

// =============================================================================
// INTERRUPT VECTOR MANAGEMENT
// =============================================================================

/// Maximum interrupt vectors
pub const MAX_VECTORS: usize = 256;

/// Interrupt vector entry
pub struct VectorEntry {
    pub handler: Option<InterruptHandler>,
    pub name: &'static str,
    pub irq_line: Option<u8>,
    pub enabled: AtomicBool,
    pub count: AtomicU64,
}

impl Default for VectorEntry {
    fn default() -> Self {
        Self {
            handler: None,
            name: "",
            irq_line: None,
            enabled: AtomicBool::new(false),
            count: AtomicU64::new(0),
        }
    }
}

/// Exception vectors for x86_64
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    DivideError         = 0,
    Debug               = 1,
    Nmi                 = 2,
    Breakpoint          = 3,
    Overflow            = 4,
    BoundRange          = 5,
    InvalidOpcode       = 6,
    DeviceNotAvailable  = 7,
    DoubleFault         = 8,
    CoprocessorOverrun  = 9,
    InvalidTss          = 10,
    SegmentNotPresent   = 11,
    StackFault          = 12,
    GeneralProtection   = 13,
    PageFault           = 14,
    // 15 reserved
    X87Fpu              = 16,
    AlignmentCheck      = 17,
    MachineCheck        = 18,
    SimdFp              = 19,
    Virtualization      = 20,
    ControlProtection   = 21,
    // 22-27 reserved
    HypervisorInjection = 28,
    VmmCommunication    = 29,
    Security            = 30,
    // 31 reserved
}

impl Exception {
    /// Get exception name
    pub fn name(&self) -> &'static str {
        match self {
            Self::DivideError => "Divide Error",
            Self::Debug => "Debug",
            Self::Nmi => "NMI",
            Self::Breakpoint => "Breakpoint",
            Self::Overflow => "Overflow",
            Self::BoundRange => "Bound Range",
            Self::InvalidOpcode => "Invalid Opcode",
            Self::DeviceNotAvailable => "Device Not Available",
            Self::DoubleFault => "Double Fault",
            Self::CoprocessorOverrun => "Coprocessor Overrun",
            Self::InvalidTss => "Invalid TSS",
            Self::SegmentNotPresent => "Segment Not Present",
            Self::StackFault => "Stack Fault",
            Self::GeneralProtection => "General Protection",
            Self::PageFault => "Page Fault",
            Self::X87Fpu => "x87 FPU",
            Self::AlignmentCheck => "Alignment Check",
            Self::MachineCheck => "Machine Check",
            Self::SimdFp => "SIMD Floating-Point",
            Self::Virtualization => "Virtualization",
            Self::ControlProtection => "Control Protection",
            Self::HypervisorInjection => "Hypervisor Injection",
            Self::VmmCommunication => "VMM Communication",
            Self::Security => "Security",
        }
    }

    /// Does this exception push an error code?
    pub fn has_error_code(&self) -> bool {
        matches!(
            self,
            Self::DoubleFault
                | Self::InvalidTss
                | Self::SegmentNotPresent
                | Self::StackFault
                | Self::GeneralProtection
                | Self::PageFault
                | Self::AlignmentCheck
                | Self::Security
        )
    }
}

// =============================================================================
// IDT STRUCTURES (x86_64)
// =============================================================================

/// IDT entry for x86_64
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    /// Create empty IDT entry
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create IDT entry for interrupt handler
    pub fn new(handler: u64, selector: u16, ist: u8, gate_type: GateType) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist,
            type_attr: 0x80 | (gate_type as u8), // Present + type
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }
}

/// Gate types for x86_64 IDT
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum GateType {
    Interrupt = 0xE, // 64-bit interrupt gate
    Trap      = 0xF, // 64-bit trap gate
}

/// IDT descriptor (IDTR)
#[repr(C, packed)]
pub struct IdtDescriptor {
    limit: u16,
    base: u64,
}

// =============================================================================
// INTERRUPT CONTROLLER
// =============================================================================

/// Interrupt controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptController {
    // x86_64
    Pic,    // Legacy 8259 PIC
    Apic,   // Local APIC + I/O APIC
    X2Apic, // x2APIC mode

    // AArch64
    Gic2, // GICv2
    Gic3, // GICv3
    Gic4, // GICv4

    // RISC-V
    Plic,  // Platform-Level Interrupt Controller
    Clint, // Core-Local Interruptor
    Aplic, // Advanced PLIC

    None,
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::None
    }
}

// =============================================================================
// INTERRUPT SUBSYSTEM
// =============================================================================

/// Interrupt subsystem
///
/// Manages interrupt descriptor tables and interrupt controllers.
pub struct InterruptSubsystem {
    info: SubsystemInfo,
    controller: InterruptController,
    vectors: Vec<VectorEntry>,
    idt_base: u64,
    enabled: bool,

    // x86_64 specific
    #[cfg(target_arch = "x86_64")]
    apic_base: u64,
    #[cfg(target_arch = "x86_64")]
    ioapic_base: u64,

    // AArch64 specific
    #[cfg(target_arch = "aarch64")]
    gic_dist_base: u64,
    #[cfg(target_arch = "aarch64")]
    gic_cpu_base: u64,

    // RISC-V specific
    #[cfg(target_arch = "riscv64")]
    plic_base: u64,
}

static INTERRUPT_DEPS: [Dependency; 1] = [Dependency::required("cpu")];

impl InterruptSubsystem {
    /// Create new interrupt subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("interrupts", InitPhase::Core)
                .with_priority(1000)
                .with_description("Interrupt controller and exception handling")
                .with_dependencies(&INTERRUPT_DEPS)
                .provides(PhaseCapabilities::INTERRUPTS)
                .essential(),
            controller: InterruptController::None,
            vectors: Vec::with_capacity(MAX_VECTORS),
            idt_base: 0,
            enabled: false,

            #[cfg(target_arch = "x86_64")]
            apic_base: 0xFEE0_0000, // Default LAPIC base
            #[cfg(target_arch = "x86_64")]
            ioapic_base: 0xFEC0_0000, // Default I/O APIC base

            #[cfg(target_arch = "aarch64")]
            gic_dist_base: 0,
            #[cfg(target_arch = "aarch64")]
            gic_cpu_base: 0,

            #[cfg(target_arch = "riscv64")]
            plic_base: 0,
        }
    }

    /// Get interrupt controller type
    pub fn controller(&self) -> InterruptController {
        self.controller
    }

    /// Are interrupts enabled?
    pub fn interrupts_enabled(&self) -> bool {
        self.enabled
    }

    /// Register interrupt handler
    pub fn register_handler(
        &mut self,
        vector: u8,
        handler: InterruptHandler,
        name: &'static str,
    ) -> InitResult<()> {
        if (vector as usize) >= self.vectors.len() {
            return Err(InitError::new(
                ErrorKind::InvalidArgument,
                "Invalid interrupt vector",
            ));
        }

        let entry = &mut self.vectors[vector as usize];
        if entry.handler.is_some() {
            return Err(InitError::new(
                ErrorKind::AlreadyInitialized,
                "Vector already has handler",
            ));
        }

        entry.handler = Some(handler);
        entry.name = name;
        entry.enabled.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Unregister interrupt handler
    pub fn unregister_handler(&mut self, vector: u8) -> InitResult<()> {
        if (vector as usize) >= self.vectors.len() {
            return Err(InitError::new(
                ErrorKind::InvalidArgument,
                "Invalid interrupt vector",
            ));
        }

        let entry = &mut self.vectors[vector as usize];
        entry.handler = None;
        entry.enabled.store(false, Ordering::SeqCst);

        Ok(())
    }

    /// Get interrupt count for vector
    pub fn get_count(&self, vector: u8) -> u64 {
        if (vector as usize) < self.vectors.len() {
            self.vectors[vector as usize].count.load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Enable interrupts
    pub fn enable(&mut self) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("sti", options(nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("msr daifclr, #0xf", options(nostack));
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!(
                "csrsi sstatus, 0x2", // Set SIE bit
                options(nostack)
            );
        }

        self.enabled = true;
    }

    /// Disable interrupts
    pub fn disable(&mut self) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("cli", options(nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("msr daifset, #0xf", options(nostack));
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!(
                "csrci sstatus, 0x2", // Clear SIE bit
                options(nostack)
            );
        }

        self.enabled = false;
    }

    // =========================================================================
    // x86_64 IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "x86_64")]
    fn init_x86(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Initialize vectors
        for _ in 0..MAX_VECTORS {
            self.vectors.push(VectorEntry::default());
        }

        // Set up exception names
        self.setup_exception_names();

        // Detect interrupt controller
        self.detect_apic(ctx)?;

        // Initialize IDT
        self.init_idt(ctx)?;

        // Initialize APIC
        if self.controller == InterruptController::Apic
            || self.controller == InterruptController::X2Apic
        {
            self.init_apic(ctx)?;
        } else {
            // Fall back to PIC
            self.init_pic(ctx)?;
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn setup_exception_names(&mut self) {
        let exceptions = [
            (0, "Divide Error"),
            (1, "Debug"),
            (2, "NMI"),
            (3, "Breakpoint"),
            (4, "Overflow"),
            (5, "Bound Range"),
            (6, "Invalid Opcode"),
            (7, "Device Not Available"),
            (8, "Double Fault"),
            (9, "Coprocessor Overrun"),
            (10, "Invalid TSS"),
            (11, "Segment Not Present"),
            (12, "Stack Fault"),
            (13, "General Protection"),
            (14, "Page Fault"),
            (16, "x87 FPU"),
            (17, "Alignment Check"),
            (18, "Machine Check"),
            (19, "SIMD FP"),
        ];

        for (vec, name) in exceptions {
            if (vec as usize) < self.vectors.len() {
                self.vectors[vec as usize].name = name;
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn detect_apic(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Check CPUID for APIC
        let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
        let has_apic = (cpuid.edx & (1 << 9)) != 0;
        let has_x2apic = (cpuid.ecx & (1 << 21)) != 0;

        if has_x2apic {
            self.controller = InterruptController::X2Apic;
            ctx.debug("Detected x2APIC");
        } else if has_apic {
            self.controller = InterruptController::Apic;
            ctx.debug("Detected Local APIC");
        } else {
            self.controller = InterruptController::Pic;
            ctx.debug("Falling back to 8259 PIC");
        }

        // Get APIC base from MSR
        if has_apic {
            let apic_base: u64;
            unsafe {
                let lo: u32;
                let hi: u32;
                core::arch::asm!(
                    "rdmsr",
                    in("ecx") 0x1Bu32, // IA32_APIC_BASE MSR
                    out("eax") lo,
                    out("edx") hi,
                    options(nostack)
                );
                apic_base = ((hi as u64) << 32) | (lo as u64);
            }
            self.apic_base = apic_base & 0xFFFF_F000;
            ctx.debug(alloc::format!("APIC base: 0x{:x}", self.apic_base));
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn init_idt(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // In a real kernel, we'd allocate memory for the IDT
        // and set up all 256 entries with proper handlers

        ctx.debug("Initializing IDT");

        // For now, just note that IDT setup would happen here
        // with proper interrupt stubs in assembly

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn init_apic(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.debug("Initializing Local APIC");

        // Enable APIC via MSR
        unsafe {
            let lo: u32;
            let hi: u32;
            core::arch::asm!(
                "rdmsr",
                in("ecx") 0x1Bu32,
                out("eax") lo,
                out("edx") hi,
                options(nostack)
            );

            // Set enable bit
            let new_lo = lo | (1 << 11);
            core::arch::asm!(
                "wrmsr",
                in("ecx") 0x1Bu32,
                in("eax") new_lo,
                in("edx") hi,
                options(nostack)
            );
        }

        // Enable spurious interrupt vector
        let svr_offset = 0xF0; // Spurious Vector Register
        let svr_ptr = (self.apic_base + svr_offset) as *mut u32;
        unsafe {
            // Set spurious vector to 0xFF and enable APIC
            core::ptr::write_volatile(svr_ptr, 0x1FF);
        }

        ctx.debug("APIC enabled");

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn init_pic(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.debug("Initializing 8259 PIC");

        const PIC1_CMD: u16 = 0x20;
        const PIC1_DATA: u16 = 0x21;
        const PIC2_CMD: u16 = 0xA0;
        const PIC2_DATA: u16 = 0xA1;

        // ICW1: Initialize + ICW4 needed
        Self::outb(PIC1_CMD, 0x11);
        Self::outb(PIC2_CMD, 0x11);

        // ICW2: Vector offset
        Self::outb(PIC1_DATA, 0x20); // IRQ 0-7 -> INT 0x20-0x27
        Self::outb(PIC2_DATA, 0x28); // IRQ 8-15 -> INT 0x28-0x2F

        // ICW3: Cascade
        Self::outb(PIC1_DATA, 0x04); // IRQ2 has slave
        Self::outb(PIC2_DATA, 0x02); // Cascade identity

        // ICW4: 8086 mode
        Self::outb(PIC1_DATA, 0x01);
        Self::outb(PIC2_DATA, 0x01);

        // Mask all interrupts
        Self::outb(PIC1_DATA, 0xFF);
        Self::outb(PIC2_DATA, 0xFF);

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn outb(port: u16, value: u8) {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") port,
                in("al") value,
                options(nostack)
            );
        }
    }

    // =========================================================================
    // AArch64 IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "aarch64")]
    fn init_arm(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        for _ in 0..MAX_VECTORS {
            self.vectors.push(VectorEntry::default());
        }

        // Detect GIC version from DTB or ACPI
        // For now, assume GICv2
        self.controller = InterruptController::Gic2;

        // TODO: Get GIC addresses from device tree
        self.gic_dist_base = 0x0800_0000; // Placeholder
        self.gic_cpu_base = 0x0801_0000;

        ctx.debug(alloc::format!(
            "GIC distributor: 0x{:x}, CPU interface: 0x{:x}",
            self.gic_dist_base,
            self.gic_cpu_base
        ));

        // Initialize GIC
        self.init_gic(ctx)?;

        Ok(())
    }

    #[cfg(target_arch = "aarch64")]
    fn init_gic(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.debug("Initializing GIC");

        // GIC Distributor initialization
        let gicd = self.gic_dist_base as *mut u32;
        unsafe {
            // Disable distributor
            core::ptr::write_volatile(gicd, 0);

            // Enable distributor
            core::ptr::write_volatile(gicd, 1);
        }

        // GIC CPU interface initialization
        let gicc = self.gic_cpu_base as *mut u32;
        unsafe {
            // Set priority mask (all priorities)
            core::ptr::write_volatile(gicc.offset(1), 0xFF);

            // Enable CPU interface
            core::ptr::write_volatile(gicc, 1);
        }

        Ok(())
    }

    // =========================================================================
    // RISC-V IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "riscv64")]
    fn init_riscv(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        for _ in 0..MAX_VECTORS {
            self.vectors.push(VectorEntry::default());
        }

        // Detect PLIC from DTB
        self.controller = InterruptController::Plic;
        self.plic_base = 0x0C00_0000; // Common PLIC base

        ctx.debug(alloc::format!("PLIC base: 0x{:x}", self.plic_base));

        // Initialize PLIC
        self.init_plic(ctx)?;

        Ok(())
    }

    #[cfg(target_arch = "riscv64")]
    fn init_plic(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.debug("Initializing PLIC");

        // Set priority threshold to 0 (accept all)
        let threshold_ptr = (self.plic_base + 0x20_0000) as *mut u32;
        unsafe {
            core::ptr::write_volatile(threshold_ptr, 0);
        }

        // Enable external interrupts in sie
        unsafe {
            core::arch::asm!(
                "csrs sie, {}",
                in(reg) (1 << 9), // SEIE bit
                options(nostack)
            );
        }

        Ok(())
    }
}

impl Default for InterruptSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for InterruptSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing interrupt subsystem");

        #[cfg(target_arch = "x86_64")]
        self.init_x86(ctx)?;

        #[cfg(target_arch = "aarch64")]
        self.init_arm(ctx)?;

        #[cfg(target_arch = "riscv64")]
        self.init_riscv(ctx)?;

        ctx.info(alloc::format!(
            "Interrupt controller: {:?}",
            self.controller
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Interrupt subsystem shutdown");
        self.disable();
        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_subsystem() {
        let sub = InterruptSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Core);
        assert!(sub.info().provides.contains(PhaseCapabilities::INTERRUPTS));
    }

    #[test]
    fn test_exception_names() {
        assert_eq!(Exception::PageFault.name(), "Page Fault");
        assert!(Exception::PageFault.has_error_code());
        assert!(!Exception::DivideError.has_error_code());
    }

    #[test]
    fn test_gate_type() {
        assert_eq!(GateType::Interrupt as u8, 0xE);
        assert_eq!(GateType::Trap as u8, 0xF);
    }
}
