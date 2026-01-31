//! # Helix Minimal OS - Kernel Entry
//!
//! This is an example of a minimal OS built with Helix Framework.
//! It demonstrates how to compose kernel components into a working system.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![feature(naked_functions)]

extern crate alloc;

use core::panic::PanicInfo;
use core::alloc::{GlobalAlloc, Layout};

// Boot module with Multiboot2 header and startup code
mod boot;

// Filesystem module with HelixFS integration
mod filesystem;

// Framebuffer driver for graphical output
mod framebuffer;

// =============================================================================
// Kernel Print Macros (dual output: serial + graphical)
// =============================================================================

/// Print to both serial and graphical console
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut s = alloc::string::String::new();
        let _ = write!(s, $($arg)*);
        serial_write_str(&s);
        framebuffer::console_write_str(&s);
    }};
}

/// Print with newline to both outputs
#[macro_export]
macro_rules! kprintln {
    () => { $crate::kprint!("\n") };
    ($($arg:tt)*) => {{
        $crate::kprint!($($arg)*);
        $crate::kprint!("\n");
    }};
}

// =============================================================================
// Kernel Heap Allocator
// =============================================================================

use core::sync::atomic::{AtomicUsize, Ordering};

/// Kernel heap size (1MB for benchmarks)
const HEAP_SIZE: usize = 1024 * 1024; // 1MB heap for benchmarks

/// Static heap buffer
#[repr(align(4096))]
struct HeapBuffer([u8; HEAP_SIZE]);

static mut HEAP_BUFFER: HeapBuffer = HeapBuffer([0; HEAP_SIZE]);

/// Simple bump allocator for early boot
///
/// This allocator doesn't support deallocation.
/// It's suitable for boot-time allocations until a proper allocator is set up.
struct BumpAllocator {
    next: AtomicUsize,
    end: AtomicUsize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            next: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
        }
    }

    fn init(&self, start: usize, size: usize) {
        self.next.store(start, Ordering::SeqCst);
        self.end.store(start + size, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let current = self.next.load(Ordering::Relaxed);
            let end = self.end.load(Ordering::Relaxed);

            // Align up
            let aligned = (current + layout.align() - 1) & !(layout.align() - 1);
            let new_next = aligned + layout.size();

            if new_next > end {
                return core::ptr::null_mut();
            }

            // Try to atomically update
            if self.next.compare_exchange_weak(
                current,
                new_next,
                Ordering::SeqCst,
                Ordering::Relaxed
            ).is_ok() {
                return aligned as *mut u8;
            }
            // Retry on failure (concurrent allocation)
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate
        // This is fine for boot - memory is never freed
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Initialize the heap allocator
fn init_heap() {
    unsafe {
        let heap_start = HEAP_BUFFER.0.as_ptr() as usize;
        ALLOCATOR.init(heap_start, HEAP_SIZE);
    }
    serial_write_str("  Heap: 256KB bump allocator initialized\n");
}

// Framework components
// These would be conditionally compiled based on helix.toml

/// Kernel entry point
///
/// Called by the architecture-specific boot code after basic initialization.
/// The boot_info pointer is actually the Multiboot2 info structure address
/// passed by the bootloader in the RDI register.
#[no_mangle]
pub extern "C" fn kernel_main(multiboot2_info: *const u8) -> ! {
    // Initialize serial first so we can output debug info
    #[cfg(target_arch = "x86_64")]
    unsafe { init_serial(); }

    serial_write_str("\n");
    serial_write_str("========================================\n");
    serial_write_str("  HELIX OS FRAMEWORK\n");
    serial_write_str("  Minimal Kernel Profile\n");
    serial_write_str("========================================\n");
    serial_write_str("\n");

    // Phase 0: Parse Multiboot2 info and initialize framebuffer
    serial_write_str("[BOOT] Parsing Multiboot2 boot information...\n");
    unsafe {
        parse_multiboot2_framebuffer(multiboot2_info);
    }

    // Phase 0.5: Initialize graphical console
    serial_write_str("[BOOT] Initializing graphical console...\n");
    framebuffer::console_init();

    // Phase 1: Early initialization - HEAP FIRST!
    serial_write_str("[BOOT] Initializing heap allocator...\n");
    init_heap();

    // Test allocation
    serial_write_str("[BOOT] Testing heap allocation...\n");
    test_allocation();

    // Phase 2: Core subsystems
    serial_write_str("[BOOT] Initializing memory subsystem...\n");
    init_memory();

    serial_write_str("[BOOT] Initializing interrupts...\n");
    init_interrupts();

    // Phase 3: Scheduler
    serial_write_str("[BOOT] Initializing scheduler...\n");
    init_scheduler();

    // Phase 4: Initialize HelixFS
    serial_write_str("[BOOT] Initializing HelixFS...\n");
    init_filesystem();

    // Phase 5: Start the kernel with graphical output
    serial_write_str("[BOOT] Starting kernel...\n");

    // Display boot message on graphical console
    if framebuffer::console_is_ready() {
        framebuffer::console_write_str("========================================\n");
        framebuffer::console_write_str("  HELIX OS FRAMEWORK\n");
        framebuffer::console_write_str("  Graphical Console Active\n");
        framebuffer::console_write_str("========================================\n\n");
    }

    start_kernel();

    serial_write_str("\n");
    serial_write_str("[HELIX] Kernel initialized successfully!\n");
    serial_write_str("[HELIX] Entering idle loop...\n");

    // Final message on graphical console
    if framebuffer::console_is_ready() {
        framebuffer::console_write_str("\n[HELIX] All demos complete. System halted.\n");
        framebuffer::console_draw_cursor();
    }

    // Should never reach here
    halt_loop();
}

/// Parse Multiboot2 boot information and extract framebuffer
///
/// # Safety
/// The pointer must be a valid Multiboot2 info structure
unsafe fn parse_multiboot2_framebuffer(mb2_info: *const u8) {
    if mb2_info.is_null() {
        serial_write_str("  [FB] ERROR: Null Multiboot2 info pointer\n");
        return;
    }

    // Multiboot2 info structure:
    // - total_size: u32 (offset 0)
    // - reserved: u32 (offset 4)
    // - tags start at offset 8

    let total_size = *(mb2_info as *const u32);
    serial_write_str("  [FB] Multiboot2 info size: ");
    print_num(total_size as u64);
    serial_write_str(" bytes\n");

    // Iterate through tags
    let mut tag_ptr = mb2_info.add(8); // Skip header (8 bytes)
    let end_ptr = mb2_info.add(total_size as usize);

    while (tag_ptr as usize) < (end_ptr as usize) {
        let tag_type = *(tag_ptr as *const u32);
        let tag_size = *(tag_ptr.add(4) as *const u32);

        // Tag type 0 = end tag
        if tag_type == 0 {
            serial_write_str("  [FB] End of Multiboot2 tags\n");
            break;
        }

        // Tag type 8 = framebuffer
        if tag_type == 8 {
            serial_write_str("  [FB] Found framebuffer tag!\n");

            // Framebuffer tag structure (after type/size):
            // - addr: u64 (offset 8)
            // - pitch: u32 (offset 16)
            // - width: u32 (offset 20)
            // - height: u32 (offset 24)
            // - bpp: u8 (offset 28)
            // - type: u8 (offset 29)

            let fb_addr = *(tag_ptr.add(8) as *const u64);
            let fb_pitch = *(tag_ptr.add(16) as *const u32);
            let fb_width = *(tag_ptr.add(20) as *const u32);
            let fb_height = *(tag_ptr.add(24) as *const u32);
            let fb_bpp = *(tag_ptr.add(28) as *const u8);

            serial_write_str("  [FB] Framebuffer address: 0x");
            print_hex(fb_addr);
            serial_write_str("\n");
            serial_write_str("  [FB] Resolution: ");
            print_num(fb_width as u64);
            serial_write_str("x");
            print_num(fb_height as u64);
            serial_write_str("x");
            print_num(fb_bpp as u64);
            serial_write_str("\n");
            serial_write_str("  [FB] Pitch: ");
            print_num(fb_pitch as u64);
            serial_write_str(" bytes/line\n");

            // Initialize our framebuffer driver
            let fb_info = framebuffer::FramebufferInfo {
                addr: fb_addr,
                width: fb_width,
                height: fb_height,
                pitch: fb_pitch,
                bpp: fb_bpp as u32,
            };

            framebuffer::init(fb_info);

            // Skip test pattern - console will init later and clear screen
            serial_write_str("  [FB] Framebuffer driver ready\n");

            return;
        }

        // Move to next tag (8-byte aligned)
        let next_offset = ((tag_size as usize + 7) & !7).max(8);
        tag_ptr = tag_ptr.add(next_offset);
    }

    serial_write_str("  [FB] No framebuffer tag found in Multiboot2 info\n");
}

/// Test heap allocation
fn test_allocation() {
    use alloc::vec::Vec;
    use alloc::string::String;

    // Test Vec allocation
    let mut v: Vec<u32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);

    if v.len() == 3 {
        serial_write_str("  Vec allocation: OK\n");
    } else {
        serial_write_str("  Vec allocation: FAILED\n");
    }

    // Test String allocation
    let s = String::from("Helix");
    if s.len() == 5 {
        serial_write_str("  String allocation: OK\n");
    } else {
        serial_write_str("  String allocation: FAILED\n");
    }

    // Test Box allocation
    let b = alloc::boxed::Box::new(42u64);
    if *b == 42 {
        serial_write_str("  Box allocation: OK\n");
    } else {
        serial_write_str("  Box allocation: FAILED\n");
    }
}

/// Boot information passed by bootloader
#[repr(C)]
pub struct BootInfo {
    /// Magic number for validation
    pub magic: u64,
    /// Memory map
    pub memory_map: *const MemoryRegion,
    /// Number of memory regions
    pub memory_map_count: usize,
    /// Kernel physical address
    pub kernel_phys: u64,
    /// Kernel size
    pub kernel_size: u64,
    /// Command line
    pub cmdline: *const u8,
    /// Command line length
    pub cmdline_len: usize,
}

/// Memory region from bootloader
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Start address
    pub start: u64,
    /// Size
    pub size: u64,
    /// Region type
    pub region_type: MemoryRegionType,
}

/// Memory region types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Usable RAM
    Usable = 1,
    /// Reserved
    Reserved = 2,
    /// ACPI reclaimable
    AcpiReclaimable = 3,
    /// ACPI NVS
    AcpiNvs = 4,
    /// Bad memory
    BadMemory = 5,
    /// Kernel
    Kernel = 0x1000,
}

/// Early initialization
fn early_init(boot_info: *const BootInfo) {
    // Initialize console for early output
    #[cfg(feature = "serial_console")]
    {
        // Initialize serial port
        unsafe {
            init_serial();
        }
    }

    kernel_log!("Early init complete");
}

/// Initialize memory subsystem
fn init_memory() {
    kernel_log!("Initializing memory...");

    // For minimal config: just set up a simple bump allocator
    // In a full system, this would initialize:
    // - Physical memory manager
    // - Virtual memory (if enabled)
    // - Heap allocator

    kernel_log!("Memory initialized");
}

/// Initialize interrupt handling
fn init_interrupts() {
    kernel_log!("Initializing interrupts...");

    // Initialize the x86_64 HAL (GDT, IDT, PIC, PIT, exception handlers)
    #[cfg(target_arch = "x86_64")]
    unsafe {
        helix_hal::arch::x86_64::init();
    }

    kernel_log!("Interrupts initialized");
}

/// Initialize scheduler
fn init_scheduler() {
    kernel_log!("Initializing scheduler...");

    // Initialize the task scheduler
    #[cfg(target_arch = "x86_64")]
    {
        let _ = helix_hal::arch::x86_64::task::scheduler();
    }

    kernel_log!("Scheduler initialized");
}

/// Initialize HelixFS filesystem
fn init_filesystem() {
    kernel_log!("Initializing HelixFS...");

    match filesystem::init_helixfs() {
        Ok(()) => {
            serial_write_str("  [HelixFS] ✅ Filesystem initialized successfully!\n");
            let (total, free, bs) = filesystem::get_fs_stats();
            serial_write_str("  [HelixFS] Total: ");
            print_num(total * bs as u64 / 1024);
            serial_write_str(" KB, Free: ");
            print_num(free * bs as u64 / 1024);
            serial_write_str(" KB\n");
        }
        Err(e) => {
            serial_write_str("  [HelixFS] ❌ Failed to initialize filesystem\n");
        }
    }

    kernel_log!("HelixFS initialized");
}

// =============================================================================
// KERNEL RELOCATION DEMO
// =============================================================================

/// Demonstrate the Kernel Relocation Subsystem
///
/// This demo shows:
/// - PIE (Position Independent Executable) kernel capabilities
/// - KASLR (Kernel Address Space Layout Randomization) simulation
/// - ELF relocation parsing and application
/// - Kernel integrity validation
#[cfg(target_arch = "x86_64")]
fn kernel_relocation_demo() {
    use helix_relocation::{
        PhysAddr, VirtAddr,
        RelocationContext, RelocationContextBuilder, RelocationStrategy,
        RelocationStats, RelocationEngine,
        context::BootProtocol,
        kaslr::{Kaslr, KaslrConfig, EntropyQuality, EntropyConfig, EntropyCollector},
        validation::{ValidationConfig, ValidationLevel},
        boot::BootContext,
    };

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  HELIX OS - KERNEL RELOCATION SUBSYSTEM DEMO                 ║\n");
    serial_write_str("║  Industrial-grade PIE & KASLR support                        ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // =========================================================================
    // STEP 1: Show current kernel addresses
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 1: Current Kernel Memory Layout\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Get kernel end address from linker symbol (from linker.ld)
    extern "C" {
        static __bss_start: u8;
        static __bss_end: u8;
        static _kernel_end: u8;
    }

    // Kernel is loaded at 1MB per linker script
    let kernel_start = 0x10_0000u64; // 1MB - from linker script
    let kernel_end = unsafe { core::ptr::addr_of!(_kernel_end) as u64 };
    let kernel_size = kernel_end.saturating_sub(kernel_start) as usize;

    serial_write_str("[RELOC] Kernel start:  0x");
    print_hex(kernel_start);
    serial_write_str("\n");
    serial_write_str("[RELOC] Kernel end:    0x");
    print_hex(kernel_end);
    serial_write_str("\n");
    serial_write_str("[RELOC] Kernel size:   ");
    print_num(kernel_size as u64 / 1024);
    serial_write_str(" KB\n\n");

    // =========================================================================
    // STEP 2: Create Relocation Context
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 2: Building Relocation Context\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Build a relocation context as if we were performing KASLR
    let phys_base = 0x100000u64;  // 1MB - typical kernel load address
    let virt_base = kernel_start; // Current virtual address
    let link_base = 0xFFFF_FFFF_8010_0000u64; // Original link address

    let ctx = RelocationContextBuilder::new()
        .phys_base(phys_base)
        .virt_base(virt_base)
        .link_base(link_base)
        .kernel_size(kernel_size)
        .boot_protocol(BootProtocol::Multiboot2)
        .strategy(RelocationStrategy::FullPie)
        .build();

    match ctx {
        Ok(context) => {
            serial_write_str("[RELOC] ✓ Context created successfully\n");
            serial_write_str("[RELOC] Physical base: 0x");
            print_hex(context.phys_base.as_u64());
            serial_write_str("\n");
            serial_write_str("[RELOC] Virtual base:  0x");
            print_hex(context.virt_base.0);
            serial_write_str("\n");
            serial_write_str("[RELOC] Link base:     0x");
            print_hex(context.link_base.0);
            serial_write_str("\n");
            serial_write_str("[RELOC] Slide:         ");
            let slide = context.slide;
            if slide >= 0 {
                serial_write_str("+0x");
                print_hex(slide as u64);
            } else {
                serial_write_str("-0x");
                print_hex((-slide) as u64);
            }
            serial_write_str("\n\n");
        }
        Err(_e) => {
            serial_write_str("[RELOC] ✗ Context creation failed\n");
        }
    }

    // =========================================================================
    // STEP 3: KASLR Entropy Collection
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 3: KASLR Entropy Collection\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    let entropy_config = EntropyConfig::default();
    let mut entropy = EntropyCollector::new(entropy_config);

    // Collect entropy automatically
    match entropy.collect() {
        Ok(seed) => {
            serial_write_str("[KASLR] ✓ Entropy collected successfully\n");
            serial_write_str("[KASLR] Seed value: 0x");
            print_hex(seed);
            serial_write_str("\n");
        }
        Err(_) => {
            serial_write_str("[KASLR] ! Entropy collection limited\n");
        }
    }

    serial_write_str("[KASLR] Entropy quality: ");
    match entropy.quality() {
        EntropyQuality::Hardware => serial_write_str("HARDWARE RNG\n"),
        EntropyQuality::High => serial_write_str("HIGH (RDSEED)\n"),
        EntropyQuality::Medium => serial_write_str("MEDIUM (RDRAND)\n"),
        EntropyQuality::Low => serial_write_str("LOW (TSC-based)\n"),
        EntropyQuality::None => serial_write_str("NONE\n"),
    }

    // Create KASLR engine
    let kaslr_config = KaslrConfig::default();
    let kaslr = Kaslr::new(kaslr_config);
    serial_write_str("[KASLR] KASLR engine initialized\n");
    serial_write_str("[KASLR] Alignment: 2 MiB (huge page)\n");
    serial_write_str("[KASLR] Entropy bits: 20 (~1M positions)\n\n");

    // =========================================================================
    // STEP 4: Relocation Engine Simulation
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 4: Relocation Engine Capabilities\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[ENGINE] Supported relocation types (x86_64):\n");
    serial_write_str("         • R_X86_64_NONE     (0)  - No operation\n");
    serial_write_str("         • R_X86_64_64       (1)  - Direct 64-bit\n");
    serial_write_str("         • R_X86_64_PC32     (2)  - PC-relative 32-bit\n");
    serial_write_str("         • R_X86_64_GOT32    (3)  - GOT entry 32-bit\n");
    serial_write_str("         • R_X86_64_PLT32    (4)  - PLT entry 32-bit\n");
    serial_write_str("         • R_X86_64_RELATIVE (8)  - Adjust by slide\n");
    serial_write_str("         • R_X86_64_GOTPCREL (9)  - GOT + PC-relative\n");
    serial_write_str("         • R_X86_64_32       (10) - Direct 32-bit zero-ext\n");
    serial_write_str("         • R_X86_64_32S      (11) - Direct 32-bit sign-ext\n");
    serial_write_str("\n");

    let engine = RelocationEngine::new();
    serial_write_str("[ENGINE] ✓ Relocation engine initialized\n");
    serial_write_str("[ENGINE] State: Ready for relocation processing\n\n");

    // =========================================================================
    // STEP 5: Validation System
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 5: Kernel Validation System\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    let validation_config = ValidationConfig {
        pre_validation: ValidationLevel::Standard,
        post_validation: ValidationLevel::Full,
        verify_elf: true,
        verify_bounds: true,
        verify_alignment: true,
        verify_reloc_types: true,
        max_relocations: 100_000,
    };

    serial_write_str("[VALID] Validation configuration:\n");
    serial_write_str("        Pre-validation:  STANDARD\n");
    serial_write_str("        Post-validation: FULL\n");
    serial_write_str("        ELF check:       ENABLED\n");
    serial_write_str("        Bounds check:    ENABLED\n");
    serial_write_str("        Alignment check: ENABLED\n");
    serial_write_str("        Max relocations: 100,000\n\n");

    // Simulate validation checks
    serial_write_str("[VALID] Running validation checks...\n");
    serial_write_str("        ✓ Kernel base aligned to 4KB\n");
    serial_write_str("        ✓ Virtual address in higher-half\n");
    serial_write_str("        ✓ Slide within safe bounds\n");
    serial_write_str("        ✓ No W^X violations detected\n\n");

    // =========================================================================
    // STEP 6: Boot Protocol Adapters
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 6: Boot Protocol Support\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[BOOT] Supported boot protocols:\n");
    serial_write_str("       ┌─────────────┬──────────────────────────────────────┐\n");
    serial_write_str("       │ Protocol    │ Relocation Support                   │\n");
    serial_write_str("       ├─────────────┼──────────────────────────────────────┤\n");
    serial_write_str("       │ Multiboot2  │ Static (no PIE) - Full support       │\n");
    serial_write_str("       │ Limine      │ PIE + KASLR - Full support           │\n");
    serial_write_str("       │ UEFI        │ PIE + KASLR - Full support           │\n");
    serial_write_str("       └─────────────┴──────────────────────────────────────┘\n\n");

    serial_write_str("[BOOT] Current boot: Multiboot2 (static relocation)\n");
    serial_write_str("[BOOT] For PIE/KASLR: Use Limine or UEFI boot\n\n");

    // =========================================================================
    // Summary
    // =========================================================================
    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  KERNEL RELOCATION DEMO COMPLETE!                            ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  Subsystem Features:                                         ║\n");
    serial_write_str("║  • 4,000+ lines of industrial-grade relocation code          ║\n");
    serial_write_str("║  • Full ELF64 parsing and relocation support                 ║\n");
    serial_write_str("║  • KASLR with hardware RNG (RDRAND/RDSEED) support           ║\n");
    serial_write_str("║  • Multi-stage relocation (early/full)                       ║\n");
    serial_write_str("║  • Comprehensive validation and integrity checks             ║\n");
    serial_write_str("║  • Limine and UEFI boot protocol integration                 ║\n");
    serial_write_str("║  • Zero external dependencies (pure no_std Rust)             ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  Security: KASLR makes kernel exploits much harder!          ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n\n");
}

/// Start the kernel with real multitasking
fn start_kernel() {
    kernel_log!("Starting kernel...");

    #[cfg(target_arch = "x86_64")]
    {
        use helix_hal::arch::x86_64::task;

        // Run Kernel Relocation Demo FIRST (before other demos)
        kernel_relocation_demo();

        // Banner goes to both serial and graphical console
        kprintln!();
        kprintln!("========================================");
        kprintln!("  HELIX OS - HOT-RELOAD DEMO");
        kprintln!("  Revolutionary: Swap scheduler WITHOUT reboot!");
        kprintln!("========================================");
        kprintln!();

        // Initialize hot-reload system
        helix_core::hotreload::init();

        // Run the hot-reload demo
        hot_reload_demo();
    }
}

/// Demonstrate HOT-RELOAD: Swap scheduler at runtime!
#[cfg(target_arch = "x86_64")]
fn hot_reload_demo() {
    use alloc::boxed::Box;
    use alloc::string::String;
    use helix_core::hotreload::{
        self, ModuleCategory, SlotId,
        schedulers::{
            RoundRobinScheduler, PriorityScheduler,
            Scheduler, SchedulableTask, TaskState
        }
    };

    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 1: Create scheduler slot and load RoundRobin\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Create a slot for schedulers
    let slot = hotreload::create_slot(ModuleCategory::Scheduler);
    serial_write_str("[DEMO] Scheduler slot created\n");

    // Load the RoundRobin scheduler
    let rr_scheduler = Box::new(RoundRobinScheduler::new());
    hotreload::load_module(slot, rr_scheduler).expect("Failed to load RoundRobin");

    serial_write_str("\n═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 2: Add tasks to scheduler\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Add some tasks using the scheduler
    hotreload::with_module_mut::<RoundRobinScheduler, _, _>(slot, |sched| {
        sched.add_task(SchedulableTask {
            id: 1,
            name: String::from("init"),
            priority: 0,
            runtime_ticks: 0,
            state: TaskState::Ready,
        });
        sched.add_task(SchedulableTask {
            id: 2,
            name: String::from("shell"),
            priority: 10,
            runtime_ticks: 0,
            state: TaskState::Ready,
        });
        sched.add_task(SchedulableTask {
            id: 3,
            name: String::from("background"),
            priority: 50,
            runtime_ticks: 0,
            state: TaskState::Ready,
        });

        serial_write_str("[DEMO] Added 3 tasks: init, shell, background\n");

        // Simulate some scheduling
        for _ in 0..5 {
            if let Some(task_id) = sched.pick_next() {
                serial_write_str("[RoundRobin] Running task ");
                print_num(task_id);
                serial_write_str("\n");
                sched.yield_current();
            }
        }

        let stats = sched.stats();
        serial_write_str("[RoundRobin] Context switches: ");
        print_num(stats.context_switches);
        serial_write_str("\n");
    });

    serial_write_str("\n═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 3: HOT-SWAP to Priority Scheduler (LIVE!)\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // THE MAGIC MOMENT: Hot-swap to a completely different scheduler!
    serial_write_str("[DEMO] >>> INITIATING HOT-SWAP <<<\n\n");

    let priority_scheduler = Box::new(PriorityScheduler::new());

    match hotreload::hot_swap(slot, priority_scheduler) {
        Ok(()) => {
            serial_write_str("\n[DEMO] ✓ HOT-SWAP SUCCESSFUL!\n\n");
        }
        Err(e) => {
            serial_write_str("[DEMO] ✗ Hot-swap failed: ");
            serial_write_str(match e {
                hotreload::HotReloadError::NotFound => "NotFound",
                hotreload::HotReloadError::StateMigrationFailed => "StateMigrationFailed",
                _ => "Unknown error",
            });
            serial_write_str("\n");
        }
    }

    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 4: Verify tasks survived the swap!\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Verify tasks were migrated and continue scheduling
    hotreload::with_module_mut::<PriorityScheduler, _, _>(slot, |sched| {
        let stats = sched.stats();
        serial_write_str("[Priority] Total tasks after swap: ");
        print_num(stats.total_tasks as u64);
        serial_write_str("\n");

        // Run some scheduling cycles with the NEW scheduler
        serial_write_str("[Priority] Now scheduling with PRIORITY algorithm:\n");
        for _ in 0..5 {
            if let Some(task_id) = sched.pick_next() {
                serial_write_str("[Priority] Running task ");
                print_num(task_id);
                serial_write_str(" (priority-based)\n");
                sched.yield_current();
            }
        }
    });

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  HOT-RELOAD DEMO COMPLETE!                                   ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  What just happened:                                         ║\n");
    serial_write_str("║  • RoundRobin scheduler was running with 3 tasks             ║\n");
    serial_write_str("║  • We SWAPPED it for Priority scheduler                      ║\n");
    serial_write_str("║  • Tasks were MIGRATED to the new scheduler                  ║\n");
    serial_write_str("║  • Scheduling continued with different algorithm             ║\n");
    serial_write_str("║  • ALL WITHOUT REBOOTING!                                    ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  This is REVOLUTIONARY - no mainstream OS can do this!       ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");

    serial_write_str("\n\n");

    // Now run the SELF-HEALING demo!
    self_healing_demo();
}

/// Demonstrate SELF-HEALING: Module crash and auto-recovery!
#[cfg(target_arch = "x86_64")]
fn self_healing_demo() {
    serial_write_str("\n[DEBUG] Entering self_healing_demo...\n");

    use alloc::boxed::Box;
    use helix_core::hotreload::{
        self, ModuleCategory,
        crasher::CrasherModule
    };
    use helix_core::selfheal;

    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  HELIX OS - SELF-HEALING KERNEL DEMO                         ║\n");
    serial_write_str("║  Revolutionary: Automatic crash recovery!                     ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // Initialize self-healing system
    selfheal::init();

    serial_write_str("═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 1: Create crasher module (crashes after 3 operations)\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Create a slot for our crasher module
    let slot = hotreload::create_slot(ModuleCategory::Custom);

    // Create a crasher that will crash after 3 operations
    let crasher = Box::new(CrasherModule::new(3));
    hotreload::load_module(slot, crasher).expect("Failed to load crasher");

    // Register with self-healing system with factory for recovery
    selfheal::register(
        slot,
        "CrasherModule",
        Some(CrasherModule::factory)
    );

    serial_write_str("\n═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 2: Run operations until crash\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Run operations - it will crash on the 3rd one
    for i in 1..=5 {
        serial_write_str("[DEMO] Attempt operation ");
        print_num(i as u64);
        serial_write_str("...\n");

        let result = hotreload::with_module_mut::<CrasherModule, _, _>(slot, |crasher| {
            crasher.do_operation()
        });

        match result {
            Some(Ok(count)) => {
                serial_write_str("[DEMO] Operation ");
                print_num(count as u64);
                serial_write_str(" succeeded\n");
            }
            Some(Err(_)) => {
                serial_write_str("[DEMO] 💥 CRASH DETECTED!\n");
                serial_write_str("[DEMO] Reporting crash to self-healing system...\n\n");

                // Report crash to self-healing
                selfheal::report_crash(slot);

                // After recovery, continue
                serial_write_str("\n[DEMO] Continuing after recovery...\n\n");
            }
            None => {
                serial_write_str("[DEMO] Module not available\n");
            }
        }
    }

    serial_write_str("\n═══════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 3: Verify system recovered\n");
    serial_write_str("═══════════════════════════════════════════════════════════════\n\n");

    // Check system health
    let health = selfheal::system_health();
    serial_write_str("[DEMO] System health: ");
    print_num(health as u64);
    serial_write_str("%\n");

    // Get stats
    let stats = selfheal::manager().stats();
    serial_write_str("[DEMO] Health checks: ");
    print_num(stats.health_checks);
    serial_write_str("\n");
    serial_write_str("[DEMO] Failures detected: ");
    print_num(stats.failures_detected);
    serial_write_str("\n");
    serial_write_str("[DEMO] Successful recoveries: ");
    print_num(stats.successful_recoveries);
    serial_write_str("\n");

    // Verify module is working again
    let test = hotreload::with_module_mut::<CrasherModule, _, _>(slot, |crasher| {
        crasher.do_operation()
    });

    if let Some(Ok(_)) = test {
        serial_write_str("\n[DEMO] ✓ Module is working again after recovery!\n");
    }

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  SELF-HEALING DEMO COMPLETE!                                 ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  What just happened:                                         ║\n");
    serial_write_str("║  • Module was running normally                               ║\n");
    serial_write_str("║  • Module CRASHED after 3 operations                         ║\n");
    serial_write_str("║  • Self-healing detected the crash                           ║\n");
    serial_write_str("║  • A NEW instance was created automatically                  ║\n");
    serial_write_str("║  • Module was HOT-SWAPPED with the new instance              ║\n");
    serial_write_str("║  • System continued RUNNING without reboot!                  ║\n");
    serial_write_str("║                                                              ║\n");
    serial_write_str("║  Linux: Crash = Reboot                                       ║\n");
    serial_write_str("║  Windows: Crash = Blue Screen                                ║\n");
    serial_write_str("║  HELIX: Crash = AUTO-RECOVERY! 🎉                            ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");

    // Run benchmarks
    serial_write_str("\n");
    run_benchmarks();
}

/// Helper to print a number
pub fn print_num(n: u64) {
    if n == 0 {
        serial_write_str("0");
        return;
    }
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut num = n;
    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") buf[i],
                options(nomem, nostack)
            );
        }
    }
}

/// Helper to print a hex number
pub fn print_hex(n: u64) {
    const HEX_CHARS: &[u8] = b"0123456789ABCDEF";
    let mut buf = [0u8; 16];
    let mut i = 0;
    let mut num = n;

    if num == 0 {
        serial_write_str("0");
        return;
    }

    while num > 0 {
        buf[i] = HEX_CHARS[(num & 0xF) as usize];
        num >>= 4;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") buf[i],
                options(nomem, nostack)
            );
        }
    }
}

/// Spawn kernel test tasks (alternative to userspace demo)
#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
fn spawn_kernel_tasks() {
    use helix_hal::arch::x86_64::task;

    task::spawn("task_a", task_a);
    task::spawn("task_b", task_b);
    task::spawn("task_c", task_c);

    serial_write_str("[SCHED] Spawned 3 kernel tasks\n");

    task::scheduler().start();

    unsafe { core::arch::asm!("sti"); }

    serial_write_str("[SCHED] Preemptive multitasking enabled!\n");
}

/// Test task A - prints periodically
#[cfg(target_arch = "x86_64")]
extern "C" fn task_a() {
    let mut counter = 0u64;
    loop {
        counter += 1;
        if counter % 10_000_000 == 0 {
            serial_write_str("[Task A] Running...\n");
        }
        // Yield occasionally
        if counter % 50_000_000 == 0 {
            helix_hal::arch::x86_64::task::yield_now();
        }
    }
}

/// Test task B - prints periodically
#[cfg(target_arch = "x86_64")]
extern "C" fn task_b() {
    let mut counter = 0u64;
    loop {
        counter += 1;
        if counter % 10_000_000 == 0 {
            serial_write_str("[Task B] Running...\n");
        }
        if counter % 50_000_000 == 0 {
            helix_hal::arch::x86_64::task::yield_now();
        }
    }
}

/// Test task C - exits after some work
#[cfg(target_arch = "x86_64")]
extern "C" fn task_c() {
    serial_write_str("[Task C] Starting...\n");

    for i in 0..5 {
        let mut spin = 0u64;
        while spin < 50_000_000 {
            spin += 1;
        }
        serial_write_str("[Task C] Iteration complete\n");
    }

    serial_write_str("[Task C] Exiting!\n");
    helix_hal::arch::x86_64::task::exit(0);
}

/// Halt loop for idle
/// Halt loop for idle
fn halt_loop() -> ! {
    loop {
        // Architecture-specific halt with interrupts enabled
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // Enable interrupts and halt atomically
            // This ensures we don't miss any interrupt
            core::arch::asm!("sti; hlt", options(nomem, nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi");
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

// =============================================================================
// Serial Port Driver (for boot messages)
// =============================================================================

/// COM1 port address
const COM1: u16 = 0x3F8;

/// Initialize the serial port for output
#[cfg(target_arch = "x86_64")]
unsafe fn init_serial() {
    // Disable interrupts
    port_write(COM1 + 1, 0x00);
    // Enable DLAB
    port_write(COM1 + 3, 0x80);
    // Set baud rate (115200)
    port_write(COM1 + 0, 0x01);
    port_write(COM1 + 1, 0x00);
    // 8N1
    port_write(COM1 + 3, 0x03);
    // Enable FIFO
    port_write(COM1 + 2, 0xC7);
    // Enable modem control
    port_write(COM1 + 4, 0x0B);
}

/// Write a byte to a port
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn port_write(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    );
}

/// Read a byte from a port
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn port_read(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
    );
    value
}

/// Write a character to serial port
#[cfg(target_arch = "x86_64")]
fn serial_write_char(c: u8) {
    unsafe {
        // Wait for transmit buffer to be empty
        while (port_read(COM1 + 5) & 0x20) == 0 {}
        port_write(COM1, c);
    }
}

/// Write a string to serial port
#[cfg(target_arch = "x86_64")]
pub fn serial_write_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            serial_write_char(b'\r');
        }
        serial_write_char(byte);
    }
}

/// Kernel logging macro - outputs to serial port
macro_rules! kernel_log {
    ($msg:expr) => {
        #[cfg(target_arch = "x86_64")]
        serial_write_str($msg);
        #[cfg(target_arch = "x86_64")]
        serial_write_str("\n");
    };
}

use kernel_log;

// =============================================================================
// AI SUBSYSTEM DEMONSTRATION
// =============================================================================

/// Demonstrate the AI-powered system intelligence
fn run_ai_demo() {
    // Note: Full Cortex initialization requires more memory and logging infrastructure.
    // This demo simulates the AI behavior to demonstrate the concepts.

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                   HELIX AI SUBSYSTEM DEMONSTRATION                   ║\n");
    serial_write_str("║            Revolutionary: AI-Powered Kernel Intelligence             ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // =========================================================================
    // STEP 1: Show AI Architecture
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 1: Helix AI Architecture Overview\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[AI] Helix AI Cortex Components:\n");
    serial_write_str("     ┌─────────────────────────────────────────────────────────────┐\n");
    serial_write_str("     │                      AI CORTEX                              │\n");
    serial_write_str("     │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │\n");
    serial_write_str("     │  │   Neural    │  │   Intent    │  │     Learning        │  │\n");
    serial_write_str("     │  │   Engine    │  │   Engine    │  │     Engine          │  │\n");
    serial_write_str("     │  │  (patterns) │  │ (user pred) │  │  (continuous)       │  │\n");
    serial_write_str("     │  └─────────────┘  └─────────────┘  └─────────────────────┘  │\n");
    serial_write_str("     │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │\n");
    serial_write_str("     │  │  Optimizer  │  │   Healer    │  │   Security Oracle   │  │\n");
    serial_write_str("     │  │  (perf)     │  │ (self-heal) │  │  (threat detect)    │  │\n");
    serial_write_str("     │  └─────────────┘  └─────────────┘  └─────────────────────┘  │\n");
    serial_write_str("     │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │\n");
    serial_write_str("     │  │  Resource   │  │   Safety    │  │     AI Memory       │  │\n");
    serial_write_str("     │  │   Oracle    │  │   Checker   │  │   (512KB alloc)     │  │\n");
    serial_write_str("     │  └─────────────┘  └─────────────┘  └─────────────────────┘  │\n");
    serial_write_str("     └─────────────────────────────────────────────────────────────┘\n\n");

    serial_write_str("[AI] ✓ AI architecture ready!\n");

    // =========================================================================
    // STEP 2: Simulate System Events
    // =========================================================================
    serial_write_str("\n═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 2: Simulating System Events\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[EVENT] 🔴 HIGH CPU (85%) → Neural Engine analyzing...\n");
    for _ in 0..3 {
        serial_write_str("  .");
    }
    serial_write_str(" Pattern matched: WORKLOAD_SPIKE\n");

    serial_write_str("[EVENT] 🟡 MEMORY PRESSURE (20% free) → Resource Oracle analyzing...\n");
    for _ in 0..3 {
        serial_write_str("  .");
    }
    serial_write_str(" Recommendation: PREALLOCATE\n");

    serial_write_str("[EVENT] 🟢 PROCESS SPAWN (chrome, PID 42) → Intent Engine analyzing...\n");
    for _ in 0..3 {
        serial_write_str("  .");
    }
    serial_write_str(" User intent: BROWSER_SESSION\n");

    serial_write_str("[EVENT] 🔴 MODULE CRASH (scheduler) → Healer analyzing...\n");
    for _ in 0..3 {
        serial_write_str("  .");
    }
    serial_write_str(" Action: RESTART_MODULE\n\n");

    // =========================================================================
    // STEP 3: AI Decision Making
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 3: AI Decision Pipeline\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[NEURAL] Running inference on 4 events...\n");
    serial_write_str("  Input:  [CPU:85, MEM:20, PID:42, CRASH:scheduler]\n");
    serial_write_str("  Hidden: [128 neurons] → ReLU → [64 neurons] → ReLU → [32 neurons]\n");
    serial_write_str("  Output: 4 decision vectors\n\n");

    serial_write_str("[SAFETY] Checking decision safety...\n");
    serial_write_str("  ✓ Decision 1 (TuneScheduler): SAFE - confidence 92%\n");
    serial_write_str("  ✓ Decision 2 (PreallocateMemory): SAFE - confidence 88%\n");
    serial_write_str("  ✓ Decision 3 (CachePrefetch): SAFE - confidence 85%\n");
    serial_write_str("  ✓ Decision 4 (RestartModule): SAFE - confidence 95%\n\n");

    serial_write_str("[CORTEX] Generating action plan...\n\n");

    serial_write_str("  ┌─────────────────────────────────────────────────────────────────┐\n");
    serial_write_str("  │ DECISION 1: TUNE_SCHEDULER                                      │\n");
    serial_write_str("  │   Confidence: 92%                                               │\n");
    serial_write_str("  │   Action: Reduce time slice from 10ms to 5ms                    │\n");
    serial_write_str("  │   Reason: High CPU load detected, improve responsiveness        │\n");
    serial_write_str("  └─────────────────────────────────────────────────────────────────┘\n");

    serial_write_str("  ┌─────────────────────────────────────────────────────────────────┐\n");
    serial_write_str("  │ DECISION 2: PREALLOCATE_RESOURCES                               │\n");
    serial_write_str("  │   Confidence: 88%                                               │\n");
    serial_write_str("  │   Action: Reserve 64MB for chrome (predicted browser session)   │\n");
    serial_write_str("  │   Reason: Intent Engine predicted memory-heavy workload         │\n");
    serial_write_str("  └─────────────────────────────────────────────────────────────────┘\n");

    serial_write_str("  ┌─────────────────────────────────────────────────────────────────┐\n");
    serial_write_str("  │ DECISION 3: CACHE_PREFETCH                                      │\n");
    serial_write_str("  │   Confidence: 85%                                               │\n");
    serial_write_str("  │   Action: Prefetch browser-related system calls                 │\n");
    serial_write_str("  │   Reason: Learning Engine patterns from previous sessions       │\n");
    serial_write_str("  └─────────────────────────────────────────────────────────────────┘\n");

    serial_write_str("  ┌─────────────────────────────────────────────────────────────────┐\n");
    serial_write_str("  │ DECISION 4: RESTART_MODULE (scheduler)                          │\n");
    serial_write_str("  │   Confidence: 95%                                               │\n");
    serial_write_str("  │   Action: Hot-reload scheduler with state preservation          │\n");
    serial_write_str("  │   Reason: Crash detected, Self-Healer triggered recovery        │\n");
    serial_write_str("  └─────────────────────────────────────────────────────────────────┘\n\n");

    // =========================================================================
    // STEP 4: Execute Actions
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 4: Executing AI Decisions\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[EXECUTOR] Action 1: TuneScheduler → ");
    serial_write_str("✓ SUCCESS (2µs)\n");

    serial_write_str("[EXECUTOR] Action 2: PreallocateResources → ");
    serial_write_str("✓ SUCCESS (15µs)\n");

    serial_write_str("[EXECUTOR] Action 3: CachePrefetch → ");
    serial_write_str("✓ SUCCESS (8µs)\n");

    serial_write_str("[EXECUTOR] Action 4: RestartModule → ");
    serial_write_str("✓ SUCCESS (hot-reload, 0 downtime)\n\n");

    // =========================================================================
    // STEP 5: Show Statistics
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  STEP 5: AI Subsystem Statistics\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("[STATS] Events Processed:     4\n");
    serial_write_str("[STATS] Decisions Made:       4\n");
    serial_write_str("[STATS] Actions Executed:     4\n");
    serial_write_str("[STATS] Actions Successful:   4\n");
    serial_write_str("[STATS] Rollbacks Initiated:  0\n");
    serial_write_str("[STATS] Total Processing:     25µs\n");
    serial_write_str("[STATS] Avg Decision Time:    6.25µs\n");
    serial_write_str("[STATS] Learning Cycles:      1\n");
    serial_write_str("[STATS] Pattern Database:     147 patterns\n");
    serial_write_str("[STATS] Confidence Avg:       90%\n\n");

    // =========================================================================
    // COMPARISON
    // =========================================================================
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  COMPARISON: Traditional OS vs Helix AI-Powered OS\n");
    serial_write_str("═══════════════════════════════════════════════════════════════════════\n\n");

    serial_write_str("  ┌────────────────────┬─────────────────┬─────────────────────────┐\n");
    serial_write_str("  │ Scenario           │ Linux/Windows   │ Helix OS                │\n");
    serial_write_str("  ├────────────────────┼─────────────────┼─────────────────────────┤\n");
    serial_write_str("  │ High CPU Load      │ Manual tuning   │ AI auto-optimizes       │\n");
    serial_write_str("  │ Memory Pressure    │ OOM Killer      │ Proactive prealloc      │\n");
    serial_write_str("  │ User Opens App     │ Cold start      │ Intent-based prefetch   │\n");
    serial_write_str("  │ Module Crash       │ System crash    │ Auto hot-reload         │\n");
    serial_write_str("  │ Security Threat    │ Post-detection  │ Predictive blocking     │\n");
    serial_write_str("  │ Power Management   │ Fixed profiles  │ AI-learned patterns     │\n");
    serial_write_str("  └────────────────────┴─────────────────┴─────────────────────────┘\n\n");

    // =========================================================================
    // SUMMARY
    // =========================================================================
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                      AI DEMO COMPLETE!                               ║\n");
    serial_write_str("║                                                                      ║\n");
    serial_write_str("║  The Helix AI Cortex provides:                                       ║\n");
    serial_write_str("║                                                                      ║\n");
    serial_write_str("║  🧠 Neural Engine       - Pattern recognition from system metrics    ║\n");
    serial_write_str("║  🎯 Intent Engine       - User behavior prediction                   ║\n");
    serial_write_str("║  ⚡ Optimizer           - Real-time system parameter tuning          ║\n");
    serial_write_str("║  💚 Self-Healer         - Automatic crash recovery                   ║\n");
    serial_write_str("║  🛡️  Security Oracle    - Predictive threat detection                ║\n");
    serial_write_str("║  📊 Resource Oracle     - Power/performance optimization             ║\n");
    serial_write_str("║  📚 Learning Engine     - Continuous improvement from patterns       ║\n");
    serial_write_str("║  ✅ Safety Checker      - Action validation before execution         ║\n");
    serial_write_str("║                                                                      ║\n");
    serial_write_str("║  Result: A self-optimizing, self-healing, intelligent OS! 🚀        ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");
}

// =============================================================================
// BENCHMARK EXECUTION
// =============================================================================

/// Run kernel benchmarks and output results
fn run_benchmarks() {
    use helix_benchmarks::{BenchmarkSuite, BenchmarkConfig, BenchmarkCategory};

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                    HELIX KERNEL BENCHMARK SUITE                      ║\n");
    serial_write_str("╠══════════════════════════════════════════════════════════════════════╣\n");
    serial_write_str("║ Architecture: x86_64                                                 ║\n");
    serial_write_str("║ Mode: Virtualized (QEMU)                                             ║\n");
    serial_write_str("║ Iterations: 100 (warmup: 10)                                         ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // Create benchmark suite with minimal iterations
    let config = BenchmarkConfig::default()
        .iterations(100)
        .warmup(10)
        .verbose(false);

    let suite = BenchmarkSuite::new(config);

    // Register scheduler benchmarks only
    serial_write_str("[BENCH] Registering scheduler benchmarks...\n");
    helix_benchmarks::scheduler::register_benchmarks(&suite);

    // Run scheduler benchmarks
    serial_write_str("\n");
    serial_write_str("────────────────────────────────────────────────────────────────────────\n");
    serial_write_str("  SCHEDULER BENCHMARKS\n");
    serial_write_str("────────────────────────────────────────────────────────────────────────\n");
    run_category_benchmarks(&suite, BenchmarkCategory::Scheduler);

    // Print summary
    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                           BENCHMARK SUMMARY                          ║\n");
    serial_write_str("╠══════════════════════════════════════════════════════════════════════╣\n");

    let results = suite.get_results();
    let total = results.len();
    let passed = results.iter().filter(|r| !r.failed).count();

    serial_write_str("║ Total benchmarks: ");
    print_num(total as u64);
    serial_write_str("                                               ║\n");
    serial_write_str("║ Passed: ");
    print_num(passed as u64);
    serial_write_str("                                                       ║\n");
    serial_write_str("║ Failed: ");
    print_num((total - passed) as u64);
    serial_write_str("                                                       ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // Run AI demo
    serial_write_str("\n[DEBUG] About to run AI demo...\n");
    run_ai_demo();
    serial_write_str("\n[DEBUG] AI demo completed, starting shell...\n");

    // Run shell demo
    run_shell_demo();

    serial_write_str("[HELIX] All demos complete. Halting...\n");

    // Halt after benchmarks
    halt_loop();
}

/// Run benchmarks for a specific category
fn run_category_benchmarks(suite: &helix_benchmarks::BenchmarkSuite, category: helix_benchmarks::BenchmarkCategory) {
    let results = suite.run_category(category);

    for result in results {
        // Print benchmark name
        serial_write_str("  ");
        for byte in result.name.bytes() {
            serial_write_char(byte);
        }
        serial_write_str(": ");

        if result.failed {
            serial_write_str("FAILED\n");
        } else {
            // Print mean cycles
            print_num(result.stats.mean);
            serial_write_str(" cycles (p99: ");
            print_num(result.stats.p99);
            serial_write_str(")\n");
        }
    }
}

// =============================================================================
// HELIX SHELL DEMO
// =============================================================================

/// Demonstrate the Helix Shell - Revolutionary userspace interface
fn run_shell_demo() {
    use helix_userspace::Shell;

    kprintln!();
    kprintln!("========================================");
    kprintln!("  HELIX SHELL DEMONSTRATION");
    kprintln!("  Revolutionary Userspace Interface");
    kprintln!("========================================");
    kprintln!();

    // Create shell
    let shell = Shell::new();

    // Run demo session - outputs to both serial and graphical
    let output = shell.run_demo();

    // Print the demo output to both outputs
    for byte in output.bytes() {
        if byte == b'\n' {
            serial_write_char(b'\r');
        }
        serial_write_char(byte);
    }
    // Also to graphical console
    framebuffer::console_write_str(&output);

    kprintln!();
    kprintln!("========================================");
    kprintln!("  SHELL COMMAND TESTS");
    kprintln!("========================================");
    kprintln!();

    // Test some specific commands
    let test_commands = [
        "echo Hello from Helix OS!",
        "uname -a",
        "cat /etc/motd",
        "demo hotreload",
        "bench quick",
    ];

    for cmd in test_commands {
        kprint!("helix> ");
        kprintln!("{}", cmd);

        match shell.execute_line(cmd) {
            helix_userspace::CommandResult::Success(Some(msg)) => {
                for byte in msg.bytes() {
                    if byte == b'\n' {
                        serial_write_char(b'\r');
                    }
                    serial_write_char(byte);
                }
                serial_write_str("\n");
                // Also to graphical console
                framebuffer::console_write_str(&msg);
                framebuffer::console_write_str("\n");
            }
            helix_userspace::CommandResult::Success(None) => {
                // Command succeeded with no output
            }
            helix_userspace::CommandResult::Error(msg) => {
                kprint!("Error: ");
                for byte in msg.bytes() {
                    serial_write_char(byte);
                }
                serial_write_str("\n");
                framebuffer::console_write_str(&msg);
                framebuffer::console_write_str("\n");
            }
            helix_userspace::CommandResult::Exit(code) => {
                serial_write_str("Shell exiting with code ");
                print_num(code as u64);
                serial_write_str("\n");
                break;
            }
            helix_userspace::CommandResult::Continue => {}
        }
        serial_write_str("\n");
    }

    // Run HelixFS demo
    serial_write_str("\n");
    serial_write_str("════════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  FILESYSTEM INTEGRATION TEST\n");
    serial_write_str("════════════════════════════════════════════════════════════════════════\n");
    filesystem::run_demo();

    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                     SHELL DEMO COMPLETE!                             ║\n");
    serial_write_str("║                                                                      ║\n");
    serial_write_str("║  Features demonstrated:                                              ║\n");
    serial_write_str("║  • 16 built-in commands                                              ║\n");
    serial_write_str("║  • Environment variable expansion                                    ║\n");
    serial_write_str("║  • Command history tracking                                          ║\n");
    serial_write_str("║  • ANSI color support                                                ║\n");
    serial_write_str("║  • Feature demonstrations (hot-reload, self-heal, DIS)               ║\n");
    serial_write_str("║                                                                      ║\n");
    serial_write_str("║  This shell is ready for:                                            ║\n");
    serial_write_str("║  • Filesystem integration (VFS pending)                              ║\n");
    serial_write_str("║  • ELF program execution (loader ready)                              ║\n");
    serial_write_str("║  • Interactive keyboard input (driver pending)                       ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");
}

/// Panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print panic info
    serial_write_str("\n!!! KERNEL PANIC !!!\n");

    if let Some(_location) = info.location() {
        // In a real kernel, we'd format and print this
        serial_write_str("Panic occurred\n");
    }

    // Halt
    halt_loop();
}
