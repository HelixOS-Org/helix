//! Module slot for hot substitution.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};

use super::info::ModuleInfo;

/// A slot that can hold a module
pub struct ModuleSlot {
    /// Slot name
    pub name: String,
    /// Component type
    pub component: ComponentId,
    /// Current module
    pub(crate) current: Option<ModuleInfo>,
    /// Fallback modules (priority ordered)
    fallbacks: Vec<ModuleInfo>,
    /// Is slot enabled?
    enabled: AtomicBool,
    /// Substitution count
    substitution_count: AtomicU64,
    /// Last substitution time
    last_substitution: Option<NexusTimestamp>,
}

impl ModuleSlot {
    /// Create a new slot
    pub fn new(name: impl Into<String>, component: ComponentId) -> Self {
        Self {
            name: name.into(),
            component,
            current: None,
            fallbacks: Vec::new(),
            enabled: AtomicBool::new(true),
            substitution_count: AtomicU64::new(0),
            last_substitution: None,
        }
    }

    /// Set current module
    pub fn set_current(&mut self, module: ModuleInfo) {
        self.current = Some(module);
    }

    /// Get current module
    pub fn current(&self) -> Option<&ModuleInfo> {
        self.current.as_ref()
    }

    /// Add fallback module
    pub fn add_fallback(&mut self, module: ModuleInfo) {
        self.fallbacks.push(module);
    }

    /// Get next fallback
    pub fn next_fallback(&self) -> Option<&ModuleInfo> {
        self.fallbacks.first()
    }

    /// Pop next fallback
    pub fn pop_fallback(&mut self) -> Option<ModuleInfo> {
        if self.fallbacks.is_empty() {
            None
        } else {
            Some(self.fallbacks.remove(0))
        }
    }

    /// Has fallbacks?
    pub fn has_fallbacks(&self) -> bool {
        !self.fallbacks.is_empty()
    }

    /// Get fallback count
    pub fn fallback_count(&self) -> usize {
        self.fallbacks.len()
    }

    /// Enable slot
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable slot
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Is enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Record substitution
    pub fn record_substitution(&mut self) {
        self.substitution_count.fetch_add(1, Ordering::Relaxed);
        self.last_substitution = Some(NexusTimestamp::now());
    }

    /// Get substitution count
    pub fn substitution_count(&self) -> u64 {
        self.substitution_count.load(Ordering::Relaxed)
    }
}
