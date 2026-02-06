//! Memory Allocator
//!
//! Page and pool allocators for UEFI boot environment.

use core::alloc::{GlobalAlloc, Layout};
use core::fmt;
use core::ptr::NonNull;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Page size (4 KB)
pub const PAGE_SIZE: usize = 4096;

/// Page size as u64
pub const PAGE_SIZE_U64: u64 = PAGE_SIZE as u64;

/// Large page size (2 MB)
pub const LARGE_PAGE_SIZE: usize = 2 * 1024 * 1024;

/// Huge page size (1 GB)
pub const HUGE_PAGE_SIZE: usize = 1024 * 1024 * 1024;

// =============================================================================
// PAGE FRAME ALLOCATOR
// =============================================================================

/// Memory region descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Base physical address
    pub base: u64,
    /// Size in bytes
    pub size: u64,
    /// Memory type
    pub memory_type: MemoryType,
}

impl MemoryRegion {
    /// Create new region
    pub const fn new(base: u64, size: u64, memory_type: MemoryType) -> Self {
        Self {
            base,
            size,
            memory_type,
        }
    }

    /// End address
    pub fn end(&self) -> u64 {
        self.base + self.size
    }

    /// Page count
    pub fn page_count(&self) -> u64 {
        self.size / PAGE_SIZE_U64
    }

    /// Contains address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.end()
    }

    /// Overlaps with another region
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.base < other.end() && other.base < self.end()
    }
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Available for use
    Available,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Loader code
    LoaderCode,
    /// Loader data
    LoaderData,
    /// Boot services code
    BootServicesCode,
    /// Boot services data
    BootServicesData,
    /// Runtime services code
    RuntimeServicesCode,
    /// Runtime services data
    RuntimeServicesData,
    /// Conventional (usable)
    Conventional,
    /// Unusable
    Unusable,
    /// Memory mapped I/O
    Mmio,
    /// Memory mapped I/O port space
    MmioPortSpace,
    /// PAL code (Itanium)
    PalCode,
    /// Persistent memory
    PersistentMemory,
}

impl MemoryType {
    /// Is usable for allocation
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            Self::Available
                | Self::Conventional
                | Self::BootServicesCode
                | Self::BootServicesData
                | Self::LoaderCode
                | Self::LoaderData
        )
    }

    /// Is reclaimable after boot
    pub fn is_reclaimable(&self) -> bool {
        matches!(
            self,
            Self::BootServicesCode
                | Self::BootServicesData
                | Self::LoaderCode
                | Self::LoaderData
                | Self::AcpiReclaimable
        )
    }

    /// Should be preserved for runtime
    pub fn is_runtime(&self) -> bool {
        matches!(
            self,
            Self::RuntimeServicesCode | Self::RuntimeServicesData | Self::AcpiNvs
        )
    }
}

/// Bitmap page frame allocator
pub struct BitmapAllocator {
    /// Bitmap storage
    bitmap: [u64; 1024], // 64K pages = 256 MB
    /// Base address
    base_address: u64,
    /// Total pages
    total_pages: usize,
    /// Free pages
    free_pages: usize,
    /// Next search hint
    next_free: usize,
}

impl Default for BitmapAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapAllocator {
    /// Bits per entry
    const BITS_PER_ENTRY: usize = 64;

    /// Maximum pages
    const MAX_PAGES: usize = 1024 * 64;

    /// Create new allocator
    pub const fn new() -> Self {
        Self {
            bitmap: [0; 1024],
            base_address: 0,
            total_pages: 0,
            free_pages: 0,
            next_free: 0,
        }
    }

    /// Initialize with memory region
    pub fn init(&mut self, base: u64, size: u64) {
        self.base_address = base;
        self.total_pages = (size as usize / PAGE_SIZE).min(Self::MAX_PAGES);
        self.free_pages = self.total_pages;
        self.next_free = 0;

        // Clear bitmap (all free)
        for entry in &mut self.bitmap {
            *entry = 0;
        }
    }

    /// Allocate pages
    pub fn allocate(&mut self, count: usize) -> Option<u64> {
        if count == 0 || count > self.free_pages {
            return None;
        }

        // Find contiguous free pages
        let mut start_page = None;
        let mut consecutive = 0;

        // Start from hint
        for i in self.next_free..self.total_pages {
            if self.is_page_free(i) {
                if start_page.is_none() {
                    start_page = Some(i);
                }
                consecutive += 1;
                if consecutive >= count {
                    break;
                }
            } else {
                start_page = None;
                consecutive = 0;
            }
        }

        // Wrap around if needed
        if consecutive < count {
            for i in 0..self.next_free {
                if self.is_page_free(i) {
                    if start_page.is_none() {
                        start_page = Some(i);
                    }
                    consecutive += 1;
                    if consecutive >= count {
                        break;
                    }
                } else {
                    start_page = None;
                    consecutive = 0;
                }
            }
        }

        // Allocate if found
        let start = start_page?;
        if consecutive < count {
            return None;
        }

        for i in start..start + count {
            self.mark_page_used(i);
        }

        self.free_pages -= count;
        self.next_free = start + count;

        Some(self.base_address + (start as u64 * PAGE_SIZE_U64))
    }

    /// Allocate aligned pages
    pub fn allocate_aligned(&mut self, count: usize, alignment_pages: usize) -> Option<u64> {
        if count == 0 || count > self.free_pages || alignment_pages == 0 {
            return None;
        }

        let mut start_page = None;

        for i in (0..self.total_pages).step_by(alignment_pages) {
            let mut found = true;
            for j in 0..count {
                if i + j >= self.total_pages || !self.is_page_free(i + j) {
                    found = false;
                    break;
                }
            }

            if found {
                start_page = Some(i);
                break;
            }
        }

        let start = start_page?;

        for i in start..start + count {
            self.mark_page_used(i);
        }

        self.free_pages -= count;

        Some(self.base_address + (start as u64 * PAGE_SIZE_U64))
    }

    /// Free pages
    pub fn free(&mut self, address: u64, count: usize) {
        if address < self.base_address {
            return;
        }

        let page = ((address - self.base_address) / PAGE_SIZE_U64) as usize;

        for i in page..page + count {
            if i < self.total_pages {
                self.mark_page_free(i);
                self.free_pages += 1;
            }
        }

        self.next_free = page;
    }

    /// Check if page is free
    fn is_page_free(&self, page: usize) -> bool {
        let index = page / Self::BITS_PER_ENTRY;
        let bit = page % Self::BITS_PER_ENTRY;

        if index >= self.bitmap.len() {
            return false;
        }

        self.bitmap[index] & (1u64 << bit) == 0
    }

    /// Mark page as used
    fn mark_page_used(&mut self, page: usize) {
        let index = page / Self::BITS_PER_ENTRY;
        let bit = page % Self::BITS_PER_ENTRY;

        if index < self.bitmap.len() {
            self.bitmap[index] |= 1u64 << bit;
        }
    }

    /// Mark page as free
    fn mark_page_free(&mut self, page: usize) {
        let index = page / Self::BITS_PER_ENTRY;
        let bit = page % Self::BITS_PER_ENTRY;

        if index < self.bitmap.len() {
            self.bitmap[index] &= !(1u64 << bit);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> AllocatorStats {
        AllocatorStats {
            total_pages: self.total_pages,
            free_pages: self.free_pages,
            used_pages: self.total_pages - self.free_pages,
            base_address: self.base_address,
        }
    }
}

/// Allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub total_pages: usize,
    pub free_pages: usize,
    pub used_pages: usize,
    pub base_address: u64,
}

impl AllocatorStats {
    /// Total bytes
    pub fn total_bytes(&self) -> usize {
        self.total_pages * PAGE_SIZE
    }

    /// Free bytes
    pub fn free_bytes(&self) -> usize {
        self.free_pages * PAGE_SIZE
    }

    /// Used bytes
    pub fn used_bytes(&self) -> usize {
        self.used_pages * PAGE_SIZE
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.total_pages == 0 {
            return 0;
        }
        ((self.used_pages * 100) / self.total_pages) as u8
    }
}

// =============================================================================
// POOL ALLOCATOR
// =============================================================================

/// Block header for pool allocator
#[repr(C)]
struct BlockHeader {
    /// Size of the block (including header)
    size: usize,
    /// Is the block free
    is_free: bool,
    /// Magic for validation
    magic: u32,
    /// Next block in list
    next: Option<NonNull<BlockHeader>>,
    /// Previous block in list
    prev: Option<NonNull<BlockHeader>>,
}

impl BlockHeader {
    const MAGIC: u32 = 0xDEAD_BEEF;

    /// Header size (aligned to 16 bytes)
    const SIZE: usize = 48;

    /// Create new header
    fn new(size: usize) -> Self {
        Self {
            size,
            is_free: false,
            magic: Self::MAGIC,
            next: None,
            prev: None,
        }
    }

    /// Is valid
    fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }

    /// Get data pointer
    fn data_ptr(&self) -> *mut u8 {
        let header_ptr = self as *const Self as *mut u8;
        unsafe { header_ptr.add(Self::SIZE) }
    }

    /// Get usable size
    fn usable_size(&self) -> usize {
        self.size.saturating_sub(Self::SIZE)
    }
}

/// Pool allocator (first-fit with coalescing)
pub struct PoolAllocator {
    /// Pool start
    pool_start: *mut u8,
    /// Pool size
    pool_size: usize,
    /// Free list head
    free_list: Option<NonNull<BlockHeader>>,
    /// Total allocated
    allocated: usize,
    /// Peak allocated
    peak_allocated: usize,
    /// Allocation count
    allocation_count: usize,
}

impl Default for PoolAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolAllocator {
    /// Minimum block size
    const MIN_BLOCK_SIZE: usize = BlockHeader::SIZE + 16;

    /// Create new pool allocator
    pub const fn new() -> Self {
        Self {
            pool_start: core::ptr::null_mut(),
            pool_size: 0,
            free_list: None,
            allocated: 0,
            peak_allocated: 0,
            allocation_count: 0,
        }
    }

    /// Initialize with memory pool
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init(&mut self, pool: *mut u8, size: usize) {
        self.pool_start = pool;
        self.pool_size = size;
        self.allocated = 0;
        self.peak_allocated = 0;
        self.allocation_count = 0;

        // Create initial free block
        let header = pool as *mut BlockHeader;
        core::ptr::write(header, BlockHeader::new(size));
        (*header).is_free = true;

        self.free_list = NonNull::new(header);
    }

    /// Allocate memory
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        if size == 0 {
            return None;
        }

        // Calculate required size (aligned)
        let align = align.max(16);
        let required_size = align_up(size + BlockHeader::SIZE, align);

        // Find first fit
        let mut current = self.free_list;

        while let Some(block_ptr) = current {
            let block = unsafe { block_ptr.as_ref() };

            if block.is_free && block.size >= required_size {
                // Found a suitable block
                let block_mut = unsafe { &mut *block_ptr.as_ptr() };

                // Split if possible
                if block.size >= required_size + Self::MIN_BLOCK_SIZE {
                    let new_block_addr =
                        (block_ptr.as_ptr() as usize + required_size) as *mut BlockHeader;

                    unsafe {
                        let new_block = BlockHeader::new(block.size - required_size);
                        core::ptr::write(new_block_addr, new_block);
                        (*new_block_addr).is_free = true;
                        (*new_block_addr).next = block.next;
                        (*new_block_addr).prev = Some(block_ptr);

                        if let Some(next) = block.next {
                            (*next.as_ptr()).prev = NonNull::new(new_block_addr);
                        }

                        block_mut.next = NonNull::new(new_block_addr);
                        block_mut.size = required_size;
                    }
                }

                block_mut.is_free = false;
                self.allocated += block_mut.size;
                self.allocation_count += 1;

                if self.allocated > self.peak_allocated {
                    self.peak_allocated = self.allocated;
                }

                return Some(block_mut.data_ptr());
            }

            current = block.next;
        }

        None // Out of memory
    }

    /// Free memory
    pub fn free(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        // Find header
        let header_ptr = unsafe { ptr.sub(BlockHeader::SIZE) } as *mut BlockHeader;
        let header = unsafe { &mut *header_ptr };

        if !header.is_valid() || header.is_free {
            return; // Invalid or already free
        }

        // Mark as free
        self.allocated -= header.size;
        header.is_free = true;

        // Coalesce with next block
        if let Some(next) = header.next {
            let next_block = unsafe { &mut *next.as_ptr() };
            if next_block.is_valid() && next_block.is_free {
                header.size += next_block.size;
                header.next = next_block.next;

                if let Some(next_next) = header.next {
                    unsafe { (*next_next.as_ptr()).prev = NonNull::new(header_ptr) };
                }
            }
        }

        // Coalesce with previous block
        if let Some(prev) = header.prev {
            let prev_block = unsafe { &mut *prev.as_ptr() };
            if prev_block.is_valid() && prev_block.is_free {
                prev_block.size += header.size;
                prev_block.next = header.next;

                if let Some(next) = header.next {
                    unsafe { (*next.as_ptr()).prev = Some(prev) };
                }
            }
        }
    }

    /// Reallocate memory
    pub fn reallocate(&mut self, ptr: *mut u8, new_size: usize) -> Option<*mut u8> {
        if ptr.is_null() {
            return self.allocate(new_size, 16);
        }

        if new_size == 0 {
            self.free(ptr);
            return None;
        }

        // Get current block
        let header_ptr = unsafe { ptr.sub(BlockHeader::SIZE) } as *mut BlockHeader;
        let header = unsafe { &*header_ptr };

        if !header.is_valid() {
            return None;
        }

        let current_size = header.usable_size();

        // If current block is big enough, keep it
        if current_size >= new_size {
            return Some(ptr);
        }

        // Allocate new block
        let new_ptr = self.allocate(new_size, 16)?;

        // Copy data
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, current_size);
        }

        // Free old block
        self.free(ptr);

        Some(new_ptr)
    }

    /// Get allocation info
    pub fn info(&self, ptr: *const u8) -> Option<AllocationInfo> {
        if ptr.is_null() {
            return None;
        }

        let header_ptr = unsafe { ptr.sub(BlockHeader::SIZE) } as *const BlockHeader;
        let header = unsafe { &*header_ptr };

        if !header.is_valid() {
            return None;
        }

        Some(AllocationInfo {
            size: header.usable_size(),
            is_free: header.is_free,
        })
    }

    /// Get statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            pool_size: self.pool_size,
            allocated: self.allocated,
            peak_allocated: self.peak_allocated,
            allocation_count: self.allocation_count,
            free: self.pool_size.saturating_sub(self.allocated),
        }
    }

    /// Validate heap integrity
    pub fn validate(&self) -> bool {
        let mut current = self.free_list;
        let mut visited = 0usize;

        while let Some(block_ptr) = current {
            let block = unsafe { block_ptr.as_ref() };

            if !block.is_valid() {
                return false;
            }

            visited += 1;
            if visited > self.pool_size / Self::MIN_BLOCK_SIZE {
                return false; // Probably a loop
            }

            current = block.next;
        }

        true
    }
}

unsafe impl Send for PoolAllocator {}
unsafe impl Sync for PoolAllocator {}

/// Allocation info
#[derive(Debug, Clone, Copy)]
pub struct AllocationInfo {
    pub size: usize,
    pub is_free: bool,
}

/// Pool statistics
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    pub pool_size: usize,
    pub allocated: usize,
    pub peak_allocated: usize,
    pub allocation_count: usize,
    pub free: usize,
}

impl PoolStats {
    /// Usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.pool_size == 0 {
            return 0;
        }
        ((self.allocated * 100) / self.pool_size) as u8
    }

    /// Fragmentation estimate
    pub fn fragmentation_percent(&self) -> u8 {
        if self.peak_allocated == 0 {
            return 0;
        }

        let ratio = (self.peak_allocated - self.allocated) * 100 / self.peak_allocated;
        ratio.min(100) as u8
    }
}

// =============================================================================
// GLOBAL ALLOCATOR
// =============================================================================

/// Boot allocator for global allocation
pub struct BootAllocator {
    pool: core::cell::UnsafeCell<PoolAllocator>,
}

// SAFETY: Access is protected at a higher level (single-threaded boot environment)
unsafe impl Sync for BootAllocator {}

impl Default for BootAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl BootAllocator {
    /// Create new allocator
    pub const fn new() -> Self {
        Self {
            pool: core::cell::UnsafeCell::new(PoolAllocator::new()),
        }
    }

    /// Initialize with memory
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init(&mut self, pool_mem: *mut u8, size: usize) {
        (*self.pool.get()).init(pool_mem, size);
    }
}

unsafe impl GlobalAlloc for BootAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pool = &mut *self.pool.get();
        pool.allocate(layout.size(), layout.align())
            .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let pool = &mut *self.pool.get();
        pool.free(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        let pool = &mut *self.pool.get();
        pool.reallocate(ptr, new_size)
            .unwrap_or(core::ptr::null_mut())
    }
}

// =============================================================================
// BUMP ALLOCATOR (SIMPLE, FAST)
// =============================================================================

/// Simple bump allocator (arena style)
pub struct BumpAllocator {
    /// Start of memory region
    start: usize,
    /// End of memory region
    end: usize,
    /// Current allocation pointer
    next: usize,
    /// Allocation count
    allocations: usize,
}

impl Default for BumpAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl BumpAllocator {
    /// Create new bump allocator
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize with memory region
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init(&mut self, start: *mut u8, size: usize) {
        self.start = start as usize;
        self.end = self.start + size;
        self.next = self.start;
        self.allocations = 0;
    }

    /// Allocate memory
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let aligned_next = align_up(self.next, align);
        let alloc_end = aligned_next.checked_add(size)?;

        if alloc_end > self.end {
            return None;
        }

        self.next = alloc_end;
        self.allocations += 1;

        Some(aligned_next as *mut u8)
    }

    /// Allocate zeroed memory
    pub fn allocate_zeroed(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let ptr = self.allocate(size, align)?;
        unsafe {
            core::ptr::write_bytes(ptr, 0, size);
        }
        Some(ptr)
    }

    /// Reset allocator (free all)
    pub fn reset(&mut self) {
        self.next = self.start;
        self.allocations = 0;
    }

    /// Get statistics
    pub fn stats(&self) -> BumpStats {
        BumpStats {
            total: self.end - self.start,
            used: self.next - self.start,
            free: self.end - self.next,
            allocations: self.allocations,
        }
    }
}

/// Bump allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct BumpStats {
    pub total: usize,
    pub used: usize,
    pub free: usize,
    pub allocations: usize,
}

// =============================================================================
// STACK ALLOCATOR
// =============================================================================

/// Stack allocator (LIFO)
pub struct StackAllocator {
    /// Memory region
    memory: *mut u8,
    /// Total size
    size: usize,
    /// Current offset
    offset: usize,
    /// Markers for deallocation
    markers: [usize; 32],
    /// Marker count
    marker_count: usize,
}

impl Default for StackAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl StackAllocator {
    /// Create new stack allocator
    pub const fn new() -> Self {
        Self {
            memory: core::ptr::null_mut(),
            size: 0,
            offset: 0,
            markers: [0; 32],
            marker_count: 0,
        }
    }

    /// Initialize with memory
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init(&mut self, memory: *mut u8, size: usize) {
        self.memory = memory;
        self.size = size;
        self.offset = 0;
        self.marker_count = 0;
    }

    /// Allocate memory
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let aligned_offset = align_up(self.offset, align);
        let new_offset = aligned_offset.checked_add(size)?;

        if new_offset > self.size {
            return None;
        }

        let ptr = unsafe { self.memory.add(aligned_offset) };
        self.offset = new_offset;

        Some(ptr)
    }

    /// Push marker for later rollback
    pub fn push_marker(&mut self) -> Option<usize> {
        if self.marker_count >= 32 {
            return None;
        }

        let marker = self.offset;
        self.markers[self.marker_count] = marker;
        self.marker_count += 1;

        Some(marker)
    }

    /// Pop to marker
    pub fn pop_to_marker(&mut self) -> bool {
        if self.marker_count == 0 {
            return false;
        }

        self.marker_count -= 1;
        self.offset = self.markers[self.marker_count];

        true
    }

    /// Pop to specific marker
    pub fn pop_to(&mut self, marker: usize) {
        if marker <= self.size {
            self.offset = marker;

            // Remove any markers above this
            while self.marker_count > 0 && self.markers[self.marker_count - 1] > marker {
                self.marker_count -= 1;
            }
        }
    }

    /// Reset allocator
    pub fn reset(&mut self) {
        self.offset = 0;
        self.marker_count = 0;
    }

    /// Get used size
    pub fn used(&self) -> usize {
        self.offset
    }

    /// Get free size
    pub fn free(&self) -> usize {
        self.size - self.offset
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Align up to boundary
pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Align down to boundary
pub const fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

/// Is aligned
pub const fn is_aligned(value: usize, align: usize) -> bool {
    value & (align - 1) == 0
}

/// Calculate pages needed for size
pub const fn pages_for_bytes(bytes: usize) -> usize {
    bytes.div_ceil(PAGE_SIZE)
}

/// Convert pages to bytes
pub const fn pages_to_bytes(pages: usize) -> usize {
    pages * PAGE_SIZE
}

// =============================================================================
// ALLOCATOR ERROR
// =============================================================================

/// Allocator error
#[derive(Debug, Clone)]
pub enum AllocError {
    /// Out of memory
    OutOfMemory,
    /// Invalid size
    InvalidSize,
    /// Invalid alignment
    InvalidAlignment,
    /// Invalid pointer
    InvalidPointer,
    /// Double free
    DoubleFree,
    /// Corruption detected
    Corruption,
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::InvalidSize => write!(f, "invalid size"),
            Self::InvalidAlignment => write!(f, "invalid alignment"),
            Self::InvalidPointer => write!(f, "invalid pointer"),
            Self::DoubleFree => write!(f, "double free"),
            Self::Corruption => write!(f, "heap corruption"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(7, 8), 8);
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0, 4), 0);
        assert_eq!(align_down(1, 4), 0);
        assert_eq!(align_down(4, 4), 4);
        assert_eq!(align_down(7, 4), 4);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0, 4));
        assert!(!is_aligned(1, 4));
        assert!(is_aligned(4, 4));
        assert!(!is_aligned(5, 4));
        assert!(is_aligned(4096, 4096));
    }

    #[test]
    fn test_pages_for_bytes() {
        assert_eq!(pages_for_bytes(0), 0);
        assert_eq!(pages_for_bytes(1), 1);
        assert_eq!(pages_for_bytes(4096), 1);
        assert_eq!(pages_for_bytes(4097), 2);
        assert_eq!(pages_for_bytes(8192), 2);
    }

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::new(0x1000, 0x2000, MemoryType::Available);
        assert_eq!(region.end(), 0x3000);
        assert!(region.contains(0x1000));
        assert!(region.contains(0x2FFF));
        assert!(!region.contains(0x3000));
    }

    #[test]
    fn test_memory_type() {
        assert!(MemoryType::Available.is_usable());
        assert!(MemoryType::Conventional.is_usable());
        assert!(!MemoryType::Reserved.is_usable());

        assert!(MemoryType::BootServicesCode.is_reclaimable());
        assert!(!MemoryType::RuntimeServicesCode.is_reclaimable());

        assert!(MemoryType::RuntimeServicesCode.is_runtime());
        assert!(!MemoryType::Available.is_runtime());
    }
}
