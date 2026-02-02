//! # x86_64 Segmentation Framework
//!
//! Industrial-grade GDT, TSS, and segment management for SMP systems.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         SEGMENTATION FRAMEWORK                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                    Global Descriptor Table (GDT)                   │ │
//! │  ├───────┬────────────┬────────────┬────────────┬────────────────────┤ │
//! │  │ Null  │ Kernel CS  │ Kernel DS  │  User CS   │  User DS   │  TSS  │ │
//! │  │  [0]  │    [1]     │    [2]     │    [3]     │    [4]     │ [5-6] │ │
//! │  └───────┴────────────┴────────────┴────────────┴────────────────────┘ │
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                   Task State Segment (TSS)                         │ │
//! │  ├───────────────────────────────────────────────────────────────────┤ │
//! │  │  RSP0 (kernel stack)  │  RSP1  │  RSP2  │  IST1-7  │  IOPB        │ │
//! │  └───────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                  Per-CPU Segmentation                            │   │
//! │  │    CPU 0: GDT₀ + TSS₀        CPU 1: GDT₁ + TSS₁                 │   │
//! │  │    CPU 2: GDT₂ + TSS₂        CPU N: GDTₙ + TSSₙ                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Segment Layout
//!
//! | Index | Selector | Description          | DPL |
//! |-------|----------|---------------------|-----|
//! | 0     | 0x00     | Null descriptor     | -   |
//! | 1     | 0x08     | Kernel Code (64-bit)| 0   |
//! | 2     | 0x10     | Kernel Data         | 0   |
//! | 3     | 0x18     | User Data           | 3   |
//! | 4     | 0x20     | User Code (64-bit)  | 3   |
//! | 5-6   | 0x28     | TSS (128-bit entry) | 0   |
//!
//! ## IST Stack Allocation
//!
//! | IST | Purpose                  | Size    |
//! |-----|--------------------------|---------|
//! | 1   | #DF (Double Fault)       | 16 KB   |
//! | 2   | #NMI (Non-Maskable Int)  | 8 KB    |
//! | 3   | #MC (Machine Check)      | 8 KB    |
//! | 4   | #DB (Debug)              | 8 KB    |
//! | 5   | Reserved                 | 8 KB    |
//! | 6   | Reserved                 | 8 KB    |
//! | 7   | Reserved                 | 8 KB    |

pub mod gdt;
pub mod per_cpu;
pub mod selectors;
pub mod tss;

// Re-exports
pub use gdt::{DescriptorFlags, DescriptorType, Gdt, GdtDescriptor, GdtEntry};
pub use per_cpu::{init_ap, init_bsp, PerCpuSegmentation, MAX_CPUS};
pub use selectors::{Rpl, SegmentSelector, KERNEL_CS, KERNEL_DS, TSS_SELECTOR, USER_CS, USER_DS};
pub use tss::{IstIndex, IstStack, Tss, TssEntry, IST_STACK_SIZE, KERNEL_STACK_SIZE};

/// Initialize segmentation for the bootstrap processor
///
/// # Safety
/// Must be called exactly once during early boot.
pub unsafe fn init() {
    per_cpu::init_bsp();
}

/// Initialize segmentation for an application processor
///
/// # Safety
/// Must be called exactly once per AP during SMP initialization.
pub unsafe fn init_for_ap(cpu_id: usize) {
    per_cpu::init_ap(cpu_id);
}
