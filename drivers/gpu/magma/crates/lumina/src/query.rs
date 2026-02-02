//! Query pools for occlusion, timestamp, and pipeline statistics
//!
//! This module provides types for GPU query operations.

use crate::types::BufferHandle;

/// Query pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct QueryPoolHandle(pub u64);

impl QueryPoolHandle {
    /// Null/invalid query pool
    pub const NULL: Self = Self(0);

    /// Creates a query pool handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Query type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryType {
    /// Occlusion query (counts fragments that pass depth test)
    Occlusion,
    /// Pipeline statistics (various counters)
    PipelineStatistics(PipelineStatisticsFlags),
    /// Timestamp query
    Timestamp,
}

/// Pipeline statistics flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct PipelineStatisticsFlags(pub u32);

impl PipelineStatisticsFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Input assembly vertices
    pub const INPUT_ASSEMBLY_VERTICES: Self = Self(1 << 0);
    /// Input assembly primitives
    pub const INPUT_ASSEMBLY_PRIMITIVES: Self = Self(1 << 1);
    /// Vertex shader invocations
    pub const VERTEX_SHADER_INVOCATIONS: Self = Self(1 << 2);
    /// Geometry shader invocations
    pub const GEOMETRY_SHADER_INVOCATIONS: Self = Self(1 << 3);
    /// Geometry shader primitives
    pub const GEOMETRY_SHADER_PRIMITIVES: Self = Self(1 << 4);
    /// Clipping invocations
    pub const CLIPPING_INVOCATIONS: Self = Self(1 << 5);
    /// Clipping primitives
    pub const CLIPPING_PRIMITIVES: Self = Self(1 << 6);
    /// Fragment shader invocations
    pub const FRAGMENT_SHADER_INVOCATIONS: Self = Self(1 << 7);
    /// Tessellation control patches
    pub const TESSELLATION_CONTROL_PATCHES: Self = Self(1 << 8);
    /// Tessellation evaluation invocations
    pub const TESSELLATION_EVALUATION_INVOCATIONS: Self = Self(1 << 9);
    /// Compute shader invocations
    pub const COMPUTE_SHADER_INVOCATIONS: Self = Self(1 << 10);
    /// All statistics
    pub const ALL: Self = Self(0x7FF);

    /// Checks if flag is set
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Counts the number of statistics flags set
    pub const fn count(self) -> u32 {
        (self.0 as u32).count_ones()
    }
}

impl core::ops::BitOr for PipelineStatisticsFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for PipelineStatisticsFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Query pool descriptor
#[derive(Clone, Debug)]
pub struct QueryPoolDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Query type
    pub query_type: QueryType,
    /// Number of queries in the pool
    pub query_count: u32,
}

impl<'a> QueryPoolDesc<'a> {
    /// Creates an occlusion query pool
    pub const fn occlusion(query_count: u32) -> Self {
        Self {
            label: None,
            query_type: QueryType::Occlusion,
            query_count,
        }
    }

    /// Creates a timestamp query pool
    pub const fn timestamp(query_count: u32) -> Self {
        Self {
            label: None,
            query_type: QueryType::Timestamp,
            query_count,
        }
    }

    /// Creates a pipeline statistics query pool
    pub const fn pipeline_statistics(query_count: u32, flags: PipelineStatisticsFlags) -> Self {
        Self {
            label: None,
            query_type: QueryType::PipelineStatistics(flags),
            query_count,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Query result flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct QueryResultFlags(pub u32);

impl QueryResultFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Return 64-bit results
    pub const RESULT_64: Self = Self(1 << 0);
    /// Wait for results
    pub const WAIT: Self = Self(1 << 1);
    /// Include availability bit
    pub const WITH_AVAILABILITY: Self = Self(1 << 2);
    /// Allow partial results
    pub const PARTIAL: Self = Self(1 << 3);
}

impl core::ops::BitOr for QueryResultFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Occlusion query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OcclusionQueryResult {
    /// Number of samples that passed
    pub samples_passed: u64,
    /// Whether result is available
    pub available: bool,
}

impl OcclusionQueryResult {
    /// Checks if any samples passed
    pub const fn is_visible(&self) -> bool {
        self.samples_passed > 0
    }
}

/// Timestamp query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TimestampQueryResult {
    /// Timestamp value in GPU ticks
    pub timestamp: u64,
    /// Whether result is available
    pub available: bool,
}

/// Pipeline statistics query result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineStatisticsResult {
    /// Input assembly vertices
    pub input_assembly_vertices: u64,
    /// Input assembly primitives
    pub input_assembly_primitives: u64,
    /// Vertex shader invocations
    pub vertex_shader_invocations: u64,
    /// Geometry shader invocations
    pub geometry_shader_invocations: u64,
    /// Geometry shader primitives
    pub geometry_shader_primitives: u64,
    /// Clipping invocations
    pub clipping_invocations: u64,
    /// Clipping primitives
    pub clipping_primitives: u64,
    /// Fragment shader invocations
    pub fragment_shader_invocations: u64,
    /// Tessellation control patches
    pub tessellation_control_patches: u64,
    /// Tessellation evaluation invocations
    pub tessellation_evaluation_invocations: u64,
    /// Compute shader invocations
    pub compute_shader_invocations: u64,
    /// Whether result is available
    pub available: bool,
}

/// Copy query results to buffer
#[derive(Clone, Copy, Debug)]
pub struct CopyQueryResults {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// First query index
    pub first_query: u32,
    /// Number of queries to copy
    pub query_count: u32,
    /// Destination buffer
    pub dst_buffer: BufferHandle,
    /// Offset in destination buffer
    pub dst_offset: u64,
    /// Stride between query results
    pub stride: u64,
    /// Result flags
    pub flags: QueryResultFlags,
}

impl CopyQueryResults {
    /// Creates a copy query results command
    pub const fn new(
        query_pool: QueryPoolHandle,
        first_query: u32,
        query_count: u32,
        dst_buffer: BufferHandle,
    ) -> Self {
        Self {
            query_pool,
            first_query,
            query_count,
            dst_buffer,
            dst_offset: 0,
            stride: 8, // 64-bit result
            flags: QueryResultFlags::RESULT_64,
        }
    }

    /// Sets the destination offset
    pub const fn with_offset(mut self, offset: u64) -> Self {
        self.dst_offset = offset;
        self
    }

    /// Sets the stride
    pub const fn with_stride(mut self, stride: u64) -> Self {
        self.stride = stride;
        self
    }

    /// Sets flags
    pub const fn with_flags(mut self, flags: QueryResultFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Timestamp period converter
#[derive(Clone, Copy, Debug)]
pub struct TimestampPeriod {
    /// Nanoseconds per tick
    pub nanoseconds_per_tick: f32,
}

impl TimestampPeriod {
    /// Creates a timestamp period converter
    pub const fn new(nanoseconds_per_tick: f32) -> Self {
        Self { nanoseconds_per_tick }
    }

    /// Converts GPU ticks to nanoseconds
    pub fn ticks_to_nanoseconds(&self, ticks: u64) -> f64 {
        ticks as f64 * self.nanoseconds_per_tick as f64
    }

    /// Converts GPU ticks to microseconds
    pub fn ticks_to_microseconds(&self, ticks: u64) -> f64 {
        self.ticks_to_nanoseconds(ticks) / 1000.0
    }

    /// Converts GPU ticks to milliseconds
    pub fn ticks_to_milliseconds(&self, ticks: u64) -> f64 {
        self.ticks_to_nanoseconds(ticks) / 1_000_000.0
    }

    /// Computes duration between two timestamps in nanoseconds
    pub fn duration_nanoseconds(&self, start: u64, end: u64) -> f64 {
        let diff = if end >= start { end - start } else { start - end };
        self.ticks_to_nanoseconds(diff)
    }
}

/// GPU timing scope for profiling
#[derive(Clone, Copy, Debug)]
pub struct TimingScope {
    /// Query pool
    pub pool: QueryPoolHandle,
    /// Start query index
    pub start_query: u32,
    /// End query index
    pub end_query: u32,
}

impl TimingScope {
    /// Creates a timing scope
    pub const fn new(pool: QueryPoolHandle, start_query: u32) -> Self {
        Self {
            pool,
            start_query,
            end_query: start_query + 1,
        }
    }

    /// Computes duration between start and end timestamps
    pub fn duration(&self, start_time: u64, end_time: u64, period: TimestampPeriod) -> f64 {
        period.duration_nanoseconds(start_time, end_time)
    }
}

/// Query pool allocator for managing query indices
pub struct QueryPoolAllocator {
    /// Next available query index
    next_query: u32,
    /// Total capacity
    capacity: u32,
    /// Recycled queries (simple free list)
    free_list: [u32; 64],
    /// Number of free queries
    free_count: usize,
}

impl QueryPoolAllocator {
    /// Creates a new query pool allocator
    pub const fn new(capacity: u32) -> Self {
        Self {
            next_query: 0,
            capacity,
            free_list: [0; 64],
            free_count: 0,
        }
    }

    /// Allocates a query index
    pub fn allocate(&mut self) -> Option<u32> {
        // Check free list first
        if self.free_count > 0 {
            self.free_count -= 1;
            return Some(self.free_list[self.free_count]);
        }

        // Allocate new
        if self.next_query < self.capacity {
            let query = self.next_query;
            self.next_query += 1;
            Some(query)
        } else {
            None
        }
    }

    /// Allocates a range of consecutive queries
    pub fn allocate_range(&mut self, count: u32) -> Option<u32> {
        if self.next_query + count <= self.capacity {
            let start = self.next_query;
            self.next_query += count;
            Some(start)
        } else {
            None
        }
    }

    /// Frees a query index
    pub fn free(&mut self, query: u32) {
        if self.free_count < 64 {
            self.free_list[self.free_count] = query;
            self.free_count += 1;
        }
    }

    /// Resets the allocator
    pub fn reset(&mut self) {
        self.next_query = 0;
        self.free_count = 0;
    }

    /// Returns the number of allocated queries
    pub const fn allocated_count(&self) -> u32 {
        self.next_query
    }

    /// Returns remaining capacity
    pub const fn remaining(&self) -> u32 {
        self.capacity - self.next_query + self.free_count as u32
    }
}

/// Conditional rendering flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ConditionalRenderingFlags(pub u32);

impl ConditionalRenderingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Invert condition
    pub const INVERTED: Self = Self(1 << 0);
}

/// Conditional rendering info
#[derive(Clone, Copy, Debug)]
pub struct ConditionalRenderingInfo {
    /// Buffer containing condition value
    pub buffer: BufferHandle,
    /// Offset in buffer
    pub offset: u64,
    /// Flags
    pub flags: ConditionalRenderingFlags,
}

impl ConditionalRenderingInfo {
    /// Creates conditional rendering info
    pub const fn new(buffer: BufferHandle, offset: u64) -> Self {
        Self {
            buffer,
            offset,
            flags: ConditionalRenderingFlags::NONE,
        }
    }

    /// Inverts the condition
    pub const fn inverted(mut self) -> Self {
        self.flags = ConditionalRenderingFlags::INVERTED;
        self
    }
}

/// Performance counter descriptor
#[derive(Clone, Debug)]
pub struct PerformanceCounterDesc<'a> {
    /// Counter name
    pub name: &'a str,
    /// Counter description
    pub description: Option<&'a str>,
    /// Unit of measurement
    pub unit: PerformanceCounterUnit,
}

/// Performance counter unit
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceCounterUnit {
    /// Generic counter
    Generic,
    /// Percentage (0-100)
    Percentage,
    /// Nanoseconds
    Nanoseconds,
    /// Bytes
    Bytes,
    /// Bytes per second
    BytesPerSecond,
    /// Kelvin
    Kelvin,
    /// Watts
    Watts,
    /// Volts
    Volts,
    /// Amps
    Amps,
    /// Hertz
    Hertz,
    /// Cycles
    Cycles,
}
