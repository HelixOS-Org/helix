//! # Boot Phase Subsystems
//!
//! Subsystems that run during the Boot phase when no heap is available.
//! These handle firmware handoff, boot info parsing, and early console.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// FIRMWARE SUBSYSTEM
// =============================================================================

/// Firmware handoff subsystem
///
/// Handles the transition from firmware (UEFI/BIOS) to kernel control.
/// This is the very first subsystem to run.
pub struct FirmwareSubsystem {
    info: SubsystemInfo,
    firmware_type: FirmwareType,
    exit_boot_services_called: bool,
}

/// Type of firmware
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    Unknown,
    Bios,
    Uefi,
    DeviceTree,
    Acpi,
}

impl FirmwareSubsystem {
    /// Create new firmware subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("firmware", InitPhase::Boot)
                .with_priority(1000)
                .with_description("Firmware handoff and initialization")
                .essential(),
            firmware_type: FirmwareType::Unknown,
            exit_boot_services_called: false,
        }
    }

    /// Get firmware type
    pub fn firmware_type(&self) -> FirmwareType {
        self.firmware_type
    }

    /// Detect firmware type from boot info
    fn detect_firmware_type(&mut self, ctx: &InitContext) {
        if let Some(boot_info) = ctx.boot_info() {
            if boot_info.efi_system_table.is_some() {
                self.firmware_type = FirmwareType::Uefi;
            } else if boot_info.dtb_addr.is_some() {
                self.firmware_type = FirmwareType::DeviceTree;
            } else if boot_info.rsdp_addr.is_some() {
                self.firmware_type = FirmwareType::Acpi;
            } else {
                self.firmware_type = FirmwareType::Bios;
            }
        }
    }
}

impl Default for FirmwareSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for FirmwareSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn validate(&self, _ctx: &InitContext) -> InitResult<()> {
        // Firmware subsystem always validates
        Ok(())
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing firmware subsystem");

        // Detect firmware type
        self.detect_firmware_type(ctx);

        ctx.info(alloc::format!(
            "Detected firmware: {:?}",
            self.firmware_type
        ));

        // Architecture-specific initialization
        #[cfg(target_arch = "x86_64")]
        {
            ctx.debug("x86_64: Preparing to exit boot services");
            // In real code: call UEFI ExitBootServices or similar
        }

        #[cfg(target_arch = "aarch64")]
        {
            ctx.debug("AArch64: Processing device tree / ACPI");
        }

        #[cfg(target_arch = "riscv64")]
        {
            ctx.debug("RISC-V: Initializing SBI interface");
        }

        self.exit_boot_services_called = true;

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Firmware subsystem shutdown");
        // Firmware shutdown is usually a no-op
        Ok(())
    }
}

// =============================================================================
// BOOT INFO SUBSYSTEM
// =============================================================================

/// Boot information parsing subsystem
///
/// Parses boot information from the bootloader (Limine, multiboot2, etc.)
/// and makes it available to other subsystems.
pub struct BootInfoSubsystem {
    info: SubsystemInfo,
    memory_map_entries: usize,
    total_memory: u64,
    usable_memory: u64,
    kernel_physical_base: u64,
    kernel_virtual_base: u64,
    kernel_size: u64,
    cmdline: Option<String>,
}

static BOOT_INFO_DEPS: [Dependency; 1] = [Dependency::required("firmware")];

impl BootInfoSubsystem {
    /// Create new boot info subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("boot_info", InitPhase::Boot)
                .with_priority(900)
                .with_description("Boot information parsing")
                .with_dependencies(&BOOT_INFO_DEPS)
                .essential(),
            memory_map_entries: 0,
            total_memory: 0,
            usable_memory: 0,
            kernel_physical_base: 0,
            kernel_virtual_base: 0,
            kernel_size: 0,
            cmdline: None,
        }
    }

    /// Get total system memory
    pub fn total_memory(&self) -> u64 {
        self.total_memory
    }

    /// Get usable memory
    pub fn usable_memory(&self) -> u64 {
        self.usable_memory
    }

    /// Get kernel command line
    pub fn cmdline(&self) -> Option<&str> {
        self.cmdline.as_deref()
    }

    /// Parse memory map
    fn parse_memory_map(&mut self, ctx: &InitContext) {
        if let Some(boot_info) = ctx.boot_info() {
            self.memory_map_entries = boot_info.memory_map.len();

            for region in &boot_info.memory_map {
                self.total_memory += region.length;

                use crate::context::MemoryKind;
                if matches!(
                    region.kind,
                    MemoryKind::Usable
                        | MemoryKind::BootloaderReclaimable
                        | MemoryKind::AcpiReclaimable
                ) {
                    self.usable_memory += region.length;
                }

                if region.kind == MemoryKind::KernelAndModules {
                    self.kernel_physical_base = region.base;
                    self.kernel_size = region.length;
                }
            }
        }
    }
}

impl Default for BootInfoSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for BootInfoSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Parsing boot information");

        // Parse command line
        if let Some(boot_info) = ctx.boot_info() {
            self.cmdline = boot_info.cmdline.clone();
        }

        // Parse memory map
        self.parse_memory_map(ctx);

        ctx.info(alloc::format!(
            "Memory: {} MB total, {} MB usable",
            self.total_memory / (1024 * 1024),
            self.usable_memory / (1024 * 1024)
        ));

        ctx.info(alloc::format!(
            "Memory map: {} entries",
            self.memory_map_entries
        ));

        if let Some(ref cmdline) = self.cmdline {
            ctx.debug(alloc::format!("Command line: {}", cmdline));
        }

        Ok(())
    }
}

// =============================================================================
// EARLY CONSOLE SUBSYSTEM
// =============================================================================

/// Early console subsystem
///
/// Provides basic output capability during early boot before the
/// full console driver is available.
pub struct EarlyConsoleSubsystem {
    info: SubsystemInfo,
    console_type: ConsoleType,
    initialized: bool,
}

/// Type of early console
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleType {
    None,
    Serial,
    Framebuffer,
    Both,
}

impl EarlyConsoleSubsystem {
    /// Create new early console subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("early_console", InitPhase::Boot)
                .with_priority(800)
                .with_description("Early console output")
                .provides(PhaseCapabilities::CONSOLE),
            console_type: ConsoleType::None,
            initialized: false,
        }
    }

    /// Get console type
    pub fn console_type(&self) -> ConsoleType {
        self.console_type
    }

    /// Write string to console
    pub fn write(&self, s: &str) {
        if !self.initialized {
            return;
        }

        match self.console_type {
            ConsoleType::Serial | ConsoleType::Both => {
                self.write_serial(s);
            },
            ConsoleType::Framebuffer => {
                self.write_framebuffer(s);
            },
            ConsoleType::None => {},
        }
    }

    fn write_serial(&self, s: &str) {
        // In real code: write to serial port
        #[cfg(target_arch = "x86_64")]
        {
            for byte in s.bytes() {
                // Serial port write (0x3F8 = COM1)
                unsafe {
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") 0x3F8u16,
                        in("al") byte,
                        options(nostack, preserves_flags)
                    );
                }
            }
        }
    }

    fn write_framebuffer(&self, _s: &str) {
        // In real code: write to framebuffer
    }

    fn detect_console(&mut self, ctx: &InitContext) {
        // Check for framebuffer
        if let Some(boot_info) = ctx.boot_info() {
            if boot_info.framebuffer.is_some() {
                self.console_type = ConsoleType::Both;
                return;
            }
        }

        // Default to serial
        self.console_type = ConsoleType::Serial;
    }
}

impl Default for EarlyConsoleSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for EarlyConsoleSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing early console");

        // Detect available console
        self.detect_console(ctx);

        // Initialize serial port
        #[cfg(target_arch = "x86_64")]
        {
            // Initialize COM1
            unsafe {
                // Disable interrupts
                core::arch::asm!("out dx, al", in("dx") 0x3F9u16, in("al") 0u8);
                // Enable DLAB
                core::arch::asm!("out dx, al", in("dx") 0x3FBu16, in("al") 0x80u8);
                // Set baud rate divisor (115200)
                core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") 1u8);
                core::arch::asm!("out dx, al", in("dx") 0x3F9u16, in("al") 0u8);
                // 8 bits, no parity, one stop bit
                core::arch::asm!("out dx, al", in("dx") 0x3FBu16, in("al") 0x03u8);
                // Enable FIFO
                core::arch::asm!("out dx, al", in("dx") 0x3FAu16, in("al") 0xC7u8);
                // IRQs enabled, RTS/DSR set
                core::arch::asm!("out dx, al", in("dx") 0x3FCu16, in("al") 0x0Bu8);
            }
        }

        self.initialized = true;

        ctx.info(alloc::format!("Console type: {:?}", self.console_type));

        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        self.initialized = false;
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
    fn test_firmware_subsystem() {
        let sub = FirmwareSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Boot);
        assert!(sub.info().essential);
        assert_eq!(sub.firmware_type(), FirmwareType::Unknown);
    }

    #[test]
    fn test_boot_info_subsystem() {
        let sub = BootInfoSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Boot);
        assert_eq!(sub.total_memory(), 0);
    }

    #[test]
    fn test_early_console_subsystem() {
        let sub = EarlyConsoleSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Boot);
        assert!(sub.info().provides.contains(PhaseCapabilities::CONSOLE));
    }
}
