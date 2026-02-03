//! GPU Instancing Types for Lumina
//!
//! This module provides GPU instancing infrastructure including
//! instance buffers, indirect drawing, and instanced mesh rendering.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Instancing Handles
// ============================================================================

/// Instance buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceBufferHandle(pub u64);

impl InstanceBufferHandle {
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

impl Default for InstanceBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Instanced mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstancedMeshHandle(pub u64);

impl InstancedMeshHandle {
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

impl Default for InstancedMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Instance batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceBatchHandle(pub u64);

impl InstanceBatchHandle {
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

impl Default for InstanceBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Instance Data Types
// ============================================================================

/// Per-instance data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InstanceData {
    /// Model matrix (row-major)
    pub model_matrix: [[f32; 4]; 4],
}

impl InstanceData {
    /// Creates instance data
    pub const fn new(model_matrix: [[f32; 4]; 4]) -> Self {
        Self { model_matrix }
    }

    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            model_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// From translation
    pub fn from_translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            model_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [x, y, z, 1.0],
            ],
        }
    }

    /// From scale
    pub fn from_scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            model_matrix: [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy, 0.0, 0.0],
                [0.0, 0.0, sz, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// From uniform scale
    pub fn from_uniform_scale(s: f32) -> Self {
        Self::from_scale(s, s, s)
    }
}

impl Default for InstanceData {
    fn default() -> Self {
        Self::identity()
    }
}

/// Extended instance data with color
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColoredInstanceData {
    /// Model matrix
    pub model_matrix: [[f32; 4]; 4],
    /// Color
    pub color: [f32; 4],
}

impl ColoredInstanceData {
    /// Creates instance
    pub const fn new(model_matrix: [[f32; 4]; 4], color: [f32; 4]) -> Self {
        Self { model_matrix, color }
    }

    /// From transform and color
    pub fn from_transform_color(transform: InstanceData, color: [f32; 4]) -> Self {
        Self {
            model_matrix: transform.model_matrix,
            color,
        }
    }

    /// White instance
    pub fn white(model_matrix: [[f32; 4]; 4]) -> Self {
        Self {
            model_matrix,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl Default for ColoredInstanceData {
    fn default() -> Self {
        Self {
            model_matrix: InstanceData::identity().model_matrix,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Extended instance data with custom parameters
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExtendedInstanceData {
    /// Model matrix
    pub model_matrix: [[f32; 4]; 4],
    /// Normal matrix (inverse transpose of model, 3x3 padded)
    pub normal_matrix: [[f32; 4]; 3],
    /// Color
    pub color: [f32; 4],
    /// Custom parameters
    pub custom: [f32; 4],
}

impl ExtendedInstanceData {
    /// Creates instance
    pub fn new(model_matrix: [[f32; 4]; 4]) -> Self {
        Self {
            model_matrix,
            normal_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
            color: [1.0, 1.0, 1.0, 1.0],
            custom: [0.0; 4],
        }
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// With custom parameters
    pub fn with_custom(mut self, custom: [f32; 4]) -> Self {
        self.custom = custom;
        self
    }
}

impl Default for ExtendedInstanceData {
    fn default() -> Self {
        Self::new(InstanceData::identity().model_matrix)
    }
}

/// Sprite instance data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpriteInstanceData {
    /// Position
    pub position: [f32; 3],
    /// Rotation (radians)
    pub rotation: f32,
    /// Scale
    pub scale: [f32; 2],
    /// UV offset
    pub uv_offset: [f32; 2],
    /// UV scale
    pub uv_scale: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

impl SpriteInstanceData {
    /// Creates sprite instance
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            uv_scale: [1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// With scale
    pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
        self.scale = [sx, sy];
        self
    }

    /// With uniform scale
    pub fn with_uniform_scale(self, s: f32) -> Self {
        self.with_scale(s, s)
    }

    /// With UV
    pub fn with_uv(mut self, offset: [f32; 2], scale: [f32; 2]) -> Self {
        self.uv_offset = offset;
        self.uv_scale = scale;
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

impl Default for SpriteInstanceData {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}

// ============================================================================
// Instance Buffer
// ============================================================================

/// Instance buffer create info
#[derive(Clone, Debug)]
pub struct InstanceBufferCreateInfo {
    /// Capacity (number of instances)
    pub capacity: u32,
    /// Instance stride in bytes
    pub stride: u32,
    /// Usage
    pub usage: InstanceBufferUsage,
    /// Debug label
    pub label: Option<&'static str>,
}

impl InstanceBufferCreateInfo {
    /// Creates info
    pub fn new(capacity: u32, stride: u32) -> Self {
        Self {
            capacity,
            stride,
            usage: InstanceBufferUsage::Dynamic,
            label: None,
        }
    }

    /// For InstanceData
    pub fn for_instance_data(capacity: u32) -> Self {
        Self::new(capacity, core::mem::size_of::<InstanceData>() as u32)
    }

    /// For ColoredInstanceData
    pub fn for_colored_instance_data(capacity: u32) -> Self {
        Self::new(capacity, core::mem::size_of::<ColoredInstanceData>() as u32)
    }

    /// For ExtendedInstanceData
    pub fn for_extended_instance_data(capacity: u32) -> Self {
        Self::new(capacity, core::mem::size_of::<ExtendedInstanceData>() as u32)
    }

    /// For SpriteInstanceData
    pub fn for_sprite_instance_data(capacity: u32) -> Self {
        Self::new(capacity, core::mem::size_of::<SpriteInstanceData>() as u32)
    }

    /// Static usage
    pub fn static_usage(mut self) -> Self {
        self.usage = InstanceBufferUsage::Static;
        self
    }

    /// Stream usage
    pub fn stream(mut self) -> Self {
        self.usage = InstanceBufferUsage::Stream;
        self
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// Buffer size in bytes
    pub fn buffer_size(&self) -> u64 {
        (self.capacity * self.stride) as u64
    }
}

impl Default for InstanceBufferCreateInfo {
    fn default() -> Self {
        Self::for_instance_data(1024)
    }
}

/// Instance buffer usage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InstanceBufferUsage {
    /// Static (rarely updated)
    Static = 0,
    /// Dynamic (frequently updated)
    #[default]
    Dynamic = 1,
    /// Stream (updated every frame)
    Stream = 2,
}

// ============================================================================
// Instanced Mesh
// ============================================================================

/// Instanced mesh create info
#[derive(Clone, Debug)]
pub struct InstancedMeshCreateInfo {
    /// Mesh handle (reference to base mesh)
    pub mesh: u64,
    /// Instance buffer
    pub instance_buffer: InstanceBufferHandle,
    /// Max instances
    pub max_instances: u32,
    /// Culling mode
    pub culling: InstanceCullingMode,
    /// LOD settings
    pub lod: Option<InstanceLodSettings>,
}

impl InstancedMeshCreateInfo {
    /// Creates info
    pub fn new(mesh: u64, instance_buffer: InstanceBufferHandle, max_instances: u32) -> Self {
        Self {
            mesh,
            instance_buffer,
            max_instances,
            culling: InstanceCullingMode::PerBatch,
            lod: None,
        }
    }

    /// Per-instance culling
    pub fn per_instance_culling(mut self) -> Self {
        self.culling = InstanceCullingMode::PerInstance;
        self
    }

    /// No culling
    pub fn no_culling(mut self) -> Self {
        self.culling = InstanceCullingMode::None;
        self
    }

    /// With LOD
    pub fn with_lod(mut self, lod: InstanceLodSettings) -> Self {
        self.lod = Some(lod);
        self
    }
}

/// Instance culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InstanceCullingMode {
    /// No culling
    None = 0,
    /// Per-batch culling
    #[default]
    PerBatch = 1,
    /// Per-instance culling
    PerInstance = 2,
    /// GPU culling
    GpuCulling = 3,
}

/// Instance LOD settings
#[derive(Clone, Debug)]
pub struct InstanceLodSettings {
    /// LOD levels (distance thresholds)
    pub levels: Vec<InstanceLodLevel>,
    /// Fade transition range
    pub fade_range: f32,
    /// Use screen size instead of distance
    pub screen_size_based: bool,
}

impl InstanceLodSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            fade_range: 5.0,
            screen_size_based: false,
        }
    }

    /// Adds LOD level
    pub fn with_level(mut self, level: InstanceLodLevel) -> Self {
        self.levels.push(level);
        self
    }

    /// Simple two-level LOD
    pub fn simple(high_distance: f32, low_mesh: u64) -> Self {
        Self::new()
            .with_level(InstanceLodLevel::new(0, 0.0))
            .with_level(InstanceLodLevel::with_mesh(high_distance, low_mesh))
    }

    /// With fade
    pub fn with_fade(mut self, range: f32) -> Self {
        self.fade_range = range;
        self
    }
}

impl Default for InstanceLodSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Instance LOD level
#[derive(Clone, Copy, Debug)]
pub struct InstanceLodLevel {
    /// Mesh handle (0 = use base mesh)
    pub mesh: u64,
    /// Distance threshold
    pub distance: f32,
    /// Screen size threshold (if screen_size_based)
    pub screen_size: f32,
}

impl InstanceLodLevel {
    /// Creates level
    pub fn new(mesh: u64, distance: f32) -> Self {
        Self {
            mesh,
            distance,
            screen_size: 0.0,
        }
    }

    /// With different mesh
    pub fn with_mesh(distance: f32, mesh: u64) -> Self {
        Self {
            mesh,
            distance,
            screen_size: 0.0,
        }
    }
}

// ============================================================================
// Instance Batching
// ============================================================================

/// Instance batch
#[derive(Clone, Debug)]
pub struct InstanceBatch<T: Copy> {
    /// Instances
    instances: Vec<T>,
    /// Capacity
    capacity: usize,
    /// Dirty flag
    dirty: bool,
}

impl<T: Copy + Default> InstanceBatch<T> {
    /// Creates batch
    pub fn new(capacity: usize) -> Self {
        Self {
            instances: Vec::with_capacity(capacity),
            capacity,
            dirty: true,
        }
    }

    /// Clears batch
    pub fn clear(&mut self) {
        self.instances.clear();
        self.dirty = true;
    }

    /// Adds instance
    pub fn add(&mut self, instance: T) -> bool {
        if self.instances.len() >= self.capacity {
            return false;
        }
        self.instances.push(instance);
        self.dirty = true;
        true
    }

    /// Adds multiple instances
    pub fn add_many(&mut self, instances: &[T]) -> usize {
        let available = self.capacity - self.instances.len();
        let count = instances.len().min(available);
        self.instances.extend_from_slice(&instances[..count]);
        self.dirty = true;
        count
    }

    /// Gets instance
    pub fn get(&self, index: usize) -> Option<&T> {
        self.instances.get(index)
    }

    /// Gets mutable instance
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.dirty = true;
        self.instances.get_mut(index)
    }

    /// Sets instance
    pub fn set(&mut self, index: usize, instance: T) -> bool {
        if index >= self.instances.len() {
            return false;
        }
        self.instances[index] = instance;
        self.dirty = true;
        true
    }

    /// Removes instance (swap remove)
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.instances.len() {
            return None;
        }
        self.dirty = true;
        Some(self.instances.swap_remove(index))
    }

    /// Instance count
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Is full
    pub fn is_full(&self) -> bool {
        self.instances.len() >= self.capacity
    }

    /// Capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Available space
    pub fn available(&self) -> usize {
        self.capacity - self.instances.len()
    }

    /// Is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clears dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Gets data slice
    pub fn data(&self) -> &[T] {
        &self.instances
    }

    /// Gets data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self.instances.as_ptr() as *const u8;
        let len = self.instances.len() * core::mem::size_of::<T>();
        unsafe { core::slice::from_raw_parts(ptr, len) }
    }
}

impl<T: Copy + Default> Default for InstanceBatch<T> {
    fn default() -> Self {
        Self::new(1024)
    }
}

// ============================================================================
// Multi-Draw Indirect
// ============================================================================

/// Multi-draw batch
#[derive(Clone, Debug)]
pub struct MultiDrawBatch {
    /// Draw commands
    pub commands: Vec<MultiDrawCommand>,
    /// Instance buffer offset
    pub instance_offset: u32,
}

impl MultiDrawBatch {
    /// Creates batch
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            instance_offset: 0,
        }
    }

    /// Adds command
    pub fn add(&mut self, command: MultiDrawCommand) {
        self.commands.push(command);
    }

    /// Clears batch
    pub fn clear(&mut self) {
        self.commands.clear();
        self.instance_offset = 0;
    }

    /// Command count
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for MultiDrawBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-draw command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultiDrawCommand {
    /// Index count
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First index
    pub first_index: u32,
    /// Base vertex
    pub base_vertex: i32,
    /// Base instance
    pub base_instance: u32,
    /// Object ID (for bindless)
    pub object_id: u32,
    /// Material ID
    pub material_id: u32,
    /// Padding
    pub _padding: u32,
}

impl MultiDrawCommand {
    /// Creates command
    pub fn new(
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        base_instance: u32,
    ) -> Self {
        Self {
            index_count,
            instance_count,
            first_index,
            base_vertex,
            base_instance,
            object_id: 0,
            material_id: 0,
            _padding: 0,
        }
    }

    /// With object ID
    pub fn with_object_id(mut self, id: u32) -> Self {
        self.object_id = id;
        self
    }

    /// With material ID
    pub fn with_material_id(mut self, id: u32) -> Self {
        self.material_id = id;
        self
    }
}

impl Default for MultiDrawCommand {
    fn default() -> Self {
        Self {
            index_count: 0,
            instance_count: 1,
            first_index: 0,
            base_vertex: 0,
            base_instance: 0,
            object_id: 0,
            material_id: 0,
            _padding: 0,
        }
    }
}

// ============================================================================
// Instance Transform Utilities
// ============================================================================

/// Creates instances in a grid pattern
pub fn create_grid_instances(
    count_x: u32,
    count_y: u32,
    count_z: u32,
    spacing: f32,
    center: [f32; 3],
) -> Vec<InstanceData> {
    let mut instances = Vec::with_capacity((count_x * count_y * count_z) as usize);

    let offset_x = (count_x as f32 - 1.0) * spacing * 0.5;
    let offset_y = (count_y as f32 - 1.0) * spacing * 0.5;
    let offset_z = (count_z as f32 - 1.0) * spacing * 0.5;

    for z in 0..count_z {
        for y in 0..count_y {
            for x in 0..count_x {
                let px = center[0] + (x as f32 * spacing) - offset_x;
                let py = center[1] + (y as f32 * spacing) - offset_y;
                let pz = center[2] + (z as f32 * spacing) - offset_z;
                instances.push(InstanceData::from_translation(px, py, pz));
            }
        }
    }

    instances
}

/// Creates instances in a circle pattern
pub fn create_circle_instances(
    count: u32,
    radius: f32,
    center: [f32; 3],
    up_axis: u32,
) -> Vec<InstanceData> {
    let mut instances = Vec::with_capacity(count as usize);

    for i in 0..count {
        let angle = (i as f32 / count as f32) * core::f32::consts::TAU;
        let cos_a = angle.cos() * radius;
        let sin_a = angle.sin() * radius;

        let position = match up_axis {
            0 => [center[0], center[1] + cos_a, center[2] + sin_a], // X-up
            2 => [center[0] + cos_a, center[1] + sin_a, center[2]], // Z-up
            _ => [center[0] + cos_a, center[1], center[2] + sin_a], // Y-up (default)
        };

        instances.push(InstanceData::from_translation(position[0], position[1], position[2]));
    }

    instances
}

/// Creates instances with random transforms
pub fn create_random_instances(
    count: u32,
    min_pos: [f32; 3],
    max_pos: [f32; 3],
    seed: u32,
) -> Vec<InstanceData> {
    let mut instances = Vec::with_capacity(count as usize);
    let mut rng = SimpleLcg::new(seed);

    for _ in 0..count {
        let x = min_pos[0] + rng.next_f32() * (max_pos[0] - min_pos[0]);
        let y = min_pos[1] + rng.next_f32() * (max_pos[1] - min_pos[1]);
        let z = min_pos[2] + rng.next_f32() * (max_pos[2] - min_pos[2]);
        instances.push(InstanceData::from_translation(x, y, z));
    }

    instances
}

/// Simple LCG for deterministic random
struct SimpleLcg {
    state: u32,
}

impl SimpleLcg {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next() & 0x7FFFFFFF) as f32 / 0x7FFFFFFF as f32
    }
}

// ============================================================================
// Instancing Statistics
// ============================================================================

/// Instancing statistics
#[derive(Clone, Debug, Default)]
pub struct InstancingStats {
    /// Total instances
    pub total_instances: u32,
    /// Visible instances
    pub visible_instances: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Instances per draw call (average)
    pub instances_per_draw: f32,
    /// Buffer updates
    pub buffer_updates: u32,
    /// Bytes uploaded
    pub bytes_uploaded: u64,
}

impl InstancingStats {
    /// Culled instances
    pub fn culled_instances(&self) -> u32 {
        self.total_instances - self.visible_instances
    }

    /// Cull rate
    pub fn cull_rate(&self) -> f32 {
        if self.total_instances == 0 {
            0.0
        } else {
            self.culled_instances() as f32 / self.total_instances as f32
        }
    }
}
