//! # Culling System
//!
//! Frustum and occlusion culling.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use super::ecs::{Entity, World};
use super::transform::Transform;

/// Culling system
pub struct CullingSystem {
    visible_entities: BTreeSet<Entity>,
    config: CullingConfig,
    stats: CullingStats,
}

impl CullingSystem {
    pub fn new() -> Self {
        Self {
            visible_entities: BTreeSet::new(),
            config: CullingConfig::default(),
            stats: CullingStats::default(),
        }
    }

    /// Perform frustum culling
    pub fn perform_culling(
        &mut self,
        world: &World,
        frustum_planes: &[lumina_3d::camera::Plane; 6],
    ) {
        self.visible_entities.clear();
        self.stats = CullingStats::default();

        for (entity, transform) in world.query::<Transform>() {
            self.stats.total_objects += 1;

            // Get bounds component or use default
            let bounds = world
                .get_component::<BoundsComponent>(entity)
                .map(|b| b.bounds)
                .unwrap_or_else(|| Bounds::sphere(transform.world_position(), 1.0));

            // Transform bounds to world space
            let world_bounds = bounds.transform(transform.world_matrix());

            // Frustum test
            if self.test_frustum(&world_bounds, frustum_planes) {
                self.visible_entities.insert(entity);
            } else {
                self.stats.frustum_culled += 1;
            }
        }

        self.stats.visible_objects = self.visible_entities.len() as u32;
    }

    fn test_frustum(&self, bounds: &Bounds, planes: &[lumina_3d::camera::Plane; 6]) -> bool {
        match bounds {
            Bounds::Sphere { center, radius } => {
                for plane in planes {
                    let distance = plane.distance_to_point(*center);
                    if distance < -*radius {
                        return false;
                    }
                }
                true
            },
            Bounds::Aabb { min, max } => {
                for plane in planes {
                    // Test closest point to plane
                    let p = [
                        if plane.normal[0] >= 0.0 {
                            max[0]
                        } else {
                            min[0]
                        },
                        if plane.normal[1] >= 0.0 {
                            max[1]
                        } else {
                            min[1]
                        },
                        if plane.normal[2] >= 0.0 {
                            max[2]
                        } else {
                            min[2]
                        },
                    ];

                    if plane.distance_to_point(p) < 0.0 {
                        return false;
                    }
                }
                true
            },
            Bounds::Obb {
                center,
                half_extents,
                rotation: _,
            } => {
                // Approximate with sphere for now
                let radius =
                    (half_extents[0].powi(2) + half_extents[1].powi(2) + half_extents[2].powi(2))
                        .sqrt();
                for plane in planes {
                    if plane.distance_to_point(*center) < -radius {
                        return false;
                    }
                }
                true
            },
        }
    }

    /// Check if entity is visible
    pub fn is_visible(&self, entity: Entity) -> bool {
        self.visible_entities.contains(&entity)
    }

    /// Get visible entity count
    pub fn visible_count(&self) -> usize {
        self.visible_entities.len()
    }

    /// Get culling statistics
    pub fn stats(&self) -> &CullingStats {
        &self.stats
    }
}

impl Default for CullingSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Culling configuration
#[derive(Debug, Clone)]
pub struct CullingConfig {
    pub frustum_culling: bool,
    pub occlusion_culling: bool,
    pub small_object_culling: bool,
    pub small_object_threshold: f32,
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            frustum_culling: true,
            occlusion_culling: false,
            small_object_culling: true,
            small_object_threshold: 1.0,
        }
    }
}

/// Culling statistics
#[derive(Debug, Clone, Default)]
pub struct CullingStats {
    pub total_objects: u32,
    pub visible_objects: u32,
    pub frustum_culled: u32,
    pub occlusion_culled: u32,
    pub small_culled: u32,
}

/// Bounds component
#[derive(Debug, Clone)]
pub struct BoundsComponent {
    pub bounds: Bounds,
}

/// Bounding volume
#[derive(Debug, Clone)]
pub enum Bounds {
    Sphere {
        center: [f32; 3],
        radius: f32,
    },
    Aabb {
        min: [f32; 3],
        max: [f32; 3],
    },
    Obb {
        center: [f32; 3],
        half_extents: [f32; 3],
        rotation: [f32; 4],
    },
}

impl Bounds {
    pub fn sphere(center: [f32; 3], radius: f32) -> Self {
        Bounds::Sphere { center, radius }
    }

    pub fn aabb(min: [f32; 3], max: [f32; 3]) -> Self {
        Bounds::Aabb { min, max }
    }

    pub fn transform(&self, matrix: [[f32; 4]; 4]) -> Self {
        match self {
            Bounds::Sphere { center, radius } => {
                let new_center = transform_point(*center, matrix);
                // Scale radius by maximum scale factor
                let scale = ((matrix[0][0].powi(2) + matrix[0][1].powi(2) + matrix[0][2].powi(2))
                    .sqrt())
                .max((matrix[1][0].powi(2) + matrix[1][1].powi(2) + matrix[1][2].powi(2)).sqrt())
                .max((matrix[2][0].powi(2) + matrix[2][1].powi(2) + matrix[2][2].powi(2)).sqrt());

                Bounds::Sphere {
                    center: new_center,
                    radius: radius * scale,
                }
            },
            Bounds::Aabb { min, max } => {
                // Transform all 8 corners and create new AABB
                let corners = [
                    [min[0], min[1], min[2]],
                    [max[0], min[1], min[2]],
                    [min[0], max[1], min[2]],
                    [max[0], max[1], min[2]],
                    [min[0], min[1], max[2]],
                    [max[0], min[1], max[2]],
                    [min[0], max[1], max[2]],
                    [max[0], max[1], max[2]],
                ];

                let first = transform_point(corners[0], matrix);
                let mut new_min = first;
                let mut new_max = first;

                for corner in &corners[1..] {
                    let p = transform_point(*corner, matrix);
                    new_min[0] = new_min[0].min(p[0]);
                    new_min[1] = new_min[1].min(p[1]);
                    new_min[2] = new_min[2].min(p[2]);
                    new_max[0] = new_max[0].max(p[0]);
                    new_max[1] = new_max[1].max(p[1]);
                    new_max[2] = new_max[2].max(p[2]);
                }

                Bounds::Aabb {
                    min: new_min,
                    max: new_max,
                }
            },
            Bounds::Obb {
                center,
                half_extents,
                rotation,
            } => Bounds::Obb {
                center: transform_point(*center, matrix),
                half_extents: *half_extents,
                rotation: *rotation,
            },
        }
    }
}

/// Occlusion culling using HZB
pub struct OcclusionCuller {
    hzb_pyramid: Vec<Vec<f32>>,
    width: u32,
    height: u32,
    mip_count: u32,
}

impl OcclusionCuller {
    pub fn new(width: u32, height: u32) -> Self {
        let mip_count = (width.max(height) as f32).log2().ceil() as u32;
        let mut pyramid = Vec::with_capacity(mip_count as usize);

        let mut w = width;
        let mut h = height;
        for _ in 0..mip_count {
            pyramid.push(vec![1.0; (w * h) as usize]);
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }

        Self {
            hzb_pyramid: pyramid,
            width,
            height,
            mip_count,
        }
    }

    /// Update HZB from depth buffer
    pub fn update(&mut self, depth_buffer: &[f32]) {
        // Copy first level
        if let Some(first) = self.hzb_pyramid.first_mut() {
            first.copy_from_slice(depth_buffer);
        }

        // Build mip chain
        for i in 1..self.mip_count as usize {
            let (src_w, src_h) = self.mip_dimensions(i - 1);
            let (dst_w, dst_h) = self.mip_dimensions(i);

            for y in 0..dst_h {
                for x in 0..dst_w {
                    let sx = x * 2;
                    let sy = y * 2;

                    let max_depth = self
                        .sample_mip(i - 1, sx, sy)
                        .max(self.sample_mip(i - 1, sx + 1, sy))
                        .max(self.sample_mip(i - 1, sx, sy + 1))
                        .max(self.sample_mip(i - 1, sx + 1, sy + 1));

                    let idx = (y * dst_w + x) as usize;
                    if let Some(mip) = self.hzb_pyramid.get_mut(i) {
                        if idx < mip.len() {
                            mip[idx] = max_depth;
                        }
                    }
                }
            }
        }
    }

    fn mip_dimensions(&self, level: usize) -> (u32, u32) {
        let w = (self.width >> level).max(1);
        let h = (self.height >> level).max(1);
        (w, h)
    }

    fn sample_mip(&self, level: usize, x: u32, y: u32) -> f32 {
        let (w, h) = self.mip_dimensions(level);
        let x = x.min(w - 1);
        let y = y.min(h - 1);

        self.hzb_pyramid
            .get(level)
            .and_then(|mip| mip.get((y * w + x) as usize).copied())
            .unwrap_or(1.0)
    }

    /// Test if bounds are occluded
    pub fn is_occluded(&self, _screen_rect: [f32; 4], _depth: f32) -> bool {
        // Would sample HZB at appropriate mip level
        false
    }
}

fn transform_point(p: [f32; 3], m: [[f32; 4]; 4]) -> [f32; 3] {
    [
        m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
        m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
        m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
    ]
}
