//! Memory Management Benchmarks
//!
//! Tests for measuring memory subsystem performance:
//! - Allocation/deallocation latency
//! - Page mapping/unmapping
//! - Memory fragmentation
//! - Cache effects

use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::{
    BenchmarkCategory, BenchmarkDef, BenchmarkId, BenchmarkSuite,
    benchmark, timing,
};

// =============================================================================
// Benchmark Registration
// =============================================================================

/// Register all memory benchmarks
pub fn register_benchmarks(suite: &BenchmarkSuite) {
    // Allocation benchmarks
    suite.register(benchmark!(
        "mem.alloc.small_16",
        BenchmarkCategory::Memory,
        bench_alloc_16
    ));
    
    suite.register(benchmark!(
        "mem.alloc.small_64",
        BenchmarkCategory::Memory,
        bench_alloc_64
    ));
    
    suite.register(benchmark!(
        "mem.alloc.small_256",
        BenchmarkCategory::Memory,
        bench_alloc_256
    ));
    
    suite.register(benchmark!(
        "mem.alloc.medium_4k",
        BenchmarkCategory::Memory,
        bench_alloc_4k
    ));
    
    suite.register(benchmark!(
        "mem.alloc.large_64k",
        BenchmarkCategory::Memory,
        bench_alloc_64k
    ));
    
    suite.register(benchmark!(
        "mem.alloc.huge_1m",
        BenchmarkCategory::Memory,
        bench_alloc_1m
    ));
    
    // Free benchmarks
    suite.register(benchmark!(
        "mem.free.small",
        BenchmarkCategory::Memory,
        bench_free_small
    ));
    
    suite.register(benchmark!(
        "mem.free.large",
        BenchmarkCategory::Memory,
        bench_free_large
    ));
    
    // Page operations
    suite.register(benchmark!(
        "mem.page.map",
        BenchmarkCategory::Memory,
        bench_page_map
    ));
    
    suite.register(benchmark!(
        "mem.page.unmap",
        BenchmarkCategory::Memory,
        bench_page_unmap
    ));
    
    suite.register(benchmark!(
        "mem.page.protect",
        BenchmarkCategory::Memory,
        bench_page_protect
    ));
    
    suite.register(benchmark!(
        "mem.page.fault",
        BenchmarkCategory::Memory,
        bench_page_fault_handler
    ));
    
    // TLB operations
    suite.register(benchmark!(
        "mem.tlb.flush_single",
        BenchmarkCategory::Memory,
        bench_tlb_flush_single
    ));
    
    suite.register(benchmark!(
        "mem.tlb.flush_all",
        BenchmarkCategory::Memory,
        bench_tlb_flush_all
    ));
    
    // Memory access patterns
    suite.register(benchmark!(
        "mem.access.sequential",
        BenchmarkCategory::Memory,
        bench_sequential_access
    ));
    
    suite.register(benchmark!(
        "mem.access.random",
        BenchmarkCategory::Memory,
        bench_random_access
    ));
    
    suite.register(benchmark!(
        "mem.access.stride",
        BenchmarkCategory::Memory,
        bench_stride_access
    ));
    
    // Allocator specific
    suite.register(benchmark!(
        "mem.allocator.buddy_split",
        BenchmarkCategory::Memory,
        bench_buddy_split
    ));
    
    suite.register(benchmark!(
        "mem.allocator.buddy_merge",
        BenchmarkCategory::Memory,
        bench_buddy_merge
    ));
    
    suite.register(benchmark!(
        "mem.allocator.slab_alloc",
        BenchmarkCategory::Memory,
        bench_slab_alloc
    ));
    
    // Region operations
    suite.register(benchmark!(
        "mem.region.create",
        BenchmarkCategory::Memory,
        bench_region_create
    ));
    
    suite.register(benchmark!(
        "mem.region.lookup",
        BenchmarkCategory::Memory,
        bench_region_lookup
    ));
    
    // Protection domain
    suite.register(benchmark!(
        "mem.protection.check",
        BenchmarkCategory::Memory,
        bench_protection_check
    ));
}

// =============================================================================
// Allocation Benchmarks
// =============================================================================

/// Allocate 16 bytes
fn bench_alloc_16() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate small allocation from slab/bump allocator
    static BUMP: AtomicUsize = AtomicUsize::new(0);
    let ptr = BUMP.fetch_add(16, Ordering::SeqCst);
    
    // Simulate alignment check and header setup
    let aligned = (ptr + 7) & !7;
    core::hint::black_box(aligned);
    
    let end = timing::read_tsc();
    end - start
}

/// Allocate 64 bytes
fn bench_alloc_64() -> u64 {
    let start = timing::read_tsc();
    
    static BUMP: AtomicUsize = AtomicUsize::new(0);
    let ptr = BUMP.fetch_add(64, Ordering::SeqCst);
    
    // Simulate slab lookup
    let slab_index = 1; // 64-byte slab
    core::hint::black_box(slab_index);
    
    // Simulate bitmap update
    static BITMAP: AtomicU64 = AtomicU64::new(0);
    let old = BITMAP.fetch_or(1 << (ptr % 64), Ordering::SeqCst);
    core::hint::black_box(old);
    
    let end = timing::read_tsc();
    end - start
}

/// Allocate 256 bytes
fn bench_alloc_256() -> u64 {
    let start = timing::read_tsc();
    
    static BUMP: AtomicUsize = AtomicUsize::new(0);
    let ptr = BUMP.fetch_add(256, Ordering::SeqCst);
    
    // Simulate slab cache lookup
    let cache_index = 3; // 256-byte cache
    core::hint::black_box(cache_index);
    
    // Simulate free list pop
    static FREE_LIST_HEAD: AtomicUsize = AtomicUsize::new(0x1000);
    let block = FREE_LIST_HEAD.swap(ptr, Ordering::SeqCst);
    core::hint::black_box(block);
    
    let end = timing::read_tsc();
    end - start
}

/// Allocate 4KB (page-sized)
fn bench_alloc_4k() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate page allocator
    static NEXT_PAGE: AtomicUsize = AtomicUsize::new(0x100000);
    let page = NEXT_PAGE.fetch_add(4096, Ordering::SeqCst);
    
    // Simulate bitmap update for page frame
    static PAGE_BITMAP: AtomicU64 = AtomicU64::new(0);
    let page_idx = (page / 4096) % 64;
    PAGE_BITMAP.fetch_or(1 << page_idx, Ordering::SeqCst);
    
    core::hint::black_box(page);
    
    let end = timing::read_tsc();
    end - start
}

/// Allocate 64KB
fn bench_alloc_64k() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate buddy allocator for large allocation
    // Order 4 (16 pages = 64KB)
    
    static BUDDY_FREE: [AtomicU64; 5] = [
        AtomicU64::new(0xFFFF),
        AtomicU64::new(0xFFFF),
        AtomicU64::new(0xFFFF),
        AtomicU64::new(0xFFFF),
        AtomicU64::new(0xFFFF),
    ];
    
    let order = 4;
    
    // Find free block at order
    let bitmap = BUDDY_FREE[order].load(Ordering::SeqCst);
    let bit = bitmap.trailing_zeros();
    
    if bit < 64 {
        BUDDY_FREE[order].fetch_and(!(1 << bit), Ordering::SeqCst);
        core::hint::black_box(bit);
    }
    
    let end = timing::read_tsc();
    end - start
}

/// Allocate 1MB
fn bench_alloc_1m() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate large allocation with potential splitting
    
    static LARGE_BITMAP: AtomicU64 = AtomicU64::new(0xFFFFFFFFFFFFFFFF);
    
    // Find contiguous region (256 pages)
    let bitmap = LARGE_BITMAP.load(Ordering::SeqCst);
    
    // Simulate search for contiguous blocks
    let mut found_start = 0;
    let mut found_len = 0;
    let mut current = 0;
    let mut len = 0;
    
    for i in 0..64 {
        if (bitmap >> i) & 1 == 1 {
            len += 1;
            if len >= 4 { // Need 4 x 64KB blocks
                found_start = current;
                found_len = len;
                break;
            }
        } else {
            current = i + 1;
            len = 0;
        }
    }
    
    if found_len >= 4 {
        // Mark as allocated
        let mask = ((1u64 << 4) - 1) << found_start;
        LARGE_BITMAP.fetch_and(!mask, Ordering::SeqCst);
    }
    
    core::hint::black_box(found_start);
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Free Benchmarks
// =============================================================================

/// Free small allocation
fn bench_free_small() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate free list push
    static FREE_LIST: AtomicUsize = AtomicUsize::new(0);
    let ptr = 0x1000usize;
    
    // Add to free list (lock-free)
    let old_head = FREE_LIST.swap(ptr, Ordering::SeqCst);
    
    // Simulate writing next pointer
    core::hint::black_box(old_head);
    
    let end = timing::read_tsc();
    end - start
}

/// Free large allocation
fn bench_free_large() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate buddy merge
    static BUDDY_FREE: [AtomicU64; 5] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];
    
    let block = 4usize;
    let order = 2;
    
    // Free block
    BUDDY_FREE[order].fetch_or(1 << block, Ordering::SeqCst);
    
    // Check buddy for merge
    let buddy = block ^ 1;
    let bitmap = BUDDY_FREE[order].load(Ordering::SeqCst);
    
    if (bitmap >> buddy) & 1 == 1 {
        // Can merge
        BUDDY_FREE[order].fetch_and(!(1 << block | 1 << buddy), Ordering::SeqCst);
        BUDDY_FREE[order + 1].fetch_or(1 << (block / 2), Ordering::SeqCst);
    }
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Page Operation Benchmarks
// =============================================================================

/// Map a page
fn bench_page_map() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate page table walk and mapping
    let virt = 0x1000_0000u64;
    let phys = 0x2000_0000u64;
    let flags = 0x003u64; // Present | Writable
    
    // PML4 lookup
    let pml4_idx = (virt >> 39) & 0x1FF;
    core::hint::black_box(pml4_idx);
    
    // PDPT lookup
    let pdpt_idx = (virt >> 30) & 0x1FF;
    core::hint::black_box(pdpt_idx);
    
    // PD lookup
    let pd_idx = (virt >> 21) & 0x1FF;
    core::hint::black_box(pd_idx);
    
    // PT lookup and write
    let pt_idx = (virt >> 12) & 0x1FF;
    let entry = phys | flags;
    core::hint::black_box(pt_idx);
    core::hint::black_box(entry);
    
    let end = timing::read_tsc();
    end - start
}

/// Unmap a page
fn bench_page_unmap() -> u64 {
    let start = timing::read_tsc();
    
    let virt = 0x1000_0000u64;
    
    // Page table walk
    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;
    
    // Clear entry
    let entry = 0u64;
    core::hint::black_box((pml4_idx, pdpt_idx, pd_idx, pt_idx, entry));
    
    // Simulate TLB invalidation
    core::hint::spin_loop();
    
    let end = timing::read_tsc();
    end - start
}

/// Change page protection
fn bench_page_protect() -> u64 {
    let start = timing::read_tsc();
    
    let virt = 0x1000_0000u64;
    let new_flags = 0x001u64; // Present only (read-only)
    
    // Page table walk
    let pt_idx = (virt >> 12) & 0x1FF;
    
    // Read-modify-write
    let old_entry = 0x2000_0003u64; // Was present|writable
    let new_entry = (old_entry & !0xFFFu64) | new_flags;
    
    core::hint::black_box((pt_idx, new_entry));
    
    // TLB invalidation
    core::hint::spin_loop();
    
    let end = timing::read_tsc();
    end - start
}

/// Page fault handler simulation
fn bench_page_fault_handler() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate page fault handling
    let fault_addr = 0x1000_1234u64;
    let error_code = 0x0002u64; // Write fault
    
    // 1. Lookup VMA
    let vma = VirtualMemoryArea {
        start: 0x1000_0000,
        end: 0x1000_FFFF,
        flags: VMA_READ | VMA_WRITE,
    };
    
    // 2. Check permissions
    let is_write = (error_code & 2) != 0;
    let allowed = if is_write {
        (vma.flags & VMA_WRITE) != 0
    } else {
        (vma.flags & VMA_READ) != 0
    };
    
    // 3. Allocate physical page
    static PHYS_ALLOC: AtomicU64 = AtomicU64::new(0x3000_0000);
    let phys = if allowed {
        PHYS_ALLOC.fetch_add(4096, Ordering::SeqCst)
    } else {
        0
    };
    
    // 4. Map page
    let entry = phys | 0x003; // Present | Writable
    core::hint::black_box(entry);
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// TLB Benchmarks
// =============================================================================

/// Flush single TLB entry
fn bench_tlb_flush_single() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate INVLPG
    let addr = 0x1000_0000u64;
    core::hint::black_box(addr);
    core::hint::spin_loop(); // Simulate instruction latency
    
    let end = timing::read_tsc();
    end - start
}

/// Flush entire TLB
fn bench_tlb_flush_all() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate CR3 reload (flushes TLB)
    let cr3 = 0x0010_0000u64;
    core::hint::black_box(cr3);
    
    // This is expensive - simulate it
    for _ in 0..10 {
        core::hint::spin_loop();
    }
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Memory Access Pattern Benchmarks
// =============================================================================

/// Sequential memory access
fn bench_sequential_access() -> u64 {
    let mut buffer = [0u64; 128];
    
    let start = timing::read_tsc();
    
    // Sequential write
    for i in 0..128 {
        unsafe {
            core::ptr::write_volatile(&mut buffer[i], i as u64);
        }
    }
    
    // Sequential read
    let mut sum = 0u64;
    for i in 0..128 {
        sum += unsafe { core::ptr::read_volatile(&buffer[i]) };
    }
    
    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

/// Random memory access
fn bench_random_access() -> u64 {
    let mut buffer = [0u64; 128];
    
    // Pseudo-random indices
    let indices = [
        37, 91, 12, 65, 83, 4, 52, 98, 23, 71,
        45, 16, 88, 3, 59, 102, 28, 76, 41, 95,
        8, 63, 114, 33, 79, 19, 57, 106, 48, 84,
        1, 67, 121, 39, 86, 14, 55, 99, 26, 73,
    ];
    
    let start = timing::read_tsc();
    
    // Random write
    for &idx in &indices {
        let i = idx % 128;
        unsafe {
            core::ptr::write_volatile(&mut buffer[i], i as u64);
        }
    }
    
    // Random read
    let mut sum = 0u64;
    for &idx in &indices {
        let i = idx % 128;
        sum += unsafe { core::ptr::read_volatile(&buffer[i]) };
    }
    
    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

/// Stride memory access (cache line testing)
fn bench_stride_access() -> u64 {
    let mut buffer = [0u64; 512];
    
    let start = timing::read_tsc();
    
    // Stride = 8 (64 bytes = cache line)
    let stride = 8;
    
    // Stride write
    let mut i = 0;
    while i < 512 {
        unsafe {
            core::ptr::write_volatile(&mut buffer[i], i as u64);
        }
        i += stride;
    }
    
    // Stride read
    let mut sum = 0u64;
    let mut i = 0;
    while i < 512 {
        sum += unsafe { core::ptr::read_volatile(&buffer[i]) };
        i += stride;
    }
    
    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

// =============================================================================
// Allocator-Specific Benchmarks
// =============================================================================

/// Buddy allocator split operation
fn bench_buddy_split() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate splitting order-4 block into two order-3 blocks
    static BUDDY: [AtomicU64; 5] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0xFFFF),
    ];
    
    let order = 4;
    let block = 0;
    
    // Remove from higher order
    BUDDY[order].fetch_and(!(1 << block), Ordering::SeqCst);
    
    // Add two blocks to lower order
    let child1 = block * 2;
    let child2 = block * 2 + 1;
    BUDDY[order - 1].fetch_or((1 << child1) | (1 << child2), Ordering::SeqCst);
    
    let end = timing::read_tsc();
    end - start
}

/// Buddy allocator merge operation
fn bench_buddy_merge() -> u64 {
    let start = timing::read_tsc();
    
    static BUDDY: [AtomicU64; 5] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0x3), // Two adjacent free blocks
        AtomicU64::new(0),
    ];
    
    let order = 3;
    let block = 0;
    let buddy = 1;
    
    // Check if buddy is free
    let bitmap = BUDDY[order].load(Ordering::SeqCst);
    if (bitmap >> buddy) & 1 == 1 {
        // Remove both from current order
        BUDDY[order].fetch_and(!((1 << block) | (1 << buddy)), Ordering::SeqCst);
        
        // Add merged block to higher order
        BUDDY[order + 1].fetch_or(1 << (block / 2), Ordering::SeqCst);
    }
    
    let end = timing::read_tsc();
    end - start
}

/// Slab allocator allocation
fn bench_slab_alloc() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate slab allocation
    static SLAB_BITMAP: AtomicU64 = AtomicU64::new(0xFFFFFFFFFFFFFFFF);
    
    // Find first free slot
    let bitmap = SLAB_BITMAP.load(Ordering::SeqCst);
    let slot = bitmap.trailing_zeros();
    
    if slot < 64 {
        // Allocate
        SLAB_BITMAP.fetch_and(!(1 << slot), Ordering::SeqCst);
        
        // Calculate address
        let base = 0x1000_0000u64;
        let object_size = 256u64;
        let addr = base + (slot as u64) * object_size;
        
        core::hint::black_box(addr);
    }
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Region Benchmarks
// =============================================================================

/// Region creation
fn bench_region_create() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate VMA creation
    let region = VirtualMemoryArea {
        start: 0x1000_0000,
        end: 0x1000_FFFF,
        flags: VMA_READ | VMA_WRITE,
    };
    
    // Simulate insertion into tree/list
    static REGION_COUNT: AtomicU64 = AtomicU64::new(0);
    REGION_COUNT.fetch_add(1, Ordering::SeqCst);
    
    core::hint::black_box(region);
    
    let end = timing::read_tsc();
    end - start
}

/// Region lookup
fn bench_region_lookup() -> u64 {
    let start = timing::read_tsc();
    
    // Simulate VMA lookup
    let addr = 0x1000_5000u64;
    
    // Simulate tree traversal (simplified)
    let regions = [
        VirtualMemoryArea { start: 0x0000_0000, end: 0x0000_FFFF, flags: VMA_READ },
        VirtualMemoryArea { start: 0x1000_0000, end: 0x1000_FFFF, flags: VMA_READ | VMA_WRITE },
        VirtualMemoryArea { start: 0x2000_0000, end: 0x2000_FFFF, flags: VMA_READ | VMA_EXEC },
    ];
    
    let mut found = None;
    for region in &regions {
        if addr >= region.start && addr <= region.end {
            found = Some(region);
            break;
        }
    }
    
    core::hint::black_box(found);
    
    let end = timing::read_tsc();
    end - start
}

/// Protection check
fn bench_protection_check() -> u64 {
    let start = timing::read_tsc();
    
    let region = VirtualMemoryArea {
        start: 0x1000_0000,
        end: 0x1000_FFFF,
        flags: VMA_READ | VMA_WRITE,
    };
    
    // Check various access types
    let can_read = (region.flags & VMA_READ) != 0;
    let can_write = (region.flags & VMA_WRITE) != 0;
    let can_exec = (region.flags & VMA_EXEC) != 0;
    
    core::hint::black_box((can_read, can_write, can_exec));
    
    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Helper Types
// =============================================================================

const VMA_READ: u32 = 1;
const VMA_WRITE: u32 = 2;
const VMA_EXEC: u32 = 4;

#[derive(Clone, Copy)]
struct VirtualMemoryArea {
    start: u64,
    end: u64,
    flags: u32,
}
