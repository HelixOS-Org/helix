//! Sampler configuration
//!
//! This module provides types for configuring texture samplers.

use crate::color::Color;
use crate::types::SamplerHandle;

/// Texture filtering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FilterMode {
    /// Nearest neighbor (pixelated)
    Nearest,
    /// Linear interpolation (smooth)
    #[default]
    Linear,
}

impl FilterMode {
    /// Returns the Vulkan filter
    pub const fn vk_filter(self) -> u32 {
        match self {
            Self::Nearest => 0, // VK_FILTER_NEAREST
            Self::Linear => 1,  // VK_FILTER_LINEAR
        }
    }
}

/// Texture wrapping mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum WrapMode {
    /// Repeat the texture
    #[default]
    Repeat,
    /// Mirror and repeat
    MirroredRepeat,
    /// Clamp to edge pixels
    ClampToEdge,
    /// Clamp to border color
    ClampToBorder,
}

impl WrapMode {
    /// Returns the Vulkan address mode
    pub const fn vk_address_mode(self) -> u32 {
        match self {
            Self::Repeat => 0,         // VK_SAMPLER_ADDRESS_MODE_REPEAT
            Self::MirroredRepeat => 1, // VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT
            Self::ClampToEdge => 2,    // VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE
            Self::ClampToBorder => 3,  // VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_BORDER
        }
    }
}

/// Mipmap filtering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum MipmapMode {
    /// Nearest mipmap level
    Nearest,
    /// Linear interpolation between mipmap levels
    #[default]
    Linear,
}

impl MipmapMode {
    /// Returns the Vulkan mipmap mode
    pub const fn vk_mipmap_mode(self) -> u32 {
        match self {
            Self::Nearest => 0, // VK_SAMPLER_MIPMAP_MODE_NEAREST
            Self::Linear => 1,  // VK_SAMPLER_MIPMAP_MODE_LINEAR
        }
    }
}

/// Border color for ClampToBorder mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BorderColor {
    /// Transparent black (0, 0, 0, 0)
    #[default]
    TransparentBlack,
    /// Opaque black (0, 0, 0, 1)
    OpaqueBlack,
    /// Opaque white (1, 1, 1, 1)
    OpaqueWhite,
}

impl BorderColor {
    /// Returns the Vulkan border color
    pub const fn vk_border_color(self) -> u32 {
        match self {
            Self::TransparentBlack => 0, // VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK
            Self::OpaqueBlack => 2,      // VK_BORDER_COLOR_FLOAT_OPAQUE_BLACK
            Self::OpaqueWhite => 4,      // VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE
        }
    }
}

/// Comparison function for shadow samplers
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl CompareOp {
    /// Returns the Vulkan compare op
    pub const fn vk_compare_op(self) -> u32 {
        match self {
            Self::Never => 0,
            Self::Less => 1,
            Self::Equal => 2,
            Self::LessEqual => 3,
            Self::Greater => 4,
            Self::NotEqual => 5,
            Self::GreaterEqual => 6,
            Self::Always => 7,
        }
    }
}

/// Sampler description
#[derive(Clone, Debug)]
pub struct SamplerDesc {
    /// Magnification filter
    pub mag_filter: FilterMode,
    /// Minification filter
    pub min_filter: FilterMode,
    /// Mipmap mode
    pub mipmap_mode: MipmapMode,
    /// U (horizontal) wrap mode
    pub wrap_u: WrapMode,
    /// V (vertical) wrap mode
    pub wrap_v: WrapMode,
    /// W (depth) wrap mode
    pub wrap_w: WrapMode,
    /// Mip LOD bias
    pub mip_lod_bias: f32,
    /// Enable anisotropic filtering
    pub anisotropy_enable: bool,
    /// Maximum anisotropy level
    pub max_anisotropy: f32,
    /// Enable comparison mode (for shadow maps)
    pub compare_enable: bool,
    /// Comparison function
    pub compare_op: CompareOp,
    /// Minimum LOD
    pub min_lod: f32,
    /// Maximum LOD
    pub max_lod: f32,
    /// Border color
    pub border_color: BorderColor,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Linear,
            wrap_u: WrapMode::Repeat,
            wrap_v: WrapMode::Repeat,
            wrap_w: WrapMode::Repeat,
            mip_lod_bias: 0.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            min_lod: 0.0,
            max_lod: 1000.0,
            border_color: BorderColor::TransparentBlack,
        }
    }
}

impl SamplerDesc {
    /// Creates a sampler for nearest-neighbor filtering
    pub fn nearest() -> Self {
        Self {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_mode: MipmapMode::Nearest,
            ..Default::default()
        }
    }

    /// Creates a sampler for linear filtering
    pub fn linear() -> Self {
        Self::default()
    }

    /// Creates a sampler for shadow map sampling
    pub fn shadow() -> Self {
        Self {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Nearest,
            wrap_u: WrapMode::ClampToBorder,
            wrap_v: WrapMode::ClampToBorder,
            wrap_w: WrapMode::ClampToBorder,
            compare_enable: true,
            compare_op: CompareOp::LessEqual,
            border_color: BorderColor::OpaqueWhite,
            ..Default::default()
        }
    }

    /// Sets anisotropic filtering
    pub fn with_anisotropy(mut self, level: f32) -> Self {
        self.anisotropy_enable = true;
        self.max_anisotropy = level;
        self
    }

    /// Sets the wrap mode for all axes
    pub fn with_wrap(mut self, mode: WrapMode) -> Self {
        self.wrap_u = mode;
        self.wrap_v = mode;
        self.wrap_w = mode;
        self
    }
}

/// A GPU sampler
pub struct Sampler {
    handle: SamplerHandle,
    desc: SamplerDesc,
}

impl Sampler {
    /// Creates a new sampler with the given description
    pub fn new(desc: SamplerDesc) -> Self {
        Self {
            handle: SamplerHandle::null(),
            desc,
        }
    }

    /// Creates a nearest-neighbor sampler
    pub fn nearest() -> Self {
        Self::new(SamplerDesc::nearest())
    }

    /// Creates a linear sampler
    pub fn linear() -> Self {
        Self::new(SamplerDesc::linear())
    }

    /// Creates a shadow map sampler
    pub fn shadow() -> Self {
        Self::new(SamplerDesc::shadow())
    }

    /// Returns the sampler description
    pub fn desc(&self) -> &SamplerDesc {
        &self.desc
    }

    /// Returns the underlying handle
    pub(crate) fn handle(&self) -> SamplerHandle {
        self.handle
    }

    /// Sets the underlying handle
    pub(crate) fn set_handle(&mut self, handle: SamplerHandle) {
        self.handle = handle;
    }
}
