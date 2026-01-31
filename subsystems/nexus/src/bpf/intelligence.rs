//! BPF Intelligence
//!
//! AI-powered BPF program analysis and optimization.

use alloc::string::String;
use alloc::vec::Vec;

use super::{BpfInsn, BpfManager, BpfMapId, BpfMapType, BpfProgId, BpfProgState, BpfProgType};

/// BPF analysis result
#[derive(Debug, Clone)]
pub struct BpfAnalysis {
    /// Program ID
    pub prog_id: BpfProgId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Performance score
    pub performance_score: f32,
    /// Issues detected
    pub issues: Vec<BpfIssue>,
    /// Recommendations
    pub recommendations: Vec<BpfRecommendation>,
}

/// BPF issue
#[derive(Debug, Clone)]
pub struct BpfIssue {
    /// Issue type
    pub issue_type: BpfIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

impl BpfIssue {
    /// Create new issue
    pub fn new(issue_type: BpfIssueType, severity: u8, description: String) -> Self {
        Self {
            issue_type,
            severity,
            description,
        }
    }
}

/// BPF issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfIssueType {
    /// High CPU usage
    HighCpuUsage,
    /// Map overflow risk
    MapOverflowRisk,
    /// Not JIT compiled
    NotJitCompiled,
    /// Slow helper call
    SlowHelperCall,
    /// Large program
    LargeProgram,
    /// Verification warning
    VerificationWarning,
}

/// BPF recommendation
#[derive(Debug, Clone)]
pub struct BpfRecommendation {
    /// Action
    pub action: BpfAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

impl BpfRecommendation {
    /// Create new recommendation
    pub fn new(action: BpfAction, expected_improvement: f32, reason: String) -> Self {
        Self {
            action,
            expected_improvement,
            reason,
        }
    }
}

/// BPF actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfAction {
    /// Enable JIT
    EnableJit,
    /// Increase map size
    IncreaseMapSize,
    /// Optimize program
    OptimizeProgram,
    /// Use batch operations
    UseBatchOps,
    /// Split program
    SplitProgram,
}

/// BPF Intelligence - comprehensive BPF analysis and optimization
pub struct BpfIntelligence {
    /// BPF manager
    manager: BpfManager,
}

impl BpfIntelligence {
    /// Create new BPF intelligence
    pub fn new() -> Self {
        Self {
            manager: BpfManager::new(),
        }
    }

    /// Load program
    pub fn load_program(
        &mut self,
        name: String,
        prog_type: BpfProgType,
        insns: &[BpfInsn],
        timestamp: u64,
    ) -> Result<BpfProgId, String> {
        self.manager.load_program(name, prog_type, insns, timestamp)
    }

    /// Create map
    pub fn create_map(
        &mut self,
        name: String,
        map_type: BpfMapType,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
        timestamp: u64,
    ) -> BpfMapId {
        self.manager
            .create_map(name, map_type, key_size, value_size, max_entries, timestamp)
    }

    /// Analyze program
    pub fn analyze_program(&self, id: BpfProgId) -> Option<BpfAnalysis> {
        let prog = self.manager.get_program(id)?;

        let mut health_score = 100.0f32;
        let mut performance_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check JIT status
        if !matches!(prog.state, BpfProgState::JitCompiled) {
            health_score -= 15.0;
            performance_score -= 30.0;
            issues.push(BpfIssue {
                issue_type: BpfIssueType::NotJitCompiled,
                severity: 5,
                description: String::from("Program not JIT compiled"),
            });
            recommendations.push(BpfRecommendation {
                action: BpfAction::EnableJit,
                expected_improvement: 30.0,
                reason: String::from("Enable JIT for better performance"),
            });
        }

        // Check program size
        if prog.insn_count > 10000 {
            health_score -= 10.0;
            issues.push(BpfIssue {
                issue_type: BpfIssueType::LargeProgram,
                severity: 4,
                description: alloc::format!("Program has {} instructions", prog.insn_count),
            });
            recommendations.push(BpfRecommendation {
                action: BpfAction::SplitProgram,
                expected_improvement: 15.0,
                reason: String::from("Consider splitting into tail calls"),
            });
        }

        // Check run time
        let avg_time = prog.avg_run_time();
        if avg_time > 100000.0 {
            health_score -= 20.0;
            performance_score -= 25.0;
            issues.push(BpfIssue {
                issue_type: BpfIssueType::HighCpuUsage,
                severity: 7,
                description: alloc::format!("High average run time: {:.0}ns", avg_time),
            });
            recommendations.push(BpfRecommendation {
                action: BpfAction::OptimizeProgram,
                expected_improvement: 20.0,
                reason: String::from("Optimize hot paths in program"),
            });
        }

        health_score = health_score.max(0.0);
        performance_score = performance_score.max(0.0);

        Some(BpfAnalysis {
            prog_id: id,
            health_score,
            performance_score,
            issues,
            recommendations,
        })
    }

    /// Analyze map
    pub fn analyze_map(&self, id: BpfMapId) -> Option<(f32, Vec<BpfIssue>)> {
        let map = self.manager.get_map(id)?;

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();

        // Check fill ratio
        let fill_ratio = map.fill_ratio();
        if fill_ratio > 0.9 {
            health_score -= 20.0;
            issues.push(BpfIssue {
                issue_type: BpfIssueType::MapOverflowRisk,
                severity: 8,
                description: alloc::format!("Map is {:.1}% full", fill_ratio * 100.0),
            });
        }

        Some((health_score, issues))
    }

    /// Get manager
    pub fn manager(&self) -> &BpfManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut BpfManager {
        &mut self.manager
    }
}

impl Default for BpfIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
