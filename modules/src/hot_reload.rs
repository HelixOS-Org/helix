//! # Hot Reload Engine
//!
//! Enables replacing modules at runtime without system restart.

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::any::Any;

use spin::RwLock;

use crate::registry::ModuleRegistry;
use crate::{Module, ModuleError, ModuleFlags, ModuleId, ModuleResult, ModuleState};

/// Hot reload state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadState {
    /// Not reloading
    Idle,
    /// Preparing to reload
    Preparing,
    /// Saving state
    SavingState,
    /// Unloading old module
    Unloading,
    /// Loading new module
    Loading,
    /// Restoring state
    RestoringState,
    /// Reload completed
    Completed,
    /// Reload failed
    Failed,
}

/// Hot reload result
pub struct ReloadResult {
    /// Was the reload successful?
    pub success: bool,
    /// Old module (if rollback needed)
    pub old_module: Option<Box<dyn Any + Send + Sync>>,
    /// Error message (if failed)
    pub error: Option<ModuleError>,
}

/// Hot reload engine
pub struct HotReloadEngine {
    /// Current reload state
    state: RwLock<ReloadState>,
    /// Module being reloaded
    current_module: RwLock<Option<ModuleId>>,
    /// Saved state during reload
    saved_state: RwLock<Option<Box<dyn Any + Send + Sync>>>,
}

impl Default for HotReloadEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HotReloadEngine {
    /// Create a new hot reload engine
    pub const fn new() -> Self {
        Self {
            state: RwLock::new(ReloadState::Idle),
            current_module: RwLock::new(None),
            saved_state: RwLock::new(None),
        }
    }

    /// Get current reload state
    pub fn state(&self) -> ReloadState {
        *self.state.read()
    }

    /// Check if a module can be hot-reloaded
    pub fn can_reload(&self, registry: &ModuleRegistry, id: ModuleId) -> ModuleResult<()> {
        // Check state
        if self.state() != ReloadState::Idle {
            return Err(ModuleError::WrongState {
                current: ModuleState::Loading,
                required: ModuleState::Running,
            });
        }

        // Check module exists and supports hot reload
        let metadata = registry.get(id).ok_or(ModuleError::NotFound)?;

        if !metadata.flags.contains(ModuleFlags::HOT_RELOADABLE) {
            return Err(ModuleError::Internal(
                "Module does not support hot reload".into(),
            ));
        }

        // Check module is running
        let state = registry.get_state(id).ok_or(ModuleError::NotFound)?;

        if state != ModuleState::Running {
            return Err(ModuleError::WrongState {
                current: state,
                required: ModuleState::Running,
            });
        }

        Ok(())
    }

    /// Start hot reload process
    pub fn begin_reload(&self, id: ModuleId) -> ModuleResult<()> {
        let mut state = self.state.write();
        if *state != ReloadState::Idle {
            return Err(ModuleError::WrongState {
                current: ModuleState::Loading,
                required: ModuleState::Running,
            });
        }

        *state = ReloadState::Preparing;
        *self.current_module.write() = Some(id);

        log::info!("Beginning hot reload for module {:?}", id);

        Ok(())
    }

    /// Save module state
    pub fn save_state(&self, module: &dyn Module) -> ModuleResult<()> {
        *self.state.write() = ReloadState::SavingState;

        if let Some(state) = module.get_state() {
            *self.saved_state.write() = Some(state);
            log::debug!("Module state saved");
        } else {
            log::debug!("Module has no state to save");
        }

        Ok(())
    }

    /// Get saved state
    pub fn take_saved_state(&self) -> Option<Box<dyn Any + Send + Sync>> {
        self.saved_state.write().take()
    }

    /// Complete reload
    pub fn complete_reload(&self) -> ModuleResult<()> {
        *self.state.write() = ReloadState::Completed;
        *self.current_module.write() = None;
        *self.saved_state.write() = None;

        log::info!("Hot reload completed successfully");

        // Reset to idle after completion
        *self.state.write() = ReloadState::Idle;

        Ok(())
    }

    /// Fail reload and initiate rollback
    pub fn fail_reload(&self, error: ModuleError) -> ModuleError {
        log::error!("Hot reload failed: {:?}", error);

        *self.state.write() = ReloadState::Failed;

        // TODO: Implement rollback

        *self.state.write() = ReloadState::Idle;
        *self.current_module.write() = None;
        *self.saved_state.write() = None;

        error
    }

    /// Perform a full hot reload
    pub fn reload<F>(
        &self,
        registry: &ModuleRegistry,
        id: ModuleId,
        _new_binary: &[u8],
        get_module: F,
    ) -> ReloadResult
    where
        F: FnOnce(ModuleId) -> Option<Arc<RwLock<dyn Module>>>,
    {
        // Step 1: Validate
        if let Err(e) = self.can_reload(registry, id) {
            return ReloadResult {
                success: false,
                old_module: None,
                error: Some(e),
            };
        }

        // Step 2: Begin
        if let Err(e) = self.begin_reload(id) {
            return ReloadResult {
                success: false,
                old_module: None,
                error: Some(e),
            };
        }

        // Step 3: Get current module and save state
        let module = match get_module(id) {
            Some(m) => m,
            None => {
                return ReloadResult {
                    success: false,
                    old_module: None,
                    error: Some(ModuleError::NotFound),
                }
            },
        };

        if let Err(e) = self.save_state(&*module.read()) {
            return ReloadResult {
                success: false,
                old_module: None,
                error: Some(self.fail_reload(e)),
            };
        }

        // Step 4: Load new module
        *self.state.write() = ReloadState::Loading;

        // TODO: Actually load the new module from binary
        // For now, just demonstrate the framework

        // Step 5: Restore state
        *self.state.write() = ReloadState::RestoringState;

        if let Some(state) = self.take_saved_state() {
            // TODO: Pass state to new module
            let _ = state;
        }

        // Step 6: Complete
        if let Err(e) = self.complete_reload() {
            return ReloadResult {
                success: false,
                old_module: None,
                error: Some(e),
            };
        }

        ReloadResult {
            success: true,
            old_module: None,
            error: None,
        }
    }
}

/// Global hot reload engine
static ENGINE: HotReloadEngine = HotReloadEngine::new();

/// Get the hot reload engine
pub fn engine() -> &'static HotReloadEngine {
    &ENGINE
}
