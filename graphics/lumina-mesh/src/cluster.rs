//! Cluster Hierarchy System
//!
//! Hierarchical spatial structure for efficient GPU-driven culling.
//! Clusters group meshlets for coarse culling before fine-grained
//! meshlet visibility testing.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::mesh::{BoundingSphere, AABB};
use crate::meshlet::{Meshlet, MeshletBounds};

// ============================================================================
// Cluster Bounds
// ============================================================================

/// Bounding volume for a cluster.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterBounds {
    /// AABB minimum.
    pub aabb_min: [f32; 3],
    /// Padding.
    _pad0: f32,
    /// AABB maximum.
    pub aabb_max: [f32; 3],
    /// Padding.
    _pad1: f32,
    /// Bounding sphere center.
    pub sphere_center: [f32; 3],
    /// Bounding sphere radius.
    pub sphere_radius: f32,
    /// Normal cone axis.
    pub cone_axis: [f32; 3],
    /// Normal cone cutoff (dot product).
    pub cone_cutoff: f32,
}

impl ClusterBounds {
    /// Size in bytes.
    pub const SIZE: usize = 64;

    /// Create from AABB.
    pub fn from_aabb(aabb: &AABB) -> Self {
        let center = aabb.center();
        let sphere = BoundingSphere::from_aabb(aabb);

        Self {
            aabb_min: aabb.min,
            _pad0: 0.0,
            aabb_max: aabb.max,
            _pad1: 0.0,
            sphere_center: center,
            sphere_radius: sphere.radius,
            cone_axis: [0.0, 1.0, 0.0],
            cone_cutoff: -1.0, // No cone culling by default
        }
    }

    /// Create from meshlet bounds.
    pub fn from_meshlets(bounds: &[MeshletBounds]) -> Self {
        if bounds.is_empty() {
            return Self::default();
        }

        // Compute combined AABB
        let mut aabb_min = [f32::MAX; 3];
        let mut aabb_max = [f32::MIN; 3];
        let mut cone_sum = [0.0f32; 3];

        for b in bounds {
            for i in 0..3 {
                aabb_min[i] = aabb_min[i].min(b.aabb_min[i]);
                aabb_max[i] = aabb_max[i].max(b.aabb_max[i]);
            }
            cone_sum[0] += b.cone_axis[0];
            cone_sum[1] += b.cone_axis[1];
            cone_sum[2] += b.cone_axis[2];
        }

        let center = [
            (aabb_min[0] + aabb_max[0]) * 0.5,
            (aabb_min[1] + aabb_max[1]) * 0.5,
            (aabb_min[2] + aabb_max[2]) * 0.5,
        ];

        // Compute radius
        let mut radius = 0.0f32;
        for b in bounds {
            let dx = b.center[0] - center[0];
            let dy = b.center[1] - center[1];
            let dz = b.center[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt() + b.radius;
            radius = radius.max(dist);
        }

        // Normalize cone axis
        let len =
            (cone_sum[0] * cone_sum[0] + cone_sum[1] * cone_sum[1] + cone_sum[2] * cone_sum[2])
                .sqrt();
        let cone_axis = if len > 0.0 {
            [cone_sum[0] / len, cone_sum[1] / len, cone_sum[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };

        // Compute worst-case cone cutoff
        let mut min_dot = 1.0f32;
        for b in bounds {
            let dot = b.cone_axis[0] * cone_axis[0]
                + b.cone_axis[1] * cone_axis[1]
                + b.cone_axis[2] * cone_axis[2];
            min_dot = min_dot.min(dot * b.cone_cutoff);
        }

        Self {
            aabb_min,
            _pad0: 0.0,
            aabb_max,
            _pad1: 0.0,
            sphere_center: center,
            sphere_radius: radius,
            cone_axis,
            cone_cutoff: min_dot,
        }
    }

    /// Test frustum visibility.
    pub fn is_in_frustum(&self, planes: &[[f32; 4]; 6]) -> bool {
        // Test sphere against each plane
        for plane in planes {
            let dist = plane[0] * self.sphere_center[0]
                + plane[1] * self.sphere_center[1]
                + plane[2] * self.sphere_center[2]
                + plane[3];

            if dist < -self.sphere_radius {
                return false;
            }
        }
        true
    }

    /// Test cone visibility (backface culling).
    pub fn is_cone_visible(&self, view_pos: [f32; 3]) -> bool {
        if self.cone_cutoff <= -1.0 {
            return true; // No cone culling
        }

        let dx = view_pos[0] - self.sphere_center[0];
        let dy = view_pos[1] - self.sphere_center[1];
        let dz = view_pos[2] - self.sphere_center[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();

        if len < 0.001 {
            return true;
        }

        let dir = [dx / len, dy / len, dz / len];
        let dot =
            dir[0] * self.cone_axis[0] + dir[1] * self.cone_axis[1] + dir[2] * self.cone_axis[2];

        dot >= self.cone_cutoff
    }
}

// ============================================================================
// Cluster
// ============================================================================

/// A cluster of meshlets.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Cluster ID.
    pub id: u32,
    /// Parent cluster ID (u32::MAX if root).
    pub parent: u32,
    /// Child cluster IDs.
    pub children: Vec<u32>,
    /// Meshlet indices.
    pub meshlets: Vec<u32>,
    /// Bounds.
    pub bounds: ClusterBounds,
    /// LOD level.
    pub lod_level: u8,
    /// Is visible this frame.
    pub visible: bool,
}

impl Cluster {
    /// Create a new cluster.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            parent: u32::MAX,
            children: Vec::new(),
            meshlets: Vec::new(),
            bounds: ClusterBounds::default(),
            lod_level: 0,
            visible: false,
        }
    }

    /// Check if this is a root cluster.
    pub fn is_root(&self) -> bool {
        self.parent == u32::MAX
    }

    /// Check if this is a leaf cluster.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Add a meshlet.
    pub fn add_meshlet(&mut self, meshlet_id: u32) {
        self.meshlets.push(meshlet_id);
    }

    /// Add a child cluster.
    pub fn add_child(&mut self, child_id: u32) {
        self.children.push(child_id);
    }

    /// Get meshlet count.
    pub fn meshlet_count(&self) -> usize {
        self.meshlets.len()
    }

    /// Test visibility.
    pub fn test_visibility(&self, view_pos: [f32; 3], frustum: &[[f32; 4]; 6]) -> bool {
        self.bounds.is_in_frustum(frustum) && self.bounds.is_cone_visible(view_pos)
    }
}

// ============================================================================
// Cluster Node (GPU-ready)
// ============================================================================

/// GPU-ready cluster node data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterNode {
    /// Bounds.
    pub bounds: ClusterBounds,
    /// Meshlet offset.
    pub meshlet_offset: u32,
    /// Meshlet count.
    pub meshlet_count: u32,
    /// First child offset (0 if leaf).
    pub child_offset: u32,
    /// Child count.
    pub child_count: u32,
    /// Parent index.
    pub parent: u32,
    /// LOD level.
    pub lod_level: u32,
    /// Flags.
    pub flags: u32,
    /// Padding.
    _pad: u32,
}

impl ClusterNode {
    /// Size in bytes.
    pub const SIZE: usize = ClusterBounds::SIZE + 32;

    /// Flag: is leaf node.
    pub const FLAG_LEAF: u32 = 1 << 0;
    /// Flag: is root node.
    pub const FLAG_ROOT: u32 = 1 << 1;
    /// Flag: is visible.
    pub const FLAG_VISIBLE: u32 = 1 << 2;

    /// Create from cluster.
    pub fn from_cluster(cluster: &Cluster, meshlet_offset: u32, child_offset: u32) -> Self {
        let mut flags = 0u32;
        if cluster.is_leaf() {
            flags |= Self::FLAG_LEAF;
        }
        if cluster.is_root() {
            flags |= Self::FLAG_ROOT;
        }
        if cluster.visible {
            flags |= Self::FLAG_VISIBLE;
        }

        Self {
            bounds: cluster.bounds,
            meshlet_offset,
            meshlet_count: cluster.meshlets.len() as u32,
            child_offset,
            child_count: cluster.children.len() as u32,
            parent: cluster.parent,
            lod_level: cluster.lod_level as u32,
            flags,
            _pad: 0,
        }
    }

    /// Check if leaf.
    pub fn is_leaf(&self) -> bool {
        self.flags & Self::FLAG_LEAF != 0
    }

    /// Check if root.
    pub fn is_root(&self) -> bool {
        self.flags & Self::FLAG_ROOT != 0
    }
}

// ============================================================================
// Cluster Cull Data
// ============================================================================

/// Data needed for GPU-driven cluster culling.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterCullData {
    /// Sphere center and radius.
    pub sphere: [f32; 4],
    /// Cone apex.
    pub cone_apex: [f32; 4],
    /// Cone axis (xyz) and cutoff (w).
    pub cone: [f32; 4],
    /// Meshlet range (offset, count, lod, flags).
    pub range: [u32; 4],
}

impl ClusterCullData {
    /// Size in bytes.
    pub const SIZE: usize = 64;

    /// Create from cluster.
    pub fn from_cluster(cluster: &Cluster, meshlet_offset: u32) -> Self {
        let apex = [
            cluster.bounds.sphere_center[0]
                - cluster.bounds.cone_axis[0] * cluster.bounds.sphere_radius,
            cluster.bounds.sphere_center[1]
                - cluster.bounds.cone_axis[1] * cluster.bounds.sphere_radius,
            cluster.bounds.sphere_center[2]
                - cluster.bounds.cone_axis[2] * cluster.bounds.sphere_radius,
        ];

        Self {
            sphere: [
                cluster.bounds.sphere_center[0],
                cluster.bounds.sphere_center[1],
                cluster.bounds.sphere_center[2],
                cluster.bounds.sphere_radius,
            ],
            cone_apex: [apex[0], apex[1], apex[2], 0.0],
            cone: [
                cluster.bounds.cone_axis[0],
                cluster.bounds.cone_axis[1],
                cluster.bounds.cone_axis[2],
                cluster.bounds.cone_cutoff,
            ],
            range: [
                meshlet_offset,
                cluster.meshlets.len() as u32,
                cluster.lod_level as u32,
                if cluster.is_leaf() { 1 } else { 0 },
            ],
        }
    }
}

// ============================================================================
// Cluster Hierarchy
// ============================================================================

/// Complete cluster hierarchy for a mesh.
#[derive(Debug, Clone)]
pub struct ClusterHierarchy {
    /// All clusters.
    pub clusters: Vec<Cluster>,
    /// Root cluster indices.
    pub roots: Vec<u32>,
    /// Total bounds.
    pub bounds: ClusterBounds,
    /// Depth of hierarchy.
    pub depth: u32,
}

impl ClusterHierarchy {
    /// Create a new empty hierarchy.
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            roots: Vec::new(),
            bounds: ClusterBounds::default(),
            depth: 0,
        }
    }

    /// Add a cluster.
    pub fn add_cluster(&mut self, cluster: Cluster) -> u32 {
        let id = self.clusters.len() as u32;
        if cluster.is_root() {
            self.roots.push(id);
        }
        self.clusters.push(cluster);
        id
    }

    /// Get cluster by ID.
    pub fn get(&self, id: u32) -> Option<&Cluster> {
        self.clusters.get(id as usize)
    }

    /// Get mutable cluster.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Cluster> {
        self.clusters.get_mut(id as usize)
    }

    /// Get cluster count.
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// Update all cluster bounds.
    pub fn update_bounds(&mut self) {
        // Process bottom-up to propagate bounds
        // First pass: find max depth
        let mut max_depth = 0u32;
        for cluster in &self.clusters {
            max_depth = max_depth.max(cluster.lod_level as u32);
        }
        self.depth = max_depth + 1;

        // Process each level from leaves to root
        for level in 0..=max_depth {
            let current_level = max_depth - level;
            for i in 0..self.clusters.len() {
                if self.clusters[i].lod_level as u32 != current_level {
                    continue;
                }

                // If this cluster has children, combine their bounds
                if !self.clusters[i].children.is_empty() {
                    let child_ids: Vec<u32> = self.clusters[i].children.clone();
                    let mut combined_min = [f32::MAX; 3];
                    let mut combined_max = [f32::MIN; 3];

                    for child_id in child_ids {
                        if let Some(child) = self.clusters.get(child_id as usize) {
                            for j in 0..3 {
                                combined_min[j] = combined_min[j].min(child.bounds.aabb_min[j]);
                                combined_max[j] = combined_max[j].max(child.bounds.aabb_max[j]);
                            }
                        }
                    }

                    let aabb = AABB::new(combined_min, combined_max);
                    self.clusters[i].bounds = ClusterBounds::from_aabb(&aabb);
                }
            }
        }

        // Update total bounds
        if !self.roots.is_empty() {
            let mut combined_min = [f32::MAX; 3];
            let mut combined_max = [f32::MIN; 3];

            for &root_id in &self.roots {
                if let Some(root) = self.clusters.get(root_id as usize) {
                    for j in 0..3 {
                        combined_min[j] = combined_min[j].min(root.bounds.aabb_min[j]);
                        combined_max[j] = combined_max[j].max(root.bounds.aabb_max[j]);
                    }
                }
            }

            let aabb = AABB::new(combined_min, combined_max);
            self.bounds = ClusterBounds::from_aabb(&aabb);
        }
    }

    /// Perform hierarchical frustum culling.
    pub fn cull(&mut self, view_pos: [f32; 3], frustum: &[[f32; 4]; 6]) -> Vec<u32> {
        let mut visible_clusters = Vec::new();
        let mut queue: Vec<u32> = self.roots.clone();

        while let Some(cluster_id) = queue.pop() {
            let cluster = &mut self.clusters[cluster_id as usize];

            if cluster.test_visibility(view_pos, frustum) {
                cluster.visible = true;

                if cluster.is_leaf() {
                    visible_clusters.push(cluster_id);
                } else {
                    // Add children to queue
                    for &child_id in &cluster.children.clone() {
                        queue.push(child_id);
                    }
                }
            } else {
                cluster.visible = false;
            }
        }

        visible_clusters
    }

    /// Get all visible meshlet indices.
    pub fn get_visible_meshlets(&self) -> Vec<u32> {
        let mut meshlets = Vec::new();

        for cluster in &self.clusters {
            if cluster.visible && cluster.is_leaf() {
                meshlets.extend_from_slice(&cluster.meshlets);
            }
        }

        meshlets
    }

    /// Build GPU node buffer.
    pub fn build_node_buffer(&self) -> Vec<ClusterNode> {
        let mut nodes = Vec::with_capacity(self.clusters.len());
        let mut meshlet_offset = 0u32;
        let mut child_offset = 0u32;

        for cluster in &self.clusters {
            nodes.push(ClusterNode::from_cluster(
                cluster,
                meshlet_offset,
                child_offset,
            ));
            meshlet_offset += cluster.meshlets.len() as u32;
            child_offset += cluster.children.len() as u32;
        }

        nodes
    }

    /// Build GPU cull data buffer.
    pub fn build_cull_buffer(&self) -> Vec<ClusterCullData> {
        let mut cull_data = Vec::with_capacity(self.clusters.len());
        let mut meshlet_offset = 0u32;

        for cluster in &self.clusters {
            cull_data.push(ClusterCullData::from_cluster(cluster, meshlet_offset));
            meshlet_offset += cluster.meshlets.len() as u32;
        }

        cull_data
    }
}

impl Default for ClusterHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cluster Tree Builder
// ============================================================================

/// Configuration for cluster tree building.
#[derive(Debug, Clone)]
pub struct ClusterTreeConfig {
    /// Target meshlets per cluster.
    pub meshlets_per_cluster: usize,
    /// Maximum children per node.
    pub max_children: usize,
    /// Build method.
    pub build_method: ClusterBuildMethod,
}

/// Build method for cluster tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClusterBuildMethod {
    /// Spatial median split.
    #[default]
    SpatialMedian,
    /// Surface area heuristic.
    SAH,
    /// Linear BVH.
    LBVH,
}

impl Default for ClusterTreeConfig {
    fn default() -> Self {
        Self {
            meshlets_per_cluster: 32,
            max_children: 4,
            build_method: ClusterBuildMethod::SpatialMedian,
        }
    }
}

/// Cluster tree builder.
pub struct ClusterTree {
    config: ClusterTreeConfig,
}

impl ClusterTree {
    /// Create a new builder.
    pub fn new(config: ClusterTreeConfig) -> Self {
        Self { config }
    }

    /// Build hierarchy from meshlets.
    pub fn build(&self, meshlets: &[Meshlet], bounds: &[MeshletBounds]) -> ClusterHierarchy {
        if meshlets.is_empty() {
            return ClusterHierarchy::new();
        }

        let mut hierarchy = ClusterHierarchy::new();
        let meshlet_count = meshlets.len();

        // Group meshlets into leaf clusters
        let leaf_clusters = self.build_leaf_clusters(meshlet_count, bounds);

        // Add leaf clusters
        let leaf_ids: Vec<u32> = leaf_clusters
            .into_iter()
            .map(|c| hierarchy.add_cluster(c))
            .collect();

        // Build hierarchy recursively
        if leaf_ids.len() > 1 {
            self.build_hierarchy_recursive(&mut hierarchy, &leaf_ids, 1);
        }

        hierarchy.update_bounds();
        hierarchy
    }

    /// Build leaf clusters from meshlets.
    fn build_leaf_clusters(&self, meshlet_count: usize, bounds: &[MeshletBounds]) -> Vec<Cluster> {
        let mut clusters = Vec::new();
        let meshlets_per_cluster = self.config.meshlets_per_cluster;

        // Simple grouping - could be improved with spatial partitioning
        for chunk_start in (0..meshlet_count).step_by(meshlets_per_cluster) {
            let chunk_end = (chunk_start + meshlets_per_cluster).min(meshlet_count);
            let chunk_bounds: Vec<_> = bounds[chunk_start..chunk_end].to_vec();

            let mut cluster = Cluster::new(clusters.len() as u32);
            cluster.lod_level = 0;

            for i in chunk_start..chunk_end {
                cluster.add_meshlet(i as u32);
            }

            cluster.bounds = ClusterBounds::from_meshlets(&chunk_bounds);
            clusters.push(cluster);
        }

        clusters
    }

    /// Build hierarchy recursively from clusters.
    fn build_hierarchy_recursive(
        &self,
        hierarchy: &mut ClusterHierarchy,
        cluster_ids: &[u32],
        lod_level: u8,
    ) {
        if cluster_ids.len() <= 1 {
            return;
        }

        let max_children = self.config.max_children;
        let mut parent_ids = Vec::new();

        // Group clusters into parents
        for chunk in cluster_ids.chunks(max_children) {
            let mut parent = Cluster::new(hierarchy.clusters.len() as u32);
            parent.lod_level = lod_level;

            for &child_id in chunk {
                parent.add_child(child_id);
                if let Some(child) = hierarchy.get_mut(child_id) {
                    child.parent = parent.id;
                }
            }

            // Combine bounds from children
            let child_bounds: Vec<ClusterBounds> = chunk
                .iter()
                .filter_map(|&id| hierarchy.get(id).map(|c| c.bounds))
                .collect();

            parent.bounds = self.combine_bounds(&child_bounds);
            parent_ids.push(hierarchy.add_cluster(parent));
        }

        // Recurse if we still have multiple clusters
        if parent_ids.len() > 1 {
            self.build_hierarchy_recursive(hierarchy, &parent_ids, lod_level + 1);
        }
    }

    /// Combine multiple cluster bounds.
    fn combine_bounds(&self, bounds: &[ClusterBounds]) -> ClusterBounds {
        if bounds.is_empty() {
            return ClusterBounds::default();
        }

        let mut aabb_min = [f32::MAX; 3];
        let mut aabb_max = [f32::MIN; 3];

        for b in bounds {
            for i in 0..3 {
                aabb_min[i] = aabb_min[i].min(b.aabb_min[i]);
                aabb_max[i] = aabb_max[i].max(b.aabb_max[i]);
            }
        }

        let aabb = AABB::new(aabb_min, aabb_max);
        ClusterBounds::from_aabb(&aabb)
    }
}

impl Default for ClusterTree {
    fn default() -> Self {
        Self::new(ClusterTreeConfig::default())
    }
}
