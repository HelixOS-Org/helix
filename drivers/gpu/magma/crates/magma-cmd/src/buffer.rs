//! # Command Buffer
//!
//! High-level command buffer recording interface.

use alloc::vec::Vec;

use magma_core::{Error, Result, GpuAddr, ByteSize};
use magma_core::command::{DrawParams, DrawIndexedParams, DispatchParams};

use crate::pushbuf::PushBuffer;

// =============================================================================
// COMMAND BUFFER STATE
// =============================================================================

/// Command buffer state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// Initial state, ready for recording
    Initial,
    /// Currently recording
    Recording,
    /// Recording complete, ready for submission
    Executable,
    /// Submitted to GPU
    Pending,
    /// Invalid (error occurred)
    Invalid,
}

// =============================================================================
// RECORDED COMMAND
// =============================================================================

/// A recorded command
#[derive(Debug, Clone)]
pub enum RecordedCommand {
    /// Bind pipeline
    BindPipeline {
        /// Pipeline type (graphics/compute)
        pipeline_type: PipelineType,
        /// Pipeline handle
        handle: u64,
    },
    /// Bind vertex buffer
    BindVertexBuffer {
        /// Binding index
        binding: u32,
        /// Buffer address
        address: GpuAddr,
        /// Buffer size
        size: u32,
        /// Stride
        stride: u32,
    },
    /// Bind index buffer
    BindIndexBuffer {
        /// Buffer address
        address: GpuAddr,
        /// Index type (16 or 32 bit)
        index_type: IndexType,
    },
    /// Bind descriptor set
    BindDescriptorSet {
        /// Set index
        set: u32,
        /// Descriptor buffer address
        address: GpuAddr,
    },
    /// Push constants
    PushConstants {
        /// Offset
        offset: u32,
        /// Data
        data: Vec<u8>,
    },
    /// Draw call
    Draw(DrawParams),
    /// Indexed draw call
    DrawIndexed(DrawIndexedParams),
    /// Compute dispatch
    Dispatch(DispatchParams),
    /// Pipeline barrier
    Barrier,
    /// Copy buffer
    CopyBuffer {
        /// Source address
        src: GpuAddr,
        /// Destination address
        dst: GpuAddr,
        /// Size
        size: u64,
    },
    /// Begin render pass
    BeginRenderPass {
        /// Framebuffer address
        framebuffer: GpuAddr,
    },
    /// End render pass
    EndRenderPass,
}

/// Pipeline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineType {
    /// Graphics pipeline
    Graphics,
    /// Compute pipeline
    Compute,
}

/// Index buffer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    /// 16-bit indices
    U16,
    /// 32-bit indices
    U32,
}

// =============================================================================
// COMMAND BUFFER
// =============================================================================

/// A command buffer for recording GPU commands
#[derive(Debug)]
pub struct CommandBuffer {
    /// Current state
    state: CommandBufferState,
    /// Recorded commands
    commands: Vec<RecordedCommand>,
    /// Compiled push buffer
    push_buffer: Option<PushBuffer>,
    /// Estimated size
    estimated_size: usize,
}

impl CommandBuffer {
    /// Create a new command buffer
    pub fn new() -> Self {
        Self {
            state: CommandBufferState::Initial,
            commands: Vec::new(),
            push_buffer: None,
            estimated_size: 0,
        }
    }

    /// Get current state
    pub fn state(&self) -> CommandBufferState {
        self.state
    }

    /// Begin recording
    pub fn begin(&mut self) -> Result<()> {
        match self.state {
            CommandBufferState::Initial | CommandBufferState::Executable => {
                self.commands.clear();
                self.push_buffer = None;
                self.estimated_size = 0;
                self.state = CommandBufferState::Recording;
                Ok(())
            }
            _ => Err(Error::InvalidState),
        }
    }

    /// End recording
    pub fn end(&mut self) -> Result<()> {
        if self.state != CommandBufferState::Recording {
            return Err(Error::InvalidState);
        }

        self.state = CommandBufferState::Executable;
        Ok(())
    }

    /// Reset command buffer
    pub fn reset(&mut self) {
        self.commands.clear();
        self.push_buffer = None;
        self.estimated_size = 0;
        self.state = CommandBufferState::Initial;
    }

    /// Check if recording
    fn check_recording(&self) -> Result<()> {
        if self.state != CommandBufferState::Recording {
            return Err(Error::InvalidState);
        }
        Ok(())
    }

    // =========================================================================
    // Recording commands
    // =========================================================================

    /// Bind graphics pipeline
    pub fn bind_graphics_pipeline(&mut self, handle: u64) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::BindPipeline {
            pipeline_type: PipelineType::Graphics,
            handle,
        });
        self.estimated_size += 8;
        Ok(())
    }

    /// Bind compute pipeline
    pub fn bind_compute_pipeline(&mut self, handle: u64) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::BindPipeline {
            pipeline_type: PipelineType::Compute,
            handle,
        });
        self.estimated_size += 8;
        Ok(())
    }

    /// Bind vertex buffer
    pub fn bind_vertex_buffer(
        &mut self,
        binding: u32,
        address: GpuAddr,
        size: u32,
        stride: u32,
    ) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::BindVertexBuffer {
            binding,
            address,
            size,
            stride,
        });
        self.estimated_size += 16;
        Ok(())
    }

    /// Bind index buffer
    pub fn bind_index_buffer(&mut self, address: GpuAddr, index_type: IndexType) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::BindIndexBuffer {
            address,
            index_type,
        });
        self.estimated_size += 12;
        Ok(())
    }

    /// Draw
    pub fn draw(&mut self, params: DrawParams) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::Draw(params));
        self.estimated_size += 20;
        Ok(())
    }

    /// Draw indexed
    pub fn draw_indexed(&mut self, params: DrawIndexedParams) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::DrawIndexed(params));
        self.estimated_size += 24;
        Ok(())
    }

    /// Dispatch compute
    pub fn dispatch(&mut self, params: DispatchParams) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::Dispatch(params));
        self.estimated_size += 16;
        Ok(())
    }

    /// Pipeline barrier
    pub fn barrier(&mut self) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::Barrier);
        self.estimated_size += 4;
        Ok(())
    }

    /// Copy buffer
    pub fn copy_buffer(&mut self, src: GpuAddr, dst: GpuAddr, size: u64) -> Result<()> {
        self.check_recording()?;
        self.commands.push(RecordedCommand::CopyBuffer { src, dst, size });
        self.estimated_size += 24;
        Ok(())
    }

    // =========================================================================
    // Compilation
    // =========================================================================

    /// Get recorded commands
    pub fn commands(&self) -> &[RecordedCommand] {
        &self.commands
    }

    /// Get estimated push buffer size
    pub fn estimated_size(&self) -> usize {
        self.estimated_size
    }

    /// Compile to push buffer
    pub fn compile(&mut self, gpu_addr: GpuAddr) -> Result<&PushBuffer> {
        if self.state != CommandBufferState::Executable {
            return Err(Error::InvalidState);
        }

        // Create push buffer with estimated size
        let size = ByteSize::from_bytes((self.estimated_size * 2).max(4096) as u64);
        let mut pb = PushBuffer::new(gpu_addr, size);

        // Compile commands
        for cmd in &self.commands {
            self.compile_command(&mut pb, cmd)?;
        }

        self.push_buffer = Some(pb);
        Ok(self.push_buffer.as_ref().unwrap())
    }

    /// Compile a single command
    fn compile_command(&self, pb: &mut PushBuffer, cmd: &RecordedCommand) -> Result<()> {
        use crate::pushbuf::methods;

        match cmd {
            RecordedCommand::Draw(params) => {
                // Simplified draw encoding
                pb.push_single(0x0586, 0, params.vertex_count)?; // VERTEX_COUNT
                pb.push_single(0x058E, 0, params.instance_count)?; // INSTANCE_COUNT
                pb.push_single(0x0590, 0, params.first_vertex)?; // VERTEX_START
                pb.push_single(0x0594, 0, 1)?; // DRAW_TRIGGER
            }
            RecordedCommand::Dispatch(params) => {
                pb.push_single(0x0200, 0, params.groups_x)?;
                pb.push_single(0x0204, 0, params.groups_y)?;
                pb.push_single(0x0208, 0, params.groups_z)?;
                pb.push_single(0x020C, 0, 1)?; // DISPATCH_TRIGGER
            }
            RecordedCommand::Barrier => {
                pb.push_single(methods::WAIT_FOR_IDLE, 0, 0)?;
            }
            // Other commands would be compiled similarly
            _ => {
                // NOP for unimplemented commands
                pb.push_single(methods::NOP, 0, 0)?;
            }
        }

        Ok(())
    }

    /// Get compiled push buffer
    pub fn push_buffer(&self) -> Option<&PushBuffer> {
        self.push_buffer.as_ref()
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}
