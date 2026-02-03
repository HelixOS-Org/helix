//! Barrier Management - Optimal GPU Synchronization
//!
//! This module provides efficient barrier generation and batching for GPU synchronization.

use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr, BitOrAssign};

use crate::graph::{ResourceId, SubresourceRange};
use crate::resource::ResourceState;

/// Pipeline stages for synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineStage(u32);

impl PipelineStage {
    /// Top of pipe.
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
    /// Draw indirect.
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input.
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader.
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control shader.
    pub const TESSELLATION_CONTROL_SHADER: Self = Self(1 << 4);
    /// Tessellation evaluation shader.
    pub const TESSELLATION_EVALUATION_SHADER: Self = Self(1 << 5);
    /// Geometry shader.
    pub const GEOMETRY_SHADER: Self = Self(1 << 6);
    /// Fragment shader.
    pub const FRAGMENT_SHADER: Self = Self(1 << 7);
    /// Early fragment tests.
    pub const EARLY_FRAGMENT_TESTS: Self = Self(1 << 8);
    /// Late fragment tests.
    pub const LATE_FRAGMENT_TESTS: Self = Self(1 << 9);
    /// Color attachment output.
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(1 << 10);
    /// Compute shader.
    pub const COMPUTE_SHADER: Self = Self(1 << 11);
    /// Transfer.
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe.
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host.
    pub const HOST: Self = Self(1 << 14);
    /// All graphics.
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands.
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Conditional rendering.
    pub const CONDITIONAL_RENDERING: Self = Self(1 << 17);
    /// Acceleration structure build.
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 18);
    /// Ray tracing shader.
    pub const RAY_TRACING_SHADER: Self = Self(1 << 19);
    /// Task shader.
    pub const TASK_SHADER: Self = Self(1 << 20);
    /// Mesh shader.
    pub const MESH_SHADER: Self = Self(1 << 21);

    /// None.
    pub const NONE: Self = Self(0);

    /// Check if contains stage.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Raw value.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// From raw value.
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Check if empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for PipelineStage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PipelineStage {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for PipelineStage {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Default for PipelineStage {
    fn default() -> Self {
        Self::NONE
    }
}

/// Access flags for memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessFlags(u32);

impl AccessFlags {
    /// No access.
    pub const NONE: Self = Self(0);
    /// Indirect command read.
    pub const INDIRECT_COMMAND_READ: Self = Self(1 << 0);
    /// Index read.
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex attribute read.
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(1 << 2);
    /// Uniform read.
    pub const UNIFORM_READ: Self = Self(1 << 3);
    /// Input attachment read.
    pub const INPUT_ATTACHMENT_READ: Self = Self(1 << 4);
    /// Shader read.
    pub const SHADER_READ: Self = Self(1 << 5);
    /// Shader write.
    pub const SHADER_WRITE: Self = Self(1 << 6);
    /// Color attachment read.
    pub const COLOR_ATTACHMENT_READ: Self = Self(1 << 7);
    /// Color attachment write.
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(1 << 8);
    /// Depth/stencil attachment read.
    pub const DEPTH_STENCIL_ATTACHMENT_READ: Self = Self(1 << 9);
    /// Depth/stencil attachment write.
    pub const DEPTH_STENCIL_ATTACHMENT_WRITE: Self = Self(1 << 10);
    /// Transfer read.
    pub const TRANSFER_READ: Self = Self(1 << 11);
    /// Transfer write.
    pub const TRANSFER_WRITE: Self = Self(1 << 12);
    /// Host read.
    pub const HOST_READ: Self = Self(1 << 13);
    /// Host write.
    pub const HOST_WRITE: Self = Self(1 << 14);
    /// Memory read.
    pub const MEMORY_READ: Self = Self(1 << 15);
    /// Memory write.
    pub const MEMORY_WRITE: Self = Self(1 << 16);
    /// Acceleration structure read.
    pub const ACCELERATION_STRUCTURE_READ: Self = Self(1 << 17);
    /// Acceleration structure write.
    pub const ACCELERATION_STRUCTURE_WRITE: Self = Self(1 << 18);

    /// All reads.
    pub const ALL_READ: Self = Self(
        Self::INDIRECT_COMMAND_READ.0
            | Self::INDEX_READ.0
            | Self::VERTEX_ATTRIBUTE_READ.0
            | Self::UNIFORM_READ.0
            | Self::INPUT_ATTACHMENT_READ.0
            | Self::SHADER_READ.0
            | Self::COLOR_ATTACHMENT_READ.0
            | Self::DEPTH_STENCIL_ATTACHMENT_READ.0
            | Self::TRANSFER_READ.0
            | Self::HOST_READ.0
            | Self::MEMORY_READ.0,
    );

    /// All writes.
    pub const ALL_WRITE: Self = Self(
        Self::SHADER_WRITE.0
            | Self::COLOR_ATTACHMENT_WRITE.0
            | Self::DEPTH_STENCIL_ATTACHMENT_WRITE.0
            | Self::TRANSFER_WRITE.0
            | Self::HOST_WRITE.0
            | Self::MEMORY_WRITE.0,
    );

    /// Check if contains flags.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if any write.
    pub fn is_write(self) -> bool {
        (self.0 & Self::ALL_WRITE.0) != 0
    }

    /// Check if any read.
    pub fn is_read(self) -> bool {
        (self.0 & Self::ALL_READ.0) != 0
    }

    /// Raw value.
    pub fn bits(self) -> u32 {
        self.0
    }
}

impl BitOr for AccessFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for AccessFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for AccessFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Default for AccessFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// A memory/execution barrier.
#[derive(Debug, Clone)]
pub struct Barrier {
    /// Resource being transitioned.
    pub resource: ResourceId,
    /// Old resource state.
    pub old_state: ResourceState,
    /// New resource state.
    pub new_state: ResourceState,
    /// Source pipeline stages.
    pub src_stages: PipelineStage,
    /// Destination pipeline stages.
    pub dst_stages: PipelineStage,
    /// Source access flags.
    pub src_access: AccessFlags,
    /// Destination access flags.
    pub dst_access: AccessFlags,
    /// Subresource range.
    pub subresource: SubresourceRange,
}

impl Barrier {
    /// Create a new barrier.
    pub fn new(
        resource: ResourceId,
        old_state: ResourceState,
        new_state: ResourceState,
    ) -> Self {
        Self {
            resource,
            old_state,
            new_state,
            src_stages: old_state.to_pipeline_stage(),
            dst_stages: new_state.to_pipeline_stage(),
            src_access: old_state.to_access_flags(),
            dst_access: new_state.to_access_flags(),
            subresource: SubresourceRange::ALL,
        }
    }

    /// With custom stages.
    pub fn with_stages(mut self, src: PipelineStage, dst: PipelineStage) -> Self {
        self.src_stages = src;
        self.dst_stages = dst;
        self
    }

    /// With custom access.
    pub fn with_access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access = src;
        self.dst_access = dst;
        self
    }

    /// With subresource range.
    pub fn with_subresource(mut self, range: SubresourceRange) -> Self {
        self.subresource = range;
        self
    }

    /// Check if this is a layout transition.
    pub fn is_layout_transition(&self) -> bool {
        self.old_state != self.new_state
    }

    /// Check if this is an execution barrier only.
    pub fn is_execution_only(&self) -> bool {
        !self.is_layout_transition() && self.src_access == self.dst_access
    }

    /// Check if this can be merged with another barrier.
    pub fn can_merge(&self, other: &Barrier) -> bool {
        self.resource == other.resource
            && self.new_state == other.old_state
            && self.subresource.base_mip == other.subresource.base_mip
            && self.subresource.base_layer == other.subresource.base_layer
    }

    /// Merge with another barrier.
    pub fn merge(&mut self, other: &Barrier) {
        self.new_state = other.new_state;
        self.dst_stages = other.dst_stages;
        self.dst_access = other.dst_access;
    }
}

/// Batch of barriers to submit together.
#[derive(Debug, Clone, Default)]
pub struct BarrierBatch {
    /// Memory barriers.
    pub memory_barriers: Vec<MemoryBarrier>,
    /// Buffer barriers.
    pub buffer_barriers: Vec<BufferBarrier>,
    /// Image barriers.
    pub image_barriers: Vec<ImageBarrier>,
    /// Source stages.
    pub src_stages: PipelineStage,
    /// Destination stages.
    pub dst_stages: PipelineStage,
}

impl BarrierBatch {
    /// Create a new empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a memory barrier.
    pub fn add_memory(&mut self, barrier: MemoryBarrier) {
        self.src_stages |= barrier.src_stages;
        self.dst_stages |= barrier.dst_stages;
        self.memory_barriers.push(barrier);
    }

    /// Add a buffer barrier.
    pub fn add_buffer(&mut self, barrier: BufferBarrier) {
        self.src_stages |= barrier.src_stages;
        self.dst_stages |= barrier.dst_stages;
        self.buffer_barriers.push(barrier);
    }

    /// Add an image barrier.
    pub fn add_image(&mut self, barrier: ImageBarrier) {
        self.src_stages |= barrier.src_stages;
        self.dst_stages |= barrier.dst_stages;
        self.image_barriers.push(barrier);
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.memory_barriers.is_empty()
            && self.buffer_barriers.is_empty()
            && self.image_barriers.is_empty()
    }

    /// Get total barrier count.
    pub fn count(&self) -> usize {
        self.memory_barriers.len() + self.buffer_barriers.len() + self.image_barriers.len()
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.memory_barriers.clear();
        self.buffer_barriers.clear();
        self.image_barriers.clear();
        self.src_stages = PipelineStage::NONE;
        self.dst_stages = PipelineStage::NONE;
    }

    /// Optimize by merging compatible barriers.
    pub fn optimize(&mut self) {
        self.merge_image_barriers();
        self.merge_buffer_barriers();
    }

    fn merge_image_barriers(&mut self) {
        if self.image_barriers.len() < 2 {
            return;
        }

        // Sort by resource and base mip
        self.image_barriers.sort_by(|a, b| {
            a.resource
                .raw()
                .cmp(&b.resource.raw())
                .then(a.subresource.base_mip.cmp(&b.subresource.base_mip))
                .then(a.subresource.base_layer.cmp(&b.subresource.base_layer))
        });

        // Merge adjacent barriers for same resource
        let mut merged = Vec::new();
        let mut current: Option<ImageBarrier> = None;

        for barrier in self.image_barriers.drain(..) {
            if let Some(ref mut curr) = current {
                if curr.resource == barrier.resource
                    && curr.new_state == barrier.old_state
                    && Self::subresources_adjacent(&curr.subresource, &barrier.subresource)
                {
                    // Extend current barrier
                    curr.new_state = barrier.new_state;
                    curr.dst_stages |= barrier.dst_stages;
                    curr.dst_access |= barrier.dst_access;
                    curr.subresource.mip_count += barrier.subresource.mip_count;
                    continue;
                } else {
                    merged.push(current.take().unwrap());
                }
            }
            current = Some(barrier);
        }

        if let Some(curr) = current {
            merged.push(curr);
        }

        self.image_barriers = merged;
    }

    fn merge_buffer_barriers(&mut self) {
        if self.buffer_barriers.len() < 2 {
            return;
        }

        // Sort by resource and offset
        self.buffer_barriers.sort_by(|a, b| {
            a.resource
                .raw()
                .cmp(&b.resource.raw())
                .then(a.offset.cmp(&b.offset))
        });

        // Merge adjacent barriers
        let mut merged = Vec::new();
        let mut current: Option<BufferBarrier> = None;

        for barrier in self.buffer_barriers.drain(..) {
            if let Some(ref mut curr) = current {
                if curr.resource == barrier.resource
                    && curr.offset + curr.size == barrier.offset
                    && curr.src_stages == barrier.src_stages
                    && curr.dst_stages == barrier.dst_stages
                {
                    curr.size += barrier.size;
                    continue;
                } else {
                    merged.push(current.take().unwrap());
                }
            }
            current = Some(barrier);
        }

        if let Some(curr) = current {
            merged.push(curr);
        }

        self.buffer_barriers = merged;
    }

    fn subresources_adjacent(a: &SubresourceRange, b: &SubresourceRange) -> bool {
        a.base_layer == b.base_layer
            && a.layer_count == b.layer_count
            && a.base_mip + a.mip_count == b.base_mip
    }
}

/// Global memory barrier.
#[derive(Debug, Clone)]
pub struct MemoryBarrier {
    /// Source stages.
    pub src_stages: PipelineStage,
    /// Destination stages.
    pub dst_stages: PipelineStage,
    /// Source access.
    pub src_access: AccessFlags,
    /// Destination access.
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Create a new memory barrier.
    pub fn new(
        src_stages: PipelineStage,
        dst_stages: PipelineStage,
        src_access: AccessFlags,
        dst_access: AccessFlags,
    ) -> Self {
        Self {
            src_stages,
            dst_stages,
            src_access,
            dst_access,
        }
    }

    /// Full pipeline barrier.
    pub fn full() -> Self {
        Self {
            src_stages: PipelineStage::ALL_COMMANDS,
            dst_stages: PipelineStage::ALL_COMMANDS,
            src_access: AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE,
        }
    }

    /// Execution-only barrier.
    pub fn execution(src: PipelineStage, dst: PipelineStage) -> Self {
        Self {
            src_stages: src,
            dst_stages: dst,
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
        }
    }
}

/// Buffer memory barrier.
#[derive(Debug, Clone)]
pub struct BufferBarrier {
    /// Buffer resource.
    pub resource: ResourceId,
    /// Source stages.
    pub src_stages: PipelineStage,
    /// Destination stages.
    pub dst_stages: PipelineStage,
    /// Source access.
    pub src_access: AccessFlags,
    /// Destination access.
    pub dst_access: AccessFlags,
    /// Offset in buffer.
    pub offset: u64,
    /// Size of region.
    pub size: u64,
    /// Source queue family.
    pub src_queue_family: Option<u32>,
    /// Destination queue family.
    pub dst_queue_family: Option<u32>,
}

impl BufferBarrier {
    /// Create a new buffer barrier.
    pub fn new(resource: ResourceId, src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self {
            resource,
            src_stages: PipelineStage::ALL_COMMANDS,
            dst_stages: PipelineStage::ALL_COMMANDS,
            src_access,
            dst_access,
            offset: 0,
            size: u64::MAX, // Whole buffer
            src_queue_family: None,
            dst_queue_family: None,
        }
    }

    /// With specific stages.
    pub fn with_stages(mut self, src: PipelineStage, dst: PipelineStage) -> Self {
        self.src_stages = src;
        self.dst_stages = dst;
        self
    }

    /// With specific range.
    pub fn with_range(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }

    /// With queue family transfer.
    pub fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = Some(src);
        self.dst_queue_family = Some(dst);
        self
    }
}

/// Image memory barrier.
#[derive(Debug, Clone)]
pub struct ImageBarrier {
    /// Image resource.
    pub resource: ResourceId,
    /// Old layout/state.
    pub old_state: ResourceState,
    /// New layout/state.
    pub new_state: ResourceState,
    /// Source stages.
    pub src_stages: PipelineStage,
    /// Destination stages.
    pub dst_stages: PipelineStage,
    /// Source access.
    pub src_access: AccessFlags,
    /// Destination access.
    pub dst_access: AccessFlags,
    /// Subresource range.
    pub subresource: SubresourceRange,
    /// Source queue family.
    pub src_queue_family: Option<u32>,
    /// Destination queue family.
    pub dst_queue_family: Option<u32>,
}

impl ImageBarrier {
    /// Create a new image barrier.
    pub fn new(
        resource: ResourceId,
        old_state: ResourceState,
        new_state: ResourceState,
    ) -> Self {
        Self {
            resource,
            old_state,
            new_state,
            src_stages: old_state.to_pipeline_stage(),
            dst_stages: new_state.to_pipeline_stage(),
            src_access: old_state.to_access_flags(),
            dst_access: new_state.to_access_flags(),
            subresource: SubresourceRange::ALL,
            src_queue_family: None,
            dst_queue_family: None,
        }
    }

    /// With custom stages.
    pub fn with_stages(mut self, src: PipelineStage, dst: PipelineStage) -> Self {
        self.src_stages = src;
        self.dst_stages = dst;
        self
    }

    /// With subresource range.
    pub fn with_subresource(mut self, range: SubresourceRange) -> Self {
        self.subresource = range;
        self
    }

    /// With queue family transfer.
    pub fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = Some(src);
        self.dst_queue_family = Some(dst);
        self
    }
}

/// Barrier optimizer for minimizing synchronization overhead.
pub struct BarrierOptimizer {
    /// Pending barriers.
    pending: Vec<Barrier>,
    /// Optimization level.
    level: OptimizationLevel,
}

impl BarrierOptimizer {
    /// Create a new optimizer.
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            pending: Vec::new(),
            level,
        }
    }

    /// Add a barrier.
    pub fn add(&mut self, barrier: Barrier) {
        match self.level {
            OptimizationLevel::None => {
                self.pending.push(barrier);
            }
            OptimizationLevel::Basic => {
                self.add_with_basic_merge(barrier);
            }
            OptimizationLevel::Aggressive => {
                self.add_with_aggressive_merge(barrier);
            }
        }
    }

    /// Flush all pending barriers.
    pub fn flush(&mut self) -> BarrierBatch {
        let mut batch = BarrierBatch::new();

        for barrier in self.pending.drain(..) {
            batch.add_image(ImageBarrier::new(
                barrier.resource,
                barrier.old_state,
                barrier.new_state,
            ));
        }

        if self.level != OptimizationLevel::None {
            batch.optimize();
        }

        batch
    }

    fn add_with_basic_merge(&mut self, barrier: Barrier) {
        // Try to find existing barrier to merge
        for existing in &mut self.pending {
            if existing.can_merge(&barrier) {
                existing.merge(&barrier);
                return;
            }
        }
        self.pending.push(barrier);
    }

    fn add_with_aggressive_merge(&mut self, barrier: Barrier) {
        // Check if barrier is redundant
        if self.is_redundant(&barrier) {
            return;
        }

        // Try to find existing barrier to merge
        for existing in &mut self.pending {
            if existing.can_merge(&barrier) {
                existing.merge(&barrier);
                return;
            }
        }

        // Check if we can split and merge
        self.pending.push(barrier);
    }

    fn is_redundant(&self, barrier: &Barrier) -> bool {
        // A barrier is redundant if we already have a barrier that
        // transitions to the same state
        for existing in &self.pending {
            if existing.resource == barrier.resource
                && existing.new_state == barrier.new_state
                && existing.subresource.base_mip == barrier.subresource.base_mip
                && existing.subresource.base_layer == barrier.subresource.base_layer
            {
                return true;
            }
        }
        false
    }
}

/// Optimization level for barriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimization.
    None,
    /// Basic merging.
    Basic,
    /// Aggressive optimization.
    Aggressive,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ResourceId;

    #[test]
    fn test_pipeline_stage_operations() {
        let stages = PipelineStage::VERTEX_SHADER | PipelineStage::FRAGMENT_SHADER;
        assert!(stages.contains(PipelineStage::VERTEX_SHADER));
        assert!(!stages.contains(PipelineStage::COMPUTE_SHADER));
    }

    #[test]
    fn test_access_flags() {
        let access = AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE;
        assert!(access.is_read());
        assert!(access.is_write());
    }

    #[test]
    fn test_barrier_batch() {
        let mut batch = BarrierBatch::new();
        assert!(batch.is_empty());

        batch.add_memory(MemoryBarrier::full());
        assert_eq!(batch.count(), 1);
    }
}
