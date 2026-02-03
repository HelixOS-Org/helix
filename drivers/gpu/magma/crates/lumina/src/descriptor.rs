//! Descriptor set and binding types
//!
//! This module provides types for descriptor set layouts and management.

extern crate alloc;
use alloc::vec::Vec;

use crate::types::{BufferHandle, SamplerHandle, TextureViewHandle};

/// Handle to a descriptor set layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutHandle(pub u64);

impl DescriptorSetLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Handle to a descriptor set
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetHandle(pub u64);

impl DescriptorSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Handle to a descriptor pool
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorPoolHandle(pub u64);

impl DescriptorPoolHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Handle to a pipeline layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLayoutHandle(pub u64);

impl PipelineLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DescriptorType {
    /// Sampler only
    Sampler,
    /// Combined image sampler
    CombinedImageSampler,
    /// Sampled image (texture)
    SampledImage,
    /// Storage image
    StorageImage,
    /// Uniform texel buffer
    UniformTexelBuffer,
    /// Storage texel buffer
    StorageTexelBuffer,
    /// Uniform buffer
    UniformBuffer,
    /// Storage buffer
    StorageBuffer,
    /// Dynamic uniform buffer
    UniformBufferDynamic,
    /// Dynamic storage buffer
    StorageBufferDynamic,
    /// Input attachment
    InputAttachment,
    /// Acceleration structure
    AccelerationStructure,
}

impl DescriptorType {
    /// Checks if this is a buffer type
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

    /// Checks if this is an image type
    pub const fn is_image(&self) -> bool {
        matches!(
            self,
            Self::SampledImage
                | Self::StorageImage
                | Self::CombinedImageSampler
                | Self::InputAttachment
        )
    }

    /// Checks if this is a dynamic type
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::UniformBufferDynamic | Self::StorageBufferDynamic)
    }
}

/// Shader stage flags for descriptor binding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// Vertex shader stage
    pub const VERTEX: Self = Self(1 << 0);
    /// Fragment shader stage
    pub const FRAGMENT: Self = Self(1 << 1);
    /// Compute shader stage
    pub const COMPUTE: Self = Self(1 << 2);
    /// Geometry shader stage
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Tessellation control stage
    pub const TESSELLATION_CONTROL: Self = Self(1 << 4);
    /// Tessellation evaluation stage
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 5);
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All stages
    pub const ALL: Self = Self(0x7FFFFFFF);
    /// Ray generation stage
    pub const RAYGEN: Self = Self(1 << 8);
    /// Any hit stage
    pub const ANY_HIT: Self = Self(1 << 9);
    /// Closest hit stage
    pub const CLOSEST_HIT: Self = Self(1 << 10);
    /// Miss stage
    pub const MISS: Self = Self(1 << 11);
    /// Intersection stage
    pub const INTERSECTION: Self = Self(1 << 12);
    /// Callable stage
    pub const CALLABLE: Self = Self(1 << 13);
    /// Task shader stage
    pub const TASK: Self = Self(1 << 6);
    /// Mesh shader stage
    pub const MESH: Self = Self(1 << 7);

    /// Checks if a flag is set
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
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

/// Descriptor binding flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorBindingFlags(pub u32);

impl DescriptorBindingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Binding can be updated after bound
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Binding can be partially bound
    pub const PARTIALLY_BOUND: Self = Self(1 << 1);
    /// Variable descriptor count
    pub const VARIABLE_DESCRIPTOR_COUNT: Self = Self(1 << 2);
    /// Update unused while pending
    pub const UPDATE_UNUSED_WHILE_PENDING: Self = Self(1 << 3);
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
    /// Number of descriptors
    pub descriptor_count: u32,
    /// Shader stages that access this binding
    pub stage_flags: ShaderStageFlags,
    /// Binding flags
    pub flags: DescriptorBindingFlags,
}

impl DescriptorSetLayoutBinding {
    /// Creates a new binding
    pub const fn new(
        binding: u32,
        descriptor_type: DescriptorType,
        stage_flags: ShaderStageFlags,
    ) -> Self {
        Self {
            binding,
            descriptor_type,
            descriptor_count: 1,
            stage_flags,
            flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Creates a uniform buffer binding
    pub const fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, stages)
    }

    /// Creates a storage buffer binding
    pub const fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, stages)
    }

    /// Creates a sampled image binding
    pub const fn sampled_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::SampledImage, stages)
    }

    /// Creates a combined image sampler binding
    pub const fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, stages)
    }

    /// Creates a storage image binding
    pub const fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageImage, stages)
    }

    /// Creates a sampler binding
    pub const fn sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::Sampler, stages)
    }

    /// Sets descriptor count (for arrays)
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// Sets binding flags
    pub const fn with_flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Descriptor set layout creation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutFlags(pub u32);

impl DescriptorSetLayoutFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Push descriptor set
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 0);
    /// Update after bind pool
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 1);
}

impl core::ops::BitOr for DescriptorSetLayoutFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Descriptor set layout description
#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutDesc {
    /// Bindings in this layout
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    /// Layout flags
    pub flags: DescriptorSetLayoutFlags,
}

impl DescriptorSetLayoutDesc {
    /// Creates a new empty layout description
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            flags: DescriptorSetLayoutFlags::NONE,
        }
    }

    /// Adds a binding
    pub fn add_binding(mut self, binding: DescriptorSetLayoutBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Sets layout flags
    pub fn with_flags(mut self, flags: DescriptorSetLayoutFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Total number of descriptors
    pub fn total_descriptors(&self) -> u32 {
        self.bindings.iter().map(|b| b.descriptor_count).sum()
    }
}

impl Default for DescriptorSetLayoutDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Descriptor pool size
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorPoolSize {
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Number of descriptors of this type
    pub descriptor_count: u32,
}

impl DescriptorPoolSize {
    /// Creates a new pool size
    pub const fn new(descriptor_type: DescriptorType, count: u32) -> Self {
        Self {
            descriptor_type,
            descriptor_count: count,
        }
    }
}

/// Descriptor pool creation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorPoolFlags(pub u32);

impl DescriptorPoolFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Allow freeing individual sets
    pub const FREE_DESCRIPTOR_SET: Self = Self(1 << 0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 1);
}

impl core::ops::BitOr for DescriptorPoolFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Descriptor pool description
#[derive(Clone, Debug)]
pub struct DescriptorPoolDesc {
    /// Maximum number of sets that can be allocated
    pub max_sets: u32,
    /// Pool sizes for each descriptor type
    pub pool_sizes: Vec<DescriptorPoolSize>,
    /// Pool flags
    pub flags: DescriptorPoolFlags,
}

impl DescriptorPoolDesc {
    /// Creates a new pool description
    pub fn new(max_sets: u32) -> Self {
        Self {
            max_sets,
            pool_sizes: Vec::new(),
            flags: DescriptorPoolFlags::NONE,
        }
    }

    /// Adds a pool size
    pub fn add_pool_size(mut self, descriptor_type: DescriptorType, count: u32) -> Self {
        self.pool_sizes
            .push(DescriptorPoolSize::new(descriptor_type, count));
        self
    }

    /// Sets pool flags
    pub fn with_flags(mut self, flags: DescriptorPoolFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Creates a pool for uniform buffers
    pub fn uniform_buffers(max_sets: u32, buffer_count: u32) -> Self {
        Self::new(max_sets).add_pool_size(DescriptorType::UniformBuffer, buffer_count)
    }

    /// Creates a pool for combined image samplers
    pub fn combined_image_samplers(max_sets: u32, sampler_count: u32) -> Self {
        Self::new(max_sets).add_pool_size(DescriptorType::CombinedImageSampler, sampler_count)
    }
}

/// Buffer info for descriptor write
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorBufferInfo {
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Offset into buffer
    pub offset: u64,
    /// Range of buffer to bind (0 for whole buffer)
    pub range: u64,
}

impl DescriptorBufferInfo {
    /// Creates a new buffer info
    pub const fn new(buffer: BufferHandle) -> Self {
        Self {
            buffer,
            offset: 0,
            range: 0, // Whole buffer
        }
    }

    /// Creates buffer info with offset and range
    pub const fn with_range(buffer: BufferHandle, offset: u64, range: u64) -> Self {
        Self {
            buffer,
            offset,
            range,
        }
    }
}

/// Image layout for descriptor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DescriptorImageLayout {
    /// Undefined layout
    Undefined,
    /// General layout
    General,
    /// Optimal for shader read
    ShaderReadOnlyOptimal,
    /// Depth stencil read only
    DepthStencilReadOnlyOptimal,
}

/// Image info for descriptor write
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorImageInfo {
    /// Sampler handle (optional)
    pub sampler: SamplerHandle,
    /// Image view handle
    pub image_view: TextureViewHandle,
    /// Image layout
    pub image_layout: DescriptorImageLayout,
}

impl DescriptorImageInfo {
    /// Creates image info for sampled image
    pub const fn sampled_image(image_view: TextureViewHandle) -> Self {
        Self {
            sampler: SamplerHandle(0),
            image_view,
            image_layout: DescriptorImageLayout::ShaderReadOnlyOptimal,
        }
    }

    /// Creates image info for combined image sampler
    pub const fn combined_image_sampler(
        sampler: SamplerHandle,
        image_view: TextureViewHandle,
    ) -> Self {
        Self {
            sampler,
            image_view,
            image_layout: DescriptorImageLayout::ShaderReadOnlyOptimal,
        }
    }

    /// Creates image info for storage image
    pub const fn storage_image(image_view: TextureViewHandle) -> Self {
        Self {
            sampler: SamplerHandle(0),
            image_view,
            image_layout: DescriptorImageLayout::General,
        }
    }

    /// Creates image info for sampler only
    pub const fn sampler(sampler: SamplerHandle) -> Self {
        Self {
            sampler,
            image_view: TextureViewHandle(0),
            image_layout: DescriptorImageLayout::Undefined,
        }
    }
}

/// Descriptor write data
#[derive(Clone, Debug)]
pub enum DescriptorWriteData {
    /// Buffer descriptors
    Buffers(Vec<DescriptorBufferInfo>),
    /// Image descriptors
    Images(Vec<DescriptorImageInfo>),
    /// Texel buffer views
    TexelBufferViews(Vec<u64>), // Buffer view handles
    /// Acceleration structures
    AccelerationStructures(Vec<u64>), // AS handles
}

/// Descriptor set write
#[derive(Clone, Debug)]
pub struct DescriptorSetWrite {
    /// Destination set
    pub dst_set: DescriptorSetHandle,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Data to write
    pub data: DescriptorWriteData,
}

impl DescriptorSetWrite {
    /// Creates a write for uniform buffer
    pub fn uniform_buffer(
        dst_set: DescriptorSetHandle,
        binding: u32,
        buffer_info: DescriptorBufferInfo,
    ) -> Self {
        Self {
            dst_set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            data: DescriptorWriteData::Buffers(alloc::vec![buffer_info]),
        }
    }

    /// Creates a write for storage buffer
    pub fn storage_buffer(
        dst_set: DescriptorSetHandle,
        binding: u32,
        buffer_info: DescriptorBufferInfo,
    ) -> Self {
        Self {
            dst_set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::StorageBuffer,
            data: DescriptorWriteData::Buffers(alloc::vec![buffer_info]),
        }
    }

    /// Creates a write for combined image sampler
    pub fn combined_image_sampler(
        dst_set: DescriptorSetHandle,
        binding: u32,
        image_info: DescriptorImageInfo,
    ) -> Self {
        Self {
            dst_set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::CombinedImageSampler,
            data: DescriptorWriteData::Images(alloc::vec![image_info]),
        }
    }

    /// Creates a write for sampled image
    pub fn sampled_image(
        dst_set: DescriptorSetHandle,
        binding: u32,
        image_info: DescriptorImageInfo,
    ) -> Self {
        Self {
            dst_set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::SampledImage,
            data: DescriptorWriteData::Images(alloc::vec![image_info]),
        }
    }

    /// Creates a write for storage image
    pub fn storage_image(
        dst_set: DescriptorSetHandle,
        binding: u32,
        image_info: DescriptorImageInfo,
    ) -> Self {
        Self {
            dst_set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::StorageImage,
            data: DescriptorWriteData::Images(alloc::vec![image_info]),
        }
    }

    /// Sets the array element
    pub fn at_array_element(mut self, element: u32) -> Self {
        self.dst_array_element = element;
        self
    }
}

/// Descriptor set copy
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorSetCopy {
    /// Source set
    pub src_set: DescriptorSetHandle,
    /// Source binding
    pub src_binding: u32,
    /// Source array element
    pub src_array_element: u32,
    /// Destination set
    pub dst_set: DescriptorSetHandle,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Number of descriptors to copy
    pub descriptor_count: u32,
}

impl DescriptorSetCopy {
    /// Creates a simple copy between sets
    pub const fn new(
        src_set: DescriptorSetHandle,
        src_binding: u32,
        dst_set: DescriptorSetHandle,
        dst_binding: u32,
        count: u32,
    ) -> Self {
        Self {
            src_set,
            src_binding,
            src_array_element: 0,
            dst_set,
            dst_binding,
            dst_array_element: 0,
            descriptor_count: count,
        }
    }
}

/// Push constant range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRange {
    /// Shader stages that access this range
    pub stage_flags: ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates a new push constant range
    pub const fn new(stage_flags: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stage_flags,
            offset,
            size,
        }
    }

    /// Creates a vertex stage push constant range
    pub const fn vertex(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::VERTEX, offset, size)
    }

    /// Creates a fragment stage push constant range
    pub const fn fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::FRAGMENT, offset, size)
    }

    /// Creates a compute stage push constant range
    pub const fn compute(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::COMPUTE, offset, size)
    }

    /// Creates an all-graphics stage push constant range
    pub const fn all_graphics(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL_GRAPHICS, offset, size)
    }
}

/// Pipeline layout description
#[derive(Clone, Debug)]
pub struct PipelineLayoutDesc {
    /// Descriptor set layouts
    pub set_layouts: Vec<DescriptorSetLayoutHandle>,
    /// Push constant ranges
    pub push_constant_ranges: Vec<PushConstantRange>,
}

impl PipelineLayoutDesc {
    /// Creates an empty pipeline layout
    pub fn new() -> Self {
        Self {
            set_layouts: Vec::new(),
            push_constant_ranges: Vec::new(),
        }
    }

    /// Adds a set layout
    pub fn add_set_layout(mut self, layout: DescriptorSetLayoutHandle) -> Self {
        self.set_layouts.push(layout);
        self
    }

    /// Adds a push constant range
    pub fn add_push_constant(mut self, range: PushConstantRange) -> Self {
        self.push_constant_ranges.push(range);
        self
    }

    /// Sets all set layouts at once
    pub fn with_set_layouts(mut self, layouts: Vec<DescriptorSetLayoutHandle>) -> Self {
        self.set_layouts = layouts;
        self
    }
}

impl Default for PipelineLayoutDesc {
    fn default() -> Self {
        Self::new()
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
    /// Number of descriptors
    pub descriptor_count: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Offset in user data
    pub offset: usize,
    /// Stride between descriptors in user data
    pub stride: usize,
}

impl DescriptorUpdateTemplateEntry {
    /// Creates a new template entry
    pub const fn new(
        dst_binding: u32,
        descriptor_type: DescriptorType,
        offset: usize,
        stride: usize,
    ) -> Self {
        Self {
            dst_binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type,
            offset,
            stride,
        }
    }

    /// Sets descriptor count
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }
}

/// Handle to a descriptor update template
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorUpdateTemplateHandle(pub u64);

impl DescriptorUpdateTemplateHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Descriptor update template description
#[derive(Clone, Debug)]
pub struct DescriptorUpdateTemplateDesc {
    /// Template entries
    pub entries: Vec<DescriptorUpdateTemplateEntry>,
    /// Set layout this template is for
    pub set_layout: DescriptorSetLayoutHandle,
}

impl DescriptorUpdateTemplateDesc {
    /// Creates a new template description
    pub fn new(set_layout: DescriptorSetLayoutHandle) -> Self {
        Self {
            entries: Vec::new(),
            set_layout,
        }
    }

    /// Adds an entry
    pub fn add_entry(mut self, entry: DescriptorUpdateTemplateEntry) -> Self {
        self.entries.push(entry);
        self
    }
}
