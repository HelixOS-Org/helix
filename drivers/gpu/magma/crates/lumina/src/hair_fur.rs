//! Hair and Fur Rendering Types for Lumina
//!
//! This module provides hair and fur rendering infrastructure
//! including strand-based rendering, LOD, and simulation.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Hair Handles
// ============================================================================

/// Hair asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HairAssetHandle(pub u64);

impl HairAssetHandle {
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

impl Default for HairInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fur asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FurAssetHandle(pub u64);

impl FurAssetHandle {
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

impl Default for FurAssetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Hair Asset
// ============================================================================

/// Hair asset create info
#[derive(Clone, Debug)]
pub struct HairAssetCreateInfo {
    /// Name
    pub name: String,
    /// Strand data
    pub strands: HairStrandData,
    /// Rendering mode
    pub rendering_mode: HairRenderingMode,
    /// LOD settings
    pub lod: HairLodSettings,
    /// Shading model
    pub shading: HairShadingModel,
}

impl HairAssetCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            strands: HairStrandData::default(),
            rendering_mode: HairRenderingMode::Strand,
            lod: HairLodSettings::default(),
            shading: HairShadingModel::Marschner,
        }
    }

    /// With strands
    pub fn with_strands(mut self, strands: HairStrandData) -> Self {
        self.strands = strands;
        self
    }

    /// With rendering mode
    pub fn with_mode(mut self, mode: HairRenderingMode) -> Self {
        self.rendering_mode = mode;
        self
    }
}

impl Default for HairAssetCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Hair strand data
#[derive(Clone, Debug)]
pub struct HairStrandData {
    /// Number of strands
    pub strand_count: u32,
    /// Vertices per strand
    pub vertices_per_strand: u32,
    /// Position buffer
    pub positions: Vec<[f32; 3]>,
    /// Tangent buffer (optional)
    pub tangents: Vec<[f32; 3]>,
    /// Thickness values (per vertex or per strand)
    pub thickness: Vec<f32>,
    /// Root UV coordinates
    pub root_uvs: Vec<[f32; 2]>,
    /// Colors (optional)
    pub colors: Vec<[f32; 4]>,
}

impl HairStrandData {
    /// Creates data
    pub fn new(strand_count: u32, vertices_per_strand: u32) -> Self {
        Self {
            strand_count,
            vertices_per_strand,
            positions: Vec::new(),
            tangents: Vec::new(),
            thickness: Vec::new(),
            root_uvs: Vec::new(),
            colors: Vec::new(),
        }
    }

    /// Total vertex count
    pub fn vertex_count(&self) -> u32 {
        self.strand_count * self.vertices_per_strand
    }

    /// Memory size in bytes
    pub fn memory_size(&self) -> u64 {
        let pos_size = self.positions.len() * 12; // 3 floats
        let tan_size = self.tangents.len() * 12;
        let thick_size = self.thickness.len() * 4;
        let uv_size = self.root_uvs.len() * 8;
        let color_size = self.colors.len() * 16;

        (pos_size + tan_size + thick_size + uv_size + color_size) as u64
    }
}

impl Default for HairStrandData {
    fn default() -> Self {
        Self::new(0, 16)
    }
}

/// Hair rendering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HairRenderingMode {
    /// Full strand rendering
    #[default]
    Strand = 0,
    /// Tessellated ribbons
    Ribbon = 1,
    /// Billboard cards
    Card = 2,
    /// Shell rendering
    Shell = 3,
}

impl HairRenderingMode {
    /// Get triangle multiplier
    pub fn triangle_multiplier(&self) -> u32 {
        match self {
            Self::Strand => 2, // Line strip to triangles
            Self::Ribbon => 4,
            Self::Card => 2,
            Self::Shell => 1,
        }
    }
}

/// Hair shading model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HairShadingModel {
    /// Marschner (physically based)
    #[default]
    Marschner = 0,
    /// Kajiya-Kay (classic)
    KajiyaKay = 1,
    /// D'Eon (dual scattering)
    DEon = 2,
    /// Simplified
    Simplified = 3,
}

// ============================================================================
// Hair Instance
// ============================================================================

/// Hair instance create info
#[derive(Clone, Debug)]
pub struct HairInstanceCreateInfo {
    /// Hair asset
    pub asset: HairAssetHandle,
    /// Transform
    pub transform: [[f32; 4]; 4],
    /// Material
    pub material: HairMaterial,
    /// Simulation enabled
    pub simulation: bool,
    /// Cast shadows
    pub cast_shadows: bool,
}

impl HairInstanceCreateInfo {
    /// Creates info
    pub fn new(asset: HairAssetHandle) -> Self {
        Self {
            asset,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            material: HairMaterial::default(),
            simulation: true,
            cast_shadows: true,
        }
    }

    /// With transform
    pub fn with_transform(mut self, transform: [[f32; 4]; 4]) -> Self {
        self.transform = transform;
        self
    }

    /// With material
    pub fn with_material(mut self, material: HairMaterial) -> Self {
        self.material = material;
        self
    }

    /// Without simulation
    pub fn without_simulation(mut self) -> Self {
        self.simulation = false;
        self
    }
}

impl Default for HairInstanceCreateInfo {
    fn default() -> Self {
        Self::new(HairAssetHandle::NULL)
    }
}

/// Hair material
#[derive(Clone, Copy, Debug)]
pub struct HairMaterial {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Melanin (0=blonde, 1=black)
    pub melanin: f32,
    /// Melanin redness (0=brunette, 1=ginger)
    pub melanin_redness: f32,
    /// Roughness
    pub roughness: f32,
    /// Scatter
    pub scatter: f32,
    /// Shift (specular highlight shift)
    pub shift: f32,
    /// IOR
    pub ior: f32,
    /// Cuticle angle
    pub cuticle_angle: f32,
    /// Random color variation
    pub color_variation: f32,
}

impl HairMaterial {
    /// Creates material
    pub fn new() -> Self {
        Self {
            base_color: [0.1, 0.05, 0.02, 1.0],
            tip_color: [0.1, 0.05, 0.02, 1.0],
            melanin: 0.5,
            melanin_redness: 0.2,
            roughness: 0.3,
            scatter: 0.7,
            shift: 0.03,
            ior: 1.55,
            cuticle_angle: 3.0,
            color_variation: 0.1,
        }
    }

    /// Blonde hair
    pub fn blonde() -> Self {
        Self {
            base_color: [0.8, 0.7, 0.5, 1.0],
            tip_color: [0.9, 0.8, 0.6, 1.0],
            melanin: 0.1,
            melanin_redness: 0.0,
            ..Self::new()
        }
    }

    /// Brown hair
    pub fn brown() -> Self {
        Self {
            base_color: [0.2, 0.1, 0.05, 1.0],
            tip_color: [0.2, 0.1, 0.05, 1.0],
            melanin: 0.5,
            melanin_redness: 0.2,
            ..Self::new()
        }
    }

    /// Black hair
    pub fn black() -> Self {
        Self {
            base_color: [0.02, 0.02, 0.02, 1.0],
            tip_color: [0.02, 0.02, 0.02, 1.0],
            melanin: 1.0,
            melanin_redness: 0.0,
            ..Self::new()
        }
    }

    /// Red/ginger hair
    pub fn ginger() -> Self {
        Self {
            base_color: [0.5, 0.2, 0.05, 1.0],
            tip_color: [0.5, 0.2, 0.05, 1.0],
            melanin: 0.3,
            melanin_redness: 1.0,
            ..Self::new()
        }
    }

    /// White/gray hair
    pub fn white() -> Self {
        Self {
            base_color: [0.9, 0.9, 0.9, 1.0],
            tip_color: [0.95, 0.95, 0.95, 1.0],
            melanin: 0.0,
            melanin_redness: 0.0,
            ..Self::new()
        }
    }

    /// With roughness
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness;
        self
    }
}

impl Default for HairMaterial {
    fn default() -> Self {
        Self::new()
    }
}

/// Hair GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HairGpuData {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Melanin, redness, roughness, scatter
    pub params: [f32; 4],
    /// Shift, IOR, cuticle angle, variation
    pub params2: [f32; 4],
}

impl HairGpuData {
    /// From material
    pub fn from_material(mat: &HairMaterial) -> Self {
        Self {
            base_color: mat.base_color,
            tip_color: mat.tip_color,
            params: [mat.melanin, mat.melanin_redness, mat.roughness, mat.scatter],
            params2: [mat.shift, mat.ior, mat.cuticle_angle, mat.color_variation],
        }
    }
}

// ============================================================================
// Hair LOD
// ============================================================================

/// Hair LOD settings
#[derive(Clone, Copy, Debug)]
pub struct HairLodSettings {
    /// LOD bias
    pub lod_bias: f32,
    /// Max LOD level
    pub max_lod: u32,
    /// Strand decimation per LOD
    pub strand_decimation: f32,
    /// Vertex decimation per LOD
    pub vertex_decimation: f32,
    /// Width scale per LOD
    pub width_scale: f32,
    /// Shadow LOD offset
    pub shadow_lod_offset: i32,
}

impl HairLodSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            lod_bias: 0.0,
            max_lod: 4,
            strand_decimation: 0.5,
            vertex_decimation: 0.5,
            width_scale: 1.5,
            shadow_lod_offset: 1,
        }
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            lod_bias: -1.0,
            max_lod: 2,
            strand_decimation: 0.7,
            vertex_decimation: 0.7,
            width_scale: 1.2,
            shadow_lod_offset: 0,
        }
    }

    /// Performance
    pub fn performance() -> Self {
        Self {
            lod_bias: 1.0,
            max_lod: 6,
            strand_decimation: 0.4,
            vertex_decimation: 0.4,
            width_scale: 2.0,
            shadow_lod_offset: 2,
        }
    }

    /// Get strand count at LOD
    pub fn strands_at_lod(&self, base_strands: u32, lod: u32) -> u32 {
        let factor = self.strand_decimation.powi(lod as i32);
        (base_strands as f32 * factor) as u32
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
pub struct HairSimulationSettings {
    /// Gravity
    pub gravity: [f32; 3],
    /// Damping
    pub damping: f32,
    /// Stiffness (root to tip)
    pub stiffness: f32,
    /// Stiffness curve
    pub stiffness_curve: f32,
    /// Mass per strand
    pub mass: f32,
    /// Collision margin
    pub collision_margin: f32,
    /// Iterations
    pub iterations: u32,
    /// Substeps
    pub substeps: u32,
}

impl HairSimulationSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            damping: 0.1,
            stiffness: 0.5,
            stiffness_curve: 0.8,
            mass: 0.01,
            collision_margin: 0.02,
            iterations: 4,
            substeps: 2,
        }
    }

    /// Stiff hair (short)
    pub fn stiff() -> Self {
        Self {
            stiffness: 0.9,
            stiffness_curve: 0.5,
            ..Self::new()
        }
    }

    /// Soft hair (long)
    pub fn soft() -> Self {
        Self {
            stiffness: 0.2,
            stiffness_curve: 0.95,
            damping: 0.15,
            ..Self::new()
        }
    }

    /// With gravity
    pub fn with_gravity(mut self, gravity: [f32; 3]) -> Self {
        self.gravity = gravity;
        self
    }
}

impl Default for HairSimulationSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Hair simulation GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HairSimGpuParams {
    /// Gravity
    pub gravity: [f32; 3],
    /// Delta time
    pub dt: f32,
    /// Damping
    pub damping: f32,
    /// Stiffness
    pub stiffness: f32,
    /// Stiffness curve
    pub stiffness_curve: f32,
    /// Iterations
    pub iterations: u32,
}

// ============================================================================
// Fur
// ============================================================================

/// Fur asset create info
#[derive(Clone, Debug)]
pub struct FurAssetCreateInfo {
    /// Name
    pub name: String,
    /// Fur type
    pub fur_type: FurType,
    /// Density (strands per unit area)
    pub density: f32,
    /// Length
    pub length: f32,
    /// Length variation
    pub length_variation: f32,
    /// Thickness
    pub thickness: f32,
    /// Segments per strand
    pub segments: u32,
    /// Shell count (for shell rendering)
    pub shell_count: u32,
}

impl FurAssetCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            fur_type: FurType::Short,
            density: 1000.0,
            length: 0.02,
            length_variation: 0.2,
            thickness: 0.001,
            segments: 8,
            shell_count: 32,
        }
    }

    /// Short fur (like velvet)
    pub fn short() -> Self {
        Self {
            fur_type: FurType::Short,
            length: 0.005,
            density: 5000.0,
            ..Self::new()
        }
    }

    /// Medium fur (like cat)
    pub fn medium() -> Self {
        Self {
            fur_type: FurType::Medium,
            length: 0.02,
            density: 2000.0,
            ..Self::new()
        }
    }

    /// Long fur (like dog)
    pub fn long() -> Self {
        Self {
            fur_type: FurType::Long,
            length: 0.1,
            density: 500.0,
            ..Self::new()
        }
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// With length
    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }
}

impl Default for FurAssetCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fur type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FurType {
    /// Short (velvet, peach fuzz)
    #[default]
    Short = 0,
    /// Medium (cat, rabbit)
    Medium = 1,
    /// Long (dog, bear)
    Long = 2,
    /// Feathers
    Feathers = 3,
}

/// Fur material
#[derive(Clone, Copy, Debug)]
pub struct FurMaterial {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Scatter
    pub scatter: f32,
    /// Roughness
    pub roughness: f32,
    /// Density falloff
    pub density_falloff: f32,
    /// Self-shadowing intensity
    pub self_shadow: f32,
}

impl FurMaterial {
    /// Creates material
    pub fn new() -> Self {
        Self {
            base_color: [0.3, 0.2, 0.15, 1.0],
            tip_color: [0.5, 0.4, 0.3, 1.0],
            scatter: 0.5,
            roughness: 0.6,
            density_falloff: 0.3,
            self_shadow: 0.5,
        }
    }

    /// White fur
    pub fn white() -> Self {
        Self {
            base_color: [0.9, 0.9, 0.88, 1.0],
            tip_color: [1.0, 1.0, 0.98, 1.0],
            ..Self::new()
        }
    }

    /// Orange fur (fox, cat)
    pub fn orange() -> Self {
        Self {
            base_color: [0.5, 0.2, 0.05, 1.0],
            tip_color: [0.7, 0.3, 0.1, 1.0],
            ..Self::new()
        }
    }

    /// Gray fur
    pub fn gray() -> Self {
        Self {
            base_color: [0.3, 0.3, 0.32, 1.0],
            tip_color: [0.5, 0.5, 0.52, 1.0],
            ..Self::new()
        }
    }
}

impl Default for FurMaterial {
    fn default() -> Self {
        Self::new()
    }
}

/// Fur GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FurGpuData {
    /// Base color
    pub base_color: [f32; 4],
    /// Tip color
    pub tip_color: [f32; 4],
    /// Scatter, roughness, density falloff, self shadow
    pub params: [f32; 4],
    /// Length, thickness, density, shell index
    pub geometry: [f32; 4],
}

// ============================================================================
// Strand Generation
// ============================================================================

/// Strand generation settings
#[derive(Clone, Copy, Debug)]
pub struct StrandGenerationSettings {
    /// Target strand count
    pub strand_count: u32,
    /// Segments per strand
    pub segments: u32,
    /// Clumping
    pub clumping: f32,
    /// Clump count
    pub clump_count: u32,
    /// Noise amplitude
    pub noise_amplitude: f32,
    /// Noise frequency
    pub noise_frequency: f32,
    /// Curl radius
    pub curl_radius: f32,
    /// Curl frequency
    pub curl_frequency: f32,
}

impl StrandGenerationSettings {
    /// Creates settings
    pub fn new(strand_count: u32) -> Self {
        Self {
            strand_count,
            segments: 16,
            clumping: 0.3,
            clump_count: 100,
            noise_amplitude: 0.02,
            noise_frequency: 1.0,
            curl_radius: 0.0,
            curl_frequency: 0.0,
        }
    }

    /// Straight hair
    pub fn straight(strand_count: u32) -> Self {
        Self {
            curl_radius: 0.0,
            curl_frequency: 0.0,
            noise_amplitude: 0.01,
            ..Self::new(strand_count)
        }
    }

    /// Wavy hair
    pub fn wavy(strand_count: u32) -> Self {
        Self {
            curl_radius: 0.01,
            curl_frequency: 2.0,
            noise_amplitude: 0.02,
            ..Self::new(strand_count)
        }
    }

    /// Curly hair
    pub fn curly(strand_count: u32) -> Self {
        Self {
            curl_radius: 0.03,
            curl_frequency: 5.0,
            noise_amplitude: 0.01,
            clumping: 0.5,
            ..Self::new(strand_count)
        }
    }
}

impl Default for StrandGenerationSettings {
    fn default() -> Self {
        Self::new(10000)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Hair/fur statistics
#[derive(Clone, Debug, Default)]
pub struct HairStats {
    /// Total strand count
    pub strand_count: u32,
    /// Total vertex count
    pub vertex_count: u32,
    /// Visible strands (after culling/LOD)
    pub visible_strands: u32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Simulation time (microseconds)
    pub simulation_time_us: u64,
    /// Render time (microseconds)
    pub render_time_us: u64,
}
