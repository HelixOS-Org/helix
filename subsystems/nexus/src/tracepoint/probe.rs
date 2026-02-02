//! Probe Management
//!
//! Probe types, info, and management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ProbeId, TracepointId};

/// Probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProbeType {
    /// Static tracepoint probe
    Static,
    /// Dynamic kprobe
    Kprobe,
    /// Kretprobe (return probe)
    Kretprobe,
    /// Uprobe (userspace probe)
    Uprobe,
    /// Uretprobe (userspace return probe)
    Uretprobe,
    /// USDT (user-level statically defined tracing)
    Usdt,
}

/// Probe information
#[derive(Debug)]
pub struct ProbeInfo {
    /// Probe ID
    pub id: ProbeId,
    /// Probe type
    pub probe_type: ProbeType,
    /// Target tracepoint or address
    pub target: String,
    /// Is enabled
    pub enabled: bool,
    /// Hit count
    pub hits: AtomicU64,
    /// Registered timestamp
    pub registered_at: u64,
    /// Filter (optional)
    pub filter: Option<u64>, // Filter ID
}

impl ProbeInfo {
    /// Create new probe info
    pub fn new(id: ProbeId, probe_type: ProbeType, target: String, timestamp: u64) -> Self {
        Self {
            id,
            probe_type,
            target,
            enabled: false,
            hits: AtomicU64::new(0),
            registered_at: timestamp,
            filter: None,
        }
    }

    /// Record hit
    pub fn hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Get hit count
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Set filter
    pub fn set_filter(&mut self, filter_id: u64) {
        self.filter = Some(filter_id);
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        self.filter = None;
    }
}

/// Probe manager
pub struct ProbeManager {
    /// Registered probes
    probes: BTreeMap<ProbeId, ProbeInfo>,
    /// Probes by tracepoint
    by_tracepoint: BTreeMap<TracepointId, Vec<ProbeId>>,
    /// Next probe ID
    next_id: AtomicU64,
    /// Total probes registered
    total_registered: AtomicU64,
    /// Active probe count
    active_count: u32,
}

impl ProbeManager {
    /// Create new probe manager
    pub fn new() -> Self {
        Self {
            probes: BTreeMap::new(),
            by_tracepoint: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            total_registered: AtomicU64::new(0),
            active_count: 0,
        }
    }

    /// Allocate probe ID
    pub fn allocate_id(&self) -> ProbeId {
        ProbeId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Register probe
    pub fn register(&mut self, probe_type: ProbeType, target: String, timestamp: u64) -> ProbeId {
        let id = self.allocate_id();
        let info = ProbeInfo::new(id, probe_type, target, timestamp);
        self.probes.insert(id, info);
        self.total_registered.fetch_add(1, Ordering::Relaxed);
        id
    }

    /// Attach probe to tracepoint
    pub fn attach(&mut self, probe_id: ProbeId, tracepoint_id: TracepointId) -> bool {
        if !self.probes.contains_key(&probe_id) {
            return false;
        }

        self.by_tracepoint
            .entry(tracepoint_id)
            .or_default()
            .push(probe_id);
        true
    }

    /// Detach probe from tracepoint
    pub fn detach(&mut self, probe_id: ProbeId, tracepoint_id: TracepointId) -> bool {
        if let Some(probes) = self.by_tracepoint.get_mut(&tracepoint_id) {
            probes.retain(|&id| id != probe_id);
            return true;
        }
        false
    }

    /// Enable probe
    pub fn enable(&mut self, id: ProbeId) -> bool {
        if let Some(probe) = self.probes.get_mut(&id) {
            if !probe.enabled {
                probe.enabled = true;
                self.active_count += 1;
            }
            return true;
        }
        false
    }

    /// Disable probe
    pub fn disable(&mut self, id: ProbeId) -> bool {
        if let Some(probe) = self.probes.get_mut(&id) {
            if probe.enabled {
                probe.enabled = false;
                self.active_count = self.active_count.saturating_sub(1);
            }
            return true;
        }
        false
    }

    /// Unregister probe
    pub fn unregister(&mut self, id: ProbeId) -> bool {
        if let Some(probe) = self.probes.remove(&id) {
            if probe.enabled {
                self.active_count = self.active_count.saturating_sub(1);
            }
            // Remove from all tracepoints
            for probes in self.by_tracepoint.values_mut() {
                probes.retain(|&pid| pid != id);
            }
            return true;
        }
        false
    }

    /// Get probe
    pub fn get(&self, id: ProbeId) -> Option<&ProbeInfo> {
        self.probes.get(&id)
    }

    /// Get probe mutably
    pub fn get_mut(&mut self, id: ProbeId) -> Option<&mut ProbeInfo> {
        self.probes.get_mut(&id)
    }

    /// Get probes for tracepoint
    pub fn get_for_tracepoint(&self, tracepoint_id: TracepointId) -> Vec<&ProbeInfo> {
        self.by_tracepoint
            .get(&tracepoint_id)
            .map(|ids| ids.iter().filter_map(|id| self.probes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get active count
    pub fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Get total registered
    pub fn total_registered(&self) -> u64 {
        self.total_registered.load(Ordering::Relaxed)
    }

    /// Get all probes
    pub fn all(&self) -> impl Iterator<Item = &ProbeInfo> {
        self.probes.values()
    }
}

impl Default for ProbeManager {
    fn default() -> Self {
        Self::new()
    }
}
