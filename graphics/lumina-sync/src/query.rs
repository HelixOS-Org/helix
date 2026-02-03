//! GPU Queries
//!
//! Timestamp, occlusion, and pipeline statistics queries.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;

use lumina_core::Handle;

// ============================================================================
// Query Type
// ============================================================================

/// Query type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Occlusion query.
    Occlusion,
    /// Pipeline statistics query.
    PipelineStatistics,
    /// Timestamp query.
    Timestamp,
    /// Transform feedback primitives written.
    TransformFeedbackStream,
    /// Acceleration structure compacted size.
    AccelerationStructureCompactedSize,
    /// Acceleration structure serialization size.
    AccelerationStructureSerializationSize,
}

// ============================================================================
// Pipeline Statistics Flags
// ============================================================================

bitflags! {
    /// Pipeline statistics flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PipelineStatisticsFlags: u32 {
        /// Input assembly vertices.
        const INPUT_ASSEMBLY_VERTICES = 1 << 0;
        /// Input assembly primitives.
        const INPUT_ASSEMBLY_PRIMITIVES = 1 << 1;
        /// Vertex shader invocations.
        const VERTEX_SHADER_INVOCATIONS = 1 << 2;
        /// Geometry shader invocations.
        const GEOMETRY_SHADER_INVOCATIONS = 1 << 3;
        /// Geometry shader primitives.
        const GEOMETRY_SHADER_PRIMITIVES = 1 << 4;
        /// Clipping invocations.
        const CLIPPING_INVOCATIONS = 1 << 5;
        /// Clipping primitives.
        const CLIPPING_PRIMITIVES = 1 << 6;
        /// Fragment shader invocations.
        const FRAGMENT_SHADER_INVOCATIONS = 1 << 7;
        /// Tessellation control shader patches.
        const TESSELLATION_CONTROL_SHADER_PATCHES = 1 << 8;
        /// Tessellation evaluation shader invocations.
        const TESSELLATION_EVALUATION_SHADER_INVOCATIONS = 1 << 9;
        /// Compute shader invocations.
        const COMPUTE_SHADER_INVOCATIONS = 1 << 10;
        /// Task shader invocations.
        const TASK_SHADER_INVOCATIONS = 1 << 11;
        /// Mesh shader invocations.
        const MESH_SHADER_INVOCATIONS = 1 << 12;
    }
}

impl Default for PipelineStatisticsFlags {
    fn default() -> Self {
        PipelineStatisticsFlags::all()
    }
}

// ============================================================================
// Query Pool Handle
// ============================================================================

/// Handle to a query pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryPoolHandle(Handle<QueryPool>);

impl QueryPoolHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }
}

// ============================================================================
// Query Pool Description
// ============================================================================

/// Description for query pool creation.
#[derive(Debug, Clone)]
pub struct QueryPoolDesc {
    /// Query type.
    pub query_type: QueryType,
    /// Query count.
    pub query_count: u32,
    /// Pipeline statistics flags.
    pub pipeline_statistics: PipelineStatisticsFlags,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for QueryPoolDesc {
    fn default() -> Self {
        Self {
            query_type: QueryType::Timestamp,
            query_count: 64,
            pipeline_statistics: PipelineStatisticsFlags::empty(),
            label: None,
        }
    }
}

impl QueryPoolDesc {
    /// Create a timestamp query pool.
    pub fn timestamp(count: u32) -> Self {
        Self {
            query_type: QueryType::Timestamp,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::empty(),
            label: None,
        }
    }

    /// Create an occlusion query pool.
    pub fn occlusion(count: u32) -> Self {
        Self {
            query_type: QueryType::Occlusion,
            query_count: count,
            pipeline_statistics: PipelineStatisticsFlags::empty(),
            label: None,
        }
    }

    /// Create a pipeline statistics query pool.
    pub fn pipeline_statistics(count: u32, flags: PipelineStatisticsFlags) -> Self {
        Self {
            query_type: QueryType::PipelineStatistics,
            query_count: count,
            pipeline_statistics: flags,
            label: None,
        }
    }

    /// Set debug label.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(String::from(label));
        self
    }
}

// ============================================================================
// Query Pool
// ============================================================================

/// A query pool.
pub struct QueryPool {
    /// Handle.
    pub handle: QueryPoolHandle,
    /// Query type.
    pub query_type: QueryType,
    /// Query count.
    pub query_count: u32,
    /// Pipeline statistics flags.
    pub pipeline_statistics: PipelineStatisticsFlags,
    /// Query results.
    results: Vec<u64>,
    /// Query availability.
    availability: Vec<bool>,
    /// Debug label.
    pub label: Option<String>,
}

impl QueryPool {
    /// Create a new query pool.
    pub fn new(handle: QueryPoolHandle, desc: &QueryPoolDesc) -> Self {
        let result_count = match desc.query_type {
            QueryType::PipelineStatistics => {
                desc.query_count as usize * desc.pipeline_statistics.bits().count_ones() as usize
            }
            _ => desc.query_count as usize,
        };

        Self {
            handle,
            query_type: desc.query_type,
            query_count: desc.query_count,
            pipeline_statistics: desc.pipeline_statistics,
            results: vec![0; result_count],
            availability: vec![false; desc.query_count as usize],
            label: desc.label.clone(),
        }
    }

    /// Reset queries.
    pub fn reset(&mut self, first_query: u32, query_count: u32) {
        let end = ((first_query + query_count) as usize).min(self.availability.len());
        for i in first_query as usize..end {
            self.availability[i] = false;
        }
    }

    /// Set query result.
    pub fn set_result(&mut self, query: u32, result: u64) {
        if (query as usize) < self.results.len() {
            self.results[query as usize] = result;
            self.availability[query as usize] = true;
        }
    }

    /// Get query result.
    pub fn get_result(&self, query: u32) -> Option<u64> {
        if (query as usize) < self.results.len() && self.availability[query as usize] {
            Some(self.results[query as usize])
        } else {
            None
        }
    }

    /// Get all results.
    pub fn get_results(&self, first_query: u32, query_count: u32) -> Vec<Option<u64>> {
        let mut results = Vec::with_capacity(query_count as usize);
        for i in 0..query_count {
            results.push(self.get_result(first_query + i));
        }
        results
    }

    /// Check if query is available.
    pub fn is_available(&self, query: u32) -> bool {
        (query as usize) < self.availability.len() && self.availability[query as usize]
    }
}

// ============================================================================
// Timestamp Query
// ============================================================================

/// Timestamp query helper.
pub struct TimestampQuery {
    /// Query pool.
    pub pool: QueryPoolHandle,
    /// Start query index.
    pub start_query: u32,
    /// End query index.
    pub end_query: u32,
    /// Timestamp period (nanoseconds per tick).
    pub timestamp_period: f32,
}

impl TimestampQuery {
    /// Create a new timestamp query.
    pub fn new(pool: QueryPoolHandle, start: u32, end: u32, period: f32) -> Self {
        Self {
            pool,
            start_query: start,
            end_query: end,
            timestamp_period: period,
        }
    }

    /// Calculate duration from results.
    pub fn duration_ns(&self, start_timestamp: u64, end_timestamp: u64) -> f64 {
        let diff = end_timestamp.saturating_sub(start_timestamp);
        diff as f64 * self.timestamp_period as f64
    }

    /// Calculate duration in milliseconds.
    pub fn duration_ms(&self, start_timestamp: u64, end_timestamp: u64) -> f64 {
        self.duration_ns(start_timestamp, end_timestamp) / 1_000_000.0
    }
}

// ============================================================================
// Occlusion Query
// ============================================================================

/// Occlusion query helper.
pub struct OcclusionQuery {
    /// Query pool.
    pub pool: QueryPoolHandle,
    /// Query index.
    pub query: u32,
    /// Is precise (exact count vs. any visible).
    pub precise: bool,
}

impl OcclusionQuery {
    /// Create a new occlusion query.
    pub fn new(pool: QueryPoolHandle, query: u32, precise: bool) -> Self {
        Self { pool, query, precise }
    }

    /// Check if any samples passed.
    pub fn is_visible(&self, result: u64) -> bool {
        result > 0
    }

    /// Get sample count (precise only).
    pub fn sample_count(&self, result: u64) -> u64 {
        result
    }
}

// ============================================================================
// Pipeline Statistics Query
// ============================================================================

/// Pipeline statistics query result.
#[derive(Debug, Clone, Default)]
pub struct PipelineStatisticsResult {
    /// Input assembly vertices.
    pub input_assembly_vertices: u64,
    /// Input assembly primitives.
    pub input_assembly_primitives: u64,
    /// Vertex shader invocations.
    pub vertex_shader_invocations: u64,
    /// Geometry shader invocations.
    pub geometry_shader_invocations: u64,
    /// Geometry shader primitives.
    pub geometry_shader_primitives: u64,
    /// Clipping invocations.
    pub clipping_invocations: u64,
    /// Clipping primitives.
    pub clipping_primitives: u64,
    /// Fragment shader invocations.
    pub fragment_shader_invocations: u64,
    /// Tessellation control shader patches.
    pub tessellation_control_shader_patches: u64,
    /// Tessellation evaluation shader invocations.
    pub tessellation_evaluation_shader_invocations: u64,
    /// Compute shader invocations.
    pub compute_shader_invocations: u64,
}

/// Pipeline statistics query helper.
pub struct PipelineStatisticsQuery {
    /// Query pool.
    pub pool: QueryPoolHandle,
    /// Query index.
    pub query: u32,
    /// Statistics flags.
    pub flags: PipelineStatisticsFlags,
}

impl PipelineStatisticsQuery {
    /// Create a new pipeline statistics query.
    pub fn new(pool: QueryPoolHandle, query: u32, flags: PipelineStatisticsFlags) -> Self {
        Self { pool, query, flags }
    }

    /// Parse results into statistics structure.
    pub fn parse_results(&self, results: &[u64]) -> PipelineStatisticsResult {
        let mut stats = PipelineStatisticsResult::default();
        let mut index = 0;

        if self.flags.contains(PipelineStatisticsFlags::INPUT_ASSEMBLY_VERTICES) {
            stats.input_assembly_vertices = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::INPUT_ASSEMBLY_PRIMITIVES) {
            stats.input_assembly_primitives = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::VERTEX_SHADER_INVOCATIONS) {
            stats.vertex_shader_invocations = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::GEOMETRY_SHADER_INVOCATIONS) {
            stats.geometry_shader_invocations = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::GEOMETRY_SHADER_PRIMITIVES) {
            stats.geometry_shader_primitives = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::CLIPPING_INVOCATIONS) {
            stats.clipping_invocations = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::CLIPPING_PRIMITIVES) {
            stats.clipping_primitives = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::FRAGMENT_SHADER_INVOCATIONS) {
            stats.fragment_shader_invocations = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::TESSELLATION_CONTROL_SHADER_PATCHES) {
            stats.tessellation_control_shader_patches = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::TESSELLATION_EVALUATION_SHADER_INVOCATIONS) {
            stats.tessellation_evaluation_shader_invocations = results.get(index).copied().unwrap_or(0);
            index += 1;
        }
        if self.flags.contains(PipelineStatisticsFlags::COMPUTE_SHADER_INVOCATIONS) {
            stats.compute_shader_invocations = results.get(index).copied().unwrap_or(0);
        }

        stats
    }
}

// ============================================================================
// Query Pool Manager
// ============================================================================

/// Manages query pools.
pub struct QueryPoolManager {
    /// Query pools.
    pools: Vec<Option<QueryPool>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Pool count.
    count: AtomicU32,
}

impl QueryPoolManager {
    /// Create a new query pool manager.
    pub fn new() -> Self {
        Self {
            pools: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            count: AtomicU32::new(0),
        }
    }

    /// Create a query pool.
    pub fn create(&mut self, desc: &QueryPoolDesc) -> QueryPoolHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.pools.len() as u32;
            self.pools.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = QueryPoolHandle::new(index, generation);
        let pool = QueryPool::new(handle, desc);

        self.pools[index as usize] = Some(pool);
        self.count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Destroy a query pool.
    pub fn destroy(&mut self, handle: QueryPoolHandle) {
        let index = handle.index() as usize;
        if index < self.pools.len() && self.generations[index] == handle.generation() {
            self.pools[index] = None;
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
            self.count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get a query pool.
    pub fn get(&self, handle: QueryPoolHandle) -> Option<&QueryPool> {
        let index = handle.index() as usize;
        if index >= self.pools.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.pools[index].as_ref()
    }

    /// Get a query pool mutably.
    pub fn get_mut(&mut self, handle: QueryPoolHandle) -> Option<&mut QueryPool> {
        let index = handle.index() as usize;
        if index >= self.pools.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.pools[index].as_mut()
    }

    /// Reset queries.
    pub fn reset(&mut self, handle: QueryPoolHandle, first_query: u32, query_count: u32) {
        if let Some(pool) = self.get_mut(handle) {
            pool.reset(first_query, query_count);
        }
    }

    /// Get query result.
    pub fn get_result(&self, handle: QueryPoolHandle, query: u32) -> Option<u64> {
        self.get(handle).and_then(|p| p.get_result(query))
    }

    /// Get pool count.
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }
}

impl Default for QueryPoolManager {
    fn default() -> Self {
        Self::new()
    }
}
