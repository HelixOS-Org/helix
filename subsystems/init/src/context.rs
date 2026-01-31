//! # Initialization Context
//!
//! This module provides the `InitContext` type which is passed to subsystems
//! during initialization. It provides access to kernel services, resources,
//! and inter-subsystem communication.
//!
//! ## Context Hierarchy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                          INIT CONTEXT                                        │
//! │                                                                              │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                         PHASE CONTEXT                                 │   │
//! │  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
//! │  │  │                                                                  │ │   │
//! │  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │ │   │
//! │  │  │  │   Config    │  │  Services   │  │  Resources  │              │ │   │
//! │  │  │  └─────────────┘  └─────────────┘  └─────────────┘              │ │   │
//! │  │  │        │               │                │                       │ │   │
//! │  │  │        ▼               ▼                ▼                       │ │   │
//! │  │  │  ┌─────────────────────────────────────────────────────┐       │ │   │
//! │  │  │  │              SUBSYSTEM CONTEXT                       │       │ │   │
//! │  │  │  │                                                      │       │ │   │
//! │  │  │  │  - Current subsystem info                           │       │ │   │
//! │  │  │  │  - Allocated resources                              │       │ │   │
//! │  │  │  │  - Rollback chain                                   │       │ │   │
//! │  │  │  │  - Dependency handles                               │       │ │   │
//! │  │  │  │                                                      │       │ │   │
//! │  │  │  └─────────────────────────────────────────────────────┘       │ │   │
//! │  │  │                                                                  │ │   │
//! │  │  └─────────────────────────────────────────────────────────────────┘ │   │
//! │  │                                                                       │   │
//! │  └──────────────────────────────────────────────────────────────────────┘   │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Available Services by Phase
//!
//! | Service | Boot | Early | Core | Late | Runtime |
//! |---------|------|-------|------|------|---------|
//! | Console | ✅ | ✅ | ✅ | ✅ | ✅ |
//! | Memory | ❌ | ✅ | ✅ | ✅ | ✅ |
//! | Scheduler | ❌ | ❌ | ✅ | ✅ | ✅ |
//! | IPC | ❌ | ❌ | ✅ | ✅ | ✅ |
//! | Drivers | ❌ | ❌ | ❌ | ✅ | ✅ |
//! | Filesystem | ❌ | ❌ | ❌ | ✅ | ✅ |

use core::any::Any;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::{ErrorKind, InitError, InitResult, RollbackAction, RollbackChain};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::SubsystemId;

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

// =============================================================================
// RESOURCE HANDLE
// =============================================================================

/// A handle to a managed resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceHandle(u64);

impl ResourceHandle {
    /// Create new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Invalid handle
    pub const INVALID: Self = Self(0);

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for ResourceHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Resource handle generator
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn generate_handle() -> ResourceHandle {
    ResourceHandle::new(NEXT_HANDLE.fetch_add(1, Ordering::Relaxed))
}

// =============================================================================
// RESOURCE
// =============================================================================

/// Trait for resources that can be managed by the context
pub trait Resource: Any + Send + Sync {
    /// Get resource type name
    fn type_name(&self) -> &'static str;

    /// Release the resource
    fn release(&mut self) -> InitResult<()>;

    /// Check if resource is valid
    fn is_valid(&self) -> bool {
        true
    }

    /// Downcast to concrete type
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A boxed resource
pub type BoxedResource = Box<dyn Resource>;

/// Resource entry in the registry
struct ResourceEntry {
    /// The resource
    resource: BoxedResource,

    /// Owning subsystem
    owner: SubsystemId,

    /// Reference count
    refcount: u32,
}

// =============================================================================
// CONFIG PROVIDER
// =============================================================================

/// Configuration value
#[derive(Debug, Clone)]
pub enum ConfigValue {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// String
    String(String),
    /// Bytes
    Bytes(Vec<u8>),
    /// Nested map
    Map(BTreeMap<String, ConfigValue>),
    /// List
    List(Vec<ConfigValue>),
}

impl ConfigValue {
    /// Get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as int
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as uint
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            ConfigValue::Uint(u) => Some(*u),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ConfigValue::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Get as map
    pub fn as_map(&self) -> Option<&BTreeMap<String, ConfigValue>> {
        match self {
            ConfigValue::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Get as list
    pub fn as_list(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::List(l) => Some(l),
            _ => None,
        }
    }
}

/// Configuration provider trait
pub trait ConfigProvider: Send + Sync {
    /// Get a configuration value
    fn get(&self, key: &str) -> Option<ConfigValue>;

    /// Get with default
    fn get_or(&self, key: &str, default: ConfigValue) -> ConfigValue {
        self.get(key).unwrap_or(default)
    }

    /// Get bool with default
    fn get_bool(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// Get int with default
    fn get_int(&self, key: &str, default: i64) -> i64 {
        self.get(key).and_then(|v| v.as_int()).unwrap_or(default)
    }

    /// Get uint with default
    fn get_uint(&self, key: &str, default: u64) -> u64 {
        self.get(key).and_then(|v| v.as_uint()).unwrap_or(default)
    }

    /// Get string with default
    fn get_str(&self, key: &str, default: &str) -> String {
        self.get(key)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| default.to_string())
    }
}

/// Simple in-memory config provider
pub struct MemoryConfig {
    values: BTreeMap<String, ConfigValue>,
}

impl MemoryConfig {
    /// Create empty config
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Set a value
    pub fn set(&mut self, key: impl Into<String>, value: ConfigValue) {
        self.values.insert(key.into(), value);
    }

    /// Set bool
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set(key, ConfigValue::Bool(value));
    }

    /// Set int
    pub fn set_int(&mut self, key: impl Into<String>, value: i64) {
        self.set(key, ConfigValue::Int(value));
    }

    /// Set uint
    pub fn set_uint(&mut self, key: impl Into<String>, value: u64) {
        self.set(key, ConfigValue::Uint(value));
    }

    /// Set string
    pub fn set_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.set(key, ConfigValue::String(value.into()));
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProvider for MemoryConfig {
    fn get(&self, key: &str) -> Option<ConfigValue> {
        self.values.get(key).cloned()
    }
}

// =============================================================================
// SERVICE REGISTRY
// =============================================================================

/// A kernel service that can be provided by subsystems
pub trait Service: Any + Send + Sync {
    /// Get service name
    fn name(&self) -> &'static str;

    /// Get service version
    fn version(&self) -> (u16, u16, u16) {
        (1, 0, 0)
    }

    /// Check if service is ready
    fn is_ready(&self) -> bool {
        true
    }

    /// Downcast
    fn as_any(&self) -> &dyn Any;
}

/// Boxed service
pub type BoxedService = Arc<dyn Service>;

/// Service registry
pub struct ServiceRegistry {
    services: BTreeMap<&'static str, BoxedService>,
}

impl ServiceRegistry {
    /// Create empty registry
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
        }
    }

    /// Register a service
    pub fn register(&mut self, service: BoxedService) -> InitResult<()> {
        let name = service.name();
        if self.services.contains_key(name) {
            return Err(
                InitError::new(ErrorKind::AlreadyExists, "Service already registered")
                    .with_details(alloc::format!("Service: {}", name)),
            );
        }
        self.services.insert(name, service);
        Ok(())
    }

    /// Get a service by name
    pub fn get(&self, name: &str) -> Option<&BoxedService> {
        self.services.get(name)
    }

    /// Check if service exists
    pub fn has(&self, name: &str) -> bool {
        self.services.contains_key(name)
    }

    /// Get service and downcast
    pub fn get_as<T: Service + 'static>(&self, name: &str) -> Option<&T> {
        self.services
            .get(name)
            .and_then(|s| s.as_any().downcast_ref::<T>())
    }

    /// List all services
    pub fn list(&self) -> Vec<&'static str> {
        self.services.keys().copied().collect()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// INIT CONTEXT
// =============================================================================

/// Context provided to subsystems during initialization
///
/// This is the main interface for subsystems to interact with the kernel
/// during initialization.
pub struct InitContext {
    // -------------------------------------------------------------------------
    // Phase Information
    // -------------------------------------------------------------------------
    /// Current phase
    phase: InitPhase,

    /// Available capabilities
    capabilities: PhaseCapabilities,

    // -------------------------------------------------------------------------
    // Configuration
    // -------------------------------------------------------------------------
    /// Configuration provider
    config: Box<dyn ConfigProvider>,

    // -------------------------------------------------------------------------
    // Services
    // -------------------------------------------------------------------------
    /// Service registry
    services: ServiceRegistry,

    // -------------------------------------------------------------------------
    // Resources
    // -------------------------------------------------------------------------
    /// Managed resources
    resources: BTreeMap<ResourceHandle, ResourceEntry>,

    /// Resources by subsystem
    by_subsystem: BTreeMap<SubsystemId, Vec<ResourceHandle>>,

    // -------------------------------------------------------------------------
    // Rollback
    // -------------------------------------------------------------------------
    /// Rollback chain
    rollback: RollbackChain,

    // -------------------------------------------------------------------------
    // Current Context
    // -------------------------------------------------------------------------
    /// Current subsystem being initialized
    current_subsystem: Option<SubsystemId>,

    /// Boot info (architecture-specific)
    boot_info: Option<BootInfo>,

    // -------------------------------------------------------------------------
    // Logging
    // -------------------------------------------------------------------------
    /// Log buffer for early boot
    log_buffer: Vec<LogEntry>,

    /// Maximum log entries
    max_log_entries: usize,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: u64,
    /// Level
    pub level: LogLevel,
    /// Subsystem
    pub subsystem: Option<SubsystemId>,
    /// Message
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Architecture-specific boot info
pub struct BootInfo {
    /// Command line
    pub cmdline: Option<String>,

    /// Memory map
    pub memory_map: Vec<MemoryRegion>,

    /// Framebuffer info
    pub framebuffer: Option<FramebufferInfo>,

    /// ACPI RSDP address
    pub rsdp_addr: Option<u64>,

    /// Device tree address (ARM/RISC-V)
    pub dtb_addr: Option<u64>,

    /// EFI system table
    pub efi_system_table: Option<u64>,

    /// Custom data
    pub custom: BTreeMap<String, Vec<u8>>,
}

impl Default for BootInfo {
    fn default() -> Self {
        Self {
            cmdline: None,
            memory_map: Vec::new(),
            framebuffer: None,
            rsdp_addr: None,
            dtb_addr: None,
            efi_system_table: None,
            custom: BTreeMap::new(),
        }
    }
}

/// Memory region
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Base address
    pub base: u64,
    /// Length
    pub length: u64,
    /// Type
    pub kind: MemoryKind,
}

/// Memory region kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// Usable RAM
    Usable,
    /// Reserved
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Bad memory
    BadMemory,
    /// Bootloader reclaimable
    BootloaderReclaimable,
    /// Kernel and modules
    KernelAndModules,
    /// Framebuffer
    Framebuffer,
}

/// Framebuffer info
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Address
    pub address: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per row)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u16,
    /// Pixel format
    pub format: PixelFormat,
}

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Unknown,
}

impl InitContext {
    /// Create new context for a phase
    pub fn new(phase: InitPhase) -> Self {
        Self {
            phase,
            capabilities: phase.capabilities(),
            config: Box::new(MemoryConfig::new()),
            services: ServiceRegistry::new(),
            resources: BTreeMap::new(),
            by_subsystem: BTreeMap::new(),
            rollback: RollbackChain::new(),
            current_subsystem: None,
            boot_info: None,
            log_buffer: Vec::new(),
            max_log_entries: 1024,
        }
    }

    // -------------------------------------------------------------------------
    // Phase Information
    // -------------------------------------------------------------------------

    /// Get current phase
    pub fn phase(&self) -> InitPhase {
        self.phase
    }

    /// Get available capabilities
    pub fn capabilities(&self) -> PhaseCapabilities {
        self.capabilities
    }

    /// Check if a capability is available
    pub fn has_capability(&self, cap: PhaseCapabilities) -> bool {
        self.capabilities.contains(cap)
    }

    /// Transition to next phase
    pub fn advance_phase(&mut self) -> InitResult<InitPhase> {
        match self.phase.next() {
            Some(next) => {
                self.phase = next;
                self.capabilities = next.capabilities();
                Ok(next)
            },
            None => Err(InitError::new(
                ErrorKind::InvalidState,
                "Already at final phase",
            )),
        }
    }

    // -------------------------------------------------------------------------
    // Configuration
    // -------------------------------------------------------------------------

    /// Get configuration
    pub fn config(&self) -> &dyn ConfigProvider {
        &*self.config
    }

    /// Set configuration provider
    pub fn set_config(&mut self, config: Box<dyn ConfigProvider>) {
        self.config = config;
    }

    // -------------------------------------------------------------------------
    // Services
    // -------------------------------------------------------------------------

    /// Get service registry
    pub fn services(&self) -> &ServiceRegistry {
        &self.services
    }

    /// Get mutable service registry
    pub fn services_mut(&mut self) -> &mut ServiceRegistry {
        &mut self.services
    }

    /// Register a service
    pub fn register_service(&mut self, service: BoxedService) -> InitResult<()> {
        self.services.register(service)
    }

    /// Get a service
    pub fn get_service(&self, name: &str) -> Option<&BoxedService> {
        self.services.get(name)
    }

    // -------------------------------------------------------------------------
    // Resources
    // -------------------------------------------------------------------------

    /// Allocate a resource
    pub fn allocate_resource(&mut self, resource: BoxedResource) -> InitResult<ResourceHandle> {
        let handle = generate_handle();
        let owner = self.current_subsystem.unwrap_or(SubsystemId::INVALID);

        self.resources.insert(handle, ResourceEntry {
            resource,
            owner,
            refcount: 1,
        });

        self.by_subsystem
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(handle);

        Ok(handle)
    }

    /// Get a resource
    pub fn get_resource(&self, handle: ResourceHandle) -> Option<&dyn Resource> {
        self.resources.get(&handle).map(|e| &*e.resource)
    }

    /// Get mutable resource
    pub fn get_resource_mut(&mut self, handle: ResourceHandle) -> Option<&mut dyn Resource> {
        self.resources.get_mut(&handle).map(|e| &mut *e.resource)
    }

    /// Release a resource
    pub fn release_resource(&mut self, handle: ResourceHandle) -> InitResult<()> {
        if let Some(mut entry) = self.resources.remove(&handle) {
            entry.resource.release()?;

            // Remove from subsystem list
            if let Some(handles) = self.by_subsystem.get_mut(&entry.owner) {
                handles.retain(|h| *h != handle);
            }
        }
        Ok(())
    }

    /// Release all resources for a subsystem
    pub fn release_subsystem_resources(&mut self, subsystem: SubsystemId) -> InitResult<()> {
        if let Some(handles) = self.by_subsystem.remove(&subsystem) {
            for handle in handles {
                if let Some(mut entry) = self.resources.remove(&handle) {
                    entry.resource.release()?;
                }
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Rollback
    // -------------------------------------------------------------------------

    /// Get rollback chain
    pub fn rollback(&mut self) -> &mut RollbackChain {
        &mut self.rollback
    }

    /// Add rollback action
    pub fn add_rollback<A: RollbackAction + 'static>(&mut self, action: A) {
        if let Some(subsystem) = self.current_subsystem {
            self.rollback.push_for_subsystem(subsystem, action);
        } else {
            self.rollback.push(action);
        }
    }

    /// Execute rollback for current subsystem
    pub fn rollback_current(&mut self) -> InitResult<()> {
        if let Some(subsystem) = self.current_subsystem {
            self.rollback.execute_for_subsystem(subsystem)?;
            self.release_subsystem_resources(subsystem)?;
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Current Subsystem
    // -------------------------------------------------------------------------

    /// Set current subsystem
    pub fn set_current_subsystem(&mut self, id: SubsystemId) {
        self.current_subsystem = Some(id);
    }

    /// Clear current subsystem
    pub fn clear_current_subsystem(&mut self) {
        self.current_subsystem = None;
    }

    /// Get current subsystem
    pub fn current_subsystem(&self) -> Option<SubsystemId> {
        self.current_subsystem
    }

    // -------------------------------------------------------------------------
    // Boot Info
    // -------------------------------------------------------------------------

    /// Get boot info
    pub fn boot_info(&self) -> Option<&BootInfo> {
        self.boot_info.as_ref()
    }

    /// Get mutable boot info
    pub fn boot_info_mut(&mut self) -> Option<&mut BootInfo> {
        self.boot_info.as_mut()
    }

    /// Set boot info
    pub fn set_boot_info(&mut self, info: BootInfo) {
        self.boot_info = Some(info);
    }

    // -------------------------------------------------------------------------
    // Logging
    // -------------------------------------------------------------------------

    /// Log a message
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        if self.log_buffer.len() >= self.max_log_entries {
            self.log_buffer.remove(0);
        }

        self.log_buffer.push(LogEntry {
            timestamp: crate::get_timestamp(),
            level,
            subsystem: self.current_subsystem,
            message: message.into(),
        });
    }

    /// Log trace
    pub fn trace(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Trace, message);
    }

    /// Log debug
    pub fn debug(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Debug, message);
    }

    /// Log info
    pub fn info(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    /// Log warning
    pub fn warn(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Warn, message);
    }

    /// Log error
    pub fn error(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    /// Get log buffer
    pub fn logs(&self) -> &[LogEntry] {
        &self.log_buffer
    }

    /// Clear log buffer
    pub fn clear_logs(&mut self) {
        self.log_buffer.clear();
    }

    // -------------------------------------------------------------------------
    // Assertions and Checks
    // -------------------------------------------------------------------------

    /// Assert we're in the expected phase
    pub fn require_phase(&self, phase: InitPhase) -> InitResult<()> {
        if self.phase != phase {
            Err(InitError::new(ErrorKind::WrongPhase, "Wrong init phase")
                .with_details(alloc::format!("Expected {:?}, got {:?}", phase, self.phase))
                .with_phase(self.phase))
        } else {
            Ok(())
        }
    }

    /// Assert capability is available
    pub fn require_capability(&self, cap: PhaseCapabilities) -> InitResult<()> {
        if !self.capabilities.contains(cap) {
            Err(
                InitError::new(ErrorKind::NotSupported, "Capability not available")
                    .with_phase(self.phase),
            )
        } else {
            Ok(())
        }
    }

    /// Assert we're at or past a phase
    pub fn require_min_phase(&self, min: InitPhase) -> InitResult<()> {
        if self.phase < min {
            Err(
                InitError::new(ErrorKind::WrongPhase, "Phase not reached yet").with_details(
                    alloc::format!("Required {:?}, currently {:?}", min, self.phase),
                ),
            )
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for InitContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitContext")
            .field("phase", &self.phase)
            .field("capabilities", &self.capabilities)
            .field("resources", &self.resources.len())
            .field("current_subsystem", &self.current_subsystem)
            .field("log_entries", &self.log_buffer.len())
            .finish()
    }
}

// =============================================================================
// CONTEXT BUILDER
// =============================================================================

/// Builder for InitContext
pub struct ContextBuilder {
    phase: InitPhase,
    config: Option<Box<dyn ConfigProvider>>,
    boot_info: Option<BootInfo>,
    max_log_entries: usize,
}

impl ContextBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            phase: InitPhase::Boot,
            config: None,
            boot_info: None,
            max_log_entries: 1024,
        }
    }

    /// Set starting phase
    pub fn phase(mut self, phase: InitPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Set config provider
    pub fn config(mut self, config: Box<dyn ConfigProvider>) -> Self {
        self.config = Some(config);
        self
    }

    /// Set boot info
    pub fn boot_info(mut self, info: BootInfo) -> Self {
        self.boot_info = Some(info);
        self
    }

    /// Set max log entries
    pub fn max_log_entries(mut self, max: usize) -> Self {
        self.max_log_entries = max;
        self
    }

    /// Build the context
    pub fn build(self) -> InitContext {
        let mut ctx = InitContext::new(self.phase);

        if let Some(config) = self.config {
            ctx.set_config(config);
        }

        if let Some(boot_info) = self.boot_info {
            ctx.set_boot_info(boot_info);
        }

        ctx.max_log_entries = self.max_log_entries;

        ctx
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_handle() {
        let h1 = generate_handle();
        let h2 = generate_handle();

        assert!(h1.is_valid());
        assert!(h2.is_valid());
        assert_ne!(h1, h2);
        assert!(!ResourceHandle::INVALID.is_valid());
    }

    #[test]
    fn test_config_value() {
        let bool_val = ConfigValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_int(), None);

        let int_val = ConfigValue::Int(42);
        assert_eq!(int_val.as_int(), Some(42));
    }

    #[test]
    fn test_memory_config() {
        let mut config = MemoryConfig::new();
        config.set_bool("debug", true);
        config.set_int("timeout", 1000);
        config.set_string("name", "test");

        assert_eq!(config.get_bool("debug", false), true);
        assert_eq!(config.get_int("timeout", 0), 1000);
        assert_eq!(config.get_str("name", "default"), "test");
        assert_eq!(config.get_bool("missing", false), false);
    }

    #[test]
    fn test_context_phase() {
        let mut ctx = InitContext::new(InitPhase::Boot);

        assert_eq!(ctx.phase(), InitPhase::Boot);
        assert!(ctx.require_phase(InitPhase::Boot).is_ok());
        assert!(ctx.require_phase(InitPhase::Early).is_err());

        ctx.advance_phase().unwrap();
        assert_eq!(ctx.phase(), InitPhase::Early);
    }

    #[test]
    fn test_context_logging() {
        let mut ctx = InitContext::new(InitPhase::Boot);

        ctx.info("Test message");
        ctx.warn("Warning message");

        assert_eq!(ctx.logs().len(), 2);
        assert_eq!(ctx.logs()[0].level, LogLevel::Info);
        assert_eq!(ctx.logs()[1].level, LogLevel::Warn);
    }

    #[test]
    fn test_context_builder() {
        let ctx = ContextBuilder::new()
            .phase(InitPhase::Core)
            .max_log_entries(100)
            .build();

        assert_eq!(ctx.phase(), InitPhase::Core);
        assert_eq!(ctx.max_log_entries, 100);
    }
}
