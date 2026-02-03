//! GPU LOD (Level of Detail) System for Lumina
//!
//! This module provides comprehensive GPU-accelerated LOD management including
//! automatic LOD selection, crossfade transitions, and screen-size metrics.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// LOD System Handles
// ============================================================================

/// GPU LOD system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuLodSystemHandle(pub u64);

impl GpuLodSystemHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for GpuLodSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// LOD group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LodGroupHandle(pub u64);

impl LodGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for LodGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// LOD mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LodMeshHandle(pub u64);

impl LodMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LodMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// LOD override handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LodOverrideHandle(pub u64);

impl LodOverrideHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LodOverrideHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// LOD System Creation
// ============================================================================

/// GPU LOD system create info
#[derive(Clone, Debug)]
pub struct GpuLodSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max LOD groups
    pub max_lod_groups: u32,
    /// Max LOD levels
    pub max_lod_levels: u32,
    /// Features
    pub features: LodFeatures,
    /// Global settings
    pub global_settings: LodGlobalSettings,
}

impl GpuLodSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_lod_groups: 10000,
            max_lod_levels: 8,
            features: LodFeatures::all(),
            global_settings: LodGlobalSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max groups
    pub fn with_max_groups(mut self, count: u32) -> Self {
        self.max_lod_groups = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: LodFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With global settings
    pub fn with_settings(mut self, settings: LodGlobalSettings) -> Self {
        self.global_settings = settings;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new().with_settings(LodGlobalSettings::high_quality())
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_features(LodFeatures::BASIC)
            .with_settings(LodGlobalSettings::mobile())
    }
}

impl Default for GpuLodSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// LOD features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct LodFeatures: u32 {
        /// None
        const NONE = 0;
        /// Screen-size based selection
        const SCREEN_SIZE = 1 << 0;
        /// Distance-based selection
        const DISTANCE = 1 << 1;
        /// Crossfade transitions
        const CROSSFADE = 1 << 2;
        /// Dithered transitions
        const DITHER = 1 << 3;
        /// GPU selection (compute)
        const GPU_SELECT = 1 << 4;
        /// Hysteresis
        const HYSTERESIS = 1 << 5;
        /// LOD bias support
        const BIAS = 1 << 6;
        /// Per-camera LOD
        const PER_CAMERA = 1 << 7;
        /// Basic features
        const BASIC = Self::SCREEN_SIZE.bits() | Self::DISTANCE.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for LodFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Global Settings
// ============================================================================

/// LOD global settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LodGlobalSettings {
    /// LOD bias (positive = higher quality)
    pub lod_bias: f32,
    /// Max LOD level
    pub max_lod: u32,
    /// Force LOD (-1 = auto)
    pub force_lod: i32,
    /// Crossfade distance
    pub crossfade_distance: f32,
    /// Hysteresis factor (0-1)
    pub hysteresis: f32,
    /// Reference screen height
    pub reference_screen_height: f32,
    /// Cull ratio (screen size below which to cull)
    pub cull_ratio: f32,
}

impl LodGlobalSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            lod_bias: 0.0,
            max_lod: 7,
            force_lod: -1,
            crossfade_distance: 10.0,
            hysteresis: 0.1,
            reference_screen_height: 1080.0,
            cull_ratio: 0.001,
        }
    }

    /// With bias
    pub const fn with_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }

    /// With max LOD
    pub const fn with_max_lod(mut self, max: u32) -> Self {
        self.max_lod = max;
        self
    }

    /// With force LOD
    pub const fn with_force_lod(mut self, lod: i32) -> Self {
        self.force_lod = lod;
        self
    }

    /// With crossfade
    pub const fn with_crossfade(mut self, distance: f32) -> Self {
        self.crossfade_distance = distance;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self::new().with_bias(1.0).with_crossfade(15.0)
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            lod_bias: -1.0,
            max_lod: 4,
            force_lod: -1,
            crossfade_distance: 5.0,
            hysteresis: 0.15,
            reference_screen_height: 720.0,
            cull_ratio: 0.002,
        }
    }

    /// Performance preset
    pub const fn performance() -> Self {
        Self {
            lod_bias: -2.0,
            max_lod: 3,
            force_lod: -1,
            crossfade_distance: 0.0,
            hysteresis: 0.2,
            reference_screen_height: 1080.0,
            cull_ratio: 0.005,
        }
    }
}

impl Default for LodGlobalSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LOD Group
// ============================================================================

/// LOD group create info
#[derive(Clone, Debug)]
pub struct LodGroupCreateInfo {
    /// Name
    pub name: String,
    /// LOD levels
    pub levels: Vec<LodLevel>,
    /// Selection mode
    pub selection_mode: LodSelectionMode,
    /// Fade mode
    pub fade_mode: LodFadeMode,
    /// LOD bias
    pub lod_bias: f32,
}

impl LodGroupCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            levels: Vec::new(),
            selection_mode: LodSelectionMode::ScreenSize,
            fade_mode: LodFadeMode::Crossfade,
            lod_bias: 0.0,
        }
    }

    /// Add LOD level
    pub fn add_level(mut self, level: LodLevel) -> Self {
        self.levels.push(level);
        self
    }

    /// With levels
    pub fn with_levels(mut self, levels: Vec<LodLevel>) -> Self {
        self.levels = levels;
        self
    }

    /// With selection mode
    pub fn with_selection(mut self, mode: LodSelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// With fade mode
    pub fn with_fade(mut self, mode: LodFadeMode) -> Self {
        self.fade_mode = mode;
        self
    }

    /// With bias
    pub fn with_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }

    /// Simple 3-level LOD
    pub fn simple_3_lod(name: impl Into<String>) -> Self {
        Self::new(name)
            .add_level(LodLevel::new(0).with_screen_size(0.5))
            .add_level(LodLevel::new(1).with_screen_size(0.2))
            .add_level(LodLevel::new(2).with_screen_size(0.05))
    }

    /// Simple 4-level LOD
    pub fn simple_4_lod(name: impl Into<String>) -> Self {
        Self::new(name)
            .add_level(LodLevel::new(0).with_screen_size(0.6))
            .add_level(LodLevel::new(1).with_screen_size(0.3))
            .add_level(LodLevel::new(2).with_screen_size(0.1))
            .add_level(LodLevel::new(3).with_screen_size(0.02))
    }
}

impl Default for LodGroupCreateInfo {
    fn default() -> Self {
        Self::new("LODGroup")
    }
}

/// LOD selection mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LodSelectionMode {
    /// Screen-size based
    #[default]
    ScreenSize     = 0,
    /// Distance based
    Distance       = 1,
    /// Screen coverage
    ScreenCoverage = 2,
    /// Custom metric
    Custom         = 3,
}

/// LOD fade mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LodFadeMode {
    /// Instant switch
    Instant   = 0,
    /// Crossfade
    #[default]
    Crossfade = 1,
    /// Dither
    Dither    = 2,
    /// Speed tree style
    SpeedTree = 3,
}

// ============================================================================
// LOD Level
// ============================================================================

/// LOD level
#[derive(Clone, Debug)]
pub struct LodLevel {
    /// LOD index
    pub index: u32,
    /// Mesh handle
    pub mesh: u64,
    /// Screen size threshold (0-1)
    pub screen_size: f32,
    /// Distance threshold
    pub distance: f32,
    /// Triangle reduction (0-1)
    pub reduction: f32,
    /// Shadow LOD offset
    pub shadow_lod_offset: i32,
}

impl LodLevel {
    /// Creates new LOD level
    pub fn new(index: u32) -> Self {
        Self {
            index,
            mesh: 0,
            screen_size: 1.0,
            distance: 0.0,
            reduction: 1.0 - (index as f32 * 0.25),
            shadow_lod_offset: 0,
        }
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: u64) -> Self {
        self.mesh = mesh;
        self
    }

    /// With screen size
    pub fn with_screen_size(mut self, size: f32) -> Self {
        self.screen_size = size;
        self
    }

    /// With distance
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// With reduction
    pub fn with_reduction(mut self, reduction: f32) -> Self {
        self.reduction = reduction;
        self
    }

    /// With shadow offset
    pub fn with_shadow_offset(mut self, offset: i32) -> Self {
        self.shadow_lod_offset = offset;
        self
    }
}

impl Default for LodLevel {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// LOD Thresholds
// ============================================================================

/// LOD thresholds configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LodThresholds {
    /// Screen size thresholds (8 levels max)
    pub screen_sizes: [f32; 8],
    /// Distance thresholds (8 levels max)
    pub distances: [f32; 8],
    /// Number of levels
    pub level_count: u32,
}

impl LodThresholds {
    /// Creates new thresholds
    pub const fn new() -> Self {
        Self {
            screen_sizes: [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125, 0.015625, 0.0],
            distances: [0.0, 10.0, 25.0, 50.0, 100.0, 200.0, 400.0, 800.0],
            level_count: 4,
        }
    }

    /// With screen sizes
    pub const fn with_screen_sizes(mut self, sizes: [f32; 8]) -> Self {
        self.screen_sizes = sizes;
        self
    }

    /// With distances
    pub const fn with_distances(mut self, distances: [f32; 8]) -> Self {
        self.distances = distances;
        self
    }

    /// With level count
    pub const fn with_levels(mut self, count: u32) -> Self {
        self.level_count = count;
        self
    }

    /// Close range preset
    pub const fn close_range() -> Self {
        Self {
            screen_sizes: [1.0, 0.7, 0.4, 0.2, 0.1, 0.05, 0.02, 0.0],
            distances: [0.0, 5.0, 15.0, 30.0, 50.0, 80.0, 120.0, 200.0],
            level_count: 4,
        }
    }

    /// Far range preset
    pub const fn far_range() -> Self {
        Self {
            screen_sizes: [1.0, 0.4, 0.15, 0.05, 0.02, 0.01, 0.005, 0.0],
            distances: [0.0, 50.0, 150.0, 300.0, 500.0, 800.0, 1200.0, 2000.0],
            level_count: 4,
        }
    }
}

impl Default for LodThresholds {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LOD Transition
// ============================================================================

/// LOD transition info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LodTransition {
    /// From LOD level
    pub from_lod: u32,
    /// To LOD level
    pub to_lod: u32,
    /// Transition factor (0-1)
    pub factor: f32,
    /// Transition mode
    pub mode: LodFadeMode,
    /// Dither pattern
    pub dither_pattern: u32,
}

impl LodTransition {
    /// No transition
    pub const fn none(lod: u32) -> Self {
        Self {
            from_lod: lod,
            to_lod: lod,
            factor: 1.0,
            mode: LodFadeMode::Instant,
            dither_pattern: 0,
        }
    }

    /// Crossfade transition
    pub const fn crossfade(from: u32, to: u32, factor: f32) -> Self {
        Self {
            from_lod: from,
            to_lod: to,
            factor,
            mode: LodFadeMode::Crossfade,
            dither_pattern: 0,
        }
    }

    /// Dither transition
    pub const fn dither(from: u32, to: u32, factor: f32, pattern: u32) -> Self {
        Self {
            from_lod: from,
            to_lod: to,
            factor,
            mode: LodFadeMode::Dither,
            dither_pattern: pattern,
        }
    }

    /// Is transitioning
    pub const fn is_transitioning(&self) -> bool {
        self.from_lod != self.to_lod && self.factor > 0.0 && self.factor < 1.0
    }
}

impl Default for LodTransition {
    fn default() -> Self {
        Self::none(0)
    }
}

// ============================================================================
// LOD Metrics
// ============================================================================

/// LOD metrics for an object
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LodMetrics {
    /// Distance from camera
    pub distance: f32,
    /// Screen size (0-1 of screen height)
    pub screen_size: f32,
    /// Screen coverage (0-1 of screen area)
    pub screen_coverage: f32,
    /// Bounding sphere radius
    pub radius: f32,
    /// Importance factor
    pub importance: f32,
}

impl LodMetrics {
    /// Creates new metrics
    pub const fn new() -> Self {
        Self {
            distance: 0.0,
            screen_size: 1.0,
            screen_coverage: 1.0,
            radius: 1.0,
            importance: 1.0,
        }
    }

    /// Calculate screen size from distance and radius
    pub fn calculate_screen_size(
        distance: f32,
        radius: f32,
        fov_y: f32,
        screen_height: f32,
    ) -> f32 {
        if distance <= 0.0 {
            return 1.0;
        }
        let projected_size = radius / distance;
        let fov_scale = 1.0 / (fov_y * 0.5).tan();
        (projected_size * fov_scale * screen_height * 0.5).min(1.0)
    }

    /// Get recommended LOD level
    pub fn recommended_lod(&self, thresholds: &LodThresholds) -> u32 {
        for i in 0..thresholds.level_count {
            if self.screen_size >= thresholds.screen_sizes[i as usize] {
                return i;
            }
        }
        thresholds.level_count.saturating_sub(1)
    }
}

impl Default for LodMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LOD Override
// ============================================================================

/// LOD override
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LodOverride {
    /// Override type
    pub override_type: LodOverrideType,
    /// Force LOD level
    pub force_level: u32,
    /// Bias adjustment
    pub bias: f32,
    /// Max LOD
    pub max_lod: u32,
    /// Priority
    pub priority: i32,
}

impl LodOverride {
    /// No override
    pub const fn none() -> Self {
        Self {
            override_type: LodOverrideType::None,
            force_level: 0,
            bias: 0.0,
            max_lod: 7,
            priority: 0,
        }
    }

    /// Force specific LOD
    pub const fn force(level: u32) -> Self {
        Self {
            override_type: LodOverrideType::Force,
            force_level: level,
            bias: 0.0,
            max_lod: 7,
            priority: 0,
        }
    }

    /// Apply bias
    pub const fn bias(bias: f32) -> Self {
        Self {
            override_type: LodOverrideType::Bias,
            force_level: 0,
            bias,
            max_lod: 7,
            priority: 0,
        }
    }

    /// Clamp max LOD
    pub const fn clamp_max(max_lod: u32) -> Self {
        Self {
            override_type: LodOverrideType::ClampMax,
            force_level: 0,
            bias: 0.0,
            max_lod,
            priority: 0,
        }
    }
}

impl Default for LodOverride {
    fn default() -> Self {
        Self::none()
    }
}

/// LOD override type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LodOverrideType {
    /// No override
    #[default]
    None     = 0,
    /// Force specific LOD
    Force    = 1,
    /// Apply bias
    Bias     = 2,
    /// Clamp max LOD
    ClampMax = 3,
    /// Clamp min LOD
    ClampMin = 4,
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU LOD data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuLodData {
    /// Current LOD level
    pub current_lod: u32,
    /// Previous LOD level
    pub previous_lod: u32,
    /// Transition factor
    pub transition_factor: f32,
    /// Dither threshold
    pub dither_threshold: f32,
    /// Screen size
    pub screen_size: f32,
    /// Distance
    pub distance: f32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad: u32,
}

/// GPU LOD constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuLodConstants {
    /// Camera position
    pub camera_position: [f32; 3],
    /// LOD bias
    pub lod_bias: f32,
    /// Screen height
    pub screen_height: f32,
    /// FOV Y (radians)
    pub fov_y: f32,
    /// Crossfade distance
    pub crossfade_distance: f32,
    /// Cull ratio
    pub cull_ratio: f32,
    /// Screen size thresholds
    pub screen_thresholds: [f32; 8],
    /// Time
    pub time: f32,
    /// Max LOD
    pub max_lod: u32,
    /// Force LOD (-1 = auto)
    pub force_lod: i32,
    /// Flags
    pub flags: u32,
}

impl Default for GpuLodConstants {
    fn default() -> Self {
        Self {
            camera_position: [0.0; 3],
            lod_bias: 0.0,
            screen_height: 1080.0,
            fov_y: 1.0472, // 60 degrees
            crossfade_distance: 10.0,
            cull_ratio: 0.001,
            screen_thresholds: [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125, 0.015625, 0.0],
            time: 0.0,
            max_lod: 7,
            force_lod: -1,
            flags: 0,
        }
    }
}

/// GPU LOD selection params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuLodSelectionParams {
    /// Object center
    pub center: [f32; 3],
    /// Radius
    pub radius: f32,
    /// LOD group index
    pub group_index: u32,
    /// Instance index
    pub instance_index: u32,
    /// Importance
    pub importance: f32,
    /// Shadow bias
    pub shadow_bias: f32,
}

impl Default for GpuLodSelectionParams {
    fn default() -> Self {
        Self {
            center: [0.0; 3],
            radius: 1.0,
            group_index: 0,
            instance_index: 0,
            importance: 1.0,
            shadow_bias: 0.0,
        }
    }
}

// ============================================================================
// LOD Statistics
// ============================================================================

/// LOD system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuLodStats {
    /// Total objects
    pub total_objects: u32,
    /// Objects by LOD level
    pub objects_by_lod: [u32; 8],
    /// Culled objects
    pub culled_objects: u32,
    /// Transitioning objects
    pub transitioning_objects: u32,
    /// Total triangles
    pub total_triangles: u64,
    /// Triangles saved
    pub triangles_saved: u64,
    /// Selection time (ms)
    pub selection_time_ms: f32,
    /// Average LOD level
    pub average_lod: f32,
}

impl GpuLodStats {
    /// LOD distribution
    pub fn lod_distribution(&self) -> [f32; 8] {
        let mut distribution = [0.0; 8];
        if self.total_objects > 0 {
            for i in 0..8 {
                distribution[i] = self.objects_by_lod[i] as f32 / self.total_objects as f32;
            }
        }
        distribution
    }

    /// Triangle reduction ratio
    pub fn triangle_reduction(&self) -> f32 {
        let total_possible = self.total_triangles + self.triangles_saved;
        if total_possible > 0 {
            self.triangles_saved as f32 / total_possible as f32
        } else {
            0.0
        }
    }

    /// Cull ratio
    pub fn cull_ratio(&self) -> f32 {
        let total = self.total_objects + self.culled_objects;
        if total > 0 {
            self.culled_objects as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Triangles in millions
    pub fn triangles_millions(&self) -> f32 {
        self.total_triangles as f32 / 1_000_000.0
    }
}
