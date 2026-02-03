//! Pipeline library and caching types
//!
//! This module provides types for pipeline caching and library management.

extern crate alloc;
use alloc::vec::Vec;

/// Pipeline cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheHandle(pub u64);

impl PipelineCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Pipeline library handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLibraryHandle(pub u64);

impl PipelineLibraryHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Pipeline cache create info
#[derive(Clone, Debug, Default)]
pub struct PipelineCacheCreateInfo {
    /// Initial data
    pub initial_data: Vec<u8>,
    /// Flags
    pub flags: PipelineCacheCreateFlags,
}

impl PipelineCacheCreateInfo {
    /// Creates empty cache
    pub const fn new() -> Self {
        Self {
            initial_data: Vec::new(),
            flags: PipelineCacheCreateFlags::NONE,
        }
    }

    /// Creates from existing data
    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            initial_data: data,
            flags: PipelineCacheCreateFlags::NONE,
        }
    }

    /// Externally synchronized
    pub fn externally_synchronized(mut self) -> Self {
        self.flags = self.flags.union(PipelineCacheCreateFlags::EXTERNALLY_SYNCHRONIZED);
        self
    }
}

/// Pipeline cache create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineCacheCreateFlags(pub u32);

impl PipelineCacheCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Externally synchronized
    pub const EXTERNALLY_SYNCHRONIZED: Self = Self(1 << 0);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Pipeline cache header
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PipelineCacheHeader {
    /// Header size
    pub header_size: u32,
    /// Header version
    pub header_version: PipelineCacheHeaderVersion,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
}

impl PipelineCacheHeader {
    /// Validates the header
    pub const fn is_valid(&self) -> bool {
        self.header_size >= core::mem::size_of::<Self>() as u32
    }
}

/// Pipeline cache header version
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PipelineCacheHeaderVersion {
    /// Version 1
    One = 1,
}

/// Pipeline library create info
#[derive(Clone, Debug, Default)]
pub struct PipelineLibraryCreateInfo {
    /// Library type
    pub library_type: PipelineLibraryType,
    /// Flags
    pub flags: PipelineLibraryFlags,
}

/// Pipeline library type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PipelineLibraryType {
    /// Graphics pipeline library
    #[default]
    Graphics,
    /// Compute pipeline library
    Compute,
    /// Ray tracing pipeline library
    RayTracing,
}

/// Pipeline library flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineLibraryFlags(pub u32);

impl PipelineLibraryFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Link time optimization
    pub const LINK_TIME_OPTIMIZATION: Self = Self(1 << 0);
    /// Retain link time optimization info
    pub const RETAIN_LINK_TIME_OPTIMIZATION_INFO: Self = Self(1 << 1);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Graphics pipeline library flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct GraphicsPipelineLibraryFlags(pub u32);

impl GraphicsPipelineLibraryFlags {
    /// Vertex input interface
    pub const VERTEX_INPUT_INTERFACE: Self = Self(1 << 0);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 1);
    /// Fragment shader
    pub const FRAGMENT_SHADER: Self = Self(1 << 2);
    /// Fragment output interface
    pub const FRAGMENT_OUTPUT_INTERFACE: Self = Self(1 << 3);

    /// All parts
    pub const ALL: Self = Self(0xF);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Pipeline creation feedback
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineCreationFeedback {
    /// Flags
    pub flags: PipelineCreationFeedbackFlags,
    /// Duration in nanoseconds
    pub duration_ns: u64,
}

impl PipelineCreationFeedback {
    /// Checks if pipeline was cached
    pub const fn was_cached(&self) -> bool {
        self.flags.contains(PipelineCreationFeedbackFlags::APPLICATION_PIPELINE_CACHE_HIT)
    }

    /// Checks if base pipeline was used
    pub const fn used_base_pipeline(&self) -> bool {
        self.flags.contains(PipelineCreationFeedbackFlags::BASE_PIPELINE_ACCELERATION)
    }

    /// Duration in milliseconds
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns as f64 / 1_000_000.0
    }
}

/// Pipeline creation feedback flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineCreationFeedbackFlags(pub u32);

impl PipelineCreationFeedbackFlags {
    /// Valid feedback
    pub const VALID: Self = Self(1 << 0);
    /// Application pipeline cache hit
    pub const APPLICATION_PIPELINE_CACHE_HIT: Self = Self(1 << 1);
    /// Base pipeline acceleration
    pub const BASE_PIPELINE_ACCELERATION: Self = Self(1 << 2);

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Pipeline creation feedback create info
#[derive(Clone, Debug)]
pub struct PipelineCreationFeedbackCreateInfo {
    /// Overall pipeline feedback
    pub pipeline_creation_feedback: PipelineCreationFeedback,
    /// Per-stage feedback
    pub stage_creation_feedbacks: Vec<PipelineCreationFeedback>,
}

impl PipelineCreationFeedbackCreateInfo {
    /// Creates new feedback info
    pub fn new(stage_count: usize) -> Self {
        Self {
            pipeline_creation_feedback: PipelineCreationFeedback::default(),
            stage_creation_feedbacks: alloc::vec![PipelineCreationFeedback::default(); stage_count],
        }
    }
}

/// Pipeline compile hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PipelineCompileHint {
    /// Default compilation
    Default,
    /// Optimize for fast linking
    FastLink,
    /// Full optimization
    FullOptimization,
    /// Skip optimization
    SkipOptimization,
}

impl Default for PipelineCompileHint {
    fn default() -> Self {
        Self::Default
    }
}

/// Pipeline robustness
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineRobustness {
    /// Storage buffer robustness
    pub storage_buffers: RobustnessLevel,
    /// Uniform buffer robustness
    pub uniform_buffers: RobustnessLevel,
    /// Vertex input robustness
    pub vertex_inputs: RobustnessLevel,
    /// Image access robustness
    pub images: RobustnessLevel,
}

impl PipelineRobustness {
    /// No robustness
    pub const fn none() -> Self {
        Self {
            storage_buffers: RobustnessLevel::NotRobust,
            uniform_buffers: RobustnessLevel::NotRobust,
            vertex_inputs: RobustnessLevel::NotRobust,
            images: RobustnessLevel::NotRobust,
        }
    }

    /// Full robustness
    pub const fn full() -> Self {
        Self {
            storage_buffers: RobustnessLevel::RobustBufferAccess2,
            uniform_buffers: RobustnessLevel::RobustBufferAccess2,
            vertex_inputs: RobustnessLevel::RobustBufferAccess2,
            images: RobustnessLevel::RobustImageAccess2,
        }
    }

    /// Buffer access only
    pub const fn buffer_access() -> Self {
        Self {
            storage_buffers: RobustnessLevel::RobustBufferAccess,
            uniform_buffers: RobustnessLevel::RobustBufferAccess,
            vertex_inputs: RobustnessLevel::RobustBufferAccess,
            images: RobustnessLevel::NotRobust,
        }
    }
}

/// Robustness level
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RobustnessLevel {
    /// Not robust
    #[default]
    NotRobust,
    /// Robust buffer access
    RobustBufferAccess,
    /// Robust buffer access 2
    RobustBufferAccess2,
    /// Robust image access
    RobustImageAccess,
    /// Robust image access 2
    RobustImageAccess2,
}

/// Shader binary info
#[derive(Clone, Debug)]
pub struct ShaderBinaryInfo {
    /// Binary data
    pub data: Vec<u8>,
    /// Shader stage
    pub stage: ShaderStage,
    /// Entry point name
    pub entry_point: Vec<u8>,
}

impl ShaderBinaryInfo {
    /// Creates new shader binary info
    pub fn new(data: Vec<u8>, stage: ShaderStage, entry_point: &[u8]) -> Self {
        Self {
            data,
            stage,
            entry_point: entry_point.to_vec(),
        }
    }
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex,
    /// Tessellation control
    TessellationControl,
    /// Tessellation evaluation
    TessellationEvaluation,
    /// Geometry shader
    Geometry,
    /// Fragment shader
    Fragment,
    /// Compute shader
    Compute,
    /// Task shader
    Task,
    /// Mesh shader
    Mesh,
    /// Ray generation
    RayGeneration,
    /// Any hit
    AnyHit,
    /// Closest hit
    ClosestHit,
    /// Miss
    Miss,
    /// Intersection
    Intersection,
    /// Callable
    Callable,
}

impl ShaderStage {
    /// All graphics stages
    pub const GRAPHICS: &'static [Self] = &[
        Self::Vertex,
        Self::TessellationControl,
        Self::TessellationEvaluation,
        Self::Geometry,
        Self::Fragment,
    ];

    /// All ray tracing stages
    pub const RAY_TRACING: &'static [Self] = &[
        Self::RayGeneration,
        Self::AnyHit,
        Self::ClosestHit,
        Self::Miss,
        Self::Intersection,
        Self::Callable,
    ];

    /// Mesh shading stages
    pub const MESH_SHADING: &'static [Self] = &[Self::Task, Self::Mesh];

    /// Is graphics stage
    pub const fn is_graphics(&self) -> bool {
        matches!(
            self,
            Self::Vertex
                | Self::TessellationControl
                | Self::TessellationEvaluation
                | Self::Geometry
                | Self::Fragment
        )
    }

    /// Is ray tracing stage
    pub const fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            Self::RayGeneration
                | Self::AnyHit
                | Self::ClosestHit
                | Self::Miss
                | Self::Intersection
                | Self::Callable
        )
    }
}

/// Pipeline executable info
#[derive(Clone, Debug)]
pub struct PipelineExecutableInfo {
    /// Name
    pub name: Vec<u8>,
    /// Description
    pub description: Vec<u8>,
    /// Stages
    pub stages: ShaderStageFlags,
    /// Subgroup size
    pub subgroup_size: u32,
}

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// Vertex
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 2);
    /// Geometry
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Fragment
    pub const FRAGMENT: Self = Self(1 << 4);
    /// Compute
    pub const COMPUTE: Self = Self(1 << 5);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All
    pub const ALL: Self = Self(0x7FFFFFFF);

    /// Task
    pub const TASK: Self = Self(1 << 6);
    /// Mesh
    pub const MESH: Self = Self(1 << 7);

    /// Ray generation
    pub const RAYGEN: Self = Self(1 << 8);
    /// Any hit
    pub const ANY_HIT: Self = Self(1 << 9);
    /// Closest hit
    pub const CLOSEST_HIT: Self = Self(1 << 10);
    /// Miss
    pub const MISS: Self = Self(1 << 11);
    /// Intersection
    pub const INTERSECTION: Self = Self(1 << 12);
    /// Callable
    pub const CALLABLE: Self = Self(1 << 13);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl core::ops::BitOr for ShaderStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Pipeline executable statistic
#[derive(Clone, Debug)]
pub struct PipelineExecutableStatistic {
    /// Name
    pub name: Vec<u8>,
    /// Description
    pub description: Vec<u8>,
    /// Format
    pub format: PipelineExecutableStatisticFormat,
    /// Value
    pub value: PipelineExecutableStatisticValue,
}

/// Pipeline executable statistic format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PipelineExecutableStatisticFormat {
    /// Boolean
    Bool32,
    /// 64-bit integer
    Int64,
    /// 64-bit unsigned integer
    Uint64,
    /// 64-bit float
    Float64,
}

/// Pipeline executable statistic value
#[derive(Clone, Copy, Debug)]
pub enum PipelineExecutableStatisticValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int64(i64),
    /// Unsigned integer value
    Uint64(u64),
    /// Float value
    Float64(f64),
}

/// Pipeline executable internal representation
#[derive(Clone, Debug)]
pub struct PipelineExecutableInternalRepresentation {
    /// Name
    pub name: Vec<u8>,
    /// Description
    pub description: Vec<u8>,
    /// Is text representation
    pub is_text: bool,
    /// Data
    pub data: Vec<u8>,
}

/// Pipeline compile required flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineCompileRequiredFlags(pub u32);

impl PipelineCompileRequiredFlags {
    /// No compilation needed
    pub const NONE: Self = Self(0);
    /// Full compilation required
    pub const FULL: Self = Self(1 << 0);
    /// Link required
    pub const LINK: Self = Self(1 << 1);
}

/// Deferred operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DeferredOperation {
    /// Handle
    pub handle: DeferredOperationHandle,
    /// Maximum concurrency
    pub max_concurrency: u32,
    /// Result when complete
    pub result: DeferredOperationResult,
}

/// Deferred operation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeferredOperationHandle(pub u64);

impl DeferredOperationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Deferred operation result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum DeferredOperationResult {
    /// Not started
    NotStarted = 0,
    /// In progress
    InProgress = 1,
    /// Complete success
    Success = 2,
    /// Failed
    Failed = -1,
    /// Out of memory
    OutOfMemory = -2,
}
