//! Mesh LOD (Level of Detail) Types for Lumina
//!
//! This module provides mesh LOD infrastructure including
//! LOD selection, mesh simplification, and LOD streaming.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// LOD Handles
// ============================================================================

/// LOD mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LodMeshHandle(pub u64);

impl LodMeshHandle {
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

impl Default for LodMeshHandle {
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

impl Default for LodGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// LOD chain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LodChainHandle(pub u64);

impl LodChainHandle {
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

impl Default for LodChainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// LOD Settings
// ============================================================================

/// LOD settings
#[derive(Clone, Debug)]
pub struct LodSettings {
    /// Selection method
    pub method: LodSelectionMethod,
    /// LOD bias (positive = higher quality)
    pub bias: f32,
    /// Minimum screen coverage (0-1)
    pub min_coverage: f32,
    /// Maximum LOD level
    pub max_lod: u32,
    /// Force LOD level (-1 = automatic)
    pub force_lod: i32,
    /// Enable dithering for transitions
    pub dither_transitions: bool,
    /// Hysteresis factor
    pub hysteresis: f32,
}

impl LodSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            method: LodSelectionMethod::ScreenCoverage,
            bias: 0.0,
            min_coverage: 0.001,
            max_lod: 4,
            force_lod: -1,
            dither_transitions: true,
            hysteresis: 0.1,
        }
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self {
            bias: 1.0,
            ..Self::new()
        }
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self {
            bias: -1.0,
            max_lod: 8,
            ..Self::new()
        }
    }

    /// With bias
    pub fn with_bias(mut self, bias: f32) -> Self {
        self.bias = bias;
        self
    }

    /// With method
    pub fn with_method(mut self, method: LodSelectionMethod) -> Self {
        self.method = method;
        self
    }

    /// Force specific LOD
    pub fn with_force_lod(mut self, lod: i32) -> Self {
        self.force_lod = lod;
        self
    }

    /// With dithering
    pub fn with_dithering(mut self, enabled: bool) -> Self {
        self.dither_transitions = enabled;
        self
    }
}

impl Default for LodSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// LOD selection method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LodSelectionMethod {
    /// Screen coverage based
    #[default]
    ScreenCoverage = 0,
    /// Distance based
    Distance       = 1,
    /// Error metric based
    ErrorMetric    = 2,
    /// Hybrid (coverage + error)
    Hybrid         = 3,
}

// ============================================================================
// LOD Level
// ============================================================================

/// LOD level definition
#[derive(Clone, Debug)]
pub struct LodLevel {
    /// Level index
    pub level: u32,
    /// Mesh handle
    pub mesh: LodMeshHandle,
    /// Screen coverage threshold
    pub screen_coverage: f32,
    /// Distance threshold
    pub distance: f32,
    /// Vertex count
    pub vertex_count: u32,
    /// Triangle count
    pub triangle_count: u32,
    /// Quality metric (0-1)
    pub quality: f32,
}

impl LodLevel {
    /// Creates LOD level
    pub fn new(level: u32) -> Self {
        Self {
            level,
            mesh: LodMeshHandle::NULL,
            screen_coverage: 0.0,
            distance: 0.0,
            vertex_count: 0,
            triangle_count: 0,
            quality: 1.0,
        }
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: LodMeshHandle) -> Self {
        self.mesh = mesh;
        self
    }

    /// With screen coverage threshold
    pub fn with_screen_coverage(mut self, coverage: f32) -> Self {
        self.screen_coverage = coverage;
        self
    }

    /// With distance
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// Reduction ratio from LOD 0
    pub fn reduction_ratio(&self, lod0_triangles: u32) -> f32 {
        if lod0_triangles == 0 {
            0.0
        } else {
            self.triangle_count as f32 / lod0_triangles as f32
        }
    }
}

/// LOD level thresholds
#[derive(Clone, Copy, Debug)]
pub struct LodThresholds {
    /// Screen coverage thresholds per level
    pub coverage: [f32; 8],
    /// Distance thresholds per level
    pub distance: [f32; 8],
    /// Number of levels
    pub level_count: u32,
}

impl LodThresholds {
    /// Creates thresholds
    pub fn new(level_count: u32) -> Self {
        Self {
            coverage: [0.5, 0.25, 0.1, 0.05, 0.02, 0.01, 0.005, 0.001],
            distance: [10.0, 25.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0],
            level_count: level_count.min(8),
        }
    }

    /// Standard thresholds
    pub fn standard() -> Self {
        Self::new(4)
    }

    /// Get level for screen coverage
    pub fn level_for_coverage(&self, coverage: f32) -> u32 {
        for i in 0..self.level_count as usize {
            if coverage >= self.coverage[i] {
                return i as u32;
            }
        }
        self.level_count.saturating_sub(1)
    }

    /// Get level for distance
    pub fn level_for_distance(&self, distance: f32) -> u32 {
        for i in 0..self.level_count as usize {
            if distance <= self.distance[i] {
                return i as u32;
            }
        }
        self.level_count.saturating_sub(1)
    }
}

impl Default for LodThresholds {
    fn default() -> Self {
        Self::standard()
    }
}

// ============================================================================
// LOD Chain
// ============================================================================

/// LOD chain create info
#[derive(Clone, Debug)]
pub struct LodChainCreateInfo {
    /// LOD levels
    pub levels: Vec<LodLevel>,
    /// Thresholds
    pub thresholds: LodThresholds,
    /// Bounding radius
    pub bounding_radius: f32,
}

impl LodChainCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            thresholds: LodThresholds::default(),
            bounding_radius: 1.0,
        }
    }

    /// Add level
    pub fn add_level(mut self, level: LodLevel) -> Self {
        self.levels.push(level);
        self.thresholds.level_count = self.levels.len() as u32;
        self
    }

    /// With bounding radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.bounding_radius = radius;
        self
    }
}

impl Default for LodChainCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mesh Simplification
// ============================================================================

/// Mesh simplification settings
#[derive(Clone, Debug)]
pub struct SimplificationSettings {
    /// Target triangle ratio (0-1)
    pub target_ratio: f32,
    /// Target triangle count (0 = use ratio)
    pub target_count: u32,
    /// Algorithm
    pub algorithm: SimplificationAlgorithm,
    /// Error tolerance
    pub error_tolerance: f32,
    /// Lock boundary edges
    pub lock_borders: bool,
    /// Preserve attributes
    pub preserve: PreserveFlags,
    /// Quality vs speed (0-1)
    pub quality_speed: f32,
}

impl SimplificationSettings {
    /// Creates settings
    pub fn new(target_ratio: f32) -> Self {
        Self {
            target_ratio,
            target_count: 0,
            algorithm: SimplificationAlgorithm::QuadricError,
            error_tolerance: 0.001,
            lock_borders: true,
            preserve: PreserveFlags::ALL,
            quality_speed: 0.5,
        }
    }

    /// For LOD level
    pub fn for_lod(level: u32) -> Self {
        let ratios = [1.0, 0.5, 0.25, 0.125, 0.0625];
        let ratio = ratios.get(level as usize).copied().unwrap_or(0.0625);
        Self::new(ratio)
    }

    /// Fast simplification
    pub fn fast(target_ratio: f32) -> Self {
        Self {
            quality_speed: 0.0,
            preserve: PreserveFlags::NONE,
            lock_borders: false,
            ..Self::new(target_ratio)
        }
    }

    /// With algorithm
    pub fn with_algorithm(mut self, algorithm: SimplificationAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// With error tolerance
    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.error_tolerance = tolerance;
        self
    }
}

impl Default for SimplificationSettings {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Simplification algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SimplificationAlgorithm {
    /// Quadric error metrics
    #[default]
    QuadricError     = 0,
    /// Edge collapse
    EdgeCollapse     = 1,
    /// Vertex clustering
    VertexClustering = 2,
    /// Sloppy (fast, lower quality)
    Sloppy           = 3,
}

/// Attributes to preserve during simplification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PreserveFlags(pub u32);

impl PreserveFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Normals
    pub const NORMALS: Self = Self(1 << 0);
    /// UVs
    pub const UVS: Self = Self(1 << 1);
    /// Colors
    pub const COLORS: Self = Self(1 << 2);
    /// Tangents
    pub const TANGENTS: Self = Self(1 << 3);
    /// Seams
    pub const SEAMS: Self = Self(1 << 4);
    /// All
    pub const ALL: Self = Self(0x1F);

    /// Has flag
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for PreserveFlags {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitOr for PreserveFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// LOD Selection
// ============================================================================

/// LOD selection result
#[derive(Clone, Copy, Debug)]
pub struct LodSelection {
    /// Selected LOD level
    pub level: u32,
    /// Blend factor (for transitions)
    pub blend_factor: f32,
    /// Next LOD level (for blending)
    pub next_level: u32,
    /// Screen coverage
    pub screen_coverage: f32,
    /// Distance
    pub distance: f32,
}

impl LodSelection {
    /// Creates selection
    pub fn new(level: u32) -> Self {
        Self {
            level,
            blend_factor: 1.0,
            next_level: level,
            screen_coverage: 0.0,
            distance: 0.0,
        }
    }

    /// Is blending between levels
    pub fn is_blending(&self) -> bool {
        self.blend_factor < 1.0 && self.level != self.next_level
    }
}

/// LOD selector
#[derive(Clone, Debug)]
pub struct LodSelector {
    /// Settings
    pub settings: LodSettings,
    /// Camera position
    pub camera_position: [f32; 3],
    /// Camera FOV (degrees)
    pub fov: f32,
    /// Screen height (pixels)
    pub screen_height: f32,
}

impl LodSelector {
    /// Creates selector
    pub fn new(settings: LodSettings) -> Self {
        Self {
            settings,
            camera_position: [0.0, 0.0, 0.0],
            fov: 60.0,
            screen_height: 1080.0,
        }
    }

    /// Update camera
    pub fn update_camera(&mut self, position: [f32; 3], fov: f32, screen_height: f32) {
        self.camera_position = position;
        self.fov = fov;
        self.screen_height = screen_height;
    }

    /// Calculate screen coverage
    pub fn screen_coverage(&self, object_pos: [f32; 3], radius: f32) -> f32 {
        let dx = object_pos[0] - self.camera_position[0];
        let dy = object_pos[1] - self.camera_position[1];
        let dz = object_pos[2] - self.camera_position[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        if distance <= 0.0 {
            return 1.0;
        }

        let fov_rad = self.fov * core::f32::consts::PI / 180.0;
        let projected_size = (radius * 2.0) / (distance * (fov_rad / 2.0).tan());
        let coverage = projected_size / 2.0;

        coverage.clamp(0.0, 1.0)
    }

    /// Select LOD
    pub fn select(
        &self,
        thresholds: &LodThresholds,
        object_pos: [f32; 3],
        radius: f32,
    ) -> LodSelection {
        if self.settings.force_lod >= 0 {
            return LodSelection::new(self.settings.force_lod as u32);
        }

        let coverage = self.screen_coverage(object_pos, radius);
        let biased_coverage = coverage * (1.0 + self.settings.bias * 0.5);

        let level = thresholds.level_for_coverage(biased_coverage);
        let level = level.min(self.settings.max_lod);

        let mut selection = LodSelection::new(level);
        selection.screen_coverage = coverage;

        // Calculate blend factor if dithering
        if self.settings.dither_transitions && level < thresholds.level_count.saturating_sub(1) {
            let current_threshold = thresholds.coverage[level as usize];
            let next_threshold = thresholds.coverage[(level + 1) as usize];
            let range = current_threshold - next_threshold;
            if range > 0.0 {
                selection.blend_factor = (coverage - next_threshold) / range;
                selection.next_level = level + 1;
            }
        }

        selection
    }
}

impl Default for LodSelector {
    fn default() -> Self {
        Self::new(LodSettings::default())
    }
}

// ============================================================================
// LOD Transition
// ============================================================================

/// LOD transition settings
#[derive(Clone, Debug)]
pub struct LodTransitionSettings {
    /// Transition type
    pub transition_type: LodTransitionType,
    /// Transition duration (seconds)
    pub duration: f32,
    /// Dither pattern size
    pub dither_size: u32,
}

impl LodTransitionSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            transition_type: LodTransitionType::Dither,
            duration: 0.25,
            dither_size: 8,
        }
    }

    /// Instant transition
    pub fn instant() -> Self {
        Self {
            transition_type: LodTransitionType::Instant,
            duration: 0.0,
            ..Self::new()
        }
    }

    /// Crossfade transition
    pub fn crossfade(duration: f32) -> Self {
        Self {
            transition_type: LodTransitionType::Crossfade,
            duration,
            ..Self::new()
        }
    }
}

impl Default for LodTransitionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// LOD transition type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LodTransitionType {
    /// Instant switch
    Instant   = 0,
    /// Screen-space dither
    #[default]
    Dither    = 1,
    /// Alpha crossfade
    Crossfade = 2,
}

/// LOD dither pattern
#[derive(Clone, Copy, Debug)]
pub struct LodDitherPattern {
    /// Pattern size
    pub size: u32,
    /// Pattern data (8x8 max)
    pub pattern: [u8; 64],
}

impl LodDitherPattern {
    /// Creates pattern
    pub fn new(size: u32) -> Self {
        Self {
            size: size.min(8),
            pattern: [0; 64],
        }
    }

    /// Bayer 4x4 pattern
    pub fn bayer_4x4() -> Self {
        Self {
            size: 4,
            pattern: [
                0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        }
    }

    /// Sample pattern
    pub fn sample(&self, x: u32, y: u32) -> u8 {
        let idx = (y % self.size) * self.size + (x % self.size);
        self.pattern[idx as usize]
    }

    /// Should discard pixel
    pub fn should_discard(&self, x: u32, y: u32, threshold: f32) -> bool {
        let value = self.sample(x, y) as f32 / (self.size * self.size) as f32;
        value > threshold
    }
}

impl Default for LodDitherPattern {
    fn default() -> Self {
        Self::bayer_4x4()
    }
}

// ============================================================================
// GPU LOD Data
// ============================================================================

/// LOD GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LodGpuParams {
    /// Camera position
    pub camera_position: [f32; 4],
    /// LOD bias
    pub lod_bias: f32,
    /// Screen height
    pub screen_height: f32,
    /// FOV factor
    pub fov_factor: f32,
    /// Dither scale
    pub dither_scale: f32,
}

/// Per-instance LOD data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct InstanceLodData {
    /// Selected LOD
    pub lod_level: u32,
    /// Blend factor
    pub blend_factor: f32,
    /// Next LOD
    pub next_lod: u32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// HLOD (Hierarchical LOD)
// ============================================================================

/// HLOD node
#[derive(Clone, Debug)]
pub struct HlodNode {
    /// Node bounds (center + radius)
    pub bounds: [f32; 4],
    /// Child nodes (indices)
    pub children: Vec<u32>,
    /// Combined mesh for this node
    pub combined_mesh: LodMeshHandle,
    /// Individual meshes (for detailed view)
    pub individual_meshes: Vec<LodMeshHandle>,
    /// Distance threshold
    pub distance_threshold: f32,
}

impl HlodNode {
    /// Creates node
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            bounds: [center[0], center[1], center[2], radius],
            children: Vec::new(),
            combined_mesh: LodMeshHandle::NULL,
            individual_meshes: Vec::new(),
            distance_threshold: 100.0,
        }
    }

    /// Add child
    pub fn add_child(&mut self, child_index: u32) {
        self.children.push(child_index);
    }

    /// Should use combined
    pub fn should_use_combined(&self, distance: f32) -> bool {
        distance > self.distance_threshold
    }
}

/// HLOD tree
#[derive(Clone, Debug)]
pub struct HlodTree {
    /// Nodes
    pub nodes: Vec<HlodNode>,
    /// Root node index
    pub root: u32,
}

impl HlodTree {
    /// Creates tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: 0,
        }
    }

    /// Add node
    pub fn add_node(&mut self, node: HlodNode) -> u32 {
        let index = self.nodes.len() as u32;
        self.nodes.push(node);
        index
    }
}

impl Default for HlodTree {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Impostor LOD
// ============================================================================

/// Impostor settings
#[derive(Clone, Debug)]
pub struct ImpostorSettings {
    /// Atlas size
    pub atlas_size: u32,
    /// Views count (horizontal)
    pub views_horizontal: u32,
    /// Views count (vertical)
    pub views_vertical: u32,
    /// Use octahedral mapping
    pub octahedral: bool,
    /// Include depth
    pub with_depth: bool,
    /// Include normals
    pub with_normals: bool,
}

impl ImpostorSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            atlas_size: 2048,
            views_horizontal: 8,
            views_vertical: 4,
            octahedral: false,
            with_depth: true,
            with_normals: true,
        }
    }

    /// Billboard impostor (2 views)
    pub fn billboard() -> Self {
        Self {
            views_horizontal: 2,
            views_vertical: 1,
            ..Self::new()
        }
    }

    /// Octahedral impostor
    pub fn octahedral() -> Self {
        Self {
            octahedral: true,
            views_horizontal: 8,
            views_vertical: 8,
            ..Self::new()
        }
    }

    /// Total view count
    pub fn view_count(&self) -> u32 {
        if self.octahedral {
            self.views_horizontal * self.views_vertical
        } else {
            self.views_horizontal * self.views_vertical
        }
    }

    /// View size in atlas
    pub fn view_size(&self) -> u32 {
        self.atlas_size / self.views_horizontal.max(self.views_vertical)
    }
}

impl Default for ImpostorSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Impostor data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImpostorGpuData {
    /// Atlas size
    pub atlas_size: [f32; 2],
    /// Frame count
    pub frame_count: [f32; 2],
    /// Center offset
    pub center_offset: [f32; 4],
    /// Scale
    pub scale: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// LOD statistics
#[derive(Clone, Debug, Default)]
pub struct LodStats {
    /// Objects per LOD level
    pub objects_per_level: [u32; 8],
    /// Triangles per LOD level
    pub triangles_per_level: [u64; 8],
    /// Total objects
    pub total_objects: u32,
    /// Total triangles (rendered)
    pub total_triangles: u64,
    /// LOD transitions this frame
    pub transitions: u32,
    /// Memory saved (bytes)
    pub memory_saved: u64,
}

impl LodStats {
    /// Average LOD level
    pub fn average_lod(&self) -> f32 {
        if self.total_objects == 0 {
            return 0.0;
        }
        let mut sum = 0u64;
        for (level, &count) in self.objects_per_level.iter().enumerate() {
            sum += level as u64 * count as u64;
        }
        sum as f32 / self.total_objects as f32
    }
}
