//! Probe Registry
//!
//! Manages registration and lifecycle of all probes.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::types::ProbeId;
use super::probe::{Probe, ProbeError, ProbeType};

// ============================================================================
// PROBE REGISTRY
// ============================================================================

/// Probe registry - manages all probes
pub struct ProbeRegistry {
    /// Registered probes
    probes: BTreeMap<ProbeId, Box<dyn Probe>>,
    /// Probes by type
    by_type: BTreeMap<ProbeType, Vec<ProbeId>>,
    /// Active probe count
    active_count: usize,
}

impl ProbeRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            probes: BTreeMap::new(),
            by_type: BTreeMap::new(),
            active_count: 0,
        }
    }

    /// Register a probe
    pub fn register(&mut self, probe: Box<dyn Probe>) -> ProbeId {
        let id = probe.id();
        let probe_type = probe.probe_type();

        self.probes.insert(id, probe);
        self.by_type.entry(probe_type).or_default().push(id);

        id
    }

    /// Unregister a probe
    pub fn unregister(&mut self, id: ProbeId) -> Option<Box<dyn Probe>> {
        if let Some(probe) = self.probes.remove(&id) {
            let probe_type = probe.probe_type();
            if let Some(ids) = self.by_type.get_mut(&probe_type) {
                ids.retain(|&i| i != id);
            }
            if probe.state().is_active() {
                self.active_count = self.active_count.saturating_sub(1);
            }
            Some(probe)
        } else {
            None
        }
    }

    /// Get probe by ID
    pub fn get(&self, id: ProbeId) -> Option<&dyn Probe> {
        self.probes.get(&id).map(|p| p.as_ref())
    }

    /// Get mutable probe by ID
    pub fn get_mut(&mut self, id: ProbeId) -> Option<&mut dyn Probe> {
        self.probes.get_mut(&id).map(|p| p.as_mut())
    }

    /// Get probes by type
    pub fn by_type(&self, probe_type: ProbeType) -> Vec<&dyn Probe> {
        self.by_type
            .get(&probe_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.probes.get(id).map(|p| p.as_ref()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all probe IDs
    pub fn all_ids(&self) -> Vec<ProbeId> {
        self.probes.keys().copied().collect()
    }

    /// Get all active probes
    pub fn active(&self) -> Vec<&dyn Probe> {
        self.probes
            .values()
            .filter(|p| p.state().is_active())
            .map(|p| p.as_ref())
            .collect()
    }

    /// Get all active probe IDs
    pub fn active_ids(&self) -> Vec<ProbeId> {
        self.probes
            .iter()
            .filter(|(_, p)| p.state().is_active())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Start a specific probe
    pub fn start(&mut self, id: ProbeId) -> Result<(), ProbeError> {
        if let Some(probe) = self.probes.get_mut(&id) {
            probe.start()?;
            self.active_count += 1;
            Ok(())
        } else {
            Err(ProbeError::new(
                super::probe::ProbeErrorCode::NotInitialized,
                "Probe not found",
            ))
        }
    }

    /// Stop a specific probe
    pub fn stop(&mut self, id: ProbeId) -> Result<(), ProbeError> {
        if let Some(probe) = self.probes.get_mut(&id) {
            probe.stop()?;
            self.active_count = self.active_count.saturating_sub(1);
            Ok(())
        } else {
            Err(ProbeError::new(
                super::probe::ProbeErrorCode::NotInitialized,
                "Probe not found",
            ))
        }
    }

    /// Start all probes
    pub fn start_all(&mut self) -> Result<(), Vec<(ProbeId, ProbeError)>> {
        let mut errors = Vec::new();

        for (&id, probe) in &mut self.probes {
            if let Err(e) = probe.start() {
                errors.push((id, e));
            } else {
                self.active_count += 1;
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Stop all probes
    pub fn stop_all(&mut self) -> Result<(), Vec<(ProbeId, ProbeError)>> {
        let mut errors = Vec::new();

        for (&id, probe) in &mut self.probes {
            if let Err(e) = probe.stop() {
                errors.push((id, e));
            } else {
                self.active_count = self.active_count.saturating_sub(1);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Pause all probes
    pub fn pause_all(&mut self) -> Result<(), Vec<(ProbeId, ProbeError)>> {
        let mut errors = Vec::new();

        for (&id, probe) in &mut self.probes {
            if probe.state().is_active() {
                if let Err(e) = probe.pause() {
                    errors.push((id, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Resume all probes
    pub fn resume_all(&mut self) -> Result<(), Vec<(ProbeId, ProbeError)>> {
        let mut errors = Vec::new();

        for (&id, probe) in &mut self.probes {
            if probe.state().is_paused() {
                if let Err(e) = probe.resume() {
                    errors.push((id, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Count of registered probes
    pub fn count(&self) -> usize {
        self.probes.len()
    }

    /// Count of active probes
    pub fn active_count(&self) -> usize {
        self.active_count
    }

    /// Count by type
    pub fn count_by_type(&self, probe_type: ProbeType) -> usize {
        self.by_type.get(&probe_type).map(|v| v.len()).unwrap_or(0)
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    /// Clear all probes
    pub fn clear(&mut self) {
        self.probes.clear();
        self.by_type.clear();
        self.active_count = 0;
    }
}

impl Default for ProbeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::probes::CpuProbe;

    #[test]
    fn test_registry_register() {
        let mut registry = ProbeRegistry::new();

        let probe = Box::new(CpuProbe::new());
        let id = registry.register(probe);

        assert_eq!(registry.count(), 1);
        assert!(registry.get(id).is_some());
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = ProbeRegistry::new();

        let probe = Box::new(CpuProbe::new());
        let id = registry.register(probe);

        let removed = registry.unregister(id);
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_by_type() {
        let mut registry = ProbeRegistry::new();

        registry.register(Box::new(CpuProbe::new()));
        registry.register(Box::new(CpuProbe::new()));

        let cpu_probes = registry.by_type(ProbeType::Cpu);
        assert_eq!(cpu_probes.len(), 2);
    }
}
