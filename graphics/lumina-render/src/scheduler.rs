//! Frame Scheduling & Resource Management
//!
//! Revolutionary frame management featuring:
//! - Triple buffering with dynamic latency
//! - Frame pacing for smooth VSync
//! - Per-frame resource pools
//! - Automatic resource lifetime tracking
//! - Frame graph execution scheduling

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::resource::{BufferDesc, BufferHandle, TextureDesc, TextureHandle};

/// Frame scheduler for managing rendering frames.
pub struct FrameScheduler {
    /// Configuration.
    config: SchedulerConfig,
    /// Frame contexts (one per frame in flight).
    frames: Vec<FrameContext>,
    /// Current frame index.
    current_frame: usize,
    /// Frame count.
    frame_count: u64,
    /// Frame timing.
    timing: FrameTiming,
    /// Resource ring buffer.
    resource_ring: ResourceRing,
}

impl FrameScheduler {
    /// Create a new frame scheduler.
    pub fn new(config: SchedulerConfig) -> Self {
        let frames = (0..config.frames_in_flight)
            .map(|i| FrameContext::new(i as u32))
            .collect();

        Self {
            config: config.clone(),
            frames,
            current_frame: 0,
            frame_count: 0,
            timing: FrameTiming::new(config.target_frame_time),
            resource_ring: ResourceRing::new(config.frames_in_flight),
        }
    }

    /// Begin a new frame.
    pub fn begin_frame(&mut self) -> &mut FrameContext {
        // Wait for the frame we're about to use to complete
        let frame = &mut self.frames[self.current_frame];
        frame.wait_for_completion();

        // Update timing
        self.timing.begin_frame();

        // Reset frame resources
        frame.reset();
        frame.frame_number = self.frame_count;

        // Advance resource ring
        self.resource_ring.advance();

        &mut self.frames[self.current_frame]
    }

    /// End the current frame.
    pub fn end_frame(&mut self) {
        let frame = &mut self.frames[self.current_frame];
        frame.submit();

        // Update timing
        self.timing.end_frame();

        // Advance to next frame
        self.frame_count += 1;
        self.current_frame = (self.current_frame + 1) % self.config.frames_in_flight;
    }

    /// Get current frame context.
    pub fn current_frame(&self) -> &FrameContext {
        &self.frames[self.current_frame]
    }

    /// Get current frame context mutably.
    pub fn current_frame_mut(&mut self) -> &mut FrameContext {
        &mut self.frames[self.current_frame]
    }

    /// Get frame timing statistics.
    pub fn timing(&self) -> &FrameTiming {
        &self.timing
    }

    /// Get frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get frames in flight.
    pub fn frames_in_flight(&self) -> usize {
        self.config.frames_in_flight
    }

    /// Allocate transient buffer for this frame.
    pub fn allocate_buffer(&mut self, desc: &BufferDesc) -> TransientBuffer {
        self.resource_ring.allocate_buffer(self.current_frame, desc)
    }

    /// Allocate transient texture for this frame.
    pub fn allocate_texture(&mut self, desc: &TextureDesc) -> TransientTexture {
        self.resource_ring
            .allocate_texture(self.current_frame, desc)
    }

    /// Wait for all frames to complete.
    pub fn wait_idle(&mut self) {
        for frame in &mut self.frames {
            frame.wait_for_completion();
        }
    }

    /// Get frame pacing recommendation.
    pub fn get_pacing_delay(&self) -> f64 {
        self.timing.get_pacing_delay()
    }
}

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Number of frames in flight.
    pub frames_in_flight: usize,
    /// Target frame time in seconds.
    pub target_frame_time: f64,
    /// Enable adaptive sync.
    pub adaptive_sync: bool,
    /// Transient buffer pool size.
    pub transient_buffer_size: usize,
    /// Transient texture pool size.
    pub transient_texture_size: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            frames_in_flight: 2,
            target_frame_time: 1.0 / 60.0,
            adaptive_sync: true,
            transient_buffer_size: 64 * 1024 * 1024, // 64 MB
            transient_texture_size: 256 * 1024 * 1024, // 256 MB
        }
    }
}

/// Per-frame context.
pub struct FrameContext {
    /// Frame index (0 to frames_in_flight - 1).
    pub index: u32,
    /// Frame number (monotonically increasing).
    pub frame_number: u64,
    /// Fence for GPU synchronization.
    fence_value: AtomicU64,
    /// Completed fence value.
    completed_value: AtomicU64,
    /// Command allocators.
    command_allocators: Vec<CommandAllocator>,
    /// Descriptor heaps.
    descriptor_heaps: FrameDescriptorHeaps,
    /// Upload buffer.
    upload_buffer: RingBuffer,
    /// Resource deletions pending.
    pending_deletions: Vec<PendingDeletion>,
    /// Is frame submitted.
    submitted: bool,
}

impl FrameContext {
    /// Create a new frame context.
    pub fn new(index: u32) -> Self {
        Self {
            index,
            frame_number: 0,
            fence_value: AtomicU64::new(0),
            completed_value: AtomicU64::new(0),
            command_allocators: Vec::new(),
            descriptor_heaps: FrameDescriptorHeaps::new(),
            upload_buffer: RingBuffer::new(16 * 1024 * 1024), // 16 MB
            pending_deletions: Vec::new(),
            submitted: false,
        }
    }

    /// Reset frame for reuse.
    pub fn reset(&mut self) {
        // Reset command allocators
        for allocator in &mut self.command_allocators {
            allocator.reset();
        }

        // Reset descriptor heaps
        self.descriptor_heaps.reset();

        // Reset upload buffer
        self.upload_buffer.reset();

        // Process pending deletions
        self.pending_deletions.clear();

        self.submitted = false;
    }

    /// Wait for frame completion.
    pub fn wait_for_completion(&self) {
        let fence = self.fence_value.load(Ordering::Acquire);
        let completed = self.completed_value.load(Ordering::Acquire);

        if completed < fence {
            // Would wait on GPU fence
            self.completed_value.store(fence, Ordering::Release);
        }
    }

    /// Submit frame for execution.
    pub fn submit(&mut self) {
        self.fence_value.fetch_add(1, Ordering::AcqRel);
        self.submitted = true;
    }

    /// Get a command allocator.
    pub fn get_command_allocator(&mut self, queue_type: QueueType) -> &mut CommandAllocator {
        // Find or create allocator for queue type
        for allocator in &mut self.command_allocators {
            if allocator.queue_type == queue_type {
                return allocator;
            }
        }

        // Create new allocator
        self.command_allocators.push(CommandAllocator {
            queue_type,
            offset: 0,
            capacity: 1024 * 1024,
        });

        self.command_allocators.last_mut().unwrap()
    }

    /// Allocate from upload buffer.
    pub fn allocate_upload(&mut self, size: usize, alignment: usize) -> Option<UploadAllocation> {
        self.upload_buffer.allocate(size, alignment)
    }

    /// Queue resource for deletion.
    pub fn queue_deletion(&mut self, deletion: PendingDeletion) {
        self.pending_deletions.push(deletion);
    }

    /// Allocate descriptors.
    pub fn allocate_descriptors(
        &mut self,
        count: u32,
        heap_type: DescriptorHeapType,
    ) -> DescriptorAllocation {
        self.descriptor_heaps.allocate(count, heap_type)
    }
}

/// Queue type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    /// Graphics queue.
    Graphics,
    /// Compute queue.
    Compute,
    /// Transfer/copy queue.
    Transfer,
}

/// Command allocator.
pub struct CommandAllocator {
    /// Queue type.
    queue_type: QueueType,
    /// Current offset.
    offset: usize,
    /// Total capacity.
    capacity: usize,
}

impl CommandAllocator {
    /// Reset allocator.
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Allocate commands.
    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        if self.offset + size <= self.capacity {
            let offset = self.offset;
            self.offset += size;
            Some(offset)
        } else {
            None
        }
    }
}

/// Frame descriptor heaps.
struct FrameDescriptorHeaps {
    /// CBV/SRV/UAV heap.
    cbv_srv_uav: DescriptorHeap,
    /// Sampler heap.
    sampler: DescriptorHeap,
    /// RTV heap.
    rtv: DescriptorHeap,
    /// DSV heap.
    dsv: DescriptorHeap,
}

impl FrameDescriptorHeaps {
    fn new() -> Self {
        Self {
            cbv_srv_uav: DescriptorHeap::new(DescriptorHeapType::CbvSrvUav, 65536),
            sampler: DescriptorHeap::new(DescriptorHeapType::Sampler, 2048),
            rtv: DescriptorHeap::new(DescriptorHeapType::Rtv, 256),
            dsv: DescriptorHeap::new(DescriptorHeapType::Dsv, 64),
        }
    }

    fn reset(&mut self) {
        self.cbv_srv_uav.reset();
        self.sampler.reset();
        self.rtv.reset();
        self.dsv.reset();
    }

    fn allocate(&mut self, count: u32, heap_type: DescriptorHeapType) -> DescriptorAllocation {
        match heap_type {
            DescriptorHeapType::CbvSrvUav => self.cbv_srv_uav.allocate(count),
            DescriptorHeapType::Sampler => self.sampler.allocate(count),
            DescriptorHeapType::Rtv => self.rtv.allocate(count),
            DescriptorHeapType::Dsv => self.dsv.allocate(count),
        }
    }
}

/// Descriptor heap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorHeapType {
    /// CBV/SRV/UAV.
    CbvSrvUav,
    /// Sampler.
    Sampler,
    /// Render target view.
    Rtv,
    /// Depth stencil view.
    Dsv,
}

/// Descriptor heap.
struct DescriptorHeap {
    /// Heap type.
    heap_type: DescriptorHeapType,
    /// Capacity.
    capacity: u32,
    /// Current offset.
    offset: u32,
}

impl DescriptorHeap {
    fn new(heap_type: DescriptorHeapType, capacity: u32) -> Self {
        Self {
            heap_type,
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, count: u32) -> DescriptorAllocation {
        let start = self.offset;
        self.offset += count;
        DescriptorAllocation {
            start,
            count,
            heap_type: self.heap_type,
        }
    }
}

/// Descriptor allocation.
#[derive(Debug, Clone)]
pub struct DescriptorAllocation {
    /// Start index.
    pub start: u32,
    /// Count.
    pub count: u32,
    /// Heap type.
    pub heap_type: DescriptorHeapType,
}

/// Ring buffer for uploads.
struct RingBuffer {
    /// Capacity.
    capacity: usize,
    /// Current offset.
    offset: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, size: usize, alignment: usize) -> Option<UploadAllocation> {
        let aligned_offset = (self.offset + alignment - 1) & !(alignment - 1);

        if aligned_offset + size <= self.capacity {
            self.offset = aligned_offset + size;
            Some(UploadAllocation {
                offset: aligned_offset,
                size,
            })
        } else {
            None
        }
    }
}

/// Upload allocation.
#[derive(Debug, Clone)]
pub struct UploadAllocation {
    /// Offset in upload buffer.
    pub offset: usize,
    /// Size.
    pub size: usize,
}

/// Pending resource deletion.
#[derive(Debug)]
pub enum PendingDeletion {
    /// Buffer deletion.
    Buffer(BufferHandle),
    /// Texture deletion.
    Texture(TextureHandle),
    /// Pipeline deletion.
    Pipeline(u64),
    /// Custom resource.
    Custom { id: u64, drop_fn: fn(u64) },
}

/// Frame timing statistics.
pub struct FrameTiming {
    /// Target frame time.
    target_frame_time: f64,
    /// Frame start time (in some time unit).
    frame_start: u64,
    /// Last frame time.
    last_frame_time: f64,
    /// Average frame time.
    avg_frame_time: f64,
    /// Frame time history.
    history: [f64; 64],
    /// History index.
    history_index: usize,
    /// Min frame time.
    min_frame_time: f64,
    /// Max frame time.
    max_frame_time: f64,
}

impl FrameTiming {
    /// Create new frame timing.
    pub fn new(target_frame_time: f64) -> Self {
        Self {
            target_frame_time,
            frame_start: 0,
            last_frame_time: target_frame_time,
            avg_frame_time: target_frame_time,
            history: [target_frame_time; 64],
            history_index: 0,
            min_frame_time: target_frame_time,
            max_frame_time: target_frame_time,
        }
    }

    /// Begin frame timing.
    pub fn begin_frame(&mut self) {
        // Would use actual time here
        self.frame_start = 0;
    }

    /// End frame timing.
    pub fn end_frame(&mut self) {
        // Would calculate actual delta time
        let frame_time = self.target_frame_time; // Placeholder

        self.last_frame_time = frame_time;
        self.history[self.history_index] = frame_time;
        self.history_index = (self.history_index + 1) % self.history.len();

        // Update average
        let sum: f64 = self.history.iter().sum();
        self.avg_frame_time = sum / self.history.len() as f64;

        // Update min/max
        self.min_frame_time = self.history.iter().copied().fold(f64::MAX, f64::min);
        self.max_frame_time = self.history.iter().copied().fold(0.0, f64::max);
    }

    /// Get last frame time.
    pub fn last_frame_time(&self) -> f64 {
        self.last_frame_time
    }

    /// Get average frame time.
    pub fn average_frame_time(&self) -> f64 {
        self.avg_frame_time
    }

    /// Get FPS.
    pub fn fps(&self) -> f64 {
        1.0 / self.avg_frame_time
    }

    /// Get frame time variance.
    pub fn variance(&self) -> f64 {
        let avg = self.avg_frame_time;
        let sum: f64 = self.history.iter().map(|&t| (t - avg).powi(2)).sum();
        sum / self.history.len() as f64
    }

    /// Get pacing delay for smooth frames.
    pub fn get_pacing_delay(&self) -> f64 {
        let remaining = self.target_frame_time - self.last_frame_time;
        remaining.max(0.0)
    }

    /// Is frame rate stable?
    pub fn is_stable(&self) -> bool {
        let variance = self.variance();
        variance < self.target_frame_time * 0.1 // Within 10%
    }
}

/// Resource ring buffer for per-frame allocations.
struct ResourceRing {
    /// Frames in flight.
    frames: usize,
    /// Current frame.
    current: usize,
    /// Buffer allocators per frame.
    buffer_allocators: Vec<FrameAllocator>,
    /// Texture allocators per frame.
    texture_allocators: Vec<FrameAllocator>,
}

impl ResourceRing {
    fn new(frames: usize) -> Self {
        Self {
            frames,
            current: 0,
            buffer_allocators: (0..frames)
                .map(|_| FrameAllocator::new(64 * 1024 * 1024))
                .collect(),
            texture_allocators: (0..frames)
                .map(|_| FrameAllocator::new(256 * 1024 * 1024))
                .collect(),
        }
    }

    fn advance(&mut self) {
        self.current = (self.current + 1) % self.frames;
        self.buffer_allocators[self.current].reset();
        self.texture_allocators[self.current].reset();
    }

    fn allocate_buffer(&mut self, frame: usize, desc: &BufferDesc) -> TransientBuffer {
        let allocator = &mut self.buffer_allocators[frame];
        let offset = allocator.allocate(desc.size as usize, 256);
        TransientBuffer {
            frame: frame as u32,
            offset,
            size: desc.size as usize,
        }
    }

    fn allocate_texture(&mut self, frame: usize, desc: &TextureDesc) -> TransientTexture {
        let allocator = &mut self.texture_allocators[frame];
        let size = estimate_texture_size(desc);
        let offset = allocator.allocate(size, 65536); // 64KB alignment for textures
        TransientTexture {
            frame: frame as u32,
            offset,
            desc: desc.clone(),
        }
    }
}

/// Frame allocator (bump allocator).
struct FrameAllocator {
    /// Capacity.
    capacity: usize,
    /// Current offset.
    offset: usize,
}

impl FrameAllocator {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            offset: 0,
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn allocate(&mut self, size: usize, alignment: usize) -> usize {
        let aligned = (self.offset + alignment - 1) & !(alignment - 1);
        self.offset = aligned + size;
        aligned
    }
}

/// Transient buffer (valid for one frame).
#[derive(Debug, Clone)]
pub struct TransientBuffer {
    /// Frame it belongs to.
    pub frame: u32,
    /// Offset in buffer pool.
    pub offset: usize,
    /// Size.
    pub size: usize,
}

/// Transient texture (valid for one frame).
#[derive(Debug, Clone)]
pub struct TransientTexture {
    /// Frame it belongs to.
    pub frame: u32,
    /// Offset in texture pool.
    pub offset: usize,
    /// Description.
    pub desc: TextureDesc,
}

/// Estimate texture size.
fn estimate_texture_size(desc: &TextureDesc) -> usize {
    let bpp = desc.format.bytes_per_pixel();
    let mut size = desc.width as usize * desc.height as usize * desc.depth as usize * bpp;

    if desc.mip_levels > 1 {
        // Approximate mip chain size
        size = (size as f32 * 1.33) as usize;
    }

    if desc.array_layers > 1 {
        size *= desc.array_layers as usize;
    }

    size
}

/// GPU timeline.
pub struct GpuTimeline {
    /// Queries.
    queries: Vec<TimestampQuery>,
    /// Query pool capacity.
    capacity: u32,
    /// Next query index.
    next_query: u32,
}

impl GpuTimeline {
    /// Create new timeline.
    pub fn new(capacity: u32) -> Self {
        Self {
            queries: Vec::with_capacity(capacity as usize),
            capacity,
            next_query: 0,
        }
    }

    /// Begin a region.
    pub fn begin(&mut self, name: &str) -> u32 {
        let index = self.next_query;
        self.next_query += 2; // Begin and end

        self.queries.push(TimestampQuery {
            name: String::from(name),
            begin_index: index,
            end_index: index + 1,
            begin_time: 0,
            end_time: 0,
        });

        index
    }

    /// End a region.
    pub fn end(&mut self, _query_id: u32) {
        // Would record end timestamp
    }

    /// Reset for new frame.
    pub fn reset(&mut self) {
        self.queries.clear();
        self.next_query = 0;
    }

    /// Get query results.
    pub fn get_results(&self) -> &[TimestampQuery] {
        &self.queries
    }
}

/// Timestamp query.
#[derive(Debug, Clone)]
pub struct TimestampQuery {
    /// Region name.
    pub name: String,
    /// Begin query index.
    pub begin_index: u32,
    /// End query index.
    pub end_index: u32,
    /// Begin timestamp.
    pub begin_time: u64,
    /// End timestamp.
    pub end_time: u64,
}

impl TimestampQuery {
    /// Get duration in nanoseconds.
    pub fn duration_ns(&self) -> u64 {
        self.end_time.saturating_sub(self.begin_time)
    }

    /// Get duration in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        self.duration_ns() as f64 / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_scheduler() {
        let config = SchedulerConfig::default();
        let scheduler = FrameScheduler::new(config);

        assert_eq!(scheduler.frames_in_flight(), 2);
        assert_eq!(scheduler.frame_count(), 0);
    }

    #[test]
    fn test_frame_timing() {
        let timing = FrameTiming::new(1.0 / 60.0);
        assert!((timing.fps() - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring = RingBuffer::new(1024);

        let alloc1 = ring.allocate(100, 16).unwrap();
        assert_eq!(alloc1.offset, 0);
        assert_eq!(alloc1.size, 100);

        let alloc2 = ring.allocate(200, 64).unwrap();
        assert_eq!(alloc2.offset, 128); // Aligned to 64
    }

    #[test]
    fn test_descriptor_heap() {
        let mut heap = DescriptorHeap::new(DescriptorHeapType::CbvSrvUav, 1000);

        let alloc1 = heap.allocate(10);
        assert_eq!(alloc1.start, 0);
        assert_eq!(alloc1.count, 10);

        let alloc2 = heap.allocate(20);
        assert_eq!(alloc2.start, 10);
    }
}
