//! Resource Binding Types for Lumina
//!
//! This module provides comprehensive resource binding infrastructure
//! for efficient GPU resource management and bindless rendering.

use alloc::vec::Vec;

// ============================================================================
// Bindless Resource System
// ============================================================================

/// Handle to a bindless resource heap
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceHeapHandle(pub u64);

impl ResourceHeapHandle {
    /// Null handle constant
    pub const NULL: Self = Self(0);

    /// Creates a new handle from raw value
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw handle value
    #[inline]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    /// Checks if this is a null handle
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for ResourceHeapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bindless resource index
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BindlessIndex(pub u32);

impl BindlessIndex {
    /// Invalid index
    pub const INVALID: Self = Self(u32::MAX);

    /// Creates a new index
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the raw index
    #[inline]
    pub const fn as_raw(&self) -> u32 {
        self.0
    }

    /// Checks if this is a valid index
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

// ============================================================================
// Resource Heap Configuration
// ============================================================================

/// Configuration for a bindless resource heap
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ResourceHeapConfig {
    /// Maximum number of CBVs (Constant Buffer Views)
    pub max_cbv: u32,
    /// Maximum number of SRVs (Shader Resource Views)
    pub max_srv: u32,
    /// Maximum number of UAVs (Unordered Access Views)
    pub max_uav: u32,
    /// Maximum number of samplers
    pub max_samplers: u32,
    /// Heap flags
    pub flags: ResourceHeapFlags,
}

impl ResourceHeapConfig {
    /// Creates a new heap config
    #[inline]
    pub const fn new() -> Self {
        Self {
            max_cbv: 1024,
            max_srv: 16384,
            max_uav: 1024,
            max_samplers: 2048,
            flags: ResourceHeapFlags::NONE,
        }
    }

    /// Small heap for simple applications
    #[inline]
    pub const fn small() -> Self {
        Self {
            max_cbv: 256,
            max_srv: 4096,
            max_uav: 256,
            max_samplers: 512,
            flags: ResourceHeapFlags::NONE,
        }
    }

    /// Large heap for complex scenes
    #[inline]
    pub const fn large() -> Self {
        Self {
            max_cbv: 4096,
            max_srv: 65536,
            max_uav: 4096,
            max_samplers: 4096,
            flags: ResourceHeapFlags::ALLOW_VARIABLE_COUNT,
        }
    }

    /// Heap optimized for bindless textures
    #[inline]
    pub const fn bindless_textures() -> Self {
        Self {
            max_cbv: 256,
            max_srv: 1000000, // 1 million textures
            max_uav: 256,
            max_samplers: 2048,
            flags: ResourceHeapFlags::ALLOW_VARIABLE_COUNT
                .union(ResourceHeapFlags::UPDATE_AFTER_BIND),
        }
    }

    /// Sets maximum CBVs
    #[inline]
    pub const fn with_cbv(mut self, count: u32) -> Self {
        self.max_cbv = count;
        self
    }

    /// Sets maximum SRVs
    #[inline]
    pub const fn with_srv(mut self, count: u32) -> Self {
        self.max_srv = count;
        self
    }

    /// Sets maximum UAVs
    #[inline]
    pub const fn with_uav(mut self, count: u32) -> Self {
        self.max_uav = count;
        self
    }

    /// Sets maximum samplers
    #[inline]
    pub const fn with_samplers(mut self, count: u32) -> Self {
        self.max_samplers = count;
        self
    }

    /// Enables update after bind
    #[inline]
    pub const fn with_update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(ResourceHeapFlags::UPDATE_AFTER_BIND);
        self
    }

    /// Enables partially bound descriptors
    #[inline]
    pub const fn with_partially_bound(mut self) -> Self {
        self.flags = self.flags.union(ResourceHeapFlags::PARTIALLY_BOUND);
        self
    }

    /// Returns total descriptor count
    #[inline]
    pub const fn total_descriptors(&self) -> u32 {
        self.max_cbv + self.max_srv + self.max_uav + self.max_samplers
    }
}

impl Default for ResourceHeapConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource heap flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ResourceHeapFlags(pub u32);

impl ResourceHeapFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Allow variable descriptor count
    pub const ALLOW_VARIABLE_COUNT: Self = Self(1 << 0);
    /// Allow updates after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 1);
    /// Allow partially bound descriptors
    pub const PARTIALLY_BOUND: Self = Self(1 << 2);
    /// Shader visible (can be bound to pipeline)
    pub const SHADER_VISIBLE: Self = Self(1 << 3);

    /// Bindless preset
    pub const BINDLESS: Self = Self(
        Self::ALLOW_VARIABLE_COUNT.0
            | Self::UPDATE_AFTER_BIND.0
            | Self::PARTIALLY_BOUND.0
            | Self::SHADER_VISIBLE.0,
    );

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Resource Binding
// ============================================================================

/// A binding of a resource to a slot
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ResourceBinding {
    /// Binding type
    pub binding_type: ResourceBindingType,
    /// Binding slot/register
    pub slot: u32,
    /// Binding space/set
    pub space: u32,
    /// Array element (for arrayed bindings)
    pub array_element: u32,
    /// Resource handle
    pub resource: ResourceHandle,
    /// Binding flags
    pub flags: ResourceBindingFlags,
}

impl ResourceBinding {
    /// Creates a new resource binding
    #[inline]
    pub const fn new(
        binding_type: ResourceBindingType,
        slot: u32,
        resource: ResourceHandle,
    ) -> Self {
        Self {
            binding_type,
            slot,
            space: 0,
            array_element: 0,
            resource,
            flags: ResourceBindingFlags::NONE,
        }
    }

    /// Creates a CBV binding
    #[inline]
    pub const fn cbv(slot: u32, buffer: BufferHandle) -> Self {
        Self::new(
            ResourceBindingType::ConstantBuffer,
            slot,
            ResourceHandle::Buffer(buffer),
        )
    }

    /// Creates an SRV for a texture
    #[inline]
    pub const fn texture_srv(slot: u32, texture: TextureHandle) -> Self {
        Self::new(
            ResourceBindingType::ShaderResource,
            slot,
            ResourceHandle::Texture(texture),
        )
    }

    /// Creates an SRV for a buffer
    #[inline]
    pub const fn buffer_srv(slot: u32, buffer: BufferHandle) -> Self {
        Self::new(
            ResourceBindingType::ShaderResource,
            slot,
            ResourceHandle::Buffer(buffer),
        )
    }

    /// Creates a UAV for a texture
    #[inline]
    pub const fn texture_uav(slot: u32, texture: TextureHandle) -> Self {
        Self::new(
            ResourceBindingType::UnorderedAccess,
            slot,
            ResourceHandle::Texture(texture),
        )
    }

    /// Creates a UAV for a buffer
    #[inline]
    pub const fn buffer_uav(slot: u32, buffer: BufferHandle) -> Self {
        Self::new(
            ResourceBindingType::UnorderedAccess,
            slot,
            ResourceHandle::Buffer(buffer),
        )
    }

    /// Creates a sampler binding
    #[inline]
    pub const fn sampler(slot: u32, sampler: SamplerHandle) -> Self {
        Self::new(
            ResourceBindingType::Sampler,
            slot,
            ResourceHandle::Sampler(sampler),
        )
    }

    /// Creates an acceleration structure binding
    #[inline]
    pub const fn acceleration_structure(slot: u32, accel: AccelerationStructureHandle) -> Self {
        Self::new(
            ResourceBindingType::AccelerationStructure,
            slot,
            ResourceHandle::AccelerationStructure(accel),
        )
    }

    /// Sets the space/set
    #[inline]
    pub const fn in_space(mut self, space: u32) -> Self {
        self.space = space;
        self
    }

    /// Sets the array element
    #[inline]
    pub const fn at_element(mut self, element: u32) -> Self {
        self.array_element = element;
        self
    }

    /// Marks as read-only
    #[inline]
    pub const fn read_only(mut self) -> Self {
        self.flags = self.flags.union(ResourceBindingFlags::READ_ONLY);
        self
    }

    /// Marks as dynamic offset
    #[inline]
    pub const fn dynamic_offset(mut self) -> Self {
        self.flags = self.flags.union(ResourceBindingFlags::DYNAMIC_OFFSET);
        self
    }
}

/// Resource binding type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ResourceBindingType {
    /// Constant buffer view
    ConstantBuffer  = 0,
    /// Shader resource view
    ShaderResource  = 1,
    /// Unordered access view
    UnorderedAccess = 2,
    /// Sampler
    Sampler         = 3,
    /// Acceleration structure
    AccelerationStructure = 4,
    /// Input attachment
    InputAttachment = 5,
}

/// Resource handle union
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum ResourceHandle {
    /// Buffer resource
    Buffer(BufferHandle),
    /// Texture resource
    Texture(TextureHandle),
    /// Sampler
    Sampler(SamplerHandle),
    /// Acceleration structure
    AccelerationStructure(AccelerationStructureHandle),
}

/// Buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferHandle(pub u64);

impl BufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates from raw
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Checks if null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for BufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureHandle(pub u64);

impl TextureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates from raw
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Checks if null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for TextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sampler handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SamplerHandle(pub u64);

impl SamplerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates from raw
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Checks if null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for SamplerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Acceleration structure handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccelerationStructureHandle(pub u64);

impl AccelerationStructureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates from raw
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Checks if null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for AccelerationStructureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Resource binding flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ResourceBindingFlags(pub u32);

impl ResourceBindingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Read-only binding
    pub const READ_ONLY: Self = Self(1 << 0);
    /// Dynamic offset
    pub const DYNAMIC_OFFSET: Self = Self(1 << 1);
    /// Bindless
    pub const BINDLESS: Self = Self(1 << 2);
    /// Partially bound
    pub const PARTIALLY_BOUND: Self = Self(1 << 3);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Binding Layout
// ============================================================================

/// Binding layout description
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BindingLayoutDesc {
    /// Layout entries
    pub entries: Vec<BindingLayoutEntry>,
    /// Layout flags
    pub flags: BindingLayoutFlags,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantRange>,
}

impl BindingLayoutDesc {
    /// Creates a new empty layout
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            flags: BindingLayoutFlags::NONE,
            push_constants: Vec::new(),
        }
    }

    /// Adds a binding entry
    #[inline]
    pub fn add_binding(mut self, entry: BindingLayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Adds a CBV
    #[inline]
    pub fn add_cbv(self, slot: u32, visibility: ShaderVisibility) -> Self {
        self.add_binding(BindingLayoutEntry::cbv(slot, visibility))
    }

    /// Adds an SRV
    #[inline]
    pub fn add_srv(self, slot: u32, visibility: ShaderVisibility) -> Self {
        self.add_binding(BindingLayoutEntry::srv(slot, visibility))
    }

    /// Adds a UAV
    #[inline]
    pub fn add_uav(self, slot: u32, visibility: ShaderVisibility) -> Self {
        self.add_binding(BindingLayoutEntry::uav(slot, visibility))
    }

    /// Adds a sampler
    #[inline]
    pub fn add_sampler(self, slot: u32, visibility: ShaderVisibility) -> Self {
        self.add_binding(BindingLayoutEntry::sampler(slot, visibility))
    }

    /// Adds a bindless texture array
    #[inline]
    pub fn add_bindless_textures(self, slot: u32, max_count: u32) -> Self {
        self.add_binding(BindingLayoutEntry::bindless_textures(slot, max_count))
    }

    /// Adds push constants
    #[inline]
    pub fn add_push_constants(mut self, range: PushConstantRange) -> Self {
        self.push_constants.push(range);
        self
    }

    /// Enables update after bind
    #[inline]
    pub fn with_update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(BindingLayoutFlags::UPDATE_AFTER_BIND);
        self
    }
}

impl Default for BindingLayoutDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Binding layout entry
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BindingLayoutEntry {
    /// Binding slot
    pub slot: u32,
    /// Binding type
    pub binding_type: ResourceBindingType,
    /// Descriptor count
    pub count: u32,
    /// Shader visibility
    pub visibility: ShaderVisibility,
    /// Entry flags
    pub flags: BindingEntryFlags,
}

impl BindingLayoutEntry {
    /// Creates a new entry
    #[inline]
    pub const fn new(
        slot: u32,
        binding_type: ResourceBindingType,
        visibility: ShaderVisibility,
    ) -> Self {
        Self {
            slot,
            binding_type,
            count: 1,
            visibility,
            flags: BindingEntryFlags::NONE,
        }
    }

    /// Creates a CBV entry
    #[inline]
    pub const fn cbv(slot: u32, visibility: ShaderVisibility) -> Self {
        Self::new(slot, ResourceBindingType::ConstantBuffer, visibility)
    }

    /// Creates an SRV entry
    #[inline]
    pub const fn srv(slot: u32, visibility: ShaderVisibility) -> Self {
        Self::new(slot, ResourceBindingType::ShaderResource, visibility)
    }

    /// Creates a UAV entry
    #[inline]
    pub const fn uav(slot: u32, visibility: ShaderVisibility) -> Self {
        Self::new(slot, ResourceBindingType::UnorderedAccess, visibility)
    }

    /// Creates a sampler entry
    #[inline]
    pub const fn sampler(slot: u32, visibility: ShaderVisibility) -> Self {
        Self::new(slot, ResourceBindingType::Sampler, visibility)
    }

    /// Creates an acceleration structure entry
    #[inline]
    pub const fn acceleration_structure(slot: u32, visibility: ShaderVisibility) -> Self {
        Self::new(slot, ResourceBindingType::AccelerationStructure, visibility)
    }

    /// Creates a bindless texture array
    #[inline]
    pub fn bindless_textures(slot: u32, max_count: u32) -> Self {
        Self {
            slot,
            binding_type: ResourceBindingType::ShaderResource,
            count: max_count,
            visibility: ShaderVisibility::All,
            flags: BindingEntryFlags::VARIABLE_COUNT
                .union(BindingEntryFlags::UPDATE_AFTER_BIND)
                .union(BindingEntryFlags::PARTIALLY_BOUND),
        }
    }

    /// Sets the count (for arrays)
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// Marks as partially bound
    #[inline]
    pub fn partially_bound(mut self) -> Self {
        self.flags = self.flags.union(BindingEntryFlags::PARTIALLY_BOUND);
        self
    }

    /// Marks as update after bind
    #[inline]
    pub fn update_after_bind(mut self) -> Self {
        self.flags = self.flags.union(BindingEntryFlags::UPDATE_AFTER_BIND);
        self
    }
}

/// Shader visibility
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderVisibility {
    /// All shader stages
    #[default]
    All                 = 0,
    /// Vertex shader only
    Vertex              = 1,
    /// Fragment/pixel shader only
    Fragment            = 2,
    /// Compute shader only
    Compute             = 3,
    /// Geometry shader only
    Geometry            = 4,
    /// Tessellation control
    TessellationControl = 5,
    /// Tessellation evaluation
    TessellationEvaluation = 6,
    /// Task shader
    Task                = 7,
    /// Mesh shader
    Mesh                = 8,
    /// Ray generation shader
    RayGeneration       = 9,
    /// Any-hit shader
    AnyHit              = 10,
    /// Closest-hit shader
    ClosestHit          = 11,
    /// Miss shader
    Miss                = 12,
    /// Intersection shader
    Intersection        = 13,
    /// Callable shader
    Callable            = 14,
}

impl ShaderVisibility {
    /// Graphics stages
    pub const GRAPHICS: &'static [Self] = &[
        Self::Vertex,
        Self::Fragment,
        Self::Geometry,
        Self::TessellationControl,
        Self::TessellationEvaluation,
    ];

    /// Ray tracing stages
    pub const RAY_TRACING: &'static [Self] = &[
        Self::RayGeneration,
        Self::AnyHit,
        Self::ClosestHit,
        Self::Miss,
        Self::Intersection,
        Self::Callable,
    ];
}

/// Binding layout flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BindingLayoutFlags(pub u32);

impl BindingLayoutFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Push descriptor layout
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 1);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Binding entry flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BindingEntryFlags(pub u32);

impl BindingEntryFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Update after bind
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Update unused while pending
    pub const UPDATE_UNUSED_WHILE_PENDING: Self = Self(1 << 1);
    /// Partially bound
    pub const PARTIALLY_BOUND: Self = Self(1 << 2);
    /// Variable count
    pub const VARIABLE_COUNT: Self = Self(1 << 3);

    /// Bindless preset
    pub const BINDLESS: Self = Self(
        Self::UPDATE_AFTER_BIND.0
            | Self::UPDATE_UNUSED_WHILE_PENDING.0
            | Self::PARTIALLY_BOUND.0
            | Self::VARIABLE_COUNT.0,
    );

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Push constant range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRange {
    /// Shader visibility
    pub visibility: ShaderVisibility,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates a new push constant range
    #[inline]
    pub const fn new(visibility: ShaderVisibility, offset: u32, size: u32) -> Self {
        Self {
            visibility,
            offset,
            size,
        }
    }

    /// Creates for all stages
    #[inline]
    pub const fn all(size: u32) -> Self {
        Self::new(ShaderVisibility::All, 0, size)
    }

    /// Creates for vertex shader
    #[inline]
    pub const fn vertex(offset: u32, size: u32) -> Self {
        Self::new(ShaderVisibility::Vertex, offset, size)
    }

    /// Creates for fragment shader
    #[inline]
    pub const fn fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderVisibility::Fragment, offset, size)
    }

    /// Creates for compute shader
    #[inline]
    pub const fn compute(size: u32) -> Self {
        Self::new(ShaderVisibility::Compute, 0, size)
    }
}

// ============================================================================
// Binding Group
// ============================================================================

/// Binding group (a set of bindings)
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BindingGroup {
    /// Group index
    pub group: u32,
    /// Bindings in this group
    pub bindings: Vec<ResourceBinding>,
}

impl BindingGroup {
    /// Creates a new binding group
    #[inline]
    pub fn new(group: u32) -> Self {
        Self {
            group,
            bindings: Vec::new(),
        }
    }

    /// Adds a binding
    #[inline]
    pub fn add(mut self, binding: ResourceBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds a CBV
    #[inline]
    pub fn add_cbv(self, slot: u32, buffer: BufferHandle) -> Self {
        self.add(ResourceBinding::cbv(slot, buffer))
    }

    /// Adds a texture SRV
    #[inline]
    pub fn add_texture(self, slot: u32, texture: TextureHandle) -> Self {
        self.add(ResourceBinding::texture_srv(slot, texture))
    }

    /// Adds a buffer SRV
    #[inline]
    pub fn add_buffer(self, slot: u32, buffer: BufferHandle) -> Self {
        self.add(ResourceBinding::buffer_srv(slot, buffer))
    }

    /// Adds a sampler
    #[inline]
    pub fn add_sampler(self, slot: u32, sampler: SamplerHandle) -> Self {
        self.add(ResourceBinding::sampler(slot, sampler))
    }

    /// Returns the number of bindings
    #[inline]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Checks if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

// ============================================================================
// Root Signature (for D3D12-style binding)
// ============================================================================

/// Root signature description
#[derive(Clone, Debug)]
#[repr(C)]
pub struct RootSignatureDesc {
    /// Root parameters
    pub parameters: Vec<RootParameter>,
    /// Static samplers
    pub static_samplers: Vec<StaticSamplerDesc>,
    /// Root signature flags
    pub flags: RootSignatureFlags,
}

impl RootSignatureDesc {
    /// Creates a new root signature
    #[inline]
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            static_samplers: Vec::new(),
            flags: RootSignatureFlags::NONE,
        }
    }

    /// Adds a root parameter
    #[inline]
    pub fn add_parameter(mut self, param: RootParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Adds a root constant
    #[inline]
    pub fn add_constants(
        self,
        slot: u32,
        space: u32,
        count: u32,
        visibility: ShaderVisibility,
    ) -> Self {
        self.add_parameter(RootParameter::constants(slot, space, count, visibility))
    }

    /// Adds a root CBV
    #[inline]
    pub fn add_cbv(self, slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        self.add_parameter(RootParameter::cbv(slot, space, visibility))
    }

    /// Adds a root SRV
    #[inline]
    pub fn add_srv(self, slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        self.add_parameter(RootParameter::srv(slot, space, visibility))
    }

    /// Adds a root UAV
    #[inline]
    pub fn add_uav(self, slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        self.add_parameter(RootParameter::uav(slot, space, visibility))
    }

    /// Adds a descriptor table
    #[inline]
    pub fn add_descriptor_table(
        self,
        ranges: Vec<DescriptorRange>,
        visibility: ShaderVisibility,
    ) -> Self {
        self.add_parameter(RootParameter::descriptor_table(ranges, visibility))
    }

    /// Adds a static sampler
    #[inline]
    pub fn add_static_sampler(mut self, sampler: StaticSamplerDesc) -> Self {
        self.static_samplers.push(sampler);
        self
    }

    /// Enables input assembler
    #[inline]
    pub fn with_input_assembler(mut self) -> Self {
        self.flags = self.flags.union(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER);
        self
    }
}

impl Default for RootSignatureDesc {
    fn default() -> Self {
        Self::new()
    }
}

/// Root parameter
#[derive(Clone, Debug)]
#[repr(C)]
pub struct RootParameter {
    /// Parameter type
    pub param_type: RootParameterType,
    /// Shader visibility
    pub visibility: ShaderVisibility,
    /// Parameter data
    pub data: RootParameterData,
}

impl RootParameter {
    /// Creates root constants
    #[inline]
    pub fn constants(slot: u32, space: u32, count: u32, visibility: ShaderVisibility) -> Self {
        Self {
            param_type: RootParameterType::Constants,
            visibility,
            data: RootParameterData::Constants(RootConstants {
                slot,
                space,
                num_32bit_values: count,
            }),
        }
    }

    /// Creates a root CBV
    #[inline]
    pub fn cbv(slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            param_type: RootParameterType::Cbv,
            visibility,
            data: RootParameterData::Descriptor(RootDescriptor { slot, space }),
        }
    }

    /// Creates a root SRV
    #[inline]
    pub fn srv(slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            param_type: RootParameterType::Srv,
            visibility,
            data: RootParameterData::Descriptor(RootDescriptor { slot, space }),
        }
    }

    /// Creates a root UAV
    #[inline]
    pub fn uav(slot: u32, space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            param_type: RootParameterType::Uav,
            visibility,
            data: RootParameterData::Descriptor(RootDescriptor { slot, space }),
        }
    }

    /// Creates a descriptor table
    #[inline]
    pub fn descriptor_table(ranges: Vec<DescriptorRange>, visibility: ShaderVisibility) -> Self {
        Self {
            param_type: RootParameterType::DescriptorTable,
            visibility,
            data: RootParameterData::DescriptorTable(DescriptorTable { ranges }),
        }
    }
}

/// Root parameter type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RootParameterType {
    /// 32-bit constants
    Constants       = 0,
    /// CBV
    Cbv             = 1,
    /// SRV
    Srv             = 2,
    /// UAV
    Uav             = 3,
    /// Descriptor table
    DescriptorTable = 4,
}

/// Root parameter data
#[derive(Clone, Debug)]
#[repr(C)]
pub enum RootParameterData {
    /// Root constants
    Constants(RootConstants),
    /// Root descriptor
    Descriptor(RootDescriptor),
    /// Descriptor table
    DescriptorTable(DescriptorTable),
}

/// Root constants
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RootConstants {
    /// Shader register
    pub slot: u32,
    /// Register space
    pub space: u32,
    /// Number of 32-bit values
    pub num_32bit_values: u32,
}

/// Root descriptor
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RootDescriptor {
    /// Shader register
    pub slot: u32,
    /// Register space
    pub space: u32,
}

/// Descriptor table
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DescriptorTable {
    /// Descriptor ranges
    pub ranges: Vec<DescriptorRange>,
}

/// Descriptor range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorRange {
    /// Range type
    pub range_type: DescriptorRangeType,
    /// Number of descriptors
    pub count: u32,
    /// Base shader register
    pub base_slot: u32,
    /// Register space
    pub space: u32,
    /// Offset from table start
    pub offset: u32,
    /// Range flags
    pub flags: DescriptorRangeFlags,
}

impl DescriptorRange {
    /// Creates a new range
    #[inline]
    pub const fn new(
        range_type: DescriptorRangeType,
        count: u32,
        base_slot: u32,
        space: u32,
    ) -> Self {
        Self {
            range_type,
            count,
            base_slot,
            space,
            offset: 0xFFFFFFFF, // Append
            flags: DescriptorRangeFlags::NONE,
        }
    }

    /// Creates an SRV range
    #[inline]
    pub const fn srv(count: u32, slot: u32, space: u32) -> Self {
        Self::new(DescriptorRangeType::Srv, count, slot, space)
    }

    /// Creates a UAV range
    #[inline]
    pub const fn uav(count: u32, slot: u32, space: u32) -> Self {
        Self::new(DescriptorRangeType::Uav, count, slot, space)
    }

    /// Creates a CBV range
    #[inline]
    pub const fn cbv(count: u32, slot: u32, space: u32) -> Self {
        Self::new(DescriptorRangeType::Cbv, count, slot, space)
    }

    /// Creates a sampler range
    #[inline]
    pub const fn sampler(count: u32, slot: u32, space: u32) -> Self {
        Self::new(DescriptorRangeType::Sampler, count, slot, space)
    }

    /// Creates a bindless range
    #[inline]
    pub fn bindless(range_type: DescriptorRangeType, slot: u32, space: u32) -> Self {
        Self {
            range_type,
            count: u32::MAX, // Unbounded
            base_slot: slot,
            space,
            offset: 0xFFFFFFFF,
            flags: DescriptorRangeFlags::DESCRIPTORS_VOLATILE
                .union(DescriptorRangeFlags::DATA_STATIC_WHILE_SET_AT_EXECUTE),
        }
    }
}

/// Descriptor range type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DescriptorRangeType {
    /// Shader resource view
    Srv     = 0,
    /// Unordered access view
    Uav     = 1,
    /// Constant buffer view
    Cbv     = 2,
    /// Sampler
    Sampler = 3,
}

/// Descriptor range flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DescriptorRangeFlags(pub u32);

impl DescriptorRangeFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Descriptors are volatile
    pub const DESCRIPTORS_VOLATILE: Self = Self(1 << 0);
    /// Data is volatile
    pub const DATA_VOLATILE: Self = Self(1 << 1);
    /// Data is static while set at execute
    pub const DATA_STATIC_WHILE_SET_AT_EXECUTE: Self = Self(1 << 2);
    /// Data is static
    pub const DATA_STATIC: Self = Self(1 << 3);
    /// Descriptors are static keeping buffer bounds checks
    pub const DESCRIPTORS_STATIC_KEEPING_BUFFER_BOUNDS_CHECKS: Self = Self(1 << 4);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Static sampler description
#[derive(Clone, Debug)]
#[repr(C)]
pub struct StaticSamplerDesc {
    /// Filter mode
    pub filter: FilterMode,
    /// Address mode U
    pub address_u: AddressMode,
    /// Address mode V
    pub address_v: AddressMode,
    /// Address mode W
    pub address_w: AddressMode,
    /// Mip LOD bias
    pub mip_lod_bias: f32,
    /// Max anisotropy
    pub max_anisotropy: u32,
    /// Comparison function
    pub comparison_func: CompareFunc,
    /// Border color
    pub border_color: BorderColor,
    /// Min LOD
    pub min_lod: f32,
    /// Max LOD
    pub max_lod: f32,
    /// Shader register
    pub slot: u32,
    /// Register space
    pub space: u32,
    /// Shader visibility
    pub visibility: ShaderVisibility,
}

impl StaticSamplerDesc {
    /// Creates a point sampler
    #[inline]
    pub const fn point(slot: u32, space: u32) -> Self {
        Self {
            filter: FilterMode::Point,
            address_u: AddressMode::Wrap,
            address_v: AddressMode::Wrap,
            address_w: AddressMode::Wrap,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            comparison_func: CompareFunc::Never,
            border_color: BorderColor::TransparentBlack,
            min_lod: 0.0,
            max_lod: f32::MAX,
            slot,
            space,
            visibility: ShaderVisibility::All,
        }
    }

    /// Creates a linear sampler
    #[inline]
    pub const fn linear(slot: u32, space: u32) -> Self {
        Self {
            filter: FilterMode::Linear,
            address_u: AddressMode::Wrap,
            address_v: AddressMode::Wrap,
            address_w: AddressMode::Wrap,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            comparison_func: CompareFunc::Never,
            border_color: BorderColor::TransparentBlack,
            min_lod: 0.0,
            max_lod: f32::MAX,
            slot,
            space,
            visibility: ShaderVisibility::All,
        }
    }

    /// Creates an anisotropic sampler
    #[inline]
    pub const fn anisotropic(slot: u32, space: u32, max_aniso: u32) -> Self {
        Self {
            filter: FilterMode::Anisotropic,
            address_u: AddressMode::Wrap,
            address_v: AddressMode::Wrap,
            address_w: AddressMode::Wrap,
            mip_lod_bias: 0.0,
            max_anisotropy: max_aniso,
            comparison_func: CompareFunc::Never,
            border_color: BorderColor::TransparentBlack,
            min_lod: 0.0,
            max_lod: f32::MAX,
            slot,
            space,
            visibility: ShaderVisibility::All,
        }
    }
}

/// Filter mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FilterMode {
    /// Point filtering
    Point       = 0,
    /// Linear filtering
    #[default]
    Linear      = 1,
    /// Anisotropic filtering
    Anisotropic = 2,
}

/// Address mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AddressMode {
    /// Wrap/repeat
    #[default]
    Wrap       = 0,
    /// Mirror
    Mirror     = 1,
    /// Clamp to edge
    Clamp      = 2,
    /// Clamp to border
    Border     = 3,
    /// Mirror once then clamp
    MirrorOnce = 4,
}

/// Comparison function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompareFunc {
    /// Never pass
    #[default]
    Never        = 0,
    /// Pass if less
    Less         = 1,
    /// Pass if equal
    Equal        = 2,
    /// Pass if less or equal
    LessEqual    = 3,
    /// Pass if greater
    Greater      = 4,
    /// Pass if not equal
    NotEqual     = 5,
    /// Pass if greater or equal
    GreaterEqual = 6,
    /// Always pass
    Always       = 7,
}

/// Border color
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BorderColor {
    /// Transparent black
    #[default]
    TransparentBlack = 0,
    /// Opaque black
    OpaqueBlack      = 1,
    /// Opaque white
    OpaqueWhite      = 2,
}

/// Root signature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct RootSignatureFlags(pub u32);

impl RootSignatureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Allow input assembler
    pub const ALLOW_INPUT_ASSEMBLER: Self = Self(1 << 0);
    /// Deny vertex shader access
    pub const DENY_VERTEX_SHADER: Self = Self(1 << 1);
    /// Deny hull shader access
    pub const DENY_HULL_SHADER: Self = Self(1 << 2);
    /// Deny domain shader access
    pub const DENY_DOMAIN_SHADER: Self = Self(1 << 3);
    /// Deny geometry shader access
    pub const DENY_GEOMETRY_SHADER: Self = Self(1 << 4);
    /// Deny pixel shader access
    pub const DENY_PIXEL_SHADER: Self = Self(1 << 5);
    /// Allow stream output
    pub const ALLOW_STREAM_OUTPUT: Self = Self(1 << 6);
    /// Local root signature
    pub const LOCAL_ROOT_SIGNATURE: Self = Self(1 << 7);
    /// Deny amplification shader access
    pub const DENY_AMPLIFICATION_SHADER: Self = Self(1 << 8);
    /// Deny mesh shader access
    pub const DENY_MESH_SHADER: Self = Self(1 << 9);
    /// CBV/SRV/UAV heap directly indexed
    pub const CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED: Self = Self(1 << 10);
    /// Sampler heap directly indexed
    pub const SAMPLER_HEAP_DIRECTLY_INDEXED: Self = Self(1 << 11);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
