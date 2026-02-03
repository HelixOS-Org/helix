//! Animation Types for Lumina
//!
//! This module provides animation infrastructure including
//! keyframes, curves, skeletal animation, and blend trees.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Animation Handle
// ============================================================================

/// Animation clip handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnimationHandle(pub u64);

impl AnimationHandle {
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

impl Default for AnimationHandle {
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

impl Default for SkeletonHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Animator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AnimatorHandle(pub u64);

impl AnimatorHandle {
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

impl Default for AnimatorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Animation Clip
// ============================================================================

/// Animation clip containing keyframe data
#[derive(Clone, Debug)]
pub struct AnimationClip {
    /// Handle
    pub handle: AnimationHandle,
    /// Name
    pub name: String,
    /// Duration in seconds
    pub duration: f32,
    /// Sample rate (frames per second)
    pub sample_rate: f32,
    /// Channels
    pub channels: Vec<AnimationChannel>,
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Events
    pub events: Vec<AnimationEvent>,
}

impl AnimationClip {
    /// Creates new animation clip
    pub fn new(handle: AnimationHandle, name: &str, duration: f32) -> Self {
        Self {
            handle,
            name: String::from(name),
            duration,
            sample_rate: 30.0,
            channels: Vec::new(),
            loop_mode: LoopMode::Once,
            events: Vec::new(),
        }
    }

    /// With sample rate
    pub fn with_sample_rate(mut self, rate: f32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// With loop mode
    pub fn with_loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Add channel
    pub fn add_channel(&mut self, channel: AnimationChannel) {
        self.channels.push(channel);
    }

    /// Add event
    pub fn add_event(&mut self, event: AnimationEvent) {
        self.events.push(event);
    }

    /// Sample at time
    pub fn sample(&self, time: f32) -> Vec<ChannelSample> {
        let wrapped_time = self.wrap_time(time);
        self.channels
            .iter()
            .map(|c| ChannelSample {
                target: c.target.clone(),
                value: c.sample(wrapped_time),
            })
            .collect()
    }

    /// Wrap time according to loop mode
    pub fn wrap_time(&self, time: f32) -> f32 {
        match self.loop_mode {
            LoopMode::Once => time.clamp(0.0, self.duration),
            LoopMode::Loop => {
                if self.duration > 0.0 {
                    time % self.duration
                } else {
                    0.0
                }
            }
            LoopMode::PingPong => {
                if self.duration > 0.0 {
                    let cycle = time / self.duration;
                    let t = time % self.duration;
                    if (cycle as i32) % 2 == 0 {
                        t
                    } else {
                        self.duration - t
                    }
                } else {
                    0.0
                }
            }
            LoopMode::ClampForever => {
                if time < 0.0 {
                    0.0
                } else if time > self.duration {
                    self.duration
                } else {
                    time
                }
            }
        }
    }

    /// Get events in time range
    pub fn events_in_range(&self, start: f32, end: f32) -> Vec<&AnimationEvent> {
        self.events
            .iter()
            .filter(|e| e.time >= start && e.time < end)
            .collect()
    }

    /// Frame count
    pub fn frame_count(&self) -> u32 {
        (self.duration * self.sample_rate).ceil() as u32
    }
}

/// Loop mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoopMode {
    /// Play once
    #[default]
    Once = 0,
    /// Loop continuously
    Loop = 1,
    /// Ping pong (reverse at end)
    PingPong = 2,
    /// Clamp at end
    ClampForever = 3,
}

// ============================================================================
// Animation Channel
// ============================================================================

/// Animation channel for a single property
#[derive(Clone, Debug)]
pub struct AnimationChannel {
    /// Target path
    pub target: AnimationTarget,
    /// Property type
    pub property: AnimationProperty,
    /// Keyframes
    pub keyframes: Vec<Keyframe>,
    /// Interpolation mode
    pub interpolation: InterpolationMode,
}

impl AnimationChannel {
    /// Creates new channel
    pub fn new(target: AnimationTarget, property: AnimationProperty) -> Self {
        Self {
            target,
            property,
            keyframes: Vec::new(),
            interpolation: InterpolationMode::Linear,
        }
    }

    /// Add keyframe
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Sample at time
    pub fn sample(&self, time: f32) -> AnimationValue {
        if self.keyframes.is_empty() {
            return AnimationValue::Float(0.0);
        }

        if self.keyframes.len() == 1 {
            return self.keyframes[0].value.clone();
        }

        // Find surrounding keyframes
        let (prev, next) = self.find_keyframes(time);

        if prev == next {
            return self.keyframes[prev].value.clone();
        }

        let kf_prev = &self.keyframes[prev];
        let kf_next = &self.keyframes[next];

        let t = if kf_next.time > kf_prev.time {
            (time - kf_prev.time) / (kf_next.time - kf_prev.time)
        } else {
            0.0
        };

        self.interpolate(&kf_prev.value, &kf_next.value, t, &kf_prev.out_tangent, &kf_next.in_tangent)
    }

    fn find_keyframes(&self, time: f32) -> (usize, usize) {
        if time <= self.keyframes[0].time {
            return (0, 0);
        }

        let last = self.keyframes.len() - 1;
        if time >= self.keyframes[last].time {
            return (last, last);
        }

        for i in 0..last {
            if time >= self.keyframes[i].time && time < self.keyframes[i + 1].time {
                return (i, i + 1);
            }
        }

        (last, last)
    }

    fn interpolate(
        &self,
        a: &AnimationValue,
        b: &AnimationValue,
        t: f32,
        _out_tan: &Option<AnimationValue>,
        _in_tan: &Option<AnimationValue>,
    ) -> AnimationValue {
        match self.interpolation {
            InterpolationMode::Step => a.clone(),
            InterpolationMode::Linear => a.lerp(b, t),
            InterpolationMode::Cubic => {
                // Hermite interpolation
                let t2 = t * t;
                let t3 = t2 * t;
                let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                let h10 = t3 - 2.0 * t2 + t;
                let h01 = -2.0 * t3 + 3.0 * t2;
                let h11 = t3 - t2;

                // Simplified cubic using hermite weights
                let a_weight = a.scale(h00 + h10 * 0.5);
                let b_weight = b.scale(h01 + h11 * 0.5);
                a_weight.add(&b_weight)
            }
        }
    }
}

/// Animation target
#[derive(Clone, Debug)]
pub struct AnimationTarget {
    /// Node path or bone name
    pub path: String,
    /// Bone index (if skeletal)
    pub bone_index: Option<u32>,
}

impl AnimationTarget {
    /// Creates node target
    pub fn node(path: &str) -> Self {
        Self {
            path: String::from(path),
            bone_index: None,
        }
    }

    /// Creates bone target
    pub fn bone(name: &str, index: u32) -> Self {
        Self {
            path: String::from(name),
            bone_index: Some(index),
        }
    }
}

/// Animation property
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AnimationProperty {
    /// Position
    Position = 0,
    /// Rotation (quaternion)
    Rotation = 1,
    /// Scale
    Scale = 2,
    /// Blend shape weight
    BlendShape = 3,
    /// Material property
    Material = 4,
    /// Custom float
    CustomFloat = 5,
    /// Custom vector
    CustomVector = 6,
}

/// Interpolation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InterpolationMode {
    /// Step (no interpolation)
    Step = 0,
    /// Linear interpolation
    #[default]
    Linear = 1,
    /// Cubic (hermite) interpolation
    Cubic = 2,
}

// ============================================================================
// Keyframe
// ============================================================================

/// Animation keyframe
#[derive(Clone, Debug)]
pub struct Keyframe {
    /// Time in seconds
    pub time: f32,
    /// Value
    pub value: AnimationValue,
    /// In tangent (for cubic)
    pub in_tangent: Option<AnimationValue>,
    /// Out tangent (for cubic)
    pub out_tangent: Option<AnimationValue>,
}

impl Keyframe {
    /// Creates new keyframe
    pub fn new(time: f32, value: AnimationValue) -> Self {
        Self {
            time,
            value,
            in_tangent: None,
            out_tangent: None,
        }
    }

    /// With tangents
    pub fn with_tangents(mut self, in_tan: AnimationValue, out_tan: AnimationValue) -> Self {
        self.in_tangent = Some(in_tan);
        self.out_tangent = Some(out_tan);
        self
    }

    /// Float keyframe
    pub fn float(time: f32, value: f32) -> Self {
        Self::new(time, AnimationValue::Float(value))
    }

    /// Vector3 keyframe
    pub fn vec3(time: f32, x: f32, y: f32, z: f32) -> Self {
        Self::new(time, AnimationValue::Vector3([x, y, z]))
    }

    /// Quaternion keyframe
    pub fn quat(time: f32, x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::new(time, AnimationValue::Quaternion([x, y, z, w]))
    }
}

/// Animation value
#[derive(Clone, Debug)]
pub enum AnimationValue {
    /// Float
    Float(f32),
    /// Vector2
    Vector2([f32; 2]),
    /// Vector3
    Vector3([f32; 3]),
    /// Vector4
    Vector4([f32; 4]),
    /// Quaternion
    Quaternion([f32; 4]),
}

impl AnimationValue {
    /// Linear interpolation
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        match (self, other) {
            (Self::Float(a), Self::Float(b)) => Self::Float(a + (b - a) * t),
            (Self::Vector2(a), Self::Vector2(b)) => Self::Vector2([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
            ]),
            (Self::Vector3(a), Self::Vector3(b)) => Self::Vector3([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]),
            (Self::Vector4(a), Self::Vector4(b)) => Self::Vector4([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
                a[3] + (b[3] - a[3]) * t,
            ]),
            (Self::Quaternion(a), Self::Quaternion(b)) => Self::Quaternion(slerp(*a, *b, t)),
            _ => self.clone(),
        }
    }

    /// Scale value
    pub fn scale(&self, s: f32) -> Self {
        match self {
            Self::Float(v) => Self::Float(v * s),
            Self::Vector2(v) => Self::Vector2([v[0] * s, v[1] * s]),
            Self::Vector3(v) => Self::Vector3([v[0] * s, v[1] * s, v[2] * s]),
            Self::Vector4(v) => Self::Vector4([v[0] * s, v[1] * s, v[2] * s, v[3] * s]),
            Self::Quaternion(v) => Self::Quaternion([v[0] * s, v[1] * s, v[2] * s, v[3] * s]),
        }
    }

    /// Add values
    pub fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Float(a), Self::Float(b)) => Self::Float(a + b),
            (Self::Vector2(a), Self::Vector2(b)) => Self::Vector2([a[0] + b[0], a[1] + b[1]]),
            (Self::Vector3(a), Self::Vector3(b)) => {
                Self::Vector3([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
            }
            (Self::Vector4(a), Self::Vector4(b)) => {
                Self::Vector4([a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]])
            }
            (Self::Quaternion(a), Self::Quaternion(b)) => {
                Self::Quaternion([a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]])
            }
            _ => self.clone(),
        }
    }

    /// As float
    pub fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// As vector3
    pub fn as_vec3(&self) -> Option<[f32; 3]> {
        match self {
            Self::Vector3(v) => Some(*v),
            _ => None,
        }
    }

    /// As quaternion
    pub fn as_quat(&self) -> Option<[f32; 4]> {
        match self {
            Self::Quaternion(v) => Some(*v),
            _ => None,
        }
    }
}

/// Spherical lerp for quaternions
fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

    let mut b = b;
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }

    if dot > 0.9995 {
        let result = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ];
        let len =
            (result[0] * result[0] + result[1] * result[1] + result[2] * result[2] + result[3] * result[3]).sqrt();
        [result[0] / len, result[1] / len, result[2] / len, result[3] / len]
    } else {
        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        [
            a[0] * s0 + b[0] * s1,
            a[1] * s0 + b[1] * s1,
            a[2] * s0 + b[2] * s1,
            a[3] * s0 + b[3] * s1,
        ]
    }
}

/// Channel sample result
#[derive(Clone, Debug)]
pub struct ChannelSample {
    /// Target
    pub target: AnimationTarget,
    /// Value
    pub value: AnimationValue,
}

// ============================================================================
// Animation Event
// ============================================================================

/// Animation event at specific time
#[derive(Clone, Debug)]
pub struct AnimationEvent {
    /// Time in seconds
    pub time: f32,
    /// Event name
    pub name: String,
    /// Event data
    pub data: EventData,
}

impl AnimationEvent {
    /// Creates new event
    pub fn new(time: f32, name: &str) -> Self {
        Self {
            time,
            name: String::from(name),
            data: EventData::None,
        }
    }

    /// With int data
    pub fn with_int(mut self, value: i32) -> Self {
        self.data = EventData::Int(value);
        self
    }

    /// With float data
    pub fn with_float(mut self, value: f32) -> Self {
        self.data = EventData::Float(value);
        self
    }

    /// With string data
    pub fn with_string(mut self, value: &str) -> Self {
        self.data = EventData::String(String::from(value));
        self
    }
}

/// Event data
#[derive(Clone, Debug)]
pub enum EventData {
    /// No data
    None,
    /// Integer
    Int(i32),
    /// Float
    Float(f32),
    /// String
    String(String),
}

// ============================================================================
// Skeleton
// ============================================================================

/// Skeleton for skeletal animation
#[derive(Clone, Debug)]
pub struct Skeleton {
    /// Handle
    pub handle: SkeletonHandle,
    /// Name
    pub name: String,
    /// Bones
    pub bones: Vec<Bone>,
    /// Inverse bind matrices
    pub inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
}

impl Skeleton {
    /// Creates new skeleton
    pub fn new(handle: SkeletonHandle, name: &str) -> Self {
        Self {
            handle,
            name: String::from(name),
            bones: Vec::new(),
            inverse_bind_matrices: Vec::new(),
        }
    }

    /// Add bone
    pub fn add_bone(&mut self, bone: Bone, inverse_bind: [[f32; 4]; 4]) {
        self.bones.push(bone);
        self.inverse_bind_matrices.push(inverse_bind);
    }

    /// Find bone by name
    pub fn find_bone(&self, name: &str) -> Option<u32> {
        self.bones.iter().position(|b| b.name == name).map(|i| i as u32)
    }

    /// Bone count
    pub fn bone_count(&self) -> u32 {
        self.bones.len() as u32
    }

    /// Get parent chain
    pub fn parent_chain(&self, bone_index: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut current = bone_index;

        while let Some(parent) = self.bones.get(current as usize).and_then(|b| b.parent) {
            chain.push(parent);
            current = parent;
        }

        chain
    }
}

/// Bone in skeleton
#[derive(Clone, Debug)]
pub struct Bone {
    /// Bone name
    pub name: String,
    /// Parent bone index
    pub parent: Option<u32>,
    /// Children bone indices
    pub children: Vec<u32>,
    /// Local bind pose
    pub local_bind_pose: BonePose,
}

impl Bone {
    /// Creates new bone
    pub fn new(name: &str, parent: Option<u32>) -> Self {
        Self {
            name: String::from(name),
            parent,
            children: Vec::new(),
            local_bind_pose: BonePose::IDENTITY,
        }
    }

    /// With bind pose
    pub fn with_bind_pose(mut self, pose: BonePose) -> Self {
        self.local_bind_pose = pose;
        self
    }

    /// Add child
    pub fn add_child(&mut self, child: u32) {
        self.children.push(child);
    }
}

/// Bone pose
#[derive(Clone, Copy, Debug)]
pub struct BonePose {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl BonePose {
    /// Identity pose
    pub const IDENTITY: Self = Self {
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    };

    /// Creates new pose
    pub fn new(position: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    /// To matrix
    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        let [qx, qy, qz, qw] = self.rotation;
        let [sx, sy, sz] = self.scale;
        let [px, py, pz] = self.position;

        let xx = qx * qx;
        let yy = qy * qy;
        let zz = qz * qz;
        let xy = qx * qy;
        let xz = qx * qz;
        let yz = qy * qz;
        let wx = qw * qx;
        let wy = qw * qy;
        let wz = qw * qz;

        [
            [sx * (1.0 - 2.0 * (yy + zz)), sx * 2.0 * (xy + wz), sx * 2.0 * (xz - wy), 0.0],
            [sy * 2.0 * (xy - wz), sy * (1.0 - 2.0 * (xx + zz)), sy * 2.0 * (yz + wx), 0.0],
            [sz * 2.0 * (xz + wy), sz * 2.0 * (yz - wx), sz * (1.0 - 2.0 * (xx + yy)), 0.0],
            [px, py, pz, 1.0],
        ]
    }

    /// Interpolate poses
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: [
                self.position[0] + (other.position[0] - self.position[0]) * t,
                self.position[1] + (other.position[1] - self.position[1]) * t,
                self.position[2] + (other.position[2] - self.position[2]) * t,
            ],
            rotation: slerp(self.rotation, other.rotation, t),
            scale: [
                self.scale[0] + (other.scale[0] - self.scale[0]) * t,
                self.scale[1] + (other.scale[1] - self.scale[1]) * t,
                self.scale[2] + (other.scale[2] - self.scale[2]) * t,
            ],
        }
    }
}

impl Default for BonePose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// ============================================================================
// Animation State
// ============================================================================

/// Animation playback state
#[derive(Clone, Debug)]
pub struct AnimationState {
    /// Animation handle
    pub animation: AnimationHandle,
    /// Current time
    pub time: f32,
    /// Playback speed
    pub speed: f32,
    /// Weight for blending
    pub weight: f32,
    /// Is playing
    pub playing: bool,
    /// Is paused
    pub paused: bool,
}

impl AnimationState {
    /// Creates new state
    pub fn new(animation: AnimationHandle) -> Self {
        Self {
            animation,
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            playing: false,
            paused: false,
        }
    }

    /// Play
    pub fn play(&mut self) {
        self.playing = true;
        self.paused = false;
    }

    /// Pause
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Stop
    pub fn stop(&mut self) {
        self.playing = false;
        self.paused = false;
        self.time = 0.0;
    }

    /// Update
    pub fn update(&mut self, delta_time: f32) {
        if self.playing && !self.paused {
            self.time += delta_time * self.speed;
        }
    }

    /// Set normalized time (0-1)
    pub fn set_normalized_time(&mut self, t: f32, duration: f32) {
        self.time = t * duration;
    }

    /// Get normalized time (0-1)
    pub fn normalized_time(&self, duration: f32) -> f32 {
        if duration > 0.0 {
            self.time / duration
        } else {
            0.0
        }
    }
}

// ============================================================================
// Blend Tree
// ============================================================================

/// Blend tree for animation blending
#[derive(Clone, Debug)]
pub enum BlendNode {
    /// Single animation
    Clip(AnimationHandle),
    /// Linear blend of two animations
    Blend1D {
        animations: Vec<AnimationHandle>,
        thresholds: Vec<f32>,
        parameter: f32,
    },
    /// 2D blend of animations
    Blend2D {
        animations: Vec<AnimationHandle>,
        positions: Vec<[f32; 2]>,
        parameter: [f32; 2],
    },
    /// Additive blend
    Additive {
        base: Box<BlendNode>,
        additive: Box<BlendNode>,
        weight: f32,
    },
    /// Override layers
    Override {
        base: Box<BlendNode>,
        overlay: Box<BlendNode>,
        weight: f32,
    },
}

impl BlendNode {
    /// Creates clip node
    pub fn clip(animation: AnimationHandle) -> Self {
        Self::Clip(animation)
    }

    /// Creates 1D blend
    pub fn blend_1d(animations: Vec<AnimationHandle>, thresholds: Vec<f32>) -> Self {
        Self::Blend1D {
            animations,
            thresholds,
            parameter: 0.0,
        }
    }

    /// Creates 2D blend
    pub fn blend_2d(animations: Vec<AnimationHandle>, positions: Vec<[f32; 2]>) -> Self {
        Self::Blend2D {
            animations,
            positions,
            parameter: [0.0, 0.0],
        }
    }

    /// Creates additive blend
    pub fn additive(base: BlendNode, additive: BlendNode, weight: f32) -> Self {
        Self::Additive {
            base: Box::new(base),
            additive: Box::new(additive),
            weight,
        }
    }

    /// Creates override blend
    pub fn override_blend(base: BlendNode, overlay: BlendNode, weight: f32) -> Self {
        Self::Override {
            base: Box::new(base),
            overlay: Box::new(overlay),
            weight,
        }
    }

    /// Get animations with weights
    pub fn get_weighted_animations(&self) -> Vec<(AnimationHandle, f32)> {
        match self {
            Self::Clip(handle) => alloc::vec![(*handle, 1.0)],
            Self::Blend1D {
                animations,
                thresholds,
                parameter,
            } => {
                if animations.is_empty() {
                    return Vec::new();
                }

                // Find surrounding animations
                let mut result = Vec::new();
                for i in 0..thresholds.len() - 1 {
                    if *parameter >= thresholds[i] && *parameter <= thresholds[i + 1] {
                        let t = (parameter - thresholds[i]) / (thresholds[i + 1] - thresholds[i]);
                        result.push((animations[i], 1.0 - t));
                        result.push((animations[i + 1], t));
                        return result;
                    }
                }

                // At boundary
                if *parameter <= thresholds[0] {
                    alloc::vec![(animations[0], 1.0)]
                } else {
                    alloc::vec![(animations[animations.len() - 1], 1.0)]
                }
            }
            Self::Blend2D { animations, positions, parameter } => {
                // Simple barycentric blend for 2D
                if animations.len() < 3 {
                    return animations.iter().map(|a| (*a, 1.0 / animations.len() as f32)).collect();
                }

                // Find closest 3 animations and blend
                let mut distances: Vec<(usize, f32)> = positions
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let dx = p[0] - parameter[0];
                        let dy = p[1] - parameter[1];
                        (i, dx * dx + dy * dy)
                    })
                    .collect();

                distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                let sum: f32 = distances.iter().take(3).map(|(_, d)| 1.0 / (d + 0.0001)).sum();
                distances
                    .iter()
                    .take(3)
                    .map(|(i, d)| (animations[*i], (1.0 / (d + 0.0001)) / sum))
                    .collect()
            }
            Self::Additive { base, additive, weight } => {
                let mut result = base.get_weighted_animations();
                for (anim, w) in additive.get_weighted_animations() {
                    result.push((anim, w * weight));
                }
                result
            }
            Self::Override { base, overlay, weight } => {
                let mut result: Vec<(AnimationHandle, f32)> = base
                    .get_weighted_animations()
                    .into_iter()
                    .map(|(a, w)| (a, w * (1.0 - weight)))
                    .collect();

                for (anim, w) in overlay.get_weighted_animations() {
                    result.push((anim, w * weight));
                }
                result
            }
        }
    }
}

// ============================================================================
// Skin Data (GPU)
// ============================================================================

/// GPU skin uniform data
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SkinUniformData {
    /// Bone matrices (max 256 bones)
    pub bone_matrices: [[[f32; 4]; 4]; 256],
    /// Bone count
    pub bone_count: u32,
    /// Padding
    pub _padding: [u32; 3],
}

impl SkinUniformData {
    /// Creates new skin data
    pub fn new() -> Self {
        Self {
            bone_matrices: [[[0.0; 4]; 4]; 256],
            bone_count: 0,
            _padding: [0; 3],
        }
    }

    /// Set bone matrix
    pub fn set_bone_matrix(&mut self, index: u32, matrix: [[f32; 4]; 4]) {
        if (index as usize) < 256 {
            self.bone_matrices[index as usize] = matrix;
        }
    }
}

impl Default for SkinUniformData {
    fn default() -> Self {
        Self::new()
    }
}
