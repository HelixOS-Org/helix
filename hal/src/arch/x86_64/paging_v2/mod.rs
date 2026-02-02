//! # x86_64 Paging Framework
//!
//! This module provides an industrial-grade paging implementation for x86_64
//! systems, supporting both 4-level and 5-level paging with PCID, TLB management,
//! and proper SMP support.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    4-Level Paging (48-bit Virtual)                      │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   Virtual Address (48-bit, sign-extended to 64-bit)                     │
//! │   ┌──────┬──────┬──────┬──────┬──────┬────────────┐                    │
//! │   │Sign  │PML4  │PDPT  │PD    │PT    │Page Offset │                    │
//! │   │Ext   │[47:39│[38:30│[29:21│[20:12│[11:0]      │                    │
//! │   │[63:48│9 bits│9 bits│9 bits│9 bits│12 bits     │                    │
//! │   └──────┴──────┴──────┴──────┴──────┴────────────┘                    │
//! │                                                                          │
//! │   CR3 ──► PML4 ──► PDPT ──► PD ──► PT ──► Physical Frame               │
//! │             │        │       │      │                                    │
//! │             │        │       │      └──► 4KB Page                       │
//! │             │        │       └──► 2MB Huge Page (if PS=1)               │
//! │             │        └──► 1GB Huge Page (if PS=1)                       │
//! │             └──► (unused in 4-level paging)                             │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    5-Level Paging (57-bit Virtual)                      │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   Virtual Address (57-bit, sign-extended to 64-bit)                     │
//! │   ┌──────┬──────┬──────┬──────┬──────┬──────┬────────────┐             │
//! │   │Sign  │PML5  │PML4  │PDPT  │PD    │PT    │Page Offset │             │
//! │   │Ext   │[56:48│[47:39│[38:30│[29:21│[20:12│[11:0]      │             │
//! │   │[63:57│9 bits│9 bits│9 bits│9 bits│9 bits│12 bits     │             │
//! │   └──────┴──────┴──────┴──────┴──────┴──────┴────────────┘             │
//! │                                                                          │
//! │   CR3 ──► PML5 ──► PML4 ──► PDPT ──► PD ──► PT ──► Physical Frame      │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - 4-level paging (48-bit virtual address space)
//! - 5-level paging (57-bit virtual address space) with LA57 support
//! - 4KB, 2MB, and 1GB page sizes
//! - PCID support for TLB efficiency
//! - TLB management (INVLPG, INVPCID)
//! - NX (No-Execute) bit support
//! - Global pages
//! - Write protection
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use hal::arch::x86_64::paging::{PageTable, PageTableEntry, PageFlags};
//!
//! // Create a new page table
//! let mut pml4 = PageTable::new();
//!
//! // Map a virtual address to a physical address
//! let virt = VirtualAddress::new(0xFFFF_8000_0000_0000);
//! let phys = PhysicalAddress::new(0x1000);
//! let flags = PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::NO_EXECUTE;
//!
//! unsafe {
//!     paging::map_page(virt, phys, flags)?;
//! }
//! ```

#![allow(dead_code)]

mod addresses;
mod entries;
mod table;
mod tlb;
mod walker;

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub use addresses::{Frame, Page, PageSize, PhysicalAddress, VirtualAddress};
pub use entries::{PageFlags, PageTableEntry, PageTableLevel};
pub use table::{PageTable, PageTableIndex};
pub use tlb::{flush_tlb, flush_tlb_all, flush_tlb_pcid, Pcid};
pub use walker::{MappingInfo, PageTableWalker, TranslationError};

// =============================================================================
// Constants
// =============================================================================

/// Page size: 4 KB
pub const PAGE_SIZE_4K: usize = 4096;

/// Page size: 2 MB
pub const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;

/// Page size: 1 GB
pub const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;

/// Number of entries in a page table
pub const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry size in bytes
pub const ENTRY_SIZE: usize = 8;

/// Page table size in bytes
pub const TABLE_SIZE: usize = ENTRIES_PER_TABLE * ENTRY_SIZE;

/// 4-level paging maximum virtual address bits
pub const VIRT_BITS_4LEVEL: u8 = 48;

/// 5-level paging maximum virtual address bits
pub const VIRT_BITS_5LEVEL: u8 = 57;

/// Physical address mask (52 bits on most systems)
pub const PHYS_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Maximum physical address bits (typically 52)
pub const PHYS_BITS_MAX: u8 = 52;

/// Canonical address high bit for 4-level paging
pub const CANONICAL_BIT_4LEVEL: u64 = 1 << 47;

/// Canonical address high bit for 5-level paging
pub const CANONICAL_BIT_5LEVEL: u64 = 1 << 56;

// =============================================================================
// Paging State
// =============================================================================

/// Whether 5-level paging is enabled
static FIVE_LEVEL_PAGING: AtomicBool = AtomicBool::new(false);

/// Current CR3 value
static CURRENT_CR3: AtomicU64 = AtomicU64::new(0);

/// Maximum supported physical address bits
static PHYS_ADDR_BITS: core::sync::atomic::AtomicU8 =
    core::sync::atomic::AtomicU8::new(PHYS_BITS_MAX);

// =============================================================================
// Public Interface
// =============================================================================

/// Initialize the paging subsystem
///
/// This detects CPU paging capabilities and sets up internal state.
///
/// # Safety
///
/// Must be called early in boot, before any paging operations.
pub unsafe fn init() {
    // Detect 5-level paging support
    let la57_supported = detect_la57_support();
    FIVE_LEVEL_PAGING.store(la57_supported, Ordering::SeqCst);

    // Detect physical address bits
    let phys_bits = detect_phys_addr_bits();
    PHYS_ADDR_BITS.store(phys_bits, Ordering::SeqCst);

    // Read current CR3
    let cr3 = read_cr3();
    CURRENT_CR3.store(cr3, Ordering::SeqCst);

    log::info!(
        "Paging: initialized (5-level={}, phys_bits={}, CR3={:#x})",
        la57_supported,
        phys_bits,
        cr3
    );
}

/// Check if 5-level paging is active
#[inline]
pub fn is_5level_paging() -> bool {
    FIVE_LEVEL_PAGING.load(Ordering::Relaxed)
}

/// Get the number of physical address bits
#[inline]
pub fn physical_address_bits() -> u8 {
    PHYS_ADDR_BITS.load(Ordering::Relaxed)
}

/// Get the number of virtual address bits
#[inline]
pub fn virtual_address_bits() -> u8 {
    if is_5level_paging() {
        VIRT_BITS_5LEVEL
    } else {
        VIRT_BITS_4LEVEL
    }
}

/// Read the current CR3 value
#[inline]
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Write to CR3 (change page table root)
///
/// # Safety
///
/// The new CR3 value must point to a valid page table.
/// This will flush the TLB (unless PCID is used with the noflush bit).
#[inline]
pub unsafe fn write_cr3(value: u64) {
    unsafe {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) value,
            options(nostack, preserves_flags),
        );
    }
    CURRENT_CR3.store(value, Ordering::Release);
}

/// Get the physical address of the current page table root
#[inline]
pub fn current_page_table_root() -> PhysicalAddress {
    PhysicalAddress::new(read_cr3() & PHYS_ADDR_MASK)
}

/// Get the current PCID (if any)
#[inline]
pub fn current_pcid() -> Option<Pcid> {
    let cr3 = read_cr3();
    let pcid_bits = (cr3 & 0xFFF) as u16;
    if pcid_bits != 0 {
        Some(Pcid::new(pcid_bits))
    } else {
        None
    }
}

/// Switch to a new page table
///
/// # Safety
///
/// The physical address must point to a valid page table.
#[inline]
pub unsafe fn switch_page_table(root: PhysicalAddress) {
    unsafe { write_cr3(root.as_u64()) };
}

/// Switch to a new page table with PCID
///
/// # Safety
///
/// The physical address must point to a valid page table.
/// PCID must be supported and enabled.
#[inline]
pub unsafe fn switch_page_table_pcid(root: PhysicalAddress, pcid: Pcid, noflush: bool) {
    let mut cr3 = root.as_u64() | (pcid.as_u16() as u64);
    if noflush {
        cr3 |= 1 << 63; // CR3.NOFLUSH bit
    }
    unsafe { write_cr3(cr3) };
}

/// Check if an address is canonical for the current paging mode
pub fn is_canonical(addr: u64) -> bool {
    let bits = virtual_address_bits();
    let mask = 1u64 << (bits - 1);

    // All bits above the virtual address range must match the sign bit
    let sign_extended = if addr & mask != 0 {
        addr | !((1u64 << bits) - 1)
    } else {
        addr & ((1u64 << bits) - 1)
    };

    addr == sign_extended
}

/// Make an address canonical
#[inline]
pub fn canonicalize(addr: u64) -> u64 {
    let bits = virtual_address_bits();
    let mask = 1u64 << (bits - 1);

    if addr & mask != 0 {
        addr | !((1u64 << bits) - 1)
    } else {
        addr & ((1u64 << bits) - 1)
    }
}

// =============================================================================
// Internal Functions
// =============================================================================

/// Detect LA57 (5-level paging) support
fn detect_la57_support() -> bool {
    // Check CPUID for LA57 support
    // CPUID.07H:ECX.LA57[bit 16]
    let result: u32;
    unsafe {
        core::arch::asm!(
            "mov {tmp}, rbx",
            "mov eax, 7",
            "xor ecx, ecx",
            "cpuid",
            "xchg {tmp}, rbx",
            tmp = out(reg) _,
            out("ecx") result,
            out("eax") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
    }

    // Also check CR4.LA57 to see if it's actually enabled
    let cr4: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nomem, nostack, preserves_flags),
        );
    }

    // LA57 is bit 12 of CR4
    let la57_supported = result & (1 << 16) != 0;
    let la57_enabled = cr4 & (1 << 12) != 0;

    la57_supported && la57_enabled
}

/// Detect physical address bits from CPUID
fn detect_phys_addr_bits() -> u8 {
    // CPUID.80000008H:EAX[7:0]
    let max_leaf: u32;
    unsafe {
        core::arch::asm!(
            "mov {tmp}, rbx",
            "mov eax, 0x80000000",
            "cpuid",
            "xchg {tmp}, rbx",
            tmp = out(reg) _,
            out("eax") max_leaf,
            out("ecx") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
    }

    if max_leaf >= 0x80000008 {
        let result: u32;
        unsafe {
            core::arch::asm!(
                "mov {tmp}, rbx",
                "mov eax, 0x80000008",
                "cpuid",
                "xchg {tmp}, rbx",
                tmp = out(reg) _,
                out("eax") result,
                out("ecx") _,
                out("edx") _,
                options(nostack, preserves_flags),
            );
        }
        (result & 0xFF) as u8
    } else {
        // Default to 36 bits for older CPUs (PAE minimum)
        36
    }
}

// =============================================================================
// Kernel Address Space Constants
// =============================================================================

/// Kernel virtual address space base (higher half)
/// This is the start of kernel virtual addresses for 4-level paging.
pub const KERNEL_BASE_4LEVEL: u64 = 0xFFFF_8000_0000_0000;

/// Kernel virtual address space base for 5-level paging
pub const KERNEL_BASE_5LEVEL: u64 = 0xFF00_0000_0000_0000;

/// Get the kernel base address for the current paging mode
#[inline]
pub fn kernel_base() -> u64 {
    if is_5level_paging() {
        KERNEL_BASE_5LEVEL
    } else {
        KERNEL_BASE_4LEVEL
    }
}

/// Physical memory direct mapping base
/// Maps all physical memory starting at this virtual address.
pub const PHYS_MAP_BASE_4LEVEL: u64 = 0xFFFF_8800_0000_0000;

/// Physical memory direct mapping base for 5-level paging
pub const PHYS_MAP_BASE_5LEVEL: u64 = 0xFF80_0000_0000_0000;

/// Get the physical memory mapping base for the current mode
#[inline]
pub fn physical_memory_base() -> u64 {
    if is_5level_paging() {
        PHYS_MAP_BASE_5LEVEL
    } else {
        PHYS_MAP_BASE_4LEVEL
    }
}

/// Convert a physical address to a kernel virtual address
/// (assuming identity-mapped or direct-mapped physical memory)
#[inline]
pub fn phys_to_virt(phys: PhysicalAddress) -> VirtualAddress {
    VirtualAddress::new(physical_memory_base() + phys.as_u64())
}

/// Convert a kernel virtual address to a physical address
/// (for addresses in the direct-mapped region)
#[inline]
pub fn virt_to_phys(virt: VirtualAddress) -> Option<PhysicalAddress> {
    let base = physical_memory_base();
    if virt.as_u64() >= base {
        Some(PhysicalAddress::new(virt.as_u64() - base))
    } else {
        None
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    // Verify sizes
    assert!(PAGE_SIZE_4K == 4096);
    assert!(PAGE_SIZE_2M == 512 * PAGE_SIZE_4K);
    assert!(PAGE_SIZE_1G == 512 * PAGE_SIZE_2M);
    assert!(TABLE_SIZE == PAGE_SIZE_4K);
    assert!(ENTRIES_PER_TABLE == 512);
};
