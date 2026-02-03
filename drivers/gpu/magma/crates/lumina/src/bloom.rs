//! Bloom Effect Types for Lumina
//!
//! This module provides bloom and glow effect infrastructure
//! for HDR rendering pipelines.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Bloom Handles
// ============================================================================

/// Bloom handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BloomHandle(pub u64);

impl BloomHandle {
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

impl Default for BloomHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bloom mip chain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BloomMipChainHandle(pub u64);

impl BloomMipChainHandle {
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

impl Default for BloomMipChainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Bloom Settings
// ============================================================================

/// Bloom settings
#[derive(Clone, Debug)]
pub struct BloomSettings {
    /// Enable bloom
    pub enabled: bool,
    /// Bloom method
    pub method: BloomMethod,
    /// Quality
    pub quality: BloomQuality,
    /// Intensity
    pub intensity: f32,
    /// Threshold
    pub threshold: f32,
    /// Soft threshold (knee)
    pub soft_threshold: f32,
    /// Radius
    pub radius: f32,
    /// Scatter
    pub scatter: f32,
    /// Anamorphic ratio (-1 to 1)
    pub anamorphic: f32,
    /// Tint
    pub tint: [f32; 3],
    /// Dirt mask settings
    pub dirt_mask: Option<BloomDirtMask>,
}

impl BloomSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            method: BloomMethod::Progressive,
            quality: BloomQuality::Medium,
            intensity: 0.5,
            threshold: 1.0,
            soft_threshold: 0.5,
            radius: 4.0,
            scatter: 0.7,
            anamorphic: 0.0,
            tint: [1.0, 1.0, 1.0],
            dirt_mask: None,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// Subtle bloom
    pub fn subtle() -> Self {
        Self {
            intensity: 0.2,
            threshold: 1.5,
            radius: 3.0,
            ..Self::new()
        }
    }

    /// Strong bloom
    pub fn strong() -> Self {
        Self {
            intensity: 1.0,
            threshold: 0.8,
            radius: 6.0,
            scatter: 0.8,
            ..Self::new()
        }
    }

    /// Cinematic bloom
    pub fn cinematic() -> Self {
        Self {
            intensity: 0.4,
            threshold: 1.0,
            radius: 5.0,
            anamorphic: 0.3,
            ..Self::new()
        }
    }

    /// Dream-like bloom
    pub fn dreamy() -> Self {
        Self {
            intensity: 0.8,
            threshold: 0.5,
            radius: 8.0,
            scatter: 0.9,
            soft_threshold: 0.8,
            ..Self::new()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With tint
    pub fn with_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.tint = [r, g, b];
        self
    }

    /// With anamorphic
    pub fn with_anamorphic(mut self, ratio: f32) -> Self {
        self.anamorphic = ratio.clamp(-1.0, 1.0);
        self
    }

    /// With dirt mask
    pub fn with_dirt_mask(mut self, dirt: BloomDirtMask) -> Self {
        self.dirt_mask = Some(dirt);
        self
    }
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Bloom method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BloomMethod {
    /// Simple Gaussian
    Gaussian = 0,
    /// Progressive downscale/upscale
    #[default]
    Progressive = 1,
    /// Kawase blur
    Kawase = 2,
    /// Dual filter
    DualFilter = 3,
    /// FFT convolution
    FftConvolution = 4,
}

impl BloomMethod {
    /// Requires mip chain
    pub const fn requires_mip_chain(&self) -> bool {
        matches!(self, Self::Progressive | Self::DualFilter)
    }
}

/// Bloom quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BloomQuality {
    /// Low (fewer mips, faster)
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
    /// Ultra
    Ultra = 3,
}

impl BloomQuality {
    /// Mip count
    pub const fn mip_count(&self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 6,
            Self::High => 8,
            Self::Ultra => 10,
        }
    }

    /// Blur iterations per mip
    pub const fn blur_iterations(&self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Ultra => 4,
        }
    }
}

// ============================================================================
// Bloom Dirt Mask
// ============================================================================

/// Bloom dirt mask settings
#[derive(Clone, Debug)]
pub struct BloomDirtMask {
    /// Texture handle
    pub texture: u64,
    /// Intensity
    pub intensity: f32,
    /// Tint
    pub tint: [f32; 3],
}

impl BloomDirtMask {
    /// Creates dirt mask
    pub fn new(texture: u64) -> Self {
        Self {
            texture,
            intensity: 1.0,
            tint: [1.0, 1.0, 1.0],
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With tint
    pub fn with_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.tint = [r, g, b];
        self
    }
}

// ============================================================================
// Bloom Mip Chain
// ============================================================================

/// Bloom mip chain create info
#[derive(Clone, Debug)]
pub struct BloomMipChainCreateInfo {
    /// Base width
    pub width: u32,
    /// Base height
    pub height: u32,
    /// Mip count
    pub mip_count: u32,
    /// Format
    pub format: BloomFormat,
}

impl BloomMipChainCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32, mip_count: u32) -> Self {
        Self {
            width,
            height,
            mip_count,
            format: BloomFormat::Rgba16F,
        }
    }

    /// From quality preset
    pub fn from_quality(width: u32, height: u32, quality: BloomQuality) -> Self {
        Self::new(width, height, quality.mip_count())
    }

    /// Total texels (all mips)
    pub fn total_texels(&self) -> u64 {
        let mut total = 0u64;
        let mut w = self.width;
        let mut h = self.height;
        for _ in 0..self.mip_count {
            total += w as u64 * h as u64;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        total
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        self.total_texels() * self.format.bytes_per_pixel() as u64
    }
}

impl Default for BloomMipChainCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080, 6)
    }
}

/// Bloom mip
#[derive(Clone, Copy, Debug)]
pub struct BloomMip {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Mip level
    pub level: u32,
}

impl BloomMip {
    /// Creates mip
    pub fn new(width: u32, height: u32, level: u32) -> Self {
        Self {
            width,
            height,
            level,
        }
    }

    /// Texels
    pub fn texels(&self) -> u32 {
        self.width * self.height
    }
}

/// Bloom format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BloomFormat {
    /// RGBA16F (default)
    #[default]
    Rgba16F = 0,
    /// R11G11B10F (smaller)
    R11G11B10F = 1,
    /// RGBA32F (high precision)
    Rgba32F = 2,
}

impl BloomFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R11G11B10F => 4,
            Self::Rgba16F => 8,
            Self::Rgba32F => 16,
        }
    }
}

// ============================================================================
// Bloom Threshold
// ============================================================================

/// Bloom threshold settings
#[derive(Clone, Copy, Debug)]
pub struct BloomThreshold {
    /// Threshold value
    pub threshold: f32,
    /// Soft knee (0 = hard, 1 = soft)
    pub knee: f32,
}

impl BloomThreshold {
    /// Creates threshold
    pub fn new(threshold: f32, knee: f32) -> Self {
        Self { threshold, knee }
    }

    /// Hard threshold
    pub fn hard(threshold: f32) -> Self {
        Self::new(threshold, 0.0)
    }

    /// Soft threshold
    pub fn soft(threshold: f32) -> Self {
        Self::new(threshold, 0.5)
    }

    /// Very soft
    pub fn very_soft(threshold: f32) -> Self {
        Self::new(threshold, 1.0)
    }

    /// Apply threshold to luminance
    pub fn apply(&self, luminance: f32) -> f32 {
        let soft = self.threshold - self.knee;
        let hard = self.threshold + self.knee;

        if luminance < soft {
            0.0
        } else if luminance > hard {
            luminance - self.threshold
        } else {
            // Smooth transition
            let t = (luminance - soft) / (self.knee * 2.0);
            t * t * (luminance - self.threshold)
        }
    }

    /// GPU curve parameters
    pub fn curve_params(&self) -> [f32; 4] {
        let x = self.threshold;
        let y = self.knee;
        let z = x - y;
        let w = 2.0 * y;
        [x, y, z, w]
    }
}

impl Default for BloomThreshold {
    fn default() -> Self {
        Self::new(1.0, 0.5)
    }
}

// ============================================================================
// Blur Kernels
// ============================================================================

/// Gaussian blur kernel
#[derive(Clone, Debug)]
pub struct GaussianKernel {
    /// Weights
    pub weights: Vec<f32>,
    /// Offsets (for linear sampling)
    pub offsets: Vec<f32>,
    /// Sigma
    pub sigma: f32,
}

impl GaussianKernel {
    /// Creates kernel
    pub fn new(radius: u32, sigma: f32) -> Self {
        let size = (radius * 2 + 1) as usize;
        let mut weights = Vec::with_capacity(size);

        // Generate weights
        let sigma2 = 2.0 * sigma * sigma;
        let mut sum = 0.0;

        for i in 0..size {
            let x = i as f32 - radius as f32;
            let w = (-x * x / sigma2).exp();
            weights.push(w);
            sum += w;
        }

        // Normalize
        for w in &mut weights {
            *w /= sum;
        }

        // Generate offsets
        let mut offsets = Vec::with_capacity(size);
        for i in 0..size {
            offsets.push(i as f32 - radius as f32);
        }

        Self {
            weights,
            offsets,
            sigma,
        }
    }

    /// 3-tap kernel
    pub fn kernel_3() -> Self {
        Self::new(1, 0.84)
    }

    /// 5-tap kernel
    pub fn kernel_5() -> Self {
        Self::new(2, 1.4)
    }

    /// 9-tap kernel
    pub fn kernel_9() -> Self {
        Self::new(4, 2.0)
    }

    /// 13-tap kernel
    pub fn kernel_13() -> Self {
        Self::new(6, 3.0)
    }

    /// Optimized for linear sampling (half the taps)
    pub fn optimized(&self) -> OptimizedGaussianKernel {
        let mut weights = Vec::new();
        let mut offsets = Vec::new();

        // Center tap
        weights.push(self.weights[self.weights.len() / 2]);
        offsets.push(0.0);

        // Combine pairs
        let half = self.weights.len() / 2;
        for i in 1..=half {
            if half + i < self.weights.len() {
                let w1 = self.weights[half + i];
                let w2 = if half >= i { self.weights[half - i] } else { 0.0 };
                let combined = w1 + w2;
                if combined > 0.0 {
                    let offset = (w1 * i as f32 + w2 * -(i as f32)) / combined;
                    weights.push(combined);
                    offsets.push(offset);
                }
            }
        }

        OptimizedGaussianKernel { weights, offsets }
    }
}

impl Default for GaussianKernel {
    fn default() -> Self {
        Self::kernel_9()
    }
}

/// Optimized Gaussian kernel (linear sampling)
#[derive(Clone, Debug)]
pub struct OptimizedGaussianKernel {
    /// Weights
    pub weights: Vec<f32>,
    /// Offsets
    pub offsets: Vec<f32>,
}

/// Kawase blur settings
#[derive(Clone, Debug)]
pub struct KawaseBlurSettings {
    /// Iteration count
    pub iterations: u32,
    /// Offsets per iteration
    pub offsets: Vec<f32>,
}

impl KawaseBlurSettings {
    /// Creates settings
    pub fn new(iterations: u32) -> Self {
        // Typical Kawase offsets
        let offsets = (0..iterations)
            .map(|i| 0.5 + i as f32)
            .collect();
        Self { iterations, offsets }
    }

    /// Light blur (3 iterations)
    pub fn light() -> Self {
        Self::new(3)
    }

    /// Medium blur (5 iterations)
    pub fn medium() -> Self {
        Self::new(5)
    }

    /// Heavy blur (7 iterations)
    pub fn heavy() -> Self {
        Self::new(7)
    }
}

impl Default for KawaseBlurSettings {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// Bloom GPU Params
// ============================================================================

/// Bloom GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BloomGpuParams {
    /// Threshold params (threshold, knee, soft, hard)
    pub threshold_params: [f32; 4],
    /// Intensity, scatter, radius, padding
    pub bloom_params: [f32; 4],
    /// Tint RGB, anamorphic
    pub tint_params: [f32; 4],
    /// Screen size (width, height, 1/width, 1/height)
    pub screen_params: [f32; 4],
}

/// Bloom composite params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BloomCompositeParams {
    /// Intensity
    pub intensity: f32,
    /// Dirt intensity
    pub dirt_intensity: f32,
    /// Padding
    pub _padding: [f32; 2],
    /// Tint
    pub tint: [f32; 4],
    /// Dirt tint
    pub dirt_tint: [f32; 4],
}

// ============================================================================
// Lens Flare
// ============================================================================

/// Lens flare settings
#[derive(Clone, Debug)]
pub struct LensFlareSettings {
    /// Enable
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Threshold
    pub threshold: f32,
    /// Ghost count
    pub ghost_count: u32,
    /// Ghost dispersal
    pub ghost_dispersal: f32,
    /// Halo width
    pub halo_width: f32,
    /// Chromatic aberration
    pub chromatic_aberration: f32,
    /// Elements
    pub elements: Vec<LensFlareElement>,
}

impl LensFlareSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            threshold: 1.5,
            ghost_count: 5,
            ghost_dispersal: 0.5,
            halo_width: 0.5,
            chromatic_aberration: 0.5,
            elements: Vec::new(),
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            elements: Vec::from([
                LensFlareElement::ghost(0.5, 0.3),
                LensFlareElement::ghost(-0.3, 0.2),
                LensFlareElement::ghost(0.7, 0.15),
                LensFlareElement::halo(0.4, 0.5),
            ]),
            ..Self::new()
        }
    }

    /// Cinematic
    pub fn cinematic() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            ghost_count: 8,
            chromatic_aberration: 0.7,
            ..Self::enabled()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for LensFlareSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Lens flare element
#[derive(Clone, Copy, Debug)]
pub struct LensFlareElement {
    /// Element type
    pub element_type: LensFlareElementType,
    /// Position (along flare axis)
    pub position: f32,
    /// Size
    pub size: f32,
    /// Color
    pub color: [f32; 4],
}

impl LensFlareElement {
    /// Ghost element
    pub fn ghost(position: f32, size: f32) -> Self {
        Self {
            element_type: LensFlareElementType::Ghost,
            position,
            size,
            color: [1.0, 1.0, 1.0, 0.5],
        }
    }

    /// Halo element
    pub fn halo(position: f32, size: f32) -> Self {
        Self {
            element_type: LensFlareElementType::Halo,
            position,
            size,
            color: [1.0, 0.9, 0.8, 0.3],
        }
    }

    /// Streak element
    pub fn streak(position: f32, size: f32) -> Self {
        Self {
            element_type: LensFlareElementType::Streak,
            position,
            size,
            color: [1.0, 1.0, 1.0, 0.4],
        }
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }
}

/// Lens flare element type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LensFlareElementType {
    /// Ghost (circular)
    #[default]
    Ghost = 0,
    /// Halo (ring)
    Halo = 1,
    /// Streak (line)
    Streak = 2,
    /// Star burst
    StarBurst = 3,
}

// ============================================================================
// Statistics
// ============================================================================

/// Bloom statistics
#[derive(Clone, Debug, Default)]
pub struct BloomStats {
    /// Enabled
    pub enabled: bool,
    /// Method
    pub method: BloomMethod,
    /// Mip count
    pub mip_count: u32,
    /// Total texels
    pub total_texels: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}
