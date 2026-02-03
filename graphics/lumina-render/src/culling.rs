//! GPU Culling System
//!
//! Revolutionary GPU-driven culling featuring:
//! - Hierarchical-Z occlusion culling
//! - GPU frustum culling
//! - Two-phase occlusion culling
//! - Cluster/meshlet culling
//! - Instance culling
//! - Triangle culling

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::graph::{RenderGraph, VirtualTextureHandle};
use crate::resource::{BufferDesc, BufferHandle, TextureDesc, TextureFormat};
use crate::view::View;

/// GPU culling system.
pub struct GpuCulling {
    /// Configuration.
    config: CullingConfig,
    /// Hierarchical-Z buffer.
    hzb: Option<HierarchicalZBuffer>,
    /// Occlusion query system.
    occlusion_queries: OcclusionQuerySystem,
    /// Culling statistics.
    stats: CullingStats,
}

impl GpuCulling {
    /// Create a new GPU culling system.
    pub fn new(config: CullingConfig) -> Self {
        Self {
            config,
            hzb: None,
            occlusion_queries: OcclusionQuerySystem::new(65536),
            stats: CullingStats::default(),
        }
    }

    /// Initialize HZB for resolution.
    pub fn initialize(&mut self, width: u32, height: u32) {
        self.hzb = Some(HierarchicalZBuffer::new(width, height));
    }

    /// Build HZB from depth buffer.
    pub fn build_hzb(
        &mut self,
        graph: &mut RenderGraph,
        depth: VirtualTextureHandle,
    ) -> VirtualTextureHandle {
        let hzb = self.hzb.as_ref().expect("HZB not initialized");

        // Create HZB texture with mips
        let hzb_texture = graph.create_texture(TextureDesc {
            format: TextureFormat::R32Float,
            width: hzb.width,
            height: hzb.height,
            mip_levels: hzb.mip_count,
            ..Default::default()
        });

        // Copy depth to HZB mip 0
        graph.add_compute_pass("hzb_copy", |builder| {
            builder.read_texture(depth).storage_image(hzb_texture);
        });

        // Generate mip chain
        for mip in 1..hzb.mip_count {
            graph.add_compute_pass(&alloc::format!("hzb_mip_{}", mip), |builder| {
                builder.storage_image(hzb_texture);
            });
        }

        hzb_texture
    }

    /// Perform frustum culling on GPU.
    pub fn frustum_cull(
        &self,
        graph: &mut RenderGraph,
        instances: VirtualTextureHandle,
        instance_count: u32,
        view: &View,
    ) -> CullingOutput {
        // Output indirect draw buffer
        let indirect_buffer = graph.create_buffer(BufferDesc::indirect(instance_count as u64 * 20));

        // Output visibility buffer
        let visibility_buffer = graph.create_buffer(BufferDesc::storage(instance_count as u64 * 4));

        // Visible count buffer
        let count_buffer = graph.create_buffer(BufferDesc::storage(4));

        graph.add_compute_pass("frustum_cull", |builder| {
            builder
                .read_buffer(instances)
                .storage_buffer(indirect_buffer)
                .storage_buffer(visibility_buffer)
                .storage_buffer(count_buffer);
        });

        CullingOutput {
            indirect_buffer,
            visibility_buffer,
            count_buffer,
            visible_count: 0, // Will be read back
        }
    }

    /// Perform occlusion culling on GPU.
    pub fn occlusion_cull(
        &self,
        graph: &mut RenderGraph,
        instances: VirtualTextureHandle,
        hzb: VirtualTextureHandle,
        instance_count: u32,
        view: &View,
    ) -> CullingOutput {
        // Phase 1: Cull using HZB
        let visible_phase1 = graph.create_buffer(BufferDesc::storage(instance_count as u64 * 4));

        graph.add_compute_pass("occlusion_cull_phase1", |builder| {
            builder
                .read_buffer(instances)
                .read_texture(hzb)
                .storage_buffer(visible_phase1);
        });

        // Output buffers
        let indirect_buffer = graph.create_buffer(BufferDesc::indirect(instance_count as u64 * 20));
        let visibility_buffer = graph.create_buffer(BufferDesc::storage(instance_count as u64 * 4));
        let count_buffer = graph.create_buffer(BufferDesc::storage(4));

        // Phase 2: Fine-grained occlusion test
        graph.add_compute_pass("occlusion_cull_phase2", |builder| {
            builder
                .read_buffer(instances)
                .read_buffer(visible_phase1)
                .read_texture(hzb)
                .storage_buffer(indirect_buffer)
                .storage_buffer(visibility_buffer)
                .storage_buffer(count_buffer);
        });

        CullingOutput {
            indirect_buffer,
            visibility_buffer,
            count_buffer,
            visible_count: 0,
        }
    }

    /// Two-phase occlusion culling (for static + dynamic objects).
    pub fn two_phase_cull(
        &self,
        graph: &mut RenderGraph,
        static_instances: VirtualTextureHandle,
        dynamic_instances: VirtualTextureHandle,
        prev_hzb: VirtualTextureHandle,
        static_count: u32,
        dynamic_count: u32,
        view: &View,
    ) -> TwoPhaseCullingOutput {
        // Phase 1: Cull against previous frame HZB
        let phase1_visible = graph.create_buffer(BufferDesc::storage(
            (static_count + dynamic_count) as u64 * 4,
        ));

        graph.add_compute_pass("two_phase_cull_1", |builder| {
            builder
                .read_buffer(static_instances)
                .read_buffer(dynamic_instances)
                .read_texture(prev_hzb)
                .storage_buffer(phase1_visible);
        });

        // Render occluders from phase 1
        let occluder_depth = graph.create_texture(TextureDesc::depth(view.config.reverse_z));

        graph.add_graphics_pass("render_occluders", |builder| {
            builder
                .read_buffer(phase1_visible)
                .depth_attachment(occluder_depth);
        });

        // Build new HZB from occluder depth
        let new_hzb = self.build_hzb(graph, occluder_depth);

        // Phase 2: Cull remaining against new HZB
        let phase2_static =
            self.occlusion_cull(graph, static_instances, new_hzb, static_count, view);
        let phase2_dynamic =
            self.occlusion_cull(graph, dynamic_instances, new_hzb, dynamic_count, view);

        TwoPhaseCullingOutput {
            static_output: phase2_static,
            dynamic_output: phase2_dynamic,
            hzb: new_hzb,
        }
    }

    /// Meshlet/cluster culling.
    pub fn meshlet_cull(
        &self,
        graph: &mut RenderGraph,
        meshlets: VirtualTextureHandle,
        meshlet_count: u32,
        hzb: VirtualTextureHandle,
        view: &View,
    ) -> MeshletCullingOutput {
        // Visible meshlet indices
        let visible_meshlets = graph.create_buffer(BufferDesc::storage(meshlet_count as u64 * 4));

        // Meshlet draw commands
        let meshlet_commands = graph.create_buffer(BufferDesc::indirect(meshlet_count as u64 * 12));

        // Visible count
        let count_buffer = graph.create_buffer(BufferDesc::storage(4));

        graph.add_compute_pass("meshlet_cull", |builder| {
            builder
                .read_buffer(meshlets)
                .read_texture(hzb)
                .storage_buffer(visible_meshlets)
                .storage_buffer(meshlet_commands)
                .storage_buffer(count_buffer);
        });

        MeshletCullingOutput {
            visible_meshlets,
            meshlet_commands,
            count_buffer,
        }
    }

    /// Triangle culling (for software rasterization).
    pub fn triangle_cull(
        &self,
        graph: &mut RenderGraph,
        vertices: VirtualTextureHandle,
        indices: VirtualTextureHandle,
        triangle_count: u32,
        view: &View,
    ) -> TriangleCullingOutput {
        // Visible triangle indices
        let visible_triangles = graph.create_buffer(BufferDesc::storage(triangle_count as u64 * 4));

        // Visible count
        let count_buffer = graph.create_buffer(BufferDesc::storage(4));

        graph.add_compute_pass("triangle_cull", |builder| {
            builder
                .read_buffer(vertices)
                .read_buffer(indices)
                .storage_buffer(visible_triangles)
                .storage_buffer(count_buffer);
        });

        TriangleCullingOutput {
            visible_triangles,
            count_buffer,
        }
    }

    /// Get culling statistics.
    pub fn stats(&self) -> &CullingStats {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = CullingStats::default();
    }
}

/// Culling configuration.
#[derive(Debug, Clone)]
pub struct CullingConfig {
    /// Enable frustum culling.
    pub frustum_culling: bool,
    /// Enable occlusion culling.
    pub occlusion_culling: bool,
    /// Enable backface culling.
    pub backface_culling: bool,
    /// Enable small object culling.
    pub small_object_culling: bool,
    /// Minimum screen size for objects (pixels).
    pub min_screen_size: f32,
    /// Enable distance culling.
    pub distance_culling: bool,
    /// Maximum draw distance.
    pub max_distance: f32,
    /// HZB mip bias.
    pub hzb_mip_bias: f32,
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            frustum_culling: true,
            occlusion_culling: true,
            backface_culling: true,
            small_object_culling: true,
            min_screen_size: 4.0,
            distance_culling: true,
            max_distance: 10000.0,
            hzb_mip_bias: 0.0,
        }
    }
}

/// Culling output.
#[derive(Debug, Clone)]
pub struct CullingOutput {
    /// Indirect draw buffer.
    pub indirect_buffer: VirtualTextureHandle,
    /// Visibility buffer (per instance).
    pub visibility_buffer: VirtualTextureHandle,
    /// Visible count buffer.
    pub count_buffer: VirtualTextureHandle,
    /// Visible count (if read back).
    pub visible_count: u32,
}

/// Two-phase culling output.
#[derive(Debug, Clone)]
pub struct TwoPhaseCullingOutput {
    /// Static objects output.
    pub static_output: CullingOutput,
    /// Dynamic objects output.
    pub dynamic_output: CullingOutput,
    /// New HZB.
    pub hzb: VirtualTextureHandle,
}

/// Meshlet culling output.
#[derive(Debug, Clone)]
pub struct MeshletCullingOutput {
    /// Visible meshlet indices.
    pub visible_meshlets: VirtualTextureHandle,
    /// Meshlet draw commands.
    pub meshlet_commands: VirtualTextureHandle,
    /// Count buffer.
    pub count_buffer: VirtualTextureHandle,
}

/// Triangle culling output.
#[derive(Debug, Clone)]
pub struct TriangleCullingOutput {
    /// Visible triangle indices.
    pub visible_triangles: VirtualTextureHandle,
    /// Count buffer.
    pub count_buffer: VirtualTextureHandle,
}

/// Hierarchical-Z buffer for occlusion culling.
pub struct HierarchicalZBuffer {
    /// Width at mip 0.
    width: u32,
    /// Height at mip 0.
    height: u32,
    /// Number of mip levels.
    mip_count: u32,
    /// Reduction method.
    reduction: HzbReduction,
}

impl HierarchicalZBuffer {
    /// Create new HZB.
    pub fn new(width: u32, height: u32) -> Self {
        let mip_count = (width.max(height) as f32).log2().ceil() as u32;

        Self {
            width: width.next_power_of_two(),
            height: height.next_power_of_two(),
            mip_count,
            reduction: HzbReduction::Max,
        }
    }

    /// Get mip dimensions.
    pub fn mip_size(&self, mip: u32) -> (u32, u32) {
        let w = (self.width >> mip).max(1);
        let h = (self.height >> mip).max(1);
        (w, h)
    }

    /// Sample HZB for bounding box test.
    pub fn test_aabb(&self, min_ndc: [f32; 3], max_ndc: [f32; 3], hzb_data: &[f32]) -> bool {
        // Calculate screen-space bounds
        let screen_min = [
            (min_ndc[0] * 0.5 + 0.5) * self.width as f32,
            (min_ndc[1] * 0.5 + 0.5) * self.height as f32,
        ];
        let screen_max = [
            (max_ndc[0] * 0.5 + 0.5) * self.width as f32,
            (max_ndc[1] * 0.5 + 0.5) * self.height as f32,
        ];

        // Calculate appropriate mip level
        let size = (screen_max[0] - screen_min[0]).max(screen_max[1] - screen_min[1]);
        let mip = (size.log2().ceil() as u32).min(self.mip_count - 1);

        // Sample HZB at mip level
        // Would sample and compare depth
        true
    }
}

/// HZB reduction method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HzbReduction {
    /// Maximum depth (reverse-Z).
    Max,
    /// Minimum depth (forward-Z).
    Min,
}

/// Occlusion query system.
pub struct OcclusionQuerySystem {
    /// Query pool capacity.
    capacity: u32,
    /// Pending queries.
    pending: Vec<OcclusionQuery>,
    /// Completed queries.
    completed: Vec<OcclusionQuery>,
    /// Next query ID.
    next_id: u32,
}

impl OcclusionQuerySystem {
    /// Create new query system.
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            pending: Vec::new(),
            completed: Vec::new(),
            next_id: 0,
        }
    }

    /// Begin an occlusion query.
    pub fn begin_query(&mut self, object_id: u64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        self.pending.push(OcclusionQuery {
            id,
            object_id,
            visible_samples: 0,
            complete: false,
        });

        id
    }

    /// End an occlusion query.
    pub fn end_query(&mut self, id: u32) {
        // Would end GPU query
    }

    /// Read query results.
    pub fn read_results(&mut self) {
        // Would read back query results from GPU
        self.completed.clear();
        for query in self.pending.drain(..) {
            self.completed.push(OcclusionQuery {
                complete: true,
                ..query
            });
        }
    }

    /// Check if object is visible.
    pub fn is_visible(&self, object_id: u64) -> bool {
        for query in &self.completed {
            if query.object_id == object_id {
                return query.visible_samples > 0;
            }
        }
        true // Assume visible if no query
    }
}

/// Occlusion query.
#[derive(Debug, Clone)]
pub struct OcclusionQuery {
    /// Query ID.
    pub id: u32,
    /// Object being queried.
    pub object_id: u64,
    /// Number of visible samples.
    pub visible_samples: u32,
    /// Query is complete.
    pub complete: bool,
}

/// Culling statistics.
#[derive(Debug, Clone, Default)]
pub struct CullingStats {
    /// Total objects.
    pub total_objects: u32,
    /// Frustum culled.
    pub frustum_culled: u32,
    /// Occlusion culled.
    pub occlusion_culled: u32,
    /// Small object culled.
    pub small_object_culled: u32,
    /// Distance culled.
    pub distance_culled: u32,
    /// Backface culled.
    pub backface_culled: u32,
    /// Final visible.
    pub visible: u32,
    /// Cull time in microseconds.
    pub cull_time_us: f32,
}

impl CullingStats {
    /// Get total culled.
    pub fn total_culled(&self) -> u32 {
        self.frustum_culled
            + self.occlusion_culled
            + self.small_object_culled
            + self.distance_culled
            + self.backface_culled
    }

    /// Get visibility ratio.
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_objects > 0 {
            self.visible as f32 / self.total_objects as f32
        } else {
            1.0
        }
    }
}

/// Instance data for culling.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CullInstance {
    /// Bounding sphere (xyz = center, w = radius).
    pub bounding_sphere: [f32; 4],
    /// AABB min.
    pub aabb_min: [f32; 3],
    /// Object flags.
    pub flags: u32,
    /// AABB max.
    pub aabb_max: [f32; 3],
    /// LOD distance.
    pub lod_distance: f32,
    /// World transform row 0.
    pub transform_0: [f32; 4],
    /// World transform row 1.
    pub transform_1: [f32; 4],
    /// World transform row 2.
    pub transform_2: [f32; 4],
}

impl CullInstance {
    /// Create from transform and bounds.
    pub fn new(transform: [[f32; 4]; 3], aabb_min: [f32; 3], aabb_max: [f32; 3]) -> Self {
        // Calculate bounding sphere from AABB
        let center = [
            (aabb_min[0] + aabb_max[0]) * 0.5,
            (aabb_min[1] + aabb_max[1]) * 0.5,
            (aabb_min[2] + aabb_max[2]) * 0.5,
        ];
        let extent = [
            aabb_max[0] - aabb_min[0],
            aabb_max[1] - aabb_min[1],
            aabb_max[2] - aabb_min[2],
        ];
        let radius =
            (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5;

        Self {
            bounding_sphere: [center[0], center[1], center[2], radius],
            aabb_min,
            flags: 0,
            aabb_max,
            lod_distance: 100.0,
            transform_0: transform[0],
            transform_1: transform[1],
            transform_2: transform[2],
        }
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: CullFlags) -> Self {
        self.flags = flags.bits();
        self
    }
}

bitflags::bitflags! {
    /// Culling flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CullFlags: u32 {
        /// Object casts shadows.
        const CASTS_SHADOW = 1 << 0;
        /// Object is static.
        const STATIC = 1 << 1;
        /// Disable frustum culling.
        const NO_FRUSTUM_CULL = 1 << 2;
        /// Disable occlusion culling.
        const NO_OCCLUSION_CULL = 1 << 3;
        /// Always render (important object).
        const ALWAYS_RENDER = 1 << 4;
        /// Is LOD parent.
        const LOD_PARENT = 1 << 5;
        /// Has been culled.
        const CULLED = 1 << 6;
    }
}

/// Indirect draw command.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IndirectDrawCommand {
    /// Vertex count.
    pub vertex_count: u32,
    /// Instance count.
    pub instance_count: u32,
    /// First vertex.
    pub first_vertex: u32,
    /// First instance.
    pub first_instance: u32,
}

/// Indirect indexed draw command.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IndirectDrawIndexedCommand {
    /// Index count.
    pub index_count: u32,
    /// Instance count.
    pub instance_count: u32,
    /// First index.
    pub first_index: u32,
    /// Vertex offset.
    pub vertex_offset: i32,
    /// First instance.
    pub first_instance: u32,
}

/// Indirect dispatch command.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IndirectDispatchCommand {
    /// Group count X.
    pub group_count_x: u32,
    /// Group count Y.
    pub group_count_y: u32,
    /// Group count Z.
    pub group_count_z: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hzb_creation() {
        let hzb = HierarchicalZBuffer::new(1920, 1080);
        assert_eq!(hzb.width, 2048);
        assert_eq!(hzb.height, 2048);
        assert!(hzb.mip_count >= 10);
    }

    #[test]
    fn test_hzb_mip_size() {
        let hzb = HierarchicalZBuffer::new(1024, 1024);
        assert_eq!(hzb.mip_size(0), (1024, 1024));
        assert_eq!(hzb.mip_size(1), (512, 512));
        assert_eq!(hzb.mip_size(10), (1, 1));
    }

    #[test]
    fn test_cull_instance() {
        let transform = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
            0.0, 0.0, 1.0, 0.0,
        ]];
        let instance = CullInstance::new(transform, [-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

        assert!((instance.bounding_sphere[0] - 0.0).abs() < 0.001);
        assert!(instance.bounding_sphere[3] > 0.0);
    }

    #[test]
    fn test_culling_stats() {
        let stats = CullingStats {
            total_objects: 1000,
            frustum_culled: 500,
            occlusion_culled: 200,
            small_object_culled: 50,
            distance_culled: 50,
            backface_culled: 0,
            visible: 200,
            cull_time_us: 100.0,
        };

        assert_eq!(stats.total_culled(), 800);
        assert!((stats.visibility_ratio() - 0.2).abs() < 0.001);
    }
}
