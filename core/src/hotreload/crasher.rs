//! # Crasher Module - Test module that crashes on purpose
//!
//! This module is used to demonstrate the self-healing capabilities.
//! It will crash after a configurable number of operations.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;
use core::sync::atomic::{AtomicU32, Ordering};

use super::{HotReloadError, HotReloadableModule, ModuleCategory, ModuleState, ModuleVersion};

/// State for the crasher module
#[derive(Debug, Clone)]
pub struct CrasherState {
    /// How many operations performed
    pub operations_count: u32,
    /// How many crashes simulated
    pub crashes_count: u32,
}

impl ModuleState for CrasherState {
    fn export(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.operations_count.to_le_bytes());
        data.extend_from_slice(&self.crashes_count.to_le_bytes());
        data
    }

    fn version(&self) -> u32 {
        1
    }
}

/// A module that crashes after a certain number of operations
pub struct CrasherModule {
    /// Name of this instance
    _name: String,
    /// Crash after this many operations
    crash_threshold: u32,
    /// Current operation count
    operations: AtomicU32,
    /// Crash count
    crashes: AtomicU32,
    /// Whether this instance has crashed
    has_crashed: bool,
}

impl CrasherModule {
    /// Create a new crasher that will crash after `threshold` operations
    pub fn new(threshold: u32) -> Self {
        Self {
            _name: String::from("CrasherModule"),
            crash_threshold: threshold,
            operations: AtomicU32::new(0),
            crashes: AtomicU32::new(0),
            has_crashed: false,
        }
    }

    /// Factory function to create default crasher (crashes after 5 ops)
    pub fn factory() -> Box<dyn HotReloadableModule> {
        Box::new(Self::new(5))
    }

    /// Perform an operation. Returns Ok if successful, Err if crashed
    pub fn do_operation(&mut self) -> Result<u32, &'static str> {
        let count = self.operations.fetch_add(1, Ordering::SeqCst) + 1;

        log_crasher(&alloc::format!("[CRASHER] Operation #{}", count));

        if count >= self.crash_threshold {
            self.has_crashed = true;
            self.crashes.fetch_add(1, Ordering::SeqCst);
            log_crasher(&alloc::format!(
                "[CRASHER] 💥 CRASH! Threshold {} reached!",
                self.crash_threshold
            ));
            return Err("Module crashed!");
        }

        Ok(count)
    }

    /// Check if the module has crashed
    pub fn has_crashed(&self) -> bool {
        self.has_crashed
    }

    /// Get operation count
    pub fn operations(&self) -> u32 {
        self.operations.load(Ordering::SeqCst)
    }

    /// Get crash count
    pub fn crashes(&self) -> u32 {
        self.crashes.load(Ordering::SeqCst)
    }

    /// Reset crash state (for recovery)
    pub fn reset(&mut self) {
        self.has_crashed = false;
        self.operations.store(0, Ordering::SeqCst);
    }
}

impl HotReloadableModule for CrasherModule {
    fn name(&self) -> &'static str {
        "CrasherModule"
    }

    fn version(&self) -> ModuleVersion {
        ModuleVersion::new(1, 0, 0)
    }

    fn category(&self) -> ModuleCategory {
        ModuleCategory::Custom
    }

    fn init(&mut self) -> Result<(), HotReloadError> {
        log_crasher("[CRASHER] Initialized (will crash after operations)");
        self.has_crashed = false;
        self.operations.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn prepare_unload(&mut self) -> Result<(), HotReloadError> {
        log_crasher("[CRASHER] Preparing for unload");
        Ok(())
    }

    fn export_state(&self) -> Option<Box<dyn ModuleState>> {
        let state = CrasherState {
            operations_count: self.operations.load(Ordering::SeqCst),
            crashes_count: self.crashes.load(Ordering::SeqCst),
        };
        Some(Box::new(state))
    }

    fn import_state(&mut self, state: &dyn ModuleState) -> Result<(), HotReloadError> {
        let data = state.export();
        if data.len() < 8 {
            return Err(HotReloadError::StateMigrationFailed);
        }
        let crashes_count = u32::from_le_bytes(data[4..8].try_into().unwrap());
        // Import crash count but reset operation counter
        self.crashes.store(crashes_count, Ordering::SeqCst);
        self.operations.store(0, Ordering::SeqCst);
        self.has_crashed = false;
        log_crasher(&alloc::format!(
            "[CRASHER] State imported: {} previous crashes, resetting ops counter",
            crashes_count
        ));
        Ok(())
    }

    fn can_unload(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn log_crasher(msg: &str) {
    for &c in msg.as_bytes() {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") c,
                options(nomem, nostack)
            );
        }
    }
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3F8u16,
            in("al") b'\n',
            options(nomem, nostack)
        );
    }
}
