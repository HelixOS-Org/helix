//! # Transform System
//!
//! Hierarchical transform management.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::ecs::{Entity, World};

/// Transform component
#[derive(Debug, Clone, Default)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // Quaternion (x, y, z, w)
    pub scale: [f32; 3],

    // Cached world transform
    world_matrix: [[f32; 4]; 4],
    dirty: bool,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            world_matrix: identity(),
            dirty: true,
        }
    }

    pub fn from_position(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    pub fn from_position_rotation(position: [f32; 3], rotation: [f32; 4]) -> Self {
        Self {
            position,
            rotation,
            ..Self::new()
        }
    }

    /// Get local matrix
    pub fn local_matrix(&self) -> [[f32; 4]; 4] {
        let (x, y, z, w) = (
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
            self.rotation[3],
        );
        let (sx, sy, sz) = (self.scale[0], self.scale[1], self.scale[2]);

        [
            [
                (1.0 - 2.0 * (y * y + z * z)) * sx,
                2.0 * (x * y - z * w) * sx,
                2.0 * (x * z + y * w) * sx,
                0.0,
            ],
            [
                2.0 * (x * y + z * w) * sy,
                (1.0 - 2.0 * (x * x + z * z)) * sy,
                2.0 * (y * z - x * w) * sy,
                0.0,
            ],
            [
                2.0 * (x * z - y * w) * sz,
                2.0 * (y * z + x * w) * sz,
                (1.0 - 2.0 * (x * x + y * y)) * sz,
                0.0,
            ],
            [self.position[0], self.position[1], self.position[2], 1.0],
        ]
    }

    /// Get world matrix (cached)
    pub fn world_matrix(&self) -> [[f32; 4]; 4] {
        self.world_matrix
    }

    /// Update world matrix from parent
    pub fn update_world_matrix(&mut self, parent: Option<&Transform>) {
        let local = self.local_matrix();

        self.world_matrix = match parent {
            Some(p) => mul_mat4(p.world_matrix, local),
            None => local,
        };

        self.dirty = false;
    }

    /// Get world position
    pub fn world_position(&self) -> [f32; 3] {
        [
            self.world_matrix[3][0],
            self.world_matrix[3][1],
            self.world_matrix[3][2],
        ]
    }

    /// Get forward direction
    pub fn forward(&self) -> [f32; 3] {
        normalize([
            self.world_matrix[2][0],
            self.world_matrix[2][1],
            self.world_matrix[2][2],
        ])
    }

    /// Get right direction
    pub fn right(&self) -> [f32; 3] {
        normalize([
            self.world_matrix[0][0],
            self.world_matrix[0][1],
            self.world_matrix[0][2],
        ])
    }

    /// Get up direction
    pub fn up(&self) -> [f32; 3] {
        normalize([
            self.world_matrix[1][0],
            self.world_matrix[1][1],
            self.world_matrix[1][2],
        ])
    }

    /// Translate
    pub fn translate(&mut self, delta: [f32; 3]) {
        self.position[0] += delta[0];
        self.position[1] += delta[1];
        self.position[2] += delta[2];
        self.dirty = true;
    }

    /// Rotate by euler angles (radians)
    pub fn rotate_euler(&mut self, euler: [f32; 3]) {
        let q = euler_to_quaternion(euler);
        self.rotation = mul_quaternion(self.rotation, q);
        self.dirty = true;
    }

    /// Look at target
    pub fn look_at(&mut self, target: [f32; 3], up: [f32; 3]) {
        let forward = normalize([
            target[0] - self.position[0],
            target[1] - self.position[1],
            target[2] - self.position[2],
        ]);

        let right = normalize(cross(up, forward));
        let up = cross(forward, right);

        // Convert rotation matrix to quaternion
        self.rotation =
            matrix_to_quaternion([[right[0], right[1], right[2]], [up[0], up[1], up[2]], [
                forward[0], forward[1], forward[2],
            ]]);

        self.dirty = true;
    }

    /// Check if dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

/// Parent component
#[derive(Debug, Clone)]
pub struct Parent {
    pub entity: Entity,
}

/// Children component
#[derive(Debug, Clone, Default)]
pub struct Children {
    pub entities: Vec<Entity>,
}

/// Transform system
pub struct TransformSystem {
    hierarchy: BTreeMap<Entity, Vec<Entity>>,
    roots: Vec<Entity>,
}

impl TransformSystem {
    pub fn new() -> Self {
        Self {
            hierarchy: BTreeMap::new(),
            roots: Vec::new(),
        }
    }

    /// Build hierarchy from Parent components
    pub fn build_hierarchy(&mut self, world: &World) {
        self.hierarchy.clear();
        self.roots.clear();

        // Find all entities with transforms
        let mut has_parent = alloc::collections::BTreeSet::new();

        for (entity, _) in world.query::<Transform>() {
            if let Some(parent) = world.get_component::<Parent>(entity) {
                has_parent.insert(entity);

                self.hierarchy
                    .entry(parent.entity)
                    .or_insert_with(Vec::new)
                    .push(entity);
            }
        }

        // Find roots (entities with transforms but no parent)
        for (entity, _) in world.query::<Transform>() {
            if !has_parent.contains(&entity) {
                self.roots.push(entity);
            }
        }
    }

    /// Update transforms hierarchically
    pub fn update(&mut self, world: &mut World) {
        // Update roots first
        for &root in &self.roots.clone() {
            self.update_entity(world, root, None);
        }
    }

    fn update_entity(
        &self,
        world: &mut World,
        entity: Entity,
        parent_transform: Option<Transform>,
    ) {
        let transform = {
            if let Some(t) = world.get_component_mut::<Transform>(entity) {
                t.update_world_matrix(parent_transform.as_ref());
                t.clone()
            } else {
                return;
            }
        };

        // Update children
        if let Some(children) = self.hierarchy.get(&entity) {
            for &child in children {
                self.update_entity(world, child, Some(transform.clone()));
            }
        }
    }
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self::new()
    }
}

// Math helpers

fn identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn euler_to_quaternion(euler: [f32; 3]) -> [f32; 4] {
    let (sx, cx) = (euler[0] * 0.5).sin_cos();
    let (sy, cy) = (euler[1] * 0.5).sin_cos();
    let (sz, cz) = (euler[2] * 0.5).sin_cos();

    [
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz,
    ]
}

fn mul_quaternion(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn matrix_to_quaternion(m: [[f32; 3]; 3]) -> [f32; 4] {
    let trace = m[0][0] + m[1][1] + m[2][2];

    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [
            (m[2][1] - m[1][2]) / s,
            (m[0][2] - m[2][0]) / s,
            (m[1][0] - m[0][1]) / s,
            0.25 * s,
        ]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [
            0.25 * s,
            (m[0][1] + m[1][0]) / s,
            (m[0][2] + m[2][0]) / s,
            (m[2][1] - m[1][2]) / s,
        ]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [
            (m[0][1] + m[1][0]) / s,
            0.25 * s,
            (m[1][2] + m[2][1]) / s,
            (m[0][2] - m[2][0]) / s,
        ]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [
            (m[0][2] + m[2][0]) / s,
            (m[1][2] + m[2][1]) / s,
            0.25 * s,
            (m[1][0] - m[0][1]) / s,
        ]
    }
}
