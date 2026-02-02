//! Backend interface for GPU drivers
//!
//! This module defines the trait that GPU backends must implement.

use alloc::vec::Vec;

use crate::buffer::BufferUsage;
use crate::error::Result;
use crate::graph::CompiledGraph;
use crate::pipeline::{ComputePipelineDesc, GraphicsPipelineDesc};
use crate::texture::TextureDesc;
use crate::types::{BufferHandle, PipelineHandle, TextureHandle};

/// Trait for GPU backend implementations
///
/// This is the interface between Lumina and the underlying GPU driver.
/// The Magma driver implements this trait to provide Vulkan-based rendering.
pub trait Backend: Send + Sync {
    /// Initialize the backend
    fn init(&mut self) -> Result<()>;

    /// Shutdown the backend
    fn shutdown(&mut self);

    /// Create a GPU buffer
    fn create_buffer(&mut self, size: usize, usage: BufferUsage) -> Result<BufferHandle>;

    /// Destroy a GPU buffer
    fn destroy_buffer(&mut self, handle: BufferHandle);

    /// Upload data to a buffer
    fn upload_buffer(&mut self, handle: BufferHandle, offset: usize, data: &[u8]) -> Result<()>;

    /// Create a GPU texture
    fn create_texture(&mut self, desc: &TextureDesc) -> Result<TextureHandle>;

    /// Destroy a GPU texture
    fn destroy_texture(&mut self, handle: TextureHandle);

    /// Upload data to a texture
    fn upload_texture(&mut self, handle: TextureHandle, mip_level: u32, data: &[u8]) -> Result<()>;

    /// Create a graphics pipeline
    fn create_graphics_pipeline(&mut self, desc: &GraphicsPipelineDesc) -> Result<PipelineHandle>;

    /// Create a compute pipeline
    fn create_compute_pipeline(&mut self, desc: &ComputePipelineDesc) -> Result<PipelineHandle>;

    /// Destroy a pipeline
    fn destroy_pipeline(&mut self, handle: PipelineHandle);

    /// Submit a compiled render graph
    fn submit(&mut self, graph: &CompiledGraph) -> Result<SubmitHandle>;

    /// Wait for a submission to complete
    fn wait(&mut self, handle: SubmitHandle) -> Result<()>;

    /// Acquire the next swapchain image
    fn acquire_frame(&mut self) -> Result<FrameHandle>;

    /// Present a frame
    fn present(&mut self, handle: FrameHandle) -> Result<()>;

    /// Get device information
    fn device_info(&self) -> &DeviceInfo;

    /// Get memory statistics
    fn memory_stats(&self) -> MemoryStats;
}

/// Handle to a submitted command batch
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmitHandle {
    /// Submission ID
    pub id: u64,
}

/// Handle to a swapchain frame
#[derive(Clone, Copy, Debug)]
pub struct FrameHandle {
    /// Frame index
    pub index: u32,
    /// Swapchain image index
    pub image_index: u32,
}

/// GPU device information
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    /// Device name
    pub name: &'static str,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Driver version
    pub driver_version: u32,
    /// API version
    pub api_version: u32,
    /// Device type
    pub device_type: DeviceType,
    /// Maximum texture dimensions
    pub max_texture_size: u32,
    /// Maximum compute workgroup size
    pub max_workgroup_size: [u32; 3],
    /// Maximum push constant size
    pub max_push_constant_size: u32,
    /// Supports raytracing
    pub raytracing_supported: bool,
    /// Supports mesh shaders
    pub mesh_shaders_supported: bool,
}

/// GPU device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceType {
    /// Discrete GPU
    Discrete,
    /// Integrated GPU
    Integrated,
    /// Virtual GPU
    Virtual,
    /// CPU (software rendering)
    Cpu,
    /// Unknown
    Unknown,
}

/// GPU memory statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryStats {
    /// Total device memory (bytes)
    pub total_device_memory: u64,
    /// Used device memory (bytes)
    pub used_device_memory: u64,
    /// Total host-visible memory (bytes)
    pub total_host_memory: u64,
    /// Used host-visible memory (bytes)
    pub used_host_memory: u64,
    /// Number of allocations
    pub allocation_count: u64,
}

/// Null backend for testing
pub struct NullBackend {
    device_info: DeviceInfo,
    next_buffer_id: u32,
    next_texture_id: u32,
    next_pipeline_id: u32,
    next_submit_id: u64,
}

impl NullBackend {
    /// Creates a new null backend
    pub fn new() -> Self {
        Self {
            device_info: DeviceInfo {
                name: "Null Device",
                vendor_id: 0,
                device_id: 0,
                driver_version: 1,
                api_version: 1,
                device_type: DeviceType::Virtual,
                max_texture_size: 16384,
                max_workgroup_size: [1024, 1024, 64],
                max_push_constant_size: 256,
                raytracing_supported: false,
                mesh_shaders_supported: false,
            },
            next_buffer_id: 1,
            next_texture_id: 1,
            next_pipeline_id: 1,
            next_submit_id: 1,
        }
    }
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NullBackend {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) {}

    fn create_buffer(&mut self, _size: usize, _usage: BufferUsage) -> Result<BufferHandle> {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;
        Ok(BufferHandle::new(id, 0))
    }

    fn destroy_buffer(&mut self, _handle: BufferHandle) {}

    fn upload_buffer(&mut self, _handle: BufferHandle, _offset: usize, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn create_texture(&mut self, _desc: &TextureDesc) -> Result<TextureHandle> {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        Ok(TextureHandle::new(id, 0))
    }

    fn destroy_texture(&mut self, _handle: TextureHandle) {}

    fn upload_texture(
        &mut self,
        _handle: TextureHandle,
        _mip_level: u32,
        _data: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    fn create_graphics_pipeline(&mut self, _desc: &GraphicsPipelineDesc) -> Result<PipelineHandle> {
        let id = self.next_pipeline_id;
        self.next_pipeline_id += 1;
        Ok(PipelineHandle::new(id, 0))
    }

    fn create_compute_pipeline(&mut self, _desc: &ComputePipelineDesc) -> Result<PipelineHandle> {
        let id = self.next_pipeline_id;
        self.next_pipeline_id += 1;
        Ok(PipelineHandle::new(id, 0))
    }

    fn destroy_pipeline(&mut self, _handle: PipelineHandle) {}

    fn submit(&mut self, _graph: &CompiledGraph) -> Result<SubmitHandle> {
        let id = self.next_submit_id;
        self.next_submit_id += 1;
        Ok(SubmitHandle { id })
    }

    fn wait(&mut self, _handle: SubmitHandle) -> Result<()> {
        Ok(())
    }

    fn acquire_frame(&mut self) -> Result<FrameHandle> {
        Ok(FrameHandle {
            index: 0,
            image_index: 0,
        })
    }

    fn present(&mut self, _handle: FrameHandle) -> Result<()> {
        Ok(())
    }

    fn device_info(&self) -> &DeviceInfo {
        &self.device_info
    }

    fn memory_stats(&self) -> MemoryStats {
        MemoryStats::default()
    }
}
