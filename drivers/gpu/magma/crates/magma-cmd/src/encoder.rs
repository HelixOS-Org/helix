//! # Command Encoder
//!
//! Encode high-level commands to GPU push buffer format.

use magma_core::{Error, Result, GpuAddr};

use crate::buffer::RecordedCommand;
use crate::pushbuf::PushBuffer;

// =============================================================================
// ENCODER TRAIT
// =============================================================================

/// Trait for command encoders
pub trait CommandEncoder {
    /// Encode a command to push buffer
    fn encode(&self, cmd: &RecordedCommand, pb: &mut PushBuffer) -> Result<()>;

    /// Get encoder name
    fn name(&self) -> &'static str;
}

// =============================================================================
// GRAPHICS ENCODER
// =============================================================================

/// Graphics command encoder
#[derive(Debug)]
pub struct GraphicsEncoder {
    /// Engine class
    class: u32,
}

impl GraphicsEncoder {
    /// Create encoder for Turing
    pub fn turing() -> Self {
        Self { class: 0xC597 }
    }

    /// Create encoder for Ampere
    pub fn ampere() -> Self {
        Self { class: 0xC697 }
    }

    /// Create encoder for Ada
    pub fn ada() -> Self {
        Self { class: 0xC797 }
    }
}

impl CommandEncoder for GraphicsEncoder {
    fn encode(&self, cmd: &RecordedCommand, pb: &mut PushBuffer) -> Result<()> {
        match cmd {
            RecordedCommand::BindPipeline { pipeline_type, handle } => {
                // Set object (bind class)
                pb.push_single(0x0000, 0, self.class)?;
                // Bind shader program
                pb.push_single(0x0228, 0, *handle as u32)?;
            }
            RecordedCommand::BindVertexBuffer { binding, address, size, stride } => {
                let base = 0x0700 + (*binding * 0x10) as u16;
                pb.push_single(base, 0, (address.0 >> 32) as u32)?;
                pb.push_single(base + 4, 0, address.0 as u32)?;
                pb.push_single(base + 8, 0, *size)?;
                pb.push_single(base + 12, 0, *stride)?;
            }
            RecordedCommand::BindIndexBuffer { address, index_type } => {
                let type_val = match index_type {
                    crate::buffer::IndexType::U16 => 0,
                    crate::buffer::IndexType::U32 => 1,
                };
                pb.push_single(0x05f8, 0, (address.0 >> 32) as u32)?;
                pb.push_single(0x05fc, 0, address.0 as u32)?;
                pb.push_single(0x0604, 0, type_val)?;
            }
            RecordedCommand::Draw(params) => {
                // Configure draw
                pb.push_single(0x0586, 0, params.vertex_count)?;
                pb.push_single(0x058e, 0, params.instance_count)?;
                pb.push_single(0x0590, 0, params.first_vertex)?;
                pb.push_single(0x0592, 0, params.first_instance)?;
                // Trigger draw
                pb.push_single(0x0594, 0, 1)?;
            }
            RecordedCommand::DrawIndexed(params) => {
                pb.push_single(0x05f0, 0, params.index_count)?;
                pb.push_single(0x058e, 0, params.instance_count)?;
                pb.push_single(0x05f4, 0, params.first_index)?;
                pb.push_single(0x0598, 0, params.vertex_offset as u32)?;
                pb.push_single(0x0592, 0, params.first_instance)?;
                // Trigger indexed draw
                pb.push_single(0x0596, 0, 1)?;
            }
            RecordedCommand::BeginRenderPass { framebuffer } => {
                // Set render targets
                pb.push_single(0x0200, 0, (framebuffer.0 >> 32) as u32)?;
                pb.push_single(0x0204, 0, framebuffer.0 as u32)?;
            }
            RecordedCommand::EndRenderPass => {
                // Flush render targets
                pb.push_single(0x1360, 0, 0)?;
            }
            RecordedCommand::Barrier => {
                pb.push_single(0x0110, 0, 0)?; // WAIT_FOR_IDLE
            }
            _ => {
                // Not a graphics command
                return Err(Error::NotSupported);
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "graphics"
    }
}

// =============================================================================
// COMPUTE ENCODER
// =============================================================================

/// Compute command encoder
#[derive(Debug)]
pub struct ComputeEncoder {
    /// Engine class
    class: u32,
}

impl ComputeEncoder {
    /// Create compute encoder
    pub fn new() -> Self {
        Self { class: 0xC6C0 }
    }
}

impl Default for ComputeEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandEncoder for ComputeEncoder {
    fn encode(&self, cmd: &RecordedCommand, pb: &mut PushBuffer) -> Result<()> {
        match cmd {
            RecordedCommand::BindPipeline { handle, .. } => {
                pb.push_single(0x0000, 0, self.class)?;
                pb.push_single(0x0100, 0, *handle as u32)?;
            }
            RecordedCommand::BindDescriptorSet { set, address } => {
                let base = 0x0200 + (*set * 8) as u16;
                pb.push_single(base, 0, (address.0 >> 32) as u32)?;
                pb.push_single(base + 4, 0, address.0 as u32)?;
            }
            RecordedCommand::Dispatch(params) => {
                pb.push_single(0x0300, 0, params.groups_x)?;
                pb.push_single(0x0304, 0, params.groups_y)?;
                pb.push_single(0x0308, 0, params.groups_z)?;
                // Launch
                pb.push_single(0x030C, 0, 1)?;
            }
            RecordedCommand::Barrier => {
                pb.push_single(0x0110, 0, 0)?;
            }
            _ => {
                return Err(Error::NotSupported);
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "compute"
    }
}

// =============================================================================
// COPY ENCODER
// =============================================================================

/// Copy engine encoder
#[derive(Debug)]
pub struct CopyEncoder {
    /// Engine class
    class: u32,
}

impl CopyEncoder {
    /// Create copy encoder
    pub fn new() -> Self {
        Self { class: 0xC6B5 }
    }
}

impl Default for CopyEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandEncoder for CopyEncoder {
    fn encode(&self, cmd: &RecordedCommand, pb: &mut PushBuffer) -> Result<()> {
        match cmd {
            RecordedCommand::CopyBuffer { src, dst, size } => {
                // Source address
                pb.push_single(0x0400, 0, (src.0 >> 32) as u32)?;
                pb.push_single(0x0404, 0, src.0 as u32)?;
                // Destination address
                pb.push_single(0x0408, 0, (dst.0 >> 32) as u32)?;
                pb.push_single(0x040C, 0, dst.0 as u32)?;
                // Size
                pb.push_single(0x0410, 0, (*size >> 32) as u32)?;
                pb.push_single(0x0414, 0, *size as u32)?;
                // Launch copy
                pb.push_single(0x0418, 0, 1)?;
            }
            _ => {
                return Err(Error::NotSupported);
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "copy"
    }
}
