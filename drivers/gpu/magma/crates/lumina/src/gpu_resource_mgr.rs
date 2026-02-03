//! GPU Resource Manager System for Lumina
//!
//! This module provides comprehensive GPU resource management including
//! resource tracking, lifecycle management, and garbage collection.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Resource Manager Handles
// ============================================================================

/// GPU resource manager handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuResourceManagerHandle(pub u64);

impl GpuResourceManagerHandle {
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

impl Default for GpuResourceManagerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Resource handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceHandle(pub u64);

impl ResourceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Generation (for validation)
    pub const fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Index
    pub const fn index(&self) -> u32 {
        self.0 as u32
    }
}

impl Default for ResourceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Resource pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourcePoolHandle(pub u64);

impl ResourcePoolHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ResourcePoolHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Resource group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceGroupHandle(pub u64);

impl ResourceGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ResourceGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Resource Manager Creation
// ============================================================================

/// GPU resource manager create info
#[derive(Clone, Debug)]
pub struct GpuResourceManagerCreateInfo {
    /// Name
    pub name: String,
    /// Max resources
    pub max_resources: u32,
    /// Max pools
    pub max_pools: u32,
    /// Max groups
    pub max_groups: u32,
    /// Features
    pub features: ResourceManagerFeatures,
    /// GC settings
    pub gc_settings: GcSettings,
}

impl GpuResourceManagerCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_resources: 100000,
            max_pools: 256,
            max_groups: 1024,
            features: ResourceManagerFeatures::all(),
            gc_settings: GcSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max resources
    pub fn with_max_resources(mut self, count: u32) -> Self {
        self.max_resources = count;
        self
    }

    /// With max pools
    pub fn with_max_pools(mut self, count: u32) -> Self {
        self.max_pools = count;
        self
    }

    /// With max groups
    pub fn with_max_groups(mut self, count: u32) -> Self {
        self.max_groups = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ResourceManagerFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With GC settings
    pub fn with_gc(mut self, gc: GcSettings) -> Self {
        self.gc_settings = gc;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_resources(1000000)
            .with_max_pools(1024)
            .with_max_groups(4096)
    }
}

impl Default for GpuResourceManagerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Resource manager features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ResourceManagerFeatures: u32 {
        /// None
        const NONE = 0;
        /// Reference counting
        const REF_COUNTING = 1 << 0;
        /// Garbage collection
        const GC = 1 << 1;
        /// Resource pooling
        const POOLING = 1 << 2;
        /// Deferred destruction
        const DEFERRED_DESTROY = 1 << 3;
        /// Memory tracking
        const MEMORY_TRACKING = 1 << 4;
        /// Leak detection
        const LEAK_DETECTION = 1 << 5;
        /// Resource validation
        const VALIDATION = 1 << 6;
        /// Async loading
        const ASYNC_LOADING = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for ResourceManagerFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Resource Types
// ============================================================================

/// Resource type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceType {
    /// Unknown
    #[default]
    Unknown = 0,
    /// Buffer
    Buffer = 1,
    /// Texture
    Texture = 2,
    /// Sampler
    Sampler = 3,
    /// Shader
    Shader = 4,
    /// Pipeline
    Pipeline = 5,
    /// Render pass
    RenderPass = 6,
    /// Framebuffer
    Framebuffer = 7,
    /// Descriptor set
    DescriptorSet = 8,
    /// Query pool
    QueryPool = 9,
    /// Fence
    Fence = 10,
    /// Semaphore
    Semaphore = 11,
    /// Command buffer
    CommandBuffer = 12,
    /// Acceleration structure
    AccelerationStructure = 13,
}

/// Resource state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceState {
    /// Unknown
    #[default]
    Unknown = 0,
    /// Creating
    Creating = 1,
    /// Ready
    Ready = 2,
    /// In use
    InUse = 3,
    /// Pending destroy
    PendingDestroy = 4,
    /// Destroyed
    Destroyed = 5,
    /// Error
    Error = 6,
}

// ============================================================================
// Resource Info
// ============================================================================

/// Resource create info
#[derive(Clone, Debug)]
pub struct ResourceCreateInfo {
    /// Name
    pub name: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Size in bytes
    pub size: u64,
    /// Flags
    pub flags: ResourceFlags,
    /// Group
    pub group: ResourceGroupHandle,
    /// Pool (optional)
    pub pool: ResourcePoolHandle,
}

impl ResourceCreateInfo {
    /// Creates new info
    pub fn new(resource_type: ResourceType) -> Self {
        Self {
            name: String::new(),
            resource_type,
            size: 0,
            flags: ResourceFlags::empty(),
            group: ResourceGroupHandle::NULL,
            pool: ResourcePoolHandle::NULL,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: ResourceFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// In group
    pub fn in_group(mut self, group: ResourceGroupHandle) -> Self {
        self.group = group;
        self
    }

    /// From pool
    pub fn from_pool(mut self, pool: ResourcePoolHandle) -> Self {
        self.pool = pool;
        self
    }

    /// Buffer resource
    pub fn buffer(size: u64) -> Self {
        Self::new(ResourceType::Buffer).with_size(size)
    }

    /// Texture resource
    pub fn texture(size: u64) -> Self {
        Self::new(ResourceType::Texture).with_size(size)
    }

    /// Shader resource
    pub fn shader() -> Self {
        Self::new(ResourceType::Shader)
    }

    /// Pipeline resource
    pub fn pipeline() -> Self {
        Self::new(ResourceType::Pipeline)
    }
}

impl Default for ResourceCreateInfo {
    fn default() -> Self {
        Self::new(ResourceType::Unknown)
    }
}

bitflags::bitflags! {
    /// Resource flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ResourceFlags: u32 {
        /// None
        const NONE = 0;
        /// Persistent (never GC'd)
        const PERSISTENT = 1 << 0;
        /// Transient (short-lived)
        const TRANSIENT = 1 << 1;
        /// Pooled
        const POOLED = 1 << 2;
        /// Shared across threads
        const SHARED = 1 << 3;
        /// External (not managed)
        const EXTERNAL = 1 << 4;
        /// Debug only
        const DEBUG_ONLY = 1 << 5;
    }
}

/// Resource descriptor
#[derive(Clone, Debug)]
pub struct ResourceDescriptor {
    /// Handle
    pub handle: ResourceHandle,
    /// Name
    pub name: String,
    /// Type
    pub resource_type: ResourceType,
    /// State
    pub state: ResourceState,
    /// Size in bytes
    pub size: u64,
    /// Reference count
    pub ref_count: u32,
    /// Creation frame
    pub creation_frame: u64,
    /// Last access frame
    pub last_access_frame: u64,
    /// Flags
    pub flags: ResourceFlags,
    /// Group
    pub group: ResourceGroupHandle,
}

impl ResourceDescriptor {
    /// Is ready for use
    pub fn is_ready(&self) -> bool {
        self.state == ResourceState::Ready || self.state == ResourceState::InUse
    }

    /// Age in frames
    pub fn age_frames(&self, current_frame: u64) -> u64 {
        current_frame.saturating_sub(self.last_access_frame)
    }

    /// Is stale (not used for many frames)
    pub fn is_stale(&self, current_frame: u64, threshold: u64) -> bool {
        self.age_frames(current_frame) > threshold
    }
}

impl Default for ResourceDescriptor {
    fn default() -> Self {
        Self {
            handle: ResourceHandle::NULL,
            name: String::new(),
            resource_type: ResourceType::Unknown,
            state: ResourceState::Unknown,
            size: 0,
            ref_count: 0,
            creation_frame: 0,
            last_access_frame: 0,
            flags: ResourceFlags::empty(),
            group: ResourceGroupHandle::NULL,
        }
    }
}

// ============================================================================
// Resource Pool
// ============================================================================

/// Resource pool create info
#[derive(Clone, Debug)]
pub struct ResourcePoolCreateInfo {
    /// Name
    pub name: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Initial capacity
    pub initial_capacity: u32,
    /// Max capacity
    pub max_capacity: u32,
    /// Resource size
    pub resource_size: u64,
    /// Growth policy
    pub growth_policy: PoolGrowthPolicy,
}

impl ResourcePoolCreateInfo {
    /// Creates new info
    pub fn new(resource_type: ResourceType) -> Self {
        Self {
            name: String::new(),
            resource_type,
            initial_capacity: 16,
            max_capacity: 1024,
            resource_size: 0,
            growth_policy: PoolGrowthPolicy::Double,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With capacity
    pub fn with_capacity(mut self, initial: u32, max: u32) -> Self {
        self.initial_capacity = initial;
        self.max_capacity = max;
        self
    }

    /// With resource size
    pub fn with_resource_size(mut self, size: u64) -> Self {
        self.resource_size = size;
        self
    }

    /// With growth policy
    pub fn with_growth_policy(mut self, policy: PoolGrowthPolicy) -> Self {
        self.growth_policy = policy;
        self
    }

    /// Buffer pool
    pub fn buffer_pool(buffer_size: u64) -> Self {
        Self::new(ResourceType::Buffer)
            .with_name("BufferPool")
            .with_resource_size(buffer_size)
    }

    /// Texture pool
    pub fn texture_pool(texture_size: u64) -> Self {
        Self::new(ResourceType::Texture)
            .with_name("TexturePool")
            .with_resource_size(texture_size)
    }

    /// Descriptor set pool
    pub fn descriptor_pool() -> Self {
        Self::new(ResourceType::DescriptorSet)
            .with_name("DescriptorPool")
            .with_capacity(64, 4096)
    }
}

impl Default for ResourcePoolCreateInfo {
    fn default() -> Self {
        Self::new(ResourceType::Unknown)
    }
}

/// Pool growth policy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PoolGrowthPolicy {
    /// Fixed size
    Fixed = 0,
    /// Double on grow
    #[default]
    Double = 1,
    /// Linear growth
    Linear = 2,
    /// Custom
    Custom = 3,
}

/// Pool statistics
#[derive(Clone, Debug, Default)]
pub struct PoolStats {
    /// Pool name
    pub name: String,
    /// Current capacity
    pub capacity: u32,
    /// Used count
    pub used: u32,
    /// Free count
    pub free: u32,
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Memory usage
    pub memory_usage: u64,
}

impl PoolStats {
    /// Usage ratio
    pub fn usage_ratio(&self) -> f32 {
        if self.capacity > 0 {
            self.used as f32 / self.capacity as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// Resource Group
// ============================================================================

/// Resource group create info
#[derive(Clone, Debug)]
pub struct ResourceGroupCreateInfo {
    /// Name
    pub name: String,
    /// Parent group
    pub parent: ResourceGroupHandle,
    /// Memory budget
    pub memory_budget: u64,
    /// Priority
    pub priority: i32,
}

impl ResourceGroupCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: ResourceGroupHandle::NULL,
            memory_budget: 0,
            priority: 0,
        }
    }

    /// With parent
    pub fn with_parent(mut self, parent: ResourceGroupHandle) -> Self {
        self.parent = parent;
        self
    }

    /// With memory budget
    pub fn with_budget(mut self, budget: u64) -> Self {
        self.memory_budget = budget;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl Default for ResourceGroupCreateInfo {
    fn default() -> Self {
        Self::new("Group")
    }
}

/// Group statistics
#[derive(Clone, Debug, Default)]
pub struct GroupStats {
    /// Group name
    pub name: String,
    /// Resource count
    pub resource_count: u32,
    /// Memory usage
    pub memory_usage: u64,
    /// Memory budget
    pub memory_budget: u64,
    /// Child groups
    pub child_count: u32,
}

impl GroupStats {
    /// Budget usage ratio
    pub fn budget_ratio(&self) -> f32 {
        if self.memory_budget > 0 {
            self.memory_usage as f32 / self.memory_budget as f32
        } else {
            0.0
        }
    }

    /// Over budget
    pub fn is_over_budget(&self) -> bool {
        self.memory_budget > 0 && self.memory_usage > self.memory_budget
    }
}

// ============================================================================
// Garbage Collection
// ============================================================================

/// GC settings
#[derive(Clone, Copy, Debug)]
pub struct GcSettings {
    /// GC enabled
    pub enabled: bool,
    /// Stale threshold (frames)
    pub stale_threshold: u64,
    /// GC interval (frames)
    pub gc_interval: u32,
    /// Max resources per GC
    pub max_per_gc: u32,
    /// Memory pressure threshold (0-1)
    pub memory_pressure_threshold: f32,
}

impl GcSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            stale_threshold: 300,
            gc_interval: 60,
            max_per_gc: 100,
            memory_pressure_threshold: 0.9,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            stale_threshold: 0,
            gc_interval: 0,
            max_per_gc: 0,
            memory_pressure_threshold: 1.0,
        }
    }

    /// Aggressive
    pub const fn aggressive() -> Self {
        Self {
            enabled: true,
            stale_threshold: 60,
            gc_interval: 10,
            max_per_gc: 500,
            memory_pressure_threshold: 0.7,
        }
    }

    /// Conservative
    pub const fn conservative() -> Self {
        Self {
            enabled: true,
            stale_threshold: 600,
            gc_interval: 120,
            max_per_gc: 50,
            memory_pressure_threshold: 0.95,
        }
    }

    /// With stale threshold
    pub const fn with_stale_threshold(mut self, frames: u64) -> Self {
        self.stale_threshold = frames;
        self
    }

    /// With interval
    pub const fn with_interval(mut self, frames: u32) -> Self {
        self.gc_interval = frames;
        self
    }
}

impl Default for GcSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// GC result
#[derive(Clone, Debug, Default)]
pub struct GcResult {
    /// Resources freed
    pub resources_freed: u32,
    /// Memory freed (bytes)
    pub memory_freed: u64,
    /// Duration (ms)
    pub duration_ms: f32,
    /// Resources scanned
    pub resources_scanned: u32,
}

// ============================================================================
// Resource Statistics
// ============================================================================

/// Resource manager statistics
#[derive(Clone, Debug, Default)]
pub struct ResourceManagerStats {
    /// Total resources
    pub total_resources: u32,
    /// Active resources
    pub active_resources: u32,
    /// Pending destroy
    pub pending_destroy: u32,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Active memory (bytes)
    pub active_memory: u64,
    /// Pool count
    pub pool_count: u32,
    /// Group count
    pub group_count: u32,
    /// Allocations this frame
    pub allocations: u32,
    /// Deallocations this frame
    pub deallocations: u32,
    /// GC runs
    pub gc_runs: u32,
    /// GC freed memory
    pub gc_freed_memory: u64,
    /// Resources by type
    pub by_type: ResourcesByType,
}

/// Resources by type
#[derive(Clone, Debug, Default)]
pub struct ResourcesByType {
    /// Buffers
    pub buffers: u32,
    /// Textures
    pub textures: u32,
    /// Samplers
    pub samplers: u32,
    /// Shaders
    pub shaders: u32,
    /// Pipelines
    pub pipelines: u32,
    /// Descriptor sets
    pub descriptor_sets: u32,
    /// Other
    pub other: u32,
}

impl ResourcesByType {
    /// Total
    pub fn total(&self) -> u32 {
        self.buffers
            + self.textures
            + self.samplers
            + self.shaders
            + self.pipelines
            + self.descriptor_sets
            + self.other
    }
}

// ============================================================================
// Leak Detection
// ============================================================================

/// Resource leak info
#[derive(Clone, Debug)]
pub struct ResourceLeak {
    /// Handle
    pub handle: ResourceHandle,
    /// Name
    pub name: String,
    /// Type
    pub resource_type: ResourceType,
    /// Size
    pub size: u64,
    /// Creation frame
    pub creation_frame: u64,
    /// Stack trace (if available)
    pub stack_trace: Option<String>,
}

impl ResourceLeak {
    /// Creates new leak info
    pub fn new(handle: ResourceHandle, name: String, resource_type: ResourceType) -> Self {
        Self {
            handle,
            name,
            resource_type,
            size: 0,
            creation_frame: 0,
            stack_trace: None,
        }
    }
}

/// Leak report
#[derive(Clone, Debug, Default)]
pub struct LeakReport {
    /// Total leaks
    pub total_leaks: u32,
    /// Total leaked memory
    pub leaked_memory: u64,
    /// Leaked resources
    pub leaks: Vec<ResourceLeak>,
}

impl LeakReport {
    /// Has leaks
    pub fn has_leaks(&self) -> bool {
        !self.leaks.is_empty()
    }

    /// Leaked memory in MB
    pub fn leaked_mb(&self) -> f32 {
        self.leaked_memory as f32 / (1024.0 * 1024.0)
    }
}
