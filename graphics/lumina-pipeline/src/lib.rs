//! LUMINA Pipeline - Revolutionary Pipeline State Management
//!
//! This crate provides a sophisticated pipeline management system featuring:
//!
//! - **Pipeline State Objects (PSO)**: Complete graphics/compute pipeline configuration
//! - **Shader Compilation**: SPIRV compilation with reflection and hot-reload
//! - **Descriptor System**: Bindless descriptors with automatic layout generation
//! - **Pipeline Cache**: Intelligent caching with disk persistence
//! - **Root Signatures**: Flexible resource binding with push constants
//! - **Specialization Constants**: Runtime shader customization
//!
//! # Architecture
//!
//! The pipeline system is designed for:
//! - Zero-overhead abstractions over graphics APIs
//! - Compile-time pipeline validation
//! - Runtime hot-reload for development
//! - Efficient descriptor management
//!
//! # Example
//!
//! ```rust,ignore
//! use lumina_pipeline::prelude::*;
//!
//! // Create graphics pipeline
//! let pipeline = GraphicsPipelineBuilder::new()
//!     .vertex_shader("shaders/mesh.vert.spv")
//!     .fragment_shader("shaders/pbr.frag.spv")
//!     .vertex_input::<Vertex>()
//!     .depth_test(true)
//!     .blend_mode(BlendMode::Opaque)
//!     .build(&device)?;
//!
//! // Create compute pipeline
//! let compute = ComputePipelineBuilder::new()
//!     .shader("shaders/culling.comp.spv")
//!     .build(&device)?;
//! ```

#![no_std]
#![warn(missing_docs)]
#![allow(dead_code)]

extern crate alloc;

pub mod pipeline;
pub mod shader;
pub mod descriptor;
pub mod layout;
pub mod cache;
pub mod state;
pub mod blend;
pub mod depth;
pub mod raster;
pub mod vertex;
pub mod specialization;
pub mod reflection;
pub mod bindless;

/// Prelude for common imports.
pub mod prelude {
    pub use crate::pipeline::{
        GraphicsPipeline, ComputePipeline, RayTracingPipeline,
        GraphicsPipelineBuilder, ComputePipelineBuilder,
        PipelineHandle, PipelineType,
    };
    pub use crate::shader::{
        ShaderModule, ShaderStage, ShaderSource,
        ShaderCompiler, ShaderError,
    };
    pub use crate::descriptor::{
        DescriptorSet, DescriptorSetLayout, DescriptorBinding,
        DescriptorType, DescriptorPoolConfig,
    };
    pub use crate::layout::{
        PipelineLayout, PipelineLayoutBuilder,
        PushConstantRange, BindingFrequency,
    };
    pub use crate::cache::{
        PipelineCache, CacheConfig, CacheStats,
    };
    pub use crate::state::{
        PipelineState, DynamicState, DynamicStateFlags,
    };
    pub use crate::blend::{
        BlendState, BlendMode, BlendFactor, BlendOp,
        ColorWriteMask, AttachmentBlend,
    };
    pub use crate::depth::{
        DepthState, DepthTest, CompareOp,
        StencilState, StencilOp, StencilOpState,
    };
    pub use crate::raster::{
        RasterState, CullMode, FrontFace, PolygonMode,
        DepthBias,
    };
    pub use crate::vertex::{
        VertexLayout, VertexAttribute, VertexBinding,
        VertexFormat, VertexInputRate,
    };
    pub use crate::specialization::{
        SpecializationConstants, SpecializationEntry,
    };
    pub use crate::reflection::{
        ShaderReflection, ReflectedBinding, ReflectedPushConstant,
    };
    pub use crate::bindless::{
        BindlessDescriptorSet, BindlessResourceType,
    };
}
