//! Multisample anti-aliasing types
//!
//! This module provides types for multisampling configuration.

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleCount {
    /// 1 sample (no multisampling)
    #[default]
    One = 1,
    /// 2 samples
    Two = 2,
    /// 4 samples
    Four = 4,
    /// 8 samples
    Eight = 8,
    /// 16 samples
    Sixteen = 16,
    /// 32 samples
    ThirtyTwo = 32,
    /// 64 samples
    SixtyFour = 64,
}

impl SampleCount {
    /// As integer
    pub const fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Is multisampled
    pub const fn is_multisampled(&self) -> bool {
        !matches!(self, Self::One)
    }

    /// From integer
    pub const fn from_u32(count: u32) -> Option<Self> {
        match count {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            4 => Some(Self::Four),
            8 => Some(Self::Eight),
            16 => Some(Self::Sixteen),
            32 => Some(Self::ThirtyTwo),
            64 => Some(Self::SixtyFour),
            _ => None,
        }
    }

    /// Next higher sample count
    pub const fn next(&self) -> Option<Self> {
        match self {
            Self::One => Some(Self::Two),
            Self::Two => Some(Self::Four),
            Self::Four => Some(Self::Eight),
            Self::Eight => Some(Self::Sixteen),
            Self::Sixteen => Some(Self::ThirtyTwo),
            Self::ThirtyTwo => Some(Self::SixtyFour),
            Self::SixtyFour => None,
        }
    }

    /// Previous lower sample count
    pub const fn prev(&self) -> Option<Self> {
        match self {
            Self::One => None,
            Self::Two => Some(Self::One),
            Self::Four => Some(Self::Two),
            Self::Eight => Some(Self::Four),
            Self::Sixteen => Some(Self::Eight),
            Self::ThirtyTwo => Some(Self::Sixteen),
            Self::SixtyFour => Some(Self::ThirtyTwo),
        }
    }
}

/// Sample mask
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SampleMask(pub u64);

impl SampleMask {
    /// All samples enabled
    pub const ALL: Self = Self(u64::MAX);
    /// No samples
    pub const NONE: Self = Self(0);

    /// Creates mask for N samples
    pub const fn for_count(count: SampleCount) -> Self {
        let bits = match count {
            SampleCount::One => 0x1,
            SampleCount::Two => 0x3,
            SampleCount::Four => 0xF,
            SampleCount::Eight => 0xFF,
            SampleCount::Sixteen => 0xFFFF,
            SampleCount::ThirtyTwo => 0xFFFF_FFFF,
            SampleCount::SixtyFour => u64::MAX,
        };
        Self(bits)
    }

    /// Creates from individual sample indices
    pub const fn from_indices(indices: &[u32]) -> Self {
        let mut mask = 0u64;
        let mut i = 0;
        while i < indices.len() {
            if indices[i] < 64 {
                mask |= 1 << indices[i];
            }
            i += 1;
        }
        Self(mask)
    }

    /// Checks if sample is enabled
    pub const fn is_enabled(&self, sample: u32) -> bool {
        if sample >= 64 {
            false
        } else {
            (self.0 & (1 << sample)) != 0
        }
    }

    /// Count of enabled samples
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl Default for SampleMask {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitAnd for SampleMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitOr for SampleMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Sample count
    pub sample_count: SampleCount,
    /// Enable sample shading
    pub sample_shading_enable: bool,
    /// Minimum sample shading (0.0 to 1.0)
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: SampleMask,
    /// Enable alpha to coverage
    pub alpha_to_coverage_enable: bool,
    /// Enable alpha to one
    pub alpha_to_one_enable: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self::disabled()
    }
}

impl MultisampleState {
    /// Disabled multisampling
    pub const fn disabled() -> Self {
        Self {
            sample_count: SampleCount::One,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: SampleMask::ALL,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// Creates with sample count
    pub const fn new(sample_count: SampleCount) -> Self {
        Self {
            sample_count,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: SampleMask::ALL,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// 2x MSAA
    pub const fn msaa_2x() -> Self {
        Self::new(SampleCount::Two)
    }

    /// 4x MSAA
    pub const fn msaa_4x() -> Self {
        Self::new(SampleCount::Four)
    }

    /// 8x MSAA
    pub const fn msaa_8x() -> Self {
        Self::new(SampleCount::Eight)
    }

    /// With sample shading (super-sampling)
    pub const fn with_sample_shading(mut self, min: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min;
        self
    }

    /// With alpha to coverage
    pub const fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }

    /// With alpha to one
    pub const fn with_alpha_to_one(mut self) -> Self {
        self.alpha_to_one_enable = true;
        self
    }

    /// With sample mask
    pub const fn with_sample_mask(mut self, mask: SampleMask) -> Self {
        self.sample_mask = mask;
        self
    }

    /// Is multisampled
    pub const fn is_multisampled(&self) -> bool {
        self.sample_count.is_multisampled()
    }
}

/// Sample position
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct SamplePosition {
    /// X position (0.0 to 1.0)
    pub x: f32,
    /// Y position (0.0 to 1.0)
    pub y: f32,
}

impl SamplePosition {
    /// Creates new position
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Center position
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };
}

/// Standard sample patterns
pub mod patterns {
    use super::SamplePosition;

    /// 2x MSAA pattern
    pub const MSAA_2X: [SamplePosition; 2] = [
        SamplePosition::new(0.75, 0.75),
        SamplePosition::new(0.25, 0.25),
    ];

    /// 4x MSAA pattern (rotated grid)
    pub const MSAA_4X: [SamplePosition; 4] = [
        SamplePosition::new(0.375, 0.125),
        SamplePosition::new(0.875, 0.375),
        SamplePosition::new(0.125, 0.625),
        SamplePosition::new(0.625, 0.875),
    ];

    /// 8x MSAA pattern
    pub const MSAA_8X: [SamplePosition; 8] = [
        SamplePosition::new(0.5625, 0.3125),
        SamplePosition::new(0.4375, 0.6875),
        SamplePosition::new(0.8125, 0.5625),
        SamplePosition::new(0.3125, 0.1875),
        SamplePosition::new(0.1875, 0.8125),
        SamplePosition::new(0.0625, 0.4375),
        SamplePosition::new(0.6875, 0.9375),
        SamplePosition::new(0.9375, 0.0625),
    ];
}

/// Resolve mode for multisampled images
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ResolveMode {
    /// No resolve
    None = 0,
    /// Average samples
    #[default]
    Average = 1,
    /// Sample 0
    SampleZero = 2,
    /// Minimum sample
    Min = 3,
    /// Maximum sample
    Max = 4,
}

/// Multisample resolve info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ResolveInfo {
    /// Resolve mode for color
    pub color_resolve_mode: ResolveMode,
    /// Resolve mode for depth
    pub depth_resolve_mode: ResolveMode,
    /// Resolve mode for stencil
    pub stencil_resolve_mode: ResolveMode,
}

impl Default for ResolveInfo {
    fn default() -> Self {
        Self {
            color_resolve_mode: ResolveMode::Average,
            depth_resolve_mode: ResolveMode::SampleZero,
            stencil_resolve_mode: ResolveMode::SampleZero,
        }
    }
}

impl ResolveInfo {
    /// Standard resolve (average color, sample 0 for depth/stencil)
    pub const fn standard() -> Self {
        Self {
            color_resolve_mode: ResolveMode::Average,
            depth_resolve_mode: ResolveMode::SampleZero,
            stencil_resolve_mode: ResolveMode::SampleZero,
        }
    }

    /// Min resolve for depth
    pub const fn depth_min() -> Self {
        Self {
            color_resolve_mode: ResolveMode::Average,
            depth_resolve_mode: ResolveMode::Min,
            stencil_resolve_mode: ResolveMode::SampleZero,
        }
    }

    /// Max resolve for depth
    pub const fn depth_max() -> Self {
        Self {
            color_resolve_mode: ResolveMode::Average,
            depth_resolve_mode: ResolveMode::Max,
            stencil_resolve_mode: ResolveMode::SampleZero,
        }
    }
}

/// Coverage modulation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CoverageModulationMode {
    /// No modulation
    #[default]
    None = 0,
    /// Modulate RGB
    Rgb = 1,
    /// Modulate Alpha
    Alpha = 2,
    /// Modulate RGBA
    Rgba = 3,
}

/// Coverage reduction mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum CoverageReductionMode {
    /// Merge samples
    #[default]
    Merge = 0,
    /// Truncate samples
    Truncate = 1,
}

/// Fragment shading rate (VRS)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FragmentShadingRate {
    /// 1x1 (full rate)
    #[default]
    Rate1x1 = 0,
    /// 1x2 (half vertical)
    Rate1x2 = 1,
    /// 2x1 (half horizontal)
    Rate2x1 = 2,
    /// 2x2 (quarter rate)
    Rate2x2 = 3,
    /// 2x4
    Rate2x4 = 4,
    /// 4x2
    Rate4x2 = 5,
    /// 4x4 (1/16 rate)
    Rate4x4 = 6,
}

impl FragmentShadingRate {
    /// Width in fragments
    pub const fn width(&self) -> u32 {
        match self {
            Self::Rate1x1 | Self::Rate1x2 => 1,
            Self::Rate2x1 | Self::Rate2x2 | Self::Rate2x4 => 2,
            Self::Rate4x2 | Self::Rate4x4 => 4,
        }
    }

    /// Height in fragments
    pub const fn height(&self) -> u32 {
        match self {
            Self::Rate1x1 | Self::Rate2x1 => 1,
            Self::Rate1x2 | Self::Rate2x2 | Self::Rate4x2 => 2,
            Self::Rate2x4 | Self::Rate4x4 => 4,
        }
    }

    /// Fragment count
    pub const fn fragment_count(&self) -> u32 {
        self.width() * self.height()
    }
}

/// Shading rate combiner operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ShadingRateCombinerOp {
    /// Keep the first rate
    #[default]
    Keep = 0,
    /// Replace with second rate
    Replace = 1,
    /// Use minimum of both
    Min = 2,
    /// Use maximum of both
    Max = 3,
    /// Multiply rates
    Mul = 4,
}
