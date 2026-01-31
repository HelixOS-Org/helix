//! # Boot Stages and Sequencing
//!
//! Implements the boot stage execution engine and individual stage executors.
//!
//! ## Execution Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                       BOOT SEQUENCE EXECUTION                                │
//! │                                                                              │
//! │   for each stage in [PreInit, CpuInit, MemoryInit, ...]:                    │
//! │       │                                                                      │
//! │       ├──▶ Call pre_stage() hook                                            │
//! │       │                                                                      │
//! │       ├──▶ Check should_skip()                                              │
//! │       │       │                                                              │
//! │       │       ├── Yes ──▶ Skip stage, continue                              │
//! │       │       │                                                              │
//! │       │       └── No ──▶ Execute stage                                      │
//! │       │                     │                                                │
//! │       │                     ├── Ok ──▶ Call post_stage() hook               │
//! │       │                     │              │                                 │
//! │       │                     │              └──▶ Mark complete, continue     │
//! │       │                     │                                                │
//! │       │                     └── Err ──▶ Call on_error() hook                │
//! │       │                                    │                                 │
//! │       │                                    ├── Recoverable? ──▶ Try next    │
//! │       │                                    │                                 │
//! │       │                                    └── Fatal ──▶ Boot panic         │
//! │       │                                                                      │
//! │       └──▶ Next stage                                                        │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::core::{
    BootContext, BootHooks, BootStage, BootState, CpuState, InterruptState, MemoryState, NoopHooks,
    PagingMode, SmpState, StageExecutor, TimerState,
};
use crate::error::{BootError, BootResult};
use crate::info::BootInfo;
use crate::{BootConfig, BootStatus, BOOT_STATE};

// =============================================================================
// STAGE RESULT
// =============================================================================

/// Result of a stage execution
#[derive(Debug)]
pub struct StageResult {
    /// Stage that was executed
    pub stage: BootStage,
    /// Whether the stage was skipped
    pub skipped: bool,
    /// Execution time in microseconds
    pub time_us: u64,
    /// Any warnings generated
    pub warnings: Vec<&'static str>,
}

impl StageResult {
    /// Create a successful result
    pub fn success(stage: BootStage, time_us: u64) -> Self {
        Self {
            stage,
            skipped: false,
            time_us,
            warnings: Vec::new(),
        }
    }

    /// Create a skipped result
    pub fn skipped(stage: BootStage) -> Self {
        Self {
            stage,
            skipped: true,
            time_us: 0,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: &'static str) -> Self {
        self.warnings.push(warning);
        self
    }
}

use alloc::vec::Vec;

// =============================================================================
// BOOT SEQUENCE
// =============================================================================

/// Main boot sequence executor
///
/// Orchestrates the entire early boot process by executing each stage
/// in order, handling errors, and managing hooks.
pub struct BootSequence {
    /// Configuration
    config: BootConfig,

    /// Boot hooks (for extensibility)
    hooks: Option<Box<dyn BootHooks>>,

    /// Stage execution results
    results: Vec<StageResult>,

    /// Whether sequence has been executed
    executed: bool,
}

impl BootSequence {
    /// Create a new boot sequence with configuration
    pub fn new(config: BootConfig) -> Self {
        Self {
            config,
            hooks: None,
            results: Vec::new(),
            executed: false,
        }
    }

    /// Set custom boot hooks
    pub fn with_hooks<H: BootHooks + 'static>(mut self, hooks: H) -> Self {
        self.hooks = Some(Box::new(hooks));
        self
    }

    /// Execute the complete boot sequence
    ///
    /// # Safety
    /// The boot_info pointer must be valid.
    pub unsafe fn execute(&mut self, boot_info: *const BootInfo) -> BootResult<u64> {
        if self.executed {
            return Err(BootError::AlreadyInitialized);
        }

        // Validate boot info
        if boot_info.is_null() {
            return Err(BootError::InvalidBootInfo);
        }
        (*boot_info).validate()?;

        // Create boot context
        let mut ctx = BootContext::new(self.config.clone(), boot_info);

        // Execute stages in order
        self.execute_stage::<PreInitStage>(&mut ctx)?;
        self.execute_stage::<CpuInitStage>(&mut ctx)?;
        self.execute_stage::<MemoryInitStage>(&mut ctx)?;
        self.execute_stage::<DriverInitStage>(&mut ctx)?;
        self.execute_stage::<InterruptInitStage>(&mut ctx)?;
        self.execute_stage::<TimerInitStage>(&mut ctx)?;

        // SMP is optional
        if self.config.smp_enabled {
            if let Err(e) = self.execute_stage::<SmpInitStage>(&mut ctx) {
                if !e.is_recoverable() {
                    return Err(e);
                }
                // Log warning but continue
                crate::boot_log("SMP initialization failed, continuing with single CPU");
            }
        }

        // Final handoff
        let entry = self.execute_handoff(&mut ctx)?;

        self.executed = true;

        Ok(entry)
    }

    /// Execute a single stage
    fn execute_stage<S: StageExecutor + Default>(
        &mut self,
        ctx: &mut BootContext,
    ) -> BootResult<()> {
        let mut stage = S::default();
        let stage_id = S::STAGE;

        // Update global state
        {
            let mut state = BOOT_STATE.lock();
            state.set_stage(stage_id);
        }

        // Pre-stage hook
        if let Some(ref mut hooks) = self.hooks {
            hooks.pre_stage(stage_id, ctx)?;
        }

        // Check if should skip
        if stage.should_skip(ctx) {
            self.results.push(StageResult::skipped(stage_id));
            return Ok(());
        }

        // Log stage start
        crate::boot_log(&alloc::format!("Starting stage: {}", stage.name()));

        // Get start time (architecture-specific)
        let start_time = crate::arch::read_timestamp();

        // Execute stage
        let result = stage.execute(ctx);

        // Calculate duration
        let end_time = crate::arch::read_timestamp();
        let duration_us = (end_time - start_time) / 1000; // Assuming nanosecond resolution

        match result {
            Ok(()) => {
                // Mark complete
                {
                    let mut state = BOOT_STATE.lock();
                    state.complete_stage(stage_id, duration_us);
                }

                // Post-stage hook
                if let Some(ref mut hooks) = self.hooks {
                    hooks.post_stage(stage_id, ctx)?;
                }

                self.results
                    .push(StageResult::success(stage_id, duration_us));

                crate::boot_log(&alloc::format!(
                    "Stage {} complete ({} µs)",
                    stage.name(),
                    duration_us
                ));

                Ok(())
            },
            Err(e) => {
                // Error hook
                if let Some(ref mut hooks) = self.hooks {
                    hooks.on_error(stage_id, &e, ctx);
                }

                {
                    let mut state = BOOT_STATE.lock();
                    state.set_error("Stage execution failed");
                }

                Err(e)
            },
        }
    }

    /// Execute handoff stage and return kernel entry point
    fn execute_handoff(&mut self, ctx: &mut BootContext) -> BootResult<u64> {
        let mut stage = HandoffStage::default();

        // Update state
        {
            let mut state = BOOT_STATE.lock();
            state.set_stage(BootStage::Handoff);
        }

        // Pre-handoff hook
        if let Some(ref mut hooks) = self.hooks {
            hooks.pre_handoff(ctx)?;
        }

        crate::boot_log("Preparing kernel handoff...");

        // Execute handoff preparation
        stage.execute(ctx)?;

        // Get entry point
        let entry = if let Some(ref hooks) = self.hooks {
            hooks.custom_entry_point(ctx)
        } else {
            None
        }
        .unwrap_or(stage.kernel_entry);

        if entry == 0 {
            return Err(BootError::KernelEntryNotFound);
        }

        // Mark complete
        {
            let mut state = BOOT_STATE.lock();
            state.complete_stage(BootStage::Handoff, 0);
        }

        crate::boot_log(&alloc::format!("Kernel entry point: {:#x}", entry));

        Ok(entry)
    }

    /// Get stage results
    pub fn results(&self) -> &[StageResult] {
        &self.results
    }

    /// Get total boot time in microseconds
    pub fn total_time_us(&self) -> u64 {
        self.results.iter().map(|r| r.time_us).sum()
    }
}

// =============================================================================
// STAGE 0: PRE-INIT
// =============================================================================

/// Pre-initialization stage
///
/// Minimal setup required before anything else can run:
/// - Validate we're in the correct mode/privilege level
/// - Set up minimal stack (if needed)
/// - Initialize serial output (for debugging)
/// - Validate boot info structure
#[derive(Default)]
pub struct PreInitStage;

impl StageExecutor for PreInitStage {
    const STAGE: BootStage = BootStage::PreInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        // Architecture-specific pre-init
        unsafe {
            crate::arch::pre_init(ctx)?;
        }

        // Initialize early serial if configured
        if ctx.config.serial_enabled {
            unsafe {
                crate::arch::init_serial(&ctx.config.serial_port)?;
            }
        }

        // Log that we're alive
        crate::boot_log("Helix OS Early Boot - Pre-Init");
        crate::boot_log(&alloc::format!(
            "Architecture: {}",
            crate::Architecture::current().name()
        ));

        // Validate boot info
        let boot_info = unsafe { ctx.boot_info() };
        boot_info.validate()?;

        crate::boot_log(&alloc::format!(
            "Boot protocol: {:?}",
            boot_info.header.protocol
        ));

        Ok(())
    }
}

// =============================================================================
// STAGE 1: CPU INIT
// =============================================================================

/// CPU initialization stage
///
/// Full CPU initialization:
/// - Detect CPU features and capabilities
/// - Configure caches
/// - Set up FPU/SIMD state
/// - Architecture-specific setup (GDT on x86, EL transition on ARM, etc.)
#[derive(Default)]
pub struct CpuInitStage;

impl StageExecutor for CpuInitStage {
    const STAGE: BootStage = BootStage::CpuInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        crate::boot_log("Initializing CPU...");

        // Detect CPU features
        unsafe {
            crate::arch::detect_cpu_features(&mut ctx.cpu_state)?;
        }

        // Log detected features
        crate::boot_log(&alloc::format!(
            "CPU: {} cores, {} threads",
            ctx.cpu_state.core_count,
            ctx.cpu_state.logical_count
        ));

        // Initialize FPU/SIMD
        unsafe {
            crate::arch::init_fpu()?;
        }

        // Architecture-specific CPU setup
        unsafe {
            crate::arch::cpu_init(ctx)?;
        }

        Ok(())
    }
}

// =============================================================================
// STAGE 2: MEMORY INIT
// =============================================================================

/// Memory initialization stage
///
/// Set up the memory subsystem:
/// - Process physical memory map
/// - Allocate and set up page tables
/// - Map kernel to higher half
/// - Set up HHDM (Higher Half Direct Map)
/// - Initialize early heap
#[derive(Default)]
pub struct MemoryInitStage;

impl StageExecutor for MemoryInitStage {
    const STAGE: BootStage = BootStage::MemoryInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        crate::boot_log("Initializing memory...");

        let boot_info = unsafe { ctx.boot_info() };

        // Process memory map
        let memory_map = unsafe { boot_info.memory.memory_map() };
        if memory_map.is_empty() {
            return Err(BootError::EmptyMemoryMap);
        }

        // Calculate totals
        let mut total_memory = 0u64;
        let mut usable_memory = 0u64;

        for entry in memory_map {
            total_memory += entry.length;
            if entry.is_usable() {
                usable_memory += entry.length;
            }
        }

        ctx.memory_state.total_physical = total_memory;
        ctx.memory_state.usable_physical = usable_memory;
        ctx.memory_state.hhdm_offset = boot_info.memory.hhdm_offset;
        ctx.memory_state.kernel_phys_start = boot_info.memory.kernel_phys_start;
        ctx.memory_state.kernel_phys_end = boot_info.memory.kernel_phys_end;
        ctx.memory_state.kernel_virt_base = boot_info.memory.kernel_virt_base;

        crate::boot_log(&alloc::format!(
            "Physical memory: {} MB total, {} MB usable",
            total_memory / (1024 * 1024),
            usable_memory / (1024 * 1024)
        ));

        // Set up page tables
        unsafe {
            crate::arch::setup_page_tables(ctx)?;
        }

        crate::boot_log(&alloc::format!(
            "Paging mode: {:?}",
            ctx.memory_state.paging_mode
        ));

        // Initialize early heap
        unsafe {
            crate::memory::init_early_heap(ctx)?;
        }

        crate::boot_log(&alloc::format!(
            "Early heap: {} KB",
            ctx.memory_state.early_heap_size / 1024
        ));

        Ok(())
    }
}

// =============================================================================
// STAGE 3: DRIVER INIT
// =============================================================================

/// Early driver initialization stage
///
/// Initialize essential drivers:
/// - Console (serial and/or framebuffer)
/// - Any platform-specific early drivers
/// - Debug facilities
#[derive(Default)]
pub struct DriverInitStage;

impl StageExecutor for DriverInitStage {
    const STAGE: BootStage = BootStage::DriverInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        crate::boot_log("Initializing early drivers...");

        let boot_info = unsafe { ctx.boot_info() };

        // Initialize framebuffer console if available
        if ctx.config.framebuffer_enabled {
            if let Some(fb) = &boot_info.framebuffer {
                unsafe {
                    if let Err(e) = crate::drivers::init_framebuffer(fb, ctx) {
                        crate::boot_log(&alloc::format!("Framebuffer init warning: {:?}", e));
                        // Non-fatal, continue
                    } else {
                        crate::boot_log(&alloc::format!(
                            "Framebuffer: {}x{} @ {} bpp",
                            fb.width,
                            fb.height,
                            fb.bpp
                        ));
                    }
                }
            }
        }

        // Initialize other platform drivers
        unsafe {
            crate::arch::init_platform_drivers(ctx)?;
        }

        Ok(())
    }

    fn should_skip(&self, _ctx: &BootContext) -> bool {
        // Never skip driver init, but individual drivers can fail
        false
    }
}

// =============================================================================
// STAGE 4: INTERRUPT INIT
// =============================================================================

/// Interrupt initialization stage
///
/// Set up interrupt handling:
/// - Configure IDT/GDT (x86) or exception vectors (ARM/RISC-V)
/// - Initialize interrupt controller (APIC/GIC/PLIC)
/// - Register exception handlers
#[derive(Default)]
pub struct InterruptInitStage;

impl StageExecutor for InterruptInitStage {
    const STAGE: BootStage = BootStage::InterruptInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        crate::boot_log("Initializing interrupts...");

        // Architecture-specific interrupt setup
        unsafe {
            crate::arch::init_interrupts(ctx)?;
        }

        crate::boot_log(&alloc::format!(
            "Interrupt controller: {:?}",
            ctx.interrupt_state.controller_type
        ));

        crate::boot_log(&alloc::format!(
            "IRQ count: {}",
            ctx.interrupt_state.irq_count
        ));

        Ok(())
    }
}

// =============================================================================
// STAGE 5: TIMER INIT
// =============================================================================

/// Timer initialization stage
///
/// Set up system timers:
/// - Detect available timers
/// - Calibrate timer frequency
/// - Configure system tick
#[derive(Default)]
pub struct TimerInitStage;

impl StageExecutor for TimerInitStage {
    const STAGE: BootStage = BootStage::TimerInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        crate::boot_log("Initializing timers...");

        // Architecture-specific timer setup
        unsafe {
            crate::arch::init_timers(ctx)?;
        }

        crate::boot_log(&alloc::format!(
            "Timer: {:?} @ {} Hz",
            ctx.timer_state.primary_timer,
            ctx.timer_state.frequency
        ));

        if ctx.timer_state.calibrated {
            crate::boot_log("Timer calibration: OK");
        }

        Ok(())
    }

    fn should_skip(&self, _ctx: &BootContext) -> bool {
        // Timer init is important but not strictly required
        false
    }
}

// =============================================================================
// STAGE 6: SMP INIT
// =============================================================================

/// SMP initialization stage
///
/// Start secondary CPUs:
/// - Detect CPU topology
/// - Allocate per-CPU data
/// - Start application processors
#[derive(Default)]
pub struct SmpInitStage;

impl StageExecutor for SmpInitStage {
    const STAGE: BootStage = BootStage::SmpInit;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        let boot_info = unsafe { ctx.boot_info() };

        // Get CPU count from boot info
        let cpu_count = boot_info.smp.cpu_count as usize;

        if cpu_count <= 1 {
            crate::boot_log("Single CPU system, skipping SMP init");
            ctx.smp_state.cpu_count = 1;
            ctx.smp_state.cpus_online = 1;
            return Ok(());
        }

        crate::boot_log(&alloc::format!("Initializing SMP ({} CPUs)...", cpu_count));

        // Limit to configured max
        let target_cpus = cpu_count.min(ctx.config.max_cpus);

        if target_cpus != cpu_count {
            crate::boot_log(&alloc::format!(
                "Limiting to {} CPUs (config max)",
                target_cpus
            ));
        }

        ctx.smp_state.cpu_count = target_cpus;
        ctx.smp_state.bsp_id = boot_info.smp.bsp_id;

        // Architecture-specific SMP startup
        unsafe {
            crate::arch::init_smp(ctx)?;
        }

        crate::boot_log(&alloc::format!(
            "SMP: {}/{} CPUs online",
            ctx.smp_state.cpus_online,
            ctx.smp_state.cpu_count
        ));

        Ok(())
    }

    fn should_skip(&self, ctx: &BootContext) -> bool {
        !ctx.config.smp_enabled
    }
}

// =============================================================================
// STAGE 7: HANDOFF
// =============================================================================

/// Handoff stage
///
/// Prepare for kernel entry:
/// - Apply KASLR (if enabled)
/// - Finalize memory layout
/// - Prepare kernel arguments
/// - Transfer control to main kernel
#[derive(Default)]
pub struct HandoffStage {
    /// Kernel entry point
    pub kernel_entry: u64,
}

impl StageExecutor for HandoffStage {
    const STAGE: BootStage = BootStage::Handoff;

    fn execute(&mut self, ctx: &mut BootContext) -> BootResult<()> {
        let boot_info = unsafe { ctx.boot_info() };

        // Apply KASLR if enabled
        if ctx.config.kaslr_enabled {
            crate::boot_log("Applying KASLR...");
            unsafe {
                crate::arch::apply_kaslr(ctx)?;
            }
        }

        // Get kernel entry point
        // This depends on the boot protocol and kernel format
        self.kernel_entry = if ctx.config.kernel_load_addr != 0 {
            ctx.config.kernel_load_addr
        } else {
            // Default: kernel virtual base (assuming entry at base)
            boot_info.memory.kernel_virt_base
        };

        // Final architecture-specific preparation
        unsafe {
            crate::arch::prepare_handoff(ctx)?;
        }

        // Print boot summary
        self.print_boot_summary(ctx);

        Ok(())
    }
}

impl HandoffStage {
    fn print_boot_summary(&self, ctx: &BootContext) {
        crate::boot_log("═══════════════════════════════════════════════════════");
        crate::boot_log("                 BOOT SUMMARY                          ");
        crate::boot_log("═══════════════════════════════════════════════════════");
        crate::boot_log(&alloc::format!(
            "Architecture:     {}",
            crate::Architecture::current().name()
        ));
        crate::boot_log(&alloc::format!(
            "CPUs:             {}/{}",
            ctx.smp_state.cpus_online,
            ctx.smp_state.cpu_count
        ));
        crate::boot_log(&alloc::format!(
            "Memory:           {} MB",
            ctx.memory_state.total_physical / (1024 * 1024)
        ));
        crate::boot_log(&alloc::format!(
            "Paging:           {:?}",
            ctx.memory_state.paging_mode
        ));
        crate::boot_log(&alloc::format!(
            "Timer:            {:?}",
            ctx.timer_state.primary_timer
        ));
        crate::boot_log(&alloc::format!(
            "Kernel Entry:     {:#x}",
            self.kernel_entry
        ));
        crate::boot_log("═══════════════════════════════════════════════════════");
        crate::boot_log("Transferring to main kernel...");
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Run a minimal boot sequence (for testing)
pub fn minimal_boot(boot_info: *const BootInfo) -> BootResult<()> {
    let config = BootConfig::minimal();
    let mut sequence = BootSequence::new(config);

    unsafe {
        let _ = sequence.execute(boot_info)?;
    }

    Ok(())
}

/// Run boot sequence with custom configuration
pub fn boot_with_config(boot_info: *const BootInfo, config: BootConfig) -> BootResult<u64> {
    let mut sequence = BootSequence::new(config);

    unsafe { sequence.execute(boot_info) }
}
