//! # AArch64 MMU Setup
//!
//! Memory Management Unit initialization with 4KB/16KB/64KB granules.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::{BootContext, PagingMode};
use crate::error::{BootError, BootResult};
use crate::info::MemoryType;

// =============================================================================
// PAGE TABLE CONSTANTS
// =============================================================================

/// Page size (4KB granule)
pub const PAGE_SIZE: u64 = 0x1000;

/// Large page size (2MB block)
pub const LARGE_PAGE_SIZE: u64 = 0x200000;

/// Huge page size (1GB block)
pub const HUGE_PAGE_SIZE: u64 = 0x40000000;

/// Entries per table (4KB granule)
pub const ENTRIES_PER_TABLE: usize = 512;

/// Virtual address space size (48 bits)
pub const VA_BITS: u64 = 48;

/// Physical address mask
pub const PHYS_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

/// Output address mask for blocks
pub const BLOCK_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

/// Higher-half direct map base
pub const HHDM_BASE: u64 = 0xFFFF_0000_0000_0000;

/// Kernel virtual base
pub const KERNEL_VIRT_BASE: u64 = 0xFFFF_FFFF_8000_0000;

// =============================================================================
// DESCRIPTOR FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Page/Block descriptor flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        /// Valid entry
        const VALID = 1 << 0;
        /// Table descriptor (vs block)
        const TABLE = 1 << 1;
        /// Access flag
        const AF = 1 << 10;
        /// Not global (for TLB)
        const NG = 1 << 11;
        /// Contiguous hint
        const CONTIGUOUS = 1 << 52;
        /// Privileged execute-never
        const PXN = 1 << 53;
        /// Unprivileged execute-never / Execute-never
        const UXN = 1 << 54;

        // AP (Access Permission) field bits [7:6]
        /// Read/Write at EL1, no access at EL0
        const AP_RW_EL1 = 0b00 << 6;
        /// Read/Write at all ELs
        const AP_RW_ALL = 0b01 << 6;
        /// Read-only at EL1, no access at EL0
        const AP_RO_EL1 = 0b10 << 6;
        /// Read-only at all ELs
        const AP_RO_ALL = 0b11 << 6;

        // SH (Shareability) field bits [9:8]
        /// Non-shareable
        const SH_NON = 0b00 << 8;
        /// Outer shareable
        const SH_OUTER = 0b10 << 8;
        /// Inner shareable
        const SH_INNER = 0b11 << 8;
    }
}

impl PageFlags {
    /// Block descriptor (not table)
    pub const BLOCK: Self = Self::from_bits_truncate(Self::VALID.bits());

    /// Table descriptor
    pub const TABLE_DESC: Self = Self::from_bits_truncate(Self::VALID.bits() | Self::TABLE.bits());

    /// Kernel code (read + execute)
    pub const KERNEL_CODE: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::AP_RO_EL1.bits()
            | Self::SH_INNER.bits()
            | Self::UXN.bits(), // Only executable by kernel
    );

    /// Kernel data (read + write, no execute)
    pub const KERNEL_DATA: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::AP_RW_EL1.bits()
            | Self::SH_INNER.bits()
            | Self::PXN.bits()
            | Self::UXN.bits(),
    );

    /// Kernel read-only data
    pub const KERNEL_RODATA: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::AP_RO_EL1.bits()
            | Self::SH_INNER.bits()
            | Self::PXN.bits()
            | Self::UXN.bits(),
    );

    /// User code
    pub const USER_CODE: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::NG.bits()
            | Self::AP_RO_ALL.bits()
            | Self::SH_INNER.bits()
            | Self::PXN.bits(), // Not executable by kernel
    );

    /// User data
    pub const USER_DATA: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::NG.bits()
            | Self::AP_RW_ALL.bits()
            | Self::SH_INNER.bits()
            | Self::PXN.bits()
            | Self::UXN.bits(),
    );

    /// Device memory (nGnRnE)
    pub const DEVICE: Self = Self::from_bits_truncate(
        Self::VALID.bits()
            | Self::AF.bits()
            | Self::AP_RW_EL1.bits()
            | Self::SH_NON.bits()
            | Self::PXN.bits()
            | Self::UXN.bits(),
    );

    /// Set MAIR index
    pub fn with_mair_index(self, index: u64) -> Self {
        Self::from_bits_truncate(self.bits() | ((index & 0x7) << 2))
    }
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// A page table entry (descriptor)
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an invalid entry
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a table descriptor
    pub const fn table(addr: u64) -> Self {
        Self((addr & PHYS_ADDR_MASK) | PageFlags::TABLE_DESC.bits())
    }

    /// Create a block descriptor
    pub fn block(addr: u64, flags: PageFlags, mair_index: u64) -> Self {
        Self((addr & BLOCK_ADDR_MASK) | flags.bits() | ((mair_index & 0x7) << 2))
    }

    /// Create a page descriptor (level 3)
    pub fn page(addr: u64, flags: PageFlags, mair_index: u64) -> Self {
        Self(
            (addr & PHYS_ADDR_MASK) |
            flags.bits() |
            PageFlags::TABLE.bits() | // Page descriptors have bit 1 set
            ((mair_index & 0x7) << 2),
        )
    }

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.0 & 1 != 0
    }

    /// Check if table descriptor
    pub const fn is_table(&self) -> bool {
        (self.0 & 0b11) == 0b11
    }

    /// Check if block descriptor
    pub const fn is_block(&self) -> bool {
        (self.0 & 0b11) == 0b01
    }

    /// Get output address
    pub const fn addr(&self) -> u64 {
        self.0 & PHYS_ADDR_MASK
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

// =============================================================================
// PAGE TABLE
// =============================================================================

/// A page table (512 entries for 4KB granule)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    /// Create empty page table
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry::empty(); ENTRIES_PER_TABLE],
        }
    }

    /// Get entry
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }

    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self as *const _ as u64
    }
}

// =============================================================================
// ADDRESS TRANSLATION HELPERS
// =============================================================================

/// Get level 0 index (PGD)
pub const fn l0_index(va: u64) -> usize {
    ((va >> 39) & 0x1FF) as usize
}

/// Get level 1 index (PUD)
pub const fn l1_index(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}

/// Get level 2 index (PMD)
pub const fn l2_index(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}

/// Get level 3 index (PTE)
pub const fn l3_index(va: u64) -> usize {
    ((va >> 12) & 0x1FF) as usize
}

/// Align down
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Align up
pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// Check if aligned
pub const fn is_aligned(addr: u64, align: u64) -> bool {
    addr & (align - 1) == 0
}

// =============================================================================
// FRAME ALLOCATOR
// =============================================================================

/// Simple frame allocator for early boot
static FRAME_ALLOCATOR: FrameAllocator = FrameAllocator::new();

struct FrameAllocator {
    next: AtomicU64,
    end: AtomicU64,
}

impl FrameAllocator {
    const fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            end: AtomicU64::new(0),
        }
    }

    fn init(&self, start: u64, end: u64) {
        self.next
            .store(align_up(start, PAGE_SIZE), Ordering::SeqCst);
        self.end.store(align_down(end, PAGE_SIZE), Ordering::SeqCst);
    }

    fn alloc(&self) -> Option<u64> {
        loop {
            let current = self.next.load(Ordering::SeqCst);
            let end = self.end.load(Ordering::SeqCst);

            if current >= end {
                return None;
            }

            if self
                .next
                .compare_exchange(
                    current,
                    current + PAGE_SIZE,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                // Zero the frame
                unsafe {
                    core::ptr::write_bytes(current as *mut u8, 0, PAGE_SIZE as usize);
                }
                return Some(current);
            }
        }
    }
}

/// Allocate a page table
unsafe fn alloc_page_table() -> Option<&'static mut PageTable> {
    let frame = FRAME_ALLOCATOR.alloc()?;
    Some(&mut *(frame as *mut PageTable))
}

// =============================================================================
// STATIC PAGE TABLES
// =============================================================================

/// Level 0 page table (TTBR0 - user space)
#[repr(C, align(4096))]
static mut TTBR0_L0: PageTable = PageTable::empty();

/// Level 0 page table (TTBR1 - kernel space)
#[repr(C, align(4096))]
static mut TTBR1_L0: PageTable = PageTable::empty();

// =============================================================================
// MMU INITIALIZATION
// =============================================================================

/// Set up page tables
///
/// # Safety
///
/// The caller must ensure the page table pointer is valid and properly aligned.
pub unsafe fn setup_page_tables(ctx: &mut BootContext) -> BootResult<()> {
    // Find memory for page table allocator
    let (alloc_start, alloc_end) = find_page_table_memory(ctx)?;
    FRAME_ALLOCATOR.init(alloc_start, alloc_end);

    // Clear static tables
    TTBR0_L0.clear();
    TTBR1_L0.clear();

    // Set up MAIR
    setup_mair();

    // Set up identity mapping (for boot transition)
    setup_identity_mapping()?;

    // Set up HHDM
    setup_hhdm(ctx)?;

    // Set up kernel mapping
    setup_kernel_mapping(ctx)?;

    // Set up TCR
    setup_tcr(ctx)?;

    // Load page tables
    load_page_tables();

    // Enable MMU
    enable_mmu();

    // Update context
    ctx.memory_state.paging_mode = PagingMode::Level4; // 4KB granule, 4-level
    ctx.memory_state.page_table_base = &raw const TTBR1_L0 as u64;
    ctx.memory_state.hhdm_base = HHDM_BASE;

    Ok(())
}

/// Find memory for page tables
fn find_page_table_memory(ctx: &BootContext) -> BootResult<(u64, u64)> {
    if let Some(ref memory) = ctx.boot_info.memory {
        for entry in memory.entries.iter() {
            if entry.memory_type == MemoryType::Usable && entry.size >= 32 * 1024 * 1024 {
                let start = entry.base + 16 * 1024 * 1024;
                let end = entry.base + entry.size;
                return Ok((start, end));
            }
        }
    }

    // Fallback
    Ok((0x4000_0000, 0x6000_0000))
}

/// Set up MAIR
unsafe fn setup_mair() {
    let mair: u64 = (MAIR_DEVICE_nGnRnE << (MAIR_IDX_DEVICE * 8))
        | (MAIR_NORMAL_NC << (MAIR_IDX_NORMAL_NC * 8))
        | (MAIR_NORMAL_WB << (MAIR_IDX_NORMAL * 8));

    core::arch::asm!(
        "msr MAIR_EL1, {}",
        in(reg) mair,
        options(nomem, nostack)
    );
}

/// Set up TCR
unsafe fn setup_tcr(ctx: &BootContext) -> BootResult<()> {
    let pa_bits = ctx.arch_data.arm.pa_bits;
    let ips = cpu::get_ips_value(pa_bits);

    // TCR for 48-bit VA, 4KB granule
    let t0sz: u64 = 64 - VA_BITS;
    let t1sz: u64 = 64 - VA_BITS;

    let tcr: u64 = (t0sz << TCR_T0SZ_SHIFT) |
        (t1sz << TCR_T1SZ_SHIFT) |
        (TCR_RGN_WB_WA << TCR_IRGN0_SHIFT) |
        (TCR_RGN_WB_WA << TCR_ORGN0_SHIFT) |
        (TCR_SH_INNER << TCR_SH0_SHIFT) |
        TCR_TG0_4KB |
        TCR_TG1_4KB |
        (ips << TCR_IPS_SHIFT) |
        (TCR_RGN_WB_WA << 24) | // IRGN1
        (TCR_RGN_WB_WA << 26) | // ORGN1
        (TCR_SH_INNER << 28); // SH1

    core::arch::asm!(
        "msr TCR_EL1, {}",
        in(reg) tcr,
        options(nomem, nostack)
    );
    isb();

    Ok(())
}

/// Set up identity mapping
unsafe fn setup_identity_mapping() -> BootResult<()> {
    // Map first 4GB with 1GB blocks
    for i in 0..4 {
        let addr = i as u64 * HUGE_PAGE_SIZE;
        let entry = PageTableEntry::block(addr, PageFlags::KERNEL_DATA, MAIR_IDX_NORMAL);
        *TTBR0_L0.entry_mut(i) = entry;
    }

    Ok(())
}

/// Set up HHDM
unsafe fn setup_hhdm(ctx: &BootContext) -> BootResult<()> {
    // Create L1 table for HHDM
    let l1 = alloc_page_table().ok_or(BootError::OutOfMemory)?;

    // Map first 512GB with 1GB blocks
    for i in 0..512 {
        let phys = i as u64 * HUGE_PAGE_SIZE;
        l1.entries[i] = PageTableEntry::block(phys, PageFlags::KERNEL_DATA, MAIR_IDX_NORMAL);
    }

    // Link to L0
    let l0_idx = l0_index(HHDM_BASE);
    TTBR1_L0.entries[l0_idx] = PageTableEntry::table(l1.phys_addr());

    Ok(())
}

/// Set up kernel mapping
unsafe fn setup_kernel_mapping(ctx: &BootContext) -> BootResult<()> {
    let kernel_phys = ctx.boot_info.kernel_phys_base;
    let kernel_size = ctx.boot_info.kernel_size;

    if kernel_phys == 0 || kernel_size == 0 {
        return Ok(());
    }

    // Create L1, L2, L3 tables as needed
    let l1 = alloc_page_table().ok_or(BootError::OutOfMemory)?;
    let l2 = alloc_page_table().ok_or(BootError::OutOfMemory)?;

    // Map kernel pages
    let num_pages = align_up(kernel_size, PAGE_SIZE) / PAGE_SIZE;

    let mut current_l3: Option<&'static mut PageTable> = None;
    let mut current_l3_idx: usize = usize::MAX;

    for i in 0..num_pages {
        let virt = KERNEL_VIRT_BASE + i * PAGE_SIZE;
        let phys = kernel_phys + i * PAGE_SIZE;

        let l2_idx = l2_index(virt);
        let l3_idx = l3_index(virt);

        // Check if we need a new L3 table
        if l2_idx != current_l3_idx {
            let new_l3 = alloc_page_table().ok_or(BootError::OutOfMemory)?;
            l2.entries[l2_idx] = PageTableEntry::table(new_l3.phys_addr());
            current_l3 = Some(new_l3);
            current_l3_idx = l2_idx;
        }

        // Determine flags based on section
        let flags = if i < num_pages / 3 {
            PageFlags::KERNEL_CODE
        } else if i < num_pages * 2 / 3 {
            PageFlags::KERNEL_RODATA
        } else {
            PageFlags::KERNEL_DATA
        };

        if let Some(ref mut l3) = current_l3 {
            l3.entries[l3_idx] = PageTableEntry::page(phys, flags, MAIR_IDX_NORMAL);
        }
    }

    // Link tables
    let l1_idx = l1_index(KERNEL_VIRT_BASE);
    l1.entries[l1_idx] = PageTableEntry::table(l2.phys_addr());

    let l0_idx = l0_index(KERNEL_VIRT_BASE);
    TTBR1_L0.entries[l0_idx] = PageTableEntry::table(l1.phys_addr());

    Ok(())
}

/// Load page tables
unsafe fn load_page_tables() {
    let ttbr0 = &raw const TTBR0_L0 as u64;
    let ttbr1 = &raw const TTBR1_L0 as u64;

    core::arch::asm!(
        "msr TTBR0_EL1, {}",
        "msr TTBR1_EL1, {}",
        in(reg) ttbr0,
        in(reg) ttbr1,
        options(nomem, nostack)
    );

    // Ensure writes are visible
    dsb();
    isb();

    // Invalidate TLB
    invalidate_tlb();
}

/// Enable MMU
unsafe fn enable_mmu() {
    let mut sctlr: u64;
    core::arch::asm!("mrs {}, SCTLR_EL1", out(reg) sctlr, options(nomem, nostack));

    // Enable MMU, data cache, instruction cache
    sctlr |= SCTLR_M | SCTLR_C | SCTLR_I;

    core::arch::asm!("msr SCTLR_EL1, {}", in(reg) sctlr, options(nomem, nostack));
    isb();
}

// =============================================================================
// MAPPING FUNCTIONS
// =============================================================================

/// Map a 4KB page
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_4kb_page(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    let l0 = if virt >= 0xFFFF_0000_0000_0000 {
        &mut TTBR1_L0
    } else {
        &mut TTBR0_L0
    };

    // Get or create L1
    let l1 = get_or_create_table(l0, l0_index(virt))?;

    // Get or create L2
    let l2 = get_or_create_table(l1, l1_index(virt))?;

    // Get or create L3
    let l3 = get_or_create_table(l2, l2_index(virt))?;

    // Set L3 entry
    l3.entries[l3_index(virt)] = PageTableEntry::page(phys, flags, MAIR_IDX_NORMAL);

    // Invalidate TLB for this address
    core::arch::asm!(
        "tlbi vaae1, {}",
        "dsb ish",
        "isb",
        in(reg) virt >> 12,
        options(nostack)
    );

    Ok(())
}

/// Map a 2MB block
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_2mb_block(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    if !is_aligned(virt, LARGE_PAGE_SIZE) || !is_aligned(phys, LARGE_PAGE_SIZE) {
        return Err(BootError::InvalidAddress);
    }

    let l0 = if virt >= 0xFFFF_0000_0000_0000 {
        &mut TTBR1_L0
    } else {
        &mut TTBR0_L0
    };

    let l1 = get_or_create_table(l0, l0_index(virt))?;
    let l2 = get_or_create_table(l1, l1_index(virt))?;

    // Set L2 entry as block
    l2.entries[l2_index(virt)] = PageTableEntry::block(phys, flags, MAIR_IDX_NORMAL);

    core::arch::asm!(
        "tlbi vaae1, {}",
        "dsb ish",
        "isb",
        in(reg) virt >> 12,
        options(nostack)
    );

    Ok(())
}

/// Map a 1GB block
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_1gb_block(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    if !is_aligned(virt, HUGE_PAGE_SIZE) || !is_aligned(phys, HUGE_PAGE_SIZE) {
        return Err(BootError::InvalidAddress);
    }

    let l0 = if virt >= 0xFFFF_0000_0000_0000 {
        &mut TTBR1_L0
    } else {
        &mut TTBR0_L0
    };

    let l1 = get_or_create_table(l0, l0_index(virt))?;

    // Set L1 entry as block
    l1.entries[l1_index(virt)] = PageTableEntry::block(phys, flags, MAIR_IDX_NORMAL);

    core::arch::asm!(
        "tlbi vaae1, {}",
        "dsb ish",
        "isb",
        in(reg) virt >> 12,
        options(nostack)
    );

    Ok(())
}

/// Get or create a page table
unsafe fn get_or_create_table(
    parent: &mut PageTable,
    index: usize,
) -> BootResult<&'static mut PageTable> {
    let entry = &mut parent.entries[index];

    if entry.is_valid() && entry.is_table() {
        Ok(&mut *(entry.addr() as *mut PageTable))
    } else {
        let table = alloc_page_table().ok_or(BootError::OutOfMemory)?;
        *entry = PageTableEntry::table(table.phys_addr());
        Ok(table)
    }
}

// =============================================================================
// KASLR
// =============================================================================

/// Apply KASLR offset
///
/// # Safety
///
/// The caller must ensure page tables support the randomization offset.
pub unsafe fn apply_kaslr(ctx: &mut BootContext, offset: u64) -> BootResult<()> {
    let new_base = KERNEL_VIRT_BASE + offset;

    // Remap kernel at new address
    let kernel_phys = ctx.boot_info.kernel_phys_base;
    let kernel_size = ctx.boot_info.kernel_size;

    if kernel_phys == 0 || kernel_size == 0 {
        return Ok(());
    }

    let num_pages = align_up(kernel_size, PAGE_SIZE) / PAGE_SIZE;

    for i in 0..num_pages {
        let virt = new_base + i * PAGE_SIZE;
        let phys = kernel_phys + i * PAGE_SIZE;

        let flags = if i < num_pages / 3 {
            PageFlags::KERNEL_CODE
        } else if i < num_pages * 2 / 3 {
            PageFlags::KERNEL_RODATA
        } else {
            PageFlags::KERNEL_DATA
        };

        map_4kb_page(virt, phys, flags)?;
    }

    // Update context
    ctx.boot_info.kernel_virt_base = new_base;
    ctx.kaslr_offset = offset;

    // Full TLB flush
    invalidate_tlb();

    Ok(())
}

// =============================================================================
// DEVICE MAPPING
// =============================================================================

/// Map device memory
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_device(virt: u64, phys: u64, size: u64) -> BootResult<()> {
    let mut addr = 0u64;
    while addr < size {
        if is_aligned(virt + addr, LARGE_PAGE_SIZE)
            && is_aligned(phys + addr, LARGE_PAGE_SIZE)
            && addr + LARGE_PAGE_SIZE <= size
        {
            map_2mb_block(
                virt + addr,
                phys + addr,
                PageFlags::DEVICE.with_mair_index(MAIR_IDX_DEVICE),
            )?;
            addr += LARGE_PAGE_SIZE;
        } else {
            map_4kb_page(
                virt + addr,
                phys + addr,
                PageFlags::DEVICE.with_mair_index(MAIR_IDX_DEVICE),
            )?;
            addr += PAGE_SIZE;
        }
    }
    Ok(())
}
