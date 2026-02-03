//! Performance Counters
//!
//! Advanced GPU performance monitoring, heatmaps, and bottleneck detection.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                 Performance Analysis Pipeline                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │                  Hardware Counters                             │ │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ │ │
//! │  │  │ Shader  │ │ Memory  │ │ Texture │ │ ROP     │ │ L2      │ │ │
//! │  │  │ Core    │ │ Ctrl    │ │ Unit    │ │         │ │ Cache   │ │ │
//! │  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ │ │
//! │  │       │           │           │           │           │      │ │
//! │  │       ▼           ▼           ▼           ▼           ▼      │ │
//! │  │  ┌─────────────────────────────────────────────────────────┐ │ │
//! │  │  │              Performance Counter Aggregator              │ │ │
//! │  │  └─────────────────────────────────────────────────────────┘ │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │                   Bottleneck Detector                         │ │
//! │  │  • ALU bound vs Memory bound                                  │ │
//! │  │  • Occupancy analysis                                         │ │
//! │  │  • Stall analysis                                             │ │
//! │  │  • Recommendations                                            │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │                      Heatmap Generator                        │ │
//! │  │  • Draw call cost visualization                               │ │
//! │  │  • Memory access patterns                                     │ │
//! │  │  • Occupancy visualization                                    │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Counter Types
// ============================================================================

/// Performance counter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterType {
    // Shader Execution
    /// Vertex shader invocations.
    VertexShaderInvocations,
    /// Pixel/fragment shader invocations.
    FragmentShaderInvocations,
    /// Compute shader invocations.
    ComputeShaderInvocations,
    /// Geometry shader invocations.
    GeometryShaderInvocations,
    /// Tessellation control shader invocations.
    TessControlShaderInvocations,
    /// Tessellation evaluation shader invocations.
    TessEvalShaderInvocations,
    /// Task shader invocations.
    TaskShaderInvocations,
    /// Mesh shader invocations.
    MeshShaderInvocations,

    // Primitives
    /// Input primitives.
    InputPrimitives,
    /// Primitives generated.
    PrimitivesGenerated,
    /// Primitives clipped.
    PrimitivesClipped,
    /// Primitives culled.
    PrimitivesCulled,

    // Rasterizer
    /// Rasterizer samples.
    RasterizerSamples,
    /// Early-Z tests.
    EarlyZTests,
    /// Early-Z tests passed.
    EarlyZTestsPassed,
    /// Late-Z tests.
    LateZTests,
    /// Late-Z tests passed.
    LateZTestsPassed,

    // Memory
    /// L1 cache hits.
    L1CacheHits,
    /// L1 cache misses.
    L1CacheMisses,
    /// L2 cache hits.
    L2CacheHits,
    /// L2 cache misses.
    L2CacheMisses,
    /// Memory read bytes.
    MemoryReadBytes,
    /// Memory write bytes.
    MemoryWriteBytes,
    /// Texture cache hits.
    TextureCacheHits,
    /// Texture cache misses.
    TextureCacheMisses,

    // Texture
    /// Texture fetches.
    TextureFetches,
    /// Texture samples.
    TextureSamples,
    /// Anisotropic samples.
    AnisotropicSamples,

    // ALU
    /// FP32 operations.
    Fp32Operations,
    /// FP16 operations.
    Fp16Operations,
    /// INT32 operations.
    Int32Operations,
    /// Tensor/matrix operations.
    TensorOperations,

    // Occupancy
    /// Active warps.
    ActiveWarps,
    /// Active threads.
    ActiveThreads,
    /// Theoretical occupancy.
    TheoreticalOccupancy,
    /// Achieved occupancy.
    AchievedOccupancy,

    // Stalls
    /// Instruction fetch stalls.
    InstructionFetchStalls,
    /// Memory stalls.
    MemoryStalls,
    /// Synchronization stalls.
    SyncStalls,
    /// Texture stalls.
    TextureStalls,

    // Time
    /// GPU active time (ns).
    GpuActiveTimeNs,
    /// GPU idle time (ns).
    GpuIdleTimeNs,
    /// Command processing time (ns).
    CommandProcessTimeNs,
}

/// Counter category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterCategory {
    /// Shader execution.
    Shader,
    /// Primitive processing.
    Primitive,
    /// Rasterization.
    Rasterizer,
    /// Memory subsystem.
    Memory,
    /// Texture unit.
    Texture,
    /// ALU operations.
    Alu,
    /// Occupancy.
    Occupancy,
    /// Stalls.
    Stall,
    /// Timing.
    Timing,
}

impl CounterType {
    /// Get category.
    pub fn category(&self) -> CounterCategory {
        match self {
            CounterType::VertexShaderInvocations
            | CounterType::FragmentShaderInvocations
            | CounterType::ComputeShaderInvocations
            | CounterType::GeometryShaderInvocations
            | CounterType::TessControlShaderInvocations
            | CounterType::TessEvalShaderInvocations
            | CounterType::TaskShaderInvocations
            | CounterType::MeshShaderInvocations => CounterCategory::Shader,

            CounterType::InputPrimitives
            | CounterType::PrimitivesGenerated
            | CounterType::PrimitivesClipped
            | CounterType::PrimitivesCulled => CounterCategory::Primitive,

            CounterType::RasterizerSamples
            | CounterType::EarlyZTests
            | CounterType::EarlyZTestsPassed
            | CounterType::LateZTests
            | CounterType::LateZTestsPassed => CounterCategory::Rasterizer,

            CounterType::L1CacheHits
            | CounterType::L1CacheMisses
            | CounterType::L2CacheHits
            | CounterType::L2CacheMisses
            | CounterType::MemoryReadBytes
            | CounterType::MemoryWriteBytes
            | CounterType::TextureCacheHits
            | CounterType::TextureCacheMisses => CounterCategory::Memory,

            CounterType::TextureFetches
            | CounterType::TextureSamples
            | CounterType::AnisotropicSamples => CounterCategory::Texture,

            CounterType::Fp32Operations
            | CounterType::Fp16Operations
            | CounterType::Int32Operations
            | CounterType::TensorOperations => CounterCategory::Alu,

            CounterType::ActiveWarps
            | CounterType::ActiveThreads
            | CounterType::TheoreticalOccupancy
            | CounterType::AchievedOccupancy => CounterCategory::Occupancy,

            CounterType::InstructionFetchStalls
            | CounterType::MemoryStalls
            | CounterType::SyncStalls
            | CounterType::TextureStalls => CounterCategory::Stall,

            CounterType::GpuActiveTimeNs
            | CounterType::GpuIdleTimeNs
            | CounterType::CommandProcessTimeNs => CounterCategory::Timing,
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            CounterType::VertexShaderInvocations => "VS Invocations",
            CounterType::FragmentShaderInvocations => "FS Invocations",
            CounterType::ComputeShaderInvocations => "CS Invocations",
            CounterType::GeometryShaderInvocations => "GS Invocations",
            CounterType::TessControlShaderInvocations => "TCS Invocations",
            CounterType::TessEvalShaderInvocations => "TES Invocations",
            CounterType::TaskShaderInvocations => "Task Invocations",
            CounterType::MeshShaderInvocations => "Mesh Invocations",
            CounterType::InputPrimitives => "Input Primitives",
            CounterType::PrimitivesGenerated => "Generated Primitives",
            CounterType::PrimitivesClipped => "Clipped Primitives",
            CounterType::PrimitivesCulled => "Culled Primitives",
            CounterType::RasterizerSamples => "Rasterizer Samples",
            CounterType::EarlyZTests => "Early-Z Tests",
            CounterType::EarlyZTestsPassed => "Early-Z Passed",
            CounterType::LateZTests => "Late-Z Tests",
            CounterType::LateZTestsPassed => "Late-Z Passed",
            CounterType::L1CacheHits => "L1 Cache Hits",
            CounterType::L1CacheMisses => "L1 Cache Misses",
            CounterType::L2CacheHits => "L2 Cache Hits",
            CounterType::L2CacheMisses => "L2 Cache Misses",
            CounterType::MemoryReadBytes => "Memory Read (B)",
            CounterType::MemoryWriteBytes => "Memory Write (B)",
            CounterType::TextureCacheHits => "Texture Cache Hits",
            CounterType::TextureCacheMisses => "Texture Cache Misses",
            CounterType::TextureFetches => "Texture Fetches",
            CounterType::TextureSamples => "Texture Samples",
            CounterType::AnisotropicSamples => "Anisotropic Samples",
            CounterType::Fp32Operations => "FP32 Ops",
            CounterType::Fp16Operations => "FP16 Ops",
            CounterType::Int32Operations => "INT32 Ops",
            CounterType::TensorOperations => "Tensor Ops",
            CounterType::ActiveWarps => "Active Warps",
            CounterType::ActiveThreads => "Active Threads",
            CounterType::TheoreticalOccupancy => "Theoretical Occupancy",
            CounterType::AchievedOccupancy => "Achieved Occupancy",
            CounterType::InstructionFetchStalls => "IF Stalls",
            CounterType::MemoryStalls => "Memory Stalls",
            CounterType::SyncStalls => "Sync Stalls",
            CounterType::TextureStalls => "Texture Stalls",
            CounterType::GpuActiveTimeNs => "GPU Active (ns)",
            CounterType::GpuIdleTimeNs => "GPU Idle (ns)",
            CounterType::CommandProcessTimeNs => "Cmd Process (ns)",
        }
    }
}

// ============================================================================
// Counter Values
// ============================================================================

/// Performance counter value.
#[derive(Debug, Clone, Copy, Default)]
pub struct CounterValue {
    /// Current value.
    pub current: u64,
    /// Minimum value.
    pub min: u64,
    /// Maximum value.
    pub max: u64,
    /// Average value.
    pub avg: f64,
    /// Sample count.
    pub samples: u32,
}

impl CounterValue {
    /// Create new counter value.
    pub fn new() -> Self {
        Self {
            current: 0,
            min: u64::MAX,
            max: 0,
            avg: 0.0,
            samples: 0,
        }
    }

    /// Record a sample.
    pub fn record(&mut self, value: u64) {
        self.current = value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.samples += 1;

        // Running average
        let delta = value as f64 - self.avg;
        self.avg += delta / self.samples as f64;
    }

    /// Reset.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Counter set for a single measurement.
#[derive(Debug, Clone, Default)]
pub struct CounterSet {
    /// Counters by type.
    counters: Vec<(CounterType, CounterValue)>,
}

impl CounterSet {
    /// Create new counter set.
    pub fn new() -> Self {
        Self {
            counters: Vec::new(),
        }
    }

    /// Get counter value.
    pub fn get(&self, counter_type: CounterType) -> Option<&CounterValue> {
        self.counters
            .iter()
            .find(|(t, _)| *t == counter_type)
            .map(|(_, v)| v)
    }

    /// Get counter value mutable.
    pub fn get_mut(&mut self, counter_type: CounterType) -> Option<&mut CounterValue> {
        self.counters
            .iter_mut()
            .find(|(t, _)| *t == counter_type)
            .map(|(_, v)| v)
    }

    /// Set counter value.
    pub fn set(&mut self, counter_type: CounterType, value: u64) {
        if let Some((_, v)) = self.counters.iter_mut().find(|(t, _)| *t == counter_type) {
            v.record(value);
        } else {
            let mut cv = CounterValue::new();
            cv.record(value);
            self.counters.push((counter_type, cv));
        }
    }

    /// Get all counters.
    pub fn all(&self) -> &[(CounterType, CounterValue)] {
        &self.counters
    }

    /// Get counters by category.
    pub fn by_category(&self, category: CounterCategory) -> Vec<(CounterType, &CounterValue)> {
        self.counters
            .iter()
            .filter(|(t, _)| t.category() == category)
            .map(|(t, v)| (*t, v))
            .collect()
    }
}

// ============================================================================
// Bottleneck Detection
// ============================================================================

/// Bottleneck type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BottleneckType {
    /// No significant bottleneck.
    None,
    /// ALU/compute bound.
    AluBound,
    /// Memory bandwidth bound.
    MemoryBound,
    /// Memory latency bound.
    LatencyBound,
    /// Texture sampling bound.
    TextureBound,
    /// Rasterizer bound.
    RasterizerBound,
    /// Vertex processing bound.
    VertexBound,
    /// Fragment/pixel bound.
    FragmentBound,
    /// Geometry processing bound.
    GeometryBound,
    /// Tessellation bound.
    TessellationBound,
    /// ROP/blend bound.
    RopBound,
    /// CPU bound (GPU waiting).
    CpuBound,
    /// Driver overhead.
    DriverOverhead,
}

impl BottleneckType {
    /// Get description.
    pub fn description(&self) -> &'static str {
        match self {
            BottleneckType::None => "No significant bottleneck detected",
            BottleneckType::AluBound => {
                "GPU is ALU/compute bound - consider optimizing shader arithmetic"
            },
            BottleneckType::MemoryBound => "GPU is memory bandwidth bound - reduce memory traffic",
            BottleneckType::LatencyBound => {
                "GPU is memory latency bound - improve cache utilization"
            },
            BottleneckType::TextureBound => {
                "GPU is texture sampling bound - reduce texture samples"
            },
            BottleneckType::RasterizerBound => {
                "GPU is rasterizer bound - reduce geometry complexity"
            },
            BottleneckType::VertexBound => {
                "GPU is vertex processing bound - optimize vertex shaders"
            },
            BottleneckType::FragmentBound => "GPU is fragment processing bound - reduce overdraw",
            BottleneckType::GeometryBound => "GPU is geometry bound - consider mesh shaders",
            BottleneckType::TessellationBound => {
                "GPU is tessellation bound - reduce tessellation factors"
            },
            BottleneckType::RopBound => "GPU is ROP/blend bound - reduce alpha blending",
            BottleneckType::CpuBound => "CPU is the bottleneck - reduce draw calls",
            BottleneckType::DriverOverhead => "Driver overhead is high - batch operations",
        }
    }

    /// Get severity (0-1).
    pub fn severity(&self) -> f32 {
        match self {
            BottleneckType::None => 0.0,
            BottleneckType::DriverOverhead => 0.3,
            BottleneckType::CpuBound => 0.5,
            _ => 0.7,
        }
    }
}

/// Bottleneck analysis result.
#[derive(Debug, Clone)]
pub struct BottleneckAnalysis {
    /// Primary bottleneck.
    pub primary: BottleneckType,
    /// Secondary bottleneck.
    pub secondary: Option<BottleneckType>,
    /// Confidence (0-1).
    pub confidence: f32,
    /// Recommendations.
    pub recommendations: Vec<String>,
    /// Metrics.
    pub metrics: BottleneckMetrics,
}

/// Bottleneck metrics.
#[derive(Debug, Clone, Copy, Default)]
pub struct BottleneckMetrics {
    /// ALU utilization (0-100).
    pub alu_utilization: f32,
    /// Memory utilization (0-100).
    pub memory_utilization: f32,
    /// Texture unit utilization (0-100).
    pub texture_utilization: f32,
    /// Rasterizer utilization (0-100).
    pub rasterizer_utilization: f32,
    /// ROP utilization (0-100).
    pub rop_utilization: f32,
    /// Achieved occupancy (0-100).
    pub achieved_occupancy: f32,
    /// L1 cache hit rate (0-100).
    pub l1_hit_rate: f32,
    /// L2 cache hit rate (0-100).
    pub l2_hit_rate: f32,
    /// Texture cache hit rate (0-100).
    pub texture_hit_rate: f32,
    /// Stall percentage (0-100).
    pub stall_percentage: f32,
}

/// Bottleneck detector.
pub struct BottleneckDetector {
    /// History of analyses.
    history: Vec<BottleneckAnalysis>,
    /// Max history.
    max_history: usize,
    /// Thresholds.
    thresholds: BottleneckThresholds,
}

/// Bottleneck detection thresholds.
#[derive(Debug, Clone, Copy)]
pub struct BottleneckThresholds {
    /// High utilization threshold.
    pub high_utilization: f32,
    /// Low utilization threshold.
    pub low_utilization: f32,
    /// High stall threshold.
    pub high_stall: f32,
    /// Low cache hit threshold.
    pub low_cache_hit: f32,
}

impl Default for BottleneckThresholds {
    fn default() -> Self {
        Self {
            high_utilization: 80.0,
            low_utilization: 30.0,
            high_stall: 20.0,
            low_cache_hit: 70.0,
        }
    }
}

impl BottleneckDetector {
    /// Create new detector.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 60, // 1 second at 60fps
            thresholds: BottleneckThresholds::default(),
        }
    }

    /// Analyze counters.
    pub fn analyze(&mut self, counters: &CounterSet) -> BottleneckAnalysis {
        let metrics = self.calculate_metrics(counters);
        let (primary, secondary) = self.detect_bottleneck(&metrics);
        let confidence = self.calculate_confidence(&metrics, primary);
        let recommendations = self.generate_recommendations(primary, &metrics);

        let analysis = BottleneckAnalysis {
            primary,
            secondary,
            confidence,
            recommendations,
            metrics,
        };

        // Update history
        self.history.push(analysis.clone());
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }

        analysis
    }

    fn calculate_metrics(&self, counters: &CounterSet) -> BottleneckMetrics {
        let mut metrics = BottleneckMetrics::default();

        // Calculate cache hit rates
        if let (Some(hits), Some(misses)) = (
            counters.get(CounterType::L1CacheHits),
            counters.get(CounterType::L1CacheMisses),
        ) {
            let total = hits.current + misses.current;
            if total > 0 {
                metrics.l1_hit_rate = (hits.current as f32 / total as f32) * 100.0;
            }
        }

        if let (Some(hits), Some(misses)) = (
            counters.get(CounterType::L2CacheHits),
            counters.get(CounterType::L2CacheMisses),
        ) {
            let total = hits.current + misses.current;
            if total > 0 {
                metrics.l2_hit_rate = (hits.current as f32 / total as f32) * 100.0;
            }
        }

        if let (Some(hits), Some(misses)) = (
            counters.get(CounterType::TextureCacheHits),
            counters.get(CounterType::TextureCacheMisses),
        ) {
            let total = hits.current + misses.current;
            if total > 0 {
                metrics.texture_hit_rate = (hits.current as f32 / total as f32) * 100.0;
            }
        }

        // Occupancy
        if let Some(occ) = counters.get(CounterType::AchievedOccupancy) {
            metrics.achieved_occupancy = occ.current as f32;
        }

        // Stall percentage
        let stall_counters = [
            CounterType::InstructionFetchStalls,
            CounterType::MemoryStalls,
            CounterType::SyncStalls,
            CounterType::TextureStalls,
        ];

        let mut total_stalls = 0u64;
        for counter_type in &stall_counters {
            if let Some(v) = counters.get(*counter_type) {
                total_stalls += v.current;
            }
        }

        if let Some(active) = counters.get(CounterType::GpuActiveTimeNs) {
            if active.current > 0 {
                metrics.stall_percentage = (total_stalls as f32 / active.current as f32) * 100.0;
            }
        }

        metrics
    }

    fn detect_bottleneck(
        &self,
        metrics: &BottleneckMetrics,
    ) -> (BottleneckType, Option<BottleneckType>) {
        let mut bottlenecks: Vec<(BottleneckType, f32)> = Vec::new();

        // Check memory bound
        if metrics.l1_hit_rate < self.thresholds.low_cache_hit
            || metrics.l2_hit_rate < self.thresholds.low_cache_hit
        {
            bottlenecks.push((BottleneckType::MemoryBound, 100.0 - metrics.l2_hit_rate));
        }

        // Check stall percentage
        if metrics.stall_percentage > self.thresholds.high_stall {
            bottlenecks.push((BottleneckType::LatencyBound, metrics.stall_percentage));
        }

        // Check texture bound
        if metrics.texture_hit_rate < self.thresholds.low_cache_hit {
            bottlenecks.push((
                BottleneckType::TextureBound,
                100.0 - metrics.texture_hit_rate,
            ));
        }

        // Check occupancy
        if metrics.achieved_occupancy < self.thresholds.low_utilization {
            bottlenecks.push((BottleneckType::AluBound, 100.0 - metrics.achieved_occupancy));
        }

        // Sort by severity
        bottlenecks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let primary = bottlenecks
            .get(0)
            .map(|(t, _)| *t)
            .unwrap_or(BottleneckType::None);
        let secondary = bottlenecks.get(1).map(|(t, _)| *t);

        (primary, secondary)
    }

    fn calculate_confidence(&self, metrics: &BottleneckMetrics, bottleneck: BottleneckType) -> f32 {
        match bottleneck {
            BottleneckType::None => 1.0,
            BottleneckType::MemoryBound => (100.0 - metrics.l2_hit_rate) / 100.0,
            BottleneckType::LatencyBound => metrics.stall_percentage / 100.0,
            BottleneckType::TextureBound => (100.0 - metrics.texture_hit_rate) / 100.0,
            BottleneckType::AluBound => (100.0 - metrics.achieved_occupancy) / 100.0,
            _ => 0.5,
        }
    }

    fn generate_recommendations(
        &self,
        bottleneck: BottleneckType,
        metrics: &BottleneckMetrics,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        match bottleneck {
            BottleneckType::MemoryBound => {
                recs.push(String::from("Consider using texture compression"));
                recs.push(String::from("Reduce buffer sizes where possible"));
                recs.push(String::from(
                    "Use structured buffers for better cache locality",
                ));
                if metrics.l1_hit_rate < 50.0 {
                    recs.push(String::from("Optimize memory access patterns"));
                }
            },
            BottleneckType::LatencyBound => {
                recs.push(String::from("Increase occupancy to hide latency"));
                recs.push(String::from("Use async compute to overlap work"));
                recs.push(String::from("Prefetch data where possible"));
            },
            BottleneckType::TextureBound => {
                recs.push(String::from("Use mipmaps appropriately"));
                recs.push(String::from("Consider texture atlases"));
                recs.push(String::from("Reduce anisotropic filtering level"));
            },
            BottleneckType::AluBound => {
                recs.push(String::from("Simplify shader math"));
                recs.push(String::from("Use lower precision where acceptable"));
                recs.push(String::from("Consider compute pre-pass"));
            },
            BottleneckType::FragmentBound => {
                recs.push(String::from("Reduce overdraw with depth prepass"));
                recs.push(String::from("Use Variable Rate Shading"));
                recs.push(String::from("Optimize fragment shader"));
            },
            _ => {},
        }

        recs
    }

    /// Get history.
    pub fn history(&self) -> &[BottleneckAnalysis] {
        &self.history
    }

    /// Get average bottleneck over history.
    pub fn average_bottleneck(&self) -> BottleneckType {
        if self.history.is_empty() {
            return BottleneckType::None;
        }

        // Count occurrences
        let mut counts: Vec<(BottleneckType, usize)> = Vec::new();

        for analysis in &self.history {
            if let Some((_, count)) = counts.iter_mut().find(|(t, _)| *t == analysis.primary) {
                *count += 1;
            } else {
                counts.push((analysis.primary, 1));
            }
        }

        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(t, _)| t)
            .unwrap_or(BottleneckType::None)
    }
}

impl Default for BottleneckDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Heatmap
// ============================================================================

/// Heatmap cell.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeatmapCell {
    /// Value (normalized 0-1).
    pub value: f32,
    /// Sample count.
    pub samples: u32,
}

/// Heatmap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeatmapType {
    /// Draw call cost.
    DrawCallCost,
    /// Memory access.
    MemoryAccess,
    /// Shader complexity.
    ShaderComplexity,
    /// Overdraw.
    Overdraw,
    /// Cache misses.
    CacheMisses,
}

/// Heatmap for visualization.
pub struct Heatmap {
    /// Type.
    heatmap_type: HeatmapType,
    /// Width.
    width: u32,
    /// Height.
    height: u32,
    /// Cells.
    cells: Vec<HeatmapCell>,
    /// Min value.
    min_value: f32,
    /// Max value.
    max_value: f32,
}

impl Heatmap {
    /// Create new heatmap.
    pub fn new(heatmap_type: HeatmapType, width: u32, height: u32) -> Self {
        Self {
            heatmap_type,
            width,
            height,
            cells: vec![HeatmapCell::default(); (width * height) as usize],
            min_value: 0.0,
            max_value: 1.0,
        }
    }

    /// Get type.
    pub fn heatmap_type(&self) -> HeatmapType {
        self.heatmap_type
    }

    /// Get dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Set cell value.
    pub fn set(&mut self, x: u32, y: u32, value: f32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.cells[idx].value = (self.cells[idx].value * self.cells[idx].samples as f32
                + value)
                / (self.cells[idx].samples + 1) as f32;
            self.cells[idx].samples += 1;

            self.min_value = self.min_value.min(value);
            self.max_value = self.max_value.max(value);
        }
    }

    /// Get cell value.
    pub fn get(&self, x: u32, y: u32) -> Option<&HeatmapCell> {
        if x < self.width && y < self.height {
            Some(&self.cells[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    /// Get normalized value (0-1).
    pub fn get_normalized(&self, x: u32, y: u32) -> Option<f32> {
        self.get(x, y).map(|cell| {
            if self.max_value > self.min_value {
                (cell.value - self.min_value) / (self.max_value - self.min_value)
            } else {
                0.0
            }
        })
    }

    /// Get color (heat gradient).
    pub fn get_color(&self, x: u32, y: u32) -> Option<[f32; 3]> {
        self.get_normalized(x, y).map(|t| {
            // Blue -> Green -> Yellow -> Red gradient
            if t < 0.25 {
                let s = t * 4.0;
                [0.0, s, 1.0 - s * 0.5]
            } else if t < 0.5 {
                let s = (t - 0.25) * 4.0;
                [s, 1.0, 0.5 - s * 0.5]
            } else if t < 0.75 {
                let s = (t - 0.5) * 4.0;
                [1.0, 1.0 - s * 0.5, 0.0]
            } else {
                let s = (t - 0.75) * 4.0;
                [1.0, 0.5 - s * 0.5, 0.0]
            }
        })
    }

    /// Clear.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = HeatmapCell::default();
        }
        self.min_value = 0.0;
        self.max_value = 1.0;
    }

    /// Get range.
    pub fn range(&self) -> (f32, f32) {
        (self.min_value, self.max_value)
    }
}

// ============================================================================
// Performance Counter Manager
// ============================================================================

/// Performance counter features.
#[derive(Debug, Clone, Copy, Default)]
pub struct PerfCounterFeatures {
    /// Hardware counters supported.
    pub hardware_counters: bool,
    /// Number of simultaneous counters.
    pub max_simultaneous_counters: u32,
    /// Supported counter types.
    pub supported_counters: u32, // Bitmask
}

/// Performance counter manager.
pub struct PerfCounterManager {
    /// Features.
    features: PerfCounterFeatures,
    /// Current counter set.
    current: CounterSet,
    /// Frame history.
    frame_history: Vec<CounterSet>,
    /// Max history.
    max_history: usize,
    /// Bottleneck detector.
    bottleneck_detector: BottleneckDetector,
    /// Heatmaps.
    heatmaps: Vec<Heatmap>,
    /// Enabled counters.
    enabled_counters: Vec<CounterType>,
    /// Is collecting.
    collecting: bool,
}

impl PerfCounterManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            features: PerfCounterFeatures::default(),
            current: CounterSet::new(),
            frame_history: Vec::new(),
            max_history: 120,
            bottleneck_detector: BottleneckDetector::new(),
            heatmaps: Vec::new(),
            enabled_counters: Vec::new(),
            collecting: false,
        }
    }

    /// Initialize with features.
    pub fn initialize(&mut self, features: PerfCounterFeatures) {
        self.features = features;
    }

    /// Start collecting.
    pub fn start_collection(&mut self, counters: &[CounterType]) {
        self.enabled_counters = counters.to_vec();
        self.collecting = true;
    }

    /// Stop collecting.
    pub fn stop_collection(&mut self) {
        self.collecting = false;
    }

    /// Record counter value.
    pub fn record(&mut self, counter_type: CounterType, value: u64) {
        if !self.collecting {
            return;
        }
        self.current.set(counter_type, value);
    }

    /// End frame.
    pub fn end_frame(&mut self) {
        if !self.collecting {
            return;
        }

        // Analyze bottlenecks
        self.bottleneck_detector.analyze(&self.current);

        // Store in history
        self.frame_history.push(core::mem::take(&mut self.current));
        while self.frame_history.len() > self.max_history {
            self.frame_history.remove(0);
        }
    }

    /// Get current counters.
    pub fn current(&self) -> &CounterSet {
        &self.current
    }

    /// Get frame history.
    pub fn history(&self) -> &[CounterSet] {
        &self.frame_history
    }

    /// Get bottleneck analysis.
    pub fn analyze_bottleneck(&mut self) -> BottleneckAnalysis {
        self.bottleneck_detector.analyze(&self.current)
    }

    /// Create heatmap.
    pub fn create_heatmap(&mut self, heatmap_type: HeatmapType, width: u32, height: u32) -> usize {
        self.heatmaps
            .push(Heatmap::new(heatmap_type, width, height));
        self.heatmaps.len() - 1
    }

    /// Get heatmap.
    pub fn heatmap(&self, index: usize) -> Option<&Heatmap> {
        self.heatmaps.get(index)
    }

    /// Get heatmap mutable.
    pub fn heatmap_mut(&mut self, index: usize) -> Option<&mut Heatmap> {
        self.heatmaps.get_mut(index)
    }

    /// Get features.
    pub fn features(&self) -> &PerfCounterFeatures {
        &self.features
    }

    /// Is collecting.
    pub fn is_collecting(&self) -> bool {
        self.collecting
    }
}

impl Default for PerfCounterManager {
    fn default() -> Self {
        Self::new()
    }
}
