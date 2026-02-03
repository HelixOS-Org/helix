//! Viewport and scissor types
//!
//! This module provides types for viewport and scissor state management.

extern crate alloc;
use alloc::vec::Vec;

/// Viewport
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Viewport {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Minimum depth
    pub min_depth: f32,
    /// Maximum depth
    pub max_depth: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl Viewport {
    /// Creates a new viewport
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Creates from dimensions
    pub const fn from_dimensions(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    /// Creates from u32 dimensions
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }

    /// Sets depth range
    pub const fn with_depth_range(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }

    /// Reversed depth (1 to 0)
    pub const fn reversed_depth(mut self) -> Self {
        self.min_depth = 1.0;
        self.max_depth = 0.0;
        self
    }

    /// Flipped Y (top-to-bottom)
    pub const fn flipped_y(mut self) -> Self {
        self.y = self.height;
        self.height = -self.height;
        self
    }

    /// Aspect ratio
    pub fn aspect(&self) -> f32 {
        if self.height.abs() > 0.0 {
            self.width / self.height.abs()
        } else {
            1.0
        }
    }

    /// Checks if point is inside
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// Converts screen coordinates to normalized device coordinates
    pub fn screen_to_ndc(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let ndc_x = (2.0 * (screen_x - self.x) / self.width) - 1.0;
        let ndc_y = (2.0 * (screen_y - self.y) / self.height) - 1.0;
        (ndc_x, ndc_y)
    }

    /// Converts normalized device coordinates to screen coordinates
    pub fn ndc_to_screen(&self, ndc_x: f32, ndc_y: f32) -> (f32, f32) {
        let screen_x = self.x + (ndc_x + 1.0) * 0.5 * self.width;
        let screen_y = self.y + (ndc_y + 1.0) * 0.5 * self.height;
        (screen_x, screen_y)
    }
}

/// Scissor rectangle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Scissor {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Scissor {
    /// Creates a new scissor
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates from position and size
    pub const fn from_pos_size(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates from dimensions (at origin)
    pub const fn from_dimensions(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Creates covering entire framebuffer
    pub const fn full(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Maximum scissor (no clipping)
    pub const fn max() -> Self {
        Self::new(0, 0, i32::MAX as u32, i32::MAX as u32)
    }

    /// Right edge
    pub const fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Bottom edge
    pub const fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Area
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Checks if point is inside
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Intersection with another scissor
    pub const fn intersect(&self, other: &Self) -> Self {
        let x1 = if self.x > other.x { self.x } else { other.x };
        let y1 = if self.y > other.y { self.y } else { other.y };
        let x2 = if self.right() < other.right() {
            self.right()
        } else {
            other.right()
        };
        let y2 = if self.bottom() < other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        if x2 > x1 && y2 > y1 {
            Self::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32)
        } else {
            Self::new(0, 0, 0, 0)
        }
    }

    /// Union with another scissor (bounding box)
    pub const fn union(&self, other: &Self) -> Self {
        let x1 = if self.x < other.x { self.x } else { other.x };
        let y1 = if self.y < other.y { self.y } else { other.y };
        let x2 = if self.right() > other.right() {
            self.right()
        } else {
            other.right()
        };
        let y2 = if self.bottom() > other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        Self::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32)
    }

    /// Is empty
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Viewport state
#[derive(Clone, Debug, Default)]
pub struct ViewportState {
    /// Viewports
    pub viewports: Vec<Viewport>,
    /// Scissors
    pub scissors: Vec<Scissor>,
}

impl ViewportState {
    /// Creates new viewport state
    pub const fn new() -> Self {
        Self {
            viewports: Vec::new(),
            scissors: Vec::new(),
        }
    }

    /// Creates with single viewport
    pub fn single(width: u32, height: u32) -> Self {
        Self {
            viewports: alloc::vec![Viewport::from_size(width, height)],
            scissors: alloc::vec![Scissor::from_dimensions(width, height)],
        }
    }

    /// Creates dynamic state (count only)
    pub fn dynamic(count: usize) -> Self {
        Self {
            viewports: alloc::vec![Viewport::default(); count],
            scissors: alloc::vec![Scissor::default(); count],
        }
    }

    /// Adds viewport and scissor
    pub fn add(mut self, viewport: Viewport, scissor: Scissor) -> Self {
        self.viewports.push(viewport);
        self.scissors.push(scissor);
        self
    }

    /// Gets viewport count
    pub fn count(&self) -> usize {
        self.viewports.len()
    }
}

/// Depth bias state
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct DepthBiasState {
    /// Constant factor
    pub constant_factor: f32,
    /// Clamp value
    pub clamp: f32,
    /// Slope factor
    pub slope_factor: f32,
}

impl Default for DepthBiasState {
    fn default() -> Self {
        Self {
            constant_factor: 0.0,
            clamp: 0.0,
            slope_factor: 0.0,
        }
    }
}

impl DepthBiasState {
    /// No bias
    pub const NONE: Self = Self {
        constant_factor: 0.0,
        clamp: 0.0,
        slope_factor: 0.0,
    };

    /// Creates new depth bias
    pub const fn new(constant_factor: f32, slope_factor: f32) -> Self {
        Self {
            constant_factor,
            clamp: 0.0,
            slope_factor,
        }
    }

    /// Typical shadow map bias
    pub const fn shadow() -> Self {
        Self {
            constant_factor: 1.25,
            clamp: 0.0,
            slope_factor: 1.75,
        }
    }

    /// With clamp
    pub const fn with_clamp(mut self, clamp: f32) -> Self {
        self.clamp = clamp;
        self
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.constant_factor != 0.0 || self.slope_factor != 0.0
    }
}

/// Depth bounds
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct DepthBounds {
    /// Minimum depth
    pub min: f32,
    /// Maximum depth
    pub max: f32,
}

impl Default for DepthBounds {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

impl DepthBounds {
    /// Full range
    pub const FULL: Self = Self { min: 0.0, max: 1.0 };

    /// Creates new depth bounds
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

/// Stencil operation state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct StencilOpState {
    /// Fail operation
    pub fail_op: StencilOp,
    /// Pass operation
    pub pass_op: StencilOp,
    /// Depth fail operation
    pub depth_fail_op: StencilOp,
    /// Compare operation
    pub compare_op: CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference value
    pub reference: u32,
}

impl Default for StencilOpState {
    fn default() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }
}

impl StencilOpState {
    /// Keep all
    pub const KEEP: Self = Self {
        fail_op: StencilOp::Keep,
        pass_op: StencilOp::Keep,
        depth_fail_op: StencilOp::Keep,
        compare_op: CompareOp::Always,
        compare_mask: 0xFF,
        write_mask: 0xFF,
        reference: 0,
    };

    /// Replace on pass
    pub const fn replace_on_pass(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Replace,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference,
        }
    }

    /// Increment on pass
    pub const fn increment_on_pass() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::IncrementAndClamp,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// Equal test
    pub const fn equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Equal,
            compare_mask: 0xFF,
            write_mask: 0,
            reference,
        }
    }

    /// Not equal test
    pub const fn not_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::NotEqual,
            compare_mask: 0xFF,
            write_mask: 0,
            reference,
        }
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StencilOp {
    /// Keep current value
    #[default]
    Keep,
    /// Set to zero
    Zero,
    /// Replace with reference
    Replace,
    /// Increment and clamp
    IncrementAndClamp,
    /// Decrement and clamp
    DecrementAndClamp,
    /// Bitwise invert
    Invert,
    /// Increment and wrap
    IncrementAndWrap,
    /// Decrement and wrap
    DecrementAndWrap,
}

/// Compare operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompareOp {
    /// Never pass
    Never,
    /// Less than
    Less,
    /// Equal
    Equal,
    /// Less or equal
    LessOrEqual,
    /// Greater
    Greater,
    /// Not equal
    NotEqual,
    /// Greater or equal
    GreaterOrEqual,
    /// Always pass
    #[default]
    Always,
}

/// Blend factor
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendFactor {
    /// Zero
    Zero,
    /// One
    #[default]
    One,
    /// Source color
    SrcColor,
    /// One minus source color
    OneMinusSrcColor,
    /// Destination color
    DstColor,
    /// One minus destination color
    OneMinusDstColor,
    /// Source alpha
    SrcAlpha,
    /// One minus source alpha
    OneMinusSrcAlpha,
    /// Destination alpha
    DstAlpha,
    /// One minus destination alpha
    OneMinusDstAlpha,
    /// Constant color
    ConstantColor,
    /// One minus constant color
    OneMinusConstantColor,
    /// Constant alpha
    ConstantAlpha,
    /// One minus constant alpha
    OneMinusConstantAlpha,
    /// Source alpha saturate
    SrcAlphaSaturate,
    /// Second source color
    Src1Color,
    /// One minus second source color
    OneMinusSrc1Color,
    /// Second source alpha
    Src1Alpha,
    /// One minus second source alpha
    OneMinusSrc1Alpha,
}

/// Blend operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlendOp {
    /// Add
    #[default]
    Add,
    /// Subtract
    Subtract,
    /// Reverse subtract
    ReverseSubtract,
    /// Minimum
    Min,
    /// Maximum
    Max,
}

/// Color component flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorComponentFlags(pub u8);

impl Default for ColorComponentFlags {
    fn default() -> Self {
        Self::ALL
    }
}

impl ColorComponentFlags {
    /// Red
    pub const R: Self = Self(1 << 0);
    /// Green
    pub const G: Self = Self(1 << 1);
    /// Blue
    pub const B: Self = Self(1 << 2);
    /// Alpha
    pub const A: Self = Self(1 << 3);
    /// All components
    pub const ALL: Self = Self(0xF);
    /// None
    pub const NONE: Self = Self(0);
    /// RGB only
    pub const RGB: Self = Self(0x7);

    /// Combines flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Color blend attachment state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorBlendAttachmentState {
    /// Blend enabled
    pub blend_enable: bool,
    /// Source color factor
    pub src_color_blend_factor: BlendFactor,
    /// Destination color factor
    pub dst_color_blend_factor: BlendFactor,
    /// Color blend operation
    pub color_blend_op: BlendOp,
    /// Source alpha factor
    pub src_alpha_blend_factor: BlendFactor,
    /// Destination alpha factor
    pub dst_alpha_blend_factor: BlendFactor,
    /// Alpha blend operation
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorComponentFlags,
}

impl Default for ColorBlendAttachmentState {
    fn default() -> Self {
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
}

impl ColorBlendAttachmentState {
    /// No blending
    pub const OPAQUE: Self = Self {
        blend_enable: false,
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::Zero,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::Zero,
        alpha_blend_op: BlendOp::Add,
        color_write_mask: ColorComponentFlags::ALL,
    };

    /// Alpha blending
    pub const ALPHA: Self = Self {
        blend_enable: true,
        src_color_blend_factor: BlendFactor::SrcAlpha,
        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
        color_write_mask: ColorComponentFlags::ALL,
    };

    /// Premultiplied alpha
    pub const PREMULTIPLIED: Self = Self {
        blend_enable: true,
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
        color_write_mask: ColorComponentFlags::ALL,
    };

    /// Additive blending
    pub const ADDITIVE: Self = Self {
        blend_enable: true,
        src_color_blend_factor: BlendFactor::SrcAlpha,
        dst_color_blend_factor: BlendFactor::One,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::One,
        alpha_blend_op: BlendOp::Add,
        color_write_mask: ColorComponentFlags::ALL,
    };

    /// Multiply blending
    pub const MULTIPLY: Self = Self {
        blend_enable: true,
        src_color_blend_factor: BlendFactor::DstColor,
        dst_color_blend_factor: BlendFactor::Zero,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::DstAlpha,
        dst_alpha_blend_factor: BlendFactor::Zero,
        alpha_blend_op: BlendOp::Add,
        color_write_mask: ColorComponentFlags::ALL,
    };

    /// With write mask
    pub const fn with_write_mask(mut self, mask: ColorComponentFlags) -> Self {
        self.color_write_mask = mask;
        self
    }
}

/// Logic operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LogicOp {
    /// Clear
    Clear,
    /// AND
    And,
    /// AND reverse
    AndReverse,
    /// Copy
    #[default]
    Copy,
    /// AND inverted
    AndInverted,
    /// No op
    NoOp,
    /// XOR
    Xor,
    /// OR
    Or,
    /// NOR
    Nor,
    /// Equivalent
    Equivalent,
    /// Invert
    Invert,
    /// OR reverse
    OrReverse,
    /// Copy inverted
    CopyInverted,
    /// OR inverted
    OrInverted,
    /// NAND
    Nand,
    /// Set
    Set,
}

/// Color blend state
#[derive(Clone, Debug, Default)]
pub struct ColorBlendState {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Logic operation
    pub logic_op: LogicOp,
    /// Attachments
    pub attachments: Vec<ColorBlendAttachmentState>,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl ColorBlendState {
    /// Creates new color blend state
    pub const fn new() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: Vec::new(),
            blend_constants: [0.0; 4],
        }
    }

    /// Adds an attachment
    pub fn add_attachment(mut self, attachment: ColorBlendAttachmentState) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// With blend constants
    pub fn with_constants(mut self, constants: [f32; 4]) -> Self {
        self.blend_constants = constants;
        self
    }

    /// With logic op
    pub const fn with_logic_op(mut self, op: LogicOp) -> Self {
        self.logic_op_enable = true;
        self.logic_op = op;
        self
    }
}
