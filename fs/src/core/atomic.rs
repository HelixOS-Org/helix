//! Lock-free atomic primitives and concurrent data structures.
//!
//! This module provides atomic types and lock-free data structures used
//! throughout HelixFS for high-performance concurrent access.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
pub use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Memory Ordering Helpers
// ============================================================================

/// Relaxed ordering - no synchronization guarantees
pub const RELAXED: Ordering = Ordering::Relaxed;

/// Acquire ordering - subsequent reads see prior writes
pub const ACQUIRE: Ordering = Ordering::Acquire;

/// Release ordering - prior writes are visible to subsequent reads
pub const RELEASE: Ordering = Ordering::Release;

/// Acquire-Release ordering - both acquire and release
pub const ACQ_REL: Ordering = Ordering::AcqRel;

/// Sequentially consistent ordering - total order across all threads
pub const SEQ_CST: Ordering = Ordering::SeqCst;

// ============================================================================
// Atomic Counter
// ============================================================================

/// Atomic counter with various operations.
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    /// Create a new counter with initial value
    #[inline]
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    /// Load current value
    #[inline]
    pub fn load(&self) -> u64 {
        self.value.load(ACQUIRE)
    }

    /// Store a new value
    #[inline]
    pub fn store(&self, value: u64) {
        self.value.store(value, RELEASE);
    }

    /// Increment and return new value
    #[inline]
    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, ACQ_REL) + 1
    }

    /// Decrement and return new value
    #[inline]
    pub fn decrement(&self) -> u64 {
        self.value.fetch_sub(1, ACQ_REL) - 1
    }

    /// Add a value and return new value
    #[inline]
    pub fn add(&self, delta: u64) -> u64 {
        self.value.fetch_add(delta, ACQ_REL) + delta
    }

    /// Subtract a value and return new value
    #[inline]
    pub fn sub(&self, delta: u64) -> u64 {
        self.value.fetch_sub(delta, ACQ_REL) - delta
    }

    /// Compare and swap
    #[inline]
    pub fn compare_exchange(&self, current: u64, new: u64) -> Result<u64, u64> {
        self.value.compare_exchange(current, new, ACQ_REL, ACQUIRE)
    }

    /// Swap and return old value
    #[inline]
    pub fn swap(&self, value: u64) -> u64 {
        self.value.swap(value, ACQ_REL)
    }

    /// Update with a function, retrying on contention
    #[inline]
    pub fn fetch_update<F>(&self, mut f: F) -> u64
    where
        F: FnMut(u64) -> u64,
    {
        let mut current = self.load();
        loop {
            let new = f(current);
            match self.compare_exchange(current, new) {
                Ok(_) => return new,
                Err(actual) => current = actual,
            }
        }
    }

    /// Get maximum of current and new value
    #[inline]
    pub fn fetch_max(&self, value: u64) -> u64 {
        self.value.fetch_max(value, ACQ_REL)
    }

    /// Get minimum of current and new value
    #[inline]
    pub fn fetch_min(&self, value: u64) -> u64 {
        self.value.fetch_min(value, ACQ_REL)
    }
}

// ============================================================================
// Atomic Flag
// ============================================================================

/// Atomic boolean flag.
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicFlag {
    flag: AtomicBool,
}

impl AtomicFlag {
    /// Create new flag (initially false)
    #[inline]
    pub const fn new(initial: bool) -> Self {
        Self {
            flag: AtomicBool::new(initial),
        }
    }

    /// Load current value
    #[inline]
    pub fn load(&self) -> bool {
        self.flag.load(ACQUIRE)
    }

    /// Store a value
    #[inline]
    pub fn store(&self, value: bool) {
        self.flag.store(value, RELEASE);
    }

    /// Set flag and return previous value
    #[inline]
    pub fn set(&self) -> bool {
        self.flag.swap(true, ACQ_REL)
    }

    /// Clear flag and return previous value
    #[inline]
    pub fn clear(&self) -> bool {
        self.flag.swap(false, ACQ_REL)
    }

    /// Toggle flag and return new value
    #[inline]
    pub fn toggle(&self) -> bool {
        !self.flag.fetch_xor(true, ACQ_REL)
    }

    /// Try to set flag, return true if successful (was false)
    #[inline]
    pub fn try_set(&self) -> bool {
        !self.flag.swap(true, ACQ_REL)
    }

    /// Compare and swap
    #[inline]
    pub fn compare_exchange(&self, current: bool, new: bool) -> Result<bool, bool> {
        self.flag.compare_exchange(current, new, ACQ_REL, ACQUIRE)
    }
}

// ============================================================================
// Sequence Lock (SeqLock)
// ============================================================================

/// Sequence lock for read-optimized data.
///
/// A SeqLock allows concurrent reads without blocking, by using a sequence
/// counter to detect concurrent writes. Writers must be mutually exclusive.
#[derive(Debug)]
pub struct SeqLock<T> {
    sequence: AtomicU32,
    data: UnsafeCell<T>,
}

// SAFETY: SeqLock provides its own synchronization
unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    /// Create a new SeqLock
    pub const fn new(data: T) -> Self {
        Self {
            sequence: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Read data, retrying if a write occurred during read.
    ///
    /// This is wait-free for readers when there are no concurrent writes.
    #[inline]
    pub fn read(&self) -> T {
        loop {
            // Wait for even sequence (no write in progress)
            let seq1 = loop {
                let s = self.sequence.load(ACQUIRE);
                if s & 1 == 0 {
                    break s;
                }
                core::hint::spin_loop();
            };

            // Copy data
            // SAFETY: We have a consistent view because seq is even
            let data = unsafe { *self.data.get() };

            // Memory barrier
            core::sync::atomic::fence(ACQUIRE);

            // Check sequence didn't change
            let seq2 = self.sequence.load(RELAXED);
            if seq1 == seq2 {
                return data;
            }

            // Sequence changed, retry
            core::hint::spin_loop();
        }
    }

    /// Begin a write operation (must be externally synchronized for multiple writers).
    #[inline]
    pub fn write_begin(&self) {
        // Increment to odd (write in progress)
        let prev = self.sequence.fetch_add(1, RELEASE);
        debug_assert!(prev & 1 == 0, "Nested write detected");
    }

    /// End a write operation.
    #[inline]
    pub fn write_end(&self) {
        // Increment to even (write complete)
        self.sequence.fetch_add(1, RELEASE);
    }

    /// Get mutable reference to data (caller must ensure exclusive access).
    ///
    /// # Safety
    /// Caller must ensure no concurrent reads or writes.
    /// This uses interior mutability via `UnsafeCell`, which is why
    /// returning `&mut T` from `&self` is valid.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    /// Write data with automatic begin/end.
    #[inline]
    pub fn write(&self, data: T) {
        self.write_begin();
        // SAFETY: Write is in progress (odd sequence), readers will retry
        unsafe {
            *self.data.get() = data;
        }
        self.write_end();
    }
}

// ============================================================================
// Read-Copy-Update (RCU) Primitives
// ============================================================================

/// Epoch counter for epoch-based reclamation.
#[derive(Debug, Default)]
pub struct EpochCounter {
    global: AtomicU64,
}

impl EpochCounter {
    /// Create new epoch counter
    pub const fn new() -> Self {
        Self {
            global: AtomicU64::new(0),
        }
    }

    /// Get current epoch
    #[inline]
    pub fn current(&self) -> u64 {
        self.global.load(ACQUIRE)
    }

    /// Advance to next epoch
    #[inline]
    pub fn advance(&self) -> u64 {
        self.global.fetch_add(1, ACQ_REL) + 1
    }
}

/// Per-thread epoch participant.
#[derive(Debug)]
pub struct EpochParticipant {
    active_epoch: AtomicU64,
    in_critical: AtomicBool,
}

impl EpochParticipant {
    /// Create new participant
    pub const fn new() -> Self {
        Self {
            active_epoch: AtomicU64::new(0),
            in_critical: AtomicBool::new(false),
        }
    }

    /// Enter critical section
    #[inline]
    pub fn enter(&self, current_epoch: u64) {
        self.active_epoch.store(current_epoch, RELAXED);
        self.in_critical.store(true, RELEASE);
    }

    /// Leave critical section
    #[inline]
    pub fn leave(&self) {
        self.in_critical.store(false, RELEASE);
    }

    /// Check if in critical section
    #[inline]
    pub fn is_active(&self) -> bool {
        self.in_critical.load(ACQUIRE)
    }

    /// Get active epoch (only valid if is_active)
    #[inline]
    pub fn active_epoch(&self) -> u64 {
        self.active_epoch.load(ACQUIRE)
    }
}

impl Default for EpochParticipant {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lock-Free Stack (Treiber Stack)
// ============================================================================

/// Node in lock-free stack.
pub struct StackNode<T> {
    value: T,
    next: AtomicPtr<StackNode<T>>,
}

impl<T> StackNode<T> {
    /// Create new node
    pub fn new(value: T) -> Self {
        Self {
            value,
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

/// Lock-free stack using Treiber's algorithm.
pub struct LockFreeStack<T> {
    head: AtomicPtr<StackNode<T>>,
    _marker: PhantomData<T>,
}

impl<T> LockFreeStack<T> {
    /// Create empty stack
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(core::ptr::null_mut()),
            _marker: PhantomData,
        }
    }

    /// Check if stack is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(ACQUIRE).is_null()
    }

    /// Push a node onto the stack.
    ///
    /// # Safety
    /// The node must be valid and not already in a stack.
    pub unsafe fn push_node(&self, node: NonNull<StackNode<T>>) {
        let node_ptr = node.as_ptr();
        loop {
            let head = self.head.load(ACQUIRE);
            // SAFETY: We have exclusive access to node
            unsafe {
                (*node_ptr).next.store(head, RELAXED);
            }
            if self
                .head
                .compare_exchange_weak(head, node_ptr, RELEASE, RELAXED)
                .is_ok()
            {
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// Pop a node from the stack.
    ///
    /// # Safety
    /// Caller must ensure proper memory reclamation of returned node.
    pub unsafe fn pop_node(&self) -> Option<NonNull<StackNode<T>>> {
        loop {
            let head = self.head.load(ACQUIRE);
            if head.is_null() {
                return None;
            }
            // SAFETY: head is non-null and valid
            let next = unsafe { (*head).next.load(RELAXED) };
            if self
                .head
                .compare_exchange_weak(head, next, RELEASE, RELAXED)
                .is_ok()
            {
                return Some(unsafe { NonNull::new_unchecked(head) });
            }
            core::hint::spin_loop();
        }
    }
}

impl<T> Default for LockFreeStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: Stack is lock-free and can be safely shared
unsafe impl<T: Send> Send for LockFreeStack<T> {}
unsafe impl<T: Send> Sync for LockFreeStack<T> {}

// ============================================================================
// Atomic Bitset
// ============================================================================

/// Atomic bitset for tracking allocated blocks/inodes.
pub struct AtomicBitset {
    words: [AtomicU64; 64], // 4096 bits = 64 * 64
}

impl AtomicBitset {
    /// Number of bits in the bitset
    pub const BITS: usize = 64 * 64;

    /// Create new empty bitset (all zeros)
    pub const fn new() -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self { words: [ZERO; 64] }
    }

    /// Set a bit atomically, returns previous value
    #[inline]
    pub fn set(&self, bit: usize) -> bool {
        debug_assert!(bit < Self::BITS);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);
        (self.words[word].fetch_or(mask, ACQ_REL) & mask) != 0
    }

    /// Clear a bit atomically, returns previous value
    #[inline]
    pub fn clear(&self, bit: usize) -> bool {
        debug_assert!(bit < Self::BITS);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);
        (self.words[word].fetch_and(!mask, ACQ_REL) & mask) != 0
    }

    /// Test a bit
    #[inline]
    pub fn test(&self, bit: usize) -> bool {
        debug_assert!(bit < Self::BITS);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);
        (self.words[word].load(ACQUIRE) & mask) != 0
    }

    /// Try to set a bit (set only if currently clear)
    #[inline]
    pub fn try_set(&self, bit: usize) -> bool {
        debug_assert!(bit < Self::BITS);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);

        loop {
            let current = self.words[word].load(ACQUIRE);
            if (current & mask) != 0 {
                return false; // Already set
            }
            if self.words[word]
                .compare_exchange_weak(current, current | mask, ACQ_REL, RELAXED)
                .is_ok()
            {
                return true;
            }
            core::hint::spin_loop();
        }
    }

    /// Find first clear bit and set it atomically
    #[inline]
    pub fn find_and_set(&self) -> Option<usize> {
        for word_idx in 0..64 {
            let mut current = self.words[word_idx].load(ACQUIRE);
            while current != u64::MAX {
                // Find first zero bit
                let bit_idx = current.trailing_ones() as usize;
                let mask = 1u64 << bit_idx;

                match self.words[word_idx].compare_exchange_weak(
                    current,
                    current | mask,
                    ACQ_REL,
                    RELAXED,
                ) {
                    Ok(_) => return Some(word_idx * 64 + bit_idx),
                    Err(new) => current = new,
                }
            }
        }
        None
    }

    /// Count set bits
    #[inline]
    pub fn count_ones(&self) -> usize {
        self.words
            .iter()
            .map(|w| w.load(RELAXED).count_ones() as usize)
            .sum()
    }

    /// Count clear bits
    #[inline]
    pub fn count_zeros(&self) -> usize {
        Self::BITS - self.count_ones()
    }
}

impl Default for AtomicBitset {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Spin Mutex (for when we need mutual exclusion)
// ============================================================================

/// Simple spin mutex for short critical sections.
///
/// This should only be used for very short operations where contention is
/// expected to be low. For longer operations, use proper blocking locks.
#[derive(Debug)]
pub struct SpinMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

/// Guard for SpinMutex
pub struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
}

impl<T> SpinMutex<T> {
    /// Create new unlocked mutex
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock
    #[inline]
    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        loop {
            // Try to acquire
            if !self.locked.swap(true, ACQUIRE) {
                return SpinMutexGuard { mutex: self };
            }

            // Spin with backoff
            let mut spins = 0u32;
            while self.locked.load(RELAXED) {
                core::hint::spin_loop();
                spins = spins.saturating_add(1);
                if spins > 1000 {
                    // Yield in no_std kernel context - just spin more
                    spins = 0;
                }
            }
        }
    }

    /// Try to acquire the lock without blocking
    #[inline]
    pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
        if !self.locked.swap(true, ACQUIRE) {
            Some(SpinMutexGuard { mutex: self })
        } else {
            None
        }
    }

    /// Get inner value (consumes mutex)
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: We hold the lock
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: We hold the lock exclusively
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, RELEASE);
    }
}

// SAFETY: SpinMutex provides mutual exclusion
unsafe impl<T: Send> Send for SpinMutex<T> {}
unsafe impl<T: Send> Sync for SpinMutex<T> {}

// ============================================================================
// Read-Write Spin Lock
// ============================================================================

/// Read-write spin lock.
///
/// Allows multiple concurrent readers or exclusive writer access.
/// Writers are prioritized to prevent starvation.
#[derive(Debug)]
pub struct RwSpinLock<T> {
    /// Lock state:
    /// - 0: unlocked
    /// - positive: number of readers
    /// - -1: writer holds lock
    state: AtomicU32,
    /// Number of waiting writers (for priority)
    waiting_writers: AtomicU32,
    data: UnsafeCell<T>,
}

/// Read guard for RwSpinLock
pub struct RwSpinReadGuard<'a, T> {
    lock: &'a RwSpinLock<T>,
}

/// Write guard for RwSpinLock
pub struct RwSpinWriteGuard<'a, T> {
    lock: &'a RwSpinLock<T>,
}

impl<T> RwSpinLock<T> {
    const WRITER: u32 = u32::MAX; // -1 as unsigned

    /// Create new unlocked RW lock
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            waiting_writers: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire read lock
    #[inline]
    pub fn read(&self) -> RwSpinReadGuard<'_, T> {
        loop {
            // Wait if writers are waiting or writing
            while self.waiting_writers.load(RELAXED) > 0 || self.state.load(RELAXED) == Self::WRITER
            {
                core::hint::spin_loop();
            }

            let current = self.state.load(ACQUIRE);
            if current != Self::WRITER
                && self
                    .state
                    .compare_exchange_weak(current, current + 1, ACQUIRE, RELAXED)
                    .is_ok()
            {
                return RwSpinReadGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }

    /// Try to acquire read lock
    #[inline]
    pub fn try_read(&self) -> Option<RwSpinReadGuard<'_, T>> {
        let current = self.state.load(ACQUIRE);
        if current == Self::WRITER || self.waiting_writers.load(RELAXED) > 0 {
            return None;
        }

        if self
            .state
            .compare_exchange(current, current + 1, ACQUIRE, RELAXED)
            .is_ok()
        {
            Some(RwSpinReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Acquire write lock
    #[inline]
    pub fn write(&self) -> RwSpinWriteGuard<'_, T> {
        // Mark that we're waiting
        self.waiting_writers.fetch_add(1, RELAXED);

        loop {
            if self
                .state
                .compare_exchange_weak(0, Self::WRITER, ACQUIRE, RELAXED)
                .is_ok()
            {
                self.waiting_writers.fetch_sub(1, RELAXED);
                return RwSpinWriteGuard { lock: self };
            }
            core::hint::spin_loop();
        }
    }

    /// Try to acquire write lock
    #[inline]
    pub fn try_write(&self) -> Option<RwSpinWriteGuard<'_, T>> {
        if self
            .state
            .compare_exchange(0, Self::WRITER, ACQUIRE, RELAXED)
            .is_ok()
        {
            Some(RwSpinWriteGuard { lock: self })
        } else {
            None
        }
    }
}

impl<T> Deref for RwSpinReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Deref for RwSpinWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for RwSpinWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for RwSpinReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, RELEASE);
    }
}

impl<T> Drop for RwSpinWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, RELEASE);
    }
}

// SAFETY: RwSpinLock provides proper synchronization
unsafe impl<T: Send> Send for RwSpinLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwSpinLock<T> {}
