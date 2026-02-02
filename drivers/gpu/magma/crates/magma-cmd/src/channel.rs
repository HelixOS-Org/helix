//! # GPU Channel
//!
//! GPU channels (FIFO contexts) for command submission.

use magma_core::{Error, GpuAddr, Handle, Result};

use crate::ring::{CommandRing, RingConfig};

// =============================================================================
// CHANNEL ID
// =============================================================================

/// GPU channel identifier
pub type ChannelId = Handle<GpuChannel>;

// =============================================================================
// CHANNEL CLASS
// =============================================================================

/// GPU engine class for channel binding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum EngineClass {
    /// Graphics (3D) engine - Turing
    GraphicsTuring = 0xC597,
    /// Graphics (3D) engine - Ampere
    GraphicsAmpere = 0xC697,
    /// Graphics (3D) engine - Ada
    GraphicsAda    = 0xC797,
    /// Compute engine
    Compute        = 0xC6C0,
    /// Copy engine (DMA)
    Copy           = 0xC6B5,
    /// 2D engine
    Twod           = 0x902D,
}

impl EngineClass {
    /// Check if this is a graphics class
    pub fn is_graphics(&self) -> bool {
        matches!(
            self,
            EngineClass::GraphicsTuring | EngineClass::GraphicsAmpere | EngineClass::GraphicsAda
        )
    }

    /// Check if this is a compute class
    pub fn is_compute(&self) -> bool {
        matches!(self, EngineClass::Compute)
    }
}

// =============================================================================
// CHANNEL STATE
// =============================================================================

/// GPU channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// Channel not yet initialized
    Uninitialized,
    /// Channel ready for submissions
    Ready,
    /// Channel is executing
    Running,
    /// Channel is preempted
    Preempted,
    /// Channel is in error state
    Error,
    /// Channel is closed
    Closed,
}

// =============================================================================
// GPU CHANNEL
// =============================================================================

/// A GPU channel (FIFO context)
#[derive(Debug)]
pub struct GpuChannel {
    /// Channel ID
    id: ChannelId,
    /// Engine class
    engine_class: EngineClass,
    /// Channel state
    state: ChannelState,
    /// Command ring
    ring: CommandRing,
    /// GPU instance block address
    instance_addr: GpuAddr,
    /// User memory for fence
    fence_addr: GpuAddr,
    /// Current fence value
    fence_value: u64,
}

impl GpuChannel {
    /// Create a new GPU channel
    ///
    /// # Safety
    /// All GPU addresses and ring pointers must be valid
    pub unsafe fn new(
        id: ChannelId,
        engine_class: EngineClass,
        ring_config: RingConfig,
        ring_gpu_addr: GpuAddr,
        ring_cpu_ptr: *mut u8,
        instance_addr: GpuAddr,
        fence_addr: GpuAddr,
    ) -> Result<Self> {
        // SAFETY: Caller guarantees pointer validity
        let ring = unsafe { CommandRing::new(ring_config, ring_gpu_addr, ring_cpu_ptr) };

        Ok(Self {
            id,
            engine_class,
            state: ChannelState::Ready,
            ring,
            instance_addr,
            fence_addr,
            fence_value: 0,
        })
    }

    /// Get channel ID
    pub fn id(&self) -> ChannelId {
        self.id
    }

    /// Get engine class
    pub fn engine_class(&self) -> EngineClass {
        self.engine_class
    }

    /// Get channel state
    pub fn state(&self) -> ChannelState {
        self.state
    }

    /// Check if channel can accept submissions
    pub fn can_submit(&self) -> bool {
        matches!(self.state, ChannelState::Ready | ChannelState::Running)
    }

    /// Submit commands
    pub fn submit(&mut self, cmd_addr: GpuAddr, cmd_size: u32) -> Result<u64> {
        if !self.can_submit() {
            return Err(Error::InvalidState);
        }

        let fence = self.ring.submit(cmd_addr, cmd_size, 0)?;
        self.state = ChannelState::Running;

        Ok(fence)
    }

    /// Get command ring reference
    pub fn ring(&self) -> &CommandRing {
        &self.ring
    }

    /// Get mutable command ring reference
    pub fn ring_mut(&mut self) -> &mut CommandRing {
        &mut self.ring
    }

    /// Get instance block address
    pub fn instance_addr(&self) -> GpuAddr {
        self.instance_addr
    }

    /// Get fence address
    pub fn fence_addr(&self) -> GpuAddr {
        self.fence_addr
    }

    /// Update fence value (call after GPU signals completion)
    pub fn update_fence(&mut self, value: u64) {
        if value > self.fence_value {
            self.fence_value = value;
            self.ring.update_completions(value);

            if self.ring.pending_count() == 0 {
                self.state = ChannelState::Ready;
            }
        }
    }

    /// Check if specific fence has completed
    pub fn is_fence_complete(&self, fence: u64) -> bool {
        fence <= self.fence_value
    }

    /// Wait for fence (busy-wait)
    pub fn wait_fence(&self, fence: u64) {
        while !self.is_fence_complete(fence) {
            core::hint::spin_loop();
        }
    }

    /// Close channel
    pub fn close(&mut self) {
        self.state = ChannelState::Closed;
    }

    /// Mark channel as errored
    pub fn mark_error(&mut self) {
        self.state = ChannelState::Error;
    }
}

// SAFETY: Channel designed for single-owner access
unsafe impl Send for GpuChannel {}

// =============================================================================
// CHANNEL MANAGER
// =============================================================================

/// Manages GPU channels
#[derive(Debug)]
pub struct ChannelManager {
    /// Active channels
    channels: alloc::collections::BTreeMap<ChannelId, GpuChannel>,
    /// Next channel ID
    next_id: u64,
}

impl ChannelManager {
    /// Create new channel manager
    pub fn new() -> Self {
        Self {
            channels: alloc::collections::BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Allocate a new channel ID
    pub fn alloc_id(&mut self) -> ChannelId {
        let id = ChannelId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Register a channel
    pub fn register(&mut self, channel: GpuChannel) {
        let id = channel.id();
        self.channels.insert(id, channel);
    }

    /// Get channel
    pub fn get(&self, id: ChannelId) -> Option<&GpuChannel> {
        self.channels.get(&id)
    }

    /// Get mutable channel
    pub fn get_mut(&mut self, id: ChannelId) -> Option<&mut GpuChannel> {
        self.channels.get_mut(&id)
    }

    /// Remove and return channel
    pub fn remove(&mut self, id: ChannelId) -> Option<GpuChannel> {
        self.channels.remove(&id)
    }

    /// Get all channels for an engine class
    pub fn by_engine(&self, class: EngineClass) -> impl Iterator<Item = &GpuChannel> {
        self.channels
            .values()
            .filter(move |c| c.engine_class == class)
    }

    /// Count active channels
    pub fn active_count(&self) -> usize {
        self.channels.values().filter(|c| c.can_submit()).count()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
