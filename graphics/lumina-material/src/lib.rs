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

pub mod instance;
pub mod layer;
pub mod material;
pub mod parameter;
pub mod pbr;
pub mod procedural;
pub mod sampler;
pub mod shader_graph;
pub mod texture;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::instance::{InstanceData, InstanceId, InstancePool, MaterialInstance};
    pub use crate::layer::{LayerBlend, LayerMask, LayeredMaterial, MaterialLayer};
    pub use crate::material::{
        AlphaMode, BlendMode, Material, MaterialDesc, MaterialFlags, MaterialHandle,
        MaterialManager,
    };
    pub use crate::parameter::{
        Parameter, ParameterBinding, ParameterBlock, ParameterType, ParameterValue,
    };
    pub use crate::pbr::{
        ClearCoat, Iridescence, MetallicRoughness, PbrMaterial, PbrParameters, Sheen,
        SpecularGlossiness, Transmission, Volume,
    };
    pub use crate::procedural::{Gradient, Noise, NoiseType, Pattern, ProceduralTexture};
    pub use crate::sampler::{
        AddressMode, FilterMode, Sampler, SamplerCache, SamplerDesc, SamplerHandle,
    };
    pub use crate::shader_graph::{
        CompiledGraph, Connection, GraphCompiler, NodeId, NodeInput, NodeOutput, NodeType,
        ShaderGraph, ShaderNode,
    };
    pub use crate::texture::{
        TextureDesc, TextureFormat, TextureHandle, TextureManager, TextureUsage, TextureView,
        TextureViewDesc,
    };
}
