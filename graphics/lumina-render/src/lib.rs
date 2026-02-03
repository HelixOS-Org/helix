//! # LUMINA Render - Revolutionary Rendering System
//!
//! This crate provides a next-generation rendering architecture featuring:
//!
//! ## Core Systems
//!
//! - **Render Graph**: Automatic resource management and barrier optimization
//! - **Frame Graph**: Multi-frame resource scheduling and temporal effects
//! - **Pass System**: Modular rendering passes with automatic dependencies
//! - **Resource Pool**: Transient resource allocation with aliasing
//!
//! ## Advanced Features
//!
//! - **Hybrid Rendering**: Seamless Ray Tracing + Rasterization fusion
//! - **Virtual Geometry**: Nanite-style virtualized geometry system
//! - **Neural Upscaling**: AI-powered temporal upscaling and denoising
//! - **Volumetric Rendering**: Real-time clouds, fog, and atmospheric effects
//! - **Global Illumination**: Real-time GI with temporal accumulation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Frame Graph                               │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
//! │  │ Shadow  │→│ GBuffer │→│Lighting │→│  Post   │→ Output     │
//! │  │  Pass   │  │  Pass   │  │  Pass   │  │ Process │            │
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │
//! │       ↓            ↓            ↓            ↓                  │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │              Resource Pool (Aliased Memory)              │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use lumina_render::prelude::*;
//!
//! let mut graph = RenderGraph::new();
//!
//! // Define resources
//! let gbuffer = graph.create_texture(TextureDesc::gbuffer(1920, 1080));
//! let depth = graph.create_texture(TextureDesc::depth(1920, 1080));
//! let output = graph.create_texture(TextureDesc::color(1920, 1080));
//!
//! // Add passes
//! graph.add_pass("gbuffer", |builder| {
//!     builder
//!         .write_color(gbuffer)
//!         .write_depth(depth)
//!         .render(|ctx| {
//!             ctx.draw_scene();
//!         })
//! });
//!
//! graph.add_pass("lighting", |builder| {
//!     builder
//!         .read_texture(gbuffer)
//!         .read_texture(depth)
//!         .write_color(output)
//!         .render(|ctx| {
//!             ctx.apply_lighting();
//!         })
//! });
//!
//! // Compile and execute
//! let compiled = graph.compile()?;
//! compiled.execute(&device)?;
//! ```

#![no_std]
#![cfg_attr(feature = "std", feature(error_in_core))]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod graph;
pub mod pass;
pub mod resource;
pub mod target;
pub mod scheduler;
pub mod barrier;
pub mod frame;
pub mod view;
pub mod culling;
pub mod visibility;
pub mod hybrid;
pub mod neural;
pub mod volumetric;
pub mod gi;
pub mod temporal;
pub mod debug;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::graph::{RenderGraph, RenderGraphBuilder, CompiledGraph};
    pub use crate::pass::{RenderPass, PassBuilder, PassContext, PassType};
    pub use crate::resource::{
        ResourcePool, ResourceHandle, TextureHandle, BufferHandle,
        TextureDesc, BufferDesc, ResourceUsage, ResourceState,
    };
    pub use crate::target::{RenderTarget, RenderTargetDesc, Attachment, AttachmentOp};
    pub use crate::scheduler::{FrameScheduler, SubmitInfo, QueueType};
    pub use crate::barrier::{Barrier, BarrierBatch, PipelineStage, AccessFlags};
    pub use crate::frame::{FrameContext, FrameResources, FrameIndex};
    pub use crate::view::{View, ViewType, Frustum, ViewUniforms};
    pub use crate::culling::{CullingSystem, CullingResult, OcclusionQuery};
    pub use crate::visibility::{VisibilityBuffer, VisibilityPass};
    pub use crate::hybrid::{HybridRenderer, RayTracingPass, RasterPass};
    pub use crate::neural::{NeuralUpscaler, TemporalAccumulator, Denoiser};
    pub use crate::volumetric::{VolumetricRenderer, CloudSystem, FogVolume};
    pub use crate::gi::{GlobalIllumination, ProbeGrid, RadianceCache};
    pub use crate::temporal::{TemporalAA, MotionVectors, JitterSequence};
    pub use crate::debug::{DebugRenderer, DebugOverlay, GpuProfiler};
}

pub use prelude::*;
