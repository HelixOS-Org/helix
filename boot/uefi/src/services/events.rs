//! Event and Timer Services
//!
//! Safe wrappers for UEFI event and timer management.

use super::boot::{boot_services, EventType, TPL_CALLBACK};
use crate::raw::types::*;

// =============================================================================
// EVENT WRAPPER
// =============================================================================

/// Safe event wrapper
pub struct EventGuard {
    event: Event,
    auto_close: bool,
}

impl EventGuard {
    /// Create a new event guard
    ///
    /// # Safety
    /// Event must be valid.
    pub unsafe fn new(event: Event, auto_close: bool) -> Self {
        Self { event, auto_close }
    }

    /// Get the raw event handle
    pub fn handle(&self) -> Event {
        self.event
    }

    /// Take ownership of the event (won't auto-close)
    pub fn take(mut self) -> Event {
        self.auto_close = false;
        self.event
    }

    /// Signal the event
    pub fn signal(&self) -> Result<(), Status> {
        unsafe { boot_services().signal_event(self.event) }
    }

    /// Check if event is signaled
    pub fn check(&self) -> Result<bool, Status> {
        unsafe { boot_services().check_event(self.event) }
    }

    /// Close the event
    pub fn close(mut self) -> Result<(), Status> {
        self.auto_close = false;
        unsafe { boot_services().close_event(self.event) }
    }
}

impl Drop for EventGuard {
    fn drop(&mut self) {
        if self.auto_close && !self.event.is_null() {
            unsafe {
                let _ = boot_services().close_event(self.event);
            }
        }
    }
}

// =============================================================================
// TIMER
// =============================================================================

/// Timer wrapper
pub struct Timer {
    event: EventGuard,
}

impl Timer {
    /// Create a new timer
    pub fn new() -> Result<Self, Status> {
        let bs = unsafe { boot_services() };
        let event = bs.create_timer_event()?;

        Ok(Self {
            event: unsafe { EventGuard::new(event, true) },
        })
    }

    /// Get the event handle
    pub fn event(&self) -> Event {
        self.event.handle()
    }

    /// Set timer to trigger periodically
    pub fn set_periodic(&self, interval_100ns: u64) -> Result<(), Status> {
        let bs = unsafe { boot_services() };
        bs.set_timer(self.event.handle(), TimerDelay::Periodic, interval_100ns)
    }

    /// Set timer to trigger once
    pub fn set_relative(&self, delay_100ns: u64) -> Result<(), Status> {
        let bs = unsafe { boot_services() };
        bs.set_timer(self.event.handle(), TimerDelay::Relative, delay_100ns)
    }

    /// Cancel timer
    pub fn cancel(&self) -> Result<(), Status> {
        let bs = unsafe { boot_services() };
        bs.set_timer(self.event.handle(), TimerDelay::Cancel, 0)
    }

    /// Check if timer has triggered
    pub fn check(&self) -> Result<bool, Status> {
        self.event.check()
    }

    /// Wait for timer to trigger
    pub fn wait(&self) -> Result<(), Status> {
        let bs = unsafe { boot_services() };
        let events = [self.event.handle()];
        bs.wait_for_event(&events)?;
        Ok(())
    }
}

// =============================================================================
// TIMEOUT HELPER
// =============================================================================

/// Timeout helper
pub struct Timeout {
    timer: Timer,
    expired: bool,
}

impl Timeout {
    /// Create a new timeout
    pub fn new(milliseconds: u64) -> Result<Self, Status> {
        let timer = Timer::new()?;
        // Convert milliseconds to 100ns units
        timer.set_relative(milliseconds * 10_000)?;

        Ok(Self {
            timer,
            expired: false,
        })
    }

    /// Check if timeout has expired
    pub fn is_expired(&mut self) -> Result<bool, Status> {
        if self.expired {
            return Ok(true);
        }

        self.expired = self.timer.check()?;
        Ok(self.expired)
    }

    /// Reset the timeout
    pub fn reset(&mut self, milliseconds: u64) -> Result<(), Status> {
        self.expired = false;
        self.timer.set_relative(milliseconds * 10_000)
    }
}

// =============================================================================
// PERIODIC CALLBACK
// =============================================================================

/// Periodic callback wrapper
pub struct PeriodicCallback {
    event: Event,
    active: bool,
}

impl PeriodicCallback {
    /// Create a new periodic callback
    ///
    /// # Safety
    /// The callback function must be safe to call from UEFI context.
    pub unsafe fn new<F>(interval_ms: u64, callback: F) -> Result<Self, Status>
    where
        F: FnMut() + 'static,
    {
        // Box the callback
        let callback_box = Box::new(callback);
        let callback_ptr = Box::into_raw(callback_box);

        let bs = boot_services();
        let event = bs.create_event(
            EventType::TIMER | EventType::NOTIFY_SIGNAL,
            TPL_CALLBACK,
            Some(periodic_callback_handler::<F>),
            callback_ptr as *mut core::ffi::c_void,
        )?;

        // Set the timer
        bs.set_timer(event, TimerDelay::Periodic, interval_ms * 10_000)?;

        Ok(Self {
            event,
            active: true,
        })
    }

    /// Stop the periodic callback
    pub fn stop(&mut self) -> Result<(), Status> {
        if !self.active {
            return Ok(());
        }

        let bs = unsafe { boot_services() };
        bs.set_timer(self.event, TimerDelay::Cancel, 0)?;
        self.active = false;

        Ok(())
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for PeriodicCallback {
    fn drop(&mut self) {
        let _ = self.stop();
        if !self.event.is_null() {
            unsafe {
                let _ = boot_services().close_event(self.event);
            }
        }
    }
}

/// Handler for periodic callbacks
unsafe extern "efiapi" fn periodic_callback_handler<F: FnMut()>(
    _event: Event,
    context: *mut core::ffi::c_void,
) {
    if !context.is_null() {
        let callback = &mut *(context as *mut F);
        callback();
    }
}

// =============================================================================
// WAIT GROUP
// =============================================================================

/// Wait for multiple events
pub struct WaitGroup {
    events: alloc::vec::Vec<Event>,
}

impl WaitGroup {
    /// Create a new wait group
    pub fn new() -> Self {
        Self {
            events: alloc::vec::Vec::new(),
        }
    }

    /// Add an event to the group
    pub fn add(&mut self, event: Event) {
        self.events.push(event);
    }

    /// Add a timer event
    pub fn add_timer(&mut self, timer: &Timer) {
        self.events.push(timer.event());
    }

    /// Wait for any event to signal
    pub fn wait_any(&self) -> Result<usize, Status> {
        if self.events.is_empty() {
            return Err(Status::INVALID_PARAMETER);
        }

        let bs = unsafe { boot_services() };
        bs.wait_for_event(&self.events)
    }

    /// Check which events are signaled
    pub fn check_all(&self) -> Result<alloc::vec::Vec<bool>, Status> {
        let bs = unsafe { boot_services() };
        let mut results = alloc::vec::Vec::with_capacity(self.events.len());

        for &event in &self.events {
            results.push(bs.check_event(event)?);
        }

        Ok(results)
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for WaitGroup {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DELAY FUNCTIONS
// =============================================================================

/// Sleep for specified milliseconds
pub fn sleep_ms(milliseconds: u64) -> Result<(), Status> {
    let bs = unsafe { boot_services() };
    bs.stall((milliseconds * 1000) as usize)
}

/// Sleep for specified microseconds
pub fn sleep_us(microseconds: usize) -> Result<(), Status> {
    let bs = unsafe { boot_services() };
    bs.stall(microseconds)
}

/// Busy-wait for specified iterations
pub fn busy_wait(iterations: u64) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

// =============================================================================
// ONE-SHOT EVENT
// =============================================================================

/// Create a one-shot notification event
///
/// # Safety
/// The callback function must be safe to call from UEFI context.
pub unsafe fn create_notification_event<F>(callback: F) -> Result<EventGuard, Status>
where
    F: FnOnce() + 'static,
{
    let callback_box = Box::new(Some(callback));
    let callback_ptr = Box::into_raw(callback_box);

    let bs = boot_services();
    let event = bs.create_event(
        EventType::NOTIFY_SIGNAL,
        TPL_CALLBACK,
        Some(oneshot_callback_handler::<F>),
        callback_ptr as *mut core::ffi::c_void,
    )?;

    Ok(EventGuard::new(event, true))
}

/// Handler for one-shot callbacks
unsafe extern "efiapi" fn oneshot_callback_handler<F: FnOnce()>(
    _event: Event,
    context: *mut core::ffi::c_void,
) {
    if !context.is_null() {
        let callback_box = Box::from_raw(context as *mut Option<F>);
        if let Some(callback) = *callback_box {
            callback();
        }
    }
}

// =============================================================================
// EXIT BOOT SERVICES EVENT
// =============================================================================

/// Register a callback for ExitBootServices
///
/// # Safety
/// The callback will be called during ExitBootServices.
pub unsafe fn on_exit_boot_services<F>(callback: F) -> Result<EventGuard, Status>
where
    F: FnOnce() + 'static,
{
    let callback_box = Box::new(Some(callback));
    let callback_ptr = Box::into_raw(callback_box);

    let bs = boot_services();
    let event = bs.create_event(
        EventType::SIGNAL_EXIT_BOOT_SERVICES,
        TPL_CALLBACK,
        Some(oneshot_callback_handler::<F>),
        callback_ptr as *mut core::ffi::c_void,
    )?;

    Ok(EventGuard::new(event, true))
}

/// Register a callback for virtual address change
///
/// # Safety
/// The callback will be called during SetVirtualAddressMap.
pub unsafe fn on_virtual_address_change<F>(callback: F) -> Result<EventGuard, Status>
where
    F: FnOnce() + 'static,
{
    let callback_box = Box::new(Some(callback));
    let callback_ptr = Box::into_raw(callback_box);

    let bs = boot_services();
    let event = bs.create_event(
        EventType::SIGNAL_VIRTUAL_ADDRESS_CHANGE,
        TPL_CALLBACK,
        Some(oneshot_callback_handler::<F>),
        callback_ptr as *mut core::ffi::c_void,
    )?;

    Ok(EventGuard::new(event, true))
}

// =============================================================================
// EXTERN ALLOC
// =============================================================================

extern crate alloc;
use alloc::boxed::Box;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_group() {
        let mut wg = WaitGroup::new();
        assert!(wg.is_empty());
        assert_eq!(wg.len(), 0);
    }
}
