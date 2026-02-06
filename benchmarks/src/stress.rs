//! Stress Tests
//!
//! Tests for measuring system stability under load:
//! - Sustained workloads
//! - Memory pressure
//! - Concurrent operations
//! - Edge cases

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use spin::RwLock;

use crate::{benchmark, timing, BenchmarkCategory, BenchmarkDef, BenchmarkId, BenchmarkSuite};

// =============================================================================
// Benchmark Registration
// =============================================================================

/// Register all stress tests
pub fn register_benchmarks(suite: &BenchmarkSuite) {
    // CPU stress
    suite.register(benchmark!(
        "stress.cpu.spin",
        BenchmarkCategory::Stress,
        bench_cpu_spin
    ));

    suite.register(benchmark!(
        "stress.cpu.compute",
        BenchmarkCategory::Stress,
        bench_cpu_compute
    ));

    suite.register(benchmark!(
        "stress.cpu.context_flood",
        BenchmarkCategory::Stress,
        bench_context_flood
    ));

    // Memory stress
    suite.register(benchmark!(
        "stress.memory.alloc_free_cycle",
        BenchmarkCategory::Stress,
        bench_alloc_free_cycle
    ));

    suite.register(benchmark!(
        "stress.memory.fragmentation",
        BenchmarkCategory::Stress,
        bench_memory_fragmentation
    ));

    suite.register(benchmark!(
        "stress.memory.pressure",
        BenchmarkCategory::Stress,
        bench_memory_pressure
    ));

    // Scheduler stress
    suite.register(benchmark!(
        "stress.sched.many_tasks",
        BenchmarkCategory::Stress,
        bench_many_tasks
    ));

    suite.register(benchmark!(
        "stress.sched.priority_inversion",
        BenchmarkCategory::Stress,
        bench_priority_inversion
    ));

    suite.register(benchmark!(
        "stress.sched.thundering_herd",
        BenchmarkCategory::Stress,
        bench_thundering_herd
    ));

    // IPC stress
    suite.register(benchmark!(
        "stress.ipc.message_flood",
        BenchmarkCategory::Stress,
        bench_message_flood
    ));

    suite.register(benchmark!(
        "stress.ipc.channel_contention",
        BenchmarkCategory::Stress,
        bench_channel_contention
    ));

    // IRQ stress
    suite.register(benchmark!(
        "stress.irq.high_frequency",
        BenchmarkCategory::Stress,
        bench_irq_high_frequency
    ));

    suite.register(benchmark!(
        "stress.irq.nested_deep",
        BenchmarkCategory::Stress,
        bench_nested_irq_deep
    ));

    // Lock stress
    suite.register(benchmark!(
        "stress.lock.contention",
        BenchmarkCategory::Stress,
        bench_lock_contention
    ));

    suite.register(benchmark!(
        "stress.lock.reader_writer",
        BenchmarkCategory::Stress,
        bench_reader_writer_stress
    ));

    // Combined stress
    suite.register(benchmark!(
        "stress.combined.mixed_workload",
        BenchmarkCategory::Stress,
        bench_mixed_workload
    ));

    suite.register(benchmark!(
        "stress.combined.worst_case",
        BenchmarkCategory::Stress,
        bench_worst_case
    ));
}

// =============================================================================
// CPU Stress
// =============================================================================

/// Pure spin (baseline)
fn bench_cpu_spin() -> u64 {
    let start = timing::read_tsc();

    // Spin for fixed iterations
    for _ in 0..1000 {
        core::hint::spin_loop();
    }

    let end = timing::read_tsc();
    end - start
}

/// Compute-intensive workload
fn bench_cpu_compute() -> u64 {
    let start = timing::read_tsc();

    // Simulate compute workload
    let mut result = 0u64;

    // Integer operations
    for i in 0..100u64 {
        result = result.wrapping_add(i);
        result = result.wrapping_mul(i.wrapping_add(1));
        result ^= i << (i % 32);
    }

    // Bit manipulation
    for i in 0..50 {
        result = result.rotate_left(i % 64);
        result = !result;
        result &= 0xDEAD_BEEF_CAFE_BABE;
    }

    core::hint::black_box(result);

    let end = timing::read_tsc();
    end - start
}

/// Flood context switches
fn bench_context_flood() -> u64 {
    let start = timing::read_tsc();

    // Simulate 10 rapid context switches
    for _ in 0..10 {
        // Save context
        let mut regs = [0u64; 16];
        for i in 0..16 {
            unsafe {
                core::ptr::write_volatile(&mut regs[i], i as u64);
            }
        }

        // Switch
        static CURRENT: AtomicU64 = AtomicU64::new(0);
        CURRENT.fetch_add(1, Ordering::SeqCst);

        // Restore context
        let mut sum = 0u64;
        for i in 0..16 {
            sum += unsafe { core::ptr::read_volatile(&regs[i]) };
        }
        core::hint::black_box(sum);
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Memory Stress
// =============================================================================

/// Rapid alloc/free cycles
fn bench_alloc_free_cycle() -> u64 {
    let start = timing::read_tsc();

    static ALLOC_PTR: AtomicU64 = AtomicU64::new(0x1_0000_0000);
    static FREE_LIST: AtomicU64 = AtomicU64::new(0);

    // 20 alloc/free cycles
    for _ in 0..20 {
        // Alloc
        let ptr = if FREE_LIST.load(Ordering::Acquire) != 0 {
            FREE_LIST.swap(0, Ordering::SeqCst)
        } else {
            ALLOC_PTR.fetch_add(256, Ordering::SeqCst)
        };

        // Use
        core::hint::black_box(ptr);

        // Free
        FREE_LIST.store(ptr, Ordering::Release);
    }

    let end = timing::read_tsc();
    end - start
}

/// Cause fragmentation
fn bench_memory_fragmentation() -> u64 {
    let start = timing::read_tsc();

    // Simulate fragmented allocation pattern
    static BLOCKS: [AtomicU64; 32] = {
        const INIT: AtomicU64 = AtomicU64::new(0);
        [INIT; 32]
    };
    static ALLOC_PTR: AtomicU64 = AtomicU64::new(0x2_0000_0000);

    // Allocate all blocks
    for i in 0..32 {
        let ptr = ALLOC_PTR.fetch_add(128, Ordering::SeqCst);
        BLOCKS[i].store(ptr, Ordering::Release);
    }

    // Free odd blocks (creates fragmentation)
    for i in (1..32).step_by(2) {
        BLOCKS[i].store(0, Ordering::Release);
    }

    // Try to allocate larger block (won't fit in holes)
    let large = ALLOC_PTR.fetch_add(1024, Ordering::SeqCst);
    core::hint::black_box(large);

    // Free remaining
    for i in (0..32).step_by(2) {
        BLOCKS[i].store(0, Ordering::Release);
    }

    let end = timing::read_tsc();
    end - start
}

/// Memory pressure simulation
fn bench_memory_pressure() -> u64 {
    let start = timing::read_tsc();

    // Simulate near-OOM conditions
    static TOTAL_MEMORY: AtomicU64 = AtomicU64::new(100_000_000);
    static USED_MEMORY: AtomicU64 = AtomicU64::new(90_000_000);

    // Attempt allocation under pressure
    let alloc_size = 1_000_000u64;

    let total = TOTAL_MEMORY.load(Ordering::Acquire);
    let used = USED_MEMORY.load(Ordering::Acquire);
    let available = total - used;

    if alloc_size <= available {
        // Allocate
        USED_MEMORY.fetch_add(alloc_size, Ordering::SeqCst);
        core::hint::black_box(1);
    } else {
        // Trigger memory reclaim simulation
        for _ in 0..5 {
            // Check caches
            static CACHE_SIZE: AtomicU64 = AtomicU64::new(5_000_000);
            let reclaimable = CACHE_SIZE.load(Ordering::Acquire);

            if reclaimable > 0 {
                let reclaimed = reclaimable.min(alloc_size);
                CACHE_SIZE.fetch_sub(reclaimed, Ordering::SeqCst);
                USED_MEMORY.fetch_sub(reclaimed, Ordering::SeqCst);
            }

            core::hint::black_box(reclaimable);
        }
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Scheduler Stress
// =============================================================================

/// Many concurrent tasks
fn bench_many_tasks() -> u64 {
    let start = timing::read_tsc();

    // Simulate scheduling 64 tasks
    static TASK_QUEUE: RwLock<[u64; 64]> = RwLock::new([0; 64]);
    static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

    // Create 64 tasks
    {
        let mut queue = TASK_QUEUE.write();
        for i in 0..64 {
            queue[i] = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Schedule round-robin
    for _ in 0..64 {
        let task = {
            let queue = TASK_QUEUE.read();
            queue[0]
        };

        // Simulate running task
        core::hint::black_box(task);

        // Rotate queue
        {
            let mut queue = TASK_QUEUE.write();
            let first = queue[0];
            for i in 0..63 {
                queue[i] = queue[i + 1];
            }
            queue[63] = first;
        }
    }

    let end = timing::read_tsc();
    end - start
}

/// Priority inversion scenario
fn bench_priority_inversion() -> u64 {
    let start = timing::read_tsc();

    // Low priority task holds lock
    static LOCK: AtomicU32 = AtomicU32::new(1); // Held by low priority
    static LOW_PRIORITY: AtomicU32 = AtomicU32::new(10);
    static HIGH_PRIORITY: AtomicU32 = AtomicU32::new(90);

    // High priority tries to acquire
    let can_acquire = LOCK.load(Ordering::Acquire) == 0;

    if !can_acquire {
        // Priority inheritance: boost low priority
        let low = LOW_PRIORITY.load(Ordering::Acquire);
        let high = HIGH_PRIORITY.load(Ordering::Acquire);

        if low < high {
            LOW_PRIORITY.store(high, Ordering::Release);
            core::hint::black_box("boosted");
        }

        // Wait for lock
        while LOCK.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
            // Simulate low priority releasing
            LOCK.store(0, Ordering::Release);
            break;
        }

        // Restore priority
        LOW_PRIORITY.store(10, Ordering::Release);
    }

    // Acquire lock
    LOCK.store(1, Ordering::SeqCst);

    // Release lock
    LOCK.store(0, Ordering::Release);

    let end = timing::read_tsc();

    // Reset
    LOCK.store(1, Ordering::Release);

    end - start
}

/// Thundering herd problem
fn bench_thundering_herd() -> u64 {
    let start = timing::read_tsc();

    // 16 tasks waiting on event
    static WAITING: AtomicU32 = AtomicU32::new(16);
    static EVENT: AtomicBool = AtomicBool::new(false);

    // Signal event
    EVENT.store(true, Ordering::Release);

    // All tasks wake up
    let mut woken = 0u32;
    while WAITING.load(Ordering::Acquire) > 0 {
        // Simulate task waking
        if EVENT.load(Ordering::Acquire) {
            WAITING.fetch_sub(1, Ordering::SeqCst);
            woken += 1;
        }
    }

    // Only one task should succeed
    static RESOURCE: AtomicU32 = AtomicU32::new(0);
    let winner = RESOURCE.compare_exchange(0, 1, Ordering::SeqCst, Ordering::Relaxed);

    let _ = core::hint::black_box((woken, winner));

    let end = timing::read_tsc();

    // Reset
    WAITING.store(16, Ordering::Release);
    EVENT.store(false, Ordering::Release);
    RESOURCE.store(0, Ordering::Release);

    end - start
}

// =============================================================================
// IPC Stress
// =============================================================================

/// Flood messages
fn bench_message_flood() -> u64 {
    let start = timing::read_tsc();

    static MSG_COUNT: AtomicU32 = AtomicU32::new(0);

    // Send 50 messages rapidly
    for _ in 0..50 {
        MSG_COUNT.fetch_add(1, Ordering::SeqCst);

        // Minimal message setup
        let msg_type = 1u64;
        let payload = 42u64;
        core::hint::black_box((msg_type, payload));
    }

    // Drain all messages
    while MSG_COUNT.load(Ordering::Acquire) > 0 {
        MSG_COUNT.fetch_sub(1, Ordering::SeqCst);
    }

    let end = timing::read_tsc();
    end - start
}

/// Channel contention
fn bench_channel_contention() -> u64 {
    let start = timing::read_tsc();

    // Multiple producers, single consumer
    static CHANNEL: RwLock<[u64; 16]> = RwLock::new([0; 16]);
    static HEAD: AtomicU32 = AtomicU32::new(0);
    static TAIL: AtomicU32 = AtomicU32::new(0);

    // 8 producers each send 2 messages
    for producer in 0..8u64 {
        for msg in 0..2u64 {
            loop {
                let head = HEAD.load(Ordering::Acquire);
                let tail = TAIL.load(Ordering::Acquire);

                // Check if full
                if (head + 1) % 16 != tail {
                    // Try to reserve slot
                    if HEAD
                        .compare_exchange(
                            head,
                            (head + 1) % 16,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        // Write message
                        {
                            let mut channel = CHANNEL.write();
                            channel[head as usize] = producer * 10 + msg;
                        }
                        break;
                    }
                } else {
                    break; // Channel full
                }
            }
        }
    }

    // Consumer drains
    loop {
        let head = HEAD.load(Ordering::Acquire);
        let tail = TAIL.load(Ordering::Acquire);

        if head == tail {
            break; // Empty
        }

        // Read message
        let msg = {
            let channel = CHANNEL.read();
            channel[tail as usize]
        };

        TAIL.store((tail + 1) % 16, Ordering::Release);
        core::hint::black_box(msg);
    }

    let end = timing::read_tsc();

    // Reset
    HEAD.store(0, Ordering::Release);
    TAIL.store(0, Ordering::Release);

    end - start
}

// =============================================================================
// IRQ Stress
// =============================================================================

/// High frequency interrupts
fn bench_irq_high_frequency() -> u64 {
    let start = timing::read_tsc();

    // Simulate 100 rapid interrupts
    static IRQ_COUNT: AtomicU64 = AtomicU64::new(0);

    for _ in 0..100 {
        // Minimal IRQ handler
        IRQ_COUNT.fetch_add(1, Ordering::Relaxed);

        // EOI
        static EOI: AtomicU32 = AtomicU32::new(0);
        EOI.store(0, Ordering::Relaxed);
    }

    let end = timing::read_tsc();
    end - start
}

/// Deep nested interrupts
fn bench_nested_irq_deep() -> u64 {
    let start = timing::read_tsc();

    static NEST_LEVEL: AtomicU32 = AtomicU32::new(0);
    const MAX_NEST: u32 = 8;

    // Recursive interrupt simulation
    fn handle_irq(depth: u32) {
        if depth >= MAX_NEST {
            return;
        }

        NEST_LEVEL.fetch_add(1, Ordering::SeqCst);

        // Save state
        let mut regs = [0u64; 8];
        for i in 0..8 {
            unsafe {
                core::ptr::write_volatile(&mut regs[i], depth as u64 + i as u64);
            }
        }

        // Nested interrupt
        handle_irq(depth + 1);

        // Restore state
        let mut sum = 0u64;
        for i in 0..8 {
            sum += unsafe { core::ptr::read_volatile(&regs[i]) };
        }
        core::hint::black_box(sum);

        NEST_LEVEL.fetch_sub(1, Ordering::SeqCst);
    }

    handle_irq(0);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Lock Stress
// =============================================================================

/// Heavy lock contention
fn bench_lock_contention() -> u64 {
    let start = timing::read_tsc();

    static LOCK: AtomicU32 = AtomicU32::new(0);
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    // 20 lock/unlock cycles with contention simulation
    for _ in 0..20 {
        // Acquire
        let mut spins = 0u32;
        while LOCK
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spins += 1;
            core::hint::spin_loop();
            if spins > 100 {
                break; // Prevent infinite loop
            }
        }

        // Critical section
        COUNTER.fetch_add(1, Ordering::Relaxed);

        // Release
        LOCK.store(0, Ordering::Release);
    }

    let end = timing::read_tsc();
    end - start
}

/// Reader/writer lock stress
fn bench_reader_writer_stress() -> u64 {
    let start = timing::read_tsc();

    static DATA: RwLock<u64> = RwLock::new(0);

    // Mixed read/write pattern
    for i in 0..30 {
        if i % 5 == 0 {
            // Write
            let mut data = DATA.write();
            *data = i as u64;
        } else {
            // Read
            let data = DATA.read();
            core::hint::black_box(*data);
        }
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Combined Stress
// =============================================================================

/// Mixed workload
fn bench_mixed_workload() -> u64 {
    let start = timing::read_tsc();

    // Interleave different operations
    for i in 0..10 {
        // Memory operation
        static MEM: AtomicU64 = AtomicU64::new(0);
        MEM.fetch_add(i as u64, Ordering::SeqCst);

        // Compute
        let mut result = i as u64;
        for _ in 0..10 {
            result = result.wrapping_mul(7);
        }
        core::hint::black_box(result);

        // IPC simulation
        static MSG_QUEUE: AtomicU32 = AtomicU32::new(0);
        MSG_QUEUE.fetch_add(1, Ordering::SeqCst);
        MSG_QUEUE.fetch_sub(1, Ordering::SeqCst);

        // IRQ simulation
        static IRQ: AtomicU64 = AtomicU64::new(0);
        IRQ.fetch_add(1, Ordering::Relaxed);
    }

    let end = timing::read_tsc();
    end - start
}

/// Worst case scenario
fn bench_worst_case() -> u64 {
    let start = timing::read_tsc();

    // Everything at once
    static LOCK: spin::Mutex<u64> = spin::Mutex::new(0);
    static IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
    static MSG_COUNT: AtomicU32 = AtomicU32::new(0);

    for _ in 0..5 {
        // Lock contention
        {
            let mut guard = LOCK.lock();
            *guard += 1;

            // Nested IRQ while holding lock
            IRQ_COUNT.fetch_add(1, Ordering::SeqCst);

            // Memory pressure
            static MEM: [AtomicU64; 64] = {
                const INIT: AtomicU64 = AtomicU64::new(0);
                [INIT; 64]
            };
            for j in 0..64 {
                MEM[j].fetch_add(1, Ordering::Relaxed);
            }

            // IPC flood
            for _ in 0..10 {
                MSG_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Drain messages
        while MSG_COUNT.load(Ordering::Acquire) > 0 {
            MSG_COUNT.fetch_sub(1, Ordering::Relaxed);
        }
    }

    let end = timing::read_tsc();
    end - start
}
