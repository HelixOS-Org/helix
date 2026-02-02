//! # Helix-GL: OpenGL Translation Layer
//!
//! OpenGL 3.3+ Core Profile implementation on top of Magma Vulkan.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Helix-GL Layer                            │
//! │                                                              │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
//! │  │ Context  │  │  State   │  │  Shader  │  │ Resource │    │
//! │  │ Manager  │  │ Tracker  │  │ Compiler │  │ Manager  │    │
//! │  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
//! │                        │                                     │
//! │                        ▼                                     │
//! │              ┌──────────────────┐                           │
//! │              │ Command Builder  │                           │
//! │              └──────────────────┘                           │
//! │                        │                                     │
//! └────────────────────────┼────────────────────────────────────┘
//!                          ▼
//!                   Magma Vulkan Driver
//! ```

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate alloc;

pub mod buffer;
pub mod context;
pub mod dispatch;
pub mod enums;
pub mod framebuffer;
pub mod pipeline;
pub mod shader;
pub mod state;
pub mod texture;
pub mod types;

// Re-exports
pub use context::GlContext;
pub use enums::*;
pub use state::GlState;
pub use types::*;
