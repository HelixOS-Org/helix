//! Resource Management
//!
//! Unified resource handle management and lifecycle tracking.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;

use crate::buffer::BufferHandle;
use crate::descriptor::{DescriptorSetHandle, DescriptorSetLayoutHandle, PipelineLayoutHandle};
use crate::pipeline::{ComputePipelineHandle, RayTracingPipelineHandle, RenderPipelineHandle};
use crate::sampler::SamplerHandle;
use crate::texture::{TextureHandle, TextureViewHandle};

// ============================================================================
// Resource Type
// ============================================================================

/// Resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// Buffer.
    Buffer,
    /// Texture.
    Texture,
    /// Texture view.
    TextureView,
    /// Sampler.
    Sampler,
    /// Render pipeline.
    RenderPipeline,
    /// Compute pipeline.
    ComputePipeline,
    /// Ray tracing pipeline.
    RayTracingPipeline,
    /// Descriptor set.
    DescriptorSet,
    /// Descriptor set layout.
    DescriptorSetLayout,
    /// Pipeline layout.
    PipelineLayout,
    /// Acceleration structure.
    AccelerationStructure,
    /// Query pool.
    QueryPool,
    /// Framebuffer.
    Framebuffer,
    /// Render pass.
    RenderPass,
}

impl ResourceType {
    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::Texture => "Texture",
            Self::TextureView => "TextureView",
            Self::Sampler => "Sampler",
            Self::RenderPipeline => "RenderPipeline",
            Self::ComputePipeline => "ComputePipeline",
            Self::RayTracingPipeline => "RayTracingPipeline",
            Self::DescriptorSet => "DescriptorSet",
            Self::DescriptorSetLayout => "DescriptorSetLayout",
            Self::PipelineLayout => "PipelineLayout",
            Self::AccelerationStructure => "AccelerationStructure",
            Self::QueryPool => "QueryPool",
            Self::Framebuffer => "Framebuffer",
            Self::RenderPass => "RenderPass",
        }
    }
}

// ============================================================================
// Resource State Flags
// ============================================================================

bitflags! {
    /// Resource state flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ResourceStateFlags: u32 {
        /// Resource is initialized.
        const INITIALIZED = 1 << 0;
        /// Resource is in use by GPU.
        const IN_USE = 1 << 1;
        /// Resource is mapped.
        const MAPPED = 1 << 2;
        /// Resource is pending deletion.
        const PENDING_DELETE = 1 << 3;
        /// Resource has debug name.
        const HAS_NAME = 1 << 4;
        /// Resource is external.
        const EXTERNAL = 1 << 5;
        /// Resource is shared.
        const SHARED = 1 << 6;
    }
}

// ============================================================================
// Resource Handle
// ============================================================================

/// Unified resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle {
    /// Resource type.
    pub resource_type: ResourceType,
    /// Index.
    pub index: u32,
    /// Generation.
    pub generation: u32,
}

impl ResourceHandle {
    /// Create a new handle.
    pub fn new(resource_type: ResourceType, index: u32, generation: u32) -> Self {
        Self {
            resource_type,
            index,
            generation,
        }
    }

    /// Create from buffer handle.
    pub fn from_buffer(handle: BufferHandle) -> Self {
        Self::new(ResourceType::Buffer, handle.index(), 0)
    }

    /// Create from texture handle.
    pub fn from_texture(handle: TextureHandle) -> Self {
        Self::new(ResourceType::Texture, handle.index(), 0)
    }

    /// Create from texture view handle.
    pub fn from_texture_view(handle: TextureViewHandle) -> Self {
        Self::new(ResourceType::TextureView, handle.index(), 0)
    }

    /// Create from sampler handle.
    pub fn from_sampler(handle: SamplerHandle) -> Self {
        Self::new(ResourceType::Sampler, handle.index(), 0)
    }

    /// Create from render pipeline handle.
    pub fn from_render_pipeline(handle: RenderPipelineHandle) -> Self {
        Self::new(ResourceType::RenderPipeline, handle.index(), 0)
    }

    /// Create from compute pipeline handle.
    pub fn from_compute_pipeline(handle: ComputePipelineHandle) -> Self {
        Self::new(ResourceType::ComputePipeline, handle.index(), 0)
    }
}

// ============================================================================
// Resource Info
// ============================================================================

/// Resource information.
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// Handle.
    pub handle: ResourceHandle,
    /// Debug name.
    pub name: Option<String>,
    /// State flags.
    pub state: ResourceStateFlags,
    /// Size in bytes.
    pub size: u64,
    /// Creation frame.
    pub created_frame: u64,
    /// Last used frame.
    pub last_used_frame: u64,
    /// Reference count.
    pub ref_count: u32,
}

impl ResourceInfo {
    /// Create new resource info.
    pub fn new(handle: ResourceHandle, size: u64, current_frame: u64) -> Self {
        Self {
            handle,
            name: None,
            state: ResourceStateFlags::INITIALIZED,
            size,
            created_frame: current_frame,
            last_used_frame: current_frame,
            ref_count: 1,
        }
    }

    /// Mark as used.
    pub fn mark_used(&mut self, frame: u64) {
        self.last_used_frame = frame;
        self.state.insert(ResourceStateFlags::IN_USE);
    }

    /// Mark as free.
    pub fn mark_free(&mut self) {
        self.state.remove(ResourceStateFlags::IN_USE);
    }

    /// Check if stale.
    pub fn is_stale(&self, current_frame: u64, frames_threshold: u64) -> bool {
        current_frame.saturating_sub(self.last_used_frame) > frames_threshold
    }
}

// ============================================================================
// Resource Statistics
// ============================================================================

/// Resource statistics.
#[derive(Debug, Clone, Default)]
pub struct ResourceStatistics {
    /// Buffer count.
    pub buffer_count: u32,
    /// Texture count.
    pub texture_count: u32,
    /// Texture view count.
    pub texture_view_count: u32,
    /// Sampler count.
    pub sampler_count: u32,
    /// Render pipeline count.
    pub render_pipeline_count: u32,
    /// Compute pipeline count.
    pub compute_pipeline_count: u32,
    /// Ray tracing pipeline count.
    pub ray_tracing_pipeline_count: u32,
    /// Descriptor set count.
    pub descriptor_set_count: u32,
    /// Total memory used.
    pub total_memory: u64,
    /// Buffer memory.
    pub buffer_memory: u64,
    /// Texture memory.
    pub texture_memory: u64,
    /// Peak memory used.
    pub peak_memory: u64,
    /// Resources created this frame.
    pub created_this_frame: u32,
    /// Resources destroyed this frame.
    pub destroyed_this_frame: u32,
}

impl ResourceStatistics {
    /// Get total resource count.
    pub fn total_count(&self) -> u32 {
        self.buffer_count
            + self.texture_count
            + self.texture_view_count
            + self.sampler_count
            + self.render_pipeline_count
            + self.compute_pipeline_count
            + self.ray_tracing_pipeline_count
            + self.descriptor_set_count
    }

    /// Reset frame counters.
    pub fn reset_frame_counters(&mut self) {
        self.created_this_frame = 0;
        self.destroyed_this_frame = 0;
    }
}

// ============================================================================
// Deferred Deletion Queue
// ============================================================================

/// Pending deletion entry.
#[derive(Debug, Clone)]
struct DeletionEntry {
    /// Resource handle.
    handle: ResourceHandle,
    /// Frame when deletion was requested.
    frame: u64,
}

/// Deferred deletion queue.
pub struct DeletionQueue {
    /// Pending deletions.
    entries: Vec<DeletionEntry>,
    /// Frames to wait before deletion.
    frames_in_flight: u64,
    /// Current frame.
    current_frame: AtomicU64,
}

impl DeletionQueue {
    /// Create a new deletion queue.
    pub fn new(frames_in_flight: u64) -> Self {
        Self {
            entries: Vec::new(),
            frames_in_flight,
            current_frame: AtomicU64::new(0),
        }
    }

    /// Queue a resource for deletion.
    pub fn queue(&mut self, handle: ResourceHandle) {
        let frame = self.current_frame.load(Ordering::Relaxed);
        self.entries.push(DeletionEntry { handle, frame });
    }

    /// Advance to next frame.
    pub fn advance_frame(&mut self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
    }

    /// Get resources ready for deletion.
    pub fn drain_ready(&mut self) -> Vec<ResourceHandle> {
        let current = self.current_frame.load(Ordering::Relaxed);
        let threshold = current.saturating_sub(self.frames_in_flight);

        let mut ready = Vec::new();
        self.entries.retain(|entry| {
            if entry.frame <= threshold {
                ready.push(entry.handle);
                false
            } else {
                true
            }
        });

        ready
    }

    /// Get pending count.
    pub fn pending_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all pending.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

// ============================================================================
// Resource Registry
// ============================================================================

/// Resource registry.
pub struct ResourceRegistry {
    /// Resource infos.
    resources: Vec<Option<ResourceInfo>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: ResourceStatistics,
    /// Deletion queue.
    deletion_queue: DeletionQueue,
    /// Current frame.
    current_frame: AtomicU64,
}

impl ResourceRegistry {
    /// Create a new registry.
    pub fn new(frames_in_flight: u64) -> Self {
        Self {
            resources: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: ResourceStatistics::default(),
            deletion_queue: DeletionQueue::new(frames_in_flight),
            current_frame: AtomicU64::new(0),
        }
    }

    /// Register a resource.
    pub fn register(&mut self, resource_type: ResourceType, size: u64) -> ResourceHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.resources.len() as u32;
            self.resources.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = ResourceHandle::new(resource_type, index, generation);
        let frame = self.current_frame.load(Ordering::Relaxed);
        let info = ResourceInfo::new(handle, size, frame);

        self.resources[index as usize] = Some(info);
        self.update_stats_on_create(resource_type, size);

        handle
    }

    /// Unregister a resource (deferred).
    pub fn unregister(&mut self, handle: ResourceHandle) {
        self.deletion_queue.queue(handle);
    }

    /// Get resource info.
    pub fn get(&self, handle: ResourceHandle) -> Option<&ResourceInfo> {
        let index = handle.index as usize;
        if index >= self.resources.len() {
            return None;
        }
        if self.generations[index] != handle.generation {
            return None;
        }
        self.resources[index].as_ref()
    }

    /// Get resource info mutably.
    pub fn get_mut(&mut self, handle: ResourceHandle) -> Option<&mut ResourceInfo> {
        let index = handle.index as usize;
        if index >= self.resources.len() {
            return None;
        }
        if self.generations[index] != handle.generation {
            return None;
        }
        self.resources[index].as_mut()
    }

    /// Set resource name.
    pub fn set_name(&mut self, handle: ResourceHandle, name: &str) {
        if let Some(info) = self.get_mut(handle) {
            info.name = Some(String::from(name));
            info.state.insert(ResourceStateFlags::HAS_NAME);
        }
    }

    /// Mark resource as used.
    pub fn mark_used(&mut self, handle: ResourceHandle) {
        let frame = self.current_frame.load(Ordering::Relaxed);
        if let Some(info) = self.get_mut(handle) {
            info.mark_used(frame);
        }
    }

    /// Advance frame.
    pub fn advance_frame(&mut self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
        self.deletion_queue.advance_frame();
        self.stats.reset_frame_counters();

        // Process deferred deletions
        for handle in self.deletion_queue.drain_ready() {
            self.destroy_immediate(handle);
        }
    }

    /// Destroy immediately.
    fn destroy_immediate(&mut self, handle: ResourceHandle) {
        let index = handle.index as usize;
        if index >= self.resources.len() {
            return;
        }
        if self.generations[index] != handle.generation {
            return;
        }

        if let Some(info) = self.resources[index].take() {
            self.update_stats_on_destroy(info.handle.resource_type, info.size);
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
        }
    }

    /// Update stats on create.
    fn update_stats_on_create(&mut self, resource_type: ResourceType, size: u64) {
        match resource_type {
            ResourceType::Buffer => {
                self.stats.buffer_count += 1;
                self.stats.buffer_memory += size;
            },
            ResourceType::Texture => {
                self.stats.texture_count += 1;
                self.stats.texture_memory += size;
            },
            ResourceType::TextureView => self.stats.texture_view_count += 1,
            ResourceType::Sampler => self.stats.sampler_count += 1,
            ResourceType::RenderPipeline => self.stats.render_pipeline_count += 1,
            ResourceType::ComputePipeline => self.stats.compute_pipeline_count += 1,
            ResourceType::RayTracingPipeline => self.stats.ray_tracing_pipeline_count += 1,
            ResourceType::DescriptorSet => self.stats.descriptor_set_count += 1,
            _ => {},
        }
        self.stats.total_memory += size;
        self.stats.peak_memory = self.stats.peak_memory.max(self.stats.total_memory);
        self.stats.created_this_frame += 1;
    }

    /// Update stats on destroy.
    fn update_stats_on_destroy(&mut self, resource_type: ResourceType, size: u64) {
        match resource_type {
            ResourceType::Buffer => {
                self.stats.buffer_count = self.stats.buffer_count.saturating_sub(1);
                self.stats.buffer_memory = self.stats.buffer_memory.saturating_sub(size);
            },
            ResourceType::Texture => {
                self.stats.texture_count = self.stats.texture_count.saturating_sub(1);
                self.stats.texture_memory = self.stats.texture_memory.saturating_sub(size);
            },
            ResourceType::TextureView => {
                self.stats.texture_view_count = self.stats.texture_view_count.saturating_sub(1);
            },
            ResourceType::Sampler => {
                self.stats.sampler_count = self.stats.sampler_count.saturating_sub(1);
            },
            ResourceType::RenderPipeline => {
                self.stats.render_pipeline_count =
                    self.stats.render_pipeline_count.saturating_sub(1);
            },
            ResourceType::ComputePipeline => {
                self.stats.compute_pipeline_count =
                    self.stats.compute_pipeline_count.saturating_sub(1);
            },
            ResourceType::RayTracingPipeline => {
                self.stats.ray_tracing_pipeline_count =
                    self.stats.ray_tracing_pipeline_count.saturating_sub(1);
            },
            ResourceType::DescriptorSet => {
                self.stats.descriptor_set_count = self.stats.descriptor_set_count.saturating_sub(1);
            },
            _ => {},
        }
        self.stats.total_memory = self.stats.total_memory.saturating_sub(size);
        self.stats.destroyed_this_frame += 1;
    }

    /// Get statistics.
    pub fn statistics(&self) -> &ResourceStatistics {
        &self.stats
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    /// Find stale resources.
    pub fn find_stale(&self, frames_threshold: u64) -> Vec<ResourceHandle> {
        let current = self.current_frame.load(Ordering::Relaxed);
        self.resources
            .iter()
            .filter_map(|r| r.as_ref())
            .filter(|r| r.is_stale(current, frames_threshold))
            .map(|r| r.handle)
            .collect()
    }

    /// Force garbage collection.
    pub fn garbage_collect(&mut self) {
        // Clear all pending deletions
        let handles: Vec<_> = self
            .deletion_queue
            .entries
            .drain(..)
            .map(|e| e.handle)
            .collect();
        for handle in handles {
            self.destroy_immediate(handle);
        }
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new(3) // Default 3 frames in flight
    }
}
