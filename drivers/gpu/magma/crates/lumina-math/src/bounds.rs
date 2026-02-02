//! Bounding volumes and spatial types
//!
//! This module provides bounding box, sphere, and frustum types.

use crate::{Vec2, Vec3, Vec4, Mat4};

/// Axis-aligned bounding box (2D)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Aabb2 {
    /// Minimum corner
    pub min: Vec2,
    /// Maximum corner
    pub max: Vec2,
}

impl Aabb2 {
    /// Empty AABB (inverted for union operations)
    pub const EMPTY: Self = Self {
        min: Vec2::new(f32::INFINITY, f32::INFINITY),
        max: Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// Unit AABB (0 to 1)
    pub const UNIT: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ONE,
    };

    /// Creates an AABB from min/max corners
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Creates an AABB from center and half-extents
    pub fn from_center_half_extents(center: Vec2, half_extents: Vec2) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Creates an AABB from center and size
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self::from_center_half_extents(center, half)
    }

    /// Creates an AABB from a set of points
    pub fn from_points(points: &[Vec2]) -> Self {
        let mut aabb = Self::EMPTY;
        for p in points {
            aabb = aabb.expand_to_include(*p);
        }
        aabb
    }

    /// Returns the center of the AABB
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Returns the size of the AABB
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Returns the half-extents of the AABB
    pub fn half_extents(&self) -> Vec2 {
        self.size() * 0.5
    }

    /// Returns the area of the AABB
    pub fn area(&self) -> f32 {
        let s = self.size();
        s.x * s.y
    }

    /// Returns the perimeter of the AABB
    pub fn perimeter(&self) -> f32 {
        let s = self.size();
        2.0 * (s.x + s.y)
    }

    /// Checks if the AABB contains a point
    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y
    }

    /// Checks if this AABB intersects another
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y
    }

    /// Returns the union of two AABBs
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns the intersection of two AABBs
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min.x <= max.x && min.y <= max.y {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Expands the AABB to include a point
    pub fn expand_to_include(&self, point: Vec2) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    /// Expands the AABB by a margin
    pub fn expand(&self, margin: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(margin),
            max: self.max + Vec2::splat(margin),
        }
    }

    /// Shrinks the AABB by a margin
    pub fn shrink(&self, margin: f32) -> Self {
        self.expand(-margin)
    }

    /// Checks if the AABB is valid (min <= max)
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
    }

    /// Clamps a point to be inside the AABB
    pub fn clamp_point(&self, point: Vec2) -> Vec2 {
        point.clamp(self.min, self.max)
    }

    /// Returns the closest point on the AABB to a given point
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        self.clamp_point(point)
    }

    /// Returns the squared distance from a point to the AABB
    pub fn distance_squared(&self, point: Vec2) -> f32 {
        let closest = self.closest_point(point);
        (point - closest).length_squared()
    }

    /// Returns the distance from a point to the AABB
    pub fn distance(&self, point: Vec2) -> f32 {
        self.distance_squared(point).sqrt()
    }
}

impl Default for Aabb2 {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Axis-aligned bounding box (3D)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Aabb3 {
    /// Minimum corner
    pub min: Vec3,
    /// Maximum corner
    pub max: Vec3,
}

impl Aabb3 {
    /// Empty AABB (inverted for union operations)
    pub const EMPTY: Self = Self {
        min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// Unit AABB (0 to 1)
    pub const UNIT: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ONE,
    };

    /// Creates an AABB from min/max corners
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Creates an AABB from center and half-extents
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Creates an AABB from center and size
    pub fn from_center_size(center: Vec3, size: Vec3) -> Self {
        let half = size * 0.5;
        Self::from_center_half_extents(center, half)
    }

    /// Creates an AABB from a set of points
    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::EMPTY;
        for p in points {
            aabb = aabb.expand_to_include(*p);
        }
        aabb
    }

    /// Returns the center of the AABB
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the size of the AABB
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the half-extents of the AABB
    pub fn half_extents(&self) -> Vec3 {
        self.size() * 0.5
    }

    /// Returns the volume of the AABB
    pub fn volume(&self) -> f32 {
        let s = self.size();
        s.x * s.y * s.z
    }

    /// Returns the surface area of the AABB
    pub fn surface_area(&self) -> f32 {
        let s = self.size();
        2.0 * (s.x * s.y + s.y * s.z + s.z * s.x)
    }

    /// Returns the 8 corners of the AABB
    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    /// Checks if the AABB contains a point
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    /// Checks if this AABB fully contains another
    pub fn contains_aabb(&self, other: &Self) -> bool {
        self.min.x <= other.min.x && self.max.x >= other.max.x &&
        self.min.y <= other.min.y && self.max.y >= other.max.y &&
        self.min.z <= other.min.z && self.max.z >= other.max.z
    }

    /// Checks if this AABB intersects another
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// Returns the union of two AABBs
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Returns the intersection of two AABBs
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Expands the AABB to include a point
    pub fn expand_to_include(&self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    /// Expands the AABB by a margin
    pub fn expand(&self, margin: f32) -> Self {
        Self {
            min: self.min - Vec3::splat(margin),
            max: self.max + Vec3::splat(margin),
        }
    }

    /// Shrinks the AABB by a margin
    pub fn shrink(&self, margin: f32) -> Self {
        self.expand(-margin)
    }

    /// Checks if the AABB is valid (min <= max)
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    /// Clamps a point to be inside the AABB
    pub fn clamp_point(&self, point: Vec3) -> Vec3 {
        point.clamp(self.min, self.max)
    }

    /// Returns the closest point on the AABB to a given point
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        self.clamp_point(point)
    }

    /// Returns the squared distance from a point to the AABB
    pub fn distance_squared(&self, point: Vec3) -> f32 {
        let closest = self.closest_point(point);
        (point - closest).length_squared()
    }

    /// Returns the distance from a point to the AABB
    pub fn distance(&self, point: Vec3) -> f32 {
        self.distance_squared(point).sqrt()
    }

    /// Transforms the AABB by a matrix (returns a new AABB enclosing the transformed one)
    pub fn transform(&self, matrix: &Mat4) -> Self {
        let corners = self.corners();
        let mut transformed = Self::EMPTY;
        for corner in corners.iter() {
            let p = matrix.transform_point(*corner);
            transformed = transformed.expand_to_include(p);
        }
        transformed
    }

    /// Ray-AABB intersection test
    /// Returns (t_min, t_max) if intersecting, None otherwise
    pub fn ray_intersect(&self, origin: Vec3, dir_inv: Vec3) -> Option<(f32, f32)> {
        let t1 = (self.min - origin) * dir_inv;
        let t2 = (self.max - origin) * dir_inv;

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        let t_enter = t_min.x.max(t_min.y).max(t_min.z);
        let t_exit = t_max.x.min(t_max.y).min(t_max.z);

        if t_enter <= t_exit && t_exit >= 0.0 {
            Some((t_enter.max(0.0), t_exit))
        } else {
            None
        }
    }
}

impl Default for Aabb3 {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Bounding sphere
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct BoundingSphere {
    /// Center point
    pub center: Vec3,
    /// Radius
    pub radius: f32,
}

impl BoundingSphere {
    /// Creates a bounding sphere
    pub const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Creates a bounding sphere at the origin
    pub const fn from_radius(radius: f32) -> Self {
        Self {
            center: Vec3::ZERO,
            radius,
        }
    }

    /// Creates a bounding sphere from an AABB
    pub fn from_aabb(aabb: &Aabb3) -> Self {
        let center = aabb.center();
        let radius = aabb.half_extents().length();
        Self { center, radius }
    }

    /// Creates a minimal bounding sphere from points (Ritter's algorithm)
    pub fn from_points(points: &[Vec3]) -> Self {
        if points.is_empty() {
            return Self::new(Vec3::ZERO, 0.0);
        }

        // Find extreme points
        let mut min_x = points[0];
        let mut max_x = points[0];
        let mut min_y = points[0];
        let mut max_y = points[0];
        let mut min_z = points[0];
        let mut max_z = points[0];

        for p in points.iter() {
            if p.x < min_x.x { min_x = *p; }
            if p.x > max_x.x { max_x = *p; }
            if p.y < min_y.y { min_y = *p; }
            if p.y > max_y.y { max_y = *p; }
            if p.z < min_z.z { min_z = *p; }
            if p.z > max_z.z { max_z = *p; }
        }

        // Find the pair with maximum distance
        let dx = (max_x - min_x).length_squared();
        let dy = (max_y - min_y).length_squared();
        let dz = (max_z - min_z).length_squared();

        let (p1, p2) = if dx >= dy && dx >= dz {
            (min_x, max_x)
        } else if dy >= dz {
            (min_y, max_y)
        } else {
            (min_z, max_z)
        };

        let mut center = (p1 + p2) * 0.5;
        let mut radius = (p2 - center).length();

        // Grow sphere to include all points
        for p in points.iter() {
            let d = (*p - center).length();
            if d > radius {
                let new_radius = (radius + d) * 0.5;
                let k = (new_radius - radius) / d;
                center = center + (*p - center) * k;
                radius = new_radius;
            }
        }

        Self { center, radius }
    }

    /// Returns the volume of the sphere
    pub fn volume(&self) -> f32 {
        (4.0 / 3.0) * core::f32::consts::PI * self.radius * self.radius * self.radius
    }

    /// Returns the surface area of the sphere
    pub fn surface_area(&self) -> f32 {
        4.0 * core::f32::consts::PI * self.radius * self.radius
    }

    /// Checks if the sphere contains a point
    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Checks if this sphere intersects another
    pub fn intersects(&self, other: &Self) -> bool {
        let dist_sq = (other.center - self.center).length_squared();
        let r_sum = self.radius + other.radius;
        dist_sq <= r_sum * r_sum
    }

    /// Checks if this sphere intersects an AABB
    pub fn intersects_aabb(&self, aabb: &Aabb3) -> bool {
        aabb.distance_squared(self.center) <= self.radius * self.radius
    }

    /// Returns the union of two spheres (minimal enclosing sphere)
    pub fn union(&self, other: &Self) -> Self {
        let d = other.center - self.center;
        let dist = d.length();

        // One sphere contains the other
        if dist + other.radius <= self.radius {
            return *self;
        }
        if dist + self.radius <= other.radius {
            return *other;
        }

        // Create new sphere
        let radius = (dist + self.radius + other.radius) * 0.5;
        let center = self.center + d * ((radius - self.radius) / dist);
        Self { center, radius }
    }

    /// Expands the sphere to include a point
    pub fn expand_to_include(&self, point: Vec3) -> Self {
        let d = point - self.center;
        let dist = d.length();

        if dist <= self.radius {
            return *self;
        }

        let radius = (self.radius + dist) * 0.5;
        let center = self.center + d * ((radius - self.radius) / dist);
        Self { center, radius }
    }

    /// Transforms the sphere by a matrix (assumes uniform scale)
    pub fn transform(&self, matrix: &Mat4) -> Self {
        let center = matrix.transform_point(self.center);
        // Approximate scale from matrix
        let scale = matrix.transform_vector(Vec3::X).length();
        Self {
            center,
            radius: self.radius * scale,
        }
    }
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 0.0)
    }
}

/// Frustum (6 planes)
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    /// Planes (left, right, bottom, top, near, far)
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Plane indices
    pub const LEFT: usize = 0;
    pub const RIGHT: usize = 1;
    pub const BOTTOM: usize = 2;
    pub const TOP: usize = 3;
    pub const NEAR: usize = 4;
    pub const FAR: usize = 5;

    /// Creates a frustum from a view-projection matrix
    pub fn from_view_projection(vp: &Mat4) -> Self {
        // Extract planes from the matrix (Gribb/Hartmann method)
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let planes = [
            Plane::from_vec4(row3 + row0).normalize(), // Left
            Plane::from_vec4(row3 - row0).normalize(), // Right
            Plane::from_vec4(row3 + row1).normalize(), // Bottom
            Plane::from_vec4(row3 - row1).normalize(), // Top
            Plane::from_vec4(row3 + row2).normalize(), // Near
            Plane::from_vec4(row3 - row2).normalize(), // Far
        ];

        Self { planes }
    }

    /// Tests if a point is inside the frustum
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(point) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Tests if a sphere intersects the frustum
    pub fn intersects_sphere(&self, sphere: &BoundingSphere) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// Tests if an AABB intersects the frustum
    pub fn intersects_aabb(&self, aabb: &Aabb3) -> bool {
        for plane in &self.planes {
            // Find the positive vertex (furthest along plane normal)
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );

            if plane.signed_distance(p) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Tests intersection and returns whether fully inside, partially inside, or outside
    pub fn test_aabb(&self, aabb: &Aabb3) -> FrustumTestResult {
        let mut all_inside = true;

        for plane in &self.planes {
            let p_vertex = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );
            let n_vertex = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.min.x } else { aabb.max.x },
                if plane.normal.y >= 0.0 { aabb.min.y } else { aabb.max.y },
                if plane.normal.z >= 0.0 { aabb.min.z } else { aabb.max.z },
            );

            if plane.signed_distance(p_vertex) < 0.0 {
                return FrustumTestResult::Outside;
            }
            if plane.signed_distance(n_vertex) < 0.0 {
                all_inside = false;
            }
        }

        if all_inside {
            FrustumTestResult::Inside
        } else {
            FrustumTestResult::Intersecting
        }
    }
}

/// Result of frustum culling test
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrustumTestResult {
    /// Completely outside the frustum
    Outside,
    /// Completely inside the frustum
    Inside,
    /// Intersecting the frustum boundary
    Intersecting,
}

/// A plane in 3D space (ax + by + cz + d = 0)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Plane {
    /// Normal vector (a, b, c)
    pub normal: Vec3,
    /// Distance from origin (d)
    pub d: f32,
}

impl Plane {
    /// Creates a plane from normal and distance
    pub const fn new(normal: Vec3, d: f32) -> Self {
        Self { normal, d }
    }

    /// Creates a plane from a Vec4 (normal.xyz, d)
    pub fn from_vec4(v: Vec4) -> Self {
        Self {
            normal: Vec3::new(v.x, v.y, v.z),
            d: v.w,
        }
    }

    /// Creates a plane from a point and normal
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let normal = normal.normalize();
        let d = -normal.dot(point);
        Self { normal, d }
    }

    /// Creates a plane from three points
    pub fn from_points(p0: Vec3, p1: Vec3, p2: Vec3) -> Self {
        let v1 = p1 - p0;
        let v2 = p2 - p0;
        let normal = v1.cross(v2).normalize();
        let d = -normal.dot(p0);
        Self { normal, d }
    }

    /// Normalizes the plane
    pub fn normalize(&self) -> Self {
        let len = self.normal.length();
        if len > 0.0 {
            Self {
                normal: self.normal / len,
                d: self.d / len,
            }
        } else {
            *self
        }
    }

    /// Returns the signed distance from a point to the plane
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.d
    }

    /// Returns the absolute distance from a point to the plane
    pub fn distance(&self, point: Vec3) -> f32 {
        self.signed_distance(point).abs()
    }

    /// Projects a point onto the plane
    pub fn project_point(&self, point: Vec3) -> Vec3 {
        point - self.normal * self.signed_distance(point)
    }

    /// Reflects a point across the plane
    pub fn reflect_point(&self, point: Vec3) -> Vec3 {
        point - self.normal * (2.0 * self.signed_distance(point))
    }

    /// Reflects a vector across the plane
    pub fn reflect_vector(&self, v: Vec3) -> Vec3 {
        v - self.normal * (2.0 * self.normal.dot(v))
    }

    /// Intersects a ray with the plane
    /// Returns t such that origin + t * direction is on the plane
    pub fn ray_intersect(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let denom = self.normal.dot(direction);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = -(self.normal.dot(origin) + self.d) / denom;
        Some(t)
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self::new(Vec3::Y, 0.0) // XZ plane
    }
}

/// Ray in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Ray {
    /// Origin point
    pub origin: Vec3,
    /// Direction (should be normalized)
    pub direction: Vec3,
}

impl Ray {
    /// Creates a ray
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Returns a point along the ray at parameter t
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Returns the inverse direction (1/dir for each component)
    pub fn direction_inverse(&self) -> Vec3 {
        Vec3::new(
            1.0 / self.direction.x,
            1.0 / self.direction.y,
            1.0 / self.direction.z,
        )
    }

    /// Intersects with an AABB
    pub fn intersect_aabb(&self, aabb: &Aabb3) -> Option<(f32, f32)> {
        aabb.ray_intersect(self.origin, self.direction_inverse())
    }

    /// Intersects with a sphere
    pub fn intersect_sphere(&self, sphere: &BoundingSphere) -> Option<(f32, f32)> {
        let oc = self.origin - sphere.center;
        let a = self.direction.dot(self.direction);
        let b = 2.0 * oc.dot(self.direction);
        let c = oc.dot(oc) - sphere.radius * sphere.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        Some((t1, t2))
    }

    /// Intersects with a plane
    pub fn intersect_plane(&self, plane: &Plane) -> Option<f32> {
        plane.ray_intersect(self.origin, self.direction)
    }

    /// Intersects with a triangle
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<(f32, f32, f32)> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-6 {
            return None;
        }

        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > 1e-6 {
            Some((t, u, v))
        } else {
            None
        }
    }
}

impl Default for Ray {
    fn default() -> Self {
        Self::new(Vec3::ZERO, Vec3::Z)
    }
}
