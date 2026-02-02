//! Geometric primitives for 3D graphics
//!
//! This module provides types for common geometric primitives
//! used in rendering and physics calculations.

use crate::vec::{Vec2, Vec3, Vec4};
use crate::mat::Mat4;

/// A ray in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Ray {
    /// Origin point of the ray
    pub origin: Vec3,
    /// Direction of the ray (should be normalized)
    pub direction: Vec3,
}

impl Ray {
    /// Creates a new ray
    #[inline]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Returns a point along the ray at distance t
    #[inline]
    pub fn at(self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Transforms this ray by a matrix
    #[inline]
    pub fn transform(self, matrix: Mat4) -> Self {
        let origin = matrix * self.origin;
        let direction = (matrix * (self.origin + self.direction)) - origin;
        Self::new(origin, direction)
    }
}

/// An infinite plane in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Plane {
    /// Normal vector of the plane
    pub normal: Vec3,
    /// Distance from origin along the normal
    pub distance: f32,
}

impl Plane {
    /// Creates a plane from normal and distance
    #[inline]
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self {
            normal: normal.normalize(),
            distance,
        }
    }

    /// Creates a plane from three points
    #[inline]
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let normal = (b - a).cross(c - a).normalize();
        let distance = normal.dot(a);
        Self { normal, distance }
    }

    /// Creates a plane from a normal and a point on the plane
    #[inline]
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let normal = normal.normalize();
        Self {
            normal,
            distance: normal.dot(point),
        }
    }

    /// Creates a plane from a Vec4 (normal.xyz, distance.w)
    #[inline]
    pub fn from_vec4(v: Vec4) -> Self {
        let len = Vec3::new(v.x, v.y, v.z).length();
        if len > 0.0 {
            Self {
                normal: Vec3::new(v.x / len, v.y / len, v.z / len),
                distance: v.w / len,
            }
        } else {
            Self {
                normal: Vec3::Y,
                distance: 0.0,
            }
        }
    }

    /// Returns the signed distance from a point to the plane
    #[inline]
    pub fn signed_distance(self, point: Vec3) -> f32 {
        self.normal.dot(point) - self.distance
    }

    /// Returns the closest point on the plane to a given point
    #[inline]
    pub fn closest_point(self, point: Vec3) -> Vec3 {
        point - self.normal * self.signed_distance(point)
    }

    /// Tests if a point is in front of the plane
    #[inline]
    pub fn is_front(self, point: Vec3) -> bool {
        self.signed_distance(point) > 0.0
    }

    /// Tests if a point is behind the plane
    #[inline]
    pub fn is_behind(self, point: Vec3) -> bool {
        self.signed_distance(point) < 0.0
    }

    /// Intersects a ray with this plane
    #[inline]
    pub fn intersect_ray(self, ray: Ray) -> Option<f32> {
        let denom = self.normal.dot(ray.direction);
        if denom.abs() < 1e-6 {
            return None;
        }

        let t = (self.distance - self.normal.dot(ray.origin)) / denom;
        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }

    /// Normalizes the plane equation
    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.normal.length();
        if len > 0.0 {
            Self {
                normal: self.normal / len,
                distance: self.distance / len,
            }
        } else {
            self
        }
    }
}

/// An axis-aligned bounding box
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct AABB {
    /// Minimum corner
    pub min: Vec3,
    /// Maximum corner
    pub max: Vec3,
}

impl AABB {
    /// An empty AABB
    pub const EMPTY: Self = Self {
        min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// An infinite AABB
    pub const INFINITE: Self = Self {
        min: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        max: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
    };

    /// Creates an AABB from min and max corners
    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Creates an AABB from center and half-extents
    #[inline]
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Creates an AABB from a set of points
    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::EMPTY;
        for point in points {
            aabb = aabb.expand_point(*point);
        }
        aabb
    }

    /// Returns the center of the AABB
    #[inline]
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Returns the size (dimensions) of the AABB
    #[inline]
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the half-extents of the AABB
    #[inline]
    pub fn half_extents(self) -> Vec3 {
        self.size() * 0.5
    }

    /// Returns the volume of the AABB
    #[inline]
    pub fn volume(self) -> f32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    /// Returns the surface area of the AABB
    #[inline]
    pub fn surface_area(self) -> f32 {
        let size = self.size();
        2.0 * (size.x * size.y + size.y * size.z + size.z * size.x)
    }

    /// Tests if a point is inside the AABB
    #[inline]
    pub fn contains_point(self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Tests if this AABB contains another AABB
    #[inline]
    pub fn contains_aabb(self, other: AABB) -> bool {
        self.min.x <= other.min.x
            && self.min.y <= other.min.y
            && self.min.z <= other.min.z
            && self.max.x >= other.max.x
            && self.max.y >= other.max.y
            && self.max.z >= other.max.z
    }

    /// Tests if this AABB intersects another AABB
    #[inline]
    pub fn intersects_aabb(self, other: AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns the union of two AABBs
    #[inline]
    pub fn union(self, other: AABB) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    /// Returns the intersection of two AABBs
    #[inline]
    pub fn intersection(self, other: AABB) -> Option<Self> {
        let min = Vec3::new(
            self.min.x.max(other.min.x),
            self.min.y.max(other.min.y),
            self.min.z.max(other.min.z),
        );
        let max = Vec3::new(
            self.max.x.min(other.max.x),
            self.max.y.min(other.max.y),
            self.max.z.min(other.max.z),
        );

        if min.x <= max.x && min.y <= max.y && min.z <= max.z {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Expands this AABB to include a point
    #[inline]
    pub fn expand_point(self, point: Vec3) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.min(point.x),
                self.min.y.min(point.y),
                self.min.z.min(point.z),
            ),
            max: Vec3::new(
                self.max.x.max(point.x),
                self.max.y.max(point.y),
                self.max.z.max(point.z),
            ),
        }
    }

    /// Expands this AABB by a given amount in all directions
    #[inline]
    pub fn expand(self, amount: f32) -> Self {
        let expansion = Vec3::splat(amount);
        Self {
            min: self.min - expansion,
            max: self.max + expansion,
        }
    }

    /// Transforms this AABB by a matrix
    pub fn transform(self, matrix: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        let mut result = Self::EMPTY;
        for corner in corners {
            result = result.expand_point(matrix * corner);
        }
        result
    }

    /// Returns the 8 corners of the AABB
    #[inline]
    pub fn corners(self) -> [Vec3; 8] {
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

    /// Intersects a ray with this AABB
    #[inline]
    pub fn intersect_ray(self, ray: Ray) -> Option<f32> {
        let inv_dir = Vec3::new(1.0 / ray.direction.x, 1.0 / ray.direction.y, 1.0 / ray.direction.z);

        let t1 = (self.min.x - ray.origin.x) * inv_dir.x;
        let t2 = (self.max.x - ray.origin.x) * inv_dir.x;
        let t3 = (self.min.y - ray.origin.y) * inv_dir.y;
        let t4 = (self.max.y - ray.origin.y) * inv_dir.y;
        let t5 = (self.min.z - ray.origin.z) * inv_dir.z;
        let t6 = (self.max.z - ray.origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            None
        } else if tmin < 0.0 {
            Some(tmax)
        } else {
            Some(tmin)
        }
    }
}

impl Default for AABB {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// A sphere in 3D space
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Sphere {
    /// Center of the sphere
    pub center: Vec3,
    /// Radius of the sphere
    pub radius: f32,
}

impl Sphere {
    /// Creates a new sphere
    #[inline]
    pub const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Creates a sphere from an AABB (bounding sphere)
    #[inline]
    pub fn from_aabb(aabb: AABB) -> Self {
        let center = aabb.center();
        let radius = aabb.half_extents().length();
        Self { center, radius }
    }

    /// Returns the volume of the sphere
    #[inline]
    pub fn volume(self) -> f32 {
        (4.0 / 3.0) * core::f32::consts::PI * self.radius * self.radius * self.radius
    }

    /// Returns the surface area of the sphere
    #[inline]
    pub fn surface_area(self) -> f32 {
        4.0 * core::f32::consts::PI * self.radius * self.radius
    }

    /// Tests if a point is inside the sphere
    #[inline]
    pub fn contains_point(self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Tests if this sphere contains another sphere
    #[inline]
    pub fn contains_sphere(self, other: Sphere) -> bool {
        let distance = (other.center - self.center).length();
        distance + other.radius <= self.radius
    }

    /// Tests if this sphere intersects another sphere
    #[inline]
    pub fn intersects_sphere(self, other: Sphere) -> bool {
        let distance_sq = (other.center - self.center).length_squared();
        let radius_sum = self.radius + other.radius;
        distance_sq <= radius_sum * radius_sum
    }

    /// Tests if this sphere intersects an AABB
    #[inline]
    pub fn intersects_aabb(self, aabb: AABB) -> bool {
        let closest = Vec3::new(
            self.center.x.clamp(aabb.min.x, aabb.max.x),
            self.center.y.clamp(aabb.min.y, aabb.max.y),
            self.center.z.clamp(aabb.min.z, aabb.max.z),
        );
        (closest - self.center).length_squared() <= self.radius * self.radius
    }

    /// Returns a bounding AABB for this sphere
    #[inline]
    pub fn to_aabb(self) -> AABB {
        let half_extents = Vec3::splat(self.radius);
        AABB {
            min: self.center - half_extents,
            max: self.center + half_extents,
        }
    }

    /// Intersects a ray with this sphere
    #[inline]
    pub fn intersect_ray(self, ray: Ray) -> Option<f32> {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = 2.0 * oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            None
        } else {
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            if t >= 0.0 {
                Some(t)
            } else {
                let t = (-b + discriminant.sqrt()) / (2.0 * a);
                if t >= 0.0 {
                    Some(t)
                } else {
                    None
                }
            }
        }
    }

    /// Expands this sphere to include a point
    #[inline]
    pub fn expand_point(self, point: Vec3) -> Self {
        let to_point = point - self.center;
        let distance = to_point.length();

        if distance <= self.radius {
            self
        } else {
            let new_radius = (self.radius + distance) * 0.5;
            let new_center = self.center + to_point.normalize() * (new_radius - self.radius);
            Self {
                center: new_center,
                radius: new_radius,
            }
        }
    }
}

/// A view frustum for culling
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Frustum {
    /// Left plane
    pub left: Plane,
    /// Right plane
    pub right: Plane,
    /// Bottom plane
    pub bottom: Plane,
    /// Top plane
    pub top: Plane,
    /// Near plane
    pub near: Plane,
    /// Far plane
    pub far: Plane,
}

impl Frustum {
    /// Creates a frustum from a view-projection matrix
    pub fn from_view_projection(vp: Mat4) -> Self {
        // Extract frustum planes from the view-projection matrix
        let row0 = Vec4::new(vp.x_axis.x, vp.y_axis.x, vp.z_axis.x, vp.w_axis.x);
        let row1 = Vec4::new(vp.x_axis.y, vp.y_axis.y, vp.z_axis.y, vp.w_axis.y);
        let row2 = Vec4::new(vp.x_axis.z, vp.y_axis.z, vp.z_axis.z, vp.w_axis.z);
        let row3 = Vec4::new(vp.x_axis.w, vp.y_axis.w, vp.z_axis.w, vp.w_axis.w);

        Self {
            left: Plane::from_vec4(Vec4::new(
                row3.x + row0.x,
                row3.y + row0.y,
                row3.z + row0.z,
                row3.w + row0.w,
            )),
            right: Plane::from_vec4(Vec4::new(
                row3.x - row0.x,
                row3.y - row0.y,
                row3.z - row0.z,
                row3.w - row0.w,
            )),
            bottom: Plane::from_vec4(Vec4::new(
                row3.x + row1.x,
                row3.y + row1.y,
                row3.z + row1.z,
                row3.w + row1.w,
            )),
            top: Plane::from_vec4(Vec4::new(
                row3.x - row1.x,
                row3.y - row1.y,
                row3.z - row1.z,
                row3.w - row1.w,
            )),
            near: Plane::from_vec4(Vec4::new(
                row3.x + row2.x,
                row3.y + row2.y,
                row3.z + row2.z,
                row3.w + row2.w,
            )),
            far: Plane::from_vec4(Vec4::new(
                row3.x - row2.x,
                row3.y - row2.y,
                row3.z - row2.z,
                row3.w - row2.w,
            )),
        }
    }

    /// Returns an array of all 6 planes
    #[inline]
    pub fn planes(&self) -> [Plane; 6] {
        [self.left, self.right, self.bottom, self.top, self.near, self.far]
    }

    /// Tests if a point is inside the frustum
    #[inline]
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in self.planes() {
            if plane.signed_distance(point) < 0.0 {
                return false;
            }
        }
        true
    }

    /// Tests if a sphere is inside or intersects the frustum
    #[inline]
    pub fn intersects_sphere(&self, sphere: Sphere) -> bool {
        for plane in self.planes() {
            if plane.signed_distance(sphere.center) < -sphere.radius {
                return false;
            }
        }
        true
    }

    /// Tests if an AABB is inside or intersects the frustum
    pub fn intersects_aabb(&self, aabb: AABB) -> bool {
        for plane in self.planes() {
            // Find the positive vertex (farthest along the plane normal)
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
}

/// Intersection result for culling operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intersection {
    /// Completely outside
    Outside,
    /// Partially inside (intersecting)
    Intersecting,
    /// Completely inside
    Inside,
}

impl Frustum {
    /// Tests if an AABB is inside, outside, or intersecting the frustum
    pub fn test_aabb(&self, aabb: AABB) -> Intersection {
        let mut inside_count = 0;

        for plane in self.planes() {
            let p = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.normal.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.normal.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );

            let n = Vec3::new(
                if plane.normal.x >= 0.0 { aabb.min.x } else { aabb.max.x },
                if plane.normal.y >= 0.0 { aabb.min.y } else { aabb.max.y },
                if plane.normal.z >= 0.0 { aabb.min.z } else { aabb.max.z },
            );

            if plane.signed_distance(p) < 0.0 {
                return Intersection::Outside;
            }

            if plane.signed_distance(n) >= 0.0 {
                inside_count += 1;
            }
        }

        if inside_count == 6 {
            Intersection::Inside
        } else {
            Intersection::Intersecting
        }
    }

    /// Tests if a sphere is inside, outside, or intersecting the frustum
    pub fn test_sphere(&self, sphere: Sphere) -> Intersection {
        let mut inside_count = 0;

        for plane in self.planes() {
            let distance = plane.signed_distance(sphere.center);

            if distance < -sphere.radius {
                return Intersection::Outside;
            }

            if distance >= sphere.radius {
                inside_count += 1;
            }
        }

        if inside_count == 6 {
            Intersection::Inside
        } else {
            Intersection::Intersecting
        }
    }
}

/// A 2D rectangle
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Rect {
    /// X position (left edge)
    pub x: f32,
    /// Y position (top edge)
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

impl Rect {
    /// Creates a new rectangle
    #[inline]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates a rectangle from min and max corners
    #[inline]
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            x: min.x,
            y: min.y,
            width: max.x - min.x,
            height: max.y - min.y,
        }
    }

    /// Creates a rectangle from center and size
    #[inline]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self {
            x: center.x - size.x * 0.5,
            y: center.y - size.y * 0.5,
            width: size.x,
            height: size.y,
        }
    }

    /// Returns the minimum corner
    #[inline]
    pub fn min(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Returns the maximum corner
    #[inline]
    pub fn max(self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    /// Returns the center
    #[inline]
    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Returns the size
    #[inline]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    /// Returns the area
    #[inline]
    pub fn area(self) -> f32 {
        self.width * self.height
    }

    /// Tests if a point is inside the rectangle
    #[inline]
    pub fn contains_point(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Tests if this rectangle contains another rectangle
    #[inline]
    pub fn contains_rect(self, other: Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }

    /// Tests if this rectangle intersects another rectangle
    #[inline]
    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Returns the union of two rectangles
    #[inline]
    pub fn union(self, other: Rect) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);
        Self {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Returns the intersection of two rectangles
    #[inline]
    pub fn intersection(self, other: Rect) -> Option<Self> {
        let min_x = self.x.max(other.x);
        let min_y = self.y.max(other.y);
        let max_x = (self.x + self.width).min(other.x + other.width);
        let max_y = (self.y + self.height).min(other.y + other.height);

        if min_x < max_x && min_y < max_y {
            Some(Self {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            })
        } else {
            None
        }
    }

    /// Expands the rectangle by an amount in all directions
    #[inline]
    pub fn expand(self, amount: f32) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_at() {
        let ray = Ray::new(Vec3::ZERO, Vec3::Z);
        let point = ray.at(5.0);
        assert!((point.z - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_plane_distance() {
        let plane = Plane::new(Vec3::Y, 0.0);
        let point = Vec3::new(0.0, 5.0, 0.0);
        assert!((plane.signed_distance(point) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_aabb_contains() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::ONE);
        assert!(aabb.contains_point(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!aabb.contains_point(Vec3::new(2.0, 0.5, 0.5)));
    }

    #[test]
    fn test_sphere_intersection() {
        let s1 = Sphere::new(Vec3::ZERO, 1.0);
        let s2 = Sphere::new(Vec3::new(1.5, 0.0, 0.0), 1.0);
        assert!(s1.intersects_sphere(s2));

        let s3 = Sphere::new(Vec3::new(3.0, 0.0, 0.0), 1.0);
        assert!(!s1.intersects_sphere(s3));
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert!(r1.intersects(r2));

        let intersection = r1.intersection(r2).unwrap();
        assert!((intersection.width - 5.0).abs() < 1e-5);
        assert!((intersection.height - 5.0).abs() < 1e-5);
    }
}
