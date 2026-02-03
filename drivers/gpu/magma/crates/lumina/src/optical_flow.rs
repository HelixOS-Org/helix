//! Optical Flow Types for Lumina
//!
//! This module provides hardware-accelerated optical flow estimation
//! for motion analysis, video interpolation, and temporal upscaling.

use alloc::vec::Vec;

use crate::video_decode::{Extent2D, ImageViewHandle, Offset2D, PictureFormat};

// ============================================================================
// Optical Flow Session Handle
// ============================================================================

/// Handle to an optical flow session
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OpticalFlowSessionHandle(pub u64);

impl OpticalFlowSessionHandle {
    /// Null handle constant
    pub const NULL: Self = Self(0);

    /// Creates a new handle from raw value
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw handle value
    #[inline]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    /// Checks if this is a null handle
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Checks if this is a valid handle
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for OpticalFlowSessionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Optical Flow Session Create Info
// ============================================================================

/// Optical flow session creation parameters
#[derive(Clone, Debug)]
#[repr(C)]
pub struct OpticalFlowSessionCreateInfo {
    /// Input image width
    pub width: u32,
    /// Input image height
    pub height: u32,
    /// Input image format
    pub input_format: OpticalFlowImageFormat,
    /// Output grid size
    pub output_grid_size: OutputGridSize,
    /// Hint grid size (optional input)
    pub hint_grid_size: OutputGridSize,
    /// Performance mode
    pub performance_mode: PerformanceMode,
    /// Flow direction
    pub flow_direction: FlowDirection,
    /// Session flags
    pub flags: OpticalFlowFlags,
}

impl OpticalFlowSessionCreateInfo {
    /// Creates a new session create info
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            input_format: OpticalFlowImageFormat::R8G8B8A8Unorm,
            output_grid_size: OutputGridSize::Size4x4,
            hint_grid_size: OutputGridSize::Size4x4,
            performance_mode: PerformanceMode::Balanced,
            flow_direction: FlowDirection::Forward,
            flags: OpticalFlowFlags::NONE,
        }
    }

    /// Creates for 1080p content
    #[inline]
    pub const fn hd_1080p() -> Self {
        Self::new(1920, 1080)
    }

    /// Creates for 4K content
    #[inline]
    pub const fn uhd_4k() -> Self {
        Self::new(3840, 2160)
    }

    /// Creates for frame interpolation
    #[inline]
    pub const fn for_interpolation(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            input_format: OpticalFlowImageFormat::R8G8B8A8Unorm,
            output_grid_size: OutputGridSize::Size1x1,
            hint_grid_size: OutputGridSize::Size4x4,
            performance_mode: PerformanceMode::Quality,
            flow_direction: FlowDirection::Bidirectional,
            flags: OpticalFlowFlags::ENABLE_COST.union(OpticalFlowFlags::ENABLE_GLOBAL_FLOW),
        }
    }

    /// Creates for motion estimation
    #[inline]
    pub const fn for_motion_estimation(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            input_format: OpticalFlowImageFormat::R8Unorm,
            output_grid_size: OutputGridSize::Size4x4,
            hint_grid_size: OutputGridSize::Size4x4,
            performance_mode: PerformanceMode::Fast,
            flow_direction: FlowDirection::Forward,
            flags: OpticalFlowFlags::NONE,
        }
    }

    /// Sets the input format
    #[inline]
    pub const fn with_format(mut self, format: OpticalFlowImageFormat) -> Self {
        self.input_format = format;
        self
    }

    /// Sets the output grid size
    #[inline]
    pub const fn with_grid_size(mut self, size: OutputGridSize) -> Self {
        self.output_grid_size = size;
        self
    }

    /// Sets performance mode
    #[inline]
    pub const fn with_performance(mut self, mode: PerformanceMode) -> Self {
        self.performance_mode = mode;
        self
    }

    /// Sets bidirectional flow
    #[inline]
    pub const fn bidirectional(mut self) -> Self {
        self.flow_direction = FlowDirection::Bidirectional;
        self
    }

    /// Enables cost output
    #[inline]
    pub const fn with_cost(mut self) -> Self {
        self.flags = self.flags.union(OpticalFlowFlags::ENABLE_COST);
        self
    }

    /// Enables global flow
    #[inline]
    pub const fn with_global_flow(mut self) -> Self {
        self.flags = self.flags.union(OpticalFlowFlags::ENABLE_GLOBAL_FLOW);
        self
    }

    /// Enables hint input
    #[inline]
    pub const fn with_hints(mut self) -> Self {
        self.flags = self.flags.union(OpticalFlowFlags::ENABLE_HINT);
        self
    }
}

impl Default for OpticalFlowSessionCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// Input image format for optical flow
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OpticalFlowImageFormat {
    /// Single channel 8-bit
    R8Unorm            = 0,
    /// Two channel 8-bit
    R8G8Unorm          = 1,
    /// RGBA 8-bit
    #[default]
    R8G8B8A8Unorm      = 2,
    /// BGRA 8-bit
    B8G8R8A8Unorm      = 3,
    /// Single channel 16-bit float
    R16Sfloat          = 4,
    /// Two channel 16-bit float
    R16G16Sfloat       = 5,
    /// RGBA 16-bit float
    R16G16B16A16Sfloat = 6,
    /// Single channel 32-bit float
    R32Sfloat          = 7,
    /// Two channel 32-bit float
    R32G32Sfloat       = 8,
    /// RGBA 32-bit float
    R32G32B32A32Sfloat = 9,
    /// NV12 format
    Nv12               = 10,
}

impl OpticalFlowImageFormat {
    /// Returns the number of channels
    #[inline]
    pub const fn channels(&self) -> u32 {
        match self {
            Self::R8Unorm | Self::R16Sfloat | Self::R32Sfloat => 1,
            Self::R8G8Unorm | Self::R16G16Sfloat | Self::R32G32Sfloat | Self::Nv12 => 2,
            Self::R8G8B8A8Unorm
            | Self::B8G8R8A8Unorm
            | Self::R16G16B16A16Sfloat
            | Self::R32G32B32A32Sfloat => 4,
        }
    }

    /// Returns the bytes per pixel
    #[inline]
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::R8G8Unorm => 2,
            Self::R8G8B8A8Unorm | Self::B8G8R8A8Unorm => 4,
            Self::R16Sfloat => 2,
            Self::R16G16Sfloat => 4,
            Self::R16G16B16A16Sfloat => 8,
            Self::R32Sfloat => 4,
            Self::R32G32Sfloat => 8,
            Self::R32G32B32A32Sfloat => 16,
            Self::Nv12 => 1, // Luma plane
        }
    }

    /// Checks if format is floating point
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(
            self,
            Self::R16Sfloat
                | Self::R16G16Sfloat
                | Self::R16G16B16A16Sfloat
                | Self::R32Sfloat
                | Self::R32G32Sfloat
                | Self::R32G32B32A32Sfloat
        )
    }
}

/// Output grid size for optical flow
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OutputGridSize {
    /// 1x1 (per-pixel flow)
    Size1x1 = 0,
    /// 2x2 grid
    Size2x2 = 1,
    /// 4x4 grid
    #[default]
    Size4x4 = 2,
    /// 8x8 grid
    Size8x8 = 3,
}

impl OutputGridSize {
    /// Returns the grid dimension
    #[inline]
    pub const fn dimension(&self) -> u32 {
        match self {
            Self::Size1x1 => 1,
            Self::Size2x2 => 2,
            Self::Size4x4 => 4,
            Self::Size8x8 => 8,
        }
    }

    /// Calculates the output dimensions
    #[inline]
    pub const fn output_dimensions(&self, width: u32, height: u32) -> (u32, u32) {
        let dim = self.dimension();
        ((width + dim - 1) / dim, (height + dim - 1) / dim)
    }

    /// Calculates the output size in flow vectors
    #[inline]
    pub const fn output_count(&self, width: u32, height: u32) -> u32 {
        let (w, h) = self.output_dimensions(width, height);
        w * h
    }
}

/// Performance mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PerformanceMode {
    /// Fast processing (lower quality)
    Fast     = 0,
    /// Balanced performance and quality
    #[default]
    Balanced = 1,
    /// Best quality (slower)
    Quality  = 2,
}

/// Flow direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FlowDirection {
    /// Forward flow only (frame N to N+1)
    #[default]
    Forward       = 0,
    /// Backward flow only (frame N+1 to N)
    Backward      = 1,
    /// Both forward and backward
    Bidirectional = 2,
}

/// Optical flow session flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct OpticalFlowFlags(pub u32);

impl OpticalFlowFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Enable cost output
    pub const ENABLE_COST: Self = Self(1 << 0);
    /// Enable global flow computation
    pub const ENABLE_GLOBAL_FLOW: Self = Self(1 << 1);
    /// Enable hint input
    pub const ENABLE_HINT: Self = Self(1 << 2);
    /// Enable external hints
    pub const ENABLE_EXTERNAL_HINT: Self = Self(1 << 3);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Optical Flow Execute Info
// ============================================================================

/// Information for executing optical flow
#[derive(Clone, Debug)]
#[repr(C)]
pub struct OpticalFlowExecuteInfo {
    /// Reference image (frame N)
    pub reference: OpticalFlowImage,
    /// Target image (frame N+1)
    pub target: OpticalFlowImage,
    /// Optional hint flow input
    pub hint: Option<OpticalFlowVector>,
    /// Output flow vector
    pub output_flow: OpticalFlowVector,
    /// Output backward flow (for bidirectional)
    pub output_backward_flow: Option<OpticalFlowVector>,
    /// Output cost (if enabled)
    pub output_cost: Option<OpticalFlowCost>,
    /// Execute flags
    pub flags: OpticalFlowExecuteFlags,
    /// Regions of interest
    pub regions: Vec<FlowRegion>,
}

impl OpticalFlowExecuteInfo {
    /// Creates a new execute info
    #[inline]
    pub fn new(
        reference: OpticalFlowImage,
        target: OpticalFlowImage,
        output: OpticalFlowVector,
    ) -> Self {
        Self {
            reference,
            target,
            hint: None,
            output_flow: output,
            output_backward_flow: None,
            output_cost: None,
            flags: OpticalFlowExecuteFlags::NONE,
            regions: Vec::new(),
        }
    }

    /// Sets hint input
    #[inline]
    pub fn with_hint(mut self, hint: OpticalFlowVector) -> Self {
        self.hint = Some(hint);
        self
    }

    /// Enables backward flow output
    #[inline]
    pub fn with_backward(mut self, backward: OpticalFlowVector) -> Self {
        self.output_backward_flow = Some(backward);
        self
    }

    /// Enables cost output
    #[inline]
    pub fn with_cost(mut self, cost: OpticalFlowCost) -> Self {
        self.output_cost = Some(cost);
        self
    }

    /// Adds a region of interest
    #[inline]
    pub fn with_region(mut self, region: FlowRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Uses temporal hints
    #[inline]
    pub fn use_temporal_hints(mut self) -> Self {
        self.flags = self.flags.union(OpticalFlowExecuteFlags::TEMPORAL_HINTS);
        self
    }
}

/// Optical flow input/output image
#[derive(Clone, Debug)]
#[repr(C)]
pub struct OpticalFlowImage {
    /// Image view handle
    pub image_view: ImageViewHandle,
    /// Image offset
    pub offset: Offset2D,
    /// Image extent
    pub extent: Extent2D,
}

impl OpticalFlowImage {
    /// Creates a new optical flow image
    #[inline]
    pub const fn new(image_view: ImageViewHandle, width: u32, height: u32) -> Self {
        Self {
            image_view,
            offset: Offset2D { x: 0, y: 0 },
            extent: Extent2D { width, height },
        }
    }

    /// With offset
    #[inline]
    pub const fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = Offset2D { x, y };
        self
    }

    /// With region
    #[inline]
    pub const fn with_region(mut self, x: i32, y: i32, width: u32, height: u32) -> Self {
        self.offset = Offset2D { x, y };
        self.extent = Extent2D { width, height };
        self
    }
}

/// Optical flow vector output
#[derive(Clone, Debug)]
#[repr(C)]
pub struct OpticalFlowVector {
    /// Image view for flow vectors
    pub image_view: ImageViewHandle,
    /// Vector format
    pub format: FlowVectorFormat,
    /// Grid size
    pub grid_size: OutputGridSize,
}

impl OpticalFlowVector {
    /// Creates a new flow vector output
    #[inline]
    pub const fn new(image_view: ImageViewHandle) -> Self {
        Self {
            image_view,
            format: FlowVectorFormat::RG16Sfloat,
            grid_size: OutputGridSize::Size4x4,
        }
    }

    /// Sets the format
    #[inline]
    pub const fn with_format(mut self, format: FlowVectorFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the grid size
    #[inline]
    pub const fn with_grid_size(mut self, size: OutputGridSize) -> Self {
        self.grid_size = size;
        self
    }

    /// Calculates storage size in bytes
    #[inline]
    pub const fn storage_size(&self, width: u32, height: u32) -> usize {
        let count = self.grid_size.output_count(width, height);
        (count * self.format.bytes_per_vector()) as usize
    }
}

/// Flow vector format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FlowVectorFormat {
    /// RG16 signed float (high precision)
    #[default]
    RG16Sfloat = 0,
    /// RG32 signed float (highest precision)
    RG32Sfloat = 1,
    /// RG16 signed normalized
    RG16Snorm  = 2,
    /// Compact fixed point
    FixedPoint = 3,
}

impl FlowVectorFormat {
    /// Returns bytes per vector
    #[inline]
    pub const fn bytes_per_vector(&self) -> u32 {
        match self {
            Self::RG16Sfloat | Self::RG16Snorm | Self::FixedPoint => 4,
            Self::RG32Sfloat => 8,
        }
    }

    /// Returns the maximum representable magnitude
    #[inline]
    pub const fn max_magnitude(&self) -> f32 {
        match self {
            Self::RG16Sfloat => 65504.0,
            Self::RG32Sfloat => f32::MAX,
            Self::RG16Snorm => 1.0,
            Self::FixedPoint => 128.0,
        }
    }
}

/// Optical flow cost output
#[derive(Clone, Debug)]
#[repr(C)]
pub struct OpticalFlowCost {
    /// Image view for cost
    pub image_view: ImageViewHandle,
    /// Cost format
    pub format: CostFormat,
}

impl OpticalFlowCost {
    /// Creates a new cost output
    #[inline]
    pub const fn new(image_view: ImageViewHandle) -> Self {
        Self {
            image_view,
            format: CostFormat::R8Unorm,
        }
    }

    /// With format
    #[inline]
    pub const fn with_format(mut self, format: CostFormat) -> Self {
        self.format = format;
        self
    }
}

/// Cost format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CostFormat {
    /// 8-bit unsigned normalized
    #[default]
    R8Unorm   = 0,
    /// 16-bit unsigned normalized
    R16Unorm  = 1,
    /// 16-bit float
    R16Sfloat = 2,
    /// 32-bit float
    R32Sfloat = 3,
}

impl CostFormat {
    /// Returns bytes per pixel
    #[inline]
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::R16Unorm | Self::R16Sfloat => 2,
            Self::R32Sfloat => 4,
        }
    }
}

/// Optical flow execute flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct OpticalFlowExecuteFlags(pub u32);

impl OpticalFlowExecuteFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable temporal hints
    pub const DISABLE_TEMPORAL_HINTS: Self = Self(1 << 0);
    /// Use temporal hints
    pub const TEMPORAL_HINTS: Self = Self(1 << 1);

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Region of interest for flow computation
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FlowRegion {
    /// Offset
    pub offset: Offset2D,
    /// Extent
    pub extent: Extent2D,
}

impl FlowRegion {
    /// Creates a new region
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D { x, y },
            extent: Extent2D { width, height },
        }
    }

    /// Creates a full-frame region
    #[inline]
    pub const fn full_frame(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

// ============================================================================
// Flow Analysis
// ============================================================================

/// Motion vector for a pixel or block
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MotionVector {
    /// Horizontal motion in pixels
    pub dx: f32,
    /// Vertical motion in pixels
    pub dy: f32,
}

impl MotionVector {
    /// Zero motion
    pub const ZERO: Self = Self { dx: 0.0, dy: 0.0 };

    /// Creates a new motion vector
    #[inline]
    pub const fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }

    /// Returns the magnitude of the motion
    #[inline]
    pub fn magnitude(&self) -> f32 {
        libm::sqrtf(self.dx * self.dx + self.dy * self.dy)
    }

    /// Returns the direction in radians
    #[inline]
    pub fn direction(&self) -> f32 {
        libm::atan2f(self.dy, self.dx)
    }

    /// Returns the direction in degrees
    #[inline]
    pub fn direction_degrees(&self) -> f32 {
        self.direction() * 180.0 / core::f32::consts::PI
    }

    /// Normalizes the motion vector
    #[inline]
    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag < 1e-6 {
            Self::ZERO
        } else {
            Self {
                dx: self.dx / mag,
                dy: self.dy / mag,
            }
        }
    }

    /// Scales the motion vector
    #[inline]
    pub const fn scaled(&self, factor: f32) -> Self {
        Self {
            dx: self.dx * factor,
            dy: self.dy * factor,
        }
    }

    /// Adds two motion vectors
    #[inline]
    pub const fn add(&self, other: &Self) -> Self {
        Self {
            dx: self.dx + other.dx,
            dy: self.dy + other.dy,
        }
    }

    /// Subtracts two motion vectors
    #[inline]
    pub const fn sub(&self, other: &Self) -> Self {
        Self {
            dx: self.dx - other.dx,
            dy: self.dy - other.dy,
        }
    }

    /// Linear interpolation
    #[inline]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            dx: self.dx + (other.dx - self.dx) * t,
            dy: self.dy + (other.dy - self.dy) * t,
        }
    }
}

/// Global motion model
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GlobalMotion {
    /// Translation X
    pub tx: f32,
    /// Translation Y
    pub ty: f32,
    /// Rotation angle in radians
    pub rotation: f32,
    /// Scale factor
    pub scale: f32,
    /// Shear X
    pub shear_x: f32,
    /// Shear Y
    pub shear_y: f32,
}

impl GlobalMotion {
    /// Identity (no motion)
    pub const IDENTITY: Self = Self {
        tx: 0.0,
        ty: 0.0,
        rotation: 0.0,
        scale: 1.0,
        shear_x: 0.0,
        shear_y: 0.0,
    };

    /// Creates a translation-only motion
    #[inline]
    pub const fn translation(tx: f32, ty: f32) -> Self {
        Self {
            tx,
            ty,
            rotation: 0.0,
            scale: 1.0,
            shear_x: 0.0,
            shear_y: 0.0,
        }
    }

    /// Creates a rotation motion
    #[inline]
    pub const fn rotation(angle: f32) -> Self {
        Self {
            tx: 0.0,
            ty: 0.0,
            rotation: angle,
            scale: 1.0,
            shear_x: 0.0,
            shear_y: 0.0,
        }
    }

    /// Creates a zoom motion
    #[inline]
    pub const fn zoom(scale: f32) -> Self {
        Self {
            tx: 0.0,
            ty: 0.0,
            rotation: 0.0,
            scale,
            shear_x: 0.0,
            shear_y: 0.0,
        }
    }

    /// Transforms a point
    #[inline]
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        // Apply shear
        let sx = x + self.shear_x * y;
        let sy = y + self.shear_y * x;

        // Apply scale
        let sx = sx * self.scale;
        let sy = sy * self.scale;

        // Apply rotation
        let cos_r = libm::cosf(self.rotation);
        let sin_r = libm::sinf(self.rotation);
        let rx = sx * cos_r - sy * sin_r;
        let ry = sx * sin_r + sy * cos_r;

        // Apply translation
        (rx + self.tx, ry + self.ty)
    }

    /// Checks if this is pure translation
    #[inline]
    pub fn is_translation(&self) -> bool {
        libm::fabsf(self.rotation) < 1e-6
            && libm::fabsf(self.scale - 1.0) < 1e-6
            && libm::fabsf(self.shear_x) < 1e-6
            && libm::fabsf(self.shear_y) < 1e-6
    }

    /// Checks if this is identity
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.is_translation() && libm::fabsf(self.tx) < 1e-6 && libm::fabsf(self.ty) < 1e-6
    }
}

// ============================================================================
// Frame Interpolation
// ============================================================================

/// Frame interpolation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InterpolationMode {
    /// Simple linear blend
    Linear            = 0,
    /// Motion-compensated interpolation
    #[default]
    MotionCompensated = 1,
    /// Bidirectional motion compensation
    Bidirectional     = 2,
    /// Adaptive blending
    Adaptive          = 3,
}

/// Frame interpolation configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct FrameInterpolationConfig {
    /// Interpolation mode
    pub mode: InterpolationMode,
    /// Number of intermediate frames to generate
    pub intermediate_frames: u32,
    /// Occlusion handling
    pub occlusion_handling: OcclusionHandling,
    /// Motion blur simulation
    pub motion_blur: bool,
    /// Motion blur strength (0.0 - 1.0)
    pub motion_blur_strength: f32,
    /// Edge handling
    pub edge_handling: EdgeHandling,
}

impl FrameInterpolationConfig {
    /// Creates a new config for single frame interpolation
    #[inline]
    pub const fn single_frame() -> Self {
        Self {
            mode: InterpolationMode::MotionCompensated,
            intermediate_frames: 1,
            occlusion_handling: OcclusionHandling::Blend,
            motion_blur: false,
            motion_blur_strength: 0.0,
            edge_handling: EdgeHandling::Mirror,
        }
    }

    /// Creates for 2x frame rate
    #[inline]
    pub const fn double_framerate() -> Self {
        Self::single_frame()
    }

    /// Creates for 4x frame rate
    #[inline]
    pub const fn quad_framerate() -> Self {
        Self {
            mode: InterpolationMode::Bidirectional,
            intermediate_frames: 3,
            occlusion_handling: OcclusionHandling::Forward,
            motion_blur: false,
            motion_blur_strength: 0.0,
            edge_handling: EdgeHandling::Mirror,
        }
    }

    /// Creates for slow motion
    #[inline]
    pub const fn slow_motion(factor: u32) -> Self {
        Self {
            mode: InterpolationMode::Bidirectional,
            intermediate_frames: factor - 1,
            occlusion_handling: OcclusionHandling::Adaptive,
            motion_blur: true,
            motion_blur_strength: 0.5,
            edge_handling: EdgeHandling::Mirror,
        }
    }

    /// Sets the mode
    #[inline]
    pub const fn with_mode(mut self, mode: InterpolationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enables motion blur
    #[inline]
    pub const fn with_motion_blur(mut self, strength: f32) -> Self {
        self.motion_blur = true;
        self.motion_blur_strength = strength;
        self
    }
}

impl Default for FrameInterpolationConfig {
    fn default() -> Self {
        Self::single_frame()
    }
}

/// Occlusion handling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OcclusionHandling {
    /// Blend occluded regions
    #[default]
    Blend    = 0,
    /// Use forward warped pixels
    Forward  = 1,
    /// Use backward warped pixels
    Backward = 2,
    /// Adaptive selection
    Adaptive = 3,
}

/// Edge handling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EdgeHandling {
    /// Clamp to edge
    Clamp  = 0,
    /// Mirror at edge
    #[default]
    Mirror = 1,
    /// Repeat/wrap
    Repeat = 2,
    /// Use border color (black)
    Border = 3,
}

// ============================================================================
// Optical Flow Statistics
// ============================================================================

/// Statistics from optical flow computation
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct OpticalFlowStats {
    /// Average motion magnitude
    pub avg_magnitude: f32,
    /// Maximum motion magnitude
    pub max_magnitude: f32,
    /// Percentage of moving pixels
    pub motion_coverage: f32,
    /// Global motion estimate
    pub global_motion: GlobalMotion,
    /// Average confidence
    pub avg_confidence: f32,
    /// Computation time in microseconds
    pub compute_time_us: u64,
}

impl OpticalFlowStats {
    /// Checks if scene is mostly static
    #[inline]
    pub fn is_static(&self, threshold: f32) -> bool {
        self.avg_magnitude < threshold
    }

    /// Checks if there's significant camera motion
    #[inline]
    pub fn has_camera_motion(&self) -> bool {
        !self.global_motion.is_identity() && self.motion_coverage > 0.7
    }

    /// Checks if there's significant object motion
    #[inline]
    pub fn has_object_motion(&self, threshold: f32) -> bool {
        self.motion_coverage > 0.1 && self.max_magnitude > threshold
    }
}

/// Temporal coherence check
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TemporalCoherence {
    /// Coherence score (0.0 - 1.0)
    pub score: f32,
    /// Number of inconsistent regions
    pub inconsistent_regions: u32,
    /// Average temporal error
    pub avg_temporal_error: f32,
}

impl TemporalCoherence {
    /// Perfect coherence
    pub const PERFECT: Self = Self {
        score: 1.0,
        inconsistent_regions: 0,
        avg_temporal_error: 0.0,
    };

    /// Checks if temporally coherent
    #[inline]
    pub const fn is_coherent(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
}

impl Default for TemporalCoherence {
    fn default() -> Self {
        Self::PERFECT
    }
}
