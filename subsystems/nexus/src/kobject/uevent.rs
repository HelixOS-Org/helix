//! Uevent Handler
//!
//! Handling kernel object uevents.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{KobjectId, UeventAction};

/// Uevent
#[derive(Debug, Clone)]
pub struct Uevent {
    /// Action
    pub action: UeventAction,
    /// Device path
    pub devpath: String,
    /// Subsystem
    pub subsystem: String,
    /// Kobject
    pub kobject: KobjectId,
    /// Environment variables
    pub env: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: u64,
    /// Sequence number
    pub seqnum: u64,
}

impl Uevent {
    /// Create new uevent
    pub fn new(
        action: UeventAction,
        devpath: String,
        subsystem: String,
        kobject: KobjectId,
        timestamp: u64,
        seqnum: u64,
    ) -> Self {
        let mut env = BTreeMap::new();
        env.insert(String::from("ACTION"), String::from(action.as_str()));
        env.insert(String::from("DEVPATH"), devpath.clone());
        env.insert(String::from("SUBSYSTEM"), subsystem.clone());
        env.insert(String::from("SEQNUM"), format!("{}", seqnum));

        Self {
            action,
            devpath,
            subsystem,
            kobject,
            env,
            timestamp,
            seqnum,
        }
    }

    /// Add environment variable
    pub fn add_env(&mut self, key: String, value: String) {
        self.env.insert(key, value);
    }

    /// Format as netlink message
    pub fn to_netlink_format(&self) -> String {
        let mut msg = format!("{}@{}\0", self.action.as_str(), self.devpath);
        for (key, value) in &self.env {
            msg.push_str(&format!("{}={}\0", key, value));
        }
        msg
    }
}

/// Uevent handler
pub struct UeventHandler {
    /// Pending uevents
    pending: Vec<Uevent>,
    /// Sent uevents
    history: Vec<Uevent>,
    /// Maximum history
    max_history: usize,
    /// Next sequence number
    next_seqnum: AtomicU64,
    /// Uevent suppression enabled
    suppressed: bool,
    /// Per-subsystem counts
    subsystem_counts: BTreeMap<String, u64>,
    /// Total uevents sent
    total_sent: AtomicU64,
}

impl UeventHandler {
    /// Create new uevent handler
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            history: Vec::with_capacity(1000),
            max_history: 1000,
            next_seqnum: AtomicU64::new(1),
            suppressed: false,
            subsystem_counts: BTreeMap::new(),
            total_sent: AtomicU64::new(0),
        }
    }

    /// Queue uevent
    pub fn queue_uevent(
        &mut self,
        action: UeventAction,
        devpath: String,
        subsystem: String,
        kobject: KobjectId,
        timestamp: u64,
    ) -> u64 {
        let seqnum = self.next_seqnum.fetch_add(1, Ordering::Relaxed);
        let uevent = Uevent::new(
            action,
            devpath,
            subsystem.clone(),
            kobject,
            timestamp,
            seqnum,
        );

        *self.subsystem_counts.entry(subsystem).or_default() += 1;
        self.pending.push(uevent);

        seqnum
    }

    /// Send pending uevents
    pub fn send_pending(&mut self) -> Vec<Uevent> {
        if self.suppressed {
            return Vec::new();
        }

        let events: Vec<_> = self.pending.drain(..).collect();

        for event in &events {
            if self.history.len() >= self.max_history {
                self.history.remove(0);
            }
            self.history.push(event.clone());
            self.total_sent.fetch_add(1, Ordering::Relaxed);
        }

        events
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Suppress uevents
    pub fn suppress(&mut self, suppress: bool) {
        self.suppressed = suppress;
    }

    /// Check if suppressed
    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    /// Get total sent
    pub fn total_sent(&self) -> u64 {
        self.total_sent.load(Ordering::Relaxed)
    }

    /// Get subsystem count
    pub fn subsystem_count(&self, subsystem: &str) -> u64 {
        self.subsystem_counts.get(subsystem).copied().unwrap_or(0)
    }

    /// Get recent uevents
    pub fn recent_uevents(&self, limit: usize) -> &[Uevent] {
        let start = self.history.len().saturating_sub(limit);
        &self.history[start..]
    }
}

impl Default for UeventHandler {
    fn default() -> Self {
        Self::new()
    }
}
