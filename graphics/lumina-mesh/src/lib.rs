//! Lumina Mesh System
//!
//! Revolutionary GPU-driven geometry system featuring:
//! - Nanite-style virtual geometry with automatic LOD
//! - Meshlet-based rendering for mesh shaders
//! - Hierarchical cluster culling
//! - Geometry streaming and compression
//! - Ray tracing acceleration structure support
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Mesh Pipeline                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
//! │  │  Source Mesh   │──│    Meshlet     │──│    Virtual     │            │
//! │  │   Importer     │  │   Generator    │  │   Geometry     │            │
//! │  └────────────────┘  └────────────────┘  └────────────────┘            │
//! │          │                  │                    │                      │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
//! │  │     LOD        │──│    Cluster     │──│   Streaming    │            │
//! │  │   Generator    │  │   Hierarchy    │  │    Manager     │            │
//! │  └────────────────┘  └────────────────┘  └────────────────┘            │
//! │          │                  │                    │                      │
//! │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐            │
//! │  │   Mesh Shader  │──│    GPU-Driven  │──│   Ray Tracing  │            │
//! │  │   Pipeline     │  │    Culling     │  │      BLAS      │            │
//! │  └────────────────┘  └────────────────┘  └────────────────┘            │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Concepts
//!
//! ## Meshlets
//! Small fixed-size clusters of triangles (typically 64-128 vertices, 64-128 triangles)
//! that enable efficient GPU-driven culling and mesh shader rendering.
//!
//! ## Virtual Geometry
//! Nanite-inspired system that automatically streams and LODs geometry based on
//! screen-space error, enabling massive geometric detail without manual LOD authoring.
//!
//! ## Cluster Hierarchy
//! Hierarchical spatial structure for efficient culling - clusters can be rejected
//! early based on bounding volumes before individual meshlet culling.

#![no_std]
#![warn(missing_docs)]
#![allow(dead_code)]

extern crate alloc;

pub mod mesh;
pub mod meshlet;
pub mod virtual_geometry;
pub mod lod;
pub mod cluster;
pub mod streaming;
pub mod blas;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::mesh::{
        Mesh, MeshBuilder, MeshData, MeshDesc, MeshFlags, MeshHandle, MeshManager, MeshPrimitive,
        Submesh, Vertex, VertexAttribute,
    };
    pub use crate::meshlet::{
        Meshlet, MeshletBounds, MeshletData, MeshletGenerator, MeshletMesh, MeshletStats,
    };
    pub use crate::virtual_geometry::{
        VirtualGeometry, VirtualGeometryNode, VirtualMesh, VirtualMeshDesc, VirtualPage,
        PageRequest, StreamingPriority,
    };
    pub use crate::lod::{
        LodBias, LodChain, LodLevel, LodManager, LodMesh, LodSelection, LodSettings,
        ScreenSpaceError,
    };
    pub use crate::cluster::{
        Cluster, ClusterBounds, ClusterCullData, ClusterHierarchy, ClusterNode, ClusterTree,
    };
    pub use crate::streaming::{
        GeometryCache, GeometryPage, GeometryStreamer, PageId, PageState, StreamingConfig,
        StreamingStats,
    };
    pub use crate::blas::{
        AccelerationStructure, BlasBuilder, BlasDesc, BlasFlags, BlasGeometry, BlasHandle,
        BlasInstance, BlasManager,
    };
}
