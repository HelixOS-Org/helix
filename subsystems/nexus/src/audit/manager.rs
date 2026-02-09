//! Audit Manager
//!
//! Audit event processing and rule management.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{AuditEvent, AuditLog, AuditRule, AuditRuleId, RuleAction, RuleList};

/// Audit manager
pub struct AuditManager {
    /// Audit log
    log: AuditLog,
    /// Rules
    rules: BTreeMap<AuditRuleId, AuditRule>,
    /// Next rule ID
    next_rule_id: AtomicU64,
    /// Total events
    total_events: AtomicU64,
    /// Filtered events
    filtered_events: AtomicU64,
}

impl AuditManager {
    /// Create new audit manager
    pub fn new(log_size: usize) -> Self {
        Self {
            log: AuditLog::new(log_size),
            rules: BTreeMap::new(),
            next_rule_id: AtomicU64::new(1),
            total_events: AtomicU64::new(0),
            filtered_events: AtomicU64::new(0),
        }
    }

    /// Add rule
    #[inline]
    pub fn add_rule(&mut self, action: RuleAction, list: RuleList, timestamp: u64) -> AuditRuleId {
        let id = AuditRuleId::new(self.next_rule_id.fetch_add(1, Ordering::Relaxed));
        let rule = AuditRule::new(id, action, list, timestamp);
        self.rules.insert(id, rule);
        id
    }

    /// Remove rule
    #[inline(always)]
    pub fn remove_rule(&mut self, id: AuditRuleId) -> bool {
        self.rules.remove(&id).is_some()
    }

    /// Get rule
    #[inline(always)]
    pub fn get_rule(&self, id: AuditRuleId) -> Option<&AuditRule> {
        self.rules.get(&id)
    }

    /// Get rule mutably
    #[inline(always)]
    pub fn get_rule_mut(&mut self, id: AuditRuleId) -> Option<&mut AuditRule> {
        self.rules.get_mut(&id)
    }

    /// Get all rules
    #[inline(always)]
    pub fn all_rules(&self) -> impl Iterator<Item = &AuditRule> {
        self.rules.values()
    }

    /// Rule count
    #[inline(always)]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Process event
    pub fn process_event(&mut self, event: AuditEvent) {
        self.total_events.fetch_add(1, Ordering::Relaxed);

        // Check rules
        let mut should_log = false;
        for rule in self.rules.values() {
            if rule.matches(&event) {
                match rule.action {
                    RuleAction::Always => should_log = true,
                    RuleAction::Never => {
                        self.filtered_events.fetch_add(1, Ordering::Relaxed);
                        return;
                    },
                }
            }
        }

        if should_log || self.rules.is_empty() {
            self.log.log(event);
        }
    }

    /// Get log
    #[inline(always)]
    pub fn log(&self) -> &AuditLog {
        &self.log
    }

    /// Get log mutably
    #[inline(always)]
    pub fn log_mut(&mut self) -> &mut AuditLog {
        &mut self.log
    }

    /// Get total events
    #[inline(always)]
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }

    /// Get filtered events
    #[inline(always)]
    pub fn filtered_events(&self) -> u64 {
        self.filtered_events.load(Ordering::Relaxed)
    }

    /// Check if rules are empty
    #[inline(always)]
    pub fn has_rules(&self) -> bool {
        !self.rules.is_empty()
    }
}

impl Default for AuditManager {
    fn default() -> Self {
        Self::new(10000)
    }
}
