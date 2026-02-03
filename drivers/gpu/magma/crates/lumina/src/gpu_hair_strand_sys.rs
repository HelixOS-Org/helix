//! GPU Hair/Strand Rendering System for Lumina
//!
//! This module provides GPU-accelerated strand-based hair and fur rendering
//! with physically-based shading, simulation, and LOD.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Hair System Handles
// ============================================================================

/// GPU hair system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuHairSystemHandle(pub u64);

impl GpuHairSystemHandle {
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

impl Default for GpuHairSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Hair asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HairAssetHandle(pub u64);

impl HairAssetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HairAssetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Hair instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HairInstanceHandle(pub u64);

impl HairInstanceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HairInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Hair groom handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HairGroomHandle(pub u64);

impl HairGroomHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HairGroomHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Hair System Creation
// ============================================================================

/// GPU hair system create info
#[derive(Clone, Debug)]
pub struct GpuHairSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max hair assets
    pub max_hair_assets: u32,
    /// Max hair instances
    pub max_hair_instances: u32,
    /// Max strands total
    pub max_strands: u32,
    /// Max vertices per strand
    pub max_vertices_per_strand: u32,
    /// Features
    pub features: HairFeatures,
    /// Quality
    pub quality: HairQuality,
}

impl GpuHairSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_hair_assets: 64,
            max_hair_instances: 256,
            max_strands: 100000,
            max_vertices_per_strand: 32,
            features: HairFeatures::all(),
            quality: HairQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max strands
    pub fn with_max_strands(mut self, count: u32) -> Self {
        self.max_strands = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: HairFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: HairQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_strands(500000)
            .with_quality(HairQuality::Ultra)
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_strands(20000)
            .with_quality(HairQuality::Low)
            .with_features(HairFeatures::BASIC)
    }
}

impl Default for GpuHairSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Hair features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct HairFeatures: u32 {
        /// None
        const NONE = 0;
        /// Strand simulation
        const SIMULATION = 1 << 0;
        /// Marschner shading
        const MARSCHNER_SHADING = 1 << 1;
        /// Self shadowing
        const SELF_SHADOW = 1 << 2;
        /// Deep shadow maps
        const DEEP_SHADOW = 1 << 3;
        /// Order independent transparency
        const OIT = 1 << 4;
        /// LOD system
        const LOD = 1 << 5;
        /// GPU tessellation
        const TESSELLATION = 1 << 6;
        /// Collision detection
        const COLLISION = 1 << 7;
        /// Wind interaction
        const WIND = 1 << 8;
        /// Wetness
        const WETNESS = 1 << 9;
        /// Basic features
        const BASIC = Self::MARSCHNER_SHADING.bits() | Self::LOD.bits();
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for HairFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Hair quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HairQuality {
    /// Low
    Low    = 0,
    /// Medium
    Medium = 1,
    /// High
    #[default]
    High   = 2,
    /// Ultra
    Ultra  = 3,
}

// ============================================================================
// Hair Asset
// ============================================================================

/// Hair asset create info
#[derive(Clone, Debug)]
pub struct HairAssetCreateInfo {
    /// Name
    pub name: String,
    /// Guide strands
    pub guide_strands: Vec<HairStrand>,
    /// Strand generation settings
    pub generation: StrandGenerationSettings,
    /// Hair material
    pub material: HairMaterial,
    /// Root mesh (for attachment)
    pub root_mesh: Option<u64>,
}

impl HairAssetCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            guide_strands: Vec::new(),
            generation: StrandGenerationSettings::default(),
            material: HairMaterial::human_hair(),
            root_mesh: None,
        }
    }

    /// With guide strands
    pub fn with_guides(mut self, guides: Vec<HairStrand>) -> Self {
        self.guide_strands = guides;
        self
    }

    /// With generation settings
    pub fn with_generation(mut self, settings: StrandGenerationSettings) -> Self {
        self.generation = settings;
        self
    }

    /// With material
    pub fn with_material(mut self, material: HairMaterial) -> Self {
        self.material = material;
        self
    }

    /// With root mesh
    pub fn with_root_mesh(mut self, mesh: u64) -> Self {
        self.root_mesh = Some(mesh);
        self
    }
}

impl Default for HairAssetCreateInfo {
    fn default() -> Self {
        Self::new("Hair")
    }
}

/// Hair strand
#[derive(Clone, Debug, Default)]
pub struct HairStrand {
    /// Vertices
    pub vertices: Vec<HairVertex>,
    /// Root UV
    pub root_uv: [f32; 2],
    /// Root normal
    pub root_normal: [f32; 3],
}

impl HairStrand {
    /// Creates new strand
    pub fn new() -> Self {
        Self::default()
    }

    /// With vertices
    pub fn with_vertices(mut self, vertices: Vec<HairVertex>) -> Self {
        self.vertices = vertices;
        self
    }

    /// Add vertex
    pub fn add_vertex(mut self, vertex: HairVertex) -> Self {
        self.vertices.push(vertex);
        self
    }

    /// Vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

/// Hair vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HairVertex {
    /// Position
    pub position: [f32; 3],
    /// Thickness
    pub thickness: f32,
}

impl HairVertex {
    /// Creates new vertex
    pub const fn new(x: f32, y: f32, z: f32, thickness: f32) -> Self {
        Self {
            position: [x, y, z],
            thickness,
        }
    }
}

// ============================================================================
// Strand Generation
// ============================================================================

/// Strand generation settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StrandGenerationSettings {
    /// Strands per guide
    pub strands_per_guide: u32,
    /// Interpolation radius
    pub interpolation_radius: f32,
    /// Random seed
    pub random_seed: u32,
    /// Length variation
    pub length_variation: f32,
    /// Thickness variation
    pub thickness_variation: f32,
    /// Curl frequency
    pub curl_frequency: f32,
    /// Curl amplitude
    pub curl_amplitude: f32,
    /// Clumping factor
    pub clumping: f32,
}

impl StrandGenerationSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            strands_per_guide: 16,
            interpolation_radius: 0.01,
            random_seed: 12345,
            length_variation: 0.1,
            thickness_variation: 0.2,
            curl_frequency: 0.0,
            curl_amplitude: 0.0,
            clumping: 0.3,
        }
    }

    /// With strands per guide
    pub const fn with_strands_per_guide(mut self, count: u32) -> Self {
        self.strands_per_guide = count;
        self
    }

    /// With curl
    pub const fn with_curl(mut self, frequency: f32, amplitude: f32) -> Self {
        self.curl_frequency = frequency;
        self.curl_amplitude = amplitude;
        self
    }

    /// With clumping
    pub const fn with_clumping(mut self, factor: f32) -> Self {
        self.clumping = factor;
        self
    }

    /// Straight hair preset
    pub const fn straight() -> Self {
        Self::new()
    }

    /// Wavy hair preset
    pub const fn wavy() -> Self {
        Self::new().with_curl(3.0, 0.02)
    }

    /// Curly hair preset
    pub const fn curly() -> Self {
        Self::new().with_curl(8.0, 0.05)
    }

    /// Coily hair preset
    pub const fn coily() -> Self {
        Self::new().with_curl(15.0, 0.08).with_clumping(0.7)
    }
}

impl Default for StrandGenerationSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hair Material
// ============================================================================

/// Hair material
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HairMaterial {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Melanin
    pub melanin: f32,
    /// Melanin redness
    pub melanin_redness: f32,
    /// Roughness
    pub roughness: f32,
    /// Radial roughness
    pub radial_roughness: f32,
    /// Cuticle angle
    pub cuticle_angle: f32,
    /// IOR
    pub ior: f32,
    /// Scatter
    pub scatter: f32,
    /// Backlit
    pub backlit: f32,
    /// Specular tint
    pub specular_tint: f32,
    /// Secondary highlight shift
    pub secondary_shift: f32,
}

impl HairMaterial {
    /// Creates new material
    pub const fn new() -> Self {
        Self {
            base_color: [0.05, 0.02, 0.01, 1.0],
            tip_color: [0.1, 0.05, 0.02, 1.0],
            melanin: 0.5,
            melanin_redness: 0.5,
            roughness: 0.3,
            radial_roughness: 0.5,
            cuticle_angle: 3.0,
            ior: 1.55,
            scatter: 0.0,
            backlit: 0.0,
            specular_tint: 0.0,
            secondary_shift: 5.0,
        }
    }

    /// With color
    pub const fn with_color(mut self, base: [f32; 4], tip: [f32; 4]) -> Self {
        self.base_color = base;
        self.tip_color = tip;
        self
    }

    /// With melanin
    pub const fn with_melanin(mut self, melanin: f32, redness: f32) -> Self {
        self.melanin = melanin;
        self.melanin_redness = redness;
        self
    }

    /// With roughness
    pub const fn with_roughness(mut self, roughness: f32, radial: f32) -> Self {
        self.roughness = roughness;
        self.radial_roughness = radial;
        self
    }

    /// Human black hair preset
    pub const fn black_hair() -> Self {
        Self::new().with_melanin(0.9, 0.3)
    }

    /// Human brown hair preset
    pub const fn brown_hair() -> Self {
        Self::new().with_melanin(0.6, 0.5)
    }

    /// Human blonde hair preset
    pub const fn blonde_hair() -> Self {
        Self::new().with_melanin(0.2, 0.6)
    }

    /// Human red hair preset
    pub const fn red_hair() -> Self {
        Self::new().with_melanin(0.4, 0.9)
    }

    /// Human white/gray hair preset
    pub const fn white_hair() -> Self {
        Self::new().with_melanin(0.0, 0.0)
    }

    /// Default human hair
    pub const fn human_hair() -> Self {
        Self::brown_hair()
    }

    /// Animal fur preset
    pub const fn animal_fur() -> Self {
        Self::new().with_roughness(0.5, 0.6).with_melanin(0.5, 0.4)
    }

    /// Synthetic fiber preset
    pub const fn synthetic() -> Self {
        Self {
            base_color: [0.8, 0.2, 0.3, 1.0],
            tip_color: [0.8, 0.2, 0.3, 1.0],
            melanin: 0.0,
            melanin_redness: 0.0,
            roughness: 0.1,
            radial_roughness: 0.2,
            cuticle_angle: 0.0,
            ior: 1.45,
            scatter: 0.2,
            backlit: 0.5,
            specular_tint: 0.0,
            secondary_shift: 3.0,
        }
    }
}

impl Default for HairMaterial {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hair Instance
// ============================================================================

/// Hair instance create info
#[derive(Clone, Debug)]
pub struct HairInstanceCreateInfo {
    /// Asset
    pub asset: HairAssetHandle,
    /// Transform
    pub transform: [[f32; 4]; 4],
    /// LOD settings
    pub lod: HairLodSettings,
    /// Simulation enabled
    pub simulate: bool,
    /// Collision enabled
    pub collide: bool,
}

impl HairInstanceCreateInfo {
    /// Creates new info
    pub fn new(asset: HairAssetHandle) -> Self {
        Self {
            asset,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            lod: HairLodSettings::default(),
            simulate: true,
            collide: true,
        }
    }

    /// With transform
    pub fn with_transform(mut self, transform: [[f32; 4]; 4]) -> Self {
        self.transform = transform;
        self
    }

    /// With LOD
    pub fn with_lod(mut self, lod: HairLodSettings) -> Self {
        self.lod = lod;
        self
    }

    /// With simulation
    pub fn with_simulation(mut self, enabled: bool) -> Self {
        self.simulate = enabled;
        self
    }

    /// With collision
    pub fn with_collision(mut self, enabled: bool) -> Self {
        self.collide = enabled;
        self
    }
}

/// Hair LOD settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HairLodSettings {
    /// LOD bias
    pub lod_bias: f32,
    /// Min LOD
    pub min_lod: u32,
    /// Max LOD
    pub max_lod: u32,
    /// Screen size thresholds
    pub lod_distances: [f32; 4],
    /// Strand reduction per LOD
    pub strand_reduction: [f32; 4],
    /// Width scale per LOD
    pub width_scale: [f32; 4],
}

impl HairLodSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            lod_bias: 0.0,
            min_lod: 0,
            max_lod: 3,
            lod_distances: [5.0, 15.0, 30.0, 60.0],
            strand_reduction: [1.0, 0.5, 0.25, 0.1],
            width_scale: [1.0, 1.5, 2.0, 3.0],
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            lod_bias: 0.0,
            min_lod: 0,
            max_lod: 0,
            lod_distances: [1000.0, 1000.0, 1000.0, 1000.0],
            strand_reduction: [1.0, 1.0, 1.0, 1.0],
            width_scale: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Aggressive LOD
    pub const fn aggressive() -> Self {
        Self {
            lod_bias: 1.0,
            min_lod: 0,
            max_lod: 3,
            lod_distances: [3.0, 8.0, 15.0, 30.0],
            strand_reduction: [1.0, 0.3, 0.1, 0.03],
            width_scale: [1.0, 2.0, 4.0, 8.0],
        }
    }
}

impl Default for HairLodSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hair Simulation
// ============================================================================

/// Hair simulation settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HairSimulationSettings {
    /// Gravity
    pub gravity: [f32; 3],
    /// Damping
    pub damping: f32,
    /// Stiffness
    pub stiffness: f32,
    /// Root stiffness
    pub root_stiffness: f32,
    /// Tip stiffness
    pub tip_stiffness: f32,
    /// Collision radius
    pub collision_radius: f32,
    /// Friction
    pub friction: f32,
    /// Iterations
    pub iterations: u32,
    /// Substeps
    pub substeps: u32,
}

impl HairSimulationSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            damping: 0.02,
            stiffness: 0.8,
            root_stiffness: 1.0,
            tip_stiffness: 0.1,
            collision_radius: 0.002,
            friction: 0.2,
            iterations: 4,
            substeps: 2,
        }
    }

    /// With gravity
    pub const fn with_gravity(mut self, x: f32, y: f32, z: f32) -> Self {
        self.gravity = [x, y, z];
        self
    }

    /// With stiffness
    pub const fn with_stiffness(mut self, stiffness: f32, root: f32, tip: f32) -> Self {
        self.stiffness = stiffness;
        self.root_stiffness = root;
        self.tip_stiffness = tip;
        self
    }

    /// With damping
    pub const fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Stiff hair preset
    pub const fn stiff() -> Self {
        Self::new().with_stiffness(0.95, 1.0, 0.5)
    }

    /// Loose hair preset
    pub const fn loose() -> Self {
        Self::new().with_stiffness(0.5, 1.0, 0.05)
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            damping: 0.02,
            stiffness: 0.8,
            root_stiffness: 1.0,
            tip_stiffness: 0.1,
            collision_radius: 0.002,
            friction: 0.2,
            iterations: 8,
            substeps: 4,
        }
    }
}

impl Default for HairSimulationSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Hair wind settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HairWindSettings {
    /// Wind direction
    pub direction: [f32; 3],
    /// Wind strength
    pub strength: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Turbulence frequency
    pub turbulence_frequency: f32,
    /// Pulse magnitude
    pub pulse_magnitude: f32,
    /// Pulse frequency
    pub pulse_frequency: f32,
}

impl HairWindSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 0.0,
            turbulence: 0.0,
            turbulence_frequency: 1.0,
            pulse_magnitude: 0.0,
            pulse_frequency: 0.5,
        }
    }

    /// No wind
    pub const fn none() -> Self {
        Self::new()
    }

    /// Light breeze
    pub const fn light_breeze() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 0.5,
            turbulence: 0.2,
            turbulence_frequency: 0.5,
            pulse_magnitude: 0.1,
            pulse_frequency: 0.3,
        }
    }

    /// Moderate wind
    pub const fn moderate() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 2.0,
            turbulence: 0.5,
            turbulence_frequency: 1.0,
            pulse_magnitude: 0.3,
            pulse_frequency: 0.5,
        }
    }

    /// Strong wind
    pub const fn strong() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 5.0,
            turbulence: 1.0,
            turbulence_frequency: 2.0,
            pulse_magnitude: 0.5,
            pulse_frequency: 0.8,
        }
    }
}

impl Default for HairWindSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hair Collision
// ============================================================================

/// Hair collider create info
#[derive(Clone, Debug)]
pub struct HairColliderCreateInfo {
    /// Name
    pub name: String,
    /// Shape
    pub shape: HairColliderShape,
    /// Transform
    pub transform: [[f32; 4]; 4],
    /// Friction
    pub friction: f32,
}

impl HairColliderCreateInfo {
    /// Creates new info
    pub fn new(shape: HairColliderShape) -> Self {
        Self {
            name: String::new(),
            shape,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            friction: 0.3,
        }
    }

    /// Sphere
    pub fn sphere(center: [f32; 3], radius: f32) -> Self {
        Self::new(HairColliderShape::Sphere { center, radius })
    }

    /// Capsule
    pub fn capsule(start: [f32; 3], end: [f32; 3], radius: f32) -> Self {
        Self::new(HairColliderShape::Capsule { start, end, radius })
    }

    /// Head collider
    pub fn head(center: [f32; 3], radius: f32) -> Self {
        Self::sphere(center, radius)
    }
}

/// Hair collider shape
#[derive(Clone, Copy, Debug)]
pub enum HairColliderShape {
    /// Sphere
    Sphere {
        /// Center
        center: [f32; 3],
        /// Radius
        radius: f32,
    },
    /// Capsule
    Capsule {
        /// Start point
        start: [f32; 3],
        /// End point
        end: [f32; 3],
        /// Radius
        radius: f32,
    },
    /// SDF (signed distance field)
    Sdf {
        /// SDF texture handle
        sdf_texture: u64,
        /// Bounds min
        bounds_min: [f32; 3],
        /// Bounds max
        bounds_max: [f32; 3],
    },
}

impl Default for HairColliderShape {
    fn default() -> Self {
        Self::Sphere {
            center: [0.0; 3],
            radius: 0.1,
        }
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU hair strand data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuHairStrand {
    /// Root position
    pub root_position: [f32; 3],
    /// Length
    pub length: f32,
    /// Root tangent
    pub root_tangent: [f32; 3],
    /// Strand index
    pub strand_index: u32,
    /// Root UV
    pub root_uv: [f32; 2],
    /// Thickness
    pub thickness: f32,
    /// Pad
    pub _pad: f32,
}

/// GPU hair vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuHairVertex {
    /// Position
    pub position: [f32; 3],
    /// Thickness
    pub thickness: f32,
    /// Previous position (for simulation)
    pub prev_position: [f32; 3],
    /// Segment index
    pub segment_index: u32,
}

/// GPU hair constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuHairConstants {
    /// View projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Time
    pub time: f32,
    /// Wind direction
    pub wind_direction: [f32; 3],
    /// Wind strength
    pub wind_strength: f32,
    /// Strand count
    pub strand_count: u32,
    /// Vertices per strand
    pub vertices_per_strand: u32,
    /// LOD level
    pub lod_level: u32,
    /// Flags
    pub flags: u32,
    /// Width scale
    pub width_scale: f32,
    /// Shadow bias
    pub shadow_bias: f32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuHairConstants {
    fn default() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            view: [[0.0; 4]; 4],
            camera_position: [0.0; 3],
            time: 0.0,
            wind_direction: [1.0, 0.0, 0.0],
            wind_strength: 0.0,
            strand_count: 0,
            vertices_per_strand: 16,
            lod_level: 0,
            flags: 0,
            width_scale: 1.0,
            shadow_bias: 0.001,
            _pad: [0.0; 2],
        }
    }
}

/// GPU hair material params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuHairMaterialParams {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Melanin and melanin redness
    pub melanin: f32,
    /// Melanin redness
    pub melanin_redness: f32,
    /// Roughness
    pub roughness: f32,
    /// Radial roughness
    pub radial_roughness: f32,
    /// Cuticle angle
    pub cuticle_angle: f32,
    /// IOR
    pub ior: f32,
    /// Scatter
    pub scatter: f32,
    /// Backlit
    pub backlit: f32,
    /// Specular tint
    pub specular_tint: f32,
    /// Secondary shift
    pub secondary_shift: f32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuHairMaterialParams {
    fn default() -> Self {
        Self {
            base_color: [0.05, 0.02, 0.01, 1.0],
            tip_color: [0.1, 0.05, 0.02, 1.0],
            melanin: 0.5,
            melanin_redness: 0.5,
            roughness: 0.3,
            radial_roughness: 0.5,
            cuticle_angle: 3.0,
            ior: 1.55,
            scatter: 0.0,
            backlit: 0.0,
            specular_tint: 0.0,
            secondary_shift: 5.0,
            _pad: [0.0; 2],
        }
    }
}

/// GPU hair simulation params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuHairSimulationParams {
    /// Gravity
    pub gravity: [f32; 3],
    /// Damping
    pub damping: f32,
    /// Stiffness
    pub stiffness: f32,
    /// Root stiffness
    pub root_stiffness: f32,
    /// Tip stiffness
    pub tip_stiffness: f32,
    /// Collision radius
    pub collision_radius: f32,
    /// Delta time
    pub delta_time: f32,
    /// Iterations
    pub iterations: u32,
    /// Strand count
    pub strand_count: u32,
    /// Vertices per strand
    pub vertices_per_strand: u32,
}

impl Default for GpuHairSimulationParams {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            damping: 0.02,
            stiffness: 0.8,
            root_stiffness: 1.0,
            tip_stiffness: 0.1,
            collision_radius: 0.002,
            delta_time: 1.0 / 60.0,
            iterations: 4,
            strand_count: 0,
            vertices_per_strand: 16,
        }
    }
}

// ============================================================================
// Hair Statistics
// ============================================================================

/// Hair system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuHairStats {
    /// Total strands
    pub total_strands: u32,
    /// Visible strands
    pub visible_strands: u32,
    /// Total vertices
    pub total_vertices: u32,
    /// Simulation time (ms)
    pub simulation_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Active instances
    pub active_instances: u32,
    /// Collision checks
    pub collision_checks: u32,
}

impl GpuHairStats {
    /// Average vertices per strand
    pub fn avg_vertices_per_strand(&self) -> f32 {
        if self.total_strands > 0 {
            self.total_vertices as f32 / self.total_strands as f32
        } else {
            0.0
        }
    }

    /// Visibility ratio
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_strands > 0 {
            self.visible_strands as f32 / self.total_strands as f32
        } else {
            0.0
        }
    }

    /// Memory in MB
    pub fn memory_mb(&self) -> f32 {
        self.memory_usage as f32 / (1024.0 * 1024.0)
    }
}
