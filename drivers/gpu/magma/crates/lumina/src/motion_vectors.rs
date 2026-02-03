//! Motion Vector Types for Lumina
//!
//! This module provides motion vector generation infrastructure for
//! temporal effects, motion blur, and TAA.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Motion Vector Handles
// ============================================================================

/// Motion vector buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MotionVectorBufferHandle(pub u64);

impl MotionVectorBufferHandle {
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

impl Default for MotionVectorBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Velocity buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VelocityBufferHandle(pub u64);

impl VelocityBufferHandle {
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

impl Default for VelocityBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Motion Vector Buffer
// ============================================================================

/// Motion vector buffer create info
#[derive(Clone, Debug)]
pub struct MotionVectorBufferCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: MotionVectorFormat,
    /// Generate from depth
    pub use_depth_reconstruction: bool,
    /// Half resolution
    pub half_resolution: bool,
}

impl MotionVectorBufferCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            format: MotionVectorFormat::Rg16Float,
            use_depth_reconstruction: false,
            half_resolution: false,
        }
    }

    /// 1080p motion vectors
    pub fn hd() -> Self {
        Self::new(1920, 1080)
    }

    /// 4K motion vectors
    pub fn uhd() -> Self {
        Self::new(3840, 2160)
    }

    /// With format
    pub fn with_format(mut self, format: MotionVectorFormat) -> Self {
        self.format = format;
        self
    }

    /// Half resolution
    pub fn half_res(mut self) -> Self {
        self.half_resolution = true;
        self.width /= 2;
        self.height /= 2;
        self
    }

    /// With depth reconstruction
    pub fn with_depth_reconstruction(mut self) -> Self {
        self.use_depth_reconstruction = true;
        self
    }

    /// Effective dimensions
    pub fn effective_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        (self.width as u64) * (self.height as u64) * (self.format.bytes_per_pixel() as u64)
    }
}

impl Default for MotionVectorBufferCreateInfo {
    fn default() -> Self {
        Self::hd()
    }
}

/// Motion vector format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MotionVectorFormat {
    /// RG16 float (good quality, 4 bytes)
    #[default]
    Rg16Float = 0,
    /// RG32 float (high precision, 8 bytes)
    Rg32Float = 1,
    /// RG16 snorm (compact, 4 bytes)
    Rg16Snorm = 2,
    /// RG8 snorm (very compact, 2 bytes)
    Rg8Snorm = 3,
    /// RGBA16 float (with depth/confidence)
    Rgba16Float = 4,
}

impl MotionVectorFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rg16Float => 4,
            Self::Rg32Float => 8,
            Self::Rg16Snorm => 4,
            Self::Rg8Snorm => 2,
            Self::Rgba16Float => 8,
        }
    }

    /// Has extra channels
    pub const fn has_extra_channels(&self) -> bool {
        matches!(self, Self::Rgba16Float)
    }

    /// Precision level (1-3)
    pub const fn precision(&self) -> u32 {
        match self {
            Self::Rg8Snorm => 1,
            Self::Rg16Snorm | Self::Rg16Float => 2,
            Self::Rg32Float | Self::Rgba16Float => 3,
        }
    }
}

// ============================================================================
// Motion Vector Generation
// ============================================================================

/// Motion vector generation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MotionVectorMode {
    /// Per-vertex (from vertex shader)
    #[default]
    PerVertex = 0,
    /// Per-pixel (from pixel shader)
    PerPixel = 1,
    /// Depth-based reconstruction
    DepthReconstruction = 2,
    /// Hybrid (vertex + depth)
    Hybrid = 3,
    /// Optical flow (compute-based)
    OpticalFlow = 4,
}

impl MotionVectorMode {
    /// Requires previous frame depth
    pub const fn requires_prev_depth(&self) -> bool {
        matches!(self, Self::DepthReconstruction | Self::Hybrid)
    }

    /// Requires per-object motion
    pub const fn requires_object_motion(&self) -> bool {
        matches!(self, Self::PerVertex | Self::PerPixel | Self::Hybrid)
    }

    /// Requires compute pass
    pub const fn requires_compute(&self) -> bool {
        matches!(self, Self::DepthReconstruction | Self::OpticalFlow)
    }
}

/// Motion vector pass config
#[derive(Clone, Debug)]
pub struct MotionVectorPassConfig {
    /// Output buffer
    pub output: MotionVectorBufferHandle,
    /// Generation mode
    pub mode: MotionVectorMode,
    /// Jitter offset (for TAA)
    pub jitter_offset: [f32; 2],
    /// Include camera motion
    pub include_camera_motion: bool,
    /// Include object motion
    pub include_object_motion: bool,
    /// Max motion vector length (pixels)
    pub max_length: f32,
    /// Velocity scale
    pub velocity_scale: f32,
}

impl MotionVectorPassConfig {
    /// Creates config
    pub fn new(output: MotionVectorBufferHandle) -> Self {
        Self {
            output,
            mode: MotionVectorMode::PerVertex,
            jitter_offset: [0.0, 0.0],
            include_camera_motion: true,
            include_object_motion: true,
            max_length: 64.0,
            velocity_scale: 1.0,
        }
    }

    /// With mode
    pub fn with_mode(mut self, mode: MotionVectorMode) -> Self {
        self.mode = mode;
        self
    }

    /// With jitter
    pub fn with_jitter(mut self, x: f32, y: f32) -> Self {
        self.jitter_offset = [x, y];
        self
    }

    /// Camera only
    pub fn camera_only(mut self) -> Self {
        self.include_camera_motion = true;
        self.include_object_motion = false;
        self
    }

    /// Object only
    pub fn object_only(mut self) -> Self {
        self.include_camera_motion = false;
        self.include_object_motion = true;
        self
    }
}

impl Default for MotionVectorPassConfig {
    fn default() -> Self {
        Self::new(MotionVectorBufferHandle::NULL)
    }
}

// ============================================================================
// Motion Vector GPU Data
// ============================================================================

/// Motion vector GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MotionVectorParams {
    /// Current view-projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Previous view-projection matrix
    pub prev_view_proj: [[f32; 4]; 4],
    /// Inverse current view-projection
    pub inv_view_proj: [[f32; 4]; 4],
    /// Jitter offset
    pub jitter: [f32; 2],
    /// Previous jitter
    pub prev_jitter: [f32; 2],
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Max motion length
    pub max_length: f32,
    /// Velocity scale
    pub velocity_scale: f32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: [u32; 3],
}

impl MotionVectorParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Include camera motion flag
    pub const FLAG_CAMERA_MOTION: u32 = 1 << 0;
    /// Include object motion flag
    pub const FLAG_OBJECT_MOTION: u32 = 1 << 1;
    /// Half resolution flag
    pub const FLAG_HALF_RES: u32 = 1 << 2;
    /// Apply jitter flag
    pub const FLAG_APPLY_JITTER: u32 = 1 << 3;
}

/// Per-object motion data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ObjectMotionData {
    /// Current transform
    pub transform: [[f32; 4]; 4],
    /// Previous transform
    pub prev_transform: [[f32; 4]; 4],
}

impl ObjectMotionData {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Identity (no motion)
    pub const fn identity() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            prev_transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

// ============================================================================
// Optical Flow
// ============================================================================

/// Optical flow create info
#[derive(Clone, Debug)]
pub struct OpticalFlowCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Pyramid levels
    pub pyramid_levels: u32,
    /// Iterations per level
    pub iterations: u32,
    /// Quality preset
    pub quality: OpticalFlowQuality,
}

impl OpticalFlowCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            pyramid_levels: 4,
            iterations: 5,
            quality: OpticalFlowQuality::Medium,
        }
    }

    /// Performance preset
    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            pyramid_levels: 3,
            iterations: 3,
            quality: OpticalFlowQuality::Low,
            ..Self::new(width, height)
        }
    }

    /// Quality preset
    pub fn quality_preset(width: u32, height: u32) -> Self {
        Self {
            pyramid_levels: 5,
            iterations: 10,
            quality: OpticalFlowQuality::High,
            ..Self::new(width, height)
        }
    }

    /// With levels
    pub fn with_levels(mut self, levels: u32) -> Self {
        self.pyramid_levels = levels;
        self
    }
}

impl Default for OpticalFlowCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// Optical flow quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OpticalFlowQuality {
    /// Low quality (fast)
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
}

impl OpticalFlowQuality {
    /// Window size
    pub const fn window_size(&self) -> u32 {
        match self {
            Self::Low => 5,
            Self::Medium => 9,
            Self::High => 15,
        }
    }
}

/// Optical flow GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct OpticalFlowParams {
    /// Image dimensions
    pub image_size: [u32; 2],
    /// Current level
    pub level: u32,
    /// Total levels
    pub num_levels: u32,
    /// Window radius
    pub window_radius: u32,
    /// Iteration
    pub iteration: u32,
    /// Smoothness weight
    pub smoothness: f32,
    /// Padding
    pub _padding: u32,
}

impl OpticalFlowParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Motion Blur Integration
// ============================================================================

/// Motion blur from velocity config
#[derive(Clone, Debug)]
pub struct MotionBlurVelocityConfig {
    /// Velocity buffer
    pub velocity_buffer: MotionVectorBufferHandle,
    /// Max blur samples
    pub max_samples: u32,
    /// Blur strength
    pub strength: f32,
    /// Center weight
    pub center_weight: f32,
    /// Use tile max
    pub use_tile_max: bool,
    /// Tile size
    pub tile_size: u32,
}

impl MotionBlurVelocityConfig {
    /// Creates config
    pub fn new(velocity: MotionVectorBufferHandle) -> Self {
        Self {
            velocity_buffer: velocity,
            max_samples: 16,
            strength: 1.0,
            center_weight: 0.5,
            use_tile_max: true,
            tile_size: 16,
        }
    }

    /// Performance preset
    pub fn performance(velocity: MotionVectorBufferHandle) -> Self {
        Self {
            max_samples: 8,
            use_tile_max: true,
            tile_size: 32,
            ..Self::new(velocity)
        }
    }

    /// Quality preset
    pub fn quality(velocity: MotionVectorBufferHandle) -> Self {
        Self {
            max_samples: 32,
            use_tile_max: true,
            tile_size: 8,
            ..Self::new(velocity)
        }
    }

    /// With strength
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

impl Default for MotionBlurVelocityConfig {
    fn default() -> Self {
        Self::new(MotionVectorBufferHandle::NULL)
    }
}

/// Tile velocity (for motion blur tiles)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TileVelocity {
    /// Max velocity in tile
    pub max_velocity: [f32; 2],
    /// Min velocity in tile
    pub min_velocity: [f32; 2],
}

impl TileVelocity {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Max length
    pub fn max_length(&self) -> f32 {
        (self.max_velocity[0].powi(2) + self.max_velocity[1].powi(2)).sqrt()
    }
}

// ============================================================================
// Motion Vector Utilities
// ============================================================================

/// Motion vector encoding
#[derive(Clone, Copy, Debug, Default)]
pub struct MotionVectorEncoding;

impl MotionVectorEncoding {
    /// Encode motion vector to RG16 snorm
    pub fn encode_rg16_snorm(motion: [f32; 2], max_length: f32) -> [i16; 2] {
        let scale = 32767.0 / max_length;
        [
            (motion[0] * scale).clamp(-32767.0, 32767.0) as i16,
            (motion[1] * scale).clamp(-32767.0, 32767.0) as i16,
        ]
    }

    /// Decode motion vector from RG16 snorm
    pub fn decode_rg16_snorm(encoded: [i16; 2], max_length: f32) -> [f32; 2] {
        let scale = max_length / 32767.0;
        [encoded[0] as f32 * scale, encoded[1] as f32 * scale]
    }

    /// Encode motion vector to RG8 snorm
    pub fn encode_rg8_snorm(motion: [f32; 2], max_length: f32) -> [i8; 2] {
        let scale = 127.0 / max_length;
        [
            (motion[0] * scale).clamp(-127.0, 127.0) as i8,
            (motion[1] * scale).clamp(-127.0, 127.0) as i8,
        ]
    }

    /// Decode motion vector from RG8 snorm
    pub fn decode_rg8_snorm(encoded: [i8; 2], max_length: f32) -> [f32; 2] {
        let scale = max_length / 127.0;
        [encoded[0] as f32 * scale, encoded[1] as f32 * scale]
    }
}

/// Motion vector validation
#[derive(Clone, Copy, Debug)]
pub struct MotionVectorValidation {
    /// Max valid length
    pub max_length: f32,
    /// Confidence threshold
    pub confidence_threshold: f32,
}

impl MotionVectorValidation {
    /// Default validation
    pub const fn default_validation() -> Self {
        Self {
            max_length: 64.0,
            confidence_threshold: 0.1,
        }
    }

    /// Is valid motion vector
    pub fn is_valid(&self, motion: [f32; 2]) -> bool {
        let length = (motion[0].powi(2) + motion[1].powi(2)).sqrt();
        length <= self.max_length
    }

    /// Clamp motion vector
    pub fn clamp(&self, motion: [f32; 2]) -> [f32; 2] {
        let length = (motion[0].powi(2) + motion[1].powi(2)).sqrt();
        if length > self.max_length {
            let scale = self.max_length / length;
            [motion[0] * scale, motion[1] * scale]
        } else {
            motion
        }
    }
}

impl Default for MotionVectorValidation {
    fn default() -> Self {
        Self::default_validation()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Motion vector statistics
#[derive(Clone, Debug, Default)]
pub struct MotionVectorStats {
    /// Average motion length
    pub avg_motion_length: f32,
    /// Max motion length
    pub max_motion_length: f32,
    /// Pixels with motion
    pub pixels_with_motion: u64,
    /// Total pixels
    pub total_pixels: u64,
    /// Generation time (microseconds)
    pub generation_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl MotionVectorStats {
    /// Motion coverage
    pub fn motion_coverage(&self) -> f32 {
        if self.total_pixels > 0 {
            self.pixels_with_motion as f32 / self.total_pixels as f32
        } else {
            0.0
        }
    }
}
