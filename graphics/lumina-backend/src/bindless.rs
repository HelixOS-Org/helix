//! Bindless Resources
//!
//! GPU-driven rendering with bindless resource access.
//! Enables massive draw call reduction and GPU-driven pipelines.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    Bindless Architecture                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌──────────────────────────────────────────────────────────────┐  │
//! │  │                   Descriptor Heap                             │  │
//! │  │  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐    │  │
//! │  │  │ T0  │ T1  │ T2  │ ... │ Tn  │ S0  │ S1  │ ... │ Sm  │    │  │
//! │  │  └──┬──┴──┬──┴──┬──┴─────┴──┬──┴──┬──┴──┬──┴─────┴──┬──┘    │  │
//! │  │     │     │     │           │     │     │           │        │  │
//! │  │     ▼     ▼     ▼           ▼     ▼     ▼           ▼        │  │
//! │  │  [Tex0][Tex1][Tex2] ... [TexN][Buf0][Buf1] ... [BufM]        │  │
//! │  └──────────────────────────────────────────────────────────────┘  │
//! │                                                                     │
//! │  ┌────────────────────────────────────────────────────────────┐    │
//! │  │                  Material Data Buffer                       │    │
//! │  │  ┌──────────────────────────────────────────────────────┐  │    │
//! │  │  │ Material 0: albedo_idx, normal_idx, roughness, ...   │  │    │
//! │  │  │ Material 1: albedo_idx, normal_idx, roughness, ...   │  │    │
//! │  │  │ ...                                                   │  │    │
//! │  │  └──────────────────────────────────────────────────────┘  │    │
//! │  └────────────────────────────────────────────────────────────┘    │
//! │                                                                     │
//! │  Shader Access: texture(textures[material.albedo_idx], uv)         │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::buffer::BufferHandle;
use crate::sampler::SamplerHandle;
use crate::texture::{TextureHandle, TextureViewHandle};

// ============================================================================
// Descriptor Index Types
// ============================================================================

/// Descriptor index into heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorIndex(pub u32);

impl DescriptorIndex {
    /// Invalid index.
    pub const INVALID: Self = Self(u32::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }

    /// Get raw index.
    pub fn index(&self) -> u32 {
        self.0
    }
}

impl Default for DescriptorIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Texture descriptor index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureIndex(pub u32);

impl TextureIndex {
    /// Invalid index.
    pub const INVALID: Self = Self(u32::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl Default for TextureIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Buffer descriptor index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferIndex(pub u32);

impl BufferIndex {
    /// Invalid index.
    pub const INVALID: Self = Self(u32::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl Default for BufferIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Sampler descriptor index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerIndex(pub u32);

impl SamplerIndex {
    /// Invalid index.
    pub const INVALID: Self = Self(u32::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl Default for SamplerIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

// ============================================================================
// Descriptor Heap
// ============================================================================

/// Descriptor heap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorHeapType {
    /// Shader resource views (textures, buffers).
    ShaderResource,
    /// Samplers.
    Sampler,
    /// Unordered access views.
    UnorderedAccess,
}

/// Descriptor heap flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorHeapFlags(u32);

impl DescriptorHeapFlags {
    /// None.
    pub const NONE: Self = Self(0);
    /// Shader visible.
    pub const SHADER_VISIBLE: Self = Self(1 << 0);
    /// Allow updates.
    pub const ALLOW_UPDATES: Self = Self(1 << 1);

    /// Combine flags.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for DescriptorHeapFlags {
    fn default() -> Self {
        Self::SHADER_VISIBLE
    }
}

/// Descriptor heap description.
#[derive(Debug, Clone)]
pub struct DescriptorHeapDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Heap type.
    pub heap_type: DescriptorHeapType,
    /// Maximum descriptor count.
    pub max_descriptors: u32,
    /// Flags.
    pub flags: DescriptorHeapFlags,
}

impl Default for DescriptorHeapDesc {
    fn default() -> Self {
        Self {
            name: None,
            heap_type: DescriptorHeapType::ShaderResource,
            max_descriptors: 100000,
            flags: DescriptorHeapFlags::SHADER_VISIBLE,
        }
    }
}

/// Descriptor heap handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorHeapHandle(pub u64);

impl DescriptorHeapHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u64::MAX);

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl Default for DescriptorHeapHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

// ============================================================================
// Descriptor Entry
// ============================================================================

/// Descriptor entry type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorEntryType {
    /// Texture SRV.
    TextureSrv,
    /// Texture UAV.
    TextureUav,
    /// Buffer SRV.
    BufferSrv,
    /// Buffer UAV.
    BufferUav,
    /// Constant buffer.
    ConstantBuffer,
    /// Sampler.
    Sampler,
    /// Acceleration structure.
    AccelerationStructure,
}

/// Descriptor entry.
#[derive(Debug, Clone)]
pub struct DescriptorEntry {
    /// Index in heap.
    pub index: DescriptorIndex,
    /// Entry type.
    pub entry_type: DescriptorEntryType,
    /// Resource handle.
    pub resource: u64,
    /// View description (format, mip levels, etc.).
    pub view_info: u64,
    /// Is valid.
    pub valid: bool,
}

impl Default for DescriptorEntry {
    fn default() -> Self {
        Self {
            index: DescriptorIndex::INVALID,
            entry_type: DescriptorEntryType::TextureSrv,
            resource: 0,
            view_info: 0,
            valid: false,
        }
    }
}

// ============================================================================
// Bindless Heap
// ============================================================================

/// Bindless descriptor heap.
pub struct BindlessHeap {
    /// Handle.
    handle: DescriptorHeapHandle,
    /// Heap type.
    heap_type: DescriptorHeapType,
    /// Max descriptors.
    max_descriptors: u32,
    /// Entries.
    entries: Vec<DescriptorEntry>,
    /// Free list.
    free_list: Vec<u32>,
    /// Next free index.
    next_index: AtomicU32,
    /// Allocated count.
    allocated_count: AtomicU32,
}

impl BindlessHeap {
    /// Create new heap.
    pub fn new(desc: &DescriptorHeapDesc) -> Self {
        Self {
            handle: DescriptorHeapHandle(1), // Placeholder
            heap_type: desc.heap_type,
            max_descriptors: desc.max_descriptors,
            entries: Vec::with_capacity(desc.max_descriptors as usize),
            free_list: Vec::new(),
            next_index: AtomicU32::new(0),
            allocated_count: AtomicU32::new(0),
        }
    }

    /// Get handle.
    pub fn handle(&self) -> DescriptorHeapHandle {
        self.handle
    }

    /// Get heap type.
    pub fn heap_type(&self) -> DescriptorHeapType {
        self.heap_type
    }

    /// Get max descriptors.
    pub fn max_descriptors(&self) -> u32 {
        self.max_descriptors
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.allocated_count.load(Ordering::Relaxed)
    }

    /// Get free count.
    pub fn free_count(&self) -> u32 {
        self.max_descriptors - self.allocated_count()
    }

    /// Allocate descriptor.
    pub fn allocate(&mut self) -> DescriptorIndex {
        // Try free list first
        if let Some(idx) = self.free_list.pop() {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
            return DescriptorIndex(idx);
        }

        // Allocate new
        let idx = self.next_index.fetch_add(1, Ordering::Relaxed);
        if idx >= self.max_descriptors {
            return DescriptorIndex::INVALID;
        }

        self.allocated_count.fetch_add(1, Ordering::Relaxed);

        // Ensure entry exists
        while self.entries.len() <= idx as usize {
            self.entries.push(DescriptorEntry::default());
        }

        DescriptorIndex(idx)
    }

    /// Free descriptor.
    pub fn free(&mut self, index: DescriptorIndex) {
        if !index.is_valid() {
            return;
        }

        if let Some(entry) = self.entries.get_mut(index.0 as usize) {
            entry.valid = false;
            entry.resource = 0;
        }

        self.free_list.push(index.0);
        self.allocated_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Write texture descriptor.
    pub fn write_texture(
        &mut self,
        index: DescriptorIndex,
        texture: TextureViewHandle,
        is_uav: bool,
    ) {
        if !index.is_valid() {
            return;
        }

        let idx = index.0 as usize;
        if idx >= self.entries.len() {
            return;
        }

        self.entries[idx] = DescriptorEntry {
            index,
            entry_type: if is_uav {
                DescriptorEntryType::TextureUav
            } else {
                DescriptorEntryType::TextureSrv
            },
            resource: texture.0,
            view_info: 0,
            valid: true,
        };
    }

    /// Write buffer descriptor.
    pub fn write_buffer(
        &mut self,
        index: DescriptorIndex,
        buffer: BufferHandle,
        offset: u64,
        size: u64,
        is_uav: bool,
    ) {
        if !index.is_valid() {
            return;
        }

        let idx = index.0 as usize;
        if idx >= self.entries.len() {
            return;
        }

        self.entries[idx] = DescriptorEntry {
            index,
            entry_type: if is_uav {
                DescriptorEntryType::BufferUav
            } else {
                DescriptorEntryType::BufferSrv
            },
            resource: buffer.0,
            view_info: (offset << 32) | size,
            valid: true,
        };
    }

    /// Write sampler descriptor.
    pub fn write_sampler(&mut self, index: DescriptorIndex, sampler: SamplerHandle) {
        if !index.is_valid() {
            return;
        }

        let idx = index.0 as usize;
        if idx >= self.entries.len() {
            return;
        }

        self.entries[idx] = DescriptorEntry {
            index,
            entry_type: DescriptorEntryType::Sampler,
            resource: sampler.0,
            view_info: 0,
            valid: true,
        };
    }

    /// Get entry.
    pub fn get_entry(&self, index: DescriptorIndex) -> Option<&DescriptorEntry> {
        if !index.is_valid() {
            return None;
        }
        self.entries.get(index.0 as usize)
    }

    /// Check if entry is valid.
    pub fn is_valid(&self, index: DescriptorIndex) -> bool {
        self.get_entry(index).map(|e| e.valid).unwrap_or(false)
    }
}

// ============================================================================
// Bindless Resource Manager
// ============================================================================

/// Bindless texture slot.
#[derive(Debug, Clone)]
pub struct BindlessTexture {
    /// Index in heap.
    pub index: TextureIndex,
    /// Texture handle.
    pub texture: TextureHandle,
    /// View handle.
    pub view: TextureViewHandle,
    /// Resident (loaded).
    pub resident: bool,
    /// Last used frame.
    pub last_used_frame: u64,
}

/// Bindless buffer slot.
#[derive(Debug, Clone)]
pub struct BindlessBuffer {
    /// Index in heap.
    pub index: BufferIndex,
    /// Buffer handle.
    pub buffer: BufferHandle,
    /// Offset.
    pub offset: u64,
    /// Size.
    pub size: u64,
    /// Last used frame.
    pub last_used_frame: u64,
}

/// Bindless resource manager.
pub struct BindlessResourceManager {
    /// Shader resource heap.
    srv_heap: BindlessHeap,
    /// UAV heap.
    uav_heap: BindlessHeap,
    /// Sampler heap.
    sampler_heap: BindlessHeap,
    /// Textures.
    textures: Vec<BindlessTexture>,
    /// Buffers.
    buffers: Vec<BindlessBuffer>,
    /// Current frame.
    current_frame: u64,
    /// Statistics.
    total_textures: AtomicU32,
    total_buffers: AtomicU32,
    resident_textures: AtomicU32,
}

impl BindlessResourceManager {
    /// Create new manager.
    pub fn new(max_textures: u32, max_buffers: u32, max_samplers: u32) -> Self {
        let srv_desc = DescriptorHeapDesc {
            name: Some(String::from("Bindless SRV Heap")),
            heap_type: DescriptorHeapType::ShaderResource,
            max_descriptors: max_textures + max_buffers,
            flags: DescriptorHeapFlags::SHADER_VISIBLE,
        };

        let uav_desc = DescriptorHeapDesc {
            name: Some(String::from("Bindless UAV Heap")),
            heap_type: DescriptorHeapType::UnorderedAccess,
            max_descriptors: max_textures / 4 + max_buffers / 2,
            flags: DescriptorHeapFlags::SHADER_VISIBLE,
        };

        let sampler_desc = DescriptorHeapDesc {
            name: Some(String::from("Bindless Sampler Heap")),
            heap_type: DescriptorHeapType::Sampler,
            max_descriptors: max_samplers,
            flags: DescriptorHeapFlags::SHADER_VISIBLE,
        };

        Self {
            srv_heap: BindlessHeap::new(&srv_desc),
            uav_heap: BindlessHeap::new(&uav_desc),
            sampler_heap: BindlessHeap::new(&sampler_desc),
            textures: Vec::new(),
            buffers: Vec::new(),
            current_frame: 0,
            total_textures: AtomicU32::new(0),
            total_buffers: AtomicU32::new(0),
            resident_textures: AtomicU32::new(0),
        }
    }

    /// Begin frame.
    pub fn begin_frame(&mut self, frame: u64) {
        self.current_frame = frame;
    }

    /// Register texture.
    pub fn register_texture(
        &mut self,
        texture: TextureHandle,
        view: TextureViewHandle,
    ) -> TextureIndex {
        let desc_index = self.srv_heap.allocate();
        if !desc_index.is_valid() {
            return TextureIndex::INVALID;
        }

        self.srv_heap.write_texture(desc_index, view, false);

        let tex_index = TextureIndex(desc_index.0);

        self.textures.push(BindlessTexture {
            index: tex_index,
            texture,
            view,
            resident: true,
            last_used_frame: self.current_frame,
        });

        self.total_textures.fetch_add(1, Ordering::Relaxed);
        self.resident_textures.fetch_add(1, Ordering::Relaxed);

        tex_index
    }

    /// Unregister texture.
    pub fn unregister_texture(&mut self, index: TextureIndex) {
        if !index.is_valid() {
            return;
        }

        if let Some(pos) = self.textures.iter().position(|t| t.index == index) {
            let tex = self.textures.remove(pos);
            self.srv_heap.free(DescriptorIndex(tex.index.0));
            self.total_textures.fetch_sub(1, Ordering::Relaxed);
            if tex.resident {
                self.resident_textures.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Register buffer.
    pub fn register_buffer(&mut self, buffer: BufferHandle, offset: u64, size: u64) -> BufferIndex {
        let desc_index = self.srv_heap.allocate();
        if !desc_index.is_valid() {
            return BufferIndex::INVALID;
        }

        self.srv_heap
            .write_buffer(desc_index, buffer, offset, size, false);

        let buf_index = BufferIndex(desc_index.0);

        self.buffers.push(BindlessBuffer {
            index: buf_index,
            buffer,
            offset,
            size,
            last_used_frame: self.current_frame,
        });

        self.total_buffers.fetch_add(1, Ordering::Relaxed);

        buf_index
    }

    /// Unregister buffer.
    pub fn unregister_buffer(&mut self, index: BufferIndex) {
        if !index.is_valid() {
            return;
        }

        if let Some(pos) = self.buffers.iter().position(|b| b.index == index) {
            let buf = self.buffers.remove(pos);
            self.srv_heap.free(DescriptorIndex(buf.index.0));
            self.total_buffers.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Register sampler.
    pub fn register_sampler(&mut self, sampler: SamplerHandle) -> SamplerIndex {
        let desc_index = self.sampler_heap.allocate();
        if !desc_index.is_valid() {
            return SamplerIndex::INVALID;
        }

        self.sampler_heap.write_sampler(desc_index, sampler);

        SamplerIndex(desc_index.0)
    }

    /// Mark texture used.
    pub fn mark_texture_used(&mut self, index: TextureIndex) {
        if let Some(tex) = self.textures.iter_mut().find(|t| t.index == index) {
            tex.last_used_frame = self.current_frame;
        }
    }

    /// Mark buffer used.
    pub fn mark_buffer_used(&mut self, index: BufferIndex) {
        if let Some(buf) = self.buffers.iter_mut().find(|b| b.index == index) {
            buf.last_used_frame = self.current_frame;
        }
    }

    /// Get SRV heap handle.
    pub fn srv_heap_handle(&self) -> DescriptorHeapHandle {
        self.srv_heap.handle()
    }

    /// Get sampler heap handle.
    pub fn sampler_heap_handle(&self) -> DescriptorHeapHandle {
        self.sampler_heap.handle()
    }

    /// Get texture count.
    pub fn texture_count(&self) -> u32 {
        self.total_textures.load(Ordering::Relaxed)
    }

    /// Get buffer count.
    pub fn buffer_count(&self) -> u32 {
        self.total_buffers.load(Ordering::Relaxed)
    }

    /// Get resident texture count.
    pub fn resident_texture_count(&self) -> u32 {
        self.resident_textures.load(Ordering::Relaxed)
    }
}

impl Default for BindlessResourceManager {
    fn default() -> Self {
        Self::new(100000, 50000, 2048)
    }
}

// ============================================================================
// GPU-Driven Rendering
// ============================================================================

/// Instance data for GPU-driven rendering.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuInstance {
    /// World matrix.
    pub world_matrix: [[f32; 4]; 4],
    /// Material index.
    pub material_index: u32,
    /// Mesh index.
    pub mesh_index: u32,
    /// LOD level.
    pub lod_level: u32,
    /// Flags.
    pub flags: u32,
}

impl Default for GpuInstance {
    fn default() -> Self {
        Self {
            world_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            material_index: 0,
            mesh_index: 0,
            lod_level: 0,
            flags: 0,
        }
    }
}

/// Material data for bindless rendering.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuMaterial {
    /// Albedo texture index.
    pub albedo_index: u32,
    /// Normal texture index.
    pub normal_index: u32,
    /// Metallic-roughness texture index.
    pub metallic_roughness_index: u32,
    /// Emissive texture index.
    pub emissive_index: u32,
    /// Albedo factor.
    pub albedo_factor: [f32; 4],
    /// Metallic factor.
    pub metallic_factor: f32,
    /// Roughness factor.
    pub roughness_factor: f32,
    /// Emissive factor.
    pub emissive_factor: [f32; 3],
    /// Alpha cutoff.
    pub alpha_cutoff: f32,
}

impl Default for GpuMaterial {
    fn default() -> Self {
        Self {
            albedo_index: u32::MAX,
            normal_index: u32::MAX,
            metallic_roughness_index: u32::MAX,
            emissive_index: u32::MAX,
            albedo_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            emissive_factor: [0.0; 3],
            alpha_cutoff: 0.5,
        }
    }
}

/// Mesh LOD data.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuMeshLod {
    /// Vertex buffer index.
    pub vertex_buffer_index: u32,
    /// Index buffer index.
    pub index_buffer_index: u32,
    /// First index.
    pub first_index: u32,
    /// Index count.
    pub index_count: u32,
    /// Vertex offset.
    pub vertex_offset: i32,
    /// Screen size threshold.
    pub screen_size_threshold: f32,
    /// Padding.
    pub _padding: [u32; 2],
}

impl Default for GpuMeshLod {
    fn default() -> Self {
        Self {
            vertex_buffer_index: 0,
            index_buffer_index: 0,
            first_index: 0,
            index_count: 0,
            vertex_offset: 0,
            screen_size_threshold: 0.0,
            _padding: [0; 2],
        }
    }
}

/// Indirect draw command (compatible with Vulkan/D3D12).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IndirectDrawIndexedCommand {
    /// Index count.
    pub index_count: u32,
    /// Instance count.
    pub instance_count: u32,
    /// First index.
    pub first_index: u32,
    /// Vertex offset.
    pub vertex_offset: i32,
    /// First instance.
    pub first_instance: u32,
}

impl Default for IndirectDrawIndexedCommand {
    fn default() -> Self {
        Self {
            index_count: 0,
            instance_count: 1,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }
}

/// Indirect dispatch command.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IndirectDispatchCommand {
    /// Workgroup count X.
    pub x: u32,
    /// Workgroup count Y.
    pub y: u32,
    /// Workgroup count Z.
    pub z: u32,
}

impl Default for IndirectDispatchCommand {
    fn default() -> Self {
        Self { x: 1, y: 1, z: 1 }
    }
}

/// GPU-driven rendering features.
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuDrivenFeatures {
    /// Multi draw indirect.
    pub multi_draw_indirect: bool,
    /// Draw indirect count.
    pub draw_indirect_count: bool,
    /// First instance.
    pub first_instance: bool,
    /// Max draw count.
    pub max_draw_count: u32,
    /// Max draw indirect count.
    pub max_draw_indirect_count: u32,
}

// ============================================================================
// Bindless Features
// ============================================================================

/// Bindless resource features.
#[derive(Debug, Clone, Copy, Default)]
pub struct BindlessFeatures {
    /// Bindless textures.
    pub bindless_textures: bool,
    /// Bindless buffers.
    pub bindless_buffers: bool,
    /// Bindless samplers.
    pub bindless_samplers: bool,
    /// Update after bind.
    pub update_after_bind: bool,
    /// Partially bound.
    pub partially_bound: bool,
    /// Max texture array size.
    pub max_texture_array_size: u32,
    /// Max buffer array size.
    pub max_buffer_array_size: u32,
    /// Max sampler array size.
    pub max_sampler_array_size: u32,
    /// GPU-driven features.
    pub gpu_driven: GpuDrivenFeatures,
}
