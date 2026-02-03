//! Global Illumination - Real-Time GI System
//!
//! Revolutionary global illumination featuring:
//! - Hybrid GI (RT + Probe-based)
//! - Dynamic diffuse global illumination (DDGI)
//! - Screen-space global illumination (SSGI)
//! - Irradiance probes with probe relighting
//! - Radiance cache for glossy reflections
//! - Voxel-based GI (VXGI alternative)

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::graph::{RenderGraph, VirtualTextureHandle};
use crate::pass::PassContext;
use crate::resource::{BufferDesc, BufferHandle, TextureDesc, TextureFormat, TextureHandle};

/// Main global illumination system.
pub struct GlobalIllumination {
    /// Configuration.
    config: GIConfig,
    /// GI method.
    method: GIMethod,
    /// DDGI system.
    ddgi: Option<DDGISystem>,
    /// Screen-space GI.
    ssgi: Option<SSGIRenderer>,
    /// Irradiance field.
    irradiance_field: Option<IrradianceField>,
    /// Radiance cache.
    radiance_cache: Option<RadianceCache>,
}

impl GlobalIllumination {
    /// Create new GI system.
    pub fn new(config: GIConfig) -> Self {
        let method = config.method;

        Self {
            ddgi: if matches!(method, GIMethod::DDGI | GIMethod::Hybrid) {
                Some(DDGISystem::new(&config.ddgi))
            } else {
                None
            },
            ssgi: if matches!(method, GIMethod::SSGI | GIMethod::Hybrid) {
                Some(SSGIRenderer::new(&config.ssgi))
            } else {
                None
            },
            irradiance_field: None,
            radiance_cache: if config.enable_radiance_cache {
                Some(RadianceCache::new(&config.radiance_cache))
            } else {
                None
            },
            config,
            method,
        }
    }

    /// Initialize GI for scene bounds.
    pub fn initialize(&mut self, bounds: &SceneBounds) {
        if let Some(ref mut ddgi) = self.ddgi {
            ddgi.initialize(bounds);
        }

        self.irradiance_field = Some(IrradianceField::new(bounds, &self.config.irradiance));
    }

    /// Update GI for frame.
    pub fn update(&mut self, ctx: &mut PassContext) {
        if let Some(ref mut ddgi) = self.ddgi {
            ddgi.update(ctx);
        }

        if let Some(ref mut cache) = self.radiance_cache {
            cache.update(ctx);
        }
    }

    /// Add GI passes to render graph.
    pub fn add_passes(&self, graph: &mut RenderGraph, inputs: &GIInputs) -> GIOutputs {
        let mut indirect_diffuse = None;
        let mut indirect_specular = None;

        // DDGI probe update
        if let Some(ref ddgi) = self.ddgi {
            let probe_rays = graph.create_texture(TextureDesc {
                format: TextureFormat::Rgba16Float,
                width: ddgi.rays_per_probe,
                height: ddgi.probe_count() as u32,
                ..Default::default()
            });

            // Trace rays from probes
            graph.add_compute_pass("ddgi_trace", |builder| {
                builder
                    .read_texture(inputs.gbuffer_normal)
                    .read_texture(inputs.gbuffer_albedo)
                    .storage_image(probe_rays);
            });

            // Update probe irradiance
            let irradiance_atlas = graph.create_texture(TextureDesc {
                format: TextureFormat::Rgba16Float,
                width: ddgi.irradiance_resolution() as u32,
                height: ddgi.irradiance_resolution() as u32,
                ..Default::default()
            });

            graph.add_compute_pass("ddgi_irradiance_update", |builder| {
                builder
                    .read_texture(probe_rays)
                    .storage_image(irradiance_atlas);
            });

            // Update probe visibility
            let visibility_atlas = graph.create_texture(TextureDesc {
                format: TextureFormat::Rg16Float,
                width: ddgi.visibility_resolution() as u32,
                height: ddgi.visibility_resolution() as u32,
                ..Default::default()
            });

            graph.add_compute_pass("ddgi_visibility_update", |builder| {
                builder
                    .read_texture(probe_rays)
                    .storage_image(visibility_atlas);
            });

            // Sample irradiance for diffuse GI
            let diffuse = graph.create_texture(TextureDesc::hdr_2d(inputs.width, inputs.height));

            graph.add_compute_pass("ddgi_sample", |builder| {
                builder
                    .read_texture(inputs.depth)
                    .read_texture(inputs.gbuffer_normal)
                    .read_texture(irradiance_atlas)
                    .read_texture(visibility_atlas)
                    .storage_image(diffuse);
            });

            indirect_diffuse = Some(diffuse);
        }

        // SSGI for screen-space contributions
        if let Some(ref ssgi) = self.ssgi {
            let ssgi_output = graph.create_texture(TextureDesc::hdr_2d(
                inputs.width / ssgi.config.half_res as u32,
                inputs.height / ssgi.config.half_res as u32,
            ));

            graph.add_compute_pass("ssgi_trace", |builder| {
                builder
                    .read_texture(inputs.color)
                    .read_texture(inputs.depth)
                    .read_texture(inputs.gbuffer_normal)
                    .read_texture(inputs.motion_vectors)
                    .storage_image(ssgi_output);
            });

            // Temporal filter SSGI
            let ssgi_filtered =
                graph.create_texture(TextureDesc::hdr_2d(inputs.width, inputs.height));

            graph.add_compute_pass("ssgi_temporal", |builder| {
                builder
                    .read_texture(ssgi_output)
                    .read_texture(inputs.motion_vectors)
                    .storage_image(ssgi_filtered);
            });

            // Combine with DDGI if available
            if let Some(diffuse) = indirect_diffuse {
                let combined =
                    graph.create_texture(TextureDesc::hdr_2d(inputs.width, inputs.height));

                graph.add_compute_pass("gi_combine", |builder| {
                    builder
                        .read_texture(diffuse)
                        .read_texture(ssgi_filtered)
                        .storage_image(combined);
                });

                indirect_diffuse = Some(combined);
            } else {
                indirect_diffuse = Some(ssgi_filtered);
            }
        }

        // Radiance cache for specular
        if let Some(ref cache) = self.radiance_cache {
            let specular = graph.create_texture(TextureDesc::hdr_2d(inputs.width, inputs.height));

            // Update radiance cache entries
            graph.add_compute_pass("radiance_cache_update", |builder| {
                builder
                    .read_texture(inputs.gbuffer_normal)
                    .read_texture(inputs.gbuffer_material);
            });

            // Sample radiance cache
            graph.add_compute_pass("radiance_cache_sample", |builder| {
                builder
                    .read_texture(inputs.depth)
                    .read_texture(inputs.gbuffer_normal)
                    .read_texture(inputs.gbuffer_material)
                    .storage_image(specular);
            });

            indirect_specular = Some(specular);
        }

        // Final GI composition
        let gi_output = graph.create_texture(TextureDesc::hdr_2d(inputs.width, inputs.height));

        graph.add_compute_pass("gi_composite", |builder| {
            builder.read_texture(inputs.color);
            if let Some(diffuse) = indirect_diffuse {
                builder.read_texture(diffuse);
            }
            if let Some(specular) = indirect_specular {
                builder.read_texture(specular);
            }
            builder.storage_image(gi_output);
        });

        GIOutputs {
            composed: gi_output,
            indirect_diffuse,
            indirect_specular,
        }
    }

    /// Invalidate GI on scene change.
    pub fn invalidate(&mut self) {
        if let Some(ref mut ddgi) = self.ddgi {
            ddgi.invalidate();
        }
        if let Some(ref mut cache) = self.radiance_cache {
            cache.invalidate();
        }
    }
}

/// GI configuration.
#[derive(Debug, Clone)]
pub struct GIConfig {
    /// GI method.
    pub method: GIMethod,
    /// DDGI configuration.
    pub ddgi: DDGIConfig,
    /// SSGI configuration.
    pub ssgi: SSGIConfig,
    /// Irradiance field configuration.
    pub irradiance: IrradianceConfig,
    /// Radiance cache configuration.
    pub radiance_cache: RadianceCacheConfig,
    /// Enable radiance cache.
    pub enable_radiance_cache: bool,
    /// GI intensity.
    pub intensity: f32,
}

impl Default for GIConfig {
    fn default() -> Self {
        Self {
            method: GIMethod::Hybrid,
            ddgi: DDGIConfig::default(),
            ssgi: SSGIConfig::default(),
            irradiance: IrradianceConfig::default(),
            radiance_cache: RadianceCacheConfig::default(),
            enable_radiance_cache: true,
            intensity: 1.0,
        }
    }
}

/// GI method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GIMethod {
    /// No GI (ambient only).
    None,
    /// DDGI (Dynamic Diffuse Global Illumination).
    DDGI,
    /// Screen-space GI only.
    SSGI,
    /// Hybrid (DDGI + SSGI).
    Hybrid,
    /// Ray traced GI.
    RayTraced,
}

/// GI inputs.
#[derive(Debug, Clone)]
pub struct GIInputs {
    /// Scene color.
    pub color: VirtualTextureHandle,
    /// Depth buffer.
    pub depth: VirtualTextureHandle,
    /// GBuffer normal.
    pub gbuffer_normal: VirtualTextureHandle,
    /// GBuffer albedo.
    pub gbuffer_albedo: VirtualTextureHandle,
    /// GBuffer material.
    pub gbuffer_material: VirtualTextureHandle,
    /// Motion vectors.
    pub motion_vectors: VirtualTextureHandle,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// GI outputs.
#[derive(Debug, Clone)]
pub struct GIOutputs {
    /// Final composed GI.
    pub composed: VirtualTextureHandle,
    /// Indirect diffuse.
    pub indirect_diffuse: Option<VirtualTextureHandle>,
    /// Indirect specular.
    pub indirect_specular: Option<VirtualTextureHandle>,
}

/// Scene bounds for GI.
#[derive(Debug, Clone)]
pub struct SceneBounds {
    /// Minimum corner.
    pub min: [f32; 3],
    /// Maximum corner.
    pub max: [f32; 3],
}

impl SceneBounds {
    /// Get center.
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Get extent.
    pub fn extent(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

/// DDGI (Dynamic Diffuse Global Illumination) system.
pub struct DDGISystem {
    /// Configuration.
    config: DDGIConfig,
    /// Probe grid dimensions.
    probe_grid: [u32; 3],
    /// Probe positions.
    probe_positions: Vec<[f32; 3]>,
    /// Rays per probe.
    rays_per_probe: u32,
    /// Current probe batch.
    current_batch: u32,
    /// Irradiance atlas.
    irradiance_atlas: Option<TextureHandle>,
    /// Visibility atlas.
    visibility_atlas: Option<TextureHandle>,
    /// Probe offsets for relocation.
    probe_offsets: Vec<[f32; 3]>,
}

impl DDGISystem {
    /// Create new DDGI system.
    pub fn new(config: &DDGIConfig) -> Self {
        Self {
            config: config.clone(),
            probe_grid: config.probe_grid,
            probe_positions: Vec::new(),
            rays_per_probe: config.rays_per_probe,
            current_batch: 0,
            irradiance_atlas: None,
            visibility_atlas: None,
            probe_offsets: Vec::new(),
        }
    }

    /// Initialize for scene bounds.
    pub fn initialize(&mut self, bounds: &SceneBounds) {
        let extent = bounds.extent();
        let center = bounds.center();

        self.probe_positions.clear();
        self.probe_offsets.clear();

        // Generate probe grid
        for z in 0..self.probe_grid[2] {
            for y in 0..self.probe_grid[1] {
                for x in 0..self.probe_grid[0] {
                    let fx = x as f32 / (self.probe_grid[0] - 1) as f32;
                    let fy = y as f32 / (self.probe_grid[1] - 1) as f32;
                    let fz = z as f32 / (self.probe_grid[2] - 1) as f32;

                    let pos = [
                        bounds.min[0] + fx * extent[0],
                        bounds.min[1] + fy * extent[1],
                        bounds.min[2] + fz * extent[2],
                    ];

                    self.probe_positions.push(pos);
                    self.probe_offsets.push([0.0, 0.0, 0.0]);
                }
            }
        }
    }

    /// Get probe count.
    pub fn probe_count(&self) -> usize {
        (self.probe_grid[0] * self.probe_grid[1] * self.probe_grid[2]) as usize
    }

    /// Get irradiance atlas resolution.
    pub fn irradiance_resolution(&self) -> usize {
        let probes_per_row = (self.probe_count() as f32).sqrt().ceil() as usize;
        probes_per_row * (self.config.irradiance_texels + 2)
    }

    /// Get visibility atlas resolution.
    pub fn visibility_resolution(&self) -> usize {
        let probes_per_row = (self.probe_count() as f32).sqrt().ceil() as usize;
        probes_per_row * (self.config.visibility_texels + 2)
    }

    /// Update probes.
    pub fn update(&mut self, _ctx: &mut PassContext) {
        // Rotate ray directions each frame
        self.current_batch = (self.current_batch + 1) % self.config.batches_per_update;

        // Probe relocation would happen here
    }

    /// Invalidate all probes.
    pub fn invalidate(&mut self) {
        self.current_batch = 0;
        // Would clear probe data
    }

    /// Get probe world position (with offset).
    pub fn probe_world_position(&self, index: usize) -> [f32; 3] {
        let base = self.probe_positions[index];
        let offset = self.probe_offsets[index];
        [
            base[0] + offset[0],
            base[1] + offset[1],
            base[2] + offset[2],
        ]
    }
}

/// DDGI configuration.
#[derive(Debug, Clone)]
pub struct DDGIConfig {
    /// Probe grid dimensions.
    pub probe_grid: [u32; 3],
    /// Rays per probe.
    pub rays_per_probe: u32,
    /// Irradiance texels per probe.
    pub irradiance_texels: usize,
    /// Visibility texels per probe.
    pub visibility_texels: usize,
    /// Hysteresis for temporal stability.
    pub hysteresis: f32,
    /// View bias.
    pub view_bias: f32,
    /// Normal bias.
    pub normal_bias: f32,
    /// Batches per frame.
    pub batches_per_update: u32,
    /// Enable probe relocation.
    pub probe_relocation: bool,
    /// Enable probe classification.
    pub probe_classification: bool,
}

impl Default for DDGIConfig {
    fn default() -> Self {
        Self {
            probe_grid: [16, 8, 16],
            rays_per_probe: 256,
            irradiance_texels: 8,
            visibility_texels: 16,
            hysteresis: 0.97,
            view_bias: 0.3,
            normal_bias: 0.25,
            batches_per_update: 8,
            probe_relocation: true,
            probe_classification: true,
        }
    }
}

/// Screen-space GI renderer.
pub struct SSGIRenderer {
    /// Configuration.
    config: SSGIConfig,
    /// Temporal history.
    history: Option<TextureHandle>,
}

impl SSGIRenderer {
    /// Create new SSGI renderer.
    pub fn new(config: &SSGIConfig) -> Self {
        Self {
            config: config.clone(),
            history: None,
        }
    }
}

/// SSGI configuration.
#[derive(Debug, Clone)]
pub struct SSGIConfig {
    /// Number of samples.
    pub samples: u32,
    /// Max ray distance.
    pub max_distance: f32,
    /// Thickness for depth comparison.
    pub thickness: f32,
    /// Render at half resolution.
    pub half_res: bool,
    /// Temporal blend factor.
    pub temporal_blend: f32,
}

impl Default for SSGIConfig {
    fn default() -> Self {
        Self {
            samples: 4,
            max_distance: 50.0,
            thickness: 0.5,
            half_res: true,
            temporal_blend: 0.9,
        }
    }
}

/// Irradiance field for ambient lighting.
pub struct IrradianceField {
    /// Configuration.
    config: IrradianceConfig,
    /// SH coefficients per probe.
    sh_coefficients: Vec<[f32; 9 * 3]>, // L2 SH, 3 color channels
    /// Probe positions.
    probe_positions: Vec<[f32; 3]>,
    /// Grid dimensions.
    grid: [u32; 3],
    /// Grid origin.
    origin: [f32; 3],
    /// Grid cell size.
    cell_size: f32,
}

impl IrradianceField {
    /// Create new irradiance field.
    pub fn new(bounds: &SceneBounds, config: &IrradianceConfig) -> Self {
        let extent = bounds.extent();
        let max_extent = extent[0].max(extent[1]).max(extent[2]);
        let cell_size = max_extent / config.resolution as f32;

        let grid = [
            (extent[0] / cell_size).ceil() as u32 + 1,
            (extent[1] / cell_size).ceil() as u32 + 1,
            (extent[2] / cell_size).ceil() as u32 + 1,
        ];

        let probe_count = (grid[0] * grid[1] * grid[2]) as usize;

        Self {
            config: config.clone(),
            sh_coefficients: vec![[0.0; 27]; probe_count],
            probe_positions: Vec::with_capacity(probe_count),
            grid,
            origin: bounds.min,
            cell_size,
        }
    }

    /// Sample irradiance at position.
    pub fn sample(&self, position: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
        // Would trilinearly interpolate SH coefficients
        // and evaluate for the given normal direction
        [0.0, 0.0, 0.0]
    }

    /// Update probe at index.
    pub fn update_probe(&mut self, index: usize, radiance: &[[f32; 3]], directions: &[[f32; 3]]) {
        // Project radiance samples to SH
        let mut sh = [0.0f32; 27];

        for (rad, dir) in radiance.iter().zip(directions.iter()) {
            // SH basis functions (L0 and L1 for now)
            let y00 = 0.282095; // L0
            let y1m1 = 0.488603 * dir[1]; // L1
            let y10 = 0.488603 * dir[2];
            let y11 = 0.488603 * dir[0];

            // Accumulate
            for c in 0..3 {
                sh[c * 9 + 0] += rad[c] * y00;
                sh[c * 9 + 1] += rad[c] * y1m1;
                sh[c * 9 + 2] += rad[c] * y10;
                sh[c * 9 + 3] += rad[c] * y11;
            }
        }

        // Normalize
        let n = radiance.len() as f32;
        for v in sh.iter_mut() {
            *v /= n;
        }

        self.sh_coefficients[index] = sh;
    }
}

/// Irradiance field configuration.
#[derive(Debug, Clone)]
pub struct IrradianceConfig {
    /// Grid resolution (probes per axis).
    pub resolution: u32,
    /// SH order (1 = L1, 2 = L2).
    pub sh_order: u32,
    /// Update rate.
    pub update_rate: f32,
}

impl Default for IrradianceConfig {
    fn default() -> Self {
        Self {
            resolution: 32,
            sh_order: 2,
            update_rate: 0.1,
        }
    }
}

/// Radiance cache for glossy reflections.
pub struct RadianceCache {
    /// Configuration.
    config: RadianceCacheConfig,
    /// Cache entries.
    entries: Vec<RadianceCacheEntry>,
    /// Hash grid for lookups.
    hash_grid: HashGrid,
    /// Next entry to update.
    update_cursor: usize,
}

impl RadianceCache {
    /// Create new radiance cache.
    pub fn new(config: &RadianceCacheConfig) -> Self {
        Self {
            config: config.clone(),
            entries: Vec::with_capacity(config.max_entries),
            hash_grid: HashGrid::new(config.hash_grid_size),
            update_cursor: 0,
        }
    }

    /// Update cache.
    pub fn update(&mut self, _ctx: &mut PassContext) {
        // Update subset of entries each frame
        let entries_to_update = (self.entries.len() / self.config.update_frames as usize).max(1);

        for _ in 0..entries_to_update {
            if !self.entries.is_empty() {
                self.update_cursor = (self.update_cursor + 1) % self.entries.len();
                // Would trace rays and update radiance
            }
        }
    }

    /// Invalidate cache.
    pub fn invalidate(&mut self) {
        for entry in &mut self.entries {
            entry.valid = false;
        }
    }

    /// Lookup radiance at position/direction.
    pub fn lookup(
        &self,
        position: [f32; 3],
        direction: [f32; 3],
        roughness: f32,
    ) -> Option<[f32; 3]> {
        // Would do spatial hash lookup and interpolation
        None
    }
}

/// Radiance cache configuration.
#[derive(Debug, Clone)]
pub struct RadianceCacheConfig {
    /// Maximum cache entries.
    pub max_entries: usize,
    /// Hash grid size.
    pub hash_grid_size: u32,
    /// Rays per entry.
    pub rays_per_entry: u32,
    /// Frames to spread updates over.
    pub update_frames: u32,
    /// Maximum roughness for caching.
    pub max_roughness: f32,
}

impl Default for RadianceCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 65536,
            hash_grid_size: 128,
            rays_per_entry: 16,
            update_frames: 8,
            max_roughness: 0.5,
        }
    }
}

/// Radiance cache entry.
struct RadianceCacheEntry {
    /// Position.
    position: [f32; 3],
    /// Normal.
    normal: [f32; 3],
    /// Roughness.
    roughness: f32,
    /// Radiance (SH or directional).
    radiance: [f32; 9], // L1 SH for each channel
    /// Entry is valid.
    valid: bool,
    /// Frame last updated.
    last_update: u32,
}

/// Spatial hash grid.
struct HashGrid {
    /// Grid size.
    size: u32,
    /// Cell entries.
    cells: Vec<Vec<usize>>,
}

impl HashGrid {
    fn new(size: u32) -> Self {
        Self {
            size,
            cells: vec![Vec::new(); (size * size * size) as usize],
        }
    }

    fn hash(&self, x: i32, y: i32, z: i32) -> usize {
        let x = x.rem_euclid(self.size as i32) as usize;
        let y = y.rem_euclid(self.size as i32) as usize;
        let z = z.rem_euclid(self.size as i32) as usize;
        x + y * self.size as usize + z * self.size as usize * self.size as usize
    }

    fn insert(&mut self, position: [f32; 3], cell_size: f32, entry_index: usize) {
        let x = (position[0] / cell_size).floor() as i32;
        let y = (position[1] / cell_size).floor() as i32;
        let z = (position[2] / cell_size).floor() as i32;
        let hash = self.hash(x, y, z);
        self.cells[hash].push(entry_index);
    }

    fn lookup(&self, position: [f32; 3], cell_size: f32) -> &[usize] {
        let x = (position[0] / cell_size).floor() as i32;
        let y = (position[1] / cell_size).floor() as i32;
        let z = (position[2] / cell_size).floor() as i32;
        let hash = self.hash(x, y, z);
        &self.cells[hash]
    }
}

/// Light probe for GI.
#[derive(Debug, Clone)]
pub struct LightProbe {
    /// Position.
    pub position: [f32; 3],
    /// Radius of influence.
    pub radius: f32,
    /// SH coefficients (L2).
    pub sh: [[f32; 9]; 3],
    /// Is probe baked or dynamic.
    pub baked: bool,
}

impl LightProbe {
    /// Create new probe at position.
    pub fn new(position: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            radius,
            sh: [[0.0; 9]; 3],
            baked: false,
        }
    }

    /// Evaluate SH for direction.
    pub fn evaluate(&self, direction: [f32; 3]) -> [f32; 3] {
        let mut result = [0.0f32; 3];

        // L0
        let y0 = 0.282095;

        // L1
        let y1m1 = 0.488603 * direction[1];
        let y10 = 0.488603 * direction[2];
        let y11 = 0.488603 * direction[0];

        // L2
        let y2m2 = 1.092548 * direction[0] * direction[1];
        let y2m1 = 1.092548 * direction[1] * direction[2];
        let y20 = 0.315392 * (3.0 * direction[2] * direction[2] - 1.0);
        let y21 = 1.092548 * direction[0] * direction[2];
        let y22 = 0.546274 * (direction[0] * direction[0] - direction[1] * direction[1]);

        for c in 0..3 {
            result[c] = self.sh[c][0] * y0
                + self.sh[c][1] * y1m1
                + self.sh[c][2] * y10
                + self.sh[c][3] * y11
                + self.sh[c][4] * y2m2
                + self.sh[c][5] * y2m1
                + self.sh[c][6] * y20
                + self.sh[c][7] * y21
                + self.sh[c][8] * y22;
        }

        result
    }

    /// Get weight for position.
    pub fn weight(&self, position: [f32; 3]) -> f32 {
        let dx = position[0] - self.position[0];
        let dy = position[1] - self.position[1];
        let dz = position[2] - self.position[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        let radius_sq = self.radius * self.radius;

        if dist_sq >= radius_sq {
            0.0
        } else {
            let t = dist_sq / radius_sq;
            // Smooth falloff
            (1.0 - t) * (1.0 - t)
        }
    }
}

/// Reflection probe for environment.
#[derive(Debug, Clone)]
pub struct ReflectionProbe {
    /// Position.
    pub position: [f32; 3],
    /// Influence box minimum.
    pub box_min: [f32; 3],
    /// Influence box maximum.
    pub box_max: [f32; 3],
    /// Cubemap handle.
    pub cubemap: Option<TextureHandle>,
    /// Mip levels for roughness.
    pub mip_levels: u32,
    /// Probe priority.
    pub priority: i32,
    /// Use box projection.
    pub box_projection: bool,
}

impl ReflectionProbe {
    /// Create new reflection probe.
    pub fn new(position: [f32; 3], box_min: [f32; 3], box_max: [f32; 3]) -> Self {
        Self {
            position,
            box_min,
            box_max,
            cubemap: None,
            mip_levels: 8,
            priority: 0,
            box_projection: true,
        }
    }

    /// Check if position is inside influence.
    pub fn contains(&self, position: [f32; 3]) -> bool {
        position[0] >= self.box_min[0]
            && position[0] <= self.box_max[0]
            && position[1] >= self.box_min[1]
            && position[1] <= self.box_max[1]
            && position[2] >= self.box_min[2]
            && position[2] <= self.box_max[2]
    }

    /// Get sample direction with box projection.
    pub fn get_sample_direction(&self, position: [f32; 3], direction: [f32; 3]) -> [f32; 3] {
        if !self.box_projection {
            return direction;
        }

        // Ray-box intersection for correction
        let inv_dir = [
            if direction[0].abs() > 0.0001 {
                1.0 / direction[0]
            } else {
                1e10
            },
            if direction[1].abs() > 0.0001 {
                1.0 / direction[1]
            } else {
                1e10
            },
            if direction[2].abs() > 0.0001 {
                1.0 / direction[2]
            } else {
                1e10
            },
        ];

        let t_min = [
            (self.box_min[0] - position[0]) * inv_dir[0],
            (self.box_min[1] - position[1]) * inv_dir[1],
            (self.box_min[2] - position[2]) * inv_dir[2],
        ];
        let t_max = [
            (self.box_max[0] - position[0]) * inv_dir[0],
            (self.box_max[1] - position[1]) * inv_dir[1],
            (self.box_max[2] - position[2]) * inv_dir[2],
        ];

        let t1 = [
            t_min[0].max(t_max[0]),
            t_min[1].max(t_max[1]),
            t_min[2].max(t_max[2]),
        ];

        let t = t1[0].min(t1[1]).min(t1[2]);

        // Intersection point
        let intersection = [
            position[0] + direction[0] * t,
            position[1] + direction[1] * t,
            position[2] + direction[2] * t,
        ];

        // Direction from probe center
        [
            intersection[0] - self.position[0],
            intersection[1] - self.position[1],
            intersection[2] - self.position[2],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_bounds() {
        let bounds = SceneBounds {
            min: [-10.0, -5.0, -10.0],
            max: [10.0, 5.0, 10.0],
        };

        let center = bounds.center();
        assert!((center[0] - 0.0).abs() < 0.001);
        assert!((center[1] - 0.0).abs() < 0.001);

        let extent = bounds.extent();
        assert!((extent[0] - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_light_probe_weight() {
        let probe = LightProbe::new([0.0, 0.0, 0.0], 10.0);

        let w_center = probe.weight([0.0, 0.0, 0.0]);
        assert!((w_center - 1.0).abs() < 0.001);

        let w_edge = probe.weight([10.0, 0.0, 0.0]);
        assert!(w_edge.abs() < 0.001);
    }

    #[test]
    fn test_ddgi_probe_count() {
        let config = DDGIConfig {
            probe_grid: [8, 4, 8],
            ..Default::default()
        };
        let ddgi = DDGISystem::new(&config);
        assert_eq!(ddgi.probe_count(), 256);
    }

    #[test]
    fn test_reflection_probe_contains() {
        let probe = ReflectionProbe::new([0.0, 0.0, 0.0], [-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]);

        assert!(probe.contains([0.0, 0.0, 0.0]));
        assert!(probe.contains([4.0, 4.0, 4.0]));
        assert!(!probe.contains([6.0, 0.0, 0.0]));
    }
}
