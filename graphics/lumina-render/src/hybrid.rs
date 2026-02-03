//! Hybrid Rendering - Seamless Ray Tracing and Rasterization Fusion
//!
//! This revolutionary module provides a hybrid rendering approach that combines:
//! - Hardware ray tracing for reflections, shadows, and GI
//! - High-performance rasterization for primary visibility
//! - Adaptive switching based on scene complexity and GPU capabilities
//! - Seamless blending between techniques

use alloc::{boxed::Box, string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::graph::{RenderGraph, VirtualTextureHandle};
use crate::pass::{PassContext, PassType, RenderPass, ShaderStages};
use crate::resource::{BufferDesc, BufferHandle, TextureDesc, TextureFormat, TextureHandle};

/// Hybrid renderer combining ray tracing and rasterization.
pub struct HybridRenderer {
    /// Configuration.
    config: HybridConfig,
    /// Current mode.
    mode: RenderMode,
    /// Ray tracing support level.
    rt_support: RayTracingSupport,
    /// Acceleration structure.
    accel_structure: Option<AccelerationStructure>,
    /// Statistics.
    stats: HybridStats,
}

impl HybridRenderer {
    /// Create a new hybrid renderer.
    pub fn new(config: HybridConfig) -> Self {
        Self {
            config,
            mode: RenderMode::Auto,
            rt_support: RayTracingSupport::None,
            accel_structure: None,
            stats: HybridStats::default(),
        }
    }

    /// Initialize with device capabilities.
    pub fn initialize(&mut self, caps: &DeviceCapabilities) {
        self.rt_support = if caps.ray_tracing_pipeline && caps.acceleration_structure {
            if caps.ray_query {
                RayTracingSupport::Full
            } else {
                RayTracingSupport::Pipeline
            }
        } else if caps.ray_query {
            RayTracingSupport::InlineOnly
        } else {
            RayTracingSupport::None
        };
    }

    /// Set render mode.
    pub fn set_mode(&mut self, mode: RenderMode) {
        self.mode = mode;
    }

    /// Build acceleration structure from scene.
    pub fn build_acceleration_structure(&mut self, scene: &SceneData) {
        let mut blas_instances = Vec::new();

        for mesh in &scene.meshes {
            let blas = BottomLevelAS {
                vertex_buffer: mesh.vertex_buffer,
                index_buffer: mesh.index_buffer,
                vertex_count: mesh.vertex_count,
                index_count: mesh.index_count,
                vertex_stride: mesh.vertex_stride,
                transform: mesh.transform,
                flags: GeometryFlags::OPAQUE,
            };
            blas_instances.push(blas);
        }

        self.accel_structure = Some(AccelerationStructure {
            tlas: TopLevelAS {
                instances: blas_instances.len() as u32,
                instance_buffer: BufferHandle::INVALID,
            },
            blas: blas_instances,
            build_flags: BuildFlags::PREFER_FAST_TRACE,
        });
    }

    /// Add hybrid rendering passes to graph.
    pub fn add_passes(&self, graph: &mut RenderGraph, targets: &HybridTargets) {
        match self.determine_mode() {
            RenderMode::RasterOnly => {
                self.add_raster_passes(graph, targets);
            }
            RenderMode::RayTracingOnly => {
                self.add_rt_passes(graph, targets);
            }
            RenderMode::Hybrid | RenderMode::Auto => {
                self.add_hybrid_passes(graph, targets);
            }
        }
    }

    /// Determine effective render mode.
    fn determine_mode(&self) -> RenderMode {
        match self.mode {
            RenderMode::Auto => {
                match self.rt_support {
                    RayTracingSupport::Full => RenderMode::Hybrid,
                    RayTracingSupport::Pipeline | RayTracingSupport::InlineOnly => {
                        RenderMode::Hybrid
                    }
                    RayTracingSupport::None => RenderMode::RasterOnly,
                }
            }
            other => other,
        }
    }

    fn add_raster_passes(&self, graph: &mut RenderGraph, targets: &HybridTargets) {
        // Shadow pass
        graph.add_pass("shadow_raster", |builder| {
            builder.write_depth(targets.shadow_map);
        });

        // GBuffer pass
        graph.add_pass("gbuffer", |builder| {
            builder
                .write_color(targets.gbuffer_albedo)
                .write_color(targets.gbuffer_normal)
                .write_color(targets.gbuffer_material)
                .write_depth(targets.depth);
        });

        // Screen-space reflections
        graph.add_pass("ssr", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.depth)
                .read_texture(targets.color_history)
                .write_color(targets.reflections);
        });

        // Screen-space shadows
        graph.add_pass("screen_shadows", |builder| {
            builder
                .read_texture(targets.depth)
                .read_texture(targets.shadow_map)
                .write_color(targets.shadows);
        });

        // Lighting
        graph.add_pass("deferred_lighting", |builder| {
            builder
                .read_texture(targets.gbuffer_albedo)
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.gbuffer_material)
                .read_texture(targets.depth)
                .read_texture(targets.reflections)
                .read_texture(targets.shadows)
                .write_color(targets.hdr_output);
        });
    }

    fn add_rt_passes(&self, graph: &mut RenderGraph, targets: &HybridTargets) {
        // Ray traced primary visibility
        graph.add_pass("rt_primary", |builder| {
            builder
                .write_color(targets.gbuffer_albedo)
                .write_color(targets.gbuffer_normal)
                .write_color(targets.gbuffer_material)
                .write_color(targets.depth);
        });

        // Ray traced shadows
        graph.add_pass("rt_shadows", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.depth)
                .write_color(targets.shadows);
        });

        // Ray traced reflections
        graph.add_pass("rt_reflections", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.gbuffer_material)
                .read_texture(targets.depth)
                .write_color(targets.reflections);
        });

        // Ray traced GI
        graph.add_pass("rt_gi", |builder| {
            builder
                .read_texture(targets.gbuffer_albedo)
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.depth)
                .write_color(targets.gi_output);
        });

        // Compose
        graph.add_pass("compose", |builder| {
            builder
                .read_texture(targets.gbuffer_albedo)
                .read_texture(targets.reflections)
                .read_texture(targets.shadows)
                .read_texture(targets.gi_output)
                .write_color(targets.hdr_output);
        });
    }

    fn add_hybrid_passes(&self, graph: &mut RenderGraph, targets: &HybridTargets) {
        // Rasterized GBuffer for primary visibility (fast)
        graph.add_pass("gbuffer", |builder| {
            builder
                .write_color(targets.gbuffer_albedo)
                .write_color(targets.gbuffer_normal)
                .write_color(targets.gbuffer_material)
                .write_color(targets.motion_vectors)
                .write_depth(targets.depth);
        });

        // Ray traced shadows (accurate)
        graph.add_compute_pass("rt_shadows", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.depth)
                .storage_image(targets.shadows);
        });

        // Hybrid reflections - RT for rough, SSR for smooth
        graph.add_compute_pass("hybrid_reflections", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.gbuffer_material)
                .read_texture(targets.depth)
                .read_texture(targets.color_history)
                .storage_image(targets.reflections);
        });

        // Ray traced ambient occlusion
        graph.add_compute_pass("rtao", |builder| {
            builder
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.depth)
                .storage_image(targets.ao);
        });

        // Deferred lighting with all effects
        graph.add_pass("deferred_lighting", |builder| {
            builder
                .read_texture(targets.gbuffer_albedo)
                .read_texture(targets.gbuffer_normal)
                .read_texture(targets.gbuffer_material)
                .read_texture(targets.depth)
                .read_texture(targets.reflections)
                .read_texture(targets.shadows)
                .read_texture(targets.ao)
                .write_color(targets.hdr_output);
        });
    }

    /// Get statistics.
    pub fn stats(&self) -> &HybridStats {
        &self.stats
    }
}

/// Hybrid rendering configuration.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Maximum ray bounces.
    pub max_bounces: u32,
    /// Samples per pixel for reflections.
    pub reflection_samples: u32,
    /// Samples per pixel for shadows.
    pub shadow_samples: u32,
    /// Samples per pixel for GI.
    pub gi_samples: u32,
    /// Enable temporal accumulation.
    pub temporal_accumulation: bool,
    /// Roughness threshold for RT reflections.
    pub rt_roughness_threshold: f32,
    /// Distance threshold for RT shadows.
    pub rt_shadow_distance: f32,
    /// Enable adaptive sampling.
    pub adaptive_sampling: bool,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            max_bounces: 2,
            reflection_samples: 1,
            shadow_samples: 1,
            gi_samples: 1,
            temporal_accumulation: true,
            rt_roughness_threshold: 0.5,
            rt_shadow_distance: 100.0,
            adaptive_sampling: true,
        }
    }
}

/// Render mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Automatic mode selection.
    Auto,
    /// Rasterization only.
    RasterOnly,
    /// Ray tracing only.
    RayTracingOnly,
    /// Hybrid mode.
    Hybrid,
}

/// Ray tracing support level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RayTracingSupport {
    /// No ray tracing.
    None,
    /// Inline ray tracing only (ray query).
    InlineOnly,
    /// Full ray tracing pipeline.
    Pipeline,
    /// Full support with ray query.
    Full,
}

/// Device capabilities.
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Ray tracing pipeline support.
    pub ray_tracing_pipeline: bool,
    /// Acceleration structure support.
    pub acceleration_structure: bool,
    /// Ray query support.
    pub ray_query: bool,
    /// Mesh shader support.
    pub mesh_shaders: bool,
    /// Variable rate shading support.
    pub variable_rate_shading: bool,
    /// Max ray recursion depth.
    pub max_ray_recursion: u32,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            ray_tracing_pipeline: false,
            acceleration_structure: false,
            ray_query: false,
            mesh_shaders: false,
            variable_rate_shading: false,
            max_ray_recursion: 1,
        }
    }
}

/// Hybrid render targets.
#[derive(Debug, Clone)]
pub struct HybridTargets {
    /// GBuffer albedo + metallic.
    pub gbuffer_albedo: VirtualTextureHandle,
    /// GBuffer normal + roughness.
    pub gbuffer_normal: VirtualTextureHandle,
    /// GBuffer material properties.
    pub gbuffer_material: VirtualTextureHandle,
    /// Motion vectors.
    pub motion_vectors: VirtualTextureHandle,
    /// Depth buffer.
    pub depth: VirtualTextureHandle,
    /// Shadow map.
    pub shadow_map: VirtualTextureHandle,
    /// Shadows result.
    pub shadows: VirtualTextureHandle,
    /// Reflections result.
    pub reflections: VirtualTextureHandle,
    /// Ambient occlusion.
    pub ao: VirtualTextureHandle,
    /// Global illumination.
    pub gi_output: VirtualTextureHandle,
    /// HDR output.
    pub hdr_output: VirtualTextureHandle,
    /// Color history for temporal.
    pub color_history: VirtualTextureHandle,
}

/// Acceleration structure for ray tracing.
#[derive(Debug)]
pub struct AccelerationStructure {
    /// Top-level AS.
    pub tlas: TopLevelAS,
    /// Bottom-level AS instances.
    pub blas: Vec<BottomLevelAS>,
    /// Build flags.
    pub build_flags: BuildFlags,
}

/// Top-level acceleration structure.
#[derive(Debug)]
pub struct TopLevelAS {
    /// Instance count.
    pub instances: u32,
    /// Instance buffer.
    pub instance_buffer: BufferHandle,
}

/// Bottom-level acceleration structure.
#[derive(Debug)]
pub struct BottomLevelAS {
    /// Vertex buffer.
    pub vertex_buffer: BufferHandle,
    /// Index buffer.
    pub index_buffer: BufferHandle,
    /// Vertex count.
    pub vertex_count: u32,
    /// Index count.
    pub index_count: u32,
    /// Vertex stride.
    pub vertex_stride: u32,
    /// Transform matrix.
    pub transform: [[f32; 4]; 3],
    /// Geometry flags.
    pub flags: GeometryFlags,
}

/// Geometry flags for BLAS.
#[derive(Debug, Clone, Copy)]
pub struct GeometryFlags(u32);

impl GeometryFlags {
    /// Opaque geometry.
    pub const OPAQUE: Self = Self(1 << 0);
    /// No duplicate any-hit invocation.
    pub const NO_DUPLICATE_ANY_HIT: Self = Self(1 << 1);
}

/// Build flags for acceleration structures.
#[derive(Debug, Clone, Copy)]
pub enum BuildFlags {
    /// Prefer fast trace.
    PreferFastTrace,
    /// Prefer fast build.
    PreferFastBuild,
    /// Allow update.
    AllowUpdate,
    /// Low memory.
    LowMemory,
}

/// Scene data for AS building.
#[derive(Debug)]
pub struct SceneData {
    /// Meshes.
    pub meshes: Vec<MeshInstance>,
}

/// Mesh instance.
#[derive(Debug)]
pub struct MeshInstance {
    /// Vertex buffer.
    pub vertex_buffer: BufferHandle,
    /// Index buffer.
    pub index_buffer: BufferHandle,
    /// Vertex count.
    pub vertex_count: u32,
    /// Index count.
    pub index_count: u32,
    /// Vertex stride.
    pub vertex_stride: u32,
    /// Transform.
    pub transform: [[f32; 4]; 3],
}

/// Hybrid rendering statistics.
#[derive(Debug, Default, Clone)]
pub struct HybridStats {
    /// Rays traced this frame.
    pub rays_traced: u64,
    /// TLAS build time in microseconds.
    pub tlas_build_us: u64,
    /// RT shadow time in microseconds.
    pub rt_shadow_us: u64,
    /// RT reflection time in microseconds.
    pub rt_reflection_us: u64,
    /// RT GI time in microseconds.
    pub rt_gi_us: u64,
    /// Raster time in microseconds.
    pub raster_us: u64,
}

/// Ray tracing pass implementation.
pub struct RayTracingPass {
    /// Pass name.
    name: String,
    /// Pass type.
    pass_type: RayTracingPassType,
    /// Configuration.
    config: RayTracingPassConfig,
}

impl RayTracingPass {
    /// Create a new ray tracing pass.
    pub fn new(name: impl Into<String>, pass_type: RayTracingPassType) -> Self {
        Self {
            name: name.into(),
            pass_type,
            config: RayTracingPassConfig::default(),
        }
    }

    /// Configure the pass.
    pub fn with_config(mut self, config: RayTracingPassConfig) -> Self {
        self.config = config;
        self
    }

    /// Execute the pass.
    pub fn execute(&self, ctx: &mut PassContext, accel: &AccelerationStructure) {
        let (width, height) = self.get_dispatch_size(ctx);
        ctx.trace_rays(width, height, 1);
    }

    fn get_dispatch_size(&self, _ctx: &PassContext) -> (u32, u32) {
        // Would get from render target
        (1920, 1080)
    }
}

/// Ray tracing pass type.
#[derive(Debug, Clone, Copy)]
pub enum RayTracingPassType {
    /// Primary visibility.
    PrimaryVisibility,
    /// Shadows.
    Shadows,
    /// Reflections.
    Reflections,
    /// Ambient occlusion.
    AmbientOcclusion,
    /// Global illumination.
    GlobalIllumination,
    /// Custom.
    Custom,
}

/// Ray tracing pass configuration.
#[derive(Debug, Clone)]
pub struct RayTracingPassConfig {
    /// Samples per pixel.
    pub samples_per_pixel: u32,
    /// Max bounces.
    pub max_bounces: u32,
    /// Use inline ray tracing.
    pub use_inline: bool,
    /// Enable temporal accumulation.
    pub temporal: bool,
}

impl Default for RayTracingPassConfig {
    fn default() -> Self {
        Self {
            samples_per_pixel: 1,
            max_bounces: 1,
            use_inline: false,
            temporal: true,
        }
    }
}

/// Raster pass for hybrid rendering.
pub struct RasterPass {
    /// Pass name.
    name: String,
    /// Pass type.
    pass_type: RasterPassType,
}

impl RasterPass {
    /// Create a new raster pass.
    pub fn new(name: impl Into<String>, pass_type: RasterPassType) -> Self {
        Self {
            name: name.into(),
            pass_type,
        }
    }
}

/// Raster pass type.
#[derive(Debug, Clone, Copy)]
pub enum RasterPassType {
    /// GBuffer generation.
    GBuffer,
    /// Forward rendering.
    Forward,
    /// Shadow mapping.
    ShadowMap,
    /// Screen-space effects.
    ScreenSpace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_renderer() {
        let renderer = HybridRenderer::new(HybridConfig::default());
        assert_eq!(renderer.rt_support, RayTracingSupport::None);
    }

    #[test]
    fn test_device_capabilities() {
        let mut caps = DeviceCapabilities::default();
        caps.ray_tracing_pipeline = true;
        caps.acceleration_structure = true;
        caps.ray_query = true;

        let mut renderer = HybridRenderer::new(HybridConfig::default());
        renderer.initialize(&caps);
        assert_eq!(renderer.rt_support, RayTracingSupport::Full);
    }
}
