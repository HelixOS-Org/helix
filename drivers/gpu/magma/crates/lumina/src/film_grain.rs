//! Film Grain Types for Lumina
//!
//! This module provides film grain simulation infrastructure for
//! cinematic effects and procedural noise.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Film Grain Handles
// ============================================================================

/// Film grain effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FilmGrainHandle(pub u64);

impl FilmGrainHandle {
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

impl Default for FilmGrainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Grain texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GrainTextureHandle(pub u64);

impl GrainTextureHandle {
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

impl Default for GrainTextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Film Grain Configuration
// ============================================================================

/// Film grain create info
#[derive(Clone, Debug)]
pub struct FilmGrainCreateInfo {
    /// Name
    pub name: String,
    /// Grain type
    pub grain_type: GrainType,
    /// Intensity
    pub intensity: f32,
    /// Size
    pub size: f32,
    /// Response curve
    pub response: GrainResponse,
    /// Color grain
    pub colored: bool,
}

impl FilmGrainCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            grain_type: GrainType::Procedural,
            intensity: 0.1,
            size: 1.0,
            response: GrainResponse::default(),
            colored: false,
        }
    }

    /// Subtle grain
    pub fn subtle() -> Self {
        Self {
            intensity: 0.05,
            size: 0.8,
            ..Self::new()
        }
    }

    /// Heavy grain (vintage look)
    pub fn heavy() -> Self {
        Self {
            intensity: 0.25,
            size: 1.5,
            ..Self::new()
        }
    }

    /// Film emulation
    pub fn film_emulation() -> Self {
        Self {
            grain_type: GrainType::FilmEmulation,
            intensity: 0.15,
            size: 1.2,
            response: GrainResponse::film(),
            colored: true,
            ..Self::new()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With size
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Colored grain
    pub fn with_color(mut self) -> Self {
        self.colored = true;
        self
    }
}

impl Default for FilmGrainCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Grain type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GrainType {
    /// Procedural noise
    #[default]
    Procedural    = 0,
    /// Texture-based grain
    TextureBased  = 1,
    /// Film emulation (physically-based)
    FilmEmulation = 2,
    /// Digital noise simulation
    DigitalNoise  = 3,
}

impl GrainType {
    /// Requires texture
    pub const fn requires_texture(&self) -> bool {
        matches!(self, Self::TextureBased)
    }

    /// Is physically-based
    pub const fn is_physical(&self) -> bool {
        matches!(self, Self::FilmEmulation)
    }
}

// ============================================================================
// Grain Response
// ============================================================================

/// Grain response curve (how grain responds to luminance)
#[derive(Clone, Copy, Debug)]
pub struct GrainResponse {
    /// Shadow grain strength
    pub shadows: f32,
    /// Midtone grain strength
    pub midtones: f32,
    /// Highlight grain strength
    pub highlights: f32,
    /// Shadow luminance threshold
    pub shadow_threshold: f32,
    /// Highlight luminance threshold
    pub highlight_threshold: f32,
}

impl GrainResponse {
    /// Default response
    pub const fn default_response() -> Self {
        Self {
            shadows: 1.0,
            midtones: 1.0,
            highlights: 0.5,
            shadow_threshold: 0.2,
            highlight_threshold: 0.8,
        }
    }

    /// Film-like response (more grain in shadows)
    pub const fn film() -> Self {
        Self {
            shadows: 1.2,
            midtones: 0.8,
            highlights: 0.3,
            shadow_threshold: 0.25,
            highlight_threshold: 0.75,
        }
    }

    /// Uniform response
    pub const fn uniform() -> Self {
        Self {
            shadows: 1.0,
            midtones: 1.0,
            highlights: 1.0,
            shadow_threshold: 0.0,
            highlight_threshold: 1.0,
        }
    }

    /// Evaluate at luminance
    pub fn evaluate(&self, luminance: f32) -> f32 {
        let l = luminance.clamp(0.0, 1.0);

        if l < self.shadow_threshold {
            self.shadows
        } else if l > self.highlight_threshold {
            self.highlights
        } else {
            // Interpolate through midtones
            let t =
                (l - self.shadow_threshold) / (self.highlight_threshold - self.shadow_threshold);
            if t < 0.5 {
                let t2 = t * 2.0;
                self.shadows * (1.0 - t2) + self.midtones * t2
            } else {
                let t2 = (t - 0.5) * 2.0;
                self.midtones * (1.0 - t2) + self.highlights * t2
            }
        }
    }
}

impl Default for GrainResponse {
    fn default() -> Self {
        Self::default_response()
    }
}

// ============================================================================
// Grain Settings
// ============================================================================

/// Film grain settings
#[derive(Clone, Copy, Debug)]
pub struct FilmGrainSettings {
    /// Intensity (0-1)
    pub intensity: f32,
    /// Size (1.0 = default)
    pub size: f32,
    /// Luminance (mix of color vs luminance grain)
    pub luminance_ratio: f32,
    /// Color grain strength
    pub color_strength: f32,
    /// Animation speed
    pub animation_speed: f32,
    /// Response curve
    pub response: GrainResponse,
}

impl FilmGrainSettings {
    /// Default settings
    pub const fn default_settings() -> Self {
        Self {
            intensity: 0.1,
            size: 1.0,
            luminance_ratio: 1.0,
            color_strength: 0.0,
            animation_speed: 1.0,
            response: GrainResponse::default_response(),
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            intensity: 0.0,
            ..Self::default_settings()
        }
    }

    /// Cinematic
    pub const fn cinematic() -> Self {
        Self {
            intensity: 0.12,
            size: 1.2,
            luminance_ratio: 0.8,
            color_strength: 0.15,
            animation_speed: 0.8,
            response: GrainResponse::film(),
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With color
    pub const fn with_color(mut self, strength: f32) -> Self {
        self.luminance_ratio = 1.0 - strength;
        self.color_strength = strength;
        self
    }
}

impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

// ============================================================================
// Film Stock Emulation
// ============================================================================

/// Film stock preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FilmStock {
    /// Kodak Portra 400
    #[default]
    KodakPortra400 = 0,
    /// Kodak Ektar 100
    KodakEktar100  = 1,
    /// Fuji Velvia 50
    FujiVelvia50   = 2,
    /// Fuji Pro 400H
    FujiPro400H    = 3,
    /// Ilford HP5 (B&W)
    IlfordHp5      = 4,
    /// Kodak Tri-X (B&W)
    KodakTriX      = 5,
    /// CineStill 800T
    CineStill800T  = 6,
    /// Custom
    Custom         = 7,
}

impl FilmStock {
    /// Grain characteristics for film stock
    pub fn grain_settings(&self) -> FilmGrainSettings {
        match self {
            Self::KodakPortra400 => FilmGrainSettings {
                intensity: 0.08,
                size: 1.0,
                luminance_ratio: 0.85,
                color_strength: 0.1,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::KodakEktar100 => FilmGrainSettings {
                intensity: 0.04,
                size: 0.7,
                luminance_ratio: 0.9,
                color_strength: 0.08,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::FujiVelvia50 => FilmGrainSettings {
                intensity: 0.03,
                size: 0.6,
                luminance_ratio: 0.95,
                color_strength: 0.05,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::FujiPro400H => FilmGrainSettings {
                intensity: 0.07,
                size: 0.9,
                luminance_ratio: 0.88,
                color_strength: 0.08,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::IlfordHp5 => FilmGrainSettings {
                intensity: 0.12,
                size: 1.3,
                luminance_ratio: 1.0,
                color_strength: 0.0,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::KodakTriX => FilmGrainSettings {
                intensity: 0.15,
                size: 1.4,
                luminance_ratio: 1.0,
                color_strength: 0.0,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::CineStill800T => FilmGrainSettings {
                intensity: 0.18,
                size: 1.5,
                luminance_ratio: 0.75,
                color_strength: 0.2,
                animation_speed: 1.0,
                response: GrainResponse::film(),
            },
            Self::Custom => FilmGrainSettings::default_settings(),
        }
    }

    /// Is black and white film
    pub const fn is_bw(&self) -> bool {
        matches!(self, Self::IlfordHp5 | Self::KodakTriX)
    }
}

// ============================================================================
// Grain Texture Generation
// ============================================================================

/// Grain texture create info
#[derive(Clone, Debug)]
pub struct GrainTextureCreateInfo {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: GrainTextureFormat,
    /// Tile count (for animation)
    pub tile_count: u32,
    /// Seed
    pub seed: u32,
}

impl GrainTextureCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: GrainTextureFormat::R8,
            tile_count: 16,
            seed: 0,
        }
    }

    /// Standard grain texture
    pub fn standard() -> Self {
        Self::new(512, 512)
    }

    /// High quality grain texture
    pub fn high_quality() -> Self {
        Self {
            width: 1024,
            height: 1024,
            format: GrainTextureFormat::R16,
            tile_count: 32,
            seed: 0,
        }
    }

    /// Color grain texture
    pub fn color() -> Self {
        Self {
            format: GrainTextureFormat::Rgb8,
            ..Self::standard()
        }
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        (self.width as u64)
            * (self.height as u64)
            * (self.tile_count as u64)
            * (self.format.bytes_per_pixel() as u64)
    }
}

impl Default for GrainTextureCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// Grain texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GrainTextureFormat {
    /// 8-bit luminance
    #[default]
    R8    = 0,
    /// 16-bit luminance
    R16   = 1,
    /// RGB8 (color grain)
    Rgb8  = 2,
    /// RGB16 (high precision color)
    Rgb16 = 3,
}

impl GrainTextureFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::R16 => 2,
            Self::Rgb8 => 3,
            Self::Rgb16 => 6,
        }
    }

    /// Is colored
    pub const fn is_colored(&self) -> bool {
        matches!(self, Self::Rgb8 | Self::Rgb16)
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// Film grain GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FilmGrainGpuParams {
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Intensity
    pub intensity: f32,
    /// Size
    pub size: f32,
    /// Luminance ratio
    pub luminance_ratio: f32,
    /// Color strength
    pub color_strength: f32,
    /// Time (for animation)
    pub time: f32,
    /// Animation speed
    pub animation_speed: f32,
    /// Response curve params
    pub response_shadows: f32,
    /// Response midtones
    pub response_midtones: f32,
    /// Response highlights
    pub response_highlights: f32,
    /// Shadow threshold
    pub shadow_threshold: f32,
    /// Highlight threshold
    pub highlight_threshold: f32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: [f32; 2],
}

impl FilmGrainGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Use texture flag
    pub const FLAG_USE_TEXTURE: u32 = 1 << 0;
    /// Color grain flag
    pub const FLAG_COLORED: u32 = 1 << 1;
    /// Animate flag
    pub const FLAG_ANIMATED: u32 = 1 << 2;
}

// ============================================================================
// Dithering
// ============================================================================

/// Dithering configuration (often combined with grain)
#[derive(Clone, Copy, Debug)]
pub struct DitheringSettings {
    /// Enable dithering
    pub enabled: bool,
    /// Dithering method
    pub method: DitheringMethod,
    /// Strength
    pub strength: f32,
}

impl DitheringSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            method: DitheringMethod::BlueNoise,
            strength: 1.0,
        }
    }

    /// Blue noise dithering
    pub const fn blue_noise() -> Self {
        Self {
            enabled: true,
            method: DitheringMethod::BlueNoise,
            strength: 1.0,
        }
    }

    /// Ordered dithering
    pub const fn ordered() -> Self {
        Self {
            enabled: true,
            method: DitheringMethod::Ordered,
            strength: 1.0,
        }
    }
}

impl Default for DitheringSettings {
    fn default() -> Self {
        Self::blue_noise()
    }
}

/// Dithering method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DitheringMethod {
    /// No dithering
    None                = 0,
    /// Ordered (Bayer) dithering
    Ordered             = 1,
    /// Blue noise dithering
    #[default]
    BlueNoise           = 2,
    /// White noise dithering
    WhiteNoise          = 3,
    /// Interleaved gradient noise
    InterleavedGradient = 4,
}

// ============================================================================
// Pass Configuration
// ============================================================================

/// Film grain pass config
#[derive(Clone, Debug)]
pub struct FilmGrainPassConfig {
    /// Input color
    pub input: u64,
    /// Output
    pub output: u64,
    /// Grain texture (optional)
    pub grain_texture: GrainTextureHandle,
    /// Settings
    pub settings: FilmGrainSettings,
    /// Frame index (for animation)
    pub frame_index: u32,
    /// Dithering
    pub dithering: DitheringSettings,
}

impl FilmGrainPassConfig {
    /// Creates config
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input,
            output,
            grain_texture: GrainTextureHandle::NULL,
            settings: FilmGrainSettings::default(),
            frame_index: 0,
            dithering: DitheringSettings::default(),
        }
    }

    /// With settings
    pub fn with_settings(mut self, settings: FilmGrainSettings) -> Self {
        self.settings = settings;
        self
    }

    /// With texture
    pub fn with_texture(mut self, texture: GrainTextureHandle) -> Self {
        self.grain_texture = texture;
        self
    }
}

impl Default for FilmGrainPassConfig {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Film grain statistics
#[derive(Clone, Debug, Default)]
pub struct FilmGrainStats {
    /// Pass time (microseconds)
    pub pass_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
