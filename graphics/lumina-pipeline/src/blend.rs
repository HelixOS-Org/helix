//! Blend State
//!
//! This module provides blend state configuration for color blending
//! in the graphics pipeline.

use alloc::vec::Vec;

// ============================================================================
// Blend Mode Presets
// ============================================================================

/// Common blend mode presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// No blending (opaque).
    #[default]
    Opaque,
    /// Alpha blending (standard transparency).
    AlphaBlend,
    /// Premultiplied alpha blending.
    PremultipliedAlpha,
    /// Additive blending.
    Additive,
    /// Multiply blending.
    Multiply,
    /// Screen blending.
    Screen,
    /// Custom blending (use full AttachmentBlend).
    Custom,
}

impl BlendMode {
    /// Convert to attachment blend.
    pub fn to_attachment_blend(&self) -> AttachmentBlend {
        match self {
            Self::Opaque => AttachmentBlend::opaque(),
            Self::AlphaBlend => AttachmentBlend::alpha_blend(),
            Self::PremultipliedAlpha => AttachmentBlend::premultiplied_alpha(),
            Self::Additive => AttachmentBlend::additive(),
            Self::Multiply => AttachmentBlend::multiply(),
            Self::Screen => AttachmentBlend::screen(),
            Self::Custom => AttachmentBlend::opaque(),
        }
    }
}

// ============================================================================
// Blend Factor
// ============================================================================

/// Blend factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendFactor {
    /// Factor of 0.
    Zero,
    /// Factor of 1.
    #[default]
    One,
    /// Source color.
    SrcColor,
    /// One minus source color.
    OneMinusSrcColor,
    /// Destination color.
    DstColor,
    /// One minus destination color.
    OneMinusDstColor,
    /// Source alpha.
    SrcAlpha,
    /// One minus source alpha.
    OneMinusSrcAlpha,
    /// Destination alpha.
    DstAlpha,
    /// One minus destination alpha.
    OneMinusDstAlpha,
    /// Constant color.
    ConstantColor,
    /// One minus constant color.
    OneMinusConstantColor,
    /// Constant alpha.
    ConstantAlpha,
    /// One minus constant alpha.
    OneMinusConstantAlpha,
    /// Source alpha saturate.
    SrcAlphaSaturate,
    /// Second source color.
    Src1Color,
    /// One minus second source color.
    OneMinusSrc1Color,
    /// Second source alpha.
    Src1Alpha,
    /// One minus second source alpha.
    OneMinusSrc1Alpha,
}

// ============================================================================
// Blend Operation
// ============================================================================

/// Blend operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendOp {
    /// Add.
    #[default]
    Add,
    /// Subtract.
    Subtract,
    /// Reverse subtract.
    ReverseSubtract,
    /// Minimum.
    Min,
    /// Maximum.
    Max,
}

// ============================================================================
// Color Write Mask
// ============================================================================

/// Color write mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorWriteMask(u8);

impl ColorWriteMask {
    /// No channels.
    pub const NONE: Self = Self(0);
    /// Red channel.
    pub const R: Self = Self(1 << 0);
    /// Green channel.
    pub const G: Self = Self(1 << 1);
    /// Blue channel.
    pub const B: Self = Self(1 << 2);
    /// Alpha channel.
    pub const A: Self = Self(1 << 3);
    /// All channels.
    pub const ALL: Self = Self(0xF);
    /// RGB channels.
    pub const RGB: Self = Self(0x7);

    /// Create a new mask.
    pub fn new(r: bool, g: bool, b: bool, a: bool) -> Self {
        let mut mask = 0u8;
        if r {
            mask |= 1;
        }
        if g {
            mask |= 2;
        }
        if b {
            mask |= 4;
        }
        if a {
            mask |= 8;
        }
        Self(mask)
    }

    /// Combine masks.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if red is enabled.
    pub fn r(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Check if green is enabled.
    pub fn g(&self) -> bool {
        (self.0 & 2) != 0
    }

    /// Check if blue is enabled.
    pub fn b(&self) -> bool {
        (self.0 & 4) != 0
    }

    /// Check if alpha is enabled.
    pub fn a(&self) -> bool {
        (self.0 & 8) != 0
    }

    /// Check if all channels are enabled.
    pub fn is_all(&self) -> bool {
        self.0 == 0xF
    }

    /// Check if no channels are enabled.
    pub fn is_none(&self) -> bool {
        self.0 == 0
    }
}

impl Default for ColorWriteMask {
    fn default() -> Self {
        Self::ALL
    }
}

// ============================================================================
// Attachment Blend
// ============================================================================

/// Per-attachment blend state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AttachmentBlend {
    /// Enable blending.
    pub blend_enable: bool,
    /// Source color factor.
    pub src_color_blend_factor: BlendFactor,
    /// Destination color factor.
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend operation.
    pub color_blend_op: BlendOp,
    /// Source alpha factor.
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha factor.
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend operation.
    pub alpha_blend_op: BlendOp,
    /// Color write mask.
    pub color_write_mask: ColorWriteMask,
}

impl AttachmentBlend {
    /// Create opaque blend (no blending).
    pub fn opaque() -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create standard alpha blend.
    pub fn alpha_blend() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create premultiplied alpha blend.
    pub fn premultiplied_alpha() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create additive blend.
    pub fn additive() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::SrcAlpha,
            dst_color_blend_factor: BlendFactor::One,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::One,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create multiply blend.
    pub fn multiply() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::DstColor,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::DstAlpha,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create screen blend.
    pub fn screen() -> Self {
        Self {
            blend_enable: true,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::OneMinusSrcColor,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteMask::ALL,
        }
    }

    /// Create with write mask only.
    pub fn write_mask_only(mask: ColorWriteMask) -> Self {
        Self {
            blend_enable: false,
            src_color_blend_factor: BlendFactor::One,
            dst_color_blend_factor: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend_factor: BlendFactor::One,
            dst_alpha_blend_factor: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: mask,
        }
    }

    /// Set color write mask.
    pub fn with_write_mask(mut self, mask: ColorWriteMask) -> Self {
        self.color_write_mask = mask;
        self
    }

    /// Disable color writes.
    pub fn disable_writes(mut self) -> Self {
        self.color_write_mask = ColorWriteMask::NONE;
        self
    }
}

impl Default for AttachmentBlend {
    fn default() -> Self {
        Self::opaque()
    }
}

// ============================================================================
// Logic Operation
// ============================================================================

/// Logic operation (for integer render targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LogicOp {
    /// Clear (0).
    Clear,
    /// Set (1).
    Set,
    /// Copy (source).
    #[default]
    Copy,
    /// Copy inverted (NOT source).
    CopyInverted,
    /// No-op (destination).
    NoOp,
    /// Invert (NOT destination).
    Invert,
    /// AND.
    And,
    /// NAND.
    Nand,
    /// OR.
    Or,
    /// NOR.
    Nor,
    /// XOR.
    Xor,
    /// Equivalent (NOT XOR).
    Equivalent,
    /// AND with reverse.
    AndReverse,
    /// AND with invert.
    AndInverted,
    /// OR with reverse.
    OrReverse,
    /// OR with invert.
    OrInverted,
}

// ============================================================================
// Blend State
// ============================================================================

/// Complete blend state.
#[derive(Clone)]
pub struct BlendState {
    /// Enable logic operations.
    pub logic_op_enable: bool,
    /// Logic operation.
    pub logic_op: LogicOp,
    /// Per-attachment blend states.
    pub attachments: Vec<AttachmentBlend>,
    /// Blend constants.
    pub blend_constants: [f32; 4],
}

impl BlendState {
    /// Create default blend state.
    pub fn new() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: Vec::new(),
            blend_constants: [0.0; 4],
        }
    }

    /// Create opaque blend state.
    pub fn opaque() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![AttachmentBlend::opaque()],
            blend_constants: [0.0; 4],
        }
    }

    /// Create alpha blend state.
    pub fn alpha_blend() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![AttachmentBlend::alpha_blend()],
            blend_constants: [0.0; 4],
        }
    }

    /// Create additive blend state.
    pub fn additive() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![AttachmentBlend::additive()],
            blend_constants: [0.0; 4],
        }
    }

    /// Create from blend mode.
    pub fn from_mode(mode: BlendMode) -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: alloc::vec![mode.to_attachment_blend()],
            blend_constants: [0.0; 4],
        }
    }

    /// Create with multiple attachments.
    pub fn with_attachments(attachments: Vec<AttachmentBlend>) -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments,
            blend_constants: [0.0; 4],
        }
    }

    /// Add attachment.
    pub fn attachment(mut self, blend: AttachmentBlend) -> Self {
        self.attachments.push(blend);
        self
    }

    /// Set blend constants.
    pub fn constants(mut self, constants: [f32; 4]) -> Self {
        self.blend_constants = constants;
        self
    }

    /// Enable logic operation.
    pub fn logic_op(mut self, op: LogicOp) -> Self {
        self.logic_op_enable = true;
        self.logic_op = op;
        self
    }

    /// Get attachment count.
    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    /// Get attachment at index.
    pub fn get_attachment(&self, index: usize) -> Option<&AttachmentBlend> {
        self.attachments.get(index)
    }
}

impl Default for BlendState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Blend State Builder
// ============================================================================

/// Builder for blend state.
pub struct BlendStateBuilder {
    state: BlendState,
}

impl BlendStateBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            state: BlendState::new(),
        }
    }

    /// Add opaque attachment.
    pub fn opaque(mut self) -> Self {
        self.state.attachments.push(AttachmentBlend::opaque());
        self
    }

    /// Add alpha blend attachment.
    pub fn alpha_blend(mut self) -> Self {
        self.state.attachments.push(AttachmentBlend::alpha_blend());
        self
    }

    /// Add additive attachment.
    pub fn additive(mut self) -> Self {
        self.state.attachments.push(AttachmentBlend::additive());
        self
    }

    /// Add custom attachment.
    pub fn attachment(mut self, blend: AttachmentBlend) -> Self {
        self.state.attachments.push(blend);
        self
    }

    /// Set blend constants.
    pub fn constants(mut self, constants: [f32; 4]) -> Self {
        self.state.blend_constants = constants;
        self
    }

    /// Enable logic operation.
    pub fn logic_op(mut self, op: LogicOp) -> Self {
        self.state.logic_op_enable = true;
        self.state.logic_op = op;
        self
    }

    /// Build the blend state.
    pub fn build(self) -> BlendState {
        self.state
    }
}

impl Default for BlendStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
