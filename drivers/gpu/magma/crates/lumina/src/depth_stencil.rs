//! Depth and stencil state types
//!
//! This module provides types for depth and stencil testing configuration.

/// Compare operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CompareOp {
    /// Never pass
    Never = 0,
    /// Pass if less
    Less = 1,
    /// Pass if equal
    Equal = 2,
    /// Pass if less or equal
    #[default]
    LessOrEqual = 3,
    /// Pass if greater
    Greater = 4,
    /// Pass if not equal
    NotEqual = 5,
    /// Pass if greater or equal
    GreaterOrEqual = 6,
    /// Always pass
    Always = 7,
}

impl CompareOp {
    /// Reversed for reversed-Z depth
    pub const fn reversed(&self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessOrEqual => Self::GreaterOrEqual,
            Self::Greater => Self::Less,
            Self::GreaterOrEqual => Self::LessOrEqual,
            _ => *self,
        }
    }
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StencilOp {
    /// Keep current value
    #[default]
    Keep = 0,
    /// Set to zero
    Zero = 1,
    /// Replace with reference
    Replace = 2,
    /// Increment and clamp
    IncrementClamp = 3,
    /// Decrement and clamp
    DecrementClamp = 4,
    /// Bitwise invert
    Invert = 5,
    /// Increment and wrap
    IncrementWrap = 6,
    /// Decrement and wrap
    DecrementWrap = 7,
}

/// Stencil operation state for one face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct StencilOpState {
    /// Stencil fail operation
    pub fail_op: StencilOp,
    /// Depth fail operation
    pub depth_fail_op: StencilOp,
    /// Pass operation
    pub pass_op: StencilOp,
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
        Self::disabled()
    }
}

impl StencilOpState {
    /// Disabled stencil (always pass, keep values)
    pub const fn disabled() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// Write stencil value
    pub const fn write(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Replace,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference,
        }
    }

    /// Test stencil equal
    pub const fn test_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            compare_op: CompareOp::Equal,
            compare_mask: 0xFF,
            write_mask: 0x00,
            reference,
        }
    }

    /// Test stencil not equal
    pub const fn test_not_equal(reference: u32) -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            compare_op: CompareOp::NotEqual,
            compare_mask: 0xFF,
            write_mask: 0x00,
            reference,
        }
    }

    /// Increment on pass
    pub const fn increment() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::IncrementClamp,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// Decrement on pass
    pub const fn decrement() -> Self {
        Self {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::DecrementClamp,
            compare_op: CompareOp::Always,
            compare_mask: 0xFF,
            write_mask: 0xFF,
            reference: 0,
        }
    }

    /// With reference value
    pub const fn with_reference(mut self, reference: u32) -> Self {
        self.reference = reference;
        self
    }

    /// With masks
    pub const fn with_masks(mut self, compare: u32, write: u32) -> Self {
        self.compare_mask = compare;
        self.write_mask = write;
        self
    }
}

/// Depth state
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct DepthState {
    /// Enable depth testing
    pub test_enable: bool,
    /// Enable depth writing
    pub write_enable: bool,
    /// Depth compare operation
    pub compare_op: CompareOp,
    /// Enable depth bounds testing
    pub bounds_test_enable: bool,
    /// Minimum depth bound
    pub min_bounds: f32,
    /// Maximum depth bound
    pub max_bounds: f32,
}

impl Default for DepthState {
    fn default() -> Self {
        Self::read_write()
    }
}

impl DepthState {
    /// Disabled depth testing
    pub const fn disabled() -> Self {
        Self {
            test_enable: false,
            write_enable: false,
            compare_op: CompareOp::Always,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Read and write depth
    pub const fn read_write() -> Self {
        Self {
            test_enable: true,
            write_enable: true,
            compare_op: CompareOp::Less,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Read-only depth
    pub const fn read_only() -> Self {
        Self {
            test_enable: true,
            write_enable: false,
            compare_op: CompareOp::Less,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Write-only depth
    pub const fn write_only() -> Self {
        Self {
            test_enable: false,
            write_enable: true,
            compare_op: CompareOp::Always,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Reversed-Z depth (for better precision)
    pub const fn reversed_z() -> Self {
        Self {
            test_enable: true,
            write_enable: true,
            compare_op: CompareOp::Greater,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Read-only reversed-Z
    pub const fn reversed_z_read_only() -> Self {
        Self {
            test_enable: true,
            write_enable: false,
            compare_op: CompareOp::Greater,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// Equal comparison (for decals, etc.)
    pub const fn equal() -> Self {
        Self {
            test_enable: true,
            write_enable: false,
            compare_op: CompareOp::Equal,
            bounds_test_enable: false,
            min_bounds: 0.0,
            max_bounds: 1.0,
        }
    }

    /// With compare operation
    pub const fn with_compare_op(mut self, op: CompareOp) -> Self {
        self.compare_op = op;
        self
    }

    /// With depth bounds
    pub const fn with_bounds(mut self, min: f32, max: f32) -> Self {
        self.bounds_test_enable = true;
        self.min_bounds = min;
        self.max_bounds = max;
        self
    }

    /// With write enable
    pub const fn with_write(mut self, enable: bool) -> Self {
        self.write_enable = enable;
        self
    }
}

/// Stencil state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct StencilState {
    /// Enable stencil testing
    pub test_enable: bool,
    /// Front face operations
    pub front: StencilOpState,
    /// Back face operations
    pub back: StencilOpState,
}

impl Default for StencilState {
    fn default() -> Self {
        Self::disabled()
    }
}

impl StencilState {
    /// Disabled stencil testing
    pub const fn disabled() -> Self {
        Self {
            test_enable: false,
            front: StencilOpState::disabled(),
            back: StencilOpState::disabled(),
        }
    }

    /// Enabled with same front/back
    pub const fn enabled(op: StencilOpState) -> Self {
        Self {
            test_enable: true,
            front: op,
            back: op,
        }
    }

    /// Two-sided stencil
    pub const fn two_sided(front: StencilOpState, back: StencilOpState) -> Self {
        Self {
            test_enable: true,
            front,
            back,
        }
    }

    /// Write reference value
    pub const fn write(reference: u32) -> Self {
        Self {
            test_enable: true,
            front: StencilOpState::write(reference),
            back: StencilOpState::write(reference),
        }
    }

    /// Test equal to reference
    pub const fn test_equal(reference: u32) -> Self {
        Self {
            test_enable: true,
            front: StencilOpState::test_equal(reference),
            back: StencilOpState::test_equal(reference),
        }
    }

    /// Test not equal to reference
    pub const fn test_not_equal(reference: u32) -> Self {
        Self {
            test_enable: true,
            front: StencilOpState::test_not_equal(reference),
            back: StencilOpState::test_not_equal(reference),
        }
    }

    /// With reference value
    pub const fn with_reference(mut self, reference: u32) -> Self {
        self.front.reference = reference;
        self.back.reference = reference;
        self
    }
}

/// Combined depth-stencil state
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct DepthStencilState {
    /// Depth state
    pub depth: DepthState,
    /// Stencil state
    pub stencil: StencilState,
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth: DepthState::read_write(),
            stencil: StencilState::disabled(),
        }
    }
}

impl DepthStencilState {
    /// Disabled depth and stencil
    pub const fn disabled() -> Self {
        Self {
            depth: DepthState::disabled(),
            stencil: StencilState::disabled(),
        }
    }

    /// Depth only (no stencil)
    pub const fn depth_only() -> Self {
        Self {
            depth: DepthState::read_write(),
            stencil: StencilState::disabled(),
        }
    }

    /// Depth read-only
    pub const fn depth_read_only() -> Self {
        Self {
            depth: DepthState::read_only(),
            stencil: StencilState::disabled(),
        }
    }

    /// Reversed-Z depth
    pub const fn reversed_z() -> Self {
        Self {
            depth: DepthState::reversed_z(),
            stencil: StencilState::disabled(),
        }
    }

    /// Stencil write (for masking)
    pub const fn stencil_write(reference: u32) -> Self {
        Self {
            depth: DepthState::disabled(),
            stencil: StencilState::write(reference),
        }
    }

    /// Stencil test (for masking)
    pub const fn stencil_test(reference: u32) -> Self {
        Self {
            depth: DepthState::read_write(),
            stencil: StencilState::test_equal(reference),
        }
    }

    /// With depth state
    pub const fn with_depth(mut self, depth: DepthState) -> Self {
        self.depth = depth;
        self
    }

    /// With stencil state
    pub const fn with_stencil(mut self, stencil: StencilState) -> Self {
        self.stencil = stencil;
        self
    }
}

/// Common depth-stencil presets
pub mod presets {
    use super::*;

    /// No depth or stencil
    pub const DISABLED: DepthStencilState = DepthStencilState::disabled();
    /// Standard depth test and write
    pub const DEPTH_WRITE: DepthStencilState = DepthStencilState::depth_only();
    /// Depth test only (no write)
    pub const DEPTH_READ: DepthStencilState = DepthStencilState::depth_read_only();
    /// Reversed-Z for better precision
    pub const REVERSED_Z: DepthStencilState = DepthStencilState::reversed_z();
}
