//! # Lifecycle Manager
//!
//! Manages the kernel lifecycle: boot, shutdown, suspend, resume.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::{set_kernel_state, KernelError, KernelResult, KernelState};

/// Lifecycle stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleStage {
    /// Very early boot, before memory management
    EarlyBoot,
    /// Memory management initialized
    MemoryReady,
    /// Core subsystems initialized
    CoreReady,
    /// Modules loaded
    ModulesReady,
    /// Fully operational
    Running,
}

/// Lifecycle callback trait
pub trait LifecycleCallback: Send + Sync {
    /// Called when entering a lifecycle stage
    fn on_stage(&self, stage: LifecycleStage) -> KernelResult<()>;

    /// Called during shutdown
    fn on_shutdown(&self) -> KernelResult<()>;

    /// Called during suspend
    fn on_suspend(&self) -> KernelResult<()> {
        Ok(())
    }

    /// Called during resume
    fn on_resume(&self) -> KernelResult<()> {
        Ok(())
    }
}

/// Lifecycle manager
pub struct LifecycleManager {
    /// Current stage
    current_stage: spin::RwLock<LifecycleStage>,

    /// Registered callbacks
    callbacks: spin::RwLock<Vec<Box<dyn LifecycleCallback>>>,

    /// Is shutdown in progress?
    shutdown_in_progress: AtomicBool,
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub const fn new() -> Self {
        Self {
            current_stage: spin::RwLock::new(LifecycleStage::EarlyBoot),
            callbacks: spin::RwLock::new(Vec::new()),
            shutdown_in_progress: AtomicBool::new(false),
        }
    }

    /// Get the current lifecycle stage
    pub fn current_stage(&self) -> LifecycleStage {
        *self.current_stage.read()
    }

    /// Advance to the next lifecycle stage
    pub fn advance_to(&self, stage: LifecycleStage) -> KernelResult<()> {
        let mut current = self.current_stage.write();

        if stage <= *current {
            return Err(KernelError::InvalidArgument);
        }

        log::info!("Lifecycle: advancing to {:?}", stage);

        // Notify all callbacks
        let callbacks = self.callbacks.read();
        for callback in callbacks.iter() {
            callback.on_stage(stage)?;
        }

        *current = stage;

        if stage == LifecycleStage::Running {
            set_kernel_state(KernelState::Running);
        }

        Ok(())
    }

    /// Register a lifecycle callback
    pub fn register_callback(&self, callback: Box<dyn LifecycleCallback>) {
        self.callbacks.write().push(callback);
    }

    /// Initiate system shutdown
    pub fn shutdown(&self, reason: ShutdownReason) -> KernelResult<!> {
        if self.shutdown_in_progress.swap(true, Ordering::SeqCst) {
            // Shutdown already in progress
            loop {
                core::hint::spin_loop();
            }
        }

        set_kernel_state(KernelState::ShuttingDown);

        log::info!("System shutdown initiated: {:?}", reason);

        // Notify all callbacks in reverse order
        let callbacks = self.callbacks.read();
        for callback in callbacks.iter().rev() {
            if let Err(e) = callback.on_shutdown() {
                log::error!("Shutdown callback error: {:?}", e);
            }
        }

        // Final shutdown (arch-specific)
        match reason {
            ShutdownReason::Poweroff => {
                log::info!("Powering off...");
                // HAL shutdown
            },
            ShutdownReason::Reboot => {
                log::info!("Rebooting...");
                // HAL reboot
            },
            ShutdownReason::Halt => {
                log::info!("Halting...");
            },
            ShutdownReason::Panic => {
                log::error!("Panic shutdown");
            },
        }

        // This should not return
        loop {
            core::hint::spin_loop();
        }
    }

    /// Initiate system suspend
    pub fn suspend(&self) -> KernelResult<()> {
        log::info!("System suspending...");

        set_kernel_state(KernelState::Suspended);

        let callbacks = self.callbacks.read();
        for callback in callbacks.iter() {
            callback.on_suspend()?;
        }

        Ok(())
    }

    /// Resume from suspend
    pub fn resume(&self) -> KernelResult<()> {
        log::info!("System resuming...");

        let callbacks = self.callbacks.read();
        for callback in callbacks.iter().rev() {
            callback.on_resume()?;
        }

        set_kernel_state(KernelState::Running);

        Ok(())
    }
}

/// Reason for shutdown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    /// Normal poweroff
    Poweroff,
    /// Reboot
    Reboot,
    /// Halt (stop but don't power off)
    Halt,
    /// Shutdown due to panic
    Panic,
}
