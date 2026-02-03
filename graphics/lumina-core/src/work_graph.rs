//! Work Graphs for LUMINA
//!
//! Work graphs provide a GPU-driven compute execution model where
//! the GPU schedules work dynamically without CPU intervention.
//!
//! ```text
//! Traditional Compute:
//!   CPU: Dispatch(X,Y,Z) → GPU: Execute → CPU: Dispatch(X,Y,Z) → ...
//!
//! Work Graphs:
//!   CPU: Initialize → GPU: Nodes spawn nodes → GPU: Automatic scheduling
//! ```
//!
//! ## Use Cases
//!
//! - GPU-driven culling and rendering
//! - Adaptive tessellation
//! - Ray tracing denoising
//! - Dynamic LOD selection
//! - Physics simulation

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::ops::Range;

use crate::error::{Error, Result};

// ============================================================================
// Work Graph Limits
// ============================================================================

/// Hardware limits for work graphs
#[derive(Clone, Copy, Debug)]
pub struct WorkGraphLimits {
    /// Maximum number of nodes in a work graph
    pub max_nodes: u32,
    /// Maximum node recursion depth
    pub max_recursion_depth: u32,
    /// Maximum work graph memory size in bytes
    pub max_memory_size: u64,
    /// Maximum input records per node
    pub max_input_records: u32,
    /// Maximum output records per node
    pub max_output_records: u32,
    /// Maximum payload size in bytes
    pub max_payload_size: u32,
    /// Maximum node outputs
    pub max_node_outputs: u32,
    /// Maximum entry points
    pub max_entry_points: u32,
    /// Shared memory size per work graph
    pub max_shared_memory_size: u32,
}

impl Default for WorkGraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 256,
            max_recursion_depth: 32,
            max_memory_size: 256 * 1024 * 1024, // 256 MB
            max_input_records: 1024 * 1024,
            max_output_records: 1024 * 1024,
            max_payload_size: 4096,
            max_node_outputs: 16,
            max_entry_points: 16,
            max_shared_memory_size: 64 * 1024,
        }
    }
}

// ============================================================================
// Node Types
// ============================================================================

/// Type of node in a work graph
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// Broadcasting launch node (spawns work from CPU)
    BroadcastingLaunch,
    /// Coalescing launch node (aggregates work)
    CoalescingLaunch,
    /// Thread launch node (one thread per record)
    ThreadLaunch,
    /// Empty node (no shader, just routing)
    Empty,
}

impl NodeType {
    /// Human-readable name
    pub const fn name(self) -> &'static str {
        match self {
            Self::BroadcastingLaunch => "BroadcastingLaunch",
            Self::CoalescingLaunch => "CoalescingLaunch",
            Self::ThreadLaunch => "ThreadLaunch",
            Self::Empty => "Empty",
        }
    }
}

/// Dispatch grid dimensions for a node
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeDispatchGrid {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl NodeDispatchGrid {
    /// Create new dispatch grid
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// 1D dispatch
    pub const fn linear(count: u32) -> Self {
        Self::new(count, 1, 1)
    }

    /// Total thread groups
    pub const fn total_groups(&self) -> u32 {
        self.x * self.y * self.z
    }
}

// ============================================================================
// Node Definition
// ============================================================================

/// Definition of a node in the work graph
#[derive(Clone, Debug)]
pub struct WorkGraphNode {
    /// Unique node ID within the graph
    pub id: u32,
    /// Node name (for debugging)
    pub name: NodeName,
    /// Type of node
    pub node_type: NodeType,
    /// Shader code for this node
    pub shader: NodeShader,
    /// Input record definition
    pub input_record: Option<RecordDefinition>,
    /// Output record definitions
    pub output_records: [Option<RecordDefinition>; 8],
    /// Number of output records
    pub output_count: u32,
    /// Local work group size
    pub local_size: [u32; 3],
    /// Maximum dispatch grid (if known statically)
    pub max_dispatch_grid: Option<NodeDispatchGrid>,
    /// Whether this is an entry point
    pub is_entry_point: bool,
    /// Node flags
    pub flags: NodeFlags,
}

/// Node name (fixed size for no_std)
#[derive(Clone, Debug)]
pub struct NodeName {
    data: [u8; 64],
    len: usize,
}

impl NodeName {
    /// Create from string
    pub fn new(name: &str) -> Self {
        let mut node_name = Self {
            data: [0; 64],
            len: name.len().min(63),
        };
        node_name.data[..node_name.len].copy_from_slice(name.as_bytes());
        node_name
    }

    /// Get as string slice
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

impl Default for NodeName {
    fn default() -> Self {
        Self::new("unnamed")
    }
}

/// Shader for a work graph node
#[derive(Clone, Debug)]
pub struct NodeShader {
    /// SPIR-V or native bytecode
    pub code: ShaderBytecode,
    /// Entry point name
    pub entry_point: EntryPoint,
}

/// Shader bytecode
#[derive(Clone, Debug)]
pub struct ShaderBytecode {
    data: [u8; Self::MAX_SIZE],
    size: usize,
}

impl ShaderBytecode {
    /// Maximum shader size
    pub const MAX_SIZE: usize = 64 * 1024;

    /// Create from slice
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        if data.len() > Self::MAX_SIZE {
            return Err(Error::InvalidParameter);
        }
        let mut bytecode = Self {
            data: [0; Self::MAX_SIZE],
            size: data.len(),
        };
        bytecode.data[..data.len()].copy_from_slice(data);
        Ok(bytecode)
    }

    /// Get as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }
}

/// Entry point name
#[derive(Clone, Debug)]
pub struct EntryPoint {
    name: [u8; 64],
    len: usize,
}

impl EntryPoint {
    pub fn new(name: &str) -> Self {
        let mut ep = Self {
            name: [0; 64],
            len: name.len().min(63),
        };
        ep.name[..ep.len].copy_from_slice(name.as_bytes());
        ep
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.len]).unwrap_or("main")
    }
}

impl Default for EntryPoint {
    fn default() -> Self {
        Self::new("main")
    }
}

/// Record definition (input/output structure)
#[derive(Clone, Debug)]
pub struct RecordDefinition {
    /// Name of the record type
    pub name: RecordName,
    /// Size in bytes
    pub size: u32,
    /// Alignment requirement
    pub alignment: u32,
    /// Field definitions
    pub fields: [RecordField; 16],
    /// Number of fields
    pub field_count: u32,
}

/// Record name
#[derive(Clone, Debug)]
pub struct RecordName {
    data: [u8; 64],
    len: usize,
}

impl RecordName {
    pub fn new(name: &str) -> Self {
        let mut rn = Self {
            data: [0; 64],
            len: name.len().min(63),
        };
        rn.data[..rn.len].copy_from_slice(name.as_bytes());
        rn
    }
}

impl Default for RecordName {
    fn default() -> Self {
        Self::new("Record")
    }
}

/// A field within a record
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordField {
    /// Offset in bytes from record start
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Data type
    pub data_type: RecordFieldType,
}

/// Data type for record fields
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordFieldType {
    #[default]
    U32,
    I32,
    F32,
    U64,
    I64,
    F64,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
    Struct,
}

/// Node flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeFlags(u32);

impl NodeFlags {
    pub const NONE: Self = Self(0);
    /// Node can be executed multiple times per graph execution
    pub const REENTRANT: Self = Self(1 << 0);
    /// Node requires ordered execution
    pub const ORDERED: Self = Self(1 << 1);
    /// Track completed count
    pub const TRACK_COMPLETED: Self = Self(1 << 2);
    /// Allow coalescing of multiple spawns
    pub const ALLOW_COALESCING: Self = Self(1 << 3);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Work Graph Definition
// ============================================================================

/// Complete work graph definition
#[derive(Clone, Debug)]
pub struct WorkGraphDefinition {
    /// Graph name
    pub name: GraphName,
    /// Nodes in the graph
    pub nodes: [Option<WorkGraphNode>; 64],
    /// Number of nodes
    pub node_count: u32,
    /// Entry point node IDs
    pub entry_points: [u32; 16],
    /// Number of entry points
    pub entry_point_count: u32,
    /// Edges (connections between nodes)
    pub edges: [NodeEdge; 256],
    /// Number of edges
    pub edge_count: u32,
    /// Required memory allocation
    pub backing_memory_size: u64,
    /// Graph flags
    pub flags: WorkGraphFlags,
}

/// Graph name
#[derive(Clone, Debug)]
pub struct GraphName {
    data: [u8; 64],
    len: usize,
}

impl GraphName {
    pub fn new(name: &str) -> Self {
        let mut gn = Self {
            data: [0; 64],
            len: name.len().min(63),
        };
        gn.data[..gn.len].copy_from_slice(name.as_bytes());
        gn
    }
}

impl Default for GraphName {
    fn default() -> Self {
        Self::new("WorkGraph")
    }
}

/// Edge connecting two nodes
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeEdge {
    /// Source node ID
    pub source_node: u32,
    /// Source output index
    pub source_output: u32,
    /// Destination node ID
    pub dest_node: u32,
    /// Destination input index (usually 0)
    pub dest_input: u32,
}

impl NodeEdge {
    pub const fn new(source: u32, output: u32, dest: u32, input: u32) -> Self {
        Self {
            source_node: source,
            source_output: output,
            dest_node: dest,
            dest_input: input,
        }
    }
}

/// Work graph flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkGraphFlags(u32);

impl WorkGraphFlags {
    pub const NONE: Self = Self(0);
    /// Allow graph to be modified after creation
    pub const MUTABLE: Self = Self(1 << 0);
    /// Enable profiling/debugging
    pub const ENABLE_PROFILING: Self = Self(1 << 1);
    /// Initialize backing memory to zero
    pub const ZERO_INITIALIZE: Self = Self(1 << 2);
}

// ============================================================================
// Work Graph Handle
// ============================================================================

/// Handle to a compiled work graph
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkGraph {
    handle: u64,
}

impl WorkGraph {
    pub const fn null() -> Self {
        Self { handle: 0 }
    }

    pub const fn is_valid(&self) -> bool {
        self.handle != 0
    }

    pub const fn raw(&self) -> u64 {
        self.handle
    }

    pub const unsafe fn from_raw(handle: u64) -> Self {
        Self { handle }
    }
}

// ============================================================================
// Backing Memory
// ============================================================================

/// Memory requirements for a work graph
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkGraphMemoryRequirements {
    /// Size in bytes
    pub size: u64,
    /// Alignment requirement
    pub alignment: u64,
    /// Maximum size needed for max recursion
    pub max_size: u64,
}

/// Backing memory for work graph execution
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkGraphBackingMemory {
    handle: u64,
}

impl WorkGraphBackingMemory {
    pub const fn null() -> Self {
        Self { handle: 0 }
    }

    pub const fn is_valid(&self) -> bool {
        self.handle != 0
    }
}

// ============================================================================
// Dispatch Description
// ============================================================================

/// Description for dispatching a work graph
#[derive(Clone, Debug)]
pub struct DispatchGraphDescription {
    /// Work graph to dispatch
    pub graph: WorkGraph,
    /// Backing memory
    pub backing_memory: WorkGraphBackingMemory,
    /// Entry point index
    pub entry_point: u32,
    /// Input records for the entry point
    pub input_records: DispatchInputRecords,
    /// Flags
    pub flags: DispatchGraphFlags,
}

/// Input records for dispatch
#[derive(Clone, Debug)]
pub enum DispatchInputRecords {
    /// CPU-provided records (uploaded before dispatch)
    CpuRecords {
        /// Record data
        data: [u8; 4096],
        /// Data size
        size: usize,
        /// Record count
        count: u32,
        /// Stride between records
        stride: u32,
    },
    /// GPU buffer containing records
    GpuBuffer {
        /// Buffer handle
        buffer: u64,
        /// Offset in buffer
        offset: u64,
        /// Record count
        count: u32,
        /// Stride between records
        stride: u32,
    },
    /// GPU buffer with count stored in another buffer
    GpuBufferWithCount {
        /// Records buffer handle
        buffer: u64,
        /// Offset in records buffer
        offset: u64,
        /// Count buffer handle
        count_buffer: u64,
        /// Offset in count buffer
        count_offset: u64,
        /// Maximum record count
        max_count: u32,
        /// Stride between records
        stride: u32,
    },
    /// No input records (graph generates its own work)
    None,
}

impl Default for DispatchInputRecords {
    fn default() -> Self {
        Self::None
    }
}

/// Dispatch graph flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchGraphFlags(u32);

impl DispatchGraphFlags {
    pub const NONE: Self = Self(0);
    /// Initialize backing memory before dispatch
    pub const INITIALIZE: Self = Self(1 << 0);
}

// ============================================================================
// Work Graph Statistics
// ============================================================================

/// Statistics from a work graph execution
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkGraphStatistics {
    /// Total nodes executed
    pub nodes_executed: u64,
    /// Total records processed
    pub records_processed: u64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Total GPU cycles (if available)
    pub gpu_cycles: u64,
    /// Per-node execution counts
    pub node_execution_counts: [u64; 64],
}

// ============================================================================
// Command Buffer Extensions
// ============================================================================

/// Extension trait for work graph commands
pub trait WorkGraphCommands {
    /// Initialize work graph backing memory
    fn initialize_work_graph_memory(
        &mut self,
        graph: WorkGraph,
        backing_memory: WorkGraphBackingMemory,
    );

    /// Dispatch a work graph
    fn dispatch_graph(&mut self, desc: &DispatchGraphDescription);

    /// Set work graph root constants
    fn set_work_graph_root_constants(&mut self, graph: WorkGraph, offset: u32, data: &[u32]);

    /// Barrier between work graph dispatches
    fn work_graph_barrier(&mut self);
}

// ============================================================================
// Work Graph Builder
// ============================================================================

/// Builder for creating work graphs
#[derive(Clone, Debug, Default)]
pub struct WorkGraphBuilder {
    name: GraphName,
    nodes: [Option<WorkGraphNode>; 64],
    node_count: u32,
    edges: [NodeEdge; 256],
    edge_count: u32,
    entry_points: [u32; 16],
    entry_point_count: u32,
    flags: WorkGraphFlags,
}

impl WorkGraphBuilder {
    /// Create a new builder
    pub fn new(name: &str) -> Self {
        Self {
            name: GraphName::new(name),
            ..Default::default()
        }
    }

    /// Add a broadcasting launch node
    pub fn add_broadcasting_node(
        &mut self,
        name: &str,
        shader: NodeShader,
        local_size: [u32; 3],
    ) -> Result<u32> {
        self.add_node_internal(
            name,
            NodeType::BroadcastingLaunch,
            shader,
            local_size,
            false,
        )
    }

    /// Add a coalescing launch node
    pub fn add_coalescing_node(
        &mut self,
        name: &str,
        shader: NodeShader,
        local_size: [u32; 3],
    ) -> Result<u32> {
        self.add_node_internal(name, NodeType::CoalescingLaunch, shader, local_size, false)
    }

    /// Add a thread launch node
    pub fn add_thread_node(&mut self, name: &str, shader: NodeShader) -> Result<u32> {
        self.add_node_internal(name, NodeType::ThreadLaunch, shader, [1, 1, 1], false)
    }

    /// Add an entry point node
    pub fn add_entry_point(
        &mut self,
        name: &str,
        node_type: NodeType,
        shader: NodeShader,
        local_size: [u32; 3],
    ) -> Result<u32> {
        let node_id = self.add_node_internal(name, node_type, shader, local_size, true)?;

        if self.entry_point_count >= 16 {
            return Err(Error::OutOfMemory);
        }
        self.entry_points[self.entry_point_count as usize] = node_id;
        self.entry_point_count += 1;

        Ok(node_id)
    }

    fn add_node_internal(
        &mut self,
        name: &str,
        node_type: NodeType,
        shader: NodeShader,
        local_size: [u32; 3],
        is_entry_point: bool,
    ) -> Result<u32> {
        if self.node_count >= 64 {
            return Err(Error::OutOfMemory);
        }

        let id = self.node_count;
        self.nodes[id as usize] = Some(WorkGraphNode {
            id,
            name: NodeName::new(name),
            node_type,
            shader,
            input_record: None,
            output_records: Default::default(),
            output_count: 0,
            local_size,
            max_dispatch_grid: None,
            is_entry_point,
            flags: NodeFlags::NONE,
        });
        self.node_count += 1;

        Ok(id)
    }

    /// Connect two nodes
    pub fn connect(&mut self, source: u32, source_output: u32, dest: u32) -> Result<()> {
        if self.edge_count >= 256 {
            return Err(Error::OutOfMemory);
        }

        self.edges[self.edge_count as usize] = NodeEdge::new(source, source_output, dest, 0);
        self.edge_count += 1;

        Ok(())
    }

    /// Set input record for a node
    pub fn set_input_record(&mut self, node_id: u32, record: RecordDefinition) -> Result<()> {
        if let Some(ref mut node) = self
            .nodes
            .get_mut(node_id as usize)
            .and_then(|n| n.as_mut())
        {
            node.input_record = Some(record);
            Ok(())
        } else {
            Err(Error::InvalidHandle)
        }
    }

    /// Add output record to a node
    pub fn add_output_record(&mut self, node_id: u32, record: RecordDefinition) -> Result<u32> {
        if let Some(ref mut node) = self
            .nodes
            .get_mut(node_id as usize)
            .and_then(|n| n.as_mut())
        {
            if node.output_count >= 8 {
                return Err(Error::OutOfMemory);
            }
            let output_index = node.output_count;
            node.output_records[output_index as usize] = Some(record);
            node.output_count += 1;
            Ok(output_index)
        } else {
            Err(Error::InvalidHandle)
        }
    }

    /// Build the work graph definition
    pub fn build(self) -> Result<WorkGraphDefinition> {
        if self.node_count == 0 {
            return Err(Error::InvalidParameter);
        }
        if self.entry_point_count == 0 {
            return Err(Error::InvalidParameter);
        }

        // Calculate backing memory size (simplified)
        let backing_memory_size = self.calculate_backing_memory();

        Ok(WorkGraphDefinition {
            name: self.name,
            nodes: self.nodes,
            node_count: self.node_count,
            entry_points: self.entry_points,
            entry_point_count: self.entry_point_count,
            edges: self.edges,
            edge_count: self.edge_count,
            backing_memory_size,
            flags: self.flags,
        })
    }

    fn calculate_backing_memory(&self) -> u64 {
        // Simplified calculation - real implementation would be more complex
        let mut size = 0u64;

        for node in self.nodes.iter().flatten() {
            // Base overhead per node
            size += 4096;

            // Input record buffer
            if let Some(ref input) = node.input_record {
                size += (input.size as u64) * 1024; // Assume up to 1K records
            }

            // Output record buffers
            for output in node.output_records.iter().flatten() {
                size += (output.size as u64) * 1024;
            }
        }

        // Align to 64KB
        (size + 65535) & !65535
    }
}

// ============================================================================
// Helper Macros
// ============================================================================

/// Macro for defining a record structure
#[macro_export]
macro_rules! define_record {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name {
            $(pub $field: $ty,)*
        }
    };
}
