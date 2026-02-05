//! # Round-Robin Scheduler Implementation

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use helix_execution::scheduler::queue::FifoQueue;
use helix_execution::scheduler::traits::RunQueue;
use helix_execution::scheduler::{
    Priority, SchedulableThread, Scheduler, SchedulerStats, SchedulingPolicy,
};
use helix_execution::{ExecError, ExecResult, ThreadId};
use spin::{Mutex, RwLock};

use crate::RoundRobinConfig;

/// Thread entry in the scheduler
struct ThreadEntry {
    /// Thread info
    info: SchedulableThread,
    /// Remaining time slice (nanoseconds)
    remaining_slice: AtomicU64,
    /// Total time slice for this thread
    time_slice: u64,
    /// State
    state: ThreadState,
}

/// Thread state within the scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadState {
    Runnable,
    Running,
    Blocked,
}

/// Per-CPU run queue
struct PerCpuQueue {
    /// Run queue (FIFO within priority level)
    queue: Mutex<FifoQueue>,
    /// Currently running thread
    current: RwLock<Option<ThreadId>>,
    /// Need reschedule flag
    need_reschedule: AtomicBool,
    /// Idle count
    idle_ticks: AtomicU64,
}

impl PerCpuQueue {
    fn new() -> Self {
        Self {
            queue: Mutex::new(FifoQueue::new()),
            current: RwLock::new(None),
            need_reschedule: AtomicBool::new(false),
            idle_ticks: AtomicU64::new(0),
        }
    }
}

/// Round-Robin Scheduler
pub struct RoundRobinScheduler {
    /// Configuration
    config: RoundRobinConfig,
    /// All threads
    threads: RwLock<BTreeMap<ThreadId, ThreadEntry>>,
    /// Per-CPU queues
    cpu_queues: RwLock<Vec<PerCpuQueue>>,
    /// Number of CPUs
    cpu_count: RwLock<usize>,
    /// Statistics
    context_switches: AtomicU64,
    /// Tick counter for load balancing
    tick_counter: AtomicU64,
}

impl RoundRobinScheduler {
    /// Create a new scheduler
    pub fn new(config: RoundRobinConfig) -> Self {
        Self {
            config,
            threads: RwLock::new(BTreeMap::new()),
            cpu_queues: RwLock::new(Vec::new()),
            cpu_count: RwLock::new(0),
            context_switches: AtomicU64::new(0),
            tick_counter: AtomicU64::new(0),
        }
    }

    /// Get time slice for a thread
    fn time_slice_for(&self, thread: &SchedulableThread) -> u64 {
        self.config
            .time_slice_for_priority(thread.priority.static_priority())
    }

    /// Find the least loaded CPU
    fn find_least_loaded_cpu(&self) -> usize {
        let threads = self.threads.read();
        let cpu_count = *self.cpu_count.read();

        if cpu_count <= 1 {
            return 0;
        }

        let mut load = vec![0usize; cpu_count];

        for (id, entry) in threads.iter() {
            if entry.state == ThreadState::Runnable || entry.state == ThreadState::Running {
                // Check affinity and count
                for cpu in 0..cpu_count {
                    if entry.info.affinity & (1 << cpu) != 0 {
                        load[cpu] += 1;
                    }
                }
            }
        }

        load.iter()
            .enumerate()
            .min_by_key(|(_, &l)| l)
            .map(|(cpu, _)| cpu)
            .unwrap_or(0)
    }
}

impl Scheduler for RoundRobinScheduler {
    fn name(&self) -> &'static str {
        "Round-Robin"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn init(&mut self, cpu_count: usize) -> ExecResult<()> {
        log::info!("Initializing Round-Robin scheduler for {} CPUs", cpu_count);

        let mut queues = self.cpu_queues.write();
        queues.clear();
        for _ in 0..cpu_count {
            queues.push(PerCpuQueue::new());
        }

        *self.cpu_count.write() = cpu_count;
        Ok(())
    }

    fn pick_next(&self, cpu: usize) -> Option<ThreadId> {
        let cpu_queues = self.cpu_queues.read();
        let queue = cpu_queues.get(cpu)?;

        // Dequeue from the run queue
        let next = queue.queue.lock().dequeue();

        if let Some(id) = next {
            // Update current
            *queue.current.write() = Some(id);
            queue.need_reschedule.store(false, Ordering::SeqCst);

            // Update thread state
            if let Some(entry) = self.threads.write().get_mut(&id) {
                entry.state = ThreadState::Running;
                // Reset time slice
                entry
                    .remaining_slice
                    .store(entry.time_slice, Ordering::SeqCst);
            }

            self.context_switches.fetch_add(1, Ordering::Relaxed);
        } else {
            queue.idle_ticks.fetch_add(1, Ordering::Relaxed);
        }

        next
    }

    fn add_thread(&self, thread: SchedulableThread) -> ExecResult<()> {
        let id = thread.id;
        let time_slice = self.time_slice_for(&thread);

        log::debug!("Adding thread {:?} with time slice {}ns", id, time_slice);

        let entry = ThreadEntry {
            info: thread,
            remaining_slice: AtomicU64::new(time_slice),
            time_slice,
            state: ThreadState::Runnable,
        };

        self.threads.write().insert(id, entry);

        // Add to run queue
        self.thread_ready(id)?;

        Ok(())
    }

    fn remove_thread(&self, id: ThreadId) -> ExecResult<()> {
        log::debug!("Removing thread {:?}", id);

        // Remove from threads
        self.threads
            .write()
            .remove(&id)
            .ok_or(ExecError::ThreadNotFound)?;

        // Remove from all run queues
        let cpu_queues = self.cpu_queues.read();
        for queue in cpu_queues.iter() {
            queue.queue.lock().remove(id);
        }

        Ok(())
    }

    fn thread_ready(&self, id: ThreadId) -> ExecResult<()> {
        let mut threads = self.threads.write();
        let entry = threads.get_mut(&id).ok_or(ExecError::ThreadNotFound)?;

        if entry.state == ThreadState::Runnable {
            return Ok(()); // Already ready
        }

        entry.state = ThreadState::Runnable;
        let priority = entry.info.priority;
        let affinity = entry.info.affinity;
        drop(threads);

        // Find a suitable CPU
        let cpu = self.find_least_loaded_cpu();

        // Add to that CPU's queue
        let cpu_queues = self.cpu_queues.read();
        if let Some(queue) = cpu_queues.get(cpu) {
            queue.queue.lock().enqueue(id, priority);
        }

        Ok(())
    }

    fn thread_block(&self, id: ThreadId) -> ExecResult<()> {
        let mut threads = self.threads.write();
        let entry = threads.get_mut(&id).ok_or(ExecError::ThreadNotFound)?;

        entry.state = ThreadState::Blocked;
        drop(threads);

        // Remove from run queues
        let cpu_queues = self.cpu_queues.read();
        for queue in cpu_queues.iter() {
            queue.queue.lock().remove(id);

            // If this was the current thread, trigger reschedule
            if *queue.current.read() == Some(id) {
                *queue.current.write() = None;
                queue.need_reschedule.store(true, Ordering::SeqCst);
            }
        }

        Ok(())
    }

    fn yield_thread(&self, cpu: usize) {
        let cpu_queues = self.cpu_queues.read();
        if let Some(queue) = cpu_queues.get(cpu) {
            if let Some(current) = *queue.current.read() {
                // Get thread info
                let threads = self.threads.read();
                if let Some(entry) = threads.get(&current) {
                    let priority = entry.info.priority;
                    drop(threads);

                    // Re-enqueue at the back
                    queue.queue.lock().enqueue(current, priority);
                }

                queue.need_reschedule.store(true, Ordering::SeqCst);
            }
        }
    }

    fn tick(&self, cpu: usize) {
        let tick = self.tick_counter.fetch_add(1, Ordering::Relaxed);

        let cpu_queues = self.cpu_queues.read();
        if let Some(queue) = cpu_queues.get(cpu) {
            if let Some(current) = *queue.current.read() {
                // Decrement time slice
                let threads = self.threads.read();
                if let Some(entry) = threads.get(&current) {
                    let remaining = entry.remaining_slice.load(Ordering::Relaxed);
                    let tick_ns = 1_000_000; // Assume 1ms tick

                    if remaining <= tick_ns {
                        // Time slice expired
                        let priority = entry.info.priority;
                        drop(threads);

                        queue.queue.lock().enqueue(current, priority);
                        queue.need_reschedule.store(true, Ordering::SeqCst);
                    } else {
                        entry
                            .remaining_slice
                            .store(remaining - tick_ns, Ordering::Relaxed);
                    }
                }
            }
        }

        // Periodic load balancing
        if self.config.load_balancing && tick % self.config.load_balance_interval == 0 {
            // TODO: Implement load balancing
        }
    }

    fn set_priority(&self, id: ThreadId, priority: Priority) -> ExecResult<()> {
        let mut threads = self.threads.write();
        let entry = threads.get_mut(&id).ok_or(ExecError::ThreadNotFound)?;

        entry.info.priority = priority;
        entry.time_slice = self
            .config
            .time_slice_for_priority(priority.static_priority());

        Ok(())
    }

    fn get_priority(&self, id: ThreadId) -> Option<Priority> {
        self.threads.read().get(&id).map(|e| e.info.priority)
    }

    fn needs_reschedule(&self, cpu: usize) -> bool {
        self.cpu_queues
            .read()
            .get(cpu)
            .map(|q| q.need_reschedule.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    fn stats(&self) -> SchedulerStats {
        let threads = self.threads.read();
        let (runnable, blocked) = threads.values().fold((0, 0), |(r, b), e| match e.state {
            ThreadState::Runnable | ThreadState::Running => (r + 1, b),
            ThreadState::Blocked => (r, b + 1),
        });

        let cpu_count = *self.cpu_count.read();
        let cpu_load: Vec<u8> = self
            .cpu_queues
            .read()
            .iter()
            .map(|q| {
                let idle = q.idle_ticks.load(Ordering::Relaxed);
                let total = self.tick_counter.load(Ordering::Relaxed) / cpu_count as u64;
                if total == 0 {
                    0
                } else {
                    (100 - (idle * 100 / total).min(100)) as u8
                }
            })
            .collect();

        SchedulerStats {
            context_switches: self.context_switches.load(Ordering::Relaxed),
            runnable_threads: runnable,
            blocked_threads: blocked,
            avg_wait_time: 0, // TODO
            avg_run_time: 0,  // TODO
            cpu_load,
        }
    }

    fn set_policy(&self, id: ThreadId, policy: SchedulingPolicy) -> ExecResult<()> {
        // For round-robin, we mainly care about the priority in the policy
        match policy {
            SchedulingPolicy::Normal => {
                self.set_priority(id, Priority::normal(0))?;
            },
            SchedulingPolicy::Fifo | SchedulingPolicy::RoundRobin => {
                // Real-time scheduling
                self.set_priority(id, Priority::realtime(50))?;
            },
            SchedulingPolicy::Batch => {
                self.set_priority(id, Priority::normal(10))?;
            },
            SchedulingPolicy::Idle => {
                self.set_priority(id, Priority::IDLE)?;
            },
            SchedulingPolicy::Deadline { .. } => {
                // Not supported by this simple scheduler
                return Err(ExecError::InvalidArgument);
            },
        }
        Ok(())
    }

    fn migrate_thread(&self, id: ThreadId, target_cpu: usize) -> ExecResult<()> {
        let cpu_count = *self.cpu_count.read();
        if target_cpu >= cpu_count {
            return Err(ExecError::InvalidArgument);
        }

        let threads = self.threads.read();
        let entry = threads.get(&id).ok_or(ExecError::ThreadNotFound)?;
        let priority = entry.info.priority;
        let state = entry.state;
        drop(threads);

        if state != ThreadState::Runnable {
            return Ok(()); // Only migrate runnable threads
        }

        // Remove from all queues
        let cpu_queues = self.cpu_queues.read();
        for queue in cpu_queues.iter() {
            queue.queue.lock().remove(id);
        }

        // Add to target queue
        if let Some(queue) = cpu_queues.get(target_cpu) {
            queue.queue.lock().enqueue(id, priority);
        }

        log::debug!("Migrated thread {:?} to CPU {}", id, target_cpu);
        Ok(())
    }
}
