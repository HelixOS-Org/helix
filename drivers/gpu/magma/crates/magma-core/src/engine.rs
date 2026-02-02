//! # GPU Engine Abstractions
//!
//! Defines the various GPU engines (Graphics, Compute, Copy, etc.)

use crate::error::Result;
use crate::traits::EngineType;
use crate::types::*;

// =============================================================================
// ENGINE CONTEXT
// =============================================================================

/// Engine context state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    /// Context is inactive
    Inactive,
    /// Context is runnable
    Runnable,
    /// Context is currently executing
    Running,
    /// Context is preempted
    Preempted,
    /// Context has faulted
    Faulted,
}

/// Engine context identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextId(u32);

impl ContextId {
    /// Create a new context ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get raw ID
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Graphics engine context
#[derive(Debug)]
pub struct GraphicsContext {
    /// Context ID
    pub id: ContextId,
    /// Engine index
    pub engine_index: u32,
    /// Current state
    pub state: ContextState,
    /// Associated channel
    pub channel: ChannelHandle,
}

/// Compute engine context
#[derive(Debug)]
pub struct ComputeContext {
    /// Context ID
    pub id: ContextId,
    /// Engine index
    pub engine_index: u32,
    /// Current state
    pub state: ContextState,
    /// Associated channel
    pub channel: ChannelHandle,
    /// CUDA context flags
    pub cuda_flags: u32,
}

// =============================================================================
// ENGINE METHODS (method IDs for GSP RPC)
// =============================================================================

/// Graphics engine methods
pub mod graphics_methods {
    /// Begin render pass
    pub const BEGIN_RENDER_PASS: u32 = 0x0001;
    /// End render pass
    pub const END_RENDER_PASS: u32 = 0x0002;
    /// Bind pipeline
    pub const BIND_PIPELINE: u32 = 0x0003;
    /// Bind vertex buffer
    pub const BIND_VERTEX_BUFFER: u32 = 0x0004;
    /// Bind index buffer
    pub const BIND_INDEX_BUFFER: u32 = 0x0005;
    /// Draw
    pub const DRAW: u32 = 0x0010;
    /// Draw indexed
    pub const DRAW_INDEXED: u32 = 0x0011;
    /// Draw indirect
    pub const DRAW_INDIRECT: u32 = 0x0012;
}

/// Compute engine methods
pub mod compute_methods {
    /// Dispatch compute
    pub const DISPATCH: u32 = 0x0100;
    /// Dispatch indirect
    pub const DISPATCH_INDIRECT: u32 = 0x0101;
}

/// Copy engine methods
pub mod copy_methods {
    /// Buffer to buffer copy
    pub const COPY_BUFFER: u32 = 0x0200;
    /// Buffer to image copy
    pub const COPY_BUFFER_TO_IMAGE: u32 = 0x0201;
    /// Image to buffer copy
    pub const COPY_IMAGE_TO_BUFFER: u32 = 0x0202;
    /// Image to image copy
    pub const COPY_IMAGE: u32 = 0x0203;
    /// Fill buffer
    pub const FILL_BUFFER: u32 = 0x0204;
    /// Clear image
    pub const CLEAR_IMAGE: u32 = 0x0205;
}
