//! GPU Cloth Simulation System for Lumina
//!
//! This module provides GPU-accelerated cloth simulation with
//! Verlet integration, constraint solving, and collision detection.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Cloth System Handles
// ============================================================================

/// GPU cloth system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuClothSystemHandle(pub u64);

impl GpuClothSystemHandle {
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

impl Default for GpuClothSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Cloth mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClothMeshHandle(pub u64);

impl ClothMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ClothMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Cloth constraint handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClothConstraintHandle(pub u64);

impl ClothConstraintHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ClothConstraintHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Cloth collider handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClothColliderHandle(pub u64);

impl ClothColliderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ClothColliderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Cloth System Creation
// ============================================================================

/// GPU cloth system create info
#[derive(Clone, Debug)]
pub struct GpuClothSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max cloth meshes
    pub max_cloths: u32,
    /// Max total particles
    pub max_particles: u32,
    /// Max constraints
    pub max_constraints: u32,
    /// Max colliders
    pub max_colliders: u32,
    /// Features
    pub features: ClothFeatures,
    /// Solver iterations
    pub solver_iterations: u32,
}

impl GpuClothSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_cloths: 64,
            max_particles: 100000,
            max_constraints: 400000,
            max_colliders: 256,
            features: ClothFeatures::all(),
            solver_iterations: 4,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max cloths
    pub fn with_max_cloths(mut self, count: u32) -> Self {
        self.max_cloths = count;
        self
    }

    /// With max particles
    pub fn with_max_particles(mut self, count: u32) -> Self {
        self.max_particles = count;
        self
    }

    /// With max constraints
    pub fn with_max_constraints(mut self, count: u32) -> Self {
        self.max_constraints = count;
        self
    }

    /// With max colliders
    pub fn with_max_colliders(mut self, count: u32) -> Self {
        self.max_colliders = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ClothFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With solver iterations
    pub fn with_solver_iterations(mut self, iterations: u32) -> Self {
        self.solver_iterations = iterations;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_particles(500000)
            .with_max_constraints(2000000)
            .with_solver_iterations(8)
    }

    /// Mobile
    pub fn mobile() -> Self {
        Self::new()
            .with_max_cloths(16)
            .with_max_particles(10000)
            .with_max_constraints(40000)
            .with_solver_iterations(2)
    }
}

impl Default for GpuClothSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Cloth features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ClothFeatures: u32 {
        /// None
        const NONE = 0;
        /// Verlet integration
        const VERLET = 1 << 0;
        /// Self collision
        const SELF_COLLISION = 1 << 1;
        /// External collision
        const COLLISION = 1 << 2;
        /// Wind
        const WIND = 1 << 3;
        /// Tearing
        const TEARING = 1 << 4;
        /// Sleeping
        const SLEEPING = 1 << 5;
        /// LOD
        const LOD = 1 << 6;
        /// GPU compute
        const GPU_COMPUTE = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for ClothFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Cloth Mesh
// ============================================================================

/// Cloth mesh create info
#[derive(Clone, Debug)]
pub struct ClothMeshCreateInfo {
    /// Name
    pub name: String,
    /// Vertices
    pub vertices: Vec<ClothVertex>,
    /// Indices
    pub indices: Vec<u32>,
    /// Constraint groups
    pub constraints: Vec<ClothConstraintGroup>,
    /// Material
    pub material: ClothMaterial,
    /// Simulation settings
    pub simulation: ClothSimulationSettings,
}

impl ClothMeshCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            constraints: Vec::new(),
            material: ClothMaterial::default(),
            simulation: ClothSimulationSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add vertex
    pub fn add_vertex(mut self, vertex: ClothVertex) -> Self {
        self.vertices.push(vertex);
        self
    }

    /// With vertices
    pub fn with_vertices(mut self, vertices: Vec<ClothVertex>) -> Self {
        self.vertices = vertices;
        self
    }

    /// With indices
    pub fn with_indices(mut self, indices: Vec<u32>) -> Self {
        self.indices = indices;
        self
    }

    /// Add constraint group
    pub fn add_constraints(mut self, group: ClothConstraintGroup) -> Self {
        self.constraints.push(group);
        self
    }

    /// With material
    pub fn with_material(mut self, material: ClothMaterial) -> Self {
        self.material = material;
        self
    }

    /// With simulation settings
    pub fn with_simulation(mut self, settings: ClothSimulationSettings) -> Self {
        self.simulation = settings;
        self
    }

    /// Grid cloth (width x height particles)
    pub fn grid(width: u32, height: u32, spacing: f32) -> Self {
        let mut vertices = Vec::with_capacity((width * height) as usize);
        let mut indices = Vec::new();

        for y in 0..height {
            for x in 0..width {
                let fixed = y == 0; // Pin top row
                vertices.push(ClothVertex {
                    position: [x as f32 * spacing, -(y as f32) * spacing, 0.0],
                    uv: [x as f32 / (width - 1) as f32, y as f32 / (height - 1) as f32],
                    inv_mass: if fixed { 0.0 } else { 1.0 },
                    flags: if fixed { ClothVertexFlags::PINNED } else { ClothVertexFlags::empty() },
                });

                if x < width - 1 && y < height - 1 {
                    let i = y * width + x;
                    indices.push(i);
                    indices.push(i + 1);
                    indices.push(i + width);
                    indices.push(i + 1);
                    indices.push(i + width + 1);
                    indices.push(i + width);
                }
            }
        }

        Self::new()
            .with_name("GridCloth")
            .with_vertices(vertices)
            .with_indices(indices)
            .add_constraints(ClothConstraintGroup::structural(spacing))
            .add_constraints(ClothConstraintGroup::shear(spacing * 1.414))
            .add_constraints(ClothConstraintGroup::bending(spacing * 2.0))
    }
}

impl Default for ClothMeshCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cloth vertex
#[derive(Clone, Copy, Debug)]
pub struct ClothVertex {
    /// Position
    pub position: [f32; 3],
    /// UV coordinates
    pub uv: [f32; 2],
    /// Inverse mass (0 = pinned)
    pub inv_mass: f32,
    /// Vertex flags
    pub flags: ClothVertexFlags,
}

impl ClothVertex {
    /// Creates new vertex
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            uv: [0.0, 0.0],
            inv_mass: 1.0,
            flags: ClothVertexFlags::empty(),
        }
    }

    /// With UV
    pub const fn with_uv(mut self, uv: [f32; 2]) -> Self {
        self.uv = uv;
        self
    }

    /// Pinned vertex
    pub const fn pinned(mut self) -> Self {
        self.inv_mass = 0.0;
        self.flags = ClothVertexFlags::PINNED;
        self
    }

    /// With mass
    pub const fn with_mass(mut self, mass: f32) -> Self {
        self.inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        self
    }
}

impl Default for ClothVertex {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}

bitflags::bitflags! {
    /// Cloth vertex flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ClothVertexFlags: u32 {
        /// None
        const NONE = 0;
        /// Pinned (immovable)
        const PINNED = 1 << 0;
        /// Can tear
        const TEARABLE = 1 << 1;
        /// Ignore collision
        const NO_COLLISION = 1 << 2;
    }
}

// ============================================================================
// Cloth Constraints
// ============================================================================

/// Cloth constraint group
#[derive(Clone, Debug)]
pub struct ClothConstraintGroup {
    /// Constraint type
    pub constraint_type: ClothConstraintType,
    /// Rest length
    pub rest_length: f32,
    /// Stiffness
    pub stiffness: f32,
    /// Compression stiffness
    pub compression_stiffness: f32,
    /// Stretch limit
    pub stretch_limit: f32,
}

impl ClothConstraintGroup {
    /// Creates new group
    pub const fn new(constraint_type: ClothConstraintType) -> Self {
        Self {
            constraint_type,
            rest_length: 1.0,
            stiffness: 1.0,
            compression_stiffness: 1.0,
            stretch_limit: 0.1,
        }
    }

    /// With rest length
    pub const fn with_rest_length(mut self, length: f32) -> Self {
        self.rest_length = length;
        self
    }

    /// With stiffness
    pub const fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness;
        self
    }

    /// With stretch limit
    pub const fn with_stretch_limit(mut self, limit: f32) -> Self {
        self.stretch_limit = limit;
        self
    }

    /// Structural constraints
    pub const fn structural(rest_length: f32) -> Self {
        Self {
            constraint_type: ClothConstraintType::Structural,
            rest_length,
            stiffness: 1.0,
            compression_stiffness: 1.0,
            stretch_limit: 0.1,
        }
    }

    /// Shear constraints
    pub const fn shear(rest_length: f32) -> Self {
        Self {
            constraint_type: ClothConstraintType::Shear,
            rest_length,
            stiffness: 0.8,
            compression_stiffness: 0.8,
            stretch_limit: 0.15,
        }
    }

    /// Bending constraints
    pub const fn bending(rest_length: f32) -> Self {
        Self {
            constraint_type: ClothConstraintType::Bending,
            rest_length,
            stiffness: 0.5,
            compression_stiffness: 0.5,
            stretch_limit: 0.2,
        }
    }
}

impl Default for ClothConstraintGroup {
    fn default() -> Self {
        Self::structural(1.0)
    }
}

/// Cloth constraint type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ClothConstraintType {
    /// Structural (horizontal/vertical)
    #[default]
    Structural = 0,
    /// Shear (diagonal)
    Shear = 1,
    /// Bending
    Bending = 2,
}

// ============================================================================
// Cloth Material
// ============================================================================

/// Cloth material
#[derive(Clone, Copy, Debug)]
pub struct ClothMaterial {
    /// Thickness
    pub thickness: f32,
    /// Friction
    pub friction: f32,
    /// Air resistance
    pub air_resistance: f32,
    /// Self collision distance
    pub self_collision_distance: f32,
    /// Tear threshold
    pub tear_threshold: f32,
}

impl ClothMaterial {
    /// Default material
    pub const fn new() -> Self {
        Self {
            thickness: 0.01,
            friction: 0.5,
            air_resistance: 0.1,
            self_collision_distance: 0.05,
            tear_threshold: 10.0,
        }
    }

    /// Silk material
    pub const fn silk() -> Self {
        Self {
            thickness: 0.002,
            friction: 0.2,
            air_resistance: 0.2,
            self_collision_distance: 0.02,
            tear_threshold: 5.0,
        }
    }

    /// Cotton material
    pub const fn cotton() -> Self {
        Self {
            thickness: 0.005,
            friction: 0.6,
            air_resistance: 0.15,
            self_collision_distance: 0.03,
            tear_threshold: 15.0,
        }
    }

    /// Denim material
    pub const fn denim() -> Self {
        Self {
            thickness: 0.02,
            friction: 0.7,
            air_resistance: 0.05,
            self_collision_distance: 0.05,
            tear_threshold: 30.0,
        }
    }

    /// Leather material
    pub const fn leather() -> Self {
        Self {
            thickness: 0.03,
            friction: 0.8,
            air_resistance: 0.02,
            self_collision_distance: 0.06,
            tear_threshold: 50.0,
        }
    }

    /// Flag material
    pub const fn flag() -> Self {
        Self {
            thickness: 0.003,
            friction: 0.3,
            air_resistance: 0.3,
            self_collision_distance: 0.02,
            tear_threshold: 8.0,
        }
    }
}

impl Default for ClothMaterial {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Simulation Settings
// ============================================================================

/// Cloth simulation settings
#[derive(Clone, Copy, Debug)]
pub struct ClothSimulationSettings {
    /// Time step
    pub time_step: f32,
    /// Sub-steps
    pub sub_steps: u32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Damping
    pub damping: f32,
    /// Constraint iterations
    pub constraint_iterations: u32,
    /// Collision iterations
    pub collision_iterations: u32,
    /// Sleep threshold
    pub sleep_threshold: f32,
}

impl ClothSimulationSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            time_step: 1.0 / 60.0,
            sub_steps: 4,
            gravity: [0.0, -9.81, 0.0],
            damping: 0.99,
            constraint_iterations: 4,
            collision_iterations: 2,
            sleep_threshold: 0.01,
        }
    }

    /// High quality
    pub const fn high_quality() -> Self {
        Self {
            time_step: 1.0 / 120.0,
            sub_steps: 8,
            gravity: [0.0, -9.81, 0.0],
            damping: 0.995,
            constraint_iterations: 8,
            collision_iterations: 4,
            sleep_threshold: 0.005,
        }
    }

    /// Low quality
    pub const fn low_quality() -> Self {
        Self {
            time_step: 1.0 / 30.0,
            sub_steps: 2,
            gravity: [0.0, -9.81, 0.0],
            damping: 0.98,
            constraint_iterations: 2,
            collision_iterations: 1,
            sleep_threshold: 0.05,
        }
    }

    /// With gravity
    pub const fn with_gravity(mut self, gravity: [f32; 3]) -> Self {
        self.gravity = gravity;
        self
    }

    /// With damping
    pub const fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }
}

impl Default for ClothSimulationSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Colliders
// ============================================================================

/// Cloth collider create info
#[derive(Clone, Debug)]
pub struct ClothColliderCreateInfo {
    /// Name
    pub name: String,
    /// Collider shape
    pub shape: ClothColliderShape,
    /// Transform
    pub transform: ClothTransform,
    /// Friction
    pub friction: f32,
    /// Enabled
    pub enabled: bool,
}

impl ClothColliderCreateInfo {
    /// Creates new info
    pub fn new(shape: ClothColliderShape) -> Self {
        Self {
            name: String::new(),
            shape,
            transform: ClothTransform::identity(),
            friction: 0.5,
            enabled: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With transform
    pub fn with_transform(mut self, transform: ClothTransform) -> Self {
        self.transform = transform;
        self
    }

    /// With friction
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    /// Sphere collider
    pub fn sphere(center: [f32; 3], radius: f32) -> Self {
        Self::new(ClothColliderShape::Sphere { center, radius })
    }

    /// Capsule collider
    pub fn capsule(start: [f32; 3], end: [f32; 3], radius: f32) -> Self {
        Self::new(ClothColliderShape::Capsule { start, end, radius })
    }

    /// Plane collider
    pub fn plane(normal: [f32; 3], distance: f32) -> Self {
        Self::new(ClothColliderShape::Plane { normal, distance })
    }

    /// Box collider
    pub fn cube(center: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self::new(ClothColliderShape::Box { center, half_extents })
    }
}

impl Default for ClothColliderCreateInfo {
    fn default() -> Self {
        Self::new(ClothColliderShape::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 0.5,
        })
    }
}

/// Cloth collider shape
#[derive(Clone, Copy, Debug)]
pub enum ClothColliderShape {
    /// Sphere
    Sphere { center: [f32; 3], radius: f32 },
    /// Capsule
    Capsule { start: [f32; 3], end: [f32; 3], radius: f32 },
    /// Infinite plane
    Plane { normal: [f32; 3], distance: f32 },
    /// Box
    Box { center: [f32; 3], half_extents: [f32; 3] },
}

impl Default for ClothColliderShape {
    fn default() -> Self {
        Self::Sphere {
            center: [0.0, 0.0, 0.0],
            radius: 0.5,
        }
    }
}

/// Cloth transform
#[derive(Clone, Copy, Debug)]
pub struct ClothTransform {
    /// Position
    pub position: [f32; 3],
    /// Rotation quaternion
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl ClothTransform {
    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// With position
    pub const fn with_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// With scale
    pub const fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }
}

impl Default for ClothTransform {
    fn default() -> Self {
        Self::identity()
    }
}

// ============================================================================
// Wind
// ============================================================================

/// Wind settings
#[derive(Clone, Copy, Debug)]
pub struct ClothWindSettings {
    /// Wind direction
    pub direction: [f32; 3],
    /// Wind strength
    pub strength: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Gust frequency
    pub gust_frequency: f32,
    /// Gust strength
    pub gust_strength: f32,
}

impl ClothWindSettings {
    /// No wind
    pub const fn none() -> Self {
        Self {
            direction: [0.0, 0.0, 0.0],
            strength: 0.0,
            turbulence: 0.0,
            gust_frequency: 0.0,
            gust_strength: 0.0,
        }
    }

    /// Light breeze
    pub const fn light_breeze() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 2.0,
            turbulence: 0.2,
            gust_frequency: 0.5,
            gust_strength: 1.0,
        }
    }

    /// Strong wind
    pub const fn strong() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 10.0,
            turbulence: 0.5,
            gust_frequency: 1.0,
            gust_strength: 5.0,
        }
    }

    /// Storm
    pub const fn storm() -> Self {
        Self {
            direction: [1.0, 0.0, 0.2],
            strength: 25.0,
            turbulence: 1.0,
            gust_frequency: 2.0,
            gust_strength: 15.0,
        }
    }

    /// With direction
    pub const fn with_direction(mut self, direction: [f32; 3]) -> Self {
        self.direction = direction;
        self
    }

    /// With strength
    pub const fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

impl Default for ClothWindSettings {
    fn default() -> Self {
        Self::light_breeze()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU cloth particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuClothParticle {
    /// Position
    pub position: [f32; 3],
    /// Inverse mass
    pub inv_mass: f32,
    /// Previous position
    pub prev_position: [f32; 3],
    /// Flags
    pub flags: u32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Padding
    pub _pad: f32,
}

/// GPU cloth constraint
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuClothConstraint {
    /// Particle A index
    pub particle_a: u32,
    /// Particle B index
    pub particle_b: u32,
    /// Rest length
    pub rest_length: f32,
    /// Stiffness
    pub stiffness: f32,
}

/// GPU cloth constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuClothConstants {
    /// Time
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Damping
    pub damping: f32,
    /// Particle count
    pub particle_count: u32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Constraint count
    pub constraint_count: u32,
    /// Wind direction
    pub wind_direction: [f32; 3],
    /// Wind strength
    pub wind_strength: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Padding
    pub _pad: [f32; 3],
}

/// GPU cloth collider
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuClothCollider {
    /// Shape type
    pub shape_type: u32,
    /// Friction
    pub friction: f32,
    /// Padding
    pub _pad: [f32; 2],
    /// Shape data (interpretation depends on type)
    pub shape_data: [[f32; 4]; 2],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU cloth statistics
#[derive(Clone, Debug, Default)]
pub struct GpuClothStats {
    /// Active cloth meshes
    pub active_cloths: u32,
    /// Total particles
    pub total_particles: u32,
    /// Total constraints
    pub total_constraints: u32,
    /// Collision pairs
    pub collision_pairs: u32,
    /// Sleeping cloths
    pub sleeping_cloths: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}
