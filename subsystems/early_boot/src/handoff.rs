//! # Helix OS Early Boot - Boot Handoff
//!
//! This module handles the final transition from early boot to the main kernel.
//! It includes KASLR support, final memory layout preparation, and kernel entry.
//!
//! ## Responsibilities
//!
//! - **KASLR**: Kernel Address Space Layout Randomization for security
//! - **Relocation**: Apply dynamic relocations after randomization
//! - **Memory Finalization**: Set up final memory layout for kernel
//! - **State Transfer**: Package boot state for kernel consumption
//! - **Kernel Entry**: Transfer control to main kernel entry point
//!
//! ## KASLR Implementation
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         KASLR LAYOUT                                     │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  Virtual Address Space (48-bit)                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │ 0x0000_0000_0000_0000 ─────────────────────────────────────────│     │
//! │  │                     User Space (128 TB)                         │     │
//! │  │ 0x0000_7FFF_FFFF_FFFF ─────────────────────────────────────────│     │
//! │  │                     Non-canonical hole                          │     │
//! │  │ 0xFFFF_8000_0000_0000 ─────────────────────────────────────────│     │
//! │  │                     HHDM (Higher Half Direct Map)               │     │
//! │  │                     Kernel Physical Memory Map                  │     │
//! │  │ 0xFFFF_FFFF_8000_0000 ─────────────────────────────────────────│     │
//! │  │                     Kernel Base (randomized)                    │     │
//! │  │                     ┌─────────────────────┐                     │     │
//! │  │                     │ .text (code)        │ + KASLR offset      │     │
//! │  │                     │ .rodata             │                     │     │
//! │  │                     │ .data               │                     │     │
//! │  │                     │ .bss                │                     │     │
//! │  │                     └─────────────────────┘                     │     │
//! │  │ 0xFFFF_FFFF_FFFF_FFFF ─────────────────────────────────────────│     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

use crate::core::{BootContext, BootState};
use crate::error::{BootError, BootResult};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Default kernel virtual base (before KASLR)
pub const KERNEL_VIRT_BASE: u64 = 0xFFFF_FFFF_8000_0000;

/// HHDM (Higher Half Direct Map) base
pub const HHDM_BASE: u64 = 0xFFFF_8000_0000_0000;

/// KASLR alignment (2MB for huge page support)
pub const KASLR_ALIGNMENT: u64 = 2 * 1024 * 1024;

/// KASLR range size (1GB of randomization space)
pub const KASLR_RANGE: u64 = 1024 * 1024 * 1024;

/// Kernel stack size (64KB per CPU)
pub const KERNEL_STACK_SIZE: usize = 64 * 1024;

/// Guard page size
pub const GUARD_PAGE_SIZE: usize = 4096;

// =============================================================================
// KASLR CONFIGURATION
// =============================================================================

/// KASLR configuration
#[derive(Debug, Clone, Copy)]
pub struct KaslrConfig {
    /// Enable KASLR
    pub enabled: bool,

    /// Randomization range (bytes)
    pub range: u64,

    /// Alignment requirement (bytes)
    pub alignment: u64,

    /// Use hardware RNG if available
    pub use_hardware_rng: bool,

    /// Fallback seed (if no RNG available)
    pub fallback_seed: u64,
}

impl KaslrConfig {
    /// Create default KASLR configuration
    pub const fn new() -> Self {
        Self {
            enabled: true,
            range: KASLR_RANGE,
            alignment: KASLR_ALIGNMENT,
            use_hardware_rng: true,
            fallback_seed: 0xDEAD_BEEF_CAFE_BABE,
        }
    }

    /// Disable KASLR
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            range: 0,
            alignment: KASLR_ALIGNMENT,
            use_hardware_rng: false,
            fallback_seed: 0,
        }
    }
}

impl Default for KaslrConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// RANDOM NUMBER GENERATION
// =============================================================================

/// Simple PRNG for KASLR (xorshift64)
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Create new PRNG with seed
    pub const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Generate next random number
    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate random number in range [0, max)
    pub fn next_range(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next() % max
    }
}

/// Get random seed from hardware RNG
fn get_hardware_random() -> Option<u64> {
    #[cfg(target_arch = "x86_64")]
    {
        // Try RDRAND
        let mut value: u64;
        let success: u8;
        unsafe {
            core::arch::asm!(
                "rdrand {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nostack, nomem)
            );
        }
        if success != 0 {
            return Some(value);
        }

        // Try RDSEED as fallback
        unsafe {
            core::arch::asm!(
                "rdseed {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nostack, nomem)
            );
        }
        if success != 0 {
            return Some(value);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // Read RNDR (ARMv8.5+)
        let value: u64;
        let success: u64;
        unsafe {
            core::arch::asm!(
                "mrs {0}, RNDR",
                "cset {1}, ne",
                out(reg) value,
                out(reg) success,
                options(nostack, nomem)
            );
        }
        if success != 0 {
            return Some(value);
        }
    }

    #[cfg(target_arch = "riscv64")]
    {
        // RISC-V doesn't have a standard hardware RNG instruction
        // Could use SBI or Zkr extension if available
    }

    None
}

/// Get entropy from various sources for seeding
fn gather_entropy() -> u64 {
    let mut entropy: u64 = 0;

    // Try hardware RNG first
    if let Some(hw_random) = get_hardware_random() {
        entropy ^= hw_random;
    }

    // Mix in timestamp
    #[cfg(target_arch = "x86_64")]
    {
        let tsc: u64;
        unsafe {
            core::arch::asm!(
                "rdtsc",
                "shl rdx, 32",
                "or rax, rdx",
                out("rax") tsc,
                out("rdx") _,
                options(nostack, nomem)
            );
        }
        entropy ^= tsc;
    }

    #[cfg(target_arch = "aarch64")]
    {
        let cnt: u64;
        unsafe {
            core::arch::asm!(
                "mrs {0}, CNTPCT_EL0",
                out(reg) cnt,
                options(nostack, nomem)
            );
        }
        entropy ^= cnt;
    }

    #[cfg(target_arch = "riscv64")]
    {
        let time: u64;
        unsafe {
            core::arch::asm!(
                "rdtime {0}",
                out(reg) time,
                options(nostack, nomem)
            );
        }
        entropy ^= time;
    }

    // Mix in stack pointer for additional randomness
    let sp: u64;
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp, options(nostack, nomem));
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("mov {}, sp", out(reg) sp, options(nostack, nomem));
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp, options(nostack, nomem));
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    let sp: u64 = 0;

    entropy ^= sp.wrapping_mul(0x5851F42D4C957F2D);

    entropy
}

// =============================================================================
// KASLR IMPLEMENTATION
// =============================================================================

/// KASLR state
pub struct Kaslr {
    /// Configuration
    config: KaslrConfig,

    /// PRNG
    rng: Xorshift64,

    /// Calculated offset
    offset: u64,

    /// Virtual base after KASLR
    virt_base: u64,
}

impl Kaslr {
    /// Create new KASLR instance
    pub fn new(config: KaslrConfig) -> Self {
        let seed = if config.use_hardware_rng {
            gather_entropy()
        } else {
            config.fallback_seed
        };

        Self {
            config,
            rng: Xorshift64::new(seed),
            offset: 0,
            virt_base: KERNEL_VIRT_BASE,
        }
    }

    /// Calculate KASLR offset
    pub fn calculate_offset(&mut self) -> u64 {
        if !self.config.enabled {
            self.offset = 0;
            return 0;
        }

        // Generate random offset within range
        let max_offset = self.config.range / self.config.alignment;
        let random_slots = self.rng.next_range(max_offset);

        self.offset = random_slots * self.config.alignment;
        self.virt_base = KERNEL_VIRT_BASE.wrapping_add(self.offset);

        self.offset
    }

    /// Get calculated offset
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get virtual base after KASLR
    pub fn virt_base(&self) -> u64 {
        self.virt_base
    }

    /// Apply KASLR to an address
    pub fn apply(&self, addr: u64) -> u64 {
        if addr >= KERNEL_VIRT_BASE {
            addr.wrapping_add(self.offset)
        } else {
            addr
        }
    }

    /// Check if KASLR is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.offset != 0
    }
}

// =============================================================================
// RELOCATION SUPPORT
// =============================================================================

/// ELF relocation types (subset)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RelocationType {
    /// No relocation
    None            = 0,
    /// x86_64: R_X86_64_RELATIVE
    X86_64Relative  = 8,
    /// AArch64: R_AARCH64_RELATIVE
    AArch64Relative = 1027,
    /// RISC-V: R_RISCV_RELATIVE
    RiscvRelative   = 3,
}

/// Relocation entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RelocEntry {
    /// Offset to relocate
    pub offset: u64,
    /// Relocation type
    pub info: u64,
    /// Addend
    pub addend: i64,
}

impl RelocEntry {
    /// Get relocation type
    pub fn rel_type(&self) -> u32 {
        (self.info & 0xFFFF_FFFF) as u32
    }

    /// Get symbol index
    pub fn symbol(&self) -> u32 {
        (self.info >> 32) as u32
    }
}

/// Apply relocations for KASLR
///
/// # Safety
/// The kernel image must be writable and relocation entries must be valid.
pub unsafe fn apply_relocations(
    kernel_base: u64,
    rela_start: *const RelocEntry,
    rela_count: usize,
    kaslr_offset: u64,
) -> BootResult<usize> {
    let mut applied = 0;

    for i in 0..rela_count {
        let rela = &*rela_start.add(i);
        let rel_type = rela.rel_type();

        // Handle RELATIVE relocations
        let is_relative = {
            #[cfg(target_arch = "x86_64")]
            {
                rel_type == RelocationType::X86_64Relative as u32
            }
            #[cfg(target_arch = "aarch64")]
            {
                rel_type == RelocationType::AArch64Relative as u32
            }
            #[cfg(target_arch = "riscv64")]
            {
                rel_type == RelocationType::RiscvRelative as u32
            }
            #[cfg(not(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64"
            )))]
            {
                false
            }
        };

        if is_relative {
            let target_addr = kernel_base.wrapping_add(rela.offset);
            let target = target_addr as *mut u64;

            // Apply: *target = base + addend + kaslr_offset
            let old_value = core::ptr::read_volatile(target);
            let new_value = old_value.wrapping_add(kaslr_offset);
            core::ptr::write_volatile(target, new_value);

            applied += 1;
        }
    }

    Ok(applied)
}

// =============================================================================
// KERNEL HANDOFF STATE
// =============================================================================

/// State passed to kernel at handoff
#[derive(Debug)]
#[repr(C)]
pub struct HandoffState {
    /// Magic value for validation
    pub magic: u64,

    /// Boot state
    pub boot_state: *const BootState,

    /// Physical memory start
    pub phys_base: u64,

    /// Virtual memory base (after KASLR)
    pub virt_base: u64,

    /// HHDM offset
    pub hhdm_offset: u64,

    /// KASLR offset
    pub kaslr_offset: u64,

    /// Total physical memory
    pub total_memory: u64,

    /// Number of CPUs
    pub cpu_count: u32,

    /// BSP (Boot Strap Processor) ID
    pub bsp_id: u32,

    /// ACPI RSDP address
    pub acpi_rsdp: u64,

    /// Framebuffer address
    pub framebuffer_addr: u64,

    /// Framebuffer width
    pub framebuffer_width: u32,

    /// Framebuffer height
    pub framebuffer_height: u32,

    /// Framebuffer pitch
    pub framebuffer_pitch: u32,

    /// Initial page table root
    pub page_table_root: u64,

    /// Kernel stack top (for BSP)
    pub kernel_stack_top: u64,

    /// Reserved for architecture-specific data
    pub arch_data: [u64; 16],
}

/// Handoff state magic: "HLXHAND\0"
pub const HANDOFF_MAGIC: u64 = 0x444E4148584C4800;

impl HandoffState {
    /// Create new handoff state
    pub const fn new() -> Self {
        Self {
            magic: HANDOFF_MAGIC,
            boot_state: core::ptr::null(),
            phys_base: 0,
            virt_base: KERNEL_VIRT_BASE,
            hhdm_offset: HHDM_BASE,
            kaslr_offset: 0,
            total_memory: 0,
            cpu_count: 1,
            bsp_id: 0,
            acpi_rsdp: 0,
            framebuffer_addr: 0,
            framebuffer_width: 0,
            framebuffer_height: 0,
            framebuffer_pitch: 0,
            page_table_root: 0,
            kernel_stack_top: 0,
            arch_data: [0; 16],
        }
    }

    /// Validate handoff state
    pub fn validate(&self) -> bool {
        self.magic == HANDOFF_MAGIC
    }

    /// Set architecture-specific data
    pub fn set_arch_data(&mut self, index: usize, value: u64) {
        if index < 16 {
            self.arch_data[index] = value;
        }
    }

    /// Get architecture-specific data
    pub fn get_arch_data(&self, index: usize) -> u64 {
        if index < 16 {
            self.arch_data[index]
        } else {
            0
        }
    }
}

impl Default for HandoffState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// KERNEL ENTRY
// =============================================================================

/// Kernel entry point type
pub type KernelEntryFn = unsafe extern "C" fn(*const HandoffState) -> !;

/// Prepare for kernel handoff
///
/// This function prepares the final state and transitions to the kernel.
pub struct BootHandoff {
    /// KASLR state
    kaslr: Kaslr,

    /// Handoff state
    state: HandoffState,

    /// Kernel entry point
    entry_point: Option<u64>,
}

impl BootHandoff {
    /// Create new boot handoff
    pub fn new(kaslr_config: KaslrConfig) -> Self {
        Self {
            kaslr: Kaslr::new(kaslr_config),
            state: HandoffState::new(),
            entry_point: None,
        }
    }

    /// Initialize KASLR
    pub fn init_kaslr(&mut self) -> u64 {
        let offset = self.kaslr.calculate_offset();
        self.state.kaslr_offset = offset;
        self.state.virt_base = self.kaslr.virt_base();
        offset
    }

    /// Set kernel entry point
    pub fn set_entry_point(&mut self, entry: u64) {
        self.entry_point = Some(self.kaslr.apply(entry));
    }

    /// Prepare handoff state from boot context
    pub fn prepare_state(&mut self, ctx: &BootContext) {
        // Copy memory state
        self.state.phys_base = ctx.memory_state.kernel_phys_start;
        self.state.hhdm_offset = ctx.memory_state.hhdm_offset;
        self.state.total_memory = ctx.memory_state.total_physical;
        self.state.page_table_root = ctx.memory_state.page_table_root;

        // Copy SMP state
        self.state.cpu_count = ctx.smp_state.cpu_count as u32;
        self.state.bsp_id = ctx.smp_state.bsp_id;

        // Copy architecture-specific data
        #[cfg(target_arch = "x86_64")]
        {
            self.state.arch_data[0] = ctx.arch_data.x86.gdt_base;
            self.state.arch_data[1] = ctx.arch_data.x86.idt_base;
            self.state.arch_data[2] = ctx.arch_data.x86.tss_base;
            self.state.arch_data[3] = ctx.arch_data.x86.lapic_base;
            self.state.arch_data[4] = ctx.arch_data.x86.cr3;
        }

        #[cfg(target_arch = "aarch64")]
        {
            self.state.arch_data[0] = ctx.arch_data.arm.vbar_el1;
            self.state.arch_data[1] = ctx.arch_data.arm.ttbr0_el1;
            self.state.arch_data[2] = ctx.arch_data.arm.ttbr1_el1;
            self.state.arch_data[3] = ctx.arch_data.arm.gicd_base;
            self.state.arch_data[4] = ctx.arch_data.arm.timer_frequency;
        }

        #[cfg(target_arch = "riscv64")]
        {
            self.state.arch_data[0] = ctx.arch_data.riscv.satp;
            self.state.arch_data[1] = ctx.arch_data.riscv.stvec;
            self.state.arch_data[2] = ctx.arch_data.riscv.plic_base;
            self.state.arch_data[3] = ctx.arch_data.riscv.clint_base;
            self.state.arch_data[4] = ctx.arch_data.riscv.hart_id;
            self.state.arch_data[5] = ctx.arch_data.riscv.sbi_spec_version as u64;
        }
    }

    /// Set kernel stack
    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.state.kernel_stack_top = stack_top;
    }

    /// Get handoff state reference
    pub fn state(&self) -> &HandoffState {
        &self.state
    }

    /// Get mutable handoff state reference
    pub fn state_mut(&mut self) -> &mut HandoffState {
        &mut self.state
    }

    /// Transfer control to kernel
    ///
    /// # Safety
    /// This function never returns. The kernel entry point must be valid.
    pub unsafe fn jump_to_kernel(&self) -> ! {
        let entry = self.entry_point.expect("Kernel entry point not set");
        let state_ptr = &self.state as *const HandoffState;

        // Ensure everything is flushed
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Call architecture-specific kernel entry
        jump_to_kernel_impl(entry, state_ptr)
    }
}

/// Architecture-specific kernel jump
#[cfg(target_arch = "x86_64")]
unsafe fn jump_to_kernel_impl(entry: u64, state: *const HandoffState) -> ! {
    // Set up the stack and jump to kernel
    // The kernel entry expects: RDI = handoff state pointer
    core::arch::asm!(
        // Load new stack (could be in state)
        // "mov rsp, [{}]", // Optional: new stack

        // Pass handoff state as first argument
        "mov rdi, {state}",

        // Clear other registers for security
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        "xor rbp, rbp",

        // Jump to kernel (never returns)
        "jmp {entry}",

        entry = in(reg) entry,
        state = in(reg) state,
        options(noreturn)
    );
}

#[cfg(target_arch = "aarch64")]
unsafe fn jump_to_kernel_impl(entry: u64, state: *const HandoffState) -> ! {
    core::arch::asm!(
        // Pass handoff state as first argument (X0)
        "mov x0, {state}",

        // Clear other registers
        "mov x1, xzr",
        "mov x2, xzr",
        "mov x3, xzr",
        "mov x4, xzr",
        "mov x5, xzr",
        "mov x6, xzr",
        "mov x7, xzr",

        // Branch to kernel
        "br {entry}",

        entry = in(reg) entry,
        state = in(reg) state,
        options(noreturn)
    );
}

#[cfg(target_arch = "riscv64")]
unsafe fn jump_to_kernel_impl(entry: u64, state: *const HandoffState) -> ! {
    core::arch::asm!(
        // Pass handoff state as first argument (a0)
        "mv a0, {state}",

        // Clear other registers
        "li a1, 0",
        "li a2, 0",
        "li a3, 0",
        "li a4, 0",
        "li a5, 0",
        "li a6, 0",
        "li a7, 0",

        // Jump to kernel
        "jr {entry}",

        entry = in(reg) entry,
        state = in(reg) state,
        options(noreturn)
    );
}

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
unsafe fn jump_to_kernel_impl(_entry: u64, _state: *const HandoffState) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

// =============================================================================
// STACK SETUP
// =============================================================================

/// Kernel stack allocation
pub struct KernelStack {
    /// Stack base (low address)
    pub base: u64,
    /// Stack top (high address, where SP starts)
    pub top: u64,
    /// Stack size
    pub size: usize,
    /// Guard page address (if any)
    pub guard_page: u64,
}

impl KernelStack {
    /// Calculate stack layout
    pub const fn layout(base: u64, size: usize, with_guard: bool) -> Self {
        let guard_page = if with_guard { base } else { 0 };
        let actual_base = if with_guard {
            base + GUARD_PAGE_SIZE as u64
        } else {
            base
        };
        let actual_size = if with_guard {
            size - GUARD_PAGE_SIZE
        } else {
            size
        };

        Self {
            base: actual_base,
            top: actual_base + actual_size as u64,
            size: actual_size,
            guard_page,
        }
    }

    /// Check if address is in stack range
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.top
    }

    /// Check if address is in guard page
    pub fn is_guard_page(&self, addr: u64) -> bool {
        self.guard_page != 0 && addr >= self.guard_page && addr < self.base
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xorshift64() {
        let mut rng = Xorshift64::new(12345);

        let v1 = rng.next();
        let v2 = rng.next();

        assert_ne!(v1, v2);
        assert_ne!(v1, 0);
        assert_ne!(v2, 0);
    }

    #[test]
    fn test_xorshift64_range() {
        let mut rng = Xorshift64::new(12345);

        for _ in 0..100 {
            let v = rng.next_range(100);
            assert!(v < 100);
        }
    }

    #[test]
    fn test_kaslr_offset() {
        let config = KaslrConfig::new();
        let mut kaslr = Kaslr::new(config);

        let offset = kaslr.calculate_offset();

        // Offset should be aligned
        assert_eq!(offset % KASLR_ALIGNMENT, 0);

        // Offset should be within range
        assert!(offset < KASLR_RANGE);
    }

    #[test]
    fn test_kaslr_disabled() {
        let config = KaslrConfig::disabled();
        let mut kaslr = Kaslr::new(config);

        let offset = kaslr.calculate_offset();

        assert_eq!(offset, 0);
        assert!(!kaslr.is_enabled());
    }

    #[test]
    fn test_kaslr_apply() {
        let config = KaslrConfig::new();
        let mut kaslr = Kaslr::new(config);
        kaslr.calculate_offset();

        let addr = KERNEL_VIRT_BASE + 0x1000;
        let applied = kaslr.apply(addr);

        assert_eq!(applied, addr + kaslr.offset());
    }

    #[test]
    fn test_handoff_state_magic() {
        let state = HandoffState::new();
        assert!(state.validate());
    }

    #[test]
    fn test_kernel_stack_layout() {
        let stack = KernelStack::layout(0x1000, 0x10000, true);

        assert_eq!(stack.guard_page, 0x1000);
        assert_eq!(stack.base, 0x1000 + GUARD_PAGE_SIZE as u64);
        assert_eq!(stack.size, 0x10000 - GUARD_PAGE_SIZE);
        assert!(stack.contains(stack.base + 100));
        assert!(stack.is_guard_page(0x1100));
    }

    #[test]
    fn test_relocation_entry() {
        let entry = RelocEntry {
            offset: 0x1000,
            info: 0x0000_0001_0000_0008, // symbol=1, type=8 (R_X86_64_RELATIVE)
            addend: 0x100,
        };

        assert_eq!(entry.rel_type(), 8);
        assert_eq!(entry.symbol(), 1);
    }
}
