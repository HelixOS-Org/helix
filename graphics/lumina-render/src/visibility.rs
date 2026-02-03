//! Visibility Buffer Rendering
//!
//! Revolutionary visibility buffer system featuring:
//! - Deferred shading with visibility buffer
//! - Software rasterization for small triangles
//! - Triangle ID + instance ID encoding
//! - Material fetch in deferred pass
//! - Compatible with virtual geometry

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::graph::{RenderGraph, VirtualTextureHandle};
use crate::resource::{BufferDesc, TextureDesc, TextureFormat};
use crate::view::View;

/// Visibility buffer renderer.
pub struct VisibilityRenderer {
    /// Configuration.
    config: VisibilityConfig,
    /// Software rasterizer.
    software_raster: Option<SoftwareRasterizer>,
    /// Material classifier.
    material_classifier: MaterialClassifier,
    /// Statistics.
    stats: VisibilityStats,
}

impl VisibilityRenderer {
    /// Create a new visibility renderer.
    pub fn new(config: VisibilityConfig) -> Self {
        Self {
            software_raster: if config.software_raster_small_triangles {
                Some(SoftwareRasterizer::new())
            } else {
                None
            },
            material_classifier: MaterialClassifier::new(),
            config,
            stats: VisibilityStats::default(),
        }
    }

    /// Add visibility buffer passes to render graph.
    pub fn add_passes(
        &self,
        graph: &mut RenderGraph,
        inputs: &VisibilityInputs,
    ) -> VisibilityOutputs {
        // Visibility buffer (triangle ID + instance ID)
        let visibility_buffer = graph.create_texture(TextureDesc {
            format: TextureFormat::R32Uint,
            width: inputs.width,
            height: inputs.height,
            ..Default::default()
        });

        // Depth buffer
        let depth_buffer = graph.create_texture(TextureDesc::depth(
            true, // reverse-Z
        ));

        // Velocity buffer for motion vectors
        let velocity_buffer = graph.create_texture(TextureDesc {
            format: TextureFormat::Rg16Float,
            width: inputs.width,
            height: inputs.height,
            ..Default::default()
        });

        // Hardware rasterization for visible meshlets
        graph.add_graphics_pass("visibility_hw_raster", |builder| {
            builder
                .read_buffer(inputs.visible_meshlets)
                .read_buffer(inputs.meshlet_data)
                .read_buffer(inputs.vertex_buffer)
                .color_attachment(visibility_buffer)
                .color_attachment(velocity_buffer)
                .depth_attachment(depth_buffer);
        });

        // Software rasterization for small triangles
        if self.config.software_raster_small_triangles {
            graph.add_compute_pass("visibility_sw_raster", |builder| {
                builder
                    .read_buffer(inputs.small_triangles)
                    .read_buffer(inputs.vertex_buffer)
                    .storage_image(visibility_buffer)
                    .storage_image(depth_buffer);
            });
        }

        // Material classification
        let material_tiles = if self.config.material_classification {
            let tiles = graph.create_buffer(BufferDesc::storage(
                (inputs.width / 8 * inputs.height / 8 * 16) as u64,
            ));

            graph.add_compute_pass("material_classify", |builder| {
                builder
                    .read_texture(visibility_buffer)
                    .read_buffer(inputs.meshlet_data)
                    .storage_buffer(tiles);
            });

            Some(tiles)
        } else {
            None
        };

        // Deferred material evaluation
        let gbuffer_albedo = graph.create_texture(TextureDesc {
            format: TextureFormat::Rgba8Srgb,
            width: inputs.width,
            height: inputs.height,
            ..Default::default()
        });

        let gbuffer_normal = graph.create_texture(TextureDesc {
            format: TextureFormat::Rgba16Float,
            width: inputs.width,
            height: inputs.height,
            ..Default::default()
        });

        let gbuffer_material = graph.create_texture(TextureDesc {
            format: TextureFormat::Rgba8Unorm,
            width: inputs.width,
            height: inputs.height,
            ..Default::default()
        });

        graph.add_compute_pass("visibility_material_eval", |builder| {
            builder
                .read_texture(visibility_buffer)
                .read_texture(depth_buffer)
                .read_buffer(inputs.meshlet_data)
                .read_buffer(inputs.vertex_buffer)
                .read_buffer(inputs.index_buffer)
                .read_buffer(inputs.material_buffer)
                .storage_image(gbuffer_albedo)
                .storage_image(gbuffer_normal)
                .storage_image(gbuffer_material);
        });

        VisibilityOutputs {
            visibility_buffer,
            depth_buffer,
            velocity_buffer,
            gbuffer_albedo,
            gbuffer_normal,
            gbuffer_material,
            material_tiles,
        }
    }

    /// Get statistics.
    pub fn stats(&self) -> &VisibilityStats {
        &self.stats
    }
}

/// Visibility buffer configuration.
#[derive(Debug, Clone)]
pub struct VisibilityConfig {
    /// Use software rasterization for small triangles.
    pub software_raster_small_triangles: bool,
    /// Small triangle threshold (pixels).
    pub small_triangle_threshold: f32,
    /// Enable material classification.
    pub material_classification: bool,
    /// Tile size for classification.
    pub tile_size: u32,
    /// Maximum materials per tile.
    pub max_materials_per_tile: u32,
    /// Enable velocity output.
    pub velocity_output: bool,
}

impl Default for VisibilityConfig {
    fn default() -> Self {
        Self {
            software_raster_small_triangles: true,
            small_triangle_threshold: 2.0,
            material_classification: true,
            tile_size: 8,
            max_materials_per_tile: 16,
            velocity_output: true,
        }
    }
}

/// Visibility buffer inputs.
#[derive(Debug, Clone)]
pub struct VisibilityInputs {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Visible meshlets buffer.
    pub visible_meshlets: VirtualTextureHandle,
    /// Meshlet data buffer.
    pub meshlet_data: VirtualTextureHandle,
    /// Vertex buffer.
    pub vertex_buffer: VirtualTextureHandle,
    /// Index buffer.
    pub index_buffer: VirtualTextureHandle,
    /// Material buffer.
    pub material_buffer: VirtualTextureHandle,
    /// Small triangles buffer.
    pub small_triangles: VirtualTextureHandle,
}

/// Visibility buffer outputs.
#[derive(Debug, Clone)]
pub struct VisibilityOutputs {
    /// Visibility buffer (triangle ID + instance ID).
    pub visibility_buffer: VirtualTextureHandle,
    /// Depth buffer.
    pub depth_buffer: VirtualTextureHandle,
    /// Velocity/motion vector buffer.
    pub velocity_buffer: VirtualTextureHandle,
    /// GBuffer albedo.
    pub gbuffer_albedo: VirtualTextureHandle,
    /// GBuffer normal.
    pub gbuffer_normal: VirtualTextureHandle,
    /// GBuffer material (roughness, metallic, etc).
    pub gbuffer_material: VirtualTextureHandle,
    /// Material tiles for classification.
    pub material_tiles: Option<VirtualTextureHandle>,
}

/// Visibility buffer ID encoding.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityId(u32);

impl VisibilityId {
    /// Invalid ID.
    pub const INVALID: Self = Self(0xFFFFFFFF);

    /// Create from components.
    pub fn new(instance_id: u32, triangle_id: u32) -> Self {
        // 12 bits for instance, 20 bits for triangle
        debug_assert!(instance_id < 4096);
        debug_assert!(triangle_id < 1048576);
        Self((instance_id << 20) | triangle_id)
    }

    /// Get instance ID.
    pub fn instance_id(&self) -> u32 {
        self.0 >> 20
    }

    /// Get triangle ID.
    pub fn triangle_id(&self) -> u32 {
        self.0 & 0xFFFFF
    }

    /// Is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != 0xFFFFFFFF
    }

    /// Get raw value.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// Extended visibility ID for more objects.
#[derive(Debug, Clone, Copy)]
pub struct VisibilityId64(u64);

impl VisibilityId64 {
    /// Invalid ID.
    pub const INVALID: Self = Self(0xFFFFFFFFFFFFFFFF);

    /// Create from components.
    pub fn new(instance_id: u32, meshlet_id: u32, triangle_id: u16) -> Self {
        Self(((instance_id as u64) << 40) | ((meshlet_id as u64) << 16) | (triangle_id as u64))
    }

    /// Get instance ID.
    pub fn instance_id(&self) -> u32 {
        (self.0 >> 40) as u32
    }

    /// Get meshlet ID.
    pub fn meshlet_id(&self) -> u32 {
        ((self.0 >> 16) & 0xFFFFFF) as u32
    }

    /// Get triangle ID within meshlet.
    pub fn triangle_id(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != 0xFFFFFFFFFFFFFFFF
    }
}

/// Software rasterizer for small triangles.
pub struct SoftwareRasterizer {
    /// Configuration.
    config: SoftwareRasterConfig,
}

impl SoftwareRasterizer {
    /// Create new software rasterizer.
    pub fn new() -> Self {
        Self {
            config: SoftwareRasterConfig::default(),
        }
    }

    /// Rasterize small triangle.
    pub fn rasterize_triangle(
        &self,
        v0: [f32; 4],
        v1: [f32; 4],
        v2: [f32; 4],
        visibility_id: u32,
    ) -> Vec<RasterFragment> {
        let mut fragments = Vec::new();

        // Convert to screen space
        let p0 = self.to_screen(v0);
        let p1 = self.to_screen(v1);
        let p2 = self.to_screen(v2);

        // Calculate bounding box
        let min_x = p0[0].min(p1[0]).min(p2[0]).floor() as i32;
        let max_x = p0[0].max(p1[0]).max(p2[0]).ceil() as i32;
        let min_y = p0[1].min(p1[1]).min(p2[1]).floor() as i32;
        let max_y = p0[1].max(p1[1]).max(p2[1]).ceil() as i32;

        // Edge equations
        let e01 = edge_function(p0, p1);
        let e12 = edge_function(p1, p2);
        let e20 = edge_function(p2, p0);

        let area = e01[0] * (p2[0] - p0[0]) + e01[1] * (p2[1] - p0[1]);
        if area <= 0.0 {
            return fragments; // Backface
        }

        let inv_area = 1.0 / area;

        // Rasterize
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                let w0 = e12[0] * (px - p1[0]) + e12[1] * (py - p1[1]);
                let w1 = e20[0] * (px - p2[0]) + e20[1] * (py - p2[1]);
                let w2 = e01[0] * (px - p0[0]) + e01[1] * (py - p0[1]);

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    // Barycentric coordinates
                    let b0 = w0 * inv_area;
                    let b1 = w1 * inv_area;
                    let b2 = w2 * inv_area;

                    // Interpolate depth
                    let depth = b0 * v0[2] + b1 * v1[2] + b2 * v2[2];

                    fragments.push(RasterFragment {
                        x: x as u32,
                        y: y as u32,
                        depth,
                        visibility_id,
                        barycentrics: [b0, b1, b2],
                    });
                }
            }
        }

        fragments
    }

    fn to_screen(&self, v: [f32; 4]) -> [f32; 2] {
        let inv_w = 1.0 / v[3];
        [
            (v[0] * inv_w * 0.5 + 0.5) * self.config.width as f32,
            (v[1] * inv_w * 0.5 + 0.5) * self.config.height as f32,
        ]
    }
}

fn edge_function(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [b[1] - a[1], a[0] - b[0]]
}

/// Software rasterizer configuration.
#[derive(Debug, Clone)]
pub struct SoftwareRasterConfig {
    /// Render width.
    pub width: u32,
    /// Render height.
    pub height: u32,
    /// Subpixel precision bits.
    pub subpixel_bits: u32,
}

impl Default for SoftwareRasterConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            subpixel_bits: 8,
        }
    }
}

/// Rasterized fragment.
#[derive(Debug, Clone)]
pub struct RasterFragment {
    /// X coordinate.
    pub x: u32,
    /// Y coordinate.
    pub y: u32,
    /// Depth value.
    pub depth: f32,
    /// Visibility ID.
    pub visibility_id: u32,
    /// Barycentric coordinates.
    pub barycentrics: [f32; 3],
}

/// Material classifier for visibility buffer.
pub struct MaterialClassifier {
    /// Tile size.
    tile_size: u32,
    /// Maximum materials per tile.
    max_per_tile: u32,
}

impl MaterialClassifier {
    /// Create new classifier.
    pub fn new() -> Self {
        Self {
            tile_size: 8,
            max_per_tile: 16,
        }
    }

    /// Classify materials in tile.
    pub fn classify_tile(
        &self,
        visibility_buffer: &[u32],
        width: u32,
        tile_x: u32,
        tile_y: u32,
        material_lookup: &dyn Fn(u32) -> u32,
    ) -> MaterialTile {
        let mut materials = Vec::new();
        let mut material_count = 0;

        for y in 0..self.tile_size {
            for x in 0..self.tile_size {
                let px = tile_x * self.tile_size + x;
                let py = tile_y * self.tile_size + y;
                let idx = (py * width + px) as usize;

                if idx < visibility_buffer.len() {
                    let vis_id = visibility_buffer[idx];
                    if vis_id != 0xFFFFFFFF {
                        let instance_id = vis_id >> 20;
                        let material_id = material_lookup(instance_id);

                        // Check if already added
                        if !materials.contains(&material_id) && material_count < self.max_per_tile {
                            materials.push(material_id);
                            material_count += 1;
                        }
                    }
                }
            }
        }

        MaterialTile {
            tile_x,
            tile_y,
            materials,
        }
    }
}

/// Material tile for classification.
#[derive(Debug, Clone)]
pub struct MaterialTile {
    /// Tile X.
    pub tile_x: u32,
    /// Tile Y.
    pub tile_y: u32,
    /// Materials in tile.
    pub materials: Vec<u32>,
}

/// Visibility buffer statistics.
#[derive(Debug, Clone, Default)]
pub struct VisibilityStats {
    /// Triangles rasterized (hardware).
    pub hw_triangles: u32,
    /// Triangles rasterized (software).
    pub sw_triangles: u32,
    /// Unique materials.
    pub unique_materials: u32,
    /// Average materials per tile.
    pub avg_materials_per_tile: f32,
    /// Coverage percentage.
    pub coverage: f32,
}

/// Barycentric interpolation helper.
pub struct BarycentricInterpolator {
    /// Inverse W values.
    inv_w: [f32; 3],
    /// Perspective-correct weights.
    weights: [f32; 3],
}

impl BarycentricInterpolator {
    /// Create from clip-space vertices.
    pub fn new(v0_w: f32, v1_w: f32, v2_w: f32, barycentrics: [f32; 3]) -> Self {
        let inv_w = [1.0 / v0_w, 1.0 / v1_w, 1.0 / v2_w];

        // Perspective-correct interpolation
        let persp_sum =
            barycentrics[0] * inv_w[0] + barycentrics[1] * inv_w[1] + barycentrics[2] * inv_w[2];
        let inv_persp_sum = 1.0 / persp_sum;

        let weights = [
            barycentrics[0] * inv_w[0] * inv_persp_sum,
            barycentrics[1] * inv_w[1] * inv_persp_sum,
            barycentrics[2] * inv_w[2] * inv_persp_sum,
        ];

        Self { inv_w, weights }
    }

    /// Interpolate a float value.
    pub fn interpolate_f32(&self, v0: f32, v1: f32, v2: f32) -> f32 {
        v0 * self.weights[0] + v1 * self.weights[1] + v2 * self.weights[2]
    }

    /// Interpolate a vec2.
    pub fn interpolate_vec2(&self, v0: [f32; 2], v1: [f32; 2], v2: [f32; 2]) -> [f32; 2] {
        [
            self.interpolate_f32(v0[0], v1[0], v2[0]),
            self.interpolate_f32(v0[1], v1[1], v2[1]),
        ]
    }

    /// Interpolate a vec3.
    pub fn interpolate_vec3(&self, v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
        [
            self.interpolate_f32(v0[0], v1[0], v2[0]),
            self.interpolate_f32(v0[1], v1[1], v2[1]),
            self.interpolate_f32(v0[2], v1[2], v2[2]),
        ]
    }

    /// Interpolate a vec4.
    pub fn interpolate_vec4(&self, v0: [f32; 4], v1: [f32; 4], v2: [f32; 4]) -> [f32; 4] {
        [
            self.interpolate_f32(v0[0], v1[0], v2[0]),
            self.interpolate_f32(v0[1], v1[1], v2[1]),
            self.interpolate_f32(v0[2], v1[2], v2[2]),
            self.interpolate_f32(v0[3], v1[3], v2[3]),
        ]
    }

    /// Get perspective-correct W.
    pub fn interpolate_w(&self) -> f32 {
        1.0 / (self.weights[0] * self.inv_w[0]
            + self.weights[1] * self.inv_w[1]
            + self.weights[2] * self.inv_w[2])
    }
}

/// Derivative computation for visibility buffer.
pub struct DerivativeComputer {
    /// Screen-space derivatives.
    ddx: [f32; 2],
    ddy: [f32; 2],
}

impl DerivativeComputer {
    /// Compute derivatives from neighboring pixels.
    pub fn new(center_uv: [f32; 2], right_uv: [f32; 2], up_uv: [f32; 2]) -> Self {
        Self {
            ddx: [right_uv[0] - center_uv[0], right_uv[1] - center_uv[1]],
            ddy: [up_uv[0] - center_uv[0], up_uv[1] - center_uv[1]],
        }
    }

    /// Get ddx.
    pub fn ddx(&self) -> [f32; 2] {
        self.ddx
    }

    /// Get ddy.
    pub fn ddy(&self) -> [f32; 2] {
        self.ddy
    }

    /// Calculate mip level.
    pub fn calculate_mip(&self, texture_size: [f32; 2]) -> f32 {
        let ddx_scaled = [self.ddx[0] * texture_size[0], self.ddx[1] * texture_size[1]];
        let ddy_scaled = [self.ddy[0] * texture_size[0], self.ddy[1] * texture_size[1]];

        let delta_max_sqr = (ddx_scaled[0] * ddx_scaled[0] + ddx_scaled[1] * ddx_scaled[1])
            .max(ddy_scaled[0] * ddy_scaled[0] + ddy_scaled[1] * ddy_scaled[1]);

        0.5 * delta_max_sqr.log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_id() {
        let id = VisibilityId::new(123, 456789);
        assert_eq!(id.instance_id(), 123);
        assert_eq!(id.triangle_id(), 456789);
        assert!(id.is_valid());
    }

    #[test]
    fn test_visibility_id64() {
        let id = VisibilityId64::new(1000, 50000, 255);
        assert_eq!(id.instance_id(), 1000);
        assert_eq!(id.meshlet_id(), 50000);
        assert_eq!(id.triangle_id(), 255);
    }

    #[test]
    fn test_barycentric_interpolation() {
        let interp = BarycentricInterpolator::new(1.0, 1.0, 1.0, [0.33, 0.33, 0.34]);

        let result = interp.interpolate_f32(1.0, 2.0, 3.0);
        assert!((result - 2.01).abs() < 0.01);
    }

    #[test]
    fn test_derivative_mip() {
        let deriv = DerivativeComputer::new([0.5, 0.5], [0.501, 0.5], [0.5, 0.501]);
        let mip = deriv.calculate_mip([1024.0, 1024.0]);
        assert!(mip >= 0.0 && mip < 2.0);
    }
}
