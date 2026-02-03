//! LUMINA Backend - GPU Backend Abstraction Layer
//!
//! This crate provides the abstraction layer between LUMINA's high-level rendering
//! systems and low-level GPU drivers (MAGMA). It implements a modern graphics API
//! abstraction supporting Vulkan, Metal, DX12, and WebGPU backends.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  LUMINA Rendering                    │
//! │         (Render Graph, Materials, Meshes)           │
//! ├─────────────────────────────────────────────────────┤
//! │                 lumina-backend                       │
//! │     (Device, Queue, Swapchain, Command Buffers)     │
//! ├─────────────────────────────────────────────────────┤
//! │              Backend Implementation                  │
//! │         (Vulkan / Metal / DX12 / WebGPU)           │
//! ├─────────────────────────────────────────────────────┤
//! │                     MAGMA                            │
//! │              (GPU Driver Layer)                      │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Multi-Backend**: Vulkan, Metal, DX12, WebGPU support
//! - **Modern API**: Bindless, ray tracing, mesh shaders
//! - **Async Compute**: Multiple queue families with priorities
//! - **Memory Management**: Automatic allocation and streaming
//! - **Validation**: Debug layers and GPU validation
//!
//! # Modules
//!
//! - [`device`]: GPU device abstraction and capabilities
//! - [`queue`]: Command queue management and submission
//! - [`swapchain`]: Surface and presentation management
//! - [`command`]: Command buffer recording and execution
//! - [`buffer`]: GPU buffer management
//! - [`texture`]: Texture and image management
//! - [`sampler`]: Sampler state management
//! - [`shader`]: Shader module management
//! - [`pipeline`]: Pipeline state objects
//! - [`descriptor`]: Descriptor set management
//! - [`surface`]: Window surface abstraction
//! - [`instance`]: Backend instance creation

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod buffer;
pub mod command;
pub mod descriptor;
pub mod device;
pub mod instance;
pub mod pipeline;
pub mod queue;
pub mod resource;
pub mod sampler;
pub mod shader_module;
pub mod submission;
pub mod surface;
pub mod swapchain;
pub mod texture;

// Re-exports
pub use command::*;
pub use device::*;
pub use queue::*;
pub use swapchain::*;

/// Prelude for common imports
pub mod prelude {
    pub use crate::buffer::{Buffer, BufferDesc, BufferMemoryType, BufferUsage, BufferHandle};
    pub use crate::command::{
        CommandBuffer, CommandBufferLevel, CommandPool, ComputePassEncoder,
        RenderPassEncoder, TransferEncoder, CommandBufferFlags,
    };
    pub use crate::descriptor::{
        DescriptorSetLayoutBinding, DescriptorPool, DescriptorSet, DescriptorSetLayout,
        DescriptorType, DescriptorWrite, DescriptorManager, PipelineLayout,
    };
    pub use crate::device::{
        Adapter, AdapterInfo, AdapterType, BackendType, Device, DeviceCapabilities, DeviceDesc,
        DeviceFeatures, DeviceLimits, TextureFormat,
    };
    pub use crate::instance::{Instance, InstanceDesc, InstanceFlags, BackendFactory};
    pub use crate::pipeline::{
        ComputePipeline, ComputePipelineDesc, RayTracingPipeline, RayTracingPipelineDesc,
        RenderPipeline, RenderPipelineDesc, PipelineManager,
    };
    pub use crate::queue::{Queue, QueueFamily, QueuePriority, QueueManager};
    pub use crate::resource::{ResourceHandle, ResourceType, ResourceRegistry, ResourceStatistics};
    pub use crate::sampler::{AddressMode, CompareOp, FilterMode, Sampler, SamplerDesc, SamplerHandle};
    pub use crate::shader_module::{ShaderModule, ShaderModuleDesc, ShaderStage, ShaderManager};
    pub use crate::submission::{Fence, PresentInfo, Semaphore, SubmitInfo, TimelineSemaphore, SubmissionManager};
    pub use crate::surface::{Surface, SurfaceCapabilities, SurfaceFormat, SurfaceManager};
    pub use crate::swapchain::{
        CompositeAlpha, PresentMode, Swapchain, SwapchainDesc, SwapchainImage, SwapchainManager,
    };
    pub use crate::texture::{Texture, TextureDesc, TextureUsage, TextureView, TextureViewDesc};
}
