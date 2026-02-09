// SPDX-License-Identifier: GPL-2.0
//! Coop event_sink — event collection and fan-out sink for cooperative monitoring.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

/// Event category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    Sync,
    Lock,
    Queue,
    Barrier,
    Channel,
    Resource,
    Timeout,
    Deadlock,
    Performance,
    Custom,
}

/// Sink event
#[derive(Debug, Clone)]
pub struct SinkEvent {
    pub id: u64,
    pub category: EventCategory,
    pub severity: EventSeverity,
    pub source_id: u32,
    pub timestamp: u64,
    pub message: String,
    pub data: u64,
}

/// Event filter
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub min_severity: EventSeverity,
    pub categories: Option<Vec<EventCategory>>,
    pub source_ids: Option<Vec<u32>>,
}

impl EventFilter {
    #[inline(always)]
    pub fn all() -> Self {
        Self { min_severity: EventSeverity::Trace, categories: None, source_ids: None }
    }

    #[inline]
    pub fn matches(&self, event: &SinkEvent) -> bool {
        if event.severity < self.min_severity { return false; }
        if let Some(ref cats) = self.categories {
            if !cats.contains(&event.category) { return false; }
        }
        if let Some(ref ids) = self.source_ids {
            if !ids.contains(&event.source_id) { return false; }
        }
        true
    }
}

/// Subscriber to an event sink
#[derive(Debug)]
pub struct SinkSubscriber {
    pub id: u32,
    pub filter: EventFilter,
    pub buffer: Vec<SinkEvent>,
    pub max_buffer: usize,
    pub received_count: u64,
    pub dropped_count: u64,
    pub subscribed_at: u64,
}

impl SinkSubscriber {
    pub fn new(id: u32, filter: EventFilter, max_buffer: usize, now: u64) -> Self {
        Self {
            id, filter, buffer: Vec::new(), max_buffer,
            received_count: 0, dropped_count: 0, subscribed_at: now,
        }
    }

    #[inline]
    pub fn deliver(&mut self, event: &SinkEvent) -> bool {
        if !self.filter.matches(event) { return false; }
        if self.buffer.len() >= self.max_buffer {
            self.dropped_count += 1;
            return false;
        }
        self.received_count += 1;
        self.buffer.push(event.clone());
        true
    }

    #[inline(always)]
    pub fn drain(&mut self) -> Vec<SinkEvent> {
        core::mem::take(&mut self.buffer)
    }

    #[inline(always)]
    pub fn pending(&self) -> usize { self.buffer.len() }

    #[inline]
    pub fn drop_rate(&self) -> f64 {
        let total = self.received_count + self.dropped_count;
        if total == 0 { return 0.0; }
        self.dropped_count as f64 / total as f64
    }
}

/// Event sink instance
#[derive(Debug)]
pub struct EventSinkInstance {
    pub name: String,
    pub subscribers: BTreeMap<u32, SinkSubscriber>,
    pub total_published: u64,
    pub total_delivered: u64,
    pub category_counts: ArrayMap<u64, 32>,
    pub created_at: u64,
}

impl EventSinkInstance {
    pub fn new(name: String, now: u64) -> Self {
        Self {
            name, subscribers: BTreeMap::new(),
            total_published: 0, total_delivered: 0,
            category_counts: ArrayMap::new(0), created_at: now,
        }
    }

    #[inline(always)]
    pub fn subscribe(&mut self, id: u32, filter: EventFilter, max_buffer: usize, now: u64) {
        self.subscribers.insert(id, SinkSubscriber::new(id, filter, max_buffer, now));
    }

    #[inline(always)]
    pub fn unsubscribe(&mut self, id: u32) -> bool {
        self.subscribers.remove(&id).is_some()
    }

    #[inline]
    pub fn publish(&mut self, event: SinkEvent) {
        self.total_published += 1;
        *self.category_counts.entry(event.category as u32).or_insert(0) += 1;
        for sub in self.subscribers.values_mut() {
            if sub.deliver(&event) {
                self.total_delivered += 1;
            }
        }
    }

    #[inline(always)]
    pub fn subscriber_count(&self) -> usize { self.subscribers.len() }
}

/// Event sink stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventSinkStats {
    pub total_sinks: u32,
    pub total_subscribers: u32,
    pub total_published: u64,
    pub total_delivered: u64,
    pub total_pending: u64,
}

/// Main event sink manager
pub struct CoopEventSink {
    sinks: BTreeMap<String, EventSinkInstance>,
    next_event_id: u64,
}

impl CoopEventSink {
    pub fn new() -> Self {
        Self { sinks: BTreeMap::new(), next_event_id: 1 }
    }

    #[inline(always)]
    pub fn create_sink(&mut self, name: String, now: u64) {
        self.sinks.insert(name.clone(), EventSinkInstance::new(name, now));
    }

    #[inline(always)]
    pub fn remove_sink(&mut self, name: &str) -> bool {
        self.sinks.remove(name).is_some()
    }

    #[inline]
    pub fn subscribe(&mut self, sink_name: &str, subscriber_id: u32, filter: EventFilter, max_buffer: usize, now: u64) -> bool {
        if let Some(sink) = self.sinks.get_mut(sink_name) {
            sink.subscribe(subscriber_id, filter, max_buffer, now);
            true
        } else { false }
    }

    #[inline]
    pub fn publish(&mut self, sink_name: &str, category: EventCategory, severity: EventSeverity,
                    source_id: u32, message: String, data: u64, now: u64) -> Option<u64> {
        let id = self.next_event_id;
        self.next_event_id += 1;
        let event = SinkEvent { id, category, severity, source_id, timestamp: now, message, data };
        self.sinks.get_mut(sink_name)?.publish(event);
        Some(id)
    }

    #[inline]
    pub fn drain_subscriber(&mut self, sink_name: &str, subscriber_id: u32) -> Vec<SinkEvent> {
        self.sinks.get_mut(sink_name)
            .and_then(|s| s.subscribers.get_mut(&subscriber_id))
            .map(|sub| sub.drain())
            .unwrap_or_default()
    }

    pub fn stats(&self) -> EventSinkStats {
        let total_subs: u32 = self.sinks.values().map(|s| s.subscriber_count() as u32).sum();
        let total_pending: u64 = self.sinks.values()
            .flat_map(|s| s.subscribers.values())
            .map(|sub| sub.pending() as u64).sum();
        EventSinkStats {
            total_sinks: self.sinks.len() as u32,
            total_subscribers: total_subs,
            total_published: self.sinks.values().map(|s| s.total_published).sum(),
            total_delivered: self.sinks.values().map(|s| s.total_delivered).sum(),
            total_pending,
        }
    }
}
