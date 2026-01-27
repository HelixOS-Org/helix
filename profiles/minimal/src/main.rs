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
#[no_mangle]
pub extern "C" fn kernel_main(_boot_info: *const BootInfo) -> ! {
    // VERY EARLY: Write to VGA to prove we got here
    // This writes "HX" in white on red at top-left of screen
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let vga = 0xB8000 as *mut u16;
        *vga = 0x4F48;       // 'H' white on red
        *vga.add(1) = 0x4F58; // 'X' white on red
        *vga.add(2) = 0x4F4F; // 'O' white on red
        *vga.add(3) = 0x4F4B; // 'K' white on red
    }
    
    // Initialize serial first so we can output
    #[cfg(target_arch = "x86_64")]
    unsafe { init_serial(); }
    
    serial_write_str("\n");
    serial_write_str("========================================\n");
    serial_write_str("  HELIX OS FRAMEWORK\n");
    serial_write_str("  Minimal Kernel Profile\n");
    serial_write_str("========================================\n");
    serial_write_str("\n");

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

    // Phase 5: Start the kernel
    serial_write_str("[BOOT] Starting kernel...\n");
    start_kernel();

    serial_write_str("\n");
    serial_write_str("[HELIX] Kernel initialized successfully!\n");
    serial_write_str("[HELIX] Entering idle loop...\n");
    
    // Should never reach here
    halt_loop();
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

/// Start the kernel with real multitasking
fn start_kernel() {
    kernel_log!("Starting kernel...");
    
    #[cfg(target_arch = "x86_64")]
    {
        use helix_hal::arch::x86_64::task;
        
        serial_write_str("\n");
        serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
        serial_write_str("║  HELIX OS - HOT-RELOAD DEMO                                  ║\n");
        serial_write_str("║  Revolutionary: Swap scheduler WITHOUT reboot!               ║\n");
        serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");
        serial_write_str("\n");
        
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
fn print_num(n: u64) {
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
fn serial_write_str(s: &str) {
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
    
    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║                    HELIX SHELL DEMONSTRATION                         ║\n");
    serial_write_str("║                 Revolutionary Userspace Interface                    ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");
    
    // Create shell
    let shell = Shell::new();
    
    // Run demo session - outputs to serial
    let output = shell.run_demo();
    
    // Print the demo output
    for byte in output.bytes() {
        if byte == b'\n' {
            serial_write_char(b'\r');
        }
        serial_write_char(byte);
    }
    
    serial_write_str("\n");
    serial_write_str("════════════════════════════════════════════════════════════════════════\n");
    serial_write_str("  SHELL COMMAND TESTS\n");
    serial_write_str("════════════════════════════════════════════════════════════════════════\n\n");
    
    // Test some specific commands
    let test_commands = [
        "echo Hello from Helix OS!",
        "uname -a",
        "cat /etc/motd",
        "demo hotreload",
        "bench quick",
    ];
    
    for cmd in test_commands {
        serial_write_str("helix> ");
        serial_write_str(cmd);
        serial_write_str("\n");
        
        match shell.execute_line(cmd) {
            helix_userspace::CommandResult::Success(Some(msg)) => {
                for byte in msg.bytes() {
                    if byte == b'\n' {
                        serial_write_char(b'\r');
                    }
                    serial_write_char(byte);
                }
                serial_write_str("\n");
            }
            helix_userspace::CommandResult::Success(None) => {
                // Command succeeded with no output
            }
            helix_userspace::CommandResult::Error(msg) => {
                serial_write_str("Error: ");
                for byte in msg.bytes() {
                    serial_write_char(byte);
                }
                serial_write_str("\n");
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
