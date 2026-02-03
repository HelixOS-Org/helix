//! Push constants types and utilities
//!
//! This module provides types for push constants in Vulkan-style pipelines.

use core::mem::size_of;

/// Maximum push constant size (Vulkan minimum guarantee)
pub const MAX_PUSH_CONSTANT_SIZE: u32 = 128;

/// Push constant range
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PushConstantRange {
    /// Shader stages that access this range
    pub stage_flags: ShaderStageFlags,
    /// Offset in bytes from start of push constant block
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates a new push constant range
    pub const fn new(stage_flags: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self { stage_flags, offset, size }
    }

    /// Creates range for all graphics stages
    pub const fn graphics(offset: u32, size: u32) -> Self {
        Self {
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            offset,
            size,
        }
    }

    /// Creates range for compute stage
    pub const fn compute(offset: u32, size: u32) -> Self {
        Self {
            stage_flags: ShaderStageFlags::COMPUTE,
            offset,
            size,
        }
    }

    /// Creates range for vertex stage
    pub const fn vertex(offset: u32, size: u32) -> Self {
        Self {
            stage_flags: ShaderStageFlags::VERTEX,
            offset,
            size,
        }
    }

    /// Creates range for fragment stage
    pub const fn fragment(offset: u32, size: u32) -> Self {
        Self {
            stage_flags: ShaderStageFlags::FRAGMENT,
            offset,
            size,
        }
    }

    /// Creates range for all stages
    pub const fn all(size: u32) -> Self {
        Self {
            stage_flags: ShaderStageFlags::ALL,
            offset: 0,
            size,
        }
    }

    /// End offset of this range
    pub const fn end(&self) -> u32 {
        self.offset + self.size
    }

    /// Checks if ranges overlap
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.offset < other.end() && other.offset < self.end()
    }
}

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// No stages
    pub const NONE: Self = Self(0);
    /// Vertex shader
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control shader
    pub const TESSELLATION_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation shader
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 2);
    /// Geometry shader
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Fragment shader
    pub const FRAGMENT: Self = Self(1 << 4);
    /// Compute shader
    pub const COMPUTE: Self = Self(1 << 5);
    /// Task shader (mesh shading)
    pub const TASK: Self = Self(1 << 6);
    /// Mesh shader (mesh shading)
    pub const MESH: Self = Self(1 << 7);
    /// Ray generation shader
    pub const RAYGEN: Self = Self(1 << 8);
    /// Any hit shader
    pub const ANY_HIT: Self = Self(1 << 9);
    /// Closest hit shader
    pub const CLOSEST_HIT: Self = Self(1 << 10);
    /// Miss shader
    pub const MISS: Self = Self(1 << 11);
    /// Intersection shader
    pub const INTERSECTION: Self = Self(1 << 12);
    /// Callable shader
    pub const CALLABLE: Self = Self(1 << 13);

    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(
        Self::VERTEX.0
            | Self::TESSELLATION_CONTROL.0
            | Self::TESSELLATION_EVALUATION.0
            | Self::GEOMETRY.0
            | Self::FRAGMENT.0
            | Self::TASK.0
            | Self::MESH.0
    );

    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self(
        Self::RAYGEN.0
            | Self::ANY_HIT.0
            | Self::CLOSEST_HIT.0
            | Self::MISS.0
            | Self::INTERSECTION.0
            | Self::CALLABLE.0
    );

    /// All stages
    pub const ALL: Self = Self(
        Self::ALL_GRAPHICS.0 | Self::COMPUTE.0 | Self::ALL_RAY_TRACING.0
    );

    /// Checks if contains all specified flags
    pub const fn contains(&self, flags: Self) -> bool {
        (self.0 & flags.0) == flags.0
    }

    /// Union of flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection of flags
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl core::ops::BitOr for ShaderStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for ShaderStageFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Push constant layout builder
#[derive(Clone, Debug, Default)]
pub struct PushConstantLayout {
    /// Push constant ranges
    ranges: [Option<PushConstantRange>; 8],
    /// Number of ranges
    count: usize,
    /// Total size
    total_size: u32,
}

impl PushConstantLayout {
    /// Creates an empty layout
    pub const fn new() -> Self {
        Self {
            ranges: [None; 8],
            count: 0,
            total_size: 0,
        }
    }

    /// Adds a push constant range
    pub fn add_range(mut self, range: PushConstantRange) -> Self {
        if self.count < 8 {
            let end = range.end();
            if end > self.total_size {
                self.total_size = end;
            }
            self.ranges[self.count] = Some(range);
            self.count += 1;
        }
        self
    }

    /// Adds type for shader stages
    pub fn add_type<T>(self, stage_flags: ShaderStageFlags) -> Self {
        let size = size_of::<T>() as u32;
        self.add_range(PushConstantRange::new(stage_flags, self.total_size, size))
    }

    /// Adds uniform at offset for stages
    pub fn add_at_offset(self, offset: u32, size: u32, stage_flags: ShaderStageFlags) -> Self {
        self.add_range(PushConstantRange::new(stage_flags, offset, size))
    }

    /// Gets number of ranges
    pub const fn range_count(&self) -> usize {
        self.count
    }

    /// Gets total size
    pub const fn total_size(&self) -> u32 {
        self.total_size
    }

    /// Gets ranges as slice
    pub fn ranges(&self) -> &[Option<PushConstantRange>] {
        &self.ranges[..self.count]
    }

    /// Validates the layout
    pub fn validate(&self) -> Result<(), PushConstantError> {
        if self.total_size > MAX_PUSH_CONSTANT_SIZE {
            return Err(PushConstantError::SizeExceeded {
                size: self.total_size,
                max: MAX_PUSH_CONSTANT_SIZE,
            });
        }

        // Check for overlaps with different stages
        for i in 0..self.count {
            for j in (i + 1)..self.count {
                if let (Some(a), Some(b)) = (&self.ranges[i], &self.ranges[j]) {
                    if a.overlaps(b) && a.stage_flags != b.stage_flags {
                        return Err(PushConstantError::OverlappingRanges {
                            range1_offset: a.offset,
                            range2_offset: b.offset,
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Push constant error
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushConstantError {
    /// Size exceeds maximum
    SizeExceeded {
        /// Actual size
        size: u32,
        /// Maximum size
        max: u32,
    },
    /// Overlapping ranges with different stages
    OverlappingRanges {
        /// First range offset
        range1_offset: u32,
        /// Second range offset
        range2_offset: u32,
    },
    /// Alignment error
    AlignmentError {
        /// Offset with alignment error
        offset: u32,
        /// Required alignment
        alignment: u32,
    },
}

/// Standard push constant block for MVP matrices
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MvpPushConstants {
    /// Model matrix
    pub model: [[f32; 4]; 4],
}

impl Default for MvpPushConstants {
    fn default() -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

impl MvpPushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 64;

    /// Creates from model matrix
    pub const fn from_model(model: [[f32; 4]; 4]) -> Self {
        Self { model }
    }
}

/// Standard push constant block for object data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ObjectPushConstants {
    /// Object transform (position, rotation, scale packed)
    pub transform: [f32; 12],
    /// Object ID
    pub object_id: u32,
    /// Material ID
    pub material_id: u32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _pad: u32,
}

impl ObjectPushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 64;

    /// Creates with object and material IDs
    pub const fn new(object_id: u32, material_id: u32) -> Self {
        Self {
            transform: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
            ],
            object_id,
            material_id,
            flags: 0,
            _pad: 0,
        }
    }
}

/// Standard push constant block for compute shaders
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComputePushConstants {
    /// Dispatch size X
    pub dispatch_x: u32,
    /// Dispatch size Y
    pub dispatch_y: u32,
    /// Dispatch size Z
    pub dispatch_z: u32,
    /// Frame index
    pub frame: u32,
    /// Time in seconds
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// User data
    pub user_data: [u32; 2],
}

impl ComputePushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 32;

    /// Creates with dispatch dimensions
    pub const fn new(dispatch_x: u32, dispatch_y: u32, dispatch_z: u32) -> Self {
        Self {
            dispatch_x,
            dispatch_y,
            dispatch_z,
            frame: 0,
            time: 0.0,
            delta_time: 0.0,
            user_data: [0; 2],
        }
    }
}

/// Standard push constant block for materials
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MaterialPushConstants {
    /// Base color
    pub base_color: [f32; 4],
    /// Emissive color and intensity
    pub emissive: [f32; 4],
    /// Metallic factor
    pub metallic: f32,
    /// Roughness factor
    pub roughness: f32,
    /// Normal scale
    pub normal_scale: f32,
    /// Occlusion strength
    pub occlusion_strength: f32,
    /// Alpha cutoff
    pub alpha_cutoff: f32,
    /// Texture indices packed
    pub texture_indices: [u32; 3],
}

impl Default for MaterialPushConstants {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0, 0.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            alpha_cutoff: 0.5,
            texture_indices: [0; 3],
        }
    }
}

impl MaterialPushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 64;
}

/// Push constant update command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantUpdate {
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantUpdate {
    /// Creates a new update command
    pub const fn new(stage_flags: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self { stage_flags, offset, size }
    }

    /// Creates update for type
    pub fn for_type<T>(stage_flags: ShaderStageFlags, offset: u32) -> Self {
        Self {
            stage_flags,
            offset,
            size: size_of::<T>() as u32,
        }
    }
}

/// Standard layouts
pub mod layouts {
    use super::*;

    /// Simple model-only layout (64 bytes)
    pub const fn model_only() -> PushConstantLayout {
        PushConstantLayout::new()
    }

    /// Graphics pipeline with per-object data
    pub fn graphics_object() -> PushConstantLayout {
        PushConstantLayout::new()
            .add_range(PushConstantRange::graphics(0, ObjectPushConstants::SIZE))
    }

    /// Graphics pipeline with MVP
    pub fn graphics_mvp() -> PushConstantLayout {
        PushConstantLayout::new()
            .add_range(PushConstantRange::vertex(0, MvpPushConstants::SIZE))
    }

    /// Graphics pipeline with MVP and material
    pub fn graphics_mvp_material() -> PushConstantLayout {
        PushConstantLayout::new()
            .add_range(PushConstantRange::vertex(0, MvpPushConstants::SIZE))
            .add_range(PushConstantRange::fragment(64, MaterialPushConstants::SIZE))
    }

    /// Compute pipeline standard layout
    pub fn compute_standard() -> PushConstantLayout {
        PushConstantLayout::new()
            .add_range(PushConstantRange::compute(0, ComputePushConstants::SIZE))
    }
}
