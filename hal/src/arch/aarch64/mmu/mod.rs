//! # AArch64 Memory Management Unit
//!
//! This module provides comprehensive MMU support for AArch64 including:
//! - Translation table management
//! - Page table entries
//! - TLB operations
//! - ASID management
//! - Memory mapping utilities

pub mod asid;
pub mod entries;
pub mod mapping;
pub mod tables;
pub mod tlb;

// Re-exports
pub use asid::{Asid, AsidAllocator};
pub use entries::{PageTableEntry, BlockDescriptor, TableDescriptor, MemoryAttributes};
pub use mapping::{MapFlags, VirtualMemoryMapper};
pub use tables::{PageTable, TranslationGranule, TranslationLevel};
pub use tlb::{tlb_flush_all, tlb_flush_asid, tlb_flush_page};

/// Page size (4KB default)
pub const PAGE_SIZE: usize = 4096;

/// Page shift (log2 of page size)
pub const PAGE_SHIFT: usize = 12;

/// Page mask
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// 2MB block size (L2)
pub const BLOCK_2M_SIZE: usize = 2 * 1024 * 1024;

/// 1GB block size (L1)
pub const BLOCK_1G_SIZE: usize = 1024 * 1024 * 1024;

/// Kernel virtual address base
pub const KERNEL_VADDR_BASE: u64 = 0xFFFF_0000_0000_0000;

/// User space ends at this address (TTBR0 limit)
pub const USER_VADDR_END: u64 = 0x0000_FFFF_FFFF_FFFF;

/// Align address down to page boundary
pub const fn page_align_down(addr: usize) -> usize {
    addr & PAGE_MASK
}

/// Align address up to page boundary
pub const fn page_align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & PAGE_MASK
}

/// Check if address is page aligned
pub const fn is_page_aligned(addr: usize) -> bool {
    addr & (PAGE_SIZE - 1) == 0
}

/// Convert address to page number
pub const fn addr_to_pfn(addr: usize) -> usize {
    addr >> PAGE_SHIFT
}

/// Convert page number to address
pub const fn pfn_to_addr(pfn: usize) -> usize {
    pfn << PAGE_SHIFT
}
