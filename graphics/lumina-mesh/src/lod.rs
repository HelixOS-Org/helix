//! LOD System
//!
//! Level-of-detail management for traditional and virtual geometry,
//! including screen-space error calculation, LOD selection, and blending.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::mesh::{BoundingSphere, MeshHandle, AABB};

// ============================================================================
// Screen-Space Error
// ============================================================================

/// Screen-space error calculation.
#[derive(Debug, Clone, Copy)]
pub struct ScreenSpaceError {
    /// World-space error (geometric deviation).
    pub world_error: f32,
    /// Screen-space error in pixels.
    pub screen_error: f32,
    /// Distance from camera.
    pub distance: f32,
}

impl ScreenSpaceError {
    /// Calculate screen-space error.
    pub fn calculate(
        world_error: f32,
        object_pos: [f32; 3],
        camera_pos: [f32; 3],
        fov_y: f32,
        screen_height: f32,
    ) -> Self {
        let dx = object_pos[0] - camera_pos[0];
        let dy = object_pos[1] - camera_pos[1];
        let dz = object_pos[2] - camera_pos[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);

        // Project world-space error to screen space
        let cot_half_fov = 1.0 / (fov_y * 0.5).tan();
        let screen_error = world_error * cot_half_fov / distance * screen_height * 0.5;

        Self {
            world_error,
            screen_error,
            distance,
        }
    }

    /// Check if error exceeds threshold.
    pub fn exceeds(&self, threshold_pixels: f32) -> bool {
        self.screen_error > threshold_pixels
    }
}

// ============================================================================
// LOD Level
// ============================================================================

/// A single LOD level.
#[derive(Debug, Clone)]
pub struct LodLevel {
    /// Mesh for this LOD.
    pub mesh: MeshHandle,
    /// Screen-space error at which to use this LOD.
    pub min_error: f32,
    /// Screen-space error at which to switch to higher detail.
    pub max_error: f32,
    /// Distance range (min, max) - alternative to error-based.
    pub distance_range: (f32, f32),
    /// Triangle count.
    pub triangle_count: u32,
    /// Vertex count.
    pub vertex_count: u32,
    /// Reduction ratio from LOD 0.
    pub reduction_ratio: f32,
}

impl LodLevel {
    /// Create a new LOD level.
    pub fn new(mesh: MeshHandle, min_error: f32, max_error: f32) -> Self {
        Self {
            mesh,
            min_error,
            max_error,
            distance_range: (0.0, f32::MAX),
            triangle_count: 0,
            vertex_count: 0,
            reduction_ratio: 1.0,
        }
    }

    /// Set distance range.
    pub fn with_distance_range(mut self, min: f32, max: f32) -> Self {
        self.distance_range = (min, max);
        self
    }

    /// Set geometry counts.
    pub fn with_counts(mut self, vertices: u32, triangles: u32) -> Self {
        self.vertex_count = vertices;
        self.triangle_count = triangles;
        self
    }

    /// Check if this LOD should be used for given error.
    pub fn matches_error(&self, error: f32) -> bool {
        error >= self.min_error && error < self.max_error
    }

    /// Check if this LOD should be used for given distance.
    pub fn matches_distance(&self, distance: f32) -> bool {
        distance >= self.distance_range.0 && distance < self.distance_range.1
    }
}

// ============================================================================
// LOD Chain
// ============================================================================

/// A chain of LOD levels for an object.
#[derive(Debug, Clone)]
pub struct LodChain {
    /// Name.
    pub name: String,
    /// LOD levels (sorted by detail, 0 = highest).
    pub levels: Vec<LodLevel>,
    /// Bounds (same for all LODs).
    pub bounds: BoundingSphere,
    /// LOD selection mode.
    pub mode: LodSelectionMode,
    /// Hysteresis to prevent LOD popping.
    pub hysteresis: f32,
    /// Current LOD index.
    current_lod: u32,
    /// Blend factor for LOD transitions.
    blend_factor: f32,
}

/// LOD selection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LodSelectionMode {
    /// Select based on screen-space error.
    #[default]
    ScreenSpaceError,
    /// Select based on distance.
    Distance,
    /// Select based on projected area.
    ProjectedArea,
    /// Manual LOD selection.
    Manual,
}

impl LodChain {
    /// Create a new LOD chain.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            levels: Vec::new(),
            bounds: BoundingSphere::default(),
            mode: LodSelectionMode::ScreenSpaceError,
            hysteresis: 0.1,
            current_lod: 0,
            blend_factor: 0.0,
        }
    }

    /// Add a LOD level.
    pub fn add_level(&mut self, level: LodLevel) {
        self.levels.push(level);
        // Keep sorted by min_error (ascending = higher detail first)
        self.levels
            .sort_by(|a, b| a.min_error.partial_cmp(&b.min_error).unwrap());
    }

    /// Set bounds.
    pub fn set_bounds(&mut self, bounds: BoundingSphere) {
        self.bounds = bounds;
    }

    /// Set selection mode.
    pub fn set_mode(&mut self, mode: LodSelectionMode) {
        self.mode = mode;
    }

    /// Get level count.
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Get current LOD.
    pub fn current_lod(&self) -> u32 {
        self.current_lod
    }

    /// Get blend factor (for LOD transitions).
    pub fn blend_factor(&self) -> f32 {
        self.blend_factor
    }

    /// Get level at index.
    pub fn get_level(&self, index: usize) -> Option<&LodLevel> {
        self.levels.get(index)
    }

    /// Get current level.
    pub fn current_level(&self) -> Option<&LodLevel> {
        self.levels.get(self.current_lod as usize)
    }

    /// Update LOD selection.
    pub fn update(
        &mut self,
        object_pos: [f32; 3],
        camera_pos: [f32; 3],
        fov_y: f32,
        screen_height: f32,
    ) -> LodSelection {
        if self.levels.is_empty() {
            return LodSelection::none();
        }

        let error = ScreenSpaceError::calculate(
            self.bounds.radius,
            object_pos,
            camera_pos,
            fov_y,
            screen_height,
        );

        let new_lod = match self.mode {
            LodSelectionMode::ScreenSpaceError => self.select_by_error(error.screen_error),
            LodSelectionMode::Distance => self.select_by_distance(error.distance),
            LodSelectionMode::ProjectedArea => {
                self.select_by_area(error.distance, fov_y, screen_height)
            },
            LodSelectionMode::Manual => self.current_lod,
        };

        // Apply hysteresis
        let final_lod = if new_lod != self.current_lod {
            let threshold = if new_lod < self.current_lod {
                // Switching to higher detail
                1.0 - self.hysteresis
            } else {
                // Switching to lower detail
                1.0 + self.hysteresis
            };

            let level = &self.levels[new_lod as usize];
            let boundary_error = if new_lod < self.current_lod {
                level.max_error
            } else {
                level.min_error
            };

            if (error.screen_error / boundary_error) > threshold
                || (error.screen_error / boundary_error) < (1.0 / threshold)
            {
                new_lod
            } else {
                self.current_lod
            }
        } else {
            new_lod
        };

        // Calculate blend factor for smooth transitions
        let level = &self.levels[final_lod as usize];
        let range = level.max_error - level.min_error;
        self.blend_factor = if range > 0.0 {
            ((error.screen_error - level.min_error) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let old_lod = self.current_lod;
        self.current_lod = final_lod;

        LodSelection {
            primary_lod: final_lod,
            secondary_lod: if self.blend_factor > 0.0
                && (final_lod as usize) + 1 < self.levels.len()
            {
                Some(final_lod + 1)
            } else {
                None
            },
            blend_factor: self.blend_factor,
            changed: old_lod != final_lod,
            screen_error: error.screen_error,
            distance: error.distance,
        }
    }

    /// Select LOD by screen-space error.
    fn select_by_error(&self, error: f32) -> u32 {
        for (i, level) in self.levels.iter().enumerate() {
            if level.matches_error(error) {
                return i as u32;
            }
        }
        (self.levels.len() - 1) as u32
    }

    /// Select LOD by distance.
    fn select_by_distance(&self, distance: f32) -> u32 {
        for (i, level) in self.levels.iter().enumerate() {
            if level.matches_distance(distance) {
                return i as u32;
            }
        }
        (self.levels.len() - 1) as u32
    }

    /// Select LOD by projected area.
    fn select_by_area(&self, distance: f32, fov_y: f32, screen_height: f32) -> u32 {
        let cot_half_fov = 1.0 / (fov_y * 0.5).tan();
        let projected_size = self.bounds.radius * cot_half_fov / distance * screen_height;

        // Use projected size as inverse of error
        let effective_error = screen_height / projected_size.max(1.0);
        self.select_by_error(effective_error)
    }

    /// Set LOD manually.
    pub fn set_lod(&mut self, lod: u32) {
        self.current_lod = lod.min(self.levels.len().saturating_sub(1) as u32);
        self.blend_factor = 0.0;
    }

    /// Force highest LOD.
    pub fn force_highest(&mut self) {
        self.current_lod = 0;
        self.blend_factor = 0.0;
    }

    /// Force lowest LOD.
    pub fn force_lowest(&mut self) {
        self.current_lod = self.levels.len().saturating_sub(1) as u32;
        self.blend_factor = 0.0;
    }
}

/// Result of LOD selection.
#[derive(Debug, Clone, Copy)]
pub struct LodSelection {
    /// Primary LOD index.
    pub primary_lod: u32,
    /// Secondary LOD for blending (if transitioning).
    pub secondary_lod: Option<u32>,
    /// Blend factor between primary and secondary (0-1).
    pub blend_factor: f32,
    /// Whether LOD changed this frame.
    pub changed: bool,
    /// Current screen-space error.
    pub screen_error: f32,
    /// Current distance.
    pub distance: f32,
}

impl LodSelection {
    /// Create empty selection.
    pub fn none() -> Self {
        Self {
            primary_lod: 0,
            secondary_lod: None,
            blend_factor: 0.0,
            changed: false,
            screen_error: 0.0,
            distance: 0.0,
        }
    }

    /// Check if blending between LODs.
    pub fn is_blending(&self) -> bool {
        self.secondary_lod.is_some() && self.blend_factor > 0.0 && self.blend_factor < 1.0
    }
}

// ============================================================================
// LOD Mesh
// ============================================================================

/// A mesh with pre-generated LOD levels.
pub struct LodMesh {
    /// Handle.
    handle: MeshHandle,
    /// Name.
    name: String,
    /// LOD chain.
    chain: LodChain,
    /// Bounds.
    bounds: AABB,
}

impl LodMesh {
    /// Create a new LOD mesh.
    pub fn new(handle: MeshHandle, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            handle,
            name: name.clone(),
            chain: LodChain::new(name),
            bounds: AABB::INVALID,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> MeshHandle {
        self.handle
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get LOD chain.
    pub fn chain(&self) -> &LodChain {
        &self.chain
    }

    /// Get mutable LOD chain.
    pub fn chain_mut(&mut self) -> &mut LodChain {
        &mut self.chain
    }

    /// Get bounds.
    pub fn bounds(&self) -> &AABB {
        &self.bounds
    }

    /// Set bounds.
    pub fn set_bounds(&mut self, bounds: AABB) {
        self.bounds = bounds;
        self.chain.set_bounds(BoundingSphere::from_aabb(&bounds));
    }

    /// Add a LOD level.
    pub fn add_level(&mut self, level: LodLevel) {
        self.chain.add_level(level);
    }

    /// Update and get current mesh.
    pub fn update(
        &mut self,
        object_pos: [f32; 3],
        camera_pos: [f32; 3],
        fov_y: f32,
        screen_height: f32,
    ) -> (MeshHandle, LodSelection) {
        let selection = self
            .chain
            .update(object_pos, camera_pos, fov_y, screen_height);
        let mesh = self
            .chain
            .get_level(selection.primary_lod as usize)
            .map(|l| l.mesh)
            .unwrap_or(MeshHandle::INVALID);
        (mesh, selection)
    }
}

// ============================================================================
// LOD Settings
// ============================================================================

/// Global LOD settings.
#[derive(Debug, Clone)]
pub struct LodSettings {
    /// Error threshold in pixels.
    pub error_threshold: f32,
    /// LOD bias (positive = lower quality, negative = higher quality).
    pub lod_bias: f32,
    /// Minimum LOD level.
    pub min_lod: u32,
    /// Maximum LOD level.
    pub max_lod: u32,
    /// Enable LOD blending.
    pub enable_blending: bool,
    /// Blend range (in error units).
    pub blend_range: f32,
    /// Distance scale factor.
    pub distance_scale: f32,
}

impl Default for LodSettings {
    fn default() -> Self {
        Self {
            error_threshold: 1.0,
            lod_bias: 0.0,
            min_lod: 0,
            max_lod: u32::MAX,
            enable_blending: true,
            blend_range: 0.5,
            distance_scale: 1.0,
        }
    }
}

/// LOD bias that can be applied to selections.
#[derive(Debug, Clone, Copy, Default)]
pub struct LodBias {
    /// Global bias.
    pub global: f32,
    /// Per-object bias.
    pub object: f32,
    /// View-dependent bias.
    pub view: f32,
}

impl LodBias {
    /// Calculate total bias.
    pub fn total(&self) -> f32 {
        self.global + self.object + self.view
    }

    /// Apply bias to error value.
    pub fn apply(&self, error: f32) -> f32 {
        let bias = self.total();
        if bias >= 0.0 {
            error * (1.0 + bias)
        } else {
            error / (1.0 - bias)
        }
    }
}

// ============================================================================
// LOD Manager
// ============================================================================

/// Manages LOD for all objects in the scene.
pub struct LodManager {
    /// LOD meshes.
    meshes: BTreeMap<u32, LodMesh>,
    /// Name map.
    name_map: BTreeMap<String, MeshHandle>,
    /// Global settings.
    settings: LodSettings,
    /// Statistics.
    stats: LodStats,
}

/// LOD statistics.
#[derive(Debug, Clone, Default)]
pub struct LodStats {
    /// Total managed meshes.
    pub mesh_count: u32,
    /// LOD transitions this frame.
    pub transitions: u32,
    /// Objects at LOD 0.
    pub lod0_count: u32,
    /// Objects at lowest LOD.
    pub lowest_lod_count: u32,
    /// Average LOD level.
    pub average_lod: f32,
    /// Objects currently blending.
    pub blending_count: u32,
}

impl LodManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            meshes: BTreeMap::new(),
            name_map: BTreeMap::new(),
            settings: LodSettings::default(),
            stats: LodStats::default(),
        }
    }

    /// Create with settings.
    pub fn with_settings(settings: LodSettings) -> Self {
        Self {
            settings,
            ..Self::new()
        }
    }

    /// Get settings.
    pub fn settings(&self) -> &LodSettings {
        &self.settings
    }

    /// Get mutable settings.
    pub fn settings_mut(&mut self) -> &mut LodSettings {
        &mut self.settings
    }

    /// Register a LOD mesh.
    pub fn register(&mut self, mesh: LodMesh) {
        let handle = mesh.handle;
        self.name_map.insert(mesh.name.clone(), handle);
        self.meshes.insert(handle.index(), mesh);
    }

    /// Unregister a mesh.
    pub fn unregister(&mut self, handle: MeshHandle) -> Option<LodMesh> {
        if let Some(mesh) = self.meshes.remove(&handle.index()) {
            self.name_map.remove(&mesh.name);
            Some(mesh)
        } else {
            None
        }
    }

    /// Get a mesh.
    pub fn get(&self, handle: MeshHandle) -> Option<&LodMesh> {
        self.meshes.get(&handle.index())
    }

    /// Get mutable mesh.
    pub fn get_mut(&mut self, handle: MeshHandle) -> Option<&mut LodMesh> {
        self.meshes.get_mut(&handle.index())
    }

    /// Update all LODs for given camera.
    pub fn update_all(
        &mut self,
        camera_pos: [f32; 3],
        fov_y: f32,
        screen_height: f32,
        object_positions: &[(MeshHandle, [f32; 3])],
    ) -> Vec<(MeshHandle, LodSelection)> {
        // Reset stats
        self.stats = LodStats::default();
        self.stats.mesh_count = self.meshes.len() as u32;

        let mut results = Vec::with_capacity(object_positions.len());
        let mut total_lod = 0u32;

        for (handle, pos) in object_positions {
            if let Some(mesh) = self.meshes.get_mut(&handle.index()) {
                let (_, selection) = mesh.update(*pos, camera_pos, fov_y, screen_height);

                if selection.changed {
                    self.stats.transitions += 1;
                }
                if selection.primary_lod == 0 {
                    self.stats.lod0_count += 1;
                }
                if selection.primary_lod == mesh.chain().level_count().saturating_sub(1) as u32 {
                    self.stats.lowest_lod_count += 1;
                }
                if selection.is_blending() {
                    self.stats.blending_count += 1;
                }

                total_lod += selection.primary_lod;
                results.push((*handle, selection));
            }
        }

        if !results.is_empty() {
            self.stats.average_lod = total_lod as f32 / results.len() as f32;
        }

        results
    }

    /// Get statistics.
    pub fn stats(&self) -> &LodStats {
        &self.stats
    }

    /// Iterate over meshes.
    pub fn iter(&self) -> impl Iterator<Item = &LodMesh> {
        self.meshes.values()
    }
}

impl Default for LodManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LOD Generator
// ============================================================================

/// Generates LOD levels automatically from a mesh.
#[derive(Debug, Clone)]
pub struct LodGenerator {
    /// Target reduction per level.
    pub reduction_per_level: f32,
    /// Maximum LOD levels.
    pub max_levels: u32,
    /// Minimum triangle count to stop.
    pub min_triangles: u32,
    /// Error multiplier per level.
    pub error_multiplier: f32,
    /// Distance multiplier per level.
    pub distance_multiplier: f32,
}

impl Default for LodGenerator {
    fn default() -> Self {
        Self {
            reduction_per_level: 0.5,
            max_levels: 6,
            min_triangles: 32,
            error_multiplier: 2.0,
            distance_multiplier: 2.0,
        }
    }
}

impl LodGenerator {
    /// Create a new generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set reduction ratio.
    pub fn with_reduction(mut self, ratio: f32) -> Self {
        self.reduction_per_level = ratio.clamp(0.1, 0.9);
        self
    }

    /// Set max levels.
    pub fn with_max_levels(mut self, levels: u32) -> Self {
        self.max_levels = levels;
        self
    }

    /// Calculate LOD parameters for a chain.
    pub fn calculate_parameters(&self, base_triangles: u32) -> Vec<(u32, f32, f32, f32)> {
        let mut params = Vec::new();
        let mut triangles = base_triangles;
        let mut error = 0.0f32;
        let mut distance = 0.0f32;

        for i in 0..self.max_levels {
            let min_error = error;
            error = if i == 0 {
                1.0
            } else {
                error * self.error_multiplier
            };
            let max_error = error;

            let min_distance = distance;
            distance = if i == 0 {
                10.0
            } else {
                distance * self.distance_multiplier
            };

            params.push((triangles, min_error, max_error, min_distance));

            triangles = ((triangles as f32) * self.reduction_per_level) as u32;
            if triangles < self.min_triangles {
                break;
            }
        }

        params
    }
}
