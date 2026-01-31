//! Determinism provider for replay.

use super::event::{EventData, ReplayEventType};
use super::session::RecordingSession;

/// Provides deterministic values during replay
pub struct DeterminismProvider {
    /// Session being replayed
    session: Option<RecordingSession>,
    /// Position in session
    position: usize,
}

impl DeterminismProvider {
    /// Create a new provider
    pub fn new() -> Self {
        Self {
            session: None,
            position: 0,
        }
    }

    /// Load session
    pub fn load(&mut self, session: RecordingSession) {
        self.session = Some(session);
        self.position = 0;
    }

    /// Get next random bytes (during replay)
    pub fn get_random(&mut self, buffer: &mut [u8]) -> bool {
        if let Some(session) = &self.session {
            // Find next Random event
            while self.position < session.events.len() {
                let event = &session.events[self.position];
                self.position += 1;

                if event.event_type == ReplayEventType::Random {
                    if let EventData::Random(ref bytes) = event.data {
                        let len = buffer.len().min(bytes.len());
                        buffer[..len].copy_from_slice(&bytes[..len]);
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get next interrupt (during replay)
    pub fn get_interrupt(&mut self) -> Option<(u8, Option<u32>)> {
        if let Some(session) = &self.session {
            while self.position < session.events.len() {
                let event = &session.events[self.position];
                self.position += 1;

                if event.event_type == ReplayEventType::Interrupt {
                    if let EventData::Interrupt { vector, error_code } = event.data {
                        return Some((vector, error_code));
                    }
                }
            }
        }
        None
    }

    /// Reset position
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

impl Default for DeterminismProvider {
    fn default() -> Self {
        Self::new()
    }
}
