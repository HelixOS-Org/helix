//! Pipeline State
//!
//! This module provides dynamic state management for pipelines including:
//! - Dynamic state flags
//! - Viewport and scissor state
//! - Line width, depth bias, blend constants

use alloc::vec::Vec;

// ============================================================================
// Dynamic State Flags
// ============================================================================

/// Dynamic state flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DynamicStateFlags(u32);

impl DynamicStateFlags {
    /// No dynamic state.
    pub const NONE: Self = Self(0);
    /// Viewport.
    pub const VIEWPORT: Self = Self(1 << 0);
    /// Scissor.
    pub const SCISSOR: Self = Self(1 << 1);
    /// Line width.
    pub const LINE_WIDTH: Self = Self(1 << 2);
    /// Depth bias.
    pub const DEPTH_BIAS: Self = Self(1 << 3);
    /// Blend constants.
    pub const BLEND_CONSTANTS: Self = Self(1 << 4);
    /// Depth bounds.
    pub const DEPTH_BOUNDS: Self = Self(1 << 5);
    /// Stencil compare mask.
    pub const STENCIL_COMPARE_MASK: Self = Self(1 << 6);
    /// Stencil write mask.
    pub const STENCIL_WRITE_MASK: Self = Self(1 << 7);
    /// Stencil reference.
    pub const STENCIL_REFERENCE: Self = Self(1 << 8);
    /// Cull mode.
    pub const CULL_MODE: Self = Self(1 << 9);
    /// Front face.
    pub const FRONT_FACE: Self = Self(1 << 10);
    /// Primitive topology.
    pub const PRIMITIVE_TOPOLOGY: Self = Self(1 << 11);
    /// Viewport with count.
    pub const VIEWPORT_WITH_COUNT: Self = Self(1 << 12);
    /// Scissor with count.
    pub const SCISSOR_WITH_COUNT: Self = Self(1 << 13);
    /// Vertex input binding stride.
    pub const VERTEX_INPUT_BINDING_STRIDE: Self = Self(1 << 14);
    /// Depth test enable.
    pub const DEPTH_TEST_ENABLE: Self = Self(1 << 15);
    /// Depth write enable.
    pub const DEPTH_WRITE_ENABLE: Self = Self(1 << 16);
    /// Depth compare op.
    pub const DEPTH_COMPARE_OP: Self = Self(1 << 17);
    /// Depth bounds test enable.
    pub const DEPTH_BOUNDS_TEST_ENABLE: Self = Self(1 << 18);
    /// Stencil test enable.
    pub const STENCIL_TEST_ENABLE: Self = Self(1 << 19);
    /// Stencil op.
    pub const STENCIL_OP: Self = Self(1 << 20);
    /// Rasterizer discard enable.
    pub const RASTERIZER_DISCARD_ENABLE: Self = Self(1 << 21);
    /// Depth bias enable.
    pub const DEPTH_BIAS_ENABLE: Self = Self(1 << 22);
    /// Primitive restart enable.
    pub const PRIMITIVE_RESTART_ENABLE: Self = Self(1 << 23);
    /// Vertex input.
    pub const VERTEX_INPUT: Self = Self(1 << 24);
    /// Patch control points.
    pub const PATCH_CONTROL_POINTS: Self = Self(1 << 25);
    /// Logic op.
    pub const LOGIC_OP: Self = Self(1 << 26);
    /// Color write enable.
    pub const COLOR_WRITE_ENABLE: Self = Self(1 << 27);
    /// Fragment shading rate.
    pub const FRAGMENT_SHADING_RATE: Self = Self(1 << 28);

    /// Common dynamic states.
    pub const COMMON: Self = Self(Self::VIEWPORT.0 | Self::SCISSOR.0);

    /// Create empty flags.
    pub fn empty() -> Self {
        Self::NONE
    }

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if flag is set.
    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Check if any flag is set.
    pub fn any(&self) -> bool {
        self.0 != 0
    }

    /// Count set flags.
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

// ============================================================================
// Viewport
// ============================================================================

/// Viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// X position.
    pub x: f32,
    /// Y position.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
    /// Minimum depth.
    pub min_depth: f32,
    /// Maximum depth.
    pub max_depth: f32,
}

impl Viewport {
    /// Create a new viewport.
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

    /// Create from dimensions.
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    /// Create from u32 dimensions.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }

    /// Set depth range.
    pub fn with_depth_range(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }

    /// Get aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }

    /// Check if point is inside viewport.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }
}

// ============================================================================
// Scissor
// ============================================================================

/// Scissor rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scissor {
    /// X offset.
    pub x: i32,
    /// Y offset.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Scissor {
    /// Create a new scissor.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create from dimensions.
    pub fn from_dimensions(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Create from viewport.
    pub fn from_viewport(viewport: &Viewport) -> Self {
        Self::new(
            viewport.x as i32,
            viewport.y as i32,
            viewport.width as u32,
            viewport.height as u32,
        )
    }

    /// Check if point is inside scissor.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    /// Intersect with another scissor.
    pub fn intersect(&self, other: &Scissor) -> Option<Scissor> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x2 > x1 && y2 > y1 {
            Some(Scissor::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32))
        } else {
            None
        }
    }
}

impl Default for Scissor {
    fn default() -> Self {
        Self::new(0, 0, u32::MAX, u32::MAX)
    }
}

// ============================================================================
// Pipeline State
// ============================================================================

/// Complete pipeline state.
#[derive(Clone)]
pub struct PipelineState {
    /// Active dynamic state.
    pub dynamic_state: DynamicStateFlags,
    /// Viewports.
    pub viewports: Vec<Viewport>,
    /// Scissors.
    pub scissors: Vec<Scissor>,
    /// Line width.
    pub line_width: f32,
    /// Depth bias.
    pub depth_bias: DepthBiasState,
    /// Blend constants.
    pub blend_constants: [f32; 4],
    /// Depth bounds.
    pub depth_bounds: (f32, f32),
    /// Stencil compare mask.
    pub stencil_compare_mask: StencilMask,
    /// Stencil write mask.
    pub stencil_write_mask: StencilMask,
    /// Stencil reference.
    pub stencil_reference: StencilMask,
    /// Fragment shading rate.
    pub fragment_shading_rate: FragmentShadingRate,
}

impl PipelineState {
    /// Create a new pipeline state.
    pub fn new() -> Self {
        Self {
            dynamic_state: DynamicStateFlags::NONE,
            viewports: Vec::new(),
            scissors: Vec::new(),
            line_width: 1.0,
            depth_bias: DepthBiasState::default(),
            blend_constants: [0.0; 4],
            depth_bounds: (0.0, 1.0),
            stencil_compare_mask: StencilMask::default(),
            stencil_write_mask: StencilMask::default(),
            stencil_reference: StencilMask::default(),
            fragment_shading_rate: FragmentShadingRate::default(),
        }
    }

    /// Set viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewports.clear();
        self.viewports.push(viewport);
    }

    /// Set viewports.
    pub fn set_viewports(&mut self, viewports: Vec<Viewport>) {
        self.viewports = viewports;
    }

    /// Set scissor.
    pub fn set_scissor(&mut self, scissor: Scissor) {
        self.scissors.clear();
        self.scissors.push(scissor);
    }

    /// Set scissors.
    pub fn set_scissors(&mut self, scissors: Vec<Scissor>) {
        self.scissors = scissors;
    }

    /// Set line width.
    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    /// Set depth bias.
    pub fn set_depth_bias(&mut self, constant: f32, clamp: f32, slope: f32) {
        self.depth_bias = DepthBiasState::new(constant, clamp, slope);
    }

    /// Set blend constants.
    pub fn set_blend_constants(&mut self, constants: [f32; 4]) {
        self.blend_constants = constants;
    }

    /// Set depth bounds.
    pub fn set_depth_bounds(&mut self, min: f32, max: f32) {
        self.depth_bounds = (min, max);
    }

    /// Set stencil compare mask.
    pub fn set_stencil_compare_mask(&mut self, front: u32, back: u32) {
        self.stencil_compare_mask = StencilMask { front, back };
    }

    /// Set stencil write mask.
    pub fn set_stencil_write_mask(&mut self, front: u32, back: u32) {
        self.stencil_write_mask = StencilMask { front, back };
    }

    /// Set stencil reference.
    pub fn set_stencil_reference(&mut self, front: u32, back: u32) {
        self.stencil_reference = StencilMask { front, back };
    }

    /// Set fragment shading rate.
    pub fn set_fragment_shading_rate(&mut self, rate: FragmentShadingRate) {
        self.fragment_shading_rate = rate;
    }

    /// Check if state is dirty.
    pub fn is_dirty(&self, flags: DynamicStateFlags) -> bool {
        self.dynamic_state.contains(flags)
    }

    /// Mark state as dirty.
    pub fn mark_dirty(&mut self, flags: DynamicStateFlags) {
        self.dynamic_state = self.dynamic_state.or(flags);
    }

    /// Clear dirty flags.
    pub fn clear_dirty(&mut self) {
        self.dynamic_state = DynamicStateFlags::NONE;
    }
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Depth Bias State
// ============================================================================

/// Depth bias state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DepthBiasState {
    /// Constant factor.
    pub constant_factor: f32,
    /// Clamp value.
    pub clamp: f32,
    /// Slope factor.
    pub slope_factor: f32,
}

impl DepthBiasState {
    /// Create a new depth bias state.
    pub fn new(constant_factor: f32, clamp: f32, slope_factor: f32) -> Self {
        Self {
            constant_factor,
            clamp,
            slope_factor,
        }
    }

    /// Check if bias is enabled.
    pub fn is_enabled(&self) -> bool {
        self.constant_factor != 0.0 || self.slope_factor != 0.0
    }

    /// Standard shadow bias.
    pub fn shadow() -> Self {
        Self::new(1.25, 0.0, 1.75)
    }
}

// ============================================================================
// Stencil Mask
// ============================================================================

/// Stencil mask for front and back faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StencilMask {
    /// Front face mask.
    pub front: u32,
    /// Back face mask.
    pub back: u32,
}

impl StencilMask {
    /// Create a new stencil mask.
    pub fn new(front: u32, back: u32) -> Self {
        Self { front, back }
    }

    /// Create same mask for both faces.
    pub fn both(value: u32) -> Self {
        Self::new(value, value)
    }

    /// Full mask (0xFF).
    pub fn full() -> Self {
        Self::both(0xFF)
    }
}

// ============================================================================
// Fragment Shading Rate
// ============================================================================

/// Fragment shading rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FragmentShadingRate {
    /// Width (1, 2, or 4).
    pub width: u8,
    /// Height (1, 2, or 4).
    pub height: u8,
}

impl FragmentShadingRate {
    /// 1x1 (full rate).
    pub const RATE_1X1: Self = Self {
        width: 1,
        height: 1,
    };
    /// 1x2.
    pub const RATE_1X2: Self = Self {
        width: 1,
        height: 2,
    };
    /// 2x1.
    pub const RATE_2X1: Self = Self {
        width: 2,
        height: 1,
    };
    /// 2x2.
    pub const RATE_2X2: Self = Self {
        width: 2,
        height: 2,
    };
    /// 2x4.
    pub const RATE_2X4: Self = Self {
        width: 2,
        height: 4,
    };
    /// 4x2.
    pub const RATE_4X2: Self = Self {
        width: 4,
        height: 2,
    };
    /// 4x4.
    pub const RATE_4X4: Self = Self {
        width: 4,
        height: 4,
    };

    /// Create a new shading rate.
    pub fn new(width: u8, height: u8) -> Self {
        Self { width, height }
    }

    /// Get pixel count per fragment.
    pub fn pixel_count(&self) -> u32 {
        self.width as u32 * self.height as u32
    }
}

// ============================================================================
// State Stack
// ============================================================================

/// State stack for save/restore.
pub struct StateStack {
    /// Saved states.
    stack: Vec<PipelineState>,
}

impl StateStack {
    /// Create a new state stack.
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push current state.
    pub fn push(&mut self, state: &PipelineState) {
        self.stack.push(state.clone());
    }

    /// Pop and restore state.
    pub fn pop(&mut self) -> Option<PipelineState> {
        self.stack.pop()
    }

    /// Peek at top state.
    pub fn peek(&self) -> Option<&PipelineState> {
        self.stack.last()
    }

    /// Get stack depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Clear the stack.
    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

impl Default for StateStack {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// State Tracker
// ============================================================================

/// State tracker for minimizing state changes.
pub struct StateTracker {
    /// Current state.
    current: PipelineState,
    /// Pending state.
    pending: PipelineState,
    /// Changed flags.
    changed: DynamicStateFlags,
}

impl StateTracker {
    /// Create a new state tracker.
    pub fn new() -> Self {
        Self {
            current: PipelineState::new(),
            pending: PipelineState::new(),
            changed: DynamicStateFlags::NONE,
        }
    }

    /// Set pending viewport.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        if self.pending.viewports.first() != Some(&viewport) {
            self.pending.set_viewport(viewport);
            self.changed = self.changed.or(DynamicStateFlags::VIEWPORT);
        }
    }

    /// Set pending scissor.
    pub fn set_scissor(&mut self, scissor: Scissor) {
        if self.pending.scissors.first() != Some(&scissor) {
            self.pending.set_scissor(scissor);
            self.changed = self.changed.or(DynamicStateFlags::SCISSOR);
        }
    }

    /// Set pending line width.
    pub fn set_line_width(&mut self, width: f32) {
        if (self.pending.line_width - width).abs() > f32::EPSILON {
            self.pending.line_width = width;
            self.changed = self.changed.or(DynamicStateFlags::LINE_WIDTH);
        }
    }

    /// Set pending depth bias.
    pub fn set_depth_bias(&mut self, state: DepthBiasState) {
        if self.pending.depth_bias != state {
            self.pending.depth_bias = state;
            self.changed = self.changed.or(DynamicStateFlags::DEPTH_BIAS);
        }
    }

    /// Set pending blend constants.
    pub fn set_blend_constants(&mut self, constants: [f32; 4]) {
        if self.pending.blend_constants != constants {
            self.pending.blend_constants = constants;
            self.changed = self.changed.or(DynamicStateFlags::BLEND_CONSTANTS);
        }
    }

    /// Get changed flags.
    pub fn changed(&self) -> DynamicStateFlags {
        self.changed
    }

    /// Flush pending changes.
    pub fn flush(&mut self) -> Option<&PipelineState> {
        if self.changed.any() {
            self.current = self.pending.clone();
            self.changed = DynamicStateFlags::NONE;
            Some(&self.current)
        } else {
            None
        }
    }

    /// Get current state.
    pub fn current(&self) -> &PipelineState {
        &self.current
    }

    /// Reset tracker.
    pub fn reset(&mut self) {
        self.current = PipelineState::new();
        self.pending = PipelineState::new();
        self.changed = DynamicStateFlags::NONE;
    }
}

impl Default for StateTracker {
    fn default() -> Self {
        Self::new()
    }
}
