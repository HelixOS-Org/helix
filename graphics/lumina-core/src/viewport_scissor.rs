//! Viewport and Scissor Types for Lumina
//!
//! This module provides viewport configuration, scissor rectangles,
//! and dynamic viewport state.

// ============================================================================
// Viewport
// ============================================================================

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

impl Viewport {
    /// Creates new viewport
    #[inline]
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

    /// Creates viewport from dimensions
    #[inline]
    pub const fn from_dimensions(width: f32, height: f32) -> Self {
        Self::new(0.0, 0.0, width, height)
    }

    /// Creates viewport from extent
    #[inline]
    pub const fn from_extent(extent: Extent2D) -> Self {
        Self::new(0.0, 0.0, extent.width as f32, extent.height as f32)
    }

    /// Creates full screen viewport (0,0 to width,height)
    #[inline]
    pub const fn full_screen(width: u32, height: u32) -> Self {
        Self::new(0.0, 0.0, width as f32, height as f32)
    }

    /// With depth range
    #[inline]
    pub const fn with_depth(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }

    /// With reversed depth (for better precision)
    #[inline]
    pub const fn with_reversed_depth(mut self) -> Self {
        self.min_depth = 1.0;
        self.max_depth = 0.0;
        self
    }

    /// With flipped Y (Vulkan convention)
    #[inline]
    pub const fn with_flipped_y(mut self) -> Self {
        self.y = self.height;
        self.height = -self.height;
        self
    }

    /// Aspect ratio
    #[inline]
    pub const fn aspect_ratio(&self) -> f32 {
        if self.height == 0.0 || self.height == -0.0 {
            1.0
        } else {
            let h = if self.height < 0.0 {
                -self.height
            } else {
                self.height
            };
            self.width / h
        }
    }

    /// Right edge
    #[inline]
    pub const fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge
    #[inline]
    pub const fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Center X
    #[inline]
    pub const fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    /// Center Y
    #[inline]
    pub const fn center_y(&self) -> f32 {
        self.y + self.height / 2.0
    }

    /// To scissor
    #[inline]
    pub const fn to_scissor(&self) -> Scissor {
        Scissor {
            offset: Offset2D {
                x: self.x as i32,
                y: if self.height < 0.0 {
                    (self.y + self.height) as i32
                } else {
                    self.y as i32
                },
            },
            extent: Extent2D {
                width: self.width as u32,
                height: if self.height < 0.0 {
                    (-self.height) as u32
                } else {
                    self.height as u32
                },
            },
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(0.0, 0.0, 800.0, 600.0)
    }
}

// ============================================================================
// Scissor
// ============================================================================

/// Scissor rectangle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Scissor {
    /// Offset
    pub offset: Offset2D,
    /// Extent
    pub extent: Extent2D,
}

impl Scissor {
    /// Creates new scissor
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D { x, y },
            extent: Extent2D { width, height },
        }
    }

    /// Creates scissor from extent (at origin)
    #[inline]
    pub const fn from_extent(extent: Extent2D) -> Self {
        Self {
            offset: Offset2D::ZERO,
            extent,
        }
    }

    /// Creates full screen scissor
    #[inline]
    pub const fn full_screen(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Right edge
    #[inline]
    pub const fn right(&self) -> i32 {
        self.offset.x + self.extent.width as i32
    }

    /// Bottom edge
    #[inline]
    pub const fn bottom(&self) -> i32 {
        self.offset.y + self.extent.height as i32
    }

    /// Area
    #[inline]
    pub const fn area(&self) -> u64 {
        self.extent.width as u64 * self.extent.height as u64
    }

    /// Contains point
    #[inline]
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.offset.x
            && x < self.right()
            && y >= self.offset.y
            && y < self.bottom()
    }

    /// Intersects with another scissor
    #[inline]
    pub const fn intersects(&self, other: &Scissor) -> bool {
        self.offset.x < other.right()
            && self.right() > other.offset.x
            && self.offset.y < other.bottom()
            && self.bottom() > other.offset.y
    }

    /// Intersection with another scissor
    #[inline]
    pub const fn intersection(&self, other: &Scissor) -> Scissor {
        let x = if self.offset.x > other.offset.x {
            self.offset.x
        } else {
            other.offset.x
        };
        let y = if self.offset.y > other.offset.y {
            self.offset.y
        } else {
            other.offset.y
        };
        let right = if self.right() < other.right() {
            self.right()
        } else {
            other.right()
        };
        let bottom = if self.bottom() < other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        let width = if right > x { (right - x) as u32 } else { 0 };
        let height = if bottom > y { (bottom - y) as u32 } else { 0 };

        Self::new(x, y, width, height)
    }

    /// Union with another scissor (bounding box)
    #[inline]
    pub const fn union(&self, other: &Scissor) -> Scissor {
        let x = if self.offset.x < other.offset.x {
            self.offset.x
        } else {
            other.offset.x
        };
        let y = if self.offset.y < other.offset.y {
            self.offset.y
        } else {
            other.offset.y
        };
        let right = if self.right() > other.right() {
            self.right()
        } else {
            other.right()
        };
        let bottom = if self.bottom() > other.bottom() {
            self.bottom()
        } else {
            other.bottom()
        };

        Self::new(x, y, (right - x) as u32, (bottom - y) as u32)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.extent.width == 0 || self.extent.height == 0
    }
}

// ============================================================================
// Extent 2D
// ============================================================================

/// 2D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Extent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Extent2D {
    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Unit extent (1x1)
    pub const UNIT: Self = Self::new(1, 1);

    /// Common resolutions
    pub const R_480P: Self = Self::new(640, 480);
    pub const R_720P: Self = Self::new(1280, 720);
    pub const R_1080P: Self = Self::new(1920, 1080);
    pub const R_1440P: Self = Self::new(2560, 1440);
    pub const R_4K: Self = Self::new(3840, 2160);
    pub const R_8K: Self = Self::new(7680, 4320);

    /// Area
    #[inline]
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Aspect ratio
    #[inline]
    pub const fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Half size
    #[inline]
    pub const fn half(&self) -> Self {
        Self::new(self.width / 2, self.height / 2)
    }

    /// Double size
    #[inline]
    pub const fn double(&self) -> Self {
        Self::new(self.width * 2, self.height * 2)
    }
}

// ============================================================================
// Offset 2D
// ============================================================================

/// 2D offset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Offset2D {
    /// X
    pub x: i32,
    /// Y
    pub y: i32,
}

impl Offset2D {
    /// Zero offset
    pub const ZERO: Self = Self { x: 0, y: 0 };

    /// Creates new offset
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Add offsets
    #[inline]
    pub const fn add(&self, other: &Offset2D) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    /// Subtract offsets
    #[inline]
    pub const fn sub(&self, other: &Offset2D) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    /// Negate
    #[inline]
    pub const fn neg(&self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

// ============================================================================
// Rect 2D
// ============================================================================

/// 2D rectangle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Rect2D {
    /// Offset
    pub offset: Offset2D,
    /// Extent
    pub extent: Extent2D,
}

impl Rect2D {
    /// Creates new rectangle
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D { x, y },
            extent: Extent2D { width, height },
        }
    }

    /// Creates from extent at origin
    #[inline]
    pub const fn from_extent(extent: Extent2D) -> Self {
        Self {
            offset: Offset2D::ZERO,
            extent,
        }
    }

    /// To scissor
    #[inline]
    pub const fn to_scissor(&self) -> Scissor {
        Scissor {
            offset: self.offset,
            extent: self.extent,
        }
    }

    /// Right edge
    #[inline]
    pub const fn right(&self) -> i32 {
        self.offset.x + self.extent.width as i32
    }

    /// Bottom edge
    #[inline]
    pub const fn bottom(&self) -> i32 {
        self.offset.y + self.extent.height as i32
    }

    /// Area
    #[inline]
    pub const fn area(&self) -> u64 {
        self.extent.area()
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.extent.is_empty()
    }
}

// ============================================================================
// Viewport State
// ============================================================================

/// Viewport state create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ViewportStateCreateInfo<'a> {
    /// Flags
    pub flags: ViewportStateCreateFlags,
    /// Viewports
    pub viewports: &'a [Viewport],
    /// Scissors
    pub scissors: &'a [Scissor],
}

impl<'a> ViewportStateCreateInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(viewports: &'a [Viewport], scissors: &'a [Scissor]) -> Self {
        Self {
            flags: ViewportStateCreateFlags::NONE,
            viewports,
            scissors,
        }
    }

    /// Single viewport and scissor
    #[inline]
    pub const fn single(viewport: &'a [Viewport], scissor: &'a [Scissor]) -> Self {
        Self::new(viewport, scissor)
    }

    /// Dynamic (no viewports/scissors, will be set dynamically)
    #[inline]
    pub const fn dynamic(count: u32) -> DynamicViewportState {
        DynamicViewportState {
            viewport_count: count,
            scissor_count: count,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: ViewportStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for ViewportStateCreateInfo<'_> {
    fn default() -> Self {
        Self::new(&[], &[])
    }
}

/// Viewport state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ViewportStateCreateFlags(pub u32);

impl ViewportStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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

/// Dynamic viewport state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DynamicViewportState {
    /// Viewport count
    pub viewport_count: u32,
    /// Scissor count
    pub scissor_count: u32,
}

impl DynamicViewportState {
    /// Creates new state
    #[inline]
    pub const fn new(viewport_count: u32, scissor_count: u32) -> Self {
        Self {
            viewport_count,
            scissor_count,
        }
    }

    /// Single viewport and scissor
    pub const SINGLE: Self = Self::new(1, 1);
}

// ============================================================================
// Viewport W Scaling
// ============================================================================

/// Viewport W scaling
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ViewportWScaling {
    /// X coefficient
    pub xcoeff: f32,
    /// Y coefficient
    pub ycoeff: f32,
}

impl ViewportWScaling {
    /// Creates new W scaling
    #[inline]
    pub const fn new(xcoeff: f32, ycoeff: f32) -> Self {
        Self { xcoeff, ycoeff }
    }

    /// Identity (no scaling)
    pub const IDENTITY: Self = Self::new(1.0, 1.0);
}

// ============================================================================
// Viewport Swizzle
// ============================================================================

/// Viewport swizzle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ViewportSwizzle {
    /// X swizzle
    pub x: ViewportCoordinateSwizzle,
    /// Y swizzle
    pub y: ViewportCoordinateSwizzle,
    /// Z swizzle
    pub z: ViewportCoordinateSwizzle,
    /// W swizzle
    pub w: ViewportCoordinateSwizzle,
}

impl ViewportSwizzle {
    /// Identity swizzle
    pub const IDENTITY: Self = Self {
        x: ViewportCoordinateSwizzle::PositiveX,
        y: ViewportCoordinateSwizzle::PositiveY,
        z: ViewportCoordinateSwizzle::PositiveZ,
        w: ViewportCoordinateSwizzle::PositiveW,
    };

    /// Creates new swizzle
    #[inline]
    pub const fn new(
        x: ViewportCoordinateSwizzle,
        y: ViewportCoordinateSwizzle,
        z: ViewportCoordinateSwizzle,
        w: ViewportCoordinateSwizzle,
    ) -> Self {
        Self { x, y, z, w }
    }
}

impl Default for ViewportSwizzle {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Viewport coordinate swizzle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ViewportCoordinateSwizzle {
    /// +X
    #[default]
    PositiveX = 0,
    /// -X
    NegativeX = 1,
    /// +Y
    PositiveY = 2,
    /// -Y
    NegativeY = 3,
    /// +Z
    PositiveZ = 4,
    /// -Z
    NegativeZ = 5,
    /// +W
    PositiveW = 6,
    /// -W
    NegativeW = 7,
}

// ============================================================================
// Shading Rate
// ============================================================================

/// Fragment shading rate
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct FragmentShadingRate {
    /// Fragment size
    pub fragment_size: Extent2D,
    /// Combiner ops
    pub combiner_ops: [FragmentShadingRateCombinerOp; 2],
}

impl FragmentShadingRate {
    /// 1x1 (full rate)
    pub const FULL_RATE: Self = Self {
        fragment_size: Extent2D::UNIT,
        combiner_ops: [
            FragmentShadingRateCombinerOp::Keep,
            FragmentShadingRateCombinerOp::Keep,
        ],
    };

    /// 2x2 (quarter rate)
    pub const QUARTER_RATE: Self = Self {
        fragment_size: Extent2D::new(2, 2),
        combiner_ops: [
            FragmentShadingRateCombinerOp::Keep,
            FragmentShadingRateCombinerOp::Keep,
        ],
    };

    /// 4x4 (1/16 rate)
    pub const SIXTEENTH_RATE: Self = Self {
        fragment_size: Extent2D::new(4, 4),
        combiner_ops: [
            FragmentShadingRateCombinerOp::Keep,
            FragmentShadingRateCombinerOp::Keep,
        ],
    };

    /// Creates new shading rate
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            fragment_size: Extent2D::new(width, height),
            combiner_ops: [
                FragmentShadingRateCombinerOp::Keep,
                FragmentShadingRateCombinerOp::Keep,
            ],
        }
    }

    /// With combiner ops
    #[inline]
    pub const fn with_combiner_ops(
        mut self,
        primitive_op: FragmentShadingRateCombinerOp,
        attachment_op: FragmentShadingRateCombinerOp,
    ) -> Self {
        self.combiner_ops = [primitive_op, attachment_op];
        self
    }
}

/// Fragment shading rate combiner op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FragmentShadingRateCombinerOp {
    /// Keep current
    #[default]
    Keep = 0,
    /// Replace with new
    Replace = 1,
    /// Minimum
    Min = 2,
    /// Maximum
    Max = 3,
    /// Multiply
    Mul = 4,
}

// ============================================================================
// Coarse Sample Order
// ============================================================================

/// Coarse sample order type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CoarseSampleOrderType {
    /// Default
    #[default]
    Default = 0,
    /// Custom
    Custom = 1,
    /// Pixel major
    PixelMajor = 2,
    /// Sample major
    SampleMajor = 3,
}

/// Coarse sample location
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct CoarseSampleLocation {
    /// Pixel X
    pub pixel_x: u32,
    /// Pixel Y
    pub pixel_y: u32,
    /// Sample
    pub sample: u32,
}

impl CoarseSampleLocation {
    /// Creates new location
    #[inline]
    pub const fn new(pixel_x: u32, pixel_y: u32, sample: u32) -> Self {
        Self {
            pixel_x,
            pixel_y,
            sample,
        }
    }
}

/// Coarse sample order custom
#[derive(Clone, Debug)]
#[repr(C)]
pub struct CoarseSampleOrderCustom<'a> {
    /// Shading rate
    pub shading_rate: ShadingRatePalette,
    /// Sample count
    pub sample_count: u32,
    /// Sample locations
    pub sample_locations: &'a [CoarseSampleLocation],
}

/// Shading rate palette
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadingRatePalette {
    /// No invocations
    #[default]
    NoInvocations = 0,
    /// 16 invocations per pixel
    N16InvocationsPerPixel = 1,
    /// 8 invocations per pixel
    N8InvocationsPerPixel = 2,
    /// 4 invocations per pixel
    N4InvocationsPerPixel = 3,
    /// 2 invocations per pixel
    N2InvocationsPerPixel = 4,
    /// 1 invocation per pixel
    N1InvocationPerPixel = 5,
    /// 1 invocation per 2x1 pixels
    N1InvocationPer2x1Pixels = 6,
    /// 1 invocation per 1x2 pixels
    N1InvocationPer1x2Pixels = 7,
    /// 1 invocation per 2x2 pixels
    N1InvocationPer2x2Pixels = 8,
    /// 1 invocation per 4x2 pixels
    N1InvocationPer4x2Pixels = 9,
    /// 1 invocation per 2x4 pixels
    N1InvocationPer2x4Pixels = 10,
    /// 1 invocation per 4x4 pixels
    N1InvocationPer4x4Pixels = 11,
}
