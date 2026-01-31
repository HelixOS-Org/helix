//! Knowledge transfer between domains
//!
//! This module provides knowledge transfer capabilities.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::types::Timestamp;

/// Knowledge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeType {
    /// Rule
    Rule,
    /// Pattern
    Pattern,
    /// Procedure
    Procedure,
    /// Hypothesis
    Hypothesis,
}

/// Knowledge item
#[derive(Debug, Clone)]
pub struct KnowledgeItem {
    /// Knowledge type
    pub knowledge_type: KnowledgeType,
    /// ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Content (serialized)
    pub content: String,
    /// Transferability (0-1)
    pub transferability: f32,
    /// Domains applicable
    pub domains: Vec<String>,
}

/// Transfer record
#[derive(Debug, Clone)]
pub struct TransferRecord {
    /// Source domain
    pub source_domain: String,
    /// Target domain
    pub target_domain: String,
    /// Items transferred
    pub items: Vec<String>,
    /// Success
    pub success: bool,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Knowledge transfer manager
pub struct KnowledgeTransfer {
    /// Knowledge base
    knowledge: Vec<KnowledgeItem>,
    /// Transfer history
    transfers: Vec<TransferRecord>,
}

impl KnowledgeTransfer {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            knowledge: Vec::new(),
            transfers: Vec::new(),
        }
    }

    /// Add knowledge
    pub fn add(&mut self, item: KnowledgeItem) {
        self.knowledge.push(item);
    }

    /// Find transferable knowledge
    pub fn find_transferable(&self, target_domain: &str) -> Vec<&KnowledgeItem> {
        self.knowledge
            .iter()
            .filter(|k| {
                k.transferability >= 0.5
                    && (k.domains.is_empty() || k.domains.contains(&String::from(target_domain)))
            })
            .collect()
    }

    /// Record transfer
    pub fn record_transfer(
        &mut self,
        source: &str,
        target: &str,
        items: Vec<String>,
        success: bool,
        timestamp: u64,
    ) {
        self.transfers.push(TransferRecord {
            source_domain: String::from(source),
            target_domain: String::from(target),
            items,
            success,
            timestamp: Timestamp::new(timestamp),
        });
    }

    /// Knowledge count
    pub fn count(&self) -> usize {
        self.knowledge.len()
    }
}

impl Default for KnowledgeTransfer {
    fn default() -> Self {
        Self::new()
    }
}
