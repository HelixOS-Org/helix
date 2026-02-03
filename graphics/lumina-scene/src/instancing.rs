//! # GPU Instancing
//!
//! Automatic GPU instancing for repeated geometry.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::transform::Transform;
use super::RenderItem;

/// Instance manager
pub struct InstanceManager {
    instance_groups: BTreeMap<InstanceKey, Vec<InstanceData>>,
    buffer: Vec<u8>,
    stats: InstancingStats,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instance_groups: BTreeMap::new(),
            buffer: Vec::new(),
            stats: InstancingStats::default(),
        }
    }

    /// Prepare instances from render items
    pub fn prepare(&mut self, items: &[RenderItem]) {
        self.instance_groups.clear();
        self.stats = InstancingStats::default();

        // Group by mesh + material
        for item in items {
            let key = InstanceKey {
                mesh_id: item.mesh_id,
                material_id: item.material_id,
                lod_level: item.lod_level,
            };

            self.instance_groups
                .entry(key)
                .or_insert_with(Vec::new)
                .push(InstanceData {
                    transform: item.transform.world_matrix(),
                    custom_data: [0.0; 4],
                });
        }

        // Build buffer
        self.build_buffer();

        // Update stats
        self.stats.total_items = items.len() as u32;
        self.stats.instance_groups = self.instance_groups.len() as u32;
        self.stats.draw_calls_saved = items.len() as u32 - self.instance_groups.len() as u32;
    }

    fn build_buffer(&mut self) {
        self.buffer.clear();

        for instances in self.instance_groups.values() {
            for instance in instances {
                // Transform matrix (64 bytes)
                for row in &instance.transform {
                    for val in row {
                        self.buffer.extend_from_slice(&val.to_le_bytes());
                    }
                }

                // Custom data (16 bytes)
                for val in &instance.custom_data {
                    self.buffer.extend_from_slice(&val.to_le_bytes());
                }
            }
        }
    }

    /// Get instance buffer
    pub fn get_instance_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    /// Get draw calls
    pub fn get_draw_calls(&self) -> Vec<InstancedDrawCall> {
        let mut calls = Vec::new();
        let mut offset = 0u32;

        for (key, instances) in &self.instance_groups {
            calls.push(InstancedDrawCall {
                mesh_id: key.mesh_id,
                material_id: key.material_id,
                lod_level: key.lod_level,
                instance_count: instances.len() as u32,
                instance_offset: offset,
            });

            offset += instances.len() as u32;
        }

        calls
    }

    /// Get statistics
    pub fn stats(&self) -> &InstancingStats {
        &self.stats
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Instance key for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct InstanceKey {
    mesh_id: u64,
    material_id: u64,
    lod_level: u8,
}

/// Instance data
#[derive(Debug, Clone)]
pub struct InstanceData {
    pub transform: [[f32; 4]; 4],
    pub custom_data: [f32; 4],
}

/// Instanced draw call
#[derive(Debug, Clone)]
pub struct InstancedDrawCall {
    pub mesh_id: u64,
    pub material_id: u64,
    pub lod_level: u8,
    pub instance_count: u32,
    pub instance_offset: u32,
}

/// Instancing statistics
#[derive(Debug, Clone, Default)]
pub struct InstancingStats {
    pub total_items: u32,
    pub instance_groups: u32,
    pub draw_calls_saved: u32,
}

/// Indirect draw manager for GPU-driven rendering
pub struct IndirectDrawManager {
    commands: Vec<IndirectDrawCommand>,
    buffer: Vec<u8>,
}

impl IndirectDrawManager {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            buffer: Vec::new(),
        }
    }

    /// Add indirect draw command
    pub fn add_command(&mut self, cmd: IndirectDrawCommand) {
        self.commands.push(cmd);
    }

    /// Build command buffer
    pub fn build(&mut self) {
        self.buffer.clear();

        for cmd in &self.commands {
            // DrawIndexedIndirectCommand structure (20 bytes)
            self.buffer
                .extend_from_slice(&cmd.index_count.to_le_bytes());
            self.buffer
                .extend_from_slice(&cmd.instance_count.to_le_bytes());
            self.buffer
                .extend_from_slice(&cmd.first_index.to_le_bytes());
            self.buffer
                .extend_from_slice(&cmd.vertex_offset.to_le_bytes());
            self.buffer
                .extend_from_slice(&cmd.first_instance.to_le_bytes());
        }
    }

    /// Get command buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get command count
    pub fn command_count(&self) -> u32 {
        self.commands.len() as u32
    }
}

impl Default for IndirectDrawManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Indirect draw command
#[derive(Debug, Clone, Copy)]
pub struct IndirectDrawCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

/// Multi-draw indirect for batching
pub struct MultiDrawIndirect {
    commands: Vec<IndirectDrawCommand>,
    mesh_info: Vec<MeshInstanceInfo>,
}

impl MultiDrawIndirect {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            mesh_info: Vec::new(),
        }
    }

    /// Prepare multi-draw from groups
    pub fn prepare(
        &mut self,
        groups: &BTreeMap<u64, Vec<InstanceData>>,
        mesh_data: &MeshDataRegistry,
    ) {
        self.commands.clear();
        self.mesh_info.clear();

        let mut instance_offset = 0u32;

        for (mesh_id, instances) in groups {
            if let Some(mesh) = mesh_data.get(*mesh_id) {
                self.commands.push(IndirectDrawCommand {
                    index_count: mesh.index_count,
                    instance_count: instances.len() as u32,
                    first_index: mesh.first_index,
                    vertex_offset: mesh.vertex_offset,
                    first_instance: instance_offset,
                });

                self.mesh_info.push(MeshInstanceInfo {
                    mesh_id: *mesh_id,
                    instance_count: instances.len() as u32,
                });

                instance_offset += instances.len() as u32;
            }
        }
    }
}

impl Default for MultiDrawIndirect {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh instance info
#[derive(Debug, Clone)]
pub struct MeshInstanceInfo {
    pub mesh_id: u64,
    pub instance_count: u32,
}

/// Mesh data registry
pub struct MeshDataRegistry {
    meshes: BTreeMap<u64, MeshData>,
}

impl MeshDataRegistry {
    pub fn new() -> Self {
        Self {
            meshes: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, id: u64, data: MeshData) {
        self.meshes.insert(id, data);
    }

    pub fn get(&self, id: u64) -> Option<&MeshData> {
        self.meshes.get(&id)
    }
}

impl Default for MeshDataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh data
#[derive(Debug, Clone)]
pub struct MeshData {
    pub index_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub bounds: [f32; 6], // AABB min/max
}
