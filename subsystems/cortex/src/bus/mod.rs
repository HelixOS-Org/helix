//! # CORTEX Event Bus
//!
//! The CortexBus is the central nervous system of the CORTEX framework.
//! It routes events between all subsystems, enabling real-time coordination
//! of the kernel's intelligence systems.
//!
//! ## Design
//!
//! The bus is designed for:
//! - **Low latency**: O(1) dispatch to subscribers
//! - **Priority handling**: Critical events processed first
//! - **Bounded queues**: No unbounded memory growth
//! - **Lock-free operation**: Uses atomic operations where possible
//! - **Deterministic timing**: Guaranteed maximum latency
//!
//! ## Event Flow
//!
//! ```text
//!                    ┌─────────────────────────────────────┐
//!                    │            CORTEX BUS               │
//!                    │                                     │
//!  ┌──────────┐      │   ┌──────────────────────────┐     │
//!  │ Producer ├──────┼──►│    Priority Queues       │     │
//!  └──────────┘      │   │  ┌──────┐ ┌──────┐      │     │
//!                    │   │  │ CRIT │ │ HIGH │ ...  │     │
//!  ┌──────────┐      │   │  └──┬───┘ └──┬───┘      │     │
//!  │ Producer ├──────┤   │     │        │          │     │
//!  └──────────┘      │   └─────┼────────┼──────────┘     │
//!                    │         │        │                 │
//!  ┌──────────┐      │         ▼        ▼                 │
//!  │ Producer ├──────┤    ┌─────────────────────┐         │
//!  └──────────┘      │    │    Dispatcher       │         │
//!                    │    └─────────┬───────────┘         │
//!                    │              │                     │
//!                    └──────────────┼─────────────────────┘
//!                                   │
//!           ┌───────────────────────┼───────────────────────┐
//!           │                       │                       │
//!           ▼                       ▼                       ▼
//!    ┌─────────────┐         ┌─────────────┐         ┌─────────────┐
//!    │Consciousness│         │   Neural    │         │Survivability│
//!    └─────────────┘         └─────────────┘         └─────────────┘
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::consciousness::{InvariantViolation, ViolationSeverity};
use crate::neural::Pattern;
use crate::survivability::{Threat, ThreatLevel};
use crate::{CortexResult, DecisionAction, IntelligenceLevel, SubsystemId, Timestamp};

// =============================================================================
// EVENT TYPES
// =============================================================================

/// Unique event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

/// Event priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Background processing
    Background = 0,

    /// Normal priority
    Normal     = 1,

    /// High priority
    High       = 2,

    /// Critical - must process immediately
    Critical   = 3,

    /// Emergency - preempt everything
    Emergency  = 4,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Event category for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    /// System lifecycle events
    System,

    /// Consciousness/invariant events
    Consciousness,

    /// Neural/pattern events
    Neural,

    /// Temporal/version events
    Temporal,

    /// Survivability/security events
    Survivability,

    /// Meta-kernel events
    Meta,

    /// Custom/extension events
    Custom,
}

/// Core CORTEX event types
#[derive(Clone)]
pub enum CortexEvent {
    // =========================================================================
    // System Events
    // =========================================================================
    /// System initialized
    SystemInit {
        timestamp: Timestamp,
        level: IntelligenceLevel,
    },

    /// System shutdown
    SystemShutdown {
        timestamp: Timestamp,
        reason: String,
    },

    /// Intelligence level changed
    LevelChanged {
        from: IntelligenceLevel,
        to: IntelligenceLevel,
        timestamp: Timestamp,
    },

    /// Heartbeat (periodic health check)
    Heartbeat { timestamp: Timestamp, sequence: u64 },

    // =========================================================================
    // Consciousness Events
    // =========================================================================
    /// Invariant checked
    InvariantChecked {
        subsystem: SubsystemId,
        satisfied: bool,
        timestamp: Timestamp,
    },

    /// Invariant violation detected
    InvariantViolation {
        violation: InvariantViolation,
        timestamp: Timestamp,
    },

    /// Invariant violation predicted
    ViolationPredicted {
        subsystem: SubsystemId,
        probability: f32,
        time_to_violation_us: u64,
        timestamp: Timestamp,
    },

    /// Contract checked
    ContractChecked {
        from: SubsystemId,
        to: SubsystemId,
        passed: bool,
        timestamp: Timestamp,
    },

    // =========================================================================
    // Neural Events
    // =========================================================================
    /// Pattern detected
    PatternDetected {
        pattern: Pattern,
        timestamp: Timestamp,
    },

    /// Decision made
    DecisionMade {
        action: DecisionAction,
        confidence: f32,
        timestamp: Timestamp,
    },

    /// Prediction generated
    Prediction {
        subsystem: SubsystemId,
        prediction: String,
        confidence: f32,
        timestamp: Timestamp,
    },

    /// Anomaly detected
    AnomalyDetected {
        subsystem: SubsystemId,
        metric: String,
        value: f64,
        expected: f64,
        deviation: f64,
        timestamp: Timestamp,
    },

    // =========================================================================
    // Temporal Events
    // =========================================================================
    /// Snapshot created
    SnapshotCreated {
        snapshot_id: crate::SnapshotId,
        subsystem: SubsystemId,
        timestamp: Timestamp,
    },

    /// Hot-swap initiated
    HotSwapStarted {
        subsystem: SubsystemId,
        from_version: String,
        to_version: String,
        timestamp: Timestamp,
    },

    /// Hot-swap completed
    HotSwapCompleted {
        subsystem: SubsystemId,
        success: bool,
        timestamp: Timestamp,
    },

    /// Rollback initiated
    RollbackStarted {
        subsystem: SubsystemId,
        to_snapshot: crate::SnapshotId,
        timestamp: Timestamp,
    },

    /// Rollback completed
    RollbackCompleted {
        subsystem: SubsystemId,
        success: bool,
        timestamp: Timestamp,
    },

    // =========================================================================
    // Survivability Events
    // =========================================================================
    /// Threat detected
    ThreatDetected {
        threat: Threat,
        timestamp: Timestamp,
    },

    /// Threat level changed
    ThreatLevelChanged {
        from: ThreatLevel,
        to: ThreatLevel,
        timestamp: Timestamp,
    },

    /// Subsystem isolated
    SubsystemIsolated {
        subsystem: SubsystemId,
        reason: String,
        timestamp: Timestamp,
    },

    /// Recovery started
    RecoveryStarted {
        subsystem: SubsystemId,
        strategy: String,
        timestamp: Timestamp,
    },

    /// Recovery completed
    RecoveryCompleted {
        subsystem: SubsystemId,
        success: bool,
        timestamp: Timestamp,
    },

    /// Survival mode entered
    SurvivalModeEntered {
        reason: String,
        timestamp: Timestamp,
    },

    /// Survival mode exited
    SurvivalModeExited { timestamp: Timestamp },

    // =========================================================================
    // Meta Events
    // =========================================================================
    /// Watchdog warning
    WatchdogWarning {
        remaining_cycles: u64,
        timestamp: Timestamp,
    },

    /// Watchdog timeout
    WatchdogTimeout {
        action_taken: String,
        timestamp: Timestamp,
    },

    /// Kernel panic
    KernelPanic {
        message: String,
        timestamp: Timestamp,
    },

    /// Kernel restart
    KernelRestart {
        reason: String,
        count: u64,
        timestamp: Timestamp,
    },

    // =========================================================================
    // Custom Events
    // =========================================================================
    /// Custom event (for extensions)
    Custom {
        name: String,
        data: Vec<u8>,
        timestamp: Timestamp,
    },
}

impl CortexEvent {
    /// Get event category
    pub fn category(&self) -> EventCategory {
        match self {
            Self::SystemInit { .. }
            | Self::SystemShutdown { .. }
            | Self::LevelChanged { .. }
            | Self::Heartbeat { .. } => EventCategory::System,

            Self::InvariantChecked { .. }
            | Self::InvariantViolation { .. }
            | Self::ViolationPredicted { .. }
            | Self::ContractChecked { .. } => EventCategory::Consciousness,

            Self::PatternDetected { .. }
            | Self::DecisionMade { .. }
            | Self::Prediction { .. }
            | Self::AnomalyDetected { .. } => EventCategory::Neural,

            Self::SnapshotCreated { .. }
            | Self::HotSwapStarted { .. }
            | Self::HotSwapCompleted { .. }
            | Self::RollbackStarted { .. }
            | Self::RollbackCompleted { .. } => EventCategory::Temporal,

            Self::ThreatDetected { .. }
            | Self::ThreatLevelChanged { .. }
            | Self::SubsystemIsolated { .. }
            | Self::RecoveryStarted { .. }
            | Self::RecoveryCompleted { .. }
            | Self::SurvivalModeEntered { .. }
            | Self::SurvivalModeExited { .. } => EventCategory::Survivability,

            Self::WatchdogWarning { .. }
            | Self::WatchdogTimeout { .. }
            | Self::KernelPanic { .. }
            | Self::KernelRestart { .. } => EventCategory::Meta,

            Self::Custom { .. } => EventCategory::Custom,
        }
    }

    /// Get default priority for event
    pub fn default_priority(&self) -> EventPriority {
        match self {
            Self::KernelPanic { .. } | Self::SurvivalModeEntered { .. } => EventPriority::Emergency,

            Self::ThreatDetected { ref threat, .. } => match threat.level {
                ThreatLevel::Critical | ThreatLevel::Existential => EventPriority::Emergency,
                ThreatLevel::High => EventPriority::Critical,
                ThreatLevel::Medium => EventPriority::High,
                _ => EventPriority::Normal,
            },

            Self::InvariantViolation { ref violation, .. } => match violation.severity {
                ViolationSeverity::Fatal => EventPriority::Emergency,
                ViolationSeverity::Critical => EventPriority::Critical,
                ViolationSeverity::Error => EventPriority::High,
                _ => EventPriority::Normal,
            },

            Self::WatchdogTimeout { .. } | Self::WatchdogWarning { .. } => EventPriority::Critical,

            Self::SubsystemIsolated { .. }
            | Self::HotSwapStarted { .. }
            | Self::RollbackStarted { .. } => EventPriority::High,

            Self::Heartbeat { .. } => EventPriority::Background,

            _ => EventPriority::Normal,
        }
    }

    /// Get timestamp
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Self::SystemInit { timestamp, .. }
            | Self::SystemShutdown { timestamp, .. }
            | Self::LevelChanged { timestamp, .. }
            | Self::Heartbeat { timestamp, .. }
            | Self::InvariantChecked { timestamp, .. }
            | Self::InvariantViolation { timestamp, .. }
            | Self::ViolationPredicted { timestamp, .. }
            | Self::ContractChecked { timestamp, .. }
            | Self::PatternDetected { timestamp, .. }
            | Self::DecisionMade { timestamp, .. }
            | Self::Prediction { timestamp, .. }
            | Self::AnomalyDetected { timestamp, .. }
            | Self::SnapshotCreated { timestamp, .. }
            | Self::HotSwapStarted { timestamp, .. }
            | Self::HotSwapCompleted { timestamp, .. }
            | Self::RollbackStarted { timestamp, .. }
            | Self::RollbackCompleted { timestamp, .. }
            | Self::ThreatDetected { timestamp, .. }
            | Self::ThreatLevelChanged { timestamp, .. }
            | Self::SubsystemIsolated { timestamp, .. }
            | Self::RecoveryStarted { timestamp, .. }
            | Self::RecoveryCompleted { timestamp, .. }
            | Self::SurvivalModeEntered { timestamp, .. }
            | Self::SurvivalModeExited { timestamp, .. }
            | Self::WatchdogWarning { timestamp, .. }
            | Self::WatchdogTimeout { timestamp, .. }
            | Self::KernelPanic { timestamp, .. }
            | Self::KernelRestart { timestamp, .. }
            | Self::Custom { timestamp, .. } => *timestamp,
        }
    }
}

// =============================================================================
// EVENT HANDLER
// =============================================================================

/// Event handler trait
pub trait EventHandler: Send + Sync {
    /// Handle an event
    fn handle(&mut self, event: &CortexEvent) -> CortexResult;

    /// Get handler name
    fn name(&self) -> &str;

    /// Get categories this handler subscribes to
    fn subscribed_categories(&self) -> &[EventCategory];

    /// Check if handler is active
    fn is_active(&self) -> bool {
        true
    }
}

/// Handler ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlerId(pub u64);

// =============================================================================
// PRIORITY QUEUE
// =============================================================================

/// Event with metadata
struct QueuedEvent {
    id: EventId,
    event: CortexEvent,
    priority: EventPriority,
    queued_at: Timestamp,
}

/// Priority queue for events
pub struct EventQueue {
    /// Queues by priority
    queues: [Vec<QueuedEvent>; 5],

    /// Total events queued
    total: AtomicUsize,

    /// Next event ID
    next_id: AtomicU64,

    /// Maximum queue size per priority
    max_size: usize,

    /// Events dropped due to full queue
    dropped: AtomicU64,
}

impl EventQueue {
    /// Create new event queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queues: [
                Vec::with_capacity(max_size),
                Vec::with_capacity(max_size),
                Vec::with_capacity(max_size),
                Vec::with_capacity(max_size),
                Vec::with_capacity(max_size),
            ],
            total: AtomicUsize::new(0),
            next_id: AtomicU64::new(1),
            max_size,
            dropped: AtomicU64::new(0),
        }
    }

    /// Enqueue event
    pub fn enqueue(
        &mut self,
        event: CortexEvent,
        priority: EventPriority,
        timestamp: Timestamp,
    ) -> Option<EventId> {
        let idx = priority as usize;

        if self.queues[idx].len() >= self.max_size {
            // Drop lowest priority events if this is higher priority
            if idx > 0 {
                for lower_idx in (0..idx).rev() {
                    if !self.queues[lower_idx].is_empty() {
                        self.queues[lower_idx].remove(0);
                        self.total.fetch_sub(1, Ordering::SeqCst);
                        self.dropped.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                }
            } else {
                self.dropped.fetch_add(1, Ordering::SeqCst);
                return None;
            }
        }

        let id = EventId(self.next_id.fetch_add(1, Ordering::SeqCst));

        self.queues[idx].push(QueuedEvent {
            id,
            event,
            priority,
            queued_at: timestamp,
        });

        self.total.fetch_add(1, Ordering::SeqCst);

        Some(id)
    }

    /// Dequeue highest priority event
    pub fn dequeue(&mut self) -> Option<(EventId, CortexEvent)> {
        // Check from highest to lowest priority
        for idx in (0..5).rev() {
            if !self.queues[idx].is_empty() {
                let queued = self.queues[idx].remove(0);
                self.total.fetch_sub(1, Ordering::SeqCst);
                return Some((queued.id, queued.event));
            }
        }

        None
    }

    /// Peek at highest priority event
    pub fn peek(&self) -> Option<&CortexEvent> {
        for idx in (0..5).rev() {
            if let Some(queued) = self.queues[idx].first() {
                return Some(&queued.event);
            }
        }

        None
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.total.load(Ordering::SeqCst) == 0
    }

    /// Get total queued events
    pub fn len(&self) -> usize {
        self.total.load(Ordering::SeqCst)
    }

    /// Get dropped event count
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::SeqCst)
    }

    /// Clear all queues
    pub fn clear(&mut self) {
        for queue in &mut self.queues {
            queue.clear();
        }
        self.total.store(0, Ordering::SeqCst);
    }
}

// =============================================================================
// CORTEX BUS
// =============================================================================

/// Bus statistics
#[derive(Debug, Default, Clone)]
pub struct BusStats {
    /// Total events published
    pub events_published: u64,

    /// Total events delivered
    pub events_delivered: u64,

    /// Total events dropped
    pub events_dropped: u64,

    /// Average delivery time (cycles)
    pub avg_delivery_time: u64,

    /// Maximum delivery time (cycles)
    pub max_delivery_time: u64,

    /// Events by category
    pub events_by_category: [u64; 7],

    /// Events by priority
    pub events_by_priority: [u64; 5],
}

/// CORTEX Event Bus
pub struct CortexBus {
    /// Event queue
    queue: EventQueue,

    /// Registered handlers
    handlers: BTreeMap<HandlerId, Box<dyn EventHandler>>,

    /// Next handler ID
    next_handler_id: AtomicU64,

    /// Category subscriptions: category -> handler IDs
    subscriptions: [Vec<HandlerId>; 7],

    /// Statistics
    stats: BusStats,

    /// Is bus active?
    active: bool,

    /// Maximum handlers
    max_handlers: usize,
}

impl CortexBus {
    /// Create new bus
    pub fn new(queue_size: usize, max_handlers: usize) -> Self {
        Self {
            queue: EventQueue::new(queue_size),
            handlers: BTreeMap::new(),
            next_handler_id: AtomicU64::new(1),
            subscriptions: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            stats: BusStats::default(),
            active: true,
            max_handlers,
        }
    }

    /// Register event handler
    pub fn register(&mut self, handler: Box<dyn EventHandler>) -> Option<HandlerId> {
        if self.handlers.len() >= self.max_handlers {
            return None;
        }

        let id = HandlerId(self.next_handler_id.fetch_add(1, Ordering::SeqCst));

        // Add to category subscriptions
        for category in handler.subscribed_categories() {
            let idx = *category as usize;
            if idx < self.subscriptions.len() {
                self.subscriptions[idx].push(id);
            }
        }

        self.handlers.insert(id, handler);

        Some(id)
    }

    /// Unregister handler
    pub fn unregister(&mut self, id: HandlerId) -> bool {
        if self.handlers.remove(&id).is_some() {
            // Remove from subscriptions
            for subscription in &mut self.subscriptions {
                subscription.retain(|&h| h != id);
            }
            true
        } else {
            false
        }
    }

    /// Publish event
    pub fn publish(&mut self, event: CortexEvent, timestamp: Timestamp) -> Option<EventId> {
        if !self.active {
            return None;
        }

        let priority = event.default_priority();
        let category = event.category() as usize;

        // Update stats
        self.stats.events_published += 1;
        self.stats.events_by_priority[priority as usize] += 1;
        if category < 7 {
            self.stats.events_by_category[category] += 1;
        }

        self.queue.enqueue(event, priority, timestamp)
    }

    /// Publish with custom priority
    pub fn publish_priority(
        &mut self,
        event: CortexEvent,
        priority: EventPriority,
        timestamp: Timestamp,
    ) -> Option<EventId> {
        if !self.active {
            return None;
        }

        let category = event.category() as usize;

        // Update stats
        self.stats.events_published += 1;
        self.stats.events_by_priority[priority as usize] += 1;
        if category < 7 {
            self.stats.events_by_category[category] += 1;
        }

        self.queue.enqueue(event, priority, timestamp)
    }

    /// Process one event
    pub fn process_one(&mut self, start_timestamp: Timestamp) -> Option<CortexResult> {
        let (id, event) = self.queue.dequeue()?;

        let category = event.category();
        let category_idx = category as usize;

        let mut result = CortexResult::Ignored;

        // Dispatch to subscribed handlers
        if category_idx < self.subscriptions.len() {
            let handler_ids: Vec<_> = self.subscriptions[category_idx].clone();

            for handler_id in handler_ids {
                if let Some(handler) = self.handlers.get_mut(&handler_id) {
                    if handler.is_active() {
                        let handler_result = handler.handle(&event);

                        // Keep the most significant result
                        result = match (&result, &handler_result) {
                            (CortexResult::Ignored, _) => handler_result,
                            (_, CortexResult::ActionTaken(_)) => handler_result,
                            _ => result,
                        };

                        self.stats.events_delivered += 1;
                    }
                }
            }
        }

        // Update timing stats
        let end_timestamp = crate::current_timestamp();
        let delivery_time = end_timestamp.saturating_sub(start_timestamp);

        if self.stats.max_delivery_time < delivery_time {
            self.stats.max_delivery_time = delivery_time;
        }

        // Rolling average
        self.stats.avg_delivery_time = (self.stats.avg_delivery_time * 7 + delivery_time) / 8;

        Some(result)
    }

    /// Process all pending events
    pub fn process_all(&mut self, timestamp: Timestamp) -> Vec<CortexResult> {
        let mut results = Vec::new();

        while let Some(result) = self.process_one(timestamp) {
            results.push(result);
        }

        results
    }

    /// Process events up to time budget (cycles)
    pub fn process_with_budget(
        &mut self,
        timestamp: Timestamp,
        budget_cycles: u64,
    ) -> Vec<CortexResult> {
        let start = crate::current_timestamp();
        let mut results = Vec::new();

        while let Some(result) = self.process_one(timestamp) {
            results.push(result);

            let elapsed = crate::current_timestamp().saturating_sub(start);
            if elapsed >= budget_cycles {
                break;
            }
        }

        results
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Is queue empty?
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> &BusStats {
        &self.stats
    }

    /// Get handler count
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Pause bus
    pub fn pause(&mut self) {
        self.active = false;
    }

    /// Resume bus
    pub fn resume(&mut self) {
        self.active = true;
    }

    /// Is bus active?
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Clear queue
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Get dropped event count
    pub fn dropped_count(&self) -> u64 {
        self.queue.dropped_count()
    }
}

// =============================================================================
// BROADCAST HANDLER
// =============================================================================

/// Handler that broadcasts to all other handlers
pub struct BroadcastHandler {
    name: String,
}

impl BroadcastHandler {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

impl EventHandler for BroadcastHandler {
    fn handle(&mut self, _event: &CortexEvent) -> CortexResult {
        // Broadcast handler just observes
        CortexResult::Observed
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn subscribed_categories(&self) -> &[EventCategory] {
        &[
            EventCategory::System,
            EventCategory::Consciousness,
            EventCategory::Neural,
            EventCategory::Temporal,
            EventCategory::Survivability,
            EventCategory::Meta,
            EventCategory::Custom,
        ]
    }
}

// =============================================================================
// FILTER HANDLER
// =============================================================================

/// Filter function type
pub type FilterFn = fn(&CortexEvent) -> bool;

/// Handler that filters events
pub struct FilterHandler {
    name: String,
    filter: FilterFn,
    categories: Vec<EventCategory>,
    inner: Box<dyn EventHandler>,
}

impl FilterHandler {
    pub fn new(
        name: &str,
        filter: FilterFn,
        categories: Vec<EventCategory>,
        inner: Box<dyn EventHandler>,
    ) -> Self {
        Self {
            name: String::from(name),
            filter,
            categories,
            inner,
        }
    }
}

impl EventHandler for FilterHandler {
    fn handle(&mut self, event: &CortexEvent) -> CortexResult {
        if (self.filter)(event) {
            self.inner.handle(event)
        } else {
            CortexResult::Ignored
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn subscribed_categories(&self) -> &[EventCategory] {
        &self.categories
    }
}

// =============================================================================
// LOGGING HANDLER
// =============================================================================

/// Handler that logs all events
pub struct LoggingHandler {
    name: String,
    categories: Vec<EventCategory>,
    log: Vec<(Timestamp, String)>,
    max_log_size: usize,
}

impl LoggingHandler {
    pub fn new(name: &str, categories: Vec<EventCategory>, max_log_size: usize) -> Self {
        Self {
            name: String::from(name),
            categories,
            log: Vec::with_capacity(max_log_size),
            max_log_size,
        }
    }

    pub fn get_log(&self) -> &[(Timestamp, String)] {
        &self.log
    }

    pub fn clear_log(&mut self) {
        self.log.clear();
    }
}

impl EventHandler for LoggingHandler {
    fn handle(&mut self, event: &CortexEvent) -> CortexResult {
        let timestamp = event.timestamp();
        let category = event.category();

        let message = alloc::format!("[{:?}] Event at {}", category, timestamp);

        if self.log.len() >= self.max_log_size {
            self.log.remove(0);
        }

        self.log.push((timestamp, message));

        CortexResult::Observed
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn subscribed_categories(&self) -> &[EventCategory] {
        &self.categories
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: String,
        categories: Vec<EventCategory>,
        handle_count: usize,
    }

    impl TestHandler {
        fn new(name: &str, categories: Vec<EventCategory>) -> Self {
            Self {
                name: String::from(name),
                categories,
                handle_count: 0,
            }
        }
    }

    impl EventHandler for TestHandler {
        fn handle(&mut self, _event: &CortexEvent) -> CortexResult {
            self.handle_count += 1;
            CortexResult::Observed
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn subscribed_categories(&self) -> &[EventCategory] {
            &self.categories
        }
    }

    #[test]
    fn test_event_queue() {
        let mut queue = EventQueue::new(100);

        let event = CortexEvent::Heartbeat {
            timestamp: 1000,
            sequence: 1,
        };

        let id = queue.enqueue(event.clone(), EventPriority::Normal, 1000);
        assert!(id.is_some());
        assert_eq!(queue.len(), 1);

        let (_, dequeued) = queue.dequeue().unwrap();
        assert!(matches!(dequeued, CortexEvent::Heartbeat { .. }));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = EventQueue::new(100);

        // Enqueue in reverse priority order
        queue.enqueue(
            CortexEvent::Heartbeat {
                timestamp: 1,
                sequence: 1,
            },
            EventPriority::Background,
            1,
        );
        queue.enqueue(
            CortexEvent::Heartbeat {
                timestamp: 2,
                sequence: 2,
            },
            EventPriority::Emergency,
            2,
        );
        queue.enqueue(
            CortexEvent::Heartbeat {
                timestamp: 3,
                sequence: 3,
            },
            EventPriority::Normal,
            3,
        );

        // Should dequeue in priority order (highest first)
        let (_, event) = queue.dequeue().unwrap();
        assert_eq!(event.timestamp(), 2); // Emergency

        let (_, event) = queue.dequeue().unwrap();
        assert_eq!(event.timestamp(), 3); // Normal

        let (_, event) = queue.dequeue().unwrap();
        assert_eq!(event.timestamp(), 1); // Background
    }

    #[test]
    fn test_bus_publish_process() {
        let mut bus = CortexBus::new(100, 10);

        let handler = TestHandler::new("test", vec![EventCategory::System]);
        bus.register(Box::new(handler));

        let event = CortexEvent::Heartbeat {
            timestamp: 1000,
            sequence: 1,
        };

        bus.publish(event, 1000);

        let results = bus.process_all(1000);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_category_routing() {
        let mut bus = CortexBus::new(100, 10);

        // Handler only subscribed to Neural events
        let handler = TestHandler::new("neural", vec![EventCategory::Neural]);
        bus.register(Box::new(handler));

        // Publish System event - should not be delivered
        bus.publish(
            CortexEvent::Heartbeat {
                timestamp: 1000,
                sequence: 1,
            },
            1000,
        );

        let results = bus.process_all(1000);
        // Event was processed but handler wasn't called
        assert!(results.is_empty() || matches!(results[0], CortexResult::Ignored));
    }
}
