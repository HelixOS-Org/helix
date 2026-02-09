//! Privilege Usage Analysis
//!
//! Least privilege analysis and recommendations.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{Capability, CapabilitySet, Pid, ProcessCaps};

/// Privilege usage record
#[derive(Debug, Clone, Default)]
pub struct PrivilegeUsage {
    /// Capability usage count
    pub usage_count: BTreeMap<Capability, u64>,
    /// Capability last used
    pub last_used: BTreeMap<Capability, u64>,
    /// Denied capabilities
    pub denied: BTreeMap<Capability, u64>,
    /// Profile start
    pub profile_start: u64,
    /// Profile end
    pub profile_end: Option<u64>,
}

impl PrivilegeUsage {
    /// Create new usage record
    pub fn new(start_time: u64) -> Self {
        Self {
            usage_count: BTreeMap::new(),
            last_used: BTreeMap::new(),
            denied: BTreeMap::new(),
            profile_start: start_time,
            profile_end: None,
        }
    }

    /// Record usage
    #[inline(always)]
    pub fn record_usage(&mut self, cap: Capability, timestamp: u64) {
        *self.usage_count.entry(cap).or_insert(0) += 1;
        self.last_used.insert(cap, timestamp);
    }

    /// Record denial
    #[inline(always)]
    pub fn record_denial(&mut self, cap: Capability) {
        *self.denied.entry(cap).or_insert(0) += 1;
    }

    /// Get unused capabilities from set
    #[inline]
    pub fn unused_from(&self, set: &CapabilitySet) -> Vec<Capability> {
        set.iter()
            .filter(|c| !self.usage_count.contains_key(c))
            .collect()
    }

    /// Get used capabilities
    #[inline(always)]
    pub fn used_capabilities(&self) -> Vec<Capability> {
        self.usage_count.keys().copied().collect()
    }

    /// Finish profile
    #[inline(always)]
    pub fn finish(&mut self, timestamp: u64) {
        self.profile_end = Some(timestamp);
    }
}

/// Least privilege recommendation
#[derive(Debug, Clone)]
pub struct LeastPrivilegeRec {
    /// Capabilities to keep
    pub keep: Vec<Capability>,
    /// Capabilities to drop
    pub drop: Vec<Capability>,
    /// Reduction percentage
    pub reduction_percent: f32,
    /// Risk reduction
    pub risk_reduction: f32,
    /// Reason
    pub reasons: Vec<String>,
}

impl LeastPrivilegeRec {
    /// Create new recommendation
    pub fn new() -> Self {
        Self {
            keep: Vec::new(),
            drop: Vec::new(),
            reduction_percent: 0.0,
            risk_reduction: 0.0,
            reasons: Vec::new(),
        }
    }
}

impl Default for LeastPrivilegeRec {
    fn default() -> Self {
        Self::new()
    }
}

/// Least privilege analyzer
pub struct LeastPrivilegeAnalyzer {
    /// Usage profiles by process
    profiles: BTreeMap<Pid, PrivilegeUsage>,
    /// Template profiles by name
    templates: BTreeMap<String, CapabilitySet>,
}

impl LeastPrivilegeAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        let mut analyzer = Self {
            profiles: BTreeMap::new(),
            templates: BTreeMap::new(),
        };
        analyzer.init_templates();
        analyzer
    }

    /// Initialize common templates
    fn init_templates(&mut self) {
        // Web server
        self.templates.insert(
            String::from("web_server"),
            CapabilitySet::from_list(&[
                Capability::NetBindService,
                Capability::Setuid,
                Capability::Setgid,
            ]),
        );

        // Container runtime
        self.templates.insert(
            String::from("container"),
            CapabilitySet::from_list(&[
                Capability::SysAdmin,
                Capability::NetAdmin,
                Capability::Mknod,
                Capability::SysChroot,
                Capability::Setuid,
                Capability::Setgid,
                Capability::Setpcap,
                Capability::Setfcap,
            ]),
        );

        // Network tool
        self.templates.insert(
            String::from("network_tool"),
            CapabilitySet::from_list(&[Capability::NetAdmin, Capability::NetRaw]),
        );

        // Minimal
        self.templates
            .insert(String::from("minimal"), CapabilitySet::new());
    }

    /// Start profiling
    #[inline(always)]
    pub fn start_profile(&mut self, pid: Pid, timestamp: u64) {
        self.profiles.insert(pid, PrivilegeUsage::new(timestamp));
    }

    /// Record usage
    #[inline]
    pub fn record_usage(&mut self, pid: Pid, cap: Capability, timestamp: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_usage(cap, timestamp);
        }
    }

    /// Analyze and recommend
    pub fn analyze(&self, pid: Pid, current: &ProcessCaps) -> LeastPrivilegeRec {
        let mut rec = LeastPrivilegeRec::new();

        if let Some(profile) = self.profiles.get(&pid) {
            let used = profile.used_capabilities();
            let unused = profile.unused_from(&current.effective);

            rec.keep = used.clone();
            rec.drop = unused.clone();

            let total = current.effective.count() as f32;
            if total > 0.0 {
                rec.reduction_percent = (rec.drop.len() as f32 / total) * 100.0;
            }

            // Calculate risk reduction
            let mut risk_before = 0.0f32;
            let mut risk_after = 0.0f32;

            for cap in current.effective.iter() {
                risk_before += cap.risk_level().score() as f32;
            }

            for cap in &used {
                risk_after += cap.risk_level().score() as f32;
            }

            if risk_before > 0.0 {
                rec.risk_reduction = ((risk_before - risk_after) / risk_before) * 100.0;
            }

            // Add reasons
            for cap in &rec.drop {
                rec.reasons.push(alloc::format!(
                    "Drop {}: unused during profiling",
                    cap.name()
                ));
            }
        }

        rec
    }

    /// Get template
    #[inline(always)]
    pub fn get_template(&self, name: &str) -> Option<&CapabilitySet> {
        self.templates.get(name)
    }

    /// Add template
    #[inline(always)]
    pub fn add_template(&mut self, name: String, caps: CapabilitySet) {
        self.templates.insert(name, caps);
    }

    /// Get profile
    #[inline(always)]
    pub fn get_profile(&self, pid: Pid) -> Option<&PrivilegeUsage> {
        self.profiles.get(&pid)
    }
}

impl Default for LeastPrivilegeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
