//! Module information for substitution.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::ComponentId;

use super::version::ModuleVersion;

/// Information about a module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module ID
    pub id: u64,
    /// Module name
    pub name: String,
    /// Version
    pub version: ModuleVersion,
    /// Component type
    pub component: ComponentId,
    /// ABI hash for compatibility checking
    pub abi_hash: u64,
    /// Required capabilities
    pub capabilities: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Is this a fallback module?
    pub is_fallback: bool,
}

impl ModuleInfo {
    /// Create new module info
    pub fn new(name: impl Into<String>, version: ModuleVersion) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            version,
            component: ComponentId::UNKNOWN,
            abi_hash: 0,
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            is_fallback: false,
        }
    }

    /// Set component
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = component;
        self
    }

    /// Set ABI hash
    pub fn with_abi_hash(mut self, hash: u64) -> Self {
        self.abi_hash = hash;
        self
    }

    /// Add capability
    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Mark as fallback
    pub fn as_fallback(mut self) -> Self {
        self.is_fallback = true;
        self
    }

    /// Check ABI compatibility
    pub fn is_compatible_with(&self, other: &ModuleInfo) -> bool {
        self.version.is_abi_compatible(&other.version) && self.abi_hash == other.abi_hash
    }
}
