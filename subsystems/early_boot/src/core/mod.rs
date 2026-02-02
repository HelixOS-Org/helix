//! # Core Boot Abstractions
//!
//! This module defines the fundamental types and traits for the early boot sequence.
//! All architecture-specific implementations must conform to these interfaces.
//!
//! ## Boot Stage Model
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                         BOOT STAGE STATE MACHINE                            │
//! │                                                                             │
//! │   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐     │
//! │   │ PreInit │──▶│ CpuInit │──▶│ Memory  │──▶│ Drivers │──▶│Interrupt│     │
//! │   │         │   │         │   │  Init   │   │  Init   │   │  Init   │     │
//! │   └─────────┘   └─────────┘   └─────────┘   └─────────┘   └─────────┘     │
//! │        │             │             │             │             │           │
//! │        ▼             ▼             ▼             ▼             ▼           │
//! │   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐     │
//! │   │  Error  │   │  Error  │   │  Error  │   │  Error  │   │  Error  │     │
//! │   └─────────┘   └─────────┘   └─────────┘   └─────────┘   └─────────┘     │
//! │                                                                             │
//! │   ┌─────────┐   ┌─────────┐   ┌─────────┐                                  │
//! │   │  Timer  │──▶│   SMP   │──▶│ Handoff │──▶ [KERNEL ENTRY]               │
//! │   │  Init   │   │  Init   │   │         │                                  │
//! │   └─────────┘   └─────────┘   └─────────┘                                  │
//! │        │             │             │                                        │
//! │        ▼             ▼             ▼                                        │
//! │   ┌─────────┐   ┌─────────┐   ┌─────────┐                                  │
//! │   │  Error  │   │  Error  │   │  Error  │                                  │
//! │   └─────────┘   └─────────┘   └─────────┘                                  │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::error::{BootError, BootResult};
use crate::info::BootInfo;
use crate::{Architecture, BootCapabilities, BootConfig, BootStatus};

// =============================================================================
// BOOT STAGES
// =============================================================================

/// Boot stage enumeration
///
/// Represents the current stage of the early boot sequence.
/// Each stage builds upon the previous one and must complete successfully
/// before the next stage can begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BootStage {
    /// Stage 0: Pre-initialization
    /// - Minimal CPU setup (stack, basic registers)
    /// - Serial output initialization (if available)
    /// - Boot information validation
    PreInit       = 0,

    /// Stage 1: CPU initialization
    /// - Full CPU feature detection
    /// - Cache configuration
    /// - FPU/SIMD state initialization
    /// - Architecture-specific setup (GDT on x86, exception levels on ARM)
    CpuInit       = 1,

    /// Stage 2: Memory initialization
    /// - Physical memory map processing
    /// - Page table creation
    /// - Kernel mapping
    /// - Early heap setup
    MemoryInit    = 2,

    /// Stage 3: Early driver initialization
    /// - Console drivers (serial, framebuffer)
    /// - Essential platform drivers
    /// - Debug facilities
    DriverInit    = 3,

    /// Stage 4: Interrupt initialization
    /// - IDT/exception vector setup
    /// - IRQ controller configuration (APIC/GIC/PLIC)
    /// - Exception handler registration
    InterruptInit = 4,

    /// Stage 5: Timer initialization
    /// - Timer detection and calibration
    /// - System tick configuration
    /// - TSC/HPET/Generic timer setup
    TimerInit     = 5,

    /// Stage 6: SMP initialization
    /// - Secondary CPU discovery
    /// - AP startup (INIT-SIPI-SIPI / PSCI / SBI HSM)
    /// - Per-CPU data initialization
    SmpInit       = 6,

    /// Stage 7: Handoff to main kernel
    /// - KASLR relocation (if enabled)
    /// - Final memory layout
    /// - Kernel entry point preparation
    Handoff       = 7,

    /// Error state
    Error         = 255,
}

impl BootStage {
    /// Get the next stage in the sequence
    pub fn next(self) -> Option<Self> {
        match self {
            Self::PreInit => Some(Self::CpuInit),
            Self::CpuInit => Some(Self::MemoryInit),
            Self::MemoryInit => Some(Self::DriverInit),
            Self::DriverInit => Some(Self::InterruptInit),
            Self::InterruptInit => Some(Self::TimerInit),
            Self::TimerInit => Some(Self::SmpInit),
            Self::SmpInit => Some(Self::Handoff),
            Self::Handoff => None,
            Self::Error => None,
        }
    }

    /// Get stage name
    pub const fn name(self) -> &'static str {
        match self {
            Self::PreInit => "Pre-Init",
            Self::CpuInit => "CPU Init",
            Self::MemoryInit => "Memory Init",
            Self::DriverInit => "Driver Init",
            Self::InterruptInit => "Interrupt Init",
            Self::TimerInit => "Timer Init",
            Self::SmpInit => "SMP Init",
            Self::Handoff => "Handoff",
            Self::Error => "Error",
        }
    }

    /// Get stage description
    pub const fn description(self) -> &'static str {
        match self {
            Self::PreInit => "Minimal CPU and serial setup",
            Self::CpuInit => "Full CPU initialization and feature detection",
            Self::MemoryInit => "Physical memory and paging setup",
            Self::DriverInit => "Console and essential drivers",
            Self::InterruptInit => "Exception vectors and IRQ controllers",
            Self::TimerInit => "System timers and calibration",
            Self::SmpInit => "Secondary CPU startup",
            Self::Handoff => "Kernel relocation and entry",
            Self::Error => "Boot sequence error",
        }
    }

    /// Check if this stage is essential (boot fails without it)
    pub const fn is_essential(self) -> bool {
        match self {
            Self::PreInit
            | Self::CpuInit
            | Self::MemoryInit
            | Self::InterruptInit
            | Self::Handoff => true,
            Self::DriverInit | Self::TimerInit | Self::SmpInit => false,
            Self::Error => false,
        }
    }

    /// Get corresponding status flag
    pub const fn status_flag(self) -> BootStatus {
        match self {
            Self::PreInit => BootStatus::PRE_INIT,
            Self::CpuInit => BootStatus::CPU_INIT,
            Self::MemoryInit => BootStatus::MEMORY_INIT,
            Self::DriverInit => BootStatus::DRIVERS_INIT,
            Self::InterruptInit => BootStatus::INTERRUPTS_INIT,
            Self::TimerInit => BootStatus::TIMERS_INIT,
            Self::SmpInit => BootStatus::SMP_INIT,
            Self::Handoff => BootStatus::HANDOFF,
            Self::Error => BootStatus::ERROR,
        }
    }
}

// =============================================================================
// BOOT STATE
// =============================================================================

/// Global boot state tracking
///
/// Maintains the current state of the boot process, including completed stages,
/// detected capabilities, and timing information.
#[derive(Debug)]
pub struct BootState {
    /// Current boot stage
    current_stage: BootStage,

    /// Boot status flags
    status: BootStatus,

    /// Detected capabilities
    capabilities: BootCapabilities,

    /// Target architecture
    architecture: Architecture,

    /// CPU count detected
    cpu_count: usize,

    /// Total physical memory (bytes)
    total_memory: u64,

    /// Boot start timestamp (architecture-specific units)
    boot_start_time: u64,

    /// Per-stage timing (in microseconds)
    stage_times: [u64; 8],

    /// Error message if error occurred
    error_message: Option<&'static str>,
}

impl BootState {
    /// Create a new boot state
    pub const fn new() -> Self {
        Self {
            current_stage: BootStage::PreInit,
            status: BootStatus::empty(),
            capabilities: BootCapabilities::empty(),
            architecture: Architecture::Unknown,
            cpu_count: 1,
            total_memory: 0,
            boot_start_time: 0,
            stage_times: [0; 8],
            error_message: None,
        }
    }

    /// Get current stage
    pub fn current_stage(&self) -> BootStage {
        self.current_stage
    }

    /// Set current stage
    pub fn set_stage(&mut self, stage: BootStage) {
        self.current_stage = stage;
    }

    /// Mark stage as complete
    pub fn complete_stage(&mut self, stage: BootStage, time_us: u64) {
        self.status.insert(stage.status_flag());
        if (stage as usize) < 8 {
            self.stage_times[stage as usize] = time_us;
        }
    }

    /// Check if stage is complete
    pub fn stage_complete(&self, stage: BootStage) -> bool {
        self.status.contains(stage.status_flag())
    }

    /// Get boot status
    pub fn status(&self) -> BootStatus {
        self.status
    }

    /// Get capabilities
    pub fn capabilities(&self) -> BootCapabilities {
        self.capabilities
    }

    /// Add capability
    pub fn add_capability(&mut self, cap: BootCapabilities) {
        self.capabilities.insert(cap);
    }

    /// Set architecture
    pub fn set_architecture(&mut self, arch: Architecture) {
        self.architecture = arch;
    }

    /// Get architecture
    pub fn architecture(&self) -> Architecture {
        self.architecture
    }

    /// Set CPU count
    pub fn set_cpu_count(&mut self, count: usize) {
        self.cpu_count = count;
    }

    /// Get CPU count
    pub fn cpu_count(&self) -> usize {
        self.cpu_count
    }

    /// Set total memory
    pub fn set_total_memory(&mut self, bytes: u64) {
        self.total_memory = bytes;
    }

    /// Get total memory
    pub fn total_memory(&self) -> u64 {
        self.total_memory
    }

    /// Set boot start time
    pub fn set_boot_start_time(&mut self, time: u64) {
        self.boot_start_time = time;
    }

    /// Get boot start time
    pub fn boot_start_time(&self) -> u64 {
        self.boot_start_time
    }

    /// Get stage timing
    pub fn stage_time(&self, stage: BootStage) -> u64 {
        if (stage as usize) < 8 {
            self.stage_times[stage as usize]
        } else {
            0
        }
    }

    /// Get total boot time
    pub fn total_boot_time(&self) -> u64 {
        self.stage_times.iter().sum()
    }

    /// Set error
    pub fn set_error(&mut self, message: &'static str) {
        self.current_stage = BootStage::Error;
        self.status.insert(BootStatus::ERROR);
        self.error_message = Some(message);
    }

    /// Get error message
    pub fn error_message(&self) -> Option<&'static str> {
        self.error_message
    }

    /// Check if in error state
    pub fn has_error(&self) -> bool {
        self.status.contains(BootStatus::ERROR)
    }
}

impl Default for BootState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// BOOT CONTEXT
// =============================================================================

/// Boot context passed between stages
///
/// Contains all the information accumulated during the boot process,
/// used by each stage to access results from previous stages.
pub struct BootContext {
    /// Boot configuration
    pub config: BootConfig,

    /// Boot information from bootloader
    pub boot_info: *const BootInfo,

    /// CPU state
    pub cpu_state: CpuState,

    /// Memory state
    pub memory_state: MemoryState,

    /// Interrupt state
    pub interrupt_state: InterruptState,

    /// Timer state
    pub timer_state: TimerState,

    /// SMP state
    pub smp_state: SmpState,

    /// Architecture-specific data
    pub arch_data: ArchData,
}

impl BootContext {
    /// Create a new boot context
    pub fn new(config: BootConfig, boot_info: *const BootInfo) -> Self {
        Self {
            config,
            boot_info,
            cpu_state: CpuState::new(),
            memory_state: MemoryState::new(),
            interrupt_state: InterruptState::new(),
            timer_state: TimerState::new(),
            smp_state: SmpState::new(),
            arch_data: ArchData::new(),
        }
    }

    /// Get boot info reference
    ///
    /// # Safety
    /// The boot_info pointer must still be valid
    pub unsafe fn boot_info(&self) -> &BootInfo {
        &*self.boot_info
    }
}

// =============================================================================
// CPU STATE
// =============================================================================

/// CPU state accumulated during boot
#[derive(Debug, Default)]
pub struct CpuState {
    /// CPU vendor string
    pub vendor: [u8; 16],

    /// CPU model name
    pub model_name: [u8; 64],

    /// CPU family
    pub family: u32,

    /// CPU model
    pub model: u32,

    /// CPU stepping
    pub stepping: u32,

    /// Number of logical processors
    pub logical_count: u32,

    /// Number of physical cores
    pub core_count: u32,

    /// L1 data cache size (KB)
    pub l1d_cache_kb: u32,

    /// L1 instruction cache size (KB)
    pub l1i_cache_kb: u32,

    /// L2 cache size (KB)
    pub l2_cache_kb: u32,

    /// L3 cache size (KB)
    pub l3_cache_kb: u32,

    /// Features detected
    pub features: CpuFeatures,

    /// Current privilege level
    pub privilege_level: u8,

    /// BSP (Boot Strap Processor) ID
    pub bsp_id: u32,
}

impl CpuState {
    /// Create new CPU state
    pub const fn new() -> Self {
        Self {
            vendor: [0; 16],
            model_name: [0; 64],
            family: 0,
            model: 0,
            stepping: 0,
            logical_count: 1,
            core_count: 1,
            l1d_cache_kb: 0,
            l1i_cache_kb: 0,
            l2_cache_kb: 0,
            l3_cache_kb: 0,
            features: CpuFeatures::empty(),
            privilege_level: 0,
            bsp_id: 0,
        }
    }
}

use bitflags::bitflags;

bitflags! {
    /// CPU feature flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct CpuFeatures: u64 {
        // Common features
        const FPU = 1 << 0;
        const SIMD = 1 << 1;
        const ATOMIC = 1 << 2;
        const CAS = 1 << 3;

        // x86_64 features
        const SSE = 1 << 8;
        const SSE2 = 1 << 9;
        const SSE3 = 1 << 10;
        const SSSE3 = 1 << 11;
        const SSE4_1 = 1 << 12;
        const SSE4_2 = 1 << 13;
        const AVX = 1 << 14;
        const AVX2 = 1 << 15;
        const AVX512 = 1 << 16;
        const XSAVE = 1 << 17;
        const FSGSBASE = 1 << 18;
        const PCID = 1 << 19;
        const INVPCID = 1 << 20;
        const LA57 = 1 << 21;
        const TSC = 1 << 22;
        const TSC_DEADLINE = 1 << 23;
        const RDRAND = 1 << 24;
        const RDSEED = 1 << 25;
        const UMIP = 1 << 26;
        const PKU = 1 << 27;

        // ARM64 features
        const NEON = 1 << 32;
        const SVE = 1 << 33;
        const SVE2 = 1 << 34;
        const SME = 1 << 35;
        const BTI = 1 << 36;
        const MTE = 1 << 37;
        const PAC = 1 << 38;
        const RNG = 1 << 39;
        const ATOMICS_LSE = 1 << 40;
        const FP16 = 1 << 41;

        // RISC-V features
        const COMPRESSED = 1 << 48;
        const MULTIPLY = 1 << 49;
        const FLOAT = 1 << 50;
        const DOUBLE = 1 << 51;
        const VECTOR_RV = 1 << 52;
        const BITMANIP = 1 << 53;
        const CRYPTO_RV = 1 << 54;
        const HYPERVISOR_RV = 1 << 55;
    }
}

// =============================================================================
// MEMORY STATE
// =============================================================================

/// Memory state accumulated during boot
#[derive(Debug)]
pub struct MemoryState {
    /// Total physical memory (bytes)
    pub total_physical: u64,

    /// Usable physical memory (bytes)
    pub usable_physical: u64,

    /// Reserved memory (bytes)
    pub reserved: u64,

    /// ACPI reclaimable memory (bytes)
    pub acpi_reclaimable: u64,

    /// Kernel physical start
    pub kernel_phys_start: u64,

    /// Kernel physical end
    pub kernel_phys_end: u64,

    /// Kernel virtual base
    pub kernel_virt_base: u64,

    /// HHDM (Higher Half Direct Map) offset
    pub hhdm_offset: u64,

    /// Page table root physical address
    pub page_table_root: u64,

    /// Early heap start
    pub early_heap_start: u64,

    /// Early heap size
    pub early_heap_size: u64,

    /// Memory map entries (simplified)
    pub memory_map_count: usize,

    /// Paging mode
    pub paging_mode: PagingMode,
}

impl MemoryState {
    /// Create new memory state
    pub const fn new() -> Self {
        Self {
            total_physical: 0,
            usable_physical: 0,
            reserved: 0,
            acpi_reclaimable: 0,
            kernel_phys_start: 0,
            kernel_phys_end: 0,
            kernel_virt_base: 0xFFFF_FFFF_8000_0000,
            hhdm_offset: 0xFFFF_8000_0000_0000,
            page_table_root: 0,
            early_heap_start: 0,
            early_heap_size: 0,
            memory_map_count: 0,
            paging_mode: PagingMode::FourLevel,
        }
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Paging mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingMode {
    /// 4-level paging (48-bit VA)
    FourLevel,
    /// 5-level paging (57-bit VA, x86_64 LA57)
    FiveLevel,
    /// Sv39 (RISC-V 39-bit VA)
    Sv39,
    /// Sv48 (RISC-V 48-bit VA)
    Sv48,
    /// Sv57 (RISC-V 57-bit VA)
    Sv57,
}

// =============================================================================
// INTERRUPT STATE
// =============================================================================

/// Interrupt state accumulated during boot
#[derive(Debug)]
pub struct InterruptState {
    /// Interrupt controller type
    pub controller_type: InterruptControllerType,

    /// Controller base address
    pub controller_base: u64,

    /// Number of IRQs supported
    pub irq_count: u32,

    /// IRQ base for external interrupts
    pub external_irq_base: u32,

    /// Exception vector base
    pub vector_base: u64,

    /// Interrupts currently enabled
    pub enabled: bool,

    /// I/O APIC base (x86_64)
    pub ioapic_base: u64,

    /// I/O APIC count (x86_64)
    pub ioapic_count: u8,

    /// GIC redistributor base (ARM64 GICv3+)
    pub gicr_base: u64,
}

impl InterruptState {
    /// Create new interrupt state
    pub const fn new() -> Self {
        Self {
            controller_type: InterruptControllerType::Unknown,
            controller_base: 0,
            irq_count: 0,
            external_irq_base: 0,
            vector_base: 0,
            enabled: false,
            ioapic_base: 0,
            ioapic_count: 0,
            gicr_base: 0,
        }
    }
}

impl Default for InterruptState {
    fn default() -> Self {
        Self::new()
    }
}

/// Interrupt controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptControllerType {
    /// Unknown/not detected
    Unknown,
    /// x86 legacy PIC (8259)
    Pic8259,
    /// x86 Local APIC
    LocalApic,
    /// x86 x2APIC
    X2Apic,
    /// ARM GICv2
    GicV2,
    /// ARM GICv3
    GicV3,
    /// ARM GICv4
    GicV4,
    /// RISC-V PLIC
    Plic,
    /// RISC-V CLINT (for timer/software interrupts)
    Clint,
    /// RISC-V AIA (Advanced Interrupt Architecture)
    Aia,
}

// =============================================================================
// TIMER STATE
// =============================================================================

/// Timer state accumulated during boot
#[derive(Debug)]
pub struct TimerState {
    /// Primary timer type
    pub primary_timer: TimerType,

    /// Timer frequency (Hz)
    pub frequency: u64,

    /// Timer base address (for MMIO timers)
    pub base_address: u64,

    /// TSC frequency (x86_64)
    pub tsc_frequency: u64,

    /// Whether TSC is invariant (x86_64)
    pub tsc_invariant: bool,

    /// System tick rate (Hz)
    pub tick_rate: u32,

    /// Timer calibrated
    pub calibrated: bool,
}

impl TimerState {
    /// Create new timer state
    pub const fn new() -> Self {
        Self {
            primary_timer: TimerType::Unknown,
            frequency: 0,
            base_address: 0,
            tsc_frequency: 0,
            tsc_invariant: false,
            tick_rate: 1000, // Default 1000 Hz
            calibrated: false,
        }
    }
}

impl Default for TimerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    Unknown,
    /// x86 PIT (8254)
    Pit,
    /// x86 HPET
    Hpet,
    /// x86 Local APIC Timer
    ApicTimer,
    /// x86 TSC Deadline
    TscDeadline,
    /// ARM Generic Timer
    GenericTimer,
    /// RISC-V CLINT Timer
    ClintTimer,
    /// RISC-V SBI Timer
    SbiTimer,
}

// =============================================================================
// SMP STATE
// =============================================================================

/// SMP state accumulated during boot
#[derive(Debug)]
pub struct SmpState {
    /// Total CPUs detected
    pub cpu_count: usize,

    /// CPUs successfully started
    pub cpus_online: usize,

    /// BSP (Boot Strap Processor) ID
    pub bsp_id: u32,

    /// Per-CPU data base address
    pub percpu_base: u64,

    /// Per-CPU data size
    pub percpu_size: usize,

    /// CPU topology detected
    pub topology_detected: bool,

    /// Packages (sockets)
    pub package_count: u8,

    /// Cores per package
    pub cores_per_package: u8,

    /// Threads per core
    pub threads_per_core: u8,

    /// SMP startup method
    pub startup_method: SmpStartupMethod,
}

impl SmpState {
    /// Create new SMP state
    pub const fn new() -> Self {
        Self {
            cpu_count: 1,
            cpus_online: 1,
            bsp_id: 0,
            percpu_base: 0,
            percpu_size: 0,
            topology_detected: false,
            package_count: 1,
            cores_per_package: 1,
            threads_per_core: 1,
            startup_method: SmpStartupMethod::None,
        }
    }
}

impl Default for SmpState {
    fn default() -> Self {
        Self::new()
    }
}

/// SMP startup methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpStartupMethod {
    /// No SMP (single CPU)
    None,
    /// x86 INIT-SIPI-SIPI
    InitSipiSipi,
    /// ARM PSCI
    Psci,
    /// ARM Spin Table
    SpinTable,
    /// RISC-V SBI HSM
    SbiHsm,
}

// =============================================================================
// ARCHITECTURE-SPECIFIC DATA
// =============================================================================

/// Architecture-specific boot data
#[derive(Debug)]
pub struct ArchData {
    /// x86_64 specific data
    #[cfg(target_arch = "x86_64")]
    pub x86: X86Data,

    /// AArch64 specific data
    #[cfg(target_arch = "aarch64")]
    pub arm: ArmData,

    /// RISC-V specific data
    #[cfg(target_arch = "riscv64")]
    pub riscv: RiscvData,

    /// Placeholder for other architectures
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    _placeholder: (),
}

impl ArchData {
    /// Create new architecture data
    pub fn new() -> Self {
        Self {
            #[cfg(target_arch = "x86_64")]
            x86: X86Data::new(),

            #[cfg(target_arch = "aarch64")]
            arm: ArmData::new(),

            #[cfg(target_arch = "riscv64")]
            riscv: RiscvData::new(),

            #[cfg(not(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64"
            )))]
            _placeholder: (),
        }
    }
}

impl Default for ArchData {
    fn default() -> Self {
        Self::new()
    }
}

/// x86_64 specific boot data
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Default)]
pub struct X86Data {
    /// GDT base address
    pub gdt_base: u64,
    /// GDT limit
    pub gdt_limit: u16,
    /// IDT base address
    pub idt_base: u64,
    /// IDT limit
    pub idt_limit: u16,
    /// TSS base address
    pub tss_base: u64,
    /// CR0 value
    pub cr0: u64,
    /// CR3 value (page table root)
    pub cr3: u64,
    /// CR4 value
    pub cr4: u64,
    /// EFER MSR value
    pub efer: u64,
    /// PAT MSR value
    pub pat: u64,
    /// Local APIC base
    pub lapic_base: u64,
    /// Local APIC ID
    pub lapic_id: u32,
    /// x2APIC mode enabled
    pub x2apic_enabled: bool,
}

#[cfg(target_arch = "x86_64")]
impl X86Data {
    pub const fn new() -> Self {
        Self {
            gdt_base: 0,
            gdt_limit: 0,
            idt_base: 0,
            idt_limit: 0,
            tss_base: 0,
            cr0: 0,
            cr3: 0,
            cr4: 0,
            efer: 0,
            pat: 0,
            lapic_base: 0xFEE0_0000,
            lapic_id: 0,
            x2apic_enabled: false,
        }
    }
}

/// AArch64 specific boot data
#[cfg(target_arch = "aarch64")]
#[derive(Debug, Default)]
pub struct ArmData {
    /// Current exception level (0-3)
    pub current_el: u8,
    /// SCTLR_EL1 value
    pub sctlr_el1: u64,
    /// TCR_EL1 value
    pub tcr_el1: u64,
    /// MAIR_EL1 value
    pub mair_el1: u64,
    /// TTBR0_EL1 value (user space page tables)
    pub ttbr0_el1: u64,
    /// TTBR1_EL1 value (kernel page tables)
    pub ttbr1_el1: u64,
    /// VBAR_EL1 value (exception vector base)
    pub vbar_el1: u64,

    // GIC (Generic Interrupt Controller) configuration
    /// GIC distributor base address
    pub gicd_base: u64,
    /// GIC CPU interface base (GICv2)
    pub gicc_base: u64,
    /// GIC redistributor base (GICv3+)
    pub gicr_base: u64,
    /// GIC ITS base (GICv3+ Interrupt Translation Service)
    pub gic_its_base: u64,
    /// GIC version (2, 3, or 4)
    pub gic_version: u8,
    /// Number of supported IRQs
    pub gic_num_irqs: u16,
    /// Number of redistributors (GICv3+)
    pub gic_num_redist: u8,

    // Generic Timer configuration
    /// Timer frequency (CNTFRQ_EL0)
    pub timer_frequency: u64,
    /// Physical timer offset
    pub timer_phys_offset: u64,
    /// Virtual timer offset
    pub timer_virt_offset: u64,

    // PSCI (Power State Coordination Interface)
    /// PSCI version (major << 16 | minor)
    pub psci_version: u32,
    /// PSCI conduit (0=unknown, 1=SMC, 2=HVC)
    pub psci_conduit: u8,
    /// PSCI features supported (bitmask)
    pub psci_features: u32,

    // CPU identification
    /// Main ID Register (MIDR_EL1)
    pub midr: u64,
    /// Multiprocessor Affinity Register (MPIDR_EL1)
    pub mpidr: u64,
    /// Revision ID Register (REVIDR_EL1)
    pub revidr: u64,

    // Memory and cache configuration
    /// Cache Type Register (CTR_EL0)
    pub ctr_el0: u64,
    /// Data cache line size (bytes)
    pub dcache_line_size: u32,
    /// Instruction cache line size (bytes)
    pub icache_line_size: u32,

    // Feature registers
    /// ID_AA64MMFR0_EL1 - memory model features
    pub id_aa64mmfr0: u64,
    /// ID_AA64MMFR1_EL1 - memory model features
    pub id_aa64mmfr1: u64,
    /// ID_AA64PFR0_EL1 - processor features
    pub id_aa64pfr0: u64,
    /// ID_AA64ISAR0_EL1 - instruction set features
    pub id_aa64isar0: u64,

    // Debug/Trace
    /// Debug enabled
    pub debug_enabled: bool,
    /// Trace enabled
    pub trace_enabled: bool,

    // Serial console
    /// PL011 UART base address
    pub uart_base: u64,
    /// UART clock frequency
    pub uart_clock: u32,
}

#[cfg(target_arch = "aarch64")]
impl ArmData {
    pub const fn new() -> Self {
        Self {
            current_el: 1,
            sctlr_el1: 0,
            tcr_el1: 0,
            mair_el1: 0,
            ttbr0_el1: 0,
            ttbr1_el1: 0,
            vbar_el1: 0,
            gicd_base: 0,
            gicc_base: 0,
            gicr_base: 0,
            gic_its_base: 0,
            gic_version: 0,
            gic_num_irqs: 0,
            gic_num_redist: 0,
            timer_frequency: 0,
            timer_phys_offset: 0,
            timer_virt_offset: 0,
            psci_version: 0,
            psci_conduit: 0,
            psci_features: 0,
            midr: 0,
            mpidr: 0,
            revidr: 0,
            ctr_el0: 0,
            dcache_line_size: 64,
            icache_line_size: 64,
            id_aa64mmfr0: 0,
            id_aa64mmfr1: 0,
            id_aa64pfr0: 0,
            id_aa64isar0: 0,
            debug_enabled: false,
            trace_enabled: false,
            uart_base: 0x0900_0000, // QEMU virt default
            uart_clock: 24_000_000, // 24 MHz
        }
    }
}

/// RISC-V specific boot data
#[cfg(target_arch = "riscv64")]
#[derive(Debug, Default)]
pub struct RiscvData {
    // Privilege and mode information
    /// Current privilege mode (0=U, 1=S, 3=M)
    pub privilege_mode: u8,
    /// SSTATUS value (supervisor status)
    pub sstatus: u64,
    /// MSTATUS value (machine status, if available)
    pub mstatus: u64,

    // Memory management
    /// SATP value (supervisor address translation and protection)
    pub satp: u64,
    /// Paging mode (0=Sv39, 1=Sv48, 2=Sv57)
    pub paging_mode: u8,
    /// Root page table physical address
    pub root_page_table: u64,

    // Exception/Interrupt vectors
    /// STVEC value (supervisor trap vector)
    pub stvec: u64,
    /// MTVEC value (machine trap vector, if available)
    pub mtvec: u64,
    /// SIE value (supervisor interrupt enable)
    pub sie: u64,
    /// SIP value (supervisor interrupt pending)
    pub sip: u64,

    // PLIC (Platform-Level Interrupt Controller)
    /// PLIC base address
    pub plic_base: u64,
    /// Number of interrupt sources
    pub plic_num_sources: u32,
    /// Number of contexts (harts * privilege levels)
    pub plic_num_contexts: u32,
    /// PLIC priority threshold
    pub plic_threshold: u8,

    // CLINT (Core Local Interruptor)
    /// CLINT base address
    pub clint_base: u64,
    /// MTIME register value at boot
    pub mtime_at_boot: u64,

    // Hart (Hardware Thread) information
    /// Current hart ID
    pub hart_id: u64,
    /// Number of harts detected
    pub num_harts: u32,
    /// Hart mask (bitmap of available harts)
    pub hart_mask: u64,

    // SBI (Supervisor Binary Interface) information
    /// SBI specification version (major << 24 | minor << 16)
    pub sbi_spec_version: u32,
    /// SBI implementation ID
    pub sbi_impl_id: u64,
    /// SBI implementation version
    pub sbi_impl_version: u64,
    /// Available SBI extensions (bitmask)
    pub sbi_extensions: u64,
    /// SBI HSM extension available
    pub sbi_hsm_available: bool,
    /// SBI SRST extension available
    pub sbi_srst_available: bool,
    /// SBI PMU extension available
    pub sbi_pmu_available: bool,

    // Timer
    /// Timer frequency (timebase-frequency from DTB)
    pub timebase_frequency: u64,
    /// Timer ticks at boot
    pub timer_ticks_at_boot: u64,

    // ISA and CPU identification
    /// MISA value (ISA register)
    pub misa: u64,
    /// Vendor ID (mvendorid)
    pub vendor_id: u64,
    /// Architecture ID (marchid)
    pub arch_id: u64,
    /// Implementation ID (mimpid)
    pub imp_id: u64,

    // ISA extension flags (decoded from misa)
    /// Atomic extension (A)
    pub has_atomic: bool,
    /// Compressed extension (C)
    pub has_compressed: bool,
    /// Double-precision FP (D)
    pub has_double_fp: bool,
    /// Single-precision FP (F)
    pub has_single_fp: bool,
    /// Hypervisor extension (H)
    pub has_hypervisor: bool,
    /// Integer multiply/divide (M)
    pub has_multiply: bool,
    /// Supervisor mode (S)
    pub has_supervisor: bool,
    /// User mode (U)
    pub has_user: bool,
    /// Vector extension (V)
    pub has_vector: bool,
    /// Bit manipulation (B) - Zba, Zbb, Zbc, Zbs
    pub has_bitmanip: bool,
    /// Crypto extension (K) - Zkn, Zks
    pub has_crypto: bool,

    // PMP (Physical Memory Protection)
    /// Number of PMP regions
    pub pmp_count: u8,
    /// PMP granularity (log2)
    pub pmp_granularity: u8,

    // Debug/Trace
    /// Debug trigger count
    pub debug_trigger_count: u8,
    /// Debug mode available
    pub debug_available: bool,

    // Serial console
    /// UART base address (16550 compatible)
    pub uart_base: u64,
    /// UART clock frequency
    pub uart_clock: u32,
    /// Use SBI console instead of UART
    pub use_sbi_console: bool,

    // Device Tree
    /// Device tree blob address (passed from bootloader)
    pub dtb_address: u64,
    /// Device tree size
    pub dtb_size: u32,
}

#[cfg(target_arch = "riscv64")]
impl RiscvData {
    pub const fn new() -> Self {
        Self {
            privilege_mode: 1, // Supervisor mode
            sstatus: 0,
            mstatus: 0,
            satp: 0,
            paging_mode: 0, // Sv39 default
            root_page_table: 0,
            stvec: 0,
            mtvec: 0,
            sie: 0,
            sip: 0,
            plic_base: 0x0C00_0000, // QEMU virt default
            plic_num_sources: 1024,
            plic_num_contexts: 0,
            plic_threshold: 0,
            clint_base: 0x0200_0000, // QEMU virt default
            mtime_at_boot: 0,
            hart_id: 0,
            num_harts: 1,
            hart_mask: 1,
            sbi_spec_version: 0,
            sbi_impl_id: 0,
            sbi_impl_version: 0,
            sbi_extensions: 0,
            sbi_hsm_available: false,
            sbi_srst_available: false,
            sbi_pmu_available: false,
            timebase_frequency: 10_000_000, // 10 MHz default
            timer_ticks_at_boot: 0,
            misa: 0,
            vendor_id: 0,
            arch_id: 0,
            imp_id: 0,
            has_atomic: false,
            has_compressed: false,
            has_double_fp: false,
            has_single_fp: false,
            has_hypervisor: false,
            has_multiply: false,
            has_supervisor: false,
            has_user: false,
            has_vector: false,
            has_bitmanip: false,
            has_crypto: false,
            pmp_count: 0,
            pmp_granularity: 0,
            debug_trigger_count: 0,
            debug_available: false,
            uart_base: 0x1000_0000, // QEMU virt default
            uart_clock: 3_686_400,  // Standard 16550 clock
            use_sbi_console: true,  // Default to SBI console
            dtb_address: 0,
            dtb_size: 0,
        }
    }
}

// =============================================================================
// BOOT HOOKS
// =============================================================================

/// Boot hook callbacks for extensibility
///
/// Allows external code to hook into the boot process at various points.
pub trait BootHooks {
    /// Called before a stage begins
    fn pre_stage(&mut self, stage: BootStage, ctx: &mut BootContext) -> BootResult<()> {
        Ok(())
    }

    /// Called after a stage completes successfully
    fn post_stage(&mut self, stage: BootStage, ctx: &mut BootContext) -> BootResult<()> {
        Ok(())
    }

    /// Called when an error occurs
    fn on_error(&mut self, stage: BootStage, error: &BootError, ctx: &BootContext) {
        // Default: do nothing
    }

    /// Called before kernel handoff
    fn pre_handoff(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        Ok(())
    }

    /// Get custom kernel entry point (None = use default)
    fn custom_entry_point(&self, ctx: &BootContext) -> Option<u64> {
        None
    }
}

/// No-op boot hooks implementation
pub struct NoopHooks;

impl BootHooks for NoopHooks {}

// =============================================================================
// STAGE EXECUTOR TRAIT
// =============================================================================

/// Trait for boot stage executors
///
/// Each boot stage must implement this trait.
pub trait StageExecutor {
    /// Stage identifier
    const STAGE: BootStage;

    /// Execute the stage
    ///
    /// # Arguments
    /// * `ctx` - Boot context with accumulated state
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(BootError)` on failure
    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()>;

    /// Check if this stage should be skipped
    fn should_skip(&self, ctx: &BootContext) -> bool {
        false
    }

    /// Get stage name
    fn name(&self) -> &'static str {
        Self::STAGE.name()
    }

    /// Get stage description
    fn description(&self) -> &'static str {
        Self::STAGE.description()
    }
}
