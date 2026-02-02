//! # Push Buffer
//!
//! GPU push buffer encoding for NVIDIA GPUs.

use alloc::vec::Vec;

use magma_core::{Error, Result, GpuAddr, ByteSize};
use magma_core::command::{GpuMethod, MethodType};

// =============================================================================
// PUSH BUFFER
// =============================================================================

/// A push buffer for GPU command encoding
#[derive(Debug)]
pub struct PushBuffer {
    /// GPU address
    gpu_addr: GpuAddr,
    /// CPU-side data
    data: Vec<u32>,
    /// Maximum capacity in dwords
    capacity: usize,
}

impl PushBuffer {
    /// Create a new push buffer
    pub fn new(gpu_addr: GpuAddr, capacity: ByteSize) -> Self {
        let cap_dwords = capacity.as_bytes() as usize / 4;
        Self {
            gpu_addr,
            data: Vec::with_capacity(cap_dwords),
            capacity: cap_dwords,
        }
    }

    /// Get GPU address
    pub fn gpu_addr(&self) -> GpuAddr {
        self.gpu_addr
    }

    /// Get current size in bytes
    pub fn size(&self) -> usize {
        self.data.len() * 4
    }

    /// Get remaining space in dwords
    pub fn remaining(&self) -> usize {
        self.capacity - self.data.len()
    }

    /// Check if buffer has space for n dwords
    pub fn has_space(&self, dwords: usize) -> bool {
        self.remaining() >= dwords
    }

    /// Push a single dword
    pub fn push(&mut self, value: u32) -> Result<()> {
        if !self.has_space(1) {
            return Err(Error::OutOfMemory);
        }
        self.data.push(value);
        Ok(())
    }

    /// Push method header + data
    pub fn push_method(&mut self, method: GpuMethod, data: &[u32]) -> Result<()> {
        if !self.has_space(1 + data.len()) {
            return Err(Error::OutOfMemory);
        }

        self.data.push(method.as_u32());
        self.data.extend_from_slice(data);
        Ok(())
    }

    /// Push increasing method (address increments)
    pub fn push_increasing(&mut self, method_id: u16, subchannel: u8, data: &[u32]) -> Result<()> {
        let method = GpuMethod::increasing(method_id, subchannel, data.len() as u16);
        self.push_method(method, data)
    }

    /// Push non-increasing method (same address)
    pub fn push_non_increasing(&mut self, method_id: u16, subchannel: u8, data: &[u32]) -> Result<()> {
        let method = GpuMethod::non_increasing(method_id, subchannel, data.len() as u16);
        self.push_method(method, data)
    }

    /// Push single value to method
    pub fn push_single(&mut self, method_id: u16, subchannel: u8, value: u32) -> Result<()> {
        self.push_increasing(method_id, subchannel, &[value])
    }

    /// Get buffer data
    pub fn data(&self) -> &[u32] {
        &self.data
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Reset with new GPU address
    pub fn reset(&mut self, gpu_addr: GpuAddr) {
        self.gpu_addr = gpu_addr;
        self.data.clear();
    }
}

// =============================================================================
// PUSH STREAM
// =============================================================================

/// A streaming interface for building push buffers
#[derive(Debug)]
pub struct PushStream<'a> {
    /// Target buffer
    buffer: &'a mut PushBuffer,
    /// Current subchannel
    subchannel: u8,
}

impl<'a> PushStream<'a> {
    /// Create a new push stream
    pub fn new(buffer: &'a mut PushBuffer) -> Self {
        Self {
            buffer,
            subchannel: 0,
        }
    }

    /// Set current subchannel
    pub fn subchannel(&mut self, sc: u8) -> &mut Self {
        self.subchannel = sc;
        self
    }

    /// Push method with data
    pub fn method(&mut self, id: u16, data: &[u32]) -> Result<&mut Self> {
        self.buffer.push_increasing(id, self.subchannel, data)?;
        Ok(self)
    }

    /// Push single value
    pub fn write(&mut self, id: u16, value: u32) -> Result<&mut Self> {
        self.buffer.push_single(id, self.subchannel, value)?;
        Ok(self)
    }

    /// Push multiple values to same address
    pub fn write_repeat(&mut self, id: u16, values: &[u32]) -> Result<&mut Self> {
        self.buffer.push_non_increasing(id, self.subchannel, values)?;
        Ok(self)
    }

    /// Get remaining space
    pub fn remaining(&self) -> usize {
        self.buffer.remaining()
    }
}

// =============================================================================
// COMMON GPU METHODS
// =============================================================================

/// Common GPU method IDs
pub mod methods {
    //! GPU method constants

    /// NOP (no operation)
    pub const NOP: u16 = 0x0100;

    /// Semaphore offset high
    pub const SEMAPHORE_OFFSET_HI: u16 = 0x0010;
    /// Semaphore offset low
    pub const SEMAPHORE_OFFSET_LO: u16 = 0x0014;
    /// Semaphore payload
    pub const SEMAPHORE_PAYLOAD: u16 = 0x0018;
    /// Semaphore operation
    pub const SEMAPHORE_EXECUTE: u16 = 0x001C;

    /// Fence address high
    pub const FENCE_ADDR_HI: u16 = 0x0020;
    /// Fence address low
    pub const FENCE_ADDR_LO: u16 = 0x0024;
    /// Fence value
    pub const FENCE_VALUE: u16 = 0x0028;

    /// Wait for idle
    pub const WAIT_FOR_IDLE: u16 = 0x0110;
    /// PM trigger
    pub const PM_TRIGGER: u16 = 0x0140;
    /// Set object
    pub const SET_OBJECT: u16 = 0x0000;
}

/// Semaphore operation flags
pub mod semaphore_op {
    //! Semaphore operation constants

    /// Release (signal)
    pub const RELEASE: u32 = 0x00000001;
    /// Acquire (wait)
    pub const ACQUIRE: u32 = 0x00000002;
    /// Write payload
    pub const WRITE_PAYLOAD: u32 = 0x00000004;
    /// Reduction: increment
    pub const REDUCTION_INC: u32 = 0x00000010;
}
