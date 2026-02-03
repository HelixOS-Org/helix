//! Scene Graph for Lumina
//!
//! This module provides hierarchical scene management including
//! nodes, transforms, components, and scene traversal.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Scene Handle
// ============================================================================

/// Scene handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SceneHandle(pub u64);

impl SceneHandle {
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

impl Default for SceneHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Node handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeHandle(pub u64);

impl NodeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Root node handle
    pub const ROOT: Self = Self(1);

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

    /// Is root
    #[inline]
    pub const fn is_root(&self) -> bool {
        self.0 == 1
    }
}

impl Default for NodeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Transform
// ============================================================================

/// 3D transform component
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Transform {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl Transform {
    /// Identity transform
    pub const IDENTITY: Self = Self {
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
        scale: [1.0, 1.0, 1.0],
    };

    /// Creates new transform
    #[inline]
    pub const fn new() -> Self {
        Self::IDENTITY
    }

    /// Creates from position
    pub const fn from_position(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// Creates from position and scale
    pub const fn from_position_scale(x: f32, y: f32, z: f32, scale: f32) -> Self {
        Self {
            position: [x, y, z],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [scale, scale, scale],
        }
    }

    /// With position
    pub const fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With rotation quaternion
    pub const fn with_rotation(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.rotation = [x, y, z, w];
        self
    }

    /// With scale
    pub const fn with_scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.scale = [x, y, z];
        self
    }

    /// With uniform scale
    pub const fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = [scale, scale, scale];
        self
    }

    /// Set rotation from Euler angles (radians)
    pub fn set_rotation_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();

        self.rotation = [
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy,
            cr * cp * cy + sr * sp * sy,
        ];
    }

    /// Get rotation as Euler angles (radians)
    pub fn rotation_euler(&self) -> [f32; 3] {
        let [x, y, z, w] = self.rotation;

        let sinr_cosp = 2.0 * (w * x + y * z);
        let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
        let roll = sinr_cosp.atan2(cosr_cosp);

        let sinp = 2.0 * (w * y - z * x);
        let pitch = if sinp.abs() >= 1.0 {
            core::f32::consts::FRAC_PI_2.copysign(sinp)
        } else {
            sinp.asin()
        };

        let siny_cosp = 2.0 * (w * z + x * y);
        let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
        let yaw = siny_cosp.atan2(cosy_cosp);

        [pitch, yaw, roll]
    }

    /// Translate by offset
    pub fn translate(&mut self, dx: f32, dy: f32, dz: f32) {
        self.position[0] += dx;
        self.position[1] += dy;
        self.position[2] += dz;
    }

    /// Get forward vector
    pub fn forward(&self) -> [f32; 3] {
        let [x, y, z, w] = self.rotation;
        [
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ]
    }

    /// Get right vector
    pub fn right(&self) -> [f32; 3] {
        let [x, y, z, w] = self.rotation;
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ]
    }

    /// Get up vector
    pub fn up(&self) -> [f32; 3] {
        let [x, y, z, w] = self.rotation;
        [
            2.0 * (x * y - w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + w * x),
        ]
    }

    /// Compute model matrix
    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        let [qx, qy, qz, qw] = self.rotation;
        let [sx, sy, sz] = self.scale;
        let [px, py, pz] = self.position;

        let xx = qx * qx;
        let yy = qy * qy;
        let zz = qz * qz;
        let xy = qx * qy;
        let xz = qx * qz;
        let yz = qy * qz;
        let wx = qw * qx;
        let wy = qw * qy;
        let wz = qw * qz;

        [
            [
                sx * (1.0 - 2.0 * (yy + zz)),
                sx * 2.0 * (xy + wz),
                sx * 2.0 * (xz - wy),
                0.0,
            ],
            [
                sy * 2.0 * (xy - wz),
                sy * (1.0 - 2.0 * (xx + zz)),
                sy * 2.0 * (yz + wx),
                0.0,
            ],
            [
                sz * 2.0 * (xz + wy),
                sz * 2.0 * (yz - wx),
                sz * (1.0 - 2.0 * (xx + yy)),
                0.0,
            ],
            [px, py, pz, 1.0],
        ]
    }

    /// Interpolate between transforms
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: [
                self.position[0] + (other.position[0] - self.position[0]) * t,
                self.position[1] + (other.position[1] - self.position[1]) * t,
                self.position[2] + (other.position[2] - self.position[2]) * t,
            ],
            rotation: slerp(self.rotation, other.rotation, t),
            scale: [
                self.scale[0] + (other.scale[0] - self.scale[0]) * t,
                self.scale[1] + (other.scale[1] - self.scale[1]) * t,
                self.scale[2] + (other.scale[2] - self.scale[2]) * t,
            ],
        }
    }

    /// Combine with parent transform
    pub fn combine(&self, parent: &Transform) -> Transform {
        // Combine scales
        let new_scale = [
            self.scale[0] * parent.scale[0],
            self.scale[1] * parent.scale[1],
            self.scale[2] * parent.scale[2],
        ];

        // Combine rotations (quaternion multiplication)
        let [px, py, pz, pw] = parent.rotation;
        let [cx, cy, cz, cw] = self.rotation;
        let new_rotation = [
            pw * cx + px * cw + py * cz - pz * cy,
            pw * cy - px * cz + py * cw + pz * cx,
            pw * cz + px * cy - py * cx + pz * cw,
            pw * cw - px * cx - py * cy - pz * cz,
        ];

        // Rotate child position by parent rotation, then add parent position
        let rotated = rotate_vector(self.position, parent.rotation);
        let new_position = [
            parent.position[0] + rotated[0] * parent.scale[0],
            parent.position[1] + rotated[1] * parent.scale[1],
            parent.position[2] + rotated[2] * parent.scale[2],
        ];

        Transform {
            position: new_position,
            rotation: new_rotation,
            scale: new_scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Spherical linear interpolation for quaternions
fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

    let mut b = b;
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }

    if dot > 0.9995 {
        // Linear interpolation for very close quaternions
        let result = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ];
        let len = (result[0] * result[0]
            + result[1] * result[1]
            + result[2] * result[2]
            + result[3] * result[3])
            .sqrt();
        [
            result[0] / len,
            result[1] / len,
            result[2] / len,
            result[3] / len,
        ]
    } else {
        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        [
            a[0] * s0 + b[0] * s1,
            a[1] * s0 + b[1] * s1,
            a[2] * s0 + b[2] * s1,
            a[3] * s0 + b[3] * s1,
        ]
    }
}

/// Rotate vector by quaternion
fn rotate_vector(v: [f32; 3], q: [f32; 4]) -> [f32; 3] {
    let [qx, qy, qz, qw] = q;
    let [vx, vy, vz] = v;

    // q * v * q^-1
    let ux = qy * vz - qz * vy;
    let uy = qz * vx - qx * vz;
    let uz = qx * vy - qy * vx;

    let uux = qy * uz - qz * uy;
    let uuy = qz * ux - qx * uz;
    let uuz = qx * uy - qy * ux;

    [
        vx + 2.0 * (qw * ux + uux),
        vy + 2.0 * (qw * uy + uuy),
        vz + 2.0 * (qw * uz + uuz),
    ]
}

// ============================================================================
// Scene Node
// ============================================================================

/// Scene node
#[derive(Clone, Debug)]
pub struct SceneNode {
    /// Node handle
    pub handle: NodeHandle,
    /// Node name
    pub name: String,
    /// Local transform
    pub local_transform: Transform,
    /// World transform (cached)
    pub world_transform: Transform,
    /// Parent handle
    pub parent: NodeHandle,
    /// Children handles
    pub children: Vec<NodeHandle>,
    /// Node flags
    pub flags: NodeFlags,
    /// Node layer
    pub layer: u32,
    /// Components
    pub components: Vec<Component>,
}

impl SceneNode {
    /// Creates new node
    pub fn new(handle: NodeHandle, name: &str) -> Self {
        Self {
            handle,
            name: String::from(name),
            local_transform: Transform::IDENTITY,
            world_transform: Transform::IDENTITY,
            parent: NodeHandle::NULL,
            children: Vec::new(),
            flags: NodeFlags::DEFAULT,
            layer: 0,
            components: Vec::new(),
        }
    }

    /// Creates root node
    pub fn root() -> Self {
        Self::new(NodeHandle::ROOT, "Root")
    }

    /// With transform
    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self.world_transform = transform;
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.local_transform.position = [x, y, z];
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: NodeFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With layer
    pub fn with_layer(mut self, layer: u32) -> Self {
        self.layer = layer;
        self
    }

    /// Is visible
    pub fn is_visible(&self) -> bool {
        self.flags.contains(NodeFlags::VISIBLE)
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags.contains(NodeFlags::ENABLED)
    }

    /// Is static
    pub fn is_static(&self) -> bool {
        self.flags.contains(NodeFlags::STATIC)
    }

    /// Add child
    pub fn add_child(&mut self, child: NodeHandle) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    /// Remove child
    pub fn remove_child(&mut self, child: NodeHandle) {
        self.children.retain(|&c| c != child);
    }

    /// Add component
    pub fn add_component(&mut self, component: Component) {
        self.components.push(component);
    }

    /// Get component by type
    pub fn get_component(&self, component_type: ComponentType) -> Option<&Component> {
        self.components.iter().find(|c| c.component_type == component_type)
    }

    /// Get component mutable
    pub fn get_component_mut(&mut self, component_type: ComponentType) -> Option<&mut Component> {
        self.components
            .iter_mut()
            .find(|c| c.component_type == component_type)
    }

    /// Has component
    pub fn has_component(&self, component_type: ComponentType) -> bool {
        self.components.iter().any(|c| c.component_type == component_type)
    }
}

// ============================================================================
// Node Flags
// ============================================================================

/// Node flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeFlags(pub u32);

impl NodeFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Visible
    pub const VISIBLE: Self = Self(1 << 0);
    /// Enabled
    pub const ENABLED: Self = Self(1 << 1);
    /// Static (doesn't move)
    pub const STATIC: Self = Self(1 << 2);
    /// Cast shadows
    pub const CAST_SHADOW: Self = Self(1 << 3);
    /// Receive shadows
    pub const RECEIVE_SHADOW: Self = Self(1 << 4);
    /// Culling enabled
    pub const CULLING: Self = Self(1 << 5);
    /// Debug visible
    pub const DEBUG_VISIBLE: Self = Self(1 << 6);
    /// Selected
    pub const SELECTED: Self = Self(1 << 7);
    /// Highlighted
    pub const HIGHLIGHTED: Self = Self(1 << 8);
    /// Default flags
    pub const DEFAULT: Self = Self(
        Self::VISIBLE.0 | Self::ENABLED.0 | Self::CAST_SHADOW.0 | Self::RECEIVE_SHADOW.0 | Self::CULLING.0,
    );

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

    /// Difference
    #[inline]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl Default for NodeFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl core::ops::BitOr for NodeFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Component
// ============================================================================

/// Component attached to node
#[derive(Clone, Debug)]
pub struct Component {
    /// Component type
    pub component_type: ComponentType,
    /// Component data (handle to actual data)
    pub data_handle: u64,
    /// Component flags
    pub flags: ComponentFlags,
}

impl Component {
    /// Creates new component
    pub fn new(component_type: ComponentType, data_handle: u64) -> Self {
        Self {
            component_type,
            data_handle,
            flags: ComponentFlags::ENABLED,
        }
    }

    /// Mesh component
    pub fn mesh(mesh_handle: u64) -> Self {
        Self::new(ComponentType::Mesh, mesh_handle)
    }

    /// Camera component
    pub fn camera(camera_handle: u64) -> Self {
        Self::new(ComponentType::Camera, camera_handle)
    }

    /// Light component
    pub fn light(light_handle: u64) -> Self {
        Self::new(ComponentType::Light, light_handle)
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.flags.contains(ComponentFlags::ENABLED)
    }
}

/// Component type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ComponentType {
    /// Mesh renderer
    Mesh = 0,
    /// Camera
    Camera = 1,
    /// Light
    Light = 2,
    /// Collider
    Collider = 3,
    /// Rigidbody
    Rigidbody = 4,
    /// Audio source
    AudioSource = 5,
    /// Audio listener
    AudioListener = 6,
    /// Particle system
    ParticleSystem = 7,
    /// Animation
    Animation = 8,
    /// Script
    Script = 9,
    /// Custom
    Custom = 255,
}

/// Component flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ComponentFlags(pub u32);

impl ComponentFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Enabled
    pub const ENABLED: Self = Self(1 << 0);
    /// Needs update
    pub const NEEDS_UPDATE: Self = Self(1 << 1);
    /// Serializable
    pub const SERIALIZABLE: Self = Self(1 << 2);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Scene
// ============================================================================

/// Scene containing nodes
#[derive(Debug)]
pub struct Scene {
    /// Scene handle
    pub handle: SceneHandle,
    /// Scene name
    pub name: String,
    /// Root node
    pub root: NodeHandle,
    /// All nodes
    pub nodes: Vec<SceneNode>,
    /// Next node ID
    next_node_id: u64,
    /// Dirty flags
    pub dirty: SceneDirtyFlags,
}

impl Scene {
    /// Creates new scene
    pub fn new(handle: SceneHandle, name: &str) -> Self {
        let root = SceneNode::root();
        Self {
            handle,
            name: String::from(name),
            root: NodeHandle::ROOT,
            nodes: alloc::vec![root],
            next_node_id: 2,
            dirty: SceneDirtyFlags::ALL,
        }
    }

    /// Create node
    pub fn create_node(&mut self, name: &str) -> NodeHandle {
        let handle = NodeHandle::new(self.next_node_id);
        self.next_node_id += 1;

        let node = SceneNode::new(handle, name);
        self.nodes.push(node);
        self.dirty = self.dirty.union(SceneDirtyFlags::HIERARCHY);

        handle
    }

    /// Create child node
    pub fn create_child(&mut self, parent: NodeHandle, name: &str) -> NodeHandle {
        let child = self.create_node(name);
        self.set_parent(child, parent);
        child
    }

    /// Get node
    pub fn get_node(&self, handle: NodeHandle) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.handle == handle)
    }

    /// Get node mutable
    pub fn get_node_mut(&mut self, handle: NodeHandle) -> Option<&mut SceneNode> {
        self.nodes.iter_mut().find(|n| n.handle == handle)
    }

    /// Set parent
    pub fn set_parent(&mut self, node: NodeHandle, parent: NodeHandle) {
        // Remove from old parent
        if let Some(old_parent_handle) = self.get_node(node).map(|n| n.parent) {
            if let Some(old_parent) = self.get_node_mut(old_parent_handle) {
                old_parent.remove_child(node);
            }
        }

        // Set new parent
        if let Some(child) = self.get_node_mut(node) {
            child.parent = parent;
        }

        // Add to new parent's children
        if let Some(new_parent) = self.get_node_mut(parent) {
            new_parent.add_child(node);
        }

        self.dirty = self.dirty.union(SceneDirtyFlags::HIERARCHY | SceneDirtyFlags::TRANSFORMS);
    }

    /// Remove node
    pub fn remove_node(&mut self, handle: NodeHandle) {
        if handle.is_root() {
            return; // Cannot remove root
        }

        // Get children to remove
        let children: Vec<NodeHandle> = self
            .get_node(handle)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        // Remove children recursively
        for child in children {
            self.remove_node(child);
        }

        // Remove from parent
        if let Some(parent_handle) = self.get_node(handle).map(|n| n.parent) {
            if let Some(parent) = self.get_node_mut(parent_handle) {
                parent.remove_child(handle);
            }
        }

        // Remove node
        self.nodes.retain(|n| n.handle != handle);
        self.dirty = self.dirty.union(SceneDirtyFlags::HIERARCHY);
    }

    /// Update world transforms
    pub fn update_transforms(&mut self) {
        if !self.dirty.contains(SceneDirtyFlags::TRANSFORMS) {
            return;
        }

        self.update_node_transform(self.root, Transform::IDENTITY);
        self.dirty = self.dirty.difference(SceneDirtyFlags::TRANSFORMS);
    }

    fn update_node_transform(&mut self, handle: NodeHandle, parent_world: Transform) {
        // Get local transform and children
        let (local, children) = {
            if let Some(node) = self.get_node(handle) {
                (node.local_transform, node.children.clone())
            } else {
                return;
            }
        };

        // Compute world transform
        let world = local.combine(&parent_world);

        // Update world transform
        if let Some(node) = self.get_node_mut(handle) {
            node.world_transform = world;
        }

        // Update children
        for child in children {
            self.update_node_transform(child, world);
        }
    }

    /// Find node by name
    pub fn find_node(&self, name: &str) -> Option<NodeHandle> {
        self.nodes.iter().find(|n| n.name == name).map(|n| n.handle)
    }

    /// Find nodes with component type
    pub fn find_nodes_with_component(&self, component_type: ComponentType) -> Vec<NodeHandle> {
        self.nodes
            .iter()
            .filter(|n| n.has_component(component_type))
            .map(|n| n.handle)
            .collect()
    }

    /// Get all visible nodes
    pub fn visible_nodes(&self) -> Vec<NodeHandle> {
        self.nodes
            .iter()
            .filter(|n| n.is_visible() && n.is_enabled())
            .map(|n| n.handle)
            .collect()
    }

    /// Traverse depth first
    pub fn traverse_depth_first<F>(&self, mut visitor: F)
    where
        F: FnMut(&SceneNode, u32),
    {
        self.traverse_node(self.root, 0, &mut visitor);
    }

    fn traverse_node<F>(&self, handle: NodeHandle, depth: u32, visitor: &mut F)
    where
        F: FnMut(&SceneNode, u32),
    {
        if let Some(node) = self.get_node(handle) {
            visitor(node, depth);
            for &child in &node.children {
                self.traverse_node(child, depth + 1, visitor);
            }
        }
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Scene dirty flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SceneDirtyFlags(pub u32);

impl SceneDirtyFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Hierarchy changed
    pub const HIERARCHY: Self = Self(1 << 0);
    /// Transforms changed
    pub const TRANSFORMS: Self = Self(1 << 1);
    /// Components changed
    pub const COMPONENTS: Self = Self(1 << 2);
    /// All dirty
    pub const ALL: Self = Self(0x7);

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

    /// Difference
    #[inline]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

// ============================================================================
// Scene Manager
// ============================================================================

/// Scene manager
#[derive(Debug, Default)]
pub struct SceneManager {
    /// All scenes
    pub scenes: Vec<Scene>,
    /// Active scene
    pub active_scene: SceneHandle,
    /// Next scene ID
    next_scene_id: u64,
}

impl SceneManager {
    /// Creates new manager
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            active_scene: SceneHandle::NULL,
            next_scene_id: 1,
        }
    }

    /// Create scene
    pub fn create_scene(&mut self, name: &str) -> SceneHandle {
        let handle = SceneHandle::new(self.next_scene_id);
        self.next_scene_id += 1;

        let scene = Scene::new(handle, name);
        self.scenes.push(scene);

        if self.active_scene.is_null() {
            self.active_scene = handle;
        }

        handle
    }

    /// Get scene
    pub fn get_scene(&self, handle: SceneHandle) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.handle == handle)
    }

    /// Get scene mutable
    pub fn get_scene_mut(&mut self, handle: SceneHandle) -> Option<&mut Scene> {
        self.scenes.iter_mut().find(|s| s.handle == handle)
    }

    /// Get active scene
    pub fn active(&self) -> Option<&Scene> {
        self.get_scene(self.active_scene)
    }

    /// Get active scene mutable
    pub fn active_mut(&mut self) -> Option<&mut Scene> {
        self.get_scene_mut(self.active_scene)
    }

    /// Set active scene
    pub fn set_active(&mut self, handle: SceneHandle) {
        self.active_scene = handle;
    }

    /// Remove scene
    pub fn remove_scene(&mut self, handle: SceneHandle) {
        self.scenes.retain(|s| s.handle != handle);
        if self.active_scene == handle {
            self.active_scene = self.scenes.first().map(|s| s.handle).unwrap_or(SceneHandle::NULL);
        }
    }

    /// Scene count
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }
}

// ============================================================================
// Transform Hierarchy Builder
// ============================================================================

/// Transform hierarchy builder for easier scene setup
pub struct HierarchyBuilder<'a> {
    scene: &'a mut Scene,
    current: NodeHandle,
    stack: Vec<NodeHandle>,
}

impl<'a> HierarchyBuilder<'a> {
    /// Creates new builder
    pub fn new(scene: &'a mut Scene) -> Self {
        let root = scene.root;
        Self {
            scene,
            current: root,
            stack: Vec::new(),
        }
    }

    /// Add child node and enter it
    pub fn push_node(mut self, name: &str) -> Self {
        self.stack.push(self.current);
        self.current = self.scene.create_child(self.current, name);
        self
    }

    /// Add child node with transform and enter it
    pub fn push_node_with_transform(mut self, name: &str, transform: Transform) -> Self {
        self.stack.push(self.current);
        let node = self.scene.create_child(self.current, name);
        if let Some(n) = self.scene.get_node_mut(node) {
            n.local_transform = transform;
        }
        self.current = node;
        self
    }

    /// Go back to parent
    pub fn pop(mut self) -> Self {
        if let Some(parent) = self.stack.pop() {
            self.current = parent;
        }
        self
    }

    /// Add component to current node
    pub fn with_component(self, component: Component) -> Self {
        if let Some(node) = self.scene.get_node_mut(self.current) {
            node.add_component(component);
        }
        self
    }

    /// Set transform of current node
    pub fn with_transform(self, transform: Transform) -> Self {
        if let Some(node) = self.scene.get_node_mut(self.current) {
            node.local_transform = transform;
        }
        self
    }

    /// Set position of current node
    pub fn with_position(self, x: f32, y: f32, z: f32) -> Self {
        if let Some(node) = self.scene.get_node_mut(self.current) {
            node.local_transform.position = [x, y, z];
        }
        self
    }

    /// Set flags of current node
    pub fn with_flags(self, flags: NodeFlags) -> Self {
        if let Some(node) = self.scene.get_node_mut(self.current) {
            node.flags = flags;
        }
        self
    }

    /// Get current node handle
    pub fn current(&self) -> NodeHandle {
        self.current
    }

    /// Finish building
    pub fn finish(self) -> NodeHandle {
        self.current
    }
}
