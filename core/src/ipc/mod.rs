//! # Helix IPC System
//!
//! Inter-Process Communication infrastructure for the Helix kernel.
//!
//! ## Components
//!
//! - **Event Bus**: Broadcast events to subscribed modules
//! - **Message Router**: Point-to-point messaging between modules  
//! - **Channels**: Low-level bounded channels for data transfer
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_core::ipc::{EventBus, MessageRouter, channel};
//! use helix_core::ipc::event_bus::{Event, EventSubscription, EventTopic, EventResponse};
//!
//! // Subscribe to events
//! let sub = EventSubscription::new(
//!     "my_module",
//!     vec![EventTopic::Tick],
//!     Box::new(|event| EventResponse::Handled),
//! );
//! global_event_bus().subscribe(sub);
//!
//! // Broadcast an event
//! global_event_bus().publish(Event::Tick {
//!     timestamp_ns: 0,
//!     tick_number: 1
//! });
//!
//! // Send a request to a module
//! let request = Request::new(
//!     ModuleAddress::Name("sender".into()),
//!     "get_stats",
//! );
//! let response = global_router().send(&ModuleAddress::Name("scheduler".into()), request)?;
//!
//! // Create a channel
//! let (tx, rx) = channel::<u32>(16);
//! tx.send(42)?;
//! let value = rx.try_recv()?;
//! ```

pub mod channel;
pub mod event_bus;
pub mod message_router;

// Re-export main types
pub use channel::{
    channel, default_channel, oneshot, ChannelStats, OneShotReceiver, OneShotSender, Receiver,
    Sender,
};
pub use event_bus::{
    global_event_bus, publish_event, subscribe, Event, EventBus, EventDispatchResult,
    EventResponse, EventSubscription, EventTopic, MemoryPressureLevel, SubscriptionId,
    SubscriptionPriority,
};
pub use message_router::{
    global_router, send_request, send_to, MessageEnvelope, MessageHandler, MessageId,
    MessagePriority, MessageRouter, ModuleAddress, Request, Response,
};

/// IPC Error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    /// Target module not found
    ModuleNotFound,
    /// Channel is full
    ChannelFull,
    /// Channel is closed
    ChannelClosed,
    /// Timeout waiting for response
    Timeout,
    /// Module rejected the message
    Rejected,
    /// Internal error
    Internal(&'static str),
}

impl core::fmt::Display for IpcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ModuleNotFound => write!(f, "Module not found"),
            Self::ChannelFull => write!(f, "Channel is full"),
            Self::ChannelClosed => write!(f, "Channel is closed"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Rejected => write!(f, "Message rejected"),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Result type for IPC operations
pub type IpcResult<T> = Result<T, IpcError>;
