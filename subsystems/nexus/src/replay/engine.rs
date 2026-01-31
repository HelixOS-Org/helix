//! Replay engine for deterministic execution replay.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::event::{ReplayEvent, ReplayEventType};
use super::session::RecordingSession;

/// Replay engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayState {
    /// Not replaying
    Idle,
    /// Replaying forward
    Playing,
    /// Paused
    Paused,
    /// Rewinding
    Rewinding,
    /// Finished
    Finished,
}

/// The replay engine
pub struct ReplayEngine {
    /// Current session
    session: Option<RecordingSession>,
    /// Current position
    position: AtomicU64,
    /// Replay state
    state: ReplayState,
    /// Event handlers
    /// In real implementation, would have handler functions
    /// Replay speed (1.0 = real time)
    speed: f64,
    /// Breakpoints (sequence numbers)
    breakpoints: Vec<u64>,
    /// Total replays
    total_replays: AtomicU64,
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new() -> Self {
        Self {
            session: None,
            position: AtomicU64::new(0),
            state: ReplayState::Idle,
            speed: 1.0,
            breakpoints: Vec::new(),
            total_replays: AtomicU64::new(0),
        }
    }

    /// Load a session for replay
    pub fn load(&mut self, session: RecordingSession) {
        self.session = Some(session);
        self.position.store(0, Ordering::SeqCst);
        self.state = ReplayState::Paused;
    }

    /// Start replay
    pub fn play(&mut self) {
        if self.session.is_some() {
            self.state = ReplayState::Playing;
            self.total_replays.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Pause replay
    pub fn pause(&mut self) {
        if self.state == ReplayState::Playing {
            self.state = ReplayState::Paused;
        }
    }

    /// Step forward one event
    pub fn step(&mut self) -> Option<&ReplayEvent> {
        let session = self.session.as_ref()?;
        let pos = self.position.load(Ordering::SeqCst);

        if pos >= session.events.len() as u64 {
            self.state = ReplayState::Finished;
            return None;
        }

        self.position.fetch_add(1, Ordering::SeqCst);
        session.events.get(pos as usize)
    }

    /// Step backward one event
    pub fn step_back(&mut self) -> Option<&ReplayEvent> {
        let session = self.session.as_ref()?;
        let pos = self.position.load(Ordering::SeqCst);

        if pos == 0 {
            return None;
        }

        self.position.fetch_sub(1, Ordering::SeqCst);
        session.events.get((pos - 1) as usize)
    }

    /// Seek to position
    pub fn seek(&mut self, position: u64) {
        if let Some(session) = &self.session {
            let clamped = position.min(session.events.len() as u64);
            self.position.store(clamped, Ordering::SeqCst);
        }
    }

    /// Seek to start
    pub fn rewind(&mut self) {
        self.position.store(0, Ordering::SeqCst);
        self.state = ReplayState::Paused;
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position.load(Ordering::SeqCst)
    }

    /// Get current state
    pub fn state(&self) -> ReplayState {
        self.state
    }

    /// Get current event
    pub fn current_event(&self) -> Option<&ReplayEvent> {
        let session = self.session.as_ref()?;
        let pos = self.position.load(Ordering::SeqCst);
        session.events.get(pos as usize)
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, sequence: u64) {
        if !self.breakpoints.contains(&sequence) {
            self.breakpoints.push(sequence);
            self.breakpoints.sort();
        }
    }

    /// Remove breakpoint
    pub fn remove_breakpoint(&mut self, sequence: u64) {
        self.breakpoints.retain(|&bp| bp != sequence);
    }

    /// Clear breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Is at breakpoint?
    pub fn at_breakpoint(&self) -> bool {
        let pos = self.position.load(Ordering::SeqCst);
        self.breakpoints.contains(&pos)
    }

    /// Set replay speed
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.1).min(100.0);
    }

    /// Get replay speed
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Run until breakpoint or end
    pub fn run_until_breakpoint(&mut self) -> Option<u64> {
        while self.state == ReplayState::Playing {
            if self.at_breakpoint() {
                self.pause();
                return Some(self.position());
            }
            if self.step().is_none() {
                return None;
            }
        }
        None
    }

    /// Find events by type
    pub fn find_events(&self, event_type: ReplayEventType) -> Vec<u64> {
        self.session
            .as_ref()
            .map(|s| {
                s.events
                    .iter()
                    .filter(|e| e.event_type == event_type)
                    .map(|e| e.sequence)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get statistics
    pub fn stats(&self) -> ReplayStats {
        ReplayStats {
            total_events: self.session.as_ref().map(|s| s.events.len()).unwrap_or(0),
            current_position: self.position.load(Ordering::Relaxed),
            breakpoint_count: self.breakpoints.len(),
            state: self.state,
            total_replays: self.total_replays.load(Ordering::Relaxed),
        }
    }
}

impl Default for ReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Replay statistics
#[derive(Debug, Clone)]
pub struct ReplayStats {
    /// Total events in session
    pub total_events: usize,
    /// Current position
    pub current_position: u64,
    /// Breakpoint count
    pub breakpoint_count: usize,
    /// Current state
    pub state: ReplayState,
    /// Total replays performed
    pub total_replays: u64,
}
