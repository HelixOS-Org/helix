//! Recording session for replay.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::event::{EventData, ReplayEvent, ReplayEventType};
use crate::core::NexusTimestamp;

/// A recording session
pub struct RecordingSession {
    /// Session ID
    pub id: u64,
    /// Session name
    pub name: String,
    /// Recorded events
    pub(crate) events: Vec<ReplayEvent>,
    /// Sequence counter
    sequence: AtomicU64,
    /// Is recording?
    recording: AtomicBool,
    /// Start timestamp
    start_time: NexusTimestamp,
    /// Maximum events
    max_events: usize,
    /// Per-CPU buffers
    per_cpu_events: BTreeMap<u32, Vec<ReplayEvent>>,
}

impl RecordingSession {
    /// Create a new session
    pub fn new(name: impl Into<String>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            events: Vec::new(),
            sequence: AtomicU64::new(0),
            recording: AtomicBool::new(false),
            start_time: NexusTimestamp::now(),
            max_events: 1_000_000,
            per_cpu_events: BTreeMap::new(),
        }
    }

    /// Start recording
    pub fn start(&mut self) {
        self.events.clear();
        self.per_cpu_events.clear();
        self.sequence.store(0, Ordering::SeqCst);
        self.start_time = NexusTimestamp::now();
        self.recording.store(true, Ordering::SeqCst);
    }

    /// Stop recording
    pub fn stop(&self) {
        self.recording.store(false, Ordering::SeqCst);
    }

    /// Is recording?
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    /// Record an event
    pub fn record(&mut self, event_type: ReplayEventType, data: EventData) -> Option<u64> {
        if !self.is_recording() {
            return None;
        }

        if self.events.len() >= self.max_events {
            return None;
        }

        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let event = ReplayEvent::new(event_type, seq).with_data(data);

        self.events.push(event);
        Some(seq)
    }

    /// Record interrupt
    pub fn record_interrupt(&mut self, vector: u8, error_code: Option<u32>) -> Option<u64> {
        self.record(ReplayEventType::Interrupt, EventData::Interrupt {
            vector,
            error_code,
        })
    }

    /// Record syscall
    pub fn record_syscall(&mut self, number: u64, args: [u64; 6], result: i64) -> Option<u64> {
        self.record(ReplayEventType::Syscall, EventData::Syscall {
            number,
            args,
            result,
        })
    }

    /// Record random bytes
    pub fn record_random(&mut self, bytes: &[u8]) -> Option<u64> {
        self.record(ReplayEventType::Random, EventData::Random(bytes.to_vec()))
    }

    /// Get events
    pub fn events(&self) -> &[ReplayEvent] {
        &self.events
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get duration
    pub fn duration(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.start_time)
    }
}
