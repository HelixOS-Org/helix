//! GPU Skinning Types for Lumina
//!
//! This module provides GPU-accelerated skeletal animation and
//! mesh skinning infrastructure for character rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// GPU Skinning Handles
// ============================================================================

/// GPU skinning system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuSkinningSystemHandle(pub u64);

impl GpuSkinningSystemHandle {
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

impl Default for GpuSkinningSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Skeleton handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkeletonHandle(pub u64);

impl SkeletonHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SkeletonHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Skin mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkinMeshHandle(pub u64);

impl SkinMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SkinMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Animation clip handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnimationClipHandle(pub u64);

impl AnimationClipHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for AnimationClipHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Animation instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnimationInstanceHandle(pub u64);

impl AnimationInstanceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for AnimationInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// GPU Skinning System Creation
// ============================================================================

/// GPU skinning system create info
#[derive(Clone, Debug)]
pub struct GpuSkinningSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max skeletons
    pub max_skeletons: u32,
    /// Max skinned meshes
    pub max_skinned_meshes: u32,
    /// Max bones per skeleton
    pub max_bones_per_skeleton: u32,
    /// Max blend shapes
    pub max_blend_shapes: u32,
    /// Features
    pub features: SkinningFeatures,
    /// Skinning method
    pub skinning_method: SkinningMethod,
}

impl GpuSkinningSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_skeletons: 256,
            max_skinned_meshes: 512,
            max_bones_per_skeleton: 256,
            max_blend_shapes: 64,
            features: SkinningFeatures::all(),
            skinning_method: SkinningMethod::ComputeShader,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max skeletons
    pub fn with_max_skeletons(mut self, count: u32) -> Self {
        self.max_skeletons = count;
        self
    }

    /// With max meshes
    pub fn with_max_meshes(mut self, count: u32) -> Self {
        self.max_skinned_meshes = count;
        self
    }

    /// With max bones
    pub fn with_max_bones(mut self, count: u32) -> Self {
        self.max_bones_per_skeleton = count;
        self
    }

    /// With max blend shapes
    pub fn with_max_blend_shapes(mut self, count: u32) -> Self {
        self.max_blend_shapes = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: SkinningFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With method
    pub fn with_method(mut self, method: SkinningMethod) -> Self {
        self.skinning_method = method;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// Crowd simulation (many characters)
    pub fn crowd() -> Self {
        Self::new()
            .with_max_skeletons(2048)
            .with_max_meshes(4096)
            .with_max_bones(128)
            .with_method(SkinningMethod::ComputeShader)
    }

    /// High quality (detailed characters)
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_skeletons(64)
            .with_max_meshes(128)
            .with_max_bones(512)
            .with_max_blend_shapes(256)
            .with_features(SkinningFeatures::all())
    }

    /// Mobile optimized
    pub fn mobile() -> Self {
        Self::new()
            .with_max_skeletons(32)
            .with_max_meshes(64)
            .with_max_bones(64)
            .with_max_blend_shapes(16)
            .with_method(SkinningMethod::VertexShader)
    }
}

impl Default for GpuSkinningSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Skinning features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct SkinningFeatures: u32 {
        /// None
        const NONE = 0;
        /// Linear blend skinning
        const LINEAR_BLEND = 1 << 0;
        /// Dual quaternion skinning
        const DUAL_QUATERNION = 1 << 1;
        /// Blend shapes (morph targets)
        const BLEND_SHAPES = 1 << 2;
        /// Animation blending
        const ANIMATION_BLEND = 1 << 3;
        /// IK solving
        const IK = 1 << 4;
        /// Physics bones
        const PHYSICS_BONES = 1 << 5;
        /// LOD
        const LOD = 1 << 6;
        /// Instancing
        const INSTANCING = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for SkinningFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Skinning method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SkinningMethod {
    /// Vertex shader skinning
    VertexShader = 0,
    /// Compute shader skinning
    #[default]
    ComputeShader = 1,
    /// Stream out (transform feedback)
    StreamOut = 2,
}

// ============================================================================
// Skeleton Definition
// ============================================================================

/// Skeleton create info
#[derive(Clone, Debug)]
pub struct SkeletonCreateInfo {
    /// Name
    pub name: String,
    /// Bones
    pub bones: Vec<BoneInfo>,
    /// Root bone index
    pub root_bone: u32,
}

impl SkeletonCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            bones: Vec::new(),
            root_bone: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add bone
    pub fn add_bone(mut self, bone: BoneInfo) -> Self {
        self.bones.push(bone);
        self
    }

    /// With root
    pub fn with_root(mut self, index: u32) -> Self {
        self.root_bone = index;
        self
    }

    /// Bone count
    pub fn bone_count(&self) -> u32 {
        self.bones.len() as u32
    }
}

impl Default for SkeletonCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Bone information
#[derive(Clone, Debug)]
pub struct BoneInfo {
    /// Name
    pub name: String,
    /// Parent index (-1 for root)
    pub parent_index: i32,
    /// Local bind pose
    pub local_bind_pose: BoneTransform,
    /// Inverse bind matrix
    pub inverse_bind_matrix: [[f32; 4]; 4],
    /// Flags
    pub flags: BoneFlags,
}

impl BoneInfo {
    /// Creates new bone
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent_index: -1,
            local_bind_pose: BoneTransform::identity(),
            inverse_bind_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            flags: BoneFlags::empty(),
        }
    }

    /// With parent
    pub fn with_parent(mut self, index: i32) -> Self {
        self.parent_index = index;
        self
    }

    /// With bind pose
    pub fn with_bind_pose(mut self, pose: BoneTransform) -> Self {
        self.local_bind_pose = pose;
        self
    }

    /// With inverse bind matrix
    pub fn with_inverse_bind(mut self, matrix: [[f32; 4]; 4]) -> Self {
        self.inverse_bind_matrix = matrix;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: BoneFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Root bone
    pub fn root(name: impl Into<String>) -> Self {
        Self::new(name).with_parent(-1)
    }

    /// Child bone
    pub fn child(name: impl Into<String>, parent: i32) -> Self {
        Self::new(name).with_parent(parent)
    }
}

impl Default for BoneInfo {
    fn default() -> Self {
        Self::new("")
    }
}

bitflags::bitflags! {
    /// Bone flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct BoneFlags: u32 {
        /// None
        const NONE = 0;
        /// Is leaf bone
        const LEAF = 1 << 0;
        /// Has IK target
        const IK_TARGET = 1 << 1;
        /// Physics simulated
        const PHYSICS = 1 << 2;
        /// User control
        const USER_CONTROL = 1 << 3;
        /// Hidden
        const HIDDEN = 1 << 4;
    }
}

/// Bone transform
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BoneTransform {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl BoneTransform {
    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// With position
    pub const fn with_position(mut self, pos: [f32; 3]) -> Self {
        self.position = pos;
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, rot: [f32; 4]) -> Self {
        self.rotation = rot;
        self
    }

    /// With scale
    pub const fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// With uniform scale
    pub const fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = [scale, scale, scale];
        self
    }
}

impl Default for BoneTransform {
    fn default() -> Self {
        Self::identity()
    }
}

// ============================================================================
// Skin Mesh
// ============================================================================

/// Skin mesh create info
#[derive(Clone, Debug)]
pub struct SkinMeshCreateInfo {
    /// Name
    pub name: String,
    /// Vertex count
    pub vertex_count: u32,
    /// Bone weights per vertex
    pub bones_per_vertex: u32,
    /// Skeleton
    pub skeleton: SkeletonHandle,
    /// Blend shapes
    pub blend_shapes: Vec<BlendShapeInfo>,
    /// Features
    pub features: SkinMeshFeatures,
}

impl SkinMeshCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            vertex_count: 0,
            bones_per_vertex: 4,
            skeleton: SkeletonHandle::NULL,
            blend_shapes: Vec::new(),
            features: SkinMeshFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With vertex count
    pub fn with_vertices(mut self, count: u32) -> Self {
        self.vertex_count = count;
        self
    }

    /// With bones per vertex
    pub fn with_bones_per_vertex(mut self, count: u32) -> Self {
        self.bones_per_vertex = count;
        self
    }

    /// With skeleton
    pub fn with_skeleton(mut self, skeleton: SkeletonHandle) -> Self {
        self.skeleton = skeleton;
        self
    }

    /// Add blend shape
    pub fn add_blend_shape(mut self, shape: BlendShapeInfo) -> Self {
        self.blend_shapes.push(shape);
        self
    }

    /// With features
    pub fn with_features(mut self, features: SkinMeshFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard 4-bone skinning
    pub fn standard(vertex_count: u32) -> Self {
        Self::new()
            .with_vertices(vertex_count)
            .with_bones_per_vertex(4)
    }

    /// 8-bone skinning (high quality)
    pub fn high_quality(vertex_count: u32) -> Self {
        Self::new()
            .with_vertices(vertex_count)
            .with_bones_per_vertex(8)
            .with_features(SkinMeshFeatures::DUAL_QUATERNION)
    }
}

impl Default for SkinMeshCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Skin mesh features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct SkinMeshFeatures: u32 {
        /// None
        const NONE = 0;
        /// Dual quaternion
        const DUAL_QUATERNION = 1 << 0;
        /// Has blend shapes
        const BLEND_SHAPES = 1 << 1;
        /// Tangent skinning
        const TANGENT_SKINNING = 1 << 2;
        /// Velocity output
        const VELOCITY = 1 << 3;
    }
}

/// Blend shape info
#[derive(Clone, Debug)]
pub struct BlendShapeInfo {
    /// Name
    pub name: String,
    /// Vertex deltas
    pub vertex_count: u32,
    /// Has normals
    pub has_normals: bool,
    /// Has tangents
    pub has_tangents: bool,
}

impl BlendShapeInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            vertex_count: 0,
            has_normals: true,
            has_tangents: false,
        }
    }

    /// With vertex count
    pub fn with_vertices(mut self, count: u32) -> Self {
        self.vertex_count = count;
        self
    }

    /// With normals
    pub fn with_normals(mut self, has: bool) -> Self {
        self.has_normals = has;
        self
    }

    /// With tangents
    pub fn with_tangents(mut self, has: bool) -> Self {
        self.has_tangents = has;
        self
    }
}

impl Default for BlendShapeInfo {
    fn default() -> Self {
        Self::new("")
    }
}

// ============================================================================
// Animation
// ============================================================================

/// Animation clip create info
#[derive(Clone, Debug)]
pub struct AnimationClipCreateInfo {
    /// Name
    pub name: String,
    /// Duration (seconds)
    pub duration: f32,
    /// Sample rate (fps)
    pub sample_rate: f32,
    /// Bone tracks
    pub bone_tracks: Vec<BoneAnimationTrack>,
    /// Blend shape tracks
    pub blend_shape_tracks: Vec<BlendShapeTrack>,
    /// Flags
    pub flags: AnimationFlags,
}

impl AnimationClipCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            duration: 0.0,
            sample_rate: 30.0,
            bone_tracks: Vec::new(),
            blend_shape_tracks: Vec::new(),
            flags: AnimationFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// With sample rate
    pub fn with_sample_rate(mut self, fps: f32) -> Self {
        self.sample_rate = fps;
        self
    }

    /// Add bone track
    pub fn add_bone_track(mut self, track: BoneAnimationTrack) -> Self {
        self.bone_tracks.push(track);
        self
    }

    /// Add blend shape track
    pub fn add_blend_shape_track(mut self, track: BlendShapeTrack) -> Self {
        self.blend_shape_tracks.push(track);
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: AnimationFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Looping
    pub fn looping(mut self) -> Self {
        self.flags |= AnimationFlags::LOOP;
        self
    }
}

impl Default for AnimationClipCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Animation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AnimationFlags: u32 {
        /// None
        const NONE = 0;
        /// Loop
        const LOOP = 1 << 0;
        /// Additive
        const ADDITIVE = 1 << 1;
        /// Root motion
        const ROOT_MOTION = 1 << 2;
        /// Compressed
        const COMPRESSED = 1 << 3;
    }
}

/// Bone animation track
#[derive(Clone, Debug)]
pub struct BoneAnimationTrack {
    /// Bone index
    pub bone_index: u32,
    /// Position keys
    pub position_keys: Vec<PositionKey>,
    /// Rotation keys
    pub rotation_keys: Vec<RotationKey>,
    /// Scale keys
    pub scale_keys: Vec<ScaleKey>,
}

impl BoneAnimationTrack {
    /// Creates new track
    pub fn new(bone_index: u32) -> Self {
        Self {
            bone_index,
            position_keys: Vec::new(),
            rotation_keys: Vec::new(),
            scale_keys: Vec::new(),
        }
    }

    /// Add position key
    pub fn add_position(mut self, time: f32, value: [f32; 3]) -> Self {
        self.position_keys.push(PositionKey { time, value });
        self
    }

    /// Add rotation key
    pub fn add_rotation(mut self, time: f32, value: [f32; 4]) -> Self {
        self.rotation_keys.push(RotationKey { time, value });
        self
    }

    /// Add scale key
    pub fn add_scale(mut self, time: f32, value: [f32; 3]) -> Self {
        self.scale_keys.push(ScaleKey { time, value });
        self
    }
}

impl Default for BoneAnimationTrack {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Position keyframe
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PositionKey {
    /// Time
    pub time: f32,
    /// Value
    pub value: [f32; 3],
}

/// Rotation keyframe
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RotationKey {
    /// Time
    pub time: f32,
    /// Value (quaternion)
    pub value: [f32; 4],
}

impl Default for RotationKey {
    fn default() -> Self {
        Self {
            time: 0.0,
            value: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Scale keyframe
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ScaleKey {
    /// Time
    pub time: f32,
    /// Value
    pub value: [f32; 3],
}

impl Default for ScaleKey {
    fn default() -> Self {
        Self {
            time: 0.0,
            value: [1.0, 1.0, 1.0],
        }
    }
}

/// Blend shape track
#[derive(Clone, Debug)]
pub struct BlendShapeTrack {
    /// Blend shape index
    pub shape_index: u32,
    /// Weight keys
    pub keys: Vec<BlendShapeKey>,
}

impl BlendShapeTrack {
    /// Creates new track
    pub fn new(shape_index: u32) -> Self {
        Self {
            shape_index,
            keys: Vec::new(),
        }
    }

    /// Add key
    pub fn add_key(mut self, time: f32, weight: f32) -> Self {
        self.keys.push(BlendShapeKey { time, weight });
        self
    }
}

impl Default for BlendShapeTrack {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Blend shape keyframe
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BlendShapeKey {
    /// Time
    pub time: f32,
    /// Weight
    pub weight: f32,
}

// ============================================================================
// Animation Playback
// ============================================================================

/// Animation instance state
#[derive(Clone, Debug)]
pub struct AnimationInstanceState {
    /// Handle
    pub handle: AnimationInstanceHandle,
    /// Clip
    pub clip: AnimationClipHandle,
    /// Current time
    pub time: f32,
    /// Speed
    pub speed: f32,
    /// Weight
    pub weight: f32,
    /// State
    pub state: PlaybackState,
    /// Blend shape weights
    pub blend_shape_weights: Vec<f32>,
}

impl AnimationInstanceState {
    /// Creates new state
    pub fn new(clip: AnimationClipHandle) -> Self {
        Self {
            handle: AnimationInstanceHandle::NULL,
            clip,
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            state: PlaybackState::Stopped,
            blend_shape_weights: Vec::new(),
        }
    }

    /// With speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// With weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Play
    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
    }

    /// Pause
    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    /// Stop
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.time = 0.0;
    }
}

impl Default for AnimationInstanceState {
    fn default() -> Self {
        Self::new(AnimationClipHandle::NULL)
    }
}

/// Playback state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PlaybackState {
    /// Stopped
    #[default]
    Stopped = 0,
    /// Playing
    Playing = 1,
    /// Paused
    Paused = 2,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU skin vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSkinVertex {
    /// Position
    pub position: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Tangent
    pub tangent: [f32; 4],
    /// UV
    pub uv: [f32; 2],
    /// Bone indices
    pub bone_indices: [u32; 4],
    /// Bone weights
    pub bone_weights: [f32; 4],
}

/// GPU bone matrix
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GpuBoneMatrix {
    /// Transform (4x3 matrix, row major)
    pub transform: [[f32; 4]; 3],
}

impl Default for GpuBoneMatrix {
    fn default() -> Self {
        Self {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        }
    }
}

/// GPU skinning constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSkinningConstants {
    /// Vertex count
    pub vertex_count: u32,
    /// Bone count
    pub bone_count: u32,
    /// Blend shape count
    pub blend_shape_count: u32,
    /// Bones per vertex
    pub bones_per_vertex: u32,
}

/// GPU blend shape delta
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuBlendShapeDelta {
    /// Vertex index
    pub vertex_index: u32,
    /// Position delta
    pub position_delta: [f32; 3],
    /// Normal delta
    pub normal_delta: [f32; 3],
    /// Tangent delta
    pub tangent_delta: [f32; 3],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU skinning statistics
#[derive(Clone, Debug, Default)]
pub struct GpuSkinningStats {
    /// Active skeletons
    pub active_skeletons: u32,
    /// Active meshes
    pub skinned_meshes: u32,
    /// Total bones
    pub total_bones: u32,
    /// Total vertices skinned
    pub vertices_skinned: u64,
    /// Blend shapes active
    pub blend_shapes_active: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
    /// Bone buffer memory
    pub bone_buffer_memory: u64,
}

impl GpuSkinningStats {
    /// Vertices per mesh
    pub fn avg_vertices_per_mesh(&self) -> f32 {
        if self.skinned_meshes == 0 {
            0.0
        } else {
            self.vertices_skinned as f32 / self.skinned_meshes as f32
        }
    }

    /// Bones per skeleton
    pub fn avg_bones_per_skeleton(&self) -> f32 {
        if self.active_skeletons == 0 {
            0.0
        } else {
            self.total_bones as f32 / self.active_skeletons as f32
        }
    }
}
