//! Netfilter Intelligence
//!
//! Central coordinator for netfilter analysis.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    AddressFamily, ChainId, ChainType, NetfilterManager, RuleId, TableId, Verdict,
};

/// Rule analysis result
#[derive(Debug, Clone)]
pub struct RuleAnalysis {
    /// Rule ID
    pub rule_id: RuleId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Hit rate
    pub hit_rate: f32,
    /// Issues detected
    pub issues: Vec<NetfilterIssue>,
    /// Recommendations
    pub recommendations: Vec<NetfilterRecommendation>,
}

/// Netfilter issue
#[derive(Debug, Clone)]
pub struct NetfilterIssue {
    /// Issue type
    pub issue_type: NetfilterIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Netfilter issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetfilterIssueType {
    /// Unused rule
    UnusedRule,
    /// Shadowed rule
    ShadowedRule,
    /// Overly broad rule
    OverlyBroadRule,
    /// Missing conntrack
    MissingConntrack,
    /// High conntrack usage
    HighConntrackUsage,
    /// Inefficient order
    InefficientOrder,
}

/// Netfilter recommendation
#[derive(Debug, Clone)]
pub struct NetfilterRecommendation {
    /// Action
    pub action: NetfilterAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Netfilter actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetfilterAction {
    /// Delete unused rule
    DeleteUnused,
    /// Reorder rules
    ReorderRules,
    /// Add conntrack
    AddConntrack,
    /// Merge rules
    MergeRules,
    /// Split rule
    SplitRule,
    /// Increase conntrack limit
    IncreaseConntrackLimit,
}

/// Netfilter Intelligence
pub struct NetfilterIntelligence {
    /// Netfilter manager
    manager: NetfilterManager,
}

impl NetfilterIntelligence {
    /// Create new netfilter intelligence
    pub fn new() -> Self {
        Self {
            manager: NetfilterManager::new(),
        }
    }

    /// Create table
    #[inline(always)]
    pub fn create_table(
        &mut self,
        name: String,
        family: AddressFamily,
        timestamp: u64,
    ) -> TableId {
        self.manager.create_table(name, family, timestamp)
    }

    /// Create chain
    #[inline(always)]
    pub fn create_chain(
        &mut self,
        table_id: TableId,
        name: String,
        chain_type: ChainType,
    ) -> Option<ChainId> {
        self.manager.create_chain(table_id, name, chain_type)
    }

    /// Add rule
    #[inline(always)]
    pub fn add_rule(
        &mut self,
        chain_id: ChainId,
        verdict: Verdict,
        timestamp: u64,
    ) -> Option<RuleId> {
        self.manager.add_rule(chain_id, verdict, timestamp)
    }

    /// Analyze rule
    pub fn analyze_rule(&self, id: RuleId) -> Option<RuleAnalysis> {
        let rule = self.manager.get_rule(id)?;

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        let hits = rule.hit_count();

        // Check for unused rules
        if hits == 0 {
            health_score -= 15.0;
            issues.push(NetfilterIssue {
                issue_type: NetfilterIssueType::UnusedRule,
                severity: 4,
                description: String::from("Rule has never been hit"),
            });
            recommendations.push(NetfilterRecommendation {
                action: NetfilterAction::DeleteUnused,
                expected_improvement: 5.0,
                reason: String::from("Consider removing unused rule"),
            });
        }

        // Check for overly broad rules
        if rule.matches.is_empty() && matches!(rule.verdict, Verdict::Accept) {
            health_score -= 25.0;
            issues.push(NetfilterIssue {
                issue_type: NetfilterIssueType::OverlyBroadRule,
                severity: 7,
                description: String::from("Rule accepts all traffic without conditions"),
            });
        }

        health_score = health_score.max(0.0);

        // Calculate hit rate (simplified)
        let total = self.manager.total_packets();
        let hit_rate = if total > 0 {
            hits as f32 / total as f32 * 100.0
        } else {
            0.0
        };

        Some(RuleAnalysis {
            rule_id: id,
            health_score,
            hit_rate,
            issues,
            recommendations,
        })
    }

    /// Analyze conntrack
    pub fn analyze_conntrack(&self) -> (f32, Vec<NetfilterIssue>) {
        let conntrack = self.manager.conntrack();
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();

        // Check usage
        let usage = conntrack.entry_count() as f32 / conntrack.max_entries as f32;
        if usage > 0.8 {
            health_score -= 25.0;
            issues.push(NetfilterIssue {
                issue_type: NetfilterIssueType::HighConntrackUsage,
                severity: 8,
                description: format!("Conntrack is {:.1}% full", usage * 100.0),
            });
        }

        (health_score, issues)
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &NetfilterManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut NetfilterManager {
        &mut self.manager
    }
}

impl Default for NetfilterIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
