//! Scheduler Benchmarks
//!
//! Tests for measuring scheduler performance:
//! - Context switch latency
//! - Thread creation/destruction
//! - Priority scheduling
//! - Preemption latency
//! - Multi-core load balancing

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::{
    benchmark, timing, BenchmarkCategory, BenchmarkDef, BenchmarkId, BenchmarkSuite,
};

// =============================================================================
// Benchmark Registration
// =============================================================================

/// Register all scheduler benchmarks
pub fn register_benchmarks(suite: &BenchmarkSuite) {
    // Context switch benchmarks
    suite.register(benchmark!(
        "sched.context_switch.null",
        BenchmarkCategory::Scheduler,
        bench_null_context_switch
    ));

    suite.register(benchmark!(
        "sched.context_switch.full",
        BenchmarkCategory::Scheduler,
        bench_full_context_switch
    ));

    suite.register(benchmark!(
        "sched.context_switch.fpu",
        BenchmarkCategory::Scheduler,
        bench_context_switch_with_fpu
    ));

    // Thread lifecycle
    suite.register(benchmark!(
        "sched.thread.create",
        BenchmarkCategory::Scheduler,
        bench_thread_create
    ));

    suite.register(benchmark!(
        "sched.thread.destroy",
        BenchmarkCategory::Scheduler,
        bench_thread_destroy
    ));

    suite.register(benchmark!(
        "sched.thread.yield",
        BenchmarkCategory::Scheduler,
        bench_thread_yield
    ));

    // Priority scheduling
    suite.register(benchmark!(
        "sched.priority.set",
        BenchmarkCategory::Scheduler,
        bench_priority_set
    ));

    suite.register(benchmark!(
        "sched.priority.boost",
        BenchmarkCategory::Scheduler,
        bench_priority_boost
    ));

    // Queue operations
    suite.register(benchmark!(
        "sched.queue.enqueue",
        BenchmarkCategory::Scheduler,
        bench_queue_enqueue
    ));

    suite.register(benchmark!(
        "sched.queue.dequeue",
        BenchmarkCategory::Scheduler,
        bench_queue_dequeue
    ));

    suite.register(benchmark!(
        "sched.queue.requeue",
        BenchmarkCategory::Scheduler,
        bench_queue_requeue
    ));

    // Preemption
    suite.register(benchmark!(
        "sched.preempt.latency",
        BenchmarkCategory::Scheduler,
        bench_preemption_latency
    ));

    // Tick handling
    suite.register(benchmark!(
        "sched.tick.handler",
        BenchmarkCategory::Scheduler,
        bench_tick_handler
    ));

    // Intent-based scheduling (DIS-specific)
    suite.register(benchmark!(
        "sched.dis.intent_evaluation",
        BenchmarkCategory::Scheduler,
        bench_intent_evaluation
    ));

    suite.register(benchmark!(
        "sched.dis.policy_evaluation",
        BenchmarkCategory::Scheduler,
        bench_policy_evaluation
    ));

    suite.register(benchmark!(
        "sched.dis.optimization_hint",
        BenchmarkCategory::Scheduler,
        bench_optimization_hint
    ));
}

// =============================================================================
// Context Switch Benchmarks
// =============================================================================

/// Benchmark null context switch (save/restore only, no actual switch)
fn bench_null_context_switch() -> u64 {
    // Simulate saving CPU state
    let start = timing::read_tsc();

    // Simulate register save (volatile writes)
    let mut regs = [0u64; 16];
    for i in 0..16 {
        unsafe {
            core::ptr::write_volatile(&mut regs[i], i as u64);
        }
    }

    // Simulate register restore (volatile reads)
    let mut sum = 0u64;
    for i in 0..16 {
        sum += unsafe { core::ptr::read_volatile(&regs[i]) };
    }

    let end = timing::read_tsc();

    // Prevent optimization
    core::hint::black_box(sum);

    end - start
}

/// Benchmark full context switch simulation
fn bench_full_context_switch() -> u64 {
    let start = timing::read_tsc();

    // Save general purpose registers (16)
    let mut gp_regs = [0u64; 16];
    for i in 0..16 {
        unsafe {
            core::ptr::write_volatile(&mut gp_regs[i], i as u64);
        }
    }

    // Save segment registers (6)
    let mut seg_regs = [0u16; 6];
    for i in 0..6 {
        unsafe {
            core::ptr::write_volatile(&mut seg_regs[i], i as u16);
        }
    }

    // Save control registers (5)
    let mut ctrl_regs = [0u64; 5];
    for i in 0..5 {
        unsafe {
            core::ptr::write_volatile(&mut ctrl_regs[i], i as u64);
        }
    }

    // Simulate TLB flush cost
    core::hint::spin_loop();
    core::hint::spin_loop();

    // Restore
    let mut sum = 0u64;
    for i in 0..16 {
        sum += unsafe { core::ptr::read_volatile(&gp_regs[i]) };
    }
    for i in 0..6 {
        sum += unsafe { core::ptr::read_volatile(&seg_regs[i]) } as u64;
    }
    for i in 0..5 {
        sum += unsafe { core::ptr::read_volatile(&ctrl_regs[i]) };
    }

    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

/// Benchmark context switch with FPU state
fn bench_context_switch_with_fpu() -> u64 {
    let start = timing::read_tsc();

    // Save FPU/SSE state (512 bytes for FXSAVE, 1024 for XSAVE)
    let mut fpu_state = [0u8; 512];
    for i in 0..512 {
        unsafe {
            core::ptr::write_volatile(&mut fpu_state[i], i as u8);
        }
    }

    // General registers
    let mut regs = [0u64; 16];
    for i in 0..16 {
        unsafe {
            core::ptr::write_volatile(&mut regs[i], i as u64);
        }
    }

    // Restore
    let mut sum = 0u64;
    for i in 0..16 {
        sum += unsafe { core::ptr::read_volatile(&regs[i]) };
    }
    for i in 0..512 {
        sum += unsafe { core::ptr::read_volatile(&fpu_state[i]) } as u64;
    }

    let end = timing::read_tsc();
    core::hint::black_box(sum);
    end - start
}

// =============================================================================
// Thread Lifecycle Benchmarks
// =============================================================================

/// Thread creation overhead
fn bench_thread_create() -> u64 {
    let start = timing::read_tsc();

    // Simulate thread creation:
    // 1. Allocate stack
    let stack = [0u8; 4096];
    core::hint::black_box(&stack);

    // 2. Initialize thread control block
    let mut tcb = ThreadControlBlock::default();
    tcb.id = 1;
    tcb.state = ThreadState::Ready;
    tcb.priority = 50;

    // 3. Setup initial context
    tcb.stack_ptr = stack.as_ptr() as u64 + 4096;
    tcb.instruction_ptr = dummy_thread_entry as u64;

    // 4. Add to scheduler queue (simulated)
    core::hint::black_box(&tcb);

    let end = timing::read_tsc();
    end - start
}

/// Thread destruction overhead
fn bench_thread_destroy() -> u64 {
    // Create a thread first
    let tcb = ThreadControlBlock {
        id: 1,
        state: ThreadState::Terminated,
        priority: 50,
        stack_ptr: 0,
        instruction_ptr: 0,
        flags: 0,
    };

    let start = timing::read_tsc();

    // Simulate destruction:
    // 1. Remove from scheduler
    core::hint::black_box(tcb.id);

    // 2. Release resources
    core::hint::black_box(tcb.stack_ptr);

    // 3. Cleanup TCB
    let _ = core::hint::black_box(tcb);

    let end = timing::read_tsc();
    end - start
}

/// Thread yield overhead
fn bench_thread_yield() -> u64 {
    let start = timing::read_tsc();

    // Simulate yield:
    // 1. Mark current thread as ready
    let state = ThreadState::Ready;
    core::hint::black_box(state);

    // 2. Add to run queue
    static QUEUE_POS: AtomicU32 = AtomicU32::new(0);
    QUEUE_POS.fetch_add(1, Ordering::SeqCst);

    // 3. Select next thread (simulated)
    let next_id = QUEUE_POS.load(Ordering::SeqCst);
    core::hint::black_box(next_id);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Priority Benchmarks
// =============================================================================

/// Priority set operation
fn bench_priority_set() -> u64 {
    static PRIORITY: AtomicU32 = AtomicU32::new(50);

    let start = timing::read_tsc();

    // Change priority
    let old = PRIORITY.swap(75, Ordering::SeqCst);

    // Simulate re-queue due to priority change
    if old != 75 {
        core::hint::spin_loop();
    }

    let end = timing::read_tsc();
    core::hint::black_box(old);
    end - start
}

/// Priority boost (interactive task detection)
fn bench_priority_boost() -> u64 {
    let start = timing::read_tsc();

    // Simulate interactive detection
    let wait_time = 1000u64;
    let threshold = 500u64;

    let boost = if wait_time > threshold {
        // Calculate boost amount
        let boost = (wait_time - threshold).min(20);
        core::hint::black_box(boost);
        boost
    } else {
        0
    };

    // Apply boost
    static PRIORITY: AtomicU32 = AtomicU32::new(50);
    if boost > 0 {
        PRIORITY.fetch_add(boost as u32, Ordering::SeqCst);
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Queue Operation Benchmarks
// =============================================================================

/// Enqueue operation
fn bench_queue_enqueue() -> u64 {
    static QUEUE: spin::RwLock<[u64; 256]> = spin::RwLock::new([0; 256]);
    static HEAD: AtomicU32 = AtomicU32::new(0);

    let task_id = 42u64;

    let start = timing::read_tsc();

    let pos = HEAD.fetch_add(1, Ordering::SeqCst) as usize % 256;
    {
        let mut queue = QUEUE.write();
        queue[pos] = task_id;
    }

    let end = timing::read_tsc();
    end - start
}

/// Dequeue operation
fn bench_queue_dequeue() -> u64 {
    static QUEUE: spin::RwLock<[u64; 256]> = spin::RwLock::new([0; 256]);
    static TAIL: AtomicU32 = AtomicU32::new(0);

    // Pre-populate
    {
        let mut queue = QUEUE.write();
        for i in 0..256 {
            queue[i] = i as u64;
        }
    }

    let start = timing::read_tsc();

    let pos = TAIL.fetch_add(1, Ordering::SeqCst) as usize % 256;
    let task_id = {
        let queue = QUEUE.read();
        queue[pos]
    };

    let end = timing::read_tsc();
    core::hint::black_box(task_id);
    end - start
}

/// Requeue operation (task moved between queues)
fn bench_queue_requeue() -> u64 {
    static SRC_QUEUE: spin::RwLock<[u64; 64]> = spin::RwLock::new([0; 64]);
    static DST_QUEUE: spin::RwLock<[u64; 64]> = spin::RwLock::new([0; 64]);

    // Pre-populate source
    {
        let mut src = SRC_QUEUE.write();
        for i in 0..64 {
            src[i] = i as u64;
        }
    }

    let start = timing::read_tsc();

    // Dequeue from source
    let task_id = {
        let src = SRC_QUEUE.read();
        src[0]
    };

    // Enqueue to destination
    {
        let mut dst = DST_QUEUE.write();
        dst[0] = task_id;
    }

    let end = timing::read_tsc();
    core::hint::black_box(task_id);
    end - start
}

// =============================================================================
// Preemption Benchmarks
// =============================================================================

/// Preemption latency
fn bench_preemption_latency() -> u64 {
    let start = timing::read_tsc();

    // Simulate preemption check
    static NEED_RESCHED: AtomicU32 = AtomicU32::new(1);

    if NEED_RESCHED.load(Ordering::Acquire) != 0 {
        // Preemption needed
        NEED_RESCHED.store(0, Ordering::Release);

        // Save current state
        let mut state = [0u64; 16];
        for i in 0..16 {
            unsafe {
                core::ptr::write_volatile(&mut state[i], i as u64);
            }
        }

        // Select next task
        let next = 42u64;
        core::hint::black_box(next);

        // Load next state
        let mut sum = 0u64;
        for i in 0..16 {
            sum += unsafe { core::ptr::read_volatile(&state[i]) };
        }
        core::hint::black_box(sum);
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    NEED_RESCHED.store(1, Ordering::Release);

    end - start
}

// =============================================================================
// Tick Handler Benchmarks
// =============================================================================

/// Timer tick handler overhead
fn bench_tick_handler() -> u64 {
    let start = timing::read_tsc();

    // Simulate tick handler:

    // 1. Update tick count
    static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
    TICK_COUNT.fetch_add(1, Ordering::SeqCst);

    // 2. Update current task runtime
    static TASK_RUNTIME: AtomicU64 = AtomicU64::new(0);
    TASK_RUNTIME.fetch_add(1000000, Ordering::SeqCst); // 1ms in ns

    // 3. Check time slice expiry
    let runtime = TASK_RUNTIME.load(Ordering::SeqCst);
    let time_slice = 10_000_000u64; // 10ms

    let need_resched = runtime >= time_slice;
    core::hint::black_box(need_resched);

    // 4. Check for sleeping tasks to wake
    static SLEEPING_TASKS: AtomicU32 = AtomicU32::new(5);
    let sleeping = SLEEPING_TASKS.load(Ordering::SeqCst);
    core::hint::black_box(sleeping);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// DIS-Specific Benchmarks
// =============================================================================

/// Intent evaluation overhead
fn bench_intent_evaluation() -> u64 {
    let start = timing::read_tsc();

    // Simulate intent parsing and evaluation
    let intent = Intent {
        class: IntentClass::Interactive,
        latency_target: 1000, // 1ms
        cpu_budget: 50,
        memory_budget: 1024 * 1024,
        flags: 0,
    };

    // Evaluate intent
    let priority = match intent.class {
        IntentClass::RealTime => 99,
        IntentClass::Interactive => 70 + (intent.latency_target / 100).min(20) as i32,
        IntentClass::Batch => 30,
        IntentClass::Background => 10,
    };

    core::hint::black_box(priority);

    let end = timing::read_tsc();
    end - start
}

/// Policy evaluation overhead
fn bench_policy_evaluation() -> u64 {
    let start = timing::read_tsc();

    // Simulate policy evaluation
    let context = PolicyContext {
        cpu_load: 50,
        memory_usage: 40,
        task_wait_time: 5000,
        task_priority: 50,
        power_mode: 0,
    };

    // Apply policies
    let mut adjustments = 0i32;

    // Policy 1: Boost if waiting too long
    if context.task_wait_time > 10000 {
        adjustments += 10;
    }

    // Policy 2: Reduce if high load
    if context.cpu_load > 80 {
        adjustments -= 5;
    }

    // Policy 3: Power saving
    if context.power_mode == 1 && context.task_priority < 50 {
        adjustments -= 10;
    }

    core::hint::black_box(adjustments);

    let end = timing::read_tsc();
    end - start
}

/// Optimization hint generation
fn bench_optimization_hint() -> u64 {
    let start = timing::read_tsc();

    // Simulate ML-based optimization
    let stats = TaskStats {
        avg_runtime: 5000,
        avg_wait_time: 2000,
        context_switches: 100,
        burst_score: 60,
        interactive_score: 80,
    };

    // Generate hint
    let hint = if stats.interactive_score > 70 {
        OptimizationHint::Boost(10)
    } else if stats.burst_score > 80 {
        OptimizationHint::IncreaseTimeSlice(2)
    } else if stats.avg_wait_time > 10000 {
        OptimizationHint::Boost(5)
    } else {
        OptimizationHint::None
    };

    core::hint::black_box(hint);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Helper Types
// =============================================================================

#[derive(Default, Clone, Copy)]
struct ThreadControlBlock {
    id: u64,
    state: ThreadState,
    priority: i32,
    stack_ptr: u64,
    instruction_ptr: u64,
    flags: u32,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum ThreadState {
    #[default]
    New,
    Ready,
    Running,
    Blocked,
    Terminated,
}

fn dummy_thread_entry() {}

#[derive(Clone, Copy)]
struct Intent {
    class: IntentClass,
    latency_target: u64,
    cpu_budget: u32,
    memory_budget: u64,
    flags: u32,
}

#[derive(Clone, Copy)]
enum IntentClass {
    RealTime,
    Interactive,
    Batch,
    Background,
}

struct PolicyContext {
    cpu_load: u8,
    memory_usage: u8,
    task_wait_time: u64,
    task_priority: i32,
    power_mode: u8,
}

struct TaskStats {
    avg_runtime: u64,
    avg_wait_time: u64,
    context_switches: u64,
    burst_score: u8,
    interactive_score: u8,
}

#[derive(Clone, Copy)]
enum OptimizationHint {
    None,
    Boost(i32),
    IncreaseTimeSlice(u32),
    DecreaseTimeSlice(u32),
}
