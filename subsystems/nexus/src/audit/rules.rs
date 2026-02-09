//! Audit Rules
//!
//! Rule definitions and matching logic.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{AuditEvent, AuditResult, AuditRuleId};

/// Rule action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// Never audit
    Never,
    /// Always audit
    Always,
}

/// Rule list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleList {
    /// Task list (process creation)
    Task,
    /// Exit list (syscall exit)
    Exit,
    /// User list (userspace messages)
    User,
    /// Filesystem list
    Filesystem,
    /// Exclude list
    Exclude,
}

/// Field comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
    /// Bitwise AND
    BitAnd,
    /// Bitwise test
    BitTest,
}

/// Rule field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// PID
    Pid,
    /// UID
    Uid,
    /// EUID
    Euid,
    /// SUID
    Suid,
    /// FSUID
    Fsuid,
    /// GID
    Gid,
    /// EGID
    Egid,
    /// SGID
    Sgid,
    /// FSGID
    Fsgid,
    /// Login UID
    Loginuid,
    /// Architecture
    Arch,
    /// Syscall number
    Syscall,
    /// Exit value
    Exit,
    /// Success/failure
    Success,
    /// Path
    Path,
    /// Directory
    Dir,
    /// File type
    FileType,
    /// Inode
    Inode,
    /// Object UID
    ObjUid,
    /// Object GID
    ObjGid,
    /// Object level low
    ObjLevLow,
    /// Object level high
    ObjLevHigh,
    /// Permission
    Perm,
    /// Device major
    DevMajor,
    /// Device minor
    DevMinor,
    /// Session ID
    SessionId,
}

/// Rule field
#[derive(Debug, Clone)]
pub struct RuleField {
    /// Field type
    pub field_type: FieldType,
    /// Operator
    pub op: FieldOp,
    /// Value
    pub value: u64,
    /// String value (for paths)
    pub str_value: Option<String>,
}

impl RuleField {
    /// Create numeric field
    #[inline]
    pub fn numeric(field_type: FieldType, op: FieldOp, value: u64) -> Self {
        Self {
            field_type,
            op,
            value,
            str_value: None,
        }
    }

    /// Create string field
    #[inline]
    pub fn string(field_type: FieldType, op: FieldOp, str_value: String) -> Self {
        Self {
            field_type,
            op,
            value: 0,
            str_value: Some(str_value),
        }
    }

    /// Match against event
    pub fn matches(&self, event: &AuditEvent) -> bool {
        let event_value = match self.field_type {
            FieldType::Pid => event.process.pid.raw() as u64,
            FieldType::Uid => event.process.uid.raw() as u64,
            FieldType::Euid => event.process.euid.raw() as u64,
            FieldType::Gid => event.process.gid.raw() as u64,
            FieldType::Syscall => event
                .syscall
                .as_ref()
                .map(|s| s.syscall.raw() as u64)
                .unwrap_or(0),
            FieldType::Exit => event.syscall.as_ref().map(|s| s.exit as u64).unwrap_or(0),
            FieldType::Success => match event.result {
                AuditResult::Success => 1,
                AuditResult::Failure => 0,
                AuditResult::Unknown => 0,
            },
            _ => 0,
        };

        match self.op {
            FieldOp::Eq => event_value == self.value,
            FieldOp::Ne => event_value != self.value,
            FieldOp::Lt => event_value < self.value,
            FieldOp::Le => event_value <= self.value,
            FieldOp::Gt => event_value > self.value,
            FieldOp::Ge => event_value >= self.value,
            FieldOp::BitAnd => (event_value & self.value) != 0,
            FieldOp::BitTest => (event_value & self.value) == self.value,
        }
    }
}

/// Audit rule
#[derive(Debug)]
pub struct AuditRule {
    /// Rule ID
    pub id: AuditRuleId,
    /// Action
    pub action: RuleAction,
    /// List
    pub list: RuleList,
    /// Fields
    pub fields: Vec<RuleField>,
    /// Key
    pub key: Option<String>,
    /// Enabled
    pub enabled: bool,
    /// Hit count
    pub hits: AtomicU64,
    /// Created timestamp
    pub created_at: u64,
}

impl AuditRule {
    /// Create new rule
    pub fn new(id: AuditRuleId, action: RuleAction, list: RuleList, timestamp: u64) -> Self {
        Self {
            id,
            action,
            list,
            fields: Vec::new(),
            key: None,
            enabled: true,
            hits: AtomicU64::new(0),
            created_at: timestamp,
        }
    }

    /// Add field
    #[inline(always)]
    pub fn add_field(&mut self, field: RuleField) {
        self.fields.push(field);
    }

    /// Set key
    #[inline(always)]
    pub fn with_key(mut self, key: String) -> Self {
        self.key = Some(key);
        self
    }

    /// Match event
    pub fn matches(&self, event: &AuditEvent) -> bool {
        if !self.enabled {
            return false;
        }

        // All fields must match
        for field in &self.fields {
            if !field.matches(event) {
                return false;
            }
        }

        self.hits.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Get hit count
    #[inline(always)]
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Enable rule
    #[inline(always)]
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable rule
    #[inline(always)]
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}
