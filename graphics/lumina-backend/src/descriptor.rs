//! Descriptor Sets and Layouts
//!
//! Manages shader resource bindings and descriptor management.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::buffer::BufferHandle;
use crate::sampler::SamplerHandle;
use crate::shader_module::ShaderStage;
use crate::texture::TextureViewHandle;

// ============================================================================
// Descriptor Type
// ============================================================================

/// Descriptor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    /// Sampler.
    Sampler,
    /// Combined image/sampler.
    CombinedImageSampler,
    /// Sampled image.
    SampledImage,
    /// Storage image.
    StorageImage,
    /// Uniform texel buffer.
    UniformTexelBuffer,
    /// Storage texel buffer.
    StorageTexelBuffer,
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer,
    /// Dynamic uniform buffer.
    UniformBufferDynamic,
    /// Dynamic storage buffer.
    StorageBufferDynamic,
    /// Input attachment.
    InputAttachment,
    /// Acceleration structure.
    AccelerationStructure,
}

// ============================================================================
// Descriptor Binding Flags
// ============================================================================

bitflags! {
    /// Descriptor binding flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DescriptorBindingFlags: u32 {
        /// Binding has variable count.
        const VARIABLE_DESCRIPTOR_COUNT = 1 << 0;
        /// Binding can be partially bound.
        const PARTIALLY_BOUND = 1 << 1;
        /// Update binding after bind.
        const UPDATE_AFTER_BIND = 1 << 2;
        /// Update unused while pending.
        const UPDATE_UNUSED_WHILE_PENDING = 1 << 3;
    }
}

// ============================================================================
// Descriptor Set Layout Binding
// ============================================================================

/// Descriptor set layout binding.
#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutBinding {
    /// Binding index.
    pub binding: u32,
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Descriptor count.
    pub count: u32,
    /// Shader stages.
    pub stage_flags: ShaderStage,
    /// Binding flags.
    pub binding_flags: DescriptorBindingFlags,
    /// Immutable samplers.
    pub immutable_samplers: Vec<SamplerHandle>,
}

impl DescriptorSetLayoutBinding {
    /// Create a new binding.
    pub fn new(binding: u32, descriptor_type: DescriptorType, stages: ShaderStage) -> Self {
        Self {
            binding,
            descriptor_type,
            count: 1,
            stage_flags: stages,
            binding_flags: DescriptorBindingFlags::empty(),
            immutable_samplers: Vec::new(),
        }
    }

    /// Set the count.
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// Set binding flags.
    pub fn with_flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.binding_flags = flags;
        self
    }

    /// Create a uniform buffer binding.
    pub fn uniform_buffer(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, stages)
    }

    /// Create a storage buffer binding.
    pub fn storage_buffer(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, stages)
    }

    /// Create a sampled texture binding.
    pub fn sampled_texture(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::SampledImage, stages)
    }

    /// Create a storage texture binding.
    pub fn storage_texture(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::StorageImage, stages)
    }

    /// Create a sampler binding.
    pub fn sampler(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::Sampler, stages)
    }

    /// Create a combined image/sampler binding.
    pub fn combined_image_sampler(binding: u32, stages: ShaderStage) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, stages)
    }
}

// ============================================================================
// Descriptor Set Layout Description
// ============================================================================

bitflags! {
    /// Descriptor set layout flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DescriptorSetLayoutFlags: u32 {
        /// Layout supports update after bind.
        const UPDATE_AFTER_BIND_POOL = 1 << 0;
        /// Push descriptor set layout.
        const PUSH_DESCRIPTOR = 1 << 1;
    }
}

/// Description for descriptor set layout creation.
#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutDesc {
    /// Bindings.
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    /// Flags.
    pub flags: DescriptorSetLayoutFlags,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for DescriptorSetLayoutDesc {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
            flags: DescriptorSetLayoutFlags::empty(),
            label: None,
        }
    }
}

impl DescriptorSetLayoutDesc {
    /// Create a new description.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding.
    pub fn with_binding(mut self, binding: DescriptorSetLayoutBinding) -> Self {
        self.bindings.push(binding);
        self
    }
}

// ============================================================================
// Descriptor Set Layout Handle
// ============================================================================

/// Handle to a descriptor set layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutHandle(Handle<DescriptorSetLayout>);

impl DescriptorSetLayoutHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Descriptor Set Layout
// ============================================================================

/// A descriptor set layout.
pub struct DescriptorSetLayout {
    /// Handle.
    pub handle: DescriptorSetLayoutHandle,
    /// Bindings.
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    /// Flags.
    pub flags: DescriptorSetLayoutFlags,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Pipeline Layout Description
// ============================================================================

/// Push constant range.
#[derive(Debug, Clone, Copy)]
pub struct PushConstantRange {
    /// Shader stages.
    pub stages: ShaderStage,
    /// Offset in bytes.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
}

/// Description for pipeline layout creation.
#[derive(Debug, Clone)]
pub struct PipelineLayoutDesc {
    /// Descriptor set layouts.
    pub set_layouts: Vec<DescriptorSetLayoutHandle>,
    /// Push constant ranges.
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for PipelineLayoutDesc {
    fn default() -> Self {
        Self {
            set_layouts: Vec::new(),
            push_constant_ranges: Vec::new(),
            label: None,
        }
    }
}

// ============================================================================
// Pipeline Layout Handle
// ============================================================================

/// Handle to a pipeline layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineLayoutHandle(Handle<PipelineLayout>);

impl PipelineLayoutHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Pipeline Layout
// ============================================================================

/// A pipeline layout.
pub struct PipelineLayout {
    /// Handle.
    pub handle: PipelineLayoutHandle,
    /// Set layouts.
    pub set_layouts: Vec<DescriptorSetLayoutHandle>,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Descriptor Pool Size
// ============================================================================

/// Descriptor pool size.
#[derive(Debug, Clone, Copy)]
pub struct DescriptorPoolSize {
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Count.
    pub count: u32,
}

// ============================================================================
// Descriptor Pool Description
// ============================================================================

bitflags! {
    /// Descriptor pool flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DescriptorPoolFlags: u32 {
        /// Allow freeing individual sets.
        const FREE_DESCRIPTOR_SET = 1 << 0;
        /// Support update after bind.
        const UPDATE_AFTER_BIND = 1 << 1;
    }
}

/// Description for descriptor pool creation.
#[derive(Debug, Clone)]
pub struct DescriptorPoolDesc {
    /// Maximum sets.
    pub max_sets: u32,
    /// Pool sizes.
    pub pool_sizes: Vec<DescriptorPoolSize>,
    /// Flags.
    pub flags: DescriptorPoolFlags,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for DescriptorPoolDesc {
    fn default() -> Self {
        Self {
            max_sets: 1000,
            pool_sizes: Vec::new(),
            flags: DescriptorPoolFlags::empty(),
            label: None,
        }
    }
}

impl DescriptorPoolDesc {
    /// Create a new description with common defaults.
    pub fn new(max_sets: u32) -> Self {
        Self {
            max_sets,
            pool_sizes: vec![
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::UniformBuffer,
                    count: max_sets * 4,
                },
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::StorageBuffer,
                    count: max_sets * 4,
                },
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::SampledImage,
                    count: max_sets * 8,
                },
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::StorageImage,
                    count: max_sets * 2,
                },
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::Sampler,
                    count: max_sets * 4,
                },
                DescriptorPoolSize {
                    descriptor_type: DescriptorType::CombinedImageSampler,
                    count: max_sets * 8,
                },
            ],
            flags: DescriptorPoolFlags::FREE_DESCRIPTOR_SET,
            label: None,
        }
    }
}

// ============================================================================
// Descriptor Pool Handle
// ============================================================================

/// Handle to a descriptor pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorPoolHandle(Handle<DescriptorPool>);

impl DescriptorPoolHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Descriptor Pool
// ============================================================================

/// A descriptor pool.
pub struct DescriptorPool {
    /// Handle.
    pub handle: DescriptorPoolHandle,
    /// Maximum sets.
    pub max_sets: u32,
    /// Allocated sets.
    pub allocated_sets: u32,
    /// Flags.
    pub flags: DescriptorPoolFlags,
    /// Debug label.
    pub label: Option<String>,
}

impl DescriptorPool {
    /// Get remaining capacity.
    pub fn remaining_capacity(&self) -> u32 {
        self.max_sets.saturating_sub(self.allocated_sets)
    }
}

// ============================================================================
// Descriptor Set Handle
// ============================================================================

/// Handle to a descriptor set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorSetHandle(Handle<DescriptorSet>);

impl DescriptorSetHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Descriptor Set
// ============================================================================

/// A descriptor set.
pub struct DescriptorSet {
    /// Handle.
    pub handle: DescriptorSetHandle,
    /// Layout.
    pub layout: DescriptorSetLayoutHandle,
    /// Pool.
    pub pool: DescriptorPoolHandle,
    /// Debug label.
    pub label: Option<String>,
}

// ============================================================================
// Descriptor Write
// ============================================================================

/// Buffer info for descriptor write.
#[derive(Debug, Clone, Copy)]
pub struct DescriptorBufferInfo {
    /// Buffer handle.
    pub buffer: BufferHandle,
    /// Offset.
    pub offset: u64,
    /// Range size.
    pub range: u64,
}

/// Image info for descriptor write.
#[derive(Debug, Clone)]
pub struct DescriptorImageInfo {
    /// Sampler handle.
    pub sampler: Option<SamplerHandle>,
    /// Image view handle.
    pub image_view: Option<TextureViewHandle>,
    /// Image layout.
    pub layout: ImageLayout,
}

/// Image layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout {
    /// Undefined.
    Undefined,
    /// General.
    General,
    /// Color attachment.
    ColorAttachmentOptimal,
    /// Depth stencil attachment.
    DepthStencilAttachmentOptimal,
    /// Depth stencil read-only.
    DepthStencilReadOnlyOptimal,
    /// Shader read-only.
    ShaderReadOnlyOptimal,
    /// Transfer source.
    TransferSrcOptimal,
    /// Transfer destination.
    TransferDstOptimal,
    /// Present source.
    PresentSrc,
}

/// Descriptor write data.
#[derive(Debug, Clone)]
pub enum DescriptorWriteData {
    /// Buffer descriptors.
    Buffer(Vec<DescriptorBufferInfo>),
    /// Image descriptors.
    Image(Vec<DescriptorImageInfo>),
    /// Texel buffer views.
    TexelBuffer(Vec<BufferHandle>),
}

/// Descriptor write.
#[derive(Debug, Clone)]
pub struct DescriptorWrite {
    /// Destination set.
    pub dst_set: DescriptorSetHandle,
    /// Destination binding.
    pub dst_binding: u32,
    /// Destination array element.
    pub dst_array_element: u32,
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Write data.
    pub data: DescriptorWriteData,
}

impl DescriptorWrite {
    /// Create a uniform buffer write.
    pub fn uniform_buffer(
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
            descriptor_type: DescriptorType::UniformBuffer,
            data: DescriptorWriteData::Buffer(vec![DescriptorBufferInfo {
                buffer,
                offset,
                range,
            }]),
        }
    }

    /// Create a storage buffer write.
    pub fn storage_buffer(
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
            descriptor_type: DescriptorType::StorageBuffer,
            data: DescriptorWriteData::Buffer(vec![DescriptorBufferInfo {
                buffer,
                offset,
                range,
            }]),
        }
    }

    /// Create a sampled image write.
    pub fn sampled_image(set: DescriptorSetHandle, binding: u32, view: TextureViewHandle) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::SampledImage,
            data: DescriptorWriteData::Image(vec![DescriptorImageInfo {
                sampler: None,
                image_view: Some(view),
                layout: ImageLayout::ShaderReadOnlyOptimal,
            }]),
        }
    }

    /// Create a sampler write.
    pub fn sampler(set: DescriptorSetHandle, binding: u32, sampler: SamplerHandle) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::Sampler,
            data: DescriptorWriteData::Image(vec![DescriptorImageInfo {
                sampler: Some(sampler),
                image_view: None,
                layout: ImageLayout::Undefined,
            }]),
        }
    }

    /// Create a combined image/sampler write.
    pub fn combined_image_sampler(
        set: DescriptorSetHandle,
        binding: u32,
        view: TextureViewHandle,
        sampler: SamplerHandle,
    ) -> Self {
        Self {
            dst_set: set,
            dst_binding: binding,
            dst_array_element: 0,
            descriptor_type: DescriptorType::CombinedImageSampler,
            data: DescriptorWriteData::Image(vec![DescriptorImageInfo {
                sampler: Some(sampler),
                image_view: Some(view),
                layout: ImageLayout::ShaderReadOnlyOptimal,
            }]),
        }
    }
}

// ============================================================================
// Descriptor Manager
// ============================================================================

/// Manages descriptors.
pub struct DescriptorManager {
    /// Set layouts.
    layouts: Vec<Option<DescriptorSetLayout>>,
    /// Pipeline layouts.
    pipeline_layouts: Vec<Option<PipelineLayout>>,
    /// Descriptor pools.
    pools: Vec<Option<DescriptorPool>>,
    /// Descriptor sets.
    sets: Vec<Option<DescriptorSet>>,
    /// Free indices.
    free_layouts: Vec<u32>,
    free_pipeline_layouts: Vec<u32>,
    free_pools: Vec<u32>,
    free_sets: Vec<u32>,
    /// Generations.
    gen_layouts: Vec<u32>,
    gen_pipeline_layouts: Vec<u32>,
    gen_pools: Vec<u32>,
    gen_sets: Vec<u32>,
    /// Counts.
    layout_count: AtomicU32,
    pipeline_layout_count: AtomicU32,
    pool_count: AtomicU32,
    set_count: AtomicU32,
}

impl DescriptorManager {
    /// Create a new descriptor manager.
    pub fn new() -> Self {
        Self {
            layouts: Vec::new(),
            pipeline_layouts: Vec::new(),
            pools: Vec::new(),
            sets: Vec::new(),
            free_layouts: Vec::new(),
            free_pipeline_layouts: Vec::new(),
            free_pools: Vec::new(),
            free_sets: Vec::new(),
            gen_layouts: Vec::new(),
            gen_pipeline_layouts: Vec::new(),
            gen_pools: Vec::new(),
            gen_sets: Vec::new(),
            layout_count: AtomicU32::new(0),
            pipeline_layout_count: AtomicU32::new(0),
            pool_count: AtomicU32::new(0),
            set_count: AtomicU32::new(0),
        }
    }

    /// Create a descriptor set layout.
    pub fn create_set_layout(
        &mut self,
        desc: &DescriptorSetLayoutDesc,
    ) -> DescriptorSetLayoutHandle {
        let index = if let Some(index) = self.free_layouts.pop() {
            index
        } else {
            let index = self.layouts.len() as u32;
            self.layouts.push(None);
            self.gen_layouts.push(0);
            index
        };

        let generation = self.gen_layouts[index as usize];
        let handle = DescriptorSetLayoutHandle::new(index, generation);
        let layout = DescriptorSetLayout {
            handle,
            bindings: desc.bindings.clone(),
            flags: desc.flags,
            label: desc.label.clone(),
        };

        self.layouts[index as usize] = Some(layout);
        self.layout_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Create a pipeline layout.
    pub fn create_pipeline_layout(&mut self, desc: &PipelineLayoutDesc) -> PipelineLayoutHandle {
        let index = if let Some(index) = self.free_pipeline_layouts.pop() {
            index
        } else {
            let index = self.pipeline_layouts.len() as u32;
            self.pipeline_layouts.push(None);
            self.gen_pipeline_layouts.push(0);
            index
        };

        let generation = self.gen_pipeline_layouts[index as usize];
        let handle = PipelineLayoutHandle::new(index, generation);
        let layout = PipelineLayout {
            handle,
            set_layouts: desc.set_layouts.clone(),
            label: desc.label.clone(),
        };

        self.pipeline_layouts[index as usize] = Some(layout);
        self.pipeline_layout_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Create a descriptor pool.
    pub fn create_pool(&mut self, desc: &DescriptorPoolDesc) -> DescriptorPoolHandle {
        let index = if let Some(index) = self.free_pools.pop() {
            index
        } else {
            let index = self.pools.len() as u32;
            self.pools.push(None);
            self.gen_pools.push(0);
            index
        };

        let generation = self.gen_pools[index as usize];
        let handle = DescriptorPoolHandle::new(index, generation);
        let pool = DescriptorPool {
            handle,
            max_sets: desc.max_sets,
            allocated_sets: 0,
            flags: desc.flags,
            label: desc.label.clone(),
        };

        self.pools[index as usize] = Some(pool);
        self.pool_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Allocate a descriptor set.
    pub fn allocate_set(
        &mut self,
        pool: DescriptorPoolHandle,
        layout: DescriptorSetLayoutHandle,
    ) -> Option<DescriptorSetHandle> {
        // Check pool capacity
        let pool_obj = self.pools.get_mut(pool.index() as usize)?.as_mut()?;
        if pool_obj.remaining_capacity() == 0 {
            return None;
        }

        let index = if let Some(index) = self.free_sets.pop() {
            index
        } else {
            let index = self.sets.len() as u32;
            self.sets.push(None);
            self.gen_sets.push(0);
            index
        };

        let generation = self.gen_sets[index as usize];
        let handle = DescriptorSetHandle::new(index, generation);
        let set = DescriptorSet {
            handle,
            layout,
            pool,
            label: None,
        };

        self.sets[index as usize] = Some(set);
        pool_obj.allocated_sets += 1;
        self.set_count.fetch_add(1, Ordering::Relaxed);

        Some(handle)
    }

    /// Free a descriptor set.
    pub fn free_set(&mut self, handle: DescriptorSetHandle) {
        let index = handle.index() as usize;
        if let Some(set) = self.sets.get_mut(index).and_then(|s| s.take()) {
            // Update pool
            if let Some(pool) = self
                .pools
                .get_mut(set.pool.index() as usize)
                .and_then(|p| p.as_mut())
            {
                pool.allocated_sets = pool.allocated_sets.saturating_sub(1);
            }
            self.set_count.fetch_sub(1, Ordering::Relaxed);
            self.gen_sets[index] = self.gen_sets[index].wrapping_add(1);
            self.free_sets.push(index as u32);
        }
    }

    /// Reset a descriptor pool.
    pub fn reset_pool(&mut self, handle: DescriptorPoolHandle) {
        if let Some(pool) = self
            .pools
            .get_mut(handle.index() as usize)
            .and_then(|p| p.as_mut())
        {
            // Free all sets from this pool
            for i in 0..self.sets.len() {
                if let Some(set) = &self.sets[i] {
                    if set.pool == handle {
                        self.sets[i] = None;
                        self.gen_sets[i] = self.gen_sets[i].wrapping_add(1);
                        self.free_sets.push(i as u32);
                        self.set_count.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            }
            pool.allocated_sets = 0;
        }
    }

    /// Get counts.
    pub fn layout_count(&self) -> u32 {
        self.layout_count.load(Ordering::Relaxed)
    }

    pub fn pipeline_layout_count(&self) -> u32 {
        self.pipeline_layout_count.load(Ordering::Relaxed)
    }

    pub fn pool_count(&self) -> u32 {
        self.pool_count.load(Ordering::Relaxed)
    }

    pub fn set_count(&self) -> u32 {
        self.set_count.load(Ordering::Relaxed)
    }
}

impl Default for DescriptorManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Bindless Descriptor Support
// ============================================================================

/// Bindless resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindlessResourceType {
    /// Texture.
    Texture,
    /// Storage image.
    StorageImage,
    /// Buffer.
    Buffer,
    /// Sampler.
    Sampler,
}

/// Bindless descriptor heap.
pub struct BindlessDescriptorHeap {
    /// Maximum descriptors.
    pub max_descriptors: u32,
    /// Resource type.
    pub resource_type: BindlessResourceType,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Next index.
    next_index: u32,
    /// Allocated count.
    allocated: AtomicU32,
}

impl BindlessDescriptorHeap {
    /// Create a new bindless heap.
    pub fn new(max_descriptors: u32, resource_type: BindlessResourceType) -> Self {
        Self {
            max_descriptors,
            resource_type,
            free_indices: Vec::new(),
            next_index: 0,
            allocated: AtomicU32::new(0),
        }
    }

    /// Allocate a descriptor index.
    pub fn allocate(&mut self) -> Option<u32> {
        if let Some(index) = self.free_indices.pop() {
            self.allocated.fetch_add(1, Ordering::Relaxed);
            return Some(index);
        }

        if self.next_index < self.max_descriptors {
            let index = self.next_index;
            self.next_index += 1;
            self.allocated.fetch_add(1, Ordering::Relaxed);
            return Some(index);
        }

        None
    }

    /// Free a descriptor index.
    pub fn free(&mut self, index: u32) {
        if index < self.max_descriptors {
            self.free_indices.push(index);
            self.allocated.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Get remaining capacity.
    pub fn remaining_capacity(&self) -> u32 {
        self.max_descriptors.saturating_sub(self.next_index) + self.free_indices.len() as u32
    }
}
