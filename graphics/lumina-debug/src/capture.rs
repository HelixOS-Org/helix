//! Frame Capture
//!
//! GPU frame capture and debugging support.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bitflags::bitflags;

// ============================================================================
// Capture Flags
// ============================================================================

bitflags! {
    /// Capture flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CaptureFlags: u32 {
        /// Capture render passes.
        const RENDER_PASSES = 1 << 0;
        /// Capture compute.
        const COMPUTE = 1 << 1;
        /// Capture transfers.
        const TRANSFERS = 1 << 2;
        /// Capture resources.
        const RESOURCES = 1 << 3;
        /// Capture descriptors.
        const DESCRIPTORS = 1 << 4;
        /// Capture synchronization.
        const SYNCHRONIZATION = 1 << 5;
        /// Capture timestamps.
        const TIMESTAMPS = 1 << 6;
        /// Capture all.
        const ALL = Self::RENDER_PASSES.bits() | Self::COMPUTE.bits() |
                    Self::TRANSFERS.bits() | Self::RESOURCES.bits() |
                    Self::DESCRIPTORS.bits() | Self::SYNCHRONIZATION.bits() |
                    Self::TIMESTAMPS.bits();
    }
}

impl Default for CaptureFlags {
    fn default() -> Self {
        CaptureFlags::ALL
    }
}

// ============================================================================
// Capture State
// ============================================================================

/// Capture state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Not capturing.
    Idle,
    /// Capture pending (will start next frame).
    Pending,
    /// Currently capturing.
    Capturing,
    /// Capture complete.
    Complete,
    /// Capture failed.
    Failed,
}

impl Default for CaptureState {
    fn default() -> Self {
        CaptureState::Idle
    }
}

// ============================================================================
// Capture Settings
// ============================================================================

/// Capture settings.
#[derive(Debug, Clone)]
pub struct CaptureSettings {
    /// Capture flags.
    pub flags: CaptureFlags,
    /// Number of frames to capture.
    pub frame_count: u32,
    /// Output path.
    pub output_path: Option<String>,
    /// Capture name.
    pub name: Option<String>,
    /// Include pixel history.
    pub pixel_history: bool,
    /// Include resource contents.
    pub resource_contents: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            flags: CaptureFlags::ALL,
            frame_count: 1,
            output_path: None,
            name: None,
            pixel_history: false,
            resource_contents: false,
        }
    }
}

impl CaptureSettings {
    /// Create new settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: CaptureFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set frame count.
    pub fn with_frame_count(mut self, count: u32) -> Self {
        self.frame_count = count;
        self
    }

    /// Set output path.
    pub fn with_output_path(mut self, path: impl Into<String>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    /// Set capture name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Enable pixel history.
    pub fn with_pixel_history(mut self) -> Self {
        self.pixel_history = true;
        self
    }

    /// Enable resource contents.
    pub fn with_resource_contents(mut self) -> Self {
        self.resource_contents = true;
        self
    }
}

// ============================================================================
// Capture Command
// ============================================================================

/// A captured command.
#[derive(Debug, Clone)]
pub enum CaptureCommand {
    /// Begin render pass.
    BeginRenderPass {
        name: Option<String>,
        color_attachments: u32,
        depth_attachment: bool,
    },
    /// End render pass.
    EndRenderPass,
    /// Draw call.
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
    /// Indexed draw call.
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },
    /// Indirect draw.
    DrawIndirect {
        buffer_name: Option<String>,
        draw_count: u32,
    },
    /// Dispatch compute.
    Dispatch { x: u32, y: u32, z: u32 },
    /// Copy buffer.
    CopyBuffer {
        src_name: Option<String>,
        dst_name: Option<String>,
        size: u64,
    },
    /// Copy image.
    CopyImage {
        src_name: Option<String>,
        dst_name: Option<String>,
    },
    /// Blit image.
    BlitImage {
        src_name: Option<String>,
        dst_name: Option<String>,
    },
    /// Pipeline barrier.
    PipelineBarrier {
        src_stage: String,
        dst_stage: String,
    },
    /// Set pipeline.
    SetPipeline { name: Option<String> },
    /// Bind descriptor set.
    BindDescriptorSet { set_index: u32 },
    /// Push constants.
    PushConstants { size: u32 },
}

// ============================================================================
// Capture Frame
// ============================================================================

/// A captured frame.
#[derive(Debug, Clone)]
pub struct CaptureFrame {
    /// Frame index.
    pub frame_index: u64,
    /// Commands.
    pub commands: Vec<CaptureCommand>,
    /// Timestamp.
    pub timestamp: u64,
    /// Duration (ns).
    pub duration_nanos: u64,
    /// Draw call count.
    pub draw_calls: u32,
    /// Dispatch count.
    pub dispatches: u32,
    /// Copy count.
    pub copies: u32,
}

impl CaptureFrame {
    /// Create a new capture frame.
    pub fn new(frame_index: u64) -> Self {
        Self {
            frame_index,
            commands: Vec::new(),
            timestamp: 0,
            duration_nanos: 0,
            draw_calls: 0,
            dispatches: 0,
            copies: 0,
        }
    }

    /// Add a command.
    pub fn add_command(&mut self, command: CaptureCommand) {
        match &command {
            CaptureCommand::Draw { .. }
            | CaptureCommand::DrawIndexed { .. }
            | CaptureCommand::DrawIndirect { .. } => {
                self.draw_calls += 1;
            },
            CaptureCommand::Dispatch { .. } => {
                self.dispatches += 1;
            },
            CaptureCommand::CopyBuffer { .. }
            | CaptureCommand::CopyImage { .. }
            | CaptureCommand::BlitImage { .. } => {
                self.copies += 1;
            },
            _ => {},
        }
        self.commands.push(command);
    }

    /// Get command count.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

// ============================================================================
// Frame Capture
// ============================================================================

/// Frame capture interface.
pub struct FrameCapture {
    /// Is supported.
    pub supported: bool,
    /// Current state.
    state: CaptureState,
    /// Current frame.
    current_frame: Option<CaptureFrame>,
    /// Captured frames.
    frames: Vec<CaptureFrame>,
    /// Settings.
    pub settings: CaptureSettings,
    /// Frames remaining to capture.
    frames_remaining: u32,
}

impl FrameCapture {
    /// Create a new frame capture.
    pub fn new() -> Self {
        Self {
            supported: true,
            state: CaptureState::Idle,
            current_frame: None,
            frames: Vec::new(),
            settings: CaptureSettings::default(),
            frames_remaining: 0,
        }
    }

    /// Check if supported.
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    /// Get current state.
    pub fn state(&self) -> CaptureState {
        self.state
    }

    /// Check if capturing.
    pub fn is_capturing(&self) -> bool {
        self.state == CaptureState::Capturing
    }

    /// Request a capture.
    pub fn request_capture(&mut self, settings: CaptureSettings) {
        if !self.supported {
            return;
        }

        self.settings = settings;
        self.frames_remaining = self.settings.frame_count;
        self.state = CaptureState::Pending;
        self.frames.clear();
    }

    /// Request single frame capture.
    pub fn capture_frame(&mut self) {
        self.request_capture(CaptureSettings::default());
    }

    /// Begin frame capture.
    pub fn begin_frame(&mut self, frame_index: u64) {
        if self.state == CaptureState::Pending {
            self.state = CaptureState::Capturing;
        }

        if self.state == CaptureState::Capturing {
            self.current_frame = Some(CaptureFrame::new(frame_index));
        }
    }

    /// End frame capture.
    pub fn end_frame(&mut self) {
        if self.state != CaptureState::Capturing {
            return;
        }

        if let Some(frame) = self.current_frame.take() {
            self.frames.push(frame);
            self.frames_remaining = self.frames_remaining.saturating_sub(1);

            if self.frames_remaining == 0 {
                self.state = CaptureState::Complete;
            }
        }
    }

    /// Record a command.
    pub fn record(&mut self, command: CaptureCommand) {
        if let Some(frame) = &mut self.current_frame {
            frame.add_command(command);
        }
    }

    /// Get captured frames.
    pub fn frames(&self) -> &[CaptureFrame] {
        &self.frames
    }

    /// Take captured frames.
    pub fn take_frames(&mut self) -> Vec<CaptureFrame> {
        self.state = CaptureState::Idle;
        core::mem::take(&mut self.frames)
    }

    /// Cancel capture.
    pub fn cancel(&mut self) {
        self.state = CaptureState::Idle;
        self.current_frame = None;
        self.frames.clear();
    }
}

impl Default for FrameCapture {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Capture Manager
// ============================================================================

/// Capture manager for external tools.
pub struct CaptureManager {
    /// Frame capture.
    pub frame_capture: FrameCapture,
    /// Is connected to external tool.
    pub tool_connected: AtomicBool,
    /// Tool name.
    pub tool_name: Option<String>,
    /// Capture in progress (for external tools).
    capture_in_progress: AtomicBool,
    /// Frame count.
    frame_count: AtomicU64,
}

impl CaptureManager {
    /// Create a new capture manager.
    pub fn new() -> Self {
        Self {
            frame_capture: FrameCapture::new(),
            tool_connected: AtomicBool::new(false),
            tool_name: None,
            capture_in_progress: AtomicBool::new(false),
            frame_count: AtomicU64::new(0),
        }
    }

    /// Check if external tool is connected.
    pub fn is_tool_connected(&self) -> bool {
        self.tool_connected.load(Ordering::Relaxed)
    }

    /// Set tool connected state.
    pub fn set_tool_connected(&mut self, connected: bool, name: Option<String>) {
        self.tool_connected.store(connected, Ordering::Relaxed);
        self.tool_name = name;
    }

    /// Trigger external capture (for tools like RenderDoc).
    pub fn trigger_external_capture(&self) {
        if self.is_tool_connected() {
            self.capture_in_progress.store(true, Ordering::Relaxed);
        }
    }

    /// Check if capture is in progress.
    pub fn is_capture_in_progress(&self) -> bool {
        self.capture_in_progress.load(Ordering::Relaxed) || self.frame_capture.is_capturing()
    }

    /// Begin frame.
    pub fn begin_frame(&mut self) {
        let frame = self.frame_count.fetch_add(1, Ordering::Relaxed);
        self.frame_capture.begin_frame(frame);
    }

    /// End frame.
    pub fn end_frame(&mut self) {
        self.frame_capture.end_frame();

        if self.capture_in_progress.load(Ordering::Relaxed) {
            self.capture_in_progress.store(false, Ordering::Relaxed);
        }
    }

    /// Get frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }
}

impl Default for CaptureManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RenderDoc Integration
// ============================================================================

/// RenderDoc integration (stub).
pub mod renderdoc {
    /// Check if RenderDoc is available.
    pub fn is_available() -> bool {
        false // Stub - would check for RenderDoc API
    }

    /// Trigger RenderDoc capture.
    pub fn trigger_capture() {
        // Stub - would call RenderDoc API
    }

    /// Start capture.
    pub fn start_capture() {
        // Stub
    }

    /// End capture.
    pub fn end_capture() {
        // Stub
    }

    /// Get capture count.
    pub fn capture_count() -> u32 {
        0
    }

    /// Launch replay UI.
    pub fn launch_replay_ui() {
        // Stub
    }
}

// ============================================================================
// PIX Integration
// ============================================================================

/// PIX integration (stub).
pub mod pix {
    /// Check if PIX is available.
    pub fn is_available() -> bool {
        false // Stub
    }

    /// Begin PIX event.
    pub fn begin_event(_color: u32, _name: &str) {
        // Stub
    }

    /// End PIX event.
    pub fn end_event() {
        // Stub
    }

    /// Set PIX marker.
    pub fn set_marker(_color: u32, _name: &str) {
        // Stub
    }

    /// Begin programmatic capture.
    pub fn begin_capture() {
        // Stub
    }

    /// End programmatic capture.
    pub fn end_capture() {
        // Stub
    }
}
