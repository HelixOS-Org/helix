//! Terrain Rendering Types for Lumina
//!
//! This module provides terrain rendering infrastructure including
//! heightmaps, LOD, tessellation, and procedural generation.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Terrain Handles
// ============================================================================

/// Terrain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TerrainHandle(pub u64);

impl TerrainHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for TerrainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Terrain chunk handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TerrainChunkHandle(pub u64);

impl TerrainChunkHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for TerrainChunkHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Heightmap handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HeightmapHandle(pub u64);

impl HeightmapHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HeightmapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Terrain Configuration
// ============================================================================

/// Terrain create info
#[derive(Clone, Debug)]
pub struct TerrainCreateInfo {
    /// Width in world units
    pub width: f32,
    /// Depth in world units
    pub depth: f32,
    /// Max height
    pub max_height: f32,
    /// Chunk size (vertices per side)
    pub chunk_size: u32,
    /// Chunks in X
    pub chunks_x: u32,
    /// Chunks in Z
    pub chunks_z: u32,
    /// LOD levels
    pub lod_levels: u32,
    /// Use tessellation
    pub tessellation: bool,
    /// Debug label
    pub label: Option<&'static str>,
}

impl TerrainCreateInfo {
    /// Creates info
    pub fn new(width: f32, depth: f32) -> Self {
        Self {
            width,
            depth,
            max_height: 100.0,
            chunk_size: 65,
            chunks_x: 16,
            chunks_z: 16,
            lod_levels: 5,
            tessellation: false,
            label: None,
        }
    }

    /// Small terrain (1x1 km)
    pub fn small() -> Self {
        Self::new(1000.0, 1000.0).with_chunks(8, 8)
    }

    /// Medium terrain (4x4 km)
    pub fn medium() -> Self {
        Self::new(4000.0, 4000.0).with_chunks(16, 16)
    }

    /// Large terrain (16x16 km)
    pub fn large() -> Self {
        Self::new(16000.0, 16000.0).with_chunks(64, 64)
    }

    /// With max height
    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }

    /// With chunk size
    pub fn with_chunk_size(mut self, size: u32) -> Self {
        self.chunk_size = size;
        self
    }

    /// With chunks
    pub fn with_chunks(mut self, x: u32, z: u32) -> Self {
        self.chunks_x = x;
        self.chunks_z = z;
        self
    }

    /// With LOD levels
    pub fn with_lod_levels(mut self, levels: u32) -> Self {
        self.lod_levels = levels;
        self
    }

    /// Enable tessellation
    pub fn with_tessellation(mut self) -> Self {
        self.tessellation = true;
        self
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Chunk width
    pub fn chunk_width(&self) -> f32 {
        self.width / self.chunks_x as f32
    }

    /// Chunk depth
    pub fn chunk_depth(&self) -> f32 {
        self.depth / self.chunks_z as f32
    }

    /// Total vertices
    pub fn total_vertices(&self) -> u64 {
        (self.chunk_size as u64)
            * (self.chunk_size as u64)
            * (self.chunks_x as u64)
            * (self.chunks_z as u64)
    }

    /// Total chunks
    pub fn total_chunks(&self) -> u32 {
        self.chunks_x * self.chunks_z
    }
}

impl Default for TerrainCreateInfo {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// Heightmap
// ============================================================================

/// Heightmap create info
#[derive(Clone, Debug)]
pub struct HeightmapCreateInfo {
    /// Width in samples
    pub width: u32,
    /// Height in samples
    pub height: u32,
    /// Format
    pub format: HeightmapFormat,
    /// Initial data
    pub data: Option<Vec<u8>>,
}

impl HeightmapCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: HeightmapFormat::R16Unorm,
            data: None,
        }
    }

    /// With format
    pub fn with_format(mut self, format: HeightmapFormat) -> Self {
        self.format = format;
        self
    }

    /// With data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Data size in bytes
    pub fn data_size(&self) -> usize {
        (self.width * self.height) as usize * self.format.bytes_per_sample()
    }
}

impl Default for HeightmapCreateInfo {
    fn default() -> Self {
        Self::new(1025, 1025)
    }
}

/// Heightmap format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HeightmapFormat {
    /// 8-bit
    R8Unorm  = 0,
    /// 16-bit
    #[default]
    R16Unorm = 1,
    /// 32-bit float
    R32Float = 2,
}

impl HeightmapFormat {
    /// Bytes per sample
    pub const fn bytes_per_sample(&self) -> usize {
        match self {
            Self::R8Unorm => 1,
            Self::R16Unorm => 2,
            Self::R32Float => 4,
        }
    }
}

/// Heightmap data
#[derive(Clone, Debug)]
pub struct HeightmapData {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Height values (normalized 0-1)
    pub heights: Vec<f32>,
}

impl HeightmapData {
    /// Creates empty heightmap
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            heights: vec![0.0; (width * height) as usize],
        }
    }

    /// Creates flat heightmap
    pub fn flat(width: u32, height: u32, value: f32) -> Self {
        Self {
            width,
            height,
            heights: vec![value; (width * height) as usize],
        }
    }

    /// Gets height at position
    pub fn get(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        self.heights[(y * self.width + x) as usize]
    }

    /// Sets height at position
    pub fn set(&mut self, x: u32, y: u32, height: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.heights[(y * self.width + x) as usize] = height;
    }

    /// Gets interpolated height
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let fx = u * (self.width - 1) as f32;
        let fy = v * (self.height - 1) as f32;

        let x0 = (fx as u32).min(self.width - 2);
        let y0 = (fy as u32).min(self.height - 2);
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let tx = fx.fract();
        let ty = fy.fract();

        let h00 = self.get(x0, y0);
        let h10 = self.get(x1, y0);
        let h01 = self.get(x0, y1);
        let h11 = self.get(x1, y1);

        let h0 = h00 + (h10 - h00) * tx;
        let h1 = h01 + (h11 - h01) * tx;

        h0 + (h1 - h0) * ty
    }

    /// Gets normal at position
    pub fn get_normal(&self, x: u32, y: u32, scale: f32) -> [f32; 3] {
        let x0 = x.saturating_sub(1);
        let x1 = (x + 1).min(self.width - 1);
        let y0 = y.saturating_sub(1);
        let y1 = (y + 1).min(self.height - 1);

        let hl = self.get(x0, y) * scale;
        let hr = self.get(x1, y) * scale;
        let hd = self.get(x, y0) * scale;
        let hu = self.get(x, y1) * scale;

        let dx = hr - hl;
        let dy = hu - hd;

        let len = (dx * dx + 4.0 + dy * dy).sqrt();
        [-dx / len, 2.0 / len, -dy / len]
    }
}

impl Default for HeightmapData {
    fn default() -> Self {
        Self::new(1025, 1025)
    }
}

// ============================================================================
// Terrain LOD
// ============================================================================

/// Terrain LOD settings
#[derive(Clone, Debug)]
pub struct TerrainLodSettings {
    /// LOD levels
    pub levels: Vec<TerrainLodLevel>,
    /// Distance multiplier
    pub distance_multiplier: f32,
    /// Morph range (0-1, how much of LOD range is morphing)
    pub morph_range: f32,
}

impl TerrainLodSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            distance_multiplier: 1.0,
            morph_range: 0.3,
        }
    }

    /// Default LOD chain
    pub fn default_chain(max_distance: f32, lod_count: u32) -> Self {
        let mut levels = Vec::with_capacity(lod_count as usize);
        let mut distance = max_distance / (1 << (lod_count - 1)) as f32;

        for i in 0..lod_count {
            levels.push(TerrainLodLevel {
                distance,
                grid_scale: 1 << i,
                tessellation_factor: (lod_count - i) as f32,
            });
            distance *= 2.0;
        }

        Self {
            levels,
            distance_multiplier: 1.0,
            morph_range: 0.3,
        }
    }

    /// Adds LOD level
    pub fn with_level(mut self, level: TerrainLodLevel) -> Self {
        self.levels.push(level);
        self
    }

    /// With distance multiplier
    pub fn with_distance_multiplier(mut self, mult: f32) -> Self {
        self.distance_multiplier = mult;
        self
    }

    /// Get LOD level for distance
    pub fn get_lod_for_distance(&self, distance: f32) -> u32 {
        let scaled = distance / self.distance_multiplier;
        for (i, level) in self.levels.iter().enumerate() {
            if scaled < level.distance {
                return i as u32;
            }
        }
        self.levels.len().saturating_sub(1) as u32
    }
}

impl Default for TerrainLodSettings {
    fn default() -> Self {
        Self::default_chain(2000.0, 5)
    }
}

/// Terrain LOD level
#[derive(Clone, Copy, Debug)]
pub struct TerrainLodLevel {
    /// Max distance for this LOD
    pub distance: f32,
    /// Grid scale (1, 2, 4, 8...)
    pub grid_scale: u32,
    /// Tessellation factor
    pub tessellation_factor: f32,
}

impl TerrainLodLevel {
    /// Creates level
    pub fn new(distance: f32, grid_scale: u32) -> Self {
        Self {
            distance,
            grid_scale,
            tessellation_factor: 1.0,
        }
    }

    /// With tessellation factor
    pub fn with_tessellation(mut self, factor: f32) -> Self {
        self.tessellation_factor = factor;
        self
    }
}

// ============================================================================
// Terrain Materials
// ============================================================================

/// Terrain material layer
#[derive(Clone, Debug)]
pub struct TerrainMaterialLayer {
    /// Albedo texture
    pub albedo: u64,
    /// Normal texture
    pub normal: u64,
    /// Roughness texture
    pub roughness: u64,
    /// UV scale
    pub uv_scale: f32,
    /// Blend settings
    pub blend: TerrainBlendSettings,
}

impl TerrainMaterialLayer {
    /// Creates layer
    pub fn new(albedo: u64) -> Self {
        Self {
            albedo,
            normal: 0,
            roughness: 0,
            uv_scale: 1.0,
            blend: TerrainBlendSettings::default(),
        }
    }

    /// With normal
    pub fn with_normal(mut self, normal: u64) -> Self {
        self.normal = normal;
        self
    }

    /// With roughness
    pub fn with_roughness(mut self, roughness: u64) -> Self {
        self.roughness = roughness;
        self
    }

    /// With UV scale
    pub fn with_uv_scale(mut self, scale: f32) -> Self {
        self.uv_scale = scale;
        self
    }

    /// With blend settings
    pub fn with_blend(mut self, blend: TerrainBlendSettings) -> Self {
        self.blend = blend;
        self
    }
}

impl Default for TerrainMaterialLayer {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Terrain blend settings
#[derive(Clone, Copy, Debug)]
pub struct TerrainBlendSettings {
    /// Blend mode
    pub mode: TerrainBlendMode,
    /// Height influence (for height-based blending)
    pub height_influence: f32,
    /// Slope influence
    pub slope_influence: f32,
    /// Min slope (radians)
    pub min_slope: f32,
    /// Max slope (radians)
    pub max_slope: f32,
    /// Min height (normalized)
    pub min_height: f32,
    /// Max height (normalized)
    pub max_height: f32,
}

impl TerrainBlendSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            mode: TerrainBlendMode::Linear,
            height_influence: 0.0,
            slope_influence: 0.0,
            min_slope: 0.0,
            max_slope: core::f32::consts::FRAC_PI_2,
            min_height: 0.0,
            max_height: 1.0,
        }
    }

    /// Height-based blending
    pub fn height_based(min: f32, max: f32) -> Self {
        Self {
            mode: TerrainBlendMode::HeightBased,
            height_influence: 1.0,
            min_height: min,
            max_height: max,
            ..Self::new()
        }
    }

    /// Slope-based blending
    pub fn slope_based(min_slope: f32, max_slope: f32) -> Self {
        Self {
            mode: TerrainBlendMode::SlopeBased,
            slope_influence: 1.0,
            min_slope,
            max_slope,
            ..Self::new()
        }
    }
}

impl Default for TerrainBlendSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Terrain blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TerrainBlendMode {
    /// Linear blend
    #[default]
    Linear      = 0,
    /// Height-based blend
    HeightBased = 1,
    /// Slope-based blend
    SlopeBased  = 2,
    /// Triplanar
    Triplanar   = 3,
}

// ============================================================================
// Terrain Chunks
// ============================================================================

/// Terrain chunk
#[derive(Clone, Debug)]
pub struct TerrainChunk {
    /// Chunk X index
    pub chunk_x: u32,
    /// Chunk Z index
    pub chunk_z: u32,
    /// World position (min corner)
    pub world_pos: [f32; 3],
    /// Size
    pub size: [f32; 2],
    /// Current LOD level
    pub lod_level: u32,
    /// Neighbor LOD levels (N, E, S, W)
    pub neighbor_lods: [u32; 4],
    /// Bounding box min
    pub bounds_min: [f32; 3],
    /// Bounding box max
    pub bounds_max: [f32; 3],
    /// Is visible
    pub visible: bool,
}

impl TerrainChunk {
    /// Creates chunk
    pub fn new(chunk_x: u32, chunk_z: u32, world_pos: [f32; 3], size: [f32; 2]) -> Self {
        Self {
            chunk_x,
            chunk_z,
            world_pos,
            size,
            lod_level: 0,
            neighbor_lods: [0; 4],
            bounds_min: world_pos,
            bounds_max: [
                world_pos[0] + size[0],
                world_pos[1] + 100.0,
                world_pos[2] + size[1],
            ],
            visible: true,
        }
    }

    /// Center position
    pub fn center(&self) -> [f32; 3] {
        [
            self.world_pos[0] + self.size[0] * 0.5,
            (self.bounds_min[1] + self.bounds_max[1]) * 0.5,
            self.world_pos[2] + self.size[1] * 0.5,
        ]
    }

    /// Distance to point
    pub fn distance_to(&self, point: [f32; 3]) -> f32 {
        let center = self.center();
        let dx = point[0] - center[0];
        let dy = point[1] - center[1];
        let dz = point[2] - center[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Update bounds from heights
    pub fn update_bounds(&mut self, min_height: f32, max_height: f32) {
        self.bounds_min[1] = self.world_pos[1] + min_height;
        self.bounds_max[1] = self.world_pos[1] + max_height;
    }
}

impl Default for TerrainChunk {
    fn default() -> Self {
        Self::new(0, 0, [0.0, 0.0, 0.0], [100.0, 100.0])
    }
}

/// Terrain chunk GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TerrainChunkGpuData {
    /// World offset
    pub world_offset: [f32; 4],
    /// Scale (xy = horizontal, z = height, w = unused)
    pub scale: [f32; 4],
    /// LOD info (x = lod level, yzw = neighbor lods for stitching)
    pub lod_info: [f32; 4],
    /// Heightmap UV offset and scale
    pub heightmap_uv: [f32; 4],
}

impl TerrainChunkGpuData {
    /// Creates data
    pub fn new(chunk: &TerrainChunk, terrain: &TerrainCreateInfo) -> Self {
        let uv_scale_x = 1.0 / terrain.chunks_x as f32;
        let uv_scale_z = 1.0 / terrain.chunks_z as f32;

        Self {
            world_offset: [
                chunk.world_pos[0],
                chunk.world_pos[1],
                chunk.world_pos[2],
                0.0,
            ],
            scale: [chunk.size[0], chunk.size[1], terrain.max_height, 0.0],
            lod_info: [
                chunk.lod_level as f32,
                chunk.neighbor_lods[0] as f32,
                chunk.neighbor_lods[1] as f32,
                chunk.neighbor_lods[2] as f32,
            ],
            heightmap_uv: [
                chunk.chunk_x as f32 * uv_scale_x,
                chunk.chunk_z as f32 * uv_scale_z,
                uv_scale_x,
                uv_scale_z,
            ],
        }
    }
}

// ============================================================================
// Procedural Generation
// ============================================================================

/// Terrain noise settings
#[derive(Clone, Debug)]
pub struct TerrainNoiseSettings {
    /// Noise layers
    pub layers: Vec<NoiseLayer>,
    /// Global amplitude
    pub amplitude: f32,
    /// Seed
    pub seed: u32,
}

impl TerrainNoiseSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            amplitude: 1.0,
            seed: 0,
        }
    }

    /// Default fractal noise
    pub fn fractal(octaves: u32) -> Self {
        let mut layers = Vec::with_capacity(octaves as usize);
        let mut frequency = 1.0;
        let mut amplitude = 1.0;

        for _ in 0..octaves {
            layers.push(NoiseLayer {
                noise_type: NoiseType::Perlin,
                frequency,
                amplitude,
                offset: [0.0, 0.0],
            });
            frequency *= 2.0;
            amplitude *= 0.5;
        }

        Self {
            layers,
            amplitude: 1.0,
            seed: 0,
        }
    }

    /// Adds layer
    pub fn with_layer(mut self, layer: NoiseLayer) -> Self {
        self.layers.push(layer);
        self
    }

    /// With seed
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// With amplitude
    pub fn with_amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }
}

impl Default for TerrainNoiseSettings {
    fn default() -> Self {
        Self::fractal(4)
    }
}

/// Noise layer
#[derive(Clone, Copy, Debug)]
pub struct NoiseLayer {
    /// Noise type
    pub noise_type: NoiseType,
    /// Frequency
    pub frequency: f32,
    /// Amplitude
    pub amplitude: f32,
    /// Offset
    pub offset: [f32; 2],
}

impl NoiseLayer {
    /// Creates layer
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            noise_type: NoiseType::Perlin,
            frequency,
            amplitude,
            offset: [0.0, 0.0],
        }
    }

    /// With noise type
    pub fn with_type(mut self, noise_type: NoiseType) -> Self {
        self.noise_type = noise_type;
        self
    }

    /// With offset
    pub fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset = [x, y];
        self
    }
}

impl Default for NoiseLayer {
    fn default() -> Self {
        Self::new(1.0, 1.0)
    }
}

/// Noise type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NoiseType {
    /// Perlin noise
    #[default]
    Perlin  = 0,
    /// Simplex noise
    Simplex = 1,
    /// Voronoi
    Voronoi = 2,
    /// Ridged multifractal
    Ridged  = 3,
    /// Billowy
    Billowy = 4,
}

// ============================================================================
// Terrain Statistics
// ============================================================================

/// Terrain statistics
#[derive(Clone, Debug, Default)]
pub struct TerrainStats {
    /// Total chunks
    pub total_chunks: u32,
    /// Visible chunks
    pub visible_chunks: u32,
    /// Chunks per LOD level
    pub chunks_per_lod: [u32; 8],
    /// Total triangles
    pub total_triangles: u64,
    /// Visible triangles
    pub visible_triangles: u64,
    /// Heightmap samples
    pub heightmap_samples: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl TerrainStats {
    /// Cull rate
    pub fn cull_rate(&self) -> f32 {
        if self.total_chunks == 0 {
            0.0
        } else {
            1.0 - (self.visible_chunks as f32 / self.total_chunks as f32)
        }
    }
}
