//! # Memory Management Subsystems
//!
//! Subsystems for physical memory, virtual memory, and heap management.
//! These run in the Early phase and are essential for kernel operation.

use crate::context::{InitContext, MemoryKind};
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// PHYSICAL MEMORY MANAGER
// =============================================================================

/// Physical Memory Manager subsystem
///
/// Manages physical page allocation using a buddy allocator or similar.
pub struct PmmSubsystem {
    info: SubsystemInfo,
    total_pages: usize,
    free_pages: usize,
    page_size: usize,
    zones: Vec<MemoryZone>,
}

/// Memory zone (DMA, Normal, High)
#[derive(Debug, Clone)]
pub struct MemoryZone {
    pub name: &'static str,
    pub start: u64,
    pub end: u64,
    pub free_pages: usize,
    pub total_pages: usize,
}

static PMM_DEPS: [Dependency; 1] = [Dependency::required("boot_info")];

impl PmmSubsystem {
    /// Create new PMM subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("pmm", InitPhase::Early)
                .with_priority(1000)
                .with_description("Physical memory manager")
                .with_dependencies(&PMM_DEPS)
                .provides(PhaseCapabilities::MEMORY)
                .essential(),
            total_pages: 0,
            free_pages: 0,
            page_size: 4096,
            zones: Vec::new(),
        }
    }

    /// Get total pages
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }

    /// Get free pages
    pub fn free_pages(&self) -> usize {
        self.free_pages
    }

    /// Get page size
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Allocate physical pages
    pub fn alloc_pages(&mut self, count: usize) -> Option<u64> {
        if count > self.free_pages {
            return None;
        }

        // In real code: use buddy allocator
        self.free_pages -= count;

        // Return placeholder address
        Some(0x1000_0000)
    }

    /// Free physical pages
    pub fn free_pages_at(&mut self, _addr: u64, count: usize) {
        self.free_pages += count;
    }

    /// Initialize zones from memory map
    fn init_zones(&mut self, ctx: &InitContext) {
        if let Some(boot_info) = ctx.boot_info() {
            // Create zones based on memory map
            let mut dma_zone = MemoryZone {
                name: "DMA",
                start: 0,
                end: 16 * 1024 * 1024, // 16 MB
                free_pages: 0,
                total_pages: 0,
            };

            let mut normal_zone = MemoryZone {
                name: "Normal",
                start: 16 * 1024 * 1024,
                end: 4 * 1024 * 1024 * 1024, // 4 GB
                free_pages: 0,
                total_pages: 0,
            };

            let mut high_zone = MemoryZone {
                name: "High",
                start: 4 * 1024 * 1024 * 1024,
                end: u64::MAX,
                free_pages: 0,
                total_pages: 0,
            };

            for region in &boot_info.memory_map {
                if region.kind == MemoryKind::Usable {
                    let pages = (region.length as usize) / self.page_size;

                    if region.base < dma_zone.end {
                        dma_zone.total_pages += pages;
                        dma_zone.free_pages += pages;
                    } else if region.base < normal_zone.end {
                        normal_zone.total_pages += pages;
                        normal_zone.free_pages += pages;
                    } else {
                        high_zone.total_pages += pages;
                        high_zone.free_pages += pages;
                    }

                    self.total_pages += pages;
                    self.free_pages += pages;
                }
            }

            if dma_zone.total_pages > 0 {
                self.zones.push(dma_zone);
            }
            if normal_zone.total_pages > 0 {
                self.zones.push(normal_zone);
            }
            if high_zone.total_pages > 0 {
                self.zones.push(high_zone);
            }
        }
    }
}

impl Default for PmmSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for PmmSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing physical memory manager");

        // Initialize zones from memory map
        self.init_zones(ctx);

        ctx.info(alloc::format!(
            "PMM: {} pages ({} MB) total, {} free",
            self.total_pages,
            (self.total_pages * self.page_size) / (1024 * 1024),
            self.free_pages
        ));

        for zone in &self.zones {
            ctx.debug(alloc::format!(
                "Zone {}: {} pages (0x{:x} - 0x{:x})",
                zone.name,
                zone.total_pages,
                zone.start,
                zone.end
            ));
        }

        if self.free_pages == 0 {
            return Err(InitError::new(
                ErrorKind::OutOfMemory,
                "No usable memory found",
            ));
        }

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("PMM shutdown");
        Ok(())
    }
}

// =============================================================================
// VIRTUAL MEMORY MANAGER
// =============================================================================

/// Virtual Memory Manager subsystem
///
/// Manages page tables and virtual address spaces.
pub struct VmmSubsystem {
    info: SubsystemInfo,
    page_table_root: u64,
    kernel_start: u64,
    kernel_end: u64,
    heap_start: u64,
    heap_end: u64,
    page_levels: u8,
}

static VMM_DEPS: [Dependency; 1] = [Dependency::required("pmm")];

impl VmmSubsystem {
    /// Create new VMM subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("vmm", InitPhase::Early)
                .with_priority(900)
                .with_description("Virtual memory manager")
                .with_dependencies(&VMM_DEPS)
                .provides(PhaseCapabilities::MEMORY)
                .essential(),
            page_table_root: 0,
            kernel_start: 0xFFFF_8000_0000_0000, // Higher half
            kernel_end: 0,
            heap_start: 0,
            heap_end: 0,
            page_levels: 4, // 4-level paging (x86_64)
        }
    }

    /// Map a virtual address to physical
    pub fn map(&mut self, virt: u64, phys: u64, flags: PageFlags) -> InitResult<()> {
        // In real code: walk page tables and map
        ctx_trace!("VMM: map 0x{:x} -> 0x{:x}", virt, phys);
        Ok(())
    }

    /// Unmap a virtual address
    pub fn unmap(&mut self, virt: u64) -> InitResult<()> {
        // In real code: walk page tables and unmap
        Ok(())
    }

    /// Translate virtual to physical
    pub fn translate(&self, virt: u64) -> Option<u64> {
        // In real code: walk page tables
        Some(virt & 0x0000_FFFF_FFFF_F000)
    }
}

/// Page mapping flags
#[derive(Debug, Clone, Copy)]
pub struct PageFlags {
    pub present: bool,
    pub writable: bool,
    pub user: bool,
    pub no_execute: bool,
    pub global: bool,
    pub write_through: bool,
    pub cache_disable: bool,
}

impl Default for PageFlags {
    fn default() -> Self {
        Self {
            present: true,
            writable: true,
            user: false,
            no_execute: false,
            global: false,
            write_through: false,
            cache_disable: false,
        }
    }
}

impl Default for VmmSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for VmmSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing virtual memory manager");

        // Read current page table root
        #[cfg(target_arch = "x86_64")]
        {
            let cr3: u64;
            unsafe {
                core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack));
            }
            self.page_table_root = cr3 & !0xFFF;
            ctx.debug(alloc::format!("CR3: 0x{:x}", self.page_table_root));
        }

        #[cfg(target_arch = "aarch64")]
        {
            let ttbr1: u64;
            unsafe {
                core::arch::asm!("mrs {}, ttbr1_el1", out(reg) ttbr1, options(nostack));
            }
            self.page_table_root = ttbr1 & !0xFFFF;
            self.page_levels = 4; // Or 3 for 39-bit VA
        }

        #[cfg(target_arch = "riscv64")]
        {
            let satp: u64;
            unsafe {
                core::arch::asm!("csrr {}, satp", out(reg) satp, options(nostack));
            }
            self.page_table_root = (satp & 0x0FFF_FFFF_FFFF) << 12;
            self.page_levels = ((satp >> 60) & 0xF) as u8; // Sv39=8, Sv48=9, Sv57=10
        }

        // Calculate kernel address space
        self.kernel_end = self.kernel_start + 512 * 1024 * 1024 * 1024; // 512 GB
        self.heap_start = self.kernel_start + 256 * 1024 * 1024 * 1024; // Start at +256 GB
        self.heap_end = self.kernel_end;

        ctx.info(alloc::format!(
            "VMM: {} level paging, kernel at 0x{:x}",
            self.page_levels,
            self.kernel_start
        ));

        Ok(())
    }
}

// =============================================================================
// HEAP SUBSYSTEM
// =============================================================================

/// Kernel Heap subsystem
///
/// Provides dynamic memory allocation for the kernel.
pub struct HeapSubsystem {
    info: SubsystemInfo,
    heap_start: u64,
    heap_size: usize,
    heap_used: usize,
    allocator_type: AllocatorType,
}

/// Type of heap allocator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatorType {
    Bump,
    LinkedList,
    Buddy,
    Slab,
}

static HEAP_DEPS: [Dependency; 1] = [Dependency::required("vmm")];

impl HeapSubsystem {
    /// Create new heap subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("heap", InitPhase::Early)
                .with_priority(800)
                .with_description("Kernel heap allocator")
                .with_dependencies(&HEAP_DEPS)
                .provides(PhaseCapabilities::HEAP)
                .essential(),
            heap_start: 0,
            heap_size: 0,
            heap_used: 0,
            allocator_type: AllocatorType::LinkedList,
        }
    }

    /// Get heap usage
    pub fn used(&self) -> usize {
        self.heap_used
    }

    /// Get heap size
    pub fn size(&self) -> usize {
        self.heap_size
    }

    /// Get free space
    pub fn free(&self) -> usize {
        self.heap_size - self.heap_used
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.heap_size == 0 {
            0.0
        } else {
            (self.heap_used as f64 / self.heap_size as f64) * 100.0
        }
    }
}

impl Default for HeapSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for HeapSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing kernel heap");

        // Get heap configuration from context
        let heap_size_mb = ctx.config().get_uint("heap_size_mb", 64) as usize;
        self.heap_size = heap_size_mb * 1024 * 1024;

        // Get heap start from config or use default
        self.heap_start = ctx.config().get_uint("heap_start", 0xFFFF_A000_0000_0000);

        // Choose allocator based on config
        let alloc_type = ctx.config().get_str("heap_allocator", "linked_list");
        self.allocator_type = match alloc_type.as_str() {
            "bump" => AllocatorType::Bump,
            "buddy" => AllocatorType::Buddy,
            "slab" => AllocatorType::Slab,
            _ => AllocatorType::LinkedList,
        };

        ctx.info(alloc::format!(
            "Heap: {} MB at 0x{:x}, allocator: {:?}",
            self.heap_size / (1024 * 1024),
            self.heap_start,
            self.allocator_type
        ));

        // In real code: initialize the actual allocator

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info(alloc::format!(
            "Heap shutdown: {} bytes used of {} ({:.1}%)",
            self.heap_used,
            self.heap_size,
            self.usage_percent()
        ));
        Ok(())
    }
}

// Helper macro for tracing
macro_rules! ctx_trace {
    ($($arg:tt)*) => {
        // In real code: use ctx.trace()
    };
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmm_subsystem() {
        let sub = PmmSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Early);
        assert!(sub.info().essential);
        assert_eq!(sub.page_size(), 4096);
    }

    #[test]
    fn test_vmm_subsystem() {
        let sub = VmmSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Early);
        assert!(sub.info().provides.contains(PhaseCapabilities::MEMORY));
    }

    #[test]
    fn test_heap_subsystem() {
        let sub = HeapSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Early);
        assert!(sub.info().provides.contains(PhaseCapabilities::HEAP));
        assert_eq!(sub.usage_percent(), 0.0);
    }

    #[test]
    fn test_page_flags() {
        let flags = PageFlags::default();
        assert!(flags.present);
        assert!(flags.writable);
        assert!(!flags.user);
        assert!(!flags.no_execute);
    }
}
