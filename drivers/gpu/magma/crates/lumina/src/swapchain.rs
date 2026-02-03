//! Swapchain and presentation types
//!
//! This module provides types for window surface management and presentation.

extern crate alloc;
use alloc::vec::Vec;

use crate::texture::TextureHandle;
use crate::types::Format;

/// Swapchain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwapchainHandle(pub u64);

impl SwapchainHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Surface handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SurfaceHandle(pub u64);

impl SurfaceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Present mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PresentMode {
    /// Immediate (may tear)
    Immediate,
    /// Mailbox (triple-buffered vsync)
    Mailbox,
    /// FIFO (double-buffered vsync, required)
    Fifo,
    /// FIFO relaxed
    FifoRelaxed,
    /// Shared demand refresh
    SharedDemandRefresh,
    /// Shared continuous refresh
    SharedContinuousRefresh,
}

impl Default for PresentMode {
    fn default() -> Self {
        Self::Fifo
    }
}

impl PresentMode {
    /// Returns true if vsync enabled
    pub const fn is_vsync(&self) -> bool {
        matches!(self, Self::Fifo | Self::FifoRelaxed | Self::Mailbox)
    }
}

/// Color space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorSpace {
    /// sRGB nonlinear
    SrgbNonlinear,
    /// Display P3 nonlinear
    DisplayP3Nonlinear,
    /// Extended sRGB linear
    ExtendedSrgbLinear,
    /// Display P3 linear
    DisplayP3Linear,
    /// DCI-P3 nonlinear
    DciP3Nonlinear,
    /// BT709 linear
    Bt709Linear,
    /// BT709 nonlinear
    Bt709Nonlinear,
    /// BT2020 linear
    Bt2020Linear,
    /// HDR10 ST2084
    Hdr10St2084,
    /// Dolby Vision
    DolbyVision,
    /// HDR10 HLG
    Hdr10Hlg,
    /// Adobe RGB linear
    AdobeRgbLinear,
    /// Adobe RGB nonlinear
    AdobeRgbNonlinear,
    /// Pass through (no conversion)
    PassThrough,
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::SrgbNonlinear
    }
}

/// Surface format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SurfaceFormat {
    /// Texture format
    pub format: Format,
    /// Color space
    pub color_space: ColorSpace,
}

impl SurfaceFormat {
    /// Standard sRGB format
    pub const SRGB: Self = Self {
        format: Format::Bgra8UnormSrgb,
        color_space: ColorSpace::SrgbNonlinear,
    };

    /// HDR10 format
    pub const HDR10: Self = Self {
        format: Format::Rgba16Float,
        color_space: ColorSpace::Hdr10St2084,
    };

    /// Creates a new surface format
    pub const fn new(format: Format, color_space: ColorSpace) -> Self {
        Self { format, color_space }
    }
}

impl Default for SurfaceFormat {
    fn default() -> Self {
        Self::SRGB
    }
}

/// Surface capabilities
#[derive(Clone, Debug, Default)]
pub struct SurfaceCapabilities {
    /// Minimum image count
    pub min_image_count: u32,
    /// Maximum image count (0 = unlimited)
    pub max_image_count: u32,
    /// Current extent
    pub current_extent: [u32; 2],
    /// Minimum extent
    pub min_extent: [u32; 2],
    /// Maximum extent
    pub max_extent: [u32; 2],
    /// Maximum image array layers
    pub max_image_array_layers: u32,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
    /// Current transform
    pub current_transform: SurfaceTransform,
    /// Supported composite alpha
    pub supported_composite_alpha: CompositeAlphaFlags,
    /// Supported usage flags
    pub supported_usage: ImageUsageFlags,
}

/// Surface transform
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SurfaceTransform {
    /// No transform
    #[default]
    Identity,
    /// 90 degree rotation
    Rotate90,
    /// 180 degree rotation
    Rotate180,
    /// 270 degree rotation
    Rotate270,
    /// Horizontal mirror
    HorizontalMirror,
    /// Horizontal mirror + 90 degree rotation
    HorizontalMirrorRotate90,
    /// Horizontal mirror + 180 degree rotation
    HorizontalMirrorRotate180,
    /// Horizontal mirror + 270 degree rotation
    HorizontalMirrorRotate270,
    /// Inherit from surface
    Inherit,
}

/// Surface transform flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SurfaceTransformFlags(pub u32);

impl SurfaceTransformFlags {
    /// Identity
    pub const IDENTITY: Self = Self(1 << 0);
    /// Rotate 90
    pub const ROTATE_90: Self = Self(1 << 1);
    /// Rotate 180
    pub const ROTATE_180: Self = Self(1 << 2);
    /// Rotate 270
    pub const ROTATE_270: Self = Self(1 << 3);
    /// Horizontal mirror
    pub const HORIZONTAL_MIRROR: Self = Self(1 << 4);

    /// Checks if flag is set
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

/// Composite alpha mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CompositeAlpha {
    /// Opaque (ignore alpha)
    #[default]
    Opaque,
    /// Pre-multiplied alpha
    PreMultiplied,
    /// Post-multiplied alpha
    PostMultiplied,
    /// Inherit
    Inherit,
}

/// Composite alpha flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CompositeAlphaFlags(pub u32);

impl CompositeAlphaFlags {
    /// Opaque
    pub const OPAQUE: Self = Self(1 << 0);
    /// Pre-multiplied
    pub const PRE_MULTIPLIED: Self = Self(1 << 1);
    /// Post-multiplied
    pub const POST_MULTIPLIED: Self = Self(1 << 2);
    /// Inherit
    pub const INHERIT: Self = Self(1 << 3);
}

/// Image usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageUsageFlags(pub u32);

impl ImageUsageFlags {
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Sampled (texture)
    pub const SAMPLED: Self = Self(1 << 2);
    /// Storage (read/write)
    pub const STORAGE: Self = Self(1 << 3);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 4);
    /// Depth/stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 5);
    /// Transient attachment
    pub const TRANSIENT_ATTACHMENT: Self = Self(1 << 6);
    /// Input attachment
    pub const INPUT_ATTACHMENT: Self = Self(1 << 7);
}

impl core::ops::BitOr for ImageUsageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Swapchain create info
#[derive(Clone, Debug)]
pub struct SwapchainCreateInfo {
    /// Surface
    pub surface: SurfaceHandle,
    /// Minimum image count
    pub min_image_count: u32,
    /// Image format
    pub format: SurfaceFormat,
    /// Image extent
    pub extent: [u32; 2],
    /// Image array layers
    pub image_array_layers: u32,
    /// Image usage
    pub image_usage: ImageUsageFlags,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Queue family indices (for concurrent sharing)
    pub queue_family_indices: Vec<u32>,
    /// Pre-transform
    pub pre_transform: SurfaceTransform,
    /// Composite alpha
    pub composite_alpha: CompositeAlpha,
    /// Present mode
    pub present_mode: PresentMode,
    /// Clipped
    pub clipped: bool,
    /// Old swapchain (for resize)
    pub old_swapchain: SwapchainHandle,
}

impl SwapchainCreateInfo {
    /// Creates new swapchain info
    pub fn new(surface: SurfaceHandle, width: u32, height: u32) -> Self {
        Self {
            surface,
            min_image_count: 3,
            format: SurfaceFormat::SRGB,
            extent: [width, height],
            image_array_layers: 1,
            image_usage: ImageUsageFlags::COLOR_ATTACHMENT,
            sharing_mode: SharingMode::Exclusive,
            queue_family_indices: Vec::new(),
            pre_transform: SurfaceTransform::Identity,
            composite_alpha: CompositeAlpha::Opaque,
            present_mode: PresentMode::Fifo,
            clipped: true,
            old_swapchain: SwapchainHandle::NULL,
        }
    }

    /// Sets present mode
    pub fn with_present_mode(mut self, mode: PresentMode) -> Self {
        self.present_mode = mode;
        self
    }

    /// Sets format
    pub fn with_format(mut self, format: SurfaceFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets image count
    pub fn with_image_count(mut self, count: u32) -> Self {
        self.min_image_count = count;
        self
    }

    /// For HDR
    pub fn hdr(mut self) -> Self {
        self.format = SurfaceFormat::HDR10;
        self
    }
}

/// Sharing mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SharingMode {
    /// Exclusive to one queue
    #[default]
    Exclusive,
    /// Shared between queues
    Concurrent,
}

/// Acquire result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquireResult {
    /// Success with image index
    Success(u32),
    /// Suboptimal but usable
    Suboptimal(u32),
    /// Need to recreate swapchain
    OutOfDate,
    /// Timeout
    Timeout,
    /// Not ready
    NotReady,
}

impl AcquireResult {
    /// Gets image index if successful
    pub fn image_index(&self) -> Option<u32> {
        match self {
            Self::Success(i) | Self::Suboptimal(i) => Some(*i),
            _ => None,
        }
    }

    /// Whether the swapchain needs recreation
    pub fn needs_recreation(&self) -> bool {
        matches!(self, Self::OutOfDate)
    }
}

/// Present result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentResult {
    /// Success
    Success,
    /// Suboptimal
    Suboptimal,
    /// Out of date
    OutOfDate,
}

impl PresentResult {
    /// Whether the swapchain needs recreation
    pub fn needs_recreation(&self) -> bool {
        matches!(self, Self::OutOfDate)
    }
}

/// Swapchain images info
#[derive(Clone, Debug)]
pub struct SwapchainImages {
    /// Image handles
    pub images: Vec<TextureHandle>,
    /// Format
    pub format: Format,
    /// Extent
    pub extent: [u32; 2],
}

impl SwapchainImages {
    /// Number of images
    pub fn count(&self) -> usize {
        self.images.len()
    }

    /// Gets image at index
    pub fn get(&self, index: usize) -> Option<TextureHandle> {
        self.images.get(index).copied()
    }
}

/// Full screen exclusive mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FullScreenExclusive {
    /// Default (platform decides)
    Default,
    /// Application controlled
    ApplicationControlled,
    /// Disallowed
    Disallowed,
    /// Allowed
    Allowed,
}

impl Default for FullScreenExclusive {
    fn default() -> Self {
        Self::Default
    }
}

/// Display timing info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DisplayTimingInfo {
    /// Refresh duration in nanoseconds
    pub refresh_duration: u64,
    /// Present ID
    pub present_id: u32,
}

/// Present timing info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PresentTimingInfo {
    /// Present ID
    pub present_id: u32,
    /// Desired present time
    pub desired_present_time: u64,
    /// Actual present time
    pub actual_present_time: u64,
    /// Earliest present time
    pub earliest_present_time: u64,
    /// Present margin
    pub present_margin: u64,
}

/// HDR metadata
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HdrMetadata {
    /// Display primary red (x, y)
    pub display_primary_red: [f32; 2],
    /// Display primary green (x, y)
    pub display_primary_green: [f32; 2],
    /// Display primary blue (x, y)
    pub display_primary_blue: [f32; 2],
    /// White point (x, y)
    pub white_point: [f32; 2],
    /// Max luminance in nits
    pub max_luminance: f32,
    /// Min luminance in nits
    pub min_luminance: f32,
    /// Max content light level
    pub max_content_light_level: f32,
    /// Max frame average light level
    pub max_frame_average_light_level: f32,
}

impl HdrMetadata {
    /// sRGB / Rec.709 primaries
    pub const fn srgb() -> Self {
        Self {
            display_primary_red: [0.64, 0.33],
            display_primary_green: [0.30, 0.60],
            display_primary_blue: [0.15, 0.06],
            white_point: [0.3127, 0.3290],
            max_luminance: 80.0,
            min_luminance: 0.0,
            max_content_light_level: 80.0,
            max_frame_average_light_level: 80.0,
        }
    }

    /// Rec.2020 primaries (for HDR10)
    pub const fn rec2020() -> Self {
        Self {
            display_primary_red: [0.708, 0.292],
            display_primary_green: [0.170, 0.797],
            display_primary_blue: [0.131, 0.046],
            white_point: [0.3127, 0.3290],
            max_luminance: 1000.0,
            min_luminance: 0.0,
            max_content_light_level: 1000.0,
            max_frame_average_light_level: 500.0,
        }
    }

    /// Display P3 primaries
    pub const fn display_p3() -> Self {
        Self {
            display_primary_red: [0.680, 0.320],
            display_primary_green: [0.265, 0.690],
            display_primary_blue: [0.150, 0.060],
            white_point: [0.3127, 0.3290],
            max_luminance: 500.0,
            min_luminance: 0.0,
            max_content_light_level: 500.0,
            max_frame_average_light_level: 200.0,
        }
    }
}
