//! Lifecycle Traits
//!
//! Traits for component lifecycle management: tick-based updates, pausing, and configuration.

#![allow(dead_code)]

use super::decider::ValidationResult;
use crate::types::{Duration, NexusResult, Timestamp};

// ============================================================================
// TICKABLE TRAIT
// ============================================================================

/// Trait for components with tick-based lifecycle
pub trait Tickable {
    /// Process a single tick
    fn tick(&mut self, now: Timestamp) -> NexusResult<()>;

    /// Get tick interval
    fn tick_interval(&self) -> Duration;

    /// Set tick interval
    fn set_tick_interval(&mut self, interval: Duration);

    /// Get last tick timestamp
    fn last_tick(&self) -> Timestamp;

    /// Get tick count
    fn tick_count(&self) -> u64;
}

// ============================================================================
// PAUSABLE TRAIT
// ============================================================================

/// Trait for pausable components
pub trait Pausable {
    /// Pause the component
    fn pause(&mut self) -> NexusResult<()>;

    /// Resume the component
    fn resume(&mut self) -> NexusResult<()>;

    /// Is paused?
    fn is_paused(&self) -> bool;

    /// Toggle pause state
    fn toggle_pause(&mut self) -> NexusResult<bool> {
        if self.is_paused() {
            self.resume()?;
            Ok(false)
        } else {
            self.pause()?;
            Ok(true)
        }
    }
}

// ============================================================================
// CONFIGURABLE TRAIT
// ============================================================================

/// Trait for configurable components
pub trait Configurable {
    /// Configuration type
    type Config;

    /// Get current configuration
    fn config(&self) -> &Self::Config;

    /// Apply new configuration
    fn apply_config(&mut self, config: Self::Config) -> NexusResult<()>;

    /// Validate configuration before applying
    fn validate_config(&self, config: &Self::Config) -> ValidationResult;

    /// Reload configuration from source
    fn reload_config(&mut self) -> NexusResult<()>;

    /// Get default configuration
    fn default_config() -> Self::Config
    where
        Self::Config: Default,
    {
        Self::Config::default()
    }
}

// ============================================================================
// RESETTABLE TRAIT
// ============================================================================

/// Trait for resettable components
pub trait Resettable {
    /// Reset to initial state
    fn reset(&mut self) -> NexusResult<()>;

    /// Reset statistics only
    fn reset_stats(&mut self) -> NexusResult<()>;

    /// Is in initial state?
    fn is_initial(&self) -> bool;
}

// ============================================================================
// OBSERVABLE TRAIT
// ============================================================================

/// Trait for observable components (event emission)
pub trait Observable {
    /// Event type
    type Event;

    /// Register an observer
    fn register_observer(&mut self, observer: alloc::boxed::Box<dyn Observer<Event = Self::Event>>);

    /// Unregister all observers
    fn unregister_all(&mut self);

    /// Notify observers of an event
    fn notify(&self, event: &Self::Event);

    /// Get observer count
    fn observer_count(&self) -> usize;
}

/// Observer trait
pub trait Observer: Send + Sync {
    /// Event type
    type Event;

    /// Handle event
    fn on_event(&self, event: &Self::Event);
}

// ============================================================================
// STARTABLE TRAIT
// ============================================================================

/// Trait for startable/stoppable components
pub trait Startable {
    /// Start the component
    fn start(&mut self) -> NexusResult<()>;

    /// Stop the component
    fn stop(&mut self) -> NexusResult<()>;

    /// Is running?
    fn is_running(&self) -> bool;

    /// Restart (stop then start)
    fn restart(&mut self) -> NexusResult<()> {
        if self.is_running() {
            self.stop()?;
        }
        self.start()
    }
}

// ============================================================================
// GRACEFUL SHUTDOWN TRAIT
// ============================================================================

/// Trait for graceful shutdown
pub trait GracefulShutdown {
    /// Initiate graceful shutdown
    fn shutdown(&mut self) -> NexusResult<()>;

    /// Wait for shutdown to complete with timeout
    fn wait_shutdown(&self, timeout: Duration) -> NexusResult<bool>;

    /// Force immediate shutdown
    fn force_shutdown(&mut self) -> NexusResult<()>;

    /// Is shutdown in progress?
    fn is_shutting_down(&self) -> bool;

    /// Is fully shutdown?
    fn is_shutdown(&self) -> bool;
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing
    struct MockPausable {
        paused: bool,
    }

    impl Pausable for MockPausable {
        fn pause(&mut self) -> NexusResult<()> {
            self.paused = true;
            Ok(())
        }

        fn resume(&mut self) -> NexusResult<()> {
            self.paused = false;
            Ok(())
        }

        fn is_paused(&self) -> bool {
            self.paused
        }
    }

    #[test]
    fn test_pausable_toggle() {
        let mut mock = MockPausable { paused: false };

        assert!(!mock.is_paused());

        let result = mock.toggle_pause().unwrap();
        assert!(result); // Now paused
        assert!(mock.is_paused());

        let result = mock.toggle_pause().unwrap();
        assert!(!result); // Now resumed
        assert!(!mock.is_paused());
    }
}
