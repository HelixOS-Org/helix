//! Descriptor set types and layouts
//!
//! This module provides types for descriptor set configuration and binding.

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DescriptorType {
    /// Sampler
    Sampler = 0,
    /// Combined image sampler
    CombinedImageSampler = 1,
    /// Sampled image
    SampledImage = 2,
    /// Storage image
    StorageImage = 3,
    /// Uniform texel buffer
    UniformTexelBuffer = 4,
    /// Storage texel buffer
    StorageTexelBuffer = 5,
    /// Uniform buffer
    UniformBuffer = 6,
    /// Storage buffer
    StorageBuffer = 7,
    /// Uniform buffer dynamic
    UniformBufferDynamic = 8,
    /// Storage buffer dynamic
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment = 10,
    /// Inline uniform block
    InlineUniformBlock = 11,
    /// Acceleration structure
    AccelerationStructure = 12,
    /// Mutable descriptor
    MutableDescriptor = 13,
}

impl DescriptorType {
    /// Is buffer type
    pub const fn is_buffer(&self) -> bool {
        matches!(
            self,
            Self::UniformBuffer
                | Self::StorageBuffer
                | Self::UniformBufferDynamic
                | Self::StorageBufferDynamic
                | Self::UniformTexelBuffer
                | Self::StorageTexelBuffer
        )
    }

    /// Is image type
    pub const fn is_image(&self) -> bool {
        matches!(
            self,
            Self::SampledImage
                | Self::StorageImage
                | Self::CombinedImageSampler
                | Self::InputAttachment
        )
    }

    /// Is dynamic type
    pub const fn is_dynamic(&self) -> bool {
        matches!(
            self,
            Self::UniformBufferDynamic | Self::StorageBufferDynamic
        )
    }
}

/// Shader stage flags for descriptors
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorStageFlags(pub u32);

impl DescriptorStageFlags {
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
    /// Task shader
    pub const TASK: Self = Self(1 << 6);
    /// Mesh shader
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
            | Self::MESH.0,
    );

    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self(
        Self::RAYGEN.0
            | Self::ANY_HIT.0
            | Self::CLOSEST_HIT.0
            | Self::MISS.0
            | Self::INTERSECTION.0
            | Self::CALLABLE.0,
    );

    /// All stages
    pub const ALL: Self = Self(0x3FFF);

    /// Checks if contains
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl core::ops::BitOr for DescriptorStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Descriptor binding flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorBindingFlags(pub u32);

impl DescriptorBindingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Update unused while pending
    pub const UPDATE_UNUSED_WHILE_PENDING: Self = Self(1 << 1);
    /// Partially bound
    pub const PARTIALLY_BOUND: Self = Self(1 << 2);
    /// Variable descriptor count
    pub const VARIABLE_DESCRIPTOR_COUNT: Self = Self(1 << 3);

    /// Bindless (all flags)
    pub const BINDLESS: Self = Self(
        Self::UPDATE_AFTER_BIND.0
            | Self::PARTIALLY_BOUND.0
            | Self::VARIABLE_DESCRIPTOR_COUNT.0,
    );
}

impl core::ops::BitOr for DescriptorBindingFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Descriptor set layout binding
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorSetLayoutBinding {
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Stage flags
    pub stage_flags: DescriptorStageFlags,
    /// Binding flags
    pub binding_flags: DescriptorBindingFlags,
}

impl DescriptorSetLayoutBinding {
    /// Creates uniform buffer binding
    pub const fn uniform_buffer(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::UniformBuffer,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates storage buffer binding
    pub const fn storage_buffer(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::StorageBuffer,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates combined image sampler binding
    pub const fn combined_image_sampler(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::CombinedImageSampler,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates sampled image binding
    pub const fn sampled_image(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::SampledImage,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates storage image binding
    pub const fn storage_image(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::StorageImage,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates sampler binding
    pub const fn sampler(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::Sampler,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates acceleration structure binding
    pub const fn acceleration_structure(binding: u32, stages: DescriptorStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::AccelerationStructure,
            descriptor_count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates input attachment binding
    pub const fn input_attachment(binding: u32) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::InputAttachment,
            descriptor_count: 1,
            stage_flags: DescriptorStageFlags::FRAGMENT,
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// With descriptor count (array)
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// With binding flags
    pub const fn with_flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.binding_flags = flags;
        self
    }

    /// Make bindless
    pub const fn bindless(mut self, max_count: u32) -> Self {
        self.descriptor_count = max_count;
        self.binding_flags = DescriptorBindingFlags::BINDLESS;
        self
    }
}

/// Descriptor set layout create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorSetLayoutFlags(pub u32);

impl DescriptorSetLayoutFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Push descriptor
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 0);
    /// Update after bind pool
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 1);
    /// Host only pool
    pub const HOST_ONLY_POOL: Self = Self(1 << 2);
}

/// Descriptor pool size
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorPoolSize {
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count
    pub descriptor_count: u32,
}

impl DescriptorPoolSize {
    /// Creates new pool size
    pub const fn new(ty: DescriptorType, count: u32) -> Self {
        Self {
            descriptor_type: ty,
            descriptor_count: count,
        }
    }
}

/// Descriptor pool create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorPoolFlags(pub u32);

impl DescriptorPoolFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Free descriptor set
    pub const FREE_DESCRIPTOR_SET: Self = Self(1 << 0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 1);
    /// Host only
    pub const HOST_ONLY: Self = Self(1 << 2);
}

impl core::ops::BitOr for DescriptorPoolFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Descriptor write info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorBufferInfo {
    /// Buffer handle
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
    /// Range (0 for whole buffer)
    pub range: u64,
}

impl DescriptorBufferInfo {
    /// Creates new buffer info
    pub const fn new(buffer: u64, offset: u64, range: u64) -> Self {
        Self {
            buffer,
            offset,
            range,
        }
    }

    /// Whole buffer
    pub const fn whole(buffer: u64) -> Self {
        Self {
            buffer,
            offset: 0,
            range: u64::MAX,
        }
    }
}

/// Descriptor image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorImageInfo {
    /// Sampler handle
    pub sampler: u64,
    /// Image view handle
    pub image_view: u64,
    /// Image layout
    pub image_layout: u32,
}

impl DescriptorImageInfo {
    /// Creates for combined image sampler
    pub const fn combined(sampler: u64, image_view: u64, layout: u32) -> Self {
        Self {
            sampler,
            image_view,
            image_layout: layout,
        }
    }

    /// Creates for sampled image only
    pub const fn sampled(image_view: u64, layout: u32) -> Self {
        Self {
            sampler: 0,
            image_view,
            image_layout: layout,
        }
    }

    /// Creates for sampler only
    pub const fn sampler_only(sampler: u64) -> Self {
        Self {
            sampler,
            image_view: 0,
            image_layout: 0,
        }
    }
}

/// Descriptor update template entry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorUpdateTemplateEntry {
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Offset in data
    pub offset: usize,
    /// Stride between descriptors
    pub stride: usize,
}

/// Descriptor set layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutHandle(pub u64);

impl DescriptorSetLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for DescriptorSetLayoutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Descriptor set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetHandle(pub u64);

impl DescriptorSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for DescriptorSetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLayoutHandle(pub u64);

impl PipelineLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for PipelineLayoutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Common descriptor set layouts
pub mod layouts {
    use super::*;

    /// Creates camera uniform binding at 0
    pub const fn camera() -> DescriptorSetLayoutBinding {
        DescriptorSetLayoutBinding::uniform_buffer(0, DescriptorStageFlags::ALL_GRAPHICS)
    }

    /// Creates material binding at 0-3 (albedo, normal, metallic, occlusion)
    pub fn pbr_material() -> [DescriptorSetLayoutBinding; 4] {
        [
            DescriptorSetLayoutBinding::combined_image_sampler(0, DescriptorStageFlags::FRAGMENT),
            DescriptorSetLayoutBinding::combined_image_sampler(1, DescriptorStageFlags::FRAGMENT),
            DescriptorSetLayoutBinding::combined_image_sampler(2, DescriptorStageFlags::FRAGMENT),
            DescriptorSetLayoutBinding::combined_image_sampler(3, DescriptorStageFlags::FRAGMENT),
        ]
    }
}
