//! Render Target Types for Lumina
//!
//! This module provides render target configuration types for framebuffer
//! and rendering attachment setup.

// ============================================================================
// Render Target Handle
// ============================================================================

/// Render target handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderTargetHandle(pub u64);

impl RenderTargetHandle {
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

impl Default for RenderTargetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Render Target Configuration
// ============================================================================

/// Render target configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderTargetConfig {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Layers (for array/cubemap)
    pub layers: u32,
    /// Sample count
    pub samples: SampleCount,
    /// Color attachments
    pub color_attachments: [ColorAttachmentConfig; 8],
    /// Number of color attachments
    pub color_attachment_count: u32,
    /// Depth-stencil attachment
    pub depth_stencil: Option<DepthStencilAttachmentConfig>,
    /// Fragment shading rate attachment
    pub shading_rate: Option<ShadingRateAttachmentConfig>,
}

impl RenderTargetConfig {
    /// Creates new render target config
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            layers: 1,
            samples: SampleCount::S1,
            color_attachments: [ColorAttachmentConfig::empty(); 8],
            color_attachment_count: 0,
            depth_stencil: None,
            shading_rate: None,
        }
    }

    /// Common resolutions
    pub const HD_720P: Self = Self::new(1280, 720);
    pub const FULL_HD: Self = Self::new(1920, 1080);
    pub const QHD_1440P: Self = Self::new(2560, 1440);
    pub const UHD_4K: Self = Self::new(3840, 2160);
    pub const UHD_8K: Self = Self::new(7680, 4320);

    /// With layers
    #[inline]
    pub const fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }

    /// With sample count
    #[inline]
    pub const fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// MSAA 2x
    #[inline]
    pub const fn msaa_2x(mut self) -> Self {
        self.samples = SampleCount::S2;
        self
    }

    /// MSAA 4x
    #[inline]
    pub const fn msaa_4x(mut self) -> Self {
        self.samples = SampleCount::S4;
        self
    }

    /// MSAA 8x
    #[inline]
    pub const fn msaa_8x(mut self) -> Self {
        self.samples = SampleCount::S8;
        self
    }

    /// With color attachment
    pub fn with_color(mut self, config: ColorAttachmentConfig) -> Self {
        if self.color_attachment_count < 8 {
            self.color_attachments[self.color_attachment_count as usize] = config;
            self.color_attachment_count += 1;
        }
        self
    }

    /// With depth-stencil attachment
    #[inline]
    pub const fn with_depth_stencil(mut self, config: DepthStencilAttachmentConfig) -> Self {
        self.depth_stencil = Some(config);
        self
    }

    /// With shading rate attachment
    #[inline]
    pub const fn with_shading_rate(mut self, config: ShadingRateAttachmentConfig) -> Self {
        self.shading_rate = Some(config);
        self
    }

    /// Pixel count
    #[inline]
    pub const fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.layers as u64
    }

    /// Aspect ratio
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Is multisampled
    #[inline]
    pub const fn is_multisampled(&self) -> bool {
        !matches!(self.samples, SampleCount::S1)
    }
}

impl Default for RenderTargetConfig {
    fn default() -> Self {
        Self::FULL_HD
    }
}

// ============================================================================
// Sample Count
// ============================================================================

/// Sample count for multisampling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleCount {
    /// 1 sample (no MSAA)
    #[default]
    S1  = 1,
    /// 2 samples
    S2  = 2,
    /// 4 samples
    S4  = 4,
    /// 8 samples
    S8  = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

impl SampleCount {
    /// Value as u32
    #[inline]
    pub const fn value(&self) -> u32 {
        *self as u32
    }

    /// Is multisampled
    #[inline]
    pub const fn is_multisampled(&self) -> bool {
        !matches!(self, Self::S1)
    }

    /// From count
    #[inline]
    pub const fn from_count(count: u32) -> Self {
        match count {
            1 => Self::S1,
            2 => Self::S2,
            4 => Self::S4,
            8 => Self::S8,
            16 => Self::S16,
            32 => Self::S32,
            64 => Self::S64,
            _ => Self::S1,
        }
    }
}

// ============================================================================
// Color Attachment Configuration
// ============================================================================

/// Color attachment configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorAttachmentConfig {
    /// Format
    pub format: ColorFormat,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Clear color (if load_op is Clear)
    pub clear_value: ClearColorValue,
    /// Blend state
    pub blend_state: Option<BlendState>,
    /// Resolve attachment (for MSAA)
    pub resolve_mode: ResolveMode,
}

impl ColorAttachmentConfig {
    /// Empty attachment
    pub const fn empty() -> Self {
        Self {
            format: ColorFormat::Undefined,
            load_op: LoadOp::DontCare,
            store_op: StoreOp::DontCare,
            clear_value: ClearColorValue::ZERO,
            blend_state: None,
            resolve_mode: ResolveMode::None,
        }
    }

    /// Creates new color attachment
    #[inline]
    pub const fn new(format: ColorFormat) -> Self {
        Self {
            format,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: ClearColorValue::ZERO,
            blend_state: None,
            resolve_mode: ResolveMode::None,
        }
    }

    /// RGBA8 sRGB
    pub const fn rgba8_srgb() -> Self {
        Self::new(ColorFormat::Rgba8Srgb)
    }

    /// BGRA8 sRGB (common for swapchain)
    pub const fn bgra8_srgb() -> Self {
        Self::new(ColorFormat::Bgra8Srgb)
    }

    /// RGBA16 Float (HDR)
    pub const fn rgba16_float() -> Self {
        Self::new(ColorFormat::Rgba16Float)
    }

    /// RGBA32 Float (HDR/compute)
    pub const fn rgba32_float() -> Self {
        Self::new(ColorFormat::Rgba32Float)
    }

    /// RGB10A2 (HDR10)
    pub const fn rgb10a2() -> Self {
        Self::new(ColorFormat::Rgb10a2Unorm)
    }

    /// With load operation
    #[inline]
    pub const fn with_load_op(mut self, op: LoadOp) -> Self {
        self.load_op = op;
        self
    }

    /// With store operation
    #[inline]
    pub const fn with_store_op(mut self, op: StoreOp) -> Self {
        self.store_op = op;
        self
    }

    /// Clear with color
    #[inline]
    pub const fn clear_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.load_op = LoadOp::Clear;
        self.clear_value = ClearColorValue::float(r, g, b, a);
        self
    }

    /// Load existing content
    #[inline]
    pub const fn load(mut self) -> Self {
        self.load_op = LoadOp::Load;
        self
    }

    /// Don't store (transient)
    #[inline]
    pub const fn dont_store(mut self) -> Self {
        self.store_op = StoreOp::DontCare;
        self
    }

    /// With blend state
    #[inline]
    pub const fn with_blend(mut self, blend: BlendState) -> Self {
        self.blend_state = Some(blend);
        self
    }

    /// With resolve mode
    #[inline]
    pub const fn with_resolve(mut self, mode: ResolveMode) -> Self {
        self.resolve_mode = mode;
        self
    }

    /// Is HDR format
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        matches!(
            self.format,
            ColorFormat::Rgba16Float
                | ColorFormat::Rgba32Float
                | ColorFormat::Rgb10a2Unorm
                | ColorFormat::Rg11b10Float
        )
    }
}

impl Default for ColorAttachmentConfig {
    fn default() -> Self {
        Self::bgra8_srgb()
    }
}

/// Color format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorFormat {
    /// Undefined
    #[default]
    Undefined    = 0,
    /// RGBA8 UNorm
    Rgba8Unorm   = 1,
    /// RGBA8 sRGB
    Rgba8Srgb    = 2,
    /// BGRA8 UNorm
    Bgra8Unorm   = 3,
    /// BGRA8 sRGB
    Bgra8Srgb    = 4,
    /// RGBA16 Float
    Rgba16Float  = 5,
    /// RGBA32 Float
    Rgba32Float  = 6,
    /// RGB10A2 UNorm
    Rgb10a2Unorm = 7,
    /// RG11B10 Float
    Rg11b10Float = 8,
    /// R8 UNorm
    R8Unorm      = 9,
    /// RG8 UNorm
    Rg8Unorm     = 10,
    /// R16 Float
    R16Float     = 11,
    /// RG16 Float
    Rg16Float    = 12,
    /// R32 Float
    R32Float     = 13,
    /// RG32 Float
    Rg32Float    = 14,
}

impl ColorFormat {
    /// Bits per pixel
    #[inline]
    pub const fn bits_per_pixel(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm => 8,
            Self::Rg8Unorm | Self::R16Float => 16,
            Self::Rgba8Unorm
            | Self::Rgba8Srgb
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rg16Float
            | Self::R32Float
            | Self::Rgb10a2Unorm
            | Self::Rg11b10Float => 32,
            Self::Rgba16Float | Self::Rg32Float => 64,
            Self::Rgba32Float => 128,
        }
    }

    /// Is sRGB
    #[inline]
    pub const fn is_srgb(&self) -> bool {
        matches!(self, Self::Rgba8Srgb | Self::Bgra8Srgb)
    }

    /// Is HDR
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        matches!(
            self,
            Self::Rgba16Float
                | Self::Rgba32Float
                | Self::Rgb10a2Unorm
                | Self::Rg11b10Float
                | Self::R16Float
                | Self::Rg16Float
                | Self::R32Float
                | Self::Rg32Float
        )
    }
}

// ============================================================================
// Depth-Stencil Attachment Configuration
// ============================================================================

/// Depth-stencil attachment configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilAttachmentConfig {
    /// Format
    pub format: DepthStencilFormat,
    /// Depth load operation
    pub depth_load_op: LoadOp,
    /// Depth store operation
    pub depth_store_op: StoreOp,
    /// Stencil load operation
    pub stencil_load_op: LoadOp,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Clear depth value
    pub clear_depth: f32,
    /// Clear stencil value
    pub clear_stencil: u32,
    /// Read-only depth
    pub depth_read_only: bool,
    /// Read-only stencil
    pub stencil_read_only: bool,
}

impl DepthStencilAttachmentConfig {
    /// Creates new depth-stencil attachment
    #[inline]
    pub const fn new(format: DepthStencilFormat) -> Self {
        Self {
            format,
            depth_load_op: LoadOp::Clear,
            depth_store_op: StoreOp::Store,
            stencil_load_op: LoadOp::Clear,
            stencil_store_op: StoreOp::Store,
            clear_depth: 1.0,
            clear_stencil: 0,
            depth_read_only: false,
            stencil_read_only: false,
        }
    }

    /// D32 Float
    pub const fn d32_float() -> Self {
        Self::new(DepthStencilFormat::D32Float)
    }

    /// D24 S8
    pub const fn d24_s8() -> Self {
        Self::new(DepthStencilFormat::D24S8)
    }

    /// D32 S8
    pub const fn d32_s8() -> Self {
        Self::new(DepthStencilFormat::D32FloatS8)
    }

    /// D16 UNorm
    pub const fn d16() -> Self {
        Self::new(DepthStencilFormat::D16Unorm)
    }

    /// With clear values
    #[inline]
    pub const fn with_clear(mut self, depth: f32, stencil: u32) -> Self {
        self.clear_depth = depth;
        self.clear_stencil = stencil;
        self
    }

    /// Depth-only (no stencil)
    #[inline]
    pub const fn depth_only(mut self) -> Self {
        self.stencil_load_op = LoadOp::DontCare;
        self.stencil_store_op = StoreOp::DontCare;
        self
    }

    /// Read-only depth
    #[inline]
    pub const fn read_only(mut self) -> Self {
        self.depth_read_only = true;
        self.stencil_read_only = true;
        self.depth_load_op = LoadOp::Load;
        self.depth_store_op = StoreOp::DontCare;
        self.stencil_load_op = LoadOp::Load;
        self.stencil_store_op = StoreOp::DontCare;
        self
    }

    /// Has stencil
    #[inline]
    pub const fn has_stencil(&self) -> bool {
        self.format.has_stencil()
    }
}

impl Default for DepthStencilAttachmentConfig {
    fn default() -> Self {
        Self::d32_float()
    }
}

/// Depth-stencil format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DepthStencilFormat {
    /// D16 UNorm
    D16Unorm   = 0,
    /// D24 UNorm (packed with X8)
    D24Unorm   = 1,
    /// D32 Float
    #[default]
    D32Float   = 2,
    /// D24 UNorm S8 UInt
    D24S8      = 3,
    /// D32 Float S8 UInt
    D32FloatS8 = 4,
    /// S8 UInt
    S8Uint     = 5,
}

impl DepthStencilFormat {
    /// Has stencil component
    #[inline]
    pub const fn has_stencil(&self) -> bool {
        matches!(self, Self::D24S8 | Self::D32FloatS8 | Self::S8Uint)
    }

    /// Has depth component
    #[inline]
    pub const fn has_depth(&self) -> bool {
        !matches!(self, Self::S8Uint)
    }

    /// Depth bits
    #[inline]
    pub const fn depth_bits(&self) -> u32 {
        match self {
            Self::D16Unorm => 16,
            Self::D24Unorm | Self::D24S8 => 24,
            Self::D32Float | Self::D32FloatS8 => 32,
            Self::S8Uint => 0,
        }
    }

    /// Stencil bits
    #[inline]
    pub const fn stencil_bits(&self) -> u32 {
        match self {
            Self::D24S8 | Self::D32FloatS8 | Self::S8Uint => 8,
            _ => 0,
        }
    }
}

// ============================================================================
// Shading Rate Attachment Configuration
// ============================================================================

/// Shading rate attachment configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShadingRateAttachmentConfig {
    /// Texel size (tile size for shading rate)
    pub texel_size: ShadingRateTexelSize,
}

impl ShadingRateAttachmentConfig {
    /// Creates new shading rate attachment
    #[inline]
    pub const fn new(texel_size: ShadingRateTexelSize) -> Self {
        Self { texel_size }
    }

    /// 8x8 texel size
    pub const fn size_8x8() -> Self {
        Self::new(ShadingRateTexelSize::SIZE_8X8)
    }

    /// 16x16 texel size
    pub const fn size_16x16() -> Self {
        Self::new(ShadingRateTexelSize::SIZE_16X16)
    }

    /// 32x32 texel size
    pub const fn size_32x32() -> Self {
        Self::new(ShadingRateTexelSize::SIZE_32X32)
    }
}

impl Default for ShadingRateAttachmentConfig {
    fn default() -> Self {
        Self::size_16x16()
    }
}

/// Shading rate texel size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct ShadingRateTexelSize {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl ShadingRateTexelSize {
    /// 8x8 texel size
    pub const SIZE_8X8: Self = Self {
        width: 8,
        height: 8,
    };
    /// 16x16 texel size
    pub const SIZE_16X16: Self = Self {
        width: 16,
        height: 16,
    };
    /// 32x32 texel size
    pub const SIZE_32X32: Self = Self {
        width: 32,
        height: 32,
    };
    /// 8x16 texel size
    pub const SIZE_8X16: Self = Self {
        width: 8,
        height: 16,
    };
    /// 16x8 texel size
    pub const SIZE_16X8: Self = Self {
        width: 16,
        height: 8,
    };

    /// Creates new texel size
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

// ============================================================================
// Operations
// ============================================================================

/// Load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LoadOp {
    /// Load existing content
    Load     = 0,
    /// Clear to value
    #[default]
    Clear    = 1,
    /// Don't care (undefined content)
    DontCare = 2,
    /// None (for VK_EXT_load_store_op_none)
    None     = 3,
}

impl LoadOp {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Load => "Load",
            Self::Clear => "Clear",
            Self::DontCare => "Don't Care",
            Self::None => "None",
        }
    }
}

/// Store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StoreOp {
    /// Store content
    #[default]
    Store    = 0,
    /// Don't care (content discarded)
    DontCare = 1,
    /// None (for VK_EXT_load_store_op_none)
    None     = 2,
}

impl StoreOp {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Store => "Store",
            Self::DontCare => "Don't Care",
            Self::None => "None",
        }
    }
}

/// Resolve mode for MSAA
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ResolveMode {
    /// No resolve
    #[default]
    None       = 0,
    /// Sample 0
    SampleZero = 1,
    /// Average
    Average    = 2,
    /// Minimum
    Min        = 3,
    /// Maximum
    Max        = 4,
}

impl ResolveMode {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::SampleZero => "Sample Zero",
            Self::Average => "Average",
            Self::Min => "Minimum",
            Self::Max => "Maximum",
        }
    }
}

// ============================================================================
// Clear Values
// ============================================================================

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Int32 values
    pub int32: [i32; 4],
    /// Uint32 values
    pub uint32: [u32; 4],
}

impl ClearColorValue {
    /// Zero (black, transparent)
    pub const ZERO: Self = Self {
        float32: [0.0, 0.0, 0.0, 0.0],
    };

    /// White opaque
    pub const WHITE: Self = Self {
        float32: [1.0, 1.0, 1.0, 1.0],
    };

    /// Black opaque
    pub const BLACK: Self = Self {
        float32: [0.0, 0.0, 0.0, 1.0],
    };

    /// Creates from float values
    #[inline]
    pub const fn float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            float32: [r, g, b, a],
        }
    }

    /// Creates from int values
    #[inline]
    pub const fn int(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            int32: [r, g, b, a],
        }
    }

    /// Creates from uint values
    #[inline]
    pub const fn uint(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            uint32: [r, g, b, a],
        }
    }
}

impl Default for ClearColorValue {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Clear depth-stencil value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth value
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

impl ClearDepthStencilValue {
    /// Default clear (depth 1.0, stencil 0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };

    /// Reverse-Z clear (depth 0.0)
    pub const REVERSE_Z: Self = Self {
        depth: 0.0,
        stencil: 0,
    };

    /// Creates new clear value
    #[inline]
    pub const fn new(depth: f32, stencil: u32) -> Self {
        Self { depth, stencil }
    }
}

impl Default for ClearDepthStencilValue {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Blend State
// ============================================================================

/// Blend state configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BlendState {
    /// Enable blending
    pub enabled: bool,
    /// Source color factor
    pub src_color: BlendFactor,
    /// Destination color factor
    pub dst_color: BlendFactor,
    /// Color blend operation
    pub color_op: BlendOp,
    /// Source alpha factor
    pub src_alpha: BlendFactor,
    /// Destination alpha factor
    pub dst_alpha: BlendFactor,
    /// Alpha blend operation
    pub alpha_op: BlendOp,
    /// Write mask
    pub write_mask: ColorWriteMask,
}

impl BlendState {
    /// Disabled blending
    pub const DISABLED: Self = Self {
        enabled: false,
        src_color: BlendFactor::One,
        dst_color: BlendFactor::Zero,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::Zero,
        alpha_op: BlendOp::Add,
        write_mask: ColorWriteMask::ALL,
    };

    /// Alpha blending (standard)
    pub const ALPHA: Self = Self {
        enabled: true,
        src_color: BlendFactor::SrcAlpha,
        dst_color: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
        write_mask: ColorWriteMask::ALL,
    };

    /// Premultiplied alpha
    pub const PREMULTIPLIED_ALPHA: Self = Self {
        enabled: true,
        src_color: BlendFactor::One,
        dst_color: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
        write_mask: ColorWriteMask::ALL,
    };

    /// Additive blending
    pub const ADDITIVE: Self = Self {
        enabled: true,
        src_color: BlendFactor::SrcAlpha,
        dst_color: BlendFactor::One,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::Zero,
        dst_alpha: BlendFactor::One,
        alpha_op: BlendOp::Add,
        write_mask: ColorWriteMask::ALL,
    };

    /// Multiply blending
    pub const MULTIPLY: Self = Self {
        enabled: true,
        src_color: BlendFactor::DstColor,
        dst_color: BlendFactor::Zero,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::DstAlpha,
        dst_alpha: BlendFactor::Zero,
        alpha_op: BlendOp::Add,
        write_mask: ColorWriteMask::ALL,
    };

    /// With write mask
    #[inline]
    pub const fn with_write_mask(mut self, mask: ColorWriteMask) -> Self {
        self.write_mask = mask;
        self
    }
}

impl Default for BlendState {
    fn default() -> Self {
        Self::DISABLED
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendFactor {
    /// 0
    Zero              = 0,
    /// 1
    #[default]
    One               = 1,
    /// Source color
    SrcColor          = 2,
    /// 1 - Source color
    OneMinusSrcColor  = 3,
    /// Destination color
    DstColor          = 4,
    /// 1 - Destination color
    OneMinusDstColor  = 5,
    /// Source alpha
    SrcAlpha          = 6,
    /// 1 - Source alpha
    OneMinusSrcAlpha  = 7,
    /// Destination alpha
    DstAlpha          = 8,
    /// 1 - Destination alpha
    OneMinusDstAlpha  = 9,
    /// Constant color
    ConstantColor     = 10,
    /// 1 - Constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha     = 12,
    /// 1 - Constant alpha
    OneMinusConstantAlpha = 13,
    /// Source alpha saturated
    SrcAlphaSaturate  = 14,
    /// Second source color
    Src1Color         = 15,
    /// 1 - Second source color
    OneMinusSrc1Color = 16,
    /// Second source alpha
    Src1Alpha         = 17,
    /// 1 - Second source alpha
    OneMinusSrc1Alpha = 18,
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendOp {
    /// Add
    #[default]
    Add             = 0,
    /// Subtract
    Subtract        = 1,
    /// Reverse subtract
    ReverseSubtract = 2,
    /// Minimum
    Min             = 3,
    /// Maximum
    Max             = 4,
}

/// Color write mask
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ColorWriteMask(pub u8);

impl ColorWriteMask {
    /// None
    pub const NONE: Self = Self(0);
    /// Red
    pub const R: Self = Self(1 << 0);
    /// Green
    pub const G: Self = Self(1 << 1);
    /// Blue
    pub const B: Self = Self(1 << 2);
    /// Alpha
    pub const A: Self = Self(1 << 3);
    /// RGB
    pub const RGB: Self = Self(Self::R.0 | Self::G.0 | Self::B.0);
    /// All
    pub const ALL: Self = Self(0xF);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}
