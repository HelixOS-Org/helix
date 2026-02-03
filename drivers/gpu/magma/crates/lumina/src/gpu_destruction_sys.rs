//! GPU Destruction Effects System for Lumina
//!
//! This module provides GPU-accelerated destruction and fracture effects
//! including debris simulation, breakable objects, and damage visualization.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Destruction System Handles
// ============================================================================

/// GPU destruction system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuDestructionSystemHandle(pub u64);

impl GpuDestructionSystemHandle {
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

impl Default for GpuDestructionSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Destructible object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DestructibleHandle(pub u64);

impl DestructibleHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DestructibleHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Debris chunk handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DebrisChunkHandle(pub u64);

impl DebrisChunkHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DebrisChunkHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fracture pattern handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FracturePatternHandle(pub u64);

impl FracturePatternHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FracturePatternHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Destruction System Creation
// ============================================================================

/// GPU destruction system create info
#[derive(Clone, Debug)]
pub struct GpuDestructionSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max destructibles
    pub max_destructibles: u32,
    /// Max debris chunks
    pub max_debris: u32,
    /// Max active fragments
    pub max_fragments: u32,
    /// Features
    pub features: DestructionFeatures,
    /// Quality
    pub quality: DestructionQuality,
}

impl GpuDestructionSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_destructibles: 256,
            max_debris: 10000,
            max_fragments: 50000,
            features: DestructionFeatures::all(),
            quality: DestructionQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max destructibles
    pub fn with_max_destructibles(mut self, count: u32) -> Self {
        self.max_destructibles = count;
        self
    }

    /// With max debris
    pub fn with_max_debris(mut self, count: u32) -> Self {
        self.max_debris = count;
        self
    }

    /// With max fragments
    pub fn with_max_fragments(mut self, count: u32) -> Self {
        self.max_fragments = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: DestructionFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: DestructionQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_destructibles(1024)
            .with_max_debris(100000)
            .with_max_fragments(200000)
            .with_quality(DestructionQuality::Ultra)
    }

    /// Mobile
    pub fn mobile() -> Self {
        Self::new()
            .with_max_destructibles(64)
            .with_max_debris(1000)
            .with_max_fragments(5000)
            .with_quality(DestructionQuality::Low)
    }
}

impl Default for GpuDestructionSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Destruction features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct DestructionFeatures: u32 {
        /// None
        const NONE = 0;
        /// Voronoi fracture
        const VORONOI = 1 << 0;
        /// Physics simulation
        const PHYSICS = 1 << 1;
        /// GPU fracture
        const GPU_FRACTURE = 1 << 2;
        /// Debris particles
        const DEBRIS = 1 << 3;
        /// Dust effects
        const DUST = 1 << 4;
        /// Sparks
        const SPARKS = 1 << 5;
        /// Damage decals
        const DECALS = 1 << 6;
        /// Deformation
        const DEFORMATION = 1 << 7;
        /// LOD fragments
        const LOD = 1 << 8;
        /// Pooling
        const POOLING = 1 << 9;
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for DestructionFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Destruction quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DestructionQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    Medium = 1,
    /// High quality
    #[default]
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

// ============================================================================
// Destructible Object
// ============================================================================

/// Destructible create info
#[derive(Clone, Debug)]
pub struct DestructibleCreateInfo {
    /// Name
    pub name: String,
    /// Mesh handle
    pub mesh: u64,
    /// Material handle
    pub material: u64,
    /// Fracture pattern
    pub fracture_pattern: FracturePatternHandle,
    /// Health
    pub health: f32,
    /// Mass
    pub mass: f32,
    /// Destruction settings
    pub settings: DestructionSettings,
    /// Debris settings
    pub debris: DebrisSettings,
    /// Effects
    pub effects: DestructionEffects,
}

impl DestructibleCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            mesh: 0,
            material: 0,
            fracture_pattern: FracturePatternHandle::NULL,
            health: 100.0,
            mass: 100.0,
            settings: DestructionSettings::default(),
            debris: DebrisSettings::default(),
            effects: DestructionEffects::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: u64) -> Self {
        self.mesh = mesh;
        self
    }

    /// With material
    pub fn with_material(mut self, material: u64) -> Self {
        self.material = material;
        self
    }

    /// With fracture pattern
    pub fn with_pattern(mut self, pattern: FracturePatternHandle) -> Self {
        self.fracture_pattern = pattern;
        self
    }

    /// With health
    pub fn with_health(mut self, health: f32) -> Self {
        self.health = health;
        self
    }

    /// With mass
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    /// With settings
    pub fn with_settings(mut self, settings: DestructionSettings) -> Self {
        self.settings = settings;
        self
    }

    /// With debris
    pub fn with_debris(mut self, debris: DebrisSettings) -> Self {
        self.debris = debris;
        self
    }

    /// Glass preset
    pub fn glass() -> Self {
        Self::new()
            .with_name("Glass")
            .with_health(20.0)
            .with_mass(5.0)
            .with_settings(DestructionSettings::glass())
            .with_debris(DebrisSettings::glass())
    }

    /// Wood preset
    pub fn wood() -> Self {
        Self::new()
            .with_name("Wood")
            .with_health(80.0)
            .with_mass(50.0)
            .with_settings(DestructionSettings::wood())
            .with_debris(DebrisSettings::wood())
    }

    /// Concrete preset
    pub fn concrete() -> Self {
        Self::new()
            .with_name("Concrete")
            .with_health(200.0)
            .with_mass(500.0)
            .with_settings(DestructionSettings::concrete())
            .with_debris(DebrisSettings::concrete())
    }

    /// Metal preset
    pub fn metal() -> Self {
        Self::new()
            .with_name("Metal")
            .with_health(300.0)
            .with_mass(200.0)
            .with_settings(DestructionSettings::metal())
            .with_debris(DebrisSettings::metal())
    }
}

impl Default for DestructibleCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Destruction Settings
// ============================================================================

/// Destruction settings
#[derive(Clone, Copy, Debug)]
pub struct DestructionSettings {
    /// Destruction mode
    pub mode: DestructionMode,
    /// Fragment count range
    pub fragment_count_min: u32,
    /// Fragment count max
    pub fragment_count_max: u32,
    /// Impact force threshold
    pub impact_threshold: f32,
    /// Fragment velocity scale
    pub velocity_scale: f32,
    /// Angular velocity scale
    pub angular_velocity_scale: f32,
    /// Damage propagation
    pub damage_propagation: f32,
    /// Support structure
    pub support_enabled: bool,
}

impl DestructionSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            mode: DestructionMode::Fracture,
            fragment_count_min: 5,
            fragment_count_max: 20,
            impact_threshold: 10.0,
            velocity_scale: 1.0,
            angular_velocity_scale: 1.0,
            damage_propagation: 0.5,
            support_enabled: true,
        }
    }

    /// Glass settings
    pub const fn glass() -> Self {
        Self {
            mode: DestructionMode::Shatter,
            fragment_count_min: 20,
            fragment_count_max: 100,
            impact_threshold: 5.0,
            velocity_scale: 2.0,
            angular_velocity_scale: 3.0,
            damage_propagation: 1.0,
            support_enabled: false,
        }
    }

    /// Wood settings
    pub const fn wood() -> Self {
        Self {
            mode: DestructionMode::Splinter,
            fragment_count_min: 5,
            fragment_count_max: 30,
            impact_threshold: 20.0,
            velocity_scale: 1.0,
            angular_velocity_scale: 1.5,
            damage_propagation: 0.7,
            support_enabled: true,
        }
    }

    /// Concrete settings
    pub const fn concrete() -> Self {
        Self {
            mode: DestructionMode::Crumble,
            fragment_count_min: 10,
            fragment_count_max: 50,
            impact_threshold: 50.0,
            velocity_scale: 0.5,
            angular_velocity_scale: 0.5,
            damage_propagation: 0.3,
            support_enabled: true,
        }
    }

    /// Metal settings
    pub const fn metal() -> Self {
        Self {
            mode: DestructionMode::Deform,
            fragment_count_min: 3,
            fragment_count_max: 10,
            impact_threshold: 100.0,
            velocity_scale: 0.3,
            angular_velocity_scale: 0.2,
            damage_propagation: 0.2,
            support_enabled: true,
        }
    }
}

impl Default for DestructionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Destruction mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DestructionMode {
    /// Fracture into chunks
    #[default]
    Fracture = 0,
    /// Shatter (glass-like)
    Shatter = 1,
    /// Splinter (wood-like)
    Splinter = 2,
    /// Crumble (stone-like)
    Crumble = 3,
    /// Deform (metal-like)
    Deform = 4,
    /// Explode
    Explode = 5,
}

// ============================================================================
// Debris Settings
// ============================================================================

/// Debris settings
#[derive(Clone, Copy, Debug)]
pub struct DebrisSettings {
    /// Debris enabled
    pub enabled: bool,
    /// Particle count per fragment
    pub particles_per_fragment: u32,
    /// Particle lifetime
    pub particle_lifetime: f32,
    /// Particle size range
    pub particle_size_min: f32,
    /// Particle size max
    pub particle_size_max: f32,
    /// Gravity scale
    pub gravity_scale: f32,
    /// Friction
    pub friction: f32,
    /// Bounce
    pub bounce: f32,
}

impl DebrisSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            particles_per_fragment: 10,
            particle_lifetime: 5.0,
            particle_size_min: 0.01,
            particle_size_max: 0.1,
            gravity_scale: 1.0,
            friction: 0.5,
            bounce: 0.2,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            particles_per_fragment: 0,
            particle_lifetime: 0.0,
            particle_size_min: 0.0,
            particle_size_max: 0.0,
            gravity_scale: 0.0,
            friction: 0.0,
            bounce: 0.0,
        }
    }

    /// Glass debris
    pub const fn glass() -> Self {
        Self {
            enabled: true,
            particles_per_fragment: 20,
            particle_lifetime: 3.0,
            particle_size_min: 0.002,
            particle_size_max: 0.02,
            gravity_scale: 1.0,
            friction: 0.3,
            bounce: 0.5,
        }
    }

    /// Wood debris
    pub const fn wood() -> Self {
        Self {
            enabled: true,
            particles_per_fragment: 15,
            particle_lifetime: 8.0,
            particle_size_min: 0.005,
            particle_size_max: 0.05,
            gravity_scale: 0.8,
            friction: 0.6,
            bounce: 0.1,
        }
    }

    /// Concrete debris
    pub const fn concrete() -> Self {
        Self {
            enabled: true,
            particles_per_fragment: 25,
            particle_lifetime: 10.0,
            particle_size_min: 0.01,
            particle_size_max: 0.15,
            gravity_scale: 1.0,
            friction: 0.7,
            bounce: 0.1,
        }
    }

    /// Metal debris
    pub const fn metal() -> Self {
        Self {
            enabled: true,
            particles_per_fragment: 5,
            particle_lifetime: 5.0,
            particle_size_min: 0.005,
            particle_size_max: 0.03,
            gravity_scale: 1.0,
            friction: 0.4,
            bounce: 0.4,
        }
    }
}

impl Default for DebrisSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Destruction Effects
// ============================================================================

/// Destruction effects
#[derive(Clone, Copy, Debug)]
pub struct DestructionEffects {
    /// Dust enabled
    pub dust: bool,
    /// Dust color
    pub dust_color: [f32; 4],
    /// Dust intensity
    pub dust_intensity: f32,
    /// Sparks enabled
    pub sparks: bool,
    /// Sparks color
    pub sparks_color: [f32; 3],
    /// Sparks count
    pub sparks_count: u32,
    /// Decals enabled
    pub decals: bool,
    /// Screen shake
    pub screen_shake: f32,
    /// Sound enabled
    pub sound: bool,
}

impl DestructionEffects {
    /// Default effects
    pub const fn new() -> Self {
        Self {
            dust: true,
            dust_color: [0.6, 0.55, 0.5, 0.5],
            dust_intensity: 1.0,
            sparks: false,
            sparks_color: [1.0, 0.8, 0.3],
            sparks_count: 0,
            decals: true,
            screen_shake: 0.0,
            sound: true,
        }
    }

    /// Glass effects
    pub const fn glass() -> Self {
        Self {
            dust: false,
            dust_color: [0.0, 0.0, 0.0, 0.0],
            dust_intensity: 0.0,
            sparks: true,
            sparks_color: [1.0, 1.0, 1.0],
            sparks_count: 10,
            decals: false,
            screen_shake: 0.1,
            sound: true,
        }
    }

    /// Metal effects
    pub const fn metal() -> Self {
        Self {
            dust: false,
            dust_color: [0.0, 0.0, 0.0, 0.0],
            dust_intensity: 0.0,
            sparks: true,
            sparks_color: [1.0, 0.7, 0.2],
            sparks_count: 50,
            decals: true,
            screen_shake: 0.3,
            sound: true,
        }
    }

    /// Concrete effects
    pub const fn concrete() -> Self {
        Self {
            dust: true,
            dust_color: [0.7, 0.65, 0.6, 0.7],
            dust_intensity: 2.0,
            sparks: false,
            sparks_color: [0.0, 0.0, 0.0],
            sparks_count: 0,
            decals: true,
            screen_shake: 0.5,
            sound: true,
        }
    }
}

impl Default for DestructionEffects {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Fracture Pattern
// ============================================================================

/// Fracture pattern create info
#[derive(Clone, Debug)]
pub struct FracturePatternCreateInfo {
    /// Name
    pub name: String,
    /// Pattern type
    pub pattern_type: FracturePatternType,
    /// Seed
    pub seed: u32,
    /// Cell count
    pub cell_count: u32,
    /// Noise scale
    pub noise_scale: f32,
    /// Noise octaves
    pub noise_octaves: u32,
    /// Edge irregularity
    pub edge_irregularity: f32,
}

impl FracturePatternCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            pattern_type: FracturePatternType::Voronoi,
            seed: 0,
            cell_count: 10,
            noise_scale: 1.0,
            noise_octaves: 3,
            edge_irregularity: 0.3,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With pattern type
    pub fn with_type(mut self, pattern_type: FracturePatternType) -> Self {
        self.pattern_type = pattern_type;
        self
    }

    /// With seed
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// With cell count
    pub fn with_cell_count(mut self, count: u32) -> Self {
        self.cell_count = count;
        self
    }

    /// Voronoi preset
    pub fn voronoi(cells: u32) -> Self {
        Self::new()
            .with_name("Voronoi")
            .with_type(FracturePatternType::Voronoi)
            .with_cell_count(cells)
    }

    /// Radial preset
    pub fn radial(segments: u32) -> Self {
        Self::new()
            .with_name("Radial")
            .with_type(FracturePatternType::Radial)
            .with_cell_count(segments)
    }

    /// Grid preset
    pub fn grid(cells: u32) -> Self {
        Self::new()
            .with_name("Grid")
            .with_type(FracturePatternType::Grid)
            .with_cell_count(cells)
    }
}

impl Default for FracturePatternCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Fracture pattern type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FracturePatternType {
    /// Voronoi
    #[default]
    Voronoi = 0,
    /// Radial
    Radial = 1,
    /// Grid
    Grid = 2,
    /// Slicing
    Slicing = 3,
    /// Custom
    Custom = 4,
}

// ============================================================================
// Damage
// ============================================================================

/// Damage info
#[derive(Clone, Copy, Debug)]
pub struct DamageInfo {
    /// Damage amount
    pub amount: f32,
    /// Impact point
    pub impact_point: [f32; 3],
    /// Impact direction
    pub direction: [f32; 3],
    /// Impact radius
    pub radius: f32,
    /// Damage type
    pub damage_type: DamageType,
    /// Force multiplier
    pub force_multiplier: f32,
}

impl DamageInfo {
    /// Creates new damage
    pub const fn new(amount: f32) -> Self {
        Self {
            amount,
            impact_point: [0.0, 0.0, 0.0],
            direction: [0.0, -1.0, 0.0],
            radius: 0.0,
            damage_type: DamageType::Impact,
            force_multiplier: 1.0,
        }
    }

    /// With impact point
    pub const fn at_point(mut self, point: [f32; 3]) -> Self {
        self.impact_point = point;
        self
    }

    /// With direction
    pub const fn with_direction(mut self, direction: [f32; 3]) -> Self {
        self.direction = direction;
        self
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With type
    pub const fn of_type(mut self, damage_type: DamageType) -> Self {
        self.damage_type = damage_type;
        self
    }
}

impl Default for DamageInfo {
    fn default() -> Self {
        Self::new(10.0)
    }
}

/// Damage type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DamageType {
    /// Impact
    #[default]
    Impact = 0,
    /// Explosion
    Explosion = 1,
    /// Bullet
    Bullet = 2,
    /// Fire
    Fire = 3,
    /// Slice
    Slice = 4,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU destruction constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDestructionConstants {
    /// Time
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Active fragments
    pub active_fragments: u32,
    /// Active debris
    pub active_debris: u32,
    /// Padding
    pub _pad: f32,
}

/// GPU fragment data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuFragmentData {
    /// Position
    pub position: [f32; 3],
    /// Scale
    pub scale: f32,
    /// Rotation quaternion
    pub rotation: [f32; 4],
    /// Velocity
    pub velocity: [f32; 3],
    /// Lifetime
    pub lifetime: f32,
    /// Angular velocity
    pub angular_velocity: [f32; 3],
    /// Mass
    pub mass: f32,
}

/// GPU debris particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDebrisParticle {
    /// Position
    pub position: [f32; 3],
    /// Size
    pub size: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Lifetime
    pub lifetime: f32,
    /// Color
    pub color: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU destruction statistics
#[derive(Clone, Debug, Default)]
pub struct GpuDestructionStats {
    /// Active destructibles
    pub active_destructibles: u32,
    /// Active fragments
    pub active_fragments: u32,
    /// Active debris
    pub active_debris: u32,
    /// Destructions this frame
    pub destructions_this_frame: u32,
    /// Fragments spawned this frame
    pub fragments_spawned: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}
