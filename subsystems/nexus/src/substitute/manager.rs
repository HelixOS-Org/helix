//! Substitution manager for hot module replacement.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::NexusTimestamp;
use crate::error::{HealingError, NexusResult};

use super::info::ModuleInfo;
use super::slot::ModuleSlot;

/// Result of a substitution
#[derive(Debug, Clone)]
pub struct SubstitutionResult {
    /// Slot name
    pub slot: String,
    /// Old module
    pub old_module: Option<ModuleInfo>,
    /// New module
    pub new_module: ModuleInfo,
    /// State transfer success
    pub state_transferred: bool,
    /// Verification passed
    pub verified: bool,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Duration (cycles)
    pub duration: u64,
}

/// Manager for module substitution
pub struct SubstitutionManager {
    /// Slots by name
    slots: BTreeMap<String, ModuleSlot>,
    /// Substitution history
    history: Vec<SubstitutionResult>,
    /// Maximum history
    max_history: usize,
    /// State transfer handlers
    /// In real implementation, these would handle actual state transfer
    /// Total substitutions
    total_substitutions: AtomicU64,
    /// Successful substitutions
    successful_substitutions: AtomicU64,
    /// Is manager enabled?
    enabled: AtomicBool,
}

impl SubstitutionManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            history: Vec::new(),
            max_history: 1000,
            total_substitutions: AtomicU64::new(0),
            successful_substitutions: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register a slot
    pub fn register_slot(&mut self, slot: ModuleSlot) {
        self.slots.insert(slot.name.clone(), slot);
    }

    /// Get slot by name
    pub fn get_slot(&self, name: &str) -> Option<&ModuleSlot> {
        self.slots.get(name)
    }

    /// Get mutable slot
    pub fn get_slot_mut(&mut self, name: &str) -> Option<&mut ModuleSlot> {
        self.slots.get_mut(name)
    }

    /// List all slots
    pub fn slots(&self) -> impl Iterator<Item = &ModuleSlot> {
        self.slots.values()
    }

    /// Substitute module in slot
    pub fn substitute(
        &mut self,
        slot_name: &str,
        new_module: ModuleInfo,
    ) -> NexusResult<SubstitutionResult> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err(HealingError::SubstitutionFailed.into());
        }

        let slot = self
            .slots
            .get_mut(slot_name)
            .ok_or(HealingError::SubstitutionFailed)?;

        if !slot.is_enabled() {
            return Err(HealingError::SubstitutionFailed.into());
        }

        let start = NexusTimestamp::now();
        self.total_substitutions.fetch_add(1, Ordering::Relaxed);

        // Check compatibility if there's a current module
        let old_module = slot.current.clone();
        if let Some(ref current) = old_module {
            if !current.is_compatible_with(&new_module) && !new_module.is_fallback {
                // Incompatible, but allow if it's a fallback
                return Err(HealingError::SubstitutionFailed.into());
            }
        }

        // Perform substitution
        // In real implementation:
        // 1. Freeze current module
        // 2. Extract state
        // 3. Load new module
        // 4. Transfer state
        // 5. Verify
        // 6. Activate new module
        // 7. Unload old module

        slot.current = Some(new_module.clone());
        slot.record_substitution();

        let end = NexusTimestamp::now();

        self.successful_substitutions
            .fetch_add(1, Ordering::Relaxed);

        let result = SubstitutionResult {
            slot: slot_name.into(),
            old_module,
            new_module,
            state_transferred: true,
            verified: true,
            timestamp: start,
            duration: end.duration_since(start),
        };

        // Add to history
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(result.clone());

        Ok(result)
    }

    /// Substitute with fallback
    pub fn substitute_with_fallback(&mut self, slot_name: &str) -> NexusResult<SubstitutionResult> {
        let slot = self
            .slots
            .get_mut(slot_name)
            .ok_or(HealingError::SubstitutionFailed)?;

        let fallback = slot
            .pop_fallback()
            .ok_or(HealingError::SubstitutionFailed)?;

        // Re-borrow for substitute
        self.substitute(slot_name, fallback)
    }

    /// Check if substitution is possible
    pub fn can_substitute(&self, slot_name: &str) -> bool {
        self.slots
            .get(slot_name)
            .map(|s| s.is_enabled() && s.has_fallbacks())
            .unwrap_or(false)
    }

    /// Get substitution history
    pub fn history(&self) -> &[SubstitutionResult] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> SubstitutionStats {
        let total = self.total_substitutions.load(Ordering::Relaxed);
        let successful = self.successful_substitutions.load(Ordering::Relaxed);

        SubstitutionStats {
            total_slots: self.slots.len(),
            enabled_slots: self.slots.values().filter(|s| s.is_enabled()).count(),
            total_substitutions: total,
            successful_substitutions: successful,
            failed_substitutions: total - successful,
            slots_with_fallbacks: self.slots.values().filter(|s| s.has_fallbacks()).count(),
        }
    }

    /// Enable manager
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable manager
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Is enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

impl Default for SubstitutionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Substitution statistics
#[derive(Debug, Clone)]
pub struct SubstitutionStats {
    /// Total slots
    pub total_slots: usize,
    /// Enabled slots
    pub enabled_slots: usize,
    /// Total substitutions
    pub total_substitutions: u64,
    /// Successful substitutions
    pub successful_substitutions: u64,
    /// Failed substitutions
    pub failed_substitutions: u64,
    /// Slots with fallbacks
    pub slots_with_fallbacks: usize,
}
