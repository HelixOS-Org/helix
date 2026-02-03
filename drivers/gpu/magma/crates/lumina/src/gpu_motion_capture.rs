//! GPU Motion Capture System for Lumina
//!
//! This module provides GPU-accelerated motion capture data processing
//! including pose estimation, skeleton mapping, and animation retargeting.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Motion Capture Handles
// ============================================================================

/// GPU motion capture system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuMotionCaptureHandle(pub u64);

impl GpuMotionCaptureHandle {
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

impl Default for GpuMotionCaptureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Motion clip handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MotionClipHandle(pub u64);

impl MotionClipHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MotionClipHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Skeleton definition handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkeletonDefHandle(pub u64);

impl SkeletonDefHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SkeletonDefHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Motion retarget handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MotionRetargetHandle(pub u64);

impl MotionRetargetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MotionRetargetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Motion Capture System Creation
// ============================================================================

/// GPU motion capture system create info
#[derive(Clone, Debug)]
pub struct GpuMotionCaptureCreateInfo {
    /// Name
    pub name: String,
    /// Max clips
    pub max_clips: u32,
    /// Max skeletons
    pub max_skeletons: u32,
    /// Max active instances
    pub max_instances: u32,
    /// Features
    pub features: MotionCaptureFeatures,
}

impl GpuMotionCaptureCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_clips: 256,
            max_skeletons: 64,
            max_instances: 1000,
            features: MotionCaptureFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max clips
    pub fn with_max_clips(mut self, count: u32) -> Self {
        self.max_clips = count;
        self
    }

    /// With max skeletons
    pub fn with_max_skeletons(mut self, count: u32) -> Self {
        self.max_skeletons = count;
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, count: u32) -> Self {
        self.max_instances = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: MotionCaptureFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_clips(1024)
            .with_max_skeletons(256)
            .with_max_instances(10000)
    }
}

impl Default for GpuMotionCaptureCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Motion capture features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct MotionCaptureFeatures: u32 {
        /// None
        const NONE = 0;
        /// Pose estimation
        const POSE_ESTIMATION = 1 << 0;
        /// Motion retargeting
        const RETARGETING = 1 << 1;
        /// IK solving
        const IK = 1 << 2;
        /// Motion blending
        const BLENDING = 1 << 3;
        /// Motion compression
        const COMPRESSION = 1 << 4;
        /// Motion cleanup
        const CLEANUP = 1 << 5;
        /// Root motion extraction
        const ROOT_MOTION = 1 << 6;
        /// GPU processing
        const GPU_PROCESSING = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for MotionCaptureFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Motion Clip
// ============================================================================

/// Motion clip create info
#[derive(Clone, Debug)]
pub struct MotionClipCreateInfo {
    /// Name
    pub name: String,
    /// Skeleton definition
    pub skeleton: SkeletonDefHandle,
    /// Frame rate
    pub frame_rate: f32,
    /// Frames
    pub frames: Vec<MotionFrame>,
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Root motion settings
    pub root_motion: RootMotionSettings,
}

impl MotionClipCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            skeleton: SkeletonDefHandle::NULL,
            frame_rate: 30.0,
            frames: Vec::new(),
            loop_mode: LoopMode::Once,
            root_motion: RootMotionSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With skeleton
    pub fn with_skeleton(mut self, skeleton: SkeletonDefHandle) -> Self {
        self.skeleton = skeleton;
        self
    }

    /// With frame rate
    pub fn with_frame_rate(mut self, fps: f32) -> Self {
        self.frame_rate = fps;
        self
    }

    /// Add frame
    pub fn add_frame(mut self, frame: MotionFrame) -> Self {
        self.frames.push(frame);
        self
    }

    /// With frames
    pub fn with_frames(mut self, frames: Vec<MotionFrame>) -> Self {
        self.frames = frames;
        self
    }

    /// With loop mode
    pub fn with_loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// With root motion
    pub fn with_root_motion(mut self, settings: RootMotionSettings) -> Self {
        self.root_motion = settings;
        self
    }

    /// Duration in seconds
    pub fn duration(&self) -> f32 {
        if self.frame_rate > 0.0 && !self.frames.is_empty() {
            (self.frames.len() as f32 - 1.0) / self.frame_rate
        } else {
            0.0
        }
    }
}

impl Default for MotionClipCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Motion frame
#[derive(Clone, Debug)]
pub struct MotionFrame {
    /// Time
    pub time: f32,
    /// Joint poses
    pub poses: Vec<JointPose>,
    /// Root position
    pub root_position: [f32; 3],
    /// Root rotation
    pub root_rotation: [f32; 4],
}

impl MotionFrame {
    /// Creates new frame
    pub fn new(time: f32) -> Self {
        Self {
            time,
            poses: Vec::new(),
            root_position: [0.0, 0.0, 0.0],
            root_rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Add pose
    pub fn add_pose(mut self, pose: JointPose) -> Self {
        self.poses.push(pose);
        self
    }

    /// With root position
    pub fn with_root_position(mut self, position: [f32; 3]) -> Self {
        self.root_position = position;
        self
    }

    /// With root rotation
    pub fn with_root_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.root_rotation = rotation;
        self
    }
}

impl Default for MotionFrame {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Joint pose
#[derive(Clone, Copy, Debug)]
pub struct JointPose {
    /// Joint index
    pub joint_index: u32,
    /// Local rotation (quaternion)
    pub rotation: [f32; 4],
    /// Local position
    pub position: [f32; 3],
    /// Local scale
    pub scale: [f32; 3],
}

impl JointPose {
    /// Creates new pose
    pub const fn new(joint_index: u32) -> Self {
        Self {
            joint_index,
            rotation: [0.0, 0.0, 0.0, 1.0],
            position: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// With rotation
    pub const fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// With position
    pub const fn with_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// With scale
    pub const fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// Identity pose
    pub const fn identity(joint_index: u32) -> Self {
        Self::new(joint_index)
    }
}

impl Default for JointPose {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Loop mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoopMode {
    /// Play once
    #[default]
    Once = 0,
    /// Loop
    Loop = 1,
    /// Ping pong
    PingPong = 2,
    /// Clamp forever
    ClampForever = 3,
}

// ============================================================================
// Root Motion
// ============================================================================

/// Root motion settings
#[derive(Clone, Copy, Debug)]
pub struct RootMotionSettings {
    /// Extract root motion
    pub enabled: bool,
    /// Extract X translation
    pub extract_x: bool,
    /// Extract Y translation
    pub extract_y: bool,
    /// Extract Z translation
    pub extract_z: bool,
    /// Extract rotation
    pub extract_rotation: bool,
    /// Ground height
    pub ground_height: f32,
}

impl RootMotionSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: false,
            extract_x: true,
            extract_y: false,
            extract_z: true,
            extract_rotation: true,
            ground_height: 0.0,
        }
    }

    /// Enabled with defaults
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            extract_x: true,
            extract_y: false,
            extract_z: true,
            extract_rotation: true,
            ground_height: 0.0,
        }
    }

    /// Full extraction
    pub const fn full() -> Self {
        Self {
            enabled: true,
            extract_x: true,
            extract_y: true,
            extract_z: true,
            extract_rotation: true,
            ground_height: 0.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self::new()
    }
}

impl Default for RootMotionSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Skeleton Definition
// ============================================================================

/// Skeleton definition create info
#[derive(Clone, Debug)]
pub struct SkeletonDefCreateInfo {
    /// Name
    pub name: String,
    /// Joints
    pub joints: Vec<JointDef>,
    /// Root joint index
    pub root_joint: u32,
}

impl SkeletonDefCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            joints: Vec::new(),
            root_joint: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add joint
    pub fn add_joint(mut self, joint: JointDef) -> Self {
        self.joints.push(joint);
        self
    }

    /// With joints
    pub fn with_joints(mut self, joints: Vec<JointDef>) -> Self {
        self.joints = joints;
        self
    }

    /// With root joint
    pub fn with_root(mut self, index: u32) -> Self {
        self.root_joint = index;
        self
    }

    /// Humanoid skeleton preset
    pub fn humanoid() -> Self {
        Self::new()
            .with_name("Humanoid")
            .add_joint(JointDef::new("Hips", -1))
            .add_joint(JointDef::new("Spine", 0))
            .add_joint(JointDef::new("Spine1", 1))
            .add_joint(JointDef::new("Spine2", 2))
            .add_joint(JointDef::new("Neck", 3))
            .add_joint(JointDef::new("Head", 4))
            .add_joint(JointDef::new("LeftShoulder", 3))
            .add_joint(JointDef::new("LeftArm", 6))
            .add_joint(JointDef::new("LeftForeArm", 7))
            .add_joint(JointDef::new("LeftHand", 8))
            .add_joint(JointDef::new("RightShoulder", 3))
            .add_joint(JointDef::new("RightArm", 10))
            .add_joint(JointDef::new("RightForeArm", 11))
            .add_joint(JointDef::new("RightHand", 12))
            .add_joint(JointDef::new("LeftUpLeg", 0))
            .add_joint(JointDef::new("LeftLeg", 14))
            .add_joint(JointDef::new("LeftFoot", 15))
            .add_joint(JointDef::new("LeftToeBase", 16))
            .add_joint(JointDef::new("RightUpLeg", 0))
            .add_joint(JointDef::new("RightLeg", 18))
            .add_joint(JointDef::new("RightFoot", 19))
            .add_joint(JointDef::new("RightToeBase", 20))
    }
}

impl Default for SkeletonDefCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Joint definition
#[derive(Clone, Debug)]
pub struct JointDef {
    /// Joint name
    pub name: String,
    /// Parent joint index (-1 for root)
    pub parent: i32,
    /// Bind pose position
    pub bind_position: [f32; 3],
    /// Bind pose rotation
    pub bind_rotation: [f32; 4],
    /// Joint type
    pub joint_type: JointType,
}

impl JointDef {
    /// Creates new joint
    pub fn new(name: impl Into<String>, parent: i32) -> Self {
        Self {
            name: name.into(),
            parent,
            bind_position: [0.0, 0.0, 0.0],
            bind_rotation: [0.0, 0.0, 0.0, 1.0],
            joint_type: JointType::Generic,
        }
    }

    /// With bind position
    pub fn with_bind_position(mut self, position: [f32; 3]) -> Self {
        self.bind_position = position;
        self
    }

    /// With bind rotation
    pub fn with_bind_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.bind_rotation = rotation;
        self
    }

    /// With joint type
    pub fn with_type(mut self, joint_type: JointType) -> Self {
        self.joint_type = joint_type;
        self
    }
}

impl Default for JointDef {
    fn default() -> Self {
        Self::new("Joint", -1)
    }
}

/// Joint type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum JointType {
    /// Generic joint
    #[default]
    Generic = 0,
    /// Root/hips
    Root = 1,
    /// Spine
    Spine = 2,
    /// Head
    Head = 3,
    /// Arm
    Arm = 4,
    /// Hand
    Hand = 5,
    /// Finger
    Finger = 6,
    /// Leg
    Leg = 7,
    /// Foot
    Foot = 8,
    /// Toe
    Toe = 9,
}

// ============================================================================
// Motion Retargeting
// ============================================================================

/// Motion retarget create info
#[derive(Clone, Debug)]
pub struct MotionRetargetCreateInfo {
    /// Name
    pub name: String,
    /// Source skeleton
    pub source_skeleton: SkeletonDefHandle,
    /// Target skeleton
    pub target_skeleton: SkeletonDefHandle,
    /// Joint mappings
    pub mappings: Vec<JointMapping>,
    /// Retarget options
    pub options: RetargetOptions,
}

impl MotionRetargetCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            source_skeleton: SkeletonDefHandle::NULL,
            target_skeleton: SkeletonDefHandle::NULL,
            mappings: Vec::new(),
            options: RetargetOptions::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With source skeleton
    pub fn from_skeleton(mut self, skeleton: SkeletonDefHandle) -> Self {
        self.source_skeleton = skeleton;
        self
    }

    /// With target skeleton
    pub fn to_skeleton(mut self, skeleton: SkeletonDefHandle) -> Self {
        self.target_skeleton = skeleton;
        self
    }

    /// Add mapping
    pub fn add_mapping(mut self, mapping: JointMapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    /// With options
    pub fn with_options(mut self, options: RetargetOptions) -> Self {
        self.options = options;
        self
    }
}

impl Default for MotionRetargetCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Joint mapping
#[derive(Clone, Debug)]
pub struct JointMapping {
    /// Source joint index
    pub source_joint: u32,
    /// Target joint index
    pub target_joint: u32,
    /// Rotation offset
    pub rotation_offset: [f32; 4],
    /// Position scale
    pub position_scale: f32,
}

impl JointMapping {
    /// Creates new mapping
    pub const fn new(source: u32, target: u32) -> Self {
        Self {
            source_joint: source,
            target_joint: target,
            rotation_offset: [0.0, 0.0, 0.0, 1.0],
            position_scale: 1.0,
        }
    }

    /// With rotation offset
    pub const fn with_rotation_offset(mut self, offset: [f32; 4]) -> Self {
        self.rotation_offset = offset;
        self
    }

    /// With position scale
    pub const fn with_position_scale(mut self, scale: f32) -> Self {
        self.position_scale = scale;
        self
    }
}

impl Default for JointMapping {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Retarget options
#[derive(Clone, Copy, Debug)]
pub struct RetargetOptions {
    /// Preserve scale
    pub preserve_scale: bool,
    /// Use IK
    pub use_ik: bool,
    /// Maintain foot contact
    pub foot_ik: bool,
    /// Maintain hand position
    pub hand_ik: bool,
    /// Height adjustment
    pub height_adjustment: f32,
}

impl RetargetOptions {
    /// Default options
    pub const fn new() -> Self {
        Self {
            preserve_scale: false,
            use_ik: true,
            foot_ik: true,
            hand_ik: false,
            height_adjustment: 0.0,
        }
    }

    /// Simple retarget
    pub const fn simple() -> Self {
        Self {
            preserve_scale: false,
            use_ik: false,
            foot_ik: false,
            hand_ik: false,
            height_adjustment: 0.0,
        }
    }

    /// Full IK
    pub const fn full_ik() -> Self {
        Self {
            preserve_scale: true,
            use_ik: true,
            foot_ik: true,
            hand_ik: true,
            height_adjustment: 0.0,
        }
    }
}

impl Default for RetargetOptions {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU motion frame
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuMotionFrame {
    /// Joint count
    pub joint_count: u32,
    /// Time
    pub time: f32,
    /// Root position
    pub root_position: [f32; 3],
    /// Padding
    pub _pad: f32,
    /// Root rotation
    pub root_rotation: [f32; 4],
}

/// GPU joint pose
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuJointPose {
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Position
    pub position: [f32; 3],
    /// Scale X
    pub scale_x: f32,
    /// Scale YZ
    pub scale_yz: [f32; 2],
    /// Joint index
    pub joint_index: u32,
    /// Padding
    pub _pad: f32,
}

/// GPU motion constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuMotionConstants {
    /// Time
    pub time: f32,
    /// Blend factor
    pub blend_factor: f32,
    /// Frame A
    pub frame_a: u32,
    /// Frame B
    pub frame_b: u32,
    /// Clip A duration
    pub clip_a_duration: f32,
    /// Clip B duration
    pub clip_b_duration: f32,
    /// Joint count
    pub joint_count: u32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU motion capture statistics
#[derive(Clone, Debug, Default)]
pub struct GpuMotionCaptureStats {
    /// Active clips
    pub active_clips: u32,
    /// Active instances
    pub active_instances: u32,
    /// Retarget operations
    pub retarget_ops: u32,
    /// IK solves
    pub ik_solves: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}
