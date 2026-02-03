//! Descriptor Sets and Layouts
//!
//! This module provides descriptor management for GPU resources including:
//! - Descriptor set layouts
//! - Descriptor pools
//! - Descriptor set allocation and updates
//! - Bindless descriptors

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::hash::{Hash, Hasher};

use crate::shader::ShaderStageFlags;

// ============================================================================
// Descriptor Types
// ============================================================================

/// Descriptor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    /// Sampler.
    Sampler,
    /// Combined image sampler.
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
    /// Uniform buffer dynamic.
    UniformBufferDynamic,
    /// Storage buffer dynamic.
    StorageBufferDynamic,
    /// Input attachment.
    InputAttachment,
    /// Inline uniform block.
    InlineUniformBlock,
    /// Acceleration structure.
    AccelerationStructure,
    /// Mutable descriptor (EXT).
    MutableExt,
}

impl DescriptorType {
    /// Check if this is a buffer type.
    pub fn is_buffer(&self) -> bool {
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

    /// Check if this is an image type.
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Self::SampledImage
                | Self::StorageImage
                | Self::CombinedImageSampler
                | Self::InputAttachment
        )
    }

    /// Check if this is a dynamic type.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::UniformBufferDynamic | Self::StorageBufferDynamic)
    }
}

// ============================================================================
// Descriptor Binding
// ============================================================================

/// Descriptor binding flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DescriptorBindingFlags(u32);

impl DescriptorBindingFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Update after bind.
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 0);
    /// Update unused while pending.
    pub const UPDATE_UNUSED_WHILE_PENDING: Self = Self(1 << 1);
    /// Partially bound.
    pub const PARTIALLY_BOUND: Self = Self(1 << 2);
    /// Variable descriptor count.
    pub const VARIABLE_DESCRIPTOR_COUNT: Self = Self(1 << 3);

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if flag is set.
    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}

/// Descriptor binding.
#[derive(Clone)]
pub struct DescriptorBinding {
    /// Binding index.
    pub binding: u32,
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Descriptor count.
    pub count: u32,
    /// Shader stages.
    pub stages: ShaderStageFlags,
    /// Binding flags.
    pub flags: DescriptorBindingFlags,
    /// Immutable samplers.
    pub immutable_samplers: Option<Vec<SamplerHandle>>,
}

impl DescriptorBinding {
    /// Create a new binding.
    pub fn new(binding: u32, descriptor_type: DescriptorType, stages: ShaderStageFlags) -> Self {
        Self {
            binding,
            descriptor_type,
            count: 1,
            stages,
            flags: DescriptorBindingFlags::NONE,
            immutable_samplers: None,
        }
    }

    /// Create uniform buffer binding.
    pub fn uniform_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::UniformBuffer, stages)
    }

    /// Create storage buffer binding.
    pub fn storage_buffer(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageBuffer, stages)
    }

    /// Create sampled image binding.
    pub fn sampled_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::SampledImage, stages)
    }

    /// Create storage image binding.
    pub fn storage_image(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::StorageImage, stages)
    }

    /// Create sampler binding.
    pub fn sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::Sampler, stages)
    }

    /// Create combined image sampler binding.
    pub fn combined_image_sampler(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::CombinedImageSampler, stages)
    }

    /// Create acceleration structure binding.
    pub fn acceleration_structure(binding: u32, stages: ShaderStageFlags) -> Self {
        Self::new(binding, DescriptorType::AccelerationStructure, stages)
    }

    /// Set descriptor count.
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// Set binding flags.
    pub fn flags(mut self, flags: DescriptorBindingFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set immutable samplers.
    pub fn immutable_samplers(mut self, samplers: Vec<SamplerHandle>) -> Self {
        self.immutable_samplers = Some(samplers);
        self
    }

    /// Make bindless (update after bind + partially bound).
    pub fn bindless(mut self) -> Self {
        self.flags = DescriptorBindingFlags::UPDATE_AFTER_BIND
            .or(DescriptorBindingFlags::PARTIALLY_BOUND);
        self
    }
}

/// Sampler handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerHandle(pub u32);

// ============================================================================
// Descriptor Set Layout
// ============================================================================

/// Descriptor set layout flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DescriptorSetLayoutFlags(u32);

impl DescriptorSetLayoutFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Update after bind pool.
    pub const UPDATE_AFTER_BIND_POOL: Self = Self(1 << 0);
    /// Push descriptor.
    pub const PUSH_DESCRIPTOR: Self = Self(1 << 1);
    /// Host only pool.
    pub const HOST_ONLY_POOL: Self = Self(1 << 2);

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if flag is set.
    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}

/// Descriptor set layout.
#[derive(Clone)]
pub struct DescriptorSetLayout {
    /// Bindings.
    bindings: Vec<DescriptorBinding>,
    /// Flags.
    flags: DescriptorSetLayoutFlags,
    /// Layout hash.
    hash: u64,
    /// Debug name.
    name: String,
}

impl DescriptorSetLayout {
    /// Create a new descriptor set layout.
    pub fn new(bindings: Vec<DescriptorBinding>) -> Self {
        let hash = Self::compute_hash(&bindings);
        Self {
            bindings,
            flags: DescriptorSetLayoutFlags::NONE,
            hash,
            name: String::new(),
        }
    }

    /// Create with flags.
    pub fn with_flags(bindings: Vec<DescriptorBinding>, flags: DescriptorSetLayoutFlags) -> Self {
        let hash = Self::compute_hash(&bindings);
        Self {
            bindings,
            flags,
            hash,
            name: String::new(),
        }
    }

    /// Set debug name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Get bindings.
    pub fn bindings(&self) -> &[DescriptorBinding] {
        &self.bindings
    }

    /// Get flags.
    pub fn flags(&self) -> DescriptorSetLayoutFlags {
        self.flags
    }

    /// Get hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Get debug name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get binding by index.
    pub fn get_binding(&self, binding: u32) -> Option<&DescriptorBinding> {
        self.bindings.iter().find(|b| b.binding == binding)
    }

    /// Get total descriptor count.
    pub fn descriptor_count(&self) -> u32 {
        self.bindings.iter().map(|b| b.count).sum()
    }

    /// Compute layout hash.
    fn compute_hash(bindings: &[DescriptorBinding]) -> u64 {
        let mut hasher = FnvHasher::new();
        for binding in bindings {
            binding.binding.hash(&mut hasher);
            (binding.descriptor_type as u32).hash(&mut hasher);
            binding.count.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Builder for descriptor set layouts.
pub struct DescriptorSetLayoutBuilder {
    bindings: Vec<DescriptorBinding>,
    flags: DescriptorSetLayoutFlags,
    name: String,
}

impl DescriptorSetLayoutBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            flags: DescriptorSetLayoutFlags::NONE,
            name: String::new(),
        }
    }

    /// Add a binding.
    pub fn binding(mut self, binding: DescriptorBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Add uniform buffer binding.
    pub fn uniform_buffer(self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.binding(DescriptorBinding::uniform_buffer(binding, stages))
    }

    /// Add storage buffer binding.
    pub fn storage_buffer(self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.binding(DescriptorBinding::storage_buffer(binding, stages))
    }

    /// Add sampled image binding.
    pub fn sampled_image(self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.binding(DescriptorBinding::sampled_image(binding, stages))
    }

    /// Add sampler binding.
    pub fn sampler(self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.binding(DescriptorBinding::sampler(binding, stages))
    }

    /// Add combined image sampler binding.
    pub fn combined_image_sampler(self, binding: u32, stages: ShaderStageFlags) -> Self {
        self.binding(DescriptorBinding::combined_image_sampler(binding, stages))
    }

    /// Set flags.
    pub fn flags(mut self, flags: DescriptorSetLayoutFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set debug name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Build the layout.
    pub fn build(self) -> DescriptorSetLayout {
        DescriptorSetLayout::with_flags(self.bindings, self.flags).with_name(&self.name)
    }
}

impl Default for DescriptorSetLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Descriptor Pool
// ============================================================================

/// Descriptor pool size.
#[derive(Debug, Clone, Copy)]
pub struct DescriptorPoolSize {
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Descriptor count.
    pub count: u32,
}

impl DescriptorPoolSize {
    /// Create a new pool size.
    pub fn new(descriptor_type: DescriptorType, count: u32) -> Self {
        Self {
            descriptor_type,
            count,
        }
    }
}

/// Descriptor pool flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DescriptorPoolFlags(u32);

impl DescriptorPoolFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Free descriptor set.
    pub const FREE_DESCRIPTOR_SET: Self = Self(1 << 0);
    /// Update after bind.
    pub const UPDATE_AFTER_BIND: Self = Self(1 << 1);
    /// Host only.
    pub const HOST_ONLY: Self = Self(1 << 2);

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Descriptor pool configuration.
#[derive(Clone)]
pub struct DescriptorPoolConfig {
    /// Maximum sets.
    pub max_sets: u32,
    /// Pool sizes.
    pub sizes: Vec<DescriptorPoolSize>,
    /// Flags.
    pub flags: DescriptorPoolFlags,
}

impl DescriptorPoolConfig {
    /// Create a new pool config.
    pub fn new(max_sets: u32) -> Self {
        Self {
            max_sets,
            sizes: Vec::new(),
            flags: DescriptorPoolFlags::NONE,
        }
    }

    /// Add pool size.
    pub fn size(mut self, descriptor_type: DescriptorType, count: u32) -> Self {
        self.sizes.push(DescriptorPoolSize::new(descriptor_type, count));
        self
    }

    /// Set flags.
    pub fn flags(mut self, flags: DescriptorPoolFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Create default pool for common usage.
    pub fn default_pool() -> Self {
        Self::new(1024)
            .size(DescriptorType::Sampler, 256)
            .size(DescriptorType::CombinedImageSampler, 4096)
            .size(DescriptorType::SampledImage, 4096)
            .size(DescriptorType::StorageImage, 1024)
            .size(DescriptorType::UniformTexelBuffer, 256)
            .size(DescriptorType::StorageTexelBuffer, 256)
            .size(DescriptorType::UniformBuffer, 2048)
            .size(DescriptorType::StorageBuffer, 2048)
            .size(DescriptorType::UniformBufferDynamic, 256)
            .size(DescriptorType::StorageBufferDynamic, 256)
            .size(DescriptorType::InputAttachment, 128)
    }

    /// Create bindless pool.
    pub fn bindless_pool(texture_count: u32, buffer_count: u32) -> Self {
        Self::new(16)
            .size(DescriptorType::SampledImage, texture_count)
            .size(DescriptorType::StorageBuffer, buffer_count)
            .size(DescriptorType::Sampler, 32)
            .flags(DescriptorPoolFlags::UPDATE_AFTER_BIND)
    }
}

/// Descriptor pool.
pub struct DescriptorPool {
    /// Configuration.
    config: DescriptorPoolConfig,
    /// Allocated sets.
    allocated_sets: u32,
    /// Free list.
    free_list: Vec<DescriptorSetHandle>,
}

impl DescriptorPool {
    /// Create a new descriptor pool.
    pub fn new(config: DescriptorPoolConfig) -> Self {
        Self {
            config,
            allocated_sets: 0,
            free_list: Vec::new(),
        }
    }

    /// Allocate a descriptor set.
    pub fn allocate(&mut self, _layout: &DescriptorSetLayout) -> Option<DescriptorSetHandle> {
        if !self.free_list.is_empty() {
            return self.free_list.pop();
        }

        if self.allocated_sets >= self.config.max_sets {
            return None;
        }

        let handle = DescriptorSetHandle(self.allocated_sets);
        self.allocated_sets += 1;
        Some(handle)
    }

    /// Free a descriptor set.
    pub fn free(&mut self, handle: DescriptorSetHandle) {
        if self.config.flags.0 & DescriptorPoolFlags::FREE_DESCRIPTOR_SET.0 != 0 {
            self.free_list.push(handle);
        }
    }

    /// Reset the pool.
    pub fn reset(&mut self) {
        self.allocated_sets = 0;
        self.free_list.clear();
    }

    /// Get allocated set count.
    pub fn allocated_count(&self) -> u32 {
        self.allocated_sets - self.free_list.len() as u32
    }

    /// Get available set count.
    pub fn available_count(&self) -> u32 {
        self.config.max_sets - self.allocated_count()
    }
}

/// Descriptor set handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorSetHandle(pub u32);

// ============================================================================
// Descriptor Set
// ============================================================================

/// Buffer info for descriptor write.
#[derive(Debug, Clone)]
pub struct DescriptorBufferInfo {
    /// Buffer handle.
    pub buffer: BufferHandle,
    /// Offset in bytes.
    pub offset: u64,
    /// Range in bytes.
    pub range: u64,
}

impl DescriptorBufferInfo {
    /// Create a new buffer info.
    pub fn new(buffer: BufferHandle, offset: u64, range: u64) -> Self {
        Self {
            buffer,
            offset,
            range,
        }
    }

    /// Whole buffer.
    pub fn whole(buffer: BufferHandle) -> Self {
        Self {
            buffer,
            offset: 0,
            range: u64::MAX,
        }
    }
}

/// Buffer handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u32);

/// Image info for descriptor write.
#[derive(Debug, Clone)]
pub struct DescriptorImageInfo {
    /// Sampler handle.
    pub sampler: Option<SamplerHandle>,
    /// Image view handle.
    pub image_view: ImageViewHandle,
    /// Image layout.
    pub layout: ImageLayout,
}

impl DescriptorImageInfo {
    /// Create a new image info.
    pub fn new(image_view: ImageViewHandle, layout: ImageLayout) -> Self {
        Self {
            sampler: None,
            image_view,
            layout,
        }
    }

    /// Create with sampler.
    pub fn with_sampler(
        sampler: SamplerHandle,
        image_view: ImageViewHandle,
        layout: ImageLayout,
    ) -> Self {
        Self {
            sampler: Some(sampler),
            image_view,
            layout,
        }
    }
}

/// Image view handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageViewHandle(pub u32);

/// Image layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ImageLayout {
    /// Undefined layout.
    #[default]
    Undefined,
    /// General layout.
    General,
    /// Color attachment optimal.
    ColorAttachmentOptimal,
    /// Depth stencil attachment optimal.
    DepthStencilAttachmentOptimal,
    /// Depth stencil read-only optimal.
    DepthStencilReadOnlyOptimal,
    /// Shader read-only optimal.
    ShaderReadOnlyOptimal,
    /// Transfer source optimal.
    TransferSrcOptimal,
    /// Transfer destination optimal.
    TransferDstOptimal,
    /// Preinitialized.
    Preinitialized,
    /// Present source.
    PresentSrc,
    /// Shared present.
    SharedPresent,
    /// Depth read-only stencil attachment optimal.
    DepthReadOnlyStencilAttachmentOptimal,
    /// Depth attachment stencil read-only optimal.
    DepthAttachmentStencilReadOnlyOptimal,
    /// Depth attachment optimal.
    DepthAttachmentOptimal,
    /// Depth read-only optimal.
    DepthReadOnlyOptimal,
    /// Stencil attachment optimal.
    StencilAttachmentOptimal,
    /// Stencil read-only optimal.
    StencilReadOnlyOptimal,
    /// Read-only optimal.
    ReadOnlyOptimal,
    /// Attachment optimal.
    AttachmentOptimal,
}

/// Acceleration structure handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccelerationStructureHandle(pub u32);

/// Descriptor write data.
#[derive(Clone)]
pub enum DescriptorWriteData {
    /// Buffer.
    Buffer(Vec<DescriptorBufferInfo>),
    /// Image.
    Image(Vec<DescriptorImageInfo>),
    /// Texel buffer view.
    TexelBuffer(Vec<BufferViewHandle>),
    /// Acceleration structure.
    AccelerationStructure(Vec<AccelerationStructureHandle>),
    /// Inline uniform block data.
    InlineUniform(Vec<u8>),
}

/// Buffer view handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferViewHandle(pub u32);

/// Descriptor write.
#[derive(Clone)]
pub struct DescriptorWrite {
    /// Target set.
    pub set: DescriptorSetHandle,
    /// Binding index.
    pub binding: u32,
    /// Array element.
    pub array_element: u32,
    /// Descriptor type.
    pub descriptor_type: DescriptorType,
    /// Data.
    pub data: DescriptorWriteData,
}

impl DescriptorWrite {
    /// Create a uniform buffer write.
    pub fn uniform_buffer(
        set: DescriptorSetHandle,
        binding: u32,
        info: DescriptorBufferInfo,
    ) -> Self {
        Self {
            set,
            binding,
            array_element: 0,
            descriptor_type: DescriptorType::UniformBuffer,
            data: DescriptorWriteData::Buffer(alloc::vec![info]),
        }
    }

    /// Create a storage buffer write.
    pub fn storage_buffer(
        set: DescriptorSetHandle,
        binding: u32,
        info: DescriptorBufferInfo,
    ) -> Self {
        Self {
            set,
            binding,
            array_element: 0,
            descriptor_type: DescriptorType::StorageBuffer,
            data: DescriptorWriteData::Buffer(alloc::vec![info]),
        }
    }

    /// Create a sampled image write.
    pub fn sampled_image(
        set: DescriptorSetHandle,
        binding: u32,
        info: DescriptorImageInfo,
    ) -> Self {
        Self {
            set,
            binding,
            array_element: 0,
            descriptor_type: DescriptorType::SampledImage,
            data: DescriptorWriteData::Image(alloc::vec![info]),
        }
    }

    /// Create a storage image write.
    pub fn storage_image(
        set: DescriptorSetHandle,
        binding: u32,
        info: DescriptorImageInfo,
    ) -> Self {
        Self {
            set,
            binding,
            array_element: 0,
            descriptor_type: DescriptorType::StorageImage,
            data: DescriptorWriteData::Image(alloc::vec![info]),
        }
    }

    /// Create a combined image sampler write.
    pub fn combined_image_sampler(
        set: DescriptorSetHandle,
        binding: u32,
        info: DescriptorImageInfo,
    ) -> Self {
        Self {
            set,
            binding,
            array_element: 0,
            descriptor_type: DescriptorType::CombinedImageSampler,
            data: DescriptorWriteData::Image(alloc::vec![info]),
        }
    }

    /// Set array element.
    pub fn array_element(mut self, element: u32) -> Self {
        self.array_element = element;
        self
    }
}

/// Descriptor copy.
#[derive(Clone)]
pub struct DescriptorCopy {
    /// Source set.
    pub src_set: DescriptorSetHandle,
    /// Source binding.
    pub src_binding: u32,
    /// Source array element.
    pub src_array_element: u32,
    /// Destination set.
    pub dst_set: DescriptorSetHandle,
    /// Destination binding.
    pub dst_binding: u32,
    /// Destination array element.
    pub dst_array_element: u32,
    /// Descriptor count.
    pub count: u32,
}

/// Descriptor set.
pub struct DescriptorSet {
    /// Handle.
    handle: DescriptorSetHandle,
    /// Layout.
    layout: Arc<DescriptorSetLayout>,
    /// Writes pending.
    pending_writes: Vec<DescriptorWrite>,
}

impl DescriptorSet {
    /// Create a new descriptor set.
    pub fn new(handle: DescriptorSetHandle, layout: Arc<DescriptorSetLayout>) -> Self {
        Self {
            handle,
            layout,
            pending_writes: Vec::new(),
        }
    }

    /// Get the handle.
    pub fn handle(&self) -> DescriptorSetHandle {
        self.handle
    }

    /// Get the layout.
    pub fn layout(&self) -> &DescriptorSetLayout {
        &self.layout
    }

    /// Queue a write.
    pub fn write(&mut self, mut write: DescriptorWrite) {
        write.set = self.handle;
        self.pending_writes.push(write);
    }

    /// Get pending writes.
    pub fn pending_writes(&self) -> &[DescriptorWrite] {
        &self.pending_writes
    }

    /// Take pending writes.
    pub fn take_pending_writes(&mut self) -> Vec<DescriptorWrite> {
        core::mem::take(&mut self.pending_writes)
    }
}

// ============================================================================
// Descriptor Set Allocator
// ============================================================================

/// Descriptor set allocator with automatic pool management.
pub struct DescriptorSetAllocator {
    /// Active pools.
    pools: Vec<DescriptorPool>,
    /// Pool configuration.
    pool_config: DescriptorPoolConfig,
    /// Layout cache.
    layout_cache: Vec<(u64, Arc<DescriptorSetLayout>)>,
}

impl DescriptorSetAllocator {
    /// Create a new allocator.
    pub fn new(pool_config: DescriptorPoolConfig) -> Self {
        Self {
            pools: Vec::new(),
            pool_config,
            layout_cache: Vec::new(),
        }
    }

    /// Allocate a descriptor set.
    pub fn allocate(&mut self, layout: &DescriptorSetLayout) -> DescriptorSetHandle {
        // Try existing pools
        for pool in &mut self.pools {
            if let Some(handle) = pool.allocate(layout) {
                return handle;
            }
        }

        // Create new pool
        let mut pool = DescriptorPool::new(self.pool_config.clone());
        let handle = pool.allocate(layout).expect("Fresh pool should have space");
        self.pools.push(pool);
        handle
    }

    /// Free a descriptor set.
    pub fn free(&mut self, handle: DescriptorSetHandle) {
        for pool in &mut self.pools {
            pool.free(handle);
        }
    }

    /// Reset all pools.
    pub fn reset(&mut self) {
        for pool in &mut self.pools {
            pool.reset();
        }
    }

    /// Get or create a layout.
    pub fn get_or_create_layout(&mut self, bindings: Vec<DescriptorBinding>) -> Arc<DescriptorSetLayout> {
        let layout = DescriptorSetLayout::new(bindings);
        let hash = layout.hash();

        if let Some((_, cached)) = self.layout_cache.iter().find(|(h, _)| *h == hash) {
            return cached.clone();
        }

        let layout = Arc::new(layout);
        self.layout_cache.push((hash, layout.clone()));
        layout
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.pools.iter().map(|p| p.allocated_count()).sum()
    }

    /// Get pool count.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
