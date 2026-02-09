//! Denial Tracking
//!
//! LSM denial recording and analysis.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{LsmType, ObjectClass, Permission, Pid, SecurityContext};

/// Denial record
#[derive(Debug, Clone)]
pub struct Denial {
    /// Timestamp
    pub timestamp: u64,
    /// Process ID
    pub pid: Pid,
    /// Source context
    pub source: SecurityContext,
    /// Target context
    pub target: SecurityContext,
    /// Object class
    pub class: ObjectClass,
    /// Requested permissions
    pub permissions: Vec<Permission>,
    /// Path (if applicable)
    pub path: Option<String>,
    /// Comm (process name)
    pub comm: Option<String>,
    /// LSM type
    pub lsm: LsmType,
}

impl Denial {
    /// Create new denial
    pub fn new(
        timestamp: u64,
        pid: Pid,
        source: SecurityContext,
        target: SecurityContext,
        class: ObjectClass,
        permissions: Vec<Permission>,
        lsm: LsmType,
    ) -> Self {
        Self {
            timestamp,
            pid,
            source,
            target,
            class,
            permissions,
            path: None,
            comm: None,
            lsm,
        }
    }

    /// Is sensitive denial
    #[inline(always)]
    pub fn is_sensitive(&self) -> bool {
        self.permissions.iter().any(|p| p.is_sensitive())
    }
}

/// Denial tracker
pub struct DenialTracker {
    /// Denials
    denials: VecDeque<Denial>,
    /// Max denials
    max_denials: usize,
    /// Denial count by source type
    by_source_type: BTreeMap<String, u64>,
    /// Denial count by target type
    by_target_type: BTreeMap<String, u64>,
    /// Total denials
    total: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl DenialTracker {
    /// Create new tracker
    pub fn new(max_denials: usize) -> Self {
        Self {
            denials: VecDeque::new(),
            max_denials,
            by_source_type: BTreeMap::new(),
            by_target_type: BTreeMap::new(),
            total: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Record denial
    pub fn record(&mut self, denial: Denial) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        self.total.fetch_add(1, Ordering::Relaxed);

        *self
            .by_source_type
            .entry(denial.source.type_.clone())
            .or_insert(0) += 1;
        *self
            .by_target_type
            .entry(denial.target.type_.clone())
            .or_insert(0) += 1;

        if self.denials.len() >= self.max_denials {
            self.denials.pop_front();
        }
        self.denials.push_back(denial);
    }

    /// Get recent denials
    #[inline(always)]
    pub fn recent(&self, count: usize) -> &[Denial] {
        let start = self.denials.len().saturating_sub(count);
        &self.denials[start..]
    }

    /// Get denials by source type
    #[inline]
    pub fn by_source(&self, type_: &str) -> Vec<&Denial> {
        self.denials
            .iter()
            .filter(|d| d.source.type_ == type_)
            .collect()
    }

    /// Get total
    #[inline(always)]
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// Get top denied source types
    #[inline]
    pub fn top_sources(&self, n: usize) -> Vec<(String, u64)> {
        let mut sorted: Vec<_> = self
            .by_source_type
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Enable/disable
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for DenialTracker {
    fn default() -> Self {
        Self::new(10000)
    }
}
