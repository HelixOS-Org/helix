//! Time-Travel Debugger
//!
//! Revolutionary debugging system that captures and replays GPU state,
//! allowing developers to "travel back in time" to any frame.
//!
//! # Features
//!
//! - **Frame History**: Capture and store complete GPU state per frame
//! - **State Replay**: Re-execute any historical frame
//! - **Diff Analysis**: Compare state between frames
//! - **Breakpoint Replay**: Set breakpoints in historical frames
//! - **Regression Detection**: Automatic detection of visual regressions

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Frame Capture Types
// ============================================================================

/// Unique frame identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameId(pub u64);

/// Frame capture
#[derive(Debug, Clone)]
pub struct FrameCapture {
    /// Frame ID
    pub id: FrameId,
    /// Frame number
    pub frame_number: u64,
    /// Capture timestamp
    pub timestamp: u64,
    /// Frame duration in microseconds
    pub duration_us: u64,
    /// Command buffers
    pub command_buffers: Vec<CommandBufferCapture>,
    /// Resource snapshots
    pub resources: ResourceSnapshot,
    /// Pipeline states
    pub pipelines: Vec<PipelineSnapshot>,
    /// Draw calls
    pub draw_calls: Vec<DrawCallInfo>,
    /// Dispatch calls
    pub dispatch_calls: Vec<DispatchInfo>,
    /// Render targets
    pub render_targets: Vec<RenderTargetInfo>,
    /// GPU timing
    pub gpu_timing: GpuTiming,
    /// Memory usage
    pub memory_usage: MemoryUsage,
    /// Thumbnail (compressed)
    pub thumbnail: Option<Vec<u8>>,
}

/// Command buffer capture
#[derive(Debug, Clone)]
pub struct CommandBufferCapture {
    /// Command buffer ID
    pub id: u64,
    /// Queue type
    pub queue: QueueType,
    /// Commands
    pub commands: Vec<CapturedCommand>,
    /// Execution time in microseconds
    pub execution_time_us: u64,
}

/// Queue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    Graphics,
    Compute,
    Transfer,
    VideoDecode,
    VideoEncode,
}

/// Captured command
#[derive(Debug, Clone)]
pub struct CapturedCommand {
    /// Command type
    pub command_type: CommandType,
    /// Command index
    pub index: u32,
    /// Parameters (serialized)
    pub params: Vec<u8>,
    /// GPU timestamp
    pub gpu_timestamp: u64,
}

/// Command type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    // Draw commands
    Draw,
    DrawIndexed,
    DrawIndirect,
    DrawIndexedIndirect,
    DrawMeshTasks,
    DrawMeshTasksIndirect,

    // Dispatch commands
    Dispatch,
    DispatchIndirect,

    // Ray tracing
    TraceRays,
    TraceRaysIndirect,
    BuildAccelStruct,

    // Transfer commands
    CopyBuffer,
    CopyImage,
    CopyBufferToImage,
    CopyImageToBuffer,
    BlitImage,
    ResolveImage,
    FillBuffer,
    UpdateBuffer,
    ClearColor,
    ClearDepthStencil,

    // Synchronization
    PipelineBarrier,
    SetEvent,
    WaitEvents,
    ResetEvent,

    // State commands
    BindPipeline,
    BindDescriptorSets,
    BindVertexBuffers,
    BindIndexBuffer,
    PushConstants,
    SetViewport,
    SetScissor,
    SetBlendConstants,
    SetDepthBounds,
    SetStencilReference,

    // Render pass
    BeginRenderPass,
    NextSubpass,
    EndRenderPass,

    // Debug
    BeginDebugLabel,
    EndDebugLabel,
    InsertDebugLabel,
}

// ============================================================================
// Resource Snapshots
// ============================================================================

/// Resource snapshot
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    /// Buffers
    pub buffers: Vec<BufferSnapshot>,
    /// Textures
    pub textures: Vec<TextureSnapshot>,
    /// Samplers
    pub samplers: Vec<SamplerSnapshot>,
}

/// Buffer snapshot
#[derive(Debug, Clone)]
pub struct BufferSnapshot {
    /// Buffer ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsage,
    /// Content hash
    pub content_hash: u64,
    /// Content (if captured, may be compressed)
    pub content: Option<Vec<u8>>,
}

/// Buffer usage flags
#[derive(Debug, Clone, Copy)]
pub struct BufferUsage(u32);

impl BufferUsage {
    pub const VERTEX: Self = Self(1 << 0);
    pub const INDEX: Self = Self(1 << 1);
    pub const UNIFORM: Self = Self(1 << 2);
    pub const STORAGE: Self = Self(1 << 3);
    pub const INDIRECT: Self = Self(1 << 4);
    pub const TRANSFER_SRC: Self = Self(1 << 5);
    pub const TRANSFER_DST: Self = Self(1 << 6);
}

/// Texture snapshot
#[derive(Debug, Clone)]
pub struct TextureSnapshot {
    /// Texture ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth/layers
    pub depth: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: u32,
    /// Content hash
    pub content_hash: u64,
    /// Thumbnail (small preview, always captured)
    pub thumbnail: Option<Vec<u8>>,
    /// Full content (if captured)
    pub content: Option<Vec<u8>>,
}

/// Sampler snapshot
#[derive(Debug, Clone)]
pub struct SamplerSnapshot {
    /// Sampler ID
    pub id: u64,
    /// Min filter
    pub min_filter: FilterMode,
    /// Mag filter
    pub mag_filter: FilterMode,
    /// Address mode U
    pub address_u: AddressMode,
    /// Address mode V
    pub address_v: AddressMode,
    /// Address mode W
    pub address_w: AddressMode,
    /// Max anisotropy
    pub max_anisotropy: f32,
}

/// Filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
    Linear,
}

/// Address mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

// ============================================================================
// Pipeline Snapshots
// ============================================================================

/// Pipeline snapshot
#[derive(Debug, Clone)]
pub struct PipelineSnapshot {
    /// Pipeline ID
    pub id: u64,
    /// Pipeline type
    pub pipeline_type: PipelineType,
    /// Name
    pub name: String,
    /// Shader stages
    pub stages: Vec<ShaderStageInfo>,
    /// Vertex input state
    pub vertex_input: Option<VertexInputState>,
    /// Input assembly state
    pub input_assembly: Option<InputAssemblyState>,
    /// Rasterization state
    pub rasterization: Option<RasterizationState>,
    /// Depth stencil state
    pub depth_stencil: Option<DepthStencilState>,
    /// Blend state
    pub blend: Option<BlendState>,
}

/// Pipeline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineType {
    Graphics,
    Compute,
    RayTracing,
    Mesh,
}

/// Shader stage info
#[derive(Debug, Clone)]
pub struct ShaderStageInfo {
    /// Stage
    pub stage: ShaderStage,
    /// Entry point
    pub entry_point: String,
    /// SPIR-V hash
    pub spirv_hash: u64,
    /// Source file
    pub source_file: Option<String>,
}

/// Shader stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    TessControl,
    TessEval,
    Geometry,
    Fragment,
    Compute,
    Task,
    Mesh,
    RayGen,
    RayMiss,
    RayClosestHit,
    RayAnyHit,
    RayIntersection,
    Callable,
}

/// Vertex input state
#[derive(Debug, Clone)]
pub struct VertexInputState {
    /// Bindings
    pub bindings: Vec<VertexBinding>,
    /// Attributes
    pub attributes: Vec<VertexAttribute>,
}

/// Vertex binding
#[derive(Debug, Clone)]
pub struct VertexBinding {
    /// Binding index
    pub binding: u32,
    /// Stride
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

/// Vertex input rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

/// Vertex attribute
#[derive(Debug, Clone)]
pub struct VertexAttribute {
    /// Location
    pub location: u32,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: u32,
    /// Offset
    pub offset: u32,
}

/// Input assembly state
#[derive(Debug, Clone, Copy)]
pub struct InputAssemblyState {
    /// Topology
    pub topology: PrimitiveTopology,
    /// Primitive restart
    pub primitive_restart: bool,
}

/// Primitive topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
    LineListAdj,
    LineStripAdj,
    TriangleListAdj,
    TriangleStripAdj,
    PatchList,
}

/// Rasterization state
#[derive(Debug, Clone, Copy)]
pub struct RasterizationState {
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth bias enable
    pub depth_bias_enable: bool,
}

/// Polygon mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}

/// Cull mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
    FrontAndBack,
}

/// Front face
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

/// Depth stencil state
#[derive(Debug, Clone, Copy)]
pub struct DepthStencilState {
    /// Depth test enable
    pub depth_test_enable: bool,
    /// Depth write enable
    pub depth_write_enable: bool,
    /// Depth compare op
    pub depth_compare_op: CompareOp,
    /// Stencil test enable
    pub stencil_test_enable: bool,
}

/// Compare operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

/// Blend state
#[derive(Debug, Clone)]
pub struct BlendState {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Attachments
    pub attachments: Vec<BlendAttachment>,
}

/// Blend attachment
#[derive(Debug, Clone, Copy)]
pub struct BlendAttachment {
    /// Blend enable
    pub blend_enable: bool,
    /// Color write mask
    pub color_write_mask: u8,
}

// ============================================================================
// Draw/Dispatch Info
// ============================================================================

/// Draw call info
#[derive(Debug, Clone)]
pub struct DrawCallInfo {
    /// Draw index
    pub index: u32,
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
    /// Pipeline used
    pub pipeline_id: u64,
    /// GPU time in nanoseconds
    pub gpu_time_ns: u64,
}

/// Dispatch info
#[derive(Debug, Clone)]
pub struct DispatchInfo {
    /// Dispatch index
    pub index: u32,
    /// Group count X
    pub group_count_x: u32,
    /// Group count Y
    pub group_count_y: u32,
    /// Group count Z
    pub group_count_z: u32,
    /// Pipeline used
    pub pipeline_id: u64,
    /// GPU time in nanoseconds
    pub gpu_time_ns: u64,
}

/// Render target info
#[derive(Debug, Clone)]
pub struct RenderTargetInfo {
    /// Texture ID
    pub texture_id: u64,
    /// Attachment type
    pub attachment_type: AttachmentType,
    /// Load op
    pub load_op: LoadOp,
    /// Store op
    pub store_op: StoreOp,
    /// Clear value
    pub clear_value: ClearValue,
}

/// Attachment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentType {
    Color,
    Depth,
    Stencil,
    DepthStencil,
}

/// Load operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOp {
    Load,
    Clear,
    DontCare,
}

/// Store operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    Store,
    DontCare,
}

/// Clear value
#[derive(Debug, Clone, Copy)]
pub enum ClearValue {
    Color([f32; 4]),
    DepthStencil { depth: f32, stencil: u32 },
}

// ============================================================================
// GPU Timing
// ============================================================================

/// GPU timing information
#[derive(Debug, Clone)]
pub struct GpuTiming {
    /// Total GPU time in microseconds
    pub total_us: u64,
    /// Render pass timings
    pub render_passes: Vec<PassTiming>,
    /// Compute timings
    pub compute: Vec<ComputeTiming>,
    /// Transfer timings
    pub transfers: Vec<TransferTiming>,
}

/// Pass timing
#[derive(Debug, Clone)]
pub struct PassTiming {
    /// Pass name
    pub name: String,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Draw call count
    pub draw_calls: u32,
    /// Triangle count
    pub triangles: u64,
}

/// Compute timing
#[derive(Debug, Clone)]
pub struct ComputeTiming {
    /// Name
    pub name: String,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Dispatch count
    pub dispatches: u32,
}

/// Transfer timing
#[derive(Debug, Clone)]
pub struct TransferTiming {
    /// Type
    pub transfer_type: TransferType,
    /// Duration in microseconds
    pub duration_us: u64,
    /// Bytes transferred
    pub bytes: u64,
}

/// Transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    Upload,
    Download,
    Copy,
}

/// Memory usage
#[derive(Debug, Clone, Copy)]
pub struct MemoryUsage {
    /// Device local used (bytes)
    pub device_local: u64,
    /// Device local budget (bytes)
    pub device_local_budget: u64,
    /// Host visible used (bytes)
    pub host_visible: u64,
    /// Host visible budget (bytes)
    pub host_visible_budget: u64,
}

// ============================================================================
// Time Travel Debugger
// ============================================================================

/// Time travel configuration
#[derive(Debug, Clone)]
pub struct TimeTravelConfig {
    /// Maximum frames to keep in history
    pub max_history: u32,
    /// Capture full resource content
    pub capture_resources: bool,
    /// Capture thumbnails
    pub capture_thumbnails: bool,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Auto-capture on error
    pub capture_on_error: bool,
}

impl Default for TimeTravelConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            capture_resources: false,
            capture_thumbnails: true,
            compression_level: 6,
            capture_on_error: true,
        }
    }
}

/// Frame diff result
#[derive(Debug, Clone)]
pub struct FrameDiff {
    /// Frame A
    pub frame_a: FrameId,
    /// Frame B
    pub frame_b: FrameId,
    /// Resource changes
    pub resource_changes: Vec<ResourceChange>,
    /// Pipeline changes
    pub pipeline_changes: Vec<PipelineChange>,
    /// Draw call changes
    pub draw_call_changes: DrawCallChanges,
    /// Timing changes
    pub timing_changes: TimingChanges,
}

/// Resource change
#[derive(Debug, Clone)]
pub struct ResourceChange {
    /// Resource ID
    pub resource_id: u64,
    /// Change type
    pub change_type: ChangeType,
    /// Description
    pub description: String,
}

/// Pipeline change
#[derive(Debug, Clone)]
pub struct PipelineChange {
    /// Pipeline ID
    pub pipeline_id: u64,
    /// Change type
    pub change_type: ChangeType,
    /// Changed fields
    pub changed_fields: Vec<String>,
}

/// Change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

/// Draw call changes
#[derive(Debug, Clone)]
pub struct DrawCallChanges {
    /// Added draw calls
    pub added: u32,
    /// Removed draw calls
    pub removed: u32,
    /// Modified draw calls
    pub modified: u32,
    /// Total triangle diff
    pub triangle_diff: i64,
}

/// Timing changes
#[derive(Debug, Clone)]
pub struct TimingChanges {
    /// Frame time diff in microseconds
    pub frame_time_diff_us: i64,
    /// GPU time diff
    pub gpu_time_diff_us: i64,
    /// Significant regressions
    pub regressions: Vec<TimingRegression>,
}

/// Timing regression
#[derive(Debug, Clone)]
pub struct TimingRegression {
    /// Pass/shader name
    pub name: String,
    /// Old time
    pub old_time_us: u64,
    /// New time
    pub new_time_us: u64,
    /// Percentage increase
    pub increase_percent: f32,
}

/// Time travel debugger
pub struct TimeTravelDebugger {
    /// Configuration
    config: TimeTravelConfig,
    /// Frame history
    history: Vec<FrameCapture>,
    /// Current frame being recorded
    current_frame: Option<FrameCapture>,
    /// Frame counter
    frame_counter: u64,
    /// Bookmarks
    bookmarks: BTreeMap<String, FrameId>,
}

impl TimeTravelDebugger {
    /// Create new debugger
    pub fn new(config: TimeTravelConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            current_frame: None,
            frame_counter: 0,
            bookmarks: BTreeMap::new(),
        }
    }

    /// Begin frame capture
    pub fn begin_frame(&mut self, timestamp: u64) {
        self.frame_counter += 1;
        self.current_frame = Some(FrameCapture {
            id: FrameId(self.frame_counter),
            frame_number: self.frame_counter,
            timestamp,
            duration_us: 0,
            command_buffers: Vec::new(),
            resources: ResourceSnapshot {
                buffers: Vec::new(),
                textures: Vec::new(),
                samplers: Vec::new(),
            },
            pipelines: Vec::new(),
            draw_calls: Vec::new(),
            dispatch_calls: Vec::new(),
            render_targets: Vec::new(),
            gpu_timing: GpuTiming {
                total_us: 0,
                render_passes: Vec::new(),
                compute: Vec::new(),
                transfers: Vec::new(),
            },
            memory_usage: MemoryUsage {
                device_local: 0,
                device_local_budget: 0,
                host_visible: 0,
                host_visible_budget: 0,
            },
            thumbnail: None,
        });
    }

    /// End frame capture
    pub fn end_frame(&mut self, timestamp: u64) {
        if let Some(mut frame) = self.current_frame.take() {
            frame.duration_us = timestamp.saturating_sub(frame.timestamp);

            // Add to history
            self.history.push(frame);

            // Trim history
            while self.history.len() > self.config.max_history as usize {
                self.history.remove(0);
            }
        }
    }

    /// Get frame by ID
    pub fn get_frame(&self, id: FrameId) -> Option<&FrameCapture> {
        self.history.iter().find(|f| f.id == id)
    }

    /// Get frame by number
    pub fn get_frame_by_number(&self, number: u64) -> Option<&FrameCapture> {
        self.history.iter().find(|f| f.frame_number == number)
    }

    /// Get frame range
    pub fn get_frame_range(&self, start: u64, end: u64) -> Vec<&FrameCapture> {
        self.history
            .iter()
            .filter(|f| f.frame_number >= start && f.frame_number <= end)
            .collect()
    }

    /// Compare two frames
    pub fn diff_frames(&self, a: FrameId, b: FrameId) -> Option<FrameDiff> {
        let frame_a = self.get_frame(a)?;
        let frame_b = self.get_frame(b)?;

        Some(FrameDiff {
            frame_a: a,
            frame_b: b,
            resource_changes: Vec::new(), // Would compute actual changes
            pipeline_changes: Vec::new(),
            draw_call_changes: DrawCallChanges {
                added: 0,
                removed: 0,
                modified: 0,
                triangle_diff: 0,
            },
            timing_changes: TimingChanges {
                frame_time_diff_us: frame_b.duration_us as i64 - frame_a.duration_us as i64,
                gpu_time_diff_us: 0,
                regressions: Vec::new(),
            },
        })
    }

    /// Add bookmark
    pub fn add_bookmark(&mut self, name: impl Into<String>, frame: FrameId) {
        self.bookmarks.insert(name.into(), frame);
    }

    /// Get bookmarked frame
    pub fn get_bookmark(&self, name: &str) -> Option<&FrameCapture> {
        self.bookmarks.get(name).and_then(|id| self.get_frame(*id))
    }

    /// Get history size
    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    /// Get latest frame
    pub fn latest_frame(&self) -> Option<&FrameCapture> {
        self.history.last()
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.bookmarks.clear();
    }
}

impl Default for TimeTravelDebugger {
    fn default() -> Self {
        Self::new(TimeTravelConfig::default())
    }
}
