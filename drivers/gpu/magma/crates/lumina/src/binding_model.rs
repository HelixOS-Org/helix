//! Binding Model for Lumina
//!
//! This module provides descriptor set layout and binding types for
//! connecting shader resources to pipeline stages.

// ============================================================================
// Descriptor Set Layout
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

/// Descriptor set layout binding
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorSetLayoutBinding {
    /// Binding index
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Number of descriptors
    pub descriptor_count: u32,
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Binding flags
    pub binding_flags: DescriptorBindingFlags,
}

impl DescriptorSetLayoutBinding {
    /// Creates new binding
    #[inline]
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
            binding_flags: DescriptorBindingFlags::NONE,
        }
    }

    /// Uniform buffer binding
    #[inline]
    pub const fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, stages)
    }

    /// Storage buffer binding
    #[inline]
    pub const fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, stages)
    }

    /// Sampled image binding
    #[inline]
    pub const fn sampled_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::SampledImage, stages)
    }

    /// Combined image sampler binding
    #[inline]
    pub const fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, stages)
    }

    /// Storage image binding
    #[inline]
    pub const fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageImage, stages)
    }

    /// Sampler binding
    #[inline]
    pub const fn sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::Sampler, stages)
    }

    /// Input attachment binding
    #[inline]
    pub const fn input_attachment(binding: u32) -> Self {
        Self::new(
            binding,
            DescriptorType::InputAttachment,
            ShaderStageFlags::FRAGMENT,
        )
    }

    /// Acceleration structure binding
    #[inline]
    pub const fn acceleration_structure(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::AccelerationStructure, stages)
    }

    /// With descriptor count
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }

    /// With binding flags
    #[inline]
    pub const fn with_flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.binding_flags = flags;
        self
    }

    /// Variable count (bindless)
    #[inline]
    pub const fn variable_count(mut self, max_count: u32) -> Self {
        self.descriptor_count = max_count;
        self.binding_flags = DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
            .union(DescriptorBindingFlags::PARTIALLY_BOUND);
        self
    }

    /// Partially bound
    #[inline]
    pub const fn partially_bound(mut self) -> Self {
        self.binding_flags = self
            .binding_flags
            .union(DescriptorBindingFlags::PARTIALLY_BOUND);
        self
    }

    /// Update after bind
    #[inline]
    pub const fn update_after_bind(mut self) -> Self {
        self.binding_flags = self
            .binding_flags
            .union(DescriptorBindingFlags::UPDATE_AFTER_BIND);
        self
    }
}

impl Default for DescriptorSetLayoutBinding {
    fn default() -> Self {
        Self::uniform_buffer(0, ShaderStageFlags::ALL)
    }
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DescriptorType {
    /// Sampler only
    Sampler              = 0,
    /// Combined image and sampler
    #[default]
    CombinedImageSampler = 1,
    /// Sampled image (texture)
    SampledImage         = 2,
    /// Storage image (UAV)
    StorageImage         = 3,
    /// Uniform texel buffer
    UniformTexelBuffer   = 4,
    /// Storage texel buffer
    StorageTexelBuffer   = 5,
    /// Uniform buffer
    UniformBuffer        = 6,
    /// Storage buffer (SSBO)
    StorageBuffer        = 7,
    /// Dynamic uniform buffer
    UniformBufferDynamic = 8,
    /// Dynamic storage buffer
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment      = 10,
    /// Inline uniform block
    InlineUniformBlock   = 11,
    /// Acceleration structure
    AccelerationStructure = 12,
    /// Mutable descriptor
    MutableDescriptor    = 13,
}

impl DescriptorType {
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
            Self::MutableDescriptor => "Mutable Descriptor",
        }
    }

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
        matches!(
            self,
            Self::UniformBufferDynamic | Self::StorageBufferDynamic
        )
    }
}

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// No stages
    pub const NONE: Self = Self(0);
    /// Vertex shader
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation
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

    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(
        Self::VERTEX.0
            | Self::TESSELLATION_CONTROL.0
            | Self::TESSELLATION_EVALUATION.0
            | Self::GEOMETRY.0
            | Self::FRAGMENT.0,
    );

    /// Vertex + Fragment
    pub const VERTEX_FRAGMENT: Self = Self(Self::VERTEX.0 | Self::FRAGMENT.0);

    /// All ray tracing
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

    /// Contains flag
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

    /// Bindless flags
    pub const BINDLESS: Self = Self(
        Self::UPDATE_AFTER_BIND.0 | Self::PARTIALLY_BOUND.0 | Self::VARIABLE_DESCRIPTOR_COUNT.0,
    );

    /// Contains flag
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
// Descriptor Set Layout Config
// ============================================================================

/// Descriptor set layout configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DescriptorSetLayoutConfig {
    /// Bindings
    pub bindings: [Option<DescriptorSetLayoutBinding>; 32],
    /// Number of bindings
    pub binding_count: u32,
    /// Layout flags
    pub flags: DescriptorSetLayoutFlags,
}

impl DescriptorSetLayoutConfig {
    /// Creates empty layout config
    #[inline]
    pub const fn empty() -> Self {
        Self {
            bindings: [None; 32],
            binding_count: 0,
            flags: DescriptorSetLayoutFlags::NONE,
        }
    }

    /// Creates layout with single binding
    #[inline]
    pub const fn single(binding: DescriptorSetLayoutBinding) -> Self {
        let mut bindings = [None; 32];
        bindings[0] = Some(binding);
        Self {
            bindings,
            binding_count: 1,
            flags: DescriptorSetLayoutFlags::NONE,
        }
    }

    /// Adds binding
    pub fn add_binding(&mut self, binding: DescriptorSetLayoutBinding) -> &mut Self {
        if self.binding_count < 32 {
            self.bindings[self.binding_count as usize] = Some(binding);
            self.binding_count += 1;
        }
        self
    }

    /// With layout flags
    #[inline]
    pub const fn with_flags(mut self, flags: DescriptorSetLayoutFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Update after bind pool
    #[inline]
    pub const fn update_after_bind(mut self) -> Self {
        self.flags = DescriptorSetLayoutFlags::UPDATE_AFTER_BIND_POOL;
        self
    }

    /// Push descriptor
    #[inline]
    pub const fn push_descriptor(mut self) -> Self {
        self.flags = DescriptorSetLayoutFlags::PUSH_DESCRIPTOR;
        self
    }
}

impl Default for DescriptorSetLayoutConfig {
    fn default() -> Self {
        Self::empty()
    }
}

/// Descriptor set layout flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorSetLayoutFlags(pub u32);

impl DescriptorSetLayoutFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Update after bind pool
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 0);
    /// Push descriptor
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 1);
    /// Descriptor buffer (for buffer device address)
    pub const DESCRIPTOR_BUFFER: Self = Self(1 << 2);
    /// Embedded immutable samplers (descriptor buffer)
    pub const EMBEDDED_IMMUTABLE_SAMPLERS: Self = Self(1 << 3);

    /// Contains flag
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Pipeline Layout
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

/// Pipeline layout configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PipelineLayoutConfig {
    /// Set layouts
    pub set_layouts: [DescriptorSetLayoutHandle; 8],
    /// Number of set layouts
    pub set_layout_count: u32,
    /// Push constant ranges
    pub push_constant_ranges: [PushConstantRange; 8],
    /// Number of push constant ranges
    pub push_constant_range_count: u32,
}

impl PipelineLayoutConfig {
    /// Creates empty layout config
    #[inline]
    pub const fn empty() -> Self {
        Self {
            set_layouts: [DescriptorSetLayoutHandle::NULL; 8],
            set_layout_count: 0,
            push_constant_ranges: [PushConstantRange::empty(); 8],
            push_constant_range_count: 0,
        }
    }

    /// Creates with single set layout
    #[inline]
    pub const fn single(layout: DescriptorSetLayoutHandle) -> Self {
        let mut layouts = [DescriptorSetLayoutHandle::NULL; 8];
        layouts[0] = layout;
        Self {
            set_layouts: layouts,
            set_layout_count: 1,
            push_constant_ranges: [PushConstantRange::empty(); 8],
            push_constant_range_count: 0,
        }
    }

    /// Adds set layout
    pub fn add_set_layout(&mut self, layout: DescriptorSetLayoutHandle) -> &mut Self {
        if self.set_layout_count < 8 {
            self.set_layouts[self.set_layout_count as usize] = layout;
            self.set_layout_count += 1;
        }
        self
    }

    /// Adds push constant range
    pub fn add_push_constant(&mut self, range: PushConstantRange) -> &mut Self {
        if self.push_constant_range_count < 8 {
            self.push_constant_ranges[self.push_constant_range_count as usize] = range;
            self.push_constant_range_count += 1;
        }
        self
    }
}

impl Default for PipelineLayoutConfig {
    fn default() -> Self {
        Self::empty()
    }
}

/// Push constant range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRange {
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates empty range
    #[inline]
    pub const fn empty() -> Self {
        Self {
            stage_flags: ShaderStageFlags::NONE,
            offset: 0,
            size: 0,
        }
    }

    /// Creates new range
    #[inline]
    pub const fn new(stage_flags: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stage_flags,
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

    /// Vertex + Fragment
    #[inline]
    pub const fn vertex_fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::VERTEX_FRAGMENT, offset, size)
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
}

impl Default for PushConstantRange {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Descriptor Write
// ============================================================================

/// Descriptor write operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorWrite {
    /// Destination set
    pub dst_set: u64,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
}

impl DescriptorWrite {
    /// Creates new write operation
    #[inline]
    pub const fn new(set: u64, binding: u32, descriptor_type: DescriptorType) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type,
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

impl Default for DescriptorWrite {
    fn default() -> Self {
        Self::new(0, 0, DescriptorType::UniformBuffer)
    }
}

/// Descriptor copy operation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorCopy {
    /// Source set
    pub src_set: u64,
    /// Source binding
    pub src_binding: u32,
    /// Source array element
    pub src_array_element: u32,
    /// Destination set
    pub dst_set: u64,
    /// Destination binding
    pub dst_binding: u32,
    /// Destination array element
    pub dst_array_element: u32,
    /// Descriptor count
    pub descriptor_count: u32,
}

impl DescriptorCopy {
    /// Creates new copy operation
    #[inline]
    pub const fn new(src_set: u64, src_binding: u32, dst_set: u64, dst_binding: u32) -> Self {
        Self {
            src_set,
            src_binding,
            src_array_element: 0,
            dst_set,
            dst_binding,
            dst_array_element: 0,
            descriptor_count: 1,
        }
    }

    /// With count
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.descriptor_count = count;
        self
    }
}

impl Default for DescriptorCopy {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}
