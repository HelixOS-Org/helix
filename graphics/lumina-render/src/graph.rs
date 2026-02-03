//! Render Graph - Automatic Resource and Barrier Management
//!
//! The render graph is a revolutionary approach to GPU resource management that:
//! - Automatically tracks resource lifetimes and inserts barriers
//! - Enables optimal resource aliasing to minimize memory usage
//! - Parallelizes independent passes across multiple queues
//! - Supports dynamic graph modification for adaptive rendering
//!
//! ## Key Innovations
//!
//! 1. **Transient Resources**: Automatic allocation and aliasing
//! 2. **Barrier Optimization**: Minimal barrier insertion with batching
//! 3. **Async Compute**: Automatic async compute scheduling
//! 4. **Graph Caching**: Compiled graphs for zero-overhead execution

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::barrier::{AccessFlags, Barrier, BarrierBatch, PipelineStage};
use crate::pass::{PassBuilder, PassContext, PassType, RenderPass};
use crate::resource::{
    BufferDesc, BufferHandle, ResourceHandle, ResourcePool, ResourceState, ResourceUsage,
    TextureDesc, TextureHandle,
};
use crate::scheduler::{QueueType, SubmitInfo};

/// Unique identifier for graph nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// Invalid node ID.
    pub const INVALID: Self = Self(u32::MAX);

    /// Create a new node ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Check if this is a valid node ID.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

/// Unique identifier for resources in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(u32);

impl ResourceId {
    /// Create a new resource ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// Virtual resource representing a texture in the graph.
#[derive(Debug, Clone)]
pub struct VirtualTexture {
    /// Resource identifier.
    pub id: ResourceId,
    /// Texture description.
    pub desc: TextureDesc,
    /// Name for debugging.
    pub name: String,
    /// Whether this is imported (external) or transient.
    pub imported: bool,
    /// Physical resource handle (set during compilation).
    pub physical: Option<TextureHandle>,
    /// First and last usage for lifetime tracking.
    pub first_use: Option<NodeId>,
    pub last_use: Option<NodeId>,
}

/// Virtual resource representing a buffer in the graph.
#[derive(Debug, Clone)]
pub struct VirtualBuffer {
    /// Resource identifier.
    pub id: ResourceId,
    /// Buffer description.
    pub desc: BufferDesc,
    /// Name for debugging.
    pub name: String,
    /// Whether this is imported (external) or transient.
    pub imported: bool,
    /// Physical resource handle.
    pub physical: Option<BufferHandle>,
    /// Lifetime tracking.
    pub first_use: Option<NodeId>,
    pub last_use: Option<NodeId>,
}

/// Resource access information for a pass.
#[derive(Debug, Clone)]
pub struct ResourceAccess {
    /// Resource being accessed.
    pub resource: ResourceId,
    /// Type of access.
    pub usage: ResourceUsage,
    /// Pipeline stages where access occurs.
    pub stages: PipelineStage,
    /// Access flags.
    pub access: AccessFlags,
    /// Subresource range (for textures).
    pub subresource: SubresourceRange,
}

/// Subresource range for partial resource access.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubresourceRange {
    /// Base mip level.
    pub base_mip: u32,
    /// Number of mip levels (0 = all).
    pub mip_count: u32,
    /// Base array layer.
    pub base_layer: u32,
    /// Number of layers (0 = all).
    pub layer_count: u32,
}

impl SubresourceRange {
    /// Full resource range.
    pub const ALL: Self = Self {
        base_mip: 0,
        mip_count: 0,
        base_layer: 0,
        layer_count: 0,
    };

    /// Single mip level.
    pub fn mip(level: u32) -> Self {
        Self {
            base_mip: level,
            mip_count: 1,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Single array layer.
    pub fn layer(index: u32) -> Self {
        Self {
            base_mip: 0,
            mip_count: 0,
            base_layer: index,
            layer_count: 1,
        }
    }
}

/// A node in the render graph representing a pass.
#[derive(Debug)]
pub struct GraphNode {
    /// Node identifier.
    pub id: NodeId,
    /// Pass name.
    pub name: String,
    /// Pass type.
    pub pass_type: PassType,
    /// Queue type for execution.
    pub queue: QueueType,
    /// Resource reads.
    pub reads: Vec<ResourceAccess>,
    /// Resource writes.
    pub writes: Vec<ResourceAccess>,
    /// Dependencies on other nodes.
    pub dependencies: Vec<NodeId>,
    /// Nodes that depend on this one.
    pub dependents: Vec<NodeId>,
    /// Whether this pass is culled.
    pub culled: bool,
    /// Async compute candidate.
    pub async_compute: bool,
    /// Render callback.
    pub callback: Option<Box<dyn Fn(&mut PassContext) + Send + Sync>>,
}

impl GraphNode {
    /// Create a new graph node.
    pub fn new(id: NodeId, name: impl Into<String>, pass_type: PassType) -> Self {
        Self {
            id,
            name: name.into(),
            pass_type,
            queue: QueueType::Graphics,
            reads: Vec::new(),
            writes: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            culled: false,
            async_compute: false,
            callback: None,
        }
    }

    /// Add a resource read.
    pub fn add_read(&mut self, access: ResourceAccess) {
        self.reads.push(access);
    }

    /// Add a resource write.
    pub fn add_write(&mut self, access: ResourceAccess) {
        self.writes.push(access);
    }
}

/// Edge in the render graph representing a dependency.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    /// Source node.
    pub from: NodeId,
    /// Destination node.
    pub to: NodeId,
    /// Resource causing the dependency.
    pub resource: Option<ResourceId>,
    /// Type of dependency.
    pub edge_type: EdgeType,
}

/// Types of edges in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Data dependency (read-after-write).
    Data,
    /// Anti-dependency (write-after-read).
    Anti,
    /// Output dependency (write-after-write).
    Output,
    /// Explicit ordering.
    Ordering,
}

/// Render graph builder and manager.
pub struct RenderGraph {
    /// All nodes in the graph.
    nodes: Vec<GraphNode>,
    /// All edges.
    edges: Vec<GraphEdge>,
    /// Virtual textures.
    textures: Vec<VirtualTexture>,
    /// Virtual buffers.
    buffers: Vec<VirtualBuffer>,
    /// Node ID counter.
    next_node_id: u32,
    /// Resource ID counter.
    next_resource_id: u32,
    /// Name to node mapping.
    node_names: BTreeMap<String, NodeId>,
    /// Graph version for cache invalidation.
    version: u64,
    /// Enable async compute.
    async_compute_enabled: bool,
    /// Maximum async compute overlap.
    max_async_overlap: u32,
}

impl RenderGraph {
    /// Create a new render graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            textures: Vec::new(),
            buffers: Vec::new(),
            next_node_id: 0,
            next_resource_id: 0,
            node_names: BTreeMap::new(),
            version: 0,
            async_compute_enabled: true,
            max_async_overlap: 4,
        }
    }

    /// Enable or disable async compute.
    pub fn set_async_compute(&mut self, enabled: bool) {
        self.async_compute_enabled = enabled;
    }

    /// Create a transient texture resource.
    pub fn create_texture(&mut self, desc: TextureDesc) -> VirtualTextureHandle {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        let texture = VirtualTexture {
            id,
            desc,
            name: String::new(),
            imported: false,
            physical: None,
            first_use: None,
            last_use: None,
        };

        self.textures.push(texture);
        self.version += 1;

        VirtualTextureHandle { id }
    }

    /// Create a named transient texture.
    pub fn create_texture_named(
        &mut self,
        name: impl Into<String>,
        desc: TextureDesc,
    ) -> VirtualTextureHandle {
        let handle = self.create_texture(desc);
        if let Some(tex) = self.textures.iter_mut().find(|t| t.id == handle.id) {
            tex.name = name.into();
        }
        handle
    }

    /// Import an external texture.
    pub fn import_texture(
        &mut self,
        name: impl Into<String>,
        handle: TextureHandle,
        desc: TextureDesc,
    ) -> VirtualTextureHandle {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        let texture = VirtualTexture {
            id,
            desc,
            name: name.into(),
            imported: true,
            physical: Some(handle),
            first_use: None,
            last_use: None,
        };

        self.textures.push(texture);
        self.version += 1;

        VirtualTextureHandle { id }
    }

    /// Create a transient buffer resource.
    pub fn create_buffer(&mut self, desc: BufferDesc) -> VirtualBufferHandle {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        let buffer = VirtualBuffer {
            id,
            desc,
            name: String::new(),
            imported: false,
            physical: None,
            first_use: None,
            last_use: None,
        };

        self.buffers.push(buffer);
        self.version += 1;

        VirtualBufferHandle { id }
    }

    /// Import an external buffer.
    pub fn import_buffer(
        &mut self,
        name: impl Into<String>,
        handle: BufferHandle,
        desc: BufferDesc,
    ) -> VirtualBufferHandle {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        let buffer = VirtualBuffer {
            id,
            desc,
            name: name.into(),
            imported: true,
            physical: Some(handle),
            first_use: None,
            last_use: None,
        };

        self.buffers.push(buffer);
        self.version += 1;

        VirtualBufferHandle { id }
    }

    /// Add a render pass to the graph.
    pub fn add_pass<F>(&mut self, name: impl Into<String>, builder_fn: F) -> NodeId
    where
        F: FnOnce(&mut PassBuilderImpl),
    {
        let name = name.into();
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let mut node = GraphNode::new(id, name.clone(), PassType::Graphics);
        let mut builder = PassBuilderImpl::new(&mut node, &mut self.textures, &mut self.buffers);

        builder_fn(&mut builder);

        self.node_names.insert(name, id);
        self.nodes.push(node);
        self.version += 1;

        id
    }

    /// Add a compute pass.
    pub fn add_compute_pass<F>(&mut self, name: impl Into<String>, builder_fn: F) -> NodeId
    where
        F: FnOnce(&mut PassBuilderImpl),
    {
        let name = name.into();
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let mut node = GraphNode::new(id, name.clone(), PassType::Compute);
        node.queue = QueueType::Compute;
        node.async_compute = true;

        let mut builder = PassBuilderImpl::new(&mut node, &mut self.textures, &mut self.buffers);
        builder_fn(&mut builder);

        self.node_names.insert(name, id);
        self.nodes.push(node);
        self.version += 1;

        id
    }

    /// Add a transfer pass.
    pub fn add_transfer_pass<F>(&mut self, name: impl Into<String>, builder_fn: F) -> NodeId
    where
        F: FnOnce(&mut PassBuilderImpl),
    {
        let name = name.into();
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;

        let mut node = GraphNode::new(id, name.clone(), PassType::Transfer);
        node.queue = QueueType::Transfer;

        let mut builder = PassBuilderImpl::new(&mut node, &mut self.textures, &mut self.buffers);
        builder_fn(&mut builder);

        self.node_names.insert(name, id);
        self.nodes.push(node);
        self.version += 1;

        id
    }

    /// Add an explicit dependency between passes.
    pub fn add_dependency(&mut self, from: NodeId, to: NodeId) {
        self.edges.push(GraphEdge {
            from,
            to,
            resource: None,
            edge_type: EdgeType::Ordering,
        });

        if let Some(from_node) = self.nodes.iter_mut().find(|n| n.id == from) {
            if !from_node.dependents.contains(&to) {
                from_node.dependents.push(to);
            }
        }

        if let Some(to_node) = self.nodes.iter_mut().find(|n| n.id == to) {
            if !to_node.dependencies.contains(&from) {
                to_node.dependencies.push(from);
            }
        }

        self.version += 1;
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get a node by name.
    pub fn get_node_by_name(&self, name: &str) -> Option<&GraphNode> {
        self.node_names.get(name).and_then(|id| self.get_node(*id))
    }

    /// Get the current graph version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Compile the graph for execution.
    pub fn compile(&mut self) -> Result<CompiledGraph, GraphError> {
        // Phase 1: Build dependency graph from resource accesses
        self.build_dependencies()?;

        // Phase 2: Topological sort
        let execution_order = self.topological_sort()?;

        // Phase 3: Cull unused passes
        let active_nodes = self.cull_passes(&execution_order)?;

        // Phase 4: Calculate resource lifetimes
        self.calculate_lifetimes(&active_nodes)?;

        // Phase 5: Allocate physical resources with aliasing
        let allocations = self.allocate_resources(&active_nodes)?;

        // Phase 6: Generate barriers
        let barriers = self.generate_barriers(&active_nodes)?;

        // Phase 7: Schedule async compute
        let schedule = if self.async_compute_enabled {
            self.schedule_async_compute(&active_nodes)?
        } else {
            ExecutionSchedule::sequential(active_nodes.clone())
        };

        Ok(CompiledGraph {
            version: self.version,
            execution_order: active_nodes,
            schedule,
            allocations,
            barriers,
        })
    }

    /// Build dependency graph from resource accesses.
    fn build_dependencies(&mut self) -> Result<(), GraphError> {
        // Track last writer for each resource
        let mut last_writer: BTreeMap<ResourceId, NodeId> = BTreeMap::new();
        // Track readers since last write
        let mut readers: BTreeMap<ResourceId, Vec<NodeId>> = BTreeMap::new();

        // Clear existing auto-generated edges
        self.edges.retain(|e| e.edge_type == EdgeType::Ordering);

        // Sort nodes by their current order (or ID)
        let node_ids: Vec<NodeId> = self.nodes.iter().map(|n| n.id).collect();

        for &node_id in &node_ids {
            let node = self.nodes.iter().find(|n| n.id == node_id).unwrap();

            // Check reads - depend on last writer
            for read in &node.reads {
                if let Some(&writer) = last_writer.get(&read.resource) {
                    if writer != node_id {
                        self.edges.push(GraphEdge {
                            from: writer,
                            to: node_id,
                            resource: Some(read.resource),
                            edge_type: EdgeType::Data,
                        });
                    }
                }
            }

            // Check writes
            for write in &node.writes {
                // Write-after-read dependencies
                if let Some(resource_readers) = readers.get(&write.resource) {
                    for &reader in resource_readers {
                        if reader != node_id {
                            self.edges.push(GraphEdge {
                                from: reader,
                                to: node_id,
                                resource: Some(write.resource),
                                edge_type: EdgeType::Anti,
                            });
                        }
                    }
                }

                // Write-after-write dependency
                if let Some(&prev_writer) = last_writer.get(&write.resource) {
                    if prev_writer != node_id {
                        self.edges.push(GraphEdge {
                            from: prev_writer,
                            to: node_id,
                            resource: Some(write.resource),
                            edge_type: EdgeType::Output,
                        });
                    }
                }

                // Update tracking
                last_writer.insert(write.resource, node_id);
                readers.remove(&write.resource);
            }

            // Record this node as a reader
            for read in &node.reads {
                readers.entry(read.resource).or_default().push(node_id);
            }
        }

        // Update node dependency lists
        for edge in &self.edges {
            if let Some(from_node) = self.nodes.iter_mut().find(|n| n.id == edge.from) {
                if !from_node.dependents.contains(&edge.to) {
                    from_node.dependents.push(edge.to);
                }
            }
            if let Some(to_node) = self.nodes.iter_mut().find(|n| n.id == edge.to) {
                if !to_node.dependencies.contains(&edge.from) {
                    to_node.dependencies.push(edge.from);
                }
            }
        }

        Ok(())
    }

    /// Topological sort using Kahn's algorithm.
    fn topological_sort(&self) -> Result<Vec<NodeId>, GraphError> {
        let mut in_degree: BTreeMap<NodeId, usize> = BTreeMap::new();
        let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();

        // Initialize
        for node in &self.nodes {
            in_degree.insert(node.id, 0);
            adjacency.insert(node.id, Vec::new());
        }

        // Build adjacency and in-degree
        for edge in &self.edges {
            adjacency.get_mut(&edge.from).map(|v| v.push(edge.to));
            in_degree.entry(edge.to).and_modify(|d| *d += 1);
        }

        // Find nodes with no dependencies
        let mut queue: Vec<NodeId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::new();

        while let Some(node_id) = queue.pop() {
            result.push(node_id);

            if let Some(neighbors) = adjacency.get(&node_id) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(&neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(GraphError::CycleDetected);
        }

        Ok(result)
    }

    /// Cull passes that don't contribute to outputs.
    fn cull_passes(&mut self, order: &[NodeId]) -> Result<Vec<NodeId>, GraphError> {
        // Find output resources (imported with no further writes)
        let mut required_resources: Vec<ResourceId> = Vec::new();

        // Mark imported textures that are written as required
        for tex in &self.textures {
            if tex.imported {
                required_resources.push(tex.id);
            }
        }

        // Work backwards to find required passes
        let mut required_passes: Vec<NodeId> = Vec::new();

        for &node_id in order.iter().rev() {
            let node = self.nodes.iter().find(|n| n.id == node_id).unwrap();

            // Check if this pass writes to a required resource
            let writes_required = node
                .writes
                .iter()
                .any(|w| required_resources.contains(&w.resource));

            if writes_required {
                required_passes.push(node_id);

                // Mark read resources as required
                for read in &node.reads {
                    if !required_resources.contains(&read.resource) {
                        required_resources.push(read.resource);
                    }
                }
            }
        }

        // Mark culled passes
        for node in &mut self.nodes {
            node.culled = !required_passes.contains(&node.id);
        }

        // Return in execution order
        Ok(order
            .iter()
            .filter(|id| required_passes.contains(id))
            .copied()
            .collect())
    }

    /// Calculate resource lifetimes.
    fn calculate_lifetimes(&mut self, order: &[NodeId]) -> Result<(), GraphError> {
        // Reset lifetimes
        for tex in &mut self.textures {
            tex.first_use = None;
            tex.last_use = None;
        }
        for buf in &mut self.buffers {
            buf.first_use = None;
            buf.last_use = None;
        }

        // Track first and last use
        for &node_id in order {
            let node = self.nodes.iter().find(|n| n.id == node_id).unwrap();

            for access in node.reads.iter().chain(node.writes.iter()) {
                // Update textures
                if let Some(tex) = self.textures.iter_mut().find(|t| t.id == access.resource) {
                    if tex.first_use.is_none() {
                        tex.first_use = Some(node_id);
                    }
                    tex.last_use = Some(node_id);
                }

                // Update buffers
                if let Some(buf) = self.buffers.iter_mut().find(|b| b.id == access.resource) {
                    if buf.first_use.is_none() {
                        buf.first_use = Some(node_id);
                    }
                    buf.last_use = Some(node_id);
                }
            }
        }

        Ok(())
    }

    /// Allocate physical resources with aliasing.
    fn allocate_resources(&mut self, order: &[NodeId]) -> Result<ResourceAllocations, GraphError> {
        let mut allocations = ResourceAllocations::new();

        // Build timeline for aliasing
        let mut timeline: Vec<(NodeId, Vec<ResourceId>, Vec<ResourceId>)> = Vec::new();

        for &node_id in order {
            let mut starts = Vec::new();
            let mut ends = Vec::new();

            for tex in &self.textures {
                if tex.first_use == Some(node_id) {
                    starts.push(tex.id);
                }
                if tex.last_use == Some(node_id) {
                    ends.push(tex.id);
                }
            }

            for buf in &self.buffers {
                if buf.first_use == Some(node_id) {
                    starts.push(buf.id);
                }
                if buf.last_use == Some(node_id) {
                    ends.push(buf.id);
                }
            }

            timeline.push((node_id, starts, ends));
        }

        // Simple first-fit allocation with aliasing
        let mut memory_pools: Vec<MemoryPool> = Vec::new();

        for (node_id, starts, ends) in &timeline {
            // Free ended resources
            for &res_id in ends {
                for pool in &mut memory_pools {
                    pool.release(res_id);
                }
            }

            // Allocate new resources
            for &res_id in starts {
                let size = self.get_resource_size(res_id);
                let alignment = self.get_resource_alignment(res_id);

                // Try to find existing pool with space
                let mut allocated = false;
                for pool in &mut memory_pools {
                    if let Some(offset) = pool.try_allocate(res_id, size, alignment) {
                        allocations.set_offset(res_id, pool.id, offset);
                        allocated = true;
                        break;
                    }
                }

                if !allocated {
                    // Create new pool
                    let pool_id = memory_pools.len() as u32;
                    let mut pool = MemoryPool::new(pool_id, size.max(64 * 1024 * 1024));
                    if let Some(offset) = pool.try_allocate(res_id, size, alignment) {
                        allocations.set_offset(res_id, pool_id, offset);
                    }
                    memory_pools.push(pool);
                }
            }
        }

        allocations.pools = memory_pools;
        Ok(allocations)
    }

    /// Get resource size in bytes.
    fn get_resource_size(&self, id: ResourceId) -> u64 {
        if let Some(tex) = self.textures.iter().find(|t| t.id == id) {
            tex.desc.calculate_size()
        } else if let Some(buf) = self.buffers.iter().find(|b| b.id == id) {
            buf.desc.size
        } else {
            0
        }
    }

    /// Get resource alignment.
    fn get_resource_alignment(&self, id: ResourceId) -> u64 {
        if let Some(tex) = self.textures.iter().find(|t| t.id == id) {
            tex.desc.alignment()
        } else if let Some(buf) = self.buffers.iter().find(|b| b.id == id) {
            buf.desc.alignment
        } else {
            256
        }
    }

    /// Generate optimal barriers.
    fn generate_barriers(&self, order: &[NodeId]) -> Result<BarrierSchedule, GraphError> {
        let mut schedule = BarrierSchedule::new();
        let mut resource_states: BTreeMap<ResourceId, ResourceState> = BTreeMap::new();

        // Initialize states
        for tex in &self.textures {
            resource_states.insert(tex.id, ResourceState::Undefined);
        }
        for buf in &self.buffers {
            resource_states.insert(buf.id, ResourceState::Undefined);
        }

        for &node_id in order {
            let node = self.nodes.iter().find(|n| n.id == node_id).unwrap();
            let mut barriers = Vec::new();

            // Check all accesses for this pass
            for access in node.reads.iter().chain(node.writes.iter()) {
                let current_state = resource_states
                    .get(&access.resource)
                    .copied()
                    .unwrap_or(ResourceState::Undefined);

                let required_state = access.usage.to_resource_state();

                if current_state != required_state || Self::needs_barrier(current_state, &access) {
                    barriers.push(Barrier {
                        resource: access.resource,
                        old_state: current_state,
                        new_state: required_state,
                        src_stages: current_state.to_pipeline_stage(),
                        dst_stages: access.stages,
                        src_access: current_state.to_access_flags(),
                        dst_access: access.access,
                        subresource: access.subresource,
                    });

                    resource_states.insert(access.resource, required_state);
                }
            }

            if !barriers.is_empty() {
                schedule.add_barriers(node_id, barriers);
            }
        }

        Ok(schedule)
    }

    /// Check if a barrier is needed even with same state.
    fn needs_barrier(state: ResourceState, access: &ResourceAccess) -> bool {
        match state {
            ResourceState::General => true, // Always barrier from general
            ResourceState::Storage if access.access.contains(AccessFlags::WRITE) => true,
            _ => false,
        }
    }

    /// Schedule async compute passes.
    fn schedule_async_compute(&self, order: &[NodeId]) -> Result<ExecutionSchedule, GraphError> {
        let mut schedule = ExecutionSchedule::new();
        let mut current_graphics_batch = Vec::new();
        let mut current_compute_batch = Vec::new();

        for &node_id in order {
            let node = self.nodes.iter().find(|n| n.id == node_id).unwrap();

            if node.async_compute && node.queue == QueueType::Compute {
                // Check if we can run in parallel with current graphics
                let can_parallel = self.can_run_async(node, &current_graphics_batch);

                if can_parallel && current_compute_batch.len() < self.max_async_overlap as usize {
                    current_compute_batch.push(node_id);
                } else {
                    // Flush compute batch
                    if !current_compute_batch.is_empty() {
                        schedule.add_batch(ExecutionBatch {
                            nodes: current_compute_batch.clone(),
                            queue: QueueType::Compute,
                            wait_for: schedule.last_graphics_batch(),
                        });
                        current_compute_batch.clear();
                    }
                    current_compute_batch.push(node_id);
                }
            } else {
                // Graphics pass - flush compute if needed
                if !current_compute_batch.is_empty() {
                    schedule.add_batch(ExecutionBatch {
                        nodes: current_compute_batch.clone(),
                        queue: QueueType::Compute,
                        wait_for: schedule.last_graphics_batch(),
                    });
                    current_compute_batch.clear();
                }

                current_graphics_batch.push(node_id);

                // Batch graphics passes that can run together
                if self.should_flush_graphics(&current_graphics_batch, order, node_id) {
                    schedule.add_batch(ExecutionBatch {
                        nodes: current_graphics_batch.clone(),
                        queue: QueueType::Graphics,
                        wait_for: schedule.last_compute_batch(),
                    });
                    current_graphics_batch.clear();
                }
            }
        }

        // Flush remaining
        if !current_graphics_batch.is_empty() {
            schedule.add_batch(ExecutionBatch {
                nodes: current_graphics_batch,
                queue: QueueType::Graphics,
                wait_for: schedule.last_compute_batch(),
            });
        }
        if !current_compute_batch.is_empty() {
            schedule.add_batch(ExecutionBatch {
                nodes: current_compute_batch,
                queue: QueueType::Compute,
                wait_for: schedule.last_graphics_batch(),
            });
        }

        Ok(schedule)
    }

    /// Check if a compute pass can run async with graphics.
    fn can_run_async(&self, compute_node: &GraphNode, graphics_nodes: &[NodeId]) -> bool {
        // Check for resource conflicts
        for &graphics_id in graphics_nodes {
            let graphics_node = self.nodes.iter().find(|n| n.id == graphics_id).unwrap();

            // Check for overlapping writes
            for cw in &compute_node.writes {
                for gw in &graphics_node.writes {
                    if cw.resource == gw.resource {
                        return false;
                    }
                }
                for gr in &graphics_node.reads {
                    if cw.resource == gr.resource {
                        return false;
                    }
                }
            }

            // Check for read-write conflicts
            for cr in &compute_node.reads {
                for gw in &graphics_node.writes {
                    if cr.resource == gw.resource {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Check if we should flush the graphics batch.
    fn should_flush_graphics(&self, batch: &[NodeId], order: &[NodeId], current: NodeId) -> bool {
        // Flush if next pass is compute and we have pending graphics
        let current_idx = order.iter().position(|&id| id == current).unwrap_or(0);
        if current_idx + 1 < order.len() {
            let next_id = order[current_idx + 1];
            if let Some(next) = self.nodes.iter().find(|n| n.id == next_id) {
                if next.queue == QueueType::Compute && next.async_compute {
                    return true;
                }
            }
        }

        // Flush if batch is getting large
        batch.len() >= 8
    }

    /// Clear the graph for reuse.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.textures.clear();
        self.buffers.clear();
        self.node_names.clear();
        self.next_node_id = 0;
        self.next_resource_id = 0;
        self.version += 1;
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a virtual texture in the graph.
#[derive(Debug, Clone, Copy)]
pub struct VirtualTextureHandle {
    pub(crate) id: ResourceId,
}

/// Handle to a virtual buffer in the graph.
#[derive(Debug, Clone, Copy)]
pub struct VirtualBufferHandle {
    pub(crate) id: ResourceId,
}

/// Pass builder implementation.
pub struct PassBuilderImpl<'a> {
    node: &'a mut GraphNode,
    textures: &'a mut Vec<VirtualTexture>,
    buffers: &'a mut Vec<VirtualBuffer>,
}

impl<'a> PassBuilderImpl<'a> {
    fn new(
        node: &'a mut GraphNode,
        textures: &'a mut Vec<VirtualTexture>,
        buffers: &'a mut Vec<VirtualBuffer>,
    ) -> Self {
        Self {
            node,
            textures,
            buffers,
        }
    }

    /// Read a texture.
    pub fn read_texture(&mut self, handle: VirtualTextureHandle) -> &mut Self {
        self.node.reads.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::ShaderRead,
            stages: PipelineStage::FRAGMENT_SHADER,
            access: AccessFlags::SHADER_READ,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Read a texture at specific mip level.
    pub fn read_texture_mip(&mut self, handle: VirtualTextureHandle, mip: u32) -> &mut Self {
        self.node.reads.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::ShaderRead,
            stages: PipelineStage::FRAGMENT_SHADER,
            access: AccessFlags::SHADER_READ,
            subresource: SubresourceRange::mip(mip),
        });
        self
    }

    /// Write to a color attachment.
    pub fn write_color(&mut self, handle: VirtualTextureHandle) -> &mut Self {
        self.node.writes.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::ColorAttachment,
            stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            access: AccessFlags::COLOR_ATTACHMENT_WRITE,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Write to depth attachment.
    pub fn write_depth(&mut self, handle: VirtualTextureHandle) -> &mut Self {
        self.node.writes.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::DepthStencilAttachment,
            stages: PipelineStage::EARLY_FRAGMENT_TESTS | PipelineStage::LATE_FRAGMENT_TESTS,
            access: AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Read depth for testing only.
    pub fn read_depth(&mut self, handle: VirtualTextureHandle) -> &mut Self {
        self.node.reads.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::DepthStencilRead,
            stages: PipelineStage::EARLY_FRAGMENT_TESTS,
            access: AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Read/write storage image.
    pub fn storage_image(&mut self, handle: VirtualTextureHandle) -> &mut Self {
        let access = ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::StorageImage,
            stages: PipelineStage::COMPUTE_SHADER,
            access: AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
            subresource: SubresourceRange::ALL,
        };
        self.node.reads.push(access.clone());
        self.node.writes.push(access);
        self
    }

    /// Read a buffer.
    pub fn read_buffer(&mut self, handle: VirtualBufferHandle) -> &mut Self {
        self.node.reads.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::UniformBuffer,
            stages: PipelineStage::VERTEX_SHADER | PipelineStage::FRAGMENT_SHADER,
            access: AccessFlags::UNIFORM_READ,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Write to a storage buffer.
    pub fn write_buffer(&mut self, handle: VirtualBufferHandle) -> &mut Self {
        self.node.writes.push(ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::StorageBuffer,
            stages: PipelineStage::COMPUTE_SHADER,
            access: AccessFlags::SHADER_WRITE,
            subresource: SubresourceRange::ALL,
        });
        self
    }

    /// Read/write storage buffer.
    pub fn storage_buffer(&mut self, handle: VirtualBufferHandle) -> &mut Self {
        let access = ResourceAccess {
            resource: handle.id,
            usage: ResourceUsage::StorageBuffer,
            stages: PipelineStage::COMPUTE_SHADER,
            access: AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE,
            subresource: SubresourceRange::ALL,
        };
        self.node.reads.push(access.clone());
        self.node.writes.push(access);
        self
    }

    /// Set the render callback.
    pub fn render<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(&mut PassContext) + Send + Sync + 'static,
    {
        self.node.callback = Some(Box::new(callback));
        self
    }

    /// Enable async compute for this pass.
    pub fn async_compute(&mut self, enabled: bool) -> &mut Self {
        self.node.async_compute = enabled;
        self.node.queue = if enabled {
            QueueType::Compute
        } else {
            QueueType::Graphics
        };
        self
    }
}

/// Compiled render graph ready for execution.
pub struct CompiledGraph {
    /// Graph version at compilation.
    pub version: u64,
    /// Execution order.
    pub execution_order: Vec<NodeId>,
    /// Execution schedule with async compute.
    pub schedule: ExecutionSchedule,
    /// Resource allocations.
    pub allocations: ResourceAllocations,
    /// Barrier schedule.
    pub barriers: BarrierSchedule,
}

impl CompiledGraph {
    /// Execute the compiled graph.
    pub fn execute(&self, _context: &mut PassContext) -> Result<(), GraphError> {
        // Implementation would submit commands to GPU
        Ok(())
    }

    /// Check if recompilation is needed.
    pub fn needs_recompile(&self, graph: &RenderGraph) -> bool {
        self.version != graph.version()
    }
}

/// Memory pool for resource aliasing.
#[derive(Debug)]
pub struct MemoryPool {
    /// Pool ID.
    pub id: u32,
    /// Total size.
    pub size: u64,
    /// Current allocations (resource, offset, size).
    allocations: Vec<(ResourceId, u64, u64)>,
    /// Free regions (offset, size).
    free_regions: Vec<(u64, u64)>,
}

impl MemoryPool {
    /// Create a new memory pool.
    pub fn new(id: u32, size: u64) -> Self {
        Self {
            id,
            size,
            allocations: Vec::new(),
            free_regions: vec![(0, size)],
        }
    }

    /// Try to allocate from this pool.
    pub fn try_allocate(&mut self, resource: ResourceId, size: u64, alignment: u64) -> Option<u64> {
        // Find first fit
        for i in 0..self.free_regions.len() {
            let (offset, region_size) = self.free_regions[i];
            let aligned_offset = (offset + alignment - 1) & !(alignment - 1);
            let padding = aligned_offset - offset;

            if region_size >= size + padding {
                // Allocate
                self.allocations.push((resource, aligned_offset, size));

                // Update free region
                if padding > 0 {
                    // Keep the padding as free
                    self.free_regions[i] = (offset, padding);
                    let remaining = region_size - size - padding;
                    if remaining > 0 {
                        self.free_regions
                            .insert(i + 1, (aligned_offset + size, remaining));
                    }
                } else {
                    let remaining = region_size - size;
                    if remaining > 0 {
                        self.free_regions[i] = (offset + size, remaining);
                    } else {
                        self.free_regions.remove(i);
                    }
                }

                return Some(aligned_offset);
            }
        }

        None
    }

    /// Release a resource.
    pub fn release(&mut self, resource: ResourceId) {
        if let Some(pos) = self.allocations.iter().position(|(r, _, _)| *r == resource) {
            let (_, offset, size) = self.allocations.remove(pos);

            // Add back to free list
            self.free_regions.push((offset, size));

            // Merge adjacent regions
            self.merge_free_regions();
        }
    }

    /// Merge adjacent free regions.
    fn merge_free_regions(&mut self) {
        self.free_regions.sort_by_key(|(offset, _)| *offset);

        let mut i = 0;
        while i + 1 < self.free_regions.len() {
            let (offset1, size1) = self.free_regions[i];
            let (offset2, size2) = self.free_regions[i + 1];

            if offset1 + size1 == offset2 {
                self.free_regions[i] = (offset1, size1 + size2);
                self.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}

/// Resource allocation results.
#[derive(Debug, Default)]
pub struct ResourceAllocations {
    /// Memory pools.
    pub pools: Vec<MemoryPool>,
    /// Resource to (pool, offset) mapping.
    offsets: BTreeMap<ResourceId, (u32, u64)>,
}

impl ResourceAllocations {
    /// Create new allocations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set resource offset.
    pub fn set_offset(&mut self, resource: ResourceId, pool: u32, offset: u64) {
        self.offsets.insert(resource, (pool, offset));
    }

    /// Get resource allocation.
    pub fn get(&self, resource: ResourceId) -> Option<(u32, u64)> {
        self.offsets.get(&resource).copied()
    }

    /// Get total memory usage.
    pub fn total_memory(&self) -> u64 {
        self.pools.iter().map(|p| p.size).sum()
    }

    /// Get aliased memory savings.
    pub fn aliased_savings(&self) -> u64 {
        let actual_used: u64 = self
            .offsets
            .values()
            .filter_map(|&(pool_id, offset)| {
                self.pools.get(pool_id as usize).and_then(|pool| {
                    pool.allocations
                        .iter()
                        .find(|(_, o, _)| *o == offset)
                        .map(|(_, _, size)| *size)
                })
            })
            .sum();

        let total = self.total_memory();
        if actual_used > total {
            actual_used - total
        } else {
            0
        }
    }
}

/// Barrier schedule.
#[derive(Debug, Default)]
pub struct BarrierSchedule {
    /// Barriers per pass.
    barriers: BTreeMap<NodeId, Vec<Barrier>>,
}

impl BarrierSchedule {
    /// Create new barrier schedule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add barriers for a pass.
    pub fn add_barriers(&mut self, pass: NodeId, barriers: Vec<Barrier>) {
        self.barriers.insert(pass, barriers);
    }

    /// Get barriers for a pass.
    pub fn get(&self, pass: NodeId) -> Option<&Vec<Barrier>> {
        self.barriers.get(&pass)
    }

    /// Get total barrier count.
    pub fn total_barriers(&self) -> usize {
        self.barriers.values().map(|v| v.len()).sum()
    }
}

/// Execution schedule with async compute.
#[derive(Debug, Default)]
pub struct ExecutionSchedule {
    /// Execution batches.
    batches: Vec<ExecutionBatch>,
}

impl ExecutionSchedule {
    /// Create new schedule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create sequential schedule.
    pub fn sequential(nodes: Vec<NodeId>) -> Self {
        let mut schedule = Self::new();
        for node in nodes {
            schedule.add_batch(ExecutionBatch {
                nodes: vec![node],
                queue: QueueType::Graphics,
                wait_for: None,
            });
        }
        schedule
    }

    /// Add a batch.
    pub fn add_batch(&mut self, batch: ExecutionBatch) {
        self.batches.push(batch);
    }

    /// Get last graphics batch index.
    pub fn last_graphics_batch(&self) -> Option<usize> {
        self.batches
            .iter()
            .rposition(|b| b.queue == QueueType::Graphics)
    }

    /// Get last compute batch index.
    pub fn last_compute_batch(&self) -> Option<usize> {
        self.batches
            .iter()
            .rposition(|b| b.queue == QueueType::Compute)
    }

    /// Get batches.
    pub fn batches(&self) -> &[ExecutionBatch] {
        &self.batches
    }
}

/// Execution batch.
#[derive(Debug)]
pub struct ExecutionBatch {
    /// Nodes in this batch.
    pub nodes: Vec<NodeId>,
    /// Queue for execution.
    pub queue: QueueType,
    /// Wait for batch index.
    pub wait_for: Option<usize>,
}

/// Render graph builder for fluent API.
pub struct RenderGraphBuilder {
    graph: RenderGraph,
}

impl RenderGraphBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            graph: RenderGraph::new(),
        }
    }

    /// Enable async compute.
    pub fn with_async_compute(mut self, enabled: bool) -> Self {
        self.graph.set_async_compute(enabled);
        self
    }

    /// Build the graph.
    pub fn build(self) -> RenderGraph {
        self.graph
    }
}

impl Default for RenderGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph compilation/execution errors.
#[derive(Debug)]
pub enum GraphError {
    /// Cycle detected in graph.
    CycleDetected,
    /// Resource not found.
    ResourceNotFound(ResourceId),
    /// Pass not found.
    PassNotFound(String),
    /// Invalid configuration.
    InvalidConfiguration(String),
    /// Execution error.
    ExecutionError(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::CycleDetected => write!(f, "cycle detected in render graph"),
            GraphError::ResourceNotFound(id) => write!(f, "resource not found: {:?}", id),
            GraphError::PassNotFound(name) => write!(f, "pass not found: {}", name),
            GraphError::InvalidConfiguration(msg) => write!(f, "invalid configuration: {}", msg),
            GraphError::ExecutionError(msg) => write!(f, "execution error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = RenderGraph::new();
        assert_eq!(graph.version(), 0);
    }

    #[test]
    fn test_add_pass() {
        let mut graph = RenderGraph::new();

        let tex = graph.create_texture(TextureDesc::color_2d(1920, 1080));
        let _pass = graph.add_pass("test", |builder| {
            builder.write_color(tex);
        });

        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_memory_pool_allocation() {
        let mut pool = MemoryPool::new(0, 1024);

        let r1 = ResourceId::new(0);
        let r2 = ResourceId::new(1);

        assert!(pool.try_allocate(r1, 256, 64).is_some());
        assert!(pool.try_allocate(r2, 256, 64).is_some());

        pool.release(r1);
        let r3 = ResourceId::new(2);
        assert!(pool.try_allocate(r3, 256, 64).is_some());
    }
}
