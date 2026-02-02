//! # Command Primitives
//!
//! Command buffers, submission queues, and push buffer abstractions.

use crate::engine::ContextId;
use crate::error::Result;
use crate::types::*;

// =============================================================================
// COMMAND BUFFER STATE
// =============================================================================

/// Command buffer recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// Initial state, ready for recording
    Initial,
    /// Currently recording commands
    Recording,
    /// Recording complete, ready for submission
    Executable,
    /// Submitted to GPU, pending execution
    Pending,
    /// Execution complete, can be reset
    Complete,
    /// Invalid state (error occurred)
    Invalid,
}

// =============================================================================
// PUSH BUFFER
// =============================================================================

/// NVIDIA GPU push buffer (FIFO command stream)
/// 
/// Push buffers are the fundamental command submission mechanism for NVIDIA GPUs.
/// Commands are 32-bit method/data pairs that get pushed to a FIFO and consumed
/// by the GPU engines.
#[derive(Debug)]
pub struct PushBuffer {
    /// GPU address of the push buffer
    pub gpu_addr: GpuAddr,
    /// Size in bytes
    pub size: ByteSize,
    /// Current write position (in dwords)
    pub put: u32,
    /// GPU read position (in dwords)
    pub get: u32,
    /// Maximum entries
    pub capacity: u32,
}

impl PushBuffer {
    /// Check if push buffer has space for n dwords
    pub fn has_space(&self, dwords: u32) -> bool {
        self.space_available() >= dwords
    }

    /// Get available space in dwords
    pub fn space_available(&self) -> u32 {
        if self.put >= self.get {
            self.capacity - self.put + self.get
        } else {
            self.get - self.put
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.put == self.get
    }
}

// =============================================================================
// GPU METHOD ENCODING
// =============================================================================

/// GPU method encoding for push buffers
/// 
/// NVIDIA GPUs use a specific encoding for method calls:
/// - Method ID: 13 bits (engine-specific register address)
/// - Subchannel: 3 bits (selects engine within channel)
/// - Count: 13 bits (number of data dwords following)
/// - Type: 3 bits (method type: increasing, non-increasing, etc.)
#[derive(Debug, Clone, Copy)]
pub struct GpuMethod {
    /// Raw encoded dword
    raw: u32,
}

/// Method type (how address increments)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MethodType {
    /// Increasing: address increments for each data word
    Increasing = 0,
    /// Non-increasing: address stays the same
    NonIncreasing = 3,
    /// Inline: data embedded in the method word
    Inline = 4,
    /// Increase once: increment by 1 after all data
    IncreaseOnce = 5,
}

impl GpuMethod {
    /// Create a new GPU method
    pub const fn new(
        method_id: u16,
        subchannel: u8,
        count: u16,
        method_type: MethodType,
    ) -> Self {
        let raw = ((method_type as u32) << 29)
            | ((count as u32 & 0x1FFF) << 16)
            | ((subchannel as u32 & 0x7) << 13)
            | (method_id as u32 & 0x1FFF);
        Self { raw }
    }

    /// Create increasing method
    pub const fn increasing(method_id: u16, subchannel: u8, count: u16) -> Self {
        Self::new(method_id, subchannel, count, MethodType::Increasing)
    }

    /// Create non-increasing method
    pub const fn non_increasing(method_id: u16, subchannel: u8, count: u16) -> Self {
        Self::new(method_id, subchannel, count, MethodType::NonIncreasing)
    }

    /// Get raw encoded value
    pub const fn as_u32(self) -> u32 {
        self.raw
    }
}

// =============================================================================
// SEMAPHORE
// =============================================================================

/// GPU semaphore for synchronization
#[derive(Debug, Clone, Copy)]
pub struct GpuSemaphore {
    /// GPU address of semaphore memory
    pub addr: GpuAddr,
    /// Expected/signal value
    pub value: u64,
}

/// Semaphore operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreOp {
    /// Wait for semaphore >= value
    WaitGe,
    /// Wait for semaphore == value
    WaitEq,
    /// Signal semaphore to value
    Signal,
    /// Increment semaphore
    Increment,
}

// =============================================================================
// SUBMISSION
// =============================================================================

/// Command submission descriptor
#[derive(Debug)]
pub struct SubmitDesc {
    /// Target context
    pub context: ContextId,
    /// GPU address of command buffer
    pub commands: GpuAddr,
    /// Size of commands in bytes
    pub size: ByteSize,
    /// Semaphores to wait on before execution
    pub wait_semaphores: &'static [GpuSemaphore],
    /// Semaphores to signal after execution
    pub signal_semaphores: &'static [GpuSemaphore],
    /// Fence value to signal on completion
    pub fence_value: u64,
}

// =============================================================================
// WORK SUBMISSION TOKEN
// =============================================================================

/// Token returned from command submission
/// 
/// Used to track submission status and wait for completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionToken {
    /// Unique submission ID
    pub id: u64,
    /// Fence value to wait for
    pub fence_value: u64,
}

impl SubmissionToken {
    /// Create a new submission token
    pub const fn new(id: u64, fence_value: u64) -> Self {
        Self { id, fence_value }
    }
}

// =============================================================================
// COMPUTE DISPATCH
// =============================================================================

/// Compute dispatch parameters
#[derive(Debug, Clone, Copy)]
pub struct DispatchParams {
    /// Workgroup count X
    pub groups_x: u32,
    /// Workgroup count Y
    pub groups_y: u32,
    /// Workgroup count Z
    pub groups_z: u32,
}

impl DispatchParams {
    /// Create 1D dispatch
    pub const fn new_1d(groups_x: u32) -> Self {
        Self {
            groups_x,
            groups_y: 1,
            groups_z: 1,
        }
    }

    /// Create 2D dispatch
    pub const fn new_2d(groups_x: u32, groups_y: u32) -> Self {
        Self {
            groups_x,
            groups_y,
            groups_z: 1,
        }
    }

    /// Create 3D dispatch
    pub const fn new_3d(groups_x: u32, groups_y: u32, groups_z: u32) -> Self {
        Self {
            groups_x,
            groups_y,
            groups_z,
        }
    }
}

// =============================================================================
// DRAW PARAMETERS
// =============================================================================

/// Draw call parameters
#[derive(Debug, Clone, Copy)]
pub struct DrawParams {
    /// Number of vertices
    pub vertex_count: u32,
    /// Number of instances
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawParams {
    /// Simple draw with N vertices
    pub const fn vertices(count: u32) -> Self {
        Self {
            vertex_count: count,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

/// Indexed draw call parameters
#[derive(Debug, Clone, Copy)]
pub struct DrawIndexedParams {
    /// Number of indices
    pub index_count: u32,
    /// Number of instances
    pub instance_count: u32,
    /// First index
    pub first_index: u32,
    /// Vertex offset
    pub vertex_offset: i32,
    /// First instance
    pub first_instance: u32,
}
