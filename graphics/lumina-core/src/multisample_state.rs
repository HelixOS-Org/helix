//! Multisample State Types for Lumina
//!
//! This module provides comprehensive multisample configuration,
//! sample shading, and sample coverage state.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Pipeline Multisample State
// ============================================================================

/// Multisample state create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct MultisampleStateCreateInfo {
    /// Flags
    pub flags: MultisampleStateCreateFlags,
    /// Rasterization samples
    pub rasterization_samples: SampleCountFlags,
    /// Sample shading enable
    pub sample_shading_enable: bool,
    /// Minimum sample shading
    pub min_sample_shading: f32,
    /// Sample mask
    pub sample_mask: Option<Vec<u32>>,
    /// Alpha to coverage enable
    pub alpha_to_coverage_enable: bool,
    /// Alpha to one enable
    pub alpha_to_one_enable: bool,
}

impl MultisampleStateCreateInfo {
    /// Creates new info (no MSAA)
    #[inline]
    pub fn new() -> Self {
        Self {
            flags: MultisampleStateCreateFlags::NONE,
            rasterization_samples: SampleCountFlags::COUNT_1,
            sample_shading_enable: false,
            min_sample_shading: 1.0,
            sample_mask: None,
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }

    /// No MSAA (1 sample)
    pub const NONE: Self = Self {
        flags: MultisampleStateCreateFlags::NONE,
        rasterization_samples: SampleCountFlags::COUNT_1,
        sample_shading_enable: false,
        min_sample_shading: 1.0,
        sample_mask: None,
        alpha_to_coverage_enable: false,
        alpha_to_one_enable: false,
    };

    /// 2x MSAA
    #[inline]
    pub fn msaa_2x() -> Self {
        Self {
            rasterization_samples: SampleCountFlags::COUNT_2,
            ..Self::new()
        }
    }

    /// 4x MSAA
    #[inline]
    pub fn msaa_4x() -> Self {
        Self {
            rasterization_samples: SampleCountFlags::COUNT_4,
            ..Self::new()
        }
    }

    /// 8x MSAA
    #[inline]
    pub fn msaa_8x() -> Self {
        Self {
            rasterization_samples: SampleCountFlags::COUNT_8,
            ..Self::new()
        }
    }

    /// 16x MSAA
    #[inline]
    pub fn msaa_16x() -> Self {
        Self {
            rasterization_samples: SampleCountFlags::COUNT_16,
            ..Self::new()
        }
    }

    /// With samples
    #[inline]
    pub fn with_samples(mut self, samples: SampleCountFlags) -> Self {
        self.rasterization_samples = samples;
        self
    }

    /// With sample shading
    #[inline]
    pub fn with_sample_shading(mut self, min_sample_shading: f32) -> Self {
        self.sample_shading_enable = true;
        self.min_sample_shading = min_sample_shading;
        self
    }

    /// With sample mask
    #[inline]
    pub fn with_sample_mask(mut self, mask: Vec<u32>) -> Self {
        self.sample_mask = Some(mask);
        self
    }

    /// With alpha to coverage
    #[inline]
    pub fn with_alpha_to_coverage(mut self) -> Self {
        self.alpha_to_coverage_enable = true;
        self
    }

    /// With alpha to one
    #[inline]
    pub fn with_alpha_to_one(mut self) -> Self {
        self.alpha_to_one_enable = true;
        self
    }

    /// With flags
    #[inline]
    pub fn with_flags(mut self, flags: MultisampleStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for MultisampleStateCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Multisample state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MultisampleStateCreateFlags(pub u32);

impl MultisampleStateCreateFlags {
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

// ============================================================================
// Sample Count Flags
// ============================================================================

/// Sample count flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SampleCountFlags(pub u32);

impl SampleCountFlags {
    /// 1 sample (no MSAA)
    pub const COUNT_1: Self = Self(1 << 0);
    /// 2 samples
    pub const COUNT_2: Self = Self(1 << 1);
    /// 4 samples
    pub const COUNT_4: Self = Self(1 << 2);
    /// 8 samples
    pub const COUNT_8: Self = Self(1 << 3);
    /// 16 samples
    pub const COUNT_16: Self = Self(1 << 4);
    /// 32 samples
    pub const COUNT_32: Self = Self(1 << 5);
    /// 64 samples
    pub const COUNT_64: Self = Self(1 << 6);

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

    /// As count
    #[inline]
    pub const fn as_count(&self) -> u32 {
        match self.0 {
            1 => 1,
            2 => 2,
            4 => 4,
            8 => 8,
            16 => 16,
            32 => 32,
            64 => 64,
            _ => 1,
        }
    }

    /// From count
    #[inline]
    pub const fn from_count(count: u32) -> Self {
        match count {
            1 => Self::COUNT_1,
            2 => Self::COUNT_2,
            4 => Self::COUNT_4,
            8 => Self::COUNT_8,
            16 => Self::COUNT_16,
            32 => Self::COUNT_32,
            64 => Self::COUNT_64,
            _ => Self::COUNT_1,
        }
    }

    /// Is MSAA enabled
    #[inline]
    pub const fn is_msaa(&self) -> bool {
        self.0 > 1
    }

    /// Maximum samples
    #[inline]
    pub const fn max_samples(&self) -> u32 {
        if self.contains(Self::COUNT_64) {
            64
        } else if self.contains(Self::COUNT_32) {
            32
        } else if self.contains(Self::COUNT_16) {
            16
        } else if self.contains(Self::COUNT_8) {
            8
        } else if self.contains(Self::COUNT_4) {
            4
        } else if self.contains(Self::COUNT_2) {
            2
        } else {
            1
        }
    }

    /// Required sample mask words
    #[inline]
    pub const fn sample_mask_words(&self) -> u32 {
        (self.as_count() + 31) / 32
    }
}

// ============================================================================
// Sample Location
// ============================================================================

/// Sample location
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct SampleLocation {
    /// X position (0.0 to 1.0)
    pub x: f32,
    /// Y position (0.0 to 1.0)
    pub y: f32,
}

impl SampleLocation {
    /// Center (0.5, 0.5)
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };

    /// Creates new sample location
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// From pixel coords (0-16 to 0.0-1.0)
    #[inline]
    pub const fn from_subpixel(x: u8, y: u8) -> Self {
        Self {
            x: (x as f32) / 16.0,
            y: (y as f32) / 16.0,
        }
    }
}

/// Standard sample locations for 2x MSAA
pub const SAMPLE_LOCATIONS_2X: [SampleLocation; 2] = [
    SampleLocation { x: 0.75, y: 0.75 },
    SampleLocation { x: 0.25, y: 0.25 },
];

/// Standard sample locations for 4x MSAA
pub const SAMPLE_LOCATIONS_4X: [SampleLocation; 4] = [
    SampleLocation { x: 0.375, y: 0.125 },
    SampleLocation { x: 0.875, y: 0.375 },
    SampleLocation { x: 0.125, y: 0.625 },
    SampleLocation { x: 0.625, y: 0.875 },
];

/// Standard sample locations for 8x MSAA
pub const SAMPLE_LOCATIONS_8X: [SampleLocation; 8] = [
    SampleLocation { x: 0.5625, y: 0.3125 },
    SampleLocation { x: 0.4375, y: 0.6875 },
    SampleLocation { x: 0.8125, y: 0.5625 },
    SampleLocation { x: 0.3125, y: 0.1875 },
    SampleLocation { x: 0.1875, y: 0.8125 },
    SampleLocation { x: 0.0625, y: 0.4375 },
    SampleLocation { x: 0.6875, y: 0.9375 },
    SampleLocation { x: 0.9375, y: 0.0625 },
];

// ============================================================================
// Sample Locations Info
// ============================================================================

/// Sample locations info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SampleLocationsInfo<'a> {
    /// Sample locations per pixel
    pub sample_locations_per_pixel: SampleCountFlags,
    /// Sample location grid size
    pub sample_location_grid_size: Extent2D,
    /// Sample locations
    pub sample_locations: &'a [SampleLocation],
}

impl<'a> SampleLocationsInfo<'a> {
    /// Creates new info
    #[inline]
    pub const fn new(
        samples_per_pixel: SampleCountFlags,
        grid_size: Extent2D,
        locations: &'a [SampleLocation],
    ) -> Self {
        Self {
            sample_locations_per_pixel: samples_per_pixel,
            sample_location_grid_size: grid_size,
            sample_locations: locations,
        }
    }

    /// Standard 2x
    #[inline]
    pub const fn standard_2x() -> SampleLocationsInfo<'static> {
        SampleLocationsInfo {
            sample_locations_per_pixel: SampleCountFlags::COUNT_2,
            sample_location_grid_size: Extent2D::UNIT,
            sample_locations: &SAMPLE_LOCATIONS_2X,
        }
    }

    /// Standard 4x
    #[inline]
    pub const fn standard_4x() -> SampleLocationsInfo<'static> {
        SampleLocationsInfo {
            sample_locations_per_pixel: SampleCountFlags::COUNT_4,
            sample_location_grid_size: Extent2D::UNIT,
            sample_locations: &SAMPLE_LOCATIONS_4X,
        }
    }

    /// Standard 8x
    #[inline]
    pub const fn standard_8x() -> SampleLocationsInfo<'static> {
        SampleLocationsInfo {
            sample_locations_per_pixel: SampleCountFlags::COUNT_8,
            sample_location_grid_size: Extent2D::UNIT,
            sample_locations: &SAMPLE_LOCATIONS_8X,
        }
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
    /// Unit (1x1)
    pub const UNIT: Self = Self {
        width: 1,
        height: 1,
    };

    /// Creates new extent
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

// ============================================================================
// Coverage Reduction Mode
// ============================================================================

/// Coverage reduction mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CoverageReductionMode {
    /// Merge coverage
    #[default]
    Merge = 0,
    /// Truncate coverage
    Truncate = 1,
}

/// Coverage reduction state create info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct CoverageReductionStateCreateInfo {
    /// Flags
    pub flags: CoverageReductionStateCreateFlags,
    /// Coverage reduction mode
    pub coverage_reduction_mode: CoverageReductionMode,
}

impl CoverageReductionStateCreateInfo {
    /// Default (merge)
    pub const DEFAULT: Self = Self {
        flags: CoverageReductionStateCreateFlags::NONE,
        coverage_reduction_mode: CoverageReductionMode::Merge,
    };

    /// Creates new info
    #[inline]
    pub const fn new(mode: CoverageReductionMode) -> Self {
        Self {
            flags: CoverageReductionStateCreateFlags::NONE,
            coverage_reduction_mode: mode,
        }
    }
}

impl Default for CoverageReductionStateCreateInfo {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Coverage reduction state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CoverageReductionStateCreateFlags(pub u32);

impl CoverageReductionStateCreateFlags {
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

// ============================================================================
// Coverage Modulation
// ============================================================================

/// Coverage modulation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CoverageModulationMode {
    /// None
    #[default]
    None = 0,
    /// RGB
    Rgb = 1,
    /// Alpha
    Alpha = 2,
    /// RGBA
    Rgba = 3,
}

/// Coverage modulation state create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct CoverageModulationStateCreateInfo {
    /// Flags
    pub flags: CoverageModulationStateCreateFlags,
    /// Coverage modulation mode
    pub coverage_modulation_mode: CoverageModulationMode,
    /// Coverage modulation table enable
    pub coverage_modulation_table_enable: bool,
    /// Coverage modulation table
    pub coverage_modulation_table: Vec<f32>,
}

impl CoverageModulationStateCreateInfo {
    /// Creates new info (disabled)
    #[inline]
    pub fn new() -> Self {
        Self {
            flags: CoverageModulationStateCreateFlags::NONE,
            coverage_modulation_mode: CoverageModulationMode::None,
            coverage_modulation_table_enable: false,
            coverage_modulation_table: Vec::new(),
        }
    }

    /// With mode
    #[inline]
    pub fn with_mode(mut self, mode: CoverageModulationMode) -> Self {
        self.coverage_modulation_mode = mode;
        self
    }

    /// With modulation table
    #[inline]
    pub fn with_table(mut self, table: Vec<f32>) -> Self {
        self.coverage_modulation_table_enable = true;
        self.coverage_modulation_table = table;
        self
    }
}

impl Default for CoverageModulationStateCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage modulation state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CoverageModulationStateCreateFlags(pub u32);

impl CoverageModulationStateCreateFlags {
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

// ============================================================================
// Resolve Mode
// ============================================================================

/// Resolve mode flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ResolveModeFlags(pub u32);

impl ResolveModeFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Sample zero
    pub const SAMPLE_ZERO: Self = Self(1 << 0);
    /// Average
    pub const AVERAGE: Self = Self(1 << 1);
    /// Min
    pub const MIN: Self = Self(1 << 2);
    /// Max
    pub const MAX: Self = Self(1 << 3);

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

    /// Is average resolve
    #[inline]
    pub const fn is_average(&self) -> bool {
        self.contains(Self::AVERAGE)
    }
}

// ============================================================================
// MSAA Quality Settings
// ============================================================================

/// MSAA quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MsaaQuality {
    /// Off (1 sample)
    #[default]
    Off = 0,
    /// Low (2 samples)
    Low = 1,
    /// Medium (4 samples)
    Medium = 2,
    /// High (8 samples)
    High = 3,
    /// Ultra (16 samples)
    Ultra = 4,
}

impl MsaaQuality {
    /// Get sample count
    #[inline]
    pub const fn sample_count(&self) -> SampleCountFlags {
        match self {
            Self::Off => SampleCountFlags::COUNT_1,
            Self::Low => SampleCountFlags::COUNT_2,
            Self::Medium => SampleCountFlags::COUNT_4,
            Self::High => SampleCountFlags::COUNT_8,
            Self::Ultra => SampleCountFlags::COUNT_16,
        }
    }

    /// Get sample count as u32
    #[inline]
    pub const fn samples(&self) -> u32 {
        match self {
            Self::Off => 1,
            Self::Low => 2,
            Self::Medium => 4,
            Self::High => 8,
            Self::Ultra => 16,
        }
    }

    /// Create multisample state
    #[inline]
    pub fn create_state(&self) -> MultisampleStateCreateInfo {
        MultisampleStateCreateInfo {
            rasterization_samples: self.sample_count(),
            ..MultisampleStateCreateInfo::new()
        }
    }

    /// From sample count
    #[inline]
    pub const fn from_samples(samples: u32) -> Self {
        match samples {
            1 => Self::Off,
            2 => Self::Low,
            4 => Self::Medium,
            8 => Self::High,
            16 | _ => Self::Ultra,
        }
    }
}

// ============================================================================
// Sample Shading Settings
// ============================================================================

/// Sample shading preset
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct SampleShadingSettings {
    /// Enable sample shading
    pub enable: bool,
    /// Minimum sample shading (0.0 to 1.0)
    pub min_sample_shading: f32,
}

impl SampleShadingSettings {
    /// Disabled
    pub const DISABLED: Self = Self {
        enable: false,
        min_sample_shading: 0.0,
    };

    /// Full sample shading (every sample runs fragment shader)
    pub const FULL: Self = Self {
        enable: true,
        min_sample_shading: 1.0,
    };

    /// Half sample shading
    pub const HALF: Self = Self {
        enable: true,
        min_sample_shading: 0.5,
    };

    /// Quarter sample shading
    pub const QUARTER: Self = Self {
        enable: true,
        min_sample_shading: 0.25,
    };

    /// Creates new settings
    #[inline]
    pub const fn new(min_sample_shading: f32) -> Self {
        Self {
            enable: true,
            min_sample_shading,
        }
    }
}

impl Default for SampleShadingSettings {
    fn default() -> Self {
        Self::DISABLED
    }
}

// ============================================================================
// Alpha Coverage Settings
// ============================================================================

/// Alpha coverage settings
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct AlphaCoverageSettings {
    /// Alpha to coverage enable
    pub alpha_to_coverage: bool,
    /// Alpha to one enable
    pub alpha_to_one: bool,
}

impl AlphaCoverageSettings {
    /// Disabled
    pub const DISABLED: Self = Self {
        alpha_to_coverage: false,
        alpha_to_one: false,
    };

    /// Alpha to coverage enabled
    pub const ALPHA_TO_COVERAGE: Self = Self {
        alpha_to_coverage: true,
        alpha_to_one: false,
    };

    /// Both enabled
    pub const BOTH: Self = Self {
        alpha_to_coverage: true,
        alpha_to_one: true,
    };

    /// Creates new settings
    #[inline]
    pub const fn new(alpha_to_coverage: bool, alpha_to_one: bool) -> Self {
        Self {
            alpha_to_coverage,
            alpha_to_one,
        }
    }
}
