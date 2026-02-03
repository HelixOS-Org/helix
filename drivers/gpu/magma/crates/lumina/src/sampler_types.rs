//! Sampler types and configuration
//!
//! This module provides types for texture sampler creation and filtering modes.

use core::num::NonZeroU32;

/// Sampler handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SamplerHandle(pub NonZeroU32);

impl SamplerHandle {
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

/// Sampler creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SamplerCreateInfo {
    /// Magnification filter
    pub mag_filter: Filter,
    /// Minification filter
    pub min_filter: Filter,
    /// Mipmap mode
    pub mipmap_mode: SamplerMipmapMode,
    /// Address mode U
    pub address_mode_u: SamplerAddressMode,
    /// Address mode V
    pub address_mode_v: SamplerAddressMode,
    /// Address mode W
    pub address_mode_w: SamplerAddressMode,
    /// LOD bias
    pub mip_lod_bias: f32,
    /// Anisotropy enable
    pub anisotropy_enable: bool,
    /// Max anisotropy
    pub max_anisotropy: f32,
    /// Compare enable
    pub compare_enable: bool,
    /// Compare operation
    pub compare_op: CompareOp,
    /// Min LOD
    pub min_lod: f32,
    /// Max LOD
    pub max_lod: f32,
    /// Border color
    pub border_color: BorderColor,
    /// Unnormalized coordinates
    pub unnormalized_coordinates: bool,
    /// Reduction mode
    pub reduction_mode: SamplerReductionMode,
}

impl SamplerCreateInfo {
    /// Linear filtering sampler
    pub const fn linear() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Linear,
            address_mode_u: SamplerAddressMode::Repeat,
            address_mode_v: SamplerAddressMode::Repeat,
            address_mode_w: SamplerAddressMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Nearest (point) filtering sampler
    pub const fn nearest() -> Self {
        Self {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::Repeat,
            address_mode_v: SamplerAddressMode::Repeat,
            address_mode_w: SamplerAddressMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Anisotropic filtering sampler
    pub const fn anisotropic(max_anisotropy: f32) -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Linear,
            address_mode_u: SamplerAddressMode::Repeat,
            address_mode_v: SamplerAddressMode::Repeat,
            address_mode_w: SamplerAddressMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: true,
            max_anisotropy,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Shadow sampler with depth comparison
    pub const fn shadow() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::ClampToBorder,
            address_mode_v: SamplerAddressMode::ClampToBorder,
            address_mode_w: SamplerAddressMode::ClampToBorder,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: true,
            compare_op: CompareOp::LessOrEqual,
            min_lod: 0.0,
            max_lod: 1.0,
            border_color: BorderColor::FloatOpaqueWhite,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Clamp to edge sampler
    pub const fn clamp_to_edge() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Linear,
            address_mode_u: SamplerAddressMode::ClampToEdge,
            address_mode_v: SamplerAddressMode::ClampToEdge,
            address_mode_w: SamplerAddressMode::ClampToEdge,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Mirror repeat sampler
    pub const fn mirror_repeat() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Linear,
            address_mode_u: SamplerAddressMode::MirroredRepeat,
            address_mode_v: SamplerAddressMode::MirroredRepeat,
            address_mode_w: SamplerAddressMode::MirroredRepeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatTransparentBlack,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::WeightedAverage,
        }
    }

    /// Min/max reduction sampler (for depth pyramid)
    pub const fn min_reduction() -> Self {
        Self {
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            mipmap_mode: SamplerMipmapMode::Nearest,
            address_mode_u: SamplerAddressMode::ClampToEdge,
            address_mode_v: SamplerAddressMode::ClampToEdge,
            address_mode_w: SamplerAddressMode::ClampToEdge,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::FloatOpaqueWhite,
            unnormalized_coordinates: false,
            reduction_mode: SamplerReductionMode::Min,
        }
    }

    /// With address mode
    pub const fn with_address_mode(mut self, mode: SamplerAddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }

    /// With anisotropy
    pub const fn with_anisotropy(mut self, max: f32) -> Self {
        self.anisotropy_enable = true;
        self.max_anisotropy = max;
        self
    }

    /// With LOD range
    pub const fn with_lod_range(mut self, min: f32, max: f32) -> Self {
        self.min_lod = min;
        self.max_lod = max;
        self
    }

    /// With border color
    pub const fn with_border_color(mut self, color: BorderColor) -> Self {
        self.border_color = color;
        self
    }

    /// With compare operation
    pub const fn with_compare(mut self, op: CompareOp) -> Self {
        self.compare_enable = true;
        self.compare_op = op;
        self
    }
}

impl Default for SamplerCreateInfo {
    fn default() -> Self {
        Self::linear()
    }
}

/// Filter mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Filter {
    /// Nearest neighbor filtering
    Nearest = 0,
    /// Linear (bilinear) filtering
    #[default]
    Linear  = 1,
    /// Cubic filtering (VK_EXT_filter_cubic)
    Cubic   = 2,
}

/// Mipmap mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SamplerMipmapMode {
    /// Nearest mipmap selection
    Nearest = 0,
    /// Linear mipmap interpolation
    #[default]
    Linear  = 1,
}

/// Address mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SamplerAddressMode {
    /// Repeat the texture
    #[default]
    Repeat            = 0,
    /// Mirrored repeat
    MirroredRepeat    = 1,
    /// Clamp to edge
    ClampToEdge       = 2,
    /// Clamp to border
    ClampToBorder     = 3,
    /// Mirror once then clamp
    MirrorClampToEdge = 4,
}

impl SamplerAddressMode {
    /// Is this a clamping mode
    pub const fn is_clamping(self) -> bool {
        matches!(
            self,
            Self::ClampToEdge | Self::ClampToBorder | Self::MirrorClampToEdge
        )
    }

    /// Does this mode use border color
    pub const fn uses_border_color(self) -> bool {
        matches!(self, Self::ClampToBorder)
    }
}

/// Compare operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum CompareOp {
    /// Never pass
    #[default]
    Never          = 0,
    /// Pass if less
    Less           = 1,
    /// Pass if equal
    Equal          = 2,
    /// Pass if less or equal
    LessOrEqual    = 3,
    /// Pass if greater
    Greater        = 4,
    /// Pass if not equal
    NotEqual       = 5,
    /// Pass if greater or equal
    GreaterOrEqual = 6,
    /// Always pass
    Always         = 7,
}

impl CompareOp {
    /// Returns the reversed compare operation
    pub const fn reversed(self) -> Self {
        match self {
            Self::Never => Self::Always,
            Self::Less => Self::GreaterOrEqual,
            Self::Equal => Self::NotEqual,
            Self::LessOrEqual => Self::Greater,
            Self::Greater => Self::LessOrEqual,
            Self::NotEqual => Self::Equal,
            Self::GreaterOrEqual => Self::Less,
            Self::Always => Self::Never,
        }
    }
}

/// Border color
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BorderColor {
    /// Float transparent black (0, 0, 0, 0)
    #[default]
    FloatTransparentBlack = 0,
    /// Int transparent black (0, 0, 0, 0)
    IntTransparentBlack = 1,
    /// Float opaque black (0, 0, 0, 1)
    FloatOpaqueBlack    = 2,
    /// Int opaque black (0, 0, 0, 1)
    IntOpaqueBlack      = 3,
    /// Float opaque white (1, 1, 1, 1)
    FloatOpaqueWhite    = 4,
    /// Int opaque white (1, 1, 1, 1)
    IntOpaqueWhite      = 5,
    /// Custom float
    FloatCustom         = 6,
    /// Custom int
    IntCustom           = 7,
}

impl BorderColor {
    /// Is this a float color
    pub const fn is_float(self) -> bool {
        matches!(
            self,
            Self::FloatTransparentBlack
                | Self::FloatOpaqueBlack
                | Self::FloatOpaqueWhite
                | Self::FloatCustom
        )
    }

    /// Is this a custom color
    pub const fn is_custom(self) -> bool {
        matches!(self, Self::FloatCustom | Self::IntCustom)
    }
}

/// Sampler reduction mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SamplerReductionMode {
    /// Weighted average (standard filtering)
    #[default]
    WeightedAverage = 0,
    /// Minimum value
    Min             = 1,
    /// Maximum value
    Max             = 2,
}

/// Custom border color
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CustomBorderColor {
    /// Color value
    pub color: [f32; 4],
    /// Format
    pub format: BorderColorFormat,
}

/// Border color format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum BorderColorFormat {
    /// Float format
    #[default]
    Float = 0,
    /// Int format
    Int   = 1,
}

impl CustomBorderColor {
    /// Black color
    pub const BLACK: Self = Self {
        color: [0.0, 0.0, 0.0, 1.0],
        format: BorderColorFormat::Float,
    };

    /// White color
    pub const WHITE: Self = Self {
        color: [1.0, 1.0, 1.0, 1.0],
        format: BorderColorFormat::Float,
    };

    /// Red color
    pub const RED: Self = Self {
        color: [1.0, 0.0, 0.0, 1.0],
        format: BorderColorFormat::Float,
    };

    /// Creates a custom border color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            format: BorderColorFormat::Float,
        }
    }
}

/// Sampler Y'CbCr conversion info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SamplerYcbcrConversionInfo {
    /// Format
    pub format: YcbcrFormat,
    /// Y'CbCr model
    pub ycbcr_model: YcbcrModel,
    /// Y'CbCr range
    pub ycbcr_range: YcbcrRange,
    /// Component swizzle
    pub components: ComponentSwizzle4,
    /// X chroma offset
    pub x_chroma_offset: ChromaLocation,
    /// Y chroma offset
    pub y_chroma_offset: ChromaLocation,
    /// Chroma filter
    pub chroma_filter: Filter,
    /// Force explicit reconstruction
    pub force_explicit_reconstruction: bool,
}

/// Y'CbCr format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum YcbcrFormat {
    /// G8B8G8R8 422
    #[default]
    G8B8G8R8_422        = 1000156000,
    /// B8G8R8G8 422
    B8G8R8G8_422        = 1000156001,
    /// G8 B8 R8 420 3-plane
    G8_B8_R8_420_3Plane = 1000156002,
    /// G8 B8R8 420 2-plane
    G8_B8R8_420_2Plane  = 1000156003,
    /// G8 B8 R8 422 3-plane
    G8_B8_R8_422_3Plane = 1000156004,
    /// G8 B8R8 422 2-plane
    G8_B8R8_422_2Plane  = 1000156005,
    /// G8 B8 R8 444 3-plane
    G8_B8_R8_444_3Plane = 1000156006,
}

/// Y'CbCr model
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum YcbcrModel {
    /// RGB identity
    #[default]
    RgbIdentity   = 0,
    /// Y'CbCr identity
    YcbcrIdentity = 1,
    /// BT.709
    Ycbcr709      = 2,
    /// BT.601
    Ycbcr601      = 3,
    /// BT.2020
    Ycbcr2020     = 4,
}

/// Y'CbCr range
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum YcbcrRange {
    /// ITU full range
    #[default]
    ItuFull   = 0,
    /// ITU narrow range
    ItuNarrow = 1,
}

/// Chroma location
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ChromaLocation {
    /// Cosited even
    #[default]
    CositedEven = 0,
    /// Midpoint
    Midpoint    = 1,
}

/// Component swizzle for 4 components
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComponentSwizzle4 {
    /// Red component
    pub r: ComponentSwizzle,
    /// Green component
    pub g: ComponentSwizzle,
    /// Blue component
    pub b: ComponentSwizzle,
    /// Alpha component
    pub a: ComponentSwizzle,
}

impl ComponentSwizzle4 {
    /// Identity swizzle
    pub const IDENTITY: Self = Self {
        r: ComponentSwizzle::Identity,
        g: ComponentSwizzle::Identity,
        b: ComponentSwizzle::Identity,
        a: ComponentSwizzle::Identity,
    };
}

/// Component swizzle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ComponentSwizzle {
    /// Identity
    #[default]
    Identity = 0,
    /// Zero
    Zero     = 1,
    /// One
    One      = 2,
    /// R
    R        = 3,
    /// G
    G        = 4,
    /// B
    B        = 5,
    /// A
    A        = 6,
}

/// Immutable sampler descriptor
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImmutableSamplerDesc {
    /// Binding index
    pub binding: u32,
    /// Sampler handle
    pub sampler: SamplerHandle,
}

/// Sampler pool for managing multiple samplers
#[derive(Clone, Debug)]
pub struct SamplerPool {
    /// Linear sampler
    pub linear: Option<SamplerHandle>,
    /// Nearest sampler
    pub nearest: Option<SamplerHandle>,
    /// Anisotropic sampler
    pub anisotropic: Option<SamplerHandle>,
    /// Shadow sampler
    pub shadow: Option<SamplerHandle>,
    /// Clamp sampler
    pub clamp: Option<SamplerHandle>,
}

impl Default for SamplerPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SamplerPool {
    /// Creates a new empty sampler pool
    pub const fn new() -> Self {
        Self {
            linear: None,
            nearest: None,
            anisotropic: None,
            shadow: None,
            clamp: None,
        }
    }
}
