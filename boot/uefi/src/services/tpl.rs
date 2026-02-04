//! Task Priority Level (TPL) Services
//!
//! Safe wrappers for UEFI Task Priority Level management.

use super::boot_services;

// =============================================================================
// TPL LEVELS
// =============================================================================

/// Task Priority Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Tpl(pub usize);

impl Tpl {
    /// Application level (lowest priority)
    pub const APPLICATION: Self = Self(4);

    /// Callback level
    pub const CALLBACK: Self = Self(8);

    /// Notify level
    pub const NOTIFY: Self = Self(16);

    /// High level (highest priority for boot services)
    pub const HIGH_LEVEL: Self = Self(31);

    /// Get raw value
    pub const fn raw(&self) -> usize {
        self.0
    }

    /// Create from raw value
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// Check if this is application level
    pub const fn is_application(&self) -> bool {
        self.0 == Self::APPLICATION.0
    }

    /// Check if this is callback level
    pub const fn is_callback(&self) -> bool {
        self.0 == Self::CALLBACK.0
    }

    /// Check if this is notify level
    pub const fn is_notify(&self) -> bool {
        self.0 == Self::NOTIFY.0
    }

    /// Check if this is high level
    pub const fn is_high_level(&self) -> bool {
        self.0 == Self::HIGH_LEVEL.0
    }

    /// Get TPL name
    pub const fn name(&self) -> &'static str {
        match self.0 {
            4 => "TPL_APPLICATION",
            8 => "TPL_CALLBACK",
            16 => "TPL_NOTIFY",
            31 => "TPL_HIGH_LEVEL",
            _ => "TPL_UNKNOWN",
        }
    }
}

impl core::fmt::Display for Tpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({})", self.name(), self.0)
    }
}

// =============================================================================
// TPL GUARD
// =============================================================================

/// RAII guard for TPL elevation
///
/// Automatically restores the previous TPL when dropped.
pub struct TplGuard {
    /// Previous TPL to restore
    previous: Tpl,
}

impl TplGuard {
    /// Raise TPL to the specified level
    ///
    /// Returns a guard that will restore the previous TPL when dropped.
    pub fn raise(new_tpl: Tpl) -> Self {
        let bs = unsafe { boot_services() };
        let previous = unsafe { ((*bs).raise_tpl)(new_tpl.0) };

        Self {
            previous: Tpl(previous),
        }
    }

    /// Raise to callback level
    pub fn callback() -> Self {
        Self::raise(Tpl::CALLBACK)
    }

    /// Raise to notify level
    pub fn notify() -> Self {
        Self::raise(Tpl::NOTIFY)
    }

    /// Raise to high level
    pub fn high() -> Self {
        Self::raise(Tpl::HIGH_LEVEL)
    }

    /// Get the previous TPL
    pub fn previous(&self) -> Tpl {
        self.previous
    }

    /// Get the current elevated TPL
    pub fn current(&self) -> Tpl {
        get_current_tpl()
    }

    /// Manually restore TPL without dropping
    pub fn restore(self) {
        drop(self);
    }
}

impl Drop for TplGuard {
    fn drop(&mut self) {
        let bs = unsafe { boot_services() };
        unsafe { ((*bs).restore_tpl)(self.previous.0) };
    }
}

// =============================================================================
// TPL FUNCTIONS
// =============================================================================

/// Get current TPL
///
/// Note: There's no direct UEFI call to get the current TPL.
/// This raises to TPL_HIGH_LEVEL and immediately restores to detect current.
pub fn get_current_tpl() -> Tpl {
    let bs = unsafe { boot_services() };

    // Raise to high level to get current
    let current = unsafe { ((*bs).raise_tpl)(Tpl::HIGH_LEVEL.0) };

    // Immediately restore
    unsafe { ((*bs).restore_tpl)(current) };

    Tpl(current)
}

/// Raise TPL temporarily and execute closure
///
/// Returns the result of the closure.
pub fn with_tpl<F, R>(tpl: Tpl, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = TplGuard::raise(tpl);
    f()
}

/// Execute at callback level
pub fn at_callback<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    with_tpl(Tpl::CALLBACK, f)
}

/// Execute at notify level
pub fn at_notify<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    with_tpl(Tpl::NOTIFY, f)
}

/// Execute at high level (essentially disables interrupts)
pub fn at_high_level<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    with_tpl(Tpl::HIGH_LEVEL, f)
}

// =============================================================================
// CRITICAL SECTION
// =============================================================================

/// Critical section guard
///
/// Raises TPL to HIGH_LEVEL to prevent preemption.
pub struct CriticalSection {
    /// TPL guard
    guard: TplGuard,
}

impl CriticalSection {
    /// Enter critical section
    pub fn enter() -> Self {
        Self {
            guard: TplGuard::high(),
        }
    }

    /// Get the previous TPL before entering
    pub fn previous_tpl(&self) -> Tpl {
        self.guard.previous()
    }
}

/// Execute closure in critical section
pub fn critical_section<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _cs = CriticalSection::enter();
    f()
}

// =============================================================================
// TPL LOCK
// =============================================================================

/// A simple spinlock protected by TPL elevation
pub struct TplLock<T> {
    /// Data
    data: core::cell::UnsafeCell<T>,
    /// Lock state
    locked: core::sync::atomic::AtomicBool,
}

impl<T> TplLock<T> {
    /// Create a new TPL lock
    pub const fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            locked: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Lock and get mutable access
    pub fn lock(&self) -> TplLockGuard<'_, T> {
        // Raise TPL to prevent preemption
        let tpl_guard = TplGuard::high();

        // Acquire spinlock (should be instant at TPL_HIGH_LEVEL)
        while self
            .locked
            .compare_exchange(
                false,
                true,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            core::hint::spin_loop();
        }

        TplLockGuard {
            lock: self,
            _tpl_guard: tpl_guard,
        }
    }

    /// Try to lock without blocking
    pub fn try_lock(&self) -> Option<TplLockGuard<'_, T>> {
        let tpl_guard = TplGuard::high();

        if self
            .locked
            .compare_exchange(
                false,
                true,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            Some(TplLockGuard {
                lock: self,
                _tpl_guard: tpl_guard,
            })
        } else {
            None
        }
    }

    /// Get access without locking (unsafe)
    ///
    /// # Safety
    /// Caller must ensure exclusive access
    pub unsafe fn get_unchecked(&self) -> &mut T {
        &mut *self.data.get()
    }
}

unsafe impl<T: Send> Send for TplLock<T> {}
unsafe impl<T: Send> Sync for TplLock<T> {}

/// Guard for TplLock
pub struct TplLockGuard<'a, T> {
    lock: &'a TplLock<T>,
    _tpl_guard: TplGuard,
}

impl<'a, T> core::ops::Deref for TplLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for TplLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for TplLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock
            .locked
            .store(false, core::sync::atomic::Ordering::Release);
    }
}

// =============================================================================
// TPL MUTEX
// =============================================================================

/// A mutex that uses TPL for synchronization
pub struct TplMutex<T> {
    /// Inner data
    inner: TplLock<T>,
}

impl<T> TplMutex<T> {
    /// Create a new mutex
    pub const fn new(data: T) -> Self {
        Self {
            inner: TplLock::new(data),
        }
    }

    /// Lock the mutex
    pub fn lock(&self) -> TplMutexGuard<'_, T> {
        TplMutexGuard {
            guard: self.inner.lock(),
        }
    }

    /// Try to lock
    pub fn try_lock(&self) -> Option<TplMutexGuard<'_, T>> {
        self.inner.try_lock().map(|guard| TplMutexGuard { guard })
    }

    /// Consume mutex and return inner data
    pub fn into_inner(self) -> T {
        self.inner.data.into_inner()
    }
}

/// Guard for TplMutex
pub struct TplMutexGuard<'a, T> {
    guard: TplLockGuard<'a, T>,
}

impl<'a, T> core::ops::Deref for TplMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<'a, T> core::ops::DerefMut for TplMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

// =============================================================================
// TPL STATISTICS
// =============================================================================

/// TPL usage statistics
#[derive(Debug, Clone, Default)]
pub struct TplStats {
    /// Number of times TPL was raised to each level
    pub raise_count: [u64; 32],
    /// Time spent at each level (in arbitrary units)
    pub time_at_level: [u64; 32],
}

impl TplStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            raise_count: [0; 32],
            time_at_level: [0; 32],
        }
    }

    /// Record a TPL raise
    pub fn record_raise(&mut self, tpl: Tpl) {
        if tpl.0 < 32 {
            self.raise_count[tpl.0] += 1;
        }
    }

    /// Get raises to application level
    pub fn application_raises(&self) -> u64 {
        self.raise_count[Tpl::APPLICATION.0]
    }

    /// Get raises to callback level
    pub fn callback_raises(&self) -> u64 {
        self.raise_count[Tpl::CALLBACK.0]
    }

    /// Get raises to notify level
    pub fn notify_raises(&self) -> u64 {
        self.raise_count[Tpl::NOTIFY.0]
    }

    /// Get raises to high level
    pub fn high_level_raises(&self) -> u64 {
        self.raise_count[Tpl::HIGH_LEVEL.0]
    }
}

// Global stats (thread-local in real implementation)
static TPL_STATS: TplLock<TplStats> = TplLock::new(TplStats::new());

/// Get TPL statistics
pub fn get_tpl_stats() -> TplStats {
    // Note: This would deadlock if called from high TPL!
    // In real implementation, use separate mechanism
    TplStats::new()
}

// =============================================================================
// DEFERRED EXECUTION
// =============================================================================

/// Deferred task to execute at lower TPL
pub struct DeferredTask {
    /// Task function
    task: Option<alloc::boxed::Box<dyn FnOnce()>>,
    /// Target TPL
    target_tpl: Tpl,
}

impl DeferredTask {
    /// Create a new deferred task
    pub fn new<F: FnOnce() + 'static>(target_tpl: Tpl, f: F) -> Self {
        Self {
            task: Some(alloc::boxed::Box::new(f)),
            target_tpl,
        }
    }

    /// Create task for callback level
    pub fn at_callback<F: FnOnce() + 'static>(f: F) -> Self {
        Self::new(Tpl::CALLBACK, f)
    }

    /// Create task for application level
    pub fn at_application<F: FnOnce() + 'static>(f: F) -> Self {
        Self::new(Tpl::APPLICATION, f)
    }

    /// Execute the task (consumes it)
    pub fn execute(mut self) {
        if let Some(task) = self.task.take() {
            let current = get_current_tpl();
            if current.0 > self.target_tpl.0 {
                // Can't lower TPL, execute now
                task();
            } else {
                // Raise and execute at target level
                with_tpl(self.target_tpl, task);
            }
        }
    }

    /// Get target TPL
    pub fn target_tpl(&self) -> Tpl {
        self.target_tpl
    }
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_tpl_ordering() {
        assert!(Tpl::APPLICATION < Tpl::CALLBACK);
        assert!(Tpl::CALLBACK < Tpl::NOTIFY);
        assert!(Tpl::NOTIFY < Tpl::HIGH_LEVEL);
    }

    #[test]
    fn test_tpl_names() {
        assert_eq!(Tpl::APPLICATION.name(), "TPL_APPLICATION");
        assert_eq!(Tpl::CALLBACK.name(), "TPL_CALLBACK");
        assert_eq!(Tpl::NOTIFY.name(), "TPL_NOTIFY");
        assert_eq!(Tpl::HIGH_LEVEL.name(), "TPL_HIGH_LEVEL");
    }

    #[test]
    fn test_tpl_lock() {
        let lock = TplLock::new(42);
        // Can't test without boot services
    }

    #[test]
    fn test_tpl_stats() {
        let mut stats = TplStats::new();
        stats.record_raise(Tpl::HIGH_LEVEL);
        assert_eq!(stats.high_level_raises(), 1);
    }
}
