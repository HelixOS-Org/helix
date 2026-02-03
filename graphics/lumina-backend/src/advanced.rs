//! Advanced GPU Features
//!
//! Cutting-edge GPU features for next-generation rendering.
//!
//! # Features
//!
//! - **Variable Rate Shading (VRS)**: Adaptive shading rate
//! - **Work Graphs**: GPU-driven task scheduling
//! - **Async Compute**: Parallel GPU workloads
//! - **Sampler Feedback**: Texture streaming optimization
//! - **Wave Intrinsics**: Subgroup operations

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::buffer::BufferHandle;
use crate::texture::TextureHandle;

// ============================================================================
// Variable Rate Shading (VRS)
// ============================================================================

/// Shading rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShadingRate {
    /// 1x1 (full rate).
    Rate1x1 = 0,
    /// 1x2.
    Rate1x2 = 1,
    /// 2x1.
    Rate2x1 = 2,
    /// 2x2.
    Rate2x2 = 3,
    /// 2x4.
    Rate2x4 = 4,
    /// 4x2.
    Rate4x2 = 5,
    /// 4x4 (lowest rate).
    Rate4x4 = 6,
}

impl Default for ShadingRate {
    fn default() -> Self {
        ShadingRate::Rate1x1
    }
}

impl ShadingRate {
    /// Get coarse pixel width.
    pub fn width(&self) -> u32 {
        match self {
            ShadingRate::Rate1x1 | ShadingRate::Rate1x2 => 1,
            ShadingRate::Rate2x1 | ShadingRate::Rate2x2 | ShadingRate::Rate2x4 => 2,
            ShadingRate::Rate4x2 | ShadingRate::Rate4x4 => 4,
        }
    }

    /// Get coarse pixel height.
    pub fn height(&self) -> u32 {
        match self {
            ShadingRate::Rate1x1 | ShadingRate::Rate2x1 => 1,
            ShadingRate::Rate1x2 | ShadingRate::Rate2x2 | ShadingRate::Rate4x2 => 2,
            ShadingRate::Rate2x4 | ShadingRate::Rate4x4 => 4,
        }
    }

    /// Get shading reduction factor.
    pub fn reduction_factor(&self) -> f32 {
        1.0 / (self.width() * self.height()) as f32
    }
}

/// Shading rate combiner operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadingRateCombinerOp {
    /// Use source rate.
    Passthrough,
    /// Override with new rate.
    Override,
    /// Use minimum (finest).
    Min,
    /// Use maximum (coarsest).
    Max,
    /// Multiply rates.
    Mul,
}

impl Default for ShadingRateCombinerOp {
    fn default() -> Self {
        ShadingRateCombinerOp::Passthrough
    }
}

/// VRS configuration.
#[derive(Debug, Clone, Copy)]
pub struct VrsConfig {
    /// Pipeline shading rate.
    pub pipeline_rate: ShadingRate,
    /// Per-primitive combiner.
    pub primitive_combiner: ShadingRateCombinerOp,
    /// Image combiner.
    pub image_combiner: ShadingRateCombinerOp,
}

impl Default for VrsConfig {
    fn default() -> Self {
        Self {
            pipeline_rate: ShadingRate::Rate1x1,
            primitive_combiner: ShadingRateCombinerOp::Passthrough,
            image_combiner: ShadingRateCombinerOp::Passthrough,
        }
    }
}

/// VRS image description.
#[derive(Debug, Clone)]
pub struct VrsImageDesc {
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// Tile size.
    pub tile_size: u32,
}

impl Default for VrsImageDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            tile_size: 16,
        }
    }
}

/// VRS features.
#[derive(Debug, Clone, Copy, Default)]
pub struct VrsFeatures {
    /// Pipeline rate supported.
    pub pipeline_rate: bool,
    /// Per-primitive rate supported.
    pub primitive_rate: bool,
    /// Image rate supported.
    pub image_rate: bool,
    /// Non-uniform rate supported.
    pub non_uniform_rate: bool,
    /// Max fragment size width.
    pub max_fragment_size_width: u32,
    /// Max fragment size height.
    pub max_fragment_size_height: u32,
    /// Min fragment shading rate tile size.
    pub min_tile_size: u32,
    /// Max fragment shading rate tile size.
    pub max_tile_size: u32,
}

/// VRS manager.
pub struct VrsManager {
    /// Features.
    features: VrsFeatures,
    /// Current config.
    current_config: VrsConfig,
    /// Statistics.
    pixels_shaded: AtomicU64,
    full_rate_pixels: AtomicU64,
}

impl VrsManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            features: VrsFeatures::default(),
            current_config: VrsConfig::default(),
            pixels_shaded: AtomicU64::new(0),
            full_rate_pixels: AtomicU64::new(0),
        }
    }

    /// Initialize with features.
    pub fn initialize(&mut self, features: VrsFeatures) {
        self.features = features;
    }

    /// Check if supported.
    pub fn is_supported(&self) -> bool {
        self.features.pipeline_rate || self.features.image_rate
    }

    /// Get features.
    pub fn features(&self) -> &VrsFeatures {
        &self.features
    }

    /// Set config.
    pub fn set_config(&mut self, config: VrsConfig) {
        self.current_config = config;
    }

    /// Get config.
    pub fn config(&self) -> &VrsConfig {
        &self.current_config
    }

    /// Calculate optimal shading rate for distance.
    pub fn calculate_rate_for_distance(&self, distance: f32, threshold: f32) -> ShadingRate {
        if distance < threshold {
            ShadingRate::Rate1x1
        } else if distance < threshold * 2.0 {
            ShadingRate::Rate2x2
        } else {
            ShadingRate::Rate4x4
        }
    }

    /// Calculate optimal shading rate for velocity.
    pub fn calculate_rate_for_velocity(&self, velocity: f32, threshold: f32) -> ShadingRate {
        if velocity > threshold * 2.0 {
            ShadingRate::Rate4x4
        } else if velocity > threshold {
            ShadingRate::Rate2x2
        } else {
            ShadingRate::Rate1x1
        }
    }

    /// Record shading.
    pub fn record_shading(&self, pixels: u64, rate: ShadingRate) {
        self.pixels_shaded.fetch_add(pixels, Ordering::Relaxed);
        if rate == ShadingRate::Rate1x1 {
            self.full_rate_pixels.fetch_add(pixels, Ordering::Relaxed);
        }
    }

    /// Get reduction percentage.
    pub fn get_reduction_percentage(&self) -> f32 {
        let total = self.pixels_shaded.load(Ordering::Relaxed);
        let full = self.full_rate_pixels.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            (1.0 - full as f32 / total as f32) * 100.0
        }
    }

    /// Reset statistics.
    pub fn reset_statistics(&mut self) {
        self.pixels_shaded.store(0, Ordering::Relaxed);
        self.full_rate_pixels.store(0, Ordering::Relaxed);
    }
}

impl Default for VrsManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Work Graphs (D3D12 Work Graphs / Vulkan Device Generated Commands)
// ============================================================================

/// Work graph node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkGraphNodeType {
    /// Broadcasting launch node.
    BroadcastingLaunch,
    /// Coalescing launch node.
    CoalescingLaunch,
    /// Thread launch node.
    ThreadLaunch,
}

/// Work graph node.
#[derive(Debug, Clone)]
pub struct WorkGraphNode {
    /// Node name.
    pub name: String,
    /// Node type.
    pub node_type: WorkGraphNodeType,
    /// Shader module.
    pub shader: u64,
    /// Entry point.
    pub entry_point: String,
    /// Input record size.
    pub input_record_size: u32,
    /// Output record size.
    pub output_record_size: u32,
    /// Max dispatch grid.
    pub max_dispatch_grid: [u32; 3],
    /// Node ID.
    pub node_id: u32,
}

impl WorkGraphNode {
    /// Create new node.
    pub fn new(name: impl Into<String>, node_type: WorkGraphNodeType) -> Self {
        Self {
            name: name.into(),
            node_type,
            shader: 0,
            entry_point: String::new(),
            input_record_size: 0,
            output_record_size: 0,
            max_dispatch_grid: [1, 1, 1],
            node_id: 0,
        }
    }
}

/// Work graph description.
#[derive(Debug, Clone)]
pub struct WorkGraphDesc {
    /// Debug name.
    pub name: Option<String>,
    /// Nodes.
    pub nodes: Vec<WorkGraphNode>,
    /// Entry nodes (indices).
    pub entry_nodes: Vec<u32>,
    /// Max recursion depth.
    pub max_recursion: u32,
}

impl Default for WorkGraphDesc {
    fn default() -> Self {
        Self {
            name: None,
            nodes: Vec::new(),
            entry_nodes: Vec::new(),
            max_recursion: 4,
        }
    }
}

/// Work graph handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkGraphHandle(pub u64);

impl WorkGraphHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self(u64::MAX);
}

/// Work graph memory requirements.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkGraphMemoryRequirements {
    /// Backing memory size.
    pub backing_memory_size: u64,
    /// Scratch memory size.
    pub scratch_memory_size: u64,
    /// Record count.
    pub max_record_count: u32,
}

/// Work graph dispatch parameters.
#[derive(Debug, Clone)]
pub struct WorkGraphDispatchDesc {
    /// Work graph handle.
    pub graph: WorkGraphHandle,
    /// Entry node index.
    pub entry_node: u32,
    /// Input records buffer.
    pub input_buffer: BufferHandle,
    /// Input offset.
    pub input_offset: u64,
    /// Record count.
    pub record_count: u32,
    /// Backing memory.
    pub backing_memory: BufferHandle,
    /// Backing offset.
    pub backing_offset: u64,
}

/// Work graph features.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkGraphFeatures {
    /// Work graphs supported.
    pub work_graphs: bool,
    /// Max node count.
    pub max_node_count: u32,
    /// Max recursion depth.
    pub max_recursion_depth: u32,
    /// Max record size.
    pub max_record_size: u32,
}

// ============================================================================
// Async Compute
// ============================================================================

/// Compute priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComputePriority {
    /// Low priority (background).
    Low,
    /// Normal priority.
    Normal,
    /// High priority.
    High,
    /// Realtime priority.
    Realtime,
}

impl Default for ComputePriority {
    fn default() -> Self {
        ComputePriority::Normal
    }
}

/// Async compute task.
#[derive(Debug, Clone)]
pub struct AsyncComputeTask {
    /// Task ID.
    pub id: u64,
    /// Priority.
    pub priority: ComputePriority,
    /// Pipeline handle.
    pub pipeline: u64,
    /// Dispatch size.
    pub dispatch: [u32; 3],
    /// Dependencies (task IDs).
    pub dependencies: Vec<u64>,
}

/// Async compute queue.
pub struct AsyncComputeQueue {
    /// Queue ID.
    id: u32,
    /// Priority.
    priority: ComputePriority,
    /// Pending tasks.
    pending_tasks: Vec<AsyncComputeTask>,
    /// Completed task count.
    completed_count: AtomicU64,
    /// Next task ID.
    next_task_id: AtomicU64,
}

impl AsyncComputeQueue {
    /// Create new queue.
    pub fn new(id: u32, priority: ComputePriority) -> Self {
        Self {
            id,
            priority,
            pending_tasks: Vec::new(),
            completed_count: AtomicU64::new(0),
            next_task_id: AtomicU64::new(1),
        }
    }

    /// Get queue ID.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get priority.
    pub fn priority(&self) -> ComputePriority {
        self.priority
    }

    /// Submit task.
    pub fn submit(&mut self, mut task: AsyncComputeTask) -> u64 {
        task.id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let id = task.id;
        self.pending_tasks.push(task);
        id
    }

    /// Get pending count.
    pub fn pending_count(&self) -> usize {
        self.pending_tasks.len()
    }

    /// Get completed count.
    pub fn completed_count(&self) -> u64 {
        self.completed_count.load(Ordering::Relaxed)
    }

    /// Mark task completed.
    pub fn complete_task(&mut self, task_id: u64) {
        if let Some(idx) = self.pending_tasks.iter().position(|t| t.id == task_id) {
            self.pending_tasks.remove(idx);
            self.completed_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Async compute manager.
pub struct AsyncComputeManager {
    /// Compute queues.
    queues: Vec<AsyncComputeQueue>,
    /// Is supported.
    supported: bool,
    /// Max concurrent tasks.
    max_concurrent: u32,
}

impl AsyncComputeManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            queues: Vec::new(),
            supported: false,
            max_concurrent: 8,
        }
    }

    /// Initialize.
    pub fn initialize(&mut self, queue_count: u32, max_concurrent: u32) {
        self.supported = queue_count > 0;
        self.max_concurrent = max_concurrent;

        // Create queues with different priorities
        for i in 0..queue_count.min(4) {
            let priority = match i {
                0 => ComputePriority::High,
                1 => ComputePriority::Normal,
                2 => ComputePriority::Low,
                _ => ComputePriority::Normal,
            };
            self.queues.push(AsyncComputeQueue::new(i, priority));
        }
    }

    /// Is supported.
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    /// Get queue count.
    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }

    /// Get queue.
    pub fn queue(&mut self, index: usize) -> Option<&mut AsyncComputeQueue> {
        self.queues.get_mut(index)
    }

    /// Get queue by priority.
    pub fn queue_by_priority(&mut self, priority: ComputePriority) -> Option<&mut AsyncComputeQueue> {
        self.queues.iter_mut().find(|q| q.priority == priority)
    }

    /// Total pending tasks.
    pub fn total_pending(&self) -> usize {
        self.queues.iter().map(|q| q.pending_count()).sum()
    }
}

impl Default for AsyncComputeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Sampler Feedback
// ============================================================================

/// Sampler feedback type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SamplerFeedbackType {
    /// Mip region used.
    MipRegionUsed,
    /// Min mip level.
    MinMip,
}

/// Sampler feedback map description.
#[derive(Debug, Clone)]
pub struct SamplerFeedbackDesc {
    /// Feedback type.
    pub feedback_type: SamplerFeedbackType,
    /// Source texture.
    pub source_texture: TextureHandle,
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// Mip levels.
    pub mip_levels: u32,
    /// Min mip.
    pub min_mip: u32,
}

impl Default for SamplerFeedbackDesc {
    fn default() -> Self {
        Self {
            feedback_type: SamplerFeedbackType::MinMip,
            source_texture: TextureHandle::INVALID,
            width: 1,
            height: 1,
            mip_levels: 1,
            min_mip: 0,
        }
    }
}

/// Sampler feedback features.
#[derive(Debug, Clone, Copy, Default)]
pub struct SamplerFeedbackFeatures {
    /// Mip region used supported.
    pub mip_region_used: bool,
    /// Min mip supported.
    pub min_mip: bool,
}

// ============================================================================
// Wave Intrinsics
// ============================================================================

/// Wave/subgroup features.
#[derive(Debug, Clone, Copy, Default)]
pub struct WaveFeatures {
    /// Basic wave operations.
    pub basic: bool,
    /// Wave vote operations.
    pub vote: bool,
    /// Wave arithmetic operations.
    pub arithmetic: bool,
    /// Wave ballot operations.
    pub ballot: bool,
    /// Wave shuffle operations.
    pub shuffle: bool,
    /// Wave shuffle relative operations.
    pub shuffle_relative: bool,
    /// Wave clustered operations.
    pub clustered: bool,
    /// Wave quad operations.
    pub quad: bool,
    /// Min wave size.
    pub min_size: u32,
    /// Max wave size.
    pub max_size: u32,
    /// Supported stages (bitmask).
    pub supported_stages: u32,
}

/// Wave operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaveOp {
    /// Wave get lane count.
    GetLaneCount,
    /// Wave get lane index.
    GetLaneIndex,
    /// Wave is first lane.
    IsFirstLane,
    /// Wave active any.
    ActiveAny,
    /// Wave active all.
    ActiveAll,
    /// Wave active ballot.
    ActiveBallot,
    /// Wave read lane at.
    ReadLaneAt,
    /// Wave read lane first.
    ReadLaneFirst,
    /// Wave active sum.
    ActiveSum,
    /// Wave active product.
    ActiveProduct,
    /// Wave active min.
    ActiveMin,
    /// Wave active max.
    ActiveMax,
    /// Wave active bit and.
    ActiveBitAnd,
    /// Wave active bit or.
    ActiveBitOr,
    /// Wave active bit xor.
    ActiveBitXor,
    /// Wave prefix sum.
    PrefixSum,
    /// Wave prefix product.
    PrefixProduct,
    /// Wave quad read across X.
    QuadReadAcrossX,
    /// Wave quad read across Y.
    QuadReadAcrossY,
    /// Wave quad read across diagonal.
    QuadReadAcrossDiagonal,
    /// Wave quad swap horizontal.
    QuadSwapHorizontal,
    /// Wave quad swap vertical.
    QuadSwapVertical,
    /// Wave quad swap diagonal.
    QuadSwapDiagonal,
}

// ============================================================================
// Advanced Features Manager
// ============================================================================

/// Advanced features summary.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdvancedFeatures {
    /// VRS features.
    pub vrs: VrsFeatures,
    /// Work graph features.
    pub work_graphs: WorkGraphFeatures,
    /// Sampler feedback features.
    pub sampler_feedback: SamplerFeedbackFeatures,
    /// Wave features.
    pub wave: WaveFeatures,
    /// Async compute queues.
    pub async_compute_queues: u32,
}

/// Advanced features manager.
pub struct AdvancedFeaturesManager {
    /// Features.
    features: AdvancedFeatures,
    /// VRS manager.
    vrs: VrsManager,
    /// Async compute manager.
    async_compute: AsyncComputeManager,
}

impl AdvancedFeaturesManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            features: AdvancedFeatures::default(),
            vrs: VrsManager::new(),
            async_compute: AsyncComputeManager::new(),
        }
    }

    /// Initialize with features.
    pub fn initialize(&mut self, features: AdvancedFeatures) {
        self.features = features;
        self.vrs.initialize(features.vrs);
        self.async_compute.initialize(features.async_compute_queues, 16);
    }

    /// Get features.
    pub fn features(&self) -> &AdvancedFeatures {
        &self.features
    }

    /// Get VRS manager.
    pub fn vrs(&self) -> &VrsManager {
        &self.vrs
    }

    /// Get VRS manager mutable.
    pub fn vrs_mut(&mut self) -> &mut VrsManager {
        &mut self.vrs
    }

    /// Get async compute manager.
    pub fn async_compute(&self) -> &AsyncComputeManager {
        &self.async_compute
    }

    /// Get async compute manager mutable.
    pub fn async_compute_mut(&mut self) -> &mut AsyncComputeManager {
        &mut self.async_compute
    }

    /// Check if VRS is supported.
    pub fn vrs_supported(&self) -> bool {
        self.vrs.is_supported()
    }

    /// Check if work graphs are supported.
    pub fn work_graphs_supported(&self) -> bool {
        self.features.work_graphs.work_graphs
    }

    /// Check if async compute is supported.
    pub fn async_compute_supported(&self) -> bool {
        self.async_compute.is_supported()
    }

    /// Check if sampler feedback is supported.
    pub fn sampler_feedback_supported(&self) -> bool {
        self.features.sampler_feedback.min_mip || self.features.sampler_feedback.mip_region_used
    }
}

impl Default for AdvancedFeaturesManager {
    fn default() -> Self {
        Self::new()
    }
}
