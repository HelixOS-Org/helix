//! # Frame Capture Engine
//!
//! Zero-copy frame capture system with minimal GPU overhead.
//! Uses ring buffers and async readback to avoid stalls.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
    CommandBufferData, CommandData, CommandType, FrameData, InspectorError, InspectorErrorKind,
    InspectorResult, RecordedCommand, RenderPassData, ResourceBinding, StateChange, SubmissionData,
    SyncPoint,
};

/// Capture engine for recording GPU frames
pub struct CaptureEngine {
    /// Ring buffer for capture data
    buffer: CaptureBuffer,
    /// Current frame being captured
    current_frame: Option<FrameInProgress>,
    /// Capture statistics
    stats: CaptureStats,
    /// Whether capturing is enabled
    enabled: AtomicBool,
}

impl CaptureEngine {
    /// Create a new capture engine
    pub fn new(buffer_size: usize) -> InspectorResult<Self> {
        Ok(Self {
            buffer: CaptureBuffer::new(buffer_size)?,
            current_frame: None,
            stats: CaptureStats::default(),
            enabled: AtomicBool::new(true),
        })
    }

    /// Begin capturing a frame
    pub fn begin_frame(&mut self, frame_id: u64) -> InspectorResult<()> {
        if self.current_frame.is_some() {
            return Err(InspectorError::new(
                InspectorErrorKind::InvalidState,
                "Frame already in progress",
            ));
        }

        self.current_frame = Some(FrameInProgress::new(frame_id));
        Ok(())
    }

    /// End capturing a frame
    pub fn end_frame(&mut self, frame_id: u64) -> InspectorResult<FrameData> {
        let frame = self.current_frame.take().ok_or_else(|| {
            InspectorError::new(InspectorErrorKind::InvalidState, "No frame in progress")
        })?;

        if frame.frame_id != frame_id {
            return Err(InspectorError::new(
                InspectorErrorKind::InvalidState,
                "Frame ID mismatch",
            ));
        }

        self.stats.frames_captured += 1;

        Ok(frame.finalize())
    }

    /// Record a command
    pub fn record_command(&mut self, command: RecordedCommand) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        if let Some(ref mut frame) = self.current_frame {
            frame.record_command(command);
            self.stats.commands_recorded += 1;
        }
    }

    /// Record a render pass start
    pub fn begin_render_pass(&mut self, pass_id: u64, name: Option<String>) {
        if let Some(ref mut frame) = self.current_frame {
            frame.begin_render_pass(pass_id, name);
        }
    }

    /// Record a render pass end
    pub fn end_render_pass(&mut self, pass_id: u64) {
        if let Some(ref mut frame) = self.current_frame {
            frame.end_render_pass(pass_id);
        }
    }

    /// Record a command buffer submission
    pub fn record_submission(&mut self, submission: SubmissionData) {
        if let Some(ref mut frame) = self.current_frame {
            frame.record_submission(submission);
        }
    }

    /// Record a sync point
    pub fn record_sync(&mut self, sync: SyncPoint) {
        if let Some(ref mut frame) = self.current_frame {
            frame.record_sync(sync);
        }
    }

    /// Get capture statistics
    pub fn stats(&self) -> &CaptureStats {
        &self.stats
    }

    /// Enable/disable capturing
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if capturing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Ring buffer for capture data
struct CaptureBuffer {
    data: Vec<u8>,
    write_pos: usize,
    capacity: usize,
}

impl CaptureBuffer {
    fn new(capacity: usize) -> InspectorResult<Self> {
        let mut data = Vec::new();
        data.try_reserve(capacity).map_err(|_| {
            InspectorError::new(
                InspectorErrorKind::OutOfMemory,
                "Failed to allocate capture buffer",
            )
        })?;
        data.resize(capacity, 0);

        Ok(Self {
            data,
            write_pos: 0,
            capacity,
        })
    }

    fn write(&mut self, bytes: &[u8]) -> Option<usize> {
        if self.write_pos + bytes.len() > self.capacity {
            // Wrap around
            self.write_pos = 0;
        }

        let offset = self.write_pos;
        self.data[offset..offset + bytes.len()].copy_from_slice(bytes);
        self.write_pos += bytes.len();

        Some(offset)
    }

    fn clear(&mut self) {
        self.write_pos = 0;
    }
}

/// Frame currently being captured
struct FrameInProgress {
    frame_id: u64,
    command_buffers: BTreeMap<u64, CommandBufferInProgress>,
    render_passes: Vec<RenderPassInProgress>,
    submissions: Vec<SubmissionData>,
    sync_points: Vec<SyncPoint>,
    current_command_buffer: u64,
    start_time: u64,
}

impl FrameInProgress {
    fn new(frame_id: u64) -> Self {
        Self {
            frame_id,
            command_buffers: BTreeMap::new(),
            render_passes: Vec::new(),
            submissions: Vec::new(),
            sync_points: Vec::new(),
            current_command_buffer: 0,
            start_time: get_timestamp(),
        }
    }

    fn record_command(&mut self, command: RecordedCommand) {
        let cb = self
            .command_buffers
            .entry(self.current_command_buffer)
            .or_insert_with(|| CommandBufferInProgress::new(self.current_command_buffer));
        cb.commands.push(command);
    }

    fn begin_render_pass(&mut self, pass_id: u64, name: Option<String>) {
        self.render_passes.push(RenderPassInProgress {
            id: pass_id,
            name,
            start_time: get_timestamp(),
            end_time: None,
        });
    }

    fn end_render_pass(&mut self, pass_id: u64) {
        if let Some(pass) = self.render_passes.iter_mut().find(|p| p.id == pass_id) {
            pass.end_time = Some(get_timestamp());
        }
    }

    fn record_submission(&mut self, submission: SubmissionData) {
        self.submissions.push(submission);
    }

    fn record_sync(&mut self, sync: SyncPoint) {
        self.sync_points.push(sync);
    }

    fn finalize(self) -> FrameData {
        FrameData {
            command_buffers: self
                .command_buffers
                .into_values()
                .map(|cb| cb.finalize())
                .collect(),
            render_passes: self
                .render_passes
                .into_iter()
                .map(|rp| rp.finalize())
                .collect(),
            submissions: self.submissions,
            sync_points: self.sync_points,
        }
    }
}

/// Command buffer being recorded
struct CommandBufferInProgress {
    id: u64,
    name: Option<String>,
    commands: Vec<RecordedCommand>,
    state_changes: Vec<StateChange>,
    resource_bindings: Vec<ResourceBinding>,
}

impl CommandBufferInProgress {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: None,
            commands: Vec::new(),
            state_changes: Vec::new(),
            resource_bindings: Vec::new(),
        }
    }

    fn finalize(self) -> CommandBufferData {
        CommandBufferData {
            id: self.id,
            name: self.name,
            commands: self.commands,
            state_changes: self.state_changes,
            resource_bindings: self.resource_bindings,
        }
    }
}

/// Render pass being recorded
struct RenderPassInProgress {
    id: u64,
    name: Option<String>,
    start_time: u64,
    end_time: Option<u64>,
}

impl RenderPassInProgress {
    fn finalize(self) -> RenderPassData {
        RenderPassData {
            id: self.id,
            name: self.name,
            color_attachments: Vec::new(),
            depth_attachment: None,
            subpasses: Vec::new(),
            duration: self.end_time.unwrap_or(0).saturating_sub(self.start_time),
        }
    }
}

/// Capture statistics
#[derive(Debug, Clone, Default)]
pub struct CaptureStats {
    pub frames_captured: u64,
    pub commands_recorded: u64,
    pub bytes_used: u64,
    pub peak_bytes_used: u64,
    pub buffer_wraps: u64,
}

/// Capture trigger conditions
#[derive(Debug, Clone)]
pub struct CaptureTrigger {
    /// Capture when frame time exceeds threshold (microseconds)
    pub frame_time_threshold: Option<u64>,
    /// Capture when draw calls exceed threshold
    pub draw_call_threshold: Option<u32>,
    /// Capture when memory usage exceeds threshold
    pub memory_threshold: Option<u64>,
    /// Capture on GPU error
    pub on_error: bool,
    /// Capture on specific frame numbers
    pub frame_numbers: Vec<u64>,
    /// Capture every N frames
    pub interval: Option<u32>,
}

impl Default for CaptureTrigger {
    fn default() -> Self {
        Self {
            frame_time_threshold: None,
            draw_call_threshold: None,
            memory_threshold: None,
            on_error: true,
            frame_numbers: Vec::new(),
            interval: None,
        }
    }
}

/// Capture filter to reduce captured data
#[derive(Debug, Clone)]
pub struct CaptureFilter {
    /// Include draw commands
    pub include_draws: bool,
    /// Include compute commands
    pub include_compute: bool,
    /// Include transfer commands
    pub include_transfers: bool,
    /// Include state changes
    pub include_state_changes: bool,
    /// Include barriers
    pub include_barriers: bool,
    /// Filter by render pass name
    pub render_pass_filter: Option<String>,
    /// Filter by command buffer name
    pub command_buffer_filter: Option<String>,
}

impl Default for CaptureFilter {
    fn default() -> Self {
        Self {
            include_draws: true,
            include_compute: true,
            include_transfers: true,
            include_state_changes: true,
            include_barriers: true,
            render_pass_filter: None,
            command_buffer_filter: None,
        }
    }
}

fn get_timestamp() -> u64 {
    // Platform-specific timestamp
    0
}

/// Async capture for zero-stall readback
pub struct AsyncCapture {
    /// Pending readback operations
    pending: Vec<PendingReadback>,
    /// Completed readbacks
    completed: Vec<CompletedReadback>,
    /// Ring buffer for staging
    staging_buffer: StagingRingBuffer,
}

impl AsyncCapture {
    pub fn new(buffer_size: usize) -> InspectorResult<Self> {
        Ok(Self {
            pending: Vec::new(),
            completed: Vec::new(),
            staging_buffer: StagingRingBuffer::new(buffer_size)?,
        })
    }

    /// Request async readback of a resource
    pub fn request_readback(&mut self, resource_id: u64, offset: u64, size: u64) -> u64 {
        let request_id = self.pending.len() as u64;

        self.pending.push(PendingReadback {
            request_id,
            resource_id,
            offset,
            size,
            staging_offset: None,
            fence: None,
        });

        request_id
    }

    /// Poll for completed readbacks
    pub fn poll(&mut self) -> Vec<CompletedReadback> {
        // Check which pending readbacks are complete
        let mut newly_completed = Vec::new();

        self.pending.retain(|pending| {
            if pending.is_complete() {
                if let Some(data) = self.staging_buffer.read(
                    pending.staging_offset.unwrap_or(0) as usize,
                    pending.size as usize,
                ) {
                    newly_completed.push(CompletedReadback {
                        request_id: pending.request_id,
                        resource_id: pending.resource_id,
                        data,
                    });
                }
                false
            } else {
                true
            }
        });

        self.completed.extend(newly_completed.clone());
        newly_completed
    }
}

/// Pending async readback
struct PendingReadback {
    request_id: u64,
    resource_id: u64,
    offset: u64,
    size: u64,
    staging_offset: Option<u64>,
    fence: Option<u64>,
}

impl PendingReadback {
    fn is_complete(&self) -> bool {
        // Check fence status
        self.fence.is_some()
    }
}

/// Completed readback data
#[derive(Debug, Clone)]
pub struct CompletedReadback {
    pub request_id: u64,
    pub resource_id: u64,
    pub data: Vec<u8>,
}

/// Ring buffer for staging readback data
struct StagingRingBuffer {
    data: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    capacity: usize,
}

impl StagingRingBuffer {
    fn new(capacity: usize) -> InspectorResult<Self> {
        let mut data = Vec::new();
        data.try_reserve(capacity).map_err(|_| {
            InspectorError::new(
                InspectorErrorKind::OutOfMemory,
                "Failed to allocate staging buffer",
            )
        })?;
        data.resize(capacity, 0);

        Ok(Self {
            data,
            read_pos: 0,
            write_pos: 0,
            capacity,
        })
    }

    fn read(&self, offset: usize, size: usize) -> Option<Vec<u8>> {
        if offset + size > self.capacity {
            return None;
        }
        Some(self.data[offset..offset + size].to_vec())
    }

    fn allocate(&mut self, size: usize) -> Option<usize> {
        if self.write_pos + size > self.capacity {
            self.write_pos = 0;
        }

        let offset = self.write_pos;
        self.write_pos += size;
        Some(offset)
    }
}
