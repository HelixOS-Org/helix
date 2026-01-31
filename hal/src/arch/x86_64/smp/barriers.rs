//! # Synchronization Barriers
//!
//! This module provides synchronization primitives for multi-processor
//! coordination including barriers, spinlocks, and sequence locks.
//!
//! ## Barrier Types
//!
//! - `Barrier`: Simple counting barrier
//! - `SpinBarrier`: Spinning barrier with busy-wait
//! - `SeqLock`: Sequence lock for read-mostly data
//! - `ReaderWriterLock`: Read-write lock

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering, fence};
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::marker::PhantomData;

// =============================================================================
// Barrier
// =============================================================================

/// Counting barrier for CPU synchronization
pub struct Barrier {
    /// Number of CPUs expected at barrier
    count: AtomicU32,
    /// Current generation/phase
    generation: AtomicU32,
    /// Number of CPUs waiting
    waiting: AtomicU32,
}

impl Barrier {
    /// Create a new barrier for n CPUs
    pub const fn new(n: u32) -> Self {
        Self {
            count: AtomicU32::new(n),
            generation: AtomicU32::new(0),
            waiting: AtomicU32::new(0),
        }
    }

    /// Reset barrier count
    pub fn reset(&self, n: u32) {
        self.count.store(n, Ordering::SeqCst);
        self.waiting.store(0, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Wait at the barrier
    ///
    /// Returns true if this is the last CPU to arrive
    pub fn wait(&self) -> bool {
        let gen = self.generation.load(Ordering::Acquire);
        let count = self.count.load(Ordering::Relaxed);

        // Increment waiting count
        let prev_waiting = self.waiting.fetch_add(1, Ordering::SeqCst);

        if prev_waiting + 1 >= count {
            // Last CPU to arrive - release all waiters
            self.waiting.store(0, Ordering::SeqCst);
            self.generation.fetch_add(1, Ordering::SeqCst);
            return true;
        }

        // Wait for generation to change
        while self.generation.load(Ordering::Acquire) == gen {
            spin_loop();
        }

        false
    }

    /// Try to wait at barrier with timeout
    ///
    /// Returns Some(true) if last to arrive, Some(false) if not last,
    /// None if timeout
    pub fn wait_timeout(&self, timeout_cycles: u64) -> Option<bool> {
        let gen = self.generation.load(Ordering::Acquire);
        let count = self.count.load(Ordering::Relaxed);

        let prev_waiting = self.waiting.fetch_add(1, Ordering::SeqCst);

        if prev_waiting + 1 >= count {
            self.waiting.store(0, Ordering::SeqCst);
            self.generation.fetch_add(1, Ordering::SeqCst);
            return Some(true);
        }

        let start = read_tsc();
        while self.generation.load(Ordering::Acquire) == gen {
            if read_tsc().wrapping_sub(start) > timeout_cycles {
                // Timeout - decrement waiting count
                self.waiting.fetch_sub(1, Ordering::SeqCst);
                return None;
            }
            spin_loop();
        }

        Some(false)
    }
}

// =============================================================================
// Spin Barrier
// =============================================================================

/// Spinning barrier with sense reversal
///
/// More efficient for frequent synchronization
pub struct SpinBarrier {
    /// Expected count
    count: u32,
    /// Current count
    current: AtomicU32,
    /// Sense flag (alternates each barrier)
    sense: AtomicBool,
}

impl SpinBarrier {
    /// Create a new spinning barrier
    pub const fn new(n: u32) -> Self {
        Self {
            count: n,
            current: AtomicU32::new(0),
            sense: AtomicBool::new(false),
        }
    }

    /// Wait at barrier
    pub fn wait(&self) {
        let my_sense = !self.sense.load(Ordering::Acquire);

        let position = self.current.fetch_add(1, Ordering::SeqCst);

        if position + 1 == self.count {
            // Last arrival
            self.current.store(0, Ordering::SeqCst);
            self.sense.store(my_sense, Ordering::Release);
        } else {
            // Wait for sense to flip
            while self.sense.load(Ordering::Acquire) != my_sense {
                spin_loop();
            }
        }
    }
}

// =============================================================================
// Sequence Lock
// =============================================================================

/// Sequence lock for read-mostly data
///
/// Allows concurrent reads with occasional writes.
/// Writers have priority and readers may need to retry.
pub struct SeqLock<T> {
    /// Sequence counter (odd = write in progress)
    sequence: AtomicU64,
    /// Protected data
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send + Sync> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    /// Create a new sequence lock
    pub const fn new(data: T) -> Self {
        Self {
            sequence: AtomicU64::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Read data, retrying if a write occurred
    pub fn read(&self) -> T {
        loop {
            let seq1 = self.sequence.load(Ordering::Acquire);

            // Wait if write in progress
            if seq1 & 1 != 0 {
                spin_loop();
                continue;
            }

            // Read the data
            fence(Ordering::Acquire);
            let data = unsafe { *self.data.get() };
            fence(Ordering::Acquire);

            // Check if write occurred
            let seq2 = self.sequence.load(Ordering::Acquire);

            if seq1 == seq2 {
                return data;
            }

            spin_loop();
        }
    }

    /// Try to read without blocking
    ///
    /// Returns None if write is in progress
    pub fn try_read(&self) -> Option<T> {
        let seq1 = self.sequence.load(Ordering::Acquire);

        if seq1 & 1 != 0 {
            return None;
        }

        fence(Ordering::Acquire);
        let data = unsafe { *self.data.get() };
        fence(Ordering::Acquire);

        let seq2 = self.sequence.load(Ordering::Acquire);

        if seq1 == seq2 {
            Some(data)
        } else {
            None
        }
    }

    /// Write data
    pub fn write(&self, data: T) {
        // Increment to odd (write in progress)
        self.sequence.fetch_add(1, Ordering::Release);
        fence(Ordering::Release);

        unsafe {
            *self.data.get() = data;
        }

        fence(Ordering::Release);
        // Increment to even (write complete)
        self.sequence.fetch_add(1, Ordering::Release);
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Relaxed)
    }
}

// =============================================================================
// Spin Lock
// =============================================================================

/// Simple spinlock
pub struct SpinLock<T> {
    /// Lock flag
    locked: AtomicBool,
    /// Protected data
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Create a new spinlock
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Wait until lock looks free
            while self.locked.load(Ordering::Relaxed) {
                spin_loop();
            }
        }

        SpinLockGuard {
            lock: self,
            _marker: PhantomData,
        }
    }

    /// Try to acquire the lock
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard {
                lock: self,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Force unlock (unsafe)
    ///
    /// # Safety
    /// Must only be called when you own the lock
    pub unsafe fn force_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

/// Spinlock guard
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    _marker: PhantomData<*mut ()>,
}

impl<T> core::ops::Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

// =============================================================================
// Ticket Lock (Fair Spinlock)
// =============================================================================

/// Ticket lock - FIFO fair spinlock
pub struct TicketLock<T> {
    /// Next ticket to be taken
    next: AtomicU32,
    /// Currently serving ticket
    serving: AtomicU32,
    /// Protected data
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for TicketLock<T> {}
unsafe impl<T: Send> Sync for TicketLock<T> {}

impl<T> TicketLock<T> {
    /// Create a new ticket lock
    pub const fn new(data: T) -> Self {
        Self {
            next: AtomicU32::new(0),
            serving: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock
    pub fn lock(&self) -> TicketLockGuard<'_, T> {
        // Take a ticket
        let ticket = self.next.fetch_add(1, Ordering::Relaxed);

        // Wait for our turn
        while self.serving.load(Ordering::Acquire) != ticket {
            spin_loop();
        }

        TicketLockGuard {
            lock: self,
            _marker: PhantomData,
        }
    }

    /// Try to acquire the lock
    pub fn try_lock(&self) -> Option<TicketLockGuard<'_, T>> {
        let next = self.next.load(Ordering::Relaxed);
        let serving = self.serving.load(Ordering::Relaxed);

        if next == serving {
            if self
                .next
                .compare_exchange(next, next + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Some(TicketLockGuard {
                    lock: self,
                    _marker: PhantomData,
                });
            }
        }
        None
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.next.load(Ordering::Relaxed) != self.serving.load(Ordering::Relaxed)
    }
}

/// Ticket lock guard
pub struct TicketLockGuard<'a, T> {
    lock: &'a TicketLock<T>,
    _marker: PhantomData<*mut ()>,
}

impl<T> core::ops::Deref for TicketLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for TicketLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for TicketLockGuard<'_, T> {
    fn drop(&mut self) {
        // Serve next customer
        self.lock.serving.fetch_add(1, Ordering::Release);
    }
}

// =============================================================================
// Reader-Writer Lock
// =============================================================================

/// Reader-writer lock
///
/// Multiple readers or single writer
pub struct RwLock<T> {
    /// State: bits 0-30 = reader count, bit 31 = write lock
    state: AtomicU32,
    /// Protected data
    data: UnsafeCell<T>,
}

const WRITE_LOCKED: u32 = 1 << 31;
const MAX_READERS: u32 = (1 << 31) - 1;

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Create a new reader-writer lock
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire read lock
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let state = self.state.load(Ordering::Relaxed);

            // Wait if write locked
            if state & WRITE_LOCKED != 0 {
                spin_loop();
                continue;
            }

            // Try to add reader
            if self
                .state
                .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        RwLockReadGuard {
            lock: self,
            _marker: PhantomData,
        }
    }

    /// Acquire write lock
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // First, set write lock bit
        loop {
            let state = self.state.load(Ordering::Relaxed);

            if state & WRITE_LOCKED != 0 {
                spin_loop();
                continue;
            }

            if self
                .state
                .compare_exchange_weak(
                    state,
                    state | WRITE_LOCKED,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        // Wait for readers to drain
        while self.state.load(Ordering::Acquire) & MAX_READERS != 0 {
            spin_loop();
        }

        RwLockWriteGuard {
            lock: self,
            _marker: PhantomData,
        }
    }

    /// Try to acquire read lock
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        let state = self.state.load(Ordering::Relaxed);

        if state & WRITE_LOCKED != 0 {
            return None;
        }

        if self
            .state
            .compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(RwLockReadGuard {
                lock: self,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Try to acquire write lock
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(0, WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(RwLockWriteGuard {
                lock: self,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }
}

/// Read lock guard
pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
    _marker: PhantomData<*const ()>,
}

impl<T> core::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

/// Write lock guard
pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
    _marker: PhantomData<*mut ()>,
}

impl<T> core::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}

// =============================================================================
// Memory Barriers
// =============================================================================

/// Full memory barrier
#[inline]
pub fn memory_barrier() {
    fence(Ordering::SeqCst);
}

/// Read memory barrier
#[inline]
pub fn read_barrier() {
    fence(Ordering::Acquire);
}

/// Write memory barrier
#[inline]
pub fn write_barrier() {
    fence(Ordering::Release);
}

/// Compiler barrier (prevent reordering)
#[inline]
pub fn compiler_barrier() {
    fence(Ordering::SeqCst);
}

/// MFENCE instruction (serializing)
#[inline]
pub fn mfence() {
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
}

/// LFENCE instruction (load fence)
#[inline]
pub fn lfence() {
    unsafe {
        core::arch::asm!("lfence", options(nostack, preserves_flags));
    }
}

/// SFENCE instruction (store fence)
#[inline]
pub fn sfence() {
    unsafe {
        core::arch::asm!("sfence", options(nostack, preserves_flags));
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Read TSC
#[inline]
fn read_tsc() -> u64 {
    let (lo, hi): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nostack, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}
