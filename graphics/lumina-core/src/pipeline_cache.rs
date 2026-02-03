//! Pipeline cache and library management
//!
//! This module provides types for pipeline caching and precompilation.

use core::num::NonZeroU32;

/// Pipeline cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheHandle(pub NonZeroU32);

impl PipelineCacheHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Pipeline cache creation info
#[derive(Clone, Debug)]
pub struct PipelineCacheCreateInfo {
    /// Initial data
    pub initial_data: alloc::vec::Vec<u8>,
    /// Flags
    pub flags: PipelineCacheCreateFlags,
}

use alloc::vec::Vec;

impl PipelineCacheCreateInfo {
    /// Creates empty cache info
    pub fn new() -> Self {
        Self {
            initial_data: Vec::new(),
            flags: PipelineCacheCreateFlags::empty(),
        }
    }

    /// Creates from initial data
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            initial_data: data.to_vec(),
            flags: PipelineCacheCreateFlags::empty(),
        }
    }

    /// Uses externally synchronized cache
    pub fn externally_synchronized(mut self) -> Self {
        self.flags |= PipelineCacheCreateFlags::EXTERNALLY_SYNCHRONIZED;
        self
    }
}

impl Default for PipelineCacheCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Pipeline cache creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PipelineCacheCreateFlags: u32 {
        /// Cache is externally synchronized
        const EXTERNALLY_SYNCHRONIZED = 1 << 0;
    }
}

impl PipelineCacheCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Pipeline cache header info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PipelineCacheHeaderVersion {
    /// Header size
    pub header_size: u32,
    /// Header version
    pub header_version: PipelineCacheHeaderVersionNumber,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
}

/// Pipeline cache header version number
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PipelineCacheHeaderVersionNumber {
    /// Version 1
    One = 1,
}

/// Pipeline creation feedback
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PipelineCreationFeedback {
    /// Feedback flags
    pub flags: PipelineCreationFeedbackFlags,
    /// Duration in nanoseconds
    pub duration: u64,
}

impl PipelineCreationFeedback {
    /// Empty feedback
    pub const fn empty() -> Self {
        Self {
            flags: PipelineCreationFeedbackFlags::empty(),
            duration: 0,
        }
    }

    /// Was the pipeline creation successful and fast?
    pub const fn was_valid_and_fast(&self) -> bool {
        self.flags
            .contains(PipelineCreationFeedbackFlags::VALID)
            && self
                .flags
                .contains(PipelineCreationFeedbackFlags::APPLICATION_PIPELINE_CACHE_HIT)
    }

    /// Did we hit the cache?
    pub const fn cache_hit(&self) -> bool {
        self.flags
            .contains(PipelineCreationFeedbackFlags::APPLICATION_PIPELINE_CACHE_HIT)
    }

    /// Duration in milliseconds
    pub const fn duration_ms(&self) -> f64 {
        self.duration as f64 / 1_000_000.0
    }
}

bitflags::bitflags! {
    /// Pipeline creation feedback flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PipelineCreationFeedbackFlags: u32 {
        /// Feedback is valid
        const VALID = 1 << 0;
        /// Pipeline was found in application cache
        const APPLICATION_PIPELINE_CACHE_HIT = 1 << 1;
        /// Pipeline was compiled from base pipeline
        const BASE_PIPELINE_ACCELERATION = 1 << 2;
    }
}

impl PipelineCreationFeedbackFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Pipeline creation feedback info
#[derive(Clone, Debug)]
pub struct PipelineCreationFeedbackCreateInfo {
    /// Feedback for the whole pipeline
    pub pipeline_creation_feedback: PipelineCreationFeedback,
    /// Feedback for each stage
    pub pipeline_stage_creation_feedbacks: alloc::vec::Vec<PipelineCreationFeedback>,
}

impl PipelineCreationFeedbackCreateInfo {
    /// Creates feedback info
    pub fn new(stage_count: usize) -> Self {
        Self {
            pipeline_creation_feedback: PipelineCreationFeedback::empty(),
            pipeline_stage_creation_feedbacks: alloc::vec![PipelineCreationFeedback::empty(); stage_count],
        }
    }
}

/// Graphics pipeline library creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GraphicsPipelineLibraryCreateInfo {
    /// Library flags
    pub flags: GraphicsPipelineLibraryFlags,
}

bitflags::bitflags! {
    /// Graphics pipeline library flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct GraphicsPipelineLibraryFlags: u32 {
        /// Vertex input interface
        const VERTEX_INPUT_INTERFACE = 1 << 0;
        /// Pre-rasterization shaders
        const PRE_RASTERIZATION_SHADERS = 1 << 1;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 2;
        /// Fragment output interface
        const FRAGMENT_OUTPUT_INTERFACE = 1 << 3;
    }
}

impl GraphicsPipelineLibraryFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// All stages for monolithic pipeline
    pub const ALL: Self = Self::from_bits_truncate(
        Self::VERTEX_INPUT_INTERFACE.bits()
            | Self::PRE_RASTERIZATION_SHADERS.bits()
            | Self::FRAGMENT_SHADER.bits()
            | Self::FRAGMENT_OUTPUT_INTERFACE.bits(),
    );
}

/// Pipeline executable info
#[derive(Clone, Debug)]
pub struct PipelineExecutableInfo {
    /// Shader stages
    pub stages: ShaderStageFlags,
    /// Name
    pub name: alloc::string::String,
    /// Description
    pub description: alloc::string::String,
    /// Subgroup size
    pub subgroup_size: u32,
}

bitflags::bitflags! {
    /// Shader stage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ShaderStageFlags: u32 {
        /// Vertex shader
        const VERTEX = 1 << 0;
        /// Tessellation control shader
        const TESSELLATION_CONTROL = 1 << 1;
        /// Tessellation evaluation shader
        const TESSELLATION_EVALUATION = 1 << 2;
        /// Geometry shader
        const GEOMETRY = 1 << 3;
        /// Fragment shader
        const FRAGMENT = 1 << 4;
        /// Compute shader
        const COMPUTE = 1 << 5;
        /// All graphics stages
        const ALL_GRAPHICS = Self::VERTEX.bits()
            | Self::TESSELLATION_CONTROL.bits()
            | Self::TESSELLATION_EVALUATION.bits()
            | Self::GEOMETRY.bits()
            | Self::FRAGMENT.bits();
        /// Task shader
        const TASK = 1 << 6;
        /// Mesh shader
        const MESH = 1 << 7;
        /// Ray generation shader
        const RAYGEN = 1 << 8;
        /// Any hit shader
        const ANY_HIT = 1 << 9;
        /// Closest hit shader
        const CLOSEST_HIT = 1 << 10;
        /// Miss shader
        const MISS = 1 << 11;
        /// Intersection shader
        const INTERSECTION = 1 << 12;
        /// Callable shader
        const CALLABLE = 1 << 13;
    }
}

impl ShaderStageFlags {
    /// No stages
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self::from_bits_truncate(
        Self::RAYGEN.bits()
            | Self::ANY_HIT.bits()
            | Self::CLOSEST_HIT.bits()
            | Self::MISS.bits()
            | Self::INTERSECTION.bits()
            | Self::CALLABLE.bits(),
    );
}

/// Pipeline executable statistic
#[derive(Clone, Debug)]
pub struct PipelineExecutableStatistic {
    /// Name
    pub name: alloc::string::String,
    /// Description
    pub description: alloc::string::String,
    /// Format
    pub format: PipelineExecutableStatisticFormat,
    /// Value
    pub value: PipelineExecutableStatisticValue,
}

/// Statistic format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PipelineExecutableStatisticFormat {
    /// Boolean
    Bool32 = 0,
    /// Signed integer
    Int64 = 1,
    /// Unsigned integer
    Uint64 = 2,
    /// Floating point
    Float64 = 3,
}

/// Statistic value
#[derive(Clone, Copy, Debug)]
pub enum PipelineExecutableStatisticValue {
    /// Boolean value
    Bool(bool),
    /// Signed integer value
    Int(i64),
    /// Unsigned integer value
    Uint(u64),
    /// Floating point value
    Float(f64),
}

/// Pipeline executable internal representation
#[derive(Clone, Debug)]
pub struct PipelineExecutableInternalRepresentation {
    /// Name
    pub name: alloc::string::String,
    /// Description
    pub description: alloc::string::String,
    /// Is text representation
    pub is_text: bool,
    /// Data
    pub data: alloc::vec::Vec<u8>,
}

/// Pipeline binary info
#[derive(Clone, Debug)]
pub struct PipelineBinaryInfo {
    /// Binary key
    pub pipeline_binary_key: [u8; 32],
}

/// Pipeline binary creation info
#[derive(Clone, Debug)]
pub struct PipelineBinaryCreateInfo {
    /// Pipeline to capture binary from
    pub pipeline: Option<PipelineHandle>,
    /// Binary keys for binary identification
    pub keys: alloc::vec::Vec<[u8; 32]>,
}

/// Pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineHandle(pub NonZeroU32);

impl PipelineHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Pipeline binary data info
#[derive(Clone, Debug)]
pub struct PipelineBinaryDataInfo {
    /// Pipeline binary handle
    pub pipeline_binary: PipelineBinaryHandle,
}

/// Pipeline binary handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineBinaryHandle(pub NonZeroU32);

impl PipelineBinaryHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Pipeline binary key
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PipelineBinaryKey {
    /// Key size
    pub key_size: u32,
    /// Key data
    pub key: [u8; 32],
}

impl PipelineBinaryKey {
    /// Empty key
    pub const EMPTY: Self = Self {
        key_size: 0,
        key: [0; 32],
    };

    /// Creates key from data
    pub fn from_data(data: &[u8]) -> Self {
        let mut key = Self::EMPTY;
        let len = data.len().min(32);
        key.key[..len].copy_from_slice(&data[..len]);
        key.key_size = len as u32;
        key
    }
}

/// Shader warming info for precompilation
#[derive(Clone, Debug)]
pub struct ShaderWarmingInfo {
    /// Shader binaries to warm
    pub shaders: alloc::vec::Vec<ShaderModuleHandle>,
    /// Target pipeline cache
    pub cache: Option<PipelineCacheHandle>,
}

/// Shader module handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderModuleHandle(pub NonZeroU32);

impl ShaderModuleHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

impl ShaderWarmingInfo {
    /// Creates shader warming info
    pub fn new() -> Self {
        Self {
            shaders: Vec::new(),
            cache: None,
        }
    }

    /// Adds a shader to warm
    pub fn add_shader(mut self, shader: ShaderModuleHandle) -> Self {
        self.shaders.push(shader);
        self
    }

    /// Sets target cache
    pub fn with_cache(mut self, cache: PipelineCacheHandle) -> Self {
        self.cache = Some(cache);
        self
    }
}

impl Default for ShaderWarmingInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline pool info for pipeline allocation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PipelinePoolInfo {
    /// Maximum number of pipelines
    pub max_pipeline_count: u32,
    /// Maximum memory for pipeline data
    pub max_pipeline_memory: u64,
}

impl PipelinePoolInfo {
    /// Creates pool info
    pub const fn new(max_count: u32, max_memory: u64) -> Self {
        Self {
            max_pipeline_count: max_count,
            max_pipeline_memory: max_memory,
        }
    }

    /// Small pool for testing
    pub const fn small() -> Self {
        Self::new(16, 64 * 1024 * 1024) // 16 pipelines, 64MB
    }

    /// Medium pool for typical applications
    pub const fn medium() -> Self {
        Self::new(256, 256 * 1024 * 1024) // 256 pipelines, 256MB
    }

    /// Large pool for complex applications
    pub const fn large() -> Self {
        Self::new(1024, 1024 * 1024 * 1024) // 1024 pipelines, 1GB
    }
}

/// Compiled pipeline statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CompiledPipelineStats {
    /// Number of pipelines compiled
    pub pipelines_compiled: u32,
    /// Number of cache hits
    pub cache_hits: u32,
    /// Total compilation time in nanoseconds
    pub total_compile_time_ns: u64,
    /// Average compilation time in nanoseconds
    pub avg_compile_time_ns: u64,
    /// Number of pipelines failed
    pub pipelines_failed: u32,
}

impl CompiledPipelineStats {
    /// Empty stats
    pub const fn empty() -> Self {
        Self {
            pipelines_compiled: 0,
            cache_hits: 0,
            total_compile_time_ns: 0,
            avg_compile_time_ns: 0,
            pipelines_failed: 0,
        }
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        if self.pipelines_compiled == 0 {
            0.0
        } else {
            self.cache_hits as f32 / self.pipelines_compiled as f32
        }
    }

    /// Average compile time in milliseconds
    pub fn avg_compile_time_ms(&self) -> f64 {
        self.avg_compile_time_ns as f64 / 1_000_000.0
    }
}
