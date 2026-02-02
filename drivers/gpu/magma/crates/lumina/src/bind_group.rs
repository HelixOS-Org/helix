//! Bind group management
//!
//! Bind groups organize shader resources into sets that can be
//! bound together during command recording.

use crate::types::{BufferHandle, TextureHandle, SamplerHandle};

/// A set of bindings for shader resources
#[derive(Clone, Debug)]
pub struct BindGroup<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Layout used to create this bind group
    pub layout: BindGroupLayoutHandle,
    /// Entries in this bind group
    pub entries: &'a [BindGroupEntry<'a>],
}

/// Handle to a bind group layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupLayoutHandle {
    id: u32,
    generation: u32,
}

impl BindGroupLayoutHandle {
    /// Creates a new handle
    pub(crate) const fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }

    /// Returns the raw ID
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Null handle
    pub const fn null() -> Self {
        Self::new(u32::MAX, 0)
    }

    /// Is this a null handle?
    pub const fn is_null(&self) -> bool {
        self.id == u32::MAX
    }
}

/// Handle to a bind group
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindGroupHandle {
    id: u32,
    generation: u32,
}

impl BindGroupHandle {
    /// Creates a new handle
    pub(crate) const fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }

    /// Returns the raw ID
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Null handle
    pub const fn null() -> Self {
        Self::new(u32::MAX, 0)
    }

    /// Is this a null handle?
    pub const fn is_null(&self) -> bool {
        self.id == u32::MAX
    }
}

/// A single entry in a bind group
#[derive(Clone, Debug)]
pub struct BindGroupEntry<'a> {
    /// Binding index
    pub binding: u32,
    /// Resource to bind
    pub resource: BindingResource<'a>,
}

impl<'a> BindGroupEntry<'a> {
    /// Creates a buffer binding
    pub const fn buffer(binding: u32, buffer: BufferHandle) -> Self {
        Self {
            binding,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset: 0,
                size: None,
            }),
        }
    }

    /// Creates a buffer binding with offset and size
    pub const fn buffer_range(binding: u32, buffer: BufferHandle, offset: u64, size: u64) -> Self {
        Self {
            binding,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset,
                size: Some(size),
            }),
        }
    }

    /// Creates a texture binding
    pub const fn texture(binding: u32, view: TextureViewHandle) -> Self {
        Self {
            binding,
            resource: BindingResource::TextureView(view),
        }
    }

    /// Creates a sampler binding
    pub const fn sampler(binding: u32, sampler: SamplerHandle) -> Self {
        Self {
            binding,
            resource: BindingResource::Sampler(sampler),
        }
    }

    /// Creates a texture array binding
    pub fn texture_array(binding: u32, views: &'a [TextureViewHandle]) -> Self {
        Self {
            binding,
            resource: BindingResource::TextureViewArray(views),
        }
    }

    /// Creates a sampler array binding
    pub fn sampler_array(binding: u32, samplers: &'a [SamplerHandle]) -> Self {
        Self {
            binding,
            resource: BindingResource::SamplerArray(samplers),
        }
    }
}

/// Resource to bind
#[derive(Clone, Debug)]
pub enum BindingResource<'a> {
    /// Buffer binding
    Buffer(BufferBinding),
    /// Buffer array binding
    BufferArray(&'a [BufferBinding]),
    /// Texture view binding
    TextureView(TextureViewHandle),
    /// Texture view array binding
    TextureViewArray(&'a [TextureViewHandle]),
    /// Sampler binding
    Sampler(SamplerHandle),
    /// Sampler array binding
    SamplerArray(&'a [SamplerHandle]),
}

/// Buffer binding parameters
#[derive(Clone, Copy, Debug)]
pub struct BufferBinding {
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Offset in the buffer
    pub offset: u64,
    /// Size of the binding (None = whole buffer)
    pub size: Option<u64>,
}

/// Handle to a texture view
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureViewHandle {
    id: u32,
    generation: u32,
}

impl TextureViewHandle {
    /// Creates a new handle
    pub(crate) const fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }

    /// Returns the raw ID
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Null handle
    pub const fn null() -> Self {
        Self::new(u32::MAX, 0)
    }

    /// Is this a null handle?
    pub const fn is_null(&self) -> bool {
        self.id == u32::MAX
    }
}

/// Texture view descriptor
#[derive(Clone, Debug)]
pub struct TextureViewDesc<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Source texture
    pub texture: TextureHandle,
    /// View format (None = same as texture)
    pub format: Option<crate::compute::TextureFormat>,
    /// View dimension
    pub dimension: TextureViewDimension,
    /// Base mip level
    pub base_mip_level: u32,
    /// Mip level count (None = all remaining)
    pub mip_level_count: Option<u32>,
    /// Base array layer
    pub base_array_layer: u32,
    /// Array layer count (None = all remaining)
    pub array_layer_count: Option<u32>,
    /// Aspect (color, depth, stencil)
    pub aspect: TextureAspect,
}

impl<'a> TextureViewDesc<'a> {
    /// Creates a simple 2D view of the full texture
    pub const fn d2(texture: TextureHandle) -> Self {
        Self {
            label: None,
            texture,
            format: None,
            dimension: TextureViewDimension::D2,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            aspect: TextureAspect::All,
        }
    }

    /// Creates a cube view
    pub const fn cube(texture: TextureHandle) -> Self {
        Self {
            label: None,
            texture,
            format: None,
            dimension: TextureViewDimension::Cube,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(6),
            aspect: TextureAspect::All,
        }
    }

    /// Creates a depth-only view
    pub const fn depth_only(texture: TextureHandle) -> Self {
        Self {
            label: None,
            texture,
            format: None,
            dimension: TextureViewDimension::D2,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            aspect: TextureAspect::DepthOnly,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the mip range
    pub const fn with_mips(mut self, base: u32, count: u32) -> Self {
        self.base_mip_level = base;
        self.mip_level_count = Some(count);
        self
    }

    /// Sets the array range
    pub const fn with_layers(mut self, base: u32, count: u32) -> Self {
        self.base_array_layer = base;
        self.array_layer_count = Some(count);
        self
    }
}

/// Texture view dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureViewDimension {
    /// 1D texture view
    D1,
    /// 2D texture view
    D2,
    /// 2D array texture view
    D2Array,
    /// 3D texture view
    D3,
    /// Cube texture view
    Cube,
    /// Cube array texture view
    CubeArray,
}

/// Texture aspect for views
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureAspect {
    /// All aspects
    All,
    /// Stencil only
    StencilOnly,
    /// Depth only
    DepthOnly,
}

/// Pipeline layout descriptor
#[derive(Clone, Debug)]
pub struct PipelineLayoutDesc<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Bind group layouts
    pub bind_group_layouts: &'a [BindGroupLayoutHandle],
    /// Push constant ranges
    pub push_constant_ranges: &'a [PushConstantRange],
}

impl<'a> PipelineLayoutDesc<'a> {
    /// Creates an empty pipeline layout
    pub const fn empty() -> Self {
        Self {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        }
    }

    /// Creates a pipeline layout with bind groups
    pub const fn with_bind_groups(layouts: &'a [BindGroupLayoutHandle]) -> Self {
        Self {
            label: None,
            bind_group_layouts: layouts,
            push_constant_ranges: &[],
        }
    }

    /// Sets push constant ranges
    pub const fn with_push_constants(mut self, ranges: &'a [PushConstantRange]) -> Self {
        self.push_constant_ranges = ranges;
        self
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Push constant range
#[derive(Clone, Copy, Debug)]
pub struct PushConstantRange {
    /// Shader stages this range is visible to
    pub stages: crate::compute::ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

impl PushConstantRange {
    /// Creates a push constant range for all stages
    pub const fn all_stages(offset: u32, size: u32) -> Self {
        Self {
            stages: crate::compute::ShaderStageFlags::ALL,
            offset,
            size,
        }
    }

    /// Creates a push constant range for vertex stage only
    pub const fn vertex(offset: u32, size: u32) -> Self {
        Self {
            stages: crate::compute::ShaderStageFlags::VERTEX,
            offset,
            size,
        }
    }

    /// Creates a push constant range for fragment stage only
    pub const fn fragment(offset: u32, size: u32) -> Self {
        Self {
            stages: crate::compute::ShaderStageFlags::FRAGMENT,
            offset,
            size,
        }
    }

    /// Creates a push constant range for compute stage only
    pub const fn compute(offset: u32, size: u32) -> Self {
        Self {
            stages: crate::compute::ShaderStageFlags::COMPUTE,
            offset,
            size,
        }
    }
}
