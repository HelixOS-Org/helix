//! # LUMINA Core
//!
//! Native graphics API for Helix OS - Core GPU abstractions.
//!
//! LUMINA is a low-level graphics API designed specifically for Helix OS.
//! It provides direct access to GPU hardware through the MAGMA driver,
//! which communicates with NVIDIA's GSP firmware.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Application                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │                      lumina-fx                               │
//! │         (High-level effects: sky, water, shadows, etc.)      │
//! ├─────────────────────────────────────────────────────────────┤
//! │                   ► lumina-core ◄                            │
//! │         (Core GPU API: buffers, pipelines, etc.)             │
//! ├─────────────────────────────────────────────────────────────┤
//! │                        MAGMA                                 │
//! │              (Native GPU Driver for Helix)                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     GSP Firmware                             │
//! │              (NVIDIA GPU System Processor)                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                     GPU Hardware                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Differences from Vulkan
//!
//! - **Native API**: Not a Vulkan wrapper - communicates directly with MAGMA driver
//! - **Helix-Optimized**: Designed for Helix's memory model and syscall interface
//! - **No External Dependencies**: No Mesa, libdrm, or other Linux graphics stack components
//!
//! ## Modules
//!
//! - `buffer`: GPU buffer management
//! - `pipeline`: Graphics and compute pipeline creation
//! - `descriptor`: Resource binding and descriptor sets
//! - `device`: GPU device enumeration and management
//! - `queue`: Command submission queues
//! - `memory`: GPU memory allocation
//! - `image`/`texture`: Image and texture resources
//! - `render_pass`: Render pass configuration
//! - `swapchain`: Presentation swapchain
//! - `sync`: Synchronization primitives

#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]
#![allow(unused)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// ============================================================================
// Core Types & Error Handling
// ============================================================================

pub mod color;
pub mod error;
pub mod handle;
pub mod types;

// ============================================================================
// Bindless Resources
// ============================================================================

pub mod bindless;

// ============================================================================
// Instance & Device
// ============================================================================

pub mod device;
pub mod device_limits;
pub mod device_types;
pub mod instance;

// ============================================================================
// Queues & Command Submission
// ============================================================================

pub mod command;
pub mod command_buffer;
pub mod queue;
pub mod queue_family;
pub mod queue_submit;

// ============================================================================
// Memory Management
// ============================================================================

pub mod memory;
pub mod memory_allocation;
pub mod memory_allocator;
pub mod memory_types;

// ============================================================================
// Buffers
// ============================================================================

pub mod buffer;
pub mod buffer_types;

// ============================================================================
// Images & Textures
// ============================================================================

pub mod format_properties;
pub mod format_utils;
pub mod image;
pub mod image_types;
pub mod sampler;
pub mod sampler_types;
pub mod texture;
pub mod texture_types;

// ============================================================================
// Render Passes & Framebuffers
// ============================================================================

pub mod framebuffer;
pub mod render_pass;
pub mod render_pass_builder;
pub mod render_pass_types;

// ============================================================================
// Pipelines
// ============================================================================

pub mod compute;
pub mod compute_pass;
pub mod compute_pipeline;
pub mod graphics_pass;
pub mod graphics_pipeline;
pub mod pipeline;
pub mod pipeline_builder;
pub mod pipeline_cache;
pub mod pipeline_layout;
pub mod pipeline_library;
pub mod pipeline_state;
pub mod pipeline_statistics;

// ============================================================================
// Descriptors & Resource Binding
// ============================================================================

pub mod descriptor;
pub mod descriptor_builder;
pub mod descriptor_pool;
pub mod descriptor_types;
pub mod push_constants;
pub mod resource;
pub mod resource_binding;
pub mod resource_types;

// ============================================================================
// Pipeline State
// ============================================================================

pub mod blend;
pub mod blend_state;
pub mod depth_stencil;
pub mod dynamic_state;
pub mod multisample;
pub mod multisample_state;
pub mod rasterization;
pub mod rasterization_state;
pub mod vertex;
pub mod vertex_input;
pub mod viewport;
pub mod viewport_scissor;

// ============================================================================
// Drawing & Dispatch
// ============================================================================

pub mod clear;
pub mod draw;
pub mod draw_commands;
pub mod indirect;

// ============================================================================
// Synchronization
// ============================================================================

pub mod barrier;
pub mod barriers;
pub mod event;
pub mod fence;
pub mod sync;
pub mod sync_primitives;
pub mod synchronization;

// ============================================================================
// Swapchain & Presentation
// ============================================================================

pub mod surface;
pub mod surface_types;
pub mod swapchain;
pub mod swapchain_types;

// ============================================================================
// Transfer Operations
// ============================================================================

pub mod copy;
pub mod transfer;

// ============================================================================
// Queries & Timestamps
// ============================================================================

pub mod query;
pub mod query_pool;
pub mod timestamp;

// ============================================================================
// Debug & Profiling
// ============================================================================

pub mod debug;
pub mod profiling;

// ============================================================================
// Modern Pipeline Features
// ============================================================================

/// Mesh shader pipeline (modern geometry processing)
pub mod mesh_shader;

/// Work graphs (GPU-driven compute scheduling)
pub mod work_graph;

// ============================================================================
// Ray Tracing (Optional Feature)
// ============================================================================

#[cfg(feature = "ray-tracing")]
pub mod acceleration_structure;
#[cfg(feature = "ray-tracing")]
pub mod ray_tracing;
#[cfg(feature = "ray-tracing")]
pub mod ray_tracing_pipeline;
#[cfg(feature = "ray-tracing")]
pub mod ray_tracing_types;

// ============================================================================
// Sparse Resources (Optional Feature)
// ============================================================================

#[cfg(feature = "sparse")]
pub mod sparse;
#[cfg(feature = "sparse")]
pub mod sparse_binding;
#[cfg(feature = "sparse")]
pub mod sparse_memory;

// ============================================================================
// Re-exports for convenience
// ============================================================================

pub use error::{Error, Result};
pub use handle::Handle;
pub use types::*;

/// LUMINA API version
pub const LUMINA_VERSION: (u32, u32, u32) = (1, 0, 0);

/// LUMINA API version as string
pub const LUMINA_VERSION_STRING: &str = "1.0.0";
