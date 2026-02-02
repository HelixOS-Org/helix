//! # MAGMA GSP RPC Protocol
//!
//! GSP (GPU System Processor) RPC implementation for NVIDIA GPUs.
//!
//! ## Architecture
//!
//! Modern NVIDIA GPUs (Turing+) use a GSP to handle most GPU management tasks.
//! The host driver communicates with the GSP via RPC messages sent through
//! shared memory queues.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                        GSP-First Architecture                │
//! │                                                              │
//! │  ┌────────────────┐    RPC Messages    ┌─────────────────┐  │
//! │  │   Host Driver  │ ←──────────────→   │       GSP       │  │
//! │  │   (MAGMA)      │                    │    Firmware     │  │
//! │  └────────────────┘                    └─────────────────┘  │
//! │          │                                      │           │
//! │          │                                      │           │
//! │  ┌───────┴───────────────────────────────┬─────┴────────┐  │
//! │  │          Shared Memory Queues          │              │  │
//! │  │  ┌─────────────┐    ┌─────────────┐   │   GPU HW     │  │
//! │  │  │  Command Q  │    │  Response Q │   │   Engines    │  │
//! │  │  └─────────────┘    └─────────────┘   │              │  │
//! │  └───────────────────────────────────────┴──────────────┘  │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Message Flow
//!
//! 1. Host writes RPC message to command queue
//! 2. Host notifies GSP via doorbell register
//! 3. GSP processes command and writes response
//! 4. GSP signals completion via interrupt or polling
//! 5. Host reads response from response queue

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod channel;
pub mod gsp;
pub mod message;
pub mod queue;
pub mod transport;

// Re-exports
pub use channel::{RpcChannel, RpcChannelId};
pub use gsp::{GspState, GspInfo};
pub use message::{RpcMessage, RpcHeader, RpcResult};
pub use queue::{CommandQueue, ResponseQueue};
pub use transport::Transport;
