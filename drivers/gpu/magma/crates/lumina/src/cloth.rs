//! Cloth Simulation Types for Lumina
//!
//! This module provides cloth simulation and rendering infrastructure
//! including constraint solvers, collision detection, and GPU simulation.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Cloth Handles
// ============================================================================

/// Cloth handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClothHandle(pub u64);

impl ClothHandle {
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

impl Default for ClothHandle {
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

impl Default for ClothMeshHandle {
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

impl Default for ClothColliderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Cloth Creation
// ============================================================================

/// Cloth create info
#[derive(Clone, Debug)]
pub struct ClothCreateInfo {
    /// Name
    pub name: String,
    /// Mesh
    pub mesh: ClothMeshHandle,
    /// Simulation settings
    pub simulation: ClothSimulationSettings,
    /// Material
    pub material: ClothMaterial,
    /// Colliders
    pub colliders: Vec<ClothColliderHandle>,
    /// Solver type
    pub solver: ClothSolverType,
    /// GPU simulation
    pub gpu_simulation: bool,
}

impl ClothCreateInfo {
    /// Creates info
    pub fn new(mesh: ClothMeshHandle) -> Self {
        Self {
            name: String::new(),
            mesh,
            simulation: ClothSimulationSettings::default(),
            material: ClothMaterial::default(),
            colliders: Vec::new(),
            solver: ClothSolverType::Pbd,
            gpu_simulation: true,
        }
    }

    /// Light cloth (silk, thin fabric)
    pub fn light(mesh: ClothMeshHandle) -> Self {
        Self {
            simulation: ClothSimulationSettings::silk(),
            material: ClothMaterial::silk(),
            ..Self::new(mesh)
        }
    }

    /// Heavy cloth (denim, canvas)
    pub fn heavy(mesh: ClothMeshHandle) -> Self {
        Self {
            simulation: ClothSimulationSettings::heavy(),
            material: ClothMaterial::denim(),
            ..Self::new(mesh)
        }
    }

    /// With simulation settings
    pub fn with_simulation(mut self, settings: ClothSimulationSettings) -> Self {
        self.simulation = settings;
        self
    }

    /// With material
    pub fn with_material(mut self, material: ClothMaterial) -> Self {
        self.material = material;
        self
    }

    /// Add collider
    pub fn with_collider(mut self, collider: ClothColliderHandle) -> Self {
        self.colliders.push(collider);
        self
    }

    /// Use CPU simulation
    pub fn cpu_simulation(mut self) -> Self {
        self.gpu_simulation = false;
        self
    }
}

impl Default for ClothCreateInfo {
    fn default() -> Self {
        Self::new(ClothMeshHandle::NULL)
    }
}

/// Cloth solver type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ClothSolverType {
    /// Position Based Dynamics
    #[default]
    Pbd = 0,
    /// XPBD (Extended PBD)
    Xpbd = 1,
    /// Projective Dynamics
    ProjectiveDynamics = 2,
    /// Finite Element Method
    Fem = 3,
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
    /// Triangles (indices)
    pub triangles: Vec<[u32; 3]>,
    /// Constraints (auto-generated if empty)
    pub constraints: Vec<ClothConstraint>,
    /// Pin constraints
    pub pins: Vec<PinConstraint>,
}

impl ClothMeshCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            vertices: Vec::new(),
            triangles: Vec::new(),
            constraints: Vec::new(),
            pins: Vec::new(),
        }
    }

    /// Creates a grid mesh
    pub fn grid(width: u32, height: u32, cell_size: f32) -> Self {
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();

        // Generate vertices
        for y in 0..=height {
            for x in 0..=width {
                vertices.push(ClothVertex {
                    position: [x as f32 * cell_size, 0.0, y as f32 * cell_size],
                    uv: [x as f32 / width as f32, y as f32 / height as f32],
                    inverse_mass: 1.0,
                });
            }
        }

        // Generate triangles
        let row_size = width + 1;
        for y in 0..height {
            for x in 0..width {
                let i0 = y * row_size + x;
                let i1 = i0 + 1;
                let i2 = i0 + row_size;
                let i3 = i2 + 1;

                triangles.push([i0, i1, i2]);
                triangles.push([i1, i3, i2]);
            }
        }

        Self {
            vertices,
            triangles,
            ..Self::new()
        }
    }

    /// With vertex
    pub fn with_vertex(mut self, vertex: ClothVertex) -> Self {
        self.vertices.push(vertex);
        self
    }

    /// With triangle
    pub fn with_triangle(mut self, indices: [u32; 3]) -> Self {
        self.triangles.push(indices);
        self
    }

    /// Pin top edge (for grid)
    pub fn pin_top_edge(mut self, width: u32) -> Self {
        for x in 0..=width {
            self.pins.push(PinConstraint {
                vertex_index: x,
                attachment: PinAttachment::Fixed,
            });
        }
        self
    }

    /// Vertex count
    pub fn vertex_count(&self) -> u32 {
        self.vertices.len() as u32
    }

    /// Triangle count
    pub fn triangle_count(&self) -> u32 {
        self.triangles.len() as u32
    }
}

impl Default for ClothMeshCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cloth vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClothVertex {
    /// Position
    pub position: [f32; 3],
    /// UV coordinates
    pub uv: [f32; 2],
    /// Inverse mass (0 = fixed)
    pub inverse_mass: f32,
}

impl ClothVertex {
    /// Creates vertex
    pub const fn new(position: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            uv,
            inverse_mass: 1.0,
        }
    }

    /// Fixed vertex
    pub const fn fixed(position: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            uv,
            inverse_mass: 0.0,
        }
    }

    /// With mass
    pub const fn with_mass(mut self, mass: f32) -> Self {
        self.inverse_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };
        self
    }
}

// ============================================================================
// Constraints
// ============================================================================

/// Cloth constraint
#[derive(Clone, Copy, Debug)]
pub struct ClothConstraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Vertex indices
    pub indices: [u32; 4],
    /// Rest length/angle
    pub rest_value: f32,
    /// Stiffness
    pub stiffness: f32,
}

impl ClothConstraint {
    /// Creates distance constraint
    pub fn distance(v0: u32, v1: u32, rest_length: f32, stiffness: f32) -> Self {
        Self {
            constraint_type: ConstraintType::Distance,
            indices: [v0, v1, 0, 0],
            rest_value: rest_length,
            stiffness,
        }
    }

    /// Creates bending constraint
    pub fn bending(v0: u32, v1: u32, v2: u32, v3: u32, rest_angle: f32, stiffness: f32) -> Self {
        Self {
            constraint_type: ConstraintType::Bending,
            indices: [v0, v1, v2, v3],
            rest_value: rest_angle,
            stiffness,
        }
    }

    /// Creates stretch constraint (isometric)
    pub fn stretch(v0: u32, v1: u32, v2: u32, stiffness: f32) -> Self {
        Self {
            constraint_type: ConstraintType::Stretch,
            indices: [v0, v1, v2, 0],
            rest_value: 0.0, // Computed from triangle
            stiffness,
        }
    }

    /// Creates shear constraint
    pub fn shear(v0: u32, v1: u32, v2: u32, v3: u32, stiffness: f32) -> Self {
        Self {
            constraint_type: ConstraintType::Shear,
            indices: [v0, v1, v2, v3],
            rest_value: 0.0,
            stiffness,
        }
    }
}

impl Default for ClothConstraint {
    fn default() -> Self {
        Self::distance(0, 0, 0.0, 1.0)
    }
}

/// Constraint type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ConstraintType {
    /// Distance constraint (edge)
    #[default]
    Distance = 0,
    /// Bending constraint
    Bending = 1,
    /// Stretch constraint (triangle)
    Stretch = 2,
    /// Shear constraint
    Shear = 3,
    /// Volume preservation
    Volume = 4,
}

/// Pin constraint
#[derive(Clone, Copy, Debug)]
pub struct PinConstraint {
    /// Vertex index
    pub vertex_index: u32,
    /// Attachment type
    pub attachment: PinAttachment,
}

impl PinConstraint {
    /// Creates fixed pin
    pub fn fixed(vertex_index: u32) -> Self {
        Self {
            vertex_index,
            attachment: PinAttachment::Fixed,
        }
    }

    /// Creates bone attachment
    pub fn bone(vertex_index: u32, bone_index: u32, offset: [f32; 3]) -> Self {
        Self {
            vertex_index,
            attachment: PinAttachment::Bone { bone_index, offset },
        }
    }
}

impl Default for PinConstraint {
    fn default() -> Self {
        Self::fixed(0)
    }
}

/// Pin attachment type
#[derive(Clone, Copy, Debug)]
pub enum PinAttachment {
    /// Fixed in world space
    Fixed,
    /// Attached to transform
    Transform { transform_id: u32 },
    /// Attached to bone
    Bone { bone_index: u32, offset: [f32; 3] },
}

impl Default for PinAttachment {
    fn default() -> Self {
        Self::Fixed
    }
}

// ============================================================================
// Simulation Settings
// ============================================================================

/// Cloth simulation settings
#[derive(Clone, Copy, Debug)]
pub struct ClothSimulationSettings {
    /// Gravity
    pub gravity: [f32; 3],
    /// Wind force
    pub wind: [f32; 3],
    /// Wind turbulence
    pub wind_turbulence: f32,
    /// Air resistance (drag)
    pub drag: f32,
    /// Global damping
    pub damping: f32,
    /// Friction
    pub friction: f32,
    /// Collision margin
    pub collision_margin: f32,
    /// Self-collision enabled
    pub self_collision: bool,
    /// Self-collision distance
    pub self_collision_distance: f32,
    /// Solver iterations
    pub iterations: u32,
    /// Substeps per frame
    pub substeps: u32,
}

impl ClothSimulationSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            wind: [0.0, 0.0, 0.0],
            wind_turbulence: 0.0,
            drag: 0.02,
            damping: 0.01,
            friction: 0.5,
            collision_margin: 0.01,
            self_collision: true,
            self_collision_distance: 0.02,
            iterations: 4,
            substeps: 2,
        }
    }

    /// Silk-like (light, flowing)
    pub fn silk() -> Self {
        Self {
            drag: 0.05,
            damping: 0.005,
            friction: 0.2,
            iterations: 8,
            substeps: 4,
            ..Self::new()
        }
    }

    /// Heavy fabric (denim, canvas)
    pub fn heavy() -> Self {
        Self {
            drag: 0.01,
            damping: 0.02,
            friction: 0.7,
            iterations: 4,
            substeps: 2,
            ..Self::new()
        }
    }

    /// Rubber-like
    pub fn rubber() -> Self {
        Self {
            drag: 0.03,
            damping: 0.1,
            friction: 0.9,
            self_collision: true,
            iterations: 8,
            substeps: 4,
            ..Self::new()
        }
    }

    /// With gravity
    pub fn with_gravity(mut self, gravity: [f32; 3]) -> Self {
        self.gravity = gravity;
        self
    }

    /// With wind
    pub fn with_wind(mut self, wind: [f32; 3], turbulence: f32) -> Self {
        self.wind = wind;
        self.wind_turbulence = turbulence;
        self
    }

    /// Without self-collision
    pub fn without_self_collision(mut self) -> Self {
        self.self_collision = false;
        self
    }
}

impl Default for ClothSimulationSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Cloth simulation GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClothSimGpuParams {
    /// Gravity
    pub gravity: [f32; 3],
    /// Delta time
    pub dt: f32,
    /// Wind
    pub wind: [f32; 3],
    /// Wind turbulence
    pub wind_turbulence: f32,
    /// Drag
    pub drag: f32,
    /// Damping
    pub damping: f32,
    /// Friction
    pub friction: f32,
    /// Iterations
    pub iterations: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Constraint count
    pub constraint_count: u32,
    /// Time
    pub time: f32,
    /// Padding
    pub _padding: f32,
}

// ============================================================================
// Cloth Material
// ============================================================================

/// Cloth material
#[derive(Clone, Copy, Debug)]
pub struct ClothMaterial {
    /// Stretch stiffness (warp)
    pub stretch_stiffness_warp: f32,
    /// Stretch stiffness (weft)
    pub stretch_stiffness_weft: f32,
    /// Shear stiffness
    pub shear_stiffness: f32,
    /// Bending stiffness
    pub bending_stiffness: f32,
    /// Density (kg/m²)
    pub density: f32,
    /// Thickness
    pub thickness: f32,
}

impl ClothMaterial {
    /// Creates material
    pub fn new() -> Self {
        Self {
            stretch_stiffness_warp: 1.0,
            stretch_stiffness_weft: 1.0,
            shear_stiffness: 0.5,
            bending_stiffness: 0.1,
            density: 0.2,
            thickness: 0.001,
        }
    }

    /// Silk
    pub fn silk() -> Self {
        Self {
            stretch_stiffness_warp: 0.8,
            stretch_stiffness_weft: 0.8,
            shear_stiffness: 0.2,
            bending_stiffness: 0.01,
            density: 0.05,
            thickness: 0.0002,
        }
    }

    /// Cotton
    pub fn cotton() -> Self {
        Self {
            stretch_stiffness_warp: 0.95,
            stretch_stiffness_weft: 0.9,
            shear_stiffness: 0.5,
            bending_stiffness: 0.1,
            density: 0.15,
            thickness: 0.0005,
        }
    }

    /// Denim
    pub fn denim() -> Self {
        Self {
            stretch_stiffness_warp: 1.0,
            stretch_stiffness_weft: 0.98,
            shear_stiffness: 0.8,
            bending_stiffness: 0.5,
            density: 0.4,
            thickness: 0.001,
        }
    }

    /// Leather
    pub fn leather() -> Self {
        Self {
            stretch_stiffness_warp: 1.0,
            stretch_stiffness_weft: 1.0,
            shear_stiffness: 0.9,
            bending_stiffness: 0.7,
            density: 0.8,
            thickness: 0.002,
        }
    }

    /// Rubber
    pub fn rubber() -> Self {
        Self {
            stretch_stiffness_warp: 0.3,
            stretch_stiffness_weft: 0.3,
            shear_stiffness: 0.2,
            bending_stiffness: 0.05,
            density: 1.2,
            thickness: 0.002,
        }
    }

    /// With bending stiffness
    pub fn with_bending(mut self, stiffness: f32) -> Self {
        self.bending_stiffness = stiffness;
        self
    }
}

impl Default for ClothMaterial {
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
    /// Shape
    pub shape: ColliderShape,
    /// Transform
    pub transform: [[f32; 4]; 4],
    /// Friction
    pub friction: f32,
}

impl ClothColliderCreateInfo {
    /// Creates info
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            name: String::new(),
            shape,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            friction: 0.5,
        }
    }

    /// Sphere collider
    pub fn sphere(center: [f32; 3], radius: f32) -> Self {
        let mut info = Self::new(ColliderShape::Sphere { radius });
        info.transform[0][3] = center[0];
        info.transform[1][3] = center[1];
        info.transform[2][3] = center[2];
        info
    }

    /// Capsule collider
    pub fn capsule(p0: [f32; 3], p1: [f32; 3], radius: f32) -> Self {
        Self::new(ColliderShape::Capsule { p0, p1, radius })
    }

    /// Plane collider
    pub fn plane(normal: [f32; 3], distance: f32) -> Self {
        Self::new(ColliderShape::Plane { normal, distance })
    }

    /// With friction
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }
}

impl Default for ClothColliderCreateInfo {
    fn default() -> Self {
        Self::sphere([0.0, 0.0, 0.0], 1.0)
    }
}

/// Collider shape
#[derive(Clone, Copy, Debug)]
pub enum ColliderShape {
    /// Sphere
    Sphere { radius: f32 },
    /// Capsule
    Capsule {
        p0: [f32; 3],
        p1: [f32; 3],
        radius: f32,
    },
    /// Box
    Box { half_extents: [f32; 3] },
    /// Plane
    Plane { normal: [f32; 3], distance: f32 },
    /// Mesh
    Mesh { mesh_id: u64 },
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Sphere { radius: 1.0 }
    }
}

/// Collider GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ColliderGpuData {
    /// Shape type
    pub shape_type: u32,
    /// Friction
    pub friction: f32,
    /// Radius (for sphere/capsule)
    pub radius: f32,
    /// Padding
    pub _padding: f32,
    /// Transform/data (depends on shape)
    pub data: [[f32; 4]; 3],
}

// ============================================================================
// Wind
// ============================================================================

/// Wind zone
#[derive(Clone, Copy, Debug)]
pub struct WindZone {
    /// Wind direction
    pub direction: [f32; 3],
    /// Wind strength
    pub strength: f32,
    /// Zone type
    pub zone_type: WindZoneType,
    /// Zone center (for spherical)
    pub center: [f32; 3],
    /// Zone radius (for spherical)
    pub radius: f32,
    /// Turbulence frequency
    pub turbulence_frequency: f32,
    /// Turbulence amplitude
    pub turbulence_amplitude: f32,
    /// Pulse magnitude
    pub pulse_magnitude: f32,
    /// Pulse frequency
    pub pulse_frequency: f32,
}

impl WindZone {
    /// Creates directional wind
    pub fn directional(direction: [f32; 3], strength: f32) -> Self {
        Self {
            direction,
            strength,
            zone_type: WindZoneType::Directional,
            center: [0.0, 0.0, 0.0],
            radius: 0.0,
            turbulence_frequency: 1.0,
            turbulence_amplitude: 0.2,
            pulse_magnitude: 0.0,
            pulse_frequency: 0.0,
        }
    }

    /// Creates spherical wind
    pub fn spherical(center: [f32; 3], radius: f32, strength: f32) -> Self {
        Self {
            direction: [0.0, 0.0, 0.0],
            strength,
            zone_type: WindZoneType::Spherical,
            center,
            radius,
            turbulence_frequency: 1.0,
            turbulence_amplitude: 0.2,
            pulse_magnitude: 0.0,
            pulse_frequency: 0.0,
        }
    }

    /// Light breeze
    pub fn light_breeze() -> Self {
        Self::directional([1.0, 0.0, 0.0], 2.0)
    }

    /// Strong wind
    pub fn strong() -> Self {
        Self {
            turbulence_amplitude: 0.5,
            ..Self::directional([1.0, 0.0, 0.0], 15.0)
        }
    }

    /// With turbulence
    pub fn with_turbulence(mut self, frequency: f32, amplitude: f32) -> Self {
        self.turbulence_frequency = frequency;
        self.turbulence_amplitude = amplitude;
        self
    }

    /// With pulse
    pub fn with_pulse(mut self, magnitude: f32, frequency: f32) -> Self {
        self.pulse_magnitude = magnitude;
        self.pulse_frequency = frequency;
        self
    }
}

impl Default for WindZone {
    fn default() -> Self {
        Self::light_breeze()
    }
}

/// Wind zone type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WindZoneType {
    /// Directional (infinite)
    #[default]
    Directional = 0,
    /// Spherical
    Spherical = 1,
    /// Cylindrical
    Cylindrical = 2,
}

// ============================================================================
// Statistics
// ============================================================================

/// Cloth statistics
#[derive(Clone, Debug, Default)]
pub struct ClothStats {
    /// Total cloth objects
    pub cloth_count: u32,
    /// Total vertices
    pub vertex_count: u32,
    /// Total constraints
    pub constraint_count: u32,
    /// Total colliders
    pub collider_count: u32,
    /// Simulation time (microseconds)
    pub simulation_time_us: u64,
    /// Collision time (microseconds)
    pub collision_time_us: u64,
    /// Self-collision time (microseconds)
    pub self_collision_time_us: u64,
}
