//! Depth and Stencil State
//!
//! This module provides depth and stencil testing configuration
//! for the graphics pipeline.

// ============================================================================
// Compare Operation
// ============================================================================

/// Comparison operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompareOp {
    /// Never pass.
    Never,
    /// Pass if less than.
    #[default]
    Less,
    /// Pass if equal.
    Equal,
    /// Pass if less than or equal.
    LessOrEqual,
    /// Pass if greater than.
    Greater,
    /// Pass if not equal.
    NotEqual,
    /// Pass if greater than or equal.
    GreaterOrEqual,
    /// Always pass.
    Always,
}

impl CompareOp {
    /// Invert the comparison.
    pub fn invert(&self) -> Self {
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

// ============================================================================
// Depth Test
// ============================================================================

/// Depth test mode presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DepthTest {
    /// No depth test.
    None,
    /// Standard depth test with less comparison.
    #[default]
    Less,
    /// Depth test with less or equal comparison.
    LessOrEqual,
    /// Depth test with greater comparison (reverse Z).
    Greater,
    /// Depth test with greater or equal comparison (reverse Z).
    GreaterOrEqual,
    /// Depth test with equal comparison.
    Equal,
    /// Always pass depth test.
    Always,
}

impl DepthTest {
    /// Convert to compare operation.
    pub fn to_compare_op(&self) -> Option<CompareOp> {
        match self {
            Self::None => None,
            Self::Less => Some(CompareOp::Less),
            Self::LessOrEqual => Some(CompareOp::LessOrEqual),
            Self::Greater => Some(CompareOp::Greater),
            Self::GreaterOrEqual => Some(CompareOp::GreaterOrEqual),
            Self::Equal => Some(CompareOp::Equal),
            Self::Always => Some(CompareOp::Always),
        }
    }

    /// Check if depth test is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }
}

// ============================================================================
// Depth State
// ============================================================================

/// Depth state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthState {
    /// Enable depth test.
    pub depth_test_enable: bool,
    /// Enable depth write.
    pub depth_write_enable: bool,
    /// Depth compare operation.
    pub depth_compare_op: CompareOp,
    /// Enable depth bounds test.
    pub depth_bounds_test_enable: bool,
    /// Minimum depth bounds.
    pub min_depth_bounds: f32,
    /// Maximum depth bounds.
    pub max_depth_bounds: f32,
}

impl DepthState {
    /// Create disabled depth state.
    pub fn disabled() -> Self {
        Self {
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Create read-only depth state.
    pub fn read_only() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: false,
            depth_compare_op: CompareOp::LessOrEqual,
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Create read-write depth state.
    pub fn read_write() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Less,
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Create reverse-Z depth state.
    pub fn reverse_z() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: true,
            depth_compare_op: CompareOp::Greater,
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Create reverse-Z read-only depth state.
    pub fn reverse_z_read_only() -> Self {
        Self {
            depth_test_enable: true,
            depth_write_enable: false,
            depth_compare_op: CompareOp::GreaterOrEqual,
            depth_bounds_test_enable: false,
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    /// Create from depth test preset.
    pub fn from_test(test: DepthTest, write: bool) -> Self {
        if let Some(compare_op) = test.to_compare_op() {
            Self {
                depth_test_enable: true,
                depth_write_enable: write,
                depth_compare_op: compare_op,
                depth_bounds_test_enable: false,
                min_depth_bounds: 0.0,
                max_depth_bounds: 1.0,
            }
        } else {
            Self::disabled()
        }
    }

    /// Set compare operation.
    pub fn with_compare_op(mut self, op: CompareOp) -> Self {
        self.depth_compare_op = op;
        self
    }

    /// Enable depth bounds.
    pub fn with_bounds(mut self, min: f32, max: f32) -> Self {
        self.depth_bounds_test_enable = true;
        self.min_depth_bounds = min;
        self.max_depth_bounds = max;
        self
    }

    /// Disable depth write.
    pub fn without_write(mut self) -> Self {
        self.depth_write_enable = false;
        self
    }
}

impl Default for DepthState {
    fn default() -> Self {
        Self::read_write()
    }
}

// ============================================================================
// Stencil Operation
// ============================================================================

/// Stencil operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StencilOp {
    /// Keep the current value.
    #[default]
    Keep,
    /// Set to zero.
    Zero,
    /// Replace with reference value.
    Replace,
    /// Increment and clamp.
    IncrementAndClamp,
    /// Decrement and clamp.
    DecrementAndClamp,
    /// Bitwise invert.
    Invert,
    /// Increment and wrap.
    IncrementAndWrap,
    /// Decrement and wrap.
    DecrementAndWrap,
}

// ============================================================================
// Stencil Op State
// ============================================================================

/// Per-face stencil operation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StencilOpState {
    /// Fail operation.
    pub fail_op: StencilOp,
    /// Pass operation.
    pub pass_op: StencilOp,
    /// Depth fail operation.
    pub depth_fail_op: StencilOp,
    /// Compare operation.
    pub compare_op: CompareOp,
    /// Compare mask.
    pub compare_mask: u32,
    /// Write mask.
    pub write_mask: u32,
    /// Reference value.
    pub reference: u32,
}

impl StencilOpState {
    /// Create default stencil op state.
    pub fn new() -> Self {
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

    /// Create always pass state.
    pub fn always_pass() -> Self {
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

    /// Create increment on pass state.
    pub fn increment() -> Self {
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

    /// Create decrement on pass state.
    pub fn decrement() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::DecrementAndClamp,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// Create replace on pass state.
    pub fn replace(reference: u32) -> Self {
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

    /// Create equal test state.
    pub fn test_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::Equal,
            compare_mask: 0xFF,
            write_mask: 0x00,
            reference,
        }
    }

    /// Create not equal test state.
    pub fn test_not_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            compare_op: CompareOp::NotEqual,
            compare_mask: 0xFF,
            write_mask: 0x00,
            reference,
        }
    }

    /// Set reference value.
    pub fn with_reference(mut self, reference: u32) -> Self {
        self.reference = reference;
        self
    }

    /// Set compare mask.
    pub fn with_compare_mask(mut self, mask: u32) -> Self {
        self.compare_mask = mask;
        self
    }

    /// Set write mask.
    pub fn with_write_mask(mut self, mask: u32) -> Self {
        self.write_mask = mask;
        self
    }
}

impl Default for StencilOpState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Stencil State
// ============================================================================

/// Complete stencil state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StencilState {
    /// Enable stencil test.
    pub stencil_test_enable: bool,
    /// Front face stencil operations.
    pub front: StencilOpState,
    /// Back face stencil operations.
    pub back: StencilOpState,
}

impl StencilState {
    /// Create disabled stencil state.
    pub fn disabled() -> Self {
        Self {
            stencil_test_enable: false,
            front: StencilOpState::new(),
            back: StencilOpState::new(),
        }
    }

    /// Create enabled stencil state with same front/back.
    pub fn enabled(op_state: StencilOpState) -> Self {
        Self {
            stencil_test_enable: true,
            front: op_state,
            back: op_state,
        }
    }

    /// Create with separate front/back.
    pub fn separate(front: StencilOpState, back: StencilOpState) -> Self {
        Self {
            stencil_test_enable: true,
            front,
            back,
        }
    }

    /// Create stencil shadow volume front face.
    pub fn shadow_volume_front() -> Self {
        Self::enabled(StencilOpState {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::IncrementAndWrap,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        })
    }

    /// Create stencil shadow volume back face.
    pub fn shadow_volume_back() -> Self {
        Self::enabled(StencilOpState {
            fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            depth_fail_op: StencilOp::DecrementAndWrap,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        })
    }

    /// Create outline stencil write.
    pub fn outline_write(reference: u32) -> Self {
        Self::enabled(StencilOpState::replace(reference))
    }

    /// Create outline stencil test.
    pub fn outline_test(reference: u32) -> Self {
        Self::enabled(StencilOpState::test_not_equal(reference))
    }
}

impl Default for StencilState {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// Depth Stencil State
// ============================================================================

/// Combined depth and stencil state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthStencilState {
    /// Depth state.
    pub depth: DepthState,
    /// Stencil state.
    pub stencil: StencilState,
}

impl DepthStencilState {
    /// Create disabled state.
    pub fn disabled() -> Self {
        Self {
            depth: DepthState::disabled(),
            stencil: StencilState::disabled(),
        }
    }

    /// Create depth only state.
    pub fn depth_only() -> Self {
        Self {
            depth: DepthState::read_write(),
            stencil: StencilState::disabled(),
        }
    }

    /// Create depth read-only state.
    pub fn depth_read_only() -> Self {
        Self {
            depth: DepthState::read_only(),
            stencil: StencilState::disabled(),
        }
    }

    /// Create stencil only state.
    pub fn stencil_only(stencil: StencilState) -> Self {
        Self {
            depth: DepthState::disabled(),
            stencil,
        }
    }

    /// Create reverse-Z depth state.
    pub fn reverse_z() -> Self {
        Self {
            depth: DepthState::reverse_z(),
            stencil: StencilState::disabled(),
        }
    }

    /// Set depth state.
    pub fn with_depth(mut self, depth: DepthState) -> Self {
        self.depth = depth;
        self
    }

    /// Set stencil state.
    pub fn with_stencil(mut self, stencil: StencilState) -> Self {
        self.stencil = stencil;
        self
    }
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self::depth_only()
    }
}
