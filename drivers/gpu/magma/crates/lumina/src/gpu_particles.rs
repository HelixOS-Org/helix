//! GPU Particle System Types for Lumina
//!
//! This module provides GPU-accelerated particle system
//! infrastructure including emission, simulation, and rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Particle System Handles
// ============================================================================

/// Particle system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ParticleSystemHandle(pub u64);

impl ParticleSystemHandle {
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

impl Default for ParticleSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Particle emitter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ParticleEmitterHandle(pub u64);

impl ParticleEmitterHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ParticleEmitterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Particle buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ParticleBufferHandle(pub u64);

impl ParticleBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ParticleBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Particle module handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ParticleModuleHandle(pub u64);

impl ParticleModuleHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ParticleModuleHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Particle System Creation
// ============================================================================

/// Particle system create info
#[derive(Clone, Debug)]
pub struct ParticleSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max particles
    pub max_particles: u32,
    /// Simulation space
    pub simulation_space: SimulationSpace,
    /// Particle layout
    pub particle_layout: ParticleLayout,
    /// Features
    pub features: ParticleFeatures,
    /// Sort mode
    pub sort_mode: ParticleSortMode,
    /// Bounds mode
    pub bounds_mode: BoundsMode,
}

impl ParticleSystemCreateInfo {
    /// Creates new info
    pub fn new(max_particles: u32) -> Self {
        Self {
            name: String::new(),
            max_particles,
            simulation_space: SimulationSpace::World,
            particle_layout: ParticleLayout::default(),
            features: ParticleFeatures::BASIC,
            sort_mode: ParticleSortMode::None,
            bounds_mode: BoundsMode::Automatic,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With simulation space
    pub fn with_space(mut self, space: SimulationSpace) -> Self {
        self.simulation_space = space;
        self
    }

    /// With layout
    pub fn with_layout(mut self, layout: ParticleLayout) -> Self {
        self.particle_layout = layout;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ParticleFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With sort mode
    pub fn with_sort(mut self, sort: ParticleSortMode) -> Self {
        self.sort_mode = sort;
        self
    }

    /// Simple particle system
    pub fn simple(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_layout(ParticleLayout::minimal())
            .with_features(ParticleFeatures::BASIC)
    }

    /// Smoke particles
    pub fn smoke(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_features(ParticleFeatures::BASIC | ParticleFeatures::ROTATION | ParticleFeatures::NOISE)
            .with_sort(ParticleSortMode::BackToFront)
    }

    /// Fire particles
    pub fn fire(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_features(ParticleFeatures::BASIC | ParticleFeatures::COLOR_OVER_LIFE | ParticleFeatures::EMISSION)
            .with_sort(ParticleSortMode::BackToFront)
    }

    /// Sparks particles
    pub fn sparks(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_features(ParticleFeatures::BASIC | ParticleFeatures::TRAILS | ParticleFeatures::COLLISION)
    }

    /// Rain particles
    pub fn rain(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_features(ParticleFeatures::BASIC | ParticleFeatures::STRETCHED_BILLBOARD | ParticleFeatures::COLLISION)
    }

    /// GPU physics particles
    pub fn physics(max_particles: u32) -> Self {
        Self::new(max_particles)
            .with_features(ParticleFeatures::all())
            .with_layout(ParticleLayout::full())
    }
}

impl Default for ParticleSystemCreateInfo {
    fn default() -> Self {
        Self::simple(10000)
    }
}

/// Simulation space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SimulationSpace {
    /// World space
    #[default]
    World = 0,
    /// Local space (relative to emitter)
    Local = 1,
    /// Custom space
    Custom = 2,
}

/// Particle layout
#[derive(Clone, Debug)]
pub struct ParticleLayout {
    /// Attributes
    pub attributes: Vec<ParticleAttribute>,
    /// Total stride
    pub stride: u32,
}

impl ParticleLayout {
    /// Minimal layout (position, velocity, lifetime)
    pub fn minimal() -> Self {
        Self {
            attributes: vec![
                ParticleAttribute::Position,
                ParticleAttribute::Velocity,
                ParticleAttribute::Lifetime,
            ],
            stride: 32,  // vec3 + vec3 + vec2
        }
    }

    /// Standard layout
    pub fn standard() -> Self {
        Self {
            attributes: vec![
                ParticleAttribute::Position,
                ParticleAttribute::Velocity,
                ParticleAttribute::Lifetime,
                ParticleAttribute::Color,
                ParticleAttribute::Size,
            ],
            stride: 48,
        }
    }

    /// Full layout
    pub fn full() -> Self {
        Self {
            attributes: vec![
                ParticleAttribute::Position,
                ParticleAttribute::Velocity,
                ParticleAttribute::Lifetime,
                ParticleAttribute::Color,
                ParticleAttribute::Size,
                ParticleAttribute::Rotation,
                ParticleAttribute::AngularVelocity,
                ParticleAttribute::Custom0,
            ],
            stride: 80,
        }
    }
}

impl Default for ParticleLayout {
    fn default() -> Self {
        Self::standard()
    }
}

/// Particle attribute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ParticleAttribute {
    /// Position (vec3)
    Position = 0,
    /// Velocity (vec3)
    Velocity = 1,
    /// Lifetime (vec2: current, max)
    Lifetime = 2,
    /// Color (vec4)
    Color = 3,
    /// Size (vec2)
    Size = 4,
    /// Rotation (float)
    Rotation = 5,
    /// Angular velocity (float)
    AngularVelocity = 6,
    /// Mass (float)
    Mass = 7,
    /// Custom 0 (vec4)
    Custom0 = 8,
    /// Custom 1 (vec4)
    Custom1 = 9,
    /// Previous position (vec3)
    PreviousPosition = 10,
    /// Random seed (uint)
    Seed = 11,
}

impl ParticleAttribute {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Position | Self::Velocity | Self::PreviousPosition => 12,
            Self::Lifetime | Self::Size => 8,
            Self::Rotation | Self::AngularVelocity | Self::Mass => 4,
            Self::Color | Self::Custom0 | Self::Custom1 => 16,
            Self::Seed => 4,
        }
    }
}

bitflags::bitflags! {
    /// Particle features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ParticleFeatures: u32 {
        /// None
        const NONE = 0;
        /// Basic simulation
        const BASIC = 1 << 0;
        /// Color over lifetime
        const COLOR_OVER_LIFE = 1 << 1;
        /// Size over lifetime
        const SIZE_OVER_LIFE = 1 << 2;
        /// Rotation
        const ROTATION = 1 << 3;
        /// Collision
        const COLLISION = 1 << 4;
        /// Noise
        const NOISE = 1 << 5;
        /// Trails
        const TRAILS = 1 << 6;
        /// Sub-emitters
        const SUB_EMITTERS = 1 << 7;
        /// Lights
        const LIGHTS = 1 << 8;
        /// Forces
        const FORCES = 1 << 9;
        /// Stretched billboard
        const STRETCHED_BILLBOARD = 1 << 10;
        /// Emission
        const EMISSION = 1 << 11;
        /// Velocity inheritance
        const VELOCITY_INHERITANCE = 1 << 12;
        /// Attractors
        const ATTRACTORS = 1 << 13;
    }
}

/// Particle sort mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ParticleSortMode {
    /// No sorting
    #[default]
    None = 0,
    /// Sort back to front (for transparency)
    BackToFront = 1,
    /// Sort front to back
    FrontToBack = 2,
    /// Sort by age (oldest first)
    OldestFirst = 3,
    /// Sort by age (youngest first)
    YoungestFirst = 4,
    /// Sort by depth
    ByDepth = 5,
}

/// Bounds mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BoundsMode {
    /// Automatic bounds
    #[default]
    Automatic = 0,
    /// Fixed bounds
    Fixed = 1,
    /// Computed bounds
    Computed = 2,
}

// ============================================================================
// Particle Emitter
// ============================================================================

/// Emitter create info
#[derive(Clone, Debug)]
pub struct EmitterCreateInfo {
    /// Name
    pub name: String,
    /// Shape
    pub shape: EmitterShape,
    /// Emission rate (particles per second)
    pub emission_rate: f32,
    /// Burst info
    pub bursts: Vec<EmissionBurst>,
    /// Initial speed
    pub initial_speed: ParticleRange,
    /// Initial lifetime
    pub initial_lifetime: ParticleRange,
    /// Initial size
    pub initial_size: ParticleRange,
    /// Initial color
    pub initial_color: [f32; 4],
    /// Initial rotation
    pub initial_rotation: ParticleRange,
    /// Gravity multiplier
    pub gravity_multiplier: f32,
}

impl EmitterCreateInfo {
    /// Creates new info
    pub fn new(shape: EmitterShape) -> Self {
        Self {
            name: String::new(),
            shape,
            emission_rate: 10.0,
            bursts: Vec::new(),
            initial_speed: ParticleRange::constant(5.0),
            initial_lifetime: ParticleRange::range(1.0, 3.0),
            initial_size: ParticleRange::constant(1.0),
            initial_color: [1.0, 1.0, 1.0, 1.0],
            initial_rotation: ParticleRange::constant(0.0),
            gravity_multiplier: 1.0,
        }
    }

    /// With emission rate
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.emission_rate = rate;
        self
    }

    /// With burst
    pub fn with_burst(mut self, burst: EmissionBurst) -> Self {
        self.bursts.push(burst);
        self
    }

    /// With initial speed
    pub fn with_speed(mut self, speed: ParticleRange) -> Self {
        self.initial_speed = speed;
        self
    }

    /// With initial lifetime
    pub fn with_lifetime(mut self, lifetime: ParticleRange) -> Self {
        self.initial_lifetime = lifetime;
        self
    }

    /// With initial size
    pub fn with_size(mut self, size: ParticleRange) -> Self {
        self.initial_size = size;
        self
    }

    /// With initial color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.initial_color = color;
        self
    }

    /// With gravity
    pub fn with_gravity(mut self, multiplier: f32) -> Self {
        self.gravity_multiplier = multiplier;
        self
    }

    /// Point emitter preset
    pub fn point() -> Self {
        Self::new(EmitterShape::Point)
    }

    /// Sphere emitter preset
    pub fn sphere(radius: f32) -> Self {
        Self::new(EmitterShape::Sphere { radius, emit_from_shell: false })
    }

    /// Cone emitter preset
    pub fn cone(angle: f32, radius: f32) -> Self {
        Self::new(EmitterShape::Cone { angle, radius, length: 0.0 })
    }

    /// Box emitter preset
    pub fn box_shape(size: [f32; 3]) -> Self {
        Self::new(EmitterShape::Box { size, emit_from_surface: false })
    }

    /// Circle emitter preset
    pub fn circle(radius: f32) -> Self {
        Self::new(EmitterShape::Circle { radius, arc: 360.0 })
    }
}

impl Default for EmitterCreateInfo {
    fn default() -> Self {
        Self::point()
    }
}

/// Emitter shape
#[derive(Clone, Copy, Debug)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Sphere emitter
    Sphere {
        radius: f32,
        emit_from_shell: bool,
    },
    /// Hemisphere emitter
    Hemisphere {
        radius: f32,
    },
    /// Cone emitter
    Cone {
        angle: f32,
        radius: f32,
        length: f32,
    },
    /// Box emitter
    Box {
        size: [f32; 3],
        emit_from_surface: bool,
    },
    /// Circle emitter
    Circle {
        radius: f32,
        arc: f32,
    },
    /// Edge emitter
    Edge {
        length: f32,
    },
    /// Mesh emitter
    Mesh {
        mesh_handle: u64,
    },
}

impl Default for EmitterShape {
    fn default() -> Self {
        Self::Point
    }
}

/// Emission burst
#[derive(Clone, Copy, Debug)]
pub struct EmissionBurst {
    /// Time offset
    pub time: f32,
    /// Particle count
    pub count: ParticleRangeInt,
    /// Number of cycles (0 = infinite)
    pub cycles: u32,
    /// Interval between cycles
    pub interval: f32,
    /// Probability
    pub probability: f32,
}

impl EmissionBurst {
    /// Creates new burst
    pub fn new(time: f32, count: u32) -> Self {
        Self {
            time,
            count: ParticleRangeInt::constant(count),
            cycles: 1,
            interval: 0.0,
            probability: 1.0,
        }
    }

    /// With count range
    pub fn with_range(mut self, min: u32, max: u32) -> Self {
        self.count = ParticleRangeInt::range(min, max);
        self
    }

    /// With cycles
    pub fn with_cycles(mut self, cycles: u32, interval: f32) -> Self {
        self.cycles = cycles;
        self.interval = interval;
        self
    }

    /// With probability
    pub fn with_probability(mut self, prob: f32) -> Self {
        self.probability = prob;
        self
    }
}

impl Default for EmissionBurst {
    fn default() -> Self {
        Self::new(0.0, 10)
    }
}

/// Particle range (min-max)
#[derive(Clone, Copy, Debug)]
pub struct ParticleRange {
    /// Min value
    pub min: f32,
    /// Max value
    pub max: f32,
}

impl ParticleRange {
    /// Constant value
    pub const fn constant(value: f32) -> Self {
        Self { min: value, max: value }
    }

    /// Range
    pub const fn range(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// Evaluate at t
    pub fn evaluate(&self, t: f32) -> f32 {
        self.min + (self.max - self.min) * t
    }
}

impl Default for ParticleRange {
    fn default() -> Self {
        Self::constant(1.0)
    }
}

/// Particle range (integer)
#[derive(Clone, Copy, Debug)]
pub struct ParticleRangeInt {
    /// Min value
    pub min: u32,
    /// Max value
    pub max: u32,
}

impl ParticleRangeInt {
    /// Constant value
    pub const fn constant(value: u32) -> Self {
        Self { min: value, max: value }
    }

    /// Range
    pub const fn range(min: u32, max: u32) -> Self {
        Self { min, max }
    }
}

impl Default for ParticleRangeInt {
    fn default() -> Self {
        Self::constant(1)
    }
}

// ============================================================================
// Particle Modules
// ============================================================================

/// Particle module type
#[derive(Clone, Debug)]
pub enum ParticleModule {
    /// Color over lifetime
    ColorOverLifetime(ColorOverLifetimeModule),
    /// Size over lifetime
    SizeOverLifetime(SizeOverLifetimeModule),
    /// Velocity over lifetime
    VelocityOverLifetime(VelocityOverLifetimeModule),
    /// Force over lifetime
    ForceOverLifetime(ForceOverLifetimeModule),
    /// Rotation over lifetime
    RotationOverLifetime(RotationOverLifetimeModule),
    /// Noise module
    Noise(NoiseModule),
    /// Collision module
    Collision(CollisionModule),
    /// Attractor module
    Attractor(AttractorModule),
}

/// Color over lifetime module
#[derive(Clone, Debug)]
pub struct ColorOverLifetimeModule {
    /// Color gradient
    pub gradient: ColorGradient,
}

/// Color gradient
#[derive(Clone, Debug, Default)]
pub struct ColorGradient {
    /// Color keys
    pub color_keys: Vec<GradientColorKey>,
    /// Alpha keys
    pub alpha_keys: Vec<GradientAlphaKey>,
}

impl ColorGradient {
    /// Simple fade out
    pub fn fade_out() -> Self {
        Self {
            color_keys: vec![
                GradientColorKey { time: 0.0, color: [1.0, 1.0, 1.0] },
                GradientColorKey { time: 1.0, color: [1.0, 1.0, 1.0] },
            ],
            alpha_keys: vec![
                GradientAlphaKey { time: 0.0, alpha: 1.0 },
                GradientAlphaKey { time: 1.0, alpha: 0.0 },
            ],
        }
    }

    /// Fire gradient
    pub fn fire() -> Self {
        Self {
            color_keys: vec![
                GradientColorKey { time: 0.0, color: [1.0, 1.0, 0.5] },
                GradientColorKey { time: 0.3, color: [1.0, 0.5, 0.0] },
                GradientColorKey { time: 1.0, color: [0.2, 0.0, 0.0] },
            ],
            alpha_keys: vec![
                GradientAlphaKey { time: 0.0, alpha: 1.0 },
                GradientAlphaKey { time: 0.8, alpha: 0.5 },
                GradientAlphaKey { time: 1.0, alpha: 0.0 },
            ],
        }
    }
}

/// Gradient color key
#[derive(Clone, Copy, Debug)]
pub struct GradientColorKey {
    /// Time (0-1)
    pub time: f32,
    /// RGB color
    pub color: [f32; 3],
}

/// Gradient alpha key
#[derive(Clone, Copy, Debug)]
pub struct GradientAlphaKey {
    /// Time (0-1)
    pub time: f32,
    /// Alpha
    pub alpha: f32,
}

/// Size over lifetime module
#[derive(Clone, Debug)]
pub struct SizeOverLifetimeModule {
    /// Size curve
    pub curve: ParticleCurve,
    /// Separate axes
    pub separate_axes: bool,
}

/// Particle curve
#[derive(Clone, Debug)]
pub struct ParticleCurve {
    /// Keyframes
    pub keyframes: Vec<CurveKeyframe>,
}

impl ParticleCurve {
    /// Linear curve
    pub fn linear(start: f32, end: f32) -> Self {
        Self {
            keyframes: vec![
                CurveKeyframe { time: 0.0, value: start, in_tangent: 0.0, out_tangent: 0.0 },
                CurveKeyframe { time: 1.0, value: end, in_tangent: 0.0, out_tangent: 0.0 },
            ],
        }
    }

    /// Ease out curve
    pub fn ease_out(start: f32, end: f32) -> Self {
        Self {
            keyframes: vec![
                CurveKeyframe { time: 0.0, value: start, in_tangent: 0.0, out_tangent: (end - start) },
                CurveKeyframe { time: 1.0, value: end, in_tangent: 0.0, out_tangent: 0.0 },
            ],
        }
    }
}

impl Default for ParticleCurve {
    fn default() -> Self {
        Self::linear(1.0, 0.0)
    }
}

/// Curve keyframe
#[derive(Clone, Copy, Debug, Default)]
pub struct CurveKeyframe {
    /// Time (0-1)
    pub time: f32,
    /// Value
    pub value: f32,
    /// In tangent
    pub in_tangent: f32,
    /// Out tangent
    pub out_tangent: f32,
}

/// Velocity over lifetime module
#[derive(Clone, Debug)]
pub struct VelocityOverLifetimeModule {
    /// Linear velocity
    pub linear: [ParticleRange; 3],
    /// Orbital velocity
    pub orbital: [ParticleRange; 3],
    /// Radial velocity
    pub radial: ParticleRange,
    /// Speed modifier
    pub speed_modifier: ParticleCurve,
}

/// Force over lifetime module
#[derive(Clone, Debug)]
pub struct ForceOverLifetimeModule {
    /// Force
    pub force: [ParticleRange; 3],
    /// Space
    pub space: SimulationSpace,
    /// Randomize
    pub randomize: bool,
}

/// Rotation over lifetime module
#[derive(Clone, Debug)]
pub struct RotationOverLifetimeModule {
    /// Angular velocity
    pub angular_velocity: ParticleRange,
    /// Separate axes
    pub separate_axes: bool,
}

/// Noise module
#[derive(Clone, Debug)]
pub struct NoiseModule {
    /// Strength
    pub strength: f32,
    /// Frequency
    pub frequency: f32,
    /// Scroll speed
    pub scroll_speed: f32,
    /// Damping
    pub damping: bool,
    /// Octaves
    pub octaves: u32,
    /// Quality
    pub quality: NoiseQuality,
}

impl Default for NoiseModule {
    fn default() -> Self {
        Self {
            strength: 1.0,
            frequency: 1.0,
            scroll_speed: 0.0,
            damping: true,
            octaves: 1,
            quality: NoiseQuality::Medium,
        }
    }
}

/// Noise quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NoiseQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
}

/// Collision module
#[derive(Clone, Debug)]
pub struct CollisionModule {
    /// Collision type
    pub collision_type: CollisionType,
    /// Dampen
    pub dampen: f32,
    /// Bounce
    pub bounce: f32,
    /// Lifetime loss
    pub lifetime_loss: f32,
    /// Min kill speed
    pub min_kill_speed: f32,
    /// Collision quality
    pub quality: CollisionQuality,
}

impl Default for CollisionModule {
    fn default() -> Self {
        Self {
            collision_type: CollisionType::World,
            dampen: 0.0,
            bounce: 1.0,
            lifetime_loss: 0.0,
            min_kill_speed: 0.0,
            quality: CollisionQuality::Medium,
        }
    }
}

/// Collision type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CollisionType {
    /// World collision (planes)
    #[default]
    World = 0,
    /// Depth buffer collision
    DepthBuffer = 1,
    /// SDF collision
    Sdf = 2,
}

/// Collision quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CollisionQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
}

/// Attractor module
#[derive(Clone, Debug)]
pub struct AttractorModule {
    /// Attractor position
    pub position: [f32; 3],
    /// Attractor strength
    pub strength: f32,
    /// Radius of influence
    pub radius: f32,
    /// Attractor type
    pub attractor_type: AttractorType,
}

/// Attractor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AttractorType {
    /// Point attractor
    #[default]
    Point = 0,
    /// Line attractor
    Line = 1,
    /// Plane attractor
    Plane = 2,
    /// Vortex attractor
    Vortex = 3,
}

// ============================================================================
// GPU Particle Data
// ============================================================================

/// GPU particle data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuParticleData {
    /// Position
    pub position: [f32; 3],
    /// Lifetime remaining
    pub lifetime: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Max lifetime
    pub max_lifetime: f32,
    /// Color
    pub color: [f32; 4],
    /// Size
    pub size: [f32; 2],
    /// Rotation
    pub rotation: f32,
    /// Random seed
    pub seed: u32,
}

/// GPU emitter params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuEmitterParams {
    /// Emitter position
    pub position: [f32; 3],
    /// Delta time
    pub delta_time: f32,
    /// Emitter rotation (quaternion)
    pub rotation: [f32; 4],
    /// Emission rate
    pub emission_rate: f32,
    /// Time
    pub time: f32,
    /// Particles to emit
    pub emit_count: u32,
    /// Random seed
    pub seed: u32,
    /// Shape type
    pub shape_type: u32,
    /// Shape params
    pub shape_params: [f32; 4],
    /// Initial speed min
    pub speed_min: f32,
    /// Initial speed max
    pub speed_max: f32,
    /// Initial lifetime min
    pub lifetime_min: f32,
    /// Initial lifetime max
    pub lifetime_max: f32,
    /// Initial size min
    pub size_min: f32,
    /// Initial size max
    pub size_max: f32,
    /// Gravity multiplier
    pub gravity: f32,
    /// Padding
    pub _padding: f32,
}

/// GPU simulation params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSimulationParams {
    /// Delta time
    pub delta_time: f32,
    /// Time
    pub time: f32,
    /// Particle count
    pub particle_count: u32,
    /// Max particles
    pub max_particles: u32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Drag
    pub drag: f32,
    /// Noise strength
    pub noise_strength: f32,
    /// Noise frequency
    pub noise_frequency: f32,
    /// Noise scroll
    pub noise_scroll: f32,
    /// Collision enabled
    pub collision_enabled: u32,
    /// Collision plane
    pub collision_plane: [f32; 4],
    /// Bounce
    pub bounce: f32,
    /// Dampen
    pub dampen: f32,
    /// Padding
    pub _padding: [f32; 2],
}

// ============================================================================
// Statistics
// ============================================================================

/// Particle system statistics
#[derive(Clone, Debug, Default)]
pub struct ParticleSystemStats {
    /// Active particle count
    pub active_particles: u32,
    /// Max particles
    pub max_particles: u32,
    /// Particles emitted this frame
    pub emitted_this_frame: u32,
    /// Particles killed this frame
    pub killed_this_frame: u32,
    /// Emission rate
    pub emission_rate: f32,
    /// Simulation time (microseconds)
    pub simulation_time_us: u64,
    /// Render time (microseconds)
    pub render_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl ParticleSystemStats {
    /// Particle capacity usage ratio
    pub fn capacity_ratio(&self) -> f32 {
        if self.max_particles == 0 {
            return 0.0;
        }
        self.active_particles as f32 / self.max_particles as f32
    }

    /// Net particles (emitted - killed)
    pub fn net_particles(&self) -> i32 {
        self.emitted_this_frame as i32 - self.killed_this_frame as i32
    }
}
