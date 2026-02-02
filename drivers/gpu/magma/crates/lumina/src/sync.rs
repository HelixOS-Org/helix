//! Synchronization primitives
//!
//! This module provides types for GPU synchronization.

use core::sync::atomic::{AtomicU64, Ordering};

/// Fence handle for CPU-GPU synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub u64);

impl FenceHandle {
    /// Null/invalid fence
    pub const NULL: Self = Self(0);

    /// Creates a fence handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Semaphore handle for GPU-GPU synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SemaphoreHandle(pub u64);

impl SemaphoreHandle {
    /// Null/invalid semaphore
    pub const NULL: Self = Self(0);

    /// Creates a semaphore handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Event handle for fine-grained synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EventHandle(pub u64);

impl EventHandle {
    /// Null/invalid event
    pub const NULL: Self = Self(0);

    /// Creates an event handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Fence creation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FenceCreateFlags(pub u32);

impl FenceCreateFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Create fence in signaled state
    pub const SIGNALED: Self = Self(1 << 0);
}

impl core::ops::BitOr for FenceCreateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Fence descriptor
#[derive(Clone, Debug)]
pub struct FenceDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Creation flags
    pub flags: FenceCreateFlags,
}

impl<'a> FenceDesc<'a> {
    /// Creates a fence descriptor with default settings
    pub const fn new() -> Self {
        Self {
            label: None,
            flags: FenceCreateFlags::NONE,
        }
    }

    /// Creates a signaled fence
    pub const fn signaled() -> Self {
        Self {
            label: None,
            flags: FenceCreateFlags::SIGNALED,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl<'a> Default for FenceDesc<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Semaphore type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SemaphoreType {
    /// Binary semaphore
    #[default]
    Binary,
    /// Timeline semaphore with counter
    Timeline,
}

/// Semaphore descriptor
#[derive(Clone, Debug)]
pub struct SemaphoreDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Semaphore type
    pub semaphore_type: SemaphoreType,
    /// Initial timeline value (for timeline semaphores)
    pub initial_value: u64,
}

impl<'a> SemaphoreDesc<'a> {
    /// Creates a binary semaphore descriptor
    pub const fn binary() -> Self {
        Self {
            label: None,
            semaphore_type: SemaphoreType::Binary,
            initial_value: 0,
        }
    }

    /// Creates a timeline semaphore descriptor
    pub const fn timeline(initial_value: u64) -> Self {
        Self {
            label: None,
            semaphore_type: SemaphoreType::Timeline,
            initial_value,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl<'a> Default for SemaphoreDesc<'a> {
    fn default() -> Self {
        Self::binary()
    }
}

/// Event descriptor
#[derive(Clone, Debug)]
pub struct EventDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
}

impl<'a> EventDesc<'a> {
    /// Creates an event descriptor
    pub const fn new() -> Self {
        Self { label: None }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

impl<'a> Default for EventDesc<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// Wait status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitStatus {
    /// Wait completed successfully
    Success,
    /// Timeout expired
    Timeout,
    /// Device lost
    DeviceLost,
}

/// Fence status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceStatus {
    /// Fence is signaled
    Signaled,
    /// Fence is unsignaled
    Unsignaled,
    /// Unknown/error state
    Error,
}

/// Semaphore submit info for queue submission
#[derive(Clone, Debug)]
pub struct SemaphoreSubmitInfo {
    /// Semaphore handle
    pub semaphore: SemaphoreHandle,
    /// Timeline value (for timeline semaphores)
    pub value: u64,
    /// Pipeline stage mask
    pub stage_mask: crate::render_pass::PipelineStage,
}

impl SemaphoreSubmitInfo {
    /// Creates a binary semaphore submit info
    pub const fn binary(semaphore: SemaphoreHandle, stage_mask: crate::render_pass::PipelineStage) -> Self {
        Self {
            semaphore,
            value: 0,
            stage_mask,
        }
    }

    /// Creates a timeline semaphore submit info
    pub const fn timeline(semaphore: SemaphoreHandle, value: u64, stage_mask: crate::render_pass::PipelineStage) -> Self {
        Self {
            semaphore,
            value,
            stage_mask,
        }
    }
}

/// Memory barrier
#[derive(Clone, Copy, Debug)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: crate::render_pass::AccessFlags,
    /// Destination access mask
    pub dst_access: crate::render_pass::AccessFlags,
}

impl MemoryBarrier {
    /// Creates a memory barrier
    pub const fn new(src_access: crate::render_pass::AccessFlags, dst_access: crate::render_pass::AccessFlags) -> Self {
        Self { src_access, dst_access }
    }

    /// Full memory barrier
    pub const fn full() -> Self {
        Self {
            src_access: crate::render_pass::AccessFlags::MEMORY_WRITE,
            dst_access: crate::render_pass::AccessFlags::MEMORY_READ,
        }
    }
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
pub struct BufferMemoryBarrier {
    /// Source access mask
    pub src_access: crate::render_pass::AccessFlags,
    /// Destination access mask
    pub dst_access: crate::render_pass::AccessFlags,
    /// Source queue family index (for ownership transfer)
    pub src_queue_family: u32,
    /// Destination queue family index
    pub dst_queue_family: u32,
    /// Buffer handle
    pub buffer: crate::types::BufferHandle,
    /// Offset in bytes
    pub offset: u64,
    /// Size in bytes (0 = whole buffer)
    pub size: u64,
}

impl BufferMemoryBarrier {
    /// Creates a buffer memory barrier
    pub const fn new(
        buffer: crate::types::BufferHandle,
        src_access: crate::render_pass::AccessFlags,
        dst_access: crate::render_pass::AccessFlags,
    ) -> Self {
        Self {
            src_access,
            dst_access,
            src_queue_family: 0xFFFFFFFF, // QUEUE_FAMILY_IGNORED
            dst_queue_family: 0xFFFFFFFF,
            buffer,
            offset: 0,
            size: 0, // whole buffer
        }
    }

    /// Sets the range
    pub const fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// Sets queue family ownership transfer
    pub const fn with_queue_transfer(mut self, src_family: u32, dst_family: u32) -> Self {
        self.src_queue_family = src_family;
        self.dst_queue_family = dst_family;
        self
    }
}

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
pub struct ImageMemoryBarrier {
    /// Source access mask
    pub src_access: crate::render_pass::AccessFlags,
    /// Destination access mask
    pub dst_access: crate::render_pass::AccessFlags,
    /// Old image layout
    pub old_layout: ImageLayout,
    /// New image layout
    pub new_layout: ImageLayout,
    /// Source queue family index
    pub src_queue_family: u32,
    /// Destination queue family index
    pub dst_queue_family: u32,
    /// Texture handle
    pub texture: crate::types::TextureHandle,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageMemoryBarrier {
    /// Creates an image layout transition barrier
    pub const fn layout_transition(
        texture: crate::types::TextureHandle,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        src_access: crate::render_pass::AccessFlags,
        dst_access: crate::render_pass::AccessFlags,
    ) -> Self {
        Self {
            src_access,
            dst_access,
            old_layout,
            new_layout,
            src_queue_family: 0xFFFFFFFF,
            dst_queue_family: 0xFFFFFFFF,
            texture,
            subresource_range: ImageSubresourceRange::ALL_MIPS_LAYERS,
        }
    }

    /// Sets the subresource range
    pub const fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageLayout {
    /// Undefined (contents undefined, can be discarded)
    #[default]
    Undefined,
    /// General (all operations supported, may not be optimal)
    General,
    /// Optimal for color attachment
    ColorAttachment,
    /// Optimal for depth/stencil attachment
    DepthStencilAttachment,
    /// Read-only depth/stencil
    DepthStencilReadOnly,
    /// Optimal for shader read
    ShaderReadOnly,
    /// Optimal for transfer source
    TransferSrc,
    /// Optimal for transfer destination
    TransferDst,
    /// Ready for presentation
    Present,
}

/// Image subresource range
#[derive(Clone, Copy, Debug)]
pub struct ImageSubresourceRange {
    /// Aspect mask (color, depth, stencil)
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Number of mip levels (0 = all remaining)
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Number of array layers (0 = all remaining)
    pub layer_count: u32,
}

impl ImageSubresourceRange {
    /// All mips and layers, color aspect
    pub const ALL_MIPS_LAYERS: Self = Self {
        aspect_mask: ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 0xFFFFFFFF,
        base_array_layer: 0,
        layer_count: 0xFFFFFFFF,
    };

    /// Creates a range for a single mip level
    pub const fn single_mip(level: u32) -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: level,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 0xFFFFFFFF,
        }
    }

    /// Creates a range with depth aspect
    pub const fn depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        }
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
    /// Color aspect
    pub const COLOR: Self = Self(1 << 0);
    /// Depth aspect
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil aspect
    pub const STENCIL: Self = Self(1 << 2);
    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);
}

impl core::ops::BitOr for ImageAspectFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Pipeline barrier info
#[derive(Clone, Debug)]
pub struct PipelineBarrier<'a> {
    /// Source pipeline stage
    pub src_stage: crate::render_pass::PipelineStage,
    /// Destination pipeline stage
    pub dst_stage: crate::render_pass::PipelineStage,
    /// Memory barriers
    pub memory_barriers: &'a [MemoryBarrier],
    /// Buffer memory barriers
    pub buffer_barriers: &'a [BufferMemoryBarrier],
    /// Image memory barriers
    pub image_barriers: &'a [ImageMemoryBarrier],
}

impl<'a> PipelineBarrier<'a> {
    /// Creates an empty pipeline barrier
    pub const fn new(
        src_stage: crate::render_pass::PipelineStage,
        dst_stage: crate::render_pass::PipelineStage,
    ) -> Self {
        Self {
            src_stage,
            dst_stage,
            memory_barriers: &[],
            buffer_barriers: &[],
            image_barriers: &[],
        }
    }

    /// Sets memory barriers
    pub const fn with_memory_barriers(mut self, barriers: &'a [MemoryBarrier]) -> Self {
        self.memory_barriers = barriers;
        self
    }

    /// Sets buffer barriers
    pub const fn with_buffer_barriers(mut self, barriers: &'a [BufferMemoryBarrier]) -> Self {
        self.buffer_barriers = barriers;
        self
    }

    /// Sets image barriers
    pub const fn with_image_barriers(mut self, barriers: &'a [ImageMemoryBarrier]) -> Self {
        self.image_barriers = barriers;
        self
    }
}

/// CPU-side fence for manual tracking
pub struct CpuFence {
    value: AtomicU64,
}

impl CpuFence {
    /// Creates a new unsignaled CPU fence
    pub const fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    /// Creates a signaled CPU fence
    pub const fn signaled() -> Self {
        Self {
            value: AtomicU64::new(1),
        }
    }

    /// Signals the fence
    pub fn signal(&self) {
        self.value.store(1, Ordering::Release);
    }

    /// Resets the fence to unsignaled state
    pub fn reset(&self) {
        self.value.store(0, Ordering::Release);
    }

    /// Checks if fence is signaled
    pub fn is_signaled(&self) -> bool {
        self.value.load(Ordering::Acquire) != 0
    }

    /// Spin-waits until fence is signaled
    pub fn wait(&self) {
        while !self.is_signaled() {
            core::hint::spin_loop();
        }
    }

    /// Tries to wait with limited spins
    pub fn try_wait(&self, max_spins: u32) -> bool {
        for _ in 0..max_spins {
            if self.is_signaled() {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }
}

impl Default for CpuFence {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline fence for frame tracking
pub struct TimelineFence {
    value: AtomicU64,
}

impl TimelineFence {
    /// Creates a new timeline fence
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    /// Gets the current value
    pub fn value(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Signals with a new value
    pub fn signal(&self, value: u64) {
        self.value.store(value, Ordering::Release);
    }

    /// Increments and signals
    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Waits until value is at least `target`
    pub fn wait_for(&self, target: u64) {
        while self.value.load(Ordering::Acquire) < target {
            core::hint::spin_loop();
        }
    }

    /// Checks if value is at least `target`
    pub fn is_complete(&self, target: u64) -> bool {
        self.value.load(Ordering::Acquire) >= target
    }
}

impl Default for TimelineFence {
    fn default() -> Self {
        Self::new(0)
    }
}
