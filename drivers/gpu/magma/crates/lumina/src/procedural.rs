//! Procedural Generation Types for Lumina
//!
//! This module provides procedural generation infrastructure for
//! textures, geometry, and noise-based content generation.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Procedural Handles
// ============================================================================

/// Procedural texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProceduralTextureHandle(pub u64);

impl ProceduralTextureHandle {
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

impl Default for ProceduralTextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Noise generator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NoiseGeneratorHandle(pub u64);

impl NoiseGeneratorHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for NoiseGeneratorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Procedural mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProceduralMeshHandle(pub u64);

impl ProceduralMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ProceduralMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Noise Types
// ============================================================================

/// Noise type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NoiseType {
    /// Perlin noise
    #[default]
    Perlin = 0,
    /// Simplex noise
    Simplex = 1,
    /// Worley (cellular) noise
    Worley = 2,
    /// Value noise
    Value = 3,
    /// Gradient noise
    Gradient = 4,
    /// Voronoi
    Voronoi = 5,
    /// White noise
    White = 6,
    /// Blue noise
    Blue = 7,
    /// OpenSimplex 2
    OpenSimplex2 = 8,
    /// OpenSimplex 2S
    OpenSimplex2S = 9,
}

impl NoiseType {
    /// Is gradient-based
    pub const fn is_gradient_based(&self) -> bool {
        matches!(self, Self::Perlin | Self::Simplex | Self::OpenSimplex2 | Self::OpenSimplex2S)
    }

    /// Is cellular
    pub const fn is_cellular(&self) -> bool {
        matches!(self, Self::Worley | Self::Voronoi)
    }

    /// Supports 3D
    pub const fn supports_3d(&self) -> bool {
        !matches!(self, Self::Blue)
    }

    /// Supports 4D
    pub const fn supports_4d(&self) -> bool {
        matches!(self, Self::Perlin | Self::Simplex | Self::Value)
    }
}

/// Noise fractal type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FractalType {
    /// No fractal
    #[default]
    None = 0,
    /// Fractional Brownian Motion
    Fbm = 1,
    /// Ridged multifractal
    Ridged = 2,
    /// Turbulence
    Turbulence = 3,
    /// Ping-pong
    PingPong = 4,
    /// Domain warp progressive
    DomainWarp = 5,
}

impl FractalType {
    /// Default octaves
    pub const fn default_octaves(&self) -> u32 {
        match self {
            Self::None => 1,
            Self::Fbm | Self::Ridged | Self::Turbulence => 6,
            Self::PingPong | Self::DomainWarp => 4,
        }
    }
}

/// Cellular distance function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CellularDistanceFunc {
    /// Euclidean
    #[default]
    Euclidean = 0,
    /// Manhattan
    Manhattan = 1,
    /// Chebyshev
    Chebyshev = 2,
    /// Minkowski
    Minkowski = 3,
}

/// Cellular return type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CellularReturnType {
    /// Cell value
    #[default]
    CellValue = 0,
    /// Distance
    Distance = 1,
    /// Distance 2
    Distance2 = 2,
    /// Distance 2 Add
    Distance2Add = 3,
    /// Distance 2 Sub
    Distance2Sub = 4,
    /// Distance 2 Mul
    Distance2Mul = 5,
    /// Distance 2 Div
    Distance2Div = 6,
}

// ============================================================================
// Noise Configuration
// ============================================================================

/// Noise generator create info
#[derive(Clone, Debug)]
pub struct NoiseGeneratorCreateInfo {
    /// Name
    pub name: String,
    /// Noise type
    pub noise_type: NoiseType,
    /// Fractal type
    pub fractal_type: FractalType,
    /// Seed
    pub seed: u64,
    /// Frequency
    pub frequency: f32,
    /// Octaves
    pub octaves: u32,
    /// Lacunarity
    pub lacunarity: f32,
    /// Gain
    pub gain: f32,
    /// Weighted strength
    pub weighted_strength: f32,
    /// Ping-pong strength
    pub ping_pong_strength: f32,
}

impl NoiseGeneratorCreateInfo {
    /// Creates info
    pub fn new(noise_type: NoiseType) -> Self {
        let fractal = FractalType::Fbm;
        Self {
            name: String::new(),
            noise_type,
            fractal_type: fractal,
            seed: 1337,
            frequency: 0.01,
            octaves: fractal.default_octaves(),
            lacunarity: 2.0,
            gain: 0.5,
            weighted_strength: 0.0,
            ping_pong_strength: 2.0,
        }
    }

    /// With seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// With frequency
    pub fn with_frequency(mut self, freq: f32) -> Self {
        self.frequency = freq;
        self
    }

    /// With fractal
    pub fn with_fractal(mut self, fractal: FractalType, octaves: u32) -> Self {
        self.fractal_type = fractal;
        self.octaves = octaves;
        self
    }

    /// Perlin preset
    pub fn perlin() -> Self {
        Self::new(NoiseType::Perlin)
            .with_frequency(0.02)
    }

    /// Simplex preset
    pub fn simplex() -> Self {
        Self::new(NoiseType::Simplex)
            .with_frequency(0.02)
    }

    /// Worley preset
    pub fn worley() -> Self {
        Self::new(NoiseType::Worley)
            .with_frequency(0.03)
            .with_fractal(FractalType::None, 1)
    }

    /// Terrain preset
    pub fn terrain() -> Self {
        Self::new(NoiseType::OpenSimplex2)
            .with_frequency(0.002)
            .with_fractal(FractalType::Fbm, 8)
    }
}

impl Default for NoiseGeneratorCreateInfo {
    fn default() -> Self {
        Self::new(NoiseType::Perlin)
    }
}

/// Cellular noise settings
#[derive(Clone, Copy, Debug)]
pub struct CellularSettings {
    /// Distance function
    pub distance_func: CellularDistanceFunc,
    /// Return type
    pub return_type: CellularReturnType,
    /// Jitter
    pub jitter: f32,
    /// Minkowski P value
    pub minkowski_p: f32,
}

impl CellularSettings {
    /// Default settings
    pub const fn default_cellular() -> Self {
        Self {
            distance_func: CellularDistanceFunc::Euclidean,
            return_type: CellularReturnType::Distance,
            jitter: 1.0,
            minkowski_p: 2.0,
        }
    }

    /// Voronoi preset
    pub const fn voronoi() -> Self {
        Self {
            return_type: CellularReturnType::CellValue,
            ..Self::default_cellular()
        }
    }

    /// Crackle preset
    pub const fn crackle() -> Self {
        Self {
            return_type: CellularReturnType::Distance2Sub,
            ..Self::default_cellular()
        }
    }
}

impl Default for CellularSettings {
    fn default() -> Self {
        Self::default_cellular()
    }
}

// ============================================================================
// Domain Warp
// ============================================================================

/// Domain warp type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DomainWarpType {
    /// No warp
    #[default]
    None = 0,
    /// Simplex
    Simplex = 1,
    /// Simplex reduced
    SimplexReduced = 2,
    /// Basic grid
    BasicGrid = 3,
}

/// Domain warp settings
#[derive(Clone, Copy, Debug)]
pub struct DomainWarpSettings {
    /// Warp type
    pub warp_type: DomainWarpType,
    /// Amplitude
    pub amplitude: f32,
    /// Frequency
    pub frequency: f32,
    /// Fractal
    pub fractal: FractalType,
    /// Octaves
    pub octaves: u32,
    /// Lacunarity
    pub lacunarity: f32,
    /// Gain
    pub gain: f32,
}

impl DomainWarpSettings {
    /// No warp
    pub const fn none() -> Self {
        Self {
            warp_type: DomainWarpType::None,
            amplitude: 0.0,
            frequency: 0.01,
            fractal: FractalType::None,
            octaves: 1,
            lacunarity: 2.0,
            gain: 0.5,
        }
    }

    /// Subtle warp
    pub const fn subtle() -> Self {
        Self {
            warp_type: DomainWarpType::Simplex,
            amplitude: 30.0,
            frequency: 0.005,
            fractal: FractalType::None,
            octaves: 1,
            lacunarity: 2.0,
            gain: 0.5,
        }
    }

    /// Strong warp
    pub const fn strong() -> Self {
        Self {
            warp_type: DomainWarpType::Simplex,
            amplitude: 100.0,
            frequency: 0.003,
            fractal: FractalType::DomainWarp,
            octaves: 3,
            lacunarity: 2.0,
            gain: 0.5,
        }
    }
}

impl Default for DomainWarpSettings {
    fn default() -> Self {
        Self::none()
    }
}

// ============================================================================
// Procedural Textures
// ============================================================================

/// Procedural texture create info
#[derive(Clone, Debug)]
pub struct ProceduralTextureCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Pattern type
    pub pattern: ProceduralPattern,
    /// Noise settings (for noise patterns)
    pub noise: Option<NoiseGeneratorCreateInfo>,
    /// Seamless tiling
    pub seamless: bool,
    /// HDR output
    pub hdr: bool,
}

impl ProceduralTextureCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32, pattern: ProceduralPattern) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            pattern,
            noise: None,
            seamless: true,
            hdr: false,
        }
    }

    /// With noise
    pub fn with_noise(mut self, noise: NoiseGeneratorCreateInfo) -> Self {
        self.noise = Some(noise);
        self
    }

    /// With seamless
    pub fn with_seamless(mut self, seamless: bool) -> Self {
        self.seamless = seamless;
        self
    }

    /// Noise texture
    pub fn noise(width: u32, height: u32, noise_type: NoiseType) -> Self {
        Self::new(width, height, ProceduralPattern::Noise)
            .with_noise(NoiseGeneratorCreateInfo::new(noise_type))
    }
}

impl Default for ProceduralTextureCreateInfo {
    fn default() -> Self {
        Self::new(512, 512, ProceduralPattern::Noise)
    }
}

/// Procedural pattern type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProceduralPattern {
    /// Noise-based
    #[default]
    Noise = 0,
    /// Checkerboard
    Checkerboard = 1,
    /// Grid
    Grid = 2,
    /// Gradient
    Gradient = 3,
    /// Radial gradient
    RadialGradient = 4,
    /// Brick
    Brick = 5,
    /// Tile
    Tile = 6,
    /// Wood grain
    Wood = 7,
    /// Marble
    Marble = 8,
    /// Clouds
    Clouds = 9,
    /// Fire
    Fire = 10,
    /// Plasma
    Plasma = 11,
    /// Circuit board
    Circuit = 12,
    /// Hex grid
    HexGrid = 13,
}

impl ProceduralPattern {
    /// Requires noise generator
    pub const fn requires_noise(&self) -> bool {
        matches!(
            self,
            Self::Noise | Self::Wood | Self::Marble | Self::Clouds | Self::Fire | Self::Plasma
        )
    }

    /// Is geometric
    pub const fn is_geometric(&self) -> bool {
        matches!(
            self,
            Self::Checkerboard | Self::Grid | Self::Brick | Self::Tile | Self::HexGrid
        )
    }
}

// ============================================================================
// Procedural Geometry
// ============================================================================

/// Procedural mesh create info
#[derive(Clone, Debug)]
pub struct ProceduralMeshCreateInfo {
    /// Name
    pub name: String,
    /// Primitive type
    pub primitive: ProceduralPrimitive,
    /// Parameters
    pub params: ProceduralMeshParams,
    /// Generate normals
    pub generate_normals: bool,
    /// Generate tangents
    pub generate_tangents: bool,
    /// Generate UVs
    pub generate_uvs: bool,
}

impl ProceduralMeshCreateInfo {
    /// Creates info
    pub fn new(primitive: ProceduralPrimitive) -> Self {
        Self {
            name: String::new(),
            primitive,
            params: ProceduralMeshParams::default(),
            generate_normals: true,
            generate_tangents: true,
            generate_uvs: true,
        }
    }

    /// With params
    pub fn with_params(mut self, params: ProceduralMeshParams) -> Self {
        self.params = params;
        self
    }

    /// Sphere
    pub fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        Self::new(ProceduralPrimitive::Sphere)
            .with_params(ProceduralMeshParams::sphere(radius, segments, rings))
    }

    /// Cube
    pub fn cube(size: f32) -> Self {
        Self::new(ProceduralPrimitive::Cube)
            .with_params(ProceduralMeshParams::cube(size))
    }

    /// Plane
    pub fn plane(width: f32, height: f32, segments_x: u32, segments_y: u32) -> Self {
        Self::new(ProceduralPrimitive::Plane)
            .with_params(ProceduralMeshParams::plane(width, height, segments_x, segments_y))
    }

    /// Cylinder
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        Self::new(ProceduralPrimitive::Cylinder)
            .with_params(ProceduralMeshParams::cylinder(radius, height, segments))
    }
}

impl Default for ProceduralMeshCreateInfo {
    fn default() -> Self {
        Self::new(ProceduralPrimitive::Cube)
    }
}

/// Procedural primitive type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProceduralPrimitive {
    /// Cube
    #[default]
    Cube = 0,
    /// Sphere
    Sphere = 1,
    /// Cylinder
    Cylinder = 2,
    /// Cone
    Cone = 3,
    /// Capsule
    Capsule = 4,
    /// Torus
    Torus = 5,
    /// Plane
    Plane = 6,
    /// Disc
    Disc = 7,
    /// Tube
    Tube = 8,
    /// Pyramid
    Pyramid = 9,
    /// Icosphere
    Icosphere = 10,
    /// Geosphere
    Geosphere = 11,
}

/// Procedural mesh parameters
#[derive(Clone, Copy, Debug)]
pub struct ProceduralMeshParams {
    /// Size X
    pub size_x: f32,
    /// Size Y
    pub size_y: f32,
    /// Size Z
    pub size_z: f32,
    /// Radius
    pub radius: f32,
    /// Inner radius (torus)
    pub inner_radius: f32,
    /// Height
    pub height: f32,
    /// Segments X
    pub segments_x: u32,
    /// Segments Y
    pub segments_y: u32,
    /// Segments Z
    pub segments_z: u32,
    /// Rings
    pub rings: u32,
    /// Capped (cylinder/cone)
    pub capped: bool,
}

impl ProceduralMeshParams {
    /// Default params
    pub const fn default_params() -> Self {
        Self {
            size_x: 1.0,
            size_y: 1.0,
            size_z: 1.0,
            radius: 0.5,
            inner_radius: 0.25,
            height: 1.0,
            segments_x: 32,
            segments_y: 32,
            segments_z: 1,
            rings: 16,
            capped: true,
        }
    }

    /// Sphere params
    pub const fn sphere(radius: f32, segments: u32, rings: u32) -> Self {
        Self {
            radius,
            segments_x: segments,
            rings,
            ..Self::default_params()
        }
    }

    /// Cube params
    pub const fn cube(size: f32) -> Self {
        Self {
            size_x: size,
            size_y: size,
            size_z: size,
            ..Self::default_params()
        }
    }

    /// Plane params
    pub const fn plane(width: f32, height: f32, segments_x: u32, segments_y: u32) -> Self {
        Self {
            size_x: width,
            size_y: height,
            segments_x,
            segments_y,
            ..Self::default_params()
        }
    }

    /// Cylinder params
    pub const fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        Self {
            radius,
            height,
            segments_x: segments,
            ..Self::default_params()
        }
    }

    /// Torus params
    pub const fn torus(radius: f32, tube_radius: f32, segments: u32, rings: u32) -> Self {
        Self {
            radius,
            inner_radius: tube_radius,
            segments_x: segments,
            rings,
            ..Self::default_params()
        }
    }
}

impl Default for ProceduralMeshParams {
    fn default() -> Self {
        Self::default_params()
    }
}

// ============================================================================
// Noise GPU Parameters
// ============================================================================

/// Noise GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct NoiseGpuParams {
    /// Seed
    pub seed: u32,
    /// Noise type
    pub noise_type: u32,
    /// Fractal type
    pub fractal_type: u32,
    /// Octaves
    pub octaves: u32,
    /// Frequency
    pub frequency: f32,
    /// Lacunarity
    pub lacunarity: f32,
    /// Gain
    pub gain: f32,
    /// Weighted strength
    pub weighted_strength: f32,
    /// Ping pong strength
    pub ping_pong_strength: f32,
    /// Cellular distance func
    pub cellular_distance_func: u32,
    /// Cellular return type
    pub cellular_return_type: u32,
    /// Cellular jitter
    pub cellular_jitter: f32,
    /// Domain warp type
    pub warp_type: u32,
    /// Domain warp amplitude
    pub warp_amplitude: f32,
    /// Domain warp frequency
    pub warp_frequency: f32,
    /// Padding
    pub _padding: u32,
}

impl NoiseGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Height Map
// ============================================================================

/// Height map create info
#[derive(Clone, Debug)]
pub struct HeightMapCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Noise generator
    pub noise: NoiseGeneratorCreateInfo,
    /// Domain warp
    pub warp: DomainWarpSettings,
    /// Min height
    pub min_height: f32,
    /// Max height
    pub max_height: f32,
}

impl HeightMapCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            noise: NoiseGeneratorCreateInfo::terrain(),
            warp: DomainWarpSettings::subtle(),
            min_height: 0.0,
            max_height: 1.0,
        }
    }

    /// Terrain preset
    pub fn terrain(width: u32, height: u32) -> Self {
        Self {
            warp: DomainWarpSettings::strong(),
            ..Self::new(width, height)
        }
    }
}

impl Default for HeightMapCreateInfo {
    fn default() -> Self {
        Self::new(512, 512)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Procedural generation statistics
#[derive(Clone, Debug, Default)]
pub struct ProceduralStats {
    /// Textures generated
    pub textures_generated: u64,
    /// Meshes generated
    pub meshes_generated: u64,
    /// Total generation time (microseconds)
    pub generation_time_us: u64,
    /// Total vertices generated
    pub total_vertices: u64,
    /// Total triangles generated
    pub total_triangles: u64,
    /// Memory usage
    pub memory_usage: u64,
}
