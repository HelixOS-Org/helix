//! Color Blending State Types for Lumina
//!
//! This module provides comprehensive color blending configuration,
//! blend operations, and color write masks.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Pipeline Color Blend State
// ============================================================================

/// Pipeline color blend state create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ColorBlendStateCreateInfo {
    /// Flags
    pub flags: ColorBlendStateCreateFlags,
    /// Enable logic op
    pub logic_op_enable: bool,
    /// Logic operation
    pub logic_op: LogicOp,
    /// Attachments
    pub attachments: Vec<ColorBlendAttachmentState>,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl ColorBlendStateCreateInfo {
    /// Creates new info with no blending
    #[inline]
    pub fn new() -> Self {
        Self {
            flags: ColorBlendStateCreateFlags::NONE,
            logic_op_enable: false,
            logic_op: LogicOp::NoOp,
            attachments: Vec::new(),
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Creates with single attachment
    #[inline]
    pub fn with_attachment(attachment: ColorBlendAttachmentState) -> Self {
        Self {
            flags: ColorBlendStateCreateFlags::NONE,
            logic_op_enable: false,
            logic_op: LogicOp::NoOp,
            attachments: alloc::vec![attachment],
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Disabled blending for single attachment
    #[inline]
    pub fn disabled() -> Self {
        Self::with_attachment(ColorBlendAttachmentState::disabled())
    }

    /// Standard alpha blending for single attachment
    #[inline]
    pub fn alpha_blend() -> Self {
        Self::with_attachment(ColorBlendAttachmentState::alpha_blend())
    }

    /// Additive blending for single attachment
    #[inline]
    pub fn additive() -> Self {
        Self::with_attachment(ColorBlendAttachmentState::additive())
    }

    /// Premultiplied alpha for single attachment
    #[inline]
    pub fn premultiplied() -> Self {
        Self::with_attachment(ColorBlendAttachmentState::premultiplied_alpha())
    }

    /// With logic op
    #[inline]
    pub fn with_logic_op(mut self, op: LogicOp) -> Self {
        self.logic_op_enable = true;
        self.logic_op = op;
        self
    }

    /// With blend constants
    #[inline]
    pub fn with_blend_constants(mut self, constants: [f32; 4]) -> Self {
        self.blend_constants = constants;
        self
    }

    /// Add attachment
    #[inline]
    pub fn add_attachment(mut self, attachment: ColorBlendAttachmentState) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// With attachments
    #[inline]
    pub fn with_attachments(mut self, attachments: Vec<ColorBlendAttachmentState>) -> Self {
        self.attachments = attachments;
        self
    }

    /// With flags
    #[inline]
    pub fn with_flags(mut self, flags: ColorBlendStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for ColorBlendStateCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Color blend state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ColorBlendStateCreateFlags(pub u32);

impl ColorBlendStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

    /// Rasterization order attachment access (AMD)
    pub const RASTERIZATION_ORDER_ATTACHMENT_ACCESS: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Color Blend Attachment State
// ============================================================================

/// Color blend attachment state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ColorBlendAttachmentState {
    /// Blend enable
    pub blend_enable: bool,
    /// Source color blend factor
    pub src_color_blend_factor: BlendFactor,
    /// Destination color blend factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend operation
    pub color_blend_op: BlendOp,
    /// Source alpha blend factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha blend factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend operation
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorComponentFlags,
}

impl ColorBlendAttachmentState {
    /// Creates new state (disabled)
    #[inline]
    pub const fn new() -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Disabled blending (write through)
    #[inline]
    pub const fn disabled() -> Self {
        Self::new()
    }

    /// Standard alpha blending
    /// result = src.rgb * src.a + dst.rgb * (1 - src.a)
    #[inline]
    pub const fn alpha_blend() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Premultiplied alpha blending
    /// result = src.rgb + dst.rgb * (1 - src.a)
    #[inline]
    pub const fn premultiplied_alpha() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Additive blending
    /// result = src.rgb + dst.rgb
    #[inline]
    pub const fn additive() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Multiplicative blending
    /// result = src.rgb * dst.rgb
    #[inline]
    pub const fn multiply() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::DstColor,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::DstAlpha,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Screen blending
    /// result = 1 - (1 - src) * (1 - dst)
    #[inline]
    pub const fn screen() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcColor,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Subtractive blending
    /// result = dst.rgb - src.rgb
    #[inline]
    pub const fn subtract() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::ReverseSubtract,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::ReverseSubtract,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Min blending
    #[inline]
    pub const fn min() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::Min,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Min,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Max blending
    #[inline]
    pub const fn max() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::Max,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Max,
            color_write_mask: ColorComponentFlags::ALL,
        }
    }

    /// Enable blending
    #[inline]
    pub const fn enable(mut self) -> Self {
        self.blend_enable = true;
        self
    }

    /// Disable blending
    #[inline]
    pub const fn disable(mut self) -> Self {
        self.blend_enable = false;
        self
    }

    /// With color blend factors
    #[inline]
    pub const fn with_color_blend(
        mut self,
        src: BlendFactor,
        dst: BlendFactor,
        op: BlendOp,
    ) -> Self {
        self.src_color_blend_factor = src;
        self.dst_color_blend_factor = dst;
        self.color_blend_op = op;
        self
    }

    /// With alpha blend factors
    #[inline]
    pub const fn with_alpha_blend(
        mut self,
        src: BlendFactor,
        dst: BlendFactor,
        op: BlendOp,
    ) -> Self {
        self.src_alpha_blend_factor = src;
        self.dst_alpha_blend_factor = dst;
        self.alpha_blend_op = op;
        self
    }

    /// With color write mask
    #[inline]
    pub const fn with_color_write_mask(mut self, mask: ColorComponentFlags) -> Self {
        self.color_write_mask = mask;
        self
    }

    /// Only write RGB (no alpha)
    #[inline]
    pub const fn write_rgb_only(mut self) -> Self {
        self.color_write_mask = ColorComponentFlags::RGB;
        self
    }

    /// Only write alpha
    #[inline]
    pub const fn write_alpha_only(mut self) -> Self {
        self.color_write_mask = ColorComponentFlags::A;
        self
    }

    /// No color writes
    #[inline]
    pub const fn no_color_write(mut self) -> Self {
        self.color_write_mask = ColorComponentFlags::NONE;
        self
    }
}

impl Default for ColorBlendAttachmentState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Blend Factor
// ============================================================================

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendFactor {
    /// 0
    #[default]
    Zero              = 0,
    /// 1
    One               = 1,
    /// src.rgb
    SrcColor          = 2,
    /// 1 - src.rgb
    OneMinusSrcColor  = 3,
    /// dst.rgb
    DstColor          = 4,
    /// 1 - dst.rgb
    OneMinusDstColor  = 5,
    /// src.a
    SrcAlpha          = 6,
    /// 1 - src.a
    OneMinusSrcAlpha  = 7,
    /// dst.a
    DstAlpha          = 8,
    /// 1 - dst.a
    OneMinusDstAlpha  = 9,
    /// Constant color
    ConstantColor     = 10,
    /// 1 - constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha     = 12,
    /// 1 - constant alpha
    OneMinusConstantAlpha = 13,
    /// min(src.a, 1 - dst.a)
    SrcAlphaSaturate  = 14,
    /// src1.rgb
    Src1Color         = 15,
    /// 1 - src1.rgb
    OneMinusSrc1Color = 16,
    /// src1.a
    Src1Alpha         = 17,
    /// 1 - src1.a
    OneMinusSrc1Alpha = 18,
}

impl BlendFactor {
    /// Is this factor using constants?
    #[inline]
    pub const fn uses_constants(&self) -> bool {
        matches!(
            self,
            Self::ConstantColor
                | Self::OneMinusConstantColor
                | Self::ConstantAlpha
                | Self::OneMinusConstantAlpha
        )
    }

    /// Is this factor using dual source blending?
    #[inline]
    pub const fn uses_dual_source(&self) -> bool {
        matches!(
            self,
            Self::Src1Color | Self::OneMinusSrc1Color | Self::Src1Alpha | Self::OneMinusSrc1Alpha
        )
    }
}

// ============================================================================
// Blend Op
// ============================================================================

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendOp {
    /// src + dst
    #[default]
    Add                 = 0,
    /// src - dst
    Subtract            = 1,
    /// dst - src
    ReverseSubtract     = 2,
    /// min(src, dst)
    Min                 = 3,
    /// max(src, dst)
    Max                 = 4,

    // Advanced blend operations (if supported)
    /// Zero (EXT)
    ZeroExt             = 1000148000,
    /// Src (EXT)
    SrcExt              = 1000148001,
    /// Dst (EXT)
    DstExt              = 1000148002,
    /// Src over (EXT)
    SrcOverExt          = 1000148003,
    /// Dst over (EXT)
    DstOverExt          = 1000148004,
    /// Src in (EXT)
    SrcInExt            = 1000148005,
    /// Dst in (EXT)
    DstInExt            = 1000148006,
    /// Src out (EXT)
    SrcOutExt           = 1000148007,
    /// Dst out (EXT)
    DstOutExt           = 1000148008,
    /// Src atop (EXT)
    SrcAtopExt          = 1000148009,
    /// Dst atop (EXT)
    DstAtopExt          = 1000148010,
    /// Xor (EXT)
    XorExt              = 1000148011,
    /// Multiply (EXT)
    MultiplyExt         = 1000148012,
    /// Screen (EXT)
    ScreenExt           = 1000148013,
    /// Overlay (EXT)
    OverlayExt          = 1000148014,
    /// Darken (EXT)
    DarkenExt           = 1000148015,
    /// Lighten (EXT)
    LightenExt          = 1000148016,
    /// Color dodge (EXT)
    ColorDodgeExt       = 1000148017,
    /// Color burn (EXT)
    ColorBurnExt        = 1000148018,
    /// Hard light (EXT)
    HardLightExt        = 1000148019,
    /// Soft light (EXT)
    SoftLightExt        = 1000148020,
    /// Difference (EXT)
    DifferenceExt       = 1000148021,
    /// Exclusion (EXT)
    ExclusionExt        = 1000148022,
    /// Invert (EXT)
    InvertExt           = 1000148023,
    /// Invert RGB (EXT)
    InvertRgbExt        = 1000148024,
    /// Linear dodge (EXT)
    LinearDodgeExt      = 1000148025,
    /// Linear burn (EXT)
    LinearBurnExt       = 1000148026,
    /// Vivid light (EXT)
    VividLightExt       = 1000148027,
    /// Linear light (EXT)
    LinearLightExt      = 1000148028,
    /// Pin light (EXT)
    PinLightExt         = 1000148029,
    /// Hard mix (EXT)
    HardMixExt          = 1000148030,
    /// HSL hue (EXT)
    HslHueExt           = 1000148031,
    /// HSL saturation (EXT)
    HslSaturationExt    = 1000148032,
    /// HSL color (EXT)
    HslColorExt         = 1000148033,
    /// HSL luminosity (EXT)
    HslLuminosityExt    = 1000148034,
    /// Plus (EXT)
    PlusExt             = 1000148035,
    /// Plus clamped (EXT)
    PlusClampedExt      = 1000148036,
    /// Plus clamped alpha (EXT)
    PlusClampedAlphaExt = 1000148037,
    /// Plus darker (EXT)
    PlusDarkerExt       = 1000148038,
    /// Minus (EXT)
    MinusExt            = 1000148039,
    /// Minus clamped (EXT)
    MinusClampedExt     = 1000148040,
    /// Contrast (EXT)
    ContrastExt         = 1000148041,
    /// Invert OVG (EXT)
    InvertOvgExt        = 1000148042,
    /// Red (EXT)
    RedExt              = 1000148043,
    /// Green (EXT)
    GreenExt            = 1000148044,
    /// Blue (EXT)
    BlueExt             = 1000148045,
}

impl BlendOp {
    /// Is this an advanced blend operation?
    #[inline]
    pub const fn is_advanced(&self) -> bool {
        (*self as u32) >= 1000148000
    }

    /// Requires blend overlap
    #[inline]
    pub const fn requires_blend_overlap(&self) -> bool {
        self.is_advanced()
    }
}

// ============================================================================
// Logic Op
// ============================================================================

/// Logic operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LogicOp {
    /// 0
    Clear        = 0,
    /// src & dst
    And          = 1,
    /// src & ~dst
    AndReverse   = 2,
    /// src
    Copy         = 3,
    /// ~src & dst
    AndInverted  = 4,
    /// dst (no-op)
    #[default]
    NoOp         = 5,
    /// src ^ dst
    Xor          = 6,
    /// src | dst
    Or           = 7,
    /// ~(src | dst)
    Nor          = 8,
    /// ~(src ^ dst)
    Equivalent   = 9,
    /// ~dst
    Invert       = 10,
    /// src | ~dst
    OrReverse    = 11,
    /// ~src
    CopyInverted = 12,
    /// ~src | dst
    OrInverted   = 13,
    /// ~(src & dst)
    Nand         = 14,
    /// 1
    Set          = 15,
}

// ============================================================================
// Color Component Flags
// ============================================================================

/// Color component flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ColorComponentFlags(pub u32);

impl ColorComponentFlags {
    /// No components
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
    /// RGBA (all)
    pub const ALL: Self = Self(Self::R.0 | Self::G.0 | Self::B.0 | Self::A.0);
    /// RG
    pub const RG: Self = Self(Self::R.0 | Self::G.0);
    /// RB
    pub const RB: Self = Self(Self::R.0 | Self::B.0);
    /// GB
    pub const GB: Self = Self(Self::G.0 | Self::B.0);
    /// RA
    pub const RA: Self = Self(Self::R.0 | Self::A.0);
    /// GA
    pub const GA: Self = Self(Self::G.0 | Self::A.0);
    /// BA
    pub const BA: Self = Self(Self::B.0 | Self::A.0);
    /// RGA
    pub const RGA: Self = Self(Self::R.0 | Self::G.0 | Self::A.0);
    /// RBA
    pub const RBA: Self = Self(Self::R.0 | Self::B.0 | Self::A.0);
    /// GBA
    pub const GBA: Self = Self(Self::G.0 | Self::B.0 | Self::A.0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Has red
    #[inline]
    pub const fn has_red(&self) -> bool {
        self.contains(Self::R)
    }

    /// Has green
    #[inline]
    pub const fn has_green(&self) -> bool {
        self.contains(Self::G)
    }

    /// Has blue
    #[inline]
    pub const fn has_blue(&self) -> bool {
        self.contains(Self::B)
    }

    /// Has alpha
    #[inline]
    pub const fn has_alpha(&self) -> bool {
        self.contains(Self::A)
    }

    /// Component count
    #[inline]
    pub const fn count(&self) -> u32 {
        let mut count = 0;
        if self.has_red() {
            count += 1;
        }
        if self.has_green() {
            count += 1;
        }
        if self.has_blue() {
            count += 1;
        }
        if self.has_alpha() {
            count += 1;
        }
        count
    }
}

// ============================================================================
// Advanced Blend State
// ============================================================================

/// Color blend advanced state create info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ColorBlendAdvancedStateCreateInfo {
    /// Source premultiplied
    pub src_premultiplied: bool,
    /// Destination premultiplied
    pub dst_premultiplied: bool,
    /// Blend overlap mode
    pub blend_overlap: BlendOverlap,
}

impl ColorBlendAdvancedStateCreateInfo {
    /// Default
    pub const DEFAULT: Self = Self {
        src_premultiplied: false,
        dst_premultiplied: false,
        blend_overlap: BlendOverlap::Uncorrelated,
    };

    /// Creates new info
    #[inline]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// With premultiplied source
    #[inline]
    pub const fn with_src_premultiplied(mut self) -> Self {
        self.src_premultiplied = true;
        self
    }

    /// With premultiplied destination
    #[inline]
    pub const fn with_dst_premultiplied(mut self) -> Self {
        self.dst_premultiplied = true;
        self
    }

    /// With blend overlap
    #[inline]
    pub const fn with_blend_overlap(mut self, overlap: BlendOverlap) -> Self {
        self.blend_overlap = overlap;
        self
    }
}

impl Default for ColorBlendAdvancedStateCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Blend overlap
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendOverlap {
    /// Uncorrelated
    #[default]
    Uncorrelated = 0,
    /// Disjoint
    Disjoint     = 1,
    /// Conjoint
    Conjoint     = 2,
}

// ============================================================================
// Color Blend Equation
// ============================================================================

/// Color blend equation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ColorBlendEquation {
    /// Source color blend factor
    pub src_color_blend_factor: BlendFactor,
    /// Destination color blend factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend operation
    pub color_blend_op: BlendOp,
    /// Source alpha blend factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha blend factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend operation
    pub alpha_blend_op: BlendOp,
}

impl ColorBlendEquation {
    /// Default (add, one/zero)
    pub const DEFAULT: Self = Self {
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::Zero,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::Zero,
        alpha_blend_op: BlendOp::Add,
    };

    /// Alpha blend
    pub const ALPHA: Self = Self {
        src_color_blend_factor: BlendFactor::SrcAlpha,
        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
    };

    /// Additive
    pub const ADDITIVE: Self = Self {
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::One,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::One,
        alpha_blend_op: BlendOp::Add,
    };

    /// Creates new equation
    #[inline]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// From attachment state
    #[inline]
    pub const fn from_attachment(state: &ColorBlendAttachmentState) -> Self {
        Self {
            src_color_blend_factor: state.src_color_blend_factor,
            dst_color_blend_factor: state.dst_color_blend_factor,
            color_blend_op: state.color_blend_op,
            src_alpha_blend_factor: state.src_alpha_blend_factor,
            dst_alpha_blend_factor: state.dst_alpha_blend_factor,
            alpha_blend_op: state.alpha_blend_op,
        }
    }
}

impl Default for ColorBlendEquation {
    fn default() -> Self {
        Self::new()
    }
}
