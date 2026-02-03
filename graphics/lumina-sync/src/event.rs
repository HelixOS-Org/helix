//! GPU Events
//!
//! Fine-grained GPU signaling for advanced synchronization.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use lumina_core::Handle;

use crate::barrier::PipelineStageFlags;

// ============================================================================
// Event State
// ============================================================================

/// Event state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    /// Event is reset.
    Reset,
    /// Event is set.
    Set,
}

// ============================================================================
// Event Handle
// ============================================================================

/// Handle to an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventHandle(Handle<Event>);

impl EventHandle {
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

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Event Description
// ============================================================================

/// Description for event creation.
#[derive(Debug, Clone)]
pub struct EventDesc {
    /// Device only (not host visible).
    pub device_only: bool,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for EventDesc {
    fn default() -> Self {
        Self {
            device_only: false,
            label: None,
        }
    }
}

impl EventDesc {
    /// Create a new event description.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create device-only event.
    pub fn device_only() -> Self {
        Self {
            device_only: true,
            label: None,
        }
    }

    /// Set debug label.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(String::from(label));
        self
    }
}

// ============================================================================
// Event
// ============================================================================

/// A GPU event for fine-grained signaling.
pub struct Event {
    /// Handle.
    pub handle: EventHandle,
    /// State.
    state: AtomicU32,
    /// Set count.
    set_count: AtomicU64,
    /// Wait count.
    wait_count: AtomicU64,
    /// Device only.
    pub device_only: bool,
    /// Debug label.
    pub label: Option<String>,
}

impl Event {
    /// Create a new event.
    pub fn new(handle: EventHandle, desc: &EventDesc) -> Self {
        Self {
            handle,
            state: AtomicU32::new(EventState::Reset as u32),
            set_count: AtomicU64::new(0),
            wait_count: AtomicU64::new(0),
            device_only: desc.device_only,
            label: desc.label.clone(),
        }
    }

    /// Get current state.
    pub fn state(&self) -> EventState {
        if self.state.load(Ordering::Acquire) == 0 {
            EventState::Reset
        } else {
            EventState::Set
        }
    }

    /// Check if set.
    pub fn is_set(&self) -> bool {
        self.state() == EventState::Set
    }

    /// Set the event (from host).
    pub fn set(&self) {
        self.state.store(EventState::Set as u32, Ordering::Release);
        self.set_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset the event (from host).
    pub fn reset(&self) {
        self.state.store(EventState::Reset as u32, Ordering::Release);
    }

    /// Wait for the event.
    pub fn wait(&self) {
        self.wait_count.fetch_add(1, Ordering::Relaxed);
        while !self.is_set() {
            core::hint::spin_loop();
        }
    }

    /// Get set count.
    pub fn set_count(&self) -> u64 {
        self.set_count.load(Ordering::Relaxed)
    }

    /// Get wait count.
    pub fn wait_count(&self) -> u64 {
        self.wait_count.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Event Set Info
// ============================================================================

/// Event set info for command buffer recording.
#[derive(Debug, Clone)]
pub struct EventSetInfo {
    /// Event handle.
    pub event: EventHandle,
    /// Source stage mask.
    pub src_stage: PipelineStageFlags,
}

impl EventSetInfo {
    /// Create a new set info.
    pub fn new(event: EventHandle, src_stage: PipelineStageFlags) -> Self {
        Self { event, src_stage }
    }
}

/// Event wait info for command buffer recording.
#[derive(Debug, Clone)]
pub struct EventWaitInfo {
    /// Event handle.
    pub event: EventHandle,
    /// Source stage mask.
    pub src_stage: PipelineStageFlags,
    /// Destination stage mask.
    pub dst_stage: PipelineStageFlags,
}

impl EventWaitInfo {
    /// Create a new wait info.
    pub fn new(
        event: EventHandle,
        src_stage: PipelineStageFlags,
        dst_stage: PipelineStageFlags,
    ) -> Self {
        Self {
            event,
            src_stage,
            dst_stage,
        }
    }
}

// ============================================================================
// Event Manager
// ============================================================================

/// Statistics for event manager.
#[derive(Debug, Clone, Default)]
pub struct EventStatistics {
    /// Total events created.
    pub created: u64,
    /// Total events destroyed.
    pub destroyed: u64,
    /// Active events.
    pub active: u32,
    /// Device-only events.
    pub device_only: u32,
    /// Total sets.
    pub total_sets: u64,
    /// Total waits.
    pub total_waits: u64,
}

/// Manages GPU events.
pub struct EventManager {
    /// Events.
    events: Vec<Option<Event>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: EventStatistics,
}

impl EventManager {
    /// Create a new event manager.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: EventStatistics::default(),
        }
    }

    /// Create an event.
    pub fn create(&mut self, desc: &EventDesc) -> EventHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.events.len() as u32;
            self.events.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = EventHandle::new(index, generation);
        let event = Event::new(handle, desc);

        if desc.device_only {
            self.stats.device_only += 1;
        }

        self.events[index as usize] = Some(event);
        self.stats.created += 1;
        self.stats.active += 1;

        handle
    }

    /// Create a default event.
    pub fn create_default(&mut self) -> EventHandle {
        self.create(&EventDesc::default())
    }

    /// Create a device-only event.
    pub fn create_device_only(&mut self) -> EventHandle {
        self.create(&EventDesc::device_only())
    }

    /// Destroy an event.
    pub fn destroy(&mut self, handle: EventHandle) {
        let index = handle.index() as usize;
        if index < self.events.len() && self.generations[index] == handle.generation() {
            if let Some(event) = self.events[index].take() {
                if event.device_only {
                    self.stats.device_only = self.stats.device_only.saturating_sub(1);
                }
                self.stats.total_sets += event.set_count();
                self.stats.total_waits += event.wait_count();
            }
            self.generations[index] = self.generations[index].wrapping_add(1);
            self.free_indices.push(index as u32);
            self.stats.destroyed += 1;
            self.stats.active = self.stats.active.saturating_sub(1);
        }
    }

    /// Get an event.
    pub fn get(&self, handle: EventHandle) -> Option<&Event> {
        let index = handle.index() as usize;
        if index >= self.events.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.events[index].as_ref()
    }

    /// Check if event is set.
    pub fn is_set(&self, handle: EventHandle) -> bool {
        self.get(handle).map(|e| e.is_set()).unwrap_or(false)
    }

    /// Set an event.
    pub fn set(&self, handle: EventHandle) {
        if let Some(event) = self.get(handle) {
            event.set();
        }
    }

    /// Reset an event.
    pub fn reset(&self, handle: EventHandle) {
        if let Some(event) = self.get(handle) {
            event.reset();
        }
    }

    /// Wait for an event.
    pub fn wait(&self, handle: EventHandle) {
        if let Some(event) = self.get(handle) {
            event.wait();
        }
    }

    /// Wait for all events.
    pub fn wait_all(&self, handles: &[EventHandle]) {
        for handle in handles {
            self.wait(*handle);
        }
    }

    /// Wait for any event.
    pub fn wait_any(&self, handles: &[EventHandle]) -> Option<EventHandle> {
        loop {
            for handle in handles {
                if self.is_set(*handle) {
                    return Some(*handle);
                }
            }
            core::hint::spin_loop();
        }
    }

    /// Get statistics.
    pub fn statistics(&self) -> &EventStatistics {
        &self.stats
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Event Chain
// ============================================================================

/// Chain of event operations.
#[derive(Debug, Clone, Default)]
pub struct EventChain {
    /// Set operations.
    pub sets: Vec<EventSetInfo>,
    /// Wait operations.
    pub waits: Vec<EventWaitInfo>,
    /// Reset operations.
    pub resets: Vec<EventHandle>,
}

impl EventChain {
    /// Create a new chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a set operation.
    pub fn set(mut self, event: EventHandle, src_stage: PipelineStageFlags) -> Self {
        self.sets.push(EventSetInfo::new(event, src_stage));
        self
    }

    /// Add a wait operation.
    pub fn wait(
        mut self,
        event: EventHandle,
        src_stage: PipelineStageFlags,
        dst_stage: PipelineStageFlags,
    ) -> Self {
        self.waits.push(EventWaitInfo::new(event, src_stage, dst_stage));
        self
    }

    /// Add a reset operation.
    pub fn reset(mut self, event: EventHandle) -> Self {
        self.resets.push(event);
        self
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.waits.is_empty() && self.resets.is_empty()
    }
}
