//! Swapchain and presentation types
//!
//! This module provides types for swapchain management and surface presentation.

use core::num::NonZeroU32;

/// Surface handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SurfaceHandle(pub NonZeroU32);

impl SurfaceHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Swapchain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SwapchainHandle(pub NonZeroU32);

impl SwapchainHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Swapchain creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SwapchainCreateInfo {
    /// Surface to present to
    pub surface: SurfaceHandle,
    /// Minimum image count
    pub min_image_count: u32,
    /// Image format
    pub image_format: SurfaceFormat,
    /// Image extent
    pub image_extent: Extent2D,
    /// Image array layers
    pub image_array_layers: u32,
    /// Image usage
    pub image_usage: ImageUsageFlags,
    /// Image sharing mode
    pub image_sharing_mode: SharingMode,
    /// Pre-transform
    pub pre_transform: SurfaceTransformFlags,
    /// Composite alpha
    pub composite_alpha: CompositeAlphaFlags,
    /// Present mode
    pub present_mode: PresentMode,
    /// Clipped
    pub clipped: bool,
    /// Old swapchain (for recreation)
    pub old_swapchain: Option<SwapchainHandle>,
    /// Creation flags
    pub flags: SwapchainCreateFlags,
}

impl SwapchainCreateInfo {
    /// Creates basic swapchain info
    pub const fn new(surface: SurfaceHandle, width: u32, height: u32) -> Self {
        Self {
            surface,
            min_image_count: 2,
            image_format: SurfaceFormat::B8G8R8A8_SRGB,
            image_extent: Extent2D { width, height },
            image_array_layers: 1,
            image_usage: ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: SharingMode::Exclusive,
            pre_transform: SurfaceTransformFlags::IDENTITY,
            composite_alpha: CompositeAlphaFlags::OPAQUE,
            present_mode: PresentMode::Fifo,
            clipped: true,
            old_swapchain: None,
            flags: SwapchainCreateFlags::empty(),
        }
    }

    /// With triple buffering
    pub const fn with_triple_buffering(mut self) -> Self {
        self.min_image_count = 3;
        self
    }

    /// With mailbox present mode (low latency)
    pub const fn with_mailbox(mut self) -> Self {
        self.present_mode = PresentMode::Mailbox;
        self
    }

    /// With immediate present mode (no vsync)
    pub const fn with_immediate(mut self) -> Self {
        self.present_mode = PresentMode::Immediate;
        self
    }

    /// With HDR format
    pub const fn with_hdr(mut self) -> Self {
        self.image_format = SurfaceFormat::A2B10G10R10_UNORM;
        self
    }

    /// For recreation from old swapchain
    pub const fn recreate(mut self, old: SwapchainHandle) -> Self {
        self.old_swapchain = Some(old);
        self
    }
}

/// 2D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Extent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Extent2D {
    /// Creates a new extent
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// 1920x1080
    pub const FHD: Self = Self::new(1920, 1080);
    /// 2560x1440
    pub const QHD: Self = Self::new(2560, 1440);
    /// 3840x2160
    pub const UHD: Self = Self::new(3840, 2160);

    /// Aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Total pixels
    pub const fn pixel_count(&self) -> u32 {
        self.width * self.height
    }
}

/// Surface format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SurfaceFormat {
    /// B8G8R8A8 SRGB (most common)
    #[default]
    B8G8R8A8_SRGB       = 50,
    /// B8G8R8A8 UNORM
    B8G8R8A8_UNORM      = 44,
    /// R8G8B8A8 SRGB
    R8G8B8A8_SRGB       = 43,
    /// R8G8B8A8 UNORM
    R8G8B8A8_UNORM      = 37,
    /// A2B10G10R10 UNORM (HDR)
    A2B10G10R10_UNORM   = 64,
    /// R16G16B16A16 SFLOAT (HDR)
    R16G16B16A16_SFLOAT = 97,
}

impl SurfaceFormat {
    /// Is this an sRGB format
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::B8G8R8A8_SRGB | Self::R8G8B8A8_SRGB)
    }

    /// Is this an HDR format
    pub const fn is_hdr(self) -> bool {
        matches!(self, Self::A2B10G10R10_UNORM | Self::R16G16B16A16_SFLOAT)
    }

    /// Bits per pixel
    pub const fn bits_per_pixel(self) -> u32 {
        match self {
            Self::B8G8R8A8_SRGB
            | Self::B8G8R8A8_UNORM
            | Self::R8G8B8A8_SRGB
            | Self::R8G8B8A8_UNORM
            | Self::A2B10G10R10_UNORM => 32,
            Self::R16G16B16A16_SFLOAT => 64,
        }
    }
}

/// Sharing mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SharingMode {
    /// Exclusive access
    #[default]
    Exclusive  = 0,
    /// Concurrent access
    Concurrent = 1,
}

bitflags::bitflags! {
    /// Surface transform flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SurfaceTransformFlags: u32 {
        /// Identity transform
        const IDENTITY = 1 << 0;
        /// 90 degree clockwise rotation
        const ROTATE_90 = 1 << 1;
        /// 180 degree rotation
        const ROTATE_180 = 1 << 2;
        /// 270 degree clockwise rotation
        const ROTATE_270 = 1 << 3;
        /// Horizontal mirror
        const HORIZONTAL_MIRROR = 1 << 4;
        /// Horizontal mirror + 90 degree rotation
        const HORIZONTAL_MIRROR_ROTATE_90 = 1 << 5;
        /// Horizontal mirror + 180 degree rotation
        const HORIZONTAL_MIRROR_ROTATE_180 = 1 << 6;
        /// Horizontal mirror + 270 degree rotation
        const HORIZONTAL_MIRROR_ROTATE_270 = 1 << 7;
        /// Inherit from display
        const INHERIT = 1 << 8;
    }
}

impl SurfaceTransformFlags {
    /// No transformation
    pub const fn empty() -> Self {
        Self::IDENTITY
    }

    /// Is this a rotation
    pub const fn is_rotation(self) -> bool {
        self.intersects(
            Self::ROTATE_90
                .union(Self::ROTATE_180)
                .union(Self::ROTATE_270),
        )
    }

    /// Does this swap dimensions
    pub const fn swaps_dimensions(self) -> bool {
        self.intersects(
            Self::ROTATE_90
                .union(Self::ROTATE_270)
                .union(Self::HORIZONTAL_MIRROR_ROTATE_90)
                .union(Self::HORIZONTAL_MIRROR_ROTATE_270),
        )
    }
}

bitflags::bitflags! {
    /// Composite alpha flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CompositeAlphaFlags: u32 {
        /// Opaque
        const OPAQUE = 1 << 0;
        /// Pre-multiplied alpha
        const PRE_MULTIPLIED = 1 << 1;
        /// Post-multiplied alpha
        const POST_MULTIPLIED = 1 << 2;
        /// Inherit from system
        const INHERIT = 1 << 3;
    }
}

impl CompositeAlphaFlags {
    /// Default (opaque)
    pub const fn empty() -> Self {
        Self::OPAQUE
    }
}

/// Present mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PresentMode {
    /// Immediate (may tear)
    Immediate           = 0,
    /// Mailbox (low latency, no tearing)
    Mailbox             = 1,
    /// FIFO (vsync, always supported)
    #[default]
    Fifo                = 2,
    /// FIFO relaxed (vsync with late frame handling)
    FifoRelaxed         = 3,
    /// Shared demand refresh
    SharedDemandRefresh = 4,
    /// Shared continuous refresh
    SharedContinuousRefresh = 5,
}

impl PresentMode {
    /// Is this a vsync mode
    pub const fn is_vsync(self) -> bool {
        matches!(self, Self::Fifo | Self::FifoRelaxed)
    }

    /// Can this mode tear
    pub const fn can_tear(self) -> bool {
        matches!(self, Self::Immediate | Self::FifoRelaxed)
    }

    /// Is low latency
    pub const fn is_low_latency(self) -> bool {
        matches!(self, Self::Immediate | Self::Mailbox)
    }
}

bitflags::bitflags! {
    /// Image usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageUsageFlags: u32 {
        /// Transfer source
        const TRANSFER_SRC = 1 << 0;
        /// Transfer destination
        const TRANSFER_DST = 1 << 1;
        /// Sampled
        const SAMPLED = 1 << 2;
        /// Storage
        const STORAGE = 1 << 3;
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 4;
        /// Depth-stencil attachment
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Transient attachment
        const TRANSIENT_ATTACHMENT = 1 << 6;
        /// Input attachment
        const INPUT_ATTACHMENT = 1 << 7;
    }
}

impl ImageUsageFlags {
    /// Typical swapchain usage
    pub const SWAPCHAIN: Self = Self::COLOR_ATTACHMENT;
}

bitflags::bitflags! {
    /// Swapchain creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct SwapchainCreateFlags: u32 {
        /// Split instance bind regions
        const SPLIT_INSTANCE_BIND_REGIONS = 1 << 0;
        /// Protected
        const PROTECTED = 1 << 1;
        /// Mutable format
        const MUTABLE_FORMAT = 1 << 2;
    }
}

impl SwapchainCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Surface capabilities
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SurfaceCapabilities {
    /// Minimum image count
    pub min_image_count: u32,
    /// Maximum image count (0 = no limit)
    pub max_image_count: u32,
    /// Current extent
    pub current_extent: Extent2D,
    /// Minimum extent
    pub min_image_extent: Extent2D,
    /// Maximum extent
    pub max_image_extent: Extent2D,
    /// Maximum image array layers
    pub max_image_array_layers: u32,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
    /// Current transform
    pub current_transform: SurfaceTransformFlags,
    /// Supported composite alpha
    pub supported_composite_alpha: CompositeAlphaFlags,
    /// Supported usage flags
    pub supported_usage_flags: ImageUsageFlags,
}

impl SurfaceCapabilities {
    /// Gets recommended image count
    pub fn recommended_image_count(&self) -> u32 {
        let count = self.min_image_count + 1;
        if self.max_image_count > 0 {
            count.min(self.max_image_count)
        } else {
            count
        }
    }

    /// Clamps extent to valid range
    pub fn clamp_extent(&self, extent: Extent2D) -> Extent2D {
        Extent2D {
            width: extent
                .width
                .max(self.min_image_extent.width)
                .min(self.max_image_extent.width),
            height: extent
                .height
                .max(self.min_image_extent.height)
                .min(self.max_image_extent.height),
        }
    }
}

/// Present info
#[derive(Clone, Debug)]
pub struct PresentInfo {
    /// Wait semaphores
    pub wait_semaphores: alloc::vec::Vec<SemaphoreHandle>,
    /// Swapchains to present
    pub swapchains: alloc::vec::Vec<SwapchainHandle>,
    /// Image indices
    pub image_indices: alloc::vec::Vec<u32>,
    /// Results (optional, one per swapchain)
    pub results: alloc::vec::Vec<PresentResult>,
}

use alloc::vec::Vec;

/// Semaphore handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SemaphoreHandle(pub NonZeroU32);

impl SemaphoreHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

impl PresentInfo {
    /// Creates a new present info
    pub fn new() -> Self {
        Self {
            wait_semaphores: Vec::new(),
            swapchains: Vec::new(),
            image_indices: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Adds a wait semaphore
    pub fn wait_on(mut self, semaphore: SemaphoreHandle) -> Self {
        self.wait_semaphores.push(semaphore);
        self
    }

    /// Adds a swapchain to present
    pub fn add_swapchain(mut self, swapchain: SwapchainHandle, image_index: u32) -> Self {
        self.swapchains.push(swapchain);
        self.image_indices.push(image_index);
        self
    }
}

impl Default for PresentInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Present result
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum PresentResult {
    /// Success
    Success     = 0,
    /// Suboptimal
    Suboptimal  = 1,
    /// Out of date (needs recreation)
    OutOfDate   = -1,
    /// Surface lost
    SurfaceLost = -2,
    /// Full screen exclusive lost
    FullScreenExclusiveLost = -3,
}

impl PresentResult {
    /// Is this a success
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::Suboptimal)
    }

    /// Needs swapchain recreation
    pub const fn needs_recreation(self) -> bool {
        matches!(self, Self::OutOfDate | Self::SurfaceLost)
    }
}

/// Acquire next image info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AcquireNextImageInfo {
    /// Swapchain
    pub swapchain: SwapchainHandle,
    /// Timeout in nanoseconds (u64::MAX = infinite)
    pub timeout: u64,
    /// Semaphore to signal
    pub semaphore: Option<SemaphoreHandle>,
    /// Fence to signal
    pub fence: Option<FenceHandle>,
    /// Device mask
    pub device_mask: u32,
}

/// Fence handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FenceHandle(pub NonZeroU32);

impl FenceHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

impl AcquireNextImageInfo {
    /// Creates acquire info with semaphore
    pub const fn with_semaphore(swapchain: SwapchainHandle, semaphore: SemaphoreHandle) -> Self {
        Self {
            swapchain,
            timeout: u64::MAX,
            semaphore: Some(semaphore),
            fence: None,
            device_mask: 1,
        }
    }

    /// Creates acquire info with fence
    pub const fn with_fence(swapchain: SwapchainHandle, fence: FenceHandle) -> Self {
        Self {
            swapchain,
            timeout: u64::MAX,
            semaphore: None,
            fence: Some(fence),
            device_mask: 1,
        }
    }

    /// With timeout
    pub const fn with_timeout(mut self, timeout_ns: u64) -> Self {
        self.timeout = timeout_ns;
        self
    }
}

/// HDR metadata
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HdrMetadata {
    /// Display primary red
    pub display_primary_red: ChromaticityCoordinates,
    /// Display primary green
    pub display_primary_green: ChromaticityCoordinates,
    /// Display primary blue
    pub display_primary_blue: ChromaticityCoordinates,
    /// White point
    pub white_point: ChromaticityCoordinates,
    /// Max luminance (nits)
    pub max_luminance: f32,
    /// Min luminance (nits)
    pub min_luminance: f32,
    /// Max content light level (nits)
    pub max_content_light_level: f32,
    /// Max frame-average light level (nits)
    pub max_frame_average_light_level: f32,
}

/// Chromaticity coordinates
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ChromaticityCoordinates {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl HdrMetadata {
    /// sRGB display
    pub const SRGB: Self = Self {
        display_primary_red: ChromaticityCoordinates { x: 0.64, y: 0.33 },
        display_primary_green: ChromaticityCoordinates { x: 0.30, y: 0.60 },
        display_primary_blue: ChromaticityCoordinates { x: 0.15, y: 0.06 },
        white_point: ChromaticityCoordinates {
            x: 0.3127,
            y: 0.3290,
        },
        max_luminance: 100.0,
        min_luminance: 0.0,
        max_content_light_level: 100.0,
        max_frame_average_light_level: 100.0,
    };

    /// BT.2020 / HDR10
    pub const BT2020: Self = Self {
        display_primary_red: ChromaticityCoordinates { x: 0.708, y: 0.292 },
        display_primary_green: ChromaticityCoordinates { x: 0.170, y: 0.797 },
        display_primary_blue: ChromaticityCoordinates { x: 0.131, y: 0.046 },
        white_point: ChromaticityCoordinates {
            x: 0.3127,
            y: 0.3290,
        },
        max_luminance: 1000.0,
        min_luminance: 0.001,
        max_content_light_level: 1000.0,
        max_frame_average_light_level: 500.0,
    };
}

/// Full screen exclusive mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum FullScreenExclusiveMode {
    /// Default behavior
    #[default]
    Default    = 0,
    /// Allow full screen exclusive
    Allowed    = 1,
    /// Disallow full screen exclusive
    Disallowed = 2,
    /// Application controlled
    ApplicationControlled = 3,
}

/// Present scaling mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PresentScalingMode {
    /// No scaling
    #[default]
    None               = 0,
    /// Aspect ratio stretch
    AspectRatioStretch = 1,
    /// Stretch to fill
    Stretch            = 2,
}

/// Present gravity
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PresentGravity {
    /// Centered
    #[default]
    Center      = 0,
    /// Top-left
    TopLeft     = 1,
    /// Top-right
    TopRight    = 2,
    /// Bottom-left
    BottomLeft  = 3,
    /// Bottom-right
    BottomRight = 4,
}
