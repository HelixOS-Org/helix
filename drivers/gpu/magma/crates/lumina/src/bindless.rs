//! Bindless Resources for Lumina
//!
//! This module provides bindless/descriptor indexing infrastructure
//! for efficient GPU resource management without descriptor set switches.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Bindless Handles
// ============================================================================

/// Bindless heap handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessHeapHandle(pub u64);

impl BindlessHeapHandle {
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

impl Default for BindlessHeapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bindless index (descriptor index in the heap)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessIndex(pub u32);

impl BindlessIndex {
    /// Invalid index
    pub const INVALID: Self = Self(u32::MAX);

    /// Creates new index
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }

    /// Gets raw index
    #[inline]
    pub const fn index(&self) -> u32 {
        self.0
    }
}

impl Default for BindlessIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Typed bindless index
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypedBindlessIndex<T> {
    /// Index
    pub index: BindlessIndex,
    /// Phantom
    _marker: core::marker::PhantomData<T>,
}

impl<T> TypedBindlessIndex<T> {
    /// Invalid index
    pub const INVALID: Self = Self {
        index: BindlessIndex::INVALID,
        _marker: core::marker::PhantomData,
    };

    /// Creates new typed index
    pub const fn new(index: u32) -> Self {
        Self {
            index: BindlessIndex::new(index),
            _marker: core::marker::PhantomData,
        }
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.index.is_valid()
    }

    /// Gets raw index
    pub const fn raw(&self) -> u32 {
        self.index.0
    }
}

impl<T> Default for TypedBindlessIndex<T> {
    fn default() -> Self {
        Self::INVALID
    }
}

// ============================================================================
// Resource Types
// ============================================================================

/// Bindless texture index
pub type TextureIndex = TypedBindlessIndex<TextureMarker>;

/// Bindless sampler index
pub type SamplerIndex = TypedBindlessIndex<SamplerMarker>;

/// Bindless buffer index
pub type BufferIndex = TypedBindlessIndex<BufferMarker>;

/// Bindless storage image index
pub type StorageImageIndex = TypedBindlessIndex<StorageImageMarker>;

/// Marker types
pub struct TextureMarker;
pub struct SamplerMarker;
pub struct BufferMarker;
pub struct StorageImageMarker;

// ============================================================================
// Bindless Heap
// ============================================================================

/// Bindless heap create info
#[derive(Clone, Debug)]
pub struct BindlessHeapCreateInfo {
    /// Max textures
    pub max_textures: u32,
    /// Max samplers
    pub max_samplers: u32,
    /// Max buffers
    pub max_buffers: u32,
    /// Max storage images
    pub max_storage_images: u32,
    /// Debug label
    pub label: Option<&'static str>,
}

impl BindlessHeapCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            max_textures: 16384,
            max_samplers: 256,
            max_buffers: 4096,
            max_storage_images: 1024,
            label: None,
        }
    }

    /// Small heap
    pub fn small() -> Self {
        Self {
            max_textures: 4096,
            max_samplers: 128,
            max_buffers: 1024,
            max_storage_images: 256,
            label: None,
        }
    }

    /// Large heap
    pub fn large() -> Self {
        Self {
            max_textures: 65536,
            max_samplers: 1024,
            max_buffers: 16384,
            max_storage_images: 4096,
            label: None,
        }
    }

    /// With textures
    pub fn with_max_textures(mut self, count: u32) -> Self {
        self.max_textures = count;
        self
    }

    /// With samplers
    pub fn with_max_samplers(mut self, count: u32) -> Self {
        self.max_samplers = count;
        self
    }

    /// With buffers
    pub fn with_max_buffers(mut self, count: u32) -> Self {
        self.max_buffers = count;
        self
    }

    /// With storage images
    pub fn with_max_storage_images(mut self, count: u32) -> Self {
        self.max_storage_images = count;
        self
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Total descriptors
    pub fn total_descriptors(&self) -> u32 {
        self.max_textures + self.max_samplers + self.max_buffers + self.max_storage_images
    }
}

impl Default for BindlessHeapCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Bindless heap layout offsets
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BindlessHeapLayout {
    /// Texture array base offset
    pub texture_offset: u32,
    /// Sampler array base offset
    pub sampler_offset: u32,
    /// Buffer array base offset
    pub buffer_offset: u32,
    /// Storage image array base offset
    pub storage_image_offset: u32,
}

impl BindlessHeapLayout {
    /// Creates layout from heap info
    pub fn from_heap_info(info: &BindlessHeapCreateInfo) -> Self {
        Self {
            texture_offset: 0,
            sampler_offset: info.max_textures,
            buffer_offset: info.max_textures + info.max_samplers,
            storage_image_offset: info.max_textures + info.max_samplers + info.max_buffers,
        }
    }
}

// ============================================================================
// Resource Registration
// ============================================================================

/// Texture registration info
#[derive(Clone, Debug)]
pub struct RegisterTextureInfo {
    /// Texture handle
    pub texture: u64,
    /// View type
    pub view_type: TextureViewType,
    /// First mip
    pub first_mip: u32,
    /// Mip count
    pub mip_count: u32,
    /// First layer
    pub first_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl RegisterTextureInfo {
    /// Creates info
    pub fn new(texture: u64) -> Self {
        Self {
            texture,
            view_type: TextureViewType::View2D,
            first_mip: 0,
            mip_count: u32::MAX,
            first_layer: 0,
            layer_count: u32::MAX,
        }
    }

    /// Cube view
    pub fn cube(texture: u64) -> Self {
        Self {
            texture,
            view_type: TextureViewType::ViewCube,
            first_mip: 0,
            mip_count: u32::MAX,
            first_layer: 0,
            layer_count: 6,
        }
    }

    /// Array view
    pub fn array(texture: u64, layers: u32) -> Self {
        Self {
            texture,
            view_type: TextureViewType::View2DArray,
            first_mip: 0,
            mip_count: u32::MAX,
            first_layer: 0,
            layer_count: layers,
        }
    }

    /// With mip range
    pub fn with_mip_range(mut self, first: u32, count: u32) -> Self {
        self.first_mip = first;
        self.mip_count = count;
        self
    }

    /// With layer range
    pub fn with_layer_range(mut self, first: u32, count: u32) -> Self {
        self.first_layer = first;
        self.layer_count = count;
        self
    }
}

/// Texture view type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureViewType {
    /// 1D texture
    View1D        = 0,
    /// 2D texture
    #[default]
    View2D        = 1,
    /// 3D texture
    View3D        = 2,
    /// Cube texture
    ViewCube      = 3,
    /// 1D array
    View1DArray   = 4,
    /// 2D array
    View2DArray   = 5,
    /// Cube array
    ViewCubeArray = 6,
}

/// Buffer registration info
#[derive(Clone, Debug)]
pub struct RegisterBufferInfo {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size (0 = whole buffer)
    pub size: u64,
    /// Format (for typed buffers)
    pub format: BufferFormat,
}

impl RegisterBufferInfo {
    /// Creates info
    pub fn new(buffer: u64) -> Self {
        Self {
            buffer,
            offset: 0,
            size: 0,
            format: BufferFormat::Raw,
        }
    }

    /// With range
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Typed buffer
    pub fn typed(buffer: u64, format: BufferFormat) -> Self {
        Self {
            buffer,
            offset: 0,
            size: 0,
            format,
        }
    }
}

/// Buffer format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BufferFormat {
    /// Raw bytes
    #[default]
    Raw         = 0,
    /// R32 Float
    R32Float    = 1,
    /// RG32 Float
    Rg32Float   = 2,
    /// RGB32 Float
    Rgb32Float  = 3,
    /// RGBA32 Float
    Rgba32Float = 4,
    /// R32 Uint
    R32Uint     = 5,
    /// RG32 Uint
    Rg32Uint    = 6,
    /// RGB32 Uint
    Rgb32Uint   = 7,
    /// RGBA32 Uint
    Rgba32Uint  = 8,
    /// R32 Sint
    R32Sint     = 9,
}

impl BufferFormat {
    /// Element size
    pub const fn element_size(&self) -> u32 {
        match self {
            Self::Raw => 1,
            Self::R32Float | Self::R32Uint | Self::R32Sint => 4,
            Self::Rg32Float | Self::Rg32Uint => 8,
            Self::Rgb32Float | Self::Rgb32Uint => 12,
            Self::Rgba32Float | Self::Rgba32Uint => 16,
        }
    }
}

/// Storage image registration info
#[derive(Clone, Debug)]
pub struct RegisterStorageImageInfo {
    /// Texture handle
    pub texture: u64,
    /// Mip level
    pub mip: u32,
    /// First layer
    pub first_layer: u32,
    /// Layer count
    pub layer_count: u32,
    /// Format
    pub format: StorageImageFormat,
}

impl RegisterStorageImageInfo {
    /// Creates info
    pub fn new(texture: u64, format: StorageImageFormat) -> Self {
        Self {
            texture,
            mip: 0,
            first_layer: 0,
            layer_count: 1,
            format,
        }
    }

    /// With mip
    pub fn with_mip(mut self, mip: u32) -> Self {
        self.mip = mip;
        self
    }

    /// With layers
    pub fn with_layers(mut self, first: u32, count: u32) -> Self {
        self.first_layer = first;
        self.layer_count = count;
        self
    }
}

/// Storage image format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StorageImageFormat {
    /// RGBA8 unorm
    #[default]
    Rgba8Unorm  = 0,
    /// RGBA8 snorm
    Rgba8Snorm  = 1,
    /// RGBA16 float
    Rgba16Float = 2,
    /// RGBA32 float
    Rgba32Float = 3,
    /// RG32 float
    Rg32Float   = 4,
    /// R32 float
    R32Float    = 5,
    /// RGBA8 uint
    Rgba8Uint   = 6,
    /// RGBA16 uint
    Rgba16Uint  = 7,
    /// RGBA32 uint
    Rgba32Uint  = 8,
    /// R32 uint
    R32Uint     = 9,
    /// R32 sint
    R32Sint     = 10,
}

// ============================================================================
// Free List Allocator
// ============================================================================

/// Simple free list for bindless indices
#[derive(Clone, Debug)]
pub struct BindlessFreeList {
    /// Free indices
    free: Vec<u32>,
    /// Next index if free list empty
    next: u32,
    /// Maximum index
    max: u32,
}

impl BindlessFreeList {
    /// Creates free list
    pub fn new(max: u32) -> Self {
        Self {
            free: Vec::new(),
            next: 0,
            max,
        }
    }

    /// Allocates index
    pub fn allocate(&mut self) -> Option<u32> {
        if let Some(index) = self.free.pop() {
            Some(index)
        } else if self.next < self.max {
            let index = self.next;
            self.next += 1;
            Some(index)
        } else {
            None
        }
    }

    /// Frees index
    pub fn free(&mut self, index: u32) {
        if index < self.max {
            self.free.push(index);
        }
    }

    /// Available count
    pub fn available(&self) -> u32 {
        self.free.len() as u32 + (self.max - self.next)
    }

    /// Used count
    pub fn used(&self) -> u32 {
        self.next - self.free.len() as u32
    }

    /// Is full
    pub fn is_full(&self) -> bool {
        self.free.is_empty() && self.next >= self.max
    }

    /// Reset
    pub fn reset(&mut self) {
        self.free.clear();
        self.next = 0;
    }
}

impl Default for BindlessFreeList {
    fn default() -> Self {
        Self::new(16384)
    }
}

// ============================================================================
// Material Bindless Data
// ============================================================================

/// Material GPU data (bindless)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MaterialBindlessData {
    /// Albedo texture index
    pub albedo_texture: u32,
    /// Normal texture index
    pub normal_texture: u32,
    /// Metallic/roughness texture index
    pub metallic_roughness_texture: u32,
    /// Occlusion texture index
    pub occlusion_texture: u32,
    /// Emissive texture index
    pub emissive_texture: u32,
    /// Sampler index
    pub sampler: u32,
    /// Base color
    pub base_color: [f32; 4],
    /// Metallic factor
    pub metallic: f32,
    /// Roughness factor
    pub roughness: f32,
    /// Emissive factor
    pub emissive: [f32; 3],
    /// Alpha cutoff
    pub alpha_cutoff: f32,
}

impl MaterialBindlessData {
    /// Creates data
    pub fn new() -> Self {
        Self {
            albedo_texture: u32::MAX,
            normal_texture: u32::MAX,
            metallic_roughness_texture: u32::MAX,
            occlusion_texture: u32::MAX,
            emissive_texture: u32::MAX,
            sampler: 0,
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 1.0,
            roughness: 1.0,
            emissive: [0.0, 0.0, 0.0],
            alpha_cutoff: 0.5,
        }
    }

    /// With albedo
    pub fn with_albedo(mut self, index: TextureIndex) -> Self {
        self.albedo_texture = index.raw();
        self
    }

    /// With normal
    pub fn with_normal(mut self, index: TextureIndex) -> Self {
        self.normal_texture = index.raw();
        self
    }

    /// With metallic roughness
    pub fn with_metallic_roughness(mut self, index: TextureIndex) -> Self {
        self.metallic_roughness_texture = index.raw();
        self
    }

    /// With base color
    pub fn with_base_color(mut self, color: [f32; 4]) -> Self {
        self.base_color = color;
        self
    }

    /// With metallic
    pub fn with_metallic(mut self, value: f32) -> Self {
        self.metallic = value;
        self
    }

    /// With roughness
    pub fn with_roughness(mut self, value: f32) -> Self {
        self.roughness = value;
        self
    }
}

/// Instance bindless data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct InstanceBindlessData {
    /// Model matrix
    pub model_matrix: [[f32; 4]; 4],
    /// Material index
    pub material_index: u32,
    /// Mesh index
    pub mesh_index: u32,
    /// Flags
    pub flags: u32,
    /// Custom data
    pub custom: u32,
}

impl InstanceBindlessData {
    /// Creates data
    pub fn new(model_matrix: [[f32; 4]; 4], material_index: u32, mesh_index: u32) -> Self {
        Self {
            model_matrix,
            material_index,
            mesh_index,
            flags: 0,
            custom: 0,
        }
    }

    /// With flags
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }
}

// ============================================================================
// Bindless Push Constants
// ============================================================================

/// Bindless push constants (common layout)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BindlessPushConstants {
    /// Draw ID (for gl_DrawID emulation)
    pub draw_id: u32,
    /// Material buffer index
    pub material_buffer: u32,
    /// Instance buffer index
    pub instance_buffer: u32,
    /// Custom data
    pub custom: u32,
}

impl BindlessPushConstants {
    /// Creates constants
    pub fn new(draw_id: u32) -> Self {
        Self {
            draw_id,
            material_buffer: 0,
            instance_buffer: 0,
            custom: 0,
        }
    }

    /// With material buffer
    pub fn with_material_buffer(mut self, index: BufferIndex) -> Self {
        self.material_buffer = index.raw();
        self
    }

    /// With instance buffer
    pub fn with_instance_buffer(mut self, index: BufferIndex) -> Self {
        self.instance_buffer = index.raw();
        self
    }
}

// ============================================================================
// Bindless Statistics
// ============================================================================

/// Bindless statistics
#[derive(Clone, Debug, Default)]
pub struct BindlessStats {
    /// Textures registered
    pub textures_registered: u32,
    /// Samplers registered
    pub samplers_registered: u32,
    /// Buffers registered
    pub buffers_registered: u32,
    /// Storage images registered
    pub storage_images_registered: u32,
    /// Descriptor updates this frame
    pub updates_this_frame: u32,
}

impl BindlessStats {
    /// Total registered
    pub fn total_registered(&self) -> u32 {
        self.textures_registered
            + self.samplers_registered
            + self.buffers_registered
            + self.storage_images_registered
    }
}
