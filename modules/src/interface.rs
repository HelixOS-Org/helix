//! # Module Interface
//!
//! Defines the communication interface between modules.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{ModuleId, ModuleResult};

/// Module message
#[derive(Debug, Clone)]
pub struct ModuleMessage {
    /// Source module ID
    pub source: ModuleId,
    /// Target module ID
    pub target: ModuleId,
    /// Message type
    pub msg_type: MessageType,
    /// Message payload
    pub payload: MessagePayload,
    /// Message ID (for request/response matching)
    pub id: u64,
    /// Is this a response to a previous message?
    pub is_response: bool,
}

impl ModuleMessage {
    /// Create a new message
    pub fn new(
        source: ModuleId,
        target: ModuleId,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> Self {
        static ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);

        Self {
            source,
            target,
            msg_type,
            payload,
            id: ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
            is_response: false,
        }
    }

    /// Create a response to this message
    pub fn response(&self, payload: MessagePayload) -> Self {
        Self {
            source: self.target,
            target: self.source,
            msg_type: self.msg_type,
            payload,
            id: self.id,
            is_response: true,
        }
    }
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Request a service from a module
    Request,
    /// Notification (no response expected)
    Notify,
    /// Query module status
    Query,
    /// Control command
    Control,
    /// Error notification
    Error,
    /// Custom message type
    Custom(u32),
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// No payload
    Empty,
    /// Raw bytes
    Bytes(Vec<u8>),
    /// String
    Text(String),
    /// Integer
    Integer(i64),
    /// Boolean
    Boolean(bool),
    /// Key-value pairs
    Map(Vec<(String, MessagePayload)>),
    /// List of payloads
    List(Vec<MessagePayload>),
}

impl MessagePayload {
    /// Get as bytes
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            MessagePayload::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MessagePayload::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Get as integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            MessagePayload::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            MessagePayload::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Module interface trait
///
/// Defines a standardized interface that modules can implement.
pub trait ModuleInterface: Send + Sync {
    /// Get the interface name
    fn interface_name(&self) -> &'static str;

    /// Get the interface version
    fn interface_version(&self) -> (u16, u16);

    /// Handle a message
    fn handle_message(&self, message: &ModuleMessage) -> ModuleResult<Option<ModuleMessage>>;

    /// Get supported operations
    fn supported_operations(&self) -> Vec<&'static str>;
}

/// Standard interface names
pub mod interfaces {
    /// Scheduler interface
    pub const SCHEDULER: &str = "helix.scheduler";
    /// Memory allocator interface
    pub const ALLOCATOR: &str = "helix.allocator";
    /// Filesystem interface
    pub const FILESYSTEM: &str = "helix.filesystem";
    /// Block device interface
    pub const BLOCK_DEVICE: &str = "helix.block_device";
    /// Network interface
    pub const NETWORK: &str = "helix.network";
    /// Security interface
    pub const SECURITY: &str = "helix.security";
}

/// Interface registry
pub struct InterfaceRegistry {
    /// Registered interfaces
    interfaces: spin::RwLock<alloc::collections::BTreeMap<String, Vec<ModuleId>>>,
}

impl InterfaceRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            interfaces: spin::RwLock::new(alloc::collections::BTreeMap::new()),
        }
    }

    /// Register a module as implementing an interface
    pub fn register(&self, interface: &str, module_id: ModuleId) {
        self.interfaces
            .write()
            .entry(interface.into())
            .or_default()
            .push(module_id);
    }

    /// Unregister a module from an interface
    pub fn unregister(&self, interface: &str, module_id: ModuleId) {
        if let Some(modules) = self.interfaces.write().get_mut(interface) {
            modules.retain(|&id| id != module_id);
        }
    }

    /// Get modules implementing an interface
    pub fn get_implementors(&self, interface: &str) -> Vec<ModuleId> {
        self.interfaces
            .read()
            .get(interface)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the primary implementor of an interface
    pub fn get_primary(&self, interface: &str) -> Option<ModuleId> {
        self.interfaces
            .read()
            .get(interface)
            .and_then(|v| v.first().copied())
    }
}

/// Global interface registry
static INTERFACE_REGISTRY: InterfaceRegistry = InterfaceRegistry::new();

/// Get the interface registry
pub fn interface_registry() -> &'static InterfaceRegistry {
    &INTERFACE_REGISTRY
}

/// Macro to implement a standard interface
#[macro_export]
macro_rules! impl_interface {
    ($struct:ty, $name:expr, $version:expr) => {
        impl $crate::interface::ModuleInterface for $struct {
            fn interface_name(&self) -> &'static str {
                $name
            }

            fn interface_version(&self) -> (u16, u16) {
                $version
            }

            fn handle_message(
                &self,
                message: &$crate::interface::ModuleMessage,
            ) -> $crate::ModuleResult<Option<$crate::interface::ModuleMessage>> {
                // Default implementation
                Ok(None)
            }

            fn supported_operations(&self) -> alloc::vec::Vec<&'static str> {
                alloc::vec![]
            }
        }
    };
}
