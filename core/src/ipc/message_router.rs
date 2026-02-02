//! # Message Router
//!
//! Point-to-point messaging system for direct module communication.
//!
//! ## Features
//!
//! - Direct addressing by module ID
//! - Request/Response pattern with timeout
//! - Message queuing per module
//! - Priority-based delivery

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::RwLock;

use super::{IpcError, IpcResult};

// =============================================================================
// Message Types
// =============================================================================

/// Unique message identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(u64);

impl MessageId {
    /// Generate a new unique message ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get raw value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Low priority
    Low      = 0,
    /// Normal priority
    Normal   = 1,
    /// High priority
    High     = 2,
    /// Critical priority (system messages)
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Module address for routing
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleAddress {
    /// Address by module ID
    Id(u64),
    /// Address by module name
    Name(String),
    /// Broadcast to all modules
    Broadcast,
}

/// Request message
#[derive(Debug, Clone)]
pub struct Request {
    /// Message ID
    pub id: MessageId,
    /// Sender address
    pub from: ModuleAddress,
    /// Request type
    pub request_type: String,
    /// Request payload
    pub payload: Vec<u8>,
    /// Priority
    pub priority: MessagePriority,
}

impl Request {
    /// Create a new request
    pub fn new(from: ModuleAddress, request_type: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            from,
            request_type: request_type.into(),
            payload: Vec::new(),
            priority: MessagePriority::Normal,
        }
    }

    /// Set payload
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Response to a request
#[derive(Debug, Clone)]
pub enum Response {
    /// Success with optional data
    Success(Vec<u8>),
    /// Request was rejected
    Rejected(String),
    /// Error occurred
    Error(String),
    /// Not supported by this module
    NotSupported,
}

impl Response {
    /// Create success response with data
    pub fn success(data: impl Into<Vec<u8>>) -> Self {
        Self::Success(data.into())
    }

    /// Create empty success response
    pub fn ok() -> Self {
        Self::Success(Vec::new())
    }

    /// Create rejected response
    pub fn rejected(reason: impl Into<String>) -> Self {
        Self::Rejected(reason.into())
    }

    /// Create error response
    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error(msg.into())
    }

    /// Check if response is success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
}

/// Message envelope for queuing
pub struct MessageEnvelope {
    /// Destination
    pub to: ModuleAddress,
    /// The request
    pub request: Request,
    /// Response channel (callback)
    pub response_callback: Option<Box<dyn FnOnce(Response) + Send + Sync>>,
}

impl core::fmt::Debug for MessageEnvelope {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MessageEnvelope")
            .field("to", &self.to)
            .field("request", &self.request)
            .field("has_callback", &self.response_callback.is_some())
            .finish()
    }
}

// =============================================================================
// Message Queue
// =============================================================================

/// Per-module message queue
struct ModuleMailbox {
    /// Pending messages
    messages: Vec<MessageEnvelope>,
    /// Maximum queue size
    max_size: usize,
    /// Messages received
    received: u64,
    /// Messages processed
    processed: u64,
}

impl ModuleMailbox {
    fn new(max_size: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_size,
            received: 0,
            processed: 0,
        }
    }

    fn push(&mut self, envelope: MessageEnvelope) -> IpcResult<()> {
        if self.messages.len() >= self.max_size {
            return Err(IpcError::ChannelFull);
        }

        // Insert by priority (higher priority first)
        let priority = envelope.request.priority;
        let pos = self
            .messages
            .iter()
            .position(|m| m.request.priority < priority)
            .unwrap_or(self.messages.len());

        self.messages.insert(pos, envelope);
        self.received += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<MessageEnvelope> {
        if let Some(envelope) = self.messages.pop() {
            self.processed += 1;
            Some(envelope)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.messages.len()
    }
}

// =============================================================================
// Message Router
// =============================================================================

/// Message handler function type
pub type MessageHandler = Box<dyn Fn(&Request) -> Response + Send + Sync>;

/// Module registration in the router
struct RegisteredModule {
    /// Module ID
    id: u64,
    /// Module name
    name: String,
    /// Message handler
    handler: MessageHandler,
    /// Mailbox
    mailbox: ModuleMailbox,
}

/// Central message router
pub struct MessageRouter {
    /// Registered modules by ID
    modules_by_id: RwLock<BTreeMap<u64, RegisteredModule>>,
    /// Module ID to name mapping
    name_to_id: RwLock<BTreeMap<String, u64>>,
    /// Default mailbox size
    default_mailbox_size: usize,
    /// Total messages routed
    messages_routed: AtomicU64,
}

impl MessageRouter {
    /// Create a new message router
    pub const fn new() -> Self {
        Self {
            modules_by_id: RwLock::new(BTreeMap::new()),
            name_to_id: RwLock::new(BTreeMap::new()),
            default_mailbox_size: 64,
            messages_routed: AtomicU64::new(0),
        }
    }

    /// Register a module with the router
    pub fn register(
        &self,
        id: u64,
        name: impl Into<String>,
        handler: MessageHandler,
    ) -> IpcResult<()> {
        let name = name.into();

        let mut modules = self.modules_by_id.write();
        let mut names = self.name_to_id.write();

        if modules.contains_key(&id) {
            return Err(IpcError::Internal("Module already registered"));
        }

        if names.contains_key(&name) {
            return Err(IpcError::Internal("Module name already taken"));
        }

        log::debug!("MessageRouter: Registering module {} ({})", name, id);

        names.insert(name.clone(), id);
        modules.insert(id, RegisteredModule {
            id,
            name,
            handler,
            mailbox: ModuleMailbox::new(self.default_mailbox_size),
        });

        Ok(())
    }

    /// Unregister a module
    pub fn unregister(&self, id: u64) -> bool {
        let mut modules = self.modules_by_id.write();
        let mut names = self.name_to_id.write();

        if let Some(module) = modules.remove(&id) {
            names.remove(&module.name);
            log::debug!("MessageRouter: Unregistered module {}", module.name);
            true
        } else {
            false
        }
    }

    /// Send a request and get immediate response
    pub fn send(&self, to: &ModuleAddress, request: Request) -> IpcResult<Response> {
        self.messages_routed.fetch_add(1, Ordering::Relaxed);

        match to {
            ModuleAddress::Id(id) => {
                let modules = self.modules_by_id.read();
                if let Some(module) = modules.get(id) {
                    Ok((module.handler)(&request))
                } else {
                    Err(IpcError::ModuleNotFound)
                }
            },
            ModuleAddress::Name(name) => {
                let names = self.name_to_id.read();
                if let Some(&id) = names.get(name) {
                    drop(names);
                    let modules = self.modules_by_id.read();
                    if let Some(module) = modules.get(&id) {
                        Ok((module.handler)(&request))
                    } else {
                        Err(IpcError::ModuleNotFound)
                    }
                } else {
                    Err(IpcError::ModuleNotFound)
                }
            },
            ModuleAddress::Broadcast => {
                // For broadcast, we don't return a response
                Err(IpcError::Internal("Use broadcast() for broadcast messages"))
            },
        }
    }

    /// Broadcast a request to all modules
    pub fn broadcast(&self, request: &Request) -> Vec<(u64, Response)> {
        self.messages_routed.fetch_add(1, Ordering::Relaxed);

        let modules = self.modules_by_id.read();
        let mut responses = Vec::new();

        for (id, module) in modules.iter() {
            let response = (module.handler)(request);
            responses.push((*id, response));
        }

        responses
    }

    /// Queue a message for later processing
    pub fn queue(&self, to: &ModuleAddress, envelope: MessageEnvelope) -> IpcResult<()> {
        match to {
            ModuleAddress::Id(id) => {
                let mut modules = self.modules_by_id.write();
                if let Some(module) = modules.get_mut(id) {
                    module.mailbox.push(envelope)
                } else {
                    Err(IpcError::ModuleNotFound)
                }
            },
            ModuleAddress::Name(name) => {
                let names = self.name_to_id.read();
                if let Some(&id) = names.get(name) {
                    drop(names);
                    let mut modules = self.modules_by_id.write();
                    if let Some(module) = modules.get_mut(&id) {
                        module.mailbox.push(envelope)
                    } else {
                        Err(IpcError::ModuleNotFound)
                    }
                } else {
                    Err(IpcError::ModuleNotFound)
                }
            },
            ModuleAddress::Broadcast => Err(IpcError::Internal("Cannot queue broadcast messages")),
        }
    }

    /// Process pending messages for a module
    pub fn process_mailbox(&self, id: u64, max_count: usize) -> usize {
        let mut modules = self.modules_by_id.write();

        if let Some(module) = modules.get_mut(&id) {
            let mut processed = 0;

            while processed < max_count {
                if let Some(envelope) = module.mailbox.pop() {
                    let response = (module.handler)(&envelope.request);

                    if let Some(callback) = envelope.response_callback {
                        callback(response);
                    }

                    processed += 1;
                } else {
                    break;
                }
            }

            processed
        } else {
            0
        }
    }

    /// Get mailbox size for a module
    pub fn mailbox_size(&self, id: u64) -> Option<usize> {
        self.modules_by_id.read().get(&id).map(|m| m.mailbox.len())
    }

    /// Get total messages routed
    pub fn messages_routed(&self) -> u64 {
        self.messages_routed.load(Ordering::Relaxed)
    }

    /// Get number of registered modules
    pub fn module_count(&self) -> usize {
        self.modules_by_id.read().len()
    }

    /// Check if a module is registered
    pub fn is_registered(&self, id: u64) -> bool {
        self.modules_by_id.read().contains_key(&id)
    }

    /// Get module ID by name
    pub fn get_module_id(&self, name: &str) -> Option<u64> {
        self.name_to_id.read().get(name).copied()
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Message Router
// =============================================================================

use spin::Once;

static GLOBAL_ROUTER: Once<MessageRouter> = Once::new();

/// Get the global message router
pub fn global_router() -> &'static MessageRouter {
    GLOBAL_ROUTER.call_once(MessageRouter::new)
}

/// Convenience function to send a request
pub fn send_request(to: &ModuleAddress, request: Request) -> IpcResult<Response> {
    global_router().send(to, request)
}

/// Convenience function to send to a module by name
pub fn send_to(name: &str, request: Request) -> IpcResult<Response> {
    global_router().send(&ModuleAddress::Name(name.into()), request)
}
