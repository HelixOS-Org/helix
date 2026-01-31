//! Fallback module registry.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::ComponentId;

use super::info::ModuleInfo;

/// Registry of fallback modules
pub struct FallbackRegistry {
    /// Fallbacks by component type
    fallbacks: BTreeMap<u64, Vec<ModuleInfo>>,
}

impl FallbackRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            fallbacks: BTreeMap::new(),
        }
    }

    /// Register a fallback
    pub fn register(&mut self, module: ModuleInfo) {
        let comp = module.component.raw();
        self.fallbacks.entry(comp).or_default().push(module);
    }

    /// Get fallbacks for component
    pub fn get_fallbacks(&self, component: ComponentId) -> Vec<&ModuleInfo> {
        self.fallbacks
            .get(&component.raw())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get best fallback (highest version)
    pub fn best_fallback(&self, component: ComponentId) -> Option<&ModuleInfo> {
        self.fallbacks
            .get(&component.raw())
            .and_then(|v| v.iter().max_by_key(|m| m.version))
    }

    /// Remove fallback
    pub fn remove(&mut self, component: ComponentId, module_id: u64) {
        if let Some(fallbacks) = self.fallbacks.get_mut(&component.raw()) {
            fallbacks.retain(|m| m.id != module_id);
        }
    }

    /// Count fallbacks
    pub fn count(&self) -> usize {
        self.fallbacks.values().map(|v| v.len()).sum()
    }
}

impl Default for FallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}
