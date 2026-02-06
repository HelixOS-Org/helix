//! NEXUS Message Bus — Inter-Domain Communication
//!
//! The bus provides type-safe, asynchronous communication between cognitive domains.
//! It enforces the unidirectional flow and prevents tight coupling.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                           MESSAGE BUS                                │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐      │
//! │   │  sense  │────►│understand│────►│ reason  │────►│ decide  │      │
//! │   └─────────┘     └─────────┘     └─────────┘     └─────────┘      │
//! │        │               │               │               │            │
//! │        │               ▼               ▼               │            │
//! │        │          ┌─────────┐     ┌─────────┐         │            │
//! │        └─────────►│ memory  │◄────┤ reflect │◄────────┘            │
//! │                   └─────────┘     └─────────┘                       │
//! │                                                                      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Organization
//!
//! ```text
//! bus/
//! ├── domain.rs   - Domain enum and flow control
//! ├── message.rs  - Message types and payloads
//! ├── queue.rs    - Priority queue implementation
//! ├── channel.rs  - Point-to-point channels
//! ├── router.rs   - Message routing
//! └── bus.rs      - Central message bus
//! ```

#![allow(dead_code)]

// ============================================================================
// SUBMODULES
// ============================================================================

#[allow(clippy::module_inception)]
pub mod bus;
pub mod channel;
pub mod domain;
pub mod message;
pub mod queue;
pub mod router;

// ============================================================================
// RE-EXPORTS: Domain
// ============================================================================

// ============================================================================
// RE-EXPORTS: Bus
// ============================================================================
pub use bus::{BusStats, MessageBus, MessageFilter};
// ============================================================================
// RE-EXPORTS: Channel
// ============================================================================
pub use channel::{Channel, ChannelStats};
pub use domain::Domain;
// ============================================================================
// RE-EXPORTS: Message
// ============================================================================
pub use message::{
    AnomalyInfo, HealthCheckData, InsightData, KernelEventData, Message, MessagePayload,
    MessagePriority, PatternInfo, PredictionInfo, StateModelUpdate,
};
// ============================================================================
// RE-EXPORTS: Queue
// ============================================================================
pub use queue::MessageQueue;
// ============================================================================
// RE-EXPORTS: Router
// ============================================================================
pub use router::{RouteKey, Router, RouterStats};
