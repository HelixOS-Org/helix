//! Virtual Geometry System
//!
//! Nanite-inspired virtual geometry system that automatically manages
//! LOD selection and streaming based on screen-space error. Enables
//! rendering of extremely detailed geometry with constant memory usage.

use alloc::{string::String, vec::Vec, collections::BTreeMap, boxed::Box};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::mesh::{MeshHandle, AABB, BoundingSphere};
use crate::meshlet::{Meshlet, MeshletBounds, MeshletData, GpuMeshlet, GpuMeshletBounds};

// ============================================================================
// Virtual Geometry Handle
// ============================================================================

/// Handle to a virtual geometry resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualMeshHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl VirtualMeshHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Virtual Geometry Node
// ============================================================================

/// A node in the virtual geometry hierarchy.
/// Each node represents a cluster of geometry at a specific LOD.
#[derive(Debug, Clone)]
pub struct VirtualGeometryNode {
    /// Node ID.
    pub id: u32,
    /// Parent node ID (u32::MAX if root).
    pub parent: u32,
    /// Child node IDs.
    pub children: Vec<u32>,
    /// LOD level (0 = highest detail).
    pub lod_level: u8,
    /// Meshlet offset in the meshlet array.
    pub meshlet_offset: u32,
    /// Meshlet count.
    pub meshlet_count: u32,
    /// Bounding sphere.
    pub bounds: BoundingSphere,
    /// Maximum screen-space error for this node.
    pub max_error: f32,
    /// Parent error (error threshold to switch to parent).
    pub parent_error: f32,
    /// Page ID for streaming.
    pub page_id: u32,
    /// Is currently resident in GPU memory.
    pub is_resident: bool,
}

impl VirtualGeometryNode {
    /// Check if this is a root node.
    pub fn is_root(&self) -> bool {
        self.parent == u32::MAX
    }

    /// Check if this is a leaf node.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Calculate screen-space error for this node.
    pub fn screen_error(&self, view_pos: [f32; 3], fov_y: f32, screen_height: f32) -> f32 {
        let dx = view_pos[0] - self.bounds.center[0];
        let dy = view_pos[1] - self.bounds.center[1];
        let dz = view_pos[2] - self.bounds.center[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);

        // Project error to screen space
        let cot_half_fov = 1.0 / (fov_y * 0.5).tan();
        let projected_error = self.max_error * cot_half_fov / distance;

        projected_error * screen_height * 0.5
    }

    /// Check if node should be rendered or refined.
    pub fn should_refine(&self, screen_error: f32, error_threshold: f32) -> bool {
        !self.is_leaf() && screen_error > error_threshold
    }
}

// ============================================================================
// Virtual Page
// ============================================================================

/// A page of virtual geometry data that can be streamed.
#[derive(Debug, Clone)]
pub struct VirtualPage {
    /// Page ID.
    pub id: u32,
    /// Nodes in this page.
    pub nodes: Vec<u32>,
    /// Meshlet data.
    pub meshlet_data: MeshletData,
    /// Page state.
    pub state: PageState,
    /// Priority for streaming.
    pub priority: f32,
    /// Last frame used.
    pub last_used_frame: u64,
    /// Size in bytes.
    pub size_bytes: u64,
}

/// Page state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PageState {
    /// Not loaded.
    #[default]
    NotLoaded,
    /// Loading in progress.
    Loading,
    /// Loaded and ready.
    Resident,
    /// Pending eviction.
    PendingEviction,
}

/// Page request for streaming.
#[derive(Debug, Clone)]
pub struct PageRequest {
    /// Page ID.
    pub page_id: u32,
    /// Priority.
    pub priority: StreamingPriority,
    /// Screen-space error.
    pub screen_error: f32,
    /// Distance to camera.
    pub distance: f32,
}

/// Streaming priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StreamingPriority {
    /// Critical - needed immediately.
    Critical = 0,
    /// High priority.
    High = 1,
    /// Normal priority.
    Normal = 2,
    /// Low priority.
    Low = 3,
    /// Background loading.
    Background = 4,
}

// ============================================================================
// Virtual Mesh
// ============================================================================

/// A complete virtual geometry mesh with hierarchical LOD.
pub struct VirtualMesh {
    /// Handle.
    handle: VirtualMeshHandle,
    /// Name.
    name: String,
    /// Source mesh (highest LOD).
    source_mesh: MeshHandle,
    /// All nodes in the hierarchy.
    nodes: Vec<VirtualGeometryNode>,
    /// Root nodes (multiple for multi-part meshes).
    root_nodes: Vec<u32>,
    /// Pages for streaming.
    pages: Vec<VirtualPage>,
    /// Total bounding box.
    bounds: AABB,
    /// Total bounding sphere.
    sphere: BoundingSphere,
    /// LOD level count.
    lod_count: u8,
    /// Error threshold for LOD switching.
    error_threshold: f32,
    /// Statistics.
    stats: VirtualMeshStats,
}

/// Virtual mesh statistics.
#[derive(Debug, Clone, Default)]
pub struct VirtualMeshStats {
    /// Total node count.
    pub node_count: u32,
    /// Total meshlet count (all LODs).
    pub total_meshlets: u32,
    /// Total triangle count (all LODs).
    pub total_triangles: u64,
    /// LOD 0 triangle count.
    pub lod0_triangles: u64,
    /// Page count.
    pub page_count: u32,
    /// Resident pages.
    pub resident_pages: u32,
    /// Total memory (all LODs).
    pub total_memory: u64,
    /// Resident memory.
    pub resident_memory: u64,
}

/// Virtual mesh description.
#[derive(Debug, Clone)]
pub struct VirtualMeshDesc {
    /// Name.
    pub name: String,
    /// Source mesh.
    pub source_mesh: MeshHandle,
    /// Maximum LOD levels.
    pub max_lod_levels: u8,
    /// Error threshold (pixels).
    pub error_threshold: f32,
    /// Target triangles per node.
    pub triangles_per_node: u32,
    /// Page size (bytes).
    pub page_size: u64,
}

impl Default for VirtualMeshDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            source_mesh: MeshHandle::INVALID,
            max_lod_levels: 8,
            error_threshold: 1.0, // 1 pixel
            triangles_per_node: 128,
            page_size: 64 * 1024, // 64KB
        }
    }
}

impl VirtualMesh {
    /// Create a new virtual mesh.
    pub fn new(handle: VirtualMeshHandle, desc: VirtualMeshDesc) -> Self {
        Self {
            handle,
            name: desc.name,
            source_mesh: desc.source_mesh,
            nodes: Vec::new(),
            root_nodes: Vec::new(),
            pages: Vec::new(),
            bounds: AABB::INVALID,
            sphere: BoundingSphere::default(),
            lod_count: 0,
            error_threshold: desc.error_threshold,
            stats: VirtualMeshStats::default(),
        }
    }

    /// Get handle.
    pub fn handle(&self) -> VirtualMeshHandle {
        self.handle
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get source mesh.
    pub fn source_mesh(&self) -> MeshHandle {
        self.source_mesh
    }

    /// Get nodes.
    pub fn nodes(&self) -> &[VirtualGeometryNode] {
        &self.nodes
    }

    /// Get node by ID.
    pub fn get_node(&self, id: u32) -> Option<&VirtualGeometryNode> {
        self.nodes.get(id as usize)
    }

    /// Get mutable node.
    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut VirtualGeometryNode> {
        self.nodes.get_mut(id as usize)
    }

    /// Get root nodes.
    pub fn root_nodes(&self) -> &[u32] {
        &self.root_nodes
    }

    /// Get pages.
    pub fn pages(&self) -> &[VirtualPage] {
        &self.pages
    }

    /// Get mutable page.
    pub fn get_page_mut(&mut self, id: u32) -> Option<&mut VirtualPage> {
        self.pages.get_mut(id as usize)
    }

    /// Get bounds.
    pub fn bounds(&self) -> &AABB {
        &self.bounds
    }

    /// Get bounding sphere.
    pub fn bounding_sphere(&self) -> &BoundingSphere {
        &self.sphere
    }

    /// Get LOD count.
    pub fn lod_count(&self) -> u8 {
        self.lod_count
    }

    /// Get error threshold.
    pub fn error_threshold(&self) -> f32 {
        self.error_threshold
    }

    /// Get statistics.
    pub fn stats(&self) -> &VirtualMeshStats {
        &self.stats
    }

    /// Add a node.
    pub fn add_node(&mut self, node: VirtualGeometryNode) -> u32 {
        let id = self.nodes.len() as u32;
        if node.is_root() {
            self.root_nodes.push(id);
        }
        self.lod_count = self.lod_count.max(node.lod_level + 1);
        self.nodes.push(node);
        id
    }

    /// Add a page.
    pub fn add_page(&mut self, page: VirtualPage) -> u32 {
        let id = self.pages.len() as u32;
        self.pages.push(page);
        id
    }

    /// Calculate bounds from nodes.
    pub fn calculate_bounds(&mut self) {
        self.bounds = AABB::INVALID;
        for node in &self.nodes {
            let node_aabb = AABB::from_center_extents(
                node.bounds.center,
                [node.bounds.radius, node.bounds.radius, node.bounds.radius],
            );
            self.bounds.expand_aabb(&node_aabb);
        }
        self.sphere = BoundingSphere::from_aabb(&self.bounds);
    }

    /// Update statistics.
    pub fn update_stats(&mut self) {
        self.stats.node_count = self.nodes.len() as u32;
        self.stats.page_count = self.pages.len() as u32;
        
        self.stats.total_meshlets = 0;
        self.stats.total_triangles = 0;
        self.stats.lod0_triangles = 0;
        self.stats.resident_pages = 0;
        self.stats.total_memory = 0;
        self.stats.resident_memory = 0;

        for node in &self.nodes {
            self.stats.total_meshlets += node.meshlet_count;
            if node.lod_level == 0 {
                // Estimate triangles from meshlets
                self.stats.lod0_triangles += (node.meshlet_count as u64) * 126;
            }
        }

        for page in &self.pages {
            self.stats.total_memory += page.size_bytes;
            if page.state == PageState::Resident {
                self.stats.resident_pages += 1;
                self.stats.resident_memory += page.size_bytes;
            }
        }

        self.stats.total_triangles = self.stats.lod0_triangles * 2; // Rough estimate
    }

    /// Select nodes for rendering based on view parameters.
    pub fn select_nodes(
        &self,
        view_pos: [f32; 3],
        fov_y: f32,
        screen_height: f32,
    ) -> VirtualGeometrySelection {
        let mut selection = VirtualGeometrySelection::new();

        // Process nodes in breadth-first order
        let mut queue: Vec<u32> = self.root_nodes.clone();

        while let Some(node_id) = queue.pop() {
            let node = &self.nodes[node_id as usize];

            // Calculate screen-space error
            let screen_error = node.screen_error(view_pos, fov_y, screen_height);

            if node.should_refine(screen_error, self.error_threshold) {
                // Refine to children
                for &child_id in &node.children {
                    queue.push(child_id);
                }
            } else {
                // Use this node
                if node.is_resident {
                    selection.visible_nodes.push(node_id);
                    selection.visible_meshlet_count += node.meshlet_count;
                } else {
                    // Request page streaming
                    let dx = view_pos[0] - node.bounds.center[0];
                    let dy = view_pos[1] - node.bounds.center[1];
                    let dz = view_pos[2] - node.bounds.center[2];
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                    selection.page_requests.push(PageRequest {
                        page_id: node.page_id,
                        priority: if screen_error > self.error_threshold * 4.0 {
                            StreamingPriority::Critical
                        } else if screen_error > self.error_threshold * 2.0 {
                            StreamingPriority::High
                        } else {
                            StreamingPriority::Normal
                        },
                        screen_error,
                        distance,
                    });

                    // Use parent as fallback if available
                    if node.parent != u32::MAX && self.nodes[node.parent as usize].is_resident {
                        selection.fallback_nodes.push(node.parent);
                    }
                }
            }
        }

        selection
    }
}

/// Result of virtual geometry selection.
#[derive(Debug, Clone, Default)]
pub struct VirtualGeometrySelection {
    /// Visible node IDs.
    pub visible_nodes: Vec<u32>,
    /// Fallback node IDs (lower LOD when higher LOD not resident).
    pub fallback_nodes: Vec<u32>,
    /// Page requests for streaming.
    pub page_requests: Vec<PageRequest>,
    /// Total visible meshlet count.
    pub visible_meshlet_count: u32,
}

impl VirtualGeometrySelection {
    /// Create an empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total node count.
    pub fn node_count(&self) -> usize {
        self.visible_nodes.len() + self.fallback_nodes.len()
    }

    /// Get all nodes (visible + fallback).
    pub fn all_nodes(&self) -> impl Iterator<Item = u32> + '_ {
        self.visible_nodes.iter().copied().chain(self.fallback_nodes.iter().copied())
    }
}

// ============================================================================
// Virtual Geometry Builder
// ============================================================================

/// Builder for creating virtual geometry from a mesh.
pub struct VirtualGeometryBuilder {
    /// Configuration.
    config: VirtualGeometryConfig,
}

/// Configuration for virtual geometry generation.
#[derive(Debug, Clone)]
pub struct VirtualGeometryConfig {
    /// Maximum LOD levels.
    pub max_lod_levels: u8,
    /// Error threshold (pixels).
    pub error_threshold: f32,
    /// Target triangles per cluster.
    pub triangles_per_cluster: u32,
    /// Simplification ratio per LOD level.
    pub simplification_ratio: f32,
    /// Page size (bytes).
    pub page_size: u64,
    /// Group size for DAG construction.
    pub dag_group_size: u32,
}

impl Default for VirtualGeometryConfig {
    fn default() -> Self {
        Self {
            max_lod_levels: 8,
            error_threshold: 1.0,
            triangles_per_cluster: 128,
            simplification_ratio: 0.5,
            page_size: 64 * 1024,
            dag_group_size: 4,
        }
    }
}

impl VirtualGeometryBuilder {
    /// Create a new builder.
    pub fn new(config: VirtualGeometryConfig) -> Self {
        Self { config }
    }

    /// Build virtual geometry from meshlet mesh.
    pub fn build(
        &self,
        handle: VirtualMeshHandle,
        name: impl Into<String>,
        source_mesh: MeshHandle,
        meshlet_data: &MeshletData,
        bounds: &[MeshletBounds],
    ) -> VirtualMesh {
        let name = name.into();
        let desc = VirtualMeshDesc {
            name: name.clone(),
            source_mesh,
            max_lod_levels: self.config.max_lod_levels,
            error_threshold: self.config.error_threshold,
            triangles_per_node: self.config.triangles_per_cluster,
            page_size: self.config.page_size,
        };

        let mut mesh = VirtualMesh::new(handle, desc);

        // Build hierarchy from meshlets
        self.build_hierarchy(&mut mesh, meshlet_data, bounds);

        // Organize into pages
        self.build_pages(&mut mesh);

        mesh.calculate_bounds();
        mesh.update_stats();

        mesh
    }

    /// Build the node hierarchy.
    fn build_hierarchy(
        &self,
        mesh: &mut VirtualMesh,
        meshlet_data: &MeshletData,
        bounds: &[MeshletBounds],
    ) {
        let meshlet_count = meshlet_data.meshlets.len();
        if meshlet_count == 0 {
            return;
        }

        // LOD 0: Create leaf nodes from meshlets
        let mut lod0_nodes = Vec::with_capacity(meshlet_count);

        for (i, (meshlet, bound)) in meshlet_data.meshlets.iter().zip(bounds.iter()).enumerate() {
            let node = VirtualGeometryNode {
                id: mesh.nodes.len() as u32,
                parent: u32::MAX,
                children: Vec::new(),
                lod_level: 0,
                meshlet_offset: i as u32,
                meshlet_count: 1,
                bounds: BoundingSphere::new(bound.center, bound.radius),
                max_error: 0.0, // Highest detail, no error
                parent_error: f32::MAX,
                page_id: 0,
                is_resident: true, // LOD 0 always resident for now
            };

            lod0_nodes.push(mesh.add_node(node));
        }

        // Build higher LOD levels by grouping nodes
        let mut current_level = lod0_nodes;
        let mut lod_level = 1u8;

        while current_level.len() > 1 && lod_level < self.config.max_lod_levels {
            let group_size = self.config.dag_group_size as usize;
            let mut next_level = Vec::new();

            // Group nodes
            for chunk in current_level.chunks(group_size) {
                if chunk.len() == 1 {
                    // Single node, just carry forward
                    if let Some(node) = mesh.get_node_mut(chunk[0]) {
                        node.lod_level = lod_level;
                    }
                    next_level.push(chunk[0]);
                    continue;
                }

                // Calculate combined bounds and error
                let mut combined_center = [0.0f32; 3];
                let mut max_error = 0.0f32;

                for &child_id in chunk {
                    if let Some(child) = mesh.get_node(child_id) {
                        combined_center[0] += child.bounds.center[0];
                        combined_center[1] += child.bounds.center[1];
                        combined_center[2] += child.bounds.center[2];
                        max_error = max_error.max(child.max_error);
                    }
                }

                let count = chunk.len() as f32;
                combined_center[0] /= count;
                combined_center[1] /= count;
                combined_center[2] /= count;

                // Calculate radius to contain all children
                let mut radius = 0.0f32;
                for &child_id in chunk {
                    if let Some(child) = mesh.get_node(child_id) {
                        let dx = child.bounds.center[0] - combined_center[0];
                        let dy = child.bounds.center[1] - combined_center[1];
                        let dz = child.bounds.center[2] - combined_center[2];
                        let dist = (dx * dx + dy * dy + dz * dz).sqrt() + child.bounds.radius;
                        radius = radius.max(dist);
                    }
                }

                // Create parent node
                let parent_error = max_error * (1.0 + self.config.simplification_ratio);
                let parent_id = mesh.nodes.len() as u32;

                let parent_node = VirtualGeometryNode {
                    id: parent_id,
                    parent: u32::MAX,
                    children: chunk.to_vec(),
                    lod_level,
                    meshlet_offset: 0, // Will be updated during page building
                    meshlet_count: 0,  // Simplified meshlet data
                    bounds: BoundingSphere::new(combined_center, radius),
                    max_error: parent_error,
                    parent_error: f32::MAX,
                    page_id: lod_level as u32,
                    is_resident: false,
                };

                let parent_id = mesh.add_node(parent_node);

                // Update children to point to parent
                for &child_id in chunk {
                    if let Some(child) = mesh.get_node_mut(child_id) {
                        child.parent = parent_id;
                        child.parent_error = parent_error;
                    }
                }

                next_level.push(parent_id);
            }

            current_level = next_level;
            lod_level += 1;
        }

        // Mark remaining nodes as roots
        for &node_id in &current_level {
            if !mesh.root_nodes.contains(&node_id) {
                mesh.root_nodes.push(node_id);
            }
        }
    }

    /// Organize nodes into streamable pages.
    fn build_pages(&self, mesh: &mut VirtualMesh) {
        // Group nodes by LOD level into pages
        let mut lod_groups: BTreeMap<u8, Vec<u32>> = BTreeMap::new();

        for node in &mesh.nodes {
            lod_groups.entry(node.lod_level).or_default().push(node.id);
        }

        // Create pages for each LOD level
        for (lod, nodes) in lod_groups {
            let mut current_page_nodes = Vec::new();
            let mut current_size = 0u64;

            for node_id in nodes {
                let node = &mesh.nodes[node_id as usize];
                let node_size = (node.meshlet_count as u64) * 256; // Estimate

                if current_size + node_size > self.config.page_size && !current_page_nodes.is_empty() {
                    // Create page
                    let page_id = mesh.pages.len() as u32;
                    mesh.pages.push(VirtualPage {
                        id: page_id,
                        nodes: current_page_nodes.clone(),
                        meshlet_data: MeshletData::new(),
                        state: if lod == 0 { PageState::Resident } else { PageState::NotLoaded },
                        priority: 0.0,
                        last_used_frame: 0,
                        size_bytes: current_size,
                    });

                    // Update nodes
                    for &n in &current_page_nodes {
                        if let Some(node) = mesh.get_node_mut(n) {
                            node.page_id = page_id;
                        }
                    }

                    current_page_nodes.clear();
                    current_size = 0;
                }

                current_page_nodes.push(node_id);
                current_size += node_size;
            }

            // Create final page for this LOD
            if !current_page_nodes.is_empty() {
                let page_id = mesh.pages.len() as u32;
                mesh.pages.push(VirtualPage {
                    id: page_id,
                    nodes: current_page_nodes.clone(),
                    meshlet_data: MeshletData::new(),
                    state: if lod == 0 { PageState::Resident } else { PageState::NotLoaded },
                    priority: 0.0,
                    last_used_frame: 0,
                    size_bytes: current_size,
                });

                for &n in &current_page_nodes {
                    if let Some(node) = mesh.get_node_mut(n) {
                        node.page_id = page_id;
                    }
                }
            }
        }
    }
}

// ============================================================================
// Virtual Geometry Manager
// ============================================================================

/// Manages all virtual geometry resources.
pub struct VirtualGeometryManager {
    /// Virtual meshes.
    meshes: BTreeMap<u32, VirtualMesh>,
    /// Name map.
    name_map: BTreeMap<String, VirtualMeshHandle>,
    /// Next index.
    next_index: AtomicU32,
    /// Next generation.
    next_generation: AtomicU32,
    /// Current frame.
    current_frame: AtomicU64,
    /// Memory budget (bytes).
    memory_budget: u64,
    /// Current memory usage.
    memory_usage: AtomicU64,
    /// Builder config.
    builder_config: VirtualGeometryConfig,
}

impl VirtualGeometryManager {
    /// Create a new manager.
    pub fn new(memory_budget: u64) -> Self {
        Self {
            meshes: BTreeMap::new(),
            name_map: BTreeMap::new(),
            next_index: AtomicU32::new(0),
            next_generation: AtomicU32::new(1),
            current_frame: AtomicU64::new(0),
            memory_budget,
            memory_usage: AtomicU64::new(0),
            builder_config: VirtualGeometryConfig::default(),
        }
    }

    /// Create virtual geometry from meshlet data.
    pub fn create(
        &mut self,
        name: impl Into<String>,
        source_mesh: MeshHandle,
        meshlet_data: &MeshletData,
        bounds: &[MeshletBounds],
    ) -> VirtualMeshHandle {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        let handle = VirtualMeshHandle::new(index, generation);

        let name = name.into();
        let builder = VirtualGeometryBuilder::new(self.builder_config.clone());
        let mesh = builder.build(handle, name.clone(), source_mesh, meshlet_data, bounds);

        self.name_map.insert(name, handle);
        self.meshes.insert(index, mesh);

        handle
    }

    /// Get a virtual mesh.
    pub fn get(&self, handle: VirtualMeshHandle) -> Option<&VirtualMesh> {
        let mesh = self.meshes.get(&handle.index)?;
        if mesh.handle.generation == handle.generation {
            Some(mesh)
        } else {
            None
        }
    }

    /// Get mutable virtual mesh.
    pub fn get_mut(&mut self, handle: VirtualMeshHandle) -> Option<&mut VirtualMesh> {
        let mesh = self.meshes.get_mut(&handle.index)?;
        if mesh.handle.generation == handle.generation {
            Some(mesh)
        } else {
            None
        }
    }

    /// Get by name.
    pub fn get_by_name(&self, name: &str) -> Option<&VirtualMesh> {
        let handle = self.name_map.get(name)?;
        self.get(*handle)
    }

    /// Destroy a virtual mesh.
    pub fn destroy(&mut self, handle: VirtualMeshHandle) -> bool {
        if let Some(mesh) = self.meshes.remove(&handle.index) {
            if mesh.handle.generation == handle.generation {
                self.name_map.remove(&mesh.name);
                return true;
            }
        }
        false
    }

    /// Begin frame.
    pub fn begin_frame(&self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    /// Get memory budget.
    pub fn memory_budget(&self) -> u64 {
        self.memory_budget
    }

    /// Get current memory usage.
    pub fn memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Iterate over virtual meshes.
    pub fn iter(&self) -> impl Iterator<Item = &VirtualMesh> {
        self.meshes.values()
    }
}

impl Default for VirtualGeometryManager {
    fn default() -> Self {
        Self::new(256 * 1024 * 1024) // 256MB default budget
    }
}
