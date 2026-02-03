//! Lumina Material System
//!
//! Revolutionary PBR material system with shader graphs, procedural textures,
//! and GPU-driven material instancing.
//!
//! # Features
//!
//! - **PBR Materials**: Physically-based rendering with metallic-roughness workflow
//! - **Shader Graphs**: Node-based shader authoring system
//! - **Texture Management**: Efficient texture streaming and virtual texturing
//! - **Material Instancing**: GPU-driven material instance management
//! - **Layered Materials**: Complex material blending and layers
//! - **Procedural Generation**: Noise functions and procedural patterns
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Material System                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
//! │  │   Material   │  │  ShaderGraph │  │   Texture    │      │
//! │  │   Manager    │──│    System    │──│   Manager    │      │
//! │  └──────────────┘  └──────────────┘  └──────────────┘      │
//! │         │                 │                 │               │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
//! │  │     PBR      │  │    Node      │  │   Sampler    │      │
//! │  │  Parameters  │  │   Library    │  │    Cache     │      │
//! │  └──────────────┘  └──────────────┘  └──────────────┘      │
//! │         │                 │                 │               │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
//! │  │   Instance   │  │    Graph     │  │   Virtual    │      │
//! │  │     Pool     │  │   Compiler   │  │   Texture    │      │
//! │  └──────────────┘  └──────────────┘  └──────────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![no_std]
#![warn(missing_docs)]
#![allow(dead_code)]

extern crate alloc;

pub mod material;
pub mod pbr;
pub mod shader_graph;
pub mod texture;
pub mod sampler;
pub mod parameter;
pub mod instance;
pub mod layer;
pub mod procedural;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::material::{
        Material, MaterialHandle, MaterialDesc, MaterialManager,
        MaterialFlags, BlendMode, AlphaMode,
    };
    pub use crate::pbr::{
        PbrMaterial, PbrParameters, MetallicRoughness, SpecularGlossiness,
        ClearCoat, Sheen, Transmission, Volume, Iridescence,
    };
    pub use crate::shader_graph::{
        ShaderGraph, ShaderNode, NodeId, NodeInput, NodeOutput,
        NodeType, Connection, GraphCompiler, CompiledGraph,
    };
    pub use crate::texture::{
        TextureHandle, TextureDesc, TextureFormat, TextureUsage,
        TextureManager, TextureView, TextureViewDesc,
    };
    pub use crate::sampler::{
        Sampler, SamplerDesc, FilterMode, AddressMode,
        SamplerCache, SamplerHandle,
    };
    pub use crate::parameter::{
        Parameter, ParameterType, ParameterValue, ParameterBlock,
        ParameterBinding,
    };
    pub use crate::instance::{
        MaterialInstance, InstanceId, InstancePool, InstanceData,
    };
    pub use crate::layer::{
        MaterialLayer, LayerBlend, LayerMask, LayeredMaterial,
    };
    pub use crate::procedural::{
        NoiseType, Noise, Pattern, Gradient, ProceduralTexture,
    };
}
