//! Compute pipeline types
//!
//! This module provides types for compute pipeline configuration.

extern crate alloc;
use alloc::vec::Vec;

use crate::descriptor::PipelineLayoutHandle;
use crate::shader::ShaderModuleDesc;

/// Handle to a compute pipeline
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputePipelineHandle(pub u64);

impl ComputePipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Compute pipeline description
#[derive(Clone, Debug)]
pub struct ComputePipelineDesc {
    /// Compute shader
    pub shader: ShaderModuleDesc,
    /// Entry point name
    pub entry_point: alloc::string::String,
    /// Pipeline layout
    pub layout: PipelineLayoutHandle,
    /// Specialization constants
    pub specialization: Vec<SpecializationConstant>,
}

impl ComputePipelineDesc {
    /// Creates a new compute pipeline description
    pub fn new(shader: ShaderModuleDesc, entry_point: &str) -> Self {
        Self {
            shader,
            entry_point: alloc::string::String::from(entry_point),
            layout: PipelineLayoutHandle::NULL,
            specialization: Vec::new(),
        }
    }

    /// Sets the pipeline layout
    pub fn with_layout(mut self, layout: PipelineLayoutHandle) -> Self {
        self.layout = layout;
        self
    }

    /// Adds a specialization constant
    pub fn add_specialization(mut self, constant: SpecializationConstant) -> Self {
        self.specialization.push(constant);
        self
    }
}

/// Specialization constant value
#[derive(Clone, Copy, Debug)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Constant value (as bytes)
    pub value: SpecConstValue,
}

/// Specialization constant value types
#[derive(Clone, Copy, Debug)]
pub enum SpecConstValue {
    /// Boolean value
    Bool(bool),
    /// 32-bit signed integer
    I32(i32),
    /// 32-bit unsigned integer
    U32(u32),
    /// 32-bit float
    F32(f32),
    /// 64-bit signed integer
    I64(i64),
    /// 64-bit unsigned integer
    U64(u64),
    /// 64-bit float
    F64(f64),
}

impl SpecializationConstant {
    /// Creates a boolean specialization constant
    pub const fn bool(id: u32, value: bool) -> Self {
        Self {
            id,
            value: SpecConstValue::Bool(value),
        }
    }

    /// Creates an i32 specialization constant
    pub const fn i32(id: u32, value: i32) -> Self {
        Self {
            id,
            value: SpecConstValue::I32(value),
        }
    }

    /// Creates a u32 specialization constant
    pub const fn u32(id: u32, value: u32) -> Self {
        Self {
            id,
            value: SpecConstValue::U32(value),
        }
    }

    /// Creates an f32 specialization constant
    pub const fn f32(id: u32, value: f32) -> Self {
        Self {
            id,
            value: SpecConstValue::F32(value),
        }
    }
}

/// Workgroup size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct WorkgroupSize {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl WorkgroupSize {
    /// Creates a 1D workgroup size
    pub const fn d1(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// Creates a 2D workgroup size
    pub const fn d2(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// Creates a 3D workgroup size
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Standard 64-thread 1D workgroup
    pub const STANDARD_1D: Self = Self::d1(64);

    /// Standard 8x8 2D workgroup
    pub const STANDARD_2D: Self = Self::d2(8, 8);

    /// Standard 4x4x4 3D workgroup
    pub const STANDARD_3D: Self = Self::d3(4, 4, 4);

    /// Total number of threads in workgroup
    pub const fn total_threads(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Calculates dispatch size for a given total work size
    pub const fn dispatch_for(&self, total_x: u32, total_y: u32, total_z: u32) -> (u32, u32, u32) {
        (
            (total_x + self.x - 1) / self.x,
            (total_y + self.y - 1) / self.y,
            (total_z + self.z - 1) / self.z,
        )
    }
}

impl Default for WorkgroupSize {
    fn default() -> Self {
        Self::STANDARD_1D
    }
}

/// Dispatch parameters for compute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DispatchSize {
    /// Number of workgroups in X
    pub x: u32,
    /// Number of workgroups in Y
    pub y: u32,
    /// Number of workgroups in Z
    pub z: u32,
}

impl DispatchSize {
    /// Creates a 1D dispatch
    pub const fn d1(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// Creates a 2D dispatch
    pub const fn d2(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// Creates a 3D dispatch
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Calculates dispatch size for given work size and workgroup size
    pub const fn for_size(work: (u32, u32, u32), workgroup: WorkgroupSize) -> Self {
        Self {
            x: (work.0 + workgroup.x - 1) / workgroup.x,
            y: (work.1 + workgroup.y - 1) / workgroup.y,
            z: (work.2 + workgroup.z - 1) / workgroup.z,
        }
    }

    /// Total number of workgroups
    pub const fn total_workgroups(&self) -> u64 {
        self.x as u64 * self.y as u64 * self.z as u64
    }
}

impl Default for DispatchSize {
    fn default() -> Self {
        Self::d1(1)
    }
}

/// Indirect dispatch parameters (GPU-driven)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// Number of workgroups in X
    pub x: u32,
    /// Number of workgroups in Y
    pub y: u32,
    /// Number of workgroups in Z
    pub z: u32,
}

impl DispatchIndirectCommand {
    /// Creates a new indirect dispatch command
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }
}

/// Compute shader local size reflection
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LocalSizeReflection {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
    /// Whether X is a specialization constant
    pub x_id: Option<u32>,
    /// Whether Y is a specialization constant
    pub y_id: Option<u32>,
    /// Whether Z is a specialization constant
    pub z_id: Option<u32>,
}

impl LocalSizeReflection {
    /// Creates reflection for fixed local size
    pub const fn fixed(x: u32, y: u32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            x_id: None,
            y_id: None,
            z_id: None,
        }
    }

    /// Creates reflection with specialization constant for X
    pub const fn spec_x(id: u32, y: u32, z: u32) -> Self {
        Self {
            x: 0,
            y,
            z,
            x_id: Some(id),
            y_id: None,
            z_id: None,
        }
    }
}

/// Compute resource requirements
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComputeResourceRequirements {
    /// Shared memory size in bytes
    pub shared_memory: u32,
    /// Number of registers per thread
    pub registers_per_thread: u32,
    /// Maximum threads per workgroup
    pub max_threads_per_workgroup: u32,
    /// Preferred workgroup size multiple
    pub preferred_workgroup_multiple: u32,
}

impl ComputeResourceRequirements {
    /// Calculates theoretical occupancy
    pub fn theoretical_occupancy(
        &self,
        workgroup_size: u32,
        available_shared: u32,
        available_registers: u32,
    ) -> f32 {
        let shared_limit = if self.shared_memory > 0 {
            available_shared / self.shared_memory
        } else {
            u32::MAX
        };

        let register_limit = if self.registers_per_thread > 0 {
            available_registers / (self.registers_per_thread * workgroup_size)
        } else {
            u32::MAX
        };

        let max_workgroups = shared_limit.min(register_limit) as f32;
        let max_possible = (self.max_threads_per_workgroup / workgroup_size) as f32;

        (max_workgroups / max_possible).min(1.0)
    }
}

/// Subgroup features
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubgroupFeatures(pub u32);

impl SubgroupFeatures {
    /// Basic subgroup operations
    pub const BASIC: Self = Self(1 << 0);
    /// Vote operations
    pub const VOTE: Self = Self(1 << 1);
    /// Arithmetic operations
    pub const ARITHMETIC: Self = Self(1 << 2);
    /// Ballot operations
    pub const BALLOT: Self = Self(1 << 3);
    /// Shuffle operations
    pub const SHUFFLE: Self = Self(1 << 4);
    /// Shuffle relative operations
    pub const SHUFFLE_RELATIVE: Self = Self(1 << 5);
    /// Clustered operations
    pub const CLUSTERED: Self = Self(1 << 6);
    /// Quad operations
    pub const QUAD: Self = Self(1 << 7);

    /// Checks if a feature is supported
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for SubgroupFeatures {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Subgroup properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SubgroupProperties {
    /// Subgroup size
    pub subgroup_size: u32,
    /// Supported subgroup stages
    pub supported_stages: u32,
    /// Supported subgroup features
    pub supported_features: SubgroupFeatures,
    /// Whether quad operations are available in all stages
    pub quad_operations_in_all_stages: bool,
}

/// Pipeline cache
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheHandle(pub u64);

impl PipelineCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Pipeline cache creation info
#[derive(Clone, Debug, Default)]
pub struct PipelineCacheDesc {
    /// Initial data (from previous session)
    pub initial_data: Vec<u8>,
    /// Flags
    pub flags: PipelineCacheFlags,
}

/// Pipeline cache flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheFlags(pub u32);

impl PipelineCacheFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Externally synchronized
    pub const EXTERNALLY_SYNCHRONIZED: Self = Self(1 << 0);
}

impl PipelineCacheDesc {
    /// Creates an empty pipeline cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a pipeline cache with initial data
    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            initial_data: data,
            flags: PipelineCacheFlags::NONE,
        }
    }
}

/// Pipeline creation flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCreateFlags(pub u32);

impl PipelineCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable optimization
    pub const DISABLE_OPTIMIZATION: Self = Self(1 << 0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 1);
    /// Derivative pipeline
    pub const DERIVATIVE: Self = Self(1 << 2);
    /// Fail on pipeline compile required
    pub const FAIL_ON_PIPELINE_COMPILE_REQUIRED: Self = Self(1 << 8);
    /// Early return on failure
    pub const EARLY_RETURN_ON_FAILURE: Self = Self(1 << 9);
}

impl core::ops::BitOr for PipelineCreateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Pipeline statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineStatistics {
    /// Instruction count
    pub instruction_count: u64,
    /// Temporary register count
    pub temp_register_count: u32,
    /// Spilled register count
    pub spilled_register_count: u32,
    /// Shared memory used
    pub shared_memory_used: u32,
    /// Scratch memory used
    pub scratch_memory_used: u32,
}
