//! # Lumina: Intent-Based Graphics for Helix OS
//!
//! Lumina is a revolutionary graphics API that eliminates the ceremony of
//! traditional APIs like Vulkan while maintaining 100% of the performance.
//!
//! ## Philosophy
//!
//! - **Single-Source**: Write GPU code as Rust closures with `#[lumina::shader]`
//! - **Intent-Based**: Pipeline state is inferred from what you do, not configured
//! - **Safe by Design**: Rust's borrow checker prevents GPU hazards at compile time
//!
//! ## Quick Start
//!
//! ```rust
//! use lumina::prelude::*;
//!
//! #[derive(GpuVertex)]
//! struct Vertex {
//!     position: Vec3,
//!     color: Vec3,
//! }
//!
//! #[lumina::shader(vertex)]
//! fn vertex_main(v: Vertex, uniforms: &Uniforms) -> VertexOutput<Vec3> {
//!     VertexOutput {
//!         position: uniforms.mvp * v.position.extend(1.0),
//!         varying: v.color,
//!     }
//! }
//!
//! #[lumina::shader(fragment)]
//! fn fragment_main(color: Vec3) -> FragmentOutput<Rgba8> {
//!     FragmentOutput {
//!         color: color.extend(1.0).into(),
//!     }
//! }
//!
//! fn main() -> lumina::Result<()> {
//!     let app = Lumina::init("My App")?.build()?;
//!     let mesh = GpuMesh::cube(1.0);
//!
//!     app.run(|frame, _| {
//!         frame
//!             .render()
//!             .clear(Color::BLACK)
//!             .draw(&mesh)
//!             .with(vertex_main, fragment_main)
//!             .submit();
//!         true
//!     })
//! }
//! ```

#![no_std]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Re-export derive macros when the feature is enabled
#[cfg(feature = "derive")]
pub use lumina_derive::*;

// Core modules
pub mod backend;
pub mod bind_group;
pub mod buffer;
pub mod color;
pub mod command;
pub mod compute;
pub mod context;
pub mod device;
pub mod draw;
pub mod error;
pub mod frame;
pub mod graph;
pub mod graphics_pipeline;
pub mod handle;
pub mod memory;
pub mod mesh;
pub mod pipeline;
pub mod query;
pub mod render_pass;
pub mod resource;
pub mod sampler;
pub mod shader;
pub mod state;
pub mod surface;
pub mod sync;
pub mod texture;
pub mod types;

// Re-export math types
pub use lumina_math as math;

/// The Lumina prelude - import this for common types
pub mod prelude {
    #[cfg(feature = "derive")]
    pub use lumina_derive::{GpuData, GpuUniforms, GpuVertex};
    pub use lumina_math::{Mat2, Mat3, Mat4, Vec2, Vec3, Vec4};

    pub use crate::buffer::{BufferUsage, GpuBuffer};
    pub use crate::color::Color;
    pub use crate::context::{Lumina, LuminaBuilder};
    pub use crate::error::{Error, Result};
    pub use crate::frame::Frame;
    pub use crate::mesh::GpuMesh;
    pub use crate::pipeline::{BlendMode, CullMode, DepthTest};
    pub use crate::resource::GpuResource;
    pub use crate::sampler::{FilterMode, Sampler, WrapMode};
    pub use crate::state::RenderState;
    pub use crate::texture::{GpuTexture, TextureFormat};
    pub use crate::types::*;
}

/// Shader output types
pub mod output {
    use lumina_math::Vec4;

    use crate::types::*;

    /// Output from a vertex shader
    #[derive(Clone, Copy, Debug)]
    pub struct VertexOutput<V> {
        /// Clip-space position (gl_Position equivalent)
        pub position: Vec4,
        /// Varying data passed to fragment shader
        pub varying: V,
    }

    /// Output from a fragment shader
    #[derive(Clone, Copy, Debug)]
    pub struct FragmentOutput<F: FragmentFormat> {
        /// Fragment color output
        pub color: F,
    }

    /// Marker trait for valid fragment output formats
    pub trait FragmentFormat: Copy {
        /// The Vulkan format equivalent
        const VK_FORMAT: u32;
    }

    /// 8-bit RGBA normalized format
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct Rgba8 {
        /// Red channel (0.0 - 1.0)
        pub r: u8,
        /// Green channel (0.0 - 1.0)
        pub g: u8,
        /// Blue channel (0.0 - 1.0)
        pub b: u8,
        /// Alpha channel (0.0 - 1.0)
        pub a: u8,
    }

    impl FragmentFormat for Rgba8 {
        const VK_FORMAT: u32 = 37; // VK_FORMAT_R8G8B8A8_UNORM
    }

    /// 16-bit RGBA float format
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct Rgba16F {
        /// Red channel
        pub r: f32, // Actually f16, simplified for demo
        /// Green channel
        pub g: f32,
        /// Blue channel
        pub b: f32,
        /// Alpha channel
        pub a: f32,
    }

    impl FragmentFormat for Rgba16F {
        const VK_FORMAT: u32 = 97; // VK_FORMAT_R16G16B16A16_SFLOAT
    }

    /// 32-bit RGBA float format
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct Rgba32F {
        /// Red channel
        pub r: f32,
        /// Green channel
        pub g: f32,
        /// Blue channel
        pub b: f32,
        /// Alpha channel
        pub a: f32,
    }

    impl FragmentFormat for Rgba32F {
        const VK_FORMAT: u32 = 109; // VK_FORMAT_R32G32B32A32_SFLOAT
    }

    /// 32-bit depth format
    #[derive(Clone, Copy, Debug, Default)]
    #[repr(C)]
    pub struct Depth32F(pub f32);

    impl FragmentFormat for Depth32F {
        const VK_FORMAT: u32 = 126; // VK_FORMAT_D32_SFLOAT
    }
}

pub use output::{Depth32F, FragmentOutput, Rgba16F, Rgba32F, Rgba8, VertexOutput};
