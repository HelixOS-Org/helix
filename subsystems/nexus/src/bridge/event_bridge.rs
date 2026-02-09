//! # Bridge Event Bridge
//!
//! Event notification system bridging kernel events to userspace:
//! - epoll-style event multiplexing model
//! - Event source registration and management
//! - Edge-triggered vs Level-triggered modes
//! - Event coalescing for batched delivery
//! - Priority event handling
//! - Wakeup optimization

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Event trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    EdgeTriggered,
    LevelTriggered,
    OneShot,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Readable,
    Writable,
    Error,
    HangUp,
    Priority,
    ReadHangUp,
}

/// Event source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSourceType {
    FileDescriptor,
    Signal,
    Timer,
    Ipc,
    Custom,
}

/// Event interest registration
#[derive(Debug, Clone)]
pub struct EventInterest {
    pub fd: i32,
    pub source_type: EventSourceType,
    pub events: u32, // bitmask of EventKind
    pub trigger: TriggerMode,
    pub user_data: u64,
    pub priority: u8,
    pub coalesce: bool,
}

impl EventInterest {
    pub fn new(fd: i32, events: u32, user_data: u64) -> Self {
        Self {
            fd,
            source_type: EventSourceType::FileDescriptor,
            events,
            trigger: TriggerMode::LevelTriggered,
            user_data,
            priority: 0,
            coalesce: false,
        }
    }

    #[inline(always)]
    pub fn edge_triggered(mut self) -> Self {
        self.trigger = TriggerMode::EdgeTriggered;
        self
    }

    #[inline(always)]
    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }
}

/// Ready event
#[derive(Debug, Clone)]
pub struct ReadyEvent {
    pub fd: i32,
    pub events: u32,
    pub user_data: u64,
    pub timestamp: u64,
    pub coalesced_count: u32,
}

/// Epoll-like instance
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventInstance {
    pub instance_id: u64,
    pub owner_pid: u64,
    pub interests: BTreeMap<i32, EventInterest>,
    pub ready_list: Vec<ReadyEvent>,
    pub max_events: usize,
    pub total_waits: u64,
    pub total_events_delivered: u64,
    pub total_coalesced: u64,
}

impl EventInstance {
    pub fn new(instance_id: u64, owner_pid: u64) -> Self {
        Self {
            instance_id,
            owner_pid,
            interests: BTreeMap::new(),
            ready_list: Vec::new(),
            max_events: 1024,
            total_waits: 0,
            total_events_delivered: 0,
            total_coalesced: 0,
        }
    }

    #[inline(always)]
    pub fn add_interest(&mut self, interest: EventInterest) {
        self.interests.insert(interest.fd, interest);
    }

    #[inline(always)]
    pub fn remove_interest(&mut self, fd: i32) {
        self.interests.remove(&fd);
    }

    #[inline]
    pub fn modify_interest(&mut self, fd: i32, new_events: u32) {
        if let Some(interest) = self.interests.get_mut(&fd) {
            interest.events = new_events;
        }
    }

    /// Signal that fd is ready with given events
    pub fn signal_ready(&mut self, fd: i32, events: u32, now: u64) {
        let interest = match self.interests.get(&fd) {
            Some(i) => i,
            None => return,
        };

        // Check if events match interest
        if interest.events & events == 0 { return; }

        // Coalesce if enabled
        if interest.coalesce {
            if let Some(existing) = self.ready_list.iter_mut().find(|e| e.fd == fd) {
                existing.events |= events;
                existing.coalesced_count += 1;
                existing.timestamp = now;
                self.total_coalesced += 1;
                return;
            }
        }

        let ready = ReadyEvent {
            fd,
            events: interest.events & events,
            user_data: interest.user_data,
            timestamp: now,
            coalesced_count: 1,
        };

        self.ready_list.push(ready);

        // Handle oneshot
        if interest.trigger == TriggerMode::OneShot {
            self.interests.remove(&fd);
        }
    }

    /// Wait for events (return ready events up to max)
    pub fn wait(&mut self, max: usize) -> Vec<ReadyEvent> {
        self.total_waits += 1;

        // Sort by priority (higher first) then timestamp
        self.ready_list.sort_by(|a, b| {
            let pa = self.interests.get(&a.fd).map(|i| i.priority).unwrap_or(0);
            let pb = self.interests.get(&b.fd).map(|i| i.priority).unwrap_or(0);
            pb.cmp(&pa).then(a.timestamp.cmp(&b.timestamp))
        });

        let take = max.min(self.ready_list.len());
        let events: Vec<ReadyEvent> = self.ready_list.drain(..take).collect();
        self.total_events_delivered += events.len() as u64;

        // For edge-triggered, don't re-add
        // For level-triggered, they'll be re-signaled if still ready

        events
    }

    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.ready_list.len()
    }
}

/// Event bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeEventBridgeStats {
    pub active_instances: usize,
    pub total_interests: usize,
    pub total_pending: usize,
    pub total_waits: u64,
    pub total_events_delivered: u64,
    pub total_coalesced: u64,
}

/// Bridge Event Bridge
#[repr(align(64))]
pub struct BridgeEventBridge {
    instances: BTreeMap<u64, EventInstance>,
    next_instance_id: u64,
    stats: BridgeEventBridgeStats,
}

impl BridgeEventBridge {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_instance_id: 1,
            stats: BridgeEventBridgeStats::default(),
        }
    }

    #[inline]
    pub fn create_instance(&mut self, owner_pid: u64) -> u64 {
        let id = self.next_instance_id;
        self.next_instance_id += 1;
        self.instances.insert(id, EventInstance::new(id, owner_pid));
        self.recompute();
        id
    }

    #[inline(always)]
    pub fn destroy_instance(&mut self, instance_id: u64) {
        self.instances.remove(&instance_id);
        self.recompute();
    }

    #[inline]
    pub fn add_interest(&mut self, instance_id: u64, interest: EventInterest) {
        if let Some(inst) = self.instances.get_mut(&instance_id) {
            inst.add_interest(interest);
        }
        self.recompute();
    }

    #[inline]
    pub fn signal_ready(&mut self, instance_id: u64, fd: i32, events: u32, now: u64) {
        if let Some(inst) = self.instances.get_mut(&instance_id) {
            inst.signal_ready(fd, events, now);
        }
    }

    /// Broadcast event to all instances watching this fd
    #[inline]
    pub fn broadcast_ready(&mut self, fd: i32, events: u32, now: u64) {
        let ids: Vec<u64> = self.instances.keys().copied().collect();
        for id in ids {
            if let Some(inst) = self.instances.get_mut(&id) {
                inst.signal_ready(fd, events, now);
            }
        }
    }

    #[inline]
    pub fn wait(&mut self, instance_id: u64, max_events: usize) -> Vec<ReadyEvent> {
        if let Some(inst) = self.instances.get_mut(&instance_id) {
            let events = inst.wait(max_events);
            self.recompute();
            events
        } else {
            Vec::new()
        }
    }

    fn recompute(&mut self) {
        self.stats.active_instances = self.instances.len();
        self.stats.total_interests = self.instances.values().map(|i| i.interests.len()).sum();
        self.stats.total_pending = self.instances.values().map(|i| i.pending_count()).sum();
        self.stats.total_waits = self.instances.values().map(|i| i.total_waits).sum();
        self.stats.total_events_delivered = self.instances.values().map(|i| i.total_events_delivered).sum();
        self.stats.total_coalesced = self.instances.values().map(|i| i.total_coalesced).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeEventBridgeStats {
        &self.stats
    }
}
