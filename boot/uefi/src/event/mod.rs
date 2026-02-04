//! Event and Signaling Infrastructure
//!
//! UEFI event system, synchronization primitives, and notification mechanisms.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// =============================================================================
// EVENT HANDLE
// =============================================================================

/// Event handle (opaque pointer in UEFI)
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventHandle(usize);

impl EventHandle {
    /// Create from raw pointer
    pub fn from_ptr(ptr: *mut core::ffi::c_void) -> Self {
        Self(ptr as usize)
    }

    /// Convert to raw pointer
    pub fn as_ptr(self) -> *mut core::ffi::c_void {
        self.0 as *mut core::ffi::c_void
    }

    /// Null handle
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if null
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl Default for EventHandle {
    fn default() -> Self {
        Self::null()
    }
}

// =============================================================================
// EVENT TYPE FLAGS
// =============================================================================

/// Event type flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct EventType(u32);

impl EventType {
    /// Timer event
    pub const TIMER: Self = Self(0x80000000);

    /// Runtime event (called at runtime services)
    pub const RUNTIME: Self = Self(0x40000000);

    /// Notify wait (level triggered)
    pub const NOTIFY_WAIT: Self = Self(0x00000100);

    /// Notify signal (edge triggered)
    pub const NOTIFY_SIGNAL: Self = Self(0x00000200);

    /// Signal exit boot services
    pub const SIGNAL_EXIT_BOOT_SERVICES: Self = Self(0x00000201);

    /// Signal virtual address map
    pub const SIGNAL_VIRTUAL_ADDRESS_MAP: Self = Self(0x00060202);

    /// Create from raw value
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get raw value
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Combine event types
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if timer event
    pub fn is_timer(self) -> bool {
        self.0 & Self::TIMER.0 != 0
    }

    /// Check if runtime event
    pub fn is_runtime(self) -> bool {
        self.0 & Self::RUNTIME.0 != 0
    }

    /// Check if notify wait
    pub fn is_notify_wait(self) -> bool {
        self.0 & 0x00000300 == Self::NOTIFY_WAIT.0
    }

    /// Check if notify signal
    pub fn is_notify_signal(self) -> bool {
        self.0 & 0x00000300 == Self::NOTIFY_SIGNAL.0
    }
}

// =============================================================================
// TIMER DELAY
// =============================================================================

/// Timer delay type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimerDelay {
    /// Cancel timer
    Cancel   = 0,
    /// Periodic timer
    Periodic = 1,
    /// Relative timer (one-shot)
    Relative = 2,
}

// =============================================================================
// TASK PRIORITY LEVEL
// =============================================================================

/// Task Priority Level (TPL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Tpl(usize);

impl Tpl {
    /// Application level (lowest)
    pub const APPLICATION: Self = Self(4);

    /// Callback level
    pub const CALLBACK: Self = Self(8);

    /// Notify level
    pub const NOTIFY: Self = Self(16);

    /// High level (highest for normal code)
    pub const HIGH_LEVEL: Self = Self(31);

    /// Create from raw value
    pub const fn from_raw(value: usize) -> Self {
        Self(value)
    }

    /// Get raw value
    pub const fn as_raw(self) -> usize {
        self.0
    }
}

impl Default for Tpl {
    fn default() -> Self {
        Self::APPLICATION
    }
}

// =============================================================================
// EVENT STATE
// =============================================================================

/// Event state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    /// Not signaled
    NotSignaled,
    /// Signaled
    Signaled,
    /// Waiting
    Waiting,
    /// Error
    Error,
}

// =============================================================================
// SOFTWARE EVENT SYSTEM
// =============================================================================

/// Maximum number of events
const MAX_EVENTS: usize = 64;

/// Software event for boot loader
pub struct Event {
    /// Event ID
    id: u32,
    /// Event type
    event_type: EventType,
    /// Is signaled
    signaled: AtomicBool,
    /// Trigger count
    trigger_count: AtomicU32,
    /// Associated data
    context: AtomicU64,
    /// Timer target (for timer events)
    timer_target: AtomicU64,
    /// Timer period (for periodic timers)
    timer_period: AtomicU64,
    /// Is active
    active: AtomicBool,
}

impl Event {
    /// Create new event
    pub const fn new(id: u32, event_type: EventType) -> Self {
        Self {
            id,
            event_type,
            signaled: AtomicBool::new(false),
            trigger_count: AtomicU32::new(0),
            context: AtomicU64::new(0),
            timer_target: AtomicU64::new(0),
            timer_period: AtomicU64::new(0),
            active: AtomicBool::new(false),
        }
    }

    /// Get event ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get event type
    pub fn event_type(&self) -> EventType {
        self.event_type
    }

    /// Check if signaled
    pub fn is_signaled(&self) -> bool {
        self.signaled.load(Ordering::Acquire)
    }

    /// Signal the event
    pub fn signal(&self) {
        self.signaled.store(true, Ordering::Release);
        self.trigger_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Clear (reset) the event
    pub fn clear(&self) {
        self.signaled.store(false, Ordering::Release);
    }

    /// Wait for event (blocking)
    pub fn wait(&self) {
        while !self.signaled.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }

    /// Try wait (non-blocking)
    pub fn try_wait(&self) -> bool {
        self.signaled.load(Ordering::Acquire)
    }

    /// Wait with timeout (returns true if signaled, false on timeout)
    pub fn wait_timeout(&self, timeout_cycles: u64) -> bool {
        let start = read_counter();

        while !self.signaled.load(Ordering::Acquire) {
            if read_counter().saturating_sub(start) > timeout_cycles {
                return false;
            }
            core::hint::spin_loop();
        }

        true
    }

    /// Get trigger count
    pub fn trigger_count(&self) -> u32 {
        self.trigger_count.load(Ordering::Relaxed)
    }

    /// Set context data
    pub fn set_context(&self, data: u64) {
        self.context.store(data, Ordering::Release);
    }

    /// Get context data
    pub fn context(&self) -> u64 {
        self.context.load(Ordering::Acquire)
    }

    /// Set timer
    pub fn set_timer(&self, target: u64, period: u64) {
        self.timer_target.store(target, Ordering::Release);
        self.timer_period.store(period, Ordering::Release);
    }

    /// Cancel timer
    pub fn cancel_timer(&self) {
        self.timer_target.store(0, Ordering::Release);
        self.timer_period.store(0, Ordering::Release);
    }

    /// Check timer expiration
    pub fn check_timer(&self, current_tick: u64) -> bool {
        let target = self.timer_target.load(Ordering::Acquire);

        if target == 0 {
            return false;
        }

        if current_tick >= target {
            let period = self.timer_period.load(Ordering::Acquire);

            if period > 0 {
                // Periodic - reschedule
                self.timer_target
                    .store(current_tick + period, Ordering::Release);
            } else {
                // One-shot - clear
                self.timer_target.store(0, Ordering::Release);
            }

            self.signal();
            return true;
        }

        false
    }

    /// Activate event
    pub fn activate(&self) {
        self.active.store(true, Ordering::Release);
    }

    /// Deactivate event
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }

    /// Is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

// =============================================================================
// EVENT GROUP
// =============================================================================

/// Event group for waiting on multiple events
pub struct EventGroup<const N: usize> {
    events: [Option<u32>; N],
    count: usize,
}

impl<const N: usize> EventGroup<N> {
    /// Create new empty group
    pub const fn new() -> Self {
        Self {
            events: [None; N],
            count: 0,
        }
    }

    /// Add event to group
    pub fn add(&mut self, event_id: u32) -> bool {
        if self.count >= N {
            return false;
        }

        self.events[self.count] = Some(event_id);
        self.count += 1;
        true
    }

    /// Remove event from group
    pub fn remove(&mut self, event_id: u32) -> bool {
        for i in 0..self.count {
            if self.events[i] == Some(event_id) {
                // Shift remaining events
                for j in i..self.count - 1 {
                    self.events[j] = self.events[j + 1];
                }
                self.events[self.count - 1] = None;
                self.count -= 1;
                return true;
            }
        }
        false
    }

    /// Get event count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clear all events
    pub fn clear(&mut self) {
        for event in &mut self.events {
            *event = None;
        }
        self.count = 0;
    }

    /// Get events
    pub fn events(&self) -> &[Option<u32>] {
        &self.events[..self.count]
    }
}

impl<const N: usize> Default for EventGroup<N> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SEMAPHORE
// =============================================================================

/// Counting semaphore
pub struct Semaphore {
    count: AtomicU32,
    max_count: u32,
}

impl Semaphore {
    /// Create new semaphore
    pub const fn new(initial: u32, max: u32) -> Self {
        Self {
            count: AtomicU32::new(initial),
            max_count: max,
        }
    }

    /// Acquire (decrement)
    pub fn acquire(&self) {
        loop {
            let current = self.count.load(Ordering::Acquire);

            if current > 0 {
                if self
                    .count
                    .compare_exchange_weak(
                        current,
                        current - 1,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return;
                }
            }

            core::hint::spin_loop();
        }
    }

    /// Try acquire (non-blocking)
    pub fn try_acquire(&self) -> bool {
        loop {
            let current = self.count.load(Ordering::Acquire);

            if current == 0 {
                return false;
            }

            if self
                .count
                .compare_exchange_weak(current, current - 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Release (increment)
    pub fn release(&self) {
        loop {
            let current = self.count.load(Ordering::Acquire);

            if current >= self.max_count {
                return; // Already at max
            }

            if self
                .count
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
    }

    /// Get current count
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Acquire)
    }

    /// Get max count
    pub fn max_count(&self) -> u32 {
        self.max_count
    }
}

// =============================================================================
// MUTEX (SPINLOCK)
// =============================================================================

/// Simple spinlock mutex
pub struct Mutex {
    locked: AtomicBool,
}

impl Mutex {
    /// Create new unlocked mutex
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    /// Lock
    pub fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
    }

    /// Try lock
    pub fn try_lock(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Unlock
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Is locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }
}

impl Default for Mutex {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// READ-WRITE LOCK
// =============================================================================

/// Read-write lock
pub struct RwLock {
    /// State: 0 = unlocked, positive = reader count, -1 = write locked
    state: AtomicU32,
}

impl RwLock {
    const WRITE_LOCKED: u32 = u32::MAX;

    /// Create new unlocked lock
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
        }
    }

    /// Acquire read lock
    pub fn read_lock(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);

            if state != Self::WRITE_LOCKED {
                if self
                    .state
                    .compare_exchange_weak(state, state + 1, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }
            }

            core::hint::spin_loop();
        }
    }

    /// Try read lock
    pub fn try_read_lock(&self) -> bool {
        loop {
            let state = self.state.load(Ordering::Acquire);

            if state == Self::WRITE_LOCKED {
                return false;
            }

            if self
                .state
                .compare_exchange_weak(state, state + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Release read lock
    pub fn read_unlock(&self) {
        self.state.fetch_sub(1, Ordering::Release);
    }

    /// Acquire write lock
    pub fn write_lock(&self) {
        while self
            .state
            .compare_exchange_weak(0, Self::WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            while self.state.load(Ordering::Relaxed) != 0 {
                core::hint::spin_loop();
            }
        }
    }

    /// Try write lock
    pub fn try_write_lock(&self) -> bool {
        self.state
            .compare_exchange(0, Self::WRITE_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// Release write lock
    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
    }

    /// Get reader count
    pub fn reader_count(&self) -> u32 {
        let state = self.state.load(Ordering::Relaxed);
        if state == Self::WRITE_LOCKED {
            0
        } else {
            state
        }
    }

    /// Is write locked
    pub fn is_write_locked(&self) -> bool {
        self.state.load(Ordering::Relaxed) == Self::WRITE_LOCKED
    }
}

impl Default for RwLock {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ONCE (FOR INITIALIZATION)
// =============================================================================

/// One-time initialization
pub struct Once {
    state: AtomicU32,
}

impl Once {
    const INCOMPLETE: u32 = 0;
    const RUNNING: u32 = 1;
    const COMPLETE: u32 = 2;

    /// Create new Once
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(Self::INCOMPLETE),
        }
    }

    /// Call the initialization function once
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.state.load(Ordering::Acquire) == Self::COMPLETE {
            return;
        }

        if self
            .state
            .compare_exchange(
                Self::INCOMPLETE,
                Self::RUNNING,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            f();
            self.state.store(Self::COMPLETE, Ordering::Release);
        } else {
            // Wait for completion
            while self.state.load(Ordering::Acquire) != Self::COMPLETE {
                core::hint::spin_loop();
            }
        }
    }

    /// Is completed
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == Self::COMPLETE
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// BARRIER
// =============================================================================

/// Barrier for synchronizing threads
pub struct Barrier {
    count: AtomicU32,
    target: u32,
    generation: AtomicU32,
}

impl Barrier {
    /// Create new barrier
    pub const fn new(count: u32) -> Self {
        Self {
            count: AtomicU32::new(0),
            target: count,
            generation: AtomicU32::new(0),
        }
    }

    /// Wait at barrier
    pub fn wait(&self) {
        let gen = self.generation.load(Ordering::Acquire);
        let arrived = self.count.fetch_add(1, Ordering::AcqRel) + 1;

        if arrived >= self.target {
            // Last to arrive - reset and advance
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
        } else {
            // Wait for generation to change
            while self.generation.load(Ordering::Acquire) == gen {
                core::hint::spin_loop();
            }
        }
    }

    /// Get waiting count
    pub fn waiting(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }
}

// =============================================================================
// NOTIFICATION CHANNEL
// =============================================================================

/// Simple notification channel
pub struct Notification {
    signaled: AtomicBool,
    data: AtomicU64,
}

impl Notification {
    /// Create new notification
    pub const fn new() -> Self {
        Self {
            signaled: AtomicBool::new(false),
            data: AtomicU64::new(0),
        }
    }

    /// Send notification with data
    pub fn notify(&self, data: u64) {
        self.data.store(data, Ordering::Release);
        self.signaled.store(true, Ordering::Release);
    }

    /// Wait for notification
    pub fn wait(&self) -> u64 {
        while !self.signaled.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }

        let data = self.data.load(Ordering::Acquire);
        self.signaled.store(false, Ordering::Release);
        data
    }

    /// Try receive notification
    pub fn try_receive(&self) -> Option<u64> {
        if self
            .signaled
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            Some(self.data.load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// Check if signaled
    pub fn is_pending(&self) -> bool {
        self.signaled.load(Ordering::Acquire)
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Read counter (TSC or cycle counter)
#[cfg(target_arch = "x86_64")]
fn read_counter() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nostack, nomem)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

#[cfg(target_arch = "aarch64")]
fn read_counter() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) value,
            options(nostack, nomem)
        );
    }
    value
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn read_counter() -> u64 {
    0
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_event_type() {
        let timer = EventType::TIMER;
        assert!(timer.is_timer());

        let combined = EventType::TIMER.or(EventType::NOTIFY_SIGNAL);
        assert!(combined.is_timer());
        assert!(combined.is_notify_signal());
    }

    #[test]
    fn test_event() {
        let event = Event::new(1, EventType::NOTIFY_SIGNAL);
        assert!(!event.is_signaled());

        event.signal();
        assert!(event.is_signaled());
        assert_eq!(event.trigger_count(), 1);

        event.clear();
        assert!(!event.is_signaled());
    }

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(2, 5);
        assert_eq!(sem.count(), 2);

        assert!(sem.try_acquire());
        assert_eq!(sem.count(), 1);

        sem.release();
        assert_eq!(sem.count(), 2);
    }

    #[test]
    fn test_mutex() {
        let mutex = Mutex::new();
        assert!(!mutex.is_locked());

        assert!(mutex.try_lock());
        assert!(mutex.is_locked());

        mutex.unlock();
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_once() {
        let once = Once::new();
        let mut called = 0;

        once.call_once(|| called += 1);
        assert_eq!(called, 1);

        once.call_once(|| called += 1);
        assert_eq!(called, 1); // Should not call again

        assert!(once.is_completed());
    }
}
