//! # Module System v2
//!
//! Unified module API for Helix OS Framework.
//!
//! This module provides a simplified, consistent interface for all modules.
//! It maintains backward compatibility while providing a cleaner API.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;

use crate::{ModuleError, ModuleFlags, ModuleId, ModuleVersion};

// =============================================================================
// Module Metadata v2
// =============================================================================

/// Module metadata v2 - simplified and consistent
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name (unique identifier string)
    pub name: &'static str,
    /// Version
    pub version: ModuleVersion,
    /// Description
    pub description: &'static str,
    /// Author(s)
    pub author: &'static str,
    /// License
    pub license: &'static str,
    /// Module flags
    pub flags: ModuleFlags,
    /// Dependencies (module names)
    pub dependencies: &'static [&'static str],
    /// Capabilities this module provides
    pub provides: &'static [&'static str],
}

impl ModuleInfo {
    /// Create new module info with builder pattern
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            version: ModuleVersion::new(0, 1, 0),
            description: "",
            author: "Unknown",
            license: "MIT",
            flags: ModuleFlags::empty(),
            dependencies: &[],
            provides: &[],
        }
    }

    /// Set version
    pub const fn version(mut self, major: u16, minor: u16, patch: u16) -> Self {
        self.version = ModuleVersion::new(major, minor, patch);
        self
    }

    /// Set description
    pub const fn description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    /// Set author
    pub const fn author(mut self, author: &'static str) -> Self {
        self.author = author;
        self
    }

    /// Set license
    pub const fn license(mut self, license: &'static str) -> Self {
        self.license = license;
        self
    }

    /// Set flags
    pub const fn flags(mut self, flags: ModuleFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set dependencies
    pub const fn dependencies(mut self, deps: &'static [&'static str]) -> Self {
        self.dependencies = deps;
        self
    }

    /// Set provides
    pub const fn provides(mut self, provs: &'static [&'static str]) -> Self {
        self.provides = provs;
        self
    }
}

// =============================================================================
// Events and Requests
// =============================================================================

/// Event types that modules can receive
#[derive(Debug, Clone)]
pub enum Event {
    /// System tick (timer interrupt)
    Tick { timestamp_ns: u64 },
    /// System is shutting down
    Shutdown,
    /// Memory pressure notification
    MemoryPressure { level: MemoryPressureLevel },
    /// CPU hotplug event
    CpuHotplug { cpu_id: u32, online: bool },
    /// Custom event with payload
    Custom { name: String, data: Vec<u8> },
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// System memory is fine
    Normal,
    /// Memory is getting low
    Low,
    /// Memory is critically low
    Critical,
}

/// Response to an event
#[derive(Debug, Clone)]
pub enum EventResponse {
    /// Event handled successfully
    Handled,
    /// Event not relevant to this module
    Ignored,
    /// Error handling event
    Error(String),
}

/// Request from one module to another
#[derive(Debug, Clone)]
pub struct Request {
    /// Source module
    pub source: &'static str,
    /// Request type
    pub request_type: String,
    /// Request payload
    pub payload: Vec<u8>,
}

/// Response to a request
#[derive(Debug, Clone)]
pub struct Response {
    /// Success status
    pub success: bool,
    /// Response payload
    pub payload: Vec<u8>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Response {
    /// Create a success response
    pub fn ok(payload: Vec<u8>) -> Self {
        Self {
            success: true,
            payload,
            error: None,
        }
    }

    /// Create an error response
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            payload: Vec::new(),
            error: Some(message.into()),
        }
    }

    /// Create an empty success response
    pub fn ok_empty() -> Self {
        Self::ok(Vec::new())
    }
}

// =============================================================================
// Module Context v2
// =============================================================================

/// Context provided to modules during initialization
pub struct Context<'a> {
    /// Module's assigned ID
    pub id: ModuleId,
    /// Configuration key-value pairs
    config: &'a dyn Fn(&str) -> Option<&'a str>,
    /// Request a service from another module
    request_service: &'a dyn Fn(&str, Request) -> Result<Response, ModuleError>,
}

impl<'a> Context<'a> {
    /// Create a new context
    pub fn new(
        id: ModuleId,
        config: &'a dyn Fn(&str) -> Option<&'a str>,
        request_service: &'a dyn Fn(&str, Request) -> Result<Response, ModuleError>,
    ) -> Self {
        Self {
            id,
            config,
            request_service,
        }
    }

    /// Get a configuration value
    pub fn config(&self, key: &str) -> Option<&str> {
        (self.config)(key)
    }

    /// Get a configuration value or default
    pub fn config_or(&self, key: &str, default: &'a str) -> &str {
        self.config(key).unwrap_or(default)
    }

    /// Get a configuration value as a number
    pub fn config_usize(&self, key: &str) -> Option<usize> {
        self.config(key).and_then(|s| s.parse().ok())
    }

    /// Request a service from another module
    pub fn request(&self, target: &str, request: Request) -> Result<Response, ModuleError> {
        (self.request_service)(target, request)
    }
}

// =============================================================================
// Module Trait v2
// =============================================================================

/// The unified Module trait v2
///
/// All modules must implement this trait. It provides a consistent interface
/// for module lifecycle management and inter-module communication.
///
/// # Lifecycle
///
/// 1. `info()` - Called to get module metadata
/// 2. `init()` - Called once during module loading
/// 3. `start()` - Called to activate the module
/// 4. `handle_event()` - Called for system events
/// 5. `handle_request()` - Called for IPC requests
/// 6. `stop()` - Called before unloading
///
/// # Example
///
/// ```rust,ignore
/// use helix_modules::v2::{ModuleTrait, ModuleInfo, Context, Event, EventResponse, Request, Response};
///
/// pub struct MyModule {
///     initialized: bool,
/// }
///
/// impl ModuleTrait for MyModule {
///     fn info(&self) -> ModuleInfo {
///         ModuleInfo::new("my-module")
///             .version(1, 0, 0)
///             .description("Example module")
///     }
///
///     fn init(&mut self, ctx: &Context) -> Result<(), ModuleError> {
///         self.initialized = true;
///         Ok(())
///     }
///
///     fn start(&mut self) -> Result<(), ModuleError> {
///         Ok(())
///     }
///
///     fn stop(&mut self) -> Result<(), ModuleError> {
///         Ok(())
///     }
/// }
/// ```
pub trait ModuleTrait: Send + Sync {
    /// Get module information (metadata)
    fn info(&self) -> ModuleInfo;

    /// Initialize the module
    ///
    /// Called after dependencies are loaded. Use this to allocate resources.
    fn init(&mut self, ctx: &Context) -> Result<(), ModuleError>;

    /// Start the module
    ///
    /// Called to begin normal operation.
    fn start(&mut self) -> Result<(), ModuleError>;

    /// Stop the module
    ///
    /// Called before unloading. Clean up resources here.
    fn stop(&mut self) -> Result<(), ModuleError>;

    /// Handle a system event
    ///
    /// Override to respond to system events like timer ticks, shutdown, etc.
    fn handle_event(&mut self, _event: &Event) -> EventResponse {
        EventResponse::Ignored
    }

    /// Handle a request from another module
    ///
    /// Override to handle IPC requests.
    fn handle_request(&mut self, _request: &Request) -> Result<Response, ModuleError> {
        Ok(Response::err("Not implemented"))
    }

    /// Check if module is healthy
    ///
    /// Called periodically to monitor module health.
    fn is_healthy(&self) -> bool {
        true
    }

    /// Get module state for hot-reload (optional)
    fn save_state(&self) -> Option<Vec<u8>> {
        None
    }

    /// Restore module state after hot-reload (optional)
    fn restore_state(&mut self, _state: &[u8]) -> Result<(), ModuleError> {
        Ok(())
    }
}

// =============================================================================
// Compatibility Layer
// =============================================================================

use crate::{Module, ModuleContext, ModuleDependency, ModuleMetadata};

/// Adapter to use ModuleTrait with the old Module interface
pub struct ModuleAdapter<T: ModuleTrait> {
    inner: T,
    _metadata_cache: Option<ModuleMetadata>,
}

impl<T: ModuleTrait> ModuleAdapter<T> {
    /// Create a new adapter
    pub fn new(module: T) -> Self {
        Self {
            inner: module,
            _metadata_cache: None,
        }
    }

    /// Get the inner module
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get mutable access to inner module
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    fn _build_metadata(&self) -> ModuleMetadata {
        let info = self.inner.info();
        ModuleMetadata {
            id: ModuleId::new(),
            name: String::from(info.name),
            version: info.version,
            description: String::from(info.description),
            authors: alloc::vec![String::from(info.author)],
            license: String::from(info.license),
            flags: info.flags,
            dependencies: info
                .dependencies
                .iter()
                .map(|&dep| ModuleDependency {
                    name: String::from(dep),
                    min_version: ModuleVersion::new(0, 0, 0),
                    max_version: None,
                    optional: false,
                })
                .collect(),
            provides: info.provides.iter().map(|&p| String::from(p)).collect(),
            abi_version: crate::abi::AbiVersion::CURRENT,
        }
    }
}

impl<T: ModuleTrait + 'static> Module for ModuleAdapter<T> {
    fn metadata(&self) -> &ModuleMetadata {
        // This is a bit of a hack - we can't easily cache without interior mutability
        // For now, return a reference to cached metadata or create new
        // In real implementation, use OnceCell or similar
        static EMPTY: ModuleMetadata = ModuleMetadata {
            id: ModuleId::from_raw_const(0),
            name: String::new(),
            version: ModuleVersion::new(0, 0, 0),
            description: String::new(),
            authors: Vec::new(),
            license: String::new(),
            flags: ModuleFlags::empty(),
            dependencies: Vec::new(),
            provides: Vec::new(),
            abi_version: crate::abi::AbiVersion::CURRENT,
        };
        &EMPTY
    }

    fn init(&mut self, ctx: &ModuleContext) -> crate::ModuleResult<()> {
        // Create a v2 context from the old context
        let config_fn = |key: &str| -> Option<&str> { ctx.config.get(key).map(|s| s.as_str()) };
        let request_fn = |_target: &str, _request: Request| -> Result<Response, ModuleError> {
            Err(ModuleError::NotFound)
        };

        let v2_ctx = Context::new(ctx.id, &config_fn, &request_fn);
        self.inner.init(&v2_ctx)
    }

    fn start(&mut self) -> crate::ModuleResult<()> {
        self.inner.start()
    }

    fn stop(&mut self) -> crate::ModuleResult<()> {
        self.inner.stop()
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    fn get_state(&self) -> Option<Box<dyn Any + Send + Sync>> {
        self.inner
            .save_state()
            .map(|v| Box::new(v) as Box<dyn Any + Send + Sync>)
    }

    fn restore_state(&mut self, state: Box<dyn Any + Send + Sync>) -> crate::ModuleResult<()> {
        if let Ok(bytes) = state.downcast::<Vec<u8>>() {
            self.inner.restore_state(&bytes)
        } else {
            Err(ModuleError::Internal(String::from("Invalid state type")))
        }
    }
}

// =============================================================================
// Module Registration Macro
// =============================================================================

/// Macro to easily define a module with the v2 API
#[macro_export]
macro_rules! module_v2 {
    (
        $vis:vis struct $name:ident {
            $($field:ident : $type:ty = $default:expr),* $(,)?
        }

        info: {
            name: $mod_name:expr,
            version: ($major:expr, $minor:expr, $patch:expr),
            $($info_key:ident : $info_val:expr),* $(,)?
        }

        $(impl { $($impl_body:tt)* })?
    ) => {
        $vis struct $name {
            $($field: $type,)*
        }

        impl $name {
            /// Create a new module instance
            pub fn new() -> Self {
                Self {
                    $($field: $default,)*
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $crate::v2::ModuleTrait for $name {
            fn info(&self) -> $crate::v2::ModuleInfo {
                $crate::v2::ModuleInfo::new($mod_name)
                    .version($major, $minor, $patch)
                    $(.$info_key($info_val))*
            }

            $($($impl_body)*)?
        }
    };
}
