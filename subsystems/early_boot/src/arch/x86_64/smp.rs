//! # x86_64 SMP (Symmetric Multi-Processing) Initialization
//!
//! AP (Application Processor) startup via INIT-SIPI-SIPI protocol.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::apic::*;
use super::gdt::PerCpuGdt;
use super::timers::delay_us;
use super::*;
use crate::core::{BootContext, SmpStartupMethod};
use crate::error::{BootError, BootResult};

// =============================================================================
// SMP CONSTANTS
// =============================================================================

/// Maximum number of CPUs supported
pub const MAX_CPUS: usize = 256;

/// AP trampoline code location (must be < 1MB)
pub const AP_TRAMPOLINE_ADDR: u64 = 0x8000;

/// AP stack size
pub const AP_STACK_SIZE: usize = 32768;

/// AP startup timeout in microseconds
pub const AP_STARTUP_TIMEOUT_US: u64 = 200_000;

// =============================================================================
// SMP STATE
// =============================================================================

/// Number of started CPUs
static STARTED_CPUS: AtomicU32 = AtomicU32::new(1); // BSP is already running

/// CPU online bitmap (64 CPUs per u64)
static CPU_ONLINE_BITMAP: [AtomicU64; 4] = [
    AtomicU64::new(1), // BSP (CPU 0) is online
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

/// AP rendezvous flag
static AP_STARTED: AtomicBool = AtomicBool::new(false);

/// BSP APIC ID
static BSP_APIC_ID: AtomicU32 = AtomicU32::new(0);

// =============================================================================
// PER-CPU DATA
// =============================================================================

/// Per-CPU data structure
#[repr(C, align(64))] // Cache line aligned
pub struct PerCpuData {
    /// CPU index (0 = BSP)
    pub cpu_id: u32,
    /// APIC ID
    pub apic_id: u32,
    /// Is this the BSP?
    pub is_bsp: bool,
    /// CPU is online and running
    pub online: AtomicBool,
    /// GDT and TSS for this CPU
    pub gdt_tss: PerCpuGdt,
    /// Kernel stack pointer
    pub kernel_stack: u64,
    /// Current task pointer
    pub current_task: AtomicU64,
    /// Idle task pointer
    pub idle_task: u64,
    /// Tick counter
    pub ticks: AtomicU64,
    /// Padding to cache line
    _padding: [u8; 16],
}

impl PerCpuData {
    pub const fn new() -> Self {
        Self {
            cpu_id: 0,
            apic_id: 0,
            is_bsp: false,
            online: AtomicBool::new(false),
            gdt_tss: PerCpuGdt::new(),
            kernel_stack: 0,
            current_task: AtomicU64::new(0),
            idle_task: 0,
            ticks: AtomicU64::new(0),
            _padding: [0; 16],
        }
    }
}

/// Static per-CPU data array
static mut PER_CPU_DATA: [PerCpuData; MAX_CPUS] = {
    const INIT: PerCpuData = PerCpuData::new();
    [INIT; MAX_CPUS]
};

/// Get per-CPU data for current CPU
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn get_per_cpu() -> &'static PerCpuData {
    let apic_id = get_apic_id() as usize;
    // Find CPU by APIC ID
    for i in 0..MAX_CPUS {
        if PER_CPU_DATA[i].apic_id as usize == apic_id
            && PER_CPU_DATA[i].online.load(Ordering::SeqCst)
        {
            return &PER_CPU_DATA[i];
        }
    }
    // Fallback to BSP
    &PER_CPU_DATA[0]
}

/// Get per-CPU data by index
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn get_per_cpu_by_id(id: u32) -> &'static PerCpuData {
    &PER_CPU_DATA[id as usize]
}

/// Get mutable per-CPU data by index
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn get_per_cpu_by_id_mut(id: u32) -> &'static mut PerCpuData {
    &mut PER_CPU_DATA[id as usize]
}

// =============================================================================
// AP TRAMPOLINE
// =============================================================================

/// AP trampoline parameters (passed to APs)
#[repr(C)]
pub struct ApTrampolineParams {
    /// PML4 address
    pub pml4_addr: u64,
    /// Entry point (Rust function)
    pub entry_point: u64,
    /// Stack pointer
    pub stack_ptr: u64,
    /// CPU ID
    pub cpu_id: u32,
    /// APIC ID
    pub apic_id: u32,
    /// GDT pointer
    pub gdt_ptr: u64,
    /// IDT pointer
    pub idt_ptr: u64,
}

/// AP trampoline code (16-bit -> 32-bit -> 64-bit)
/// This gets copied to AP_TRAMPOLINE_ADDR
#[repr(C, align(4096))]
struct ApTrampoline {
    code: [u8; 4096],
}

/// Generate AP trampoline code
unsafe fn generate_ap_trampoline() -> &'static mut [u8] {
    let trampoline = AP_TRAMPOLINE_ADDR as *mut u8;

    // 16-bit real mode entry (CPU starts here)
    // org 0x8000
    let code: &[u8] = &[
        // cli
        0xFA, // cld
        0xFC, // xor ax, ax
        0x31, 0xC0, // mov ds, ax
        0x8E, 0xD8, // mov es, ax
        0x8E, 0xC0, // mov ss, ax
        0x8E, 0xD0, // Load GDT (offset 0x100)
        // lgdt [0x8100]
        0x0F, 0x01, 0x16, 0x00, 0x81, // Enable protected mode
        // mov eax, cr0
        0x0F, 0x20, 0xC0, // or al, 1
        0x0C, 0x01, // mov cr0, eax
        0x0F, 0x22, 0xC0, // Far jump to 32-bit code
        // jmp 0x08:trampoline32
        0x66, 0xEA, 0x30, 0x80, 0x00, 0x00, // Offset (0x8030)
        0x08, 0x00, // Segment selector
        // Padding to 0x30
        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
        // 32-bit protected mode code (offset 0x30)
        // mov ax, 0x10
        0x66, 0xB8, 0x10, 0x00, // mov ds, ax
        0x8E, 0xD8, // mov es, ax
        0x8E, 0xC0, // mov ss, ax
        0x8E, 0xD0, // Enable PAE
        // mov eax, cr4
        0x0F, 0x20, 0xE0, // or eax, 0x20
        0x0D, 0x20, 0x00, 0x00, 0x00, // mov cr4, eax
        0x0F, 0x22, 0xE0,
        // Load PML4 from params (offset 0x110)
        // mov eax, [0x8110]
        0xA1, 0x10, 0x81, 0x00, 0x00, // mov cr3, eax
        0x0F, 0x22, 0xD8, // Enable long mode in EFER MSR
        // mov ecx, 0xC0000080
        0xB9, 0x80, 0x00, 0x00, 0xC0, // rdmsr
        0x0F, 0x32, // or eax, 0x100
        0x0D, 0x00, 0x01, 0x00, 0x00, // wrmsr
        0x0F, 0x30, // Enable paging
        // mov eax, cr0
        0x0F, 0x20, 0xC0, // or eax, 0x80000001
        0x0D, 0x01, 0x00, 0x00, 0x80, // mov cr0, eax
        0x0F, 0x22, 0xC0, // Far jump to 64-bit code
        // jmp 0x08:trampoline64
        0xEA, 0x80, 0x80, 0x00, 0x00, // Offset (0x8080)
        0x08, 0x00, // Segment selector (64-bit)
        // Padding to 0x80
        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    ];

    // Copy initial code
    core::ptr::copy_nonoverlapping(code.as_ptr(), trampoline, code.len());

    // 64-bit code at offset 0x80
    let trampoline64: &[u8] = &[
        // Load 64-bit data segments
        // mov ax, 0x10
        0x66, 0xB8, 0x10, 0x00, // mov ds, ax
        0x48, 0x8E, 0xD8, // mov es, ax
        0x48, 0x8E, 0xC0, // mov ss, ax
        0x48, 0x8E, 0xD0, // xor ax, ax
        0x66, 0x31, 0xC0, // mov fs, ax
        0x48, 0x8E, 0xE0, // mov gs, ax
        0x48, 0x8E, 0xE8, // Load stack from params
        // mov rsp, [0x8120]
        0x48, 0x8B, 0x24, 0x25, 0x20, 0x81, 0x00, 0x00,
        // Load entry point
        // mov rax, [0x8118]
        0x48, 0x8B, 0x04, 0x25, 0x18, 0x81, 0x00, 0x00,
        // Load CPU ID as argument
        // mov edi, [0x8128]
        0x8B, 0x3C, 0x25, 0x28, 0x81, 0x00, 0x00, // Call entry point
        // call rax
        0xFF, 0xD0, // Halt if returned
        // cli
        0xFA, // hlt
        0xF4, // jmp $
        0xEB, 0xFC,
    ];

    core::ptr::copy_nonoverlapping(
        trampoline64.as_ptr(),
        trampoline.add(0x80),
        trampoline64.len(),
    );

    // Temporary GDT at offset 0x100
    let gdt_ptr = trampoline.add(0x100);
    // GDT limit (23 bytes for 3 entries)
    core::ptr::write(gdt_ptr as *mut u16, 23);
    // GDT base
    core::ptr::write((gdt_ptr as *mut u32).add(1), 0x8108);

    // GDT entries at 0x108
    let gdt = trampoline.add(0x108) as *mut u64;
    // Null entry
    core::ptr::write(gdt, 0);
    // 64-bit code segment (selector 0x08)
    core::ptr::write(gdt.add(1), 0x00AF9A000000FFFF);
    // 64-bit data segment (selector 0x10)
    core::ptr::write(gdt.add(2), 0x00CF92000000FFFF);

    core::slice::from_raw_parts_mut(trampoline, 4096)
}

/// Set AP trampoline parameters
unsafe fn set_ap_trampoline_params(params: &ApTrampolineParams) {
    let params_addr = (AP_TRAMPOLINE_ADDR + 0x110) as *mut ApTrampolineParams;
    core::ptr::write(params_addr, ApTrampolineParams {
        pml4_addr: params.pml4_addr,
        entry_point: params.entry_point,
        stack_ptr: params.stack_ptr,
        cpu_id: params.cpu_id,
        apic_id: params.apic_id,
        gdt_ptr: params.gdt_ptr,
        idt_ptr: params.idt_ptr,
    });
}

// =============================================================================
// AP STARTUP
// =============================================================================

/// AP entry point (called by trampoline)
#[no_mangle]
extern "C" fn ap_entry(cpu_id: u32) {
    unsafe {
        // Initialize per-CPU data
        let per_cpu = get_per_cpu_by_id_mut(cpu_id);

        // Initialize GDT/TSS for this CPU
        per_cpu.gdt_tss.init();
        per_cpu.gdt_tss.load();

        // Signal that we've started
        per_cpu.online.store(true, Ordering::SeqCst);
        AP_STARTED.store(true, Ordering::SeqCst);
        STARTED_CPUS.fetch_add(1, Ordering::SeqCst);

        // Mark CPU as online in bitmap
        let bitmap_idx = cpu_id as usize / 64;
        let bit_idx = cpu_id as usize % 64;
        CPU_ONLINE_BITMAP[bitmap_idx].fetch_or(1 << bit_idx, Ordering::SeqCst);

        // Initialize local APIC
        init_ap_apic();

        // Enable interrupts
        core::arch::asm!("sti", options(nomem, nostack));

        // Enter AP idle loop
        ap_idle_loop(cpu_id);
    }
}

/// Initialize APIC on AP
unsafe fn init_ap_apic() {
    // Enable APIC
    super::apic::enable_lapic();

    // Set spurious vector
    super::apic::lapic_write(LAPIC_SVR, 0xFF | (1 << 8));

    // Clear task priority
    super::apic::lapic_write(LAPIC_TPR, 0);

    // Set up timer
    let apic_freq = super::apic::get_apic_frequency();
    if apic_freq > 0 {
        let count = apic_freq / 1000; // 1kHz
        super::apic::lapic_write(super::apic::LAPIC_TIMER_DCR, super::apic::TIMER_DIV_16);
        super::apic::lapic_write(
            super::apic::LAPIC_LVT_TIMER,
            super::apic::LVT_TIMER_PERIODIC | 0xFE,
        );
        super::apic::lapic_write(super::apic::LAPIC_TIMER_ICR, count as u32);
    }

    // Send EOI to clear any pending interrupts
    super::apic::send_eoi();
}

/// AP idle loop
fn ap_idle_loop(cpu_id: u32) -> ! {
    loop {
        // TODO: Check for work to do

        // Halt until interrupt
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Start an AP
unsafe fn start_ap(apic_id: u32, cpu_id: u32) -> BootResult<()> {
    // Initialize per-CPU data
    let per_cpu = get_per_cpu_by_id_mut(cpu_id);
    per_cpu.cpu_id = cpu_id;
    per_cpu.apic_id = apic_id;
    per_cpu.is_bsp = false;

    // Allocate stack
    let stack = alloc_ap_stack()?;
    per_cpu.kernel_stack = stack + AP_STACK_SIZE as u64;

    // Set trampoline parameters
    let params = ApTrampolineParams {
        pml4_addr: super::read_cr3(),
        entry_point: ap_entry as u64,
        stack_ptr: per_cpu.kernel_stack,
        cpu_id,
        apic_id,
        gdt_ptr: 0, // Will use trampoline GDT initially
        idt_ptr: 0,
    };
    set_ap_trampoline_params(&params);

    // Clear started flag
    AP_STARTED.store(false, Ordering::SeqCst);

    // Send INIT IPI
    send_init_ipi(apic_id as u8);

    // Wait 10ms
    delay_us(10_000);

    // Send first SIPI
    let vector = (AP_TRAMPOLINE_ADDR >> 12) as u8;
    send_sipi(apic_id as u8, vector);

    // Wait 200us
    delay_us(200);

    // Send second SIPI if not started
    if !AP_STARTED.load(Ordering::SeqCst) {
        send_sipi(apic_id as u8, vector);
    }

    // Wait for AP to start (with timeout)
    let start_time = super::timers::get_time_us();
    while !AP_STARTED.load(Ordering::SeqCst) {
        if super::timers::get_time_us() - start_time > AP_STARTUP_TIMEOUT_US {
            return Err(BootError::CpuStartupFailed);
        }
        core::hint::spin_loop();
    }

    Ok(())
}

/// Allocate AP stack
unsafe fn alloc_ap_stack() -> BootResult<u64> {
    // Simple bump allocator for AP stacks
    static NEXT_STACK: AtomicU64 = AtomicU64::new(0);

    // Initialize if needed
    if NEXT_STACK.load(Ordering::SeqCst) == 0 {
        // Allocate stack region (1MB for up to 32 CPUs)
        let region = super::paging::alloc_frame().ok_or(BootError::OutOfMemory)?;
        NEXT_STACK.store(region, Ordering::SeqCst);
    }

    let stack = NEXT_STACK.fetch_add(AP_STACK_SIZE as u64, Ordering::SeqCst);
    Ok(stack)
}

// =============================================================================
// SMP INITIALIZATION
// =============================================================================

/// Initialize SMP
///
/// # Safety
///
/// The caller must ensure SMP initialization is done after BSP is fully initialized.
pub unsafe fn init_smp(ctx: &mut BootContext) -> BootResult<()> {
    // Store BSP APIC ID
    let bsp_apic_id = get_apic_id();
    BSP_APIC_ID.store(bsp_apic_id, Ordering::SeqCst);

    // Initialize BSP per-CPU data
    let bsp_data = get_per_cpu_by_id_mut(0);
    bsp_data.cpu_id = 0;
    bsp_data.apic_id = bsp_apic_id;
    bsp_data.is_bsp = true;
    bsp_data.online.store(true, Ordering::SeqCst);

    // Generate trampoline code
    generate_ap_trampoline();

    // Detect APs from ACPI MADT
    let ap_list = detect_aps(ctx)?;

    if ap_list.is_empty() {
        // Single CPU system
        ctx.smp_state.cpu_count = 1;
        ctx.smp_state.online_mask = 1;
        ctx.smp_state.startup_method = SmpStartupMethod::None;
        return Ok(());
    }

    // Start each AP
    let mut cpu_id = 1u32;
    let mut success_count = 0u32;

    for apic_id in ap_list {
        if apic_id == bsp_apic_id {
            continue; // Skip BSP
        }

        match start_ap(apic_id, cpu_id) {
            Ok(()) => {
                success_count += 1;
                cpu_id += 1;
            },
            Err(e) => {
                // Log but continue with other APs
                // Some CPUs may be offline or disabled
            },
        }
    }

    // Update context
    ctx.smp_state.cpu_count = success_count + 1; // +1 for BSP
    ctx.smp_state.bsp_id = 0;
    ctx.smp_state.online_mask = get_online_mask();
    ctx.smp_state.startup_method = SmpStartupMethod::InitSipi;

    Ok(())
}

/// Detect APs from ACPI MADT
fn detect_aps(_ctx: &BootContext) -> BootResult<alloc::vec::Vec<u32>> {
    // TODO: Parse ACPI MADT table
    // For now, return empty (single CPU)
    Ok(alloc::vec::Vec::new())
}

extern crate alloc;

/// Get online CPU mask
fn get_online_mask() -> u64 {
    CPU_ONLINE_BITMAP[0].load(Ordering::SeqCst)
}

/// Get number of online CPUs
pub fn get_cpu_count() -> u32 {
    STARTED_CPUS.load(Ordering::SeqCst)
}

/// Check if CPU is online
pub fn is_cpu_online(cpu_id: u32) -> bool {
    if cpu_id >= MAX_CPUS as u32 {
        return false;
    }

    let bitmap_idx = cpu_id as usize / 64;
    let bit_idx = cpu_id as usize % 64;

    (CPU_ONLINE_BITMAP[bitmap_idx].load(Ordering::SeqCst) & (1 << bit_idx)) != 0
}

/// Get current CPU ID
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn get_current_cpu_id() -> u32 {
    get_per_cpu().cpu_id
}

/// Get BSP APIC ID
pub fn get_bsp_apic_id() -> u32 {
    BSP_APIC_ID.load(Ordering::SeqCst)
}

// =============================================================================
// SMP IPI FUNCTIONS
// =============================================================================

/// Send IPI to specific CPU
///
/// # Safety
///
/// The caller must ensure the target CPU ID is valid and the IPI type is appropriate.
pub unsafe fn send_ipi_to_cpu(cpu_id: u32, vector: u8) {
    let per_cpu = get_per_cpu_by_id(cpu_id);
    send_ipi(per_cpu.apic_id as u8, vector, ICR_FIXED | ICR_ASSERT);
}

/// Send IPI to all CPUs (including self)
///
/// # Safety
///
/// The caller must ensure the target CPU ID is valid and the IPI type is appropriate.
pub unsafe fn send_ipi_all(vector: u8) {
    send_ipi(0, vector, ICR_FIXED | ICR_ASSERT | ICR_ALL_INCLUDING_SELF);
}

/// Send IPI to all other CPUs
///
/// # Safety
///
/// The caller must ensure the target CPU ID is valid and the IPI type is appropriate.
pub unsafe fn send_ipi_others(vector: u8) {
    send_ipi(0, vector, ICR_FIXED | ICR_ASSERT | ICR_ALL_EXCLUDING_SELF);
}

/// Send NMI to all CPUs
///
/// # Safety
///
/// The caller must ensure the target CPUs can handle NMI safely.
pub unsafe fn send_nmi_all() {
    send_ipi(0, 0, ICR_NMI | ICR_ASSERT | ICR_ALL_INCLUDING_SELF);
}

/// Broadcast TLB flush
///
/// # Safety
///
/// The caller must ensure this is called when page table changes need to be visible.
pub unsafe fn broadcast_tlb_flush() {
    const TLB_FLUSH_VECTOR: u8 = 0xFD;
    send_ipi_others(TLB_FLUSH_VECTOR);
}
