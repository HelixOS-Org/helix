//! Pipeline Layout Types for Lumina
//!
//! This module provides pipeline layout configuration, descriptor set layouts,
//! and push constant range definitions.

// ============================================================================
// Pipeline Layout Handle
// ============================================================================

/// Pipeline layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLayoutHandle(pub u64);

impl PipelineLayoutHandle {
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

impl Default for PipelineLayoutHandle {
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
// Pipeline Layout Create Info
// ============================================================================

/// Pipeline layout create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PipelineLayoutCreateInfo<'a> {
    /// Flags
    pub flags: PipelineLayoutCreateFlags,
    /// Set layouts
    pub set_layouts: &'a [DescriptorSetLayoutHandle],
    /// Push constant ranges
    pub push_constant_ranges: &'a [PushConstantRange],
}

impl<'a> PipelineLayoutCreateInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: PipelineLayoutCreateFlags::NONE,
            set_layouts: &[],
            push_constant_ranges: &[],
        }
    }

    /// With set layouts
    #[inline]
    pub const fn with_set_layouts(mut self, layouts: &'a [DescriptorSetLayoutHandle]) -> Self {
        self.set_layouts = layouts;
        self
    }

    /// With push constant ranges
    #[inline]
    pub const fn with_push_constants(mut self, ranges: &'a [PushConstantRange]) -> Self {
        self.push_constant_ranges = ranges;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: PipelineLayoutCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for PipelineLayoutCreateInfo<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline layout create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineLayoutCreateFlags(pub u32);

impl PipelineLayoutCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Independent sets
    pub const INDEPENDENT_SETS: Self = Self(1 << 1);

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
// Push Constant Range
// ============================================================================

/// Push constant range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRange {
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
}

impl PushConstantRange {
    /// Maximum push constant size (128 bytes minimum guaranteed)
    pub const MAX_SIZE: u32 = 128;

    /// Creates new range
    #[inline]
    pub const fn new(stages: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stage_flags: stages,
            offset,
            size,
        }
    }

    /// Vertex stage
    #[inline]
    pub const fn vertex(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::VERTEX, offset, size)
    }

    /// Fragment stage
    #[inline]
    pub const fn fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::FRAGMENT, offset, size)
    }

    /// Compute stage
    #[inline]
    pub const fn compute(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::COMPUTE, offset, size)
    }

    /// All graphics stages
    #[inline]
    pub const fn all_graphics(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL_GRAPHICS, offset, size)
    }

    /// All stages
    #[inline]
    pub const fn all(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL, offset, size)
    }

    /// End offset
    #[inline]
    pub const fn end(&self) -> u32 {
        self.offset + self.size
    }

    /// Overlaps with another range
    #[inline]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.offset < other.end() && other.offset < self.end()
    }
}

impl Default for PushConstantRange {
    fn default() -> Self {
        Self::new(ShaderStageFlags::ALL, 0, 0)
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
    /// No stages
    pub const NONE: Self = Self(0);
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
    /// Task
    pub const TASK: Self = Self(1 << 6);
    /// Mesh
    pub const MESH: Self = Self(1 << 7);
    /// Ray generation
    pub const RAY_GENERATION: Self = Self(1 << 8);
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
    /// Subpass shading
    pub const SUBPASS_SHADING: Self = Self(1 << 14);
    /// Cluster culling
    pub const CLUSTER_CULLING: Self = Self(1 << 19);

    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self(
        Self::RAY_GENERATION.0
            | Self::ANY_HIT.0
            | Self::CLOSEST_HIT.0
            | Self::MISS.0
            | Self::INTERSECTION.0
            | Self::CALLABLE.0,
    );
    /// All stages
    pub const ALL: Self = Self(0x7FFFFFFF);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Count stages
    #[inline]
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

// ============================================================================
// Descriptor Set Layout Create Info
// ============================================================================

/// Descriptor set layout create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DescriptorSetLayoutCreateInfo<'a> {
    /// Flags
    pub flags: DescriptorSetLayoutCreateFlags,
    /// Bindings
    pub bindings: &'a [DescriptorSetLayoutBinding],
    /// Binding flags (optional, same length as bindings)
    pub binding_flags: Option<&'a [DescriptorBindingFlags]>,
}

impl<'a> DescriptorSetLayoutCreateInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(bindings: &'a [DescriptorSetLayoutBinding]) -> Self {
        Self {
            flags: DescriptorSetLayoutCreateFlags::NONE,
            bindings,
            binding_flags: None,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: DescriptorSetLayoutCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With binding flags
    #[inline]
    pub const fn with_binding_flags(mut self, flags: &'a [DescriptorBindingFlags]) -> Self {
        self.binding_flags = Some(flags);
        self
    }

    /// Push descriptor layout
    #[inline]
    pub const fn push_descriptor(mut self) -> Self {
        self.flags = self.flags.union(DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR);
        self
    }

    /// Update after bind layout
    #[inline]
    pub const fn update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        self
    }
}

impl Default for DescriptorSetLayoutCreateInfo<'_> {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// Descriptor set layout create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorSetLayoutCreateFlags(pub u32);

impl DescriptorSetLayoutCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Push descriptor
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 0);
    /// Update after bind pool
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 1);
    /// Host only pool
    pub const HOST_ONLY_POOL: Self = Self(1 << 2);
    /// Descriptor buffer
    pub const DESCRIPTOR_BUFFER: Self = Self(1 << 4);
    /// Embedded immutable samplers
    pub const EMBEDDED_IMMUTABLE_SAMPLERS: Self = Self(1 << 5);
    /// Indirect bindable
    pub const INDIRECT_BINDABLE: Self = Self(1 << 7);
    /// Per stage
    pub const PER_STAGE: Self = Self(1 << 6);

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
// Descriptor Set Layout Binding
// ============================================================================

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
    pub stage_flags: ShaderStageFlags,
    /// Immutable samplers (pointer)
    pub immutable_samplers: u64,
}

impl DescriptorSetLayoutBinding {
    /// Creates new binding
    #[inline]
    pub const fn new(
        binding: u32,
        descriptor_type: DescriptorType,
        count: u32,
        stages: ShaderStageFlags,
    ) -> Self {
        Self {
            binding,
            descriptor_type,
            descriptor_count: count,
            stage_flags: stages,
            immutable_samplers: 0,
        }
    }

    /// Uniform buffer
    #[inline]
    pub const fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, 1, stages)
    }

    /// Storage buffer
    #[inline]
    pub const fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, 1, stages)
    }

    /// Sampler
    #[inline]
    pub const fn sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::Sampler, 1, stages)
    }

    /// Sampled image
    #[inline]
    pub const fn sampled_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::SampledImage, 1, stages)
    }

    /// Combined image sampler
    #[inline]
    pub const fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, 1, stages)
    }

    /// Storage image
    #[inline]
    pub const fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageImage, 1, stages)
    }

    /// Input attachment
    #[inline]
    pub const fn input_attachment(binding: u32) -> Self {
        Self::new(binding, DescriptorType::InputAttachment, 1, ShaderStageFlags::FRAGMENT)
    }

    /// Acceleration structure
    #[inline]
    pub const fn acceleration_structure(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::AccelerationStructure, 1, stages)
    }

    /// With count
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// Array
    #[inline]
    pub const fn array(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }
}

impl Default for DescriptorSetLayoutBinding {
    fn default() -> Self {
        Self::new(0, DescriptorType::UniformBuffer, 1, ShaderStageFlags::ALL)
    }
}

// ============================================================================
// Descriptor Type
// ============================================================================

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    #[default]
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
    InlineUniformBlock = 1000138000,
    /// Acceleration structure
    AccelerationStructure = 1000150000,
    /// Mutable
    Mutable = 1000351000,
    /// Sample weight image
    SampleWeightImage = 1000440000,
    /// Block match image
    BlockMatchImage = 1000440001,
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
            Self::SampledImage
                | Self::StorageImage
                | Self::CombinedImageSampler
                | Self::InputAttachment
        )
    }

    /// Is dynamic type
    #[inline]
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::UniformBufferDynamic | Self::StorageBufferDynamic)
    }

    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Sampler => "Sampler",
            Self::CombinedImageSampler => "Combined Image Sampler",
            Self::SampledImage => "Sampled Image",
            Self::StorageImage => "Storage Image",
            Self::UniformTexelBuffer => "Uniform Texel Buffer",
            Self::StorageTexelBuffer => "Storage Texel Buffer",
            Self::UniformBuffer => "Uniform Buffer",
            Self::StorageBuffer => "Storage Buffer",
            Self::UniformBufferDynamic => "Uniform Buffer Dynamic",
            Self::StorageBufferDynamic => "Storage Buffer Dynamic",
            Self::InputAttachment => "Input Attachment",
            Self::InlineUniformBlock => "Inline Uniform Block",
            Self::AccelerationStructure => "Acceleration Structure",
            Self::Mutable => "Mutable",
            Self::SampleWeightImage => "Sample Weight Image",
            Self::BlockMatchImage => "Block Match Image",
        }
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

    /// Bindless (all flags for bindless descriptors)
    pub const BINDLESS: Self = Self(
        Self::UPDATE_AFTER_BIND.0
            | Self::UPDATE_UNUSED_WHILE_PENDING.0
            | Self::PARTIALLY_BOUND.0
            | Self::VARIABLE_DESCRIPTOR_COUNT.0,
    );

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
// Descriptor Update Template
// ============================================================================

/// Descriptor update template handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorUpdateTemplateHandle(pub u64);

impl DescriptorUpdateTemplateHandle {
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

impl Default for DescriptorUpdateTemplateHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Descriptor update template create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DescriptorUpdateTemplateCreateInfo<'a> {
    /// Flags
    pub flags: DescriptorUpdateTemplateCreateFlags,
    /// Entries
    pub entries: &'a [DescriptorUpdateTemplateEntry],
    /// Template type
    pub template_type: DescriptorUpdateTemplateType,
    /// Descriptor set layout
    pub descriptor_set_layout: DescriptorSetLayoutHandle,
    /// Pipeline bind point
    pub pipeline_bind_point: PipelineBindPoint,
    /// Pipeline layout
    pub pipeline_layout: PipelineLayoutHandle,
    /// Set
    pub set: u32,
}

impl<'a> DescriptorUpdateTemplateCreateInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(
        entries: &'a [DescriptorUpdateTemplateEntry],
        layout: DescriptorSetLayoutHandle,
    ) -> Self {
        Self {
            flags: DescriptorUpdateTemplateCreateFlags::NONE,
            entries,
            template_type: DescriptorUpdateTemplateType::DescriptorSet,
            descriptor_set_layout: layout,
            pipeline_bind_point: PipelineBindPoint::Graphics,
            pipeline_layout: PipelineLayoutHandle::NULL,
            set: 0,
        }
    }

    /// For push descriptors
    #[inline]
    pub const fn push_descriptor(
        entries: &'a [DescriptorUpdateTemplateEntry],
        layout: PipelineLayoutHandle,
        bind_point: PipelineBindPoint,
        set: u32,
    ) -> Self {
        Self {
            flags: DescriptorUpdateTemplateCreateFlags::NONE,
            entries,
            template_type: DescriptorUpdateTemplateType::PushDescriptor,
            descriptor_set_layout: DescriptorSetLayoutHandle::NULL,
            pipeline_bind_point: bind_point,
            pipeline_layout: layout,
            set,
        }
    }
}

impl Default for DescriptorUpdateTemplateCreateInfo<'_> {
    fn default() -> Self {
        Self::new(&[], DescriptorSetLayoutHandle::NULL)
    }
}

/// Descriptor update template create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorUpdateTemplateCreateFlags(pub u32);

impl DescriptorUpdateTemplateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
}

/// Descriptor update template type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DescriptorUpdateTemplateType {
    /// Descriptor set
    #[default]
    DescriptorSet = 0,
    /// Push descriptor
    PushDescriptor = 1,
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
    /// Ray tracing
    RayTracing = 1000165000,
    /// Subpass shading
    SubpassShading = 1000369003,
}

impl PipelineBindPoint {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::Compute => "Compute",
            Self::RayTracing => "Ray Tracing",
            Self::SubpassShading => "Subpass Shading",
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
    /// Stride between elements
    pub stride: usize,
}

impl DescriptorUpdateTemplateEntry {
    /// Creates new entry
    #[inline]
    pub const fn new(
        binding: u32,
        descriptor_type: DescriptorType,
        offset: usize,
        stride: usize,
    ) -> Self {
        Self {
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type,
            offset,
            stride,
        }
    }

    /// With array element
    #[inline]
    pub const fn with_array_element(mut self, element: u32) -> Self {
        self.dst_array_element = element;
        self
    }

    /// With count
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }
}

impl Default for DescriptorUpdateTemplateEntry {
    fn default() -> Self {
        Self::new(0, DescriptorType::UniformBuffer, 0, 0)
    }
}

// ============================================================================
// Descriptor Pool Size
// ============================================================================

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
    /// Creates new size
    #[inline]
    pub const fn new(descriptor_type: DescriptorType, count: u32) -> Self {
        Self {
            descriptor_type,
            descriptor_count: count,
        }
    }

    /// Uniform buffers
    #[inline]
    pub const fn uniform_buffers(count: u32) -> Self {
        Self::new(DescriptorType::UniformBuffer, count)
    }

    /// Storage buffers
    #[inline]
    pub const fn storage_buffers(count: u32) -> Self {
        Self::new(DescriptorType::StorageBuffer, count)
    }

    /// Samplers
    #[inline]
    pub const fn samplers(count: u32) -> Self {
        Self::new(DescriptorType::Sampler, count)
    }

    /// Sampled images
    #[inline]
    pub const fn sampled_images(count: u32) -> Self {
        Self::new(DescriptorType::SampledImage, count)
    }

    /// Combined image samplers
    #[inline]
    pub const fn combined_image_samplers(count: u32) -> Self {
        Self::new(DescriptorType::CombinedImageSampler, count)
    }

    /// Storage images
    #[inline]
    pub const fn storage_images(count: u32) -> Self {
        Self::new(DescriptorType::StorageImage, count)
    }
}

impl Default for DescriptorPoolSize {
    fn default() -> Self {
        Self::new(DescriptorType::UniformBuffer, 1)
    }
}

// ============================================================================
// Pipeline Layout Support
// ============================================================================

/// Push constant data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantData<T> {
    /// Stage flags
    pub stages: ShaderStageFlags,
    /// Offset
    pub offset: u32,
    /// Data
    pub data: T,
}

impl<T: Copy> PushConstantData<T> {
    /// Creates new data
    #[inline]
    pub const fn new(stages: ShaderStageFlags, offset: u32, data: T) -> Self {
        Self { stages, offset, data }
    }

    /// Size of data
    #[inline]
    pub const fn size() -> u32 {
        core::mem::size_of::<T>() as u32
    }
}

/// Common push constant layout (MVP)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct MvpPushConstants {
    /// Model matrix (4x4)
    pub model: [[f32; 4]; 4],
    /// View matrix (4x4)
    pub view: [[f32; 4]; 4],
    /// Projection matrix (4x4)
    pub projection: [[f32; 4]; 4],
}

impl MvpPushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 192; // 3 * 64 bytes

    /// Identity
    pub const IDENTITY: Self = Self {
        model: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        view: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        projection: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
}

/// Simple push constant layout (model + extras)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct SimplePushConstants {
    /// Model matrix (4x4)
    pub model: [[f32; 4]; 4],
    /// Color
    pub color: [f32; 4],
    /// Extra data
    pub extra: [f32; 4],
}

impl SimplePushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 96; // 64 + 16 + 16 bytes
}

/// Compute push constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(4))]
pub struct ComputePushConstants {
    /// Dispatch size
    pub dispatch_size: [u32; 3],
    /// Flags
    pub flags: u32,
    /// Time
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Frame
    pub frame: u32,
    /// Padding
    pub _padding: u32,
}

impl ComputePushConstants {
    /// Size in bytes
    pub const SIZE: u32 = 32;
}
