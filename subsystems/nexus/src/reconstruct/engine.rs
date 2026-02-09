//! Replay engine for state events.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::event::StateEvent;
use super::reconstructor::StateReconstructor;
use crate::core::{ComponentId, NexusTimestamp};
use crate::error::{HealingError, NexusResult};

/// Engine for replaying state events
pub struct ReplayEngine {
    /// State reconstructor
    reconstructor: StateReconstructor,
    /// Event handlers
    #[allow(clippy::type_complexity)]
    handlers: BTreeMap<u64, Vec<Box<dyn Fn(&StateEvent) + Send + Sync>>>,
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new() -> Self {
        Self {
            reconstructor: StateReconstructor::new(),
            handlers: BTreeMap::new(),
        }
    }

    /// Add event handler for component
    #[inline]
    pub fn on_event(
        &mut self,
        component: ComponentId,
        handler: impl Fn(&StateEvent) + Send + Sync + 'static,
    ) {
        self.handlers
            .entry(component.raw())
            .or_default()
            .push(Box::new(handler));
    }

    /// Replay events for a component
    pub fn replay(
        &self,
        component: ComponentId,
        from: NexusTimestamp,
        to: NexusTimestamp,
    ) -> NexusResult<ReplayResult> {
        let log = self.reconstructor.get_log(component).ok_or_else(|| {
            HealingError::ReconstructionFailed("No log found for component".into())
        })?;

        let mut events_replayed = 0;
        let handlers = self.handlers.get(&component.raw());

        for event in log.events() {
            if event.timestamp.ticks() >= from.ticks() && event.timestamp.ticks() <= to.ticks() {
                // Call handlers
                if let Some(handlers) = handlers {
                    for handler in handlers {
                        handler(event);
                    }
                }
                events_replayed += 1;
            }
        }

        Ok(ReplayResult {
            component,
            from,
            to,
            events_replayed,
        })
    }

    /// Get reconstructor
    #[inline(always)]
    pub fn reconstructor(&self) -> &StateReconstructor {
        &self.reconstructor
    }

    /// Get mutable reconstructor
    #[inline(always)]
    pub fn reconstructor_mut(&mut self) -> &mut StateReconstructor {
        &mut self.reconstructor
    }
}

impl Default for ReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a replay operation
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// Component
    pub component: ComponentId,
    /// Start timestamp
    pub from: NexusTimestamp,
    /// End timestamp
    pub to: NexusTimestamp,
    /// Events replayed
    pub events_replayed: usize,
}
