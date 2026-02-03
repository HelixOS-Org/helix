//! Temporal Upscaling Types for Lumina
//!
//! This module provides temporal upscaling infrastructure
//! for DLSS, FSR, XeSS-like implementations.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Upscaling Handles
// ============================================================================

/// Temporal upscaler handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TemporalUpscalerHandle(pub u64);

impl TemporalUpscalerHandle {
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

impl Default for TemporalUpscalerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Upscaling context handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct UpscalingContextHandle(pub u64);

impl UpscalingContextHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for UpscalingContextHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// History buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HistoryBufferHandle(pub u64);

impl HistoryBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HistoryBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Upscaler Creation
// ============================================================================

/// Temporal upscaler create info
#[derive(Clone, Debug)]
pub struct TemporalUpscalerCreateInfo {
    /// Name
    pub name: String,
    /// Algorithm
    pub algorithm: UpscalingAlgorithm,
    /// Quality preset
    pub quality: UpscalingQuality,
    /// Render resolution (input)
    pub render_width: u32,
    /// Render resolution (input)
    pub render_height: u32,
    /// Display resolution (output)
    pub display_width: u32,
    /// Display resolution (output)
    pub display_height: u32,
    /// Features
    pub features: UpscalingFeatures,
    /// HDR mode
    pub hdr_mode: HdrMode,
}

impl TemporalUpscalerCreateInfo {
    /// Creates new info
    pub fn new(algorithm: UpscalingAlgorithm, display_width: u32, display_height: u32) -> Self {
        let quality = UpscalingQuality::Balanced;
        let scale = quality.scale_factor();
        let render_width = (display_width as f32 / scale) as u32;
        let render_height = (display_height as f32 / scale) as u32;

        Self {
            name: String::new(),
            algorithm,
            quality,
            render_width,
            render_height,
            display_width,
            display_height,
            features: UpscalingFeatures::empty(),
            hdr_mode: HdrMode::Sdr,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: UpscalingQuality) -> Self {
        self.quality = quality;
        let scale = quality.scale_factor();
        self.render_width = (self.display_width as f32 / scale) as u32;
        self.render_height = (self.display_height as f32 / scale) as u32;
        self
    }

    /// With render resolution
    pub fn with_render_resolution(mut self, width: u32, height: u32) -> Self {
        self.render_width = width;
        self.render_height = height;
        self
    }

    /// With features
    pub fn with_features(mut self, features: UpscalingFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With HDR
    pub fn with_hdr(mut self, mode: HdrMode) -> Self {
        self.hdr_mode = mode;
        self
    }

    /// FSR 2 preset
    pub fn fsr2(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::Fsr2, display_width, display_height)
    }

    /// FSR 3 preset
    pub fn fsr3(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::Fsr3, display_width, display_height)
            .with_features(UpscalingFeatures::FRAME_GENERATION)
    }

    /// DLSS preset
    pub fn dlss(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::Dlss, display_width, display_height)
    }

    /// XeSS preset
    pub fn xess(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::XeSS, display_width, display_height)
    }

    /// Native TAA preset
    pub fn taa_native(width: u32, height: u32) -> Self {
        Self::new(UpscalingAlgorithm::NativeTaa, width, height)
            .with_quality(UpscalingQuality::Native)
    }

    /// Performance mode preset
    pub fn performance(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::Fsr2, display_width, display_height)
            .with_quality(UpscalingQuality::Performance)
    }

    /// Ultra quality preset
    pub fn ultra_quality(display_width: u32, display_height: u32) -> Self {
        Self::new(UpscalingAlgorithm::Dlss, display_width, display_height)
            .with_quality(UpscalingQuality::UltraQuality)
    }

    /// Scale factor
    pub fn scale_factor(&self) -> f32 {
        self.display_width as f32 / self.render_width as f32
    }

    /// Render pixel count
    pub fn render_pixels(&self) -> u64 {
        self.render_width as u64 * self.render_height as u64
    }

    /// Display pixel count
    pub fn display_pixels(&self) -> u64 {
        self.display_width as u64 * self.display_height as u64
    }

    /// Performance gain estimate
    pub fn performance_gain(&self) -> f32 {
        let scale = self.scale_factor();
        scale * scale
    }
}

impl Default for TemporalUpscalerCreateInfo {
    fn default() -> Self {
        Self::fsr2(1920, 1080)
    }
}

/// Upscaling algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum UpscalingAlgorithm {
    /// FSR 2.x
    #[default]
    Fsr2      = 0,
    /// FSR 3.x (with frame generation)
    Fsr3      = 1,
    /// NVIDIA DLSS
    Dlss      = 2,
    /// Intel XeSS
    XeSS      = 3,
    /// Native TAA
    NativeTaa = 4,
    /// Custom implementation
    Custom    = 100,
}

impl UpscalingAlgorithm {
    /// Requires specific hardware
    pub const fn requires_specific_hardware(&self) -> bool {
        matches!(self, Self::Dlss | Self::XeSS)
    }

    /// Name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Fsr2 => "FSR 2",
            Self::Fsr3 => "FSR 3",
            Self::Dlss => "DLSS",
            Self::XeSS => "XeSS",
            Self::NativeTaa => "Native TAA",
            Self::Custom => "Custom",
        }
    }
}

/// Upscaling quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum UpscalingQuality {
    /// Native resolution (no upscaling)
    Native           = 0,
    /// Ultra quality (1.3x)
    UltraQuality     = 1,
    /// Quality (1.5x)
    Quality          = 2,
    /// Balanced (1.7x)
    #[default]
    Balanced         = 3,
    /// Performance (2.0x)
    Performance      = 4,
    /// Ultra performance (3.0x)
    UltraPerformance = 5,
}

impl UpscalingQuality {
    /// Scale factor (display / render)
    pub const fn scale_factor(&self) -> f32 {
        match self {
            Self::Native => 1.0,
            Self::UltraQuality => 1.3,
            Self::Quality => 1.5,
            Self::Balanced => 1.7,
            Self::Performance => 2.0,
            Self::UltraPerformance => 3.0,
        }
    }

    /// Name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Native => "Native",
            Self::UltraQuality => "Ultra Quality",
            Self::Quality => "Quality",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
            Self::UltraPerformance => "Ultra Performance",
        }
    }
}

bitflags::bitflags! {
    /// Upscaling features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct UpscalingFeatures: u32 {
        /// None
        const NONE = 0;
        /// Sharpening
        const SHARPENING = 1 << 0;
        /// Frame generation
        const FRAME_GENERATION = 1 << 1;
        /// Anti-lag
        const ANTI_LAG = 1 << 2;
        /// Dynamic resolution
        const DYNAMIC_RESOLUTION = 1 << 3;
        /// Auto exposure
        const AUTO_EXPOSURE = 1 << 4;
        /// Reactive mask
        const REACTIVE_MASK = 1 << 5;
        /// Transparency mask
        const TRANSPARENCY_MASK = 1 << 6;
    }
}

/// HDR mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrMode {
    /// SDR
    #[default]
    Sdr   = 0,
    /// HDR10
    Hdr10 = 1,
    /// scRGB
    ScRgb = 2,
    /// PQ
    Pq    = 3,
}

// ============================================================================
// Upscaling Input
// ============================================================================

/// Upscaling input data
#[derive(Clone, Debug, Default)]
pub struct UpscalingInput {
    /// Color input texture
    pub color: u64,
    /// Depth buffer
    pub depth: u64,
    /// Motion vectors
    pub motion_vectors: u64,
    /// Exposure texture (optional)
    pub exposure: Option<u64>,
    /// Reactive mask (optional)
    pub reactive_mask: Option<u64>,
    /// Transparency mask (optional)
    pub transparency_mask: Option<u64>,
    /// Output texture
    pub output: u64,
    /// Parameters
    pub params: UpscalingParams,
}

impl UpscalingInput {
    /// Creates new input
    pub fn new(color: u64, depth: u64, motion_vectors: u64, output: u64) -> Self {
        Self {
            color,
            depth,
            motion_vectors,
            exposure: None,
            reactive_mask: None,
            transparency_mask: None,
            output,
            params: UpscalingParams::default(),
        }
    }

    /// With exposure
    pub fn with_exposure(mut self, exposure: u64) -> Self {
        self.exposure = Some(exposure);
        self
    }

    /// With reactive mask
    pub fn with_reactive_mask(mut self, mask: u64) -> Self {
        self.reactive_mask = Some(mask);
        self
    }

    /// With transparency mask
    pub fn with_transparency_mask(mut self, mask: u64) -> Self {
        self.transparency_mask = Some(mask);
        self
    }

    /// With params
    pub fn with_params(mut self, params: UpscalingParams) -> Self {
        self.params = params;
        self
    }
}

/// Upscaling parameters
#[derive(Clone, Debug)]
pub struct UpscalingParams {
    /// Render width
    pub render_width: u32,
    /// Render height
    pub render_height: u32,
    /// Jitter offset X
    pub jitter_x: f32,
    /// Jitter offset Y
    pub jitter_y: f32,
    /// Delta time (seconds)
    pub delta_time: f32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Vertical FOV (radians)
    pub fov_y: f32,
    /// Sharpness (0.0 - 1.0)
    pub sharpness: f32,
    /// Camera reset (scene cut)
    pub camera_reset: bool,
    /// Motion vector scale
    pub motion_vector_scale: [f32; 2],
    /// Pre-exposure
    pub pre_exposure: f32,
}

impl UpscalingParams {
    /// Creates new params
    pub fn new(render_width: u32, render_height: u32) -> Self {
        Self {
            render_width,
            render_height,
            jitter_x: 0.0,
            jitter_y: 0.0,
            delta_time: 0.016667, // 60 FPS
            near_plane: 0.1,
            far_plane: 1000.0,
            fov_y: 1.047, // 60 degrees
            sharpness: 0.5,
            camera_reset: false,
            motion_vector_scale: [1.0, 1.0],
            pre_exposure: 1.0,
        }
    }

    /// With jitter
    pub fn with_jitter(mut self, x: f32, y: f32) -> Self {
        self.jitter_x = x;
        self.jitter_y = y;
        self
    }

    /// With delta time
    pub fn with_delta_time(mut self, dt: f32) -> Self {
        self.delta_time = dt;
        self
    }

    /// With camera
    pub fn with_camera(mut self, near: f32, far: f32, fov_y: f32) -> Self {
        self.near_plane = near;
        self.far_plane = far;
        self.fov_y = fov_y;
        self
    }

    /// With sharpness
    pub fn with_sharpness(mut self, sharpness: f32) -> Self {
        self.sharpness = sharpness.clamp(0.0, 1.0);
        self
    }

    /// Mark camera reset
    pub fn camera_reset(mut self) -> Self {
        self.camera_reset = true;
        self
    }

    /// With motion vector scale
    pub fn with_motion_scale(mut self, x: f32, y: f32) -> Self {
        self.motion_vector_scale = [x, y];
        self
    }
}

impl Default for UpscalingParams {
    fn default() -> Self {
        Self::new(1280, 720)
    }
}

// ============================================================================
// Jitter Sequence
// ============================================================================

/// Jitter pattern
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum JitterPattern {
    /// Halton 2,3
    #[default]
    Halton23        = 0,
    /// Halton 2,3 (16 samples)
    Halton23_16     = 1,
    /// R2 sequence
    R2              = 2,
    /// Blue noise
    BlueNoise       = 3,
    /// Martin Roberts R2
    MartinRobertsR2 = 4,
}

/// Jitter sequence generator
#[derive(Clone, Debug)]
pub struct JitterSequence {
    /// Pattern
    pub pattern: JitterPattern,
    /// Current index
    pub index: u32,
    /// Sequence length
    pub length: u32,
    /// Render width (for pixel offset calculation)
    pub render_width: u32,
    /// Render height
    pub render_height: u32,
}

impl JitterSequence {
    /// Creates new sequence
    pub fn new(pattern: JitterPattern, render_width: u32, render_height: u32) -> Self {
        Self {
            pattern,
            index: 0,
            length: 32,
            render_width,
            render_height,
        }
    }

    /// Advance to next sample
    pub fn advance(&mut self) {
        self.index = (self.index + 1) % self.length;
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Get current jitter offset (in pixels)
    pub fn jitter_pixels(&self) -> (f32, f32) {
        let (x, y) = self.sample(self.index);
        (x - 0.5, y - 0.5)
    }

    /// Get current jitter offset (in NDC, -1 to 1)
    pub fn jitter_ndc(&self) -> (f32, f32) {
        let (px, py) = self.jitter_pixels();
        (
            px * 2.0 / self.render_width as f32,
            py * 2.0 / self.render_height as f32,
        )
    }

    /// Get sample at index (0 to 1 range)
    fn sample(&self, index: u32) -> (f32, f32) {
        match self.pattern {
            JitterPattern::Halton23 | JitterPattern::Halton23_16 => {
                (Self::halton(index, 2), Self::halton(index, 3))
            },
            JitterPattern::R2 | JitterPattern::MartinRobertsR2 => Self::r2_sequence(index),
            JitterPattern::BlueNoise => {
                // Simple approximation
                let x = Self::halton(index, 2);
                let y = Self::halton(index, 3);
                (x, y)
            },
        }
    }

    /// Halton sequence
    fn halton(mut index: u32, base: u32) -> f32 {
        let mut result = 0.0f32;
        let mut f = 1.0 / base as f32;

        while index > 0 {
            result += f * (index % base) as f32;
            index /= base;
            f /= base as f32;
        }

        result
    }

    /// R2 sequence (Martin Roberts)
    fn r2_sequence(index: u32) -> (f32, f32) {
        const G: f32 = 1.32471795724; // Plastic constant
        const A1: f32 = 1.0 / G;
        const A2: f32 = 1.0 / (G * G);

        let n = index as f32;
        let x = (0.5 + A1 * n) % 1.0;
        let y = (0.5 + A2 * n) % 1.0;

        (x, y)
    }
}

impl Default for JitterSequence {
    fn default() -> Self {
        Self::new(JitterPattern::Halton23, 1280, 720)
    }
}

// ============================================================================
// GPU Data Structures
// ============================================================================

/// GPU upscaling constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuUpscalingConstants {
    /// Render size
    pub render_size: [f32; 2],
    /// Display size
    pub display_size: [f32; 2],
    /// Jitter
    pub jitter: [f32; 2],
    /// Delta time
    pub delta_time: f32,
    /// Sharpness
    pub sharpness: f32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// FOV Y
    pub fov_y: f32,
    /// Camera reset
    pub camera_reset: u32,
    /// Motion vector scale
    pub motion_scale: [f32; 2],
    /// Pre-exposure
    pub pre_exposure: f32,
    /// Frame index
    pub frame_index: u32,
}

impl GpuUpscalingConstants {
    /// From params
    pub fn from_params(
        params: &UpscalingParams,
        display_width: u32,
        display_height: u32,
        frame_index: u32,
    ) -> Self {
        Self {
            render_size: [params.render_width as f32, params.render_height as f32],
            display_size: [display_width as f32, display_height as f32],
            jitter: [params.jitter_x, params.jitter_y],
            delta_time: params.delta_time,
            sharpness: params.sharpness,
            near_plane: params.near_plane,
            far_plane: params.far_plane,
            fov_y: params.fov_y,
            camera_reset: params.camera_reset as u32,
            motion_scale: params.motion_vector_scale,
            pre_exposure: params.pre_exposure,
            frame_index,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Temporal upscaling statistics
#[derive(Clone, Debug, Default)]
pub struct TemporalUpscalingStats {
    /// Algorithm
    pub algorithm: UpscalingAlgorithm,
    /// Quality preset
    pub quality: UpscalingQuality,
    /// Render width
    pub render_width: u32,
    /// Render height
    pub render_height: u32,
    /// Display width
    pub display_width: u32,
    /// Display height
    pub display_height: u32,
    /// Scale factor
    pub scale_factor: f32,
    /// Upscaling time (ms)
    pub upscale_time_ms: f32,
    /// VRAM usage (bytes)
    pub vram_usage: u64,
    /// Frame index
    pub frame_index: u64,
}

impl TemporalUpscalingStats {
    /// Performance gain (theoretical)
    pub fn performance_gain(&self) -> f32 {
        self.scale_factor * self.scale_factor
    }

    /// Pixels upscaled
    pub fn pixels_upscaled(&self) -> u64 {
        self.display_width as u64 * self.display_height as u64
    }
}
