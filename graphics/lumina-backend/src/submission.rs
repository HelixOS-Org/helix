//! Command Submission
//!
//! Command buffer submission and synchronization.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::command::CommandBuffer;
use crate::queue::{QueueFamily, QueueHandle};
use crate::swapchain::SwapchainHandle;

// ============================================================================
// Fence
// ============================================================================

/// Fence for CPU-GPU synchronization.
#[derive(Debug)]
pub struct Fence {
    /// Fence ID.
    pub id: u64,
    /// Signaled value.
    pub signaled: bool,
    /// Debug label.
    pub label: Option<String>,
}

impl Fence {
    /// Create a new fence.
    pub fn new(id: u64, signaled: bool) -> Self {
        Self {
            id,
            signaled,
            label: None,
        }
    }

    /// Check if signaled.
    pub fn is_signaled(&self) -> bool {
        self.signaled
    }

    /// Wait for signal.
    pub fn wait(&mut self) {
        // Backend-specific implementation
        self.signaled = true;
    }

    /// Reset fence.
    pub fn reset(&mut self) {
        self.signaled = false;
    }
}

// ============================================================================
// Fence Handle
// ============================================================================

/// Handle to a fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FenceHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl FenceHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get generation.
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

// ============================================================================
// Semaphore
// ============================================================================

/// Semaphore for GPU-GPU synchronization.
#[derive(Debug)]
pub struct Semaphore {
    /// Semaphore ID.
    pub id: u64,
    /// Debug label.
    pub label: Option<String>,
}

impl Semaphore {
    /// Create a new semaphore.
    pub fn new(id: u64) -> Self {
        Self { id, label: None }
    }
}

// ============================================================================
// Semaphore Handle
// ============================================================================

/// Handle to a semaphore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemaphoreHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl SemaphoreHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get generation.
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

// ============================================================================
// Timeline Semaphore
// ============================================================================

/// Timeline semaphore for GPU synchronization with values.
#[derive(Debug)]
pub struct TimelineSemaphore {
    /// Semaphore ID.
    pub id: u64,
    /// Current value.
    pub value: AtomicU64,
    /// Debug label.
    pub label: Option<String>,
}

impl TimelineSemaphore {
    /// Create a new timeline semaphore.
    pub fn new(id: u64, initial_value: u64) -> Self {
        Self {
            id,
            value: AtomicU64::new(initial_value),
            label: None,
        }
    }

    /// Get current value.
    pub fn current_value(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Signal with value.
    pub fn signal(&self, value: u64) {
        self.value.store(value, Ordering::Release);
    }

    /// Wait for value.
    pub fn wait(&self, value: u64) {
        while self.value.load(Ordering::Acquire) < value {
            core::hint::spin_loop();
        }
    }
}

// ============================================================================
// Timeline Semaphore Handle
// ============================================================================

/// Handle to a timeline semaphore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineSemaphoreHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl TimelineSemaphoreHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get index.
    pub fn index(&self) -> u32 {
        self.index
    }
}

// ============================================================================
// Submit Info
// ============================================================================

/// Pipeline stage flags for synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Top of pipe.
    TopOfPipe,
    /// Draw indirect.
    DrawIndirect,
    /// Vertex input.
    VertexInput,
    /// Vertex shader.
    VertexShader,
    /// Tessellation control.
    TessellationControl,
    /// Tessellation evaluation.
    TessellationEvaluation,
    /// Geometry shader.
    GeometryShader,
    /// Fragment shader.
    FragmentShader,
    /// Early fragment tests.
    EarlyFragmentTests,
    /// Late fragment tests.
    LateFragmentTests,
    /// Color attachment output.
    ColorAttachmentOutput,
    /// Compute shader.
    ComputeShader,
    /// Transfer.
    Transfer,
    /// Bottom of pipe.
    BottomOfPipe,
    /// Host.
    Host,
    /// All graphics.
    AllGraphics,
    /// All commands.
    AllCommands,
    /// Ray tracing shader.
    RayTracingShader,
    /// Acceleration structure build.
    AccelerationStructureBuild,
    /// Task shader.
    TaskShader,
    /// Mesh shader.
    MeshShader,
}

/// Semaphore wait info.
#[derive(Debug, Clone)]
pub struct SemaphoreWaitInfo {
    /// Semaphore to wait on.
    pub semaphore: SemaphoreHandle,
    /// Pipeline stage to wait at.
    pub stage: PipelineStage,
}

/// Timeline semaphore wait info.
#[derive(Debug, Clone)]
pub struct TimelineWaitInfo {
    /// Timeline semaphore to wait on.
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to wait for.
    pub value: u64,
    /// Pipeline stage to wait at.
    pub stage: PipelineStage,
}

/// Semaphore signal info.
#[derive(Debug, Clone)]
pub struct SemaphoreSignalInfo {
    /// Semaphore to signal.
    pub semaphore: SemaphoreHandle,
}

/// Timeline semaphore signal info.
#[derive(Debug, Clone)]
pub struct TimelineSignalInfo {
    /// Timeline semaphore to signal.
    pub semaphore: TimelineSemaphoreHandle,
    /// Value to signal.
    pub value: u64,
}

/// Command buffer submission info.
#[derive(Debug, Clone)]
pub struct SubmitInfo {
    /// Wait semaphores.
    pub wait_semaphores: Vec<SemaphoreWaitInfo>,
    /// Wait timeline semaphores.
    pub wait_timeline_semaphores: Vec<TimelineWaitInfo>,
    /// Command buffer indices.
    pub command_buffers: Vec<u32>,
    /// Signal semaphores.
    pub signal_semaphores: Vec<SemaphoreSignalInfo>,
    /// Signal timeline semaphores.
    pub signal_timeline_semaphores: Vec<TimelineSignalInfo>,
}

impl Default for SubmitInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SubmitInfo {
    /// Create a new submit info.
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            wait_timeline_semaphores: Vec::new(),
            command_buffers: Vec::new(),
            signal_semaphores: Vec::new(),
            signal_timeline_semaphores: Vec::new(),
        }
    }

    /// Add a wait semaphore.
    pub fn wait(mut self, semaphore: SemaphoreHandle, stage: PipelineStage) -> Self {
        self.wait_semaphores
            .push(SemaphoreWaitInfo { semaphore, stage });
        self
    }

    /// Add a signal semaphore.
    pub fn signal(mut self, semaphore: SemaphoreHandle) -> Self {
        self.signal_semaphores
            .push(SemaphoreSignalInfo { semaphore });
        self
    }

    /// Add a command buffer.
    pub fn command_buffer(mut self, index: u32) -> Self {
        self.command_buffers.push(index);
        self
    }
}

// ============================================================================
// Present Info
// ============================================================================

/// Present info.
#[derive(Debug, Clone)]
pub struct PresentInfo {
    /// Wait semaphores.
    pub wait_semaphores: Vec<SemaphoreHandle>,
    /// Swapchains.
    pub swapchains: Vec<SwapchainHandle>,
    /// Image indices.
    pub image_indices: Vec<u32>,
}

impl Default for PresentInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentInfo {
    /// Create a new present info.
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            swapchains: Vec::new(),
            image_indices: Vec::new(),
        }
    }

    /// Add a wait semaphore.
    pub fn wait(mut self, semaphore: SemaphoreHandle) -> Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Add a swapchain.
    pub fn swapchain(mut self, swapchain: SwapchainHandle, image_index: u32) -> Self {
        self.swapchains.push(swapchain);
        self.image_indices.push(image_index);
        self
    }
}

// ============================================================================
// Present Result
// ============================================================================

/// Present result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentResult {
    /// Success.
    Success,
    /// Suboptimal.
    Suboptimal,
    /// Out of date.
    OutOfDate,
    /// Surface lost.
    SurfaceLost,
    /// Device lost.
    DeviceLost,
}

impl PresentResult {
    /// Check if success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Suboptimal)
    }

    /// Check if needs recreation.
    pub fn needs_recreation(&self) -> bool {
        matches!(self, Self::OutOfDate | Self::SurfaceLost)
    }
}

// ============================================================================
// Submission Manager
// ============================================================================

/// Submission tracking.
#[derive(Debug)]
struct SubmissionRecord {
    /// Submission ID.
    id: u64,
    /// Queue.
    queue: QueueHandle,
    /// Fence.
    fence: Option<FenceHandle>,
    /// Frame.
    frame: u64,
}

/// Submission manager.
pub struct SubmissionManager {
    /// Next submission ID.
    next_id: AtomicU64,
    /// Pending submissions.
    pending: Vec<SubmissionRecord>,
    /// Fences.
    fences: Vec<Option<Fence>>,
    /// Semaphores.
    semaphores: Vec<Option<Semaphore>>,
    /// Timeline semaphores.
    timeline_semaphores: Vec<Option<TimelineSemaphore>>,
    /// Free fence indices.
    free_fences: Vec<u32>,
    /// Free semaphore indices.
    free_semaphores: Vec<u32>,
    /// Free timeline indices.
    free_timelines: Vec<u32>,
    /// Fence generations.
    fence_generations: Vec<u32>,
    /// Semaphore generations.
    semaphore_generations: Vec<u32>,
    /// Timeline generations.
    timeline_generations: Vec<u32>,
    /// Current frame.
    current_frame: u64,
}

impl SubmissionManager {
    /// Create a new submission manager.
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(0),
            pending: Vec::new(),
            fences: Vec::new(),
            semaphores: Vec::new(),
            timeline_semaphores: Vec::new(),
            free_fences: Vec::new(),
            free_semaphores: Vec::new(),
            free_timelines: Vec::new(),
            fence_generations: Vec::new(),
            semaphore_generations: Vec::new(),
            timeline_generations: Vec::new(),
            current_frame: 0,
        }
    }

    /// Create a fence.
    pub fn create_fence(&mut self, signaled: bool) -> FenceHandle {
        let index = if let Some(index) = self.free_fences.pop() {
            index
        } else {
            let index = self.fences.len() as u32;
            self.fences.push(None);
            self.fence_generations.push(0);
            index
        };

        let generation = self.fence_generations[index as usize];
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.fences[index as usize] = Some(Fence::new(id, signaled));

        FenceHandle::new(index, generation)
    }

    /// Create a semaphore.
    pub fn create_semaphore(&mut self) -> SemaphoreHandle {
        let index = if let Some(index) = self.free_semaphores.pop() {
            index
        } else {
            let index = self.semaphores.len() as u32;
            self.semaphores.push(None);
            self.semaphore_generations.push(0);
            index
        };

        let generation = self.semaphore_generations[index as usize];
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.semaphores[index as usize] = Some(Semaphore::new(id));

        SemaphoreHandle::new(index, generation)
    }

    /// Create a timeline semaphore.
    pub fn create_timeline_semaphore(&mut self, initial_value: u64) -> TimelineSemaphoreHandle {
        let index = if let Some(index) = self.free_timelines.pop() {
            index
        } else {
            let index = self.timeline_semaphores.len() as u32;
            self.timeline_semaphores.push(None);
            self.timeline_generations.push(0);
            index
        };

        let generation = self.timeline_generations[index as usize];
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.timeline_semaphores[index as usize] = Some(TimelineSemaphore::new(id, initial_value));

        TimelineSemaphoreHandle::new(index, generation)
    }

    /// Get fence.
    pub fn get_fence(&self, handle: FenceHandle) -> Option<&Fence> {
        let index = handle.index() as usize;
        if index >= self.fences.len() {
            return None;
        }
        if self.fence_generations[index] != handle.generation() {
            return None;
        }
        self.fences[index].as_ref()
    }

    /// Get fence mutably.
    pub fn get_fence_mut(&mut self, handle: FenceHandle) -> Option<&mut Fence> {
        let index = handle.index() as usize;
        if index >= self.fences.len() {
            return None;
        }
        if self.fence_generations[index] != handle.generation() {
            return None;
        }
        self.fences[index].as_mut()
    }

    /// Wait for fence.
    pub fn wait_fence(&mut self, handle: FenceHandle) {
        if let Some(fence) = self.get_fence_mut(handle) {
            fence.wait();
        }
    }

    /// Reset fence.
    pub fn reset_fence(&mut self, handle: FenceHandle) {
        if let Some(fence) = self.get_fence_mut(handle) {
            fence.reset();
        }
    }

    /// Check if fence is signaled.
    pub fn is_fence_signaled(&self, handle: FenceHandle) -> bool {
        self.get_fence(handle)
            .map(|f| f.is_signaled())
            .unwrap_or(false)
    }

    /// Destroy fence.
    pub fn destroy_fence(&mut self, handle: FenceHandle) {
        let index = handle.index() as usize;
        if index < self.fences.len() && self.fence_generations[index] == handle.generation() {
            self.fences[index] = None;
            self.fence_generations[index] = self.fence_generations[index].wrapping_add(1);
            self.free_fences.push(index as u32);
        }
    }

    /// Destroy semaphore.
    pub fn destroy_semaphore(&mut self, handle: SemaphoreHandle) {
        let index = handle.index() as usize;
        if index < self.semaphores.len() && self.semaphore_generations[index] == handle.generation()
        {
            self.semaphores[index] = None;
            self.semaphore_generations[index] = self.semaphore_generations[index].wrapping_add(1);
            self.free_semaphores.push(index as u32);
        }
    }

    /// Submit to queue.
    pub fn submit(
        &mut self,
        queue: QueueHandle,
        info: &SubmitInfo,
        fence: Option<FenceHandle>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.pending.push(SubmissionRecord {
            id,
            queue,
            fence,
            frame: self.current_frame,
        });
        id
    }

    /// Advance frame.
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;

        // Clean up completed submissions
        self.pending.retain(|record| {
            if let Some(fence_handle) = record.fence {
                if let Some(fence) = self.get_fence(fence_handle) {
                    return !fence.is_signaled();
                }
            }
            false
        });
    }

    /// Wait for idle.
    pub fn wait_idle(&mut self) {
        for record in &self.pending {
            if let Some(fence_handle) = record.fence {
                self.wait_fence(fence_handle);
            }
        }
        self.pending.clear();
    }

    /// Get pending submission count.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame
    }
}

impl Default for SubmissionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Frame Synchronization
// ============================================================================

/// Per-frame synchronization objects.
pub struct FrameSync {
    /// Image available semaphore.
    pub image_available: SemaphoreHandle,
    /// Render finished semaphore.
    pub render_finished: SemaphoreHandle,
    /// In-flight fence.
    pub in_flight_fence: FenceHandle,
}

/// Frame synchronization manager.
pub struct FrameSyncManager {
    /// Frame syncs.
    frames: Vec<FrameSync>,
    /// Current frame index.
    current_frame: usize,
    /// Max frames in flight.
    max_frames_in_flight: usize,
}

impl FrameSyncManager {
    /// Create a new frame sync manager.
    pub fn new(submission_manager: &mut SubmissionManager, max_frames: usize) -> Self {
        let frames = (0..max_frames)
            .map(|_| FrameSync {
                image_available: submission_manager.create_semaphore(),
                render_finished: submission_manager.create_semaphore(),
                in_flight_fence: submission_manager.create_fence(true),
            })
            .collect();

        Self {
            frames,
            current_frame: 0,
            max_frames_in_flight: max_frames,
        }
    }

    /// Get current frame sync.
    pub fn current(&self) -> &FrameSync {
        &self.frames[self.current_frame]
    }

    /// Advance to next frame.
    pub fn advance(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
    }

    /// Get current frame index.
    pub fn current_index(&self) -> usize {
        self.current_frame
    }

    /// Get max frames in flight.
    pub fn max_frames(&self) -> usize {
        self.max_frames_in_flight
    }
}
