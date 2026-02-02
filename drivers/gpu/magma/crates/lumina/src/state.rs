//! Render state management
//!
//! This module provides types for tracking and managing render state.

use crate::pipeline::{BlendMode, CullMode, DepthTest};

/// Current render state
#[derive(Clone, Debug, Default)]
pub struct RenderState {
    /// Viewport
    pub viewport: Viewport,
    /// Scissor rectangle
    pub scissor: Option<Scissor>,
    /// Depth test mode
    pub depth_test: DepthTest,
    /// Depth write enabled
    pub depth_write: bool,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Depth bias
    pub depth_bias: Option<DepthBias>,
    /// Stencil state
    pub stencil: Option<StencilState>,
}

/// Viewport dimensions
#[derive(Clone, Copy, Debug, Default)]
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

impl Viewport {
    /// Creates a viewport from dimensions
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Creates a viewport from window size
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }
}

/// Scissor rectangle
#[derive(Clone, Copy, Debug, Default)]
pub struct Scissor {
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Scissor {
    /// Creates a scissor rectangle
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Depth bias configuration
#[derive(Clone, Copy, Debug, Default)]
pub struct DepthBias {
    /// Constant bias
    pub constant_factor: f32,
    /// Clamp value
    pub clamp: f32,
    /// Slope factor
    pub slope_factor: f32,
}

/// Stencil operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StencilOp {
    /// Keep current value
    #[default]
    Keep,
    /// Set to zero
    Zero,
    /// Replace with reference
    Replace,
    /// Increment and clamp
    IncrementClamp,
    /// Decrement and clamp
    DecrementClamp,
    /// Bitwise invert
    Invert,
    /// Increment and wrap
    IncrementWrap,
    /// Decrement and wrap
    DecrementWrap,
}

impl StencilOp {
    /// Returns the Vulkan stencil op
    pub const fn vk_op(self) -> u32 {
        match self {
            Self::Keep => 0,
            Self::Zero => 1,
            Self::Replace => 2,
            Self::IncrementClamp => 3,
            Self::DecrementClamp => 4,
            Self::Invert => 5,
            Self::IncrementWrap => 6,
            Self::DecrementWrap => 7,
        }
    }
}

/// Stencil test configuration for one face
#[derive(Clone, Copy, Debug, Default)]
pub struct StencilFace {
    /// Stencil fail operation
    pub fail_op: StencilOp,
    /// Depth fail operation
    pub depth_fail_op: StencilOp,
    /// Pass operation
    pub pass_op: StencilOp,
    /// Comparison function
    pub compare_op: crate::sampler::CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference value
    pub reference: u32,
}

/// Full stencil state
#[derive(Clone, Copy, Debug, Default)]
pub struct StencilState {
    /// Front face operations
    pub front: StencilFace,
    /// Back face operations
    pub back: StencilFace,
}
