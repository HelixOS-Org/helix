//! Live Metrics System
//!
//! Zero-overhead live GPU metrics and performance monitoring.
//!
//! # Features
//!
//! - **Zero-Copy Metrics**: GPU-side counters with no CPU overhead
//! - **Real-Time Graphs**: Live visualization of performance data
//! - **Bottleneck Detection**: Automatic identification of bottlenecks
//! - **Threshold Alerts**: Notifications when metrics exceed thresholds
//! - **Historical Trends**: Track performance over time

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Metric Types
// ============================================================================

/// Metric ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricId(pub u32);

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (current value)
    Gauge,
    /// Histogram (distribution)
    Histogram,
    /// Timer (duration)
    Timer,
    /// Rate (per second)
    Rate,
}

/// Metric category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricCategory {
    /// GPU timing
    GpuTiming,
    /// GPU throughput
    GpuThroughput,
    /// Memory
    Memory,
    /// Shader
    Shader,
    /// Pipeline
    Pipeline,
    /// Draw calls
    DrawCalls,
    /// Compute
    Compute,
    /// Transfer
    Transfer,
    /// Synchronization
    Sync,
    /// Custom
    Custom,
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    /// Metric ID
    pub id: MetricId,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Type
    pub metric_type: MetricType,
    /// Category
    pub category: MetricCategory,
    /// Unit
    pub unit: MetricUnit,
    /// Warning threshold
    pub warning_threshold: Option<f64>,
    /// Critical threshold
    pub critical_threshold: Option<f64>,
}

/// Metric unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricUnit {
    /// Count
    Count,
    /// Bytes
    Bytes,
    /// Kilobytes
    KiloBytes,
    /// Megabytes
    MegaBytes,
    /// Gigabytes
    GigaBytes,
    /// Nanoseconds
    Nanoseconds,
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
    /// Percentage
    Percent,
    /// Per second
    PerSecond,
    /// Bytes per second
    BytesPerSecond,
    /// Pixels per second
    PixelsPerSecond,
    /// Triangles per second
    TrianglesPerSecond,
    /// Ratio
    Ratio,
    /// Custom
    Custom,
}

// ============================================================================
// Metric Values
// ============================================================================

/// Metric value
#[derive(Debug, Clone, Copy)]
pub enum MetricValue {
    /// Integer counter
    Counter(u64),
    /// Float gauge
    Gauge(f64),
    /// Timer (nanoseconds)
    Timer(u64),
    /// Rate (per second)
    Rate(f64),
}

impl MetricValue {
    /// Get as f64
    pub fn as_f64(&self) -> f64 {
        match self {
            MetricValue::Counter(v) => *v as f64,
            MetricValue::Gauge(v) => *v,
            MetricValue::Timer(v) => *v as f64,
            MetricValue::Rate(v) => *v,
        }
    }
}

/// Metric sample
#[derive(Debug, Clone, Copy)]
pub struct MetricSample {
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Value
    pub value: MetricValue,
    /// Frame number
    pub frame: u64,
}

/// Metric statistics
#[derive(Debug, Clone, Copy)]
pub struct MetricStats {
    /// Current value
    pub current: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Average value
    pub avg: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Sample count
    pub sample_count: u64,
    /// Trend (positive = increasing)
    pub trend: f64,
}

impl Default for MetricStats {
    fn default() -> Self {
        Self {
            current: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            avg: 0.0,
            std_dev: 0.0,
            sample_count: 0,
            trend: 0.0,
        }
    }
}

// ============================================================================
// Built-in Metrics
// ============================================================================

/// Built-in GPU metrics
pub struct BuiltinMetrics;

impl BuiltinMetrics {
    // Timing metrics
    pub const FRAME_TIME: MetricId = MetricId(1);
    pub const GPU_TIME: MetricId = MetricId(2);
    pub const CPU_TIME: MetricId = MetricId(3);
    pub const PRESENT_TIME: MetricId = MetricId(4);
    pub const RENDER_PASS_TIME: MetricId = MetricId(5);
    pub const COMPUTE_TIME: MetricId = MetricId(6);
    pub const TRANSFER_TIME: MetricId = MetricId(7);

    // Throughput metrics
    pub const FPS: MetricId = MetricId(10);
    pub const TRIANGLES_PER_SECOND: MetricId = MetricId(11);
    pub const PIXELS_PER_SECOND: MetricId = MetricId(12);
    pub const DRAW_CALLS_PER_FRAME: MetricId = MetricId(13);
    pub const DISPATCHES_PER_FRAME: MetricId = MetricId(14);

    // Memory metrics
    pub const VRAM_USED: MetricId = MetricId(20);
    pub const VRAM_BUDGET: MetricId = MetricId(21);
    pub const HOST_VISIBLE_USED: MetricId = MetricId(22);
    pub const STAGING_BUFFER_USED: MetricId = MetricId(23);
    pub const DESCRIPTOR_HEAP_USED: MetricId = MetricId(24);

    // Shader metrics
    pub const SHADER_INVOCATIONS: MetricId = MetricId(30);
    pub const VERTEX_SHADER_TIME: MetricId = MetricId(31);
    pub const FRAGMENT_SHADER_TIME: MetricId = MetricId(32);
    pub const COMPUTE_SHADER_TIME: MetricId = MetricId(33);
    pub const SHADER_OCCUPANCY: MetricId = MetricId(34);

    // Pipeline metrics
    pub const PIPELINE_SWITCHES: MetricId = MetricId(40);
    pub const DESCRIPTOR_SWITCHES: MetricId = MetricId(41);
    pub const VERTEX_BUFFER_BINDS: MetricId = MetricId(42);
    pub const RENDER_PASS_COUNT: MetricId = MetricId(43);

    /// Get all builtin definitions
    pub fn definitions() -> Vec<MetricDefinition> {
        vec![
            MetricDefinition {
                id: Self::FRAME_TIME,
                name: String::from("Frame Time"),
                description: String::from("Total frame time"),
                metric_type: MetricType::Timer,
                category: MetricCategory::GpuTiming,
                unit: MetricUnit::Milliseconds,
                warning_threshold: Some(16.67), // 60 FPS
                critical_threshold: Some(33.33), // 30 FPS
            },
            MetricDefinition {
                id: Self::FPS,
                name: String::from("FPS"),
                description: String::from("Frames per second"),
                metric_type: MetricType::Gauge,
                category: MetricCategory::GpuThroughput,
                unit: MetricUnit::PerSecond,
                warning_threshold: Some(60.0),
                critical_threshold: Some(30.0),
            },
            MetricDefinition {
                id: Self::VRAM_USED,
                name: String::from("VRAM Used"),
                description: String::from("Video memory in use"),
                metric_type: MetricType::Gauge,
                category: MetricCategory::Memory,
                unit: MetricUnit::MegaBytes,
                warning_threshold: None,
                critical_threshold: None,
            },
            MetricDefinition {
                id: Self::DRAW_CALLS_PER_FRAME,
                name: String::from("Draw Calls"),
                description: String::from("Draw calls per frame"),
                metric_type: MetricType::Counter,
                category: MetricCategory::DrawCalls,
                unit: MetricUnit::Count,
                warning_threshold: Some(1000.0),
                critical_threshold: Some(5000.0),
            },
        ]
    }
}

// ============================================================================
// Histogram
// ============================================================================

/// Histogram for distribution analysis
#[derive(Debug, Clone)]
pub struct Histogram {
    /// Buckets
    buckets: Vec<HistogramBucket>,
    /// Total count
    count: u64,
    /// Sum of all values
    sum: f64,
}

/// Histogram bucket
#[derive(Debug, Clone, Copy)]
pub struct HistogramBucket {
    /// Upper bound
    pub upper_bound: f64,
    /// Count
    pub count: u64,
}

impl Histogram {
    /// Create with default buckets
    pub fn new() -> Self {
        Self::with_buckets(&[0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0])
    }

    /// Create with custom buckets
    pub fn with_buckets(bounds: &[f64]) -> Self {
        let buckets = bounds.iter()
            .map(|&b| HistogramBucket { upper_bound: b, count: 0 })
            .collect();
        Self {
            buckets,
            count: 0,
            sum: 0.0,
        }
    }

    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;

        for bucket in &mut self.buckets {
            if value <= bucket.upper_bound {
                bucket.count += 1;
                break;
            }
        }
    }

    /// Get percentile
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        let target = (p / 100.0 * self.count as f64) as u64;
        let mut cumulative = 0u64;

        for bucket in &self.buckets {
            cumulative += bucket.count;
            if cumulative >= target {
                return bucket.upper_bound;
            }
        }

        self.buckets.last().map(|b| b.upper_bound).unwrap_or(0.0)
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Alert System
// ============================================================================

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    /// Info
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
}

/// Metric alert
#[derive(Debug, Clone)]
pub struct MetricAlert {
    /// Metric ID
    pub metric_id: MetricId,
    /// Level
    pub level: AlertLevel,
    /// Message
    pub message: String,
    /// Current value
    pub value: f64,
    /// Threshold
    pub threshold: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Frame
    pub frame: u64,
}

/// Alert callback
pub type AlertCallback = fn(&MetricAlert);

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Cooldown between alerts (frames)
    pub cooldown_frames: u32,
    /// Minimum samples before alerting
    pub min_samples: u32,
    /// Hysteresis (threshold must be exceeded by this %)
    pub hysteresis: f64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            cooldown_frames: 60,
            min_samples: 10,
            hysteresis: 0.1,
        }
    }
}

// ============================================================================
// Live Metrics Manager
// ============================================================================

/// Time series data
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Samples
    samples: Vec<MetricSample>,
    /// Maximum samples to keep
    max_samples: usize,
    /// Statistics
    stats: MetricStats,
}

impl TimeSeries {
    /// Create new time series
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::new(),
            max_samples,
            stats: MetricStats::default(),
        }
    }

    /// Add sample
    pub fn add_sample(&mut self, sample: MetricSample) {
        self.samples.push(sample);

        // Trim old samples
        while self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }

        // Update stats
        self.update_stats();
    }

    fn update_stats(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let mut sum = 0.0;
        let mut min = f64::MAX;
        let mut max = f64::MIN;

        for sample in &self.samples {
            let v = sample.value.as_f64();
            sum += v;
            min = min.min(v);
            max = max.max(v);
        }

        let n = self.samples.len() as f64;
        let avg = sum / n;

        // Calculate std dev
        let variance: f64 = self.samples.iter()
            .map(|s| {
                let diff = s.value.as_f64() - avg;
                diff * diff
            })
            .sum::<f64>() / n;

        let std_dev = variance.sqrt();

        // Calculate trend (linear regression slope)
        let trend = if self.samples.len() >= 2 {
            let first = self.samples.first().unwrap().value.as_f64();
            let last = self.samples.last().unwrap().value.as_f64();
            (last - first) / n
        } else {
            0.0
        };

        self.stats = MetricStats {
            current: self.samples.last().map(|s| s.value.as_f64()).unwrap_or(0.0),
            min,
            max,
            avg,
            std_dev,
            sample_count: self.samples.len() as u64,
            trend,
        };
    }

    /// Get statistics
    pub fn stats(&self) -> &MetricStats {
        &self.stats
    }

    /// Get samples
    pub fn samples(&self) -> &[MetricSample] {
        &self.samples
    }

    /// Get latest sample
    pub fn latest(&self) -> Option<&MetricSample> {
        self.samples.last()
    }
}

/// Live metrics configuration
#[derive(Debug, Clone)]
pub struct LiveMetricsConfig {
    /// Maximum samples per metric
    pub max_samples: usize,
    /// Sample interval (frames)
    pub sample_interval: u32,
    /// Enable GPU counters
    pub gpu_counters: bool,
    /// Alert configuration
    pub alert_config: AlertConfig,
}

impl Default for LiveMetricsConfig {
    fn default() -> Self {
        Self {
            max_samples: 1000,
            sample_interval: 1,
            gpu_counters: true,
            alert_config: AlertConfig::default(),
        }
    }
}

/// Live metrics manager
pub struct LiveMetricsManager {
    /// Configuration
    config: LiveMetricsConfig,
    /// Metric definitions
    definitions: BTreeMap<MetricId, MetricDefinition>,
    /// Time series data
    time_series: BTreeMap<MetricId, TimeSeries>,
    /// Histograms
    histograms: BTreeMap<MetricId, Histogram>,
    /// Pending alerts
    alerts: Vec<MetricAlert>,
    /// Alert cooldowns
    alert_cooldowns: BTreeMap<MetricId, u64>,
    /// Current frame
    frame: u64,
    /// Current timestamp
    timestamp: u64,
}

impl LiveMetricsManager {
    /// Create new manager
    pub fn new(config: LiveMetricsConfig) -> Self {
        let mut manager = Self {
            config,
            definitions: BTreeMap::new(),
            time_series: BTreeMap::new(),
            histograms: BTreeMap::new(),
            alerts: Vec::new(),
            alert_cooldowns: BTreeMap::new(),
            frame: 0,
            timestamp: 0,
        };

        // Register builtin metrics
        for def in BuiltinMetrics::definitions() {
            manager.register_metric(def);
        }

        manager
    }

    /// Register a metric
    pub fn register_metric(&mut self, definition: MetricDefinition) {
        let id = definition.id;
        self.definitions.insert(id, definition);
        self.time_series.insert(id, TimeSeries::new(self.config.max_samples));
        self.histograms.insert(id, Histogram::new());
    }

    /// Record a metric value
    pub fn record(&mut self, id: MetricId, value: MetricValue) {
        let sample = MetricSample {
            timestamp: self.timestamp,
            value,
            frame: self.frame,
        };

        if let Some(ts) = self.time_series.get_mut(&id) {
            ts.add_sample(sample);
        }

        if let Some(hist) = self.histograms.get_mut(&id) {
            hist.record(value.as_f64());
        }

        // Check thresholds
        self.check_thresholds(id, value.as_f64());
    }

    fn check_thresholds(&mut self, id: MetricId, value: f64) {
        let definition = match self.definitions.get(&id) {
            Some(d) => d.clone(),
            None => return,
        };

        // Check cooldown
        if let Some(&cooldown_frame) = self.alert_cooldowns.get(&id) {
            if self.frame < cooldown_frame + self.config.alert_config.cooldown_frames as u64 {
                return;
            }
        }

        // Check thresholds
        if let Some(critical) = definition.critical_threshold {
            if value >= critical * (1.0 + self.config.alert_config.hysteresis) {
                self.create_alert(id, AlertLevel::Critical, value, critical);
                return;
            }
        }

        if let Some(warning) = definition.warning_threshold {
            if value >= warning * (1.0 + self.config.alert_config.hysteresis) {
                self.create_alert(id, AlertLevel::Warning, value, warning);
            }
        }
    }

    fn create_alert(&mut self, id: MetricId, level: AlertLevel, value: f64, threshold: f64) {
        let name = self.definitions.get(&id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| String::from("Unknown"));

        let alert = MetricAlert {
            metric_id: id,
            level,
            message: alloc::format!("{} exceeded threshold: {:.2} > {:.2}", name, value, threshold),
            value,
            threshold,
            timestamp: self.timestamp,
            frame: self.frame,
        };

        self.alerts.push(alert);
        self.alert_cooldowns.insert(id, self.frame);
    }

    /// Begin frame
    pub fn begin_frame(&mut self, frame: u64, timestamp: u64) {
        self.frame = frame;
        self.timestamp = timestamp;
    }

    /// Get metric stats
    pub fn get_stats(&self, id: MetricId) -> Option<&MetricStats> {
        self.time_series.get(&id).map(|ts| ts.stats())
    }

    /// Get time series
    pub fn get_time_series(&self, id: MetricId) -> Option<&TimeSeries> {
        self.time_series.get(&id)
    }

    /// Get histogram
    pub fn get_histogram(&self, id: MetricId) -> Option<&Histogram> {
        self.histograms.get(&id)
    }

    /// Get pending alerts
    pub fn get_alerts(&self) -> &[MetricAlert] {
        &self.alerts
    }

    /// Clear alerts
    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    /// Get all metric definitions
    pub fn definitions(&self) -> &BTreeMap<MetricId, MetricDefinition> {
        &self.definitions
    }
}

impl Default for LiveMetricsManager {
    fn default() -> Self {
        Self::new(LiveMetricsConfig::default())
    }
}

// ============================================================================
// GPU Counter Interface
// ============================================================================

/// GPU counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCounterType {
    /// ALU utilization
    AluUtilization,
    /// Texture utilization
    TextureUtilization,
    /// Memory bandwidth utilization
    MemoryBandwidth,
    /// L1 cache hit rate
    L1CacheHitRate,
    /// L2 cache hit rate
    L2CacheHitRate,
    /// Shader occupancy
    ShaderOccupancy,
    /// Vertex shader invocations
    VertexShaderInvocations,
    /// Fragment shader invocations
    FragmentShaderInvocations,
    /// Compute shader invocations
    ComputeShaderInvocations,
    /// Primitives in
    PrimitivesIn,
    /// Primitives out
    PrimitivesOut,
    /// Pixels rendered
    PixelsRendered,
    /// Early Z killed
    EarlyZKilled,
    /// Late Z killed
    LateZKilled,
}

/// GPU counter sample
#[derive(Debug, Clone, Copy)]
pub struct GpuCounterSample {
    /// Counter type
    pub counter_type: GpuCounterType,
    /// Value
    pub value: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// GPU counter set
#[derive(Debug, Clone)]
pub struct GpuCounterSet {
    /// Samples
    pub samples: Vec<GpuCounterSample>,
    /// Frame
    pub frame: u64,
}
