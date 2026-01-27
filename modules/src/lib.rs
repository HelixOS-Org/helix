//! # Helix Module System
//!
//! The module system is the heart of Helix's flexibility. It provides:
//!
//! - Dynamic module loading and unloading
//! - Static module linking
//! - Hot-reload capabilities
//! - Dependency resolution
//! - ABI versioning and compatibility
//!
//! ## Module Types
//!
//! - **Static Modules**: Linked into the kernel at compile time
//! - **Dynamic Modules**: Loaded at runtime
//! - **Kernel-space Modules**: Run in ring 0
//! - **User-space Modules**: Run in ring 3 with IPC
//!
//! ## Module Lifecycle
//!
//! 1. Registration (metadata)
//! 2. Dependency Resolution
//! 3. Loading
//! 4. Initialization
//! 5. Running
//! 6. Shutdown
//! 7. Unloading

#![no_std]
#![feature(negative_impls)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod loader;
pub mod registry;
pub mod dependencies;
pub mod abi;
pub mod hot_reload;
pub mod interface;
pub mod v2;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::sync::atomic::{AtomicU64, Ordering};
use bitflags::bitflags;

/// Module identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u64);

impl ModuleId {
    /// Create a new module ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Create from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create from raw value (const version)
    pub const fn from_raw_const(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for ModuleId {
    fn default() -> Self {
        Self::new()
    }
}

/// Module version (semantic versioning)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
}

impl ModuleVersion {
    /// Create a new version
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    /// Check if this version is compatible with another
    ///
    /// Follows semver rules: same major, minor >= required
    pub fn is_compatible_with(&self, required: &Self) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
}

impl core::fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

bitflags! {
    /// Module flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModuleFlags: u32 {
        /// Module is essential (cannot be unloaded)
        const ESSENTIAL = 1 << 0;
        /// Module supports hot-reload
        const HOT_RELOADABLE = 1 << 1;
        /// Module runs in user space
        const USERSPACE = 1 << 2;
        /// Module is a driver
        const DRIVER = 1 << 3;
        /// Module provides a filesystem
        const FILESYSTEM = 1 << 4;
        /// Module provides a scheduler
        const SCHEDULER = 1 << 5;
        /// Module provides memory allocation
        const ALLOCATOR = 1 << 6;
        /// Module provides security features
        const SECURITY = 1 << 7;
        /// Module is experimental
        const EXPERIMENTAL = 1 << 8;
        /// Module is deprecated
        const DEPRECATED = 1 << 9;
    }
}

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Module is registered but not loaded
    Registered,
    /// Module is being loaded
    Loading,
    /// Module is loaded but not initialized
    Loaded,
    /// Module is being initialized
    Initializing,
    /// Module is running
    Running,
    /// Module is being stopped
    Stopping,
    /// Module is stopped
    Stopped,
    /// Module is being unloaded
    Unloading,
    /// Module has encountered an error
    Error,
}

/// Module metadata
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    /// Unique identifier
    pub id: ModuleId,
    /// Human-readable name
    pub name: String,
    /// Version
    pub version: ModuleVersion,
    /// Description
    pub description: String,
    /// Author(s)
    pub authors: Vec<String>,
    /// License
    pub license: String,
    /// Module flags
    pub flags: ModuleFlags,
    /// Dependencies
    pub dependencies: Vec<ModuleDependency>,
    /// Provides (capabilities/interfaces this module provides)
    pub provides: Vec<String>,
    /// ABI version
    pub abi_version: abi::AbiVersion,
}

/// Module dependency specification
#[derive(Debug, Clone)]
pub struct ModuleDependency {
    /// Name of the required module
    pub name: String,
    /// Minimum required version
    pub min_version: ModuleVersion,
    /// Maximum compatible version (optional)
    pub max_version: Option<ModuleVersion>,
    /// Is this dependency optional?
    pub optional: bool,
}

/// Module result type
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Module errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    /// Module not found
    NotFound,
    /// Module already exists
    AlreadyExists,
    /// Module already loaded
    AlreadyLoaded,
    /// Module not loaded
    NotLoaded,
    /// Dependency not satisfied
    DependencyNotSatisfied(String),
    /// Circular dependency detected
    CircularDependency,
    /// Version mismatch
    VersionMismatch { expected: ModuleVersion, found: ModuleVersion },
    /// ABI incompatible
    AbiIncompatible,
    /// Load error
    LoadError(String),
    /// Initialization error
    InitError(String),
    /// Module in wrong state
    WrongState { current: ModuleState, required: ModuleState },
    /// Module is essential and cannot be unloaded
    Essential,
    /// Internal error
    Internal(String),
}

/// The core module trait
///
/// All modules must implement this trait.
pub trait Module: Send + Sync {
    /// Get module metadata
    fn metadata(&self) -> &ModuleMetadata;

    /// Initialize the module
    ///
    /// Called after all dependencies are loaded.
    fn init(&mut self, context: &ModuleContext) -> ModuleResult<()>;

    /// Start the module
    ///
    /// Called to begin normal operation.
    fn start(&mut self) -> ModuleResult<()>;

    /// Stop the module
    ///
    /// Called before unloading or during shutdown.
    fn stop(&mut self) -> ModuleResult<()>;

    /// Cleanup the module
    ///
    /// Called after stop, before unloading.
    fn cleanup(&mut self) -> ModuleResult<()> {
        Ok(())
    }

    /// Check if module is healthy
    fn is_healthy(&self) -> bool {
        true
    }

    /// Get module state for hot-reload
    ///
    /// Only called if HOT_RELOADABLE flag is set.
    fn get_state(&self) -> Option<Box<dyn Any + Send + Sync>> {
        None
    }

    /// Restore module state after hot-reload
    ///
    /// Only called if HOT_RELOADABLE flag is set.
    fn restore_state(&mut self, _state: Box<dyn Any + Send + Sync>) -> ModuleResult<()> {
        Ok(())
    }

    /// Handle a message from the IPC system
    fn handle_message(&mut self, _message: &interface::ModuleMessage) -> ModuleResult<Option<interface::ModuleMessage>> {
        Ok(None)
    }
}

/// Context provided to modules during initialization
pub struct ModuleContext {
    /// Module's own ID
    pub id: ModuleId,
    /// Function to get a dependency by name
    get_dependency: Box<dyn Fn(&str) -> Option<Arc<dyn Module>> + Send + Sync>,
    /// Function to send a message to another module
    send_message: Box<dyn Fn(&str, interface::ModuleMessage) -> ModuleResult<()> + Send + Sync>,
    /// Configuration parameters
    pub config: alloc::collections::BTreeMap<String, String>,
}

impl ModuleContext {
    /// Create a new module context
    pub fn new(
        id: ModuleId,
        get_dependency: impl Fn(&str) -> Option<Arc<dyn Module>> + Send + Sync + 'static,
        send_message: impl Fn(&str, interface::ModuleMessage) -> ModuleResult<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            id,
            get_dependency: Box::new(get_dependency),
            send_message: Box::new(send_message),
            config: alloc::collections::BTreeMap::new(),
        }
    }

    /// Get a dependency module
    pub fn get_dependency(&self, name: &str) -> Option<Arc<dyn Module>> {
        (self.get_dependency)(name)
    }

    /// Send a message to another module
    pub fn send_message(&self, target: &str, message: interface::ModuleMessage) -> ModuleResult<()> {
        (self.send_message)(target, message)
    }
}

/// Macro to define a module
#[macro_export]
macro_rules! define_module {
    (
        name: $name:expr,
        version: ($major:expr, $minor:expr, $patch:expr),
        description: $desc:expr,
        $(flags: $flags:expr,)?
        $(dependencies: [$($dep:expr),* $(,)?],)?
        $(provides: [$($prov:expr),* $(,)?],)?
        struct $struct_name:ident { $($body:tt)* }
    ) => {
        pub struct $struct_name {
            metadata: $crate::ModuleMetadata,
            $($body)*
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self {
                    metadata: $crate::ModuleMetadata {
                        id: $crate::ModuleId::new(),
                        name: alloc::string::String::from($name),
                        version: $crate::ModuleVersion::new($major, $minor, $patch),
                        description: alloc::string::String::from($desc),
                        authors: alloc::vec::Vec::new(),
                        license: alloc::string::String::from("MIT"),
                        flags: {
                            #[allow(unused_mut)]
                            let mut flags = $crate::ModuleFlags::empty();
                            $(flags = $flags;)?
                            flags
                        },
                        dependencies: {
                            #[allow(unused_mut)]
                            let mut deps = alloc::vec::Vec::new();
                            $($(deps.push($dep);)*)?
                            deps
                        },
                        provides: {
                            #[allow(unused_mut)]
                            let mut provs: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();
                            $($(provs.push(alloc::string::String::from($prov));)*)?
                            provs
                        },
                        abi_version: $crate::abi::AbiVersion::CURRENT,
                    },
                    // Initialize other fields to default
                }
            }
        }
    };
}
