//! Firmware Update Manager
//!
//! Firmware update management.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Firmware update state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    /// Idle
    Idle,
    /// Downloading update
    Downloading,
    /// Verifying update
    Verifying,
    /// Staging update
    Staging,
    /// Ready to apply
    Ready,
    /// Applying update
    Applying,
    /// Update complete (pending reboot)
    Complete,
    /// Update failed
    Failed,
}

/// Firmware update info
#[derive(Debug, Clone)]
pub struct FirmwareUpdate {
    /// Update ID
    pub id: u64,
    /// Component name
    pub component: String,
    /// Current version
    pub current_version: String,
    /// Target version
    pub target_version: String,
    /// Update size (bytes)
    pub size: u64,
    /// Current state
    pub state: UpdateState,
    /// Progress (0-100)
    pub progress: u8,
    /// Error message if failed
    pub error: Option<String>,
    /// Requires reboot
    pub requires_reboot: bool,
}

impl FirmwareUpdate {
    /// Create new update
    pub fn new(id: u64, component: String, current: String, target: String) -> Self {
        Self {
            id,
            component,
            current_version: current,
            target_version: target,
            size: 0,
            state: UpdateState::Idle,
            progress: 0,
            error: None,
            requires_reboot: true,
        }
    }
}

/// Firmware update manager
pub struct FirmwareUpdateManager {
    /// Pending updates
    pending_updates: Vec<FirmwareUpdate>,
    /// Active update
    active_update: Option<u64>,
    /// Completed updates
    completed_updates: Vec<FirmwareUpdate>,
    /// Failed updates
    failed_updates: Vec<FirmwareUpdate>,
    /// Next update ID
    next_id: AtomicU64,
    /// Updates allowed
    updates_allowed: bool,
}

impl FirmwareUpdateManager {
    /// Create new update manager
    pub fn new() -> Self {
        Self {
            pending_updates: Vec::new(),
            active_update: None,
            completed_updates: Vec::new(),
            failed_updates: Vec::new(),
            next_id: AtomicU64::new(1),
            updates_allowed: true,
        }
    }

    /// Queue update
    #[inline]
    pub fn queue_update(&mut self, component: String, current: String, target: String) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let update = FirmwareUpdate::new(id, component, current, target);
        self.pending_updates.push(update);
        id
    }

    /// Get update by ID
    #[inline]
    pub fn get_update(&self, id: u64) -> Option<&FirmwareUpdate> {
        self.pending_updates.iter().find(|u| u.id == id)
            .or_else(|| self.completed_updates.iter().find(|u| u.id == id))
            .or_else(|| self.failed_updates.iter().find(|u| u.id == id))
    }

    /// Get pending updates
    #[inline(always)]
    pub fn pending_updates(&self) -> &[FirmwareUpdate] {
        &self.pending_updates
    }

    /// Start update
    pub fn start_update(&mut self, id: u64) -> bool {
        if self.active_update.is_some() || !self.updates_allowed {
            return false;
        }

        if let Some(update) = self.pending_updates.iter_mut().find(|u| u.id == id) {
            update.state = UpdateState::Downloading;
            self.active_update = Some(id);
            return true;
        }

        false
    }

    /// Update progress
    #[inline]
    pub fn update_progress(&mut self, id: u64, progress: u8, state: UpdateState) {
        if let Some(update) = self.pending_updates.iter_mut().find(|u| u.id == id) {
            update.progress = progress.min(100);
            update.state = state;
        }
    }

    /// Complete update
    #[inline]
    pub fn complete_update(&mut self, id: u64) {
        if let Some(idx) = self.pending_updates.iter().position(|u| u.id == id) {
            let mut update = self.pending_updates.remove(idx);
            update.state = UpdateState::Complete;
            update.progress = 100;
            self.completed_updates.push(update);
            if self.active_update == Some(id) {
                self.active_update = None;
            }
        }
    }

    /// Fail update
    #[inline]
    pub fn fail_update(&mut self, id: u64, error: String) {
        if let Some(idx) = self.pending_updates.iter().position(|u| u.id == id) {
            let mut update = self.pending_updates.remove(idx);
            update.state = UpdateState::Failed;
            update.error = Some(error);
            self.failed_updates.push(update);
            if self.active_update == Some(id) {
                self.active_update = None;
            }
        }
    }

    /// Enable/disable updates
    #[inline(always)]
    pub fn set_updates_allowed(&mut self, allowed: bool) {
        self.updates_allowed = allowed;
    }

    /// Check if updates allowed
    #[inline(always)]
    pub fn updates_allowed(&self) -> bool {
        self.updates_allowed
    }

    /// Get active update ID
    #[inline(always)]
    pub fn active_update(&self) -> Option<u64> {
        self.active_update
    }
}

impl Default for FirmwareUpdateManager {
    fn default() -> Self {
        Self::new()
    }
}
