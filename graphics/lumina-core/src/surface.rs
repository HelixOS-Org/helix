//! Window surface and swapchain management
//!
//! This module provides types for presenting to display surfaces.

use crate::compute::TextureFormat;
use crate::sync::{SemaphoreHandle, FenceHandle};
use crate::types::TextureHandle;

/// Surface handle for window presentation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SurfaceHandle(pub u64);

impl SurfaceHandle {
    /// Null/invalid surface
    pub const NULL: Self = Self(0);

    /// Creates a surface handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Swapchain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwapchainHandle(pub u64);

impl SwapchainHandle {
    /// Null/invalid swapchain
    pub const NULL: Self = Self(0);

    /// Creates a swapchain handle from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if handle is valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Surface capabilities
#[derive(Clone, Debug)]
pub struct SurfaceCapabilities {
    /// Minimum image count
    pub min_image_count: u32,
    /// Maximum image count (0 = unlimited)
    pub max_image_count: u32,
    /// Current extent
    pub current_extent: SurfaceExtent,
    /// Minimum extent
    pub min_extent: SurfaceExtent,
    /// Maximum extent
    pub max_extent: SurfaceExtent,
    /// Maximum image array layers
    pub max_image_array_layers: u32,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
    /// Current transform
    pub current_transform: SurfaceTransform,
    /// Supported composite alpha modes
    pub supported_composite_alpha: CompositeAlphaFlags,
    /// Supported usage flags
    pub supported_usage: TextureUsageFlags,
}

impl SurfaceCapabilities {
    /// Returns the optimal image count
    pub fn optimal_image_count(&self) -> u32 {
        let preferred = 3; // Triple buffering
        if self.max_image_count == 0 {
            self.min_image_count.max(preferred)
        } else {
            preferred.clamp(self.min_image_count, self.max_image_count)
        }
    }
}

/// Surface extent
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct SurfaceExtent {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl SurfaceExtent {
    /// Creates a surface extent
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Clamps extent to min/max bounds
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            width: if self.width < min.width {
                min.width
            } else if self.width > max.width {
                max.width
            } else {
                self.width
            },
            height: if self.height < min.height {
                min.height
            } else if self.height > max.height {
                max.height
            } else {
                self.height
            },
        }
    }
}

/// Surface transform
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceTransform {
    /// Identity (no transform)
    #[default]
    Identity,
    /// Rotate 90 degrees clockwise
    Rotate90,
    /// Rotate 180 degrees
    Rotate180,
    /// Rotate 270 degrees clockwise
    Rotate270,
    /// Horizontal mirror
    HorizontalMirror,
    /// Horizontal mirror then rotate 90
    HorizontalMirrorRotate90,
    /// Horizontal mirror then rotate 180
    HorizontalMirrorRotate180,
    /// Horizontal mirror then rotate 270
    HorizontalMirrorRotate270,
    /// Inherit from display/compositor
    Inherit,
}

/// Surface transform flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct SurfaceTransformFlags(pub u32);

impl SurfaceTransformFlags {
    /// Identity transform
    pub const IDENTITY: Self = Self(1 << 0);
    /// Rotate 90
    pub const ROTATE_90: Self = Self(1 << 1);
    /// Rotate 180
    pub const ROTATE_180: Self = Self(1 << 2);
    /// Rotate 270
    pub const ROTATE_270: Self = Self(1 << 3);
    /// Horizontal mirror
    pub const HORIZONTAL_MIRROR: Self = Self(1 << 4);
    /// Horizontal mirror + rotate 90
    pub const HORIZONTAL_MIRROR_ROTATE_90: Self = Self(1 << 5);
    /// Horizontal mirror + rotate 180
    pub const HORIZONTAL_MIRROR_ROTATE_180: Self = Self(1 << 6);
    /// Horizontal mirror + rotate 270
    pub const HORIZONTAL_MIRROR_ROTATE_270: Self = Self(1 << 7);
    /// Inherit
    pub const INHERIT: Self = Self(1 << 8);
}

impl core::ops::BitOr for SurfaceTransformFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for SurfaceTransformFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Composite alpha mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompositeAlpha {
    /// Opaque (ignore alpha)
    #[default]
    Opaque,
    /// Pre-multiplied alpha
    PreMultiplied,
    /// Post-multiplied alpha
    PostMultiplied,
    /// Inherit alpha mode
    Inherit,
}

/// Composite alpha flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl core::ops::BitOr for CompositeAlphaFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for CompositeAlphaFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Texture usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct TextureUsageFlags(pub u32);

impl TextureUsageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Sampled in shader
    pub const SAMPLED: Self = Self(1 << 2);
    /// Storage image
    pub const STORAGE: Self = Self(1 << 3);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 4);
    /// Depth/stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 5);
    /// Transient attachment
    pub const TRANSIENT_ATTACHMENT: Self = Self(1 << 6);
    /// Input attachment
    pub const INPUT_ATTACHMENT: Self = Self(1 << 7);

    /// Typical usage for render targets
    pub const RENDER_TARGET: Self = Self(Self::COLOR_ATTACHMENT.0 | Self::SAMPLED.0);

    /// Typical usage for depth buffers
    pub const DEPTH_BUFFER: Self = Self(Self::DEPTH_STENCIL_ATTACHMENT.0 | Self::SAMPLED.0);

    /// Checks if flag is set
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}

impl core::ops::BitOr for TextureUsageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for TextureUsageFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Present mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresentMode {
    /// Immediate (no vsync, may tear)
    Immediate,
    /// Mailbox (triple buffering, replace queued image)
    Mailbox,
    /// FIFO (vsync, queue images)
    #[default]
    Fifo,~5μs 	200ns
    /// FIFO relaxed (vsync, may skip if late)
    FifoRelaxed,
}

/// Surface format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct SurfaceFormat {
    /// Texture format
    pub format: TextureFormat,
    /// Color space
    pub color_space: ColorSpace,
}

impl SurfaceFormat {
    /// BGRA8 sRGB format (most common)
    pub const BGRA8_SRGB: Self = Self {
        format: TextureFormat::Bgra8UnormSrgb,
        color_space: ColorSpace::SrgbNonLinear,
    };

    /// RGBA8 sRGB format
    pub const RGBA8_SRGB: Self = Self {
        format: TextureFormat::Rgba8UnormSrgb,
        color_space: ColorSpace::SrgbNonLinear,
    };

    /// BGRA8 linear format
    pub const BGRA8_LINEAR: Self = Self {
        format: TextureFormat::Bgra8Unorm,
        color_space: ColorSpace::SrgbNonLinear,
    };
}

/// Color space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// sRGB non-linear color space
    #[default]
    SrgbNonLinear,
    /// Display P3 non-linear
    DisplayP3NonLinear,
    /// Extended sRGB linear
    ExtendedSrgbLinear,
    /// Display P3 linear
    DisplayP3Linear,
    /// DCI-P3 non-linear
    DciP3NonLinear,
    /// BT709 linear
    Bt709Linear,
    /// BT709 non-linear
    Bt709NonLinear,
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
    /// Adobe RGB non-linear
    AdobeRgbNonLinear,
    /// Pass-through (no color space conversion)
    PassThrough,
}

/// Swapchain descriptor
#[derive(Clone, Debug)]
pub struct SwapchainDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Surface handle
    pub surface: SurfaceHandle,
    /// Minimum image count
    pub min_image_count: u32,
    /// Image format
    pub format: SurfaceFormat,
    /// Image extent
    pub extent: SurfaceExtent,
    /// Image array layers
    pub array_layers: u32,
    /// Image usage
    pub usage: TextureUsageFlags,
    /// Present mode
    pub present_mode: PresentMode,
    /// Surface transform
    pub transform: SurfaceTransform,
    /// Composite alpha mode
    pub composite_alpha: CompositeAlpha,
    /// Clipped (pixels may be obscured by other windows)
    pub clipped: bool,
    /// Old swapchain to recycle resources from
    pub old_swapchain: SwapchainHandle,
}

impl<'a> SwapchainDesc<'a> {
    /// Creates a swapchain descriptor with common defaults
    pub const fn new(surface: SurfaceHandle, extent: SurfaceExtent) -> Self {
        Self {
            label: None,
            surface,
            min_image_count: 3, // Triple buffering
            format: SurfaceFormat::BGRA8_SRGB,
            extent,
            array_layers: 1,
            usage: TextureUsageFlags::COLOR_ATTACHMENT,
            present_mode: PresentMode::Fifo,
            transform: SurfaceTransform::Identity,
            composite_alpha: CompositeAlpha::Opaque,
            clipped: true,
            old_swapchain: SwapchainHandle::NULL,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the format
    pub const fn with_format(mut self, format: SurfaceFormat) -> Self {
        self.format = format;
        self
    }

    /// Sets the present mode
    pub const fn with_present_mode(mut self, present_mode: PresentMode) -> Self {
        self.present_mode = present_mode;
        self
    }

    /// Sets the min image count
    pub const fn with_min_image_count(mut self, count: u32) -> Self {
        self.min_image_count = count;
        self
    }

    /// Sets the old swapchain for resource recycling
    pub const fn with_old_swapchain(mut self, old: SwapchainHandle) -> Self {
        self.old_swapchain = old;
        self
    }

    /// Adds sampled usage for post-processing
    pub const fn with_sampled(mut self) -> Self {
        self.usage = TextureUsageFlags(self.usage.0 | TextureUsageFlags::SAMPLED.0);
        self
    }
}

/// Acquire next image result
#[derive(Clone, Copy, Debug)]
pub enum AcquireResult {
    /// Successfully acquired image
    Success {
        /// Image index
        index: u32,
        /// Suboptimal (should recreate swapchain)
        suboptimal: bool,
    },
    /// Timeout expired
    Timeout,
    /// Not ready
    NotReady,
    /// Swapchain out of date (must recreate)
    OutOfDate,
    /// Surface lost (must recreate surface)
    SurfaceLost,
}

impl AcquireResult {
    /// Returns the image index if successful
    pub const fn index(self) -> Option<u32> {
        match self {
            Self::Success { index, .. } => Some(index),
            _ => None,
        }
    }

    /// Checks if swapchain needs recreation
    pub const fn needs_recreation(self) -> bool {
        matches!(self, Self::OutOfDate | Self::SurfaceLost)
    }

    /// Checks if result indicates suboptimal state
    pub const fn is_suboptimal(self) -> bool {
        matches!(self, Self::Success { suboptimal: true, .. })
    }
}

/// Acquire next image info
#[derive(Clone, Copy, Debug)]
pub struct AcquireInfo {
    /// Swapchain handle
    pub swapchain: SwapchainHandle,
    /// Timeout in nanoseconds (u64::MAX = infinite)
    pub timeout: u64,
    /// Semaphore to signal when image is available
    pub semaphore: SemaphoreHandle,
    /// Fence to signal when image is available
    pub fence: FenceHandle,
}

impl AcquireInfo {
    /// Creates acquire info with semaphore signaling
    pub const fn with_semaphore(swapchain: SwapchainHandle, semaphore: SemaphoreHandle) -> Self {
        Self {
            swapchain,
            timeout: u64::MAX,
            semaphore,
            fence: FenceHandle::NULL,
        }
    }

    /// Creates acquire info with fence signaling
    pub const fn with_fence(swapchain: SwapchainHandle, fence: FenceHandle) -> Self {
        Self {
            swapchain,
            timeout: u64::MAX,
            semaphore: SemaphoreHandle::NULL,
            fence,
        }
    }

    /// Sets the timeout
    pub const fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout = timeout_ns;
        self
    }
}

/// Present result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentResult {
    /// Success
    Success,
    /// Suboptimal (should recreate)
    Suboptimal,
    /// Out of date (must recreate)
    OutOfDate,
    /// Surface lost (must recreate surface)
    SurfaceLost,
    /// Device lost
    DeviceLost,
}

impl PresentResult {
    /// Checks if present needs swapchain recreation
    pub const fn needs_recreation(self) -> bool {
        matches!(self, Self::OutOfDate | Self::SurfaceLost)
    }
}

/// Present info
#[derive(Clone, Debug)]
pub struct PresentInfo<'a> {
    /// Wait semaphores before present
    pub wait_semaphores: &'a [SemaphoreHandle],
    /// Swapchains to present
    pub swapchains: &'a [SwapchainHandle],
    /// Image indices for each swapchain
    pub image_indices: &'a [u32],
}

impl<'a> PresentInfo<'a> {
    /// Creates present info for a single swapchain
    pub const fn single(
        swapchain: &'a [SwapchainHandle],
        image_index: &'a [u32],
        wait_semaphore: &'a [SemaphoreHandle],
    ) -> Self {
        Self {
            wait_semaphores: wait_semaphore,
            swapchains: swapchain,
            image_indices: image_index,
        }
    }
}

/// Swapchain image
#[derive(Clone, Copy, Debug)]
pub struct SwapchainImage {
    /// Texture handle
    pub texture: TextureHandle,
    /// Image index
    pub index: u32,
}

/// Display mode properties
#[derive(Clone, Copy, Debug)]
pub struct DisplayModeProperties {
    /// Visible area width
    pub width: u32,
    /// Visible area height
    pub height: u32,
    /// Refresh rate in millihertz (e.g., 60000 for 60 Hz)
    pub refresh_rate: u32,
}

impl DisplayModeProperties {
    /// Returns refresh rate in Hz
    pub const fn refresh_rate_hz(&self) -> f32 {
        self.refresh_rate as f32 / 1000.0
    }

    /// Returns frame time in milliseconds
    pub fn frame_time_ms(&self) -> f32 {
        1000.0 / self.refresh_rate_hz()
    }
}

/// HDR metadata
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HdrMetadata {
    /// Display primary red x
    pub display_primary_red_x: f32,
    /// Display primary red y
    pub display_primary_red_y: f32,
    /// Display primary green x
    pub display_primary_green_x: f32,
    /// Display primary green y
    pub display_primary_green_y: f32,
    /// Display primary blue x
    pub display_primary_blue_x: f32,
    /// Display primary blue y
    pub display_primary_blue_y: f32,
    /// White point x
    pub white_point_x: f32,
    /// White point y
    pub white_point_y: f32,
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
    /// ST.2086 metadata for Rec. 2020 mastering display
    pub const REC2020: Self = Self {
        display_primary_red_x: 0.708,
        display_primary_red_y: 0.292,
        display_primary_green_x: 0.170,
        display_primary_green_y: 0.797,
        display_primary_blue_x: 0.131,
        display_primary_blue_y: 0.046,
        white_point_x: 0.3127,
        white_point_y: 0.3290,
        max_luminance: 1000.0,
        min_luminance: 0.001,
        max_content_light_level: 1000.0,
        max_frame_average_light_level: 400.0,
    };
}

/// Full screen exclusive mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FullScreenExclusive {
    /// Default (let driver decide)
    #[default]
    Default,
    /// Disallowed
    Disallowed,
    /// Allowed
    Allowed,
    /// Application controlled
    ApplicationControlled,
}
