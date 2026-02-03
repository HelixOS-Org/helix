//! # Lighting System
//!
//! Light types and deferred lighting.

use alloc::vec::Vec;

/// Light manager
pub struct LightManager {
    lights: Vec<Light>,
    light_buffer: Vec<LightData>,
    max_lights: u32,
}

impl LightManager {
    pub fn new(max_lights: u32) -> Self {
        Self {
            lights: Vec::new(),
            light_buffer: Vec::new(),
            max_lights,
        }
    }

    /// Add a light
    pub fn add(&mut self, light: Light) -> LightId {
        let id = LightId(self.lights.len() as u32);
        self.lights.push(light);
        id
    }

    /// Remove a light
    pub fn remove(&mut self, id: LightId) {
        if (id.0 as usize) < self.lights.len() {
            self.lights.remove(id.0 as usize);
        }
    }

    /// Get light reference
    pub fn get(&self, id: LightId) -> Option<&Light> {
        self.lights.get(id.0 as usize)
    }

    /// Get mutable light reference
    pub fn get_mut(&mut self, id: LightId) -> Option<&mut Light> {
        self.lights.get_mut(id.0 as usize)
    }

    /// Update light buffer for GPU
    pub fn update_buffer(&mut self) {
        self.light_buffer.clear();

        for light in &self.lights {
            if !light.enabled {
                continue;
            }

            self.light_buffer.push(light.to_gpu_data());
        }
    }

    /// Get light buffer data
    pub fn buffer_data(&self) -> &[LightData] {
        &self.light_buffer
    }

    /// Get active light count
    pub fn active_count(&self) -> u32 {
        self.light_buffer.len() as u32
    }
}

/// Light handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightId(u32);

/// Light source
#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    pub color: [f32; 3],
    pub intensity: f32,
    pub enabled: bool,
    pub cast_shadows: bool,
    pub shadow_bias: f32,
}

impl Light {
    pub fn directional(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional { direction },
            color,
            intensity,
            enabled: true,
            cast_shadows: true,
            shadow_bias: 0.001,
        }
    }

    pub fn point(position: [f32; 3], color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            light_type: LightType::Point { position, radius },
            color,
            intensity,
            enabled: true,
            cast_shadows: false,
            shadow_bias: 0.001,
        }
    }

    pub fn spot(
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            light_type: LightType::Spot {
                position,
                direction,
                inner_angle,
                outer_angle,
                range: 100.0,
            },
            color,
            intensity,
            enabled: true,
            cast_shadows: true,
            shadow_bias: 0.001,
        }
    }

    pub fn area_rect(
        position: [f32; 3],
        normal: [f32; 3],
        size: [f32; 2],
        color: [f32; 3],
        intensity: f32,
    ) -> Self {
        Self {
            light_type: LightType::AreaRect {
                position,
                normal,
                size,
                two_sided: false,
            },
            color,
            intensity,
            enabled: true,
            cast_shadows: false,
            shadow_bias: 0.001,
        }
    }

    fn to_gpu_data(&self) -> LightData {
        match &self.light_type {
            LightType::Directional { direction } => LightData {
                position_type: [direction[0], direction[1], direction[2], 0.0],
                color_intensity: [self.color[0], self.color[1], self.color[2], self.intensity],
                direction_range: [0.0; 4],
                spot_params: [0.0; 4],
            },
            LightType::Point { position, radius } => LightData {
                position_type: [position[0], position[1], position[2], 1.0],
                color_intensity: [self.color[0], self.color[1], self.color[2], self.intensity],
                direction_range: [0.0, 0.0, 0.0, *radius],
                spot_params: [0.0; 4],
            },
            LightType::Spot {
                position,
                direction,
                inner_angle,
                outer_angle,
                range,
            } => LightData {
                position_type: [position[0], position[1], position[2], 2.0],
                color_intensity: [self.color[0], self.color[1], self.color[2], self.intensity],
                direction_range: [direction[0], direction[1], direction[2], *range],
                spot_params: [inner_angle.cos(), outer_angle.cos(), 0.0, 0.0],
            },
            LightType::AreaRect {
                position,
                normal,
                size,
                two_sided,
            } => LightData {
                position_type: [position[0], position[1], position[2], 3.0],
                color_intensity: [self.color[0], self.color[1], self.color[2], self.intensity],
                direction_range: [normal[0], normal[1], normal[2], 0.0],
                spot_params: [size[0], size[1], if *two_sided { 1.0 } else { 0.0 }, 0.0],
            },
        }
    }
}

/// Light type
#[derive(Debug, Clone)]
pub enum LightType {
    Directional {
        direction: [f32; 3],
    },
    Point {
        position: [f32; 3],
        radius: f32,
    },
    Spot {
        position: [f32; 3],
        direction: [f32; 3],
        inner_angle: f32,
        outer_angle: f32,
        range: f32,
    },
    AreaRect {
        position: [f32; 3],
        normal: [f32; 3],
        size: [f32; 2],
        two_sided: bool,
    },
}

/// Light data for GPU
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LightData {
    pub position_type: [f32; 4],   // xyz = position/direction, w = type
    pub color_intensity: [f32; 4], // rgb = color, a = intensity
    pub direction_range: [f32; 4], // xyz = direction, w = range
    pub spot_params: [f32; 4],     // x = inner cos, y = outer cos, zw = area size
}

/// Clustered lighting
pub struct ClusteredLighting {
    clusters: Vec<LightCluster>,
    grid_size: [u32; 3],
    near: f32,
    far: f32,
}

impl ClusteredLighting {
    pub fn new(grid_x: u32, grid_y: u32, grid_z: u32, near: f32, far: f32) -> Self {
        let total = (grid_x * grid_y * grid_z) as usize;
        Self {
            clusters: vec![LightCluster::default(); total],
            grid_size: [grid_x, grid_y, grid_z],
            near,
            far,
        }
    }

    /// Assign lights to clusters
    pub fn assign_lights(&mut self, lights: &[Light], view_proj: [[f32; 4]; 4]) {
        // Clear clusters
        for cluster in &mut self.clusters {
            cluster.light_count = 0;
        }

        for (i, light) in lights.iter().enumerate() {
            if !light.enabled {
                continue;
            }

            // Get light bounds in cluster space
            let (min_cluster, max_cluster) = match &light.light_type {
                LightType::Point { position, radius } => {
                    self.get_point_light_clusters(*position, *radius, view_proj)
                },
                LightType::Spot {
                    position, range, ..
                } => self.get_point_light_clusters(*position, *range, view_proj),
                LightType::Directional { .. } => {
                    // Affects all clusters
                    ([0, 0, 0], [
                        self.grid_size[0] - 1,
                        self.grid_size[1] - 1,
                        self.grid_size[2] - 1,
                    ])
                },
                LightType::AreaRect { position, .. } => {
                    // Approximate with point + radius
                    self.get_point_light_clusters(*position, 50.0, view_proj)
                },
            };

            // Add light to affected clusters
            for z in min_cluster[2]..=max_cluster[2] {
                for y in min_cluster[1]..=max_cluster[1] {
                    for x in min_cluster[0]..=max_cluster[0] {
                        let idx = self.cluster_index(x, y, z);
                        if let Some(cluster) = self.clusters.get_mut(idx) {
                            if cluster.light_count < 256 {
                                cluster.light_indices[cluster.light_count as usize] = i as u16;
                                cluster.light_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_point_light_clusters(
        &self,
        pos: [f32; 3],
        radius: f32,
        _view_proj: [[f32; 4]; 4],
    ) -> ([u32; 3], [u32; 3]) {
        // Simplified - would need proper projection
        let min_x = 0u32;
        let min_y = 0u32;
        let min_z = self.depth_to_slice(pos[2] - radius);
        let max_x = self.grid_size[0] - 1;
        let max_y = self.grid_size[1] - 1;
        let max_z = self.depth_to_slice(pos[2] + radius);

        ([min_x, min_y, min_z], [max_x, max_y, max_z])
    }

    fn depth_to_slice(&self, depth: f32) -> u32 {
        let log_far_near = (self.far / self.near).ln();
        let slice = (depth.ln() - self.near.ln()) / log_far_near * self.grid_size[2] as f32;
        (slice as u32).clamp(0, self.grid_size[2] - 1)
    }

    fn cluster_index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.grid_size[1] * self.grid_size[0] + y * self.grid_size[0] + x) as usize
    }
}

/// Light cluster
#[derive(Debug, Clone)]
struct LightCluster {
    light_indices: [u16; 256],
    light_count: u32,
}

impl Default for LightCluster {
    fn default() -> Self {
        Self {
            light_indices: [0; 256],
            light_count: 0,
        }
    }
}

/// Deferred lighting pass
pub struct DeferredLighting {
    pub ambient: [f32; 3],
    pub exposure: f32,
}

impl DeferredLighting {
    pub fn new() -> Self {
        Self {
            ambient: [0.03, 0.03, 0.03],
            exposure: 1.0,
        }
    }
}

impl Default for DeferredLighting {
    fn default() -> Self {
        Self::new()
    }
}
