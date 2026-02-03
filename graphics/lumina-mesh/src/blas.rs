//! Bottom-Level Acceleration Structure (BLAS) Management
//!
//! Ray tracing acceleration structure building and management.
//! Supports static, dynamic, and skinned geometry.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;
use lumina_math::{Mat4, Vec3};

use crate::mesh::{IndexFormat, MeshHandle, AABB};

// ============================================================================
// BLAS Handle
// ============================================================================

/// Handle to a BLAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlasHandle(Handle<Blas>);

impl BlasHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }
}

// ============================================================================
// BLAS Flags
// ============================================================================

bitflags! {
    /// Flags for BLAS creation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BlasFlags: u32 {
        /// Allow updates without full rebuild.
        const ALLOW_UPDATE = 1 << 0;
        /// Allow compaction.
        const ALLOW_COMPACTION = 1 << 1;
        /// Prefer fast trace over fast build.
        const PREFER_FAST_TRACE = 1 << 2;
        /// Prefer fast build over fast trace.
        const PREFER_FAST_BUILD = 1 << 3;
        /// Low memory mode.
        const LOW_MEMORY = 1 << 4;
        /// Contains opacity micromap.
        const CONTAINS_OPACITY_MICROMAP = 1 << 5;
        /// Contains displacement micromap.
        const CONTAINS_DISPLACEMENT_MICROMAP = 1 << 6;
    }
}

impl Default for BlasFlags {
    fn default() -> Self {
        BlasFlags::PREFER_FAST_TRACE | BlasFlags::ALLOW_COMPACTION
    }
}

// ============================================================================
// Geometry Type
// ============================================================================

/// Type of geometry in BLAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    /// Triangle mesh.
    Triangles,
    /// Procedural AABBs.
    Aabbs,
    /// Instances (for TLAS).
    Instances,
}

// ============================================================================
// Geometry Flags
// ============================================================================

bitflags! {
    /// Flags for geometry within BLAS.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GeometryFlags: u32 {
        /// Geometry is opaque.
        const OPAQUE = 1 << 0;
        /// No duplicate any-hit invocation.
        const NO_DUPLICATE_ANYHIT = 1 << 1;
    }
}

impl Default for GeometryFlags {
    fn default() -> Self {
        GeometryFlags::OPAQUE
    }
}

// ============================================================================
// Triangle Geometry
// ============================================================================

/// Triangle geometry for BLAS.
#[derive(Debug, Clone)]
pub struct TriangleGeometry {
    /// Vertex buffer data.
    pub vertex_data: Vec<u8>,
    /// Index buffer data.
    pub index_data: Vec<u8>,
    /// Vertex stride in bytes.
    pub vertex_stride: u32,
    /// Vertex count.
    pub vertex_count: u32,
    /// Index format.
    pub index_format: IndexFormat,
    /// Index count.
    pub index_count: u32,
    /// Transform (optional).
    pub transform: Option<Mat4>,
    /// Geometry flags.
    pub flags: GeometryFlags,
}

impl TriangleGeometry {
    /// Create new triangle geometry.
    pub fn new(
        vertex_stride: u32,
        vertex_count: u32,
        index_format: IndexFormat,
        index_count: u32,
    ) -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            vertex_stride,
            vertex_count,
            index_format,
            index_count,
            transform: None,
            flags: GeometryFlags::default(),
        }
    }

    /// Set vertex data.
    pub fn with_vertex_data(mut self, data: Vec<u8>) -> Self {
        self.vertex_data = data;
        self
    }

    /// Set index data.
    pub fn with_index_data(mut self, data: Vec<u8>) -> Self {
        self.index_data = data;
        self
    }

    /// Set transform.
    pub fn with_transform(mut self, transform: Mat4) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: GeometryFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Calculate primitive count.
    pub fn primitive_count(&self) -> u32 {
        self.index_count / 3
    }
}

// ============================================================================
// AABB Geometry
// ============================================================================

/// Axis-aligned bounding box geometry for BLAS.
#[derive(Debug, Clone)]
pub struct AabbGeometry {
    /// AABB data (6 floats per AABB: min_x, min_y, min_z, max_x, max_y, max_z).
    pub aabb_data: Vec<f32>,
    /// Stride in bytes.
    pub stride: u32,
    /// Number of AABBs.
    pub count: u32,
    /// Geometry flags.
    pub flags: GeometryFlags,
}

impl AabbGeometry {
    /// Create new AABB geometry.
    pub fn new(count: u32) -> Self {
        Self {
            aabb_data: Vec::new(),
            stride: 24, // 6 floats * 4 bytes
            count,
            flags: GeometryFlags::default(),
        }
    }

    /// Set AABB data.
    pub fn with_data(mut self, data: Vec<f32>) -> Self {
        self.aabb_data = data;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: GeometryFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Add an AABB.
    pub fn add_aabb(&mut self, min: Vec3, max: Vec3) {
        self.aabb_data
            .extend_from_slice(&[min.x, min.y, min.z, max.x, max.y, max.z]);
        self.count += 1;
    }
}

// ============================================================================
// BLAS Geometry
// ============================================================================

/// Geometry for BLAS construction.
#[derive(Debug, Clone)]
pub enum BlasGeometry {
    /// Triangle geometry.
    Triangles(TriangleGeometry),
    /// AABB geometry.
    Aabbs(AabbGeometry),
}

impl BlasGeometry {
    /// Get geometry type.
    pub fn geometry_type(&self) -> GeometryType {
        match self {
            BlasGeometry::Triangles(_) => GeometryType::Triangles,
            BlasGeometry::Aabbs(_) => GeometryType::Aabbs,
        }
    }

    /// Get geometry flags.
    pub fn flags(&self) -> GeometryFlags {
        match self {
            BlasGeometry::Triangles(g) => g.flags,
            BlasGeometry::Aabbs(g) => g.flags,
        }
    }

    /// Get primitive count.
    pub fn primitive_count(&self) -> u32 {
        match self {
            BlasGeometry::Triangles(g) => g.primitive_count(),
            BlasGeometry::Aabbs(g) => g.count,
        }
    }
}

// ============================================================================
// BLAS Description
// ============================================================================

/// Description for BLAS creation.
#[derive(Debug, Clone)]
pub struct BlasDesc {
    /// Name.
    pub name: String,
    /// Geometries.
    pub geometries: Vec<BlasGeometry>,
    /// Flags.
    pub flags: BlasFlags,
}

impl BlasDesc {
    /// Create a new description.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            geometries: Vec::new(),
            flags: BlasFlags::default(),
        }
    }

    /// Add triangle geometry.
    pub fn with_triangles(mut self, geometry: TriangleGeometry) -> Self {
        self.geometries.push(BlasGeometry::Triangles(geometry));
        self
    }

    /// Add AABB geometry.
    pub fn with_aabbs(mut self, geometry: AabbGeometry) -> Self {
        self.geometries.push(BlasGeometry::Aabbs(geometry));
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: BlasFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Calculate total primitives.
    pub fn total_primitives(&self) -> u32 {
        self.geometries.iter().map(|g| g.primitive_count()).sum()
    }
}

// ============================================================================
// BLAS Build Info
// ============================================================================

/// Information about BLAS build requirements.
#[derive(Debug, Clone, Default)]
pub struct BlasBuildInfo {
    /// Scratch memory required for build.
    pub scratch_size: u64,
    /// Scratch memory required for update.
    pub update_scratch_size: u64,
    /// Final acceleration structure size.
    pub acceleration_structure_size: u64,
    /// Build time (estimated).
    pub estimated_build_time_us: u64,
}

impl BlasBuildInfo {
    /// Estimate from primitives (rough approximation).
    pub fn estimate(primitive_count: u32, flags: BlasFlags) -> Self {
        let base_size = (primitive_count as u64) * 64; // ~64 bytes per primitive
        let scratch_multiplier = if flags.contains(BlasFlags::PREFER_FAST_BUILD) {
            2.0
        } else {
            1.5
        };

        Self {
            scratch_size: (base_size as f64 * scratch_multiplier) as u64,
            update_scratch_size: base_size / 2,
            acceleration_structure_size: base_size,
            estimated_build_time_us: (primitive_count as u64) / 10,
        }
    }
}

// ============================================================================
// BLAS State
// ============================================================================

/// State of a BLAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlasState {
    /// Not built.
    Pending,
    /// Currently building.
    Building,
    /// Built and ready.
    Ready,
    /// Needs rebuild.
    NeedsRebuild,
    /// Needs update.
    NeedsUpdate,
    /// Build failed.
    Failed,
}

// ============================================================================
// BLAS
// ============================================================================

/// Bottom-Level Acceleration Structure.
pub struct Blas {
    /// Handle.
    pub handle: BlasHandle,
    /// Name.
    pub name: String,
    /// Flags.
    pub flags: BlasFlags,
    /// Geometry count.
    pub geometry_count: u32,
    /// Total primitive count.
    pub primitive_count: u32,
    /// Build info.
    pub build_info: BlasBuildInfo,
    /// State.
    pub state: BlasState,
    /// GPU address (if built).
    pub gpu_address: u64,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Frame last used.
    pub last_used_frame: u64,
    /// Associated mesh.
    pub mesh: Option<MeshHandle>,
}

impl Blas {
    /// Create a new BLAS.
    pub fn new(handle: BlasHandle, desc: &BlasDesc) -> Self {
        let primitive_count = desc.total_primitives();
        let build_info = BlasBuildInfo::estimate(primitive_count, desc.flags);

        Self {
            handle,
            name: desc.name.clone(),
            flags: desc.flags,
            geometry_count: desc.geometries.len() as u32,
            primitive_count,
            build_info,
            state: BlasState::Pending,
            gpu_address: 0,
            size_bytes: 0,
            last_used_frame: 0,
            mesh: None,
        }
    }

    /// Check if ready for ray tracing.
    pub fn is_ready(&self) -> bool {
        self.state == BlasState::Ready
    }

    /// Check if needs rebuild.
    pub fn needs_rebuild(&self) -> bool {
        self.state == BlasState::NeedsRebuild
    }

    /// Check if needs update.
    pub fn needs_update(&self) -> bool {
        self.state == BlasState::NeedsUpdate
    }

    /// Check if can be updated.
    pub fn can_update(&self) -> bool {
        self.flags.contains(BlasFlags::ALLOW_UPDATE)
    }

    /// Mark for rebuild.
    pub fn mark_for_rebuild(&mut self) {
        self.state = BlasState::NeedsRebuild;
    }

    /// Mark for update.
    pub fn mark_for_update(&mut self) {
        if self.can_update() {
            self.state = BlasState::NeedsUpdate;
        } else {
            self.state = BlasState::NeedsRebuild;
        }
    }
}

// ============================================================================
// BLAS Build Request
// ============================================================================

/// Request to build a BLAS.
#[derive(Debug, Clone)]
pub struct BlasBuildRequest {
    /// Handle.
    pub handle: BlasHandle,
    /// Is update (not full rebuild).
    pub is_update: bool,
    /// Priority.
    pub priority: u32,
    /// Frame requested.
    pub frame: u64,
}

// ============================================================================
// BLAS Manager
// ============================================================================

/// Manager for BLAS instances.
pub struct BlasManager {
    /// All BLAS instances.
    instances: BTreeMap<u32, Blas>,
    /// Next handle index.
    next_index: AtomicU32,
    /// Pending builds.
    pending_builds: Vec<BlasBuildRequest>,
    /// Current frame.
    current_frame: AtomicU64,
    /// Total scratch memory available.
    scratch_budget: u64,
    /// Statistics.
    stats: BlasStats,
}

/// Statistics for BLAS manager.
#[derive(Debug, Clone, Default)]
pub struct BlasStats {
    /// Total BLAS count.
    pub total_count: u32,
    /// Ready BLAS count.
    pub ready_count: u32,
    /// Pending BLAS count.
    pub pending_count: u32,
    /// Total memory used.
    pub memory_used: u64,
    /// Total primitives.
    pub total_primitives: u64,
    /// Builds this frame.
    pub builds_this_frame: u32,
    /// Updates this frame.
    pub updates_this_frame: u32,
}

impl BlasManager {
    /// Create a new manager.
    pub fn new(scratch_budget: u64) -> Self {
        Self {
            instances: BTreeMap::new(),
            next_index: AtomicU32::new(0),
            pending_builds: Vec::new(),
            current_frame: AtomicU64::new(0),
            scratch_budget,
            stats: BlasStats::default(),
        }
    }

    /// Create a new BLAS.
    pub fn create(&mut self, desc: BlasDesc) -> BlasHandle {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let handle = BlasHandle::new(index, 0);
        let blas = Blas::new(handle, &desc);

        // Queue for build
        self.pending_builds.push(BlasBuildRequest {
            handle,
            is_update: false,
            priority: 0,
            frame: self.current_frame.load(Ordering::Relaxed),
        });

        self.instances.insert(index, blas);
        handle
    }

    /// Get a BLAS.
    pub fn get(&self, handle: BlasHandle) -> Option<&Blas> {
        self.instances.get(&handle.index())
    }

    /// Get mutable BLAS.
    pub fn get_mut(&mut self, handle: BlasHandle) -> Option<&mut Blas> {
        self.instances.get_mut(&handle.index())
    }

    /// Destroy a BLAS.
    pub fn destroy(&mut self, handle: BlasHandle) {
        self.instances.remove(&handle.index());
        self.pending_builds.retain(|r| r.handle != handle);
    }

    /// Begin frame.
    pub fn begin_frame(&mut self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
        self.stats.builds_this_frame = 0;
        self.stats.updates_this_frame = 0;
    }

    /// Process pending builds.
    pub fn update(&mut self) {
        let frame = self.current_frame.load(Ordering::Relaxed);

        // Sort by priority
        self.pending_builds.sort_by_key(|r| r.priority);

        // Process builds
        let mut scratch_used = 0u64;
        let mut completed = Vec::new();

        for request in &self.pending_builds {
            if let Some(blas) = self.instances.get(&request.handle.index()) {
                let scratch_needed = if request.is_update {
                    blas.build_info.update_scratch_size
                } else {
                    blas.build_info.scratch_size
                };

                if scratch_used + scratch_needed > self.scratch_budget {
                    break;
                }

                completed.push((request.handle, request.is_update));
                scratch_used += scratch_needed;
            }
        }

        // Complete builds
        for (handle, is_update) in &completed {
            if let Some(blas) = self.instances.get_mut(&handle.index()) {
                blas.state = BlasState::Ready;
                blas.size_bytes = blas.build_info.acceleration_structure_size;
                blas.last_used_frame = frame;

                if *is_update {
                    self.stats.updates_this_frame += 1;
                } else {
                    self.stats.builds_this_frame += 1;
                }
            }

            self.pending_builds.retain(|r| r.handle != *handle);
        }

        // Update stats
        self.update_stats();
    }

    /// Update statistics.
    fn update_stats(&mut self) {
        self.stats.total_count = self.instances.len() as u32;
        self.stats.ready_count = 0;
        self.stats.pending_count = 0;
        self.stats.memory_used = 0;
        self.stats.total_primitives = 0;

        for blas in self.instances.values() {
            match blas.state {
                BlasState::Ready => self.stats.ready_count += 1,
                BlasState::Pending | BlasState::NeedsRebuild | BlasState::NeedsUpdate => {
                    self.stats.pending_count += 1
                },
                _ => {},
            }

            self.stats.memory_used += blas.size_bytes;
            self.stats.total_primitives += blas.primitive_count as u64;
        }
    }

    /// Get statistics.
    pub fn stats(&self) -> &BlasStats {
        &self.stats
    }

    /// Get all ready BLAS handles.
    pub fn ready_blas(&self) -> Vec<BlasHandle> {
        self.instances
            .values()
            .filter(|b| b.is_ready())
            .map(|b| b.handle)
            .collect()
    }

    /// Request BLAS update.
    pub fn request_update(&mut self, handle: BlasHandle) {
        if let Some(blas) = self.instances.get_mut(&handle.index()) {
            blas.mark_for_update();

            let frame = self.current_frame.load(Ordering::Relaxed);
            self.pending_builds.push(BlasBuildRequest {
                handle,
                is_update: blas.can_update(),
                priority: 1,
                frame,
            });
        }
    }

    /// Request BLAS rebuild.
    pub fn request_rebuild(&mut self, handle: BlasHandle) {
        if let Some(blas) = self.instances.get_mut(&handle.index()) {
            blas.mark_for_rebuild();

            let frame = self.current_frame.load(Ordering::Relaxed);
            self.pending_builds.push(BlasBuildRequest {
                handle,
                is_update: false,
                priority: 1,
                frame,
            });
        }
    }

    /// Compact all compactable BLAS.
    pub fn compact_all(&mut self) {
        for blas in self.instances.values_mut() {
            if blas.flags.contains(BlasFlags::ALLOW_COMPACTION) && blas.is_ready() {
                // Simulate compaction
                blas.size_bytes = (blas.size_bytes as f64 * 0.8) as u64;
            }
        }
    }

    /// Get total memory usage.
    pub fn memory_used(&self) -> u64 {
        self.stats.memory_used
    }

    /// Get BLAS count.
    pub fn count(&self) -> usize {
        self.instances.len()
    }

    /// Clear all BLAS.
    pub fn clear(&mut self) {
        self.instances.clear();
        self.pending_builds.clear();
        self.stats = BlasStats::default();
    }
}

impl Default for BlasManager {
    fn default() -> Self {
        Self::new(256 * 1024 * 1024) // 256 MB scratch budget
    }
}

// ============================================================================
// TLAS Instance
// ============================================================================

/// Instance for Top-Level Acceleration Structure.
#[derive(Debug, Clone)]
pub struct TlasInstance {
    /// BLAS handle.
    pub blas: BlasHandle,
    /// Transform matrix (3x4).
    pub transform: [[f32; 4]; 3],
    /// Instance custom index (24-bit).
    pub custom_index: u32,
    /// Instance mask (8-bit).
    pub mask: u8,
    /// Shader binding table offset (24-bit).
    pub sbt_offset: u32,
    /// Instance flags.
    pub flags: TlasInstanceFlags,
}

bitflags! {
    /// Flags for TLAS instances.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TlasInstanceFlags: u8 {
        /// Disable triangle culling.
        const TRIANGLE_FACING_CULL_DISABLE = 1 << 0;
        /// Flip triangle facing.
        const TRIANGLE_FLIP_FACING = 1 << 1;
        /// Force opaque.
        const FORCE_OPAQUE = 1 << 2;
        /// Force no opaque.
        const FORCE_NO_OPAQUE = 1 << 3;
    }
}

impl TlasInstance {
    /// Create a new instance.
    pub fn new(blas: BlasHandle, transform: Mat4) -> Self {
        // Convert Mat4 to 3x4 row-major matrix
        let m = transform;
        let transform = [
            [m.x.x, m.y.x, m.z.x, m.w.x],
            [m.x.y, m.y.y, m.z.y, m.w.y],
            [m.x.z, m.y.z, m.z.z, m.w.z],
        ];

        Self {
            blas,
            transform,
            custom_index: 0,
            mask: 0xFF,
            sbt_offset: 0,
            flags: TlasInstanceFlags::empty(),
        }
    }

    /// Set custom index.
    pub fn with_custom_index(mut self, index: u32) -> Self {
        self.custom_index = index & 0xFFFFFF;
        self
    }

    /// Set mask.
    pub fn with_mask(mut self, mask: u8) -> Self {
        self.mask = mask;
        self
    }

    /// Set SBT offset.
    pub fn with_sbt_offset(mut self, offset: u32) -> Self {
        self.sbt_offset = offset & 0xFFFFFF;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: TlasInstanceFlags) -> Self {
        self.flags = flags;
        self
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU-ready BLAS reference.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GpuBlasReference {
    /// GPU address of BLAS.
    pub address: u64,
    /// Geometry offset.
    pub geometry_offset: u32,
    /// Geometry count.
    pub geometry_count: u32,
}

/// GPU-ready TLAS instance.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GpuTlasInstance {
    /// Transform matrix (3x4 row-major).
    pub transform: [[f32; 4]; 3],
    /// Custom index and mask packed.
    pub custom_index_and_mask: u32,
    /// SBT offset and flags packed.
    pub sbt_offset_and_flags: u32,
    /// BLAS GPU address.
    pub blas_address: u64,
}

impl GpuTlasInstance {
    /// Create from TlasInstance.
    pub fn from_instance(instance: &TlasInstance, blas_address: u64) -> Self {
        Self {
            transform: instance.transform,
            custom_index_and_mask: (instance.custom_index & 0xFFFFFF)
                | ((instance.mask as u32) << 24),
            sbt_offset_and_flags: (instance.sbt_offset & 0xFFFFFF)
                | ((instance.flags.bits() as u32) << 24),
            blas_address,
        }
    }

    /// Size in bytes.
    pub const fn size() -> usize {
        64 // 48 bytes transform + 16 bytes packed data
    }
}
