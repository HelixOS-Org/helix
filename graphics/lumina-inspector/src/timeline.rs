//! # GPU Timeline Visualization
//!
//! Records and visualizes GPU execution timeline with:
//! - Command execution times
//! - Queue dependencies
//! - Barrier synchronization
//! - Memory access patterns

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use crate::{QueueType, SyncType};

/// Timeline recorder for GPU execution
pub struct TimelineRecorder {
    enabled: bool,
    current_frame: Option<TimelineFrame>,
    completed_frames: Vec<TimelineData>,
}

impl TimelineRecorder {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            current_frame: None,
            completed_frames: Vec::new(),
        }
    }

    /// Begin recording a frame
    pub fn begin_frame(&mut self, frame_id: u64) {
        if !self.enabled {
            return;
        }

        self.current_frame = Some(TimelineFrame::new(frame_id));
    }

    /// End recording a frame
    pub fn end_frame(&mut self, _frame_id: u64) -> TimelineData {
        if let Some(frame) = self.current_frame.take() {
            let data = frame.finalize();
            self.completed_frames.push(data.clone());
            data
        } else {
            TimelineData::default()
        }
    }

    /// Record a timeline event
    pub fn record_event(&mut self, event: TimelineEvent) {
        if let Some(ref mut frame) = self.current_frame {
            frame.events.push(event);
        }
    }

    /// Record a GPU work item
    pub fn record_work(&mut self, work: GpuWork) {
        if let Some(ref mut frame) = self.current_frame {
            frame.work_items.push(work);
        }
    }

    /// Record a synchronization point
    pub fn record_sync(&mut self, sync: TimelineSync) {
        if let Some(ref mut frame) = self.current_frame {
            frame.sync_points.push(sync);
        }
    }

    /// Get completed frames
    pub fn frames(&self) -> &[TimelineData] {
        &self.completed_frames
    }
}

/// Timeline frame being recorded
struct TimelineFrame {
    frame_id: u64,
    start_time: u64,
    events: Vec<TimelineEvent>,
    work_items: Vec<GpuWork>,
    sync_points: Vec<TimelineSync>,
}

impl TimelineFrame {
    fn new(frame_id: u64) -> Self {
        Self {
            frame_id,
            start_time: get_timestamp(),
            events: Vec::new(),
            work_items: Vec::new(),
            sync_points: Vec::new(),
        }
    }

    fn finalize(self) -> TimelineData {
        let end_time = get_timestamp();

        // Organize work items by queue
        let mut lanes: BTreeMap<QueueType, Vec<TimelineLaneItem>> = BTreeMap::new();

        for work in &self.work_items {
            let lane = lanes.entry(work.queue).or_insert_with(Vec::new);
            lane.push(TimelineLaneItem {
                id: work.id,
                name: work.name.clone(),
                work_type: work.work_type,
                time_range: work.start_time..work.end_time,
                dependencies: work.dependencies.clone(),
                color: work_type_color(work.work_type),
            });
        }

        TimelineData {
            frame_id: self.frame_id,
            time_range: self.start_time..end_time,
            lanes: lanes
                .into_iter()
                .map(|(queue, items)| TimelineLane {
                    queue_type: queue,
                    items,
                })
                .collect(),
            sync_points: self.sync_points,
            events: self.events,
            statistics: compute_statistics(&self.work_items),
        }
    }
}

/// Timeline data for a frame
#[derive(Debug, Clone, Default)]
pub struct TimelineData {
    pub frame_id: u64,
    pub time_range: Range<u64>,
    pub lanes: Vec<TimelineLane>,
    pub sync_points: Vec<TimelineSync>,
    pub events: Vec<TimelineEvent>,
    pub statistics: TimelineStatistics,
}

/// A lane in the timeline (one per queue)
#[derive(Debug, Clone)]
pub struct TimelineLane {
    pub queue_type: QueueType,
    pub items: Vec<TimelineLaneItem>,
}

/// An item in a timeline lane
#[derive(Debug, Clone)]
pub struct TimelineLaneItem {
    pub id: u64,
    pub name: Option<String>,
    pub work_type: GpuWorkType,
    pub time_range: Range<u64>,
    pub dependencies: Vec<u64>,
    pub color: TimelineColor,
}

/// GPU work types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuWorkType {
    RenderPass,
    ComputePass,
    Transfer,
    RayTracing,
    Barrier,
    Query,
    AccelerationStructure,
    Other,
}

/// Timeline color for visualization
#[derive(Debug, Clone, Copy)]
pub struct TimelineColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl TimelineColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

fn work_type_color(work_type: GpuWorkType) -> TimelineColor {
    match work_type {
        GpuWorkType::RenderPass => TimelineColor::new(66, 133, 244), // Blue
        GpuWorkType::ComputePass => TimelineColor::new(52, 168, 83), // Green
        GpuWorkType::Transfer => TimelineColor::new(251, 188, 4),    // Yellow
        GpuWorkType::RayTracing => TimelineColor::new(234, 67, 53),  // Red
        GpuWorkType::Barrier => TimelineColor::new(128, 128, 128),   // Gray
        GpuWorkType::Query => TimelineColor::new(156, 39, 176),      // Purple
        GpuWorkType::AccelerationStructure => TimelineColor::new(255, 152, 0), // Orange
        GpuWorkType::Other => TimelineColor::new(96, 96, 96),        // Dark gray
    }
}

/// GPU work item
#[derive(Debug, Clone)]
pub struct GpuWork {
    pub id: u64,
    pub name: Option<String>,
    pub queue: QueueType,
    pub work_type: GpuWorkType,
    pub start_time: u64,
    pub end_time: u64,
    pub dependencies: Vec<u64>,
    pub details: GpuWorkDetails,
}

/// Details about GPU work
#[derive(Debug, Clone)]
pub enum GpuWorkDetails {
    RenderPass {
        draw_calls: u32,
        triangles: u64,
        pixels: u64,
        attachments: u32,
    },
    ComputePass {
        dispatches: u32,
        invocations: u64,
    },
    Transfer {
        bytes: u64,
        direction: TransferDirection,
    },
    RayTracing {
        rays: u64,
        bounces: u32,
    },
    Barrier {
        src_stage: u64,
        dst_stage: u64,
        resource_count: u32,
    },
    Other,
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    HostToDevice,
    DeviceToHost,
    DeviceToDevice,
}

/// Timeline synchronization point
#[derive(Debug, Clone)]
pub struct TimelineSync {
    pub sync_type: SyncType,
    pub timestamp: u64,
    pub source_work: Option<u64>,
    pub target_work: Option<u64>,
    pub description: Option<String>,
}

/// Timeline event (markers, annotations)
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub event_type: TimelineEventType,
    pub timestamp: u64,
    pub name: String,
    pub color: Option<TimelineColor>,
    pub data: Option<String>,
}

/// Event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventType {
    Marker,
    RangeStart,
    RangeEnd,
    Error,
    Warning,
    Info,
}

/// Timeline statistics
#[derive(Debug, Clone, Default)]
pub struct TimelineStatistics {
    pub total_gpu_time: u64,
    pub render_time: u64,
    pub compute_time: u64,
    pub transfer_time: u64,
    pub barrier_time: u64,
    pub idle_time: u64,
    pub queue_utilization: BTreeMap<QueueType, f32>,
    pub bottleneck: Option<BottleneckInfo>,
}

/// Bottleneck information
#[derive(Debug, Clone)]
pub struct BottleneckInfo {
    pub bottleneck_type: BottleneckType,
    pub description: String,
    pub work_id: Option<u64>,
    pub severity: f32,
}

/// Types of bottlenecks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckType {
    GpuBound,
    CpuBound,
    MemoryBandwidth,
    SyncStall,
    PipelineBubble,
}

fn compute_statistics(work_items: &[GpuWork]) -> TimelineStatistics {
    let mut stats = TimelineStatistics::default();

    for work in work_items {
        let duration = work.end_time.saturating_sub(work.start_time);
        stats.total_gpu_time += duration;

        match work.work_type {
            GpuWorkType::RenderPass => stats.render_time += duration,
            GpuWorkType::ComputePass => stats.compute_time += duration,
            GpuWorkType::Transfer => stats.transfer_time += duration,
            GpuWorkType::Barrier => stats.barrier_time += duration,
            _ => {},
        }
    }

    stats
}

fn get_timestamp() -> u64 {
    0
}

/// Timeline analysis for performance insights
pub struct TimelineAnalyzer {
    /// Minimum duration to flag as slow (microseconds)
    slow_threshold: u64,
    /// Minimum idle gap to flag as bubble
    bubble_threshold: u64,
}

impl TimelineAnalyzer {
    pub fn new() -> Self {
        Self {
            slow_threshold: 1000,  // 1ms
            bubble_threshold: 100, // 100us
        }
    }

    /// Analyze a timeline for performance issues
    pub fn analyze(&self, timeline: &TimelineData) -> Vec<TimelineIssue> {
        let mut issues = Vec::new();

        // Find slow work items
        for lane in &timeline.lanes {
            for item in &lane.items {
                let duration = item.time_range.end.saturating_sub(item.time_range.start);
                if duration > self.slow_threshold {
                    issues.push(TimelineIssue {
                        issue_type: TimelineIssueType::SlowWork,
                        work_id: Some(item.id),
                        description: alloc::format!(
                            "Work item '{}' took {}us",
                            item.name.as_deref().unwrap_or("unnamed"),
                            duration
                        ),
                        severity: (duration as f32 / self.slow_threshold as f32).min(3.0),
                        suggestion: Some(String::from(
                            "Consider optimizing or splitting this work",
                        )),
                    });
                }
            }
        }

        // Find pipeline bubbles
        for lane in &timeline.lanes {
            let mut prev_end = 0;
            for item in &lane.items {
                let gap = item.time_range.start.saturating_sub(prev_end);
                if gap > self.bubble_threshold {
                    issues.push(TimelineIssue {
                        issue_type: TimelineIssueType::PipelineBubble,
                        work_id: Some(item.id),
                        description: alloc::format!("{}us idle gap before work", gap),
                        severity: (gap as f32 / self.bubble_threshold as f32).min(2.0),
                        suggestion: Some(String::from(
                            "Consider overlapping work or reducing sync points",
                        )),
                    });
                }
                prev_end = item.time_range.end;
            }
        }

        // Find sync bottlenecks
        for sync in &timeline.sync_points {
            if matches!(sync.sync_type, SyncType::Fence | SyncType::Barrier) {
                if sync.source_work.is_some() && sync.target_work.is_some() {
                    issues.push(TimelineIssue {
                        issue_type: TimelineIssueType::SyncBottleneck,
                        work_id: sync.target_work,
                        description: String::from("GPU stall waiting for sync"),
                        severity: 1.5,
                        suggestion: Some(String::from(
                            "Consider async compute or better pipelining",
                        )),
                    });
                }
            }
        }

        issues
    }
}

impl Default for TimelineAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline performance issue
#[derive(Debug, Clone)]
pub struct TimelineIssue {
    pub issue_type: TimelineIssueType,
    pub work_id: Option<u64>,
    pub description: String,
    pub severity: f32,
    pub suggestion: Option<String>,
}

/// Types of timeline issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineIssueType {
    SlowWork,
    PipelineBubble,
    SyncBottleneck,
    MemoryBandwidth,
    QueueImbalance,
}

/// Export timeline data for external visualization
pub struct TimelineExporter;

impl TimelineExporter {
    /// Export to Chrome Trace Event format (JSON)
    pub fn to_chrome_trace(timeline: &TimelineData) -> String {
        use alloc::format;

        let mut events = Vec::new();

        // Add work items as duration events
        for lane in &timeline.lanes {
            let pid = match lane.queue_type {
                QueueType::Graphics => 1,
                QueueType::Compute => 2,
                QueueType::Transfer => 3,
                QueueType::Present => 4,
            };

            for item in &lane.items {
                let duration = item.time_range.end.saturating_sub(item.time_range.start);
                events.push(format!(
                    r#"{{"name":"{}","cat":"{}","ph":"X","ts":{},"dur":{},"pid":{},"tid":1}}"#,
                    item.name.as_deref().unwrap_or("work"),
                    work_type_name(item.work_type),
                    item.time_range.start,
                    duration,
                    pid
                ));
            }
        }

        // Add events as instant events
        for event in &timeline.events {
            events.push(format!(
                r#"{{"name":"{}","cat":"event","ph":"i","ts":{},"s":"g"}}"#,
                event.name, event.timestamp
            ));
        }

        format!("{{{}}}", events.join(","))
    }

    /// Export to custom binary format for efficient storage
    pub fn to_binary(timeline: &TimelineData) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(b"LTLN"); // Magic
        data.extend_from_slice(&1u32.to_le_bytes()); // Version
        data.extend_from_slice(&timeline.frame_id.to_le_bytes());
        data.extend_from_slice(&timeline.time_range.start.to_le_bytes());
        data.extend_from_slice(&timeline.time_range.end.to_le_bytes());

        // Lane count
        data.extend_from_slice(&(timeline.lanes.len() as u32).to_le_bytes());

        for lane in &timeline.lanes {
            data.push(lane.queue_type as u8);
            data.extend_from_slice(&(lane.items.len() as u32).to_le_bytes());

            for item in &lane.items {
                data.extend_from_slice(&item.id.to_le_bytes());
                data.push(item.work_type as u8);
                data.extend_from_slice(&item.time_range.start.to_le_bytes());
                data.extend_from_slice(&item.time_range.end.to_le_bytes());
            }
        }

        data
    }
}

fn work_type_name(work_type: GpuWorkType) -> &'static str {
    match work_type {
        GpuWorkType::RenderPass => "render",
        GpuWorkType::ComputePass => "compute",
        GpuWorkType::Transfer => "transfer",
        GpuWorkType::RayTracing => "raytracing",
        GpuWorkType::Barrier => "barrier",
        GpuWorkType::Query => "query",
        GpuWorkType::AccelerationStructure => "as",
        GpuWorkType::Other => "other",
    }
}
