//! IPC (Inter-Process Communication) Benchmarks
//!
//! Tests for measuring IPC performance:
//! - Message passing latency
//! - Channel throughput
//! - Syscall overhead
//! - Shared memory operations

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::RwLock;

use crate::{benchmark, timing, BenchmarkCategory, BenchmarkDef, BenchmarkId, BenchmarkSuite};

// =============================================================================
// Benchmark Registration
// =============================================================================

/// Register all IPC benchmarks
pub fn register_benchmarks(suite: &BenchmarkSuite) {
    // Message passing
    suite.register(benchmark!(
        "ipc.message.send_small",
        BenchmarkCategory::Ipc,
        bench_send_small_message
    ));

    suite.register(benchmark!(
        "ipc.message.send_medium",
        BenchmarkCategory::Ipc,
        bench_send_medium_message
    ));

    suite.register(benchmark!(
        "ipc.message.send_large",
        BenchmarkCategory::Ipc,
        bench_send_large_message
    ));

    suite.register(benchmark!(
        "ipc.message.receive",
        BenchmarkCategory::Ipc,
        bench_receive_message
    ));

    // Channel operations
    suite.register(benchmark!(
        "ipc.channel.create",
        BenchmarkCategory::Ipc,
        bench_channel_create
    ));

    suite.register(benchmark!(
        "ipc.channel.destroy",
        BenchmarkCategory::Ipc,
        bench_channel_destroy
    ));

    suite.register(benchmark!(
        "ipc.channel.send",
        BenchmarkCategory::Ipc,
        bench_channel_send
    ));

    suite.register(benchmark!(
        "ipc.channel.recv",
        BenchmarkCategory::Ipc,
        bench_channel_recv
    ));

    // Syscall overhead
    suite.register(benchmark!(
        "ipc.syscall.null",
        BenchmarkCategory::Ipc,
        bench_null_syscall
    ));

    suite.register(benchmark!(
        "ipc.syscall.getpid",
        BenchmarkCategory::Ipc,
        bench_getpid_syscall
    ));

    suite.register(benchmark!(
        "ipc.syscall.write",
        BenchmarkCategory::Ipc,
        bench_write_syscall
    ));

    suite.register(benchmark!(
        "ipc.syscall.read",
        BenchmarkCategory::Ipc,
        bench_read_syscall
    ));

    // Shared memory
    suite.register(benchmark!(
        "ipc.shm.create",
        BenchmarkCategory::Ipc,
        bench_shm_create
    ));

    suite.register(benchmark!(
        "ipc.shm.map",
        BenchmarkCategory::Ipc,
        bench_shm_map
    ));

    suite.register(benchmark!(
        "ipc.shm.unmap",
        BenchmarkCategory::Ipc,
        bench_shm_unmap
    ));

    // Notifications
    suite.register(benchmark!(
        "ipc.notify.send",
        BenchmarkCategory::Ipc,
        bench_notification_send
    ));

    suite.register(benchmark!(
        "ipc.notify.wait",
        BenchmarkCategory::Ipc,
        bench_notification_wait
    ));

    // Event bus
    suite.register(benchmark!(
        "ipc.event.publish",
        BenchmarkCategory::Ipc,
        bench_event_publish
    ));

    suite.register(benchmark!(
        "ipc.event.subscribe",
        BenchmarkCategory::Ipc,
        bench_event_subscribe
    ));

    // Synchronization primitives
    suite.register(benchmark!(
        "ipc.sync.mutex_lock",
        BenchmarkCategory::Ipc,
        bench_mutex_lock
    ));

    suite.register(benchmark!(
        "ipc.sync.mutex_unlock",
        BenchmarkCategory::Ipc,
        bench_mutex_unlock
    ));

    suite.register(benchmark!(
        "ipc.sync.semaphore_wait",
        BenchmarkCategory::Ipc,
        bench_semaphore_wait
    ));

    suite.register(benchmark!(
        "ipc.sync.semaphore_signal",
        BenchmarkCategory::Ipc,
        bench_semaphore_signal
    ));
}

// =============================================================================
// Message Passing Benchmarks
// =============================================================================

/// Send small message (64 bytes)
fn bench_send_small_message() -> u64 {
    let start = timing::read_tsc();

    // Create message header
    let msg = Message {
        sender: 1,
        receiver: 2,
        msg_type: MsgType::Data,
        flags: 0,
        payload_len: 64,
    };

    // Copy payload
    let payload = [0u8; 64];
    core::hint::black_box(&payload);

    // Lookup destination
    static PROCESS_TABLE: RwLock<[u64; 16]> = RwLock::new([0; 16]);
    let dest = PROCESS_TABLE.read()[msg.receiver as usize];
    core::hint::black_box(dest);

    // Add to destination queue
    static MSG_QUEUE: AtomicU32 = AtomicU32::new(0);
    MSG_QUEUE.fetch_add(1, Ordering::SeqCst);

    // Wake destination if blocked
    static BLOCKED_PROCS: AtomicU64 = AtomicU64::new(0);
    let blocked = BLOCKED_PROCS.load(Ordering::Acquire);
    if (blocked >> msg.receiver) & 1 == 1 {
        // Wake up
        BLOCKED_PROCS.fetch_and(!(1 << msg.receiver), Ordering::SeqCst);
    }

    core::hint::black_box(msg);

    let end = timing::read_tsc();
    end - start
}

/// Send medium message (1KB)
fn bench_send_medium_message() -> u64 {
    let start = timing::read_tsc();

    let msg = Message {
        sender: 1,
        receiver: 2,
        msg_type: MsgType::Data,
        flags: 0,
        payload_len: 1024,
    };

    // Copy 1KB payload
    let payload = [0u8; 1024];
    let mut sum = 0u64;
    for chunk in payload.chunks(64) {
        for &b in chunk {
            sum += b as u64;
        }
    }
    core::hint::black_box(sum);

    // Queue message
    static MSG_QUEUE: AtomicU32 = AtomicU32::new(0);
    MSG_QUEUE.fetch_add(1, Ordering::SeqCst);

    core::hint::black_box(msg);

    let end = timing::read_tsc();
    end - start
}

/// Send large message (64KB, zero-copy)
fn bench_send_large_message() -> u64 {
    let start = timing::read_tsc();

    let msg = Message {
        sender: 1,
        receiver: 2,
        msg_type: MsgType::LargeData,
        flags: MSG_FLAG_ZERO_COPY,
        payload_len: 65536,
    };

    // Zero-copy: just share page mappings
    static PAGE_TABLE: RwLock<[u64; 16]> = RwLock::new([0; 16]);

    // Map sender pages to receiver
    let pages_needed = (msg.payload_len + 4095) / 4096;
    for i in 0..pages_needed {
        let page_idx = (i % 16) as usize;
        let mut pt = PAGE_TABLE.write();
        pt[page_idx] = 0x1000_0000 + (i * 4096) as u64;
    }

    // Queue message header only
    static MSG_QUEUE: AtomicU32 = AtomicU32::new(0);
    MSG_QUEUE.fetch_add(1, Ordering::SeqCst);

    core::hint::black_box(msg);

    let end = timing::read_tsc();
    end - start
}

/// Receive message
fn bench_receive_message() -> u64 {
    // Setup: pre-queue a message
    static MSG_QUEUE: AtomicU32 = AtomicU32::new(1);

    let start = timing::read_tsc();

    // Check queue
    let msg_count = MSG_QUEUE.load(Ordering::Acquire);

    if msg_count > 0 {
        // Dequeue message
        MSG_QUEUE.fetch_sub(1, Ordering::SeqCst);

        // Read message header
        let msg = Message {
            sender: 1,
            receiver: 2,
            msg_type: MsgType::Data,
            flags: 0,
            payload_len: 64,
        };

        // Copy payload
        let mut buffer = [0u8; 64];
        for i in 0..64 {
            buffer[i] = i as u8;
        }

        core::hint::black_box((msg, buffer));
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    MSG_QUEUE.store(1, Ordering::Release);

    end - start
}

// =============================================================================
// Channel Benchmarks
// =============================================================================

/// Create channel
fn bench_channel_create() -> u64 {
    let start = timing::read_tsc();

    // Allocate channel ID
    static NEXT_CHANNEL_ID: AtomicU64 = AtomicU64::new(1);
    let channel_id = NEXT_CHANNEL_ID.fetch_add(1, Ordering::SeqCst);

    // Create channel structure
    let channel = Channel {
        id: channel_id,
        sender: 0,
        receiver: 0,
        capacity: 16,
        count: AtomicU32::new(0),
        flags: 0,
    };

    // Register in channel table
    static CHANNEL_TABLE: RwLock<[u64; 64]> = RwLock::new([0; 64]);
    {
        let mut table = CHANNEL_TABLE.write();
        let idx = (channel_id % 64) as usize;
        table[idx] = channel_id;
    }

    core::hint::black_box(channel);

    let end = timing::read_tsc();
    end - start
}

/// Destroy channel
fn bench_channel_destroy() -> u64 {
    let start = timing::read_tsc();

    let channel_id = 42u64;

    // Lookup channel
    static CHANNEL_TABLE: RwLock<[u64; 64]> = RwLock::new([0; 64]);

    // Remove from table
    {
        let mut table = CHANNEL_TABLE.write();
        let idx = (channel_id % 64) as usize;
        table[idx] = 0;
    }

    // Wake any blocked waiters
    static BLOCKED_ON_CHANNEL: AtomicU64 = AtomicU64::new(0);
    BLOCKED_ON_CHANNEL.store(0, Ordering::SeqCst);

    // Free channel resources
    core::hint::black_box(channel_id);

    let end = timing::read_tsc();
    end - start
}

/// Channel send
fn bench_channel_send() -> u64 {
    let start = timing::read_tsc();

    // Check channel capacity
    static CHANNEL_COUNT: AtomicU32 = AtomicU32::new(0);
    let count = CHANNEL_COUNT.load(Ordering::Acquire);

    if count < 16 {
        // Add to buffer
        CHANNEL_COUNT.fetch_add(1, Ordering::SeqCst);

        // Store data
        static CHANNEL_BUFFER: RwLock<[u64; 16]> = RwLock::new([0; 16]);
        {
            let mut buffer = CHANNEL_BUFFER.write();
            buffer[count as usize] = 0xDEADBEEF;
        }

        // Wake receiver if waiting
        static RECV_WAITING: AtomicU32 = AtomicU32::new(0);
        if RECV_WAITING.load(Ordering::Acquire) > 0 {
            RECV_WAITING.fetch_sub(1, Ordering::SeqCst);
            // Would wake receiver
        }
    }

    let end = timing::read_tsc();
    end - start
}

/// Channel receive
fn bench_channel_recv() -> u64 {
    // Setup: pre-populate
    static CHANNEL_COUNT: AtomicU32 = AtomicU32::new(1);
    static CHANNEL_BUFFER: RwLock<[u64; 16]> = RwLock::new([0xDEADBEEF; 16]);

    let start = timing::read_tsc();

    let count = CHANNEL_COUNT.load(Ordering::Acquire);

    if count > 0 {
        // Read data
        let data = {
            let buffer = CHANNEL_BUFFER.read();
            buffer[0]
        };

        // Update count
        CHANNEL_COUNT.fetch_sub(1, Ordering::SeqCst);

        // Wake sender if waiting
        static SEND_WAITING: AtomicU32 = AtomicU32::new(0);
        if SEND_WAITING.load(Ordering::Acquire) > 0 {
            SEND_WAITING.fetch_sub(1, Ordering::SeqCst);
        }

        core::hint::black_box(data);
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    CHANNEL_COUNT.store(1, Ordering::Release);

    end - start
}

// =============================================================================
// Syscall Benchmarks
// =============================================================================

/// Null syscall (syscall overhead only)
fn bench_null_syscall() -> u64 {
    let start = timing::read_tsc();

    // Simulate syscall entry
    // 1. Mode switch (user -> kernel)
    static MODE: AtomicU32 = AtomicU32::new(0);
    MODE.store(1, Ordering::SeqCst); // Kernel mode

    // 2. Save user registers
    let mut user_regs = [0u64; 6]; // RDI, RSI, RDX, R10, R8, R9
    for i in 0..6 {
        unsafe {
            core::ptr::write_volatile(&mut user_regs[i], i as u64);
        }
    }

    // 3. Syscall dispatch (null syscall = do nothing)
    let syscall_num = 0u64;
    core::hint::black_box(syscall_num);

    // 4. Prepare return value
    let result = 0u64;

    // 5. Restore and return
    let mut sum = 0u64;
    for i in 0..6 {
        sum += unsafe { core::ptr::read_volatile(&user_regs[i]) };
    }

    // 6. Mode switch (kernel -> user)
    MODE.store(0, Ordering::SeqCst);

    let end = timing::read_tsc();
    core::hint::black_box((result, sum));
    end - start
}

/// getpid syscall
fn bench_getpid_syscall() -> u64 {
    let start = timing::read_tsc();

    // Mode switch
    // Get current task
    static CURRENT_TASK: AtomicU64 = AtomicU64::new(42);
    let pid = CURRENT_TASK.load(Ordering::Acquire);

    // Return PID
    core::hint::black_box(pid);

    let end = timing::read_tsc();
    end - start
}

/// write syscall
fn bench_write_syscall() -> u64 {
    let start = timing::read_tsc();

    let fd = 1u64; // stdout
    let buf = [b'H', b'e', b'l', b'l', b'o'];
    let count = buf.len();

    // Validate fd
    static FD_TABLE: RwLock<[u64; 16]> = RwLock::new([1; 16]);
    let file = {
        let table = FD_TABLE.read();
        table[fd as usize]
    };

    if file != 0 {
        // Check permissions
        let can_write = true;

        if can_write {
            // Copy from user space
            let mut kernel_buf = [0u8; 16];
            kernel_buf[..count].copy_from_slice(&buf);

            // Write to device
            static BYTES_WRITTEN: AtomicU64 = AtomicU64::new(0);
            BYTES_WRITTEN.fetch_add(count as u64, Ordering::SeqCst);

            core::hint::black_box(&kernel_buf);
        }
    }

    let end = timing::read_tsc();
    end - start
}

/// read syscall
fn bench_read_syscall() -> u64 {
    let start = timing::read_tsc();

    let fd = 0u64; // stdin
    let count = 64usize;

    // Validate fd
    static FD_TABLE: RwLock<[u64; 16]> = RwLock::new([1; 16]);
    let file = {
        let table = FD_TABLE.read();
        table[fd as usize]
    };

    if file != 0 {
        // Check if data available
        static DATA_AVAILABLE: AtomicU32 = AtomicU32::new(64);
        let available = DATA_AVAILABLE.load(Ordering::Acquire);

        let to_read = core::cmp::min(count, available as usize);

        // Read from device
        let mut kernel_buf = [0u8; 64];
        for i in 0..to_read {
            kernel_buf[i] = i as u8;
        }

        // Copy to user space
        core::hint::black_box(&kernel_buf[..to_read]);
    }

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Shared Memory Benchmarks
// =============================================================================

/// Create shared memory region
fn bench_shm_create() -> u64 {
    let start = timing::read_tsc();

    // Allocate SHM ID
    static NEXT_SHM_ID: AtomicU64 = AtomicU64::new(1);
    let shm_id = NEXT_SHM_ID.fetch_add(1, Ordering::SeqCst);

    let size = 4096 * 16; // 64KB

    // Allocate backing pages
    static PAGE_ALLOC: AtomicU64 = AtomicU64::new(0x8000_0000);
    let pages = (size + 4095) / 4096;
    let base = PAGE_ALLOC.fetch_add((pages * 4096) as u64, Ordering::SeqCst);

    // Create SHM descriptor
    let shm = SharedMemory {
        id: shm_id,
        base,
        size: size as u64,
        ref_count: AtomicU32::new(1),
        flags: 0,
    };

    // Register in SHM table
    static SHM_TABLE: RwLock<[u64; 64]> = RwLock::new([0; 64]);
    {
        let mut table = SHM_TABLE.write();
        table[(shm_id % 64) as usize] = shm_id;
    }

    core::hint::black_box(shm);

    let end = timing::read_tsc();
    end - start
}

/// Map shared memory
fn bench_shm_map() -> u64 {
    let start = timing::read_tsc();

    let shm_id = 1u64;
    let target_addr = 0x7000_0000u64;

    // Lookup SHM
    static SHM_BASE: AtomicU64 = AtomicU64::new(0x8000_0000);
    let base = SHM_BASE.load(Ordering::Acquire);
    let size = 65536u64;

    // Map pages into process address space
    let pages = size / 4096;
    for i in 0..pages {
        let phys = base + i * 4096;
        let virt = target_addr + i * 4096;

        // Would update page tables
        core::hint::black_box((phys, virt));
    }

    // Increment ref count
    static REF_COUNT: AtomicU32 = AtomicU32::new(1);
    REF_COUNT.fetch_add(1, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

/// Unmap shared memory
fn bench_shm_unmap() -> u64 {
    let start = timing::read_tsc();

    let target_addr = 0x7000_0000u64;
    let size = 65536u64;

    // Unmap pages
    let pages = size / 4096;
    for i in 0..pages {
        let virt = target_addr + i * 4096;

        // Would clear page table entries
        core::hint::black_box(virt);
    }

    // TLB flush
    core::hint::spin_loop();

    // Decrement ref count
    static REF_COUNT: AtomicU32 = AtomicU32::new(2);
    REF_COUNT.fetch_sub(1, Ordering::SeqCst);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Notification Benchmarks
// =============================================================================

/// Send notification
fn bench_notification_send() -> u64 {
    let start = timing::read_tsc();

    let target_pid = 2u64;
    let notification = 0x0001u64; // Signal

    // Set notification bit
    static NOTIFICATIONS: [AtomicU64; 16] = {
        const INIT: AtomicU64 = AtomicU64::new(0);
        [INIT; 16]
    };

    NOTIFICATIONS[(target_pid % 16) as usize].fetch_or(notification, Ordering::SeqCst);

    // Wake if waiting
    static WAIT_MASK: AtomicU64 = AtomicU64::new(0);
    let waiting = WAIT_MASK.load(Ordering::Acquire);
    if (waiting >> target_pid) & 1 == 1 {
        // Would wake process
        core::hint::black_box(target_pid);
    }

    let end = timing::read_tsc();
    end - start
}

/// Wait for notification
fn bench_notification_wait() -> u64 {
    // Pre-set notification
    static NOTIFICATIONS: [AtomicU64; 16] = {
        const INIT: AtomicU64 = AtomicU64::new(1);
        [INIT; 16]
    };

    let start = timing::read_tsc();

    let my_pid = 2u64;
    let wait_mask = 0xFFFFu64;

    // Check for pending notifications
    let pending = NOTIFICATIONS[(my_pid % 16) as usize].load(Ordering::Acquire);
    let matched = pending & wait_mask;

    if matched != 0 {
        // Clear handled notifications
        NOTIFICATIONS[(my_pid % 16) as usize].fetch_and(!matched, Ordering::SeqCst);

        core::hint::black_box(matched);
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    NOTIFICATIONS[(my_pid % 16) as usize].store(1, Ordering::Release);

    end - start
}

// =============================================================================
// Event Bus Benchmarks
// =============================================================================

/// Publish event
fn bench_event_publish() -> u64 {
    let start = timing::read_tsc();

    let event_type = 1u64;
    let event_data = 0xCAFEBABEu64;

    // Lookup subscribers
    static SUBSCRIBER_MASK: AtomicU64 = AtomicU64::new(0b1111);
    let subscribers = SUBSCRIBER_MASK.load(Ordering::Acquire);

    // Notify each subscriber
    let mut notified = 0u32;
    for i in 0..64 {
        if (subscribers >> i) & 1 == 1 {
            // Queue event for subscriber
            notified += 1;
        }
    }

    core::hint::black_box((event_type, event_data, notified));

    let end = timing::read_tsc();
    end - start
}

/// Subscribe to events
fn bench_event_subscribe() -> u64 {
    let start = timing::read_tsc();

    let my_pid = 3u64;
    let event_mask = 0xFFu64;

    // Register subscription
    static SUBSCRIBER_MASK: AtomicU64 = AtomicU64::new(0);
    SUBSCRIBER_MASK.fetch_or(1 << my_pid, Ordering::SeqCst);

    // Store event mask
    static EVENT_MASKS: [AtomicU64; 16] = {
        const INIT: AtomicU64 = AtomicU64::new(0);
        [INIT; 16]
    };
    EVENT_MASKS[(my_pid % 16) as usize].store(event_mask, Ordering::Release);

    let end = timing::read_tsc();
    end - start
}

// =============================================================================
// Synchronization Benchmarks
// =============================================================================

/// Mutex lock (uncontended)
fn bench_mutex_lock() -> u64 {
    static MUTEX: AtomicU32 = AtomicU32::new(0); // Unlocked

    let start = timing::read_tsc();

    // Try to acquire
    let result = MUTEX.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed);

    core::hint::black_box(result);

    let end = timing::read_tsc();

    // Reset for next iteration
    MUTEX.store(0, Ordering::Release);

    end - start
}

/// Mutex unlock
fn bench_mutex_unlock() -> u64 {
    static MUTEX: AtomicU32 = AtomicU32::new(1); // Locked

    let start = timing::read_tsc();

    // Release
    MUTEX.store(0, Ordering::Release);

    // Check for waiters
    static WAITERS: AtomicU32 = AtomicU32::new(0);
    let waiters = WAITERS.load(Ordering::Acquire);
    if waiters > 0 {
        // Would wake one waiter
        core::hint::black_box(waiters);
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    MUTEX.store(1, Ordering::Release);

    end - start
}

/// Semaphore wait (has permits)
fn bench_semaphore_wait() -> u64 {
    static SEMAPHORE: AtomicU32 = AtomicU32::new(5);

    let start = timing::read_tsc();

    loop {
        let current = SEMAPHORE.load(Ordering::Acquire);
        if current > 0 {
            if SEMAPHORE
                .compare_exchange(current, current - 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        } else {
            // Would block
            break;
        }
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    SEMAPHORE.fetch_add(1, Ordering::Release);

    end - start
}

/// Semaphore signal
fn bench_semaphore_signal() -> u64 {
    static SEMAPHORE: AtomicU32 = AtomicU32::new(0);

    let start = timing::read_tsc();

    let old = SEMAPHORE.fetch_add(1, Ordering::Release);

    // Check for waiters if was zero
    if old == 0 {
        static WAITERS: AtomicU32 = AtomicU32::new(0);
        let waiters = WAITERS.load(Ordering::Acquire);
        if waiters > 0 {
            // Would wake one waiter
            core::hint::black_box(waiters);
        }
    }

    let end = timing::read_tsc();

    // Reset for next iteration
    SEMAPHORE.store(0, Ordering::Release);

    end - start
}

// =============================================================================
// Helper Types
// =============================================================================

#[derive(Clone, Copy)]
struct Message {
    sender: u64,
    receiver: u64,
    msg_type: MsgType,
    flags: u32,
    payload_len: u32,
}

#[derive(Clone, Copy)]
enum MsgType {
    Data,
    LargeData,
    Control,
}

const MSG_FLAG_ZERO_COPY: u32 = 1;

struct Channel {
    id: u64,
    sender: u64,
    receiver: u64,
    capacity: u32,
    count: AtomicU32,
    flags: u32,
}

struct SharedMemory {
    id: u64,
    base: u64,
    size: u64,
    ref_count: AtomicU32,
    flags: u32,
}
