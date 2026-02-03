//! # Level of Detail System
//!
//! Automatic LOD selection based on distance and screen coverage.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::ecs::{Entity, World};
use super::transform::Transform;

/// LOD system
pub struct LodSystem {
    config: LodConfig,
    entity_lods: BTreeMap<Entity, u8>,
    camera_position: [f32; 3],
}

impl LodSystem {
    pub fn new() -> Self {
        Self {
            config: LodConfig::default(),
            entity_lods: BTreeMap::new(),
            camera_position: [0.0; 3],
        }
    }

    /// Set camera position for LOD calculation
    pub fn set_camera(&mut self, position: [f32; 3]) {
        self.camera_position = position;
    }

    /// Update LOD selections
    pub fn update(&mut self, world: &mut World) {
        self.entity_lods.clear();

        for (entity, transform) in world.query::<Transform>() {
            if let Some(lod_group) = world.get_component::<LodGroup>(entity) {
                let distance = self.distance_to_camera(transform.world_position());
                let lod = self.select_lod(lod_group, distance);
                self.entity_lods.insert(entity, lod);
            }
        }
    }

    fn distance_to_camera(&self, pos: [f32; 3]) -> f32 {
        let dx = pos[0] - self.camera_position[0];
        let dy = pos[1] - self.camera_position[1];
        let dz = pos[2] - self.camera_position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn select_lod(&self, group: &LodGroup, distance: f32) -> u8 {
        for (i, lod) in group.lods.iter().enumerate() {
            if distance < lod.max_distance {
                return i as u8;
            }
        }
        (group.lods.len() - 1) as u8
    }

    /// Get current LOD for entity
    pub fn get_lod(&self, entity: Entity) -> u8 {
        self.entity_lods.get(&entity).copied().unwrap_or(0)
    }

    /// Get LOD configuration
    pub fn config(&self) -> &LodConfig {
        &self.config
    }

    /// Set LOD configuration
    pub fn set_config(&mut self, config: LodConfig) {
        self.config = config;
    }
}

impl Default for LodSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// LOD configuration
#[derive(Debug, Clone)]
pub struct LodConfig {
    pub bias: f32,
    pub fade_range: f32,
    pub max_lod: u8,
    pub selection_mode: LodSelectionMode,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            bias: 1.0,
            fade_range: 0.1,
            max_lod: 4,
            selection_mode: LodSelectionMode::Distance,
        }
    }
}

/// LOD selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodSelectionMode {
    Distance,
    ScreenCoverage,
    Combined,
}

/// LOD group component
#[derive(Debug, Clone)]
pub struct LodGroup {
    pub lods: Vec<Lod>,
    pub fade_mode: LodFadeMode,
    pub cross_fade_duration: f32,
}

impl LodGroup {
    pub fn new(lods: Vec<Lod>) -> Self {
        Self {
            lods,
            fade_mode: LodFadeMode::CrossFade,
            cross_fade_duration: 0.5,
        }
    }

    /// Create from distances
    pub fn from_distances(distances: &[f32]) -> Self {
        let lods = distances
            .iter()
            .enumerate()
            .map(|(i, &d)| Lod {
                level: i as u8,
                max_distance: d,
                screen_relative_height: 0.0,
            })
            .collect();

        Self::new(lods)
    }
}

/// Individual LOD level
#[derive(Debug, Clone)]
pub struct Lod {
    pub level: u8,
    pub max_distance: f32,
    pub screen_relative_height: f32,
}

/// LOD fade mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodFadeMode {
    None,
    CrossFade,
    SpeedTree,
}

/// LOD bias controller for performance
pub struct LodBiasController {
    target_frame_time: f32,
    current_bias: f32,
    min_bias: f32,
    max_bias: f32,
    adjust_rate: f32,
}

impl LodBiasController {
    pub fn new(target_fps: f32) -> Self {
        Self {
            target_frame_time: 1.0 / target_fps,
            current_bias: 1.0,
            min_bias: 0.5,
            max_bias: 2.0,
            adjust_rate: 0.1,
        }
    }

    /// Update bias based on frame time
    pub fn update(&mut self, frame_time: f32) {
        if frame_time > self.target_frame_time * 1.1 {
            // Too slow, increase LOD bias (use lower LODs)
            self.current_bias += self.adjust_rate;
        } else if frame_time < self.target_frame_time * 0.9 {
            // Fast enough, decrease LOD bias (use higher LODs)
            self.current_bias -= self.adjust_rate;
        }

        self.current_bias = self.current_bias.clamp(self.min_bias, self.max_bias);
    }

    /// Get current bias
    pub fn bias(&self) -> f32 {
        self.current_bias
    }
}

/// Screen coverage LOD calculator
pub struct ScreenCoverageLod {
    screen_height: f32,
    fov: f32,
}

impl ScreenCoverageLod {
    pub fn new(screen_height: f32, fov: f32) -> Self {
        Self { screen_height, fov }
    }

    /// Calculate screen relative height
    pub fn calculate(&self, object_height: f32, distance: f32) -> f32 {
        if distance <= 0.0 {
            return 1.0;
        }

        let angle = (object_height / 2.0 / distance).atan();
        let screen_fraction = angle / (self.fov / 2.0);
        screen_fraction.clamp(0.0, 1.0)
    }

    /// Select LOD based on screen coverage
    pub fn select_lod(&self, group: &LodGroup, object_height: f32, distance: f32) -> u8 {
        let coverage = self.calculate(object_height, distance);

        for (i, lod) in group.lods.iter().enumerate() {
            if coverage >= lod.screen_relative_height {
                return i as u8;
            }
        }

        (group.lods.len() - 1) as u8
    }
}

/// HLOD (Hierarchical LOD) system
pub struct HlodSystem {
    nodes: Vec<HlodNode>,
}

impl HlodSystem {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add HLOD node
    pub fn add_node(&mut self, node: HlodNode) {
        self.nodes.push(node);
    }

    /// Update HLOD visibility
    pub fn update(&mut self, camera_pos: [f32; 3]) {
        for node in &mut self.nodes {
            let distance = Self::distance(camera_pos, node.center);
            node.use_proxy = distance > node.switch_distance;
        }
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let dz = b[2] - a[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Default for HlodSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// HLOD node
#[derive(Debug, Clone)]
pub struct HlodNode {
    pub center: [f32; 3],
    pub radius: f32,
    pub switch_distance: f32,
    pub proxy_mesh: u64,
    pub children: Vec<Entity>,
    pub use_proxy: bool,
}
