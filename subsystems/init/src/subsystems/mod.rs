//! # Subsystem Implementations
//!
//! This module contains reference implementations of core kernel subsystems.
//! Each subsystem is organized into its own submodule with full lifecycle
//! support.
//!
//! ## Module Structure
//!
//! ```text
//! subsystems/
//! ├── boot/        - Boot phase subsystems (firmware, console, boot_info)
//! ├── memory/      - Memory management (pmm, vmm, heap)
//! ├── cpu/         - CPU management (features, SMP)
//! ├── interrupts/  - Interrupt handling (IDT/GIC/PLIC)
//! ├── timers/      - Timer subsystem (APIC/Generic Timer/CLINT)
//! ├── scheduler/   - Task scheduling
//! ├── ipc/         - Inter-process communication
//! ├── drivers/     - Driver framework
//! ├── filesystem/  - VFS and filesystems
//! ├── network/     - Network stack
//! ├── security/    - Security subsystem
//! ├── debug/       - Debugging and tracing
//! └── userland/    - Userspace initialization
//! ```

pub mod boot;
pub mod cpu;
pub mod debug;
pub mod drivers;
pub mod filesystem;
pub mod interrupts;
pub mod ipc;
pub mod memory;
pub mod network;
pub mod scheduler;
pub mod security;
pub mod timers;
pub mod userland;

use crate::error::InitResult;
use crate::registry::SubsystemRegistry;
use crate::subsystem::SubsystemId;

extern crate alloc;
use alloc::boxed::Box;

/// Register all core subsystems
pub fn register_core_subsystems(registry: &mut SubsystemRegistry) -> InitResult<()> {
    // Boot phase
    registry.register(Box::new(boot::FirmwareSubsystem::new()))?;
    registry.register(Box::new(boot::BootInfoSubsystem::new()))?;
    registry.register(Box::new(boot::EarlyConsoleSubsystem::new()))?;

    // Early phase - Memory
    registry.register(Box::new(memory::PmmSubsystem::new()))?;
    registry.register(Box::new(memory::VmmSubsystem::new()))?;
    registry.register(Box::new(memory::HeapSubsystem::new()))?;

    // Early phase - CPU
    registry.register(Box::new(cpu::CpuSubsystem::new()))?;
    registry.register(Box::new(interrupts::InterruptSubsystem::new()))?;

    // Core phase
    registry.register(Box::new(timers::TimerSubsystem::new()))?;
    registry.register(Box::new(scheduler::SchedulerSubsystem::new()))?;
    registry.register(Box::new(ipc::IpcSubsystem::new()))?;

    // Late phase
    registry.register(Box::new(drivers::DriverSubsystem::new()))?;
    registry.register(Box::new(filesystem::FilesystemSubsystem::new()))?;
    registry.register(Box::new(network::NetworkSubsystem::new()))?;
    registry.register(Box::new(security::SecuritySubsystem::new()))?;

    // Runtime phase
    registry.register(Box::new(debug::DebugSubsystem::new()))?;
    registry.register(Box::new(userland::UserlandSubsystem::new()))?;

    Ok(())
}

/// Get all core subsystem IDs
pub fn core_subsystem_ids() -> &'static [SubsystemId] {
    use crate::subsystem::well_known::*;

    static IDS: &[SubsystemId] = &[
        FIRMWARE,
        BOOT_INFO,
        EARLY_CONSOLE,
        PMM,
        VMM,
        HEAP,
        CPU,
        INTERRUPTS,
        TIMERS,
        SCHEDULER,
        IPC,
        DRIVERS,
        VFS,
        NETWORK,
        SECURITY,
        DEBUG,
        USERLAND,
    ];

    IDS
}
