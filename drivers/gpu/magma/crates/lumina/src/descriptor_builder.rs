//! Descriptor Set Builder for Lumina
//!
//! This module provides comprehensive descriptor set allocation, layout creation,
//! and resource binding infrastructure with fluent builder patterns.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Descriptor Set Handle
// ============================================================================

/// Descriptor set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetHandle(pub u64);

impl DescriptorSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for DescriptorSetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Descriptor Set Layout Handle
// ============================================================================

/// Descriptor set layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutHandle(pub u64);

impl DescriptorSetLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for DescriptorSetLayoutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Descriptor Pool Handle
// ============================================================================

/// Descriptor pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorPoolHandle(pub u64);

impl DescriptorPoolHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for DescriptorPoolHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Descriptor Set Layout Builder
// ============================================================================

/// Descriptor set layout builder
#[derive(Clone, Debug, Default)]
pub struct DescriptorSetLayoutBuilder {
    /// Bindings
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    /// Flags
    pub flags: DescriptorSetLayoutCreateFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl DescriptorSetLayoutBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            flags: DescriptorSetLayoutCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Add binding
    #[inline]
    pub fn binding(mut self, binding: DescriptorSetLayoutBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Add uniform buffer binding
    #[inline]
    pub fn uniform_buffer(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::uniform_buffer(binding, stages));
        self
    }

    /// Add storage buffer binding
    #[inline]
    pub fn storage_buffer(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::storage_buffer(binding, stages));
        self
    }

    /// Add combined image sampler binding
    #[inline]
    pub fn combined_image_sampler(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::combined_image_sampler(binding, stages));
        self
    }

    /// Add sampled image binding
    #[inline]
    pub fn sampled_image(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::sampled_image(binding, stages));
        self
    }

    /// Add sampler binding
    #[inline]
    pub fn sampler(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::sampler(binding, stages));
        self
    }

    /// Add storage image binding
    #[inline]
    pub fn storage_image(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::storage_image(binding, stages));
        self
    }

    /// Add uniform texel buffer binding
    #[inline]
    pub fn uniform_texel_buffer(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::uniform_texel_buffer(binding, stages));
        self
    }

    /// Add storage texel buffer binding
    #[inline]
    pub fn storage_texel_buffer(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::storage_texel_buffer(binding, stages));
        self
    }

    /// Add input attachment binding
    #[inline]
    pub fn input_attachment(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::input_attachment(binding, stages));
        self
    }

    /// Add acceleration structure binding
    #[inline]
    pub fn acceleration_structure(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::acceleration_structure(binding, stages));
        self
    }

    /// Add dynamic uniform buffer binding
    #[inline]
    pub fn uniform_buffer_dynamic(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::uniform_buffer_dynamic(binding, stages));
        self
    }

    /// Add dynamic storage buffer binding
    #[inline]
    pub fn storage_buffer_dynamic(mut self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding::storage_buffer_dynamic(binding, stages));
        self
    }

    /// Add array of descriptors
    #[inline]
    pub fn array(mut self, binding: u32, descriptor_type: DescriptorType, count: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding {
            binding,
            descriptor_type,
            descriptor_count: count,
            stage_flags: stages,
            immutable_samplers: Vec::new(),
            binding_flags: DescriptorBindingFlags::NONE,
        });
        self
    }

    /// Add bindless array (variable count, partially bound)
    #[inline]
    pub fn bindless_array(mut self, binding: u32, descriptor_type: DescriptorType, max_count: u32, stages: ShaderStageFlags) -> Self {
        self.bindings.push(DescriptorSetLayoutBinding {
            binding,
            descriptor_type,
            descriptor_count: max_count,
            stage_flags: stages,
            immutable_samplers: Vec::new(),
            binding_flags: DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
                .union(DescriptorBindingFlags::PARTIALLY_BOUND)
                .union(DescriptorBindingFlags::UPDATE_AFTER_BIND),
        });
        self.flags = self.flags.union(DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        self
    }

    /// Set layout flags
    #[inline]
    pub fn flags(mut self, flags: DescriptorSetLayoutCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Push descriptor layout
    #[inline]
    pub fn push_descriptor(mut self) -> Self {
        self.flags = self.flags.union(DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR);
        self
    }

    /// Update after bind
    #[inline]
    pub fn update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        self
    }

    /// Set debug name
    #[inline]
    pub fn name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Build the layout create info
    pub fn build(self) -> DescriptorSetLayoutCreateInfo {
        DescriptorSetLayoutCreateInfo {
            flags: self.flags,
            bindings: self.bindings,
        }
    }
}

// ============================================================================
// Descriptor Set Layout Binding
// ============================================================================

/// Descriptor set layout binding
#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutBinding {
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Shader stage flags
    pub stage_flags: ShaderStageFlags,
    /// Immutable samplers
    pub immutable_samplers: Vec<u64>,
    /// Binding flags
    pub binding_flags: DescriptorBindingFlags,
}

impl DescriptorSetLayoutBinding {
    /// Creates new binding
    #[inline]
    pub const fn new(binding: u32, descriptor_type: DescriptorType, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: Vec::new(),
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Uniform buffer
    #[inline]
    pub const fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, stages)
    }

    /// Storage buffer
    #[inline]
    pub const fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, stages)
    }

    /// Combined image sampler
    #[inline]
    pub const fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, stages)
    }

    /// Sampled image
    #[inline]
    pub const fn sampled_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::SampledImage, stages)
    }

    /// Sampler
    #[inline]
    pub const fn sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::Sampler, stages)
    }

    /// Storage image
    #[inline]
    pub const fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageImage, stages)
    }

    /// Uniform texel buffer
    #[inline]
    pub const fn uniform_texel_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformTexelBuffer, stages)
    }

    /// Storage texel buffer
    #[inline]
    pub const fn storage_texel_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageTexelBuffer, stages)
    }

    /// Input attachment
    #[inline]
    pub const fn input_attachment(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::InputAttachment, stages)
    }

    /// Acceleration structure
    #[inline]
    pub const fn acceleration_structure(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::AccelerationStructureKhr, stages)
    }

    /// Dynamic uniform buffer
    #[inline]
    pub const fn uniform_buffer_dynamic(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBufferDynamic, stages)
    }

    /// Dynamic storage buffer
    #[inline]
    pub const fn storage_buffer_dynamic(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBufferDynamic, stages)
    }

    /// With count
    #[inline]
    pub fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// With immutable samplers
    #[inline]
    pub fn with_immutable_samplers(mut self, samplers: Vec<u64>) -> Self {
        self.descriptor_count = samplers.len() as u32;
        self.immutable_samplers = samplers;
        self
    }

    /// With binding flags
    #[inline]
    pub fn with_binding_flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.binding_flags = flags;
        self
    }

    /// Partially bound
    #[inline]
    pub fn partially_bound(mut self) -> Self {
        self.binding_flags = self.binding_flags.union(DescriptorBindingFlags::PARTIALLY_BOUND);
        self
    }

    /// Update after bind
    #[inline]
    pub fn update_after_bind(mut self) -> Self {
        self.binding_flags = self.binding_flags.union(DescriptorBindingFlags::UPDATE_AFTER_BIND);
        self
    }

    /// Variable count
    #[inline]
    pub fn variable_count(mut self) -> Self {
        self.binding_flags = self.binding_flags.union(DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT);
        self
    }
}

// ============================================================================
// Descriptor Type
// ============================================================================

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
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
    /// Dynamic uniform buffer
    UniformBufferDynamic = 8,
    /// Dynamic storage buffer
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment = 10,
    /// Inline uniform block
    InlineUniformBlock = 1000138000,
    /// Acceleration structure KHR
    AccelerationStructureKhr = 1000150000,
    /// Acceleration structure NV
    AccelerationStructureNv = 1000165000,
    /// Mutable EXT
    MutableExt = 1000351000,
}

impl DescriptorType {
    /// Is buffer type
    #[inline]
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
    #[inline]
    pub const fn is_image(&self) -> bool {
        matches!(
            self,
            Self::Sampler
                | Self::CombinedImageSampler
                | Self::SampledImage
                | Self::StorageImage
                | Self::InputAttachment
        )
    }

    /// Is dynamic
    #[inline]
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::UniformBufferDynamic | Self::StorageBufferDynamic)
    }
}

// ============================================================================
// Shader Stage Flags
// ============================================================================

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Vertex
    pub const VERTEX: Self = Self(0x00000001);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(0x00000002);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION: Self = Self(0x00000004);
    /// Geometry
    pub const GEOMETRY: Self = Self(0x00000008);
    /// Fragment
    pub const FRAGMENT: Self = Self(0x00000010);
    /// Compute
    pub const COMPUTE: Self = Self(0x00000020);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(0x0000001F);
    /// All
    pub const ALL: Self = Self(0x7FFFFFFF);
    /// Task
    pub const TASK: Self = Self(0x00000040);
    /// Mesh
    pub const MESH: Self = Self(0x00000080);
    /// Ray generation
    pub const RAYGEN: Self = Self(0x00000100);
    /// Any hit
    pub const ANY_HIT: Self = Self(0x00000200);
    /// Closest hit
    pub const CLOSEST_HIT: Self = Self(0x00000400);
    /// Miss
    pub const MISS: Self = Self(0x00000800);
    /// Intersection
    pub const INTERSECTION: Self = Self(0x00001000);
    /// Callable
    pub const CALLABLE: Self = Self(0x00002000);

    /// All ray tracing
    pub const ALL_RAY_TRACING: Self = Self(
        Self::RAYGEN.0
            | Self::ANY_HIT.0
            | Self::CLOSEST_HIT.0
            | Self::MISS.0
            | Self::INTERSECTION.0
            | Self::CALLABLE.0,
    );

    /// Vertex and fragment
    pub const VERTEX_FRAGMENT: Self = Self(Self::VERTEX.0 | Self::FRAGMENT.0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Descriptor Binding Flags
// ============================================================================

/// Descriptor binding flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorBindingFlags(pub u32);

impl DescriptorBindingFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Update unused while pending
    pub const UPDATE_UNUSED_WHILE_PENDING: Self = Self(1 << 1);
    /// Partially bound
    pub const PARTIALLY_BOUND: Self = Self(1 << 2);
    /// Variable descriptor count
    pub const VARIABLE_DESCRIPTOR_COUNT: Self = Self(1 << 3);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Descriptor Set Layout Create Info
// ============================================================================

/// Descriptor set layout create info
#[derive(Clone, Debug, Default)]
pub struct DescriptorSetLayoutCreateInfo {
    /// Flags
    pub flags: DescriptorSetLayoutCreateFlags,
    /// Bindings
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

/// Descriptor set layout create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorSetLayoutCreateFlags(pub u32);

impl DescriptorSetLayoutCreateFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Update after bind pool
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 1);
    /// Push descriptor KHR
    pub const PUSH_DESCRIPTOR_KHR: Self = Self(1 << 0);
    /// Descriptor buffer EXT
    pub const DESCRIPTOR_BUFFER_EXT: Self = Self(1 << 4);
    /// Embedded immutable samplers EXT
    pub const EMBEDDED_IMMUTABLE_SAMPLERS_EXT: Self = Self(1 << 5);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Descriptor Pool Builder
// ============================================================================

/// Descriptor pool builder
#[derive(Clone, Debug, Default)]
pub struct DescriptorPoolBuilder {
    /// Max sets
    pub max_sets: u32,
    /// Pool sizes
    pub pool_sizes: Vec<DescriptorPoolSize>,
    /// Flags
    pub flags: DescriptorPoolCreateFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl DescriptorPoolBuilder {
    /// Creates new builder
    pub fn new(max_sets: u32) -> Self {
        Self {
            max_sets,
            pool_sizes: Vec::new(),
            flags: DescriptorPoolCreateFlags::NONE,
            debug_name: None,
        }
    }

    /// Add pool size
    #[inline]
    pub fn pool_size(mut self, descriptor_type: DescriptorType, count: u32) -> Self {
        // Check if type already exists
        for size in &mut self.pool_sizes {
            if size.descriptor_type == descriptor_type {
                size.descriptor_count += count;
                return self;
            }
        }
        self.pool_sizes.push(DescriptorPoolSize {
            descriptor_type,
            descriptor_count: count,
        });
        self
    }

    /// Add uniform buffers
    #[inline]
    pub fn uniform_buffers(self, count: u32) -> Self {
        self.pool_size(DescriptorType::UniformBuffer, count)
    }

    /// Add storage buffers
    #[inline]
    pub fn storage_buffers(self, count: u32) -> Self {
        self.pool_size(DescriptorType::StorageBuffer, count)
    }

    /// Add combined image samplers
    #[inline]
    pub fn combined_image_samplers(self, count: u32) -> Self {
        self.pool_size(DescriptorType::CombinedImageSampler, count)
    }

    /// Add sampled images
    #[inline]
    pub fn sampled_images(self, count: u32) -> Self {
        self.pool_size(DescriptorType::SampledImage, count)
    }

    /// Add samplers
    #[inline]
    pub fn samplers(self, count: u32) -> Self {
        self.pool_size(DescriptorType::Sampler, count)
    }

    /// Add storage images
    #[inline]
    pub fn storage_images(self, count: u32) -> Self {
        self.pool_size(DescriptorType::StorageImage, count)
    }

    /// Add dynamic uniform buffers
    #[inline]
    pub fn uniform_buffers_dynamic(self, count: u32) -> Self {
        self.pool_size(DescriptorType::UniformBufferDynamic, count)
    }

    /// Add dynamic storage buffers
    #[inline]
    pub fn storage_buffers_dynamic(self, count: u32) -> Self {
        self.pool_size(DescriptorType::StorageBufferDynamic, count)
    }

    /// Add input attachments
    #[inline]
    pub fn input_attachments(self, count: u32) -> Self {
        self.pool_size(DescriptorType::InputAttachment, count)
    }

    /// Add acceleration structures
    #[inline]
    pub fn acceleration_structures(self, count: u32) -> Self {
        self.pool_size(DescriptorType::AccelerationStructureKhr, count)
    }

    /// Set flags
    #[inline]
    pub fn flags(mut self, flags: DescriptorPoolCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Free descriptor set flag
    #[inline]
    pub fn free_descriptor_set(mut self) -> Self {
        self.flags = self.flags.union(DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);
        self
    }

    /// Update after bind flag
    #[inline]
    pub fn update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(DescriptorPoolCreateFlags::UPDATE_AFTER_BIND);
        self
    }

    /// Set debug name
    #[inline]
    pub fn name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Build
    pub fn build(self) -> DescriptorPoolCreateInfo {
        DescriptorPoolCreateInfo {
            flags: self.flags,
            max_sets: self.max_sets,
            pool_sizes: self.pool_sizes,
        }
    }
}

/// Descriptor pool size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DescriptorPoolSize {
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count
    pub descriptor_count: u32,
}

/// Descriptor pool create info
#[derive(Clone, Debug, Default)]
pub struct DescriptorPoolCreateInfo {
    /// Flags
    pub flags: DescriptorPoolCreateFlags,
    /// Max sets
    pub max_sets: u32,
    /// Pool sizes
    pub pool_sizes: Vec<DescriptorPoolSize>,
}

/// Descriptor pool create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorPoolCreateFlags(pub u32);

impl DescriptorPoolCreateFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Free descriptor set
    pub const FREE_DESCRIPTOR_SET: Self = Self(1 << 0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 1);
    /// Host only EXT
    pub const HOST_ONLY_EXT: Self = Self(1 << 2);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Descriptor Set Allocate Info
// ============================================================================

/// Descriptor set allocate info
#[derive(Clone, Debug)]
pub struct DescriptorSetAllocateInfo {
    /// Descriptor pool
    pub descriptor_pool: DescriptorPoolHandle,
    /// Set layouts
    pub set_layouts: Vec<DescriptorSetLayoutHandle>,
    /// Variable descriptor counts
    pub variable_descriptor_counts: Vec<u32>,
}

impl DescriptorSetAllocateInfo {
    /// Creates new info
    pub fn new(pool: DescriptorPoolHandle) -> Self {
        Self {
            descriptor_pool: pool,
            set_layouts: Vec::new(),
            variable_descriptor_counts: Vec::new(),
        }
    }

    /// Add layout
    #[inline]
    pub fn add_layout(mut self, layout: DescriptorSetLayoutHandle) -> Self {
        self.set_layouts.push(layout);
        self
    }

    /// Add layouts
    #[inline]
    pub fn add_layouts(mut self, layouts: &[DescriptorSetLayoutHandle]) -> Self {
        self.set_layouts.extend_from_slice(layouts);
        self
    }

    /// With variable counts
    #[inline]
    pub fn with_variable_counts(mut self, counts: Vec<u32>) -> Self {
        self.variable_descriptor_counts = counts;
        self
    }
}

impl Default for DescriptorSetAllocateInfo {
    fn default() -> Self {
        Self::new(DescriptorPoolHandle::NULL)
    }
}

// ============================================================================
// Descriptor Update Builder
// ============================================================================

/// Descriptor update builder
#[derive(Clone, Debug, Default)]
pub struct DescriptorUpdateBuilder {
    /// Writes
    pub writes: Vec<WriteDescriptorSet>,
    /// Copies
    pub copies: Vec<CopyDescriptorSet>,
}

impl DescriptorUpdateBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            writes: Vec::new(),
            copies: Vec::new(),
        }
    }

    /// Write uniform buffer
    #[inline]
    pub fn write_uniform_buffer(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        buffer: u64,
        offset: u64,
        range: u64,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            image_info: Vec::new(),
            buffer_info: alloc::vec![DescriptorBufferInfo { buffer, offset, range }],
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write storage buffer
    #[inline]
    pub fn write_storage_buffer(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        buffer: u64,
        offset: u64,
        range: u64,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::StorageBuffer,
            image_info: Vec::new(),
            buffer_info: alloc::vec![DescriptorBufferInfo { buffer, offset, range }],
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write combined image sampler
    #[inline]
    pub fn write_combined_image_sampler(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        sampler: u64,
        image_view: u64,
        image_layout: ImageLayout,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::CombinedImageSampler,
            image_info: alloc::vec![DescriptorImageInfo {
                sampler,
                image_view,
                image_layout,
            }],
            buffer_info: Vec::new(),
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write sampled image
    #[inline]
    pub fn write_sampled_image(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        image_view: u64,
        image_layout: ImageLayout,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::SampledImage,
            image_info: alloc::vec![DescriptorImageInfo {
                sampler: 0,
                image_view,
                image_layout,
            }],
            buffer_info: Vec::new(),
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write storage image
    #[inline]
    pub fn write_storage_image(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        image_view: u64,
        image_layout: ImageLayout,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::StorageImage,
            image_info: alloc::vec![DescriptorImageInfo {
                sampler: 0,
                image_view,
                image_layout,
            }],
            buffer_info: Vec::new(),
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write sampler
    #[inline]
    pub fn write_sampler(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        sampler: u64,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::Sampler,
            image_info: alloc::vec![DescriptorImageInfo {
                sampler,
                image_view: 0,
                image_layout: ImageLayout::Undefined,
            }],
            buffer_info: Vec::new(),
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Write to array element
    #[inline]
    pub fn write_array_element(
        mut self,
        set: DescriptorSetHandle,
        binding: u32,
        array_element: u32,
        descriptor_type: DescriptorType,
        buffer_info: Option<DescriptorBufferInfo>,
        image_info: Option<DescriptorImageInfo>,
    ) -> Self {
        self.writes.push(WriteDescriptorSet {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: array_element,
            descriptor_type,
            image_info: image_info.map(|i| alloc::vec![i]).unwrap_or_default(),
            buffer_info: buffer_info.map(|b| alloc::vec![b]).unwrap_or_default(),
            texel_buffer_views: Vec::new(),
        });
        self
    }

    /// Copy descriptor
    #[inline]
    pub fn copy(
        mut self,
        src_set: DescriptorSetHandle,
        src_binding: u32,
        dst_set: DescriptorSetHandle,
        dst_binding: u32,
        count: u32,
    ) -> Self {
        self.copies.push(CopyDescriptorSet {
            src_set,
            src_binding,
            src_array_element: 0,
            dst_set,
            dst_binding,
            dst_array_element: 0,
            descriptor_count: count,
        });
        self
    }

    /// Build update info
    pub fn build(self) -> DescriptorUpdateInfo {
        DescriptorUpdateInfo {
            writes: self.writes,
            copies: self.copies,
        }
    }
}

// ============================================================================
// Write Descriptor Set
// ============================================================================

/// Write descriptor set
#[derive(Clone, Debug)]
pub struct WriteDescriptorSet {
    /// Destination set
    pub dst_set: DescriptorSetHandle,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Image info
    pub image_info: Vec<DescriptorImageInfo>,
    /// Buffer info
    pub buffer_info: Vec<DescriptorBufferInfo>,
    /// Texel buffer views
    pub texel_buffer_views: Vec<u64>,
}

/// Descriptor image info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DescriptorImageInfo {
    /// Sampler handle
    pub sampler: u64,
    /// Image view handle
    pub image_view: u64,
    /// Image layout
    pub image_layout: ImageLayout,
}

/// Descriptor buffer info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct DescriptorBufferInfo {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Range
    pub range: u64,
}

impl DescriptorBufferInfo {
    /// Whole buffer
    pub const WHOLE_SIZE: u64 = !0;

    /// Creates new info
    #[inline]
    pub const fn new(buffer: u64, offset: u64, range: u64) -> Self {
        Self { buffer, offset, range }
    }

    /// Whole buffer
    #[inline]
    pub const fn whole(buffer: u64) -> Self {
        Self {
            buffer,
            offset: 0,
            range: Self::WHOLE_SIZE,
        }
    }
}

// ============================================================================
// Copy Descriptor Set
// ============================================================================

/// Copy descriptor set
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct CopyDescriptorSet {
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
    /// Descriptor count
    pub descriptor_count: u32,
}

// ============================================================================
// Image Layout
// ============================================================================

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read-only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read-only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
}

// ============================================================================
// Descriptor Update Info
// ============================================================================

/// Descriptor update info
#[derive(Clone, Debug, Default)]
pub struct DescriptorUpdateInfo {
    /// Writes
    pub writes: Vec<WriteDescriptorSet>,
    /// Copies
    pub copies: Vec<CopyDescriptorSet>,
}

// ============================================================================
// Descriptor Template
// ============================================================================

/// Descriptor update template entry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Offset
    pub offset: usize,
    /// Stride
    pub stride: usize,
}

/// Descriptor update template create info
#[derive(Clone, Debug)]
pub struct DescriptorUpdateTemplateCreateInfo {
    /// Entries
    pub entries: Vec<DescriptorUpdateTemplateEntry>,
    /// Template type
    pub template_type: DescriptorUpdateTemplateType,
    /// Descriptor set layout
    pub descriptor_set_layout: DescriptorSetLayoutHandle,
    /// Pipeline bind point
    pub pipeline_bind_point: PipelineBindPoint,
    /// Pipeline layout
    pub pipeline_layout: u64,
    /// Set number
    pub set: u32,
}

/// Descriptor update template type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DescriptorUpdateTemplateType {
    /// Descriptor set
    #[default]
    DescriptorSet = 0,
    /// Push descriptors KHR
    PushDescriptorsKhr = 1,
}

/// Pipeline bind point
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PipelineBindPoint {
    /// Graphics
    #[default]
    Graphics = 0,
    /// Compute
    Compute = 1,
    /// Ray tracing KHR
    RayTracingKhr = 1000165000,
}

// ============================================================================
// Common Descriptor Set Layouts
// ============================================================================

/// Common descriptor set layouts
pub struct CommonLayouts;

impl CommonLayouts {
    /// Simple material layout (texture + uniform buffer)
    pub fn simple_material() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .uniform_buffer(0, ShaderStageFlags::VERTEX_FRAGMENT)
            .combined_image_sampler(1, ShaderStageFlags::FRAGMENT)
    }

    /// PBR material layout
    pub fn pbr_material() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .uniform_buffer(0, ShaderStageFlags::VERTEX_FRAGMENT) // Material params
            .combined_image_sampler(1, ShaderStageFlags::FRAGMENT) // Albedo
            .combined_image_sampler(2, ShaderStageFlags::FRAGMENT) // Normal
            .combined_image_sampler(3, ShaderStageFlags::FRAGMENT) // Metallic/Roughness
            .combined_image_sampler(4, ShaderStageFlags::FRAGMENT) // AO
            .combined_image_sampler(5, ShaderStageFlags::FRAGMENT) // Emissive
    }

    /// Camera/view layout
    pub fn camera() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .uniform_buffer(0, ShaderStageFlags::ALL_GRAPHICS) // Camera matrices
    }

    /// Lighting layout
    pub fn lighting() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .uniform_buffer(0, ShaderStageFlags::FRAGMENT) // Light data
            .combined_image_sampler(1, ShaderStageFlags::FRAGMENT) // Shadow map
            .storage_buffer(2, ShaderStageFlags::FRAGMENT) // Light clusters
    }

    /// Post-process layout
    pub fn post_process() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .combined_image_sampler(0, ShaderStageFlags::FRAGMENT) // Input color
            .combined_image_sampler(1, ShaderStageFlags::FRAGMENT) // Depth
            .uniform_buffer(2, ShaderStageFlags::FRAGMENT) // Post-process params
    }

    /// Compute shader layout
    pub fn compute() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
            .storage_buffer(0, ShaderStageFlags::COMPUTE) // Input
            .storage_buffer(1, ShaderStageFlags::COMPUTE) // Output
            .uniform_buffer(2, ShaderStageFlags::COMPUTE) // Params
    }
}
