//! # MAGMA Command System
//!
//! GPU command ring, push buffers, and submission infrastructure.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                    Command Submission Pipeline                    │
//! │                                                                   │
//! │  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐   │
//! │  │   Command    │    │    Push      │    │   GPU Channel    │   │
//! │  │   Buffer     │───▶│   Buffer     │───▶│   (FIFO/Ring)    │   │
//! │  │  (Recording) │    │  (Encoded)   │    │                  │   │
//! │  └──────────────┘    └──────────────┘    └────────┬─────────┘   │
//! │                                                    │             │
//! │                                           ┌────────▼─────────┐   │
//! │                                           │   GPU Engine     │   │
//! │                                           │  (GR/CE/NVDEC)   │   │
//! │                                           └──────────────────┘   │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Submission Flow
//!
//! 1. Application records commands into CommandBuffer
//! 2. CommandBuffer is compiled into GPU push buffer format
//! 3. Push buffer is submitted to a GPU channel
//! 4. GPU engine processes commands from the channel
//! 5. Fence signals completion

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod buffer;
pub mod channel;
pub mod encoder;
pub mod pushbuf;
pub mod ring;
pub mod submit;

// Re-exports
pub use buffer::{CommandBuffer, CommandBufferState};
pub use channel::{ChannelId, GpuChannel};
pub use pushbuf::{PushBuffer, PushStream};
pub use ring::{CommandRing, RingConfig};
pub use submit::{Submission, SubmitFlags};
