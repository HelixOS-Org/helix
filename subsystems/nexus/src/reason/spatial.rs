//! # Spatial Reasoning
//!
//! Reasons about spatial relationships and structures.
//! Implements spatial logic and geometric reasoning.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SPATIAL TYPES
// ============================================================================

/// Point
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    #[inline]
    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Bounding box
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Point,
    pub max: Point,
}

impl BoundingBox {
    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    #[inline]
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    #[inline]
    pub fn center(&self) -> Point {
        Point {
            x: (self.min.x + self.max.x) / 2.0,
            y: (self.min.y + self.max.y) / 2.0,
            z: (self.min.z + self.max.z) / 2.0,
        }
    }

    #[inline]
    pub fn volume(&self) -> f64 {
        (self.max.x - self.min.x) *
        (self.max.y - self.min.y) *
        (self.max.z - self.min.z)
    }
}

/// Spatial object
#[derive(Debug, Clone)]
pub struct SpatialObject {
    /// Object ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Position
    pub position: Point,
    /// Bounds
    pub bounds: BoundingBox,
    /// Shape
    pub shape: Shape,
    /// Properties
    pub properties: BTreeMap<String, String>,
}

/// Shape
#[derive(Debug, Clone)]
pub enum Shape {
    Point,
    Sphere { radius: f64 },
    Box { dimensions: Point },
    Cylinder { radius: f64, height: f64 },
    Polygon { vertices: Vec<Point> },
}

/// Spatial relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialRelation {
    Contains,
    ContainedBy,
    Overlaps,
    Touches,
    Disjoint,
    Near,
    Far,
    Above,
    Below,
    LeftOf,
    RightOf,
    InFrontOf,
    Behind,
}

/// Region
#[derive(Debug, Clone)]
pub struct Region {
    /// Region ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Bounds
    pub bounds: BoundingBox,
    /// Objects
    pub objects: Vec<u64>,
    /// Sub-regions
    pub children: Vec<u64>,
}

/// Spatial query
#[derive(Debug, Clone)]
pub struct SpatialQuery {
    /// Query type
    pub query_type: QueryType,
    /// Limit
    pub limit: Option<usize>,
}

/// Query type
#[derive(Debug, Clone)]
pub enum QueryType {
    InBox(BoundingBox),
    InRadius { center: Point, radius: f64 },
    NearestTo { point: Point, k: usize },
    WithRelation { object: u64, relation: SpatialRelation },
}

/// Query result
#[derive(Debug, Clone)]
pub struct SpatialQueryResult {
    /// Objects
    pub objects: Vec<SpatialObject>,
    /// Distances (for nearest queries)
    pub distances: Vec<f64>,
}

// ============================================================================
// SPATIAL REASONER
// ============================================================================

/// Spatial reasoner
pub struct SpatialReasoner {
    /// Objects
    objects: BTreeMap<u64, SpatialObject>,
    /// Regions
    regions: BTreeMap<u64, Region>,
    /// Relation cache
    relation_cache: BTreeMap<(u64, u64), SpatialRelation>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: SpatialConfig,
    /// Statistics
    stats: SpatialStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SpatialConfig {
    /// Near threshold
    pub near_threshold: f64,
    /// Far threshold
    pub far_threshold: f64,
    /// Cache relations
    pub cache_relations: bool,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            near_threshold: 10.0,
            far_threshold: 100.0,
            cache_relations: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SpatialStats {
    /// Objects created
    pub objects_created: u64,
    /// Relations computed
    pub relations_computed: u64,
    /// Queries executed
    pub queries_executed: u64,
}

impl SpatialReasoner {
    /// Create new reasoner
    pub fn new(config: SpatialConfig) -> Self {
        Self {
            objects: BTreeMap::new(),
            regions: BTreeMap::new(),
            relation_cache: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SpatialStats::default(),
        }
    }

    /// Create object
    pub fn create_object(&mut self, name: &str, position: Point, shape: Shape) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let bounds = self.compute_bounds(&position, &shape);

        let object = SpatialObject {
            id,
            name: name.into(),
            position,
            bounds,
            shape,
            properties: BTreeMap::new(),
        };

        self.objects.insert(id, object);
        self.stats.objects_created += 1;

        id
    }

    fn compute_bounds(&self, position: &Point, shape: &Shape) -> BoundingBox {
        match shape {
            Shape::Point => BoundingBox::new(*position, *position),
            Shape::Sphere { radius } => BoundingBox::new(
                Point::new(position.x - radius, position.y - radius, position.z - radius),
                Point::new(position.x + radius, position.y + radius, position.z + radius),
            ),
            Shape::Box { dimensions } => BoundingBox::new(
                Point::new(
                    position.x - dimensions.x / 2.0,
                    position.y - dimensions.y / 2.0,
                    position.z - dimensions.z / 2.0,
                ),
                Point::new(
                    position.x + dimensions.x / 2.0,
                    position.y + dimensions.y / 2.0,
                    position.z + dimensions.z / 2.0,
                ),
            ),
            Shape::Cylinder { radius, height } => BoundingBox::new(
                Point::new(position.x - radius, position.y, position.z - radius),
                Point::new(position.x + radius, position.y + height, position.z + radius),
            ),
            Shape::Polygon { vertices } => {
                if vertices.is_empty() {
                    return BoundingBox::new(*position, *position);
                }
                let mut min = vertices[0];
                let mut max = vertices[0];
                for v in vertices {
                    if v.x < min.x { min.x = v.x; }
                    if v.y < min.y { min.y = v.y; }
                    if v.z < min.z { min.z = v.z; }
                    if v.x > max.x { max.x = v.x; }
                    if v.y > max.y { max.y = v.y; }
                    if v.z > max.z { max.z = v.z; }
                }
                BoundingBox::new(min, max)
            }
        }
    }

    /// Move object
    #[inline]
    pub fn move_object(&mut self, id: u64, new_position: Point) {
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.position = new_position;
            obj.bounds = self.compute_bounds(&new_position, &obj.shape.clone());

            // Invalidate cache
            self.relation_cache.retain(|&(a, b), _| a != id && b != id);
        }
    }

    /// Compute relation
    pub fn relation(&mut self, obj1: u64, obj2: u64) -> Option<SpatialRelation> {
        // Check cache
        if self.config.cache_relations {
            if let Some(&rel) = self.relation_cache.get(&(obj1, obj2)) {
                return Some(rel);
            }
        }

        let o1 = self.objects.get(&obj1)?;
        let o2 = self.objects.get(&obj2)?;

        let rel = self.compute_relation(o1, o2);

        // Cache result
        if self.config.cache_relations {
            self.relation_cache.insert((obj1, obj2), rel);
        }

        self.stats.relations_computed += 1;

        Some(rel)
    }

    fn compute_relation(&self, o1: &SpatialObject, o2: &SpatialObject) -> SpatialRelation {
        let distance = o1.position.distance(&o2.position);

        // Check containment
        if self.contains(o1, o2) {
            return SpatialRelation::Contains;
        }
        if self.contains(o2, o1) {
            return SpatialRelation::ContainedBy;
        }

        // Check overlap
        if o1.bounds.intersects(&o2.bounds) {
            return SpatialRelation::Overlaps;
        }

        // Check proximity
        if distance < self.config.near_threshold {
            return SpatialRelation::Near;
        }
        if distance > self.config.far_threshold {
            return SpatialRelation::Far;
        }

        // Check directional relations
        let dx = o2.position.x - o1.position.x;
        let dy = o2.position.y - o1.position.y;
        let dz = o2.position.z - o1.position.z;

        let abs_dx = dx.abs();
        let abs_dy = dy.abs();
        let abs_dz = dz.abs();

        if abs_dy > abs_dx && abs_dy > abs_dz {
            if dy > 0.0 {
                return SpatialRelation::Above;
            } else {
                return SpatialRelation::Below;
            }
        }

        if abs_dx > abs_dz {
            if dx > 0.0 {
                return SpatialRelation::RightOf;
            } else {
                return SpatialRelation::LeftOf;
            }
        }

        if dz > 0.0 {
            SpatialRelation::InFrontOf
        } else {
            SpatialRelation::Behind
        }
    }

    fn contains(&self, container: &SpatialObject, contained: &SpatialObject) -> bool {
        container.bounds.contains(&contained.bounds.min) &&
        container.bounds.contains(&contained.bounds.max)
    }

    /// Query objects
    pub fn query(&mut self, query: &SpatialQuery) -> SpatialQueryResult {
        self.stats.queries_executed += 1;

        let mut objects = Vec::new();
        let mut distances = Vec::new();

        match &query.query_type {
            QueryType::InBox(bbox) => {
                for obj in self.objects.values() {
                    if bbox.intersects(&obj.bounds) {
                        objects.push(obj.clone());
                    }
                }
            }
            QueryType::InRadius { center, radius } => {
                for obj in self.objects.values() {
                    let dist = center.distance(&obj.position);
                    if dist <= *radius {
                        objects.push(obj.clone());
                        distances.push(dist);
                    }
                }
            }
            QueryType::NearestTo { point, k } => {
                let mut all: Vec<_> = self.objects.values()
                    .map(|obj| (obj.clone(), point.distance(&obj.position)))
                    .collect();

                all.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

                for (obj, dist) in all.into_iter().take(*k) {
                    objects.push(obj);
                    distances.push(dist);
                }
            }
            QueryType::WithRelation { object, relation } => {
                let obj_ids: Vec<u64> = self.objects.keys().copied().collect();

                for other_id in obj_ids {
                    if other_id == *object {
                        continue;
                    }

                    if let Some(rel) = self.relation(*object, other_id) {
                        if rel == *relation {
                            if let Some(obj) = self.objects.get(&other_id) {
                                objects.push(obj.clone());
                            }
                        }
                    }
                }
            }
        }

        // Apply limit
        if let Some(limit) = query.limit {
            objects.truncate(limit);
            distances.truncate(limit);
        }

        SpatialQueryResult { objects, distances }
    }

    /// Create region
    pub fn create_region(&mut self, name: &str, bounds: BoundingBox) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let region = Region {
            id,
            name: name.into(),
            bounds,
            objects: Vec::new(),
            children: Vec::new(),
        };

        self.regions.insert(id, region);

        id
    }

    /// Add object to region
    #[inline]
    pub fn add_to_region(&mut self, region_id: u64, object_id: u64) {
        if let Some(region) = self.regions.get_mut(&region_id) {
            if !region.objects.contains(&object_id) {
                region.objects.push(object_id);
            }
        }
    }

    /// Get object
    #[inline(always)]
    pub fn get_object(&self, id: u64) -> Option<&SpatialObject> {
        self.objects.get(&id)
    }

    /// Get region
    #[inline(always)]
    pub fn get_region(&self, id: u64) -> Option<&Region> {
        self.regions.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SpatialStats {
        &self.stats
    }
}

impl Default for SpatialReasoner {
    fn default() -> Self {
        Self::new(SpatialConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_object() {
        let mut reasoner = SpatialReasoner::default();

        let id = reasoner.create_object("box", Point::origin(), Shape::Box {
            dimensions: Point::new(10.0, 10.0, 10.0),
        });

        assert!(reasoner.get_object(id).is_some());
    }

    #[test]
    fn test_distance() {
        let p1 = Point::origin();
        let p2 = Point::new(3.0, 4.0, 0.0);

        assert!((p1.distance(&p2) - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(
            Point::new(0.0, 0.0, 0.0),
            Point::new(10.0, 10.0, 10.0),
        );

        assert!(bbox.contains(&Point::new(5.0, 5.0, 5.0)));
        assert!(!bbox.contains(&Point::new(15.0, 5.0, 5.0)));
    }

    #[test]
    fn test_relation_near() {
        let mut reasoner = SpatialReasoner::new(SpatialConfig {
            near_threshold: 10.0,
            ..Default::default()
        });

        let o1 = reasoner.create_object("a", Point::origin(), Shape::Point);
        let o2 = reasoner.create_object("b", Point::new(5.0, 0.0, 0.0), Shape::Point);

        let rel = reasoner.relation(o1, o2).unwrap();
        assert_eq!(rel, SpatialRelation::Near);
    }

    #[test]
    fn test_query_in_radius() {
        let mut reasoner = SpatialReasoner::default();

        reasoner.create_object("a", Point::origin(), Shape::Point);
        reasoner.create_object("b", Point::new(5.0, 0.0, 0.0), Shape::Point);
        reasoner.create_object("c", Point::new(100.0, 0.0, 0.0), Shape::Point);

        let result = reasoner.query(&SpatialQuery {
            query_type: QueryType::InRadius {
                center: Point::origin(),
                radius: 10.0,
            },
            limit: None,
        });

        assert_eq!(result.objects.len(), 2);
    }

    #[test]
    fn test_nearest() {
        let mut reasoner = SpatialReasoner::default();

        reasoner.create_object("a", Point::new(10.0, 0.0, 0.0), Shape::Point);
        reasoner.create_object("b", Point::new(5.0, 0.0, 0.0), Shape::Point);
        reasoner.create_object("c", Point::new(20.0, 0.0, 0.0), Shape::Point);

        let result = reasoner.query(&SpatialQuery {
            query_type: QueryType::NearestTo {
                point: Point::origin(),
                k: 2,
            },
            limit: None,
        });

        assert_eq!(result.objects.len(), 2);
        assert_eq!(result.objects[0].position.x, 5.0); // Nearest first
    }

    #[test]
    fn test_region() {
        let mut reasoner = SpatialReasoner::default();

        let obj = reasoner.create_object("box", Point::new(5.0, 5.0, 5.0), Shape::Point);

        let region = reasoner.create_region("room", BoundingBox::new(
            Point::origin(),
            Point::new(10.0, 10.0, 10.0),
        ));

        reasoner.add_to_region(region, obj);

        let r = reasoner.get_region(region).unwrap();
        assert!(r.objects.contains(&obj));
    }
}
