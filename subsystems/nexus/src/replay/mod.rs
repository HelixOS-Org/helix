//! # Deterministic Execution Replay
//!
//! Record and replay kernel execution for debugging and analysis.
//!
//! ## Key Features
//!
//! - **Deterministic Replay**: Reproduce any execution exactly
//! - **Event Recording**: Capture all non-deterministic events
//! - **Checkpoint Integration**: Integrate with checkpoints
//! - **Time Travel Debugging**: Move forward and backward in execution

#![allow(dead_code)]

extern crate alloc;

mod engine;
mod event;
mod provider;
mod session;

// Re-export event types
// Re-export engine
pub use engine::{ReplayEngine, ReplayState, ReplayStats};
pub use event::{EventData, IoOperation, ReplayEvent, ReplayEventType};
// Re-export provider
pub use provider::DeterminismProvider;
// Re-export session
pub use session::RecordingSession;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_session() {
        let mut session = RecordingSession::new("test");

        session.start();
        assert!(session.is_recording());

        let seq = session.record_interrupt(32, None);
        assert!(seq.is_some());

        session.stop();
        assert!(!session.is_recording());
        assert_eq!(session.event_count(), 1);
    }

    #[test]
    fn test_replay_engine() {
        let mut session = RecordingSession::new("test");
        session.start();

        for i in 0..10 {
            session.record(ReplayEventType::Timer, EventData::Uint(i));
        }

        session.stop();

        let mut engine = ReplayEngine::new();
        engine.load(session);

        engine.play();

        let mut count = 0;
        while engine.step().is_some() {
            count += 1;
        }

        assert_eq!(count, 10);
        assert_eq!(engine.state(), ReplayState::Finished);
    }

    #[test]
    fn test_breakpoints() {
        let mut session = RecordingSession::new("test");
        session.start();

        for i in 0..10 {
            session.record(ReplayEventType::Timer, EventData::Uint(i));
        }

        session.stop();

        let mut engine = ReplayEngine::new();
        engine.load(session);
        engine.add_breakpoint(5);

        engine.play();
        let bp = engine.run_until_breakpoint();

        assert_eq!(bp, Some(5));
    }

    #[test]
    fn test_determinism_provider() {
        let mut session = RecordingSession::new("test");
        session.start();
        session.record_random(&[1, 2, 3, 4]);
        session.stop();

        let mut provider = DeterminismProvider::new();
        provider.load(session);

        let mut buffer = [0u8; 4];
        assert!(provider.get_random(&mut buffer));
        assert_eq!(buffer, [1, 2, 3, 4]);
    }
}
