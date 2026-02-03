//! GPU Crowd Simulation System for Lumina
//!
//! This module provides GPU-accelerated crowd simulation with
//! pathfinding, flocking, and agent-based behaviors.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Crowd System Handles
// ============================================================================

/// GPU crowd system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuCrowdSystemHandle(pub u64);

impl GpuCrowdSystemHandle {
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

impl Default for GpuCrowdSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Crowd agent handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CrowdAgentHandle(pub u64);

impl CrowdAgentHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CrowdAgentHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Agent group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AgentGroupHandle(pub u64);

impl AgentGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for AgentGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Navigation mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NavMeshHandle(pub u64);

impl NavMeshHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for NavMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Crowd System Creation
// ============================================================================

/// GPU crowd system create info
#[derive(Clone, Debug)]
pub struct GpuCrowdSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max agents
    pub max_agents: u32,
    /// Max groups
    pub max_groups: u32,
    /// Max obstacles
    pub max_obstacles: u32,
    /// Features
    pub features: CrowdFeatures,
    /// Quality
    pub quality: CrowdQuality,
}

impl GpuCrowdSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_agents: 10000,
            max_groups: 64,
            max_obstacles: 256,
            features: CrowdFeatures::all(),
            quality: CrowdQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max agents
    pub fn with_max_agents(mut self, count: u32) -> Self {
        self.max_agents = count;
        self
    }

    /// With max groups
    pub fn with_max_groups(mut self, count: u32) -> Self {
        self.max_groups = count;
        self
    }

    /// With max obstacles
    pub fn with_max_obstacles(mut self, count: u32) -> Self {
        self.max_obstacles = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: CrowdFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: CrowdQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// Massive crowds
    pub fn massive() -> Self {
        Self::new()
            .with_max_agents(100000)
            .with_max_groups(256)
            .with_quality(CrowdQuality::Ultra)
    }

    /// Mobile
    pub fn mobile() -> Self {
        Self::new()
            .with_max_agents(500)
            .with_max_groups(16)
            .with_quality(CrowdQuality::Low)
    }
}

impl Default for GpuCrowdSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Crowd features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct CrowdFeatures: u32 {
        /// None
        const NONE = 0;
        /// Local avoidance
        const LOCAL_AVOIDANCE = 1 << 0;
        /// Pathfinding
        const PATHFINDING = 1 << 1;
        /// Flocking
        const FLOCKING = 1 << 2;
        /// Formation
        const FORMATION = 1 << 3;
        /// Obstacle avoidance
        const OBSTACLE_AVOIDANCE = 1 << 4;
        /// GPU compute
        const GPU_COMPUTE = 1 << 5;
        /// LOD agents
        const LOD = 1 << 6;
        /// Animation blending
        const ANIMATION = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for CrowdFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Crowd quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CrowdQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    Medium = 1,
    /// High quality
    #[default]
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

// ============================================================================
// Agent
// ============================================================================

/// Crowd agent create info
#[derive(Clone, Debug)]
pub struct CrowdAgentCreateInfo {
    /// Name
    pub name: String,
    /// Agent type
    pub agent_type: AgentType,
    /// Initial position
    pub position: [f32; 3],
    /// Target position
    pub target: [f32; 3],
    /// Group
    pub group: AgentGroupHandle,
    /// Properties
    pub properties: AgentProperties,
    /// Behavior
    pub behavior: AgentBehavior,
}

impl CrowdAgentCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            agent_type: AgentType::Pedestrian,
            position: [0.0, 0.0, 0.0],
            target: [0.0, 0.0, 0.0],
            group: AgentGroupHandle::NULL,
            properties: AgentProperties::default(),
            behavior: AgentBehavior::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With type
    pub fn with_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = agent_type;
        self
    }

    /// With position
    pub fn at_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// With target
    pub fn with_target(mut self, target: [f32; 3]) -> Self {
        self.target = target;
        self
    }

    /// With group
    pub fn in_group(mut self, group: AgentGroupHandle) -> Self {
        self.group = group;
        self
    }

    /// With properties
    pub fn with_properties(mut self, properties: AgentProperties) -> Self {
        self.properties = properties;
        self
    }

    /// With behavior
    pub fn with_behavior(mut self, behavior: AgentBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    /// Pedestrian preset
    pub fn pedestrian() -> Self {
        Self::new()
            .with_type(AgentType::Pedestrian)
            .with_properties(AgentProperties::pedestrian())
            .with_behavior(AgentBehavior::wander())
    }

    /// Guard preset
    pub fn guard() -> Self {
        Self::new()
            .with_type(AgentType::Guard)
            .with_properties(AgentProperties::guard())
            .with_behavior(AgentBehavior::patrol())
    }

    /// Soldier preset
    pub fn soldier() -> Self {
        Self::new()
            .with_type(AgentType::Soldier)
            .with_properties(AgentProperties::soldier())
            .with_behavior(AgentBehavior::formation())
    }
}

impl Default for CrowdAgentCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AgentType {
    /// Pedestrian
    #[default]
    Pedestrian = 0,
    /// Guard
    Guard = 1,
    /// Soldier
    Soldier = 2,
    /// Vehicle
    Vehicle = 3,
    /// Animal
    Animal = 4,
    /// Custom
    Custom = 5,
}

// ============================================================================
// Agent Properties
// ============================================================================

/// Agent properties
#[derive(Clone, Copy, Debug)]
pub struct AgentProperties {
    /// Radius
    pub radius: f32,
    /// Height
    pub height: f32,
    /// Max speed
    pub max_speed: f32,
    /// Max acceleration
    pub max_acceleration: f32,
    /// Turn rate (radians/sec)
    pub turn_rate: f32,
    /// Separation distance
    pub separation_distance: f32,
    /// View angle (radians)
    pub view_angle: f32,
    /// View distance
    pub view_distance: f32,
}

impl AgentProperties {
    /// Default properties
    pub const fn new() -> Self {
        Self {
            radius: 0.4,
            height: 1.8,
            max_speed: 1.4,
            max_acceleration: 3.0,
            turn_rate: 3.14,
            separation_distance: 1.0,
            view_angle: 2.5,
            view_distance: 10.0,
        }
    }

    /// Pedestrian
    pub const fn pedestrian() -> Self {
        Self {
            radius: 0.4,
            height: 1.8,
            max_speed: 1.4,
            max_acceleration: 2.5,
            turn_rate: 3.14,
            separation_distance: 0.8,
            view_angle: 2.5,
            view_distance: 8.0,
        }
    }

    /// Guard
    pub const fn guard() -> Self {
        Self {
            radius: 0.5,
            height: 1.9,
            max_speed: 2.0,
            max_acceleration: 4.0,
            turn_rate: 4.0,
            separation_distance: 1.5,
            view_angle: 3.0,
            view_distance: 20.0,
        }
    }

    /// Soldier
    pub const fn soldier() -> Self {
        Self {
            radius: 0.5,
            height: 1.8,
            max_speed: 3.0,
            max_acceleration: 6.0,
            turn_rate: 5.0,
            separation_distance: 2.0,
            view_angle: 2.5,
            view_distance: 30.0,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With max speed
    pub const fn with_max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }
}

impl Default for AgentProperties {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Agent Behavior
// ============================================================================

/// Agent behavior
#[derive(Clone, Copy, Debug)]
pub struct AgentBehavior {
    /// Behavior type
    pub behavior_type: BehaviorType,
    /// Separation weight
    pub separation_weight: f32,
    /// Alignment weight
    pub alignment_weight: f32,
    /// Cohesion weight
    pub cohesion_weight: f32,
    /// Goal seeking weight
    pub goal_weight: f32,
    /// Obstacle avoidance weight
    pub obstacle_weight: f32,
    /// Wander strength
    pub wander_strength: f32,
}

impl AgentBehavior {
    /// Default behavior
    pub const fn new() -> Self {
        Self {
            behavior_type: BehaviorType::GoToTarget,
            separation_weight: 1.0,
            alignment_weight: 0.5,
            cohesion_weight: 0.3,
            goal_weight: 1.0,
            obstacle_weight: 2.0,
            wander_strength: 0.0,
        }
    }

    /// Go to target
    pub const fn go_to_target() -> Self {
        Self::new()
    }

    /// Wander
    pub const fn wander() -> Self {
        Self {
            behavior_type: BehaviorType::Wander,
            separation_weight: 1.0,
            alignment_weight: 0.2,
            cohesion_weight: 0.1,
            goal_weight: 0.0,
            obstacle_weight: 2.0,
            wander_strength: 1.0,
        }
    }

    /// Patrol
    pub const fn patrol() -> Self {
        Self {
            behavior_type: BehaviorType::Patrol,
            separation_weight: 1.5,
            alignment_weight: 0.0,
            cohesion_weight: 0.0,
            goal_weight: 1.0,
            obstacle_weight: 2.0,
            wander_strength: 0.0,
        }
    }

    /// Formation
    pub const fn formation() -> Self {
        Self {
            behavior_type: BehaviorType::Formation,
            separation_weight: 0.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            goal_weight: 0.5,
            obstacle_weight: 2.0,
            wander_strength: 0.0,
        }
    }

    /// Flee
    pub const fn flee() -> Self {
        Self {
            behavior_type: BehaviorType::Flee,
            separation_weight: 2.0,
            alignment_weight: 0.8,
            cohesion_weight: 0.5,
            goal_weight: 2.0,
            obstacle_weight: 1.5,
            wander_strength: 0.3,
        }
    }

    /// Follow leader
    pub const fn follow_leader() -> Self {
        Self {
            behavior_type: BehaviorType::FollowLeader,
            separation_weight: 1.0,
            alignment_weight: 1.0,
            cohesion_weight: 0.8,
            goal_weight: 1.5,
            obstacle_weight: 2.0,
            wander_strength: 0.0,
        }
    }

    /// With type
    pub const fn with_type(mut self, behavior_type: BehaviorType) -> Self {
        self.behavior_type = behavior_type;
        self
    }
}

impl Default for AgentBehavior {
    fn default() -> Self {
        Self::new()
    }
}

/// Behavior type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BehaviorType {
    /// Go to target
    #[default]
    GoToTarget = 0,
    /// Wander
    Wander = 1,
    /// Patrol
    Patrol = 2,
    /// Formation
    Formation = 3,
    /// Flee
    Flee = 4,
    /// Follow leader
    FollowLeader = 5,
    /// Idle
    Idle = 6,
}

// ============================================================================
// Agent Group
// ============================================================================

/// Agent group create info
#[derive(Clone, Debug)]
pub struct AgentGroupCreateInfo {
    /// Name
    pub name: String,
    /// Formation type
    pub formation: FormationType,
    /// Formation spacing
    pub spacing: f32,
    /// Group target
    pub target: [f32; 3],
    /// Leader agent
    pub leader: CrowdAgentHandle,
    /// Group color (for debug)
    pub color: [f32; 4],
}

impl AgentGroupCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            formation: FormationType::None,
            spacing: 2.0,
            target: [0.0, 0.0, 0.0],
            leader: CrowdAgentHandle::NULL,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With formation
    pub fn with_formation(mut self, formation: FormationType) -> Self {
        self.formation = formation;
        self
    }

    /// With spacing
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// With target
    pub fn with_target(mut self, target: [f32; 3]) -> Self {
        self.target = target;
        self
    }

    /// With leader
    pub fn with_leader(mut self, leader: CrowdAgentHandle) -> Self {
        self.leader = leader;
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
}

impl Default for AgentGroupCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Formation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FormationType {
    /// No formation
    #[default]
    None = 0,
    /// Line
    Line = 1,
    /// Column
    Column = 2,
    /// Wedge
    Wedge = 3,
    /// Circle
    Circle = 4,
    /// Square
    Square = 5,
    /// Custom
    Custom = 6,
}

// ============================================================================
// Obstacles
// ============================================================================

/// Crowd obstacle create info
#[derive(Clone, Debug)]
pub struct CrowdObstacleCreateInfo {
    /// Name
    pub name: String,
    /// Shape
    pub shape: ObstacleShape,
    /// Position
    pub position: [f32; 3],
    /// Is dynamic
    pub dynamic: bool,
    /// Velocity (for dynamic obstacles)
    pub velocity: [f32; 3],
}

impl CrowdObstacleCreateInfo {
    /// Creates new info
    pub fn new(shape: ObstacleShape) -> Self {
        Self {
            name: String::new(),
            shape,
            position: [0.0, 0.0, 0.0],
            dynamic: false,
            velocity: [0.0, 0.0, 0.0],
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// At position
    pub fn at_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// Dynamic obstacle
    pub fn dynamic(mut self) -> Self {
        self.dynamic = true;
        self
    }

    /// With velocity
    pub fn with_velocity(mut self, velocity: [f32; 3]) -> Self {
        self.velocity = velocity;
        self
    }

    /// Circle obstacle
    pub fn circle(radius: f32) -> Self {
        Self::new(ObstacleShape::Circle { radius })
    }

    /// Rectangle obstacle
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self::new(ObstacleShape::Rectangle { width, height })
    }

    /// Polygon obstacle
    pub fn polygon(vertices: Vec<[f32; 2]>) -> Self {
        Self::new(ObstacleShape::Polygon { vertices })
    }
}

impl Default for CrowdObstacleCreateInfo {
    fn default() -> Self {
        Self::circle(1.0)
    }
}

/// Obstacle shape
#[derive(Clone, Debug)]
pub enum ObstacleShape {
    /// Circle
    Circle { radius: f32 },
    /// Rectangle
    Rectangle { width: f32, height: f32 },
    /// Polygon
    Polygon { vertices: Vec<[f32; 2]> },
}

impl Default for ObstacleShape {
    fn default() -> Self {
        Self::Circle { radius: 1.0 }
    }
}

// ============================================================================
// Navigation Mesh
// ============================================================================

/// Navigation mesh create info
#[derive(Clone, Debug)]
pub struct NavMeshCreateInfo {
    /// Name
    pub name: String,
    /// Vertices
    pub vertices: Vec<[f32; 3]>,
    /// Polygons (indices)
    pub polygons: Vec<Vec<u32>>,
    /// Agent radius
    pub agent_radius: f32,
    /// Agent height
    pub agent_height: f32,
    /// Cell size
    pub cell_size: f32,
    /// Cell height
    pub cell_height: f32,
}

impl NavMeshCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            vertices: Vec::new(),
            polygons: Vec::new(),
            agent_radius: 0.4,
            agent_height: 1.8,
            cell_size: 0.3,
            cell_height: 0.2,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With vertices
    pub fn with_vertices(mut self, vertices: Vec<[f32; 3]>) -> Self {
        self.vertices = vertices;
        self
    }

    /// With polygons
    pub fn with_polygons(mut self, polygons: Vec<Vec<u32>>) -> Self {
        self.polygons = polygons;
        self
    }

    /// With agent radius
    pub fn with_agent_radius(mut self, radius: f32) -> Self {
        self.agent_radius = radius;
        self
    }

    /// With agent height
    pub fn with_agent_height(mut self, height: f32) -> Self {
        self.agent_height = height;
        self
    }
}

impl Default for NavMeshCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU crowd agent
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCrowdAgent {
    /// Position
    pub position: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Max speed
    pub max_speed: f32,
    /// Target
    pub target: [f32; 3],
    /// State
    pub state: u32,
    /// Orientation
    pub orientation: f32,
    /// Group ID
    pub group_id: u32,
    /// Agent type
    pub agent_type: u32,
    /// Flags
    pub flags: u32,
}

/// GPU crowd constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCrowdConstants {
    /// Time
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Agent count
    pub agent_count: u32,
    /// Obstacle count
    pub obstacle_count: u32,
    /// Separation weight
    pub separation_weight: f32,
    /// Alignment weight
    pub alignment_weight: f32,
    /// Cohesion weight
    pub cohesion_weight: f32,
    /// Obstacle weight
    pub obstacle_weight: f32,
}

/// GPU crowd obstacle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCrowdObstacle {
    /// Position
    pub position: [f32; 2],
    /// Velocity
    pub velocity: [f32; 2],
    /// Shape type
    pub shape_type: u32,
    /// Size X
    pub size_x: f32,
    /// Size Y
    pub size_y: f32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Pathfinding
// ============================================================================

/// Path request
#[derive(Clone, Debug)]
pub struct PathRequest {
    /// Start position
    pub start: [f32; 3],
    /// End position
    pub end: [f32; 3],
    /// Agent radius
    pub agent_radius: f32,
    /// Allow partial paths
    pub allow_partial: bool,
}

impl PathRequest {
    /// Creates new request
    pub fn new(start: [f32; 3], end: [f32; 3]) -> Self {
        Self {
            start,
            end,
            agent_radius: 0.4,
            allow_partial: true,
        }
    }
}

/// Path result
#[derive(Clone, Debug, Default)]
pub struct PathResult {
    /// Path waypoints
    pub waypoints: Vec<[f32; 3]>,
    /// Path length
    pub length: f32,
    /// Is complete
    pub complete: bool,
    /// Is valid
    pub valid: bool,
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU crowd statistics
#[derive(Clone, Debug, Default)]
pub struct GpuCrowdStats {
    /// Active agents
    pub active_agents: u32,
    /// Moving agents
    pub moving_agents: u32,
    /// Idle agents
    pub idle_agents: u32,
    /// Collision pairs checked
    pub collision_pairs: u32,
    /// Path requests
    pub path_requests: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}
