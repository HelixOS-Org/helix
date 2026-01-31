//! Audit Subsystem
//!
//! Comprehensive security audit infrastructure for the NEXUS kernel subsystem.
//!
//! # Architecture
//!
//! The audit module provides:
//! - **Types**: Core identifiers and audit message types
//! - **Syscall**: System call tracking and information
//! - **Event**: Audit event and process context structures
//! - **Rules**: Audit rule definitions and matching logic
//! - **Log**: Audit log buffer and search functionality
//! - **Anomaly**: Security anomaly detection and baseline analysis
//! - **Compliance**: Compliance framework checks
//! - **Manager**: Central audit event processing
//! - **Intelligence**: AI-powered security analysis engine
//!
//! # Usage
//!
//! ```rust,ignore
//! use nexus::audit::{AuditIntelligence, AuditEvent, ProcessContext, RuleAction, RuleList};
//!
//! let mut intel = AuditIntelligence::new();
//!
//! // Add rule
//! intel.add_rule(RuleAction::Always, RuleList::Exit, timestamp);
//!
//! // Process event
//! let ctx = ProcessContext::new(Pid::new(1234), Uid::new(1000));
//! let event = AuditEvent::new(intel.manager().log().allocate_id(),
//!                             AuditMessageType::Syscall, timestamp, ctx);
//! intel.process_event(event, timestamp);
//!
//! // Analyze
//! let analysis = intel.analyze();
//! ```

extern crate alloc;

// Submodules
pub mod anomaly;
pub mod compliance;
pub mod event;
pub mod intelligence;
pub mod log;
pub mod manager;
pub mod rules;
pub mod syscall;
pub mod types;

// Re-export all public types
pub use anomaly::{Anomaly, AnomalyDetector, AnomalyType, BaselineStats};
pub use compliance::{ComplianceCheck, ComplianceFramework};
pub use event::{AuditEvent, PathRecord, ProcessContext};
pub use intelligence::{
    AuditAction, AuditAnalysis, AuditIntelligence, AuditIssue, AuditIssueType,
    AuditRecommendation,
};
pub use log::AuditLog;
pub use manager::AuditManager;
pub use rules::{AuditRule, FieldOp, FieldType, RuleAction, RuleField, RuleList};
pub use syscall::{SyscallInfo, SyscallNum};
pub use types::{AuditEventId, AuditMessageType, AuditResult, AuditRuleId, Gid, Pid, Uid};
