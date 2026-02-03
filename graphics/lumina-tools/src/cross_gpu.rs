//! Cross-GPU Analysis
//!
//! Revolutionary cross-GPU analysis system that compares shader behavior
//! and performance across different GPU vendors and architectures.
//!
//! # Features
//!
//! - **Vendor Comparison**: Compare performance across NVIDIA, AMD, Intel, Apple
//! - **Compatibility Check**: Detect vendor-specific issues
//! - **Performance Matrix**: Generate performance comparison matrix
//! - **Regression Detection**: Detect performance regressions across drivers
//! - **Optimization Hints**: Vendor-specific optimization suggestions

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// GPU Identification
// ============================================================================

/// GPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuVendor {
    /// NVIDIA
    Nvidia,
    /// AMD
    Amd,
    /// Intel
    Intel,
    /// Apple
    Apple,
    /// Qualcomm
    Qualcomm,
    /// ARM (Mali)
    Arm,
    /// Imagination (PowerVR)
    Imagination,
    /// Unknown vendor
    Unknown,
}

impl GpuVendor {
    /// From PCI vendor ID
    pub fn from_vendor_id(id: u32) -> Self {
        match id {
            0x10DE => Self::Nvidia,
            0x1002 => Self::Amd,
            0x8086 => Self::Intel,
            0x106B => Self::Apple,
            0x5143 => Self::Qualcomm,
            0x13B5 => Self::Arm,
            0x1010 => Self::Imagination,
            _ => Self::Unknown,
        }
    }

    /// Get vendor name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Intel => "Intel",
            Self::Apple => "Apple",
            Self::Qualcomm => "Qualcomm",
            Self::Arm => "ARM",
            Self::Imagination => "Imagination",
            Self::Unknown => "Unknown",
        }
    }
}

/// GPU generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GpuGeneration {
    // NVIDIA
    NvidiaTuring,      // RTX 20 series
    NvidiaAmpere,      // RTX 30 series
    NvidiaAda,         // RTX 40 series
    NvidiaBlackwell,   // RTX 50 series

    // AMD
    AmdRdna1,          // RX 5000 series
    AmdRdna2,          // RX 6000 series
    AmdRdna3,          // RX 7000 series
    AmdRdna4,          // RX 8000 series

    // Intel
    IntelGen9,         // HD Graphics 500 series
    IntelGen11,        // Iris Plus Graphics
    IntelGen12,        // Iris Xe
    IntelArc,          // Arc A-series

    // Apple
    AppleM1,
    AppleM2,
    AppleM3,
    AppleM4,

    // Mobile
    AdrenoGen6,
    AdrenoGen7,
    MaliValhall,
    MaliGen5,

    /// Unknown generation
    Unknown,
}

/// Complete GPU identifier
#[derive(Debug, Clone)]
pub struct GpuIdentifier {
    /// Vendor
    pub vendor: GpuVendor,
    /// Generation
    pub generation: GpuGeneration,
    /// Device name
    pub device_name: String,
    /// Device ID
    pub device_id: u32,
    /// Driver version
    pub driver_version: String,
    /// Vulkan API version
    pub api_version: (u32, u32, u32),
    /// Features
    pub features: GpuFeatures,
}

/// GPU features
#[derive(Debug, Clone, Default)]
pub struct GpuFeatures {
    /// Ray tracing support
    pub ray_tracing: bool,
    /// Mesh shaders support
    pub mesh_shaders: bool,
    /// Variable rate shading
    pub variable_rate_shading: bool,
    /// 16-bit float support
    pub float16: bool,
    /// 64-bit integer support
    pub int64: bool,
    /// Descriptor indexing
    pub descriptor_indexing: bool,
    /// Buffer device address
    pub buffer_device_address: bool,
    /// Timeline semaphores
    pub timeline_semaphores: bool,
    /// Cooperative matrix (tensor cores)
    pub cooperative_matrix: bool,
    /// Subgroup size control
    pub subgroup_size_control: bool,
}

// ============================================================================
// Compatibility Analysis
// ============================================================================

/// Compatibility issue
#[derive(Debug, Clone)]
pub struct CompatibilityIssue {
    /// Issue type
    pub issue_type: CompatibilityIssueType,
    /// Severity
    pub severity: CompatibilitySeverity,
    /// Affected vendors
    pub affected_vendors: Vec<GpuVendor>,
    /// Description
    pub description: String,
    /// Location in shader
    pub location: Option<ShaderLocation>,
    /// Workaround
    pub workaround: Option<String>,
}

/// Compatibility issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatibilityIssueType {
    /// Missing feature
    MissingFeature,
    /// Extension not supported
    UnsupportedExtension,
    /// Precision difference
    PrecisionDifference,
    /// Behavior difference
    BehaviorDifference,
    /// Performance cliff
    PerformanceCliff,
    /// Driver bug
    DriverBug,
    /// Subgroup size mismatch
    SubgroupSizeMismatch,
    /// Memory alignment issue
    MemoryAlignment,
    /// Descriptor limit exceeded
    DescriptorLimit,
}

/// Compatibility severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilitySeverity {
    /// Info - might behave differently but works
    Info,
    /// Warning - may cause issues
    Warning,
    /// Error - will not work
    Error,
    /// Fatal - will crash
    Fatal,
}

/// Shader location
#[derive(Debug, Clone)]
pub struct ShaderLocation {
    /// File
    pub file: String,
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
    /// Instruction index (SPIR-V)
    pub instruction: Option<u32>,
}

/// Compatibility report
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    /// Shader/resource name
    pub name: String,
    /// Issues found
    pub issues: Vec<CompatibilityIssue>,
    /// Tested GPUs
    pub tested_gpus: Vec<GpuIdentifier>,
    /// Overall compatibility score (0-100)
    pub compatibility_score: u8,
    /// Pass/fail per vendor
    pub vendor_status: BTreeMap<String, bool>,
}

impl CompatibilityReport {
    /// Check if all vendors pass
    pub fn all_pass(&self) -> bool {
        self.vendor_status.values().all(|&v| v)
    }

    /// Get failing vendors
    pub fn failing_vendors(&self) -> Vec<&str> {
        self.vendor_status.iter()
            .filter(|(_, &pass)| !pass)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.issues.iter()
            .filter(|i| i.severity >= CompatibilitySeverity::Error)
            .count()
    }
}

// ============================================================================
// Performance Comparison
// ============================================================================

/// Performance sample for a GPU
#[derive(Debug, Clone)]
pub struct GpuPerformanceSample {
    /// GPU identifier
    pub gpu: GpuIdentifier,
    /// Frame time in microseconds
    pub frame_time_us: u64,
    /// GPU time in microseconds
    pub gpu_time_us: u64,
    /// Triangle throughput (per second)
    pub triangle_throughput: u64,
    /// Pixel throughput (per second)
    pub pixel_throughput: u64,
    /// Memory bandwidth usage (bytes/second)
    pub memory_bandwidth: u64,
    /// Shader occupancy (0-100%)
    pub shader_occupancy: u8,
    /// Power consumption (watts, if available)
    pub power_watts: Option<f32>,
    /// Temperature (celsius, if available)
    pub temperature_c: Option<u8>,
}

/// Performance comparison result
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    /// Shader/pass name
    pub name: String,
    /// Samples per GPU
    pub samples: Vec<GpuPerformanceSample>,
    /// Fastest GPU
    pub fastest: GpuVendor,
    /// Slowest GPU
    pub slowest: GpuVendor,
    /// Performance ratio (slowest / fastest)
    pub performance_ratio: f32,
    /// Anomalies detected
    pub anomalies: Vec<PerformanceAnomaly>,
}

/// Performance anomaly
#[derive(Debug, Clone)]
pub struct PerformanceAnomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Affected GPU
    pub affected_gpu: GpuIdentifier,
    /// Description
    pub description: String,
    /// Expected value
    pub expected: f64,
    /// Actual value
    pub actual: f64,
    /// Deviation percentage
    pub deviation_percent: f32,
}

/// Anomaly type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    /// Unexpectedly slow
    SlowerThanExpected,
    /// Unexpectedly fast (may indicate error)
    FasterThanExpected,
    /// High variance
    HighVariance,
    /// Performance regression from previous driver
    DriverRegression,
    /// Thermal throttling detected
    ThermalThrottling,
    /// Memory bottleneck
    MemoryBottleneck,
}

/// Performance matrix
#[derive(Debug, Clone)]
pub struct PerformanceMatrix {
    /// Test names (rows)
    pub tests: Vec<String>,
    /// GPU names (columns)
    pub gpus: Vec<String>,
    /// Data[test_index][gpu_index] in microseconds
    pub data: Vec<Vec<u64>>,
    /// Normalized data (fastest = 1.0)
    pub normalized: Vec<Vec<f32>>,
}

impl PerformanceMatrix {
    /// Create new matrix
    pub fn new(tests: Vec<String>, gpus: Vec<String>) -> Self {
        let rows = tests.len();
        let cols = gpus.len();
        Self {
            tests,
            gpus,
            data: vec![vec![0; cols]; rows],
            normalized: vec![vec![1.0; cols]; rows],
        }
    }

    /// Set data point
    pub fn set(&mut self, test: usize, gpu: usize, value: u64) {
        if test < self.data.len() && gpu < self.data[test].len() {
            self.data[test][gpu] = value;
        }
    }

    /// Normalize data
    pub fn normalize(&mut self) {
        for (row_idx, row) in self.data.iter().enumerate() {
            let min = *row.iter().filter(|&&v| v > 0).min().unwrap_or(&1);
            for (col_idx, &value) in row.iter().enumerate() {
                self.normalized[row_idx][col_idx] = if min > 0 && value > 0 {
                    value as f32 / min as f32
                } else {
                    1.0
                };
            }
        }
    }

    /// Get winner for test
    pub fn winner(&self, test: usize) -> Option<&str> {
        let row = self.data.get(test)?;
        let (idx, _) = row.iter()
            .enumerate()
            .filter(|(_, &v)| v > 0)
            .min_by_key(|(_, &v)| v)?;
        self.gpus.get(idx).map(|s| s.as_str())
    }
}

// ============================================================================
// Vendor-Specific Hints
// ============================================================================

/// Vendor-specific optimization hint
#[derive(Debug, Clone)]
pub struct VendorHint {
    /// Target vendor
    pub vendor: GpuVendor,
    /// Hint category
    pub category: HintCategory,
    /// Description
    pub description: String,
    /// Code location
    pub location: Option<ShaderLocation>,
    /// Suggested change
    pub suggestion: Option<String>,
    /// Expected improvement
    pub expected_improvement: Option<f32>,
}

/// Hint category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintCategory {
    /// ALU optimization
    Alu,
    /// Memory access pattern
    Memory,
    /// Texture sampling
    Texture,
    /// Branching
    Branching,
    /// Register usage
    Registers,
    /// Occupancy
    Occupancy,
    /// Wave/warp utilization
    WaveUtilization,
    /// Instruction scheduling
    Scheduling,
}

/// Vendor hint database
pub struct VendorHintDatabase {
    /// Hints by vendor
    hints: BTreeMap<GpuVendor, Vec<VendorHintRule>>,
}

/// Vendor hint rule
#[derive(Debug, Clone)]
pub struct VendorHintRule {
    /// Rule ID
    pub id: u32,
    /// Pattern to match
    pub pattern: String,
    /// Hint to generate
    pub hint: String,
    /// Category
    pub category: HintCategory,
    /// Impact estimate (0.0 - 1.0)
    pub impact: f32,
}

impl VendorHintDatabase {
    /// Create new database
    pub fn new() -> Self {
        let mut hints = BTreeMap::new();

        // NVIDIA hints
        hints.insert(GpuVendor::Nvidia, vec![
            VendorHintRule {
                id: 1,
                pattern: String::from("divergent_branch"),
                hint: String::from("NVIDIA GPUs prefer uniform branches. Consider using warp intrinsics."),
                category: HintCategory::Branching,
                impact: 0.3,
            },
            VendorHintRule {
                id: 2,
                pattern: String::from("fp16_math"),
                hint: String::from("NVIDIA Ada+ has 2x FP16 throughput. Use f16 where possible."),
                category: HintCategory::Alu,
                impact: 0.5,
            },
        ]);

        // AMD hints
        hints.insert(GpuVendor::Amd, vec![
            VendorHintRule {
                id: 1,
                pattern: String::from("wave64"),
                hint: String::from("AMD RDNA prefers Wave32. Consider workgroup size adjustments."),
                category: HintCategory::WaveUtilization,
                impact: 0.2,
            },
            VendorHintRule {
                id: 2,
                pattern: String::from("lds_bank_conflict"),
                hint: String::from("AMD has 32 LDS banks. Avoid stride-32 access patterns."),
                category: HintCategory::Memory,
                impact: 0.4,
            },
        ]);

        // Intel hints
        hints.insert(GpuVendor::Intel, vec![
            VendorHintRule {
                id: 1,
                pattern: String::from("register_spill"),
                hint: String::from("Intel GPUs have fewer registers. Reduce local variable count."),
                category: HintCategory::Registers,
                impact: 0.5,
            },
        ]);

        Self { hints }
    }

    /// Get hints for vendor
    pub fn get_hints(&self, vendor: GpuVendor) -> &[VendorHintRule] {
        self.hints.get(&vendor).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl Default for VendorHintDatabase {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cross-GPU Analyzer
// ============================================================================

/// Cross-GPU analyzer configuration
#[derive(Debug, Clone)]
pub struct CrossGpuConfig {
    /// Reference GPUs to test against
    pub reference_gpus: Vec<GpuVendor>,
    /// Enable vendor hints
    pub vendor_hints: bool,
    /// Enable compatibility checking
    pub compatibility_check: bool,
    /// Enable performance comparison
    pub performance_compare: bool,
    /// Anomaly detection threshold
    pub anomaly_threshold: f32,
}

impl Default for CrossGpuConfig {
    fn default() -> Self {
        Self {
            reference_gpus: vec![
                GpuVendor::Nvidia,
                GpuVendor::Amd,
                GpuVendor::Intel,
            ],
            vendor_hints: true,
            compatibility_check: true,
            performance_compare: true,
            anomaly_threshold: 0.5, // 50% deviation
        }
    }
}

/// Cross-GPU analysis result
#[derive(Debug, Clone)]
pub struct CrossGpuAnalysis {
    /// Shader name
    pub name: String,
    /// Compatibility report
    pub compatibility: CompatibilityReport,
    /// Performance comparison
    pub performance: PerformanceComparison,
    /// Vendor hints
    pub hints: Vec<VendorHint>,
    /// Recommended target
    pub recommended_target: GpuVendor,
    /// Overall score
    pub overall_score: u8,
}

/// Cross-GPU analyzer
pub struct CrossGpuAnalyzer {
    /// Configuration
    config: CrossGpuConfig,
    /// Vendor hint database
    hint_db: VendorHintDatabase,
    /// Known GPU profiles
    gpu_profiles: Vec<GpuProfile>,
}

/// GPU profile with capabilities
#[derive(Debug, Clone)]
pub struct GpuProfile {
    /// GPU identifier
    pub gpu: GpuIdentifier,
    /// Compute units
    pub compute_units: u32,
    /// Clock speed (MHz)
    pub clock_mhz: u32,
    /// Memory bandwidth (GB/s)
    pub memory_bandwidth_gbps: u32,
    /// FLOPS (TFLOPS)
    pub tflops: f32,
    /// Texture units
    pub texture_units: u32,
    /// ROPs
    pub rops: u32,
}

impl CrossGpuAnalyzer {
    /// Create new analyzer
    pub fn new(config: CrossGpuConfig) -> Self {
        Self {
            config,
            hint_db: VendorHintDatabase::new(),
            gpu_profiles: Self::init_profiles(),
        }
    }

    fn init_profiles() -> Vec<GpuProfile> {
        vec![
            GpuProfile {
                gpu: GpuIdentifier {
                    vendor: GpuVendor::Nvidia,
                    generation: GpuGeneration::NvidiaAda,
                    device_name: String::from("GeForce RTX 4090"),
                    device_id: 0x2684,
                    driver_version: String::from("550.0"),
                    api_version: (1, 3, 0),
                    features: GpuFeatures {
                        ray_tracing: true,
                        mesh_shaders: true,
                        variable_rate_shading: true,
                        float16: true,
                        int64: true,
                        descriptor_indexing: true,
                        buffer_device_address: true,
                        timeline_semaphores: true,
                        cooperative_matrix: true,
                        subgroup_size_control: true,
                    },
                },
                compute_units: 128,
                clock_mhz: 2520,
                memory_bandwidth_gbps: 1008,
                tflops: 82.6,
                texture_units: 512,
                rops: 176,
            },
            GpuProfile {
                gpu: GpuIdentifier {
                    vendor: GpuVendor::Amd,
                    generation: GpuGeneration::AmdRdna3,
                    device_name: String::from("Radeon RX 7900 XTX"),
                    device_id: 0x744C,
                    driver_version: String::from("24.1.1"),
                    api_version: (1, 3, 0),
                    features: GpuFeatures {
                        ray_tracing: true,
                        mesh_shaders: true,
                        variable_rate_shading: true,
                        float16: true,
                        int64: true,
                        descriptor_indexing: true,
                        buffer_device_address: true,
                        timeline_semaphores: true,
                        cooperative_matrix: false,
                        subgroup_size_control: true,
                    },
                },
                compute_units: 96,
                clock_mhz: 2500,
                memory_bandwidth_gbps: 960,
                tflops: 61.0,
                texture_units: 384,
                rops: 192,
            },
        ]
    }

    /// Analyze shader for cross-GPU compatibility
    pub fn analyze(&self, _shader_spirv: &[u8], name: &str) -> CrossGpuAnalysis {
        // In real implementation, would parse SPIR-V and analyze

        let compatibility = CompatibilityReport {
            name: name.into(),
            issues: Vec::new(),
            tested_gpus: Vec::new(),
            compatibility_score: 100,
            vendor_status: BTreeMap::new(),
        };

        let performance = PerformanceComparison {
            name: name.into(),
            samples: Vec::new(),
            fastest: GpuVendor::Nvidia,
            slowest: GpuVendor::Intel,
            performance_ratio: 1.0,
            anomalies: Vec::new(),
        };

        CrossGpuAnalysis {
            name: name.into(),
            compatibility,
            performance,
            hints: Vec::new(),
            recommended_target: GpuVendor::Nvidia,
            overall_score: 100,
        }
    }

    /// Check compatibility with specific GPU
    pub fn check_compatibility(&self, _shader_spirv: &[u8], gpu: &GpuIdentifier) -> Vec<CompatibilityIssue> {
        // Would analyze SPIR-V for vendor-specific issues
        Vec::new()
    }

    /// Get vendor hints
    pub fn get_vendor_hints(&self, vendor: GpuVendor) -> Vec<VendorHint> {
        self.hint_db.get_hints(vendor)
            .iter()
            .map(|rule| VendorHint {
                vendor,
                category: rule.category,
                description: rule.hint.clone(),
                location: None,
                suggestion: None,
                expected_improvement: Some(rule.impact),
            })
            .collect()
    }

    /// Get GPU profile
    pub fn get_profile(&self, vendor: GpuVendor) -> Option<&GpuProfile> {
        self.gpu_profiles.iter().find(|p| p.gpu.vendor == vendor)
    }
}

impl Default for CrossGpuAnalyzer {
    fn default() -> Self {
        Self::new(CrossGpuConfig::default())
    }
}
