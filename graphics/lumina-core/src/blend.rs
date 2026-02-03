//! Blend state and operations
//!
//! This module provides types for color blending configuration.

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendFactor {
    /// 0
    Zero = 0,
    /// 1
    #[default]
    One = 1,
    /// Source color
    SrcColor = 2,
    /// 1 - Source color
    OneMinusSrcColor = 3,
    /// Dest color
    DstColor = 4,
    /// 1 - Dest color
    OneMinusDstColor = 5,
    /// Source alpha
    SrcAlpha = 6,
    /// 1 - Source alpha
    OneMinusSrcAlpha = 7,
    /// Dest alpha
    DstAlpha = 8,
    /// 1 - Dest alpha
    OneMinusDstAlpha = 9,
    /// Constant color
    ConstantColor = 10,
    /// 1 - Constant color
    OneMinusConstantColor = 11,
    /// Constant alpha
    ConstantAlpha = 12,
    /// 1 - Constant alpha
    OneMinusConstantAlpha = 13,
    /// min(src_alpha, 1 - dst_alpha)
    SrcAlphaSaturate = 14,
    /// Second source color
    Src1Color = 15,
    /// 1 - Second source color
    OneMinusSrc1Color = 16,
    /// Second source alpha
    Src1Alpha = 17,
    /// 1 - Second source alpha
    OneMinusSrc1Alpha = 18,
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlendOp {
    /// src + dst
    #[default]
    Add = 0,
    /// src - dst
    Subtract = 1,
    /// dst - src
    ReverseSubtract = 2,
    /// min(src, dst)
    Min = 3,
    /// max(src, dst)
    Max = 4,
}

/// Logic operation (for integer formats)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LogicOp {
    /// Clear to 0
    Clear = 0,
    /// AND
    And = 1,
    /// AND with inverted dst
    AndReverse = 2,
    /// Copy src
    Copy = 3,
    /// AND with inverted src
    AndInverted = 4,
    /// No-op
    NoOp = 5,
    /// XOR
    Xor = 6,
    /// OR
    Or = 7,
    /// NOR
    Nor = 8,
    /// XNOR (equivalence)
    Equivalent = 9,
    /// Invert dst
    Invert = 10,
    /// OR with inverted dst
    OrReverse = 11,
    /// Copy inverted src
    CopyInverted = 12,
    /// OR with inverted src
    OrInverted = 13,
    /// NAND
    Nand = 14,
    /// Set to 1
    Set = 15,
}

/// Color write mask
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorWriteMask(pub u8);

impl ColorWriteMask {
    /// No components
    pub const NONE: Self = Self(0);
    /// Red component
    pub const R: Self = Self(1 << 0);
    /// Green component
    pub const G: Self = Self(1 << 1);
    /// Blue component
    pub const B: Self = Self(1 << 2);
    /// Alpha component
    pub const A: Self = Self(1 << 3);
    /// RGB components
    pub const RGB: Self = Self(Self::R.0 | Self::G.0 | Self::B.0);
    /// All components
    pub const ALL: Self = Self(Self::R.0 | Self::G.0 | Self::B.0 | Self::A.0);

    /// Checks if contains
    pub const fn contains(&self, mask: Self) -> bool {
        (self.0 & mask.0) == mask.0
    }
}

impl Default for ColorWriteMask {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitOr for ColorWriteMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for ColorWriteMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Blend state for a single attachment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct BlendAttachment {
    /// Enable blending
    pub blend_enable: bool,
    /// Source color factor
    pub src_color_factor: BlendFactor,
    /// Dest color factor
    pub dst_color_factor: BlendFactor,
    /// Color blend operation
    pub color_op: BlendOp,
    /// Source alpha factor
    pub src_alpha_factor: BlendFactor,
    /// Dest alpha factor
    pub dst_alpha_factor: BlendFactor,
    /// Alpha blend operation
    pub alpha_op: BlendOp,
    /// Color write mask
    pub write_mask: ColorWriteMask,
}

impl Default for BlendAttachment {
    fn default() -> Self {
        Self::disabled()
    }
}

impl BlendAttachment {
    /// Disabled blending (write-through)
    pub const fn disabled() -> Self {
        Self {
            blend_enable: false,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Standard alpha blending
    pub const fn alpha_blend() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::SrcAlpha,
            dst_color_factor: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Pre-multiplied alpha blending
    pub const fn premultiplied_alpha() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Additive blending
    pub const fn additive() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::SrcAlpha,
            dst_color_factor: BlendFactor::One,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::One,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Multiplicative blending
    pub const fn multiply() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::DstColor,
            dst_color_factor: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::DstAlpha,
            dst_alpha_factor: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Screen blending (1 - (1-src) * (1-dst))
    pub const fn screen() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::OneMinusSrcColor,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Subtractive blending
    pub const fn subtract() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::SrcAlpha,
            dst_color_factor: BlendFactor::One,
            color_op: BlendOp::ReverseSubtract,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::One,
            alpha_op: BlendOp::ReverseSubtract,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Min blending
    pub const fn min() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::One,
            color_op: BlendOp::Min,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::One,
            alpha_op: BlendOp::Min,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Max blending
    pub const fn max() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::One,
            color_op: BlendOp::Max,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::One,
            alpha_op: BlendOp::Max,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// Dual source blending
    pub const fn dual_source() -> Self {
        Self {
            blend_enable: true,
            src_color_factor: BlendFactor::One,
            dst_color_factor: BlendFactor::Src1Color,
            color_op: BlendOp::Add,
            src_alpha_factor: BlendFactor::One,
            dst_alpha_factor: BlendFactor::Src1Alpha,
            alpha_op: BlendOp::Add,
            write_mask: ColorWriteMask::ALL,
        }
    }

    /// With write mask
    pub const fn with_write_mask(mut self, mask: ColorWriteMask) -> Self {
        self.write_mask = mask;
        self
    }

    /// Separate alpha factors
    pub const fn with_alpha_factors(
        mut self,
        src: BlendFactor,
        dst: BlendFactor,
        op: BlendOp,
    ) -> Self {
        self.src_alpha_factor = src;
        self.dst_alpha_factor = dst;
        self.alpha_op = op;
        self
    }

    /// Is blending enabled
    pub const fn is_enabled(&self) -> bool {
        self.blend_enable
    }

    /// Uses dual source blending
    pub const fn uses_dual_source(&self) -> bool {
        matches!(
            self.src_color_factor,
            BlendFactor::Src1Color
                | BlendFactor::OneMinusSrc1Color
                | BlendFactor::Src1Alpha
                | BlendFactor::OneMinusSrc1Alpha
        ) || matches!(
            self.dst_color_factor,
            BlendFactor::Src1Color
                | BlendFactor::OneMinusSrc1Color
                | BlendFactor::Src1Alpha
                | BlendFactor::OneMinusSrc1Alpha
        )
    }
}

/// Blend state for all attachments
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BlendState {
    /// Logic operation enable
    pub logic_op_enable: bool,
    /// Logic operation
    pub logic_op: LogicOp,
    /// Per-attachment blend state
    pub attachments: [BlendAttachment; 8],
    /// Number of attachments
    pub attachment_count: u32,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl Default for BlendState {
    fn default() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: [BlendAttachment::disabled(); 8],
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }
}

impl BlendState {
    /// Disabled blending
    pub const fn disabled() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: [BlendAttachment::disabled(); 8],
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }

    /// Alpha blending
    pub const fn alpha_blend() -> Self {
        let mut attachments = [BlendAttachment::disabled(); 8];
        attachments[0] = BlendAttachment::alpha_blend();
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments,
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }

    /// Pre-multiplied alpha
    pub const fn premultiplied() -> Self {
        let mut attachments = [BlendAttachment::disabled(); 8];
        attachments[0] = BlendAttachment::premultiplied_alpha();
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments,
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }

    /// Additive blending
    pub const fn additive() -> Self {
        let mut attachments = [BlendAttachment::disabled(); 8];
        attachments[0] = BlendAttachment::additive();
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments,
            attachment_count: 1,
            blend_constants: [0.0; 4],
        }
    }

    /// Sets attachment blend state
    pub fn set_attachment(&mut self, index: usize, blend: BlendAttachment) -> &mut Self {
        if index < 8 {
            self.attachments[index] = blend;
            self.attachment_count = self.attachment_count.max(index as u32 + 1);
        }
        self
    }

    /// Sets all attachments to same blend state
    pub fn set_all_attachments(&mut self, blend: BlendAttachment) -> &mut Self {
        for i in 0..self.attachment_count as usize {
            self.attachments[i] = blend;
        }
        self
    }

    /// With blend constants
    pub const fn with_constants(mut self, constants: [f32; 4]) -> Self {
        self.blend_constants = constants;
        self
    }

    /// With logic operation
    pub const fn with_logic_op(mut self, op: LogicOp) -> Self {
        self.logic_op_enable = true;
        self.logic_op = op;
        self
    }

    /// Any attachment has blending enabled
    pub fn any_blend_enabled(&self) -> bool {
        self.attachments[..self.attachment_count as usize]
            .iter()
            .any(|a| a.blend_enable)
    }

    /// Uses dual source blending
    pub fn uses_dual_source(&self) -> bool {
        self.attachments[..self.attachment_count as usize]
            .iter()
            .any(|a| a.uses_dual_source())
    }
}

/// Common blend presets
pub mod presets {
    use super::*;

    /// No blending
    pub const OPAQUE: BlendAttachment = BlendAttachment::disabled();
    /// Standard alpha blend
    pub const ALPHA_BLEND: BlendAttachment = BlendAttachment::alpha_blend();
    /// Pre-multiplied alpha
    pub const PREMULTIPLIED: BlendAttachment = BlendAttachment::premultiplied_alpha();
    /// Additive
    pub const ADDITIVE: BlendAttachment = BlendAttachment::additive();
    /// Multiplicative
    pub const MULTIPLY: BlendAttachment = BlendAttachment::multiply();
    /// Screen
    pub const SCREEN: BlendAttachment = BlendAttachment::screen();
}
