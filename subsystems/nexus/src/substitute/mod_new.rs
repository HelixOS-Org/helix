//! # Hot Module Substitution
//!
//! Replace failing modules at runtime without system restart.
//!
//! ## Key Features
//!
//! - **Live Replacement**: Replace modules while running
//! - **State Transfer**: Migrate state to new module
//! - **Fallback Management**: Manage backup implementations
//! - **Compatibility Verification**: Ensure ABI compatibility

#![allow(dead_code)]

extern crate alloc;

mod info;
mod manager;
mod registry;
mod slot;
mod version;

// Re-export version
pub use version::ModuleVersion;

// Re-export info
pub use info::ModuleInfo;

// Re-export slot
pub use slot::ModuleSlot;

// Re-export manager
pub use manager::{SubstitutionManager, SubstitutionResult, SubstitutionStats};

// Re-export registry
pub use registry::FallbackRegistry;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_module_version() {
        let v1 = ModuleVersion::new(1, 0, 0);
        let v2 = ModuleVersion::new(1, 1, 0);
        let v3 = ModuleVersion::new(2, 0, 0);

        assert!(v1.is_abi_compatible(&v2));
        assert!(!v1.is_abi_compatible(&v3));
        assert!(v2.is_newer_than(&v1));
    }

    #[test]
    fn test_module_info() {
        let module = ModuleInfo::new("test_module", ModuleVersion::new(1, 0, 0))
            .with_component(ComponentId::SCHEDULER)
            .with_abi_hash(12345)
            .with_capability("scheduling");

        assert_eq!(module.name, "test_module");
        assert_eq!(module.capabilities.len(), 1);
    }

    #[test]
    fn test_module_slot() {
        let mut slot = ModuleSlot::new("scheduler", ComponentId::SCHEDULER);

        let main = ModuleInfo::new("main_scheduler", ModuleVersion::new(1, 0, 0));
        let fallback =
            ModuleInfo::new("fallback_scheduler", ModuleVersion::new(1, 0, 0)).as_fallback();

        slot.set_current(main);
        slot.add_fallback(fallback);

        assert!(slot.current().is_some());
        assert!(slot.has_fallbacks());
    }

    #[test]
    fn test_substitution_manager() {
        let mut manager = SubstitutionManager::new();

        let mut slot = ModuleSlot::new("scheduler", ComponentId::SCHEDULER);
        let main = ModuleInfo::new("main", ModuleVersion::new(1, 0, 0)).with_abi_hash(123);
        slot.set_current(main);

        let fallback = ModuleInfo::new("fallback", ModuleVersion::new(1, 0, 0))
            .with_abi_hash(123)
            .as_fallback();
        slot.add_fallback(fallback);

        manager.register_slot(slot);

        assert!(manager.can_substitute("scheduler"));

        let result = manager.substitute_with_fallback("scheduler");
        assert!(result.is_ok());
    }

    #[test]
    fn test_fallback_registry() {
        let mut registry = FallbackRegistry::new();

        let fb1 = ModuleInfo::new("fb1", ModuleVersion::new(1, 0, 0))
            .with_component(ComponentId::SCHEDULER);
        let fb2 = ModuleInfo::new("fb2", ModuleVersion::new(1, 1, 0))
            .with_component(ComponentId::SCHEDULER);

        registry.register(fb1);
        registry.register(fb2);

        assert_eq!(registry.count(), 2);

        let best = registry.best_fallback(ComponentId::SCHEDULER);
        assert!(best.is_some());
        assert_eq!(best.unwrap().name, "fb2"); // Higher version
    }
}
