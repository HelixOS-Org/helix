//! IRQ (Interrupt) Benchmarks
//!
//! Tests for measuring interrupt handling performance:
//! - IRQ latency
//! - Handler overhead
//! - Nested interrupts
//! - IRQ throughput

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crate::{benchmark, timing, BenchmarkCategory, BenchmarkDef, BenchmarkId, BenchmarkSuite};

// =============================================================================
// Benchmark Registration
// =============================================================================

/// Register all IRQ benchmarks
pub fn register_benchmarks(suite: &BenchmarkSuite) {
    // Latency benchmarks
    suite.register(benchmark!(
        "irq.latency.minimal",
        BenchmarkCategory::Irq,
        bench_irq_latency_minimal
    ));

    suite.register(benchmark!(
        "irq.latency.with_save",
        BenchmarkCategory::Irq,
        bench_irq_latency_with_save
    ));

    suite.register(benchmark!(
        "irq.latency.full",
        BenchmarkCategory::Irq,
        bench_irq_latency_full
    ));

    // Handler benchmarks
    suite.register(benchmark!(
        "irq.handler.dispatch",
        BenchmarkCategory::Irq,
        bench_irq_dispatch
    ));

    suite.register(benchmark!(
        "irq.handler.timer",
        BenchmarkCategory::Irq,
        bench_timer_handler
    ));

    suite.register(benchmark!(
        "irq.handler.keyboard",
        BenchmarkCategory::Irq,
        bench_keyboard_handler
    ));

    suite.register(benchmark!(
        "irq.handler.network",
        BenchmarkCategory::Irq,
        bench_network_handler
    ));

    // Control benchmarks
    suite.register(benchmark!(
        "irq.control.enable",
        BenchmarkCategory::Irq,
        bench_irq_enable
    ));

    suite.register(benchmark!(
        "irq.control.disable",
        BenchmarkCategory::Irq,
        bench_irq_disable
    ));

    suite.register(benchmark!(
        "irq.control.mask",
        BenchmarkCategory::Irq,
        bench_irq_mask
    ));

    suite.register(benchmark!(
        "irq.control.unmask",
        BenchmarkCategory::Irq,
        bench_irq_unmask
    ));

    // ACK benchmarks
    suite.register(benchmark!(
        "irq.ack.pic",
        BenchmarkCategory::Irq,
        bench_pic_ack
    ));

    suite.register(benchmark!(
        "irq.ack.apic",
        BenchmarkCategory::Irq,
        bench_apic_ack
    ));

    // Exception benchmarks
    suite.register(benchmark!(
        "irq.exception.dispatch",
        BenchmarkCategory::Irq,
        bench_exception_dispatch
    ));

    suite.register(benchmark!(
        "irq.exception.page_fault",
        BenchmarkCategory::Irq,
        bench_page_fault_exception
    ));

    // Throughput benchmarks
    suite.register(benchmark!(
        "irq.throughput.timer",
        BenchmarkCategory::Irq,
        bench_irq_throughput
    ));

    // Nested interrupts
    suite.register(benchmark!(
        "irq.nested.two_levels",
        BenchmarkCategory::Irq,
        bench_nested_irq
    ));
}

// =============================================================================
// IRQ Latency Benchmarks
// =============================================================================

/// Minimal IRQ latency (just entry/exit)
fn bench_irq_latency_minimal() -> u64 {
    let start = timing::read_tsc();

    // Simulate minimal interrupt entry:
    // 1. Hardware pushes SS, RSP, RFLAGS, CS, RIP (5 x 8 bytes)
    let mut stack = [0u64; 8];
    unsafe {
        core::ptr::write_volatile(&mut stack[0], 0x10); // SS
        core::ptr::write_volatile(&mut stack[1], 0x7FFF_0000); // RSP
        core::ptr::write_volatile(&mut stack[2], 0x202); // RFLAGS
        core::ptr::write_volatile(&mut stack[3], 0x08); // CS
        core::ptr::write_volatile(&mut stack[4], 0x1000); // RIP
    }

    // Interrupt handler entry
    core::hint::black_box(&stack);

    // Interrupt handler exit (iretq)
    // Pop values back
    let _ss = unsafe { core::ptr::read_volatile(&stack[0]) };
    let _rsp = unsafe { core::ptr::read_volatile(&stack[1]) };
    let _rflags = unsafe { core::ptr::read_volatile(&stack[2]) };
    let _cs = unsafe { core::ptr::read_volatile(&stack[3]) };
    let _rip = unsafe { core::ptr::read_volatile(&stack[4]) };

    let end = timing::read_tsc();
    end - start
}

/// IRQ latency with register save
fn bench_irq_latency_with_save() -> u64 {
    let start = timing::read_tsc();

    // Hardware frame
    let mut hw_frame = [0u64; 5];
    for (i, frame) in hw_frame.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(frame, i as u64);
        }
    }

    // Save all general purpose registers (15 + error code)
    let mut gp_regs = [0u64; 16];
    for (i, reg) in gp_regs.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(reg, i as u64);
        }
    }

    // Handler body (minimal)
    core::hint::black_box(&gp_regs);

    // Restore registers
    let mut sum = 0u64;
    for reg in &gp_regs {
        sum += unsafe { core::ptr::read_volatile(reg) };
    }
    for frame in &hw_frame {
        sum += unsafe { core::ptr::read_volatile(frame) };
    }

    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

/// Full IRQ latency (save, dispatch, ack, restore)
fn bench_irq_latency_full() -> u64 {
    let start = timing::read_tsc();

    // 1. Hardware frame saved
    let mut hw_frame = [0u64; 5];
    for (i, frame) in hw_frame.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(frame, i as u64);
        }
    }

    // 2. Save GP registers
    let mut gp_regs = [0u64; 16];
    for (i, reg) in gp_regs.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(reg, i as u64);
        }
    }

    // 3. Read interrupt number
    let irq_num = 32u8; // Timer interrupt
    core::hint::black_box(irq_num);

    // 4. Dispatch to handler
    static HANDLER_TABLE: [fn(); 4] = [dummy_handler, dummy_handler, dummy_handler, dummy_handler];
    let handler_idx = (irq_num - 32) as usize % 4;
    HANDLER_TABLE[handler_idx]();

    // 5. Send EOI
    static APIC_EOI: AtomicU32 = AtomicU32::new(0);
    APIC_EOI.store(0, Ordering::SeqCst);

    // 6. Restore registers
    let mut sum = 0u64;
    for reg in &gp_regs {
        sum += unsafe { core::ptr::read_volatile(reg) };
    }

    // 7. IRETQ
    for frame in &hw_frame {
        sum += unsafe { core::ptr::read_volatile(frame) };
    }

    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

fn dummy_handler() {
    core::hint::black_box(42);
}

// =============================================================================
// Handler Benchmarks
// =============================================================================

/// IRQ dispatch overhead
fn bench_irq_dispatch() -> u64 {
    let start = timing::read_tsc();

    // Read interrupt vector
    let vector = 33u8;

    // Lookup handler in IDT
    static HANDLERS: [AtomicU64; 256] = {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: AtomicU64 = AtomicU64::new(0);
        [INIT; 256]
    };

    let handler_addr = HANDLERS[vector as usize].load(Ordering::Acquire);

    // Validate handler
    let is_valid = handler_addr != 0;

    // Call handler (simulated)
    if is_valid {
        core::hint::black_box(handler_addr);
    }

    let end = timing::read_tsc();
    end - start
}

/// Timer interrupt handler
fn bench_timer_handler() -> u64 {
    let start = timing::read_tsc();

    // Acknowledge interrupt
    static APIC_EOI: AtomicU32 = AtomicU32::new(0);
    APIC_EOI.store(0, Ordering::SeqCst);

    // Update tick count
    static TICKS: AtomicU64 = AtomicU64::new(0);
    let tick = TICKS.fetch_add(1, Ordering::SeqCst);

    // Update system time
    static SYSTEM_TIME: AtomicU64 = AtomicU64::new(0);
    SYSTEM_TIME.fetch_add(1_000_000, Ordering::SeqCst); // 1ms

    // Check for preemption
    static PREEMPT_PENDING: AtomicBool = AtomicBool::new(false);
    if tick % 10 == 0 {
        PREEMPT_PENDING.store(true, Ordering::Release);
    }

    // Wake sleeping tasks
    static SLEEPING_TASKS: AtomicU32 = AtomicU32::new(0);
    let sleeping = SLEEPING_TASKS.load(Ordering::Acquire);
    if sleeping > 0 {
        // Would iterate and check timers
        core::hint::black_box(sleeping);
    }

    let end = timing::read_tsc();
    end - start
}

/// Keyboard interrupt handler
fn bench_keyboard_handler() -> u64 {
    let start = timing::read_tsc();

    // Read scancode from controller (simulated port read)
    static KEYBOARD_PORT: AtomicU32 = AtomicU32::new(0x1E); // 'A' key
    let scancode = KEYBOARD_PORT.load(Ordering::Acquire) as u8;

    // Translate scancode to keycode
    let keycode = translate_scancode(scancode);
    core::hint::black_box(keycode);

    // Add to keyboard buffer
    static KB_BUFFER_HEAD: AtomicU32 = AtomicU32::new(0);
    static KB_BUFFER: [AtomicU32; 64] = {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: AtomicU32 = AtomicU32::new(0);
        [INIT; 64]
    };

    let pos = KB_BUFFER_HEAD.fetch_add(1, Ordering::SeqCst) as usize % 64;
    KB_BUFFER[pos].store(keycode as u32, Ordering::Release);

    // Wake waiting processes (simulated)
    static KB_WAITERS: AtomicU32 = AtomicU32::new(0);
    if KB_WAITERS.load(Ordering::Acquire) > 0 {
        core::hint::black_box(1); // Would wake
    }

    // Send EOI
    static PIC_EOI: AtomicU32 = AtomicU32::new(0);
    PIC_EOI.store(0x20, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

fn translate_scancode(scancode: u8) -> u8 {
    // Simplified scancode translation
    match scancode {
        0x1E => b'a',
        0x30 => b'b',
        0x2E => b'c',
        _ => 0,
    }
}

/// Network interrupt handler
fn bench_network_handler() -> u64 {
    let start = timing::read_tsc();

    // Read interrupt status
    static NIC_STATUS: AtomicU32 = AtomicU32::new(0x01); // RX complete
    let status = NIC_STATUS.load(Ordering::Acquire);

    // Handle RX
    if status & 0x01 != 0 {
        // Read packet descriptor
        static RX_DESC: AtomicU64 = AtomicU64::new(0x1000);
        let desc_addr = RX_DESC.load(Ordering::Acquire);
        core::hint::black_box(desc_addr);

        // Get packet buffer
        let buffer_addr = desc_addr + 16;
        let packet_len = 1500u32;
        core::hint::black_box((buffer_addr, packet_len));

        // Add to RX queue
        static RX_QUEUE_HEAD: AtomicU32 = AtomicU32::new(0);
        RX_QUEUE_HEAD.fetch_add(1, Ordering::SeqCst);

        // Advance descriptor ring
        static RX_RING_HEAD: AtomicU32 = AtomicU32::new(0);
        RX_RING_HEAD.fetch_add(1, Ordering::SeqCst);
    }

    // Handle TX complete
    if status & 0x02 != 0 {
        static TX_COMPLETE: AtomicU32 = AtomicU32::new(0);
        TX_COMPLETE.fetch_add(1, Ordering::SeqCst);
    }

    // Clear interrupt
    NIC_STATUS.store(0, Ordering::Release);

    // Send EOI
    static APIC_EOI: AtomicU32 = AtomicU32::new(0);
    APIC_EOI.store(0, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// IRQ Control Benchmarks
// =============================================================================

/// Enable interrupts
fn bench_irq_enable() -> u64 {
    let start = timing::read_tsc();

    // Simulate STI
    static INTERRUPT_FLAG: AtomicBool = AtomicBool::new(false);
    INTERRUPT_FLAG.store(true, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

/// Disable interrupts
fn bench_irq_disable() -> u64 {
    let start = timing::read_tsc();

    // Simulate CLI
    static INTERRUPT_FLAG: AtomicBool = AtomicBool::new(true);
    INTERRUPT_FLAG.store(false, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

/// Mask IRQ line
fn bench_irq_mask() -> u64 {
    let start = timing::read_tsc();

    // Simulate masking IRQ 1 (keyboard) in PIC
    static PIC_MASK: AtomicU32 = AtomicU32::new(0);
    let irq = 1u32;
    PIC_MASK.fetch_or(1 << irq, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

/// Unmask IRQ line
fn bench_irq_unmask() -> u64 {
    let start = timing::read_tsc();

    // Simulate unmasking IRQ 1 in PIC
    static PIC_MASK: AtomicU32 = AtomicU32::new(0xFF);
    let irq = 1u32;
    PIC_MASK.fetch_and(!(1 << irq), Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// ACK Benchmarks
// =============================================================================

/// PIC EOI (End of Interrupt)
fn bench_pic_ack() -> u64 {
    let start = timing::read_tsc();

    // Send EOI to master PIC
    static PIC_CMD: AtomicU32 = AtomicU32::new(0);
    PIC_CMD.store(0x20, Ordering::SeqCst);

    // If IRQ >= 8, also send to slave
    let irq = 5u8;
    if irq >= 8 {
        static PIC2_CMD: AtomicU32 = AtomicU32::new(0);
        PIC2_CMD.store(0x20, Ordering::SeqCst);
    }
    core::hint::black_box(irq);

    let end = timing::read_tsc();
    end - start
}

/// APIC EOI
fn bench_apic_ack() -> u64 {
    let start = timing::read_tsc();

    // Write to APIC EOI register
    static APIC_EOI: AtomicU32 = AtomicU32::new(0);
    APIC_EOI.store(0, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Exception Benchmarks
// =============================================================================

/// Exception dispatch
fn bench_exception_dispatch() -> u64 {
    let start = timing::read_tsc();

    // Lookup exception handler
    let exception_num = 14u8; // Page fault

    static EXCEPTION_HANDLERS: [AtomicU64; 32] = {
        #[allow(clippy::declare_interior_mutable_const)]
        const INIT: AtomicU64 = AtomicU64::new(0xDEAD);
        [INIT; 32]
    };

    let handler = EXCEPTION_HANDLERS[exception_num as usize].load(Ordering::Acquire);

    // Build exception frame
    let frame = ExceptionFrame {
        _error_code: 0x02,
        _rip: 0x1000,
        _cs: 0x08,
        _rflags: 0x202,
        _rsp: 0x7FFF_0000,
        _ss: 0x10,
    };

    core::hint::black_box((handler, frame));

    let end = timing::read_tsc();
    end - start
}

/// Page fault exception handler
fn bench_page_fault_exception() -> u64 {
    let start = timing::read_tsc();

    // Read faulting address (CR2)
    static CR2: AtomicU64 = AtomicU64::new(0x1000_1234);
    let fault_addr = CR2.load(Ordering::Acquire);

    // Decode error code
    let error = 0x02u64; // Write fault
    let is_present = (error & 1) != 0;
    let is_write = (error & 2) != 0;
    let is_user = (error & 4) != 0;

    core::hint::black_box((is_present, is_write, is_user));

    // Lookup VMA
    let vma_found = true;

    if vma_found {
        // Handle fault (allocate page, etc.)
        static PAGE_ALLOC: AtomicU64 = AtomicU64::new(0x2000_0000);
        let page = PAGE_ALLOC.fetch_add(4096, Ordering::SeqCst);

        // Map page
        core::hint::black_box((fault_addr, page));
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Throughput Benchmarks
// =============================================================================

/// IRQ throughput (simulated rapid timer)
fn bench_irq_throughput() -> u64 {
    let start = timing::read_tsc();

    // Simulate 10 consecutive timer interrupts
    for _ in 0..10 {
        // Minimal timer handler
        static TICKS: AtomicU64 = AtomicU64::new(0);
        TICKS.fetch_add(1, Ordering::Relaxed);

        // EOI
        static APIC_EOI: AtomicU32 = AtomicU32::new(0);
        APIC_EOI.store(0, Ordering::Relaxed);
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Nested Interrupt Benchmarks
// =============================================================================

/// Two-level nested interrupt
fn bench_nested_irq() -> u64 {
    let start = timing::read_tsc();

    // First interrupt (timer)
    static LEVEL: AtomicU32 = AtomicU32::new(0);
    LEVEL.fetch_add(1, Ordering::SeqCst);

    // Save state
    let mut outer_regs = [0u64; 16];
    for (i, reg) in outer_regs.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(reg, i as u64);
        }
    }

    // Enable interrupts (simulate STI in handler)
    // Second interrupt arrives
    LEVEL.fetch_add(1, Ordering::SeqCst);

    // Save nested state
    let mut inner_regs = [0u64; 16];
    for (i, reg) in inner_regs.iter_mut().enumerate() {
        unsafe {
            core::ptr::write_volatile(reg, i as u64 + 100);
        }
    }

    // Inner handler completes
    let mut sum = 0u64;
    for reg in &inner_regs {
        sum += unsafe { core::ptr::read_volatile(reg) };
    }

    LEVEL.fetch_sub(1, Ordering::SeqCst);

    // Outer handler completes
    for reg in &outer_regs {
        sum += unsafe { core::ptr::read_volatile(reg) };
    }

    LEVEL.fetch_sub(1, Ordering::SeqCst);

    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

// =============================================================================
// Helper Types
// =============================================================================

#[derive(Clone, Copy)]
struct ExceptionFrame {
    _error_code: u64,
    _rip: u64,
    _cs: u64,
    _rflags: u64,
    _rsp: u64,
    _ss: u64,
}
