//! Core Pipeline Objects and Builders
//!
//! This module provides the main pipeline types for graphics, compute,
//! and ray tracing workloads.

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::hash::{Hash, Hasher};

use crate::blend::BlendState;
use crate::depth::DepthState;
use crate::descriptor::DescriptorSetLayout;
use crate::layout::PipelineLayout;
use crate::raster::RasterState;
use crate::shader::{ShaderModule, ShaderStage};
use crate::specialization::SpecializationConstants;
use crate::state::DynamicStateFlags;
use crate::vertex::VertexLayout;

// ============================================================================
// Pipeline Types
// ============================================================================

/// Type of pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineType {
    /// Traditional rasterization pipeline.
    Graphics,
    /// General-purpose compute pipeline.
    Compute,
    /// Ray tracing pipeline.
    RayTracing,
    /// Mesh shading pipeline.
    MeshShading,
}

/// Unique identifier for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle {
    /// Internal index.
    index: u32,
    /// Generation for validation.
    generation: u32,
    /// Pipeline type.
    pipeline_type: PipelineType,
}

impl PipelineHandle {
    /// Create a new pipeline handle.
    pub fn new(index: u32, generation: u32, pipeline_type: PipelineType) -> Self {
        Self {
            index,
            generation,
            pipeline_type,
        }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Get the pipeline type.
    pub fn pipeline_type(&self) -> PipelineType {
        self.pipeline_type
    }
}

// ============================================================================
// Shader Stages
// ============================================================================

/// Shader stage binding in a pipeline.
#[derive(Clone)]
pub struct ShaderStageBinding {
    /// Shader module.
    pub module: Arc<ShaderModule>,
    /// Entry point name.
    pub entry_point: String,
    /// Stage type.
    pub stage: ShaderStage,
    /// Specialization constants.
    pub specialization: Option<SpecializationConstants>,
}

impl ShaderStageBinding {
    /// Create a new shader stage binding.
    pub fn new(module: Arc<ShaderModule>, entry_point: &str, stage: ShaderStage) -> Self {
        Self {
            module,
            entry_point: String::from(entry_point),
            stage,
            specialization: None,
        }
    }

    /// Add specialization constants.
    pub fn with_specialization(mut self, constants: SpecializationConstants) -> Self {
        self.specialization = Some(constants);
        self
    }
}

// ============================================================================
// Graphics Pipeline
// ============================================================================

/// Primitive topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PrimitiveTopology {
    /// Point list.
    PointList,
    /// Line list.
    LineList,
    /// Line strip.
    LineStrip,
    /// Triangle list.
    #[default]
    TriangleList,
    /// Triangle strip.
    TriangleStrip,
    /// Triangle fan.
    TriangleFan,
    /// Line list with adjacency.
    LineListWithAdjacency,
    /// Line strip with adjacency.
    LineStripWithAdjacency,
    /// Triangle list with adjacency.
    TriangleListWithAdjacency,
    /// Triangle strip with adjacency.
    TriangleStripWithAdjacency,
    /// Patch list for tessellation.
    PatchList,
}

/// Render target format description.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderTargetFormats {
    /// Color attachment formats.
    pub color_formats: Vec<TextureFormat>,
    /// Depth/stencil format.
    pub depth_stencil_format: Option<TextureFormat>,
    /// Sample count.
    pub sample_count: u32,
}

impl Default for RenderTargetFormats {
    fn default() -> Self {
        Self {
            color_formats: Vec::new(),
            depth_stencil_format: None,
            sample_count: 1,
        }
    }
}

/// Texture format for render targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    // 8-bit formats
    /// R8 unsigned normalized.
    R8Unorm,
    /// R8 signed normalized.
    R8Snorm,
    /// R8 unsigned integer.
    R8Uint,
    /// R8 signed integer.
    R8Sint,

    // 16-bit formats
    /// R16 unsigned normalized.
    R16Unorm,
    /// R16 signed normalized.
    R16Snorm,
    /// R16 unsigned integer.
    R16Uint,
    /// R16 signed integer.
    R16Sint,
    /// R16 float.
    R16Float,
    /// RG8 unsigned normalized.
    Rg8Unorm,
    /// RG8 signed normalized.
    Rg8Snorm,
    /// RG8 unsigned integer.
    Rg8Uint,
    /// RG8 signed integer.
    Rg8Sint,

    // 32-bit formats
    /// R32 unsigned integer.
    R32Uint,
    /// R32 signed integer.
    R32Sint,
    /// R32 float.
    R32Float,
    /// RG16 unsigned normalized.
    Rg16Unorm,
    /// RG16 signed normalized.
    Rg16Snorm,
    /// RG16 unsigned integer.
    Rg16Uint,
    /// RG16 signed integer.
    Rg16Sint,
    /// RG16 float.
    Rg16Float,
    /// RGBA8 unsigned normalized.
    Rgba8Unorm,
    /// RGBA8 sRGB.
    Rgba8UnormSrgb,
    /// RGBA8 signed normalized.
    Rgba8Snorm,
    /// RGBA8 unsigned integer.
    Rgba8Uint,
    /// RGBA8 signed integer.
    Rgba8Sint,
    /// BGRA8 unsigned normalized.
    Bgra8Unorm,
    /// BGRA8 sRGB.
    Bgra8UnormSrgb,
    /// RGB10A2 unsigned normalized.
    Rgb10a2Unorm,
    /// RG11B10 float.
    Rg11b10Float,

    // 64-bit formats
    /// RG32 unsigned integer.
    Rg32Uint,
    /// RG32 signed integer.
    Rg32Sint,
    /// RG32 float.
    Rg32Float,
    /// RGBA16 unsigned normalized.
    Rgba16Unorm,
    /// RGBA16 signed normalized.
    Rgba16Snorm,
    /// RGBA16 unsigned integer.
    Rgba16Uint,
    /// RGBA16 signed integer.
    Rgba16Sint,
    /// RGBA16 float.
    Rgba16Float,

    // 128-bit formats
    /// RGBA32 unsigned integer.
    Rgba32Uint,
    /// RGBA32 signed integer.
    Rgba32Sint,
    /// RGBA32 float.
    Rgba32Float,

    // Depth/stencil formats
    /// Depth 16-bit.
    Depth16Unorm,
    /// Depth 24-bit.
    Depth24Unorm,
    /// Depth 32-bit float.
    Depth32Float,
    /// Depth 24-bit with stencil 8-bit.
    Depth24UnormStencil8,
    /// Depth 32-bit float with stencil 8-bit.
    Depth32FloatStencil8,
    /// Stencil 8-bit.
    Stencil8,

    // Compressed formats - BC
    /// BC1 RGB unsigned normalized.
    Bc1RgbaUnorm,
    /// BC1 RGB sRGB.
    Bc1RgbaUnormSrgb,
    /// BC2 RGBA unsigned normalized.
    Bc2RgbaUnorm,
    /// BC2 RGBA sRGB.
    Bc2RgbaUnormSrgb,
    /// BC3 RGBA unsigned normalized.
    Bc3RgbaUnorm,
    /// BC3 RGBA sRGB.
    Bc3RgbaUnormSrgb,
    /// BC4 R unsigned normalized.
    Bc4RUnorm,
    /// BC4 R signed normalized.
    Bc4RSnorm,
    /// BC5 RG unsigned normalized.
    Bc5RgUnorm,
    /// BC5 RG signed normalized.
    Bc5RgSnorm,
    /// BC6H RGB unsigned float.
    Bc6hRgbUfloat,
    /// BC6H RGB signed float.
    Bc6hRgbSfloat,
    /// BC7 RGBA unsigned normalized.
    Bc7RgbaUnorm,
    /// BC7 RGBA sRGB.
    Bc7RgbaUnormSrgb,

    // Compressed formats - ETC2
    /// ETC2 RGB8 unsigned normalized.
    Etc2Rgb8Unorm,
    /// ETC2 RGB8 sRGB.
    Etc2Rgb8UnormSrgb,
    /// ETC2 RGB8A1 unsigned normalized.
    Etc2Rgb8a1Unorm,
    /// ETC2 RGB8A1 sRGB.
    Etc2Rgb8a1UnormSrgb,
    /// ETC2 RGBA8 unsigned normalized.
    Etc2Rgba8Unorm,
    /// ETC2 RGBA8 sRGB.
    Etc2Rgba8UnormSrgb,

    // Compressed formats - ASTC
    /// ASTC 4x4 unsigned normalized.
    Astc4x4Unorm,
    /// ASTC 4x4 sRGB.
    Astc4x4UnormSrgb,
    /// ASTC 5x4 unsigned normalized.
    Astc5x4Unorm,
    /// ASTC 5x4 sRGB.
    Astc5x4UnormSrgb,
    /// ASTC 5x5 unsigned normalized.
    Astc5x5Unorm,
    /// ASTC 5x5 sRGB.
    Astc5x5UnormSrgb,
    /// ASTC 6x5 unsigned normalized.
    Astc6x5Unorm,
    /// ASTC 6x5 sRGB.
    Astc6x5UnormSrgb,
    /// ASTC 6x6 unsigned normalized.
    Astc6x6Unorm,
    /// ASTC 6x6 sRGB.
    Astc6x6UnormSrgb,
    /// ASTC 8x5 unsigned normalized.
    Astc8x5Unorm,
    /// ASTC 8x5 sRGB.
    Astc8x5UnormSrgb,
    /// ASTC 8x6 unsigned normalized.
    Astc8x6Unorm,
    /// ASTC 8x6 sRGB.
    Astc8x6UnormSrgb,
    /// ASTC 8x8 unsigned normalized.
    Astc8x8Unorm,
    /// ASTC 8x8 sRGB.
    Astc8x8UnormSrgb,
    /// ASTC 10x5 unsigned normalized.
    Astc10x5Unorm,
    /// ASTC 10x5 sRGB.
    Astc10x5UnormSrgb,
    /// ASTC 10x6 unsigned normalized.
    Astc10x6Unorm,
    /// ASTC 10x6 sRGB.
    Astc10x6UnormSrgb,
    /// ASTC 10x8 unsigned normalized.
    Astc10x8Unorm,
    /// ASTC 10x8 sRGB.
    Astc10x8UnormSrgb,
    /// ASTC 10x10 unsigned normalized.
    Astc10x10Unorm,
    /// ASTC 10x10 sRGB.
    Astc10x10UnormSrgb,
    /// ASTC 12x10 unsigned normalized.
    Astc12x10Unorm,
    /// ASTC 12x10 sRGB.
    Astc12x10UnormSrgb,
    /// ASTC 12x12 unsigned normalized.
    Astc12x12Unorm,
    /// ASTC 12x12 sRGB.
    Astc12x12UnormSrgb,
}

impl TextureFormat {
    /// Check if format is depth.
    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::Depth16Unorm
                | Self::Depth24Unorm
                | Self::Depth32Float
                | Self::Depth24UnormStencil8
                | Self::Depth32FloatStencil8
        )
    }

    /// Check if format is stencil.
    pub fn is_stencil(&self) -> bool {
        matches!(
            self,
            Self::Stencil8 | Self::Depth24UnormStencil8 | Self::Depth32FloatStencil8
        )
    }

    /// Check if format is compressed.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnorm
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnorm
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc4RUnorm
                | Self::Bc4RSnorm
                | Self::Bc5RgUnorm
                | Self::Bc5RgSnorm
                | Self::Bc6hRgbUfloat
                | Self::Bc6hRgbSfloat
                | Self::Bc7RgbaUnorm
                | Self::Bc7RgbaUnormSrgb
                | Self::Etc2Rgb8Unorm
                | Self::Etc2Rgb8UnormSrgb
                | Self::Etc2Rgb8a1Unorm
                | Self::Etc2Rgb8a1UnormSrgb
                | Self::Etc2Rgba8Unorm
                | Self::Etc2Rgba8UnormSrgb
                | Self::Astc4x4Unorm
                | Self::Astc4x4UnormSrgb
                | Self::Astc5x4Unorm
                | Self::Astc5x4UnormSrgb
                | Self::Astc5x5Unorm
                | Self::Astc5x5UnormSrgb
                | Self::Astc6x5Unorm
                | Self::Astc6x5UnormSrgb
                | Self::Astc6x6Unorm
                | Self::Astc6x6UnormSrgb
                | Self::Astc8x5Unorm
                | Self::Astc8x5UnormSrgb
                | Self::Astc8x6Unorm
                | Self::Astc8x6UnormSrgb
                | Self::Astc8x8Unorm
                | Self::Astc8x8UnormSrgb
                | Self::Astc10x5Unorm
                | Self::Astc10x5UnormSrgb
                | Self::Astc10x6Unorm
                | Self::Astc10x6UnormSrgb
                | Self::Astc10x8Unorm
                | Self::Astc10x8UnormSrgb
                | Self::Astc10x10Unorm
                | Self::Astc10x10UnormSrgb
                | Self::Astc12x10Unorm
                | Self::Astc12x10UnormSrgb
                | Self::Astc12x12Unorm
                | Self::Astc12x12UnormSrgb
        )
    }

    /// Check if format is sRGB.
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bgra8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
                | Self::Etc2Rgb8UnormSrgb
                | Self::Etc2Rgb8a1UnormSrgb
                | Self::Etc2Rgba8UnormSrgb
                | Self::Astc4x4UnormSrgb
                | Self::Astc5x4UnormSrgb
                | Self::Astc5x5UnormSrgb
                | Self::Astc6x5UnormSrgb
                | Self::Astc6x6UnormSrgb
                | Self::Astc8x5UnormSrgb
                | Self::Astc8x6UnormSrgb
                | Self::Astc8x8UnormSrgb
                | Self::Astc10x5UnormSrgb
                | Self::Astc10x6UnormSrgb
                | Self::Astc10x8UnormSrgb
                | Self::Astc10x10UnormSrgb
                | Self::Astc12x10UnormSrgb
                | Self::Astc12x12UnormSrgb
        )
    }

    /// Get bytes per pixel/block.
    pub fn bytes_per_block(&self) -> u32 {
        match self {
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint | Self::Stencil8 => 1,
            Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::Depth16Unorm => 2,
            Self::Depth24Unorm => 3,
            Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::Rg16Unorm
            | Self::Rg16Snorm
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::Rgba8Unorm
            | Self::Rgba8UnormSrgb
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8UnormSrgb
            | Self::Rgb10a2Unorm
            | Self::Rg11b10Float
            | Self::Depth32Float
            | Self::Depth24UnormStencil8 => 4,
            Self::Depth32FloatStencil8 => 5,
            Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::Rgba16Unorm
            | Self::Rgba16Snorm
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float
            | Self::Bc1RgbaUnorm
            | Self::Bc1RgbaUnormSrgb
            | Self::Bc4RUnorm
            | Self::Bc4RSnorm
            | Self::Etc2Rgb8Unorm
            | Self::Etc2Rgb8UnormSrgb
            | Self::Etc2Rgb8a1Unorm
            | Self::Etc2Rgb8a1UnormSrgb => 8,
            Self::Rgba32Uint
            | Self::Rgba32Sint
            | Self::Rgba32Float
            | Self::Bc2RgbaUnorm
            | Self::Bc2RgbaUnormSrgb
            | Self::Bc3RgbaUnorm
            | Self::Bc3RgbaUnormSrgb
            | Self::Bc5RgUnorm
            | Self::Bc5RgSnorm
            | Self::Bc6hRgbUfloat
            | Self::Bc6hRgbSfloat
            | Self::Bc7RgbaUnorm
            | Self::Bc7RgbaUnormSrgb
            | Self::Etc2Rgba8Unorm
            | Self::Etc2Rgba8UnormSrgb
            | Self::Astc4x4Unorm
            | Self::Astc4x4UnormSrgb
            | Self::Astc5x4Unorm
            | Self::Astc5x4UnormSrgb
            | Self::Astc5x5Unorm
            | Self::Astc5x5UnormSrgb
            | Self::Astc6x5Unorm
            | Self::Astc6x5UnormSrgb
            | Self::Astc6x6Unorm
            | Self::Astc6x6UnormSrgb
            | Self::Astc8x5Unorm
            | Self::Astc8x5UnormSrgb
            | Self::Astc8x6Unorm
            | Self::Astc8x6UnormSrgb
            | Self::Astc8x8Unorm
            | Self::Astc8x8UnormSrgb
            | Self::Astc10x5Unorm
            | Self::Astc10x5UnormSrgb
            | Self::Astc10x6Unorm
            | Self::Astc10x6UnormSrgb
            | Self::Astc10x8Unorm
            | Self::Astc10x8UnormSrgb
            | Self::Astc10x10Unorm
            | Self::Astc10x10UnormSrgb
            | Self::Astc12x10Unorm
            | Self::Astc12x10UnormSrgb
            | Self::Astc12x12Unorm
            | Self::Astc12x12UnormSrgb => 16,
        }
    }

    /// Get block dimensions (width, height).
    pub fn block_dimensions(&self) -> (u32, u32) {
        match self {
            // ASTC formats
            Self::Astc4x4Unorm | Self::Astc4x4UnormSrgb => (4, 4),
            Self::Astc5x4Unorm | Self::Astc5x4UnormSrgb => (5, 4),
            Self::Astc5x5Unorm | Self::Astc5x5UnormSrgb => (5, 5),
            Self::Astc6x5Unorm | Self::Astc6x5UnormSrgb => (6, 5),
            Self::Astc6x6Unorm | Self::Astc6x6UnormSrgb => (6, 6),
            Self::Astc8x5Unorm | Self::Astc8x5UnormSrgb => (8, 5),
            Self::Astc8x6Unorm | Self::Astc8x6UnormSrgb => (8, 6),
            Self::Astc8x8Unorm | Self::Astc8x8UnormSrgb => (8, 8),
            Self::Astc10x5Unorm | Self::Astc10x5UnormSrgb => (10, 5),
            Self::Astc10x6Unorm | Self::Astc10x6UnormSrgb => (10, 6),
            Self::Astc10x8Unorm | Self::Astc10x8UnormSrgb => (10, 8),
            Self::Astc10x10Unorm | Self::Astc10x10UnormSrgb => (10, 10),
            Self::Astc12x10Unorm | Self::Astc12x10UnormSrgb => (12, 10),
            Self::Astc12x12Unorm | Self::Astc12x12UnormSrgb => (12, 12),
            // BC and ETC2 formats
            Self::Bc1RgbaUnorm
            | Self::Bc1RgbaUnormSrgb
            | Self::Bc2RgbaUnorm
            | Self::Bc2RgbaUnormSrgb
            | Self::Bc3RgbaUnorm
            | Self::Bc3RgbaUnormSrgb
            | Self::Bc4RUnorm
            | Self::Bc4RSnorm
            | Self::Bc5RgUnorm
            | Self::Bc5RgSnorm
            | Self::Bc6hRgbUfloat
            | Self::Bc6hRgbSfloat
            | Self::Bc7RgbaUnorm
            | Self::Bc7RgbaUnormSrgb
            | Self::Etc2Rgb8Unorm
            | Self::Etc2Rgb8UnormSrgb
            | Self::Etc2Rgb8a1Unorm
            | Self::Etc2Rgb8a1UnormSrgb
            | Self::Etc2Rgba8Unorm
            | Self::Etc2Rgba8UnormSrgb => (4, 4),
            // All other formats are 1x1
            _ => (1, 1),
        }
    }
}

/// Graphics pipeline description.
#[derive(Clone)]
pub struct GraphicsPipelineDesc {
    /// Debug name.
    pub name: String,
    /// Vertex shader.
    pub vertex_shader: ShaderStageBinding,
    /// Fragment shader.
    pub fragment_shader: Option<ShaderStageBinding>,
    /// Tessellation control shader.
    pub tess_control_shader: Option<ShaderStageBinding>,
    /// Tessellation evaluation shader.
    pub tess_eval_shader: Option<ShaderStageBinding>,
    /// Geometry shader.
    pub geometry_shader: Option<ShaderStageBinding>,
    /// Task shader (mesh shading).
    pub task_shader: Option<ShaderStageBinding>,
    /// Mesh shader (mesh shading).
    pub mesh_shader: Option<ShaderStageBinding>,
    /// Pipeline layout.
    pub layout: Arc<PipelineLayout>,
    /// Vertex input layout.
    pub vertex_layout: VertexLayout,
    /// Primitive topology.
    pub topology: PrimitiveTopology,
    /// Rasterizer state.
    pub raster_state: RasterState,
    /// Depth/stencil state.
    pub depth_state: DepthState,
    /// Blend state.
    pub blend_state: BlendState,
    /// Render target formats.
    pub render_targets: RenderTargetFormats,
    /// Dynamic state flags.
    pub dynamic_state: DynamicStateFlags,
    /// Patch control points for tessellation.
    pub patch_control_points: u32,
    /// View mask for multi-view rendering.
    pub view_mask: u32,
}

/// Graphics pipeline.
pub struct GraphicsPipeline {
    /// Description.
    desc: GraphicsPipelineDesc,
    /// Handle.
    handle: PipelineHandle,
    /// Hash for caching.
    hash: u64,
}

impl GraphicsPipeline {
    /// Create a new graphics pipeline.
    pub fn new(desc: GraphicsPipelineDesc, handle: PipelineHandle) -> Self {
        let hash = Self::compute_hash(&desc);
        Self { desc, handle, hash }
    }

    /// Get the description.
    pub fn desc(&self) -> &GraphicsPipelineDesc {
        &self.desc
    }

    /// Get the handle.
    pub fn handle(&self) -> PipelineHandle {
        self.handle
    }

    /// Get the hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Compute hash for the pipeline.
    fn compute_hash(desc: &GraphicsPipelineDesc) -> u64 {
        use core::hash::Hasher;
        let mut hasher = FnvHasher::new();
        desc.name.hash(&mut hasher);
        desc.topology.hash(&mut hasher);
        desc.patch_control_points.hash(&mut hasher);
        desc.view_mask.hash(&mut hasher);
        hasher.finish()
    }
}

// ============================================================================
// Graphics Pipeline Builder
// ============================================================================

/// Builder for graphics pipelines.
pub struct GraphicsPipelineBuilder {
    name: String,
    vertex_shader: Option<ShaderStageBinding>,
    fragment_shader: Option<ShaderStageBinding>,
    tess_control_shader: Option<ShaderStageBinding>,
    tess_eval_shader: Option<ShaderStageBinding>,
    geometry_shader: Option<ShaderStageBinding>,
    task_shader: Option<ShaderStageBinding>,
    mesh_shader: Option<ShaderStageBinding>,
    layout: Option<Arc<PipelineLayout>>,
    vertex_layout: VertexLayout,
    topology: PrimitiveTopology,
    raster_state: RasterState,
    depth_state: DepthState,
    blend_state: BlendState,
    render_targets: RenderTargetFormats,
    dynamic_state: DynamicStateFlags,
    patch_control_points: u32,
    view_mask: u32,
}

impl GraphicsPipelineBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            vertex_shader: None,
            fragment_shader: None,
            tess_control_shader: None,
            tess_eval_shader: None,
            geometry_shader: None,
            task_shader: None,
            mesh_shader: None,
            layout: None,
            vertex_layout: VertexLayout::default(),
            topology: PrimitiveTopology::TriangleList,
            raster_state: RasterState::default(),
            depth_state: DepthState::default(),
            blend_state: BlendState::default(),
            render_targets: RenderTargetFormats::default(),
            dynamic_state: DynamicStateFlags::empty(),
            patch_control_points: 0,
            view_mask: 0,
        }
    }

    /// Set the debug name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Set the vertex shader.
    pub fn vertex_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.vertex_shader = Some(binding);
        self
    }

    /// Set the fragment shader.
    pub fn fragment_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.fragment_shader = Some(binding);
        self
    }

    /// Set the tessellation control shader.
    pub fn tess_control_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.tess_control_shader = Some(binding);
        self
    }

    /// Set the tessellation evaluation shader.
    pub fn tess_eval_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.tess_eval_shader = Some(binding);
        self
    }

    /// Set the geometry shader.
    pub fn geometry_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.geometry_shader = Some(binding);
        self
    }

    /// Set the task shader.
    pub fn task_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.task_shader = Some(binding);
        self
    }

    /// Set the mesh shader.
    pub fn mesh_shader(mut self, binding: ShaderStageBinding) -> Self {
        self.mesh_shader = Some(binding);
        self
    }

    /// Set the pipeline layout.
    pub fn layout(mut self, layout: Arc<PipelineLayout>) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Set the vertex layout.
    pub fn vertex_layout(mut self, layout: VertexLayout) -> Self {
        self.vertex_layout = layout;
        self
    }

    /// Set the primitive topology.
    pub fn topology(mut self, topology: PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    /// Set the rasterizer state.
    pub fn raster_state(mut self, state: RasterState) -> Self {
        self.raster_state = state;
        self
    }

    /// Set the depth/stencil state.
    pub fn depth_state(mut self, state: DepthState) -> Self {
        self.depth_state = state;
        self
    }

    /// Set the blend state.
    pub fn blend_state(mut self, state: BlendState) -> Self {
        self.blend_state = state;
        self
    }

    /// Set render target formats.
    pub fn render_targets(mut self, formats: RenderTargetFormats) -> Self {
        self.render_targets = formats;
        self
    }

    /// Add a color format.
    pub fn color_format(mut self, format: TextureFormat) -> Self {
        self.render_targets.color_formats.push(format);
        self
    }

    /// Set depth/stencil format.
    pub fn depth_format(mut self, format: TextureFormat) -> Self {
        self.render_targets.depth_stencil_format = Some(format);
        self
    }

    /// Set sample count.
    pub fn sample_count(mut self, count: u32) -> Self {
        self.render_targets.sample_count = count;
        self
    }

    /// Set dynamic state flags.
    pub fn dynamic_state(mut self, flags: DynamicStateFlags) -> Self {
        self.dynamic_state = flags;
        self
    }

    /// Set patch control points.
    pub fn patch_control_points(mut self, points: u32) -> Self {
        self.patch_control_points = points;
        self
    }

    /// Set view mask for multi-view rendering.
    pub fn view_mask(mut self, mask: u32) -> Self {
        self.view_mask = mask;
        self
    }

    /// Enable depth testing.
    pub fn depth_test(mut self, enabled: bool) -> Self {
        self.depth_state.depth_test_enable = enabled;
        self
    }

    /// Enable depth writing.
    pub fn depth_write(mut self, enabled: bool) -> Self {
        self.depth_state.depth_write_enable = enabled;
        self
    }

    /// Set cull mode.
    pub fn cull_mode(mut self, mode: crate::raster::CullMode) -> Self {
        self.raster_state.cull_mode = mode;
        self
    }

    /// Build the pipeline description.
    pub fn build_desc(self) -> Result<GraphicsPipelineDesc, PipelineError> {
        let vertex_shader = self
            .vertex_shader
            .ok_or(PipelineError::MissingVertexShader)?;

        let layout = self.layout.ok_or(PipelineError::MissingLayout)?;

        Ok(GraphicsPipelineDesc {
            name: self.name,
            vertex_shader,
            fragment_shader: self.fragment_shader,
            tess_control_shader: self.tess_control_shader,
            tess_eval_shader: self.tess_eval_shader,
            geometry_shader: self.geometry_shader,
            task_shader: self.task_shader,
            mesh_shader: self.mesh_shader,
            layout,
            vertex_layout: self.vertex_layout,
            topology: self.topology,
            raster_state: self.raster_state,
            depth_state: self.depth_state,
            blend_state: self.blend_state,
            render_targets: self.render_targets,
            dynamic_state: self.dynamic_state,
            patch_control_points: self.patch_control_points,
            view_mask: self.view_mask,
        })
    }
}

impl Default for GraphicsPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Compute Pipeline
// ============================================================================

/// Compute pipeline description.
#[derive(Clone)]
pub struct ComputePipelineDesc {
    /// Debug name.
    pub name: String,
    /// Compute shader.
    pub shader: ShaderStageBinding,
    /// Pipeline layout.
    pub layout: Arc<PipelineLayout>,
    /// Required subgroup size.
    pub required_subgroup_size: Option<u32>,
}

/// Compute pipeline.
pub struct ComputePipeline {
    /// Description.
    desc: ComputePipelineDesc,
    /// Handle.
    handle: PipelineHandle,
    /// Hash for caching.
    hash: u64,
}

impl ComputePipeline {
    /// Create a new compute pipeline.
    pub fn new(desc: ComputePipelineDesc, handle: PipelineHandle) -> Self {
        let hash = Self::compute_hash(&desc);
        Self { desc, handle, hash }
    }

    /// Get the description.
    pub fn desc(&self) -> &ComputePipelineDesc {
        &self.desc
    }

    /// Get the handle.
    pub fn handle(&self) -> PipelineHandle {
        self.handle
    }

    /// Get the hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Compute hash for the pipeline.
    fn compute_hash(desc: &ComputePipelineDesc) -> u64 {
        let mut hasher = FnvHasher::new();
        desc.name.hash(&mut hasher);
        desc.required_subgroup_size.hash(&mut hasher);
        hasher.finish()
    }
}

/// Builder for compute pipelines.
pub struct ComputePipelineBuilder {
    name: String,
    shader: Option<ShaderStageBinding>,
    layout: Option<Arc<PipelineLayout>>,
    required_subgroup_size: Option<u32>,
}

impl ComputePipelineBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            shader: None,
            layout: None,
            required_subgroup_size: None,
        }
    }

    /// Set the debug name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Set the compute shader.
    pub fn shader(mut self, binding: ShaderStageBinding) -> Self {
        self.shader = Some(binding);
        self
    }

    /// Set the pipeline layout.
    pub fn layout(mut self, layout: Arc<PipelineLayout>) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Set required subgroup size.
    pub fn required_subgroup_size(mut self, size: u32) -> Self {
        self.required_subgroup_size = Some(size);
        self
    }

    /// Build the pipeline description.
    pub fn build_desc(self) -> Result<ComputePipelineDesc, PipelineError> {
        let shader = self.shader.ok_or(PipelineError::MissingComputeShader)?;
        let layout = self.layout.ok_or(PipelineError::MissingLayout)?;

        Ok(ComputePipelineDesc {
            name: self.name,
            shader,
            layout,
            required_subgroup_size: self.required_subgroup_size,
        })
    }
}

impl Default for ComputePipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ray Tracing Pipeline
// ============================================================================

/// Ray tracing shader group type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayTracingShaderGroupType {
    /// Ray generation shader.
    RayGeneration,
    /// Miss shader.
    Miss,
    /// Hit group (closest hit, any hit, intersection).
    Hit,
    /// Callable shader.
    Callable,
}

/// Ray tracing shader group.
#[derive(Clone)]
pub struct RayTracingShaderGroup {
    /// Group type.
    pub group_type: RayTracingShaderGroupType,
    /// General shader index (ray gen, miss, callable).
    pub general_shader: Option<u32>,
    /// Closest hit shader index.
    pub closest_hit_shader: Option<u32>,
    /// Any hit shader index.
    pub any_hit_shader: Option<u32>,
    /// Intersection shader index.
    pub intersection_shader: Option<u32>,
}

impl RayTracingShaderGroup {
    /// Create a ray generation group.
    pub fn ray_generation(shader_index: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::RayGeneration,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }

    /// Create a miss group.
    pub fn miss(shader_index: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::Miss,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }

    /// Create a hit group.
    pub fn hit(closest_hit: Option<u32>, any_hit: Option<u32>, intersection: Option<u32>) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::Hit,
            general_shader: None,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: intersection,
        }
    }

    /// Create a callable group.
    pub fn callable(shader_index: u32) -> Self {
        Self {
            group_type: RayTracingShaderGroupType::Callable,
            general_shader: Some(shader_index),
            closest_hit_shader: None,
            any_hit_shader: None,
            intersection_shader: None,
        }
    }
}

/// Ray tracing pipeline description.
#[derive(Clone)]
pub struct RayTracingPipelineDesc {
    /// Debug name.
    pub name: String,
    /// Shader stages.
    pub stages: Vec<ShaderStageBinding>,
    /// Shader groups.
    pub groups: Vec<RayTracingShaderGroup>,
    /// Pipeline layout.
    pub layout: Arc<PipelineLayout>,
    /// Maximum recursion depth.
    pub max_recursion_depth: u32,
    /// Maximum ray hit attribute size.
    pub max_ray_hit_attribute_size: u32,
    /// Maximum ray payload size.
    pub max_ray_payload_size: u32,
}

/// Ray tracing pipeline.
pub struct RayTracingPipeline {
    /// Description.
    desc: RayTracingPipelineDesc,
    /// Handle.
    handle: PipelineHandle,
    /// Hash for caching.
    hash: u64,
    /// Shader group handles.
    shader_group_handles: Vec<u8>,
}

impl RayTracingPipeline {
    /// Create a new ray tracing pipeline.
    pub fn new(
        desc: RayTracingPipelineDesc,
        handle: PipelineHandle,
        shader_group_handles: Vec<u8>,
    ) -> Self {
        let hash = Self::compute_hash(&desc);
        Self {
            desc,
            handle,
            hash,
            shader_group_handles,
        }
    }

    /// Get the description.
    pub fn desc(&self) -> &RayTracingPipelineDesc {
        &self.desc
    }

    /// Get the handle.
    pub fn handle(&self) -> PipelineHandle {
        self.handle
    }

    /// Get the hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Get shader group handles.
    pub fn shader_group_handles(&self) -> &[u8] {
        &self.shader_group_handles
    }

    /// Compute hash for the pipeline.
    fn compute_hash(desc: &RayTracingPipelineDesc) -> u64 {
        let mut hasher = FnvHasher::new();
        desc.name.hash(&mut hasher);
        desc.max_recursion_depth.hash(&mut hasher);
        desc.max_ray_hit_attribute_size.hash(&mut hasher);
        desc.max_ray_payload_size.hash(&mut hasher);
        hasher.finish()
    }
}

/// Builder for ray tracing pipelines.
pub struct RayTracingPipelineBuilder {
    name: String,
    stages: Vec<ShaderStageBinding>,
    groups: Vec<RayTracingShaderGroup>,
    layout: Option<Arc<PipelineLayout>>,
    max_recursion_depth: u32,
    max_ray_hit_attribute_size: u32,
    max_ray_payload_size: u32,
}

impl RayTracingPipelineBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            stages: Vec::new(),
            groups: Vec::new(),
            layout: None,
            max_recursion_depth: 1,
            max_ray_hit_attribute_size: 8,
            max_ray_payload_size: 32,
        }
    }

    /// Set the debug name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Add a shader stage.
    pub fn add_stage(mut self, binding: ShaderStageBinding) -> Self {
        self.stages.push(binding);
        self
    }

    /// Add a shader group.
    pub fn add_group(mut self, group: RayTracingShaderGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// Set the pipeline layout.
    pub fn layout(mut self, layout: Arc<PipelineLayout>) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Set maximum recursion depth.
    pub fn max_recursion_depth(mut self, depth: u32) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Set maximum ray hit attribute size.
    pub fn max_ray_hit_attribute_size(mut self, size: u32) -> Self {
        self.max_ray_hit_attribute_size = size;
        self
    }

    /// Set maximum ray payload size.
    pub fn max_ray_payload_size(mut self, size: u32) -> Self {
        self.max_ray_payload_size = size;
        self
    }

    /// Build the pipeline description.
    pub fn build_desc(self) -> Result<RayTracingPipelineDesc, PipelineError> {
        if self.stages.is_empty() {
            return Err(PipelineError::MissingShaderStages);
        }
        if self.groups.is_empty() {
            return Err(PipelineError::MissingShaderGroups);
        }
        let layout = self.layout.ok_or(PipelineError::MissingLayout)?;

        Ok(RayTracingPipelineDesc {
            name: self.name,
            stages: self.stages,
            groups: self.groups,
            layout,
            max_recursion_depth: self.max_recursion_depth,
            max_ray_hit_attribute_size: self.max_ray_hit_attribute_size,
            max_ray_payload_size: self.max_ray_payload_size,
        })
    }
}

impl Default for RayTracingPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Pipeline Manager
// ============================================================================

/// Pipeline creation error.
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// Missing vertex shader.
    MissingVertexShader,
    /// Missing compute shader.
    MissingComputeShader,
    /// Missing pipeline layout.
    MissingLayout,
    /// Missing shader stages.
    MissingShaderStages,
    /// Missing shader groups.
    MissingShaderGroups,
    /// Shader compilation error.
    ShaderCompilation(String),
    /// Pipeline creation failed.
    CreationFailed(String),
    /// Invalid configuration.
    InvalidConfiguration(String),
}

/// Pipeline manager for creating and caching pipelines.
pub struct PipelineManager {
    /// Graphics pipelines.
    graphics_pipelines: Vec<Option<GraphicsPipeline>>,
    /// Compute pipelines.
    compute_pipelines: Vec<Option<ComputePipeline>>,
    /// Ray tracing pipelines.
    raytracing_pipelines: Vec<Option<RayTracingPipeline>>,
    /// Current generation.
    generation: u32,
    /// Pipeline cache.
    cache: Option<Box<crate::cache::PipelineCache>>,
}

impl PipelineManager {
    /// Create a new pipeline manager.
    pub fn new() -> Self {
        Self {
            graphics_pipelines: Vec::new(),
            compute_pipelines: Vec::new(),
            raytracing_pipelines: Vec::new(),
            generation: 0,
            cache: None,
        }
    }

    /// Create a new pipeline manager with cache.
    pub fn with_cache(cache: crate::cache::PipelineCache) -> Self {
        Self {
            graphics_pipelines: Vec::new(),
            compute_pipelines: Vec::new(),
            raytracing_pipelines: Vec::new(),
            generation: 0,
            cache: Some(Box::new(cache)),
        }
    }

    /// Create a graphics pipeline.
    pub fn create_graphics_pipeline(
        &mut self,
        desc: GraphicsPipelineDesc,
    ) -> Result<PipelineHandle, PipelineError> {
        let index = self.graphics_pipelines.len() as u32;
        let handle = PipelineHandle::new(index, self.generation, PipelineType::Graphics);
        let pipeline = GraphicsPipeline::new(desc, handle);
        self.graphics_pipelines.push(Some(pipeline));
        Ok(handle)
    }

    /// Create a compute pipeline.
    pub fn create_compute_pipeline(
        &mut self,
        desc: ComputePipelineDesc,
    ) -> Result<PipelineHandle, PipelineError> {
        let index = self.compute_pipelines.len() as u32;
        let handle = PipelineHandle::new(index, self.generation, PipelineType::Compute);
        let pipeline = ComputePipeline::new(desc, handle);
        self.compute_pipelines.push(Some(pipeline));
        Ok(handle)
    }

    /// Create a ray tracing pipeline.
    pub fn create_raytracing_pipeline(
        &mut self,
        desc: RayTracingPipelineDesc,
    ) -> Result<PipelineHandle, PipelineError> {
        let index = self.raytracing_pipelines.len() as u32;
        let handle = PipelineHandle::new(index, self.generation, PipelineType::RayTracing);
        let pipeline = RayTracingPipeline::new(desc, handle, Vec::new());
        self.raytracing_pipelines.push(Some(pipeline));
        Ok(handle)
    }

    /// Get a graphics pipeline.
    pub fn get_graphics_pipeline(&self, handle: PipelineHandle) -> Option<&GraphicsPipeline> {
        if handle.pipeline_type != PipelineType::Graphics {
            return None;
        }
        self.graphics_pipelines
            .get(handle.index as usize)?
            .as_ref()
    }

    /// Get a compute pipeline.
    pub fn get_compute_pipeline(&self, handle: PipelineHandle) -> Option<&ComputePipeline> {
        if handle.pipeline_type != PipelineType::Compute {
            return None;
        }
        self.compute_pipelines.get(handle.index as usize)?.as_ref()
    }

    /// Get a ray tracing pipeline.
    pub fn get_raytracing_pipeline(&self, handle: PipelineHandle) -> Option<&RayTracingPipeline> {
        if handle.pipeline_type != PipelineType::RayTracing {
            return None;
        }
        self.raytracing_pipelines
            .get(handle.index as usize)?
            .as_ref()
    }

    /// Destroy a graphics pipeline.
    pub fn destroy_graphics_pipeline(&mut self, handle: PipelineHandle) {
        if handle.pipeline_type == PipelineType::Graphics {
            if let Some(slot) = self.graphics_pipelines.get_mut(handle.index as usize) {
                *slot = None;
            }
        }
    }

    /// Destroy a compute pipeline.
    pub fn destroy_compute_pipeline(&mut self, handle: PipelineHandle) {
        if handle.pipeline_type == PipelineType::Compute {
            if let Some(slot) = self.compute_pipelines.get_mut(handle.index as usize) {
                *slot = None;
            }
        }
    }

    /// Destroy a ray tracing pipeline.
    pub fn destroy_raytracing_pipeline(&mut self, handle: PipelineHandle) {
        if handle.pipeline_type == PipelineType::RayTracing {
            if let Some(slot) = self.raytracing_pipelines.get_mut(handle.index as usize) {
                *slot = None;
            }
        }
    }

    /// Get pipeline count.
    pub fn pipeline_count(&self) -> usize {
        self.graphics_pipelines.iter().filter(|p| p.is_some()).count()
            + self.compute_pipelines.iter().filter(|p| p.is_some()).count()
            + self
                .raytracing_pipelines
                .iter()
                .filter(|p| p.is_some())
                .count()
    }
}

impl Default for PipelineManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher for pipeline hashing.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
