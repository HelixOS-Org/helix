//! Temporal Effects Types for Lumina
//!
//! This module provides temporal rendering infrastructure including
//! TAA, temporal accumulation, and history management.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Temporal Handles
// ============================================================================

/// History buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HistoryBufferHandle(pub u64);

impl HistoryBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HistoryBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// TAA resolve handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaaResolveHandle(pub u64);

impl TaaResolveHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for TaaResolveHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// History Buffer
// ============================================================================

/// History buffer create info
#[derive(Clone, Debug)]
pub struct HistoryBufferCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: HistoryFormat,
    /// History length (frames)
    pub history_length: u32,
    /// Enable mip chain
    pub with_mips: bool,
}

impl HistoryBufferCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            format: HistoryFormat::Rgba16Float,
            history_length: 2,
            with_mips: false,
        }
    }

    /// Color history
    pub fn color(width: u32, height: u32) -> Self {
        Self {
            format: HistoryFormat::Rgba16Float,
            ..Self::new(width, height)
        }
    }

    /// Depth history
    pub fn depth(width: u32, height: u32) -> Self {
        Self {
            format: HistoryFormat::R32Float,
            ..Self::new(width, height)
        }
    }

    /// Normal history
    pub fn normal(width: u32, height: u32) -> Self {
        Self {
            format: HistoryFormat::Rg16Float,
            ..Self::new(width, height)
        }
    }

    /// With history length
    pub fn with_length(mut self, frames: u32) -> Self {
        self.history_length = frames;
        self
    }

    /// With mipmaps
    pub fn with_mipmaps(mut self) -> Self {
        self.with_mips = true;
        self
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        let base = (self.width as u64) * (self.height as u64) * (self.format.bytes_per_pixel() as u64);
        base * (self.history_length as u64)
    }
}

impl Default for HistoryBufferCreateInfo {
    fn default() -> Self {
        Self::color(1920, 1080)
    }
}

/// History format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HistoryFormat {
    /// RGBA16 float
    #[default]
    Rgba16Float = 0,
    /// RGBA32 float
    Rgba32Float = 1,
    /// R11G11B10 float
    R11G11B10Float = 2,
    /// RG16 float
    Rg16Float = 3,
    /// R32 float
    R32Float = 4,
    /// R16 float
    R16Float = 5,
}

impl HistoryFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
            Self::R11G11B10Float => 4,
            Self::Rg16Float => 4,
            Self::R32Float => 4,
            Self::R16Float => 2,
        }
    }
}

// ============================================================================
// TAA Configuration
// ============================================================================

/// TAA create info
#[derive(Clone, Debug)]
pub struct TaaCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Quality preset
    pub quality: TaaQuality,
    /// Jitter sequence
    pub jitter_sequence: JitterSequence,
    /// Motion rejection
    pub motion_rejection: MotionRejection,
    /// History weight
    pub history_weight: f32,
}

impl TaaCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            quality: TaaQuality::Medium,
            jitter_sequence: JitterSequence::Halton23,
            motion_rejection: MotionRejection::ClampYCoCg,
            history_weight: 0.95,
        }
    }

    /// Performance preset
    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            quality: TaaQuality::Low,
            jitter_sequence: JitterSequence::Halton23,
            motion_rejection: MotionRejection::ClampRgb,
            history_weight: 0.9,
            ..Self::new(width, height)
        }
    }

    /// Quality preset
    pub fn quality_preset(width: u32, height: u32) -> Self {
        Self {
            quality: TaaQuality::High,
            jitter_sequence: JitterSequence::Halton23_16,
            motion_rejection: MotionRejection::ClampYCoCg,
            history_weight: 0.97,
            ..Self::new(width, height)
        }
    }

    /// With quality
    pub fn with_quality(mut self, quality: TaaQuality) -> Self {
        self.quality = quality;
        self
    }

    /// With history weight
    pub fn with_history_weight(mut self, weight: f32) -> Self {
        self.history_weight = weight;
        self
    }
}

impl Default for TaaCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// TAA quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TaaQuality {
    /// Low quality (fast)
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

impl TaaQuality {
    /// Sample count in neighborhood
    pub const fn neighborhood_size(&self) -> u32 {
        match self {
            Self::Low => 5,  // Plus pattern
            Self::Medium => 9,  // 3x3
            Self::High => 9,    // 3x3 with variance
            Self::Ultra => 25,  // 5x5
        }
    }

    /// Use variance clipping
    pub const fn use_variance_clipping(&self) -> bool {
        matches!(self, Self::High | Self::Ultra)
    }
}

// ============================================================================
// Jitter Sequences
// ============================================================================

/// Jitter sequence type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum JitterSequence {
    /// Halton 2,3 (8 samples)
    #[default]
    Halton23 = 0,
    /// Halton 2,3 (16 samples)
    Halton23_16 = 1,
    /// R2 sequence (8 samples)
    R2 = 2,
    /// R2 sequence (16 samples)
    R2_16 = 3,
    /// Uniform grid (4 samples)
    UniformGrid4 = 4,
    /// Uniform grid (16 samples)
    UniformGrid16 = 5,
    /// Blue noise (16 samples)
    BlueNoise16 = 6,
    /// No jitter
    None = 7,
}

impl JitterSequence {
    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::Halton23 => 8,
            Self::Halton23_16 => 16,
            Self::R2 => 8,
            Self::R2_16 => 16,
            Self::UniformGrid4 => 4,
            Self::UniformGrid16 => 16,
            Self::BlueNoise16 => 16,
            Self::None => 1,
        }
    }

    /// Get jitter offset for frame
    pub fn get_offset(&self, frame_index: u32) -> [f32; 2] {
        match self {
            Self::Halton23 | Self::Halton23_16 => {
                let n = frame_index % self.sample_count();
                [Self::halton(n + 1, 2) - 0.5, Self::halton(n + 1, 3) - 0.5]
            }
            Self::R2 | Self::R2_16 => {
                let n = frame_index % self.sample_count();
                let g = 1.32471795724;  // Plastic constant
                let a1 = 1.0 / g;
                let a2 = 1.0 / (g * g);
                [
                    (0.5 + a1 * (n as f32 + 1.0)) % 1.0 - 0.5,
                    (0.5 + a2 * (n as f32 + 1.0)) % 1.0 - 0.5,
                ]
            }
            Self::UniformGrid4 => {
                let n = frame_index % 4;
                let offsets = [
                    [-0.25, -0.25],
                    [0.25, -0.25],
                    [-0.25, 0.25],
                    [0.25, 0.25],
                ];
                offsets[n as usize]
            }
            Self::UniformGrid16 => {
                let n = frame_index % 16;
                let x = (n % 4) as f32;
                let y = (n / 4) as f32;
                [(x + 0.5) / 4.0 - 0.5, (y + 0.5) / 4.0 - 0.5]
            }
            Self::BlueNoise16 => {
                // Pre-computed blue noise samples
                let samples = [
                    [0.375, 0.125], [0.875, 0.375], [0.125, 0.625], [0.625, 0.875],
                    [0.625, 0.125], [0.125, 0.375], [0.875, 0.625], [0.375, 0.875],
                    [0.125, 0.125], [0.625, 0.375], [0.375, 0.625], [0.875, 0.875],
                    [0.375, 0.375], [0.875, 0.125], [0.125, 0.875], [0.625, 0.625],
                ];
                let n = (frame_index % 16) as usize;
                [samples[n][0] - 0.5, samples[n][1] - 0.5]
            }
            Self::None => [0.0, 0.0],
        }
    }

    /// Halton sequence
    fn halton(index: u32, base: u32) -> f32 {
        let mut f = 1.0;
        let mut r = 0.0;
        let mut i = index;
        while i > 0 {
            f /= base as f32;
            r += f * (i % base) as f32;
            i /= base;
        }
        r
    }
}

// ============================================================================
// Motion Rejection
// ============================================================================

/// Motion rejection mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MotionRejection {
    /// None
    None = 0,
    /// Clamp in RGB space
    ClampRgb = 1,
    /// Clamp in YCoCg space
    #[default]
    ClampYCoCg = 2,
    /// Variance clipping
    VarianceClip = 3,
    /// AABB clipping
    AabbClip = 4,
}

impl MotionRejection {
    /// Requires YCoCg conversion
    pub const fn requires_ycocg(&self) -> bool {
        matches!(self, Self::ClampYCoCg)
    }

    /// Requires variance calculation
    pub const fn requires_variance(&self) -> bool {
        matches!(self, Self::VarianceClip)
    }
}

// ============================================================================
// TAA Pass
// ============================================================================

/// TAA pass config
#[derive(Clone, Debug)]
pub struct TaaPassConfig {
    /// Current color input
    pub current_color: u64,
    /// History color
    pub history_color: HistoryBufferHandle,
    /// Motion vectors
    pub motion_vectors: u64,
    /// Depth buffer
    pub depth: u64,
    /// Output
    pub output: u64,
    /// Frame index
    pub frame_index: u32,
    /// Jitter offset
    pub jitter: [f32; 2],
    /// Settings
    pub settings: TaaSettings,
}

impl TaaPassConfig {
    /// Creates config
    pub fn new() -> Self {
        Self {
            current_color: 0,
            history_color: HistoryBufferHandle::NULL,
            motion_vectors: 0,
            depth: 0,
            output: 0,
            frame_index: 0,
            jitter: [0.0, 0.0],
            settings: TaaSettings::default(),
        }
    }

    /// With buffers
    pub fn with_buffers(
        mut self,
        current: u64,
        history: HistoryBufferHandle,
        motion: u64,
        output: u64,
    ) -> Self {
        self.current_color = current;
        self.history_color = history;
        self.motion_vectors = motion;
        self.output = output;
        self
    }

    /// With frame index
    pub fn with_frame(mut self, index: u32, jitter: [f32; 2]) -> Self {
        self.frame_index = index;
        self.jitter = jitter;
        self
    }
}

impl Default for TaaPassConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// TAA settings
#[derive(Clone, Copy, Debug)]
pub struct TaaSettings {
    /// History weight (0-1)
    pub history_weight: f32,
    /// Motion rejection mode
    pub rejection_mode: MotionRejection,
    /// Variance clipping gamma
    pub variance_gamma: f32,
    /// Sharpening amount
    pub sharpening: f32,
    /// Anti-flicker
    pub anti_flicker: f32,
    /// Velocity weight
    pub velocity_weight: f32,
    /// Depth weight
    pub depth_weight: f32,
}

impl TaaSettings {
    /// Default settings
    pub const fn default_settings() -> Self {
        Self {
            history_weight: 0.95,
            rejection_mode: MotionRejection::ClampYCoCg,
            variance_gamma: 1.0,
            sharpening: 0.0,
            anti_flicker: 0.5,
            velocity_weight: 1.0,
            depth_weight: 1.0,
        }
    }

    /// Performance settings
    pub const fn performance() -> Self {
        Self {
            history_weight: 0.9,
            rejection_mode: MotionRejection::ClampRgb,
            variance_gamma: 1.0,
            sharpening: 0.0,
            anti_flicker: 0.3,
            velocity_weight: 1.0,
            depth_weight: 0.5,
        }
    }

    /// Quality settings
    pub const fn quality() -> Self {
        Self {
            history_weight: 0.97,
            rejection_mode: MotionRejection::VarianceClip,
            variance_gamma: 1.25,
            sharpening: 0.1,
            anti_flicker: 0.7,
            velocity_weight: 1.0,
            depth_weight: 1.0,
        }
    }

    /// With sharpening
    pub const fn with_sharpening(mut self, amount: f32) -> Self {
        self.sharpening = amount;
        self
    }
}

impl Default for TaaSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

/// TAA GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TaaGpuParams {
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Jitter offset
    pub jitter: [f32; 2],
    /// Previous jitter
    pub prev_jitter: [f32; 2],
    /// History weight
    pub history_weight: f32,
    /// Variance gamma
    pub variance_gamma: f32,
    /// Sharpening
    pub sharpening: f32,
    /// Anti-flicker
    pub anti_flicker: f32,
    /// Velocity weight
    pub velocity_weight: f32,
    /// Depth weight
    pub depth_weight: f32,
    /// Flags
    pub flags: u32,
    /// Frame index
    pub frame_index: u32,
    /// Padding
    pub _padding: [u32; 2],
}

impl TaaGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Use YCoCg flag
    pub const FLAG_YCOCG: u32 = 1 << 0;
    /// Use variance clipping flag
    pub const FLAG_VARIANCE_CLIP: u32 = 1 << 1;
    /// Apply sharpening flag
    pub const FLAG_SHARPENING: u32 = 1 << 2;
    /// First frame flag
    pub const FLAG_FIRST_FRAME: u32 = 1 << 3;
}

// ============================================================================
// Temporal Accumulation
// ============================================================================

/// Temporal accumulation config
#[derive(Clone, Debug)]
pub struct TemporalAccumulationConfig {
    /// Current input
    pub current: u64,
    /// History buffer
    pub history: HistoryBufferHandle,
    /// Output
    pub output: u64,
    /// Accumulation mode
    pub mode: AccumulationMode,
    /// Max samples
    pub max_samples: u32,
    /// Current sample count
    pub sample_count: u32,
    /// Reset accumulation
    pub reset: bool,
}

impl TemporalAccumulationConfig {
    /// Creates config
    pub fn new(current: u64, history: HistoryBufferHandle) -> Self {
        Self {
            current,
            history,
            output: 0,
            mode: AccumulationMode::Average,
            max_samples: u32::MAX,
            sample_count: 0,
            reset: false,
        }
    }

    /// With mode
    pub fn with_mode(mut self, mode: AccumulationMode) -> Self {
        self.mode = mode;
        self
    }

    /// With max samples
    pub fn with_max_samples(mut self, max: u32) -> Self {
        self.max_samples = max;
        self
    }

    /// Reset
    pub fn reset_accumulation(mut self) -> Self {
        self.reset = true;
        self.sample_count = 0;
        self
    }
}

impl Default for TemporalAccumulationConfig {
    fn default() -> Self {
        Self::new(0, HistoryBufferHandle::NULL)
    }
}

/// Accumulation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AccumulationMode {
    /// Running average
    #[default]
    Average = 0,
    /// Exponential moving average
    ExponentialAverage = 1,
    /// Maximum
    Maximum = 2,
    /// Minimum
    Minimum = 3,
    /// Sum
    Sum = 4,
}

/// Temporal accumulation GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AccumulationGpuParams {
    /// Screen dimensions
    pub screen_size: [u32; 2],
    /// Sample count
    pub sample_count: u32,
    /// Max samples
    pub max_samples: u32,
    /// Blend weight
    pub blend_weight: f32,
    /// Mode
    pub mode: u32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: u32,
}

impl AccumulationGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Reset flag
    pub const FLAG_RESET: u32 = 1 << 0;
}

// ============================================================================
// History Management
// ============================================================================

/// History manager config
#[derive(Clone, Debug)]
pub struct HistoryManagerConfig {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// History buffers
    pub buffers: HistoryBufferSet,
    /// Auto-resize
    pub auto_resize: bool,
}

impl HistoryManagerConfig {
    /// Creates config
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            buffers: HistoryBufferSet::default(),
            auto_resize: true,
        }
    }

    /// With buffer set
    pub fn with_buffers(mut self, buffers: HistoryBufferSet) -> Self {
        self.buffers = buffers;
        self
    }
}

impl Default for HistoryManagerConfig {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// History buffer set
#[derive(Clone, Copy, Debug, Default)]
pub struct HistoryBufferSet {
    /// Color history
    pub color: bool,
    /// Depth history
    pub depth: bool,
    /// Normal history
    pub normal: bool,
    /// Motion history
    pub motion: bool,
    /// Lighting history
    pub lighting: bool,
}

impl HistoryBufferSet {
    /// TAA buffer set
    pub const fn taa() -> Self {
        Self {
            color: true,
            depth: true,
            normal: false,
            motion: true,
            lighting: false,
        }
    }

    /// Full buffer set
    pub const fn full() -> Self {
        Self {
            color: true,
            depth: true,
            normal: true,
            motion: true,
            lighting: true,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Temporal statistics
#[derive(Clone, Debug, Default)]
pub struct TemporalStats {
    /// Frames accumulated
    pub frames_accumulated: u32,
    /// Pixels updated
    pub pixels_updated: u64,
    /// History rejection ratio
    pub rejection_ratio: f32,
    /// Resolve time (microseconds)
    pub resolve_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
