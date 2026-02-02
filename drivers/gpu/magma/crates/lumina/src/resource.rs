//! GPU resource management
//!
//! This module provides traits and utilities for managing GPU resources.

use crate::types::{BufferHandle, TextureHandle};

/// Trait for GPU-allocated resources
pub trait GpuResource {
    /// Returns true if the resource has been uploaded to the GPU
    fn is_uploaded(&self) -> bool;

    /// Returns the GPU memory usage in bytes
    fn gpu_memory_usage(&self) -> usize;
}

/// Resource lifecycle state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceState {
    /// Resource is only in CPU memory
    CpuOnly,
    /// Resource is being uploaded
    Uploading,
    /// Resource is in GPU memory
    Resident,
    /// Resource is being evicted
    Evicting,
    /// Resource has been destroyed
    Destroyed,
}

/// Information about resource memory usage
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryStats {
    /// Total GPU memory allocated (bytes)
    pub gpu_allocated: usize,
    /// Total GPU memory used (bytes)
    pub gpu_used: usize,
    /// Number of buffers
    pub buffer_count: usize,
    /// Number of textures
    pub texture_count: usize,
    /// Number of pipelines
    pub pipeline_count: usize,
}

/// Resource upload priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UploadPriority {
    /// Low priority - uploaded when bandwidth available
    Low       = 0,
    /// Normal priority - default
    Normal    = 1,
    /// High priority - uploaded as soon as possible
    High      = 2,
    /// Immediate - blocks until uploaded
    Immediate = 3,
}

impl Default for UploadPriority {
    fn default() -> Self {
        Self::Normal
    }
}
