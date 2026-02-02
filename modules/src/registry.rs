//! # Module Registry
//!
//! Central registry for all loaded modules.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use spin::RwLock;

use crate::loader::LoadedModule;
use crate::{
    Module, ModuleError, ModuleFlags, ModuleId, ModuleMetadata, ModuleResult, ModuleState,
};

/// Registered module entry
pub struct RegisteredModule {
    /// Module metadata
    pub metadata: ModuleMetadata,
    /// Loaded module (if loaded)
    pub loaded: Option<LoadedModule>,
    /// Current state
    pub state: ModuleState,
    /// Dependents (modules that depend on this one)
    pub dependents: Vec<ModuleId>,
}

/// Module registry
pub struct ModuleRegistry {
    /// All registered modules by ID
    modules: RwLock<BTreeMap<ModuleId, RegisteredModule>>,
    /// Name to ID mapping
    name_to_id: RwLock<BTreeMap<String, ModuleId>>,
    /// Provides mapping (capability -> module)
    provides: RwLock<BTreeMap<String, Vec<ModuleId>>>,
}

impl ModuleRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            modules: RwLock::new(BTreeMap::new()),
            name_to_id: RwLock::new(BTreeMap::new()),
            provides: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a module (metadata only, not loaded)
    pub fn register(&self, metadata: ModuleMetadata) -> ModuleResult<ModuleId> {
        let name = metadata.name.clone();
        let id = metadata.id;
        let provides = metadata.provides.clone();

        // Check for duplicate
        if self.name_to_id.read().contains_key(&name) {
            return Err(ModuleError::AlreadyExists);
        }

        let entry = RegisteredModule {
            metadata,
            loaded: None,
            state: ModuleState::Registered,
            dependents: Vec::new(),
        };

        self.modules.write().insert(id, entry);
        self.name_to_id.write().insert(name, id);

        // Register provides
        for cap in provides {
            self.provides.write().entry(cap).or_default().push(id);
        }

        log::info!(
            "Registered module: {} (id={})",
            self.modules.read().get(&id).unwrap().metadata.name,
            id.as_u64()
        );

        Ok(id)
    }

    /// Unregister a module
    pub fn unregister(&self, id: ModuleId) -> ModuleResult<()> {
        let mut modules = self.modules.write();

        let entry = modules.get(&id).ok_or(ModuleError::NotFound)?;

        // Can't unregister if loaded
        if entry.loaded.is_some() {
            return Err(ModuleError::WrongState {
                current: entry.state,
                required: ModuleState::Registered,
            });
        }

        // Can't unregister if has dependents
        if !entry.dependents.is_empty() {
            return Err(ModuleError::DependencyNotSatisfied(
                "Module has dependents".into(),
            ));
        }

        let name = entry.metadata.name.clone();
        let provides = entry.metadata.provides.clone();

        modules.remove(&id);
        self.name_to_id.write().remove(&name);

        // Unregister provides
        for cap in provides {
            if let Some(v) = self.provides.write().get_mut(&cap) {
                v.retain(|&x| x != id);
            }
        }

        Ok(())
    }

    /// Get a module by ID
    pub fn get(&self, id: ModuleId) -> Option<ModuleMetadata> {
        self.modules.read().get(&id).map(|e| e.metadata.clone())
    }

    /// Get a module by name
    pub fn get_by_name(&self, name: &str) -> Option<ModuleMetadata> {
        let id = *self.name_to_id.read().get(name)?;
        self.get(id)
    }

    /// Get module ID by name
    pub fn id_by_name(&self, name: &str) -> Option<ModuleId> {
        self.name_to_id.read().get(name).copied()
    }

    /// Get modules that provide a capability
    pub fn get_providers(&self, capability: &str) -> Vec<ModuleId> {
        self.provides
            .read()
            .get(capability)
            .cloned()
            .unwrap_or_default()
    }

    /// Get module state
    pub fn get_state(&self, id: ModuleId) -> Option<ModuleState> {
        self.modules.read().get(&id).map(|e| e.state)
    }

    /// Set module state
    pub fn set_state(&self, id: ModuleId, state: ModuleState) -> ModuleResult<()> {
        self.modules
            .write()
            .get_mut(&id)
            .ok_or(ModuleError::NotFound)?
            .state = state;
        Ok(())
    }

    /// Get all registered modules
    pub fn list_all(&self) -> Vec<ModuleMetadata> {
        self.modules
            .read()
            .values()
            .map(|e| e.metadata.clone())
            .collect()
    }

    /// Get all modules with a specific flag
    pub fn list_by_flag(&self, flag: ModuleFlags) -> Vec<ModuleMetadata> {
        self.modules
            .read()
            .values()
            .filter(|e| e.metadata.flags.contains(flag))
            .map(|e| e.metadata.clone())
            .collect()
    }

    /// Get all running modules
    pub fn list_running(&self) -> Vec<ModuleMetadata> {
        self.modules
            .read()
            .values()
            .filter(|e| e.state == ModuleState::Running)
            .map(|e| e.metadata.clone())
            .collect()
    }

    /// Add a dependent
    pub fn add_dependent(&self, id: ModuleId, dependent: ModuleId) -> ModuleResult<()> {
        self.modules
            .write()
            .get_mut(&id)
            .ok_or(ModuleError::NotFound)?
            .dependents
            .push(dependent);
        Ok(())
    }

    /// Remove a dependent
    pub fn remove_dependent(&self, id: ModuleId, dependent: ModuleId) -> ModuleResult<()> {
        self.modules
            .write()
            .get_mut(&id)
            .ok_or(ModuleError::NotFound)?
            .dependents
            .retain(|&d| d != dependent);
        Ok(())
    }

    /// Get dependents of a module
    pub fn get_dependents(&self, id: ModuleId) -> Vec<ModuleId> {
        self.modules
            .read()
            .get(&id)
            .map(|e| e.dependents.clone())
            .unwrap_or_default()
    }
}

/// Global module registry
static REGISTRY: ModuleRegistry = ModuleRegistry::new();

/// Get the global module registry
pub fn registry() -> &'static ModuleRegistry {
    &REGISTRY
}
