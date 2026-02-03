//! Physics Types for Lumina
//!
//! This module provides physics and collision detection types
//! for integration with GPU-accelerated physics systems.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Physics Handles
// ============================================================================

/// Physics world handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicsWorldHandle(pub u64);

impl PhysicsWorldHandle {
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

impl Default for PhysicsWorldHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Rigid body handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RigidBodyHandle(pub u64);

impl RigidBodyHandle {
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

impl Default for RigidBodyHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Collider handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColliderHandle(pub u64);

impl ColliderHandle {
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

impl Default for ColliderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Joint handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct JointHandle(pub u64);

impl JointHandle {
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

impl Default for JointHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Physics World
// ============================================================================

/// Physics world create info
#[derive(Clone, Debug)]
pub struct PhysicsWorldCreateInfo {
    /// Gravity
    pub gravity: [f32; 3],
    /// Time step
    pub time_step: f32,
    /// Substeps
    pub substeps: u32,
    /// Solver iterations
    pub solver_iterations: u32,
    /// Enable sleeping
    pub allow_sleeping: bool,
    /// GPU acceleration
    pub gpu_acceleration: bool,
}

impl PhysicsWorldCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            time_step: 1.0 / 60.0,
            substeps: 4,
            solver_iterations: 8,
            allow_sleeping: true,
            gpu_acceleration: true,
        }
    }

    /// With gravity
    pub fn with_gravity(mut self, x: f32, y: f32, z: f32) -> Self {
        self.gravity = [x, y, z];
        self
    }

    /// With time step
    pub fn with_time_step(mut self, dt: f32) -> Self {
        self.time_step = dt;
        self
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            substeps: 8,
            solver_iterations: 16,
            ..Self::new()
        }
    }

    /// Performance
    pub fn performance() -> Self {
        Self {
            substeps: 2,
            solver_iterations: 4,
            ..Self::new()
        }
    }

    /// Zero gravity
    pub fn zero_gravity() -> Self {
        Self {
            gravity: [0.0, 0.0, 0.0],
            ..Self::new()
        }
    }
}

impl Default for PhysicsWorldCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rigid Body
// ============================================================================

/// Rigid body create info
#[derive(Clone, Debug)]
pub struct RigidBodyCreateInfo {
    /// Body type
    pub body_type: RigidBodyType,
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Linear velocity
    pub linear_velocity: [f32; 3],
    /// Angular velocity
    pub angular_velocity: [f32; 3],
    /// Linear damping
    pub linear_damping: f32,
    /// Angular damping
    pub angular_damping: f32,
    /// Gravity scale
    pub gravity_scale: f32,
    /// Can sleep
    pub can_sleep: bool,
    /// CCD enabled
    pub ccd_enabled: bool,
}

impl RigidBodyCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            linear_velocity: [0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
            linear_damping: 0.0,
            angular_damping: 0.05,
            gravity_scale: 1.0,
            can_sleep: true,
            ccd_enabled: false,
        }
    }

    /// Dynamic body
    pub fn dynamic() -> Self {
        Self {
            body_type: RigidBodyType::Dynamic,
            ..Self::new()
        }
    }

    /// Static body
    pub fn fixed() -> Self {
        Self {
            body_type: RigidBodyType::Static,
            ..Self::new()
        }
    }

    /// Kinematic body
    pub fn kinematic() -> Self {
        Self {
            body_type: RigidBodyType::Kinematic,
            ..Self::new()
        }
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With rotation (quaternion)
    pub fn with_rotation(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.rotation = [x, y, z, w];
        self
    }

    /// With velocity
    pub fn with_velocity(mut self, x: f32, y: f32, z: f32) -> Self {
        self.linear_velocity = [x, y, z];
        self
    }

    /// With damping
    pub fn with_damping(mut self, linear: f32, angular: f32) -> Self {
        self.linear_damping = linear;
        self.angular_damping = angular;
        self
    }

    /// With CCD
    pub fn with_ccd(mut self) -> Self {
        self.ccd_enabled = true;
        self
    }
}

impl Default for RigidBodyCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Rigid body type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RigidBodyType {
    /// Static (immovable)
    Static = 0,
    /// Dynamic (affected by forces)
    #[default]
    Dynamic = 1,
    /// Kinematic (user-controlled)
    Kinematic = 2,
}

/// Rigid body state (GPU data)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RigidBodyState {
    /// Position
    pub position: [f32; 3],
    /// Body type
    pub body_type: u32,
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Linear velocity
    pub linear_velocity: [f32; 3],
    /// Mass
    pub inv_mass: f32,
    /// Angular velocity
    pub angular_velocity: [f32; 3],
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Colliders
// ============================================================================

/// Collider create info
#[derive(Clone, Debug)]
pub struct ColliderCreateInfo {
    /// Shape
    pub shape: ColliderShape,
    /// Local position offset
    pub offset: [f32; 3],
    /// Local rotation offset
    pub rotation: [f32; 4],
    /// Density
    pub density: f32,
    /// Friction
    pub friction: f32,
    /// Restitution (bounciness)
    pub restitution: f32,
    /// Is sensor (no collision response)
    pub is_sensor: bool,
    /// Collision groups
    pub collision_groups: CollisionGroups,
}

impl ColliderCreateInfo {
    /// Creates info
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            offset: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            density: 1.0,
            friction: 0.5,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: CollisionGroups::all(),
        }
    }

    /// Box collider
    pub fn cuboid(half_x: f32, half_y: f32, half_z: f32) -> Self {
        Self::new(ColliderShape::Cuboid {
            half_extents: [half_x, half_y, half_z],
        })
    }

    /// Sphere collider
    pub fn sphere(radius: f32) -> Self {
        Self::new(ColliderShape::Sphere { radius })
    }

    /// Capsule collider
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::new(ColliderShape::Capsule {
            half_height,
            radius,
        })
    }

    /// Cylinder collider
    pub fn cylinder(half_height: f32, radius: f32) -> Self {
        Self::new(ColliderShape::Cylinder {
            half_height,
            radius,
        })
    }

    /// With friction
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    /// With restitution
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// As sensor
    pub fn as_sensor(mut self) -> Self {
        self.is_sensor = true;
        self
    }

    /// With offset
    pub fn with_offset(mut self, x: f32, y: f32, z: f32) -> Self {
        self.offset = [x, y, z];
        self
    }
}

impl Default for ColliderCreateInfo {
    fn default() -> Self {
        Self::sphere(0.5)
    }
}

/// Collider shape
#[derive(Clone, Debug)]
pub enum ColliderShape {
    /// Sphere
    Sphere {
        /// Radius
        radius: f32,
    },
    /// Box
    Cuboid {
        /// Half extents
        half_extents: [f32; 3],
    },
    /// Capsule
    Capsule {
        /// Half height (not including caps)
        half_height: f32,
        /// Radius
        radius: f32,
    },
    /// Cylinder
    Cylinder {
        /// Half height
        half_height: f32,
        /// Radius
        radius: f32,
    },
    /// Cone
    Cone {
        /// Half height
        half_height: f32,
        /// Radius
        radius: f32,
    },
    /// Convex hull
    ConvexHull {
        /// Vertices
        vertices: Vec<[f32; 3]>,
    },
    /// Triangle mesh
    TriMesh {
        /// Vertices
        vertices: Vec<[f32; 3]>,
        /// Indices
        indices: Vec<u32>,
    },
    /// Heightfield
    Heightfield {
        /// Heights
        heights: Vec<f32>,
        /// Columns
        columns: u32,
        /// Rows
        rows: u32,
        /// Scale
        scale: [f32; 3],
    },
}

impl ColliderShape {
    /// Shape type ID
    pub fn type_id(&self) -> u32 {
        match self {
            Self::Sphere { .. } => 0,
            Self::Cuboid { .. } => 1,
            Self::Capsule { .. } => 2,
            Self::Cylinder { .. } => 3,
            Self::Cone { .. } => 4,
            Self::ConvexHull { .. } => 5,
            Self::TriMesh { .. } => 6,
            Self::Heightfield { .. } => 7,
        }
    }
}

/// Collision groups
#[derive(Clone, Copy, Debug)]
pub struct CollisionGroups {
    /// Membership groups
    pub memberships: u32,
    /// Filter groups
    pub filter: u32,
}

impl CollisionGroups {
    /// All groups
    pub const fn all() -> Self {
        Self {
            memberships: u32::MAX,
            filter: u32::MAX,
        }
    }

    /// No groups
    pub const fn none() -> Self {
        Self {
            memberships: 0,
            filter: 0,
        }
    }

    /// Single group
    pub const fn group(group: u32) -> Self {
        let mask = 1 << group;
        Self {
            memberships: mask,
            filter: mask,
        }
    }

    /// Custom groups
    pub const fn custom(memberships: u32, filter: u32) -> Self {
        Self {
            memberships,
            filter,
        }
    }

    /// Can collide with another
    pub const fn can_collide_with(&self, other: &Self) -> bool {
        (self.memberships & other.filter) != 0 && (other.memberships & self.filter) != 0
    }
}

impl Default for CollisionGroups {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Joints
// ============================================================================

/// Joint create info
#[derive(Clone, Debug)]
pub struct JointCreateInfo {
    /// Joint type
    pub joint_type: JointType,
    /// Body A
    pub body_a: RigidBodyHandle,
    /// Body B
    pub body_b: RigidBodyHandle,
    /// Local anchor A
    pub anchor_a: [f32; 3],
    /// Local anchor B
    pub anchor_b: [f32; 3],
    /// Axis (for hinges, sliders)
    pub axis: [f32; 3],
}

impl JointCreateInfo {
    /// Creates info
    pub fn new(
        joint_type: JointType,
        body_a: RigidBodyHandle,
        body_b: RigidBodyHandle,
    ) -> Self {
        Self {
            joint_type,
            body_a,
            body_b,
            anchor_a: [0.0, 0.0, 0.0],
            anchor_b: [0.0, 0.0, 0.0],
            axis: [0.0, 1.0, 0.0],
        }
    }

    /// Fixed joint
    pub fn fixed(body_a: RigidBodyHandle, body_b: RigidBodyHandle) -> Self {
        Self::new(JointType::Fixed, body_a, body_b)
    }

    /// Ball (spherical) joint
    pub fn ball(body_a: RigidBodyHandle, body_b: RigidBodyHandle) -> Self {
        Self::new(JointType::Ball, body_a, body_b)
    }

    /// Hinge (revolute) joint
    pub fn hinge(body_a: RigidBodyHandle, body_b: RigidBodyHandle, axis: [f32; 3]) -> Self {
        let mut info = Self::new(JointType::Revolute, body_a, body_b);
        info.axis = axis;
        info
    }

    /// Slider (prismatic) joint
    pub fn slider(body_a: RigidBodyHandle, body_b: RigidBodyHandle, axis: [f32; 3]) -> Self {
        let mut info = Self::new(JointType::Prismatic, body_a, body_b);
        info.axis = axis;
        info
    }

    /// With anchors
    pub fn with_anchors(mut self, anchor_a: [f32; 3], anchor_b: [f32; 3]) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }
}

/// Joint type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum JointType {
    /// Fixed (no relative motion)
    #[default]
    Fixed = 0,
    /// Ball/Spherical (rotation only)
    Ball = 1,
    /// Revolute/Hinge (single axis rotation)
    Revolute = 2,
    /// Prismatic/Slider (single axis translation)
    Prismatic = 3,
    /// Generic 6-DOF
    Generic = 4,
}

// ============================================================================
// Collision Detection
// ============================================================================

/// Contact point
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ContactPoint {
    /// World position
    pub position: [f32; 3],
    /// Penetration depth
    pub depth: f32,
    /// Normal (from B to A)
    pub normal: [f32; 3],
    /// Impulse applied
    pub impulse: f32,
}

/// Contact manifold
#[derive(Clone, Debug)]
pub struct ContactManifold {
    /// Body A
    pub body_a: RigidBodyHandle,
    /// Body B
    pub body_b: RigidBodyHandle,
    /// Collider A
    pub collider_a: ColliderHandle,
    /// Collider B
    pub collider_b: ColliderHandle,
    /// Contact points
    pub points: Vec<ContactPoint>,
}

impl ContactManifold {
    /// Creates manifold
    pub fn new(
        body_a: RigidBodyHandle,
        body_b: RigidBodyHandle,
        collider_a: ColliderHandle,
        collider_b: ColliderHandle,
    ) -> Self {
        Self {
            body_a,
            body_b,
            collider_a,
            collider_b,
            points: Vec::new(),
        }
    }

    /// Point count
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Has contacts
    pub fn has_contacts(&self) -> bool {
        !self.points.is_empty()
    }
}

/// Ray cast result
#[derive(Clone, Copy, Debug)]
pub struct RayCastResult {
    /// Hit body
    pub body: RigidBodyHandle,
    /// Hit collider
    pub collider: ColliderHandle,
    /// Hit point
    pub point: [f32; 3],
    /// Hit normal
    pub normal: [f32; 3],
    /// Distance
    pub distance: f32,
}

/// Shape cast result
#[derive(Clone, Copy, Debug)]
pub struct ShapeCastResult {
    /// Hit body
    pub body: RigidBodyHandle,
    /// Hit collider
    pub collider: ColliderHandle,
    /// Time of impact (0-1)
    pub toi: f32,
    /// Contact point
    pub point: [f32; 3],
    /// Contact normal
    pub normal: [f32; 3],
}

// ============================================================================
// GPU Physics
// ============================================================================

/// GPU physics settings
#[derive(Clone, Debug)]
pub struct GpuPhysicsSettings {
    /// Enable GPU physics
    pub enabled: bool,
    /// Max bodies on GPU
    pub max_bodies: u32,
    /// Max contacts
    pub max_contacts: u32,
    /// Broadphase method
    pub broadphase: BroadphaseMethod,
}

impl GpuPhysicsSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_bodies: 65536,
            max_contacts: 262144,
            broadphase: BroadphaseMethod::Sap,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With max bodies
    pub fn with_max_bodies(mut self, count: u32) -> Self {
        self.max_bodies = count;
        self
    }
}

impl Default for GpuPhysicsSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Broadphase method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BroadphaseMethod {
    /// Sweep and Prune (SAP)
    #[default]
    Sap = 0,
    /// Bounding Volume Hierarchy
    Bvh = 1,
    /// Grid
    Grid = 2,
}

/// Physics GPU buffer
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhysicsGpuBuffer {
    /// Body count
    pub body_count: u32,
    /// Contact count
    pub contact_count: u32,
    /// Delta time
    pub dt: f32,
    /// Iteration
    pub iteration: u32,
    /// Gravity
    pub gravity: [f32; 3],
    /// Padding
    pub _padding: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Physics statistics
#[derive(Clone, Debug, Default)]
pub struct PhysicsStats {
    /// Total bodies
    pub body_count: u32,
    /// Active bodies
    pub active_bodies: u32,
    /// Sleeping bodies
    pub sleeping_bodies: u32,
    /// Colliders
    pub collider_count: u32,
    /// Contacts
    pub contact_count: u32,
    /// Joints
    pub joint_count: u32,
    /// Simulation time (microseconds)
    pub simulation_time_us: u64,
    /// Broadphase time
    pub broadphase_time_us: u64,
    /// Narrowphase time
    pub narrowphase_time_us: u64,
    /// Solver time
    pub solver_time_us: u64,
}
