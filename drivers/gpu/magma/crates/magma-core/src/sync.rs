//! # Synchronization Primitives
//!
//! GPU fences, timeline semaphores, and synchronization utilities.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::Result;
use crate::types::*;

// =============================================================================
// FENCE
// =============================================================================

/// GPU fence for CPU-GPU synchronization
/// 
/// Fences use a monotonically increasing value stored in GPU-visible memory.
/// The CPU can poll or wait for the fence value to reach a target.
#[derive(Debug)]
pub struct Fence {
    /// GPU address of fence memory
    gpu_addr: GpuAddr,
    /// CPU mapping (if available)
    cpu_ptr: Option<*mut u64>,
    /// Last signaled value (cached)
    last_signaled: u64,
    /// Next value to signal
    next_value: u64,
}

impl Fence {
    /// Create a new fence
    /// 
    /// # Safety
    /// - `gpu_addr` must be a valid GPU address
    /// - `cpu_ptr` must be a valid pointer if Some
    pub unsafe fn new(gpu_addr: GpuAddr, cpu_ptr: Option<*mut u64>) -> Self {
        Self {
            gpu_addr,
            cpu_ptr,
            last_signaled: 0,
            next_value: 1,
        }
    }

    /// Get GPU address
    pub fn gpu_addr(&self) -> GpuAddr {
        self.gpu_addr
    }

    /// Get next signal value
    pub fn next_value(&self) -> u64 {
        self.next_value
    }

    /// Advance to next value (returns value to signal)
    pub fn advance(&mut self) -> u64 {
        let val = self.next_value;
        self.next_value += 1;
        val
    }

    /// Check if value has been signaled
    pub fn is_signaled(&mut self, value: u64) -> bool {
        // Check cached value first
        if value <= self.last_signaled {
            return true;
        }

        // Read from memory
        if let Some(ptr) = self.cpu_ptr {
            // SAFETY: cpu_ptr validity guaranteed by constructor contract
            let current = unsafe { ptr.read_volatile() };
            self.last_signaled = current;
            value <= current
        } else {
            false
        }
    }

    /// Spin-wait for value to be signaled
    pub fn wait(&mut self, value: u64) {
        while !self.is_signaled(value) {
            core::hint::spin_loop();
        }
    }
}

// SAFETY: Fence can be sent between threads
unsafe impl Send for Fence {}

// =============================================================================
// TIMELINE SEMAPHORE
// =============================================================================

/// Timeline semaphore for multi-point synchronization
/// 
/// Unlike binary semaphores, timeline semaphores have an integer value
/// that can be waited on or signaled to specific points.
#[derive(Debug)]
pub struct TimelineSemaphore {
    /// GPU address
    gpu_addr: GpuAddr,
    /// CPU-visible value (atomic for lock-free access)
    value: AtomicU64,
}

impl TimelineSemaphore {
    /// Create a new timeline semaphore
    pub fn new(gpu_addr: GpuAddr, initial_value: u64) -> Self {
        Self {
            gpu_addr,
            value: AtomicU64::new(initial_value),
        }
    }

    /// Get current value
    pub fn get_value(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Signal to a value
    pub fn signal(&self, value: u64) {
        self.value.store(value, Ordering::Release);
    }

    /// Check if value has been reached
    pub fn is_reached(&self, value: u64) -> bool {
        self.get_value() >= value
    }

    /// Get GPU address
    pub fn gpu_addr(&self) -> GpuAddr {
        self.gpu_addr
    }
}

// =============================================================================
// EVENT
// =============================================================================

/// GPU event for GPU-GPU synchronization
#[derive(Debug, Clone, Copy)]
pub struct Event {
    /// GPU address
    pub gpu_addr: GpuAddr,
    /// Event status value
    pub status: EventStatus,
}

/// Event status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EventStatus {
    /// Event not yet signaled
    Reset = 0,
    /// Event signaled
    Set = 1,
}

// =============================================================================
// BARRIER
// =============================================================================

/// Memory barrier types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierType {
    /// Full memory barrier
    Full,
    /// Texture barrier
    Texture,
    /// Buffer barrier
    Buffer,
    /// Shader storage barrier
    ShaderStorage,
    /// Framebuffer barrier
    Framebuffer,
}

/// Pipeline stage flags for barriers
bitflags::bitflags! {
    /// Pipeline stages that can be synchronized
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PipelineStage: u32 {
        /// Top of pipe (beginning)
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control
        const TESS_CONTROL = 1 << 4;
        /// Tessellation evaluation
        const TESS_EVAL = 1 << 5;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests
        const EARLY_FRAGMENT = 1 << 8;
        /// Late fragment tests
        const LATE_FRAGMENT = 1 << 9;
        /// Color attachment output
        const COLOR_ATTACHMENT = 1 << 10;
        /// Compute shader
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer operations
        const TRANSFER = 1 << 12;
        /// Bottom of pipe (end)
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host operations
        const HOST = 1 << 14;
        /// All graphics stages
        const ALL_GRAPHICS = Self::VERTEX_INPUT.bits()
            | Self::VERTEX_SHADER.bits()
            | Self::TESS_CONTROL.bits()
            | Self::TESS_EVAL.bits()
            | Self::GEOMETRY_SHADER.bits()
            | Self::FRAGMENT_SHADER.bits()
            | Self::EARLY_FRAGMENT.bits()
            | Self::LATE_FRAGMENT.bits()
            | Self::COLOR_ATTACHMENT.bits();
        /// All commands
        const ALL_COMMANDS = 0x7FFF;
    }
}

/// Access flags for memory barriers
bitflags::bitflags! {
    /// Memory access types
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u32 {
        /// Indirect command read
        const INDIRECT_COMMAND_READ = 1 << 0;
        /// Index read
        const INDEX_READ = 1 << 1;
        /// Vertex attribute read
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        /// Uniform read
        const UNIFORM_READ = 1 << 3;
        /// Shader read
        const SHADER_READ = 1 << 4;
        /// Shader write
        const SHADER_WRITE = 1 << 5;
        /// Color attachment read
        const COLOR_ATTACHMENT_READ = 1 << 6;
        /// Color attachment write
        const COLOR_ATTACHMENT_WRITE = 1 << 7;
        /// Depth/stencil read
        const DEPTH_STENCIL_READ = 1 << 8;
        /// Depth/stencil write
        const DEPTH_STENCIL_WRITE = 1 << 9;
        /// Transfer read
        const TRANSFER_READ = 1 << 10;
        /// Transfer write
        const TRANSFER_WRITE = 1 << 11;
        /// Host read
        const HOST_READ = 1 << 12;
        /// Host write
        const HOST_WRITE = 1 << 13;
        /// Memory read
        const MEMORY_READ = 1 << 14;
        /// Memory write
        const MEMORY_WRITE = 1 << 15;
    }
}

/// Memory barrier descriptor
#[derive(Debug, Clone, Copy)]
pub struct MemoryBarrier {
    /// Source pipeline stage
    pub src_stage: PipelineStage,
    /// Destination pipeline stage
    pub dst_stage: PipelineStage,
    /// Source access flags
    pub src_access: AccessFlags,
    /// Destination access flags
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Full pipeline barrier
    pub const fn full() -> Self {
        Self {
            src_stage: PipelineStage::ALL_COMMANDS,
            dst_stage: PipelineStage::ALL_COMMANDS,
            src_access: AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::MEMORY_READ,
        }
    }

    /// Compute to graphics barrier
    pub const fn compute_to_graphics() -> Self {
        Self {
            src_stage: PipelineStage::COMPUTE_SHADER,
            dst_stage: PipelineStage::ALL_GRAPHICS,
            src_access: AccessFlags::SHADER_WRITE,
            dst_access: AccessFlags::SHADER_READ,
        }
    }
}
