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

pub mod device;
pub mod queue;
pub mod swapchain;
pub mod command;
pub mod buffer;
pub mod texture;
pub mod sampler;
pub mod shader_module;
pub mod pipeline;
pub mod descriptor;
pub mod surface;
pub mod instance;
pub mod resource;
pub mod submission;

// Re-exports
pub use device::*;
pub use queue::*;
pub use swapchain::*;
pub use command::*;

/// Prelude for common imports
pub mod prelude {
    pub use crate::device::{
        Device, DeviceDesc, DeviceCapabilities, DeviceFeatures, DeviceLimits,
        Adapter, AdapterInfo, AdapterType, BackendType,
    };
    pub use crate::queue::{
        Queue, QueueFamily, QueueType, QueuePriority, QueueCapabilities,
    };
    pub use crate::swapchain::{
        Swapchain, SwapchainDesc, SwapchainImage, PresentMode, CompositeAlpha,
    };
    pub use crate::command::{
        CommandBuffer, CommandBufferDesc, CommandBufferLevel, CommandPool,
        RenderPassEncoder, ComputePassEncoder, TransferEncoder,
    };
    pub use crate::buffer::{
        Buffer, BufferDesc, BufferUsage, BufferMemoryType,
    };
    pub use crate::texture::{
        Texture, TextureDesc, TextureUsage, TextureView, TextureViewDesc,
    };
    pub use crate::sampler::{
        Sampler, SamplerDesc, FilterMode, AddressMode, CompareOp,
    };
    pub use crate::shader_module::{
        ShaderModule, ShaderModuleDesc, ShaderStage,
    };
    pub use crate::pipeline::{
        RenderPipeline, RenderPipelineDesc, ComputePipeline, ComputePipelineDesc,
        RayTracingPipeline, RayTracingPipelineDesc,
    };
    pub use crate::descriptor::{
        DescriptorSet, DescriptorSetLayout, DescriptorPool, DescriptorBinding,
    };
    pub use crate::surface::{
        Surface, SurfaceCapabilities, SurfaceFormat,
    };
    pub use crate::instance::{
        Instance, InstanceDesc, InstanceFeatures,
    };
    pub use crate::resource::{
        ResourceHandle, ResourceType, ResourceState,
    };
    pub use crate::submission::{
        SubmitInfo, PresentInfo, TimelineSemaphore, Fence, Semaphore,
    };
}
