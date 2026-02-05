//! # Event Bus
//!
//! A publish-subscribe event system for broadcasting events to modules.
//!
//! ## Features
//!
//! - Topic-based subscriptions
//! - Priority-based delivery order
//! - Wildcard subscriptions
//! - Non-blocking event dispatch

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::RwLock;

// =============================================================================
// Event Topics
// =============================================================================

/// Event topics for subscription filtering
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventTopic {
    /// Timer tick events
    Tick,
    /// System shutdown
    Shutdown,
    /// Memory pressure notifications
    MemoryPressure,
    /// CPU hotplug events
    CpuHotplug,
    /// Process lifecycle events
    Process,
    /// Interrupt events
    Interrupt,
    /// Custom topic
    Custom(String),
    /// Subscribe to all events
    All,
}

impl EventTopic {
    /// Check if this topic matches a given event
    pub fn matches(&self, event: &Event) -> bool {
        match (self, event) {
            (EventTopic::All, _) => true,
            (EventTopic::Tick, Event::Tick { .. }) => true,
            (EventTopic::Shutdown, Event::Shutdown) => true,
            (EventTopic::MemoryPressure, Event::MemoryPressure { .. }) => true,
            (EventTopic::CpuHotplug, Event::CpuHotplug { .. }) => true,
            (EventTopic::Process, Event::ProcessCreated { .. }) => true,
            (EventTopic::Process, Event::ProcessExited { .. }) => true,
            (EventTopic::Interrupt, Event::Interrupt { .. }) => true,
            (EventTopic::Custom(a), Event::Custom { name, .. }) => a == name,
            _ => false,
        }
    }
}

// =============================================================================
// Events
// =============================================================================

/// System events
#[derive(Debug, Clone)]
pub enum Event {
    /// Timer tick
    Tick {
        /// Timestamp in nanoseconds since boot
        timestamp_ns: u64,
        /// Tick number
        tick_number: u64,
    },
    /// System shutdown initiated
    Shutdown,
    /// Memory pressure notification
    MemoryPressure {
        /// Pressure level
        level: MemoryPressureLevel,
        /// Available memory in bytes
        available_bytes: u64,
    },
    /// CPU hotplug event
    CpuHotplug {
        /// CPU ID
        cpu_id: u32,
        /// Is the CPU coming online?
        online: bool,
    },
    /// Process created
    ProcessCreated {
        /// Process ID
        pid: u64,
        /// Parent process ID
        parent_pid: u64,
    },
    /// Process exited
    ProcessExited {
        /// Process ID
        pid: u64,
        /// Exit code
        exit_code: i32,
    },
    /// Hardware interrupt
    Interrupt {
        /// Interrupt vector
        vector: u8,
        /// CPU that received it
        cpu_id: u32,
    },
    /// Custom event
    Custom {
        /// Event name
        name: String,
        /// Event data
        data: Vec<u8>,
    },
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// Memory is fine
    Normal,
    /// Memory is getting low
    Low,
    /// Memory is critically low
    Critical,
    /// Out of memory
    Oom,
}

// =============================================================================
// Event Response
// =============================================================================

/// Response from event handler
#[derive(Debug, Clone)]
pub enum EventResponse {
    /// Event was handled
    Handled,
    /// Event was ignored (not relevant)
    Ignored,
    /// Event handling was deferred
    Deferred,
    /// Error occurred
    Error(String),
}

// =============================================================================
// Subscriber
// =============================================================================

/// Event handler function type
pub type EventHandler = Box<dyn Fn(&Event) -> EventResponse + Send + Sync>;

/// Subscription ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Subscription priority (lower = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubscriptionPriority(pub u8);

impl SubscriptionPriority {
    /// Highest priority (runs first)
    pub const HIGHEST: Self = Self(0);
    /// High priority
    pub const HIGH: Self = Self(64);
    /// Normal priority
    pub const NORMAL: Self = Self(128);
    /// Low priority
    pub const LOW: Self = Self(192);
    /// Lowest priority (runs last)
    pub const LOWEST: Self = Self(255);
}

impl Default for SubscriptionPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Event subscription
pub struct EventSubscription {
    /// Unique ID
    id: SubscriptionId,
    /// Subscriber name (for debugging)
    name: String,
    /// Topics to receive
    topics: Vec<EventTopic>,
    /// Handler function
    handler: EventHandler,
    /// Priority
    priority: SubscriptionPriority,
    /// Is active?
    active: AtomicBool,
}

impl EventSubscription {
    /// Create a new subscription
    pub fn new(name: impl Into<String>, topics: Vec<EventTopic>, handler: EventHandler) -> Self {
        Self {
            id: SubscriptionId::new(),
            name: name.into(),
            topics,
            handler,
            priority: SubscriptionPriority::NORMAL,
            active: AtomicBool::new(true),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: SubscriptionPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Get subscription ID
    pub fn id(&self) -> SubscriptionId {
        self.id
    }

    /// Check if this subscription matches an event
    pub fn matches(&self, event: &Event) -> bool {
        if !self.active.load(Ordering::Relaxed) {
            return false;
        }
        self.topics.iter().any(|t| t.matches(event))
    }

    /// Handle an event
    pub fn handle(&self, event: &Event) -> EventResponse {
        (self.handler)(event)
    }

    /// Pause the subscription
    pub fn pause(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// Resume the subscription
    pub fn resume(&self) {
        self.active.store(true, Ordering::Relaxed);
    }
}

// =============================================================================
// Event Bus
// =============================================================================

/// Central event bus for the kernel
pub struct EventBus {
    /// All subscriptions, ordered by priority
    subscriptions: RwLock<Vec<EventSubscription>>,
    /// Event counter for statistics
    events_dispatched: AtomicU64,
    /// Is the bus active?
    active: AtomicBool,
}

impl EventBus {
    /// Create a new event bus
    pub const fn new() -> Self {
        Self {
            subscriptions: RwLock::new(Vec::new()),
            events_dispatched: AtomicU64::new(0),
            active: AtomicBool::new(true),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self, subscription: EventSubscription) -> SubscriptionId {
        let id = subscription.id;
        let priority = subscription.priority;

        let mut subs = self.subscriptions.write();

        // Insert in priority order
        let pos = subs
            .iter()
            .position(|s| s.priority > priority)
            .unwrap_or(subs.len());
        subs.insert(pos, subscription);

        log::debug!("EventBus: New subscription {:?}", id);
        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut subs = self.subscriptions.write();
        if let Some(pos) = subs.iter().position(|s| s.id == id) {
            subs.remove(pos);
            log::debug!("EventBus: Removed subscription {:?}", id);
            true
        } else {
            false
        }
    }

    /// Publish an event to all matching subscribers
    pub fn publish(&self, event: Event) -> EventDispatchResult {
        if !self.active.load(Ordering::Relaxed) {
            return EventDispatchResult {
                handled: 0,
                ignored: 0,
                errors: 0,
            };
        }

        self.events_dispatched.fetch_add(1, Ordering::Relaxed);

        let subs = self.subscriptions.read();
        let mut result = EventDispatchResult::default();

        for sub in subs.iter() {
            if sub.matches(&event) {
                match sub.handle(&event) {
                    EventResponse::Handled => result.handled += 1,
                    EventResponse::Ignored => result.ignored += 1,
                    EventResponse::Deferred => result.handled += 1,
                    EventResponse::Error(e) => {
                        log::warn!("EventBus: Handler '{}' error: {}", sub.name, e);
                        result.errors += 1;
                    },
                }
            }
        }

        result
    }

    /// Publish an event synchronously (blocking)
    pub fn publish_sync(&self, event: &Event) -> EventDispatchResult {
        // Same as publish but takes reference
        if !self.active.load(Ordering::Relaxed) {
            return EventDispatchResult::default();
        }

        self.events_dispatched.fetch_add(1, Ordering::Relaxed);

        let subs = self.subscriptions.read();
        let mut result = EventDispatchResult::default();

        for sub in subs.iter() {
            if sub.matches(event) {
                match sub.handle(event) {
                    EventResponse::Handled => result.handled += 1,
                    EventResponse::Ignored => result.ignored += 1,
                    EventResponse::Deferred => result.handled += 1,
                    EventResponse::Error(e) => {
                        log::warn!("EventBus: Handler '{}' error: {}", sub.name, e);
                        result.errors += 1;
                    },
                }
            }
        }

        result
    }

    /// Get the number of events dispatched
    pub fn events_dispatched(&self) -> u64 {
        self.events_dispatched.load(Ordering::Relaxed)
    }

    /// Get the number of subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.read().len()
    }

    /// Pause all event delivery
    pub fn pause(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    /// Resume event delivery
    pub fn resume(&self) {
        self.active.store(true, Ordering::SeqCst);
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of event dispatch
#[derive(Debug, Clone, Default)]
pub struct EventDispatchResult {
    /// Number of handlers that handled the event
    pub handled: usize,
    /// Number of handlers that ignored the event
    pub ignored: usize,
    /// Number of handlers that returned errors
    pub errors: usize,
}

// =============================================================================
// Global Event Bus
// =============================================================================

use spin::Once;

static GLOBAL_EVENT_BUS: Once<EventBus> = Once::new();

/// Get the global event bus
pub fn global_event_bus() -> &'static EventBus {
    GLOBAL_EVENT_BUS.call_once(EventBus::new)
}

/// Convenience function to publish an event
pub fn publish_event(event: Event) -> EventDispatchResult {
    global_event_bus().publish(event)
}

/// Convenience function to subscribe
pub fn subscribe(subscription: EventSubscription) -> SubscriptionId {
    global_event_bus().subscribe(subscription)
}
