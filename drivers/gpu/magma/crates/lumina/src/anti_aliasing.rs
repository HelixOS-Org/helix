//! Anti-Aliasing Types for Lumina
//!
//! This module provides anti-aliasing infrastructure including
//! MSAA, FXAA, TAA, SMAA, and other AA techniques.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Anti-Aliasing Handles
// ============================================================================

/// Anti-aliasing handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AntiAliasingHandle(pub u64);

impl AntiAliasingHandle {
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

impl Default for AntiAliasingHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// TAA history handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaaHistoryHandle(pub u64);

impl TaaHistoryHandle {
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

impl Default for TaaHistoryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Anti-Aliasing Method
// ============================================================================

/// Anti-aliasing method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AntiAliasingMethod {
    /// None
    None = 0,
    /// MSAA (Multisample Anti-Aliasing)
    #[default]
    Msaa = 1,
    /// FXAA (Fast Approximate Anti-Aliasing)
    Fxaa = 2,
    /// SMAA (Subpixel Morphological Anti-Aliasing)
    Smaa = 3,
    /// TAA (Temporal Anti-Aliasing)
    Taa = 4,
    /// DLSS (Deep Learning Super Sampling)
    Dlss = 5,
    /// FSR (FidelityFX Super Resolution)
    Fsr = 6,
    /// XeSS (Intel Xe Super Sampling)
    Xess = 7,
}

impl AntiAliasingMethod {
    /// Is temporal
    pub const fn is_temporal(&self) -> bool {
        matches!(self, Self::Taa | Self::Dlss | Self::Fsr | Self::Xess)
    }

    /// Requires motion vectors
    pub const fn requires_motion_vectors(&self) -> bool {
        matches!(self, Self::Taa | Self::Dlss | Self::Fsr | Self::Xess)
    }

    /// Requires depth
    pub const fn requires_depth(&self) -> bool {
        matches!(self, Self::Smaa | Self::Taa | Self::Dlss | Self::Fsr | Self::Xess)
    }
}

/// Anti-aliasing settings
#[derive(Clone, Debug)]
pub struct AntiAliasingSettings {
    /// Method
    pub method: AntiAliasingMethod,
    /// Quality
    pub quality: AaQuality,
    /// Sharpness (for post-process AA)
    pub sharpness: f32,
}

impl AntiAliasingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            method: AntiAliasingMethod::Taa,
            quality: AaQuality::Medium,
            sharpness: 0.5,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            method: AntiAliasingMethod::None,
            ..Self::new()
        }
    }

    /// With method
    pub fn with_method(mut self, method: AntiAliasingMethod) -> Self {
        self.method = method;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: AaQuality) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for AntiAliasingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// AA quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AaQuality {
    /// Low
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
    /// Ultra
    Ultra = 3,
}

// ============================================================================
// MSAA
// ============================================================================

/// MSAA settings
#[derive(Clone, Debug)]
pub struct MsaaSettings {
    /// Sample count (2, 4, 8)
    pub samples: u32,
    /// Alpha to coverage
    pub alpha_to_coverage: bool,
    /// Sample shading
    pub sample_shading: bool,
    /// Min sample shading
    pub min_sample_shading: f32,
}

impl MsaaSettings {
    /// Creates settings
    pub fn new(samples: u32) -> Self {
        Self {
            samples: samples.max(1).min(8),
            alpha_to_coverage: false,
            sample_shading: false,
            min_sample_shading: 1.0,
        }
    }

    /// 2x MSAA
    pub fn msaa_2x() -> Self {
        Self::new(2)
    }

    /// 4x MSAA
    pub fn msaa_4x() -> Self {
        Self::new(4)
    }

    /// 8x MSAA
    pub fn msaa_8x() -> Self {
        Self::new(8)
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self::new(1)
    }

    /// With alpha to coverage
    pub fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage = true;
        self
    }

    /// With sample shading
    pub fn with_sample_shading(mut self, min: f32) -> Self {
        self.sample_shading = true;
        self.min_sample_shading = min;
        self
    }

    /// Memory multiplier
    pub fn memory_multiplier(&self) -> u32 {
        self.samples
    }
}

impl Default for MsaaSettings {
    fn default() -> Self {
        Self::msaa_4x()
    }
}

/// MSAA resolve mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MsaaResolveMode {
    /// Average
    #[default]
    Average = 0,
    /// Min
    Min = 1,
    /// Max
    Max = 2,
    /// Sample zero
    SampleZero = 3,
}

// ============================================================================
// FXAA
// ============================================================================

/// FXAA settings
#[derive(Clone, Debug)]
pub struct FxaaSettings {
    /// Quality preset
    pub quality: FxaaQuality,
    /// Edge threshold
    pub edge_threshold: f32,
    /// Edge threshold min
    pub edge_threshold_min: f32,
    /// Subpixel quality
    pub subpixel_quality: f32,
}

impl FxaaSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            quality: FxaaQuality::Quality29,
            edge_threshold: 0.166,
            edge_threshold_min: 0.0833,
            subpixel_quality: 0.75,
        }
    }

    /// Low quality (fast)
    pub fn low() -> Self {
        Self {
            quality: FxaaQuality::Quality10,
            edge_threshold: 0.25,
            edge_threshold_min: 0.125,
            subpixel_quality: 0.5,
        }
    }

    /// High quality
    pub fn high() -> Self {
        Self {
            quality: FxaaQuality::Quality39,
            edge_threshold: 0.125,
            edge_threshold_min: 0.0625,
            subpixel_quality: 1.0,
        }
    }

    /// With subpixel quality
    pub fn with_subpixel_quality(mut self, quality: f32) -> Self {
        self.subpixel_quality = quality;
        self
    }
}

impl Default for FxaaSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// FXAA quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FxaaQuality {
    /// Quality 10 (fastest)
    Quality10 = 10,
    /// Quality 15
    Quality15 = 15,
    /// Quality 20
    Quality20 = 20,
    /// Quality 25
    Quality25 = 25,
    /// Quality 29 (default)
    #[default]
    Quality29 = 29,
    /// Quality 39 (best)
    Quality39 = 39,
}

impl FxaaQuality {
    /// Search steps
    pub const fn search_steps(&self) -> u32 {
        match self {
            Self::Quality10 => 3,
            Self::Quality15 => 4,
            Self::Quality20 => 5,
            Self::Quality25 => 6,
            Self::Quality29 => 8,
            Self::Quality39 => 12,
        }
    }
}

/// FXAA GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FxaaGpuParams {
    /// RCP frame (1/width, 1/height)
    pub rcp_frame: [f32; 2],
    /// Subpixel quality
    pub subpixel_quality: f32,
    /// Edge threshold
    pub edge_threshold: f32,
    /// Edge threshold min
    pub edge_threshold_min: f32,
    /// Padding
    pub _padding: [f32; 3],
}

// ============================================================================
// SMAA
// ============================================================================

/// SMAA settings
#[derive(Clone, Debug)]
pub struct SmaaSettings {
    /// Quality preset
    pub quality: SmaaQuality,
    /// Edge detection mode
    pub edge_detection: SmaaEdgeDetection,
    /// Temporal reprojection
    pub temporal: bool,
    /// Threshold
    pub threshold: f32,
    /// Max search steps
    pub max_search_steps: u32,
    /// Max diagonal search steps
    pub max_search_steps_diag: u32,
    /// Corner rounding
    pub corner_rounding: f32,
}

impl SmaaSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            quality: SmaaQuality::High,
            edge_detection: SmaaEdgeDetection::Luma,
            temporal: true,
            threshold: 0.1,
            max_search_steps: 16,
            max_search_steps_diag: 8,
            corner_rounding: 25.0,
        }
    }

    /// Low quality
    pub fn low() -> Self {
        Self {
            quality: SmaaQuality::Low,
            max_search_steps: 4,
            max_search_steps_diag: 0,
            temporal: false,
            ..Self::new()
        }
    }

    /// Ultra quality
    pub fn ultra() -> Self {
        Self {
            quality: SmaaQuality::Ultra,
            max_search_steps: 32,
            max_search_steps_diag: 16,
            threshold: 0.05,
            ..Self::new()
        }
    }

    /// With edge detection
    pub fn with_edge_detection(mut self, mode: SmaaEdgeDetection) -> Self {
        self.edge_detection = mode;
        self
    }
}

impl Default for SmaaSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SMAA quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SmaaQuality {
    /// Low
    Low = 0,
    /// Medium
    Medium = 1,
    /// High
    #[default]
    High = 2,
    /// Ultra
    Ultra = 3,
}

/// SMAA edge detection mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SmaaEdgeDetection {
    /// Luma
    #[default]
    Luma = 0,
    /// Color
    Color = 1,
    /// Depth
    Depth = 2,
}

/// SMAA GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SmaaGpuParams {
    /// RT metrics (1/width, 1/height, width, height)
    pub rt_metrics: [f32; 4],
    /// Threshold
    pub threshold: f32,
    /// Max search steps
    pub max_search_steps: f32,
    /// Max search steps diagonal
    pub max_search_steps_diag: f32,
    /// Corner rounding
    pub corner_rounding: f32,
}

// ============================================================================
// TAA
// ============================================================================

/// TAA settings
#[derive(Clone, Debug)]
pub struct TaaSettings {
    /// Enabled
    pub enabled: bool,
    /// History weight (blend factor)
    pub history_weight: f32,
    /// Use motion vectors
    pub use_motion_vectors: bool,
    /// Clipping mode
    pub clipping_mode: TaaClippingMode,
    /// Jitter samples
    pub jitter_samples: u32,
    /// Sharpness
    pub sharpness: f32,
    /// Anti-ghosting
    pub anti_ghosting: f32,
    /// Variance clipping gamma
    pub variance_gamma: f32,
}

impl TaaSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            history_weight: 0.9,
            use_motion_vectors: true,
            clipping_mode: TaaClippingMode::VarianceClipping,
            jitter_samples: 8,
            sharpness: 0.25,
            anti_ghosting: 1.0,
            variance_gamma: 1.0,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// Low quality (less ghosting)
    pub fn low_ghosting() -> Self {
        Self {
            history_weight: 0.7,
            anti_ghosting: 1.5,
            ..Self::new()
        }
    }

    /// High quality (more stable)
    pub fn high_quality() -> Self {
        Self {
            history_weight: 0.95,
            jitter_samples: 16,
            variance_gamma: 0.75,
            ..Self::new()
        }
    }

    /// With sharpness
    pub fn with_sharpness(mut self, sharpness: f32) -> Self {
        self.sharpness = sharpness;
        self
    }

    /// With history weight
    pub fn with_history_weight(mut self, weight: f32) -> Self {
        self.history_weight = weight;
        self
    }
}

impl Default for TaaSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// TAA clipping mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TaaClippingMode {
    /// No clipping
    None = 0,
    /// AABB clipping
    Aabb = 1,
    /// Variance clipping
    #[default]
    VarianceClipping = 2,
    /// Clip towards center
    ClipTowardsCenter = 3,
}

/// TAA GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TaaGpuParams {
    /// Jitter offset
    pub jitter: [f32; 2],
    /// History weight
    pub history_weight: f32,
    /// Variance gamma
    pub variance_gamma: f32,
    /// Sharpness
    pub sharpness: f32,
    /// Anti-ghosting strength
    pub anti_ghosting: f32,
    /// Frame index
    pub frame_index: u32,
    /// Padding
    pub _padding: f32,
}

/// Jitter sequence
#[derive(Clone, Debug)]
pub struct JitterSequence {
    /// Samples
    pub samples: Vec<[f32; 2]>,
    /// Current index
    pub current_index: usize,
}

impl JitterSequence {
    /// Creates sequence
    pub fn new(sample_count: u32) -> Self {
        let samples = Self::generate_halton(sample_count);
        Self {
            samples,
            current_index: 0,
        }
    }

    /// Halton 2x3 (default)
    pub fn halton() -> Self {
        Self::new(8)
    }

    /// High quality (16 samples)
    pub fn high_quality() -> Self {
        Self::new(16)
    }

    /// Generate Halton sequence
    fn generate_halton(count: u32) -> Vec<[f32; 2]> {
        let mut samples = Vec::with_capacity(count as usize);
        for i in 0..count {
            let x = Self::halton_number(i + 1, 2);
            let y = Self::halton_number(i + 1, 3);
            samples.push([x - 0.5, y - 0.5]);
        }
        samples
    }

    fn halton_number(mut index: u32, base: u32) -> f32 {
        let mut result = 0.0;
        let mut f = 1.0 / base as f32;
        while index > 0 {
            result += f * (index % base) as f32;
            index /= base;
            f /= base as f32;
        }
        result
    }

    /// Get current jitter
    pub fn current(&self) -> [f32; 2] {
        if self.samples.is_empty() {
            return [0.0, 0.0];
        }
        self.samples[self.current_index % self.samples.len()]
    }

    /// Advance to next
    pub fn advance(&mut self) -> [f32; 2] {
        let jitter = self.current();
        self.current_index = (self.current_index + 1) % self.samples.len();
        jitter
    }

    /// Reset
    pub fn reset(&mut self) {
        self.current_index = 0;
    }
}

impl Default for JitterSequence {
    fn default() -> Self {
        Self::halton()
    }
}

// ============================================================================
// Upscaling
// ============================================================================

/// Upscaling method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum UpscalingMethod {
    /// None
    #[default]
    None = 0,
    /// Bilinear
    Bilinear = 1,
    /// Bicubic
    Bicubic = 2,
    /// Lanczos
    Lanczos = 3,
    /// FSR 1.0
    Fsr1 = 4,
    /// FSR 2.0
    Fsr2 = 5,
    /// DLSS
    Dlss = 6,
    /// XeSS
    Xess = 7,
}

/// Upscaling settings
#[derive(Clone, Debug)]
pub struct UpscalingSettings {
    /// Method
    pub method: UpscalingMethod,
    /// Quality (for AI upscalers)
    pub quality: UpscalingQuality,
    /// Render scale (0.5 = half resolution)
    pub render_scale: f32,
    /// Sharpness
    pub sharpness: f32,
}

impl UpscalingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            method: UpscalingMethod::None,
            quality: UpscalingQuality::Balanced,
            render_scale: 1.0,
            sharpness: 0.5,
        }
    }

    /// With FSR 2
    pub fn fsr2(quality: UpscalingQuality) -> Self {
        Self {
            method: UpscalingMethod::Fsr2,
            quality,
            render_scale: quality.render_scale(),
            ..Self::new()
        }
    }

    /// With DLSS
    pub fn dlss(quality: UpscalingQuality) -> Self {
        Self {
            method: UpscalingMethod::Dlss,
            quality,
            render_scale: quality.render_scale(),
            ..Self::new()
        }
    }

    /// Native (no upscaling)
    pub fn native() -> Self {
        Self::new()
    }
}

impl Default for UpscalingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Upscaling quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum UpscalingQuality {
    /// Ultra performance (0.33x)
    UltraPerformance = 0,
    /// Performance (0.5x)
    Performance = 1,
    /// Balanced (0.67x)
    #[default]
    Balanced = 2,
    /// Quality (0.77x)
    Quality = 3,
    /// Ultra quality (0.85x)
    UltraQuality = 4,
    /// Native AA (1.0x)
    NativeAa = 5,
}

impl UpscalingQuality {
    /// Render scale
    pub const fn render_scale(&self) -> f32 {
        match self {
            Self::UltraPerformance => 0.33,
            Self::Performance => 0.5,
            Self::Balanced => 0.67,
            Self::Quality => 0.77,
            Self::UltraQuality => 0.85,
            Self::NativeAa => 1.0,
        }
    }

    /// Resolution multiplier (inverse of scale)
    pub const fn resolution_multiplier(&self) -> f32 {
        match self {
            Self::UltraPerformance => 3.0,
            Self::Performance => 2.0,
            Self::Balanced => 1.5,
            Self::Quality => 1.3,
            Self::UltraQuality => 1.18,
            Self::NativeAa => 1.0,
        }
    }
}

// ============================================================================
// Sharpening
// ============================================================================

/// Sharpening settings
#[derive(Clone, Debug)]
pub struct SharpeningSettings {
    /// Enable
    pub enabled: bool,
    /// Method
    pub method: SharpeningMethod,
    /// Intensity
    pub intensity: f32,
    /// Limit (prevent artifacts)
    pub limit: f32,
}

impl SharpeningSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            method: SharpeningMethod::Cas,
            intensity: 0.5,
            limit: 0.5,
        }
    }

    /// CAS (Contrast Adaptive Sharpening)
    pub fn cas(intensity: f32) -> Self {
        Self {
            enabled: true,
            method: SharpeningMethod::Cas,
            intensity,
            limit: 0.5,
        }
    }

    /// Unsharp mask
    pub fn unsharp_mask(intensity: f32) -> Self {
        Self {
            enabled: true,
            method: SharpeningMethod::UnsharpMask,
            intensity,
            limit: 0.8,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for SharpeningSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Sharpening method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SharpeningMethod {
    /// CAS (Contrast Adaptive Sharpening)
    #[default]
    Cas = 0,
    /// Unsharp mask
    UnsharpMask = 1,
    /// Laplacian
    Laplacian = 2,
    /// RCAS (Robust Contrast Adaptive Sharpening)
    Rcas = 3,
}

// ============================================================================
// Statistics
// ============================================================================

/// Anti-aliasing statistics
#[derive(Clone, Debug, Default)]
pub struct AntiAliasingStats {
    /// Current method
    pub method: AntiAliasingMethod,
    /// MSAA samples (if applicable)
    pub msaa_samples: u32,
    /// Render resolution
    pub render_resolution: [u32; 2],
    /// Output resolution
    pub output_resolution: [u32; 2],
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}
