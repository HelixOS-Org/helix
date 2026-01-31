//! # Inter-Processor Interrupts (IPI)
//!
//! This module provides IPI support for AArch64 systems, using SGIs
//! (Software Generated Interrupts) via the GIC.
//!
//! ## IPI Types
//!
//! | Vector | Purpose            | Description                              |
//! |--------|--------------------|------------------------------------------|
//! | 0      | Reschedule         | Request target CPU to run scheduler      |
//! | 1      | TLB Shootdown      | Invalidate TLB entries                   |
//! | 2      | Function Call      | Execute a function on remote CPU         |
//! | 3      | CPU Stop           | Halt target CPU                          |
//! | 4      | CPU Wake           | Wake CPU from idle                       |
//! | 5-15   | Platform/Reserved  | Available for OS use                     |
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                         IPI Flow                                      │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │   Source CPU                           Target CPU                     │
//! │  ┌────────────┐                       ┌────────────┐                  │
//! │  │            │                       │            │                  │
//! │  │  ipi_send()│                       │            │                  │
//! │  │     │      │                       │            │                  │
//! │  │     ▼      │                       │            │                  │
//! │  │ Write SGI  │                       │            │                  │
//! │  │ Register   │──────────────────────▶│ SGI IRQ    │                  │
//! │  │ (ICC_SGI1R │     GIC routes        │ triggers   │                  │
//! │  │  or SGIR)  │     interrupt         │            │                  │
//! │  │            │                       │     │      │                  │
//! │  │            │                       │     ▼      │                  │
//! │  │            │                       │ IPI handler│                  │
//! │  │            │                       │ dispatches │                  │
//! │  │            │                       │ based on   │                  │
//! │  │            │                       │ vector     │                  │
//! │  └────────────┘                       └────────────┘                  │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```

use super::{
    mpidr::Mpidr,
    percpu::PerCpuData,
    SmpError, MAX_CPUS,
    IPI_RESCHEDULE, IPI_TLB_SHOOTDOWN, IPI_CALL_FUNCTION, IPI_CPU_STOP, IPI_CPU_WAKE,
};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

// ============================================================================
// IPI Vector Definitions
// ============================================================================

/// IPI vector types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpiVector {
    /// Reschedule request
    Reschedule = IPI_RESCHEDULE,
    /// TLB shootdown
    TlbShootdown = IPI_TLB_SHOOTDOWN,
    /// Remote function call
    CallFunction = IPI_CALL_FUNCTION,
    /// Stop CPU
    CpuStop = IPI_CPU_STOP,
    /// Wake CPU
    CpuWake = IPI_CPU_WAKE,
    /// Custom vector
    Custom(u8),
}

impl IpiVector {
    /// Get the raw SGI ID
    pub const fn sgi_id(self) -> u8 {
        match self {
            IpiVector::Reschedule => IPI_RESCHEDULE,
            IpiVector::TlbShootdown => IPI_TLB_SHOOTDOWN,
            IpiVector::CallFunction => IPI_CALL_FUNCTION,
            IpiVector::CpuStop => IPI_CPU_STOP,
            IpiVector::CpuWake => IPI_CPU_WAKE,
            IpiVector::Custom(v) => v,
        }
    }

    /// Create from raw SGI ID
    pub const fn from_sgi(sgi: u8) -> Self {
        match sgi {
            IPI_RESCHEDULE => IpiVector::Reschedule,
            IPI_TLB_SHOOTDOWN => IpiVector::TlbShootdown,
            IPI_CALL_FUNCTION => IpiVector::CallFunction,
            IPI_CPU_STOP => IpiVector::CpuStop,
            IPI_CPU_WAKE => IpiVector::CpuWake,
            v => IpiVector::Custom(v),
        }
    }
}

// ============================================================================
// IPI Sending (Low-Level)
// ============================================================================

/// Send an SGI using GICv3 system registers
///
/// This writes to ICC_SGI1R_EL1 to generate an SGI.
#[inline]
pub fn send_sgi_gicv3(sgi_id: u8, target_list: u16, aff1: u8, aff2: u8, aff3: u8, irm: bool) {
    // ICC_SGI1R_EL1 format:
    // [3:0]   - Target List (bitmap of Aff0 values 0-15)
    // [15:8]  - Aff1
    // [23:16] - INTID (SGI ID)
    // [31:24] - Aff2
    // [40]    - IRM (1 = all except self)
    // [55:48] - Aff3
    let value = (target_list as u64)
        | ((aff1 as u64) << 16)
        | ((sgi_id as u64 & 0xF) << 24)
        | ((aff2 as u64) << 32)
        | ((if irm { 1u64 } else { 0u64 }) << 40)
        | ((aff3 as u64) << 48);

    unsafe {
        core::arch::asm!(
            "msr S3_0_C12_C11_5, {}", // ICC_SGI1R_EL1
            in(reg) value,
            options(nomem, nostack),
        );
    }
}

/// Send an SGI to all CPUs except self (GICv3)
#[inline]
pub fn send_sgi_all_except_self_gicv3(sgi_id: u8) {
    send_sgi_gicv3(sgi_id, 0, 0, 0, 0, true);
}

/// Send an SGI to self (GICv3)
#[inline]
pub fn send_sgi_self_gicv3(sgi_id: u8) {
    let mpidr = Mpidr::current();
    let target = 1u16 << (mpidr.aff0() & 0xF);
    send_sgi_gicv3(sgi_id, target, mpidr.aff1(), mpidr.aff2(), mpidr.aff3(), false);
}

/// Send an SGI to a specific CPU by MPIDR (GICv3)
#[inline]
pub fn send_sgi_to_mpidr_gicv3(sgi_id: u8, mpidr: Mpidr) {
    let target = 1u16 << (mpidr.aff0() & 0xF);
    send_sgi_gicv3(sgi_id, target, mpidr.aff1(), mpidr.aff2(), mpidr.aff3(), false);
}

// ============================================================================
// IPI Interface
// ============================================================================

/// IPI operations interface
pub struct IpiOps {
    /// Use GICv3 (system registers) or GICv2 (MMIO)
    use_gicv3: bool,
    /// GICv2 GICD base address (if using GICv2)
    gicd_base: *mut u8,
}

impl IpiOps {
    /// Create for GICv3
    pub const fn gicv3() -> Self {
        Self {
            use_gicv3: true,
            gicd_base: core::ptr::null_mut(),
        }
    }

    /// Create for GICv2
    pub const fn gicv2(gicd_base: *mut u8) -> Self {
        Self {
            use_gicv3: false,
            gicd_base,
        }
    }

    /// Send IPI to a specific CPU
    pub fn send_ipi(&self, cpu_id: u32, vector: IpiVector) -> Result<(), SmpError> {
        let sgi_id = vector.sgi_id();
        if sgi_id >= 16 {
            return Err(SmpError::InvalidCpu);
        }

        if self.use_gicv3 {
            // Would need to look up MPIDR from topology
            // For now, assume simple mapping
            let mpidr = Mpidr::from_affinity(0, 0, 0, cpu_id as u8);
            send_sgi_to_mpidr_gicv3(sgi_id, mpidr);
            Ok(())
        } else {
            // GICv2: Write to GICD_SGIR
            unsafe {
                let target_mask = 1u32 << (cpu_id & 7);
                let sgir_value = ((target_mask as u32) << 16) | (sgi_id as u32);
                let sgir_addr = self.gicd_base.add(0x0F00) as *mut u32;
                core::ptr::write_volatile(sgir_addr, sgir_value);
            }
            Ok(())
        }
    }

    /// Send IPI to multiple CPUs
    pub fn send_ipi_mask(&self, mask: u64, vector: IpiVector) -> Result<(), SmpError> {
        let sgi_id = vector.sgi_id();
        if sgi_id >= 16 {
            return Err(SmpError::InvalidCpu);
        }

        // Send to each CPU in the mask
        for cpu_id in 0..64 {
            if (mask & (1 << cpu_id)) != 0 {
                let _ = self.send_ipi(cpu_id, vector);
            }
        }
        Ok(())
    }

    /// Send IPI to all CPUs except self
    pub fn send_ipi_all_except_self(&self, vector: IpiVector) -> Result<(), SmpError> {
        let sgi_id = vector.sgi_id();
        if sgi_id >= 16 {
            return Err(SmpError::InvalidCpu);
        }

        if self.use_gicv3 {
            send_sgi_all_except_self_gicv3(sgi_id);
        } else {
            // GICv2: Use target filter 01 (all except self)
            unsafe {
                let sgir_value = (1u32 << 24) | (sgi_id as u32);
                let sgir_addr = self.gicd_base.add(0x0F00) as *mut u32;
                core::ptr::write_volatile(sgir_addr, sgir_value);
            }
        }
        Ok(())
    }

    /// Send IPI to self
    pub fn send_ipi_self(&self, vector: IpiVector) -> Result<(), SmpError> {
        let sgi_id = vector.sgi_id();
        if sgi_id >= 16 {
            return Err(SmpError::InvalidCpu);
        }

        if self.use_gicv3 {
            send_sgi_self_gicv3(sgi_id);
        } else {
            // GICv2: Use target filter 10 (self only)
            unsafe {
                let sgir_value = (2u32 << 24) | (sgi_id as u32);
                let sgir_addr = self.gicd_base.add(0x0F00) as *mut u32;
                core::ptr::write_volatile(sgir_addr, sgir_value);
            }
        }
        Ok(())
    }
}

// ============================================================================
// IPI Handlers
// ============================================================================

/// IPI handler function type
pub type IpiHandler = fn(cpu_id: u32, data: usize);

/// IPI handler table
static mut IPI_HANDLERS: [Option<IpiHandler>; 16] = [None; 16];

/// IPI handler data
static IPI_HANDLER_DATA: [AtomicUsize; 16] = [
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
    AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0),
];

/// Register an IPI handler
pub fn register_ipi_handler(vector: IpiVector, handler: IpiHandler) {
    let sgi_id = vector.sgi_id() as usize;
    if sgi_id < 16 {
        unsafe {
            IPI_HANDLERS[sgi_id] = Some(handler);
        }
    }
}

/// Set IPI handler data
pub fn set_ipi_data(vector: IpiVector, data: usize) {
    let sgi_id = vector.sgi_id() as usize;
    if sgi_id < 16 {
        IPI_HANDLER_DATA[sgi_id].store(data, Ordering::Release);
    }
}

/// Handle an IPI (called from interrupt handler)
pub fn handle_ipi(sgi_id: u8, cpu_id: u32) {
    if sgi_id >= 16 {
        return;
    }

    let handler = unsafe { IPI_HANDLERS[sgi_id as usize] };
    let data = IPI_HANDLER_DATA[sgi_id as usize].load(Ordering::Acquire);

    if let Some(h) = handler {
        h(cpu_id, data);
    } else {
        // Default handlers
        match IpiVector::from_sgi(sgi_id) {
            IpiVector::Reschedule => handle_reschedule_ipi(cpu_id),
            IpiVector::TlbShootdown => handle_tlb_shootdown_ipi(cpu_id),
            IpiVector::CpuStop => handle_cpu_stop_ipi(cpu_id),
            IpiVector::CpuWake => handle_cpu_wake_ipi(cpu_id),
            _ => {}
        }
    }
}

// ============================================================================
// Default IPI Handlers
// ============================================================================

/// Handle reschedule IPI
fn handle_reschedule_ipi(_cpu_id: u32) {
    if let Some(percpu) = PerCpuData::try_current() {
        percpu.set_need_resched();
    }
}

/// Handle TLB shootdown IPI
fn handle_tlb_shootdown_ipi(_cpu_id: u32) {
    if let Some(percpu) = PerCpuData::try_current() {
        let addr = percpu.tlb_flush_addr.load(Ordering::Acquire);

        if addr == 0 {
            // Full TLB flush
            unsafe {
                core::arch::asm!(
                    "tlbi vmalle1is",
                    "dsb ish",
                    "isb",
                    options(nomem, nostack),
                );
            }
        } else {
            // Single page flush
            unsafe {
                core::arch::asm!(
                    "tlbi vaae1is, {addr}",
                    "dsb ish",
                    "isb",
                    addr = in(reg) addr >> 12,
                    options(nomem, nostack),
                );
            }
        }

        percpu.tlb_shootdown_pending = false;
    }
}

/// Handle CPU stop IPI
fn handle_cpu_stop_ipi(_cpu_id: u32) -> ! {
    // Disable interrupts and halt
    unsafe {
        core::arch::asm!(
            "msr daifset, #0xf",
            "1: wfe",
            "b 1b",
            options(nomem, nostack, noreturn),
        );
    }
}

/// Handle CPU wake IPI
fn handle_cpu_wake_ipi(_cpu_id: u32) {
    // Just receiving the IPI wakes the CPU from WFE
    // No additional action needed
}

// ============================================================================
// High-Level IPI Functions
// ============================================================================

/// Global IPI operations
static mut IPI_OPS: IpiOps = IpiOps::gicv3();

/// Initialize IPI subsystem
///
/// # Safety
///
/// Must be called once during boot.
pub unsafe fn init_ipi(ops: IpiOps) {
    IPI_OPS = ops;
}

/// Get the IPI operations interface
fn ipi_ops() -> &'static IpiOps {
    unsafe { &IPI_OPS }
}

/// Send a reschedule IPI to a CPU
pub fn send_reschedule_ipi(cpu_id: u32) -> Result<(), SmpError> {
    ipi_ops().send_ipi(cpu_id, IpiVector::Reschedule)
}

/// Send a reschedule IPI to all other CPUs
pub fn send_reschedule_ipi_all() -> Result<(), SmpError> {
    ipi_ops().send_ipi_all_except_self(IpiVector::Reschedule)
}

/// Request TLB shootdown on all CPUs
pub fn request_tlb_shootdown_all() -> Result<(), SmpError> {
    // Set flush address to 0 for full flush on all CPUs
    for i in 0..MAX_CPUS {
        if let Some(percpu) = unsafe { super::percpu::get_percpu(i as u32) } {
            percpu.tlb_flush_addr.store(0, Ordering::Release);
            percpu.tlb_shootdown_pending = true;
        }
    }

    ipi_ops().send_ipi_all_except_self(IpiVector::TlbShootdown)?;

    // Flush local TLB too
    unsafe {
        core::arch::asm!(
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nomem, nostack),
        );
    }

    Ok(())
}

/// Request TLB shootdown for a specific address on all CPUs
pub fn request_tlb_shootdown_addr(addr: u64) -> Result<(), SmpError> {
    for i in 0..MAX_CPUS {
        if let Some(percpu) = unsafe { super::percpu::get_percpu(i as u32) } {
            percpu.tlb_flush_addr.store(addr, Ordering::Release);
            percpu.tlb_shootdown_pending = true;
        }
    }

    ipi_ops().send_ipi_all_except_self(IpiVector::TlbShootdown)?;

    // Flush local TLB too
    unsafe {
        core::arch::asm!(
            "tlbi vaae1is, {addr}",
            "dsb ish",
            "isb",
            addr = in(reg) addr >> 12,
            options(nomem, nostack),
        );
    }

    Ok(())
}

/// Stop all other CPUs
pub fn stop_all_cpus() -> Result<(), SmpError> {
    ipi_ops().send_ipi_all_except_self(IpiVector::CpuStop)
}

/// Wake a CPU from idle
pub fn wake_cpu(cpu_id: u32) -> Result<(), SmpError> {
    ipi_ops().send_ipi(cpu_id, IpiVector::CpuWake)
}

// ============================================================================
// Remote Function Call
// ============================================================================

/// Remote function call state
pub struct RemoteFunctionCall {
    /// Function to call
    pub func: fn(usize) -> usize,
    /// Argument
    pub arg: AtomicUsize,
    /// Result
    pub result: AtomicUsize,
    /// Completed flag per CPU
    pub completed: [AtomicU64; 4], // 256 CPUs / 64 bits
    /// Number of CPUs to wait for
    pub wait_count: AtomicU64,
}

impl RemoteFunctionCall {
    /// Create a new remote function call
    pub const fn new(func: fn(usize) -> usize) -> Self {
        Self {
            func,
            arg: AtomicUsize::new(0),
            result: AtomicUsize::new(0),
            completed: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            wait_count: AtomicU64::new(0),
        }
    }

    /// Mark a CPU as completed
    pub fn mark_completed(&self, cpu_id: u32) {
        let word = (cpu_id / 64) as usize;
        let bit = cpu_id % 64;
        self.completed[word].fetch_or(1 << bit, Ordering::Release);
    }

    /// Check if a CPU has completed
    pub fn is_completed(&self, cpu_id: u32) -> bool {
        let word = (cpu_id / 64) as usize;
        let bit = cpu_id % 64;
        (self.completed[word].load(Ordering::Acquire) & (1 << bit)) != 0
    }

    /// Reset for a new call
    pub fn reset(&self) {
        for c in &self.completed {
            c.store(0, Ordering::Release);
        }
        self.wait_count.store(0, Ordering::Release);
    }
}

/// Global remote function call state
static REMOTE_CALL: RemoteFunctionCall = RemoteFunctionCall::new(|_| 0);

/// Call a function on a remote CPU
pub fn call_function_on_cpu(
    cpu_id: u32,
    func: fn(usize) -> usize,
    arg: usize,
) -> Result<usize, SmpError> {
    // Set up the call
    REMOTE_CALL.reset();
    // Note: In a real implementation, we'd need to handle the function pointer
    // This is a simplified version
    REMOTE_CALL.arg.store(arg, Ordering::Release);
    REMOTE_CALL.wait_count.store(1, Ordering::Release);

    // Send IPI
    set_ipi_data(IpiVector::CallFunction, &REMOTE_CALL as *const _ as usize);
    ipi_ops().send_ipi(cpu_id, IpiVector::CallFunction)?;

    // Wait for completion
    while !REMOTE_CALL.is_completed(cpu_id) {
        core::hint::spin_loop();
    }

    Ok(REMOTE_CALL.result.load(Ordering::Acquire))
}
