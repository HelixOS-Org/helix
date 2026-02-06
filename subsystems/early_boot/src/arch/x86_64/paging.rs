//! # x86_64 Paging Setup
//!
//! 4-level and 5-level paging implementation for x86_64.
//! Supports identity mapping, higher-half direct map, and kernel mapping.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{BootContext, PagingMode};
use crate::error::{BootError, BootResult};
use crate::info::MemoryType;

// =============================================================================
// PAGE TABLE CONSTANTS
// =============================================================================

/// Page size (4 KB)
pub const PAGE_SIZE: u64 = 0x1000;

/// Large page size (2 MB)
pub const LARGE_PAGE_SIZE: u64 = 0x200000;

/// Huge page size (1 GB)
pub const HUGE_PAGE_SIZE: u64 = 0x40000000;

/// Entries per page table
pub const ENTRIES_PER_TABLE: usize = 512;

/// Physical address mask (for extracting address from entry)
pub const PHYS_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Higher-half direct map base (canonical address)
pub const HHDM_BASE: u64 = 0xFFFF_8000_0000_0000;

/// Kernel virtual base
pub const KERNEL_VIRT_BASE: u64 = 0xFFFF_FFFF_8000_0000;

// =============================================================================
// PAGE TABLE ENTRY FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        /// Present bit - page is mapped
        const PRESENT = 1 << 0;
        /// Writable - page can be written
        const WRITABLE = 1 << 1;
        /// User accessible - ring 3 can access
        const USER = 1 << 2;
        /// Write-through caching
        const WRITE_THROUGH = 1 << 3;
        /// Cache disable
        const NO_CACHE = 1 << 4;
        /// Accessed - set by CPU
        const ACCESSED = 1 << 5;
        /// Dirty - page was written
        const DIRTY = 1 << 6;
        /// Huge page (PS bit) - 2MB or 1GB page
        const HUGE = 1 << 7;
        /// Global - don't flush from TLB
        const GLOBAL = 1 << 8;
        /// Available for OS use (bit 9)
        const OS_BIT9 = 1 << 9;
        /// Available for OS use (bit 10)
        const OS_BIT10 = 1 << 10;
        /// Available for OS use (bit 11)
        const OS_BIT11 = 1 << 11;
        /// PAT bit (for 4KB pages)
        const PAT = 1 << 7; // Same as HUGE for non-leaf
        /// Protection key (bits 59-62)
        const PKEY_BIT0 = 1 << 59;
        const PKEY_BIT1 = 1 << 60;
        const PKEY_BIT2 = 1 << 61;
        const PKEY_BIT3 = 1 << 62;
        /// No-execute bit
        const NO_EXECUTE = 1 << 63;
    }
}

impl PageFlags {
    /// Kernel code flags (read + execute)
    pub const KERNEL_CODE: Self =
        Self::from_bits_truncate(Self::PRESENT.bits() | Self::GLOBAL.bits());

    /// Kernel data flags (read + write, no execute)
    pub const KERNEL_DATA: Self = Self::from_bits_truncate(
        Self::PRESENT.bits()
            | Self::WRITABLE.bits()
            | Self::GLOBAL.bits()
            | Self::NO_EXECUTE.bits(),
    );

    /// Kernel read-only data
    pub const KERNEL_RODATA: Self = Self::from_bits_truncate(
        Self::PRESENT.bits() | Self::GLOBAL.bits() | Self::NO_EXECUTE.bits(),
    );

    /// User code flags
    pub const USER_CODE: Self = Self::from_bits_truncate(Self::PRESENT.bits() | Self::USER.bits());

    /// User data flags
    pub const USER_DATA: Self = Self::from_bits_truncate(
        Self::PRESENT.bits() | Self::WRITABLE.bits() | Self::USER.bits() | Self::NO_EXECUTE.bits(),
    );

    /// Device memory flags (no cache)
    pub const DEVICE: Self = Self::from_bits_truncate(
        Self::PRESENT.bits()
            | Self::WRITABLE.bits()
            | Self::NO_CACHE.bits()
            | Self::WRITE_THROUGH.bits()
            | Self::NO_EXECUTE.bits(),
    );
}

// =============================================================================
// PAGE TABLE ENTRY
// =============================================================================

/// A single page table entry
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an empty entry
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create an entry with flags
    pub const fn new(addr: u64, flags: PageFlags) -> Self {
        Self((addr & PHYS_ADDR_MASK) | flags.bits())
    }

    /// Check if present
    pub const fn is_present(&self) -> bool {
        self.0 & PageFlags::PRESENT.bits() != 0
    }

    /// Check if huge page
    pub const fn is_huge(&self) -> bool {
        self.0 & PageFlags::HUGE.bits() != 0
    }

    /// Get physical address
    pub const fn addr(&self) -> u64 {
        self.0 & PHYS_ADDR_MASK
    }

    /// Get flags
    pub const fn flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0)
    }

    /// Set entry
    pub fn set(&mut self, addr: u64, flags: PageFlags) {
        self.0 = (addr & PHYS_ADDR_MASK) | flags.bits();
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

// =============================================================================
// PAGE TABLE
// =============================================================================

/// A page table (PML4, PDPT, PD, or PT)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
}

impl PageTable {
    /// Create an empty page table
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry::empty(); ENTRIES_PER_TABLE],
        }
    }

    /// Get entry by index
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry by index
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }

    /// Get physical address of this table
    pub fn phys_addr(&self) -> u64 {
        self as *const _ as u64
    }
}

// =============================================================================
// ADDRESS TRANSLATION HELPERS
// =============================================================================

/// Extract PML4 index from virtual address
pub const fn pml4_index(va: u64) -> usize {
    ((va >> 39) & 0x1FF) as usize
}

/// Extract PML5 index from virtual address (for 5-level paging)
pub const fn pml5_index(va: u64) -> usize {
    ((va >> 48) & 0x1FF) as usize
}

/// Extract PDPT index from virtual address
pub const fn pdpt_index(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}

/// Extract PD index from virtual address
pub const fn pd_index(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}

/// Extract PT index from virtual address
pub const fn pt_index(va: u64) -> usize {
    ((va >> 12) & 0x1FF) as usize
}

/// Extract page offset from virtual address
pub const fn page_offset(va: u64) -> u64 {
    va & 0xFFF
}

/// Align address down to page boundary
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Align address up to page boundary
pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// Check if address is page-aligned
pub const fn is_aligned(addr: u64, align: u64) -> bool {
    addr & (align - 1) == 0
}

// =============================================================================
// PHYSICAL FRAME ALLOCATOR (SIMPLE BUMP ALLOCATOR)
// =============================================================================

/// Simple frame allocator for early boot
pub struct BootFrameAllocator {
    /// Next free frame
    next_frame: AtomicU64,
    /// End of allocatable memory
    end_frame: u64,
    /// Total frames allocated
    allocated_count: AtomicU64,
}

impl BootFrameAllocator {
    /// Create a new allocator
    pub const fn new() -> Self {
        Self {
            next_frame: AtomicU64::new(0),
            end_frame: 0,
            allocated_count: AtomicU64::new(0),
        }
    }

    /// Initialize with memory region
    pub fn init(&mut self, start: u64, end: u64) {
        self.next_frame
            .store(align_up(start, PAGE_SIZE), Ordering::SeqCst);
        self.end_frame = align_down(end, PAGE_SIZE);
    }

    /// Allocate a frame (4KB)
    pub fn alloc_frame(&self) -> Option<u64> {
        loop {
            let current = self.next_frame.load(Ordering::SeqCst);
            if current >= self.end_frame {
                return None;
            }

            let next = current + PAGE_SIZE;
            if self
                .next_frame
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.allocated_count.fetch_add(1, Ordering::SeqCst);

                // Zero the frame
                unsafe {
                    core::ptr::write_bytes(current as *mut u8, 0, PAGE_SIZE as usize);
                }

                return Some(current);
            }
        }
    }

    /// Allocate multiple contiguous frames
    pub fn alloc_frames(&self, count: u64) -> Option<u64> {
        let size = count * PAGE_SIZE;
        loop {
            let current = self.next_frame.load(Ordering::SeqCst);
            if current + size > self.end_frame {
                return None;
            }

            let next = current + size;
            if self
                .next_frame
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.allocated_count.fetch_add(count, Ordering::SeqCst);

                // Zero the frames
                unsafe {
                    core::ptr::write_bytes(current as *mut u8, 0, size as usize);
                }

                return Some(current);
            }
        }
    }

    /// Get number of allocated frames
    pub fn allocated(&self) -> u64 {
        self.allocated_count.load(Ordering::SeqCst)
    }
}

/// Global frame allocator
static mut FRAME_ALLOCATOR: BootFrameAllocator = BootFrameAllocator::new();

/// Initialize frame allocator
///
/// # Safety
///
/// The caller must ensure memory regions are valid and not already in use.
pub unsafe fn init_frame_allocator(start: u64, end: u64) {
    FRAME_ALLOCATOR.init(start, end);
}

/// Allocate a frame
///
/// # Safety
///
/// The caller must ensure the allocator is properly initialized.
pub unsafe fn alloc_frame() -> Option<u64> {
    FRAME_ALLOCATOR.alloc_frame()
}

/// Allocate a page table
///
/// # Safety
///
/// The caller must ensure the allocator is properly initialized.
pub unsafe fn alloc_page_table() -> Option<&'static mut PageTable> {
    let frame = alloc_frame()?;
    Some(&mut *(frame as *mut PageTable))
}

// =============================================================================
// STATIC PAGE TABLES
// =============================================================================

/// Boot PML4 table
#[repr(C, align(4096))]
static mut BOOT_PML4: PageTable = PageTable::empty();

/// PML5 table (for 5-level paging)
#[repr(C, align(4096))]
static mut BOOT_PML5: PageTable = PageTable::empty();

/// Whether 5-level paging is enabled
static mut USE_5_LEVEL_PAGING: bool = false;

// =============================================================================
// PAGING SETUP
// =============================================================================

/// Check if 5-level paging is supported
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized.
pub unsafe fn supports_5_level_paging() -> bool {
    let (_, _, ecx, _) = super::cpuid(7, 0);
    (ecx & (1 << 16)) != 0 // LA57 bit
}

/// Set up initial page tables
///
/// # Safety
///
/// The caller must ensure the page table pointer is valid and properly aligned.
pub unsafe fn setup_page_tables(ctx: &mut BootContext) -> BootResult<()> {
    // Determine paging mode
    let use_5_level = ctx.config.enable_5_level_paging && supports_5_level_paging();
    USE_5_LEVEL_PAGING = use_5_level;

    // Find a suitable memory region for page tables
    let allocator_start = find_page_table_memory(ctx)?;
    let allocator_end = allocator_start + 16 * 1024 * 1024; // 16 MB for page tables
    init_frame_allocator(allocator_start, allocator_end);

    // Clear boot page tables
    BOOT_PML4.clear();
    if use_5_level {
        BOOT_PML5.clear();
    }

    // Set up identity mapping for first 4GB (needed for early boot)
    setup_identity_mapping(0, 4 * 1024 * 1024 * 1024)?;

    // Set up HHDM (higher-half direct map)
    setup_hhdm(ctx)?;

    // Set up kernel mapping
    setup_kernel_mapping(ctx)?;

    // Load new page tables
    if use_5_level {
        enable_5_level_paging()?;
    } else {
        enable_4_level_paging()?;
    }

    // Update context
    ctx.memory_state.paging_mode = if use_5_level {
        PagingMode::Level5
    } else {
        PagingMode::Level4
    };
    ctx.memory_state.page_table_base = if use_5_level {
        &raw const BOOT_PML5 as u64
    } else {
        &raw const BOOT_PML4 as u64
    };
    ctx.memory_state.hhdm_base = HHDM_BASE;

    Ok(())
}

/// Find memory for page tables
fn find_page_table_memory(ctx: &BootContext) -> BootResult<u64> {
    // Look for usable memory in boot info
    if let Some(ref memory) = ctx.boot_info.memory {
        for entry in memory.entries.iter() {
            if entry.memory_type == MemoryType::Usable && entry.size >= 32 * 1024 * 1024 {
                // Found suitable region
                return Ok(entry.base + 16 * 1024 * 1024); // Skip first 16MB
            }
        }
    }

    // Fallback: use memory after 16MB
    Ok(16 * 1024 * 1024)
}

/// Set up identity mapping
unsafe fn setup_identity_mapping(start: u64, end: u64) -> BootResult<()> {
    let mut addr = align_down(start, HUGE_PAGE_SIZE);

    while addr < end {
        // Use 1GB pages if possible
        if addr % HUGE_PAGE_SIZE == 0 && addr + HUGE_PAGE_SIZE <= end {
            map_1gb_page(addr, addr, PageFlags::KERNEL_DATA)?;
            addr += HUGE_PAGE_SIZE;
        }
        // Use 2MB pages
        else if addr % LARGE_PAGE_SIZE == 0 && addr + LARGE_PAGE_SIZE <= end {
            map_2mb_page(addr, addr, PageFlags::KERNEL_DATA)?;
            addr += LARGE_PAGE_SIZE;
        }
        // Use 4KB pages
        else {
            map_4kb_page(addr, addr, PageFlags::KERNEL_DATA)?;
            addr += PAGE_SIZE;
        }
    }

    Ok(())
}

/// Set up higher-half direct map
unsafe fn setup_hhdm(_ctx: &BootContext) -> BootResult<()> {
    // Map first 512GB of physical memory to HHDM
    // Using 1GB pages for efficiency
    for i in 0..512 {
        let phys = i as u64 * HUGE_PAGE_SIZE;
        let virt = HHDM_BASE + phys;
        map_1gb_page(virt, phys, PageFlags::KERNEL_DATA)?;
    }

    Ok(())
}

/// Set up kernel mapping
unsafe fn setup_kernel_mapping(ctx: &BootContext) -> BootResult<()> {
    // Map kernel from boot info
    let kernel_phys = ctx.boot_info.kernel_phys_base;
    let kernel_size = ctx.boot_info.kernel_size;

    if kernel_phys == 0 || kernel_size == 0 {
        // No kernel info, skip
        return Ok(());
    }

    let kernel_virt = KERNEL_VIRT_BASE;
    let num_pages = align_up(kernel_size, PAGE_SIZE) / PAGE_SIZE;

    for i in 0..num_pages {
        let phys = kernel_phys + i * PAGE_SIZE;
        let virt = kernel_virt + i * PAGE_SIZE;

        // Determine flags based on section (simplified)
        let flags = if i < num_pages / 3 {
            PageFlags::KERNEL_CODE // Text section
        } else if i < num_pages * 2 / 3 {
            PageFlags::KERNEL_RODATA // Rodata section
        } else {
            PageFlags::KERNEL_DATA // Data section
        };

        map_4kb_page(virt, phys, flags)?;
    }

    Ok(())
}

// =============================================================================
// PAGE MAPPING FUNCTIONS
// =============================================================================

/// Map a 4KB page
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_4kb_page(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    let pml4 = &mut BOOT_PML4;

    // Get or create PDPT
    let pdpt = get_or_create_table(pml4, pml4_index(virt))?;

    // Get or create PD
    let pd = get_or_create_table(pdpt, pdpt_index(virt))?;

    // Get or create PT
    let pt = get_or_create_table(pd, pd_index(virt))?;

    // Set PT entry
    let entry = pt.entry_mut(pt_index(virt));
    entry.set(phys, flags);

    // Flush TLB for this page
    super::invlpg(virt);

    Ok(())
}

/// Map a 2MB large page
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_2mb_page(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    if !is_aligned(virt, LARGE_PAGE_SIZE) || !is_aligned(phys, LARGE_PAGE_SIZE) {
        return Err(BootError::InvalidAddress);
    }

    let pml4 = &mut BOOT_PML4;

    // Get or create PDPT
    let pdpt = get_or_create_table(pml4, pml4_index(virt))?;

    // Get or create PD
    let pd = get_or_create_table(pdpt, pdpt_index(virt))?;

    // Set PD entry with HUGE flag
    let entry = pd.entry_mut(pd_index(virt));
    entry.set(phys, flags | PageFlags::HUGE);

    // Flush TLB
    super::invlpg(virt);

    Ok(())
}

/// Map a 1GB huge page
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn map_1gb_page(virt: u64, phys: u64, flags: PageFlags) -> BootResult<()> {
    if !is_aligned(virt, HUGE_PAGE_SIZE) || !is_aligned(phys, HUGE_PAGE_SIZE) {
        return Err(BootError::InvalidAddress);
    }

    let pml4 = &mut BOOT_PML4;

    // Get or create PDPT
    let pdpt = get_or_create_table(pml4, pml4_index(virt))?;

    // Set PDPT entry with HUGE flag
    let entry = pdpt.entry_mut(pdpt_index(virt));
    entry.set(phys, flags | PageFlags::HUGE);

    // Flush TLB
    super::invlpg(virt);

    Ok(())
}

/// Get or create a page table at the given index
unsafe fn get_or_create_table(
    parent: &mut PageTable,
    index: usize,
) -> BootResult<&'static mut PageTable> {
    let entry = parent.entry_mut(index);

    if entry.is_present() {
        // Table exists
        Ok(&mut *(entry.addr() as *mut PageTable))
    } else {
        // Allocate new table
        let table = alloc_page_table().ok_or(BootError::OutOfMemory)?;
        entry.set(table.phys_addr(), PageFlags::PRESENT | PageFlags::WRITABLE);
        Ok(table)
    }
}

// =============================================================================
// PAGE TABLE SWITCHING
// =============================================================================

/// Enable 4-level paging with new page tables
unsafe fn enable_4_level_paging() -> BootResult<()> {
    let pml4_addr = &raw const BOOT_PML4 as u64;

    // Load CR3 with new PML4
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) pml4_addr,
        options(nostack)
    );

    Ok(())
}

/// Enable 5-level paging
unsafe fn enable_5_level_paging() -> BootResult<()> {
    // Set up PML5 to point to PML4
    let pml5_entry = BOOT_PML5.entry_mut(pml5_index(0));
    pml5_entry.set(
        &raw const BOOT_PML4 as u64,
        PageFlags::PRESENT | PageFlags::WRITABLE,
    );

    // Also map higher-half
    let pml5_hh = BOOT_PML5.entry_mut(pml5_index(HHDM_BASE));
    pml5_hh.set(
        &raw const BOOT_PML4 as u64,
        PageFlags::PRESENT | PageFlags::WRITABLE,
    );

    let pml5_addr = &raw const BOOT_PML5 as u64;

    // Enable LA57 in CR4
    let mut cr4 = super::read_cr4();
    cr4 |= super::CR4_LA57;
    super::write_cr4(cr4);

    // Load CR3 with new PML5
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) pml5_addr,
        options(nostack)
    );

    Ok(())
}

// =============================================================================
// ADDRESS TRANSLATION
// =============================================================================

/// Translate virtual address to physical (for current page tables)
///
/// # Safety
///
/// The caller must ensure all safety invariants are upheld.
pub unsafe fn virt_to_phys(virt: u64) -> Option<u64> {
    let pml4 = &BOOT_PML4;

    // PML4 lookup
    let pml4_entry = pml4.entry(pml4_index(virt));
    if !pml4_entry.is_present() {
        return None;
    }

    // PDPT lookup
    let pdpt = &*(pml4_entry.addr() as *const PageTable);
    let pdpt_entry = pdpt.entry(pdpt_index(virt));
    if !pdpt_entry.is_present() {
        return None;
    }
    if pdpt_entry.is_huge() {
        // 1GB page
        return Some(pdpt_entry.addr() | (virt & (HUGE_PAGE_SIZE - 1)));
    }

    // PD lookup
    let pd = &*(pdpt_entry.addr() as *const PageTable);
    let pd_entry = pd.entry(pd_index(virt));
    if !pd_entry.is_present() {
        return None;
    }
    if pd_entry.is_huge() {
        // 2MB page
        return Some(pd_entry.addr() | (virt & (LARGE_PAGE_SIZE - 1)));
    }

    // PT lookup
    let pt = &*(pd_entry.addr() as *const PageTable);
    let pt_entry = pt.entry(pt_index(virt));
    if !pt_entry.is_present() {
        return None;
    }

    // 4KB page
    Some(pt_entry.addr() | page_offset(virt))
}

/// Get physical address from HHDM virtual address
pub const fn hhdm_to_phys(hhdm_addr: u64) -> u64 {
    hhdm_addr - HHDM_BASE
}

/// Get HHDM virtual address from physical address
pub const fn phys_to_hhdm(phys_addr: u64) -> u64 {
    HHDM_BASE + phys_addr
}

// =============================================================================
// TLB OPERATIONS
// =============================================================================

/// Flush entire TLB
///
/// # Safety
///
/// The caller must ensure this is called when page table changes need to be visible.
pub unsafe fn flush_tlb() {
    let cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack));
    core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack));
}

/// Flush TLB for a single page
///
/// # Safety
///
/// The caller must ensure this is called when page table changes need to be visible.
pub unsafe fn flush_tlb_page(addr: u64) {
    super::invlpg(addr);
}

/// Flush TLB for a range of pages
///
/// # Safety
///
/// The caller must ensure this is called when page table changes need to be visible.
pub unsafe fn flush_tlb_range(start: u64, end: u64) {
    let mut addr = align_down(start, PAGE_SIZE);
    while addr < end {
        super::invlpg(addr);
        addr += PAGE_SIZE;
    }
}

/// Invalidate all PCID entries
///
/// # Safety
///
/// The caller must ensure this is called when page table changes need to be visible.
pub unsafe fn flush_tlb_all_pcid() {
    // INVPCID instruction if available, otherwise full flush
    core::arch::asm!(
        "mov rax, cr4",
        "btr rax, 17", // Clear PCIDE bit temporarily
        "mov cr4, rax",
        "bts rax, 17", // Set PCIDE bit
        "mov cr4, rax",
        options(nostack)
    );
}

// =============================================================================
// KASLR SUPPORT
// =============================================================================

/// Apply KASLR offset to kernel mapping
///
/// # Safety
///
/// The caller must ensure the value is valid for the current system state.
pub unsafe fn apply_kaslr_offset(ctx: &mut BootContext, offset: u64) -> BootResult<()> {
    let kernel_phys = ctx.boot_info.kernel_phys_base;
    let kernel_size = ctx.boot_info.kernel_size;

    if kernel_phys == 0 || kernel_size == 0 {
        return Ok(());
    }

    let new_virt = KERNEL_VIRT_BASE + offset;
    let num_pages = align_up(kernel_size, PAGE_SIZE) / PAGE_SIZE;

    // Map kernel at new virtual address
    for i in 0..num_pages {
        let phys = kernel_phys + i * PAGE_SIZE;
        let virt = new_virt + i * PAGE_SIZE;

        let flags = if i < num_pages / 3 {
            PageFlags::KERNEL_CODE
        } else if i < num_pages * 2 / 3 {
            PageFlags::KERNEL_RODATA
        } else {
            PageFlags::KERNEL_DATA
        };

        map_4kb_page(virt, phys, flags)?;
    }

    // Flush TLB
    flush_tlb();

    // Update context
    ctx.boot_info.kernel_virt_base = new_virt;
    ctx.kaslr_offset = offset;

    Ok(())
}

// =============================================================================
// MAPPING HELPERS
// =============================================================================

/// Map a range of physical memory
///
/// # Safety
///
/// The caller must ensure the physical and virtual address ranges are valid and not already mapped.
pub unsafe fn map_range(
    virt_start: u64,
    phys_start: u64,
    size: u64,
    flags: PageFlags,
) -> BootResult<()> {
    let mut virt = align_down(virt_start, PAGE_SIZE);
    let mut phys = align_down(phys_start, PAGE_SIZE);
    let end = align_up(virt_start + size, PAGE_SIZE);

    while virt < end {
        // Try to use largest possible page size
        let remaining = end - virt;

        if is_aligned(virt, HUGE_PAGE_SIZE)
            && is_aligned(phys, HUGE_PAGE_SIZE)
            && remaining >= HUGE_PAGE_SIZE
        {
            map_1gb_page(virt, phys, flags)?;
            virt += HUGE_PAGE_SIZE;
            phys += HUGE_PAGE_SIZE;
        } else if is_aligned(virt, LARGE_PAGE_SIZE)
            && is_aligned(phys, LARGE_PAGE_SIZE)
            && remaining >= LARGE_PAGE_SIZE
        {
            map_2mb_page(virt, phys, flags)?;
            virt += LARGE_PAGE_SIZE;
            phys += LARGE_PAGE_SIZE;
        } else {
            map_4kb_page(virt, phys, flags)?;
            virt += PAGE_SIZE;
            phys += PAGE_SIZE;
        }
    }

    Ok(())
}

/// Unmap a virtual address
///
/// # Safety
///
/// The caller must ensure the physical and virtual addresses are valid and properly aligned.
pub unsafe fn unmap_page(virt: u64) -> BootResult<()> {
    let pml4 = &mut BOOT_PML4;

    let pml4_entry = pml4.entry_mut(pml4_index(virt));
    if !pml4_entry.is_present() {
        return Ok(());
    }

    let pdpt = &mut *(pml4_entry.addr() as *mut PageTable);
    let pdpt_entry = pdpt.entry_mut(pdpt_index(virt));
    if !pdpt_entry.is_present() {
        return Ok(());
    }
    if pdpt_entry.is_huge() {
        pdpt_entry.clear();
        flush_tlb_page(virt);
        return Ok(());
    }

    let pd = &mut *(pdpt_entry.addr() as *mut PageTable);
    let pd_entry = pd.entry_mut(pd_index(virt));
    if !pd_entry.is_present() {
        return Ok(());
    }
    if pd_entry.is_huge() {
        pd_entry.clear();
        flush_tlb_page(virt);
        return Ok(());
    }

    let pt = &mut *(pd_entry.addr() as *mut PageTable);
    let pt_entry = pt.entry_mut(pt_index(virt));
    pt_entry.clear();
    flush_tlb_page(virt);

    Ok(())
}
