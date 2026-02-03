//! Particle System Types for Lumina
//!
//! This module provides particle system infrastructure including
//! emitters, particle properties, forces, and rendering options.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Particle System Handle
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

/// Emitter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EmitterHandle(pub u64);

impl EmitterHandle {
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

impl Default for EmitterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Particle System Create Info
// ============================================================================

/// Particle system create info
#[derive(Clone, Debug)]
pub struct ParticleSystemCreateInfo {
    /// Name
    pub name: String,
    /// Maximum particle count
    pub max_particles: u32,
    /// Emitters
    pub emitters: Vec<EmitterCreateInfo>,
    /// Simulation space
    pub simulation_space: SimulationSpace,
    /// Fixed timestep
    pub fixed_timestep: f32,
    /// Flags
    pub flags: ParticleSystemFlags,
}

impl ParticleSystemCreateInfo {
    /// Creates new particle system
    pub fn new(name: &str, max_particles: u32) -> Self {
        Self {
            name: String::from(name),
            max_particles,
            emitters: Vec::new(),
            simulation_space: SimulationSpace::World,
            fixed_timestep: 1.0 / 60.0,
            flags: ParticleSystemFlags::DEFAULT,
        }
    }

    /// With emitter
    pub fn with_emitter(mut self, emitter: EmitterCreateInfo) -> Self {
        self.emitters.push(emitter);
        self
    }

    /// With simulation space
    pub fn with_simulation_space(mut self, space: SimulationSpace) -> Self {
        self.simulation_space = space;
        self
    }

    /// With fixed timestep
    pub fn with_timestep(mut self, timestep: f32) -> Self {
        self.fixed_timestep = timestep;
        self
    }

    /// GPU simulation
    pub fn gpu_simulated(mut self) -> Self {
        self.flags = self.flags.union(ParticleSystemFlags::GPU_SIMULATION);
        self
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

/// Particle system flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ParticleSystemFlags(pub u32);

impl ParticleSystemFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Looping
    pub const LOOPING: Self = Self(1 << 0);
    /// Prewarm
    pub const PREWARM: Self = Self(1 << 1);
    /// GPU simulation
    pub const GPU_SIMULATION: Self = Self(1 << 2);
    /// Play on awake
    pub const PLAY_ON_AWAKE: Self = Self(1 << 3);
    /// Cull when offscreen
    pub const CULL_OFFSCREEN: Self = Self(1 << 4);
    /// Default flags
    pub const DEFAULT: Self = Self(Self::LOOPING.0 | Self::PLAY_ON_AWAKE.0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Emitter Create Info
// ============================================================================

/// Emitter create info
#[derive(Clone, Debug)]
pub struct EmitterCreateInfo {
    /// Name
    pub name: String,
    /// Emission shape
    pub shape: EmitterShape,
    /// Emission rate (particles per second)
    pub emission_rate: f32,
    /// Burst emissions
    pub bursts: Vec<EmissionBurst>,
    /// Initial particle properties
    pub initial_properties: ParticleProperties,
    /// Particle lifetime
    pub lifetime: ValueRange,
    /// Start speed
    pub start_speed: ValueRange,
    /// Start size
    pub start_size: ValueRange,
    /// Start rotation
    pub start_rotation: ValueRange,
    /// Start color
    pub start_color: ColorGradient,
    /// Gravity modifier
    pub gravity_modifier: f32,
    /// Inherit velocity
    pub inherit_velocity: f32,
    /// Forces
    pub forces: Vec<ParticleForce>,
    /// Color over lifetime
    pub color_over_lifetime: Option<ColorGradient>,
    /// Size over lifetime
    pub size_over_lifetime: Option<ValueCurve>,
    /// Velocity over lifetime
    pub velocity_over_lifetime: Option<VelocityModule>,
    /// Rotation over lifetime
    pub rotation_over_lifetime: Option<ValueRange>,
    /// Noise module
    pub noise: Option<NoiseModule>,
    /// Collision module
    pub collision: Option<CollisionModule>,
    /// Sub-emitters
    pub sub_emitters: Vec<SubEmitter>,
}

impl EmitterCreateInfo {
    /// Creates new emitter
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            shape: EmitterShape::Point,
            emission_rate: 10.0,
            bursts: Vec::new(),
            initial_properties: ParticleProperties::default(),
            lifetime: ValueRange::constant(5.0),
            start_speed: ValueRange::constant(5.0),
            start_size: ValueRange::constant(1.0),
            start_rotation: ValueRange::constant(0.0),
            start_color: ColorGradient::solid([1.0, 1.0, 1.0, 1.0]),
            gravity_modifier: 0.0,
            inherit_velocity: 0.0,
            forces: Vec::new(),
            color_over_lifetime: None,
            size_over_lifetime: None,
            velocity_over_lifetime: None,
            rotation_over_lifetime: None,
            noise: None,
            collision: None,
            sub_emitters: Vec::new(),
        }
    }

    /// Creates fire emitter preset
    pub fn fire() -> Self {
        Self::new("Fire")
            .with_shape(EmitterShape::Cone {
                angle: 25.0,
                radius: 0.5,
                emit_from: ConeEmitFrom::Base,
            })
            .with_emission_rate(50.0)
            .with_lifetime(ValueRange::random(0.5, 1.5))
            .with_start_speed(ValueRange::random(3.0, 5.0))
            .with_start_size(ValueRange::random(0.3, 0.6))
            .with_color_over_lifetime(ColorGradient::fire())
            .with_size_over_lifetime(ValueCurve::linear_down())
    }

    /// Creates smoke emitter preset
    pub fn smoke() -> Self {
        Self::new("Smoke")
            .with_shape(EmitterShape::Sphere { radius: 0.3 })
            .with_emission_rate(20.0)
            .with_lifetime(ValueRange::random(3.0, 5.0))
            .with_start_speed(ValueRange::random(0.5, 1.5))
            .with_start_size(ValueRange::random(0.5, 1.0))
            .with_gravity_modifier(-0.1)
            .with_color_over_lifetime(ColorGradient::smoke())
            .with_size_over_lifetime(ValueCurve::linear_up())
    }

    /// Creates spark emitter preset
    pub fn sparks() -> Self {
        Self::new("Sparks")
            .with_shape(EmitterShape::Point)
            .with_emission_rate(0.0)
            .with_burst(EmissionBurst::new(0.0, 30, 50))
            .with_lifetime(ValueRange::random(0.3, 0.8))
            .with_start_speed(ValueRange::random(5.0, 15.0))
            .with_start_size(ValueRange::random(0.02, 0.05))
            .with_gravity_modifier(1.0)
            .with_color_over_lifetime(ColorGradient::sparks())
    }

    /// With shape
    pub fn with_shape(mut self, shape: EmitterShape) -> Self {
        self.shape = shape;
        self
    }

    /// With emission rate
    pub fn with_emission_rate(mut self, rate: f32) -> Self {
        self.emission_rate = rate;
        self
    }

    /// With burst
    pub fn with_burst(mut self, burst: EmissionBurst) -> Self {
        self.bursts.push(burst);
        self
    }

    /// With lifetime
    pub fn with_lifetime(mut self, lifetime: ValueRange) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// With start speed
    pub fn with_start_speed(mut self, speed: ValueRange) -> Self {
        self.start_speed = speed;
        self
    }

    /// With start size
    pub fn with_start_size(mut self, size: ValueRange) -> Self {
        self.start_size = size;
        self
    }

    /// With start color
    pub fn with_start_color(mut self, color: ColorGradient) -> Self {
        self.start_color = color;
        self
    }

    /// With gravity modifier
    pub fn with_gravity_modifier(mut self, modifier: f32) -> Self {
        self.gravity_modifier = modifier;
        self
    }

    /// With color over lifetime
    pub fn with_color_over_lifetime(mut self, gradient: ColorGradient) -> Self {
        self.color_over_lifetime = Some(gradient);
        self
    }

    /// With size over lifetime
    pub fn with_size_over_lifetime(mut self, curve: ValueCurve) -> Self {
        self.size_over_lifetime = Some(curve);
        self
    }

    /// With force
    pub fn with_force(mut self, force: ParticleForce) -> Self {
        self.forces.push(force);
        self
    }

    /// With noise
    pub fn with_noise(mut self, noise: NoiseModule) -> Self {
        self.noise = Some(noise);
        self
    }

    /// With collision
    pub fn with_collision(mut self, collision: CollisionModule) -> Self {
        self.collision = Some(collision);
        self
    }
}

// ============================================================================
// Emitter Shapes
// ============================================================================

/// Emitter shape
#[derive(Clone, Debug)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Sphere emitter
    Sphere { radius: f32 },
    /// Hemisphere emitter
    Hemisphere { radius: f32 },
    /// Cone emitter
    Cone {
        angle: f32,
        radius: f32,
        emit_from: ConeEmitFrom,
    },
    /// Box emitter
    Box { size: [f32; 3] },
    /// Circle emitter (2D)
    Circle { radius: f32 },
    /// Edge emitter (line)
    Edge { length: f32 },
    /// Mesh emitter
    Mesh { mesh_handle: u64, emit_from: MeshEmitFrom },
    /// Skinned mesh emitter
    SkinnedMesh {
        mesh_handle: u64,
        emit_from: MeshEmitFrom,
    },
}

impl EmitterShape {
    /// Creates sphere shape
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    /// Creates cone shape
    pub fn cone(angle: f32, radius: f32) -> Self {
        Self::Cone {
            angle,
            radius,
            emit_from: ConeEmitFrom::Base,
        }
    }

    /// Creates box shape
    pub fn box_shape(width: f32, height: f32, depth: f32) -> Self {
        Self::Box {
            size: [width, height, depth],
        }
    }
}

impl Default for EmitterShape {
    fn default() -> Self {
        Self::Point
    }
}

/// Cone emit from location
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ConeEmitFrom {
    /// Emit from base
    #[default]
    Base = 0,
    /// Emit from volume
    Volume = 1,
    /// Emit from shell
    Shell = 2,
}

/// Mesh emit from location
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeshEmitFrom {
    /// Emit from vertices
    #[default]
    Vertices = 0,
    /// Emit from edges
    Edges = 1,
    /// Emit from triangles
    Triangles = 2,
}

// ============================================================================
// Emission Burst
// ============================================================================

/// Emission burst
#[derive(Clone, Debug)]
pub struct EmissionBurst {
    /// Time offset
    pub time: f32,
    /// Minimum count
    pub min_count: u32,
    /// Maximum count
    pub max_count: u32,
    /// Cycle count (0 = infinite)
    pub cycles: u32,
    /// Interval between cycles
    pub interval: f32,
    /// Probability (0-1)
    pub probability: f32,
}

impl EmissionBurst {
    /// Creates new burst
    pub fn new(time: f32, min_count: u32, max_count: u32) -> Self {
        Self {
            time,
            min_count,
            max_count,
            cycles: 1,
            interval: 0.0,
            probability: 1.0,
        }
    }

    /// Creates repeating burst
    pub fn repeating(time: f32, count: u32, interval: f32, cycles: u32) -> Self {
        Self {
            time,
            min_count: count,
            max_count: count,
            cycles,
            interval,
            probability: 1.0,
        }
    }

    /// With cycles
    pub fn with_cycles(mut self, cycles: u32, interval: f32) -> Self {
        self.cycles = cycles;
        self.interval = interval;
        self
    }

    /// With probability
    pub fn with_probability(mut self, probability: f32) -> Self {
        self.probability = probability;
        self
    }
}

// ============================================================================
// Value Types
// ============================================================================

/// Value range (min/max)
#[derive(Clone, Copy, Debug)]
pub struct ValueRange {
    /// Minimum value
    pub min: f32,
    /// Maximum value
    pub max: f32,
}

impl ValueRange {
    /// Constant value
    pub const fn constant(value: f32) -> Self {
        Self { min: value, max: value }
    }

    /// Random between min and max
    pub const fn random(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    /// Evaluate at random t (0-1)
    pub fn evaluate(&self, t: f32) -> f32 {
        self.min + (self.max - self.min) * t
    }

    /// Is constant
    pub fn is_constant(&self) -> bool {
        (self.max - self.min).abs() < 0.0001
    }
}

impl Default for ValueRange {
    fn default() -> Self {
        Self::constant(1.0)
    }
}

/// Value curve with keyframes
#[derive(Clone, Debug)]
pub struct ValueCurve {
    /// Keyframes (time, value)
    pub keyframes: Vec<(f32, f32)>,
}

impl ValueCurve {
    /// Creates constant curve
    pub fn constant(value: f32) -> Self {
        Self {
            keyframes: alloc::vec![(0.0, value), (1.0, value)],
        }
    }

    /// Creates linear curve from 1 to 0
    pub fn linear_down() -> Self {
        Self {
            keyframes: alloc::vec![(0.0, 1.0), (1.0, 0.0)],
        }
    }

    /// Creates linear curve from 0 to 1
    pub fn linear_up() -> Self {
        Self {
            keyframes: alloc::vec![(0.0, 0.0), (1.0, 1.0)],
        }
    }

    /// Creates ease-out curve
    pub fn ease_out() -> Self {
        Self {
            keyframes: alloc::vec![(0.0, 1.0), (0.5, 0.5), (1.0, 0.0)],
        }
    }

    /// Creates pulse curve
    pub fn pulse() -> Self {
        Self {
            keyframes: alloc::vec![(0.0, 0.0), (0.2, 1.0), (0.5, 1.0), (1.0, 0.0)],
        }
    }

    /// Evaluate at time (0-1)
    pub fn evaluate(&self, t: f32) -> f32 {
        if self.keyframes.is_empty() {
            return 1.0;
        }

        let t = t.clamp(0.0, 1.0);

        if t <= self.keyframes[0].0 {
            return self.keyframes[0].1;
        }

        let last = self.keyframes.len() - 1;
        if t >= self.keyframes[last].0 {
            return self.keyframes[last].1;
        }

        for i in 0..last {
            let (t0, v0) = self.keyframes[i];
            let (t1, v1) = self.keyframes[i + 1];

            if t >= t0 && t <= t1 {
                let local_t = (t - t0) / (t1 - t0);
                return v0 + (v1 - v0) * local_t;
            }
        }

        1.0
    }

    /// Add keyframe
    pub fn with_keyframe(mut self, time: f32, value: f32) -> Self {
        self.keyframes.push((time, value));
        self.keyframes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self
    }
}

impl Default for ValueCurve {
    fn default() -> Self {
        Self::constant(1.0)
    }
}

// ============================================================================
// Color Gradient
// ============================================================================

/// Color gradient
#[derive(Clone, Debug)]
pub struct ColorGradient {
    /// Color keys (time, rgba)
    pub keys: Vec<(f32, [f32; 4])>,
}

impl ColorGradient {
    /// Solid color
    pub fn solid(color: [f32; 4]) -> Self {
        Self {
            keys: alloc::vec![(0.0, color), (1.0, color)],
        }
    }

    /// White gradient
    pub fn white() -> Self {
        Self::solid([1.0, 1.0, 1.0, 1.0])
    }

    /// Fire gradient (yellow -> orange -> red -> transparent)
    pub fn fire() -> Self {
        Self {
            keys: alloc::vec![
                (0.0, [1.0, 1.0, 0.5, 1.0]),
                (0.3, [1.0, 0.6, 0.0, 1.0]),
                (0.6, [1.0, 0.2, 0.0, 0.8]),
                (1.0, [0.5, 0.0, 0.0, 0.0]),
            ],
        }
    }

    /// Smoke gradient (white -> gray -> transparent)
    pub fn smoke() -> Self {
        Self {
            keys: alloc::vec![
                (0.0, [0.8, 0.8, 0.8, 0.6]),
                (0.5, [0.5, 0.5, 0.5, 0.4]),
                (1.0, [0.3, 0.3, 0.3, 0.0]),
            ],
        }
    }

    /// Sparks gradient (white -> yellow -> orange -> transparent)
    pub fn sparks() -> Self {
        Self {
            keys: alloc::vec![
                (0.0, [1.0, 1.0, 1.0, 1.0]),
                (0.2, [1.0, 1.0, 0.5, 1.0]),
                (0.5, [1.0, 0.5, 0.0, 0.8]),
                (1.0, [1.0, 0.2, 0.0, 0.0]),
            ],
        }
    }

    /// Evaluate at time (0-1)
    pub fn evaluate(&self, t: f32) -> [f32; 4] {
        if self.keys.is_empty() {
            return [1.0, 1.0, 1.0, 1.0];
        }

        let t = t.clamp(0.0, 1.0);

        if t <= self.keys[0].0 {
            return self.keys[0].1;
        }

        let last = self.keys.len() - 1;
        if t >= self.keys[last].0 {
            return self.keys[last].1;
        }

        for i in 0..last {
            let (t0, c0) = self.keys[i];
            let (t1, c1) = self.keys[i + 1];

            if t >= t0 && t <= t1 {
                let local_t = (t - t0) / (t1 - t0);
                return [
                    c0[0] + (c1[0] - c0[0]) * local_t,
                    c0[1] + (c1[1] - c0[1]) * local_t,
                    c0[2] + (c1[2] - c0[2]) * local_t,
                    c0[3] + (c1[3] - c0[3]) * local_t,
                ];
            }
        }

        [1.0, 1.0, 1.0, 1.0]
    }

    /// Add color key
    pub fn with_key(mut self, time: f32, color: [f32; 4]) -> Self {
        self.keys.push((time, color));
        self.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self
    }
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self::white()
    }
}

// ============================================================================
// Particle Properties
// ============================================================================

/// Initial particle properties
#[derive(Clone, Debug, Default)]
pub struct ParticleProperties {
    /// Custom data channels
    pub custom_data: Vec<ParticleCustomData>,
}

/// Custom particle data
#[derive(Clone, Debug)]
pub struct ParticleCustomData {
    /// Channel name
    pub name: String,
    /// Data type
    pub data_type: CustomDataType,
    /// Initial value
    pub value: ValueRange,
}

/// Custom data type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CustomDataType {
    /// Float
    #[default]
    Float = 0,
    /// Vector2
    Vector2 = 1,
    /// Vector3
    Vector3 = 2,
    /// Vector4
    Vector4 = 3,
}

// ============================================================================
// Particle Forces
// ============================================================================

/// Particle force
#[derive(Clone, Debug)]
pub enum ParticleForce {
    /// Constant force
    Constant { force: [f32; 3] },
    /// Directional wind
    Wind {
        direction: [f32; 3],
        strength: f32,
        turbulence: f32,
    },
    /// Vortex force
    Vortex {
        axis: [f32; 3],
        strength: f32,
        pull: f32,
    },
    /// Attractor
    Attractor {
        position: [f32; 3],
        strength: f32,
        radius: f32,
    },
    /// Repeller
    Repeller {
        position: [f32; 3],
        strength: f32,
        radius: f32,
    },
    /// Drag
    Drag { coefficient: f32 },
    /// Vector field
    VectorField { texture: u64, strength: f32 },
}

impl ParticleForce {
    /// Creates constant force
    pub fn constant(x: f32, y: f32, z: f32) -> Self {
        Self::Constant { force: [x, y, z] }
    }

    /// Creates wind force
    pub fn wind(direction: [f32; 3], strength: f32) -> Self {
        Self::Wind {
            direction,
            strength,
            turbulence: 0.0,
        }
    }

    /// Creates vortex force
    pub fn vortex(axis: [f32; 3], strength: f32) -> Self {
        Self::Vortex {
            axis,
            strength,
            pull: 0.0,
        }
    }

    /// Creates attractor
    pub fn attractor(position: [f32; 3], strength: f32, radius: f32) -> Self {
        Self::Attractor {
            position,
            strength,
            radius,
        }
    }

    /// Creates drag force
    pub fn drag(coefficient: f32) -> Self {
        Self::Drag { coefficient }
    }
}

// ============================================================================
// Velocity Module
// ============================================================================

/// Velocity over lifetime module
#[derive(Clone, Debug)]
pub struct VelocityModule {
    /// Linear velocity
    pub linear: Option<[ValueCurve; 3]>,
    /// Orbital velocity
    pub orbital: Option<[ValueCurve; 3]>,
    /// Radial velocity
    pub radial: Option<ValueCurve>,
    /// Speed modifier
    pub speed_modifier: Option<ValueCurve>,
}

impl VelocityModule {
    /// Creates empty module
    pub fn new() -> Self {
        Self {
            linear: None,
            orbital: None,
            radial: None,
            speed_modifier: None,
        }
    }

    /// With linear velocity
    pub fn with_linear(mut self, x: ValueCurve, y: ValueCurve, z: ValueCurve) -> Self {
        self.linear = Some([x, y, z]);
        self
    }

    /// With radial velocity
    pub fn with_radial(mut self, curve: ValueCurve) -> Self {
        self.radial = Some(curve);
        self
    }

    /// With speed modifier
    pub fn with_speed(mut self, curve: ValueCurve) -> Self {
        self.speed_modifier = Some(curve);
        self
    }
}

impl Default for VelocityModule {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Noise Module
// ============================================================================

/// Noise module for turbulence
#[derive(Clone, Debug)]
pub struct NoiseModule {
    /// Strength
    pub strength: f32,
    /// Frequency
    pub frequency: f32,
    /// Octaves
    pub octaves: u32,
    /// Scroll speed
    pub scroll_speed: f32,
    /// Position amount
    pub position_amount: f32,
    /// Rotation amount
    pub rotation_amount: f32,
    /// Size amount
    pub size_amount: f32,
}

impl NoiseModule {
    /// Creates default noise module
    pub fn new() -> Self {
        Self {
            strength: 1.0,
            frequency: 0.5,
            octaves: 1,
            scroll_speed: 0.0,
            position_amount: 1.0,
            rotation_amount: 0.0,
            size_amount: 0.0,
        }
    }

    /// With strength
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    /// With frequency
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency;
        self
    }

    /// With octaves
    pub fn with_octaves(mut self, octaves: u32) -> Self {
        self.octaves = octaves;
        self
    }
}

impl Default for NoiseModule {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Collision Module
// ============================================================================

/// Collision module
#[derive(Clone, Debug)]
pub struct CollisionModule {
    /// Collision type
    pub collision_type: CollisionType,
    /// Bounce coefficient
    pub bounce: f32,
    /// Lifetime loss on collision
    pub lifetime_loss: f32,
    /// Min kill speed
    pub min_kill_speed: f32,
    /// Collision quality
    pub quality: CollisionQuality,
    /// Planes
    pub planes: Vec<CollisionPlane>,
}

impl CollisionModule {
    /// Creates default module
    pub fn new() -> Self {
        Self {
            collision_type: CollisionType::World,
            bounce: 0.5,
            lifetime_loss: 0.0,
            min_kill_speed: 0.0,
            quality: CollisionQuality::Medium,
            planes: Vec::new(),
        }
    }

    /// World collision
    pub fn world() -> Self {
        Self {
            collision_type: CollisionType::World,
            ..Self::new()
        }
    }

    /// Plane collision
    pub fn planes(planes: Vec<CollisionPlane>) -> Self {
        Self {
            collision_type: CollisionType::Planes,
            planes,
            ..Self::new()
        }
    }

    /// With bounce
    pub fn with_bounce(mut self, bounce: f32) -> Self {
        self.bounce = bounce;
        self
    }
}

impl Default for CollisionModule {
    fn default() -> Self {
        Self::new()
    }
}

/// Collision type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CollisionType {
    /// No collision
    None = 0,
    /// World collision
    #[default]
    World = 1,
    /// Plane collision
    Planes = 2,
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

/// Collision plane
#[derive(Clone, Copy, Debug)]
pub struct CollisionPlane {
    /// Plane normal
    pub normal: [f32; 3],
    /// Distance from origin
    pub distance: f32,
}

impl CollisionPlane {
    /// Creates plane
    pub fn new(normal: [f32; 3], distance: f32) -> Self {
        Self { normal, distance }
    }

    /// Ground plane
    pub fn ground() -> Self {
        Self::new([0.0, 1.0, 0.0], 0.0)
    }
}

// ============================================================================
// Sub-Emitter
// ============================================================================

/// Sub-emitter configuration
#[derive(Clone, Debug)]
pub struct SubEmitter {
    /// When to emit
    pub trigger: SubEmitterTrigger,
    /// Emitter to spawn
    pub emitter: EmitterHandle,
    /// Inherit color
    pub inherit_color: f32,
    /// Inherit size
    pub inherit_size: f32,
    /// Inherit rotation
    pub inherit_rotation: f32,
    /// Emit probability
    pub probability: f32,
}

impl SubEmitter {
    /// Creates new sub-emitter
    pub fn new(trigger: SubEmitterTrigger, emitter: EmitterHandle) -> Self {
        Self {
            trigger,
            emitter,
            inherit_color: 0.0,
            inherit_size: 0.0,
            inherit_rotation: 0.0,
            probability: 1.0,
        }
    }

    /// On death
    pub fn on_death(emitter: EmitterHandle) -> Self {
        Self::new(SubEmitterTrigger::Death, emitter)
    }

    /// On collision
    pub fn on_collision(emitter: EmitterHandle) -> Self {
        Self::new(SubEmitterTrigger::Collision, emitter)
    }

    /// With inherit
    pub fn with_inherit(mut self, color: f32, size: f32, rotation: f32) -> Self {
        self.inherit_color = color;
        self.inherit_size = size;
        self.inherit_rotation = rotation;
        self
    }
}

/// Sub-emitter trigger
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SubEmitterTrigger {
    /// On particle birth
    Birth = 0,
    /// On particle death
    Death = 1,
    /// On collision
    Collision = 2,
    /// Manual trigger
    Manual = 3,
}

// ============================================================================
// Particle Rendering
// ============================================================================

/// Particle render mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ParticleRenderMode {
    /// Billboard (always face camera)
    #[default]
    Billboard = 0,
    /// Stretched billboard (velocity aligned)
    StretchedBillboard = 1,
    /// Horizontal billboard
    HorizontalBillboard = 2,
    /// Vertical billboard
    VerticalBillboard = 3,
    /// Mesh particles
    Mesh = 4,
    /// None (GPU only)
    None = 5,
}

/// Particle renderer settings
#[derive(Clone, Debug)]
pub struct ParticleRendererSettings {
    /// Render mode
    pub render_mode: ParticleRenderMode,
    /// Material handle
    pub material: u64,
    /// Mesh handle (for mesh particles)
    pub mesh: u64,
    /// Sort mode
    pub sort_mode: ParticleSortMode,
    /// Min particle size
    pub min_particle_size: f32,
    /// Max particle size
    pub max_particle_size: f32,
    /// Velocity scale (for stretched)
    pub velocity_scale: f32,
    /// Length scale (for stretched)
    pub length_scale: f32,
    /// Cast shadows
    pub cast_shadows: bool,
    /// Receive shadows
    pub receive_shadows: bool,
}

impl ParticleRendererSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            render_mode: ParticleRenderMode::Billboard,
            material: 0,
            mesh: 0,
            sort_mode: ParticleSortMode::None,
            min_particle_size: 0.0,
            max_particle_size: 1000.0,
            velocity_scale: 1.0,
            length_scale: 1.0,
            cast_shadows: false,
            receive_shadows: true,
        }
    }

    /// With render mode
    pub fn with_render_mode(mut self, mode: ParticleRenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// With material
    pub fn with_material(mut self, material: u64) -> Self {
        self.material = material;
        self
    }

    /// With sort mode
    pub fn with_sort_mode(mut self, mode: ParticleSortMode) -> Self {
        self.sort_mode = mode;
        self
    }
}

impl Default for ParticleRendererSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Particle sort mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ParticleSortMode {
    /// No sorting
    #[default]
    None = 0,
    /// By distance (front to back)
    ByDistance = 1,
    /// Oldest in front
    OldestInFront = 2,
    /// Youngest in front
    YoungestInFront = 3,
    /// By depth
    ByDepth = 4,
}

// ============================================================================
// GPU Particle Data
// ============================================================================

/// GPU particle data structure
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GpuParticle {
    /// Position
    pub position: [f32; 3],
    /// Age (normalized 0-1)
    pub age: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Size
    pub size: f32,
    /// Color
    pub color: [f32; 4],
    /// Rotation (radians)
    pub rotation: f32,
    /// Angular velocity
    pub angular_velocity: f32,
    /// Random seed
    pub random_seed: f32,
    /// Flags
    pub flags: u32,
}

impl GpuParticle {
    /// Creates dead particle
    pub const fn dead() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            age: 1.0,
            velocity: [0.0, 0.0, 0.0],
            size: 0.0,
            color: [0.0, 0.0, 0.0, 0.0],
            rotation: 0.0,
            angular_velocity: 0.0,
            random_seed: 0.0,
            flags: 0,
        }
    }

    /// Is alive
    pub fn is_alive(&self) -> bool {
        self.age < 1.0
    }
}

impl Default for GpuParticle {
    fn default() -> Self {
        Self::dead()
    }
}

/// Particle draw indirect args
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ParticleDrawArgs {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl Default for ParticleDrawArgs {
    fn default() -> Self {
        Self {
            vertex_count: 6,
            instance_count: 0,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}
