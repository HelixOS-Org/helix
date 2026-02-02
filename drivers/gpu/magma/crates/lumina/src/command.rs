//! Command recording and encoding
//!
//! This module provides types for recording GPU commands.

use alloc::vec::Vec;

use crate::types::PipelineHandle;

/// A command encoder for recording GPU commands
pub struct CommandEncoder {
    commands: Vec<Command>,
}

impl CommandEncoder {
    /// Creates a new command encoder
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Returns the recorded commands
    pub fn finish(self) -> Vec<Command> {
        self.commands
    }

    /// Clears the encoder for reuse
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for CommandEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// A GPU command
#[derive(Clone, Debug)]
pub enum Command {
    /// Set viewport
    SetViewport {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    },

    /// Set scissor rectangle
    SetScissor {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },

    /// Bind graphics pipeline
    BindGraphicsPipeline {
        pipeline: PipelineHandle,
    },

    /// Bind compute pipeline
    BindComputePipeline {
        pipeline: PipelineHandle,
    },

    /// Set push constants
    SetPushConstants {
        offset: u32,
        data: Vec<u8>,
    },

    /// Draw vertices
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },

    /// Draw indexed vertices
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },

    /// Dispatch compute shader
    Dispatch {
        x: u32,
        y: u32,
        z: u32,
    },
}
