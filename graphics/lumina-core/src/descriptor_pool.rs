//! Descriptor pool and allocation types
//!
//! This module provides types for managing descriptor pools and allocations.

use core::num::NonZeroU32;

/// Descriptor pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorPoolHandle(pub NonZeroU32);

impl DescriptorPoolHandle {
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

/// Descriptor set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetHandle(pub NonZeroU32);

impl DescriptorSetHandle {
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

/// Descriptor set layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorSetLayoutHandle(pub NonZeroU32);

impl DescriptorSetLayoutHandle {
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

/// Descriptor pool creation info
#[derive(Clone, Debug)]
pub struct DescriptorPoolCreateInfo {
    /// Maximum number of sets that can be allocated
    pub max_sets: u32,
    /// Pool sizes per descriptor type
    pub pool_sizes: alloc::vec::Vec<DescriptorPoolSize>,
    /// Creation flags
    pub flags: DescriptorPoolCreateFlags,
}

use alloc::vec::Vec;

impl DescriptorPoolCreateInfo {
    /// Creates a new pool info
    pub fn new(max_sets: u32) -> Self {
        Self {
            max_sets,
            pool_sizes: Vec::new(),
            flags: DescriptorPoolCreateFlags::empty(),
        }
    }

    /// For a simple material system
    pub fn for_materials(count: u32) -> Self {
        Self::new(count)
            .add_pool_size(DescriptorType::UniformBuffer, count)
            .add_pool_size(DescriptorType::CombinedImageSampler, count * 4)
    }

    /// For compute workloads
    pub fn for_compute(count: u32) -> Self {
        Self::new(count)
            .add_pool_size(DescriptorType::StorageBuffer, count * 4)
            .add_pool_size(DescriptorType::StorageImage, count * 2)
    }

    /// Adds a pool size
    pub fn add_pool_size(mut self, ty: DescriptorType, count: u32) -> Self {
        self.pool_sizes.push(DescriptorPoolSize {
            ty,
            descriptor_count: count,
        });
        self
    }

    /// With free descriptor set flag
    pub fn with_free_descriptor_sets(mut self) -> Self {
        self.flags |= DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET;
        self
    }

    /// With update after bind
    pub fn with_update_after_bind(mut self) -> Self {
        self.flags |= DescriptorPoolCreateFlags::UPDATE_AFTER_BIND;
        self
    }
}

/// Descriptor pool size
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorPoolSize {
    /// Descriptor type
    pub ty: DescriptorType,
    /// Number of descriptors of this type
    pub descriptor_count: u32,
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DescriptorType {
    /// Sampler only
    Sampler              = 0,
    /// Combined image and sampler
    CombinedImageSampler = 1,
    /// Sampled image
    SampledImage         = 2,
    /// Storage image
    StorageImage         = 3,
    /// Uniform texel buffer
    UniformTexelBuffer   = 4,
    /// Storage texel buffer
    StorageTexelBuffer   = 5,
    /// Uniform buffer
    UniformBuffer        = 6,
    /// Storage buffer
    StorageBuffer        = 7,
    /// Uniform buffer dynamic
    UniformBufferDynamic = 8,
    /// Storage buffer dynamic
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment      = 10,
    /// Inline uniform block
    InlineUniformBlock   = 1000138000,
    /// Acceleration structure
    AccelerationStructure = 1000150000,
    /// Mutable descriptor
    MutableDescriptor    = 1000351000,
}

impl DescriptorType {
    /// Is this a buffer type
    pub const fn is_buffer(self) -> bool {
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

    /// Is this an image type
    pub const fn is_image(self) -> bool {
        matches!(
            self,
            Self::CombinedImageSampler
                | Self::SampledImage
                | Self::StorageImage
                | Self::InputAttachment
        )
    }

    /// Is this a dynamic type
    pub const fn is_dynamic(self) -> bool {
        matches!(
            self,
            Self::UniformBufferDynamic | Self::StorageBufferDynamic
        )
    }
}

bitflags::bitflags! {
    /// Descriptor pool creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DescriptorPoolCreateFlags: u32 {
        /// Allow freeing individual descriptor sets
        const FREE_DESCRIPTOR_SET = 1 << 0;
        /// Allow updating descriptors after binding
        const UPDATE_AFTER_BIND = 1 << 1;
        /// Host only pool
        const HOST_ONLY = 1 << 2;
    }
}

impl DescriptorPoolCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Descriptor set allocate info
#[derive(Clone, Debug)]
pub struct DescriptorSetAllocateInfo {
    /// Pool to allocate from
    pub descriptor_pool: DescriptorPoolHandle,
    /// Layouts for the sets to allocate
    pub set_layouts: alloc::vec::Vec<DescriptorSetLayoutHandle>,
}

impl DescriptorSetAllocateInfo {
    /// Creates allocation info
    pub fn new(pool: DescriptorPoolHandle) -> Self {
        Self {
            descriptor_pool: pool,
            set_layouts: Vec::new(),
        }
    }

    /// Adds a layout
    pub fn add_layout(mut self, layout: DescriptorSetLayoutHandle) -> Self {
        self.set_layouts.push(layout);
        self
    }

    /// Adds multiple layouts
    pub fn add_layouts(
        mut self,
        layouts: impl IntoIterator<Item = DescriptorSetLayoutHandle>,
    ) -> Self {
        self.set_layouts.extend(layouts);
        self
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
    /// Number of descriptors (for arrays)
    pub descriptor_count: u32,
    /// Shader stages that can access this binding
    pub stage_flags: ShaderStageFlags,
    /// Immutable samplers (pointer for FFI)
    pub immutable_samplers: bool,
}

impl DescriptorSetLayoutBinding {
    /// Creates a uniform buffer binding
    pub const fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::UniformBuffer,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: false,
        }
    }

    /// Creates a storage buffer binding
    pub const fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::StorageBuffer,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: false,
        }
    }

    /// Creates a combined image sampler binding
    pub const fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::CombinedImageSampler,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: false,
        }
    }

    /// Creates a storage image binding
    pub const fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::StorageImage,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: false,
        }
    }

    /// Creates an input attachment binding
    pub const fn input_attachment(binding: u32) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::InputAttachment,
            descriptor_count: 1,
            stage_flags: ShaderStageFlags::FRAGMENT,
            immutable_samplers: false,
        }
    }

    /// Creates an acceleration structure binding
    pub const fn acceleration_structure(binding: u32, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type: DescriptorType::AccelerationStructure,
            descriptor_count: 1,
            stage_flags: stages,
            immutable_samplers: false,
        }
    }

    /// With array count
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// With immutable samplers
    pub const fn with_immutable_samplers(mut self) -> Self {
        self.immutable_samplers = true;
        self
    }
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
        /// All stages
        const ALL = 0x7FFFFFFF;
        /// Task shader (mesh shading)
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
    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self::from_bits_truncate(
        Self::RAYGEN.bits()
            | Self::ANY_HIT.bits()
            | Self::CLOSEST_HIT.bits()
            | Self::MISS.bits()
            | Self::INTERSECTION.bits()
            | Self::CALLABLE.bits(),
    );

    /// Vertex and fragment
    pub const VERTEX_FRAGMENT: Self =
        Self::from_bits_truncate(Self::VERTEX.bits() | Self::FRAGMENT.bits());
}

/// Descriptor set layout create info
#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutCreateInfo {
    /// Bindings in this layout
    pub bindings: alloc::vec::Vec<DescriptorSetLayoutBinding>,
    /// Creation flags
    pub flags: DescriptorSetLayoutCreateFlags,
}

impl DescriptorSetLayoutCreateInfo {
    /// Creates an empty layout
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            flags: DescriptorSetLayoutCreateFlags::empty(),
        }
    }

    /// Simple material layout (uniform buffer + 4 textures)
    pub fn material() -> Self {
        Self::new()
            .add_binding(DescriptorSetLayoutBinding::uniform_buffer(
                0,
                ShaderStageFlags::ALL_GRAPHICS,
            ))
            .add_binding(
                DescriptorSetLayoutBinding::combined_image_sampler(1, ShaderStageFlags::FRAGMENT)
                    .with_count(4),
            )
    }

    /// Compute layout (input/output storage buffers)
    pub fn compute_io() -> Self {
        Self::new()
            .add_binding(DescriptorSetLayoutBinding::storage_buffer(
                0,
                ShaderStageFlags::COMPUTE,
            ))
            .add_binding(DescriptorSetLayoutBinding::storage_buffer(
                1,
                ShaderStageFlags::COMPUTE,
            ))
    }

    /// Adds a binding
    pub fn add_binding(mut self, binding: DescriptorSetLayoutBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// With update after bind
    pub fn with_update_after_bind(mut self) -> Self {
        self.flags |= DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL;
        self
    }

    /// Push descriptor layout
    pub fn with_push_descriptor(mut self) -> Self {
        self.flags |= DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR;
        self
    }
}

impl Default for DescriptorSetLayoutCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Descriptor set layout creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DescriptorSetLayoutCreateFlags: u32 {
        /// Support update after bind
        const UPDATE_AFTER_BIND_POOL = 1 << 1;
        /// Push descriptor layout
        const PUSH_DESCRIPTOR = 1 << 0;
        /// Descriptor buffer layout
        const DESCRIPTOR_BUFFER = 1 << 4;
        /// Embedded immutable samplers
        const EMBEDDED_IMMUTABLE_SAMPLERS = 1 << 5;
    }
}

impl DescriptorSetLayoutCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Write descriptor set
#[derive(Clone, Debug)]
pub struct WriteDescriptorSet {
    /// Destination set
    pub dst_set: DescriptorSetHandle,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor write data
    pub write: DescriptorWriteData,
}

/// Descriptor write data
#[derive(Clone, Debug)]
pub enum DescriptorWriteData {
    /// Buffer info
    Buffer(alloc::vec::Vec<DescriptorBufferInfo>),
    /// Image info
    Image(alloc::vec::Vec<DescriptorImageInfo>),
    /// Texel buffer views
    TexelBuffer(alloc::vec::Vec<BufferViewHandle>),
    /// Inline uniform data
    InlineUniform(alloc::vec::Vec<u8>),
    /// Acceleration structure
    AccelerationStructure(alloc::vec::Vec<AccelerationStructureHandle>),
}

/// Buffer view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferViewHandle(pub NonZeroU32);

/// Acceleration structure handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureHandle(pub NonZeroU32);

impl WriteDescriptorSet {
    /// Creates a buffer write
    pub fn buffer(
        set: DescriptorSetHandle,
        binding: u32,
        buffer: BufferHandle,
        offset: u64,
        range: u64,
    ) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DescriptorType::UniformBuffer,
            write: DescriptorWriteData::Buffer(alloc::vec![DescriptorBufferInfo {
                buffer,
                offset,
                range,
            }]),
        }
    }

    /// Creates an image write
    pub fn image(
        set: DescriptorSetHandle,
        binding: u32,
        sampler: SamplerHandle,
        image_view: ImageViewHandle,
        image_layout: ImageLayout,
    ) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DescriptorType::CombinedImageSampler,
            write: DescriptorWriteData::Image(alloc::vec![DescriptorImageInfo {
                sampler: Some(sampler),
                image_view,
                image_layout,
            }]),
        }
    }

    /// Creates a storage image write
    pub fn storage_image(
        set: DescriptorSetHandle,
        binding: u32,
        image_view: ImageViewHandle,
    ) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DescriptorType::StorageImage,
            write: DescriptorWriteData::Image(alloc::vec![DescriptorImageInfo {
                sampler: None,
                image_view,
                image_layout: ImageLayout::General,
            }]),
        }
    }

    /// With array element offset
    pub fn at_array_element(mut self, element: u32) -> Self {
        self.dst_array_element = element;
        self
    }

    /// As storage buffer instead of uniform
    pub fn as_storage(mut self) -> Self {
        self.descriptor_type = DescriptorType::StorageBuffer;
        self
    }
}

/// Buffer handle (for descriptors)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferHandle(pub NonZeroU32);

/// Sampler handle (for descriptors)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SamplerHandle(pub NonZeroU32);

/// Image view handle (for descriptors)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageViewHandle(pub NonZeroU32);

/// Descriptor buffer info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorBufferInfo {
    /// Buffer
    pub buffer: BufferHandle,
    /// Offset
    pub offset: u64,
    /// Range (WHOLE_SIZE for whole buffer)
    pub range: u64,
}

/// Whole buffer size constant
pub const WHOLE_SIZE: u64 = !0u64;

impl DescriptorBufferInfo {
    /// Whole buffer
    pub const fn whole(buffer: BufferHandle) -> Self {
        Self {
            buffer,
            offset: 0,
            range: WHOLE_SIZE,
        }
    }

    /// With offset and size
    pub const fn range(buffer: BufferHandle, offset: u64, size: u64) -> Self {
        Self {
            buffer,
            offset,
            range: size,
        }
    }
}

/// Descriptor image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorImageInfo {
    /// Sampler (optional for storage images)
    pub sampler: Option<SamplerHandle>,
    /// Image view
    pub image_view: ImageViewHandle,
    /// Image layout
    pub image_layout: ImageLayout,
}

/// Image layout
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined (don't care about contents)
    #[default]
    Undefined          = 0,
    /// General (any operation)
    General            = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized     = 8,
    /// Present source
    PresentSrc         = 1000001002,
}

impl ImageLayout {
    /// Is this a read-only layout
    pub const fn is_read_only(self) -> bool {
        matches!(
            self,
            Self::DepthStencilReadOnlyOptimal
                | Self::ShaderReadOnlyOptimal
                | Self::TransferSrcOptimal
                | Self::PresentSrc
        )
    }
}

/// Copy descriptor set
#[derive(Clone, Copy, Debug)]
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

impl CopyDescriptorSet {
    /// Creates a copy operation
    pub const fn new(
        src: DescriptorSetHandle,
        src_binding: u32,
        dst: DescriptorSetHandle,
        dst_binding: u32,
    ) -> Self {
        Self {
            src_set: src,
            src_binding,
            src_array_element: 0,
            dst_set: dst,
            dst_binding,
            dst_array_element: 0,
            descriptor_count: 1,
        }
    }

    /// With count
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }
}

/// Descriptor update template handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DescriptorUpdateTemplateHandle(pub NonZeroU32);

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
    /// Offset in update data
    pub offset: usize,
    /// Stride in update data
    pub stride: usize,
}

/// Descriptor update template type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum DescriptorUpdateTemplateType {
    /// Descriptor set
    #[default]
    DescriptorSet   = 0,
    /// Push descriptors
    PushDescriptors = 1,
}

/// Descriptor update template create info
#[derive(Clone, Debug)]
pub struct DescriptorUpdateTemplateCreateInfo {
    /// Template entries
    pub entries: alloc::vec::Vec<DescriptorUpdateTemplateEntry>,
    /// Template type
    pub template_type: DescriptorUpdateTemplateType,
    /// Descriptor set layout (for DescriptorSet type)
    pub descriptor_set_layout: Option<DescriptorSetLayoutHandle>,
    /// Pipeline bind point (for PushDescriptors)
    pub pipeline_bind_point: PipelineBindPoint,
    /// Pipeline layout (for PushDescriptors)
    pub pipeline_layout: Option<PipelineLayoutHandle>,
    /// Set number (for PushDescriptors)
    pub set: u32,
}

/// Pipeline bind point
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PipelineBindPoint {
    /// Graphics pipeline
    #[default]
    Graphics   = 0,
    /// Compute pipeline
    Compute    = 1,
    /// Ray tracing pipeline
    RayTracing = 1000165000,
}

/// Pipeline layout handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineLayoutHandle(pub NonZeroU32);

impl PipelineLayoutHandle {
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

impl DescriptorUpdateTemplateCreateInfo {
    /// Creates template for a descriptor set
    pub fn for_set(layout: DescriptorSetLayoutHandle) -> Self {
        Self {
            entries: Vec::new(),
            template_type: DescriptorUpdateTemplateType::DescriptorSet,
            descriptor_set_layout: Some(layout),
            pipeline_bind_point: PipelineBindPoint::Graphics,
            pipeline_layout: None,
            set: 0,
        }
    }

    /// Creates template for push descriptors
    pub fn for_push_descriptors(
        pipeline_layout: PipelineLayoutHandle,
        bind_point: PipelineBindPoint,
        set: u32,
    ) -> Self {
        Self {
            entries: Vec::new(),
            template_type: DescriptorUpdateTemplateType::PushDescriptors,
            descriptor_set_layout: None,
            pipeline_bind_point: bind_point,
            pipeline_layout: Some(pipeline_layout),
            set,
        }
    }

    /// Adds an entry
    pub fn add_entry(mut self, entry: DescriptorUpdateTemplateEntry) -> Self {
        self.entries.push(entry);
        self
    }
}

/// Descriptor pool inline uniform block create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorPoolInlineUniformBlockCreateInfo {
    /// Maximum inline uniform block bindings
    pub max_inline_uniform_block_bindings: u32,
}

/// Bindless descriptor info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BindlessDescriptorInfo {
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Maximum descriptor count
    pub max_descriptor_count: u32,
    /// Binding flags
    pub binding_flags: DescriptorBindingFlags,
}

bitflags::bitflags! {
    /// Descriptor binding flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DescriptorBindingFlags: u32 {
        /// Update after bind
        const UPDATE_AFTER_BIND = 1 << 0;
        /// Update unused while pending
        const UPDATE_UNUSED_WHILE_PENDING = 1 << 1;
        /// Partially bound
        const PARTIALLY_BOUND = 1 << 2;
        /// Variable descriptor count
        const VARIABLE_DESCRIPTOR_COUNT = 1 << 3;
    }
}

impl DescriptorBindingFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// All bindless flags
    pub const BINDLESS: Self = Self::from_bits_truncate(
        Self::UPDATE_AFTER_BIND.bits()
            | Self::UPDATE_UNUSED_WHILE_PENDING.bits()
            | Self::PARTIALLY_BOUND.bits(),
    );
}
