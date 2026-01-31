//! # RISC-V MMU (Memory Management Unit)
//!
//! Page table setup for RISC-V Sv39/Sv48/Sv57 paging modes.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// PAGING MODES
// =============================================================================

/// Paging mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingMode {
    /// No paging (bare)
    Bare,
    /// Sv39: 39-bit virtual address, 3-level page table
    Sv39,
    /// Sv48: 48-bit virtual address, 4-level page table
    Sv48,
    /// Sv57: 57-bit virtual address, 5-level page table
    Sv57,
}

impl PagingMode {
    /// Get SATP mode value
    pub fn satp_mode(&self) -> u64 {
        match self {
            PagingMode::Bare => SATP_MODE_BARE,
            PagingMode::Sv39 => SATP_MODE_SV39,
            PagingMode::Sv48 => SATP_MODE_SV48,
            PagingMode::Sv57 => SATP_MODE_SV57,
        }
    }

    /// Get number of page table levels
    pub fn levels(&self) -> usize {
        match self {
            PagingMode::Bare => 0,
            PagingMode::Sv39 => 3,
            PagingMode::Sv48 => 4,
            PagingMode::Sv57 => 5,
        }
    }

    /// Get virtual address width
    pub fn va_width(&self) -> u8 {
        match self {
            PagingMode::Bare => 64,
            PagingMode::Sv39 => 39,
            PagingMode::Sv48 => 48,
            PagingMode::Sv57 => 57,
        }
    }
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// Page table entry flags
pub mod pte_flags {
    pub const VALID: u64 = 1 << 0;
    pub const READ: u64 = 1 << 1;
    pub const WRITE: u64 = 1 << 2;
    pub const EXEC: u64 = 1 << 3;
    pub const USER: u64 = 1 << 4;
    pub const GLOBAL: u64 = 1 << 5;
    pub const ACCESSED: u64 = 1 << 6;
    pub const DIRTY: u64 = 1 << 7;

    // Combined permission masks
    pub const RW: u64 = READ | WRITE;
    pub const RX: u64 = READ | EXEC;
    pub const RWX: u64 = READ | WRITE | EXEC;
}

/// Page table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create empty (invalid) entry
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create entry pointing to next level page table
    pub fn table(ppn: u64) -> Self {
        Self((ppn << PTE_PPN_SHIFT) | PTE_V)
    }

    /// Create leaf entry for 4KB page
    pub fn page(ppn: u64, flags: u64) -> Self {
        Self((ppn << PTE_PPN_SHIFT) | flags | PTE_V)
    }

    /// Create leaf entry for megapage (2MB in Sv39, 2MB/1GB in others)
    pub fn megapage(ppn: u64, flags: u64) -> Self {
        Self((ppn << PTE_PPN_SHIFT) | flags | PTE_V)
    }

    /// Create leaf entry for gigapage (1GB)
    pub fn gigapage(ppn: u64, flags: u64) -> Self {
        Self((ppn << PTE_PPN_SHIFT) | flags | PTE_V)
    }

    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        self.0 & PTE_V != 0
    }

    /// Check if entry is a leaf (has R, W, or X set)
    pub fn is_leaf(&self) -> bool {
        self.0 & (PTE_R | PTE_W | PTE_X) != 0
    }

    /// Check if entry is a table pointer
    pub fn is_table(&self) -> bool {
        self.is_valid() && !self.is_leaf()
    }

    /// Get physical page number
    pub fn ppn(&self) -> u64 {
        (self.0 >> PTE_PPN_SHIFT) & ((1 << 44) - 1)
    }

    /// Get physical address
    pub fn phys_addr(&self) -> u64 {
        self.ppn() << 12
    }

    /// Get flags
    pub fn flags(&self) -> u64 {
        self.0 & 0xFF
    }

    /// Set accessed flag
    pub fn set_accessed(&mut self) {
        self.0 |= PTE_A;
    }

    /// Set dirty flag
    pub fn set_dirty(&mut self) {
        self.0 |= PTE_D;
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.0
    }
}

// =============================================================================
// PAGE TABLE
// =============================================================================

/// Page table (512 entries, 4KB)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); 512],
        }
    }

    /// Get entry at index
    pub fn get(&self, index: usize) -> PageTableEntry {
        self.entries[index]
    }

    /// Set entry at index
    pub fn set(&mut self, index: usize, entry: PageTableEntry) {
        self.entries[index] = entry;
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry::empty();
        }
    }

    /// Get mutable reference to entries
    pub fn entries_mut(&mut self) -> &mut [PageTableEntry; 512] {
        &mut self.entries
    }
}

// =============================================================================
// FRAME ALLOCATOR
// =============================================================================

/// Simple frame allocator for early boot
pub struct FrameAllocator {
    next: u64,
    end: u64,
}

impl FrameAllocator {
    /// Create new frame allocator
    pub const fn new(start: u64, end: u64) -> Self {
        Self {
            next: (start + 0xFFF) & !0xFFF, // Align up
            end,
        }
    }

    /// Allocate a 4KB frame
    pub fn alloc(&mut self) -> Option<u64> {
        if self.next + PAGE_SIZE_4K > self.end {
            return None;
        }
        let frame = self.next;
        self.next += PAGE_SIZE_4K;
        Some(frame)
    }

    /// Allocate a 4KB frame and zero it
    pub fn alloc_zeroed(&mut self) -> Option<u64> {
        let frame = self.alloc()?;
        unsafe {
            core::ptr::write_bytes(frame as *mut u8, 0, PAGE_SIZE_4K as usize);
        }
        Some(frame)
    }

    /// Get number of remaining frames
    pub fn remaining(&self) -> u64 {
        (self.end - self.next) / PAGE_SIZE_4K
    }
}

// =============================================================================
// ADDRESS TRANSLATION HELPERS
// =============================================================================

/// Get VPN[0] (bits 12-20)
pub fn vpn0(va: u64) -> usize {
    ((va >> 12) & 0x1FF) as usize
}

/// Get VPN[1] (bits 21-29)
pub fn vpn1(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}

/// Get VPN[2] (bits 30-38)
pub fn vpn2(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}

/// Get VPN[3] (bits 39-47) - Sv48/Sv57 only
pub fn vpn3(va: u64) -> usize {
    ((va >> 39) & 0x1FF) as usize
}

/// Get VPN[4] (bits 48-56) - Sv57 only
pub fn vpn4(va: u64) -> usize {
    ((va >> 48) & 0x1FF) as usize
}

/// Get page offset (bits 0-11)
pub fn page_offset(va: u64) -> usize {
    (va & 0xFFF) as usize
}

/// Align address down to page boundary
pub fn page_align_down(addr: u64) -> u64 {
    addr & !0xFFF
}

/// Align address up to page boundary
pub fn page_align_up(addr: u64) -> u64 {
    (addr + 0xFFF) & !0xFFF
}

/// Physical address to PPN
pub fn pa_to_ppn(pa: u64) -> u64 {
    pa >> 12
}

/// PPN to physical address
pub fn ppn_to_pa(ppn: u64) -> u64 {
    ppn << 12
}

// =============================================================================
// PAGE TABLE WALKING
// =============================================================================

/// Walk page tables and return leaf entry
pub unsafe fn walk(
    root: *const PageTable,
    va: u64,
    mode: PagingMode,
) -> Option<(PageTableEntry, usize)> {
    let levels = mode.levels();
    if levels == 0 {
        return None;
    }

    let vpns = [vpn0(va), vpn1(va), vpn2(va), vpn3(va), vpn4(va)];
    let mut table = root;

    for level in (0..levels).rev() {
        let entry = (*table).get(vpns[level]);

        if !entry.is_valid() {
            return None;
        }

        if entry.is_leaf() {
            return Some((entry, level));
        }

        // Follow table pointer
        table = entry.phys_addr() as *const PageTable;
    }

    None
}

/// Translate virtual address to physical address
pub unsafe fn translate(root: *const PageTable, va: u64, mode: PagingMode) -> Option<u64> {
    let (entry, level) = walk(root, va, mode)?;

    // Calculate physical address based on page size
    let page_offset_bits = 12 + level * 9;
    let page_offset_mask = (1u64 << page_offset_bits) - 1;
    let ppn_shifted = entry.ppn() << 12;

    Some((ppn_shifted & !page_offset_mask) | (va & page_offset_mask))
}

// =============================================================================
// PAGE TABLE MAPPING
// =============================================================================

/// Map a 4KB page
pub unsafe fn map_page(
    root: *mut PageTable,
    va: u64,
    pa: u64,
    flags: u64,
    allocator: &mut FrameAllocator,
    mode: PagingMode,
) -> BootResult<()> {
    let levels = mode.levels();
    let vpns = [vpn0(va), vpn1(va), vpn2(va), vpn3(va), vpn4(va)];
    let mut table = root;

    // Walk down to level 0
    for level in (1..levels).rev() {
        let entry = (*table).get(vpns[level]);

        if !entry.is_valid() {
            // Allocate new page table
            let new_table = allocator.alloc_zeroed().ok_or(BootError::OutOfMemory)?;
            (*table).set(vpns[level], PageTableEntry::table(pa_to_ppn(new_table)));
            table = new_table as *mut PageTable;
        } else if entry.is_leaf() {
            // Already mapped as large page
            return Err(BootError::AlreadyMapped);
        } else {
            table = entry.phys_addr() as *mut PageTable;
        }
    }

    // Set leaf entry
    let entry = (*table).get(vpns[0]);
    if entry.is_valid() {
        return Err(BootError::AlreadyMapped);
    }

    (*table).set(
        vpns[0],
        PageTableEntry::page(pa_to_ppn(pa), flags | PTE_A | PTE_D),
    );

    Ok(())
}

/// Map a 2MB megapage (Sv39/Sv48/Sv57)
pub unsafe fn map_megapage(
    root: *mut PageTable,
    va: u64,
    pa: u64,
    flags: u64,
    allocator: &mut FrameAllocator,
    mode: PagingMode,
) -> BootResult<()> {
    let levels = mode.levels();
    if levels < 2 {
        return Err(BootError::InvalidParameter(
            "Mode doesn't support megapages".into(),
        ));
    }

    let vpns = [vpn0(va), vpn1(va), vpn2(va), vpn3(va), vpn4(va)];
    let mut table = root;

    // Walk down to level 1
    for level in (2..levels).rev() {
        let entry = (*table).get(vpns[level]);

        if !entry.is_valid() {
            let new_table = allocator.alloc_zeroed().ok_or(BootError::OutOfMemory)?;
            (*table).set(vpns[level], PageTableEntry::table(pa_to_ppn(new_table)));
            table = new_table as *mut PageTable;
        } else if entry.is_leaf() {
            return Err(BootError::AlreadyMapped);
        } else {
            table = entry.phys_addr() as *mut PageTable;
        }
    }

    // Set megapage at level 1
    let entry = (*table).get(vpns[1]);
    if entry.is_valid() {
        return Err(BootError::AlreadyMapped);
    }

    (*table).set(
        vpns[1],
        PageTableEntry::megapage(pa_to_ppn(pa), flags | PTE_A | PTE_D),
    );

    Ok(())
}

/// Map a 1GB gigapage (Sv39/Sv48/Sv57)
pub unsafe fn map_gigapage(
    root: *mut PageTable,
    va: u64,
    pa: u64,
    flags: u64,
    allocator: &mut FrameAllocator,
    mode: PagingMode,
) -> BootResult<()> {
    let levels = mode.levels();
    if levels < 3 {
        return Err(BootError::InvalidParameter(
            "Mode doesn't support gigapages".into(),
        ));
    }

    let vpns = [vpn0(va), vpn1(va), vpn2(va), vpn3(va), vpn4(va)];
    let mut table = root;

    // Walk down to level 2
    for level in (3..levels).rev() {
        let entry = (*table).get(vpns[level]);

        if !entry.is_valid() {
            let new_table = allocator.alloc_zeroed().ok_or(BootError::OutOfMemory)?;
            (*table).set(vpns[level], PageTableEntry::table(pa_to_ppn(new_table)));
            table = new_table as *mut PageTable;
        } else if entry.is_leaf() {
            return Err(BootError::AlreadyMapped);
        } else {
            table = entry.phys_addr() as *mut PageTable;
        }
    }

    // Set gigapage at level 2
    let entry = (*table).get(vpns[2]);
    if entry.is_valid() {
        return Err(BootError::AlreadyMapped);
    }

    (*table).set(
        vpns[2],
        PageTableEntry::gigapage(pa_to_ppn(pa), flags | PTE_A | PTE_D),
    );

    Ok(())
}

// =============================================================================
// PAGE TABLE SETUP
// =============================================================================

/// Current paging mode
static PAGING_MODE: AtomicU64 = AtomicU64::new(0);

/// Root page table address
static ROOT_TABLE: AtomicU64 = AtomicU64::new(0);

/// Detect supported paging mode
pub fn detect_paging_mode() -> PagingMode {
    // Try to enable Sv57
    let satp_sv57 = (SATP_MODE_SV57 << SATP_MODE_SHIFT) | 1; // Dummy PPN
    write_satp(satp_sv57);
    fence();
    if read_satp() != 0 {
        write_satp(0);
        return PagingMode::Sv57;
    }

    // Try Sv48
    let satp_sv48 = (SATP_MODE_SV48 << SATP_MODE_SHIFT) | 1;
    write_satp(satp_sv48);
    fence();
    if read_satp() != 0 {
        write_satp(0);
        return PagingMode::Sv48;
    }

    // Default to Sv39
    PagingMode::Sv39
}

/// Setup page tables
pub unsafe fn setup_page_tables(
    ctx: &mut BootContext,
    allocator: &mut FrameAllocator,
) -> BootResult<u64> {
    // Detect paging mode
    let mode = detect_paging_mode();
    PAGING_MODE.store(mode as u64, Ordering::SeqCst);

    // Allocate root page table
    let root = allocator.alloc_zeroed().ok_or(BootError::OutOfMemory)?;
    ROOT_TABLE.store(root, Ordering::SeqCst);

    let root_table = root as *mut PageTable;

    // Map kernel (example: 0xFFFFFFFF80000000 -> 0x80000000)
    let kernel_vbase = 0xFFFF_FFFF_8000_0000u64;
    let kernel_pbase = 0x8000_0000u64;
    let kernel_size = 0x4000_0000u64; // 1GB

    // Identity map low memory (first 1GB)
    for offset in (0..0x4000_0000u64).step_by(PAGE_SIZE_1G as usize) {
        map_gigapage(root_table, offset, offset, pte_flags::RWX, allocator, mode)?;
    }

    // Map kernel in high memory
    for offset in (0..kernel_size).step_by(PAGE_SIZE_2M as usize) {
        map_megapage(
            root_table,
            kernel_vbase + offset,
            kernel_pbase + offset,
            pte_flags::RWX | PTE_G,
            allocator,
            mode,
        )?;
    }

    // Map HHDM (higher half direct map)
    let hhdm_base = 0xFFFF_8000_0000_0000u64;
    let phys_mem_size = 0x1_0000_0000u64; // 4GB for now

    for offset in (0..phys_mem_size).step_by(PAGE_SIZE_1G as usize) {
        map_gigapage(
            root_table,
            hhdm_base + offset,
            offset,
            pte_flags::RW | PTE_G,
            allocator,
            mode,
        )?;
    }

    Ok(root)
}

/// Enable MMU
pub unsafe fn enable_mmu(root_table: u64, asid: u16, mode: PagingMode) {
    // Build SATP value
    let ppn = root_table >> 12;
    let satp = (mode.satp_mode() << SATP_MODE_SHIFT) | ((asid as u64) << SATP_ASID_SHIFT) | ppn;

    // Ensure page tables are visible
    fence();

    // Write SATP
    write_satp(satp);

    // Flush TLB
    sfence_vma();
}

/// Disable MMU (return to bare mode)
pub unsafe fn disable_mmu() {
    write_satp(0);
    sfence_vma();
}

/// Get current ASID
pub fn get_current_asid() -> u16 {
    let satp = read_satp();
    ((satp >> SATP_ASID_SHIFT) & 0xFFFF) as u16
}

/// Set ASID
pub unsafe fn set_asid(asid: u16) {
    let satp = read_satp();
    let satp = (satp & !(0xFFFF << SATP_ASID_SHIFT)) | ((asid as u64) << SATP_ASID_SHIFT);
    write_satp(satp);
    sfence_vma();
}

// =============================================================================
// TLB MANAGEMENT
// =============================================================================

/// Flush entire TLB
pub fn flush_tlb_all() {
    sfence_vma();
}

/// Flush TLB entry for address
pub fn flush_tlb_addr(va: u64) {
    sfence_vma_addr(va);
}

/// Flush TLB entries for ASID
pub fn flush_tlb_asid(asid: u64) {
    sfence_vma_asid(0, asid);
}

/// Flush TLB entry for address and ASID
pub fn flush_tlb_addr_asid(va: u64, asid: u64) {
    sfence_vma_asid(va, asid);
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize MMU
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get memory region for page tables
    let pt_region_start = ctx.boot_info.page_table_region.start;
    let pt_region_end = ctx.boot_info.page_table_region.end;

    if pt_region_start == 0 || pt_region_end == 0 {
        // Use default region if not specified
        // Assume we have memory at 0x8100_0000 for page tables
        return Err(BootError::InvalidParameter("No page table region".into()));
    }

    let mut allocator = FrameAllocator::new(pt_region_start, pt_region_end);

    // Setup page tables
    let root = setup_page_tables(ctx, &mut allocator)?;

    // Detect paging mode
    let mode = detect_paging_mode();

    // Store in context
    ctx.arch_data.riscv.satp_mode = mode as u8;
    ctx.arch_data.riscv.root_page_table = root;

    // Enable MMU
    enable_mmu(root, 0, mode);

    Ok(())
}
